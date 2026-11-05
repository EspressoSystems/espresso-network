use std::{fs, num::NonZeroU32, ops::Range, path::Path, time::Duration};

use anyhow::{Context, bail, ensure};
use chrono::{DateTime, Utc};
use sqlx::{
    QueryBuilder, Sqlite,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions},
};

use crate::{
    kind::{DataKind, Epoch, Epochs, Height, KindClass},
    missing::{ImageDiff, ImageRegion, MissingRow, MissingSet, MissingUpdate, RowKey},
    outcome::NotServed,
    schedule::{CompletedSweep, NodeProgress, RunningSweep, SweepState},
    state::{Retry, RetryDelay, UnitState},
};

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

    /// Deletes all state in one transaction: missing rows and progress; returns the number of
    /// deleted rows.
    pub async fn reset(&self) -> anyhow::Result<u64> {
        let mut tx = self.pool.begin().await?;
        let mut deleted = 0;
        for table in ["missing_chunk", "node_progress"] {
            deleted += sqlx::query(&format!("DELETE FROM {table}"))
                .execute(&mut *tx)
                .await?
                .rows_affected();
        }
        tx.commit().await?;
        Ok(deleted)
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
        let row: Option<StoredProgress> = sqlx::query_as(
            "SELECT follow_next_height, last_sweep_started_at, last_sweep_completed_at, \
             sweep_start_height, sweep_next_height, sweep_until_height, sweep_done FROM \
             node_progress WHERE node = ?",
        )
        .bind(&self.node)
        .fetch_optional(&self.pool)
        .await?;
        row.map(NodeProgress::try_from)
            .transpose()
            .with_context(|| format!("{}: reading node_progress", self.node))
    }

    /// Upserts the `node_progress` row.
    pub async fn save_progress(&self, progress: &NodeProgress) -> anyhow::Result<()> {
        let row = StoredProgress::try_from(progress)?;
        sqlx::query(
            "INSERT INTO node_progress (node, follow_next_height, last_sweep_started_at, \
             last_sweep_completed_at, sweep_start_height, sweep_next_height, sweep_until_height, \
             sweep_done) VALUES (?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT (node) DO UPDATE SET \
             follow_next_height = excluded.follow_next_height, last_sweep_started_at = \
             excluded.last_sweep_started_at, last_sweep_completed_at = \
             excluded.last_sweep_completed_at, sweep_start_height = excluded.sweep_start_height, \
             sweep_next_height = excluded.sweep_next_height, sweep_until_height = \
             excluded.sweep_until_height, sweep_done = excluded.sweep_done",
        )
        .bind(&self.node)
        .bind(row.follow_next_height)
        .bind(row.last_sweep_started_at)
        .bind(row.last_sweep_completed_at)
        .bind(row.sweep_start_height)
        .bind(row.sweep_next_height)
        .bind(row.sweep_until_height)
        .bind(row.sweep_done)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

/// A missing set and its SQLite image. Every change goes through `apply`, which writes the image
/// before returning, so the image equals `set().rows()` between calls.
pub struct PersistedMissingSet {
    set: MissingSet,
    store: NodeStore,
}

impl PersistedMissingSet {
    /// Loads the node's image (`MissingSet::from_rows`).
    pub async fn load(store: NodeStore, epochs: Epochs) -> anyhow::Result<Self> {
        let stored: Vec<StoredRow> = sqlx::query_as(
            "SELECT kind, range_start, range_end, first_missing_at, retry_delay_secs, \
             next_retry_at, failed_probes, last_status FROM missing_chunk WHERE node = ? ORDER BY \
             kind, range_start",
        )
        .bind(&store.node)
        .fetch_all(&store.pool)
        .await?;
        let rows = stored
            .into_iter()
            .map(MissingRow::try_from)
            .collect::<anyhow::Result<Vec<_>>>()
            .with_context(|| format!("{}: reading missing_chunk", store.node))?;
        let set = MissingSet::from_rows(epochs, rows)
            .with_context(|| format!("{}: loading missing_chunk", store.node))?;
        Ok(Self { set, store })
    }

    /// `MissingSet::apply`, then one transaction: per region (as `StoredRegion`), delete the rows
    /// of its kind overlapping it and insert `set.rows_in(region)`. After an `Err` the memory
    /// state is ahead of the image: the value must be dropped, and the callers propagate the
    /// error so the process exits and a restart loads the last committed image.
    pub async fn apply(
        &mut self,
        update: MissingUpdate,
        now: DateTime<Utc>,
    ) -> anyhow::Result<ImageDiff> {
        let diff = self.set.apply(update, now);
        if diff.regions.is_empty() {
            return Ok(diff);
        }
        let mut tx = self.store.pool.begin().await?;
        for region in &diff.regions {
            let StoredRegion {
                kind,
                range_start,
                range_end,
            } = StoredRegion::try_from(region)?;
            sqlx::query(
                "DELETE FROM missing_chunk WHERE node = ? AND kind = ? AND range_start >= ? AND \
                 range_start < ?",
            )
            .bind(&self.store.node)
            .bind(kind)
            .bind(range_start)
            .bind(range_end)
            .execute(&mut *tx)
            .await?;
            for row in self.set.rows_in(region) {
                let row = StoredRow::try_from(&row)?;
                sqlx::query(
                    "INSERT INTO missing_chunk (node, kind, range_start, range_end, \
                     first_missing_at, retry_delay_secs, next_retry_at, failed_probes, \
                     last_status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&self.store.node)
                .bind(row.kind)
                .bind(row.range_start)
                .bind(row.range_end)
                .bind(row.first_missing_at)
                .bind(row.retry_delay_secs)
                .bind(row.next_retry_at)
                .bind(row.failed_probes)
                .bind(row.last_status)
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        Ok(diff)
    }

    /// Read access.
    pub fn set(&self) -> &MissingSet {
        &self.set
    }
}

/// Unix seconds as a time; errors outside chrono's range.
fn from_unix(secs: i64) -> anyhow::Result<DateTime<Utc>> {
    DateTime::from_timestamp(secs, 0).with_context(|| format!("timestamp {secs} out of range"))
}

/// Unit bounds as SQLite integers.
fn sql_bounds(units: Range<u64>) -> anyhow::Result<(i64, i64)> {
    Ok((i64::try_from(units.start)?, i64::try_from(units.end)?))
}

/// A `missing_chunk` row in SQLite types; the only place heights, delays and times convert to and
/// from `i64`.
#[derive(sqlx::FromRow)]
struct StoredRow {
    kind: DataKind,
    range_start: i64,
    range_end: i64,
    first_missing_at: i64,
    retry_delay_secs: i64,
    next_retry_at: i64,
    failed_probes: i64,
    last_status: String,
}

impl TryFrom<StoredRow> for MissingRow {
    type Error = anyhow::Error;

    /// Checked conversions, `KindClass` dispatch, `NotServed::from_str`, `RetryDelay::from_secs`.
    fn try_from(row: StoredRow) -> anyhow::Result<Self> {
        let start = u64::try_from(row.range_start)?;
        let end = u64::try_from(row.range_end)?;
        ensure!(start < end, "{:?} row {start}..{end} is empty", row.kind);
        let single_unit = || {
            ensure!(
                end == start + 1,
                "{:?} row {start}..{end} is not a single unit",
                row.kind
            );
            Ok(())
        };
        let key = match row.kind.class() {
            KindClass::Range(kind) => RowKey::Range {
                kind,
                heights: Height(start)..Height(end),
            },
            KindClass::Point(kind) => {
                single_unit()?;
                RowKey::Point {
                    kind,
                    height: Height(start),
                }
            },
            KindClass::Epoch(kind) => {
                single_unit()?;
                RowKey::Epoch {
                    kind,
                    epoch: Epoch(start),
                }
            },
        };
        let last_status = row.last_status.parse::<NotServed>()?;
        let probe_count = u32::try_from(row.failed_probes)
            .ok()
            .and_then(NonZeroU32::new)
            .with_context(|| format!("failed_probes {} is not positive", row.failed_probes))?;
        let retry = Retry {
            delay: RetryDelay::from_secs(u64::try_from(row.retry_delay_secs)?)?,
            next_at: from_unix(row.next_retry_at)?,
        };
        let state = UnitState::from_stored(
            last_status,
            from_unix(row.first_missing_at)?,
            probe_count,
            retry,
        );
        Ok(Self { key, state })
    }
}

impl TryFrom<&MissingRow> for StoredRow {
    type Error = anyhow::Error;

    /// Checked conversions to SQLite types.
    fn try_from(row: &MissingRow) -> anyhow::Result<Self> {
        let (start, end) = sql_bounds(row.key.units())?;
        Ok(Self {
            kind: row.key.kind(),
            range_start: start,
            range_end: end,
            first_missing_at: row.state.first_missing_at().timestamp(),
            retry_delay_secs: i64::try_from(row.state.retry().delay.duration().as_secs())?,
            next_retry_at: row.state.retry().next_at.timestamp(),
            failed_probes: i64::from(row.state.probe_count().get()),
            last_status: row.state.last_status().label().to_owned(),
        })
    }
}

/// An `ImageRegion` in SQLite types, the bounds of the image `DELETE`.
struct StoredRegion {
    kind: DataKind,
    range_start: i64,
    range_end: i64,
}

impl TryFrom<&ImageRegion> for StoredRegion {
    type Error = anyhow::Error;

    /// Checked conversions of the region bounds.
    fn try_from(region: &ImageRegion) -> anyhow::Result<Self> {
        let (kind, units) = region.units();
        let (start, end) = sql_bounds(units)?;
        Ok(Self {
            kind,
            range_start: start,
            range_end: end,
        })
    }
}

/// A `node_progress` row in SQLite types; the only progress conversion.
#[derive(sqlx::FromRow)]
struct StoredProgress {
    follow_next_height: i64,
    last_sweep_started_at: Option<i64>,
    last_sweep_completed_at: Option<i64>,
    sweep_start_height: Option<i64>,
    sweep_next_height: Option<i64>,
    sweep_until_height: Option<i64>,
    sweep_done: bool,
}

impl TryFrom<StoredProgress> for NodeProgress {
    type Error = anyhow::Error;

    /// Exhaustive match; an inconsistent row is an error.
    fn try_from(row: StoredProgress) -> anyhow::Result<Self> {
        let height = |value: i64| u64::try_from(value).map(Height);
        let started_at = row.last_sweep_started_at.map(from_unix).transpose()?;
        let completed_at = row.last_sweep_completed_at.map(from_unix).transpose()?;
        let sweep = match (
            row.sweep_start_height,
            row.sweep_next_height,
            row.sweep_until_height,
        ) {
            (Some(start), Some(next), Some(until)) => {
                ensure!(!row.sweep_done, "a running sweep is marked done");
                SweepState::Running {
                    sweep: RunningSweep {
                        started_at: started_at.context("a running sweep has no start time")?,
                        start_height: height(start)?,
                        next_height: height(next)?,
                        until_height: height(until)?,
                    },
                    last_completed_at: completed_at,
                }
            },
            (None, None, None) => match (started_at, completed_at, row.sweep_done) {
                (None, None, false) => SweepState::NeverSwept,
                (Some(started_at), Some(completed_at), false) => SweepState::Waiting {
                    last: CompletedSweep {
                        started_at,
                        completed_at,
                    },
                },
                (Some(started_at), Some(completed_at), true) => SweepState::Done {
                    last: CompletedSweep {
                        started_at,
                        completed_at,
                    },
                },
                other => bail!("inconsistent sweep times: {other:?}"),
            },
            other => bail!("inconsistent sweep heights: {other:?}"),
        };
        Ok(Self {
            follow_next_height: height(row.follow_next_height)?,
            sweep,
        })
    }
}

impl TryFrom<&NodeProgress> for StoredProgress {
    type Error = anyhow::Error;

    /// Checked conversions to SQLite types.
    fn try_from(progress: &NodeProgress) -> anyhow::Result<Self> {
        let height = |height: Height| i64::try_from(height.0);
        let mut row = Self {
            follow_next_height: height(progress.follow_next_height)?,
            last_sweep_started_at: None,
            last_sweep_completed_at: None,
            sweep_start_height: None,
            sweep_next_height: None,
            sweep_until_height: None,
            sweep_done: false,
        };
        match progress.sweep {
            SweepState::NeverSwept => {},
            SweepState::Waiting { last } | SweepState::Done { last } => {
                row.last_sweep_started_at = Some(last.started_at.timestamp());
                row.last_sweep_completed_at = Some(last.completed_at.timestamp());
                row.sweep_done = matches!(progress.sweep, SweepState::Done { .. });
            },
            SweepState::Running {
                sweep,
                last_completed_at,
            } => {
                row.last_sweep_started_at = Some(sweep.started_at.timestamp());
                row.last_sweep_completed_at = last_completed_at.map(|t| t.timestamp());
                row.sweep_start_height = Some(height(sweep.start_height)?);
                row.sweep_next_height = Some(height(sweep.next_height)?);
                row.sweep_until_height = Some(height(sweep.until_height)?);
            },
        }
        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use pretty_assertions::assert_eq;
    use strum::VariantArray;

    use super::*;
    use crate::{
        kind::{EpochConfig, EpochKind, PointKind, RangeKind},
        outcome::{Observation, ProbeResult, RangeFindings},
    };

    const T0: i64 = 1_000;
    const EPOCHS: Epochs = Epochs::On(EpochConfig {
        epoch_height: std::num::NonZeroU64::new(30).unwrap(),
        epoch_start_block: Height(61),
    });

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).unwrap()
    }

    async fn memory_store() -> Store {
        let options = SqliteConnectOptions::from_str("sqlite::memory:").unwrap();
        Store::connect(options).await.unwrap()
    }

    fn gap(kind: RangeKind, start: u64, end: u64) -> MissingUpdate {
        let heights = Height(start)..Height(end);
        MissingUpdate::Observed(ProbeResult::Range {
            kind,
            findings: RangeFindings::uniform(heights, Observation::NotFound),
        })
    }

    fn point(kind: PointKind, height: u64, observation: Observation) -> MissingUpdate {
        MissingUpdate::Observed(ProbeResult::Point {
            kind,
            height: Height(height),
            observation,
        })
    }

    fn epoch(kind: EpochKind, epoch: u64, observation: Observation) -> MissingUpdate {
        MissingUpdate::Observed(ProbeResult::Epoch {
            kind,
            epoch: Epoch(epoch),
            observation,
        })
    }

    async fn insert_raw(store: &Store, values: &str) -> Result<(), sqlx::Error> {
        sqlx::query(&format!(
            "INSERT INTO missing_chunk (node, kind, range_start, range_end, first_missing_at, \
             retry_delay_secs, next_retry_at, failed_probes, last_status) VALUES {values}"
        ))
        .execute(&store.pool)
        .await
        .map(drop)
    }

    #[tokio::test]
    async fn the_image_roundtrips_and_matches_the_kind_labels() {
        let store = memory_store().await;
        let node = store.node("a:1");
        let mut persisted = PersistedMissingSet::load(node.clone(), EPOCHS)
            .await
            .unwrap();
        let missing = Observation::NotFound;
        let updates = [
            gap(RangeKind::Leaf, 0, 100),
            gap(RangeKind::Block, 100, 200),
            gap(RangeKind::VidCommon, 200, 300),
            point(PointKind::LcLeafProof, 99, missing),
            epoch(EpochKind::StakeTable, 5, missing),
            epoch(EpochKind::StateCert, 3, missing),
            epoch(EpochKind::LcStakeTable, 7, missing),
            // Merges with its touching neighbour and rewrites both rows.
            gap(RangeKind::Leaf, 100, 200),
            gap(RangeKind::Leaf, 50, 150),
            point(PointKind::LcLeafProof, 99, Observation::Present),
            MissingUpdate::ForgetBelow(Height(120)),
        ];
        for (i, update) in updates.into_iter().enumerate() {
            persisted.apply(update, at(T0)).await.unwrap();
            let loaded = PersistedMissingSet::load(node.clone(), EPOCHS)
                .await
                .unwrap();
            let loaded: Vec<_> = loaded.set().rows().collect();
            let memory: Vec<_> = persisted.set().rows().collect();
            assert_eq!(loaded, memory, "after update {i}");
        }

        let kinds: Vec<String> = sqlx::query_scalar("SELECT DISTINCT kind FROM missing_chunk")
            .fetch_all(&store.pool)
            .await
            .unwrap();
        for kind in kinds {
            let known = DataKind::VARIANTS
                .iter()
                .any(|&k| <&str>::from(k) == kind.as_str());
            assert!(known, "{kind}");
        }
    }

    #[tokio::test]
    async fn the_sqlx_labels_equal_the_strum_labels() {
        let store = memory_store().await;
        for &kind in DataKind::VARIANTS {
            let label: String = sqlx::query_scalar("SELECT ?")
                .bind(kind)
                .fetch_one(&store.pool)
                .await
                .unwrap();
            assert_eq!(label, <&str>::from(kind));
        }
    }

    #[tokio::test]
    async fn reset_deletes_all_state() {
        let store = memory_store().await;
        let node = store.node("a:1");
        let progress = NodeProgress {
            follow_next_height: Height(700),
            sweep: SweepState::NeverSwept,
        };
        node.save_progress(&progress).await.unwrap();
        let mut persisted = PersistedMissingSet::load(node.clone(), Epochs::Off)
            .await
            .unwrap();
        persisted
            .apply(gap(RangeKind::Leaf, 0, 100), at(T0))
            .await
            .unwrap();
        assert_eq!(
            store.reset().await.unwrap(),
            2,
            "progress and one missing row"
        );
        assert_eq!(store.reset().await.unwrap(), 0, "nothing left to delete");
        assert_eq!(node.load_progress().await.unwrap(), None);
        let loaded = PersistedMissingSet::load(node, Epochs::Off).await.unwrap();
        assert_eq!(loaded.set().rows().count(), 0);
    }

    #[tokio::test]
    async fn every_sweep_state_roundtrips() {
        let store = memory_store().await;
        let node = store.node("a:1");
        let last = CompletedSweep {
            started_at: at(T0),
            completed_at: at(T0 + 30),
        };
        let running = RunningSweep {
            started_at: at(T0 + 100),
            start_height: Height(100),
            next_height: Height(300),
            until_height: Height(900),
        };
        let states = [
            SweepState::NeverSwept,
            SweepState::Waiting { last },
            SweepState::Running {
                sweep: running,
                last_completed_at: None,
            },
            SweepState::Running {
                sweep: running,
                last_completed_at: Some(at(T0 + 30)),
            },
            SweepState::Done { last },
        ];
        for sweep in states {
            let progress = NodeProgress {
                follow_next_height: Height(700),
                sweep,
            };
            node.save_progress(&progress).await.unwrap();
            assert_eq!(node.load_progress().await.unwrap(), Some(progress));
        }
    }

    #[tokio::test]
    async fn checks_reject_invalid_rows() {
        let store = memory_store().await;
        let row = |kind: &str, start, end, failed, status: &str| {
            format!("('a:1', '{kind}', {start}, {end}, {T0}, 5, {T0}, {failed}, '{status}')")
        };
        let bad = [
            row("leaf", 10, 10, 1, "404"),
            row("leaf", 10, 5, 1, "404"),
            row("lc_leaf_proof", 10, 12, 1, "404"),
            row("leaf", 0, 10, 0, "404"),
            row("oops", 0, 10, 1, "404"),
            row("leaf", 0, 10, 1, ""),
            row("leaf", -1, 10, 1, "404"),
            row("aggregate", 0, 10, 1, "404"),
        ];
        for values in bad {
            assert!(insert_raw(&store, &values).await.is_err(), "{values}");
        }
        insert_raw(&store, &row("leaf", 0, 10, 1, "404"))
            .await
            .unwrap();

        let progress = |values: &'static str| {
            let sql = format!(
                "INSERT INTO node_progress (node, follow_next_height, last_sweep_started_at, \
                 last_sweep_completed_at, sweep_start_height, sweep_next_height, \
                 sweep_until_height, sweep_done) VALUES {values}"
            );
            let pool = store.pool.clone();
            async move { sqlx::query(&sql).execute(&pool).await }
        };
        // A running sweep that is marked done.
        let running_done = "('b:1', 0, 5, NULL, 0, 0, 10, 1)";
        assert!(progress(running_done).await.is_err());
        // A running sweep without a start time.
        assert!(
            progress("('b:1', 0, NULL, NULL, 0, 0, 10, 0)")
                .await
                .is_err()
        );
        // Heights set only in part.
        assert!(
            progress("('b:1', 0, 5, NULL, 0, NULL, 10, 0)")
                .await
                .is_err()
        );
        // Done without a completion time.
        assert!(
            progress("('b:1', 0, 5, NULL, NULL, NULL, NULL, 1)")
                .await
                .is_err()
        );
        progress("('b:1', 0, 5, 6, NULL, NULL, NULL, 1)")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn loading_an_inconsistent_image_fails() {
        let row = |start, end, status: &str, at_secs| {
            format!("('a:1', 'leaf', {start}, {end}, {at_secs}, 5, {at_secs}, 1, '{status}')")
        };
        let cases = [
            (
                "overlapping",
                vec![row(0, 10, "404", T0), row(5, 15, "404", T0 + 1)],
            ),
            (
                "touching and equal",
                vec![row(0, 10, "404", T0), row(10, 20, "404", T0)],
            ),
            ("unknown status", vec![row(0, 10, "teapot", T0)]),
            ("legacy no-height status", vec![row(0, 10, "no-height", T0)]),
        ];
        for (name, rows) in cases {
            let store = memory_store().await;
            // The primary key (node, kind, range_start) lets overlapping rows through.
            insert_raw(&store, &rows.join(", ")).await.unwrap();
            let loaded = PersistedMissingSet::load(store.node("a:1"), Epochs::Off).await;
            assert!(loaded.is_err(), "{name}");
        }
    }

    #[tokio::test]
    async fn the_image_delete_seeks_the_primary_key() {
        let store = memory_store().await;
        let plan: Vec<(i64, i64, i64, String)> = sqlx::query_as(
            "EXPLAIN QUERY PLAN DELETE FROM missing_chunk WHERE node = ? AND kind = ? AND \
             range_start >= ? AND range_start < ?",
        )
        .bind("a:1")
        .bind(DataKind::Leaf)
        .bind(0)
        .bind(10)
        .fetch_all(&store.pool)
        .await
        .unwrap();
        let details: Vec<_> = plan.into_iter().map(|row| row.3).collect();
        assert_eq!(
            details,
            [
                "SEARCH missing_chunk USING INDEX sqlite_autoindex_missing_chunk_1 (node=? AND \
                 kind=? AND range_start>? AND range_start<?)"
            ]
        );
    }
}
