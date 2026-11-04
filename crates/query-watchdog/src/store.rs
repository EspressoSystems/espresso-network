use std::{fs, ops::Range, path::Path, str::FromStr, time::Duration};

use anyhow::{Context, bail};
use chrono::{DateTime, Utc};
use sqlx::{
    QueryBuilder, Sqlite,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions},
};
use strum::VariantArray;

use crate::chunk::{DataKind, EpochConfig, Probe};

/// SQLite state of all node watches; WAL, so litestream can replicate it.
#[derive(Clone, Debug)]
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    /// `connect` on `path` (parent directory created) with WAL, `busy_timeout` 30 s,
    /// `create_if_missing`.
    pub async fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)
                .with_context(|| format!("creating directory {}", dir.display()))?;
        }
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
        delete_other_nodes("missing_chunk", node_labels)
            .build()
            .execute(&mut *tx)
            .await?;
        let forgotten = delete_other_nodes("node_progress", node_labels)
            .build()
            .execute(&mut *tx)
            .await?
            .rows_affected();
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

/// A `node_progress` row.
#[derive(sqlx::FromRow)]
struct ProgressRow {
    follow_next_height: u64,
    next_sweep_at: Option<DateTime<Utc>>,
    sweep_until_height: Option<u64>,
    sweep_next_height: Option<u64>,
    sweep_started_at: Option<DateTime<Utc>>,
    last_sweep_completed_at: Option<DateTime<Utc>>,
}

/// `DELETE` of the rows of `table` whose node is not in `labels`.
fn delete_other_nodes<'a>(table: &str, labels: &'a [String]) -> QueryBuilder<'a, Sqlite> {
    let mut query = QueryBuilder::new(format!("DELETE FROM {table} WHERE node NOT IN ("));
    let mut separated = query.separated(", ");
    for label in labels {
        separated.push_bind(label);
    }
    query.push(")");
    query
}

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
        row.map(|row| {
            let sweep = match (
                row.next_sweep_at,
                row.sweep_until_height,
                row.sweep_next_height,
                row.sweep_started_at,
            ) {
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
                follow_next_height: row.follow_next_height,
                sweep,
                last_sweep_completed_at: row.last_sweep_completed_at,
            })
        })
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
    /// raises `range_end`, doubles the delay up to `RETRY_DELAY_MAX_SECS`.
    pub async fn record_missing(&self, probe: &Probe, now: DateTime<Utc>) -> anyhow::Result<()> {
        // In `DO UPDATE`, unqualified columns are the old row, so both columns see the old delay.
        sqlx::query(
            "INSERT INTO missing_chunk (node, kind, range_start, range_end, first_missing_at, \
             retry_delay_secs, next_retry_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?5 + ?6) ON \
             CONFLICT (node, kind, range_start) DO UPDATE SET range_end = MAX(range_end, \
             excluded.range_end), retry_delay_secs = MIN(retry_delay_secs * 2, ?7), next_retry_at \
             = ?5 + MIN(retry_delay_secs * 2, ?7)",
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

    /// Deletes the row after a 200, unless it spans more than `probe.range`.
    pub async fn record_found(&self, probe: &Probe) -> anyhow::Result<()> {
        sqlx::query(
            "DELETE FROM missing_chunk WHERE node = ? AND kind = ? AND range_start = ? AND \
             range_end <= ?",
        )
        .bind(&self.node)
        .bind(<&'static str>::from(probe.kind))
        .bind(i64::try_from(probe.range.start)?)
        .bind(i64::try_from(probe.range.end)?)
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

    /// Deletes rows below `start_height` in one statement: per-height kinds with
    /// `range_start < start_height`, per-epoch kinds whose last block `epoch * epoch_height` is
    /// below it; returns the count.
    pub async fn forget_below(
        &self,
        start_height: u64,
        epochs: Option<EpochConfig>,
    ) -> anyhow::Result<u64> {
        let start_height = i64::try_from(start_height)?;
        let mut query = QueryBuilder::new("DELETE FROM missing_chunk WHERE node = ");
        query.push_bind(&self.node).push(" AND ((kind IN (");
        push_kinds(&mut query, false);
        query.push(") AND range_start < ").push_bind(start_height);
        if let Some(epochs) = epochs {
            query.push(") OR (kind IN (");
            push_kinds(&mut query, true);
            query
                .push(") AND range_start * ")
                .push_bind(i64::try_from(epochs.epoch_height)?)
                .push(" < ")
                .push_bind(start_height);
        }
        query.push("))");
        Ok(query.build().execute(&self.pool).await?.rows_affected())
    }

    /// Up to `limit` missing ranges, oldest `first_missing_at` first (then `range_start`, `kind`),
    /// with their ages at `now`, and the total number of missing ranges.
    pub async fn missing_ranges(
        &self,
        now: DateTime<Utc>,
        limit: u32,
    ) -> anyhow::Result<MissingRanges> {
        let rows: Vec<(String, u64, u64, DateTime<Utc>)> = sqlx::query_as(
            "SELECT kind, range_start, range_end, first_missing_at FROM missing_chunk WHERE node \
             = ? ORDER BY first_missing_at, range_start, kind LIMIT ?",
        )
        .bind(&self.node)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        let (total,): (u64,) = sqlx::query_as("SELECT COUNT(*) FROM missing_chunk WHERE node = ?")
            .bind(&self.node)
            .fetch_one(&self.pool)
            .await?;
        let shown = rows
            .into_iter()
            .map(|(kind, start, end, first)| {
                Ok(MissingRange {
                    probe: Probe {
                        kind: DataKind::from_str(&kind)?,
                        range: start..end,
                    },
                    age: self.age(now, first)?,
                })
            })
            .collect::<anyhow::Result<_>>()?;
        Ok(MissingRanges { shown, total })
    }

    /// `now - first`; errors when `first_missing_at` is in the future.
    fn age(&self, now: DateTime<Utc>, first: DateTime<Utc>) -> anyhow::Result<Duration> {
        (now - first)
            .to_std()
            .with_context(|| format!("{}: first_missing_at {first} is after {now}", self.node))
    }

    /// One entry per `DataKind::VARIANTS`: `SUM(end - start)`, and the range with the earliest
    /// `first_missing_at` (lowest `range_start` on a tie) with its age at `now`.
    pub async fn missing_summary(&self, now: DateTime<Utc>) -> anyhow::Result<Vec<KindSummary>> {
        let rows: Vec<(String, u64, DateTime<Utc>, u64, u64)> = sqlx::query_as(
            "SELECT kind, total, first_missing_at, range_start, range_end FROM ( SELECT kind, \
             first_missing_at, range_start, range_end, SUM(range_end - range_start) OVER \
             (PARTITION BY kind) AS total, ROW_NUMBER() OVER (PARTITION BY kind ORDER BY \
             first_missing_at, range_start) AS rn FROM missing_chunk WHERE node = ?) WHERE rn = 1",
        )
        .bind(&self.node)
        .fetch_all(&self.pool)
        .await?;
        let by_kind = rows
            .into_iter()
            .map(|(kind, count, first, start, end)| {
                let oldest = OldestMissing {
                    range: start..end,
                    age: self.age(now, first)?,
                };
                Ok((DataKind::from_str(&kind)?, count, oldest))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(DataKind::VARIANTS
            .iter()
            .map(|&kind| {
                let found = by_kind.iter().find(|(k, ..)| *k == kind);
                KindSummary {
                    kind,
                    missing_count: found.map_or(0, |(_, count, _)| *count),
                    oldest: found.map(|(.., oldest)| oldest.clone()),
                }
            })
            .collect())
    }
}

/// Pushes the quoted-bind list of the kinds with `is_per_epoch() == per_epoch`.
fn push_kinds(query: &mut QueryBuilder<'_, Sqlite>, per_epoch: bool) {
    let mut kinds = query.separated(", ");
    for kind in DataKind::VARIANTS
        .iter()
        .filter(|kind| kind.is_per_epoch() == per_epoch)
    {
        kinds.push_bind(<&'static str>::from(*kind));
    }
}

/// Persisted progress of one node watch.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeProgress {
    pub follow_next_height: u64,
    pub sweep: SweepState,
    pub last_sweep_completed_at: Option<DateTime<Utc>>,
}

impl NodeProgress {
    /// Raises the follow height and a running sweep's next height to `window_start`.
    pub fn clamped_to(self, window_start: u64) -> Self {
        let sweep = match self.sweep {
            SweepState::Running(sweep) => SweepState::Running(SweepProgress {
                next_height: sweep.next_height.max(window_start),
                ..sweep
            }),
            waiting @ SweepState::Waiting { .. } => waiting,
        };
        Self {
            follow_next_height: self.follow_next_height.max(window_start),
            sweep,
            ..self
        }
    }
}

/// Sweep schedule of one node.
#[derive(Clone, Debug, PartialEq)]
pub enum SweepState {
    Waiting { start_at: DateTime<Utc> },
    Running(SweepProgress),
}

/// A sweep in progress over `[next_height, until_height)`.
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
    pub oldest: Option<OldestMissing>,
}

/// A missing range and how long it has been missing.
#[derive(Clone, Debug, PartialEq)]
pub struct MissingRange {
    pub probe: Probe,
    pub age: Duration,
}

/// The oldest missing ranges of a node and the count of all of them.
#[derive(Clone, Debug, PartialEq)]
pub struct MissingRanges {
    pub shown: Vec<MissingRange>,
    pub total: u64,
}

/// The missing range with the earliest `first_missing_at`.
#[derive(Clone, Debug, PartialEq)]
pub struct OldestMissing {
    pub range: Range<u64>,
    pub age: Duration,
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    const T0: i64 = 1_000;

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).unwrap()
    }

    fn probe(kind: DataKind, range: Range<u64>) -> Probe {
        Probe { kind, range }
    }

    async fn node_store(label: &str) -> NodeStore {
        let options = SqliteConnectOptions::from_str("sqlite::memory:").unwrap();
        Store::connect(options).await.unwrap().node(label)
    }

    fn count(summary: &[KindSummary], kind: DataKind) -> u64 {
        summary
            .iter()
            .find(|k| k.kind == kind)
            .unwrap()
            .missing_count
    }

    #[tokio::test]
    async fn missing_backoff_and_summary() {
        let node = node_store("a:1").await;
        let leaf = probe(DataKind::Leaf, 0..100);

        let summary = node.missing_summary(at(T0)).await.unwrap();
        assert_eq!(summary.len(), DataKind::VARIANTS.len());
        assert!(
            summary
                .iter()
                .all(|k| k.missing_count == 0 && k.oldest.is_none())
        );

        node.record_missing(&leaf, at(T0)).await.unwrap();
        node.record_missing(&probe(DataKind::StakeTable, 5..6), at(T0 + 50))
            .await
            .unwrap();
        assert!(node.due_retries(at(T0 + 9), 10).await.unwrap().is_empty());
        let due = node.due_retries(at(T0 + 10), 10).await.unwrap();
        assert_eq!(due, std::slice::from_ref(&leaf));

        // The delay doubles from 10 s to the 600 s cap.
        let mut now = T0 + 10;
        for delay in [20, 40, 80, 160, 320, 600, 600] {
            node.record_missing(&leaf, at(now)).await.unwrap();
            let before = node.due_retries(at(now + delay - 1), 10).await.unwrap();
            assert!(!before.contains(&leaf));
            let after = node.due_retries(at(now + delay), 10).await.unwrap();
            assert!(after.contains(&leaf));
            now += delay;
        }

        let summary = node.missing_summary(at(T0 + 100)).await.unwrap();
        let leaf_summary = summary.iter().find(|k| k.kind == DataKind::Leaf).unwrap();
        assert_eq!(
            leaf_summary.oldest,
            Some(OldestMissing {
                range: 0..100,
                age: Duration::from_secs(100),
            })
        );
        assert_eq!(count(&summary, DataKind::Leaf), 100);
        assert_eq!(count(&summary, DataKind::StakeTable), 1);

        // A later row does not displace the oldest; a tie takes the lowest `range_start`.
        for (kind, range, secs) in [
            (DataKind::Leaf, 100..200, 20),
            (DataKind::Block, 300..400, 30),
            (DataKind::Block, 200..300, 30),
        ] {
            node.record_missing(&probe(kind, range), at(T0 + secs))
                .await
                .unwrap();
        }
        let summary = node.missing_summary(at(T0 + 100)).await.unwrap();
        let oldest = |kind| {
            summary
                .iter()
                .find(|k| k.kind == kind)
                .unwrap()
                .oldest
                .clone()
                .unwrap()
        };
        assert_eq!(count(&summary, DataKind::Leaf), 200);
        assert_eq!(oldest(DataKind::Leaf).range, 0..100);
        assert_eq!(count(&summary, DataKind::Block), 200);
        assert_eq!(oldest(DataKind::Block).range, 200..300);
        assert_eq!(oldest(DataKind::Block).age, Duration::from_secs(70));

        let listed = node.missing_ranges(at(T0 + 100), 3).await.unwrap();
        let order: Vec<_> = listed.shown.iter().map(|r| r.probe.clone()).collect();
        let expected = [
            probe(DataKind::Leaf, 0..100),
            probe(DataKind::Leaf, 100..200),
            probe(DataKind::Block, 200..300),
        ];
        assert_eq!(order, expected);
        assert_eq!(listed.total, 5);

        let other = node_store("b:2").await;
        let summary = other.missing_summary(at(T0)).await.unwrap();
        assert_eq!(summary.iter().map(|k| k.missing_count).sum::<u64>(), 0);
    }

    #[tokio::test]
    async fn record_found_keeps_longer_range() {
        let node = node_store("a:1").await;
        let long = probe(DataKind::Leaf, 100..200);
        let short = probe(DataKind::Leaf, 100..150);

        node.record_missing(&long, at(T0)).await.unwrap();
        node.record_missing(&short, at(T0 + 1)).await.unwrap();
        let summary = node.missing_summary(at(T0 + 2)).await.unwrap();
        assert_eq!(
            count(&summary, DataKind::Leaf),
            100,
            "range_end keeps the max"
        );

        node.record_found(&short).await.unwrap();
        let summary = node.missing_summary(at(T0 + 2)).await.unwrap();
        assert_eq!(
            count(&summary, DataKind::Leaf),
            100,
            "short probe cannot clear it"
        );

        node.record_found(&long).await.unwrap();
        let summary = node.missing_summary(at(T0 + 2)).await.unwrap();
        assert_eq!(count(&summary, DataKind::Leaf), 0);
    }

    #[tokio::test]
    async fn forget_below_window_start() {
        let node = node_store("a:1").await;
        let epochs = EpochConfig {
            epoch_height: 25,
            epoch_start_block: 1,
        };
        let rows = [
            (DataKind::Leaf, 0..100),
            (DataKind::Leaf, 50..150),
            (DataKind::Leaf, 100..200),
            (DataKind::StateCert, 3..4),
            (DataKind::StateCert, 4..5),
            (DataKind::StakeTable, 5..6),
        ];
        for (kind, range) in rows {
            node.record_missing(&probe(kind, range), at(T0))
                .await
                .unwrap();
        }

        // Window starts at 100: leaf rows starting below it go, including the one crossing it.
        // Epoch 3 ends at block 75 and goes; epoch 4 ends exactly at 100 and stays.
        assert_eq!(node.forget_below(100, Some(epochs)).await.unwrap(), 3);
        let summary = node.missing_summary(at(T0)).await.unwrap();
        assert_eq!(count(&summary, DataKind::Leaf), 100);
        assert_eq!(count(&summary, DataKind::StateCert), 1);
        assert_eq!(count(&summary, DataKind::StakeTable), 1);
    }

    #[test]
    fn progress_clamps_to_window_start() {
        let progress = NodeProgress {
            follow_next_height: 50,
            sweep: SweepState::Running(SweepProgress {
                until_height: 500,
                next_height: 80,
                started_at: at(T0),
            }),
            last_sweep_completed_at: None,
        };
        let clamped = progress.clone().clamped_to(100);
        assert_eq!(clamped.follow_next_height, 100);
        let SweepState::Running(sweep) = clamped.sweep else {
            panic!("sweep stays running");
        };
        assert_eq!(sweep.next_height, 100);
        assert_eq!(progress.clamped_to(0).follow_next_height, 50);
    }
}
