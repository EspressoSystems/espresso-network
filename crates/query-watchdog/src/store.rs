use std::{path::Path, str::FromStr, time::Duration};

use anyhow::{Context, bail};
use chrono::{DateTime, Utc};
use sqlx::{
    QueryBuilder,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions},
};
use strum::VariantArray;

use crate::chunk::{DataKind, Probe};

/// SQLite state of all node watches; WAL, so litestream can replicate it.
#[derive(Clone, Debug)]
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    /// `connect` on `path` with WAL, `busy_timeout` 30 s, `create_if_missing`.
    pub async fn open(path: &Path) -> anyhow::Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(30));
        Self::connect(options)
            .await
            .with_context(|| format!("opening store {}", path.display()))
    }

    /// One-connection pool on `options` with no idle timeout or max lifetime, then
    /// `sqlx::migrate!("./migrations")`; tests pass `sqlite::memory:` options.
    pub async fn connect(options: SqliteConnectOptions) -> anyhow::Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(options)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    /// Deletes rows of every node not in `node_labels` in one transaction (`QueryBuilder`
    /// `NOT IN`); returns `rows_affected` of the `node_progress` delete.
    pub async fn forget_other_nodes(&self, node_labels: &[String]) -> anyhow::Result<u64> {
        let mut tx = self.pool.begin().await?;
        let mut forgotten = 0;
        for table in ["missing_chunk", "node_progress"] {
            let mut query = QueryBuilder::new(format!("DELETE FROM {table} WHERE node NOT IN ("));
            let mut labels = query.separated(", ");
            for label in node_labels {
                labels.push_bind(label);
            }
            query.push(")");
            forgotten = query.build().execute(&mut *tx).await?.rows_affected();
        }
        tx.commit().await?;
        Ok(forgotten)
    }

    /// Handle scoped to one node label.
    pub fn node(&self, label: &str) -> NodeStore {
        NodeStore {
            pool: self.pool.clone(),
            node: label.to_owned(),
        }
    }
}

/// Store rows of one node.
#[derive(Clone, Debug)]
pub struct NodeStore {
    pool: SqlitePool,
    node: String,
}

/// First retry delay of a missing chunk.
const RETRY_DELAY_MIN_SECS: i64 = 10;
/// Cap of the doubling retry delay.
const RETRY_DELAY_MAX_SECS: i64 = 600;

type ProgressRow = (
    u64,
    Option<DateTime<Utc>>,
    Option<u64>,
    Option<u64>,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
);

impl NodeStore {
    /// The node's progress; `None` on first run.
    pub async fn load_progress(&self) -> anyhow::Result<Option<NodeProgress>> {
        let row: Option<ProgressRow> = sqlx::query_as(
            "SELECT follow_next_height, next_sweep_at, sweep_until_height, sweep_next_height, \
             sweep_started_at, last_sweep_completed_at FROM node_progress WHERE node = ?",
        )
        .bind(&self.node)
        .fetch_optional(&self.pool)
        .await?;
        row.map(
            |(follow_next_height, waiting, until, next, started, completed)| {
                let sweep = match (waiting, until, next, started) {
                    (Some(start_at), None, None, None) => SweepState::Waiting { start_at },
                    (None, Some(until_height), Some(next_height), Some(started_at)) => {
                        SweepState::Running(SweepProgress {
                            until_height,
                            next_height,
                            started_at,
                        })
                    },
                    _ => bail!("inconsistent node_progress row for {}", self.node),
                };
                Ok(NodeProgress {
                    follow_next_height,
                    sweep,
                    last_sweep_completed_at: completed,
                })
            },
        )
        .transpose()
    }

    /// Upserts the full `node_progress` row.
    pub async fn save_progress(&self, progress: &NodeProgress) -> anyhow::Result<()> {
        let (waiting, until, next, started) = match &progress.sweep {
            SweepState::Waiting { start_at } => (Some(start_at.timestamp()), None, None, None),
            SweepState::Running(sweep) => (
                None,
                Some(i64::try_from(sweep.until_height)?),
                Some(i64::try_from(sweep.next_height)?),
                Some(sweep.started_at.timestamp()),
            ),
        };
        sqlx::query(
            "INSERT INTO node_progress (node, follow_next_height, next_sweep_at, \
             sweep_until_height, sweep_next_height, sweep_started_at, last_sweep_completed_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?) ON CONFLICT (node) DO UPDATE SET follow_next_height = \
             excluded.follow_next_height, next_sweep_at = excluded.next_sweep_at, \
             sweep_until_height = excluded.sweep_until_height, sweep_next_height = \
             excluded.sweep_next_height, sweep_started_at = excluded.sweep_started_at, \
             last_sweep_completed_at = excluded.last_sweep_completed_at",
        )
        .bind(&self.node)
        .bind(i64::try_from(progress.follow_next_height)?)
        .bind(waiting)
        .bind(until)
        .bind(next)
        .bind(started)
        .bind(progress.last_sweep_completed_at.map(|t| t.timestamp()))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Upsert: new row retries after `RETRY_DELAY_MIN_SECS`; known row keeps `first_missing_at`,
    /// sets `range_end`, doubles the delay up to `RETRY_DELAY_MAX_SECS`.
    pub async fn record_missing(&self, probe: &Probe, now: DateTime<Utc>) -> anyhow::Result<()> {
        // In `DO UPDATE`, unqualified columns are the old row, so both columns see the old delay.
        sqlx::query(
            "INSERT INTO missing_chunk (node, kind, range_start, range_end, first_missing_at, \
             retry_delay_secs, next_retry_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?5 + ?6) ON \
             CONFLICT (node, kind, range_start) DO UPDATE SET range_end = excluded.range_end, \
             retry_delay_secs = MIN(retry_delay_secs * 2, ?7), next_retry_at = ?5 + \
             MIN(retry_delay_secs * 2, ?7)",
        )
        .bind(&self.node)
        .bind(<&'static str>::from(probe.kind))
        .bind(i64::try_from(probe.range.start)?)
        .bind(i64::try_from(probe.range.end)?)
        .bind(now.timestamp())
        .bind(RETRY_DELAY_MIN_SECS)
        .bind(RETRY_DELAY_MAX_SECS)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Deletes the row after a 200.
    pub async fn record_found(&self, probe: &Probe) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM missing_chunk WHERE node = ? AND kind = ? AND range_start = ?")
            .bind(&self.node)
            .bind(<&'static str>::from(probe.kind))
            .bind(i64::try_from(probe.range.start)?)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Up to `limit` probes with `next_retry_at <= now`, oldest retry first.
    pub async fn due_retries(&self, now: DateTime<Utc>, limit: u32) -> anyhow::Result<Vec<Probe>> {
        let rows: Vec<(String, u64, u64)> = sqlx::query_as(
            "SELECT kind, range_start, range_end FROM missing_chunk WHERE node = ? AND \
             next_retry_at <= ? ORDER BY next_retry_at LIMIT ?",
        )
        .bind(&self.node)
        .bind(now.timestamp())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|(kind, start, end)| {
                Ok(Probe {
                    kind: DataKind::from_str(&kind)?,
                    range: start..end,
                })
            })
            .collect()
    }

    /// One entry per `DataKind::VARIANTS`: `SUM(end - start)`, `now - MIN(first_missing_at)`.
    pub async fn missing_summary(&self, now: DateTime<Utc>) -> anyhow::Result<Vec<KindSummary>> {
        let rows: Vec<(String, u64, DateTime<Utc>)> = sqlx::query_as(
            "SELECT kind, SUM(range_end - range_start), MIN(first_missing_at) FROM missing_chunk \
             WHERE node = ? GROUP BY kind",
        )
        .bind(&self.node)
        .fetch_all(&self.pool)
        .await?;
        let by_kind = rows
            .into_iter()
            .map(|(kind, count, first)| Ok((DataKind::from_str(&kind)?, (count, first))))
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(DataKind::VARIANTS
            .iter()
            .map(|&kind| {
                let found = by_kind.iter().find(|(k, _)| *k == kind).map(|(_, v)| v);
                KindSummary {
                    kind,
                    missing_count: found.map_or(0, |&(count, _)| count),
                    // A clock step back clamps the age to zero.
                    oldest_missing_age: found
                        .map(|&(_, first)| (now - first).to_std().unwrap_or(Duration::ZERO)),
                }
            })
            .collect())
    }
}

/// Persisted progress of one node watch.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeProgress {
    pub follow_next_height: u64,
    pub sweep: SweepState,
    pub last_sweep_completed_at: Option<DateTime<Utc>>,
}

/// Sweep schedule of one node.
#[derive(Clone, Debug, PartialEq)]
pub enum SweepState {
    Waiting { start_at: DateTime<Utc> },
    Running(SweepProgress),
}

/// A sweep in progress over `[0, until_height)`.
#[derive(Clone, Debug, PartialEq)]
pub struct SweepProgress {
    pub until_height: u64,
    pub next_height: u64,
    pub started_at: DateTime<Utc>,
}

/// State of one kind on one node.
#[derive(Clone, Debug, PartialEq)]
pub struct KindSummary {
    pub kind: DataKind,
    /// Heights, or epochs for a per-epoch kind.
    pub missing_count: u64,
    /// `None` when nothing is missing.
    pub oldest_missing_age: Option<Duration>,
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use pretty_assertions::assert_eq;

    use super::*;

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).unwrap()
    }

    fn probe(kind: DataKind, range: std::ops::Range<u64>) -> Probe {
        Probe { kind, range }
    }

    #[tokio::test]
    async fn missing_lifecycle() {
        let store = Store::connect(SqliteConnectOptions::from_str("sqlite::memory:").unwrap())
            .await
            .unwrap();
        let node = store.node("a:1");
        let t0 = 1_000;
        let leaf = probe(DataKind::Leaf, 0..100);
        let table = probe(DataKind::StakeTable, 5..6);

        let summary = node.missing_summary(at(t0)).await.unwrap();
        assert_eq!(summary.len(), DataKind::VARIANTS.len());
        assert!(
            summary
                .iter()
                .all(|k| k.missing_count == 0 && k.oldest_missing_age.is_none())
        );

        node.record_missing(&leaf, at(t0)).await.unwrap();
        node.record_missing(&table, at(t0 + 50)).await.unwrap();
        assert!(node.due_retries(at(t0 + 9), 10).await.unwrap().is_empty());
        assert_eq!(
            node.due_retries(at(t0 + 10), 10).await.unwrap(),
            std::slice::from_ref(&leaf)
        );

        // Delay doubles from 10 s: next retry at +20, +40, +80, ..., capped at 600.
        let mut now = t0 + 10;
        for delay in [20, 40, 80, 160, 320, 600, 600] {
            node.record_missing(&leaf, at(now)).await.unwrap();
            assert!(
                node.due_retries(at(now + delay - 1), 10)
                    .await
                    .unwrap()
                    .iter()
                    .all(|p| *p != leaf)
            );
            assert!(
                node.due_retries(at(now + delay), 10)
                    .await
                    .unwrap()
                    .contains(&leaf)
            );
            now += delay;
        }

        let summary = node.missing_summary(at(t0 + 100)).await.unwrap();
        let find = |kind| summary.iter().find(|k| k.kind == kind).unwrap().clone();
        assert_eq!(
            find(DataKind::Leaf),
            KindSummary {
                kind: DataKind::Leaf,
                missing_count: 100,
                oldest_missing_age: Some(TimeDelta::seconds(100).to_std().unwrap()),
            }
        );
        assert_eq!(find(DataKind::StakeTable).missing_count, 1);

        node.record_found(&leaf).await.unwrap();
        let summary = node.missing_summary(at(t0 + 100)).await.unwrap();
        assert_eq!(summary.iter().map(|k| k.missing_count).sum::<u64>(), 1);
        assert_eq!(
            store.node("b:2").missing_summary(at(t0)).await.unwrap()[0].missing_count,
            0
        );
    }
}
