use std::{fs, ops::Range, path::Path, str::FromStr, time::Duration};

use anyhow::{Context, bail};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::{
    QueryBuilder, Sqlite,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions},
};
use strum::VariantArray;

use crate::{
    anchor::Anchor,
    chunk::{DataKind, EpochConfig, Probe},
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

    /// Read-only pool on an existing `path` for readers that must not queue behind the writer
    /// connection; WAL lets them run concurrently. Does not migrate.
    pub async fn open_read_only(path: &Path) -> anyhow::Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .read_only(true)
            .busy_timeout(Duration::from_secs(30));
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await
            .with_context(|| format!("opening {} read-only", path.display()))?;
        Ok(Self { pool })
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

    /// The stored anchor block, if any.
    pub async fn load_anchor(&self) -> anyhow::Result<Option<Anchor>> {
        let row: Option<(u64, String)> =
            sqlx::query_as("SELECT height, hash FROM anchor WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|(height, hash)| Anchor { height, hash }))
    }

    /// Replaces the stored anchor block.
    pub async fn save_anchor(&self, anchor: &Anchor) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO anchor (id, height, hash) VALUES (1, ?, ?) ON CONFLICT (id) DO UPDATE \
             SET height = excluded.height, hash = excluded.hash",
        )
        .bind(i64::try_from(anchor.height)?)
        .bind(&anchor.hash)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// The largest follow height of any node, `None` without progress.
    pub async fn max_follow_next_height(&self) -> anyhow::Result<Option<u64>> {
        let (max,): (Option<u64>,) =
            sqlx::query_as("SELECT MAX(follow_next_height) FROM node_progress")
                .fetch_one(&self.pool)
                .await?;
        Ok(max)
    }

    /// Deletes all state in one transaction: missing rows, progress and the anchor.
    pub async fn reset(&self) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        for table in ["missing_chunk", "node_progress", "anchor"] {
            sqlx::query(&format!("DELETE FROM {table}"))
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
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
const RETRY_DELAY_MIN_SECS: i64 = 5;
/// Cap of the doubling retry delay.
const RETRY_DELAY_MAX_SECS: i64 = 600;
/// Cap of the retry delay of a row that qualifies for alerting, so a refilled range is re-checked
/// within a minute.
const ALERT_RETRY_DELAY_MAX_SECS: i64 = 60;
/// Consecutive failed probes of a missing range before it is alerted.
pub const ALERT_MIN_FAILED_PROBES: u32 = 3;

/// `last_status` of a row refined from a 404 that named a height; the only status that alerts.
const STATUS_NOT_FOUND: &str = "404";

/// The delay cap of a row with `status` after `failed_probes` failed probes. `RETRY_CAP_SQL` is
/// the same rule for `upsert_failure`.
fn retry_delay_cap_secs(status: &str, failed_probes: i64) -> i64 {
    if status == STATUS_NOT_FOUND && failed_probes >= i64::from(ALERT_MIN_FAILED_PROBES) {
        ALERT_RETRY_DELAY_MAX_SECS
    } else {
        RETRY_DELAY_MAX_SECS
    }
}

/// `retry_delay_cap_secs` over an upsert's old row, with binds ?7 (`RETRY_DELAY_MAX_SECS`), ?8 (the
/// new status), ?9 (`STATUS_NOT_FOUND`), ?10 (`ALERT_MIN_FAILED_PROBES`) and ?11
/// (`ALERT_RETRY_DELAY_MAX_SECS`).
const RETRY_CAP_SQL: &str = "CASE WHEN ?8 = ?9 AND failed_probes + 1 >= ?10 THEN ?11 ELSE ?7 END";

/// Selects the rows of a kind that overlap a range.
const SELECT_OVERLAPPING: &str = "SELECT range_start, range_end, first_missing_at, \
                                  retry_delay_secs, failed_probes, next_retry_at, last_status \
                                  FROM missing_chunk WHERE node = ? AND kind = ? AND range_start \
                                  < ? AND range_end > ?";
/// Deletes the rows `SELECT_OVERLAPPING` selects.
const DELETE_OVERLAPPING: &str =
    "DELETE FROM missing_chunk WHERE node = ? AND kind = ? AND range_start < ? AND range_end > ?";
/// Deletes the row of a kind that starts where a range starts and does not end after it.
const DELETE_EXACT: &str =
    "DELETE FROM missing_chunk WHERE node = ? AND kind = ? AND range_start = ? AND range_end <= ?";

/// A stored missing range with its retry state.
#[derive(Clone, Debug, PartialEq, sqlx::FromRow)]
struct MissingRow {
    range_start: u64,
    range_end: u64,
    first_missing_at: DateTime<Utc>,
    retry_delay_secs: i64,
    failed_probes: i64,
    next_retry_at: DateTime<Utc>,
    last_status: String,
}

impl MissingRow {
    /// The range of the row.
    fn range(&self) -> Range<u64> {
        self.range_start..self.range_end
    }

    /// The same retry state over `range`.
    fn restricted_to(&self, range: Range<u64>) -> Self {
        Self {
            range_start: range.start,
            range_end: range.end,
            ..self.clone()
        }
    }
}

/// Rows for the `new` ranges with their status: each takes the oldest `first_missing_at`, the
/// doubled largest delay (capped) and the largest failed probe count plus one of the `old` rows it
/// overlaps, or starts fresh at `now` with the minimum delay and one failed probe.
fn merge_rows(
    old: &[MissingRow],
    new: &[(Range<u64>, &str)],
    now: DateTime<Utc>,
) -> Vec<MissingRow> {
    new.iter()
        .map(|(range, status)| {
            let overlapping: Vec<_> = old
                .iter()
                .filter(|row| row.range_start < range.end && row.range_end > range.start)
                .collect();
            let failed_probes = overlapping
                .iter()
                .map(|row| row.failed_probes)
                .max()
                .map_or(1, |failed| failed + 1);
            let retry_delay_secs = overlapping
                .iter()
                .map(|row| row.retry_delay_secs)
                .max()
                .map_or(RETRY_DELAY_MIN_SECS, |delay| {
                    (delay * 2).min(retry_delay_cap_secs(status, failed_probes))
                });
            MissingRow {
                range_start: range.start,
                range_end: range.end,
                first_missing_at: overlapping
                    .iter()
                    .map(|row| row.first_missing_at)
                    .min()
                    .unwrap_or(now),
                retry_delay_secs,
                failed_probes,
                next_retry_at: now + TimeDelta::seconds(retry_delay_secs),
                last_status: (*status).to_owned(),
            }
        })
        .collect()
}

/// Upserts a failed probe: a new row retries after `RETRY_DELAY_MIN_SECS`, a known row keeps
/// `first_missing_at`, doubles its delay and counts another failed probe.
async fn upsert_failure(
    conn: &mut sqlx::SqliteConnection,
    node: &str,
    probe: &Probe,
    status: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<()> {
    // In `DO UPDATE`, unqualified columns are the old row, so both columns see the old delay.
    sqlx::query(&format!(
        "INSERT INTO missing_chunk (node, kind, range_start, range_end, first_missing_at, \
         retry_delay_secs, next_retry_at, last_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?5 + ?6, \
         ?8) ON CONFLICT (node, kind, range_start) DO UPDATE SET range_end = MAX(range_end, \
         excluded.range_end), retry_delay_secs = MIN(retry_delay_secs * 2, {RETRY_CAP_SQL}), \
         next_retry_at = ?5 + MIN(retry_delay_secs * 2, {RETRY_CAP_SQL}), failed_probes = \
         failed_probes + 1, last_status = ?8"
    ))
    .bind(node)
    .bind(<&'static str>::from(probe.kind))
    .bind(i64::try_from(probe.range.start)?)
    .bind(i64::try_from(probe.range.end)?)
    .bind(now.timestamp())
    .bind(RETRY_DELAY_MIN_SECS)
    .bind(RETRY_DELAY_MAX_SECS)
    .bind(status)
    .bind(STATUS_NOT_FOUND)
    .bind(ALERT_MIN_FAILED_PROBES)
    .bind(ALERT_RETRY_DELAY_MAX_SECS)
    .execute(conn)
    .await?;
    Ok(())
}

/// The parts of `range` that no row of `rows` covers, in order.
fn uncovered_parts(range: &Range<u64>, rows: &[MissingRow]) -> Vec<Range<u64>> {
    let mut covering: Vec<_> = rows.iter().map(MissingRow::range).collect();
    covering.sort_by_key(|row| row.start);
    let mut parts = Vec::new();
    let mut cursor = range.start;
    for row in covering {
        if row.start > cursor {
            parts.push(cursor..row.start.min(range.end));
        }
        cursor = cursor.max(row.end);
    }
    if cursor < range.end {
        parts.push(cursor..range.end);
    }
    parts
}

/// The parts of `row` below and above `within`, with the retry state unchanged.
fn outside_parts(row: &MissingRow, within: &Range<u64>) -> Vec<MissingRow> {
    let below = (row.range_start < within.start).then_some(row.range_start..within.start);
    let above = (row.range_end > within.end).then_some(within.end..row.range_end);
    below
        .into_iter()
        .chain(above)
        .map(|range| row.restricted_to(range))
        .collect()
}

/// Writes `row`, replacing a row with the same key.
async fn insert_row(
    tx: &mut sqlx::SqliteConnection,
    node: &str,
    kind: &str,
    row: &MissingRow,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT OR REPLACE INTO missing_chunk (node, kind, range_start, range_end, \
         first_missing_at, retry_delay_secs, next_retry_at, failed_probes, last_status) VALUES \
         (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(node)
    .bind(kind)
    .bind(i64::try_from(row.range_start)?)
    .bind(i64::try_from(row.range_end)?)
    .bind(row.first_missing_at.timestamp())
    .bind(row.retry_delay_secs)
    .bind(row.next_retry_at.timestamp())
    .bind(row.failed_probes)
    .bind(&row.last_status)
    .execute(tx)
    .await?;
    Ok(())
}

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

    /// Upsert: new row retries after `RETRY_DELAY_MIN_SECS` with one failed probe; known row keeps
    /// `first_missing_at`, raises `range_end`, doubles the delay up to `RETRY_DELAY_MAX_SECS` (`ALERT_RETRY_DELAY_MAX_SECS` for a row that
    /// qualifies for alerting),
    /// counts another failed probe. `status` is the label of the failure (`"404"`, `"500"`,
    /// `"transport"`).
    pub async fn record_missing(
        &self,
        probe: &Probe,
        status: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let mut conn = self.pool.acquire().await?;
        upsert_failure(&mut conn, &self.node, probe, status, now).await
    }

    /// Records a failed probe of a range kind without breaking the disjoint rows: a row for exactly
    /// `probe.range` is upserted as `record_missing` does, otherwise rows that overlap stay as they
    /// are and the parts no row covers become new rows with `status`.
    pub async fn record_range_failure(
        &self,
        probe: &Probe,
        status: &str,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        let kind = <&'static str>::from(probe.kind);
        let (start, end) = (
            i64::try_from(probe.range.start)?,
            i64::try_from(probe.range.end)?,
        );
        let old: Vec<MissingRow> = sqlx::query_as(SELECT_OVERLAPPING)
            .bind(&self.node)
            .bind(kind)
            .bind(end)
            .bind(start)
            .fetch_all(&mut *tx)
            .await?;
        if old.iter().any(|row| row.range() == probe.range) {
            upsert_failure(&mut tx, &self.node, probe, status, now).await?;
        } else {
            let fresh: Vec<_> = uncovered_parts(&probe.range, &old)
                .into_iter()
                .map(|part| (part, status))
                .collect();
            for row in merge_rows(&[], &fresh, now) {
                insert_row(&mut tx, &self.node, kind, &row).await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    /// Clears what a 200 for `probe` says is present: for range kinds the heights of
    /// `probe.range` (rows are trimmed, the parts outside stay), for the others the row that starts
    /// where it starts and does not end after it.
    pub async fn record_found(&self, probe: &Probe, now: DateTime<Utc>) -> anyhow::Result<()> {
        if probe.kind.is_range() {
            return self
                .replace_missing(probe.kind, &probe.range, &[], None, now)
                .await;
        }
        sqlx::query(DELETE_EXACT)
            .bind(&self.node)
            .bind(<&'static str>::from(probe.kind))
            .bind(i64::try_from(probe.range.start)?)
            .bind(i64::try_from(probe.range.end)?)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Whether the kind has a row for exactly `probe.range` that was refined from a 404.
    pub async fn is_exact_gap(&self, probe: &Probe) -> anyhow::Result<bool> {
        let found: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM missing_chunk WHERE node = ? AND kind = ? AND range_start = ? AND \
             range_end = ? AND last_status = ?",
        )
        .bind(&self.node)
        .bind(<&'static str>::from(probe.kind))
        .bind(i64::try_from(probe.range.start)?)
        .bind(i64::try_from(probe.range.end)?)
        .bind(STATUS_NOT_FOUND)
        .fetch_optional(&self.pool)
        .await?;
        Ok(found.is_some())
    }

    /// Replaces what `kind` has missing inside `within` by the exact `gaps` (status `"404"`) and
    /// the `unrefined` tail with the status of the failure that stopped the search, in one
    /// transaction. A new row inherits the oldest `first_missing_at`, the doubled largest delay and
    /// the largest failed probe count plus one of the old rows it overlaps; one with no overlap
    /// starts fresh. The parts of old rows that stick out of `within` stay as they were.
    pub async fn replace_missing(
        &self,
        kind: DataKind,
        within: &Range<u64>,
        gaps: &[Range<u64>],
        unrefined: Option<(&Range<u64>, &str)>,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        let kind_label = <&'static str>::from(kind);
        let (start, end) = (i64::try_from(within.start)?, i64::try_from(within.end)?);
        let old: Vec<MissingRow> = sqlx::query_as(SELECT_OVERLAPPING)
            .bind(&self.node)
            .bind(kind_label)
            .bind(end)
            .bind(start)
            .fetch_all(&mut *tx)
            .await?;
        sqlx::query(DELETE_OVERLAPPING)
            .bind(&self.node)
            .bind(kind_label)
            .bind(end)
            .bind(start)
            .execute(&mut *tx)
            .await?;
        let new: Vec<_> = gaps
            .iter()
            .map(|gap| (gap.clone(), STATUS_NOT_FOUND))
            .chain(unrefined.map(|(rest, status)| (rest.clone(), status)))
            .collect();
        let outside = old.iter().flat_map(|row| outside_parts(row, within));
        let rows: Vec<_> = merge_rows(&old, &new, now)
            .into_iter()
            .chain(outside)
            .collect();
        for row in rows {
            insert_row(&mut tx, &self.node, kind_label, &row).await?;
        }
        tx.commit().await?;
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

    /// Forgets what is below `start_height`: per-height rows ending at or below it are deleted,
    /// range-kind rows crossing it are trimmed to start there, and per-epoch rows whose last block
    /// `epoch * epoch_height` is below it are deleted; returns the number of deleted rows.
    pub async fn forget_below(
        &self,
        start_height: u64,
        epochs: Option<EpochConfig>,
    ) -> anyhow::Result<u64> {
        let start_height = i64::try_from(start_height)?;
        let mut tx = self.pool.begin().await?;
        let mut delete = QueryBuilder::new("DELETE FROM missing_chunk WHERE node = ");
        delete.push_bind(&self.node).push(" AND ((kind IN (");
        push_kinds(&mut delete, |kind| !kind.is_per_epoch());
        delete.push(") AND range_end <= ").push_bind(start_height);
        if let Some(epochs) = epochs {
            delete.push(") OR (kind IN (");
            push_kinds(&mut delete, DataKind::is_per_epoch);
            delete
                .push(") AND range_start * ")
                .push_bind(i64::try_from(epochs.epoch_height)?)
                .push(" < ")
                .push_bind(start_height);
        }
        delete.push("))");
        let forgotten = delete.build().execute(&mut *tx).await?.rows_affected();
        // Rows are disjoint, so no other row starts at `start_height`.
        let mut trim = QueryBuilder::new("UPDATE missing_chunk SET range_start = ");
        trim.push_bind(start_height)
            .push(" WHERE node = ")
            .push_bind(&self.node)
            .push(" AND kind IN (");
        push_kinds(&mut trim, DataKind::is_range);
        trim.push(") AND range_start < ")
            .push_bind(start_height)
            .push(" AND range_end > ")
            .push_bind(start_height);
        trim.build().execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(forgotten)
    }

    /// Up to `limit` missing ranges first missing at or before `now`, oldest first (then
    /// `range_start`, `kind`), with their ages, and the total number of such ranges.
    pub async fn missing_ranges(
        &self,
        now: DateTime<Utc>,
        limit: u32,
    ) -> anyhow::Result<MissingRanges> {
        let rows: Vec<(String, u64, u64, DateTime<Utc>, u64)> = sqlx::query_as(
            "SELECT kind, range_start, range_end, first_missing_at, COUNT(*) OVER () FROM \
             missing_chunk WHERE node = ? AND first_missing_at <= ? ORDER BY first_missing_at, \
             range_start, kind LIMIT ?",
        )
        .bind(&self.node)
        .bind(now.timestamp())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        let total = rows.first().map_or(0, |row| row.4);
        let shown = rows
            .into_iter()
            .map(|(kind, start, end, first, _)| {
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

    /// One entry per `DataKind::VARIANTS`: the missing count (heights for range kinds, rows for the
    /// others), and the range with the earliest
    /// `first_missing_at` (lowest `range_start` on a tie) with its age at `now`; rows first missing
    /// after `now` are ignored.
    pub async fn missing_summary(&self, now: DateTime<Utc>) -> anyhow::Result<Vec<KindSummary>> {
        let mut query = QueryBuilder::new(
            "SELECT kind, total, first_missing_at, range_start, range_end FROM (SELECT kind, \
             first_missing_at, range_start, range_end, SUM(CASE WHEN kind IN (",
        );
        push_kinds(&mut query, DataKind::is_range);
        query
            .push(
                ") THEN range_end - range_start ELSE 1 END) OVER (PARTITION BY kind) AS total, \
                 ROW_NUMBER() OVER (PARTITION BY kind ORDER BY first_missing_at, range_start) AS \
                 rn FROM missing_chunk WHERE node = ",
            )
            .push_bind(&self.node)
            .push(" AND first_missing_at <= ")
            .push_bind(now.timestamp())
            .push(") WHERE rn = 1");
        let rows: Vec<(String, u64, DateTime<Utc>, u64, u64)> =
            query.build_query_as().fetch_all(&self.pool).await?;
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

impl NodeStore {
    /// The oldest qualifying row per kind: last failure a 404 that named a height (`"404"`), at
    /// least `min_failed` failed probes, first missing at least `min_age` before `now`, and below
    /// the tip margin: per-height kinds end at or below `max_end_height`, per-epoch kinds have
    /// their last block `epoch * epoch_height` at or below it. Ties go to the lowest `range_start`.
    pub async fn alert_candidates(
        &self,
        now: DateTime<Utc>,
        min_age: Duration,
        min_failed: u32,
        max_end_height: u64,
        epochs: Option<EpochConfig>,
    ) -> anyhow::Result<Vec<AlertCandidate>> {
        let max_first_missing = now - TimeDelta::from_std(min_age)?;
        let max_end_height = i64::try_from(max_end_height)?;
        let mut query = QueryBuilder::new(
            "SELECT kind, range_start, range_end, first_missing_at, failed_probes FROM (SELECT \
             kind, range_start, range_end, first_missing_at, failed_probes, ROW_NUMBER() OVER \
             (PARTITION BY kind ORDER BY first_missing_at, range_start) AS rn FROM missing_chunk \
             WHERE node = ",
        );
        query
            .push_bind(&self.node)
            .push(" AND last_status = ")
            .push_bind(STATUS_NOT_FOUND)
            .push(" AND failed_probes >= ")
            .push_bind(min_failed)
            .push(" AND first_missing_at <= ")
            .push_bind(max_first_missing.timestamp())
            .push(" AND ((kind IN (");
        push_kinds(&mut query, |kind| !kind.is_per_epoch());
        query.push(") AND range_end <= ").push_bind(max_end_height);
        if let Some(epochs) = epochs {
            query.push(") OR (kind IN (");
            push_kinds(&mut query, DataKind::is_per_epoch);
            query
                .push(") AND range_start * ")
                .push_bind(i64::try_from(epochs.epoch_height)?)
                .push(" <= ")
                .push_bind(max_end_height);
        }
        query.push("))) WHERE rn = 1");
        let rows: Vec<(String, u64, u64, DateTime<Utc>, u32)> =
            query.build_query_as().fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|(kind, start, end, first, failed_probes)| {
                Ok(AlertCandidate {
                    kind: DataKind::from_str(&kind)?,
                    range: start..end,
                    age: self.age(now, first)?,
                    failed_probes,
                })
            })
            .collect()
    }
}

/// Pushes the bind list of the kinds for which `filter` holds.
fn push_kinds(query: &mut QueryBuilder<'_, Sqlite>, filter: impl Fn(DataKind) -> bool) {
    let mut kinds = query.separated(", ");
    for kind in DataKind::VARIANTS.iter().filter(|kind| filter(**kind)) {
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

/// The oldest missing range of a kind that qualifies for an alert.
#[derive(Clone, Debug, PartialEq)]
pub struct AlertCandidate {
    pub kind: DataKind,
    pub range: Range<u64>,
    pub age: Duration,
    pub failed_probes: u32,
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

    async fn fail_probes(
        node: &NodeStore,
        kind: DataKind,
        range: Range<u64>,
        status: &str,
        times: i64,
        first: i64,
    ) {
        for i in 0..times {
            let probe = probe(kind, range.clone());
            node.record_missing(&probe, status, at(first + i))
                .await
                .unwrap();
        }
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

        node.record_missing(&leaf, "404", at(T0)).await.unwrap();
        node.record_missing(&probe(DataKind::StakeTable, 5..6), "404", at(T0 + 50))
            .await
            .unwrap();
        assert!(node.due_retries(at(T0 + 4), 10).await.unwrap().is_empty());
        let due = node.due_retries(at(T0 + 5), 10).await.unwrap();
        assert_eq!(due, std::slice::from_ref(&leaf));

        // The delay doubles from 5 s; a 404 row with 3 failed probes stops at 60 s.
        let mut now = T0 + 5;
        for delay in [10, 20, 40, 60, 60, 60] {
            node.record_missing(&leaf, "404", at(now)).await.unwrap();
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
            node.record_missing(&probe(kind, range), "404", at(T0 + secs))
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

        // A row first missing after `now` is ignored instead of failing the age.
        node.record_missing(&probe(DataKind::Leaf, 900..1000), "404", at(T0 + 500))
            .await
            .unwrap();
        let summary = node.missing_summary(at(T0 + 100)).await.unwrap();
        assert_eq!(count(&summary, DataKind::Leaf), 200);

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
    async fn retry_delay_stops_at_a_minute_only_for_alerting_rows() {
        let node = node_store("a:1").await;
        let cases = [
            (DataKind::Leaf, "404", vec![10, 20, 40, 60, 60]),
            (
                DataKind::Block,
                "500",
                vec![10, 20, 40, 80, 160, 320, 600, 600],
            ),
        ];
        for (kind, status, delays) in cases {
            let probe = probe(kind, 0..100);
            node.record_missing(&probe, status, at(T0)).await.unwrap();
            let mut now = T0 + 5;
            for delay in delays {
                node.record_missing(&probe, status, at(now)).await.unwrap();
                let before = node.due_retries(at(now + delay - 1), 10).await.unwrap();
                assert!(!before.contains(&probe), "{status} before {delay}");
                let after = node.due_retries(at(now + delay), 10).await.unwrap();
                assert!(after.contains(&probe), "{status} after {delay}");
                now += delay;
            }
        }
    }

    #[tokio::test]
    async fn record_found_trims_rows_to_the_heights_still_missing() {
        let node = node_store("a:1").await;
        let long = probe(DataKind::Leaf, 100..200);
        let short = probe(DataKind::Leaf, 100..150);

        node.record_missing(&long, "404", at(T0)).await.unwrap();
        node.record_missing(&short, "404", at(T0 + 1))
            .await
            .unwrap();
        let summary = node.missing_summary(at(T0 + 2)).await.unwrap();
        assert_eq!(
            count(&summary, DataKind::Leaf),
            100,
            "range_end keeps the max"
        );

        node.record_found(&short, at(T0 + 2)).await.unwrap();
        let summary = node.missing_summary(at(T0 + 2)).await.unwrap();
        assert_eq!(
            count(&summary, DataKind::Leaf),
            50,
            "the part outside the found range stays"
        );
        assert_eq!(summary[0].oldest.as_ref().unwrap().range, 150..200);

        node.record_found(&long, at(T0 + 2)).await.unwrap();
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
            (DataKind::Leaf, 0..60),
            (DataKind::Leaf, 80..140),
            (DataKind::Leaf, 150..200),
            (DataKind::LcLeafProof, 99..100),
            (DataKind::LcLeafProof, 150..151),
            (DataKind::StateCert, 3..4),
            (DataKind::StateCert, 4..5),
            (DataKind::StakeTable, 5..6),
        ];
        for (kind, range) in rows {
            node.record_missing(&probe(kind, range), "404", at(T0))
                .await
                .unwrap();
        }

        // Window starts at 100. Leaf 0..60 and lc_leaf_proof 99 end below it and go; leaf 80..140
        // crosses it and is trimmed to 100..140. Epoch 3 ends at block 75 and goes; epoch 4 ends
        // exactly at 100 and stays.
        assert_eq!(node.forget_below(100, Some(epochs)).await.unwrap(), 3);
        let summary = node.missing_summary(at(T0)).await.unwrap();
        assert_eq!(count(&summary, DataKind::Leaf), 90);
        assert_eq!(summary[0].oldest.as_ref().unwrap().range, 100..140);
        assert_eq!(count(&summary, DataKind::LcLeafProof), 1);
        assert_eq!(count(&summary, DataKind::StateCert), 1);
        assert_eq!(count(&summary, DataKind::StakeTable), 1);
    }

    #[tokio::test]
    async fn replace_missing_inherits_age_and_backoff() {
        let node = node_store("a:1").await;
        let chunk = 0..100;
        let leaf = probe(DataKind::Leaf, chunk.clone());
        node.record_missing(&leaf, "404", at(T0)).await.unwrap();
        node.record_missing(&leaf, "404", at(T0 + 5)).await.unwrap();

        let gaps = [10..20, 50..60];
        node.replace_missing(DataKind::Leaf, &chunk, &gaps, None, at(T0 + 30))
            .await
            .unwrap();
        let summary = node.missing_summary(at(T0 + 100)).await.unwrap();
        assert_eq!(count(&summary, DataKind::Leaf), 20);
        let oldest = summary[0].oldest.as_ref().unwrap();
        assert_eq!(
            (oldest.range.clone(), oldest.age),
            (10..20, Duration::from_secs(100))
        );
        // The old delay was 10 s, so the new rows retry 20 s after the replacement.
        assert!(node.due_retries(at(T0 + 49), 10).await.unwrap().is_empty());
        assert_eq!(node.due_retries(at(T0 + 50), 10).await.unwrap().len(), 2);

        let other = 100..200;
        let fresh = Range {
            start: 120,
            end: 130,
        };
        node.replace_missing(DataKind::Leaf, &other, &[fresh], None, at(T0 + 40))
            .await
            .unwrap();
        assert_eq!(node.due_retries(at(T0 + 45), 10).await.unwrap().len(), 1);

        node.record_found(&leaf, at(T0 + 100)).await.unwrap();
        let summary = node.missing_summary(at(T0 + 100)).await.unwrap();
        assert_eq!(
            count(&summary, DataKind::Leaf),
            10,
            "rows inside the found range go"
        );
    }

    #[tokio::test]
    async fn single_height_kinds_count_one_per_row() {
        let node = node_store("a:1").await;
        for kind in [DataKind::LcLeafProof, DataKind::Aggregate] {
            node.record_missing(&probe(kind, 0..100), "404", at(T0))
                .await
                .unwrap();
        }
        let summary = node.missing_summary(at(T0)).await.unwrap();
        assert_eq!(count(&summary, DataKind::LcLeafProof), 1);
        assert_eq!(count(&summary, DataKind::Aggregate), 1);
    }

    #[tokio::test]
    async fn alert_candidates_apply_every_condition() {
        let node = node_store("a:1").await;
        let epochs = Some(EpochConfig {
            epoch_height: 100,
            epoch_start_block: 1,
        });
        let fail = |kind, range, status, times, first| {
            fail_probes(&node, kind, range, status, times, first)
        };
        fail(DataKind::Leaf, 100..200, "404", 3, T0).await;
        fail(DataKind::Leaf, 50..100, "404", 3, T0 + 10).await;
        fail(DataKind::Block, 100..200, "500", 3, T0).await;
        fail(DataKind::VidCommon, 100..200, "404", 2, T0).await;
        fail(DataKind::LcLeafProof, 100..200, "404", 3, T0 + 50).await;
        fail(DataKind::Aggregate, 900..1000, "404", 3, T0).await;
        fail(DataKind::Aggregate, 400..500, "404", 3, T0).await;
        fail(DataKind::StateCert, 3..4, "404", 3, T0).await;
        fail(DataKind::StakeTable, 6..7, "404", 3, T0).await;
        fail(DataKind::StakeTable, 5..6, "404", 3, T0).await;

        let now = at(T0 + 60);
        let min_age = Duration::from_secs(60);
        let found = node
            .alert_candidates(now, min_age, 3, 500, epochs)
            .await
            .unwrap();
        let mut summary: Vec<_> = found
            .iter()
            .map(|c| (c.kind, c.range.clone(), c.age.as_secs(), c.failed_probes))
            .collect();
        summary.sort_by_key(|(kind, ..)| <&str>::from(*kind));
        // Not alerting: block (500), vid_common (2 probes), lc_leaf_proof (too young), the
        // aggregate row past the margin and the stake_table row past it. Leaf takes the older of
        // its two rows. Rows ending exactly at the margin (aggregate 400..500, the last block of
        // epoch 5 at 500) alert.
        let expected = [
            (DataKind::Aggregate, 400..500, 60, 3),
            (DataKind::Leaf, 100..200, 60, 3),
            (DataKind::StakeTable, 5..6, 60, 3),
            (DataKind::StateCert, 3..4, 60, 3),
        ];
        assert_eq!(summary, expected);

        // A later 404 sets the status again and counts another probe.
        node.record_missing(&probe(DataKind::Block, 100..200), "404", at(T0 + 3))
            .await
            .unwrap();
        let found = node
            .alert_candidates(now, min_age, 3, 500, epochs)
            .await
            .unwrap();
        let block = found.iter().find(|c| c.kind == DataKind::Block).unwrap();
        assert_eq!(block.failed_probes, 4);
    }

    #[tokio::test]
    async fn refinement_inherits_failed_probes_and_unrefined_never_alerts() {
        let node = node_store("a:1").await;
        let chunk = 0..100;
        for secs in [0, 5] {
            node.record_missing(&probe(DataKind::Leaf, chunk.clone()), "404", at(T0 + secs))
                .await
                .unwrap();
        }
        let unrefined = 50..100;
        let gap = Range { start: 10, end: 20 };
        let gaps = [gap];
        node.replace_missing(
            DataKind::Leaf,
            &chunk,
            &gaps,
            Some((&unrefined, "500")),
            at(T0 + 30),
        )
        .await
        .unwrap();
        let found = node
            .alert_candidates(at(T0 + 100), Duration::from_secs(60), 3, 1000, None)
            .await
            .unwrap();
        let ranges: Vec<_> = found
            .iter()
            .map(|c| (c.range.clone(), c.failed_probes))
            .collect();
        assert_eq!(
            ranges,
            [(10..20, 3)],
            "the unrefined tail has status unrefined"
        );
    }

    #[tokio::test]
    async fn record_found_keeps_the_part_of_a_row_that_crosses_its_edge() {
        let node = node_store("a:1").await;
        node.record_missing(&probe(DataKind::Block, 90..120), "404", at(T0))
            .await
            .unwrap();
        node.record_found(&probe(DataKind::Block, 100..200), at(T0 + 10))
            .await
            .unwrap();
        let listed = node.missing_ranges(at(T0 + 10), 10).await.unwrap();
        let rows: Vec<_> = listed
            .shown
            .iter()
            .map(|r| (r.probe.range.clone(), r.age.as_secs()))
            .collect();
        assert_eq!(rows, [(90..100, 10)]);
    }

    #[tokio::test]
    async fn record_found_clears_exact_keys_of_single_height_kinds() {
        let node = node_store("a:1").await;
        for range in [100..200, 120..180] {
            node.record_missing(&probe(DataKind::LcLeafProof, range), "404", at(T0))
                .await
                .unwrap();
        }
        node.record_found(&probe(DataKind::LcLeafProof, 100..200), at(T0))
            .await
            .unwrap();
        let summary = node.missing_summary(at(T0)).await.unwrap();
        assert_eq!(
            count(&summary, DataKind::LcLeafProof),
            1,
            "only the row at the same start goes"
        );
    }

    #[tokio::test]
    async fn replace_missing_keeps_the_parts_outside_the_range() {
        let node = node_store("a:1").await;
        node.record_missing(&probe(DataKind::Leaf, 50..150), "404", at(T0))
            .await
            .unwrap();
        node.record_missing(&probe(DataKind::Leaf, 150..250), "404", at(T0 + 10))
            .await
            .unwrap();
        let fresh = Range {
            start: 120,
            end: 130,
        };
        node.replace_missing(DataKind::Leaf, &(100..200), &[fresh], None, at(T0 + 30))
            .await
            .unwrap();

        let listed = node.missing_ranges(at(T0 + 100), 10).await.unwrap();
        let rows: Vec<_> = listed
            .shown
            .iter()
            .map(|r| (r.probe.range.clone(), r.age.as_secs()))
            .collect();
        assert_eq!(rows, [(50..100, 100), (120..130, 100), (200..250, 90)]);
        let summary = node.missing_summary(at(T0 + 100)).await.unwrap();
        assert_eq!(
            count(&summary, DataKind::Leaf),
            110,
            "no height counted twice"
        );
    }

    #[tokio::test]
    async fn a_failed_chunk_probe_leaves_refined_rows_alone() {
        let node = node_store("a:1").await;
        let leaf = |range| probe(DataKind::Leaf, range);
        let refined = Range {
            start: 110,
            end: 115,
        };
        let rest = Range {
            start: 115,
            end: 130,
        };
        node.replace_missing(DataKind::Leaf, &(100..200), &[refined], None, at(T0))
            .await
            .unwrap();
        node.record_missing(&leaf(rest.clone()), "404", at(T0 + 5))
            .await
            .unwrap();

        node.record_range_failure(&leaf(100..200), "500", at(T0 + 30))
            .await
            .unwrap();
        let listed = node.missing_ranges(at(T0 + 100), 10).await.unwrap();
        let rows: Vec<_> = listed
            .shown
            .iter()
            .map(|r| (r.probe.range.clone(), r.age.as_secs()))
            .collect();
        // The refined rows keep their age; the heights no row covers become new rows.
        let expected = [
            (110..115, 100),
            (115..130, 95),
            (100..110, 70),
            (130..200, 70),
        ];
        assert_eq!(rows, expected);
        let summary = node.missing_summary(at(T0 + 100)).await.unwrap();
        assert_eq!(
            count(&summary, DataKind::Leaf),
            100,
            "no height counted twice"
        );

        let candidates = node
            .alert_candidates(at(T0 + 100), Duration::from_secs(60), 1, 1000, None)
            .await
            .unwrap();
        assert_eq!(
            candidates[0].range,
            110..115,
            "the 404 rows still alert, the 500 rows do not"
        );

        // The same failure again backs the exact row off instead of adding rows.
        node.record_range_failure(&leaf(110..115), "500", at(T0 + 40))
            .await
            .unwrap();
        let listed = node.missing_ranges(at(T0 + 100), 10).await.unwrap();
        assert_eq!(listed.total, 4);
    }

    fn one(range: Range<u64>) -> Vec<Range<u64>> {
        vec![range]
    }

    #[tokio::test]
    async fn anchor_roundtrip_and_reset() {
        let node = node_store("a:1").await;
        let store = Store {
            pool: node.pool.clone(),
        };
        assert_eq!(store.load_anchor().await.unwrap(), None);
        assert_eq!(store.max_follow_next_height().await.unwrap(), None);

        let anchor = Anchor {
            height: 500,
            hash: "abc".to_owned(),
        };
        store.save_anchor(&anchor).await.unwrap();
        let newer = Anchor {
            height: 600,
            hash: "def".to_owned(),
        };
        store.save_anchor(&newer).await.unwrap();
        assert_eq!(store.load_anchor().await.unwrap(), Some(newer));

        let progress = NodeProgress {
            follow_next_height: 700,
            sweep: SweepState::Waiting { start_at: at(T0) },
            last_sweep_completed_at: None,
        };
        node.save_progress(&progress).await.unwrap();
        node.record_missing(&probe(DataKind::Leaf, 0..100), "404", at(T0))
            .await
            .unwrap();
        assert_eq!(store.max_follow_next_height().await.unwrap(), Some(700));

        store.reset().await.unwrap();
        assert_eq!(store.load_anchor().await.unwrap(), None);
        assert_eq!(node.load_progress().await.unwrap(), None);
        let summary = node.missing_summary(at(T0)).await.unwrap();
        assert_eq!(summary.iter().map(|k| k.missing_count).sum::<u64>(), 0);
    }

    #[test]
    fn uncovered_parts_table() {
        let row = |start, end| MissingRow {
            range_start: start,
            range_end: end,
            first_missing_at: at(T0),
            retry_delay_secs: 5,
            failed_probes: 1,
            next_retry_at: at(T0),
            last_status: String::new(),
        };
        let cases = [
            ("no rows", 0..10, vec![], one(0..10)),
            ("row inside", 0..10, vec![row(3, 6)], vec![0..3, 6..10]),
            ("row crossing the start", 5..10, vec![row(0, 7)], one(7..10)),
            ("row crossing the end", 0..10, vec![row(8, 20)], one(0..8)),
            ("covered", 0..10, vec![row(0, 4), row(4, 10)], vec![]),
            (
                "two rows apart",
                0..10,
                vec![row(6, 8), row(1, 3)],
                vec![0..1, 3..6, 8..10],
            ),
        ];
        for (name, range, rows, expected) in cases {
            assert_eq!(uncovered_parts(&range, &rows), expected, "{name}");
        }
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
