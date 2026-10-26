use std::{num::NonZeroUsize, path::PathBuf, str::FromStr, time::Duration};

use clap::Parser;
use derivative::Derivative;
use derive_more::derive::From;
use espresso_types::{BlockMerkleTree, FeeMerkleTree, Ratio, parse_duration, parse_size};
use hotshot_query_service::{
    data_source::storage::pruning::PrunerCfg, merklized_state::MerklizedState,
};

/// A few days of heights at mainnet rates, and what binds whenever the light client's history is
/// shorter than that.
pub const DEFAULT_ARCHIVE_STATE_MIN_RETENTION: u64 = 500_000;

/// Options for Postgres-backed persistence.
#[derive(Parser, Clone, Derivative)]
#[derivative(Debug)]
pub struct PostgresOptions {
    /// Hostname for the remote Postgres database server.
    #[clap(long, env = "ESPRESSO_NODE_POSTGRES_HOST")]
    pub(crate) host: Option<String>,

    /// Port for the remote Postgres database server.
    #[clap(long, env = "ESPRESSO_NODE_POSTGRES_PORT")]
    pub(crate) port: Option<u16>,

    /// Name of database to connect to.
    #[clap(long, env = "ESPRESSO_NODE_POSTGRES_DATABASE")]
    pub(crate) database: Option<String>,

    /// Postgres user to connect as.
    #[clap(long, env = "ESPRESSO_NODE_POSTGRES_USER")]
    pub(crate) user: Option<String>,

    /// Password for Postgres user.
    #[clap(long, env = "ESPRESSO_NODE_POSTGRES_PASSWORD")]
    // Hide from debug output since may contain sensitive data.
    #[derivative(Debug = "ignore")]
    pub(crate) password: Option<String>,

    /// Use TLS for an encrypted connection to the database.
    #[clap(long, env = "ESPRESSO_NODE_POSTGRES_USE_TLS")]
    pub(crate) use_tls: bool,

    /// Disable `DEFERRABLE` on read transactions for the query service.
    ///
    /// When true (the default), read transactions on Postgres start with `SERIALIZABLE READ ONLY`
    /// (no `DEFERRABLE`), so they begin immediately rather than waiting for a safe serializable
    /// snapshot. This trades start-up latency for the chance of a serialization-error retry.
    /// Set to false to restore `DEFERRABLE`.
    #[clap(
        long,
        env = "ESPRESSO_NODE_POSTGRES_NO_DEFERRABLE",
        default_value_t = true
    )]
    pub(crate) no_deferrable: bool,
}

impl Default for PostgresOptions {
    fn default() -> Self {
        Self::parse_from(std::iter::empty::<String>())
    }
}

#[derive(Parser, Clone, Derivative, Default)]
#[derivative(Debug)]
pub struct SqliteOptions {
    /// Base directory for the SQLite database.
    /// The SQLite file will be created in the `sqlite` subdirectory with filename as `database`.
    ///
    /// Required with `ESPRESSO_NODE_EMBEDDED_DB`.
    #[clap(
        long,
        env = "ESPRESSO_NODE_STORAGE_PATH",
        value_parser = build_sqlite_path
    )]
    pub(crate) path: Option<PathBuf>,
}

/// The path of the SQLite database under the base directory `path`.
///
/// The `sqlite` directory is created when the database is opened, not here: the same directory
/// is also read by Postgres and file system nodes, which never open it.
pub fn build_sqlite_path(path: &str) -> anyhow::Result<PathBuf> {
    Ok(PathBuf::from_str(path)?.join("sqlite").join("database"))
}

/// A connection pool for whichever backend [`Options::embedded_db`] selects.
#[derive(Clone, Debug, From)]
pub(crate) enum SqlPool {
    Postgres(sqlx::PgPool),
    Sqlite(sqlx::SqlitePool),
}

impl TryFrom<SqlPool> for sqlx::PgPool {
    type Error = SqlPool;

    fn try_from(pool: SqlPool) -> Result<sqlx::PgPool, SqlPool> {
        match pool {
            SqlPool::Postgres(pool) => Ok(pool),
            SqlPool::Sqlite(_) => Err(pool),
        }
    }
}

impl TryFrom<SqlPool> for sqlx::SqlitePool {
    type Error = SqlPool;

    fn try_from(pool: SqlPool) -> Result<sqlx::SqlitePool, SqlPool> {
        match pool {
            SqlPool::Sqlite(pool) => Ok(pool),
            SqlPool::Postgres(_) => Err(pool),
        }
    }
}

/// [`Options`] wrapped for the backend [`Options::embedded_db`] selects.
pub enum SqlBackendOptions {
    Postgres(super::postgres::Options),
    Sqlite(super::sqlite::Options),
}

/// Options for database-backed persistence, supporting both Postgres and SQLite.
#[derive(Parser, Clone, Derivative)]
#[derivative(Debug)]
pub struct Options {
    /// Store data in an embedded SQLite database instead of Postgres.
    #[clap(skip = cfg!(feature = "embedded-db"))]
    pub embedded_db: bool,

    #[clap(flatten)]
    pub(crate) postgres_options: PostgresOptions,

    #[clap(flatten)]
    pub(crate) sqlite_options: SqliteOptions,

    /// Database URI for Postgres or SQLite.
    ///
    /// This is a shorthand for setting a number of other options all at once. The URI has the
    /// following format ([brackets] indicate optional segments):
    ///
    /// - **Postgres:** `postgres[ql]://[username[:password]@][host[:port],]/database[?parameter_list]`
    /// - **SQLite:** `sqlite://path/to/db.sqlite`
    ///
    /// Options set explicitly via other env vars or flags will take precedence, so you can use this
    /// URI to set a baseline and then use other parameters to override or add configuration. In
    /// addition, there are some parameters which cannot be set via the URI, such as TLS.
    // Hide from debug output since may contain sensitive data.
    #[derivative(Debug = "ignore")]
    pub(crate) uri: Option<String>,

    /// This will enable the pruner and set the default pruning parameters unless provided.
    /// Default parameters:
    /// - pruning_threshold: 3 TB
    /// - minimum_retention: 1 day
    /// - target_retention: 7 days
    /// - batch_size: 100
    /// - max_usage: 80%
    /// - interval: 1 hour
    ///
    /// A pruned node with no merklized state starts it from a recent epoch boundary. Its trees then
    /// lack block proofs below that boundary, and fee accounts that no block after it touched. They
    /// stay incomplete after a switch to `--archive`: complete trees need a replay from genesis,
    /// which runs only on a database with no merklized state.
    #[clap(long, env = "ESPRESSO_NODE_DATABASE_PRUNE")]
    pub(crate) prune: bool,

    /// Pruning parameters.
    #[clap(flatten)]
    pub(crate) pruning: PruningOptions,

    /// Pruning parameters for ephemeral consensus storage.
    #[clap(flatten)]
    pub(crate) consensus_pruning: ConsensusPruningOptions,

    /// Retry/backoff parameters for PostgreSQL serialization conflicts.
    #[clap(flatten)]
    pub(crate) serializable_retry: SerializableRetryOptions,

    /// Specifies the maximum number of concurrent fetch requests allowed from peers.
    #[clap(long, env = "ESPRESSO_NODE_FETCH_RATE_LIMIT")]
    pub(crate) fetch_rate_limit: Option<usize>,

    /// The minimum delay between active fetches in a stream.
    #[clap(long, env = "ESPRESSO_NODE_ACTIVE_FETCH_DELAY", value_parser = parse_duration)]
    pub(crate) active_fetch_delay: Option<Duration>,

    /// The minimum delay between loading chunks in a stream.
    #[clap(long, env = "ESPRESSO_NODE_CHUNK_FETCH_DELAY", value_parser = parse_duration)]
    pub(crate) chunk_fetch_delay: Option<Duration>,

    /// How many of a request's height ranges to serve at once.
    #[clap(long, env = "ESPRESSO_NODE_RANGES_CONCURRENCY")]
    pub(crate) ranges_concurrency: Option<NonZeroUsize>,

    /// The number of items to process in a single transaction when scanning the database for
    /// missing objects.
    #[clap(long, env = "ESPRESSO_NODE_SYNC_STATUS_CHUNK_SIZE")]
    pub(crate) sync_status_chunk_size: Option<usize>,

    /// Duration to cache sync status results for.
    #[clap(long, env = "ESPRESSO_NODE_SYNC_STATUS_TTL", value_parser = parse_duration)]
    pub(crate) sync_status_ttl: Option<Duration>,

    /// The number of items to process at a time when scanning for proactive fetching.
    #[clap(long, env = "ESPRESSO_NODE_PROACTIVE_SCAN_CHUNK_SIZE")]
    pub(crate) proactive_scan_chunk_size: Option<usize>,

    /// The time interval between proactive fetching scans.
    #[clap(long, env = "ESPRESSO_NODE_PROACTIVE_SCAN_INTERVAL", value_parser = parse_duration)]
    pub(crate) proactive_scan_interval: Option<Duration>,

    /// How long a proactive scan waits for one request of missing ranges before fetching its
    /// chunks one at a time instead.
    #[clap(long, env = "ESPRESSO_NODE_PROACTIVE_FETCH_TIMEOUT", value_parser = parse_duration)]
    pub(crate) proactive_fetch_timeout: Option<Duration>,

    /// Disable the proactive scanner task.
    #[clap(long, env = "ESPRESSO_NODE_DISABLE_PROACTIVE_FETCHING")]
    pub(crate) disable_proactive_fetching: bool,

    /// Disable pruning and reconstruct previously pruned data.
    ///
    /// While running without pruning is the default behavior, the default will not try to
    /// reconstruct data that was pruned in a previous run where pruning was enabled. This option
    /// instructs the service to run without pruning _and_ reconstruct all previously pruned data by
    /// fetching from peers.
    ///
    /// Historical merklized state older than the light client contract history is garbage collected,
    /// but never less than ESPRESSO_NODE_ARCHIVE_STATE_MIN_RETENTION heights behind the head. Use
    /// ESPRESSO_NODE_ARCHIVE_FULL_STATE to retain all of it.
    #[clap(long, env = "ESPRESSO_NODE_ARCHIVE", conflicts_with = "prune")]
    pub(crate) archive: bool,

    /// Block heights of merklized state an archive node retains regardless of the light client.
    ///
    /// Collection stops at whichever reaches further back, this floor or the light client
    /// contract's history. Set it to zero to follow the contract alone.
    #[clap(
        long,
        env = "ESPRESSO_NODE_ARCHIVE_STATE_MIN_RETENTION",
        default_value_t = DEFAULT_ARCHIVE_STATE_MIN_RETENTION
    )]
    pub(crate) archive_state_min_retention: u64,

    /// Retain all historical merklized state on an archive node.
    ///
    /// Disables the state garbage collector that archive nodes run by default.
    #[clap(long, env = "ESPRESSO_NODE_ARCHIVE_FULL_STATE")]
    pub(crate) archive_full_state: bool,

    /// Turns on leaf only data storage
    #[clap(
        long,
        env = "ESPRESSO_NODE_LIGHTWEIGHT",
        default_value_t = false,
        conflicts_with = "archive"
    )]
    pub(crate) lightweight: bool,

    /// The maximum idle time of a database connection.
    ///
    /// Any connection which has been open and unused longer than this duration will be
    /// automatically closed to reduce load on the server.
    #[clap(long, env = "ESPRESSO_NODE_DATABASE_IDLE_CONNECTION_TIMEOUT", value_parser = parse_duration, default_value = "10m")]
    pub(crate) idle_connection_timeout: Duration,

    /// The maximum lifetime of a database connection.
    ///
    /// Any connection which has been open longer than this duration will be automatically closed
    /// (and, if needed, replaced), even if it is otherwise healthy. It is good practice to refresh
    /// even healthy connections once in a while (e.g. daily) in case of resource leaks in the
    /// server implementation.
    #[clap(long, env = "ESPRESSO_NODE_DATABASE_CONNECTION_TIMEOUT", value_parser = parse_duration, default_value = "30m")]
    pub(crate) connection_timeout: Duration,

    #[clap(long, env = "ESPRESSO_NODE_DATABASE_SLOW_STATEMENT_THRESHOLD", value_parser = parse_duration, default_value = "1s")]
    pub(crate) slow_statement_threshold: Duration,

    /// The maximum time a single SQL statement is allowed to run before being canceled.
    ///
    /// This helps prevent queries from running indefinitely and consuming resources.
    /// Set to 10 minutes by default
    #[clap(long, env = "ESPRESSO_NODE_DATABASE_STATEMENT_TIMEOUT", value_parser = parse_duration, default_value = "10m")]
    pub(crate) statement_timeout: Duration,

    /// The minimum number of database connections to maintain at any time.
    ///
    /// The database client will, to the best of its ability, maintain at least `min` open
    /// connections at all times. This can be used to reduce the latency hit of opening new
    /// connections when at least this many simultaneous connections are frequently needed.
    #[clap(
        long,
        env = "ESPRESSO_NODE_DATABASE_MIN_CONNECTIONS",
        default_value = "0"
    )]
    pub(crate) min_connections: u32,

    /// Allows setting a different maximum number of connections for query operations.
    /// Default value of None implies using the min_connections value. Postgres only.
    #[clap(long, env = "ESPRESSO_NODE_DATABASE_QUERY_MIN_CONNECTIONS", default_value = None)]
    pub(crate) query_min_connections: Option<u32>,

    /// The maximum number of database connections to maintain at any time.
    ///
    /// Once `max` connections are in use simultaneously, further attempts to acquire a connection
    /// (or begin a transaction) will block until one of the existing connections is released.
    #[clap(
        long,
        env = "ESPRESSO_NODE_DATABASE_MAX_CONNECTIONS",
        default_value = "25"
    )]
    pub(crate) max_connections: u32,

    /// Allows setting a different maximum number of connections for query operations.
    /// Default value of None implies using the max_connections value. Postgres only.
    #[clap(long, env = "ESPRESSO_NODE_DATABASE_QUERY_MAX_CONNECTIONS", default_value = None)]
    pub(crate) query_max_connections: Option<u32>,

    // Keep the database connection pool when persistence is created,
    // allowing it to be reused across multiple instances instead of creating
    // a new pool each time such as for API, consensus storage etc
    // This also ensures all storage instances adhere to the MAX_CONNECTIONS limit if set
    //
    // Note: Cloning the `Pool` is lightweight and efficient because it simply
    // creates a new reference-counted handle to the underlying pool state.
    #[clap(skip)]
    pub(crate) pool: Option<SqlPool>,

    #[clap(skip)]
    pub(crate) consensus_only: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self::parse_from(std::iter::empty::<String>())
    }
}

impl Options {
    /// These options for the backend [`embedded_db`](Self::embedded_db) selects.
    pub fn into_backend(self) -> SqlBackendOptions {
        if self.embedded_db {
            SqlBackendOptions::Sqlite(super::sqlite::Options(self))
        } else {
            SqlBackendOptions::Postgres(super::postgres::Options(self))
        }
    }
}

impl From<PostgresOptions> for Options {
    fn from(opt: PostgresOptions) -> Self {
        Options {
            embedded_db: false,
            postgres_options: opt,
            max_connections: 20,
            idle_connection_timeout: Duration::from_secs(120),
            connection_timeout: Duration::from_secs(10240),
            slow_statement_threshold: Duration::from_secs(1),
            statement_timeout: Duration::from_secs(600),
            ..Default::default()
        }
    }
}

impl From<SqliteOptions> for Options {
    fn from(opt: SqliteOptions) -> Self {
        Options {
            embedded_db: true,
            postgres_options: PostgresOptions::default(),
            sqlite_options: opt,
            max_connections: 5,
            idle_connection_timeout: Duration::from_secs(120),
            connection_timeout: Duration::from_secs(10240),
            slow_statement_threshold: Duration::from_secs(1),
            uri: None,
            statement_timeout: Duration::from_secs(600),
            prune: false,
            pruning: Default::default(),
            consensus_pruning: Default::default(),
            fetch_rate_limit: None,
            active_fetch_delay: None,
            chunk_fetch_delay: None,
            ranges_concurrency: None,
            sync_status_chunk_size: None,
            sync_status_ttl: None,
            proactive_scan_chunk_size: None,
            proactive_scan_interval: None,
            proactive_fetch_timeout: None,
            disable_proactive_fetching: false,
            archive: false,
            archive_state_min_retention: DEFAULT_ARCHIVE_STATE_MIN_RETENTION,
            archive_full_state: false,
            lightweight: false,
            min_connections: 0,
            query_min_connections: None,
            query_max_connections: None,
            pool: None,
            consensus_only: false,
            serializable_retry: SerializableRetryOptions::default(),
        }
    }
}

/// Pruning parameters.
#[derive(Parser, Clone, Copy, Debug)]
pub struct PruningOptions {
    /// Threshold for pruning, specified in bytes.
    /// If the disk usage surpasses this threshold, pruning is initiated for data older than the specified minimum retention period.
    /// Pruning continues until the disk usage drops below the MAX USAGE.
    #[clap(long, env = "ESPRESSO_NODE_PRUNER_PRUNING_THRESHOLD", value_parser = parse_size)]
    pub(crate) pruning_threshold: Option<u64>,

    /// Minimum retention period.
    /// Data is retained for at least this duration, even if there's no free disk space.
    #[clap(
        long,
        env = "ESPRESSO_NODE_PRUNER_MINIMUM_RETENTION",
        value_parser = parse_duration,
    )]
    pub(crate) minimum_retention: Option<Duration>,

    /// Minimum retention period for Merklized state.
    /// State is retained for at least this duration, even if there's no free disk space.
    #[clap(
        long,
        env = "ESPRESSO_NODE_PRUNER_STATE_MINIMUM_RETENTION",
        value_parser = parse_duration,
    )]
    state_minimum_retention: Option<Duration>,

    /// Target retention period.
    /// Data older than this is pruned to free up space.
    #[clap(
        long,
        env = "ESPRESSO_NODE_PRUNER_TARGET_RETENTION",
        value_parser = parse_duration,
    )]
    pub(crate) target_retention: Option<Duration>,

    /// Target retention period for Merklized state.
    /// State older than this is pruned to free up space, but never at or past the newest
    /// merklized state, which the state writer resumes from.
    #[clap(
        long,
        env = "ESPRESSO_NODE_PRUNER_STATE_TARGET_RETENTION",
        value_parser = parse_duration,
    )]
    state_target_retention: Option<Duration>,

    /// Batch size for pruning.
    /// This is the number of blocks worth of data to delete in a single transaction. Heights that
    /// hold no data are skipped without counting, so a batch, and the `Pruned to height` log line,
    /// can advance by more than this many heights at once.
    #[clap(long, env = "ESPRESSO_NODE_PRUNER_BATCH_SIZE")]
    pub(crate) batch_size: Option<u64>,

    /// Maximum disk usage (in basis points).
    ///
    /// Pruning stops once the disk usage falls below this value, even if
    /// some data older than the `MINIMUM_RETENTION` remains. Values range
    /// from 0 (0%) to 10000 (100%).
    #[clap(long, env = "ESPRESSO_NODE_PRUNER_MAX_USAGE")]
    pub(crate) max_usage: Option<u16>,

    /// Interval for running the pruner.
    #[clap(
        long,
        env = "ESPRESSO_NODE_PRUNER_INTERVAL",
        value_parser = parse_duration,
    )]
    pub(crate) interval: Option<Duration>,

    /// Number of SQLite pages to vacuum from the freelist
    /// during each pruner cycle.
    /// This value corresponds to `N` in the SQLite PRAGMA `incremental_vacuum(N)`,
    #[clap(long, env = "ESPRESSO_NODE_PRUNER_INCREMENTAL_VACUUM_PAGES")]
    pub(crate) pages: Option<u64>,
}

impl Default for PruningOptions {
    fn default() -> Self {
        Self::parse_from(std::iter::empty::<String>())
    }
}

impl From<PruningOptions> for PrunerCfg {
    fn from(opt: PruningOptions) -> Self {
        let mut cfg = PrunerCfg::new();
        if let Some(threshold) = opt.pruning_threshold {
            cfg = cfg.with_pruning_threshold(threshold);
        }
        if let Some(min) = opt.minimum_retention {
            cfg = cfg.with_minimum_retention(min);
        }
        if let Some(min) = opt.state_minimum_retention {
            cfg = cfg.with_state_minimum_retention(min);
        }
        if let Some(target) = opt.target_retention {
            cfg = cfg.with_target_retention(target);
        }
        if let Some(target) = opt.state_target_retention {
            cfg = cfg.with_state_target_retention(target);
        }
        if let Some(batch) = opt.batch_size {
            cfg = cfg.with_batch_size(batch);
        }
        if let Some(max) = opt.max_usage {
            cfg = cfg.with_max_usage(max);
        }
        if let Some(interval) = opt.interval {
            cfg = cfg.with_interval(interval);
        }

        if let Some(pages) = opt.pages {
            cfg = cfg.with_incremental_vacuum_pages(pages)
        }

        cfg = cfg.with_state_tables(vec![
            BlockMerkleTree::state_type().to_string(),
            FeeMerkleTree::state_type().to_string(),
        ]);

        cfg
    }
}

/// Pruning parameters for ephemeral consensus storage.
#[derive(Parser, Clone, Copy, Debug)]
pub struct ConsensusPruningOptions {
    /// Number of views to try to retain in consensus storage before data that hasn't been archived
    /// is garbage collected.
    ///
    /// The longer this is, the more certain that all data will eventually be archived, even if
    /// there are temporary problems with archive storage or partially missing data. This can be set
    /// very large, as most data is garbage collected as soon as it is finalized by consensus. This
    /// setting only applies to views which never get decided (ie forks in consensus) and views for
    /// which this node is partially offline. These should be exceptionally rare.
    ///
    /// Note that in extreme scenarios, data may be garbage collected even before TARGET_RETENTION
    /// views, if consensus storage exceeds TARGET_USAGE. For a hard lower bound on how long
    /// consensus data will be retained, see MINIMUM_RETENTION.
    ///
    /// The default of 302000 views equates to approximately 1 week (604800 seconds) at an average
    /// view time of 2s.
    #[clap(
        name = "TARGET_RETENTION",
        long = "consensus-storage-target-retention",
        env = "ESPRESSO_NODE_CONSENSUS_STORAGE_TARGET_RETENTION",
        default_value = "302000"
    )]
    pub(crate) target_retention: u64,

    /// Minimum number of views to try to retain in consensus storage before data that hasn't been
    /// archived is garbage collected.
    ///
    /// This bound allows data to be retained even if consensus storage occupies more than
    /// TARGET_USAGE. This can be used to ensure sufficient time to move consensus data to archival
    /// storage as necessary, even under extreme circumstances where otherwise garbage collection
    /// would kick in based on TARGET_RETENTION.
    ///
    /// The default of 130000 views equates to approximately 3 days (259200 seconds) at an average
    /// view time of 2s.
    #[clap(
        name = "MINIMUM_RETENTION",
        long = "consensus-storage-minimum-retention",
        env = "ESPRESSO_NODE_CONSENSUS_STORAGE_MINIMUM_RETENTION",
        default_value = "130000"
    )]
    pub(crate) minimum_retention: u64,

    /// Amount (in bytes) of data to retain in consensus storage before garbage collecting more
    /// aggressively.
    ///
    /// See also TARGET_RETENTION and MINIMUM_RETENTION.
    #[clap(
        name = "TARGET_USAGE",
        long = "consensus-storage-target-usage",
        env = "ESPRESSO_NODE_CONSENSUS_STORAGE_TARGET_USAGE",
        default_value = "1000000000"
    )]
    pub(crate) target_usage: u64,

    /// How often to measure consensus storage against TARGET_USAGE.
    ///
    /// On SQLite a measurement scans every page of the consensus tables. Between measurements,
    /// each decide prunes only to TARGET_RETENTION.
    #[clap(
        name = "USAGE_CHECK_INTERVAL",
        long = "consensus-storage-usage-check-interval",
        env = "ESPRESSO_NODE_CONSENSUS_STORAGE_USAGE_CHECK_INTERVAL",
        default_value = "5m",
        value_parser = parse_duration
    )]
    pub(crate) usage_check_interval: Duration,
}

impl Default for ConsensusPruningOptions {
    fn default() -> Self {
        Self::parse_from(std::iter::empty::<String>())
    }
}

/// Configuration for retrying transactions aborted by PostgreSQL serialization conflicts (error
/// 40001).  Under SERIALIZABLE isolation these aborts are expected and safe to retry from scratch.
#[derive(Parser, Clone, Copy, Debug)]
pub struct SerializableRetryOptions {
    /// Maximum number of retries on PostgreSQL serialization conflicts.
    #[clap(
        long = "serializable-retry-max",
        env = "ESPRESSO_NODE_SERIALIZABLE_RETRY_MAX",
        default_value = "100"
    )]
    pub(super) retry_max: u32,

    /// Initial delay before retrying after a PostgreSQL serialization conflict.
    #[clap(
        long = "serializable-backoff-base",
        env = "ESPRESSO_NODE_SERIALIZABLE_BACKOFF_BASE",
        default_value = "10ms",
        value_parser = parse_duration
    )]
    pub(super) backoff_base: Duration,

    /// Maximum delay between retries after a PostgreSQL serialization conflict.
    #[clap(
        long = "serializable-backoff-max",
        env = "ESPRESSO_NODE_SERIALIZABLE_BACKOFF_MAX",
        default_value = "500ms",
        value_parser = parse_duration
    )]
    pub(super) backoff_max: Duration,

    /// Multiplier applied to the backoff delay between successive retries.
    #[clap(
        long = "serializable-backoff-factor",
        env = "ESPRESSO_NODE_SERIALIZABLE_BACKOFF_FACTOR",
        default_value = "2"
    )]
    pub(super) backoff_factor: u32,

    /// Backoff jitter as a ratio of the backoff delay, numerator:denominator.
    #[clap(
        long = "serializable-backoff-jitter",
        env = "ESPRESSO_NODE_SERIALIZABLE_BACKOFF_JITTER",
        default_value = "5:10"
    )]
    pub(super) backoff_jitter: Ratio,

    /// When set, enable verbose diagnostic logging on PostgreSQL serialization conflicts.  Each
    /// retried conflict spawns a background query against `pg_stat_activity` and `pg_locks` to
    /// log concurrent sessions and predicate locks. Off by default.
    #[clap(
        long = "serializable-pg-stat-diag",
        env = "ESPRESSO_NODE_SERIALIZABLE_PG_STAT_DIAG"
    )]
    pub(super) pg_stat_diag: bool,
}

impl Default for SerializableRetryOptions {
    fn default() -> Self {
        Self::parse_from(std::iter::empty::<String>())
    }
}

/// How far behind the newest persisted decide an out-of-order ("gap-fill") decide can still
/// arrive. Mirrors `DECIDE_BUFFER` in `hotshot-new-protocol`; a leaf missing further behind the
/// watermark than this will never be filled in by consensus.
pub(crate) const DECIDE_GAP_FILL_HORIZON: u64 = 20;

/// Whether the height gap directly below the leaf at `view` can still be filled by a late
/// decide: the missing view (at most `view - 1`) must be within [`DECIDE_GAP_FILL_HORIZON`] of
/// `watermark`, the newest persisted decide.
pub(crate) fn within_gap_fill_horizon(view: u64, watermark: u64) -> bool {
    view.saturating_sub(1) + DECIDE_GAP_FILL_HORIZON > watermark
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parsing_the_sqlite_path_creates_no_directory() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("store");
        Options::try_parse_from(["storage-sql", "--path", base.to_str().unwrap()]).unwrap();
        assert!(!base.exists());
    }
}
