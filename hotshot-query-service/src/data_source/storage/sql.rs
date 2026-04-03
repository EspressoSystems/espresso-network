// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

#![cfg(feature = "sql-data-source")]
use std::{cmp::min, fmt::Debug, str::FromStr, time::Duration};

use async_trait::async_trait;
use chrono::Utc;
#[cfg(not(feature = "embedded-db"))]
use futures::future::FutureExt;
use hotshot_types::{
    data::VidShare,
    traits::{metrics::Metrics, node_implementation::NodeType},
};
use itertools::Itertools;
use log::LevelFilter;
#[cfg(not(feature = "embedded-db"))]
use sqlx::postgres::{PgConnectOptions, PgSslMode};
#[cfg(feature = "embedded-db")]
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{
    ConnectOptions, Row,
    pool::{Pool, PoolOptions},
};

use crate::{
    Header, QueryError, QueryResult,
    availability::{QueryableHeader, QueryablePayload, VidCommonMetadata, VidCommonQueryData},
    data_source::{
        VersionedDataSource,
        storage::pruning::{PruneStorage, PrunerCfg, PrunerConfig},
        update::Transaction as _,
    },
    metrics::PrometheusMetrics,
    node::BlockId,
    status::HasMetrics,
};
pub extern crate sqlx;
pub use sqlx::{Database, Sqlite};

mod db;
mod migrate;
mod queries;
mod transaction;

pub use anyhow::Error;
pub use db::*;
pub use include_dir::include_dir;
pub use queries::QueryBuilder;
pub use refinery::Migration;
pub use transaction::*;

use self::{migrate::Migrator, transaction::PoolMetrics};
use super::{AvailabilityStorage, NodeStorage};
// This needs to be reexported so that we can reference it by absolute path relative to this crate
// in the expansion of `include_migrations`, even when `include_migrations` is invoked from another
// crate which doesn't have `include_dir` as a dependency.
pub use crate::include_migrations;

/// Embed migrations from the given directory into the current binary for PostgreSQL or SQLite.
///
/// The macro invocation `include_migrations!(path)` evaluates to an expression of type `impl
/// Iterator<Item = Migration>`. Each migration must be a text file which is an immediate child of
/// `path`, and there must be no non-migration files in `path`. The migration files must have names
/// of the form `V${version}__${name}.sql`, where `version` is a positive integer indicating how the
/// migration is to be ordered relative to other migrations, and `name` is a descriptive name for
/// the migration.
///
/// `path` should be an absolute path. It is possible to give a path relative to the root of the
/// invoking crate by using environment variable expansions and the `CARGO_MANIFEST_DIR` environment
/// variable.
///
/// As an example, this is the invocation used to load the default migrations from the
/// `hotshot-query-service` crate. The migrations are located in a directory called `migrations` at
/// - PostgreSQL migrations are in `/migrations/postgres`.
/// - SQLite migrations are in `/migrations/sqlite`.
///
/// ```
/// # use hotshot_query_service::data_source::sql::{include_migrations, Migration};
/// // For PostgreSQL
/// #[cfg(not(feature = "embedded-db"))]
///  let mut migrations: Vec<Migration> =
///     include_migrations!("$CARGO_MANIFEST_DIR/migrations/postgres").collect();
/// // For SQLite
/// #[cfg(feature = "embedded-db")]
/// let mut migrations: Vec<Migration> =
///     include_migrations!("$CARGO_MANIFEST_DIR/migrations/sqlite").collect();
///
///     migrations.sort();
///     assert_eq!(migrations[0].version(), 10);
///     assert_eq!(migrations[0].name(), "init_schema");
/// ```
///
/// Note that a similar macro is available from Refinery:
/// [embed_migrations](https://docs.rs/refinery/0.8.11/refinery/macro.embed_migrations.html). This
/// macro differs in that it evaluates to an iterator of [migrations](Migration), making it an
/// expression macro, while `embed_migrations` is a statement macro that defines a module which
/// provides access to the embedded migrations only indirectly via a
/// [`Runner`](https://docs.rs/refinery/0.8.11/refinery/struct.Runner.html). The direct access to
/// migrations provided by [`include_migrations`] makes this macro easier to use with
/// [`Config::migrations`], for combining custom migrations with [`default_migrations`].
#[macro_export]
macro_rules! include_migrations {
    ($dir:tt) => {
        $crate::data_source::storage::sql::include_dir!($dir)
            .files()
            .map(|file| {
                let path = file.path();
                let name = path
                    .file_name()
                    .and_then(std::ffi::OsStr::to_str)
                    .unwrap_or_else(|| {
                        panic!(
                            "migration file {} must have a non-empty UTF-8 name",
                            path.display()
                        )
                    });
                let sql = file
                    .contents_utf8()
                    .unwrap_or_else(|| panic!("migration file {name} must use UTF-8 encoding"));
                $crate::data_source::storage::sql::Migration::unapplied(name, sql)
                    .expect("invalid migration")
            })
    };
}

/// The migrations required to build the default schema for this version of [`SqlStorage`].
pub fn default_migrations() -> Vec<Migration> {
    #[cfg(not(feature = "embedded-db"))]
    let mut migrations =
        include_migrations!("$CARGO_MANIFEST_DIR/migrations/postgres").collect::<Vec<_>>();

    #[cfg(feature = "embedded-db")]
    let mut migrations =
        include_migrations!("$CARGO_MANIFEST_DIR/migrations/sqlite").collect::<Vec<_>>();

    // Check version uniqueness and sort by version.
    validate_migrations(&mut migrations).expect("default migrations are invalid");

    // Check that all migration versions are multiples of 100, so that custom migrations can be
    // inserted in between.
    for m in &migrations {
        if m.version() <= 30 {
            // An older version of this software used intervals of 10 instead of 100. This was
            // changed to allow more custom migrations between each default migration, but we must
            // still accept older migrations that followed the older rule.
            assert!(
                m.version() > 0 && m.version() % 10 == 0,
                "legacy default migration version {} is not a positive multiple of 10",
                m.version()
            );
        } else {
            assert!(
                m.version() % 100 == 0,
                "default migration version {} is not a multiple of 100",
                m.version()
            );
        }
    }

    migrations
}

/// Validate and preprocess a sequence of migrations.
///
/// * Ensure all migrations have distinct versions
/// * Ensure migrations are sorted by increasing version
fn validate_migrations(migrations: &mut [Migration]) -> Result<(), Error> {
    migrations.sort_by_key(|m| m.version());

    // Check version uniqueness.
    for (prev, next) in migrations.iter().zip(migrations.iter().skip(1)) {
        if next <= prev {
            return Err(Error::msg(format!(
                "migration versions are not strictly increasing ({prev}->{next})"
            )));
        }
    }

    Ok(())
}

/// Add custom migrations to a default migration sequence.
///
/// Migrations in `custom` replace migrations in `default` with the same version. Otherwise, the two
/// sequences `default` and `custom` are merged so that the resulting sequence is sorted by
/// ascending version number. Each of `default` and `custom` is assumed to be the output of
/// [`validate_migrations`]; that is, each is sorted by version and contains no duplicate versions.
fn add_custom_migrations(
    default: impl IntoIterator<Item = Migration>,
    custom: impl IntoIterator<Item = Migration>,
) -> impl Iterator<Item = Migration> {
    default
        .into_iter()
        // Merge sorted lists, joining pairs of equal version into `EitherOrBoth::Both`.
        .merge_join_by(custom, |l, r| l.version().cmp(&r.version()))
        // Prefer the custom migration for a given version when both default and custom versions
        // are present.
        .map(|pair| pair.reduce(|_, custom| custom))
}

#[derive(Clone)]
pub struct Config {
    #[cfg(feature = "embedded-db")]
    db_opt: SqliteConnectOptions,

    #[cfg(not(feature = "embedded-db"))]
    db_opt: PgConnectOptions,

    pool_opt: PoolOptions<Db>,

    /// Extra pool_opt to allow separately configuring the connection pool for query service
    #[cfg(not(feature = "embedded-db"))]
    pool_opt_query: PoolOptions<Db>,

    #[cfg(not(feature = "embedded-db"))]
    schema: String,
    reset: bool,
    migrations: Vec<Migration>,
    no_migrations: bool,
    pruner_cfg: Option<PrunerCfg>,
    archive: bool,
    pool: Option<Pool<Db>>,
}

#[cfg(not(feature = "embedded-db"))]
impl Default for Config {
    fn default() -> Self {
        PgConnectOptions::default()
            .username("postgres")
            .password("password")
            .host("localhost")
            .port(5432)
            .into()
    }
}

#[cfg(feature = "embedded-db")]
impl Default for Config {
    fn default() -> Self {
        SqliteConnectOptions::default()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(30))
            .auto_vacuum(sqlx::sqlite::SqliteAutoVacuum::Incremental)
            .create_if_missing(true)
            .into()
    }
}

#[cfg(feature = "embedded-db")]
impl From<SqliteConnectOptions> for Config {
    fn from(db_opt: SqliteConnectOptions) -> Self {
        Self {
            db_opt,
            pool_opt: PoolOptions::default(),
            reset: false,
            migrations: vec![],
            no_migrations: false,
            pruner_cfg: None,
            archive: false,
            pool: None,
        }
    }
}

#[cfg(not(feature = "embedded-db"))]
impl From<PgConnectOptions> for Config {
    fn from(db_opt: PgConnectOptions) -> Self {
        Self {
            db_opt,
            pool_opt: PoolOptions::default(),
            pool_opt_query: PoolOptions::default(),
            schema: "hotshot".into(),
            reset: false,
            migrations: vec![],
            no_migrations: false,
            pruner_cfg: None,
            archive: false,
            pool: None,
        }
    }
}

#[cfg(not(feature = "embedded-db"))]
impl FromStr for Config {
    type Err = <PgConnectOptions as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(PgConnectOptions::from_str(s)?.into())
    }
}

#[cfg(feature = "embedded-db")]
impl FromStr for Config {
    type Err = <SqliteConnectOptions as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(SqliteConnectOptions::from_str(s)?.into())
    }
}

#[cfg(feature = "embedded-db")]
impl Config {
    pub fn busy_timeout(mut self, timeout: Duration) -> Self {
        self.db_opt = self.db_opt.busy_timeout(timeout);
        self
    }

    pub fn db_path(mut self, path: std::path::PathBuf) -> Self {
        self.db_opt = self.db_opt.filename(path);
        self
    }
}

#[cfg(not(feature = "embedded-db"))]
impl Config {
    /// Set the hostname of the database server.
    ///
    /// The default is `localhost`.
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.db_opt = self.db_opt.host(&host.into());
        self
    }

    /// Set the port on which to connect to the database.
    ///
    /// The default is 5432, the default Postgres port.
    pub fn port(mut self, port: u16) -> Self {
        self.db_opt = self.db_opt.port(port);
        self
    }

    /// Set the DB user to connect as.
    pub fn user(mut self, user: &str) -> Self {
        self.db_opt = self.db_opt.username(user);
        self
    }

    /// Set a password for connecting to the database.
    pub fn password(mut self, password: &str) -> Self {
        self.db_opt = self.db_opt.password(password);
        self
    }

    /// Set the name of the database to connect to.
    pub fn database(mut self, database: &str) -> Self {
        self.db_opt = self.db_opt.database(database);
        self
    }

    /// Use TLS for an encrypted connection to the database.
    ///
    /// Note that an encrypted connection may be established even if this option is not set, as long
    /// as both the client and server support it. This option merely causes connection to fail if an
    /// encrypted stream cannot be established.
    pub fn tls(mut self) -> Self {
        self.db_opt = self.db_opt.ssl_mode(PgSslMode::Require);
        self
    }

    /// Set the name of the schema to use for queries.
    ///
    /// The default schema is named `hotshot` and is created via the default migrations.
    pub fn schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = schema.into();
        self
    }
}

impl Config {
    /// Sets the database connection pool
    /// This allows reusing an existing connection pool when building a new `SqlStorage` instance.
    pub fn pool(mut self, pool: Pool<Db>) -> Self {
        self.pool = Some(pool);
        self
    }

    /// Reset the schema on connection.
    ///
    /// When this [`Config`] is used to [`connect`](Self::connect) a
    /// [`SqlDataSource`](crate::data_source::SqlDataSource), if this option is set, the relevant
    /// [`schema`](Self::schema) will first be dropped and then recreated, yielding a completely
    /// fresh instance of the query service.
    ///
    /// This is a particularly useful capability for development and staging environments. Still, it
    /// must be used with extreme caution, as using this will irrevocably delete any data pertaining
    /// to the query service in the database.
    pub fn reset_schema(mut self) -> Self {
        self.reset = true;
        self
    }

    /// Add custom migrations to run when connecting to the database.
    pub fn migrations(mut self, migrations: impl IntoIterator<Item = Migration>) -> Self {
        self.migrations.extend(migrations);
        self
    }

    /// Skip all migrations when connecting to the database.
    pub fn no_migrations(mut self) -> Self {
        self.no_migrations = true;
        self
    }

    /// Enable pruning with a given configuration.
    ///
    /// If [`archive`](Self::archive) was previously specified, this will override it.
    pub fn pruner_cfg(mut self, cfg: PrunerCfg) -> Result<Self, Error> {
        cfg.validate()?;
        self.pruner_cfg = Some(cfg);
        self.archive = false;
        Ok(self)
    }

    /// Disable pruning and reconstruct previously pruned data.
    ///
    /// While running without pruning is the default behavior, the default will not try to
    /// reconstruct data that was pruned in a previous run where pruning was enabled. This option
    /// instructs the service to run without pruning _and_ reconstruct all previously pruned data by
    /// fetching from peers.
    ///
    /// If [`pruner_cfg`](Self::pruner_cfg) was previously specified, this will override it.
    pub fn archive(mut self) -> Self {
        self.pruner_cfg = None;
        self.archive = true;
        self
    }

    /// Set the maximum idle time of a connection.
    ///
    /// Any connection which has been open and unused longer than this duration will be
    /// automatically closed to reduce load on the server.
    pub fn idle_connection_timeout(mut self, timeout: Duration) -> Self {
        self.pool_opt = self.pool_opt.idle_timeout(Some(timeout));

        #[cfg(not(feature = "embedded-db"))]
        {
            self.pool_opt_query = self.pool_opt_query.idle_timeout(Some(timeout));
        }

        self
    }

    /// Set the maximum lifetime of a connection.
    ///
    /// Any connection which has been open longer than this duration will be automatically closed
    /// (and, if needed, replaced), even if it is otherwise healthy. It is good practice to refresh
    /// even healthy connections once in a while (e.g. daily) in case of resource leaks in the
    /// server implementation.
    pub fn connection_timeout(mut self, timeout: Duration) -> Self {
        self.pool_opt = self.pool_opt.max_lifetime(Some(timeout));

        #[cfg(not(feature = "embedded-db"))]
        {
            self.pool_opt = self.pool_opt.max_lifetime(Some(timeout));
        }

        self
    }

    /// Set the minimum number of connections to maintain at any time.
    ///
    /// The data source will, to the best of its ability, maintain at least `min` open connections
    /// at all times. This can be used to reduce the latency hit of opening new connections when at
    /// least this many simultaneous connections are frequently needed.
    pub fn min_connections(mut self, min: u32) -> Self {
        self.pool_opt = self.pool_opt.min_connections(min);
        self
    }

    #[cfg(not(feature = "embedded-db"))]
    pub fn query_min_connections(mut self, min: u32) -> Self {
        self.pool_opt_query = self.pool_opt_query.min_connections(min);
        self
    }

    /// Set the maximum number of connections to maintain at any time.
    ///
    /// Once `max` connections are in use simultaneously, further attempts to acquire a connection
    /// (or begin a transaction) will block until one of the existing connections is released.
    pub fn max_connections(mut self, max: u32) -> Self {
        self.pool_opt = self.pool_opt.max_connections(max);
        self
    }

    #[cfg(not(feature = "embedded-db"))]
    pub fn query_max_connections(mut self, max: u32) -> Self {
        self.pool_opt_query = self.pool_opt_query.max_connections(max);
        self
    }

    /// Log at WARN level any time a SQL statement takes longer than `threshold`.
    ///
    /// The default threshold is 1s.
    pub fn slow_statement_threshold(mut self, threshold: Duration) -> Self {
        self.db_opt = self
            .db_opt
            .log_slow_statements(LevelFilter::Warn, threshold);
        self
    }

    /// Set the maximum time a single SQL statement is allowed to run before being canceled.
    ///
    /// This helps prevent queries from running indefinitely even when the client is dropped
    #[cfg(not(feature = "embedded-db"))]
    pub fn statement_timeout(mut self, timeout: Duration) -> Self {
        // Format duration as milliseconds
        // PostgreSQL interprets values without units as milliseconds
        let timeout_ms = timeout.as_millis();
        self.db_opt = self
            .db_opt
            .options([("statement_timeout", timeout_ms.to_string())]);
        self
    }

    /// not supported for SQLite.
    #[cfg(feature = "embedded-db")]
    pub fn statement_timeout(self, _timeout: Duration) -> Self {
        self
    }
}

/// Storage for the APIs provided in this crate, backed by a remote PostgreSQL database.
#[derive(Clone, Debug)]
pub struct SqlStorage {
    pool: Pool<Db>,
    metrics: PrometheusMetrics,
    pool_metrics: PoolMetrics,
    pruner_cfg: Option<PrunerCfg>,
}

#[derive(Debug, Default)]
pub struct Pruner {
    pruned_height: Option<u64>,
    target_height: Option<u64>,
    minimum_retention_height: Option<u64>,
}

#[derive(PartialEq)]
pub enum StorageConnectionType {
    Sequencer,
    Query,
}

impl SqlStorage {
    pub fn pool(&self) -> Pool<Db> {
        self.pool.clone()
    }

    /// Connect to a remote database.
    #[allow(unused_variables)]
    pub async fn connect(
        mut config: Config,
        connection_type: StorageConnectionType,
    ) -> Result<Self, Error> {
        let metrics = PrometheusMetrics::default();
        let pool_metrics = PoolMetrics::new(&*metrics.subgroup("sql".into()));

        #[cfg(feature = "embedded-db")]
        let pool = config.pool_opt.clone();
        #[cfg(not(feature = "embedded-db"))]
        let pool = match connection_type {
            StorageConnectionType::Sequencer => config.pool_opt.clone(),
            StorageConnectionType::Query => config.pool_opt_query.clone(),
        };

        let pruner_cfg = config.pruner_cfg;

        // Only reuse the same pool if we're using sqlite
        if cfg!(feature = "embedded-db") || connection_type == StorageConnectionType::Sequencer {
            // re-use the same pool if present and return early
            if let Some(pool) = config.pool {
                return Ok(Self {
                    metrics,
                    pool_metrics,
                    pool,
                    pruner_cfg,
                });
            }
        } else if config.pool.is_some() {
            tracing::info!("not reusing existing pool for query connection");
        }

        #[cfg(not(feature = "embedded-db"))]
        let schema = config.schema.clone();
        #[cfg(not(feature = "embedded-db"))]
        let pool = pool.after_connect(move |conn, _| {
            let schema = config.schema.clone();
            async move {
                query(&format!("SET search_path TO {schema}"))
                    .execute(conn)
                    .await?;
                Ok(())
            }
            .boxed()
        });

        #[cfg(feature = "embedded-db")]
        if config.reset {
            std::fs::remove_file(config.db_opt.get_filename())?;
        }

        let pool = pool.connect_with(config.db_opt).await?;

        // Create or connect to the schema for this query service.
        let mut conn = pool.acquire().await?;

        // Disable statement timeout for migrations, as they can take a long time
        #[cfg(not(feature = "embedded-db"))]
        query("SET statement_timeout = 0")
            .execute(conn.as_mut())
            .await?;

        #[cfg(not(feature = "embedded-db"))]
        if config.reset {
            query(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE"))
                .execute(conn.as_mut())
                .await?;
        }

        #[cfg(not(feature = "embedded-db"))]
        query(&format!("CREATE SCHEMA IF NOT EXISTS {schema}"))
            .execute(conn.as_mut())
            .await?;

        // Get migrations and interleave with custom migrations, sorting by version number.
        validate_migrations(&mut config.migrations)?;
        let migrations =
            add_custom_migrations(default_migrations(), config.migrations).collect::<Vec<_>>();

        // Get a migration runner. Depending on the config, we can either use this to actually run
        // the migrations or just check if the database is up to date.
        let runner = refinery::Runner::new(&migrations).set_grouped(true);

        if config.no_migrations {
            // We've been asked not to run any migrations. Abort if the DB is not already up to
            // date.
            let last_applied = runner
                .get_last_applied_migration_async(&mut Migrator::from(&mut conn))
                .await?;
            let last_expected = migrations.last();
            if last_applied.as_ref() != last_expected {
                return Err(Error::msg(format!(
                    "DB is out of date: last applied migration is {last_applied:?}, but expected \
                     {last_expected:?}"
                )));
            }
        } else {
            // Run migrations using `refinery`.
            match runner.run_async(&mut Migrator::from(&mut conn)).await {
                Ok(report) => {
                    tracing::info!("ran DB migrations: {report:?}");
                },
                Err(err) => {
                    tracing::error!("DB migrations failed: {:?}", err.report());
                    Err(err)?;
                },
            }
        }

        if config.archive {
            // If running in archive mode, ensure the pruned height is set to 0, so the fetcher will
            // reconstruct previously pruned data.
            query("DELETE FROM pruned_height WHERE id = 1")
                .execute(conn.as_mut())
                .await?;
        }

        conn.close().await?;

        Ok(Self {
            pool,
            pool_metrics,
            metrics,
            pruner_cfg,
        })
    }
}

impl PrunerConfig for SqlStorage {
    fn set_pruning_config(&mut self, cfg: PrunerCfg) {
        self.pruner_cfg = Some(cfg);
    }

    fn get_pruning_config(&self) -> Option<PrunerCfg> {
        self.pruner_cfg.clone()
    }
}

impl HasMetrics for SqlStorage {
    fn metrics(&self) -> &PrometheusMetrics {
        &self.metrics
    }
}

impl SqlStorage {
    async fn get_minimum_height(&self) -> QueryResult<Option<u64>> {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        let (Some(height),) =
            query_as::<(Option<i64>,)>("SELECT MIN(height) as height FROM header")
                .fetch_one(tx.as_mut())
                .await?
        else {
            return Ok(None);
        };
        Ok(Some(height as u64))
    }

    async fn get_height_by_timestamp(&self, timestamp: i64) -> QueryResult<Option<u64>> {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;

        // We order by timestamp and then height, even though logically this is no different than
        // just ordering by height, since timestamps are monotonic. The reason is that this order
        // allows the query planner to efficiently solve the where clause and presort the results
        // based on the timestamp index. The remaining sort on height, which guarantees a unique
        // block if multiple blocks have the same timestamp, is very efficient, because there are
        // never more than a handful of blocks with the same timestamp.
        let Some((height,)) = query_as::<(i64,)>(
            "SELECT height FROM header
              WHERE timestamp <= $1
              ORDER BY timestamp DESC, height DESC
              LIMIT 1",
        )
        .bind(timestamp)
        .fetch_optional(tx.as_mut())
        .await?
        else {
            return Ok(None);
        };
        Ok(Some(height as u64))
    }

    /// Get the stored VID share for a given block, if one exists.
    pub async fn get_vid_share<Types>(&self, block_id: BlockId<Types>) -> QueryResult<VidShare>
    where
        Types: NodeType,
        Header<Types>: QueryableHeader<Types>,
    {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        let share = tx.vid_share(block_id).await?;
        Ok(share)
    }

    /// Get the stored VID common data for a given block, if one exists.
    pub async fn get_vid_common<Types: NodeType>(
        &self,
        block_id: BlockId<Types>,
    ) -> QueryResult<VidCommonQueryData<Types>>
    where
        <Types as NodeType>::BlockPayload: QueryablePayload<Types>,
        <Types as NodeType>::BlockHeader: QueryableHeader<Types>,
    {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        let common = tx.get_vid_common(block_id).await?;
        Ok(common)
    }

    /// Get the stored VID common metadata for a given block, if one exists.
    pub async fn get_vid_common_metadata<Types: NodeType>(
        &self,
        block_id: BlockId<Types>,
    ) -> QueryResult<VidCommonMetadata<Types>>
    where
        <Types as NodeType>::BlockPayload: QueryablePayload<Types>,
        <Types as NodeType>::BlockHeader: QueryableHeader<Types>,
    {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        let common = tx.get_vid_common_metadata(block_id).await?;
        Ok(common)
    }
}

#[async_trait]
impl PruneStorage for SqlStorage {
    type Pruner = Pruner;

    async fn get_disk_usage(&self) -> anyhow::Result<u64> {
        let mut tx = self.read().await?;

        #[cfg(not(feature = "embedded-db"))]
        let query = "SELECT pg_database_size(current_database())";

        #[cfg(feature = "embedded-db")]
        let query = "
            SELECT( (SELECT page_count FROM pragma_page_count) * (SELECT * FROM pragma_page_size)) \
                     AS total_bytes";

        let row = tx.fetch_one(query).await?;
        let size: i64 = row.get(0);

        Ok(size as u64)
    }

    /// Trigger incremental vacuum to free up space in the SQLite database.
    /// Note: We don't vacuum the Postgres database,
    /// as there is no manual trigger for incremental vacuum,
    /// and a full vacuum can take a lot of time.
    #[cfg(feature = "embedded-db")]
    async fn vacuum(&self) -> anyhow::Result<()> {
        let config = self.get_pruning_config().ok_or(QueryError::Error {
            message: "Pruning config not found".to_string(),
        })?;
        let mut conn = self.pool().acquire().await?;
        query(&format!(
            "PRAGMA incremental_vacuum({})",
            config.incremental_vacuum_pages()
        ))
        .execute(conn.as_mut())
        .await?;
        conn.close().await?;
        Ok(())
    }

    /// Note: The prune operation may not immediately free up space even after rows are deleted.
    /// This is because a vacuum operation may be necessary to reclaim more space.
    /// PostgreSQL already performs auto vacuuming, so we are not including it here
    /// as running a vacuum operation can be resource-intensive.
    async fn prune(&self, pruner: &mut Pruner) -> anyhow::Result<Option<u64>> {
        let cfg = self.get_pruning_config().ok_or(QueryError::Error {
            message: "Pruning config not found".to_string(),
        })?;
        let batch_size = cfg.batch_size();
        let max_usage = cfg.max_usage();
        let state_tables = cfg.state_tables();

        // If a pruner run was already in progress, some variables may already be set,
        // depending on whether a batch was deleted and which batch it was (target or minimum retention).
        // This enables us to resume the pruner run from the exact heights.
        // If any of these values are not set, they can be loaded from the database if necessary.
        let mut minimum_retention_height = pruner.minimum_retention_height;
        let mut target_height = pruner.target_height;
        let pruned_height = match pruner.pruned_height {
            Some(h) => Some(h),
            None => {
                let Some(height) = self.get_minimum_height().await? else {
                    tracing::info!("database is empty, nothing to prune");
                    return Ok(None);
                };

                if height > 0 { Some(height - 1) } else { None }
            },
        };

        // Prune data exceeding target retention in batches
        if pruner.target_height.is_none() {
            let th = self
                .get_height_by_timestamp(
                    Utc::now().timestamp() - (cfg.target_retention().as_secs()) as i64,
                )
                .await?;
            target_height = th;
            pruner.target_height = target_height;
        };

        if let Some(th) = target_height
            && pruned_height < Some(th)
        {
            let batch_end = match pruned_height {
                None => batch_size - 1,
                Some(h) => h + batch_size,
            };
            let to = min(batch_end, th);

            // Update pruned height first so the fetcher does not
            // try to fetch data that we are about to delete.
            let mut tx = self.write().await?;
            tx.save_pruned_height(to).await?;
            tx.commit().await.map_err(|e| QueryError::Error {
                message: format!("failed to commit save_pruned_height {e}"),
            })?;

            let mut tx = self.write().await?;
            tx.delete_batch(to).await?;
            tx.commit().await.map_err(|e| QueryError::Error {
                message: format!("failed to commit delete_batch {e}"),
            })?;

            // Prune state tables in a separate transaction.
            let mut tx = self.write().await?;
            tx.delete_state_batch(state_tables, to).await?;
            tx.commit().await.map_err(|e| QueryError::Error {
                message: format!("failed to commit {e}"),
            })?;

            pruner.pruned_height = Some(to);
            return Ok(Some(to));
        }

        // If threshold is set, prune data exceeding minimum retention in batches
        // This parameter is needed for SQL storage as there is no direct way to get free space.
        if let Some(threshold) = cfg.pruning_threshold() {
            let usage = self.get_disk_usage().await?;

            // Prune data exceeding minimum retention in batches starting from minimum height
            // until usage is below threshold
            if usage > threshold {
                tracing::warn!(
                    "Disk usage {usage} exceeds pruning threshold {:?}",
                    cfg.pruning_threshold()
                );

                if minimum_retention_height.is_none() {
                    minimum_retention_height = self
                        .get_height_by_timestamp(
                            Utc::now().timestamp() - (cfg.minimum_retention().as_secs()) as i64,
                        )
                        .await?;

                    pruner.minimum_retention_height = minimum_retention_height;
                }

                if let Some(min_retention_height) = minimum_retention_height
                    && (usage as f64 / threshold as f64) > (f64::from(max_usage) / 10000.0)
                    && pruned_height < Some(min_retention_height)
                {
                    let batch_end = match pruned_height {
                        None => batch_size - 1,
                        Some(h) => h + batch_size,
                    };
                    let to = min(batch_end, min_retention_height);
                    // Update pruned height first so the fetcher does not
                    // try to fetch data that we are about to delete.
                    let mut tx = self.write().await?;
                    tx.save_pruned_height(to).await?;
                    tx.commit().await.map_err(|e| QueryError::Error {
                        message: format!("failed to commit save_pruned_height {e}"),
                    })?;

                    let mut tx = self.write().await?;
                    tx.delete_batch(to).await?;
                    tx.commit().await.map_err(|e| QueryError::Error {
                        message: format!("failed to commit delete_batch {e}"),
                    })?;

                    // Prune state tables in a separate transaction.
                    let mut tx = self.write().await?;
                    tx.delete_state_batch(state_tables, to).await?;
                    tx.commit().await.map_err(|e| QueryError::Error {
                        message: format!("failed to commit {e}"),
                    })?;

                    self.vacuum().await?;
                    pruner.pruned_height = Some(to);
                    return Ok(Some(to));
                }
            }
        }

        Ok(None)
    }
}

impl VersionedDataSource for SqlStorage {
    type Transaction<'a>
        = Transaction<Write>
    where
        Self: 'a;
    type ReadOnly<'a>
        = Transaction<Read>
    where
        Self: 'a;

    async fn write(&self) -> anyhow::Result<Transaction<Write>> {
        Transaction::new(&self.pool, self.pool_metrics.clone()).await
    }

    async fn read(&self) -> anyhow::Result<Transaction<Read>> {
        Transaction::new(&self.pool, self.pool_metrics.clone()).await
    }
}

// These tests run the `postgres` Docker image, which doesn't work on Windows.
#[cfg(all(any(test, feature = "testing"), not(target_os = "windows")))]
pub mod testing {
    #![allow(unused_imports)]
    use std::{
        env,
        process::{Command, Stdio},
        str::{self, FromStr},
        time::Duration,
    };

    use refinery::Migration;
    use test_utils::reserve_tcp_port;
    use tokio::{net::TcpStream, time::timeout};

    use super::Config;
    use crate::testing::sleep;
    #[derive(Debug)]
    pub struct TmpDb {
        #[cfg(not(feature = "embedded-db"))]
        host: String,
        #[cfg(not(feature = "embedded-db"))]
        port: u16,
        #[cfg(not(feature = "embedded-db"))]
        container_id: String,
        #[cfg(feature = "embedded-db")]
        db_path: std::path::PathBuf,
        #[allow(dead_code)]
        persistent: bool,
    }
    impl TmpDb {
        #[cfg(feature = "embedded-db")]
        fn init_sqlite_db(persistent: bool) -> Self {
            let file = tempfile::Builder::new()
                .prefix("sqlite-")
                .suffix(".db")
                .tempfile()
                .unwrap();

            let (_, db_path) = file.keep().unwrap();

            Self {
                db_path,
                persistent,
            }
        }
        pub async fn init() -> Self {
            #[cfg(feature = "embedded-db")]
            return Self::init_sqlite_db(false);

            #[cfg(not(feature = "embedded-db"))]
            Self::init_postgres(false).await
        }

        pub async fn persistent() -> Self {
            #[cfg(feature = "embedded-db")]
            return Self::init_sqlite_db(true);

            #[cfg(not(feature = "embedded-db"))]
            Self::init_postgres(true).await
        }

        #[cfg(not(feature = "embedded-db"))]
        async fn init_postgres(persistent: bool) -> Self {
            let docker_hostname = env::var("DOCKER_HOSTNAME");
            // This picks an unused port on the current system.  If docker is
            // configured to run on a different host then this may not find a
            // "free" port on that system.
            // We *might* be able to get away with this as any remote docker
            // host should hopefully be pretty open with it's port space.
            let port = reserve_tcp_port().unwrap();
            let host = docker_hostname.unwrap_or("localhost".to_string());

            let mut cmd = Command::new("docker");
            cmd.arg("run")
                .arg("-d")
                .args(["-p", &format!("{port}:5432")])
                .args(["-e", "POSTGRES_PASSWORD=password"]);

            if !persistent {
                cmd.arg("--rm");
            }

            let output = cmd.arg("postgres").output().unwrap();
            let stdout = str::from_utf8(&output.stdout).unwrap();
            let stderr = str::from_utf8(&output.stderr).unwrap();
            if !output.status.success() {
                panic!("failed to start postgres docker: {stderr}");
            }

            // Create the TmpDb object immediately after starting the Docker container, so if
            // anything panics after this `drop` will be called and we will clean up.
            let container_id = stdout.trim().to_owned();
            tracing::info!("launched postgres docker {container_id}");
            let db = Self {
                host,
                port,
                container_id: container_id.clone(),
                persistent,
            };

            db.wait_for_ready().await;
            db
        }

        #[cfg(not(feature = "embedded-db"))]
        pub fn host(&self) -> String {
            self.host.clone()
        }

        #[cfg(not(feature = "embedded-db"))]
        pub fn port(&self) -> u16 {
            self.port
        }

        #[cfg(feature = "embedded-db")]
        pub fn path(&self) -> std::path::PathBuf {
            self.db_path.clone()
        }

        pub fn config(&self) -> Config {
            #[cfg(feature = "embedded-db")]
            let mut cfg = Config::default().db_path(self.db_path.clone());

            #[cfg(not(feature = "embedded-db"))]
            let mut cfg = Config::default()
                .user("postgres")
                .password("password")
                .host(self.host())
                .port(self.port());

            cfg = cfg.migrations(vec![
                Migration::unapplied(
                    "V101__create_test_merkle_tree_table.sql",
                    &TestMerkleTreeMigration::create("test_tree"),
                )
                .unwrap(),
            ]);

            cfg
        }

        #[cfg(not(feature = "embedded-db"))]
        pub fn stop_postgres(&mut self) {
            tracing::info!(container = self.container_id, "stopping postgres");
            let output = Command::new("docker")
                .args(["stop", self.container_id.as_str()])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "error killing postgres docker {}: {}",
                self.container_id,
                str::from_utf8(&output.stderr).unwrap()
            );
        }

        #[cfg(not(feature = "embedded-db"))]
        pub async fn start_postgres(&mut self) {
            tracing::info!(container = self.container_id, "resuming postgres");
            let output = Command::new("docker")
                .args(["start", self.container_id.as_str()])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "error starting postgres docker {}: {}",
                self.container_id,
                str::from_utf8(&output.stderr).unwrap()
            );

            self.wait_for_ready().await;
        }

        #[cfg(not(feature = "embedded-db"))]
        async fn wait_for_ready(&self) {
            let timeout_duration = Duration::from_secs(
                env::var("SQL_TMP_DB_CONNECT_TIMEOUT")
                    .unwrap_or("60".to_string())
                    .parse()
                    .expect("SQL_TMP_DB_CONNECT_TIMEOUT must be an integer number of seconds"),
            );

            if let Err(err) = timeout(timeout_duration, async {
                while Command::new("docker")
                    .args([
                        "exec",
                        &self.container_id,
                        "pg_isready",
                        "-h",
                        "localhost",
                        "-U",
                        "postgres",
                    ])
                    .env("PGPASSWORD", "password")
                    // Null input so the command terminates as soon as it manages to connect.
                    .stdin(Stdio::null())
                    // Discard command output.
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    // We should ensure the exit status. A simple `unwrap`
                    // would panic on unrelated errors (such as network
                    // connection failures)
                    .and_then(|status| {
                        status
                            .success()
                            .then_some(true)
                            // Any ol' Error will do
                            .ok_or(std::io::Error::from_raw_os_error(666))
                    })
                    .is_err()
                {
                    tracing::warn!("database is not ready");
                    sleep(Duration::from_secs(1)).await;
                }

                // The above command ensures the database is ready inside the Docker container.
                // However, on some systems, there is a slight delay before the port is exposed via
                // host networking. We don't need to check again that the database is ready on the
                // host (and maybe can't, because the host might not have pg_isready installed), but
                // we can ensure the port is open by just establishing a TCP connection.
                while let Err(err) =
                    TcpStream::connect(format!("{}:{}", self.host, self.port)).await
                {
                    tracing::warn!("database is ready, but port is not available to host: {err:#}");
                    sleep(Duration::from_millis(100)).await;
                }
            })
            .await
            {
                panic!(
                    "failed to connect to TmpDb within configured timeout {timeout_duration:?}: \
                     {err:#}\n{}",
                    "Consider increasing the timeout by setting SQL_TMP_DB_CONNECT_TIMEOUT"
                );
            }
        }
    }

    #[cfg(not(feature = "embedded-db"))]
    impl Drop for TmpDb {
        fn drop(&mut self) {
            self.stop_postgres();
        }
    }

    #[cfg(feature = "embedded-db")]
    impl Drop for TmpDb {
        fn drop(&mut self) {
            if !self.persistent {
                std::fs::remove_file(self.db_path.clone()).unwrap();
            }
        }
    }

    pub struct TestMerkleTreeMigration;

    impl TestMerkleTreeMigration {
        fn create(name: &str) -> String {
            let (bit_vec, binary, hash_pk, root_stored_column) = if cfg!(feature = "embedded-db") {
                (
                    "TEXT",
                    "BLOB",
                    "INTEGER PRIMARY KEY AUTOINCREMENT",
                    " (json_extract(data, '$.test_merkle_tree_root'))",
                )
            } else {
                (
                    "BIT(8)",
                    "BYTEA",
                    "SERIAL PRIMARY KEY",
                    "(data->>'test_merkle_tree_root')",
                )
            };

            format!(
                "CREATE TABLE IF NOT EXISTS hash
            (
                id {hash_pk},
                value {binary}  NOT NULL UNIQUE
            );

            ALTER TABLE header
            ADD column test_merkle_tree_root text
            GENERATED ALWAYS as {root_stored_column} STORED;

            CREATE TABLE {name}
            (
                path JSONB NOT NULL,
                created BIGINT NOT NULL,
                hash_id INT NOT NULL,
                children JSONB,
                children_bitvec {bit_vec},
                idx JSONB,
                entry JSONB,
                PRIMARY KEY (path, created)
            );
            CREATE INDEX {name}_created ON {name} (created);"
            )
        }
    }
}

// These tests run the `postgres` Docker image, which doesn't work on Windows.
#[cfg(all(test, not(target_os = "windows")))]
mod test {
    use std::time::Duration;

    use hotshot_example_types::{
        node_types::TEST_VERSIONS,
        state_types::{TestInstanceState, TestValidatedState},
    };
    use jf_merkle_tree_compat::{
        MerkleTreeScheme, ToTraversalPath, UniversalMerkleTreeScheme, prelude::UniversalMerkleTree,
    };
    use tokio::time::sleep;

    use super::{testing::TmpDb, *};
    use crate::{
        availability::{BlockQueryData, LeafQueryData},
        data_source::storage::{UpdateAvailabilityStorage, pruning::PrunedHeightStorage},
        merklized_state::{MerklizedState, UpdateStateData},
        testing::mocks::{MockMerkleTree, MockTypes},
    };

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_migrations() {
        let db = TmpDb::init().await;
        let cfg = db.config();

        let connect = |migrations: bool, custom_migrations| {
            let cfg = cfg.clone();
            async move {
                let mut cfg = cfg.migrations(custom_migrations);
                if !migrations {
                    cfg = cfg.no_migrations();
                }
                let client = SqlStorage::connect(cfg, StorageConnectionType::Query).await?;
                Ok::<_, Error>(client)
            }
        };

        // Connecting with migrations disabled should fail if the database is not already up to date
        // (since we've just created a fresh database, it isn't).
        let err = connect(false, vec![]).await.unwrap_err();
        tracing::info!("connecting without running migrations failed as expected: {err}");

        // Now connect and run migrations to bring the database up to date.
        connect(true, vec![]).await.unwrap();
        // Now connecting without migrations should work.
        connect(false, vec![]).await.unwrap();

        // Connect with some custom migrations, to advance the schema even further. Pass in the
        // custom migrations out of order; they should still execute in order of version number.
        // The SQL commands used here will fail if not run in order.
        let migrations = vec![
            Migration::unapplied(
                "V9999__create_test_table.sql",
                "ALTER TABLE test ADD COLUMN data INTEGER;",
            )
            .unwrap(),
            Migration::unapplied(
                "V9998__create_test_table.sql",
                "CREATE TABLE test (x bigint);",
            )
            .unwrap(),
        ];
        connect(true, migrations.clone()).await.unwrap();

        // Connect using the default schema (no custom migrations) and not running migrations. This
        // should fail because the database is _ahead_ of the client in terms of schema.
        let err = connect(false, vec![]).await.unwrap_err();
        tracing::info!("connecting without running migrations failed as expected: {err}");

        // Connecting with the customized schema should work even without running migrations.
        connect(true, migrations).await.unwrap();
    }

    #[test]
    #[cfg(not(feature = "embedded-db"))]
    fn test_config_from_str() {
        let cfg = Config::from_str("postgresql://user:password@host:8080").unwrap();
        assert_eq!(cfg.db_opt.get_username(), "user");
        assert_eq!(cfg.db_opt.get_host(), "host");
        assert_eq!(cfg.db_opt.get_port(), 8080);
    }

    #[test]
    #[cfg(feature = "embedded-db")]
    fn test_config_from_str() {
        let cfg = Config::from_str("sqlite://data.db").unwrap();
        assert_eq!(cfg.db_opt.get_filename().to_string_lossy(), "data.db");
    }

    async fn vacuum(storage: &SqlStorage) {
        #[cfg(feature = "embedded-db")]
        let query = "PRAGMA incremental_vacuum(16000)";
        #[cfg(not(feature = "embedded-db"))]
        let query = "VACUUM";
        storage
            .pool
            .acquire()
            .await
            .unwrap()
            .execute(query)
            .await
            .unwrap();
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_target_period_pruning() {
        let db = TmpDb::init().await;
        let cfg = db.config();

        let mut storage = SqlStorage::connect(cfg, StorageConnectionType::Query)
            .await
            .unwrap();
        let mut leaf = LeafQueryData::<MockTypes>::genesis(
            &TestValidatedState::default(),
            &TestInstanceState::default(),
            TEST_VERSIONS.test,
        )
        .await;
        // insert some mock data
        for i in 0..20 {
            leaf.leaf.block_header_mut().block_number = i;
            leaf.leaf.block_header_mut().timestamp = Utc::now().timestamp() as u64;
            let mut tx = storage.write().await.unwrap();
            tx.insert_leaf(&leaf).await.unwrap();
            tx.commit().await.unwrap();
        }

        let height_before_pruning = storage.get_minimum_height().await.unwrap().unwrap();

        // Set pruner config to default which has minimum retention set to 1 day
        storage.set_pruning_config(PrunerCfg::new());
        // No data will be pruned
        let pruned_height = storage.prune(&mut Default::default()).await.unwrap();

        // Vacuum the database to reclaim space.
        // This is necessary to ensure the test passes.
        // Note: We don't perform a vacuum after each pruner run in production because the auto vacuum job handles it automatically.
        vacuum(&storage).await;
        // Pruned height should be none
        assert!(pruned_height.is_none());

        let height_after_pruning = storage.get_minimum_height().await.unwrap().unwrap();

        assert_eq!(
            height_after_pruning, height_before_pruning,
            "some data has been pruned"
        );

        // Set pruner config to target retention set to 1s
        storage.set_pruning_config(PrunerCfg::new().with_target_retention(Duration::from_secs(1)));
        sleep(Duration::from_secs(2)).await;
        let usage_before_pruning = storage.get_disk_usage().await.unwrap();
        // All of the data is now older than 1s.
        // This would prune all the data as the target retention is set to 1s
        let pruned_height = storage.prune(&mut Default::default()).await.unwrap();
        // Vacuum the database to reclaim space.
        // This is necessary to ensure the test passes.
        // Note: We don't perform a vacuum after each pruner run in production because the auto vacuum job handles it automatically.
        vacuum(&storage).await;

        // Pruned height should be some
        assert!(pruned_height.is_some());
        let usage_after_pruning = storage.get_disk_usage().await.unwrap();
        // All the tables should be empty
        // counting rows in header table
        let header_rows = storage
            .read()
            .await
            .unwrap()
            .fetch_one("select count(*) as count from header")
            .await
            .unwrap()
            .get::<i64, _>("count");
        // the table should be empty
        assert_eq!(header_rows, 0);

        // counting rows in leaf table.
        // Deleting rows from header table would delete rows in all the tables
        // as each of table implement "ON DELETE CASCADE" fk constraint with the header table.
        let leaf_rows = storage
            .read()
            .await
            .unwrap()
            .fetch_one("select count(*) as count from leaf")
            .await
            .unwrap()
            .get::<i64, _>("count");
        // the table should be empty
        assert_eq!(leaf_rows, 0);

        assert!(
            usage_before_pruning > usage_after_pruning,
            " disk usage should decrease after pruning"
        )
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_merklized_state_pruning() {
        let db = TmpDb::init().await;
        let config = db.config();

        let storage = SqlStorage::connect(config, StorageConnectionType::Query)
            .await
            .unwrap();
        let mut test_tree: UniversalMerkleTree<_, _, _, 8, _> =
            MockMerkleTree::new(MockMerkleTree::tree_height());

        // insert some entries into the tree and the header table
        // Header table is used the get_path query to check if the header exists for the block height.
        let mut tx = storage.write().await.unwrap();

        for block_height in 0..250 {
            test_tree.update(block_height, block_height).unwrap();

            // data field of the header
            let test_data = serde_json::json!({ MockMerkleTree::header_state_commitment_field() : serde_json::to_value(test_tree.commitment()).unwrap()});
            tx.upsert(
                "header",
                [
                    "height",
                    "hash",
                    "payload_hash",
                    "timestamp",
                    "data",
                    "ns_table",
                ],
                ["height"],
                [(
                    block_height as i64,
                    format!("randomHash{block_height}"),
                    "t".to_string(),
                    0,
                    test_data,
                    "ns_table".to_string(),
                )],
            )
            .await
            .unwrap();
            // proof for the index from the tree
            let (_, proof) = test_tree.lookup(block_height).expect_ok().unwrap();
            // traversal path for the index.
            let traversal_path =
                <usize as ToTraversalPath<8>>::to_traversal_path(&block_height, test_tree.height());

            UpdateStateData::<_, MockMerkleTree, 8>::insert_merkle_nodes(
                &mut tx,
                proof.clone(),
                traversal_path.clone(),
                block_height as u64,
            )
            .await
            .expect("failed to insert nodes");
        }

        // update saved state height
        UpdateStateData::<_, MockMerkleTree, 8>::set_last_state_height(&mut tx, 250)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let mut tx = storage.read().await.unwrap();

        // checking if the data is inserted correctly
        // there should be multiple nodes with same index but different created time
        let (count,) = query_as::<(i64,)>(
            " SELECT count(*) FROM (SELECT count(*) as count FROM test_tree GROUP BY path having \
             count(*) > 1) AS s",
        )
        .fetch_one(tx.as_mut())
        .await
        .unwrap();

        tracing::info!("Number of nodes with multiple snapshots : {count}");
        assert!(count > 0);

        // This should delete all the nodes having height < 250 and is not the newest node with its position
        let mut tx = storage.write().await.unwrap();
        tx.delete_state_batch(vec!["test_tree".to_string()], 250)
            .await
            .unwrap();

        tx.commit().await.unwrap();
        let mut tx = storage.read().await.unwrap();
        let (count,) = query_as::<(i64,)>(
            "SELECT count(*) FROM (SELECT count(*) as count FROM test_tree GROUP BY path having \
             count(*) > 1) AS s",
        )
        .fetch_one(tx.as_mut())
        .await
        .unwrap();

        tracing::info!("Number of nodes with multiple snapshots : {count}");

        assert!(count == 0);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_minimum_retention_pruning() {
        let db = TmpDb::init().await;

        let mut storage = SqlStorage::connect(db.config(), StorageConnectionType::Query)
            .await
            .unwrap();
        let mut leaf = LeafQueryData::<MockTypes>::genesis(
            &TestValidatedState::default(),
            &TestInstanceState::default(),
            TEST_VERSIONS.test,
        )
        .await;
        // insert some mock data
        for i in 0..20 {
            leaf.leaf.block_header_mut().block_number = i;
            leaf.leaf.block_header_mut().timestamp = Utc::now().timestamp() as u64;
            let mut tx = storage.write().await.unwrap();
            tx.insert_leaf(&leaf).await.unwrap();
            tx.commit().await.unwrap();
        }

        let height_before_pruning = storage.get_minimum_height().await.unwrap().unwrap();
        let cfg = PrunerCfg::new();
        // Set pruning_threshold to 1
        // SQL storage size is more than 1000 bytes even without any data indexed
        // This would mean that the threshold would always be greater than the disk usage
        // However, minimum retention is set to 24 hours by default so the data would not be pruned
        storage.set_pruning_config(cfg.clone().with_pruning_threshold(1));
        println!("{:?}", storage.get_pruning_config().unwrap());
        // Pruning would not delete any data
        // All the data is younger than minimum retention period even though the usage > threshold
        let pruned_height = storage.prune(&mut Default::default()).await.unwrap();
        // Vacuum the database to reclaim space.
        // This is necessary to ensure the test passes.
        // Note: We don't perform a vacuum after each pruner run in production because the auto vacuum job handles it automatically.
        vacuum(&storage).await;

        // Pruned height should be none
        assert!(pruned_height.is_none());

        let height_after_pruning = storage.get_minimum_height().await.unwrap().unwrap();

        assert_eq!(
            height_after_pruning, height_before_pruning,
            "some data has been pruned"
        );

        // Change minimum retention to 1s
        storage.set_pruning_config(
            cfg.with_minimum_retention(Duration::from_secs(1))
                .with_pruning_threshold(1),
        );
        // sleep for 2s to make sure the data is older than minimum retention
        sleep(Duration::from_secs(2)).await;
        // This would prune all the data
        let pruned_height = storage.prune(&mut Default::default()).await.unwrap();
        // Vacuum the database to reclaim space.
        // This is necessary to ensure the test passes.
        // Note: We don't perform a vacuum after each pruner run in production because the auto vacuum job handles it automatically.
        vacuum(&storage).await;

        // Pruned height should be some
        assert!(pruned_height.is_some());
        // All the tables should be empty
        // counting rows in header table
        let header_rows = storage
            .read()
            .await
            .unwrap()
            .fetch_one("select count(*) as count from header")
            .await
            .unwrap()
            .get::<i64, _>("count");
        // the table should be empty
        assert_eq!(header_rows, 0);
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_payload_pruning() {
        let db = TmpDb::init().await;
        let mut storage = SqlStorage::connect(db.config(), StorageConnectionType::Query)
            .await
            .unwrap();
        storage.set_pruning_config(Default::default());

        // Insert some mock data.
        let mut leaf = LeafQueryData::<MockTypes>::genesis(
            &TestValidatedState::default(),
            &TestInstanceState::default(),
            TEST_VERSIONS.test,
        )
        .await;
        let block = BlockQueryData::<MockTypes>::genesis(
            &Default::default(),
            &Default::default(),
            TEST_VERSIONS.test.base,
        )
        .await;
        let vid = VidCommonQueryData::<MockTypes>::genesis(
            &Default::default(),
            &Default::default(),
            TEST_VERSIONS.test.base,
        )
        .await;
        {
            let mut tx = storage.write().await.unwrap();
            tx.insert_leaf(&leaf).await.unwrap();
            tx.insert_block(&block).await.unwrap();
            tx.insert_vid(&vid, None).await.unwrap();
            tx.commit().await.unwrap();
        }

        // Insert a second leaf sharing the same payload.
        leaf.leaf.block_header_mut().block_number += 1;
        {
            let mut tx = storage.write().await.unwrap();
            tx.insert_leaf(&leaf).await.unwrap();
            tx.commit().await.unwrap();
        }
        {
            let mut tx = storage.read().await.unwrap();
            let (num_payloads,): (i64,) = query_as("SELECT count(*) FROM payload")
                .fetch_one(tx.as_mut())
                .await
                .unwrap();
            assert_eq!(num_payloads, 1);
            let (num_vid,): (i64,) = query_as("SELECT count(*) FROM vid_common")
                .fetch_one(tx.as_mut())
                .await
                .unwrap();
            assert_eq!(num_vid, 1);
        }

        // Prune the first leaf but not the second (and thus not the payload or VID).
        let pruned_height = storage
            .prune(&mut Pruner {
                pruned_height: None,
                target_height: Some(0),
                minimum_retention_height: None,
            })
            .await
            .unwrap();
        tracing::info!(?pruned_height, "first pruning run complete");
        {
            let mut tx = storage.read().await.unwrap();

            // First block is pruned.
            let err = tx
                .get_block(BlockId::<MockTypes>::Number(0))
                .await
                .unwrap_err();
            assert!(matches!(err, QueryError::NotFound), "{err:#}");
            let err = tx
                .get_vid_common(BlockId::<MockTypes>::Number(0))
                .await
                .unwrap_err();
            assert!(matches!(err, QueryError::NotFound), "{err:#}");

            // Second block is still available.
            assert_eq!(
                tx.get_block(BlockId::<MockTypes>::Number(1)).await.unwrap(),
                BlockQueryData::new(leaf.header().clone(), block.payload)
            );
            assert_eq!(
                tx.get_vid_common(BlockId::<MockTypes>::Number(1))
                    .await
                    .unwrap(),
                VidCommonQueryData::new(leaf.header().clone(), vid.common)
            );

            let (num_payloads,): (i64,) = query_as("SELECT count(*) FROM payload")
                .fetch_one(tx.as_mut())
                .await
                .unwrap();
            assert_eq!(num_payloads, 1);

            let (num_vid,): (i64,) = query_as("SELECT count(*) FROM vid_common")
                .fetch_one(tx.as_mut())
                .await
                .unwrap();
            assert_eq!(num_vid, 1);
        }

        // Now prune the second leaf, ensuring the payload and VID get deleted as well.
        let pruned_height = storage
            .prune(&mut Pruner {
                pruned_height,
                target_height: Some(1),
                minimum_retention_height: None,
            })
            .await
            .unwrap();
        tracing::info!(?pruned_height, "second pruning run complete");

        let mut tx = storage.read().await.unwrap();
        for i in 0..2 {
            let err = tx
                .get_block(BlockId::<MockTypes>::Number(i))
                .await
                .unwrap_err();
            assert!(matches!(err, QueryError::NotFound), "{err:#}");

            let err = tx
                .get_vid_common(BlockId::<MockTypes>::Number(i))
                .await
                .unwrap_err();
            assert!(matches!(err, QueryError::NotFound), "{err:#}");
        }
        let (num_payloads,): (i64,) = query_as("SELECT count(*) FROM payload")
            .fetch_one(tx.as_mut())
            .await
            .unwrap();
        assert_eq!(num_payloads, 0);

        let (num_vid,): (i64,) = query_as("SELECT count(*) FROM vid_common")
            .fetch_one(tx.as_mut())
            .await
            .unwrap();
        assert_eq!(num_vid, 0);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_pruned_height_storage() {
        let db = TmpDb::init().await;
        let cfg = db.config();

        let storage = SqlStorage::connect(cfg, StorageConnectionType::Query)
            .await
            .unwrap();
        assert!(
            storage
                .read()
                .await
                .unwrap()
                .load_pruned_height()
                .await
                .unwrap()
                .is_none()
        );
        for height in [10, 20, 30] {
            let mut tx = storage.write().await.unwrap();
            tx.save_pruned_height(height).await.unwrap();
            tx.commit().await.unwrap();
            assert_eq!(
                storage
                    .read()
                    .await
                    .unwrap()
                    .load_pruned_height()
                    .await
                    .unwrap(),
                Some(height)
            );
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_transaction_upsert_retries() {
        let db = TmpDb::init().await;
        let config = db.config();

        let storage = SqlStorage::connect(config, StorageConnectionType::Query)
            .await
            .unwrap();

        let mut tx = storage.write().await.unwrap();

        // Try to upsert into a table that does not exist.
        // This will fail, so our `upsert` function will enter the retry loop.
        // Since the table does not exist, all retries will eventually
        // fail and we expect an error to be returned.
        //
        // Previously, this case would cause  a panic because we were calling
        // methods on `QueryBuilder` after `.build()` without first
        // calling `.reset()`and according to the sqlx docs, that always panics.
        // Now, since we are properly calling `.reset()` inside `upsert()` for
        // the query builder, the function returns an error instead of panicking.
        tx.upsert("does_not_exist", ["test"], ["test"], [(1_i64,)])
            .await
            .unwrap_err();
    }
}
