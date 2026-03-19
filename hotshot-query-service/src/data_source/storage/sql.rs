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

use anyhow::Context;
use async_trait::async_trait;
use chrono::Utc;
use committable::Committable;
use futures::future::FutureExt;
use hotshot_types::{
    data::{Leaf, Leaf2, VidCommon, VidShare},
    simple_certificate::{QuorumCertificate, QuorumCertificate2},
    traits::{metrics::Metrics, node_implementation::NodeType},
    vid::advz::{ADVZCommon, ADVZShare},
};
use itertools::Itertools;
use log::LevelFilter;
use sqlx::{
    ConnectOptions, Row,
    pool::PoolOptions,
    postgres::{PgConnectOptions, PgSslMode},
    sqlite::SqliteConnectOptions,
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

mod db;
mod migrate;
mod queries;
mod transaction;

pub use anyhow::Error;
pub use db::{
    BackendPoolConnection, BackendTransaction, DbBackend, SqlPool, SyntaxHelpers, syntax_helpers,
};
pub use include_dir::include_dir;
pub use queries::{QueryBuilder, query, query_as};
pub use refinery::Migration;
pub use transaction::*;

use self::{migrate::Migrator, queries::BackendRow, transaction::PoolMetrics};
use super::{AvailabilityStorage, NodeStorage};
// This needs to be reexported so that we can reference it by absolute path relative to this crate
// in the expansion of `include_migrations`, even when `include_migrations` is invoked from another
// crate which doesn't have `include_dir` as a dependency.
pub use crate::include_migrations;
use crate::with_backend;

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
/// # use hotshot_query_service::data_source::sql::{include_migrations, Migration, DbBackend};
/// let pg_migrations: Vec<Migration> =
///     include_migrations!("$CARGO_MANIFEST_DIR/migrations/postgres").collect();
/// let sqlite_migrations: Vec<Migration> =
///     include_migrations!("$CARGO_MANIFEST_DIR/migrations/sqlite").collect();
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
///
/// The `backend` parameter selects the appropriate migration set for the given database backend.
pub fn default_migrations(backend: DbBackend) -> Vec<Migration> {
    let mut migrations = match backend {
        DbBackend::Postgres => {
            include_migrations!("$CARGO_MANIFEST_DIR/migrations/postgres").collect::<Vec<_>>()
        },
        DbBackend::Sqlite => {
            include_migrations!("$CARGO_MANIFEST_DIR/migrations/sqlite").collect::<Vec<_>>()
        },
    };

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

#[derive(Clone, Debug)]
pub enum Config {
    Postgres(PostgresConfig),
    Sqlite(SqliteConfig),
}

#[derive(Clone, Debug)]
pub struct PostgresConfig {
    db_opt: PgConnectOptions,
    pool_opt: PoolOptions<sqlx::Postgres>,
    /// Extra pool options to allow separately configuring the connection pool for query service.
    pool_opt_query: PoolOptions<sqlx::Postgres>,
    /// The name of the schema to use for queries.
    ///
    /// The default schema is named `hotshot` and is created via the default migrations.
    schema: String,
    reset: bool,
    migrations: Vec<Migration>,
    no_migrations: bool,
    pruner_cfg: Option<PrunerCfg>,
    archive: bool,
    pool: Option<SqlPool>,
}

#[derive(Clone, Debug)]
pub struct SqliteConfig {
    db_opt: SqliteConnectOptions,
    pool_opt: PoolOptions<sqlx::Sqlite>,
    reset: bool,
    migrations: Vec<Migration>,
    no_migrations: bool,
    pruner_cfg: Option<PrunerCfg>,
    archive: bool,
    pool: Option<SqlPool>,
}

impl Config {
    pub fn backend(&self) -> DbBackend {
        match self {
            Self::Postgres(_) => DbBackend::Postgres,
            Self::Sqlite(_) => DbBackend::Sqlite,
        }
    }

    /// Create a default Postgres config.
    pub fn postgres_default() -> Self {
        PgConnectOptions::default()
            .username("postgres")
            .password("password")
            .host("localhost")
            .port(5432)
            .into()
    }

    /// Create a default SQLite config.
    pub fn sqlite_default() -> Self {
        SqliteConnectOptions::default()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(30))
            .auto_vacuum(sqlx::sqlite::SqliteAutoVacuum::Incremental)
            .create_if_missing(true)
            .into()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::postgres_default()
    }
}

impl From<PgConnectOptions> for Config {
    fn from(db_opt: PgConnectOptions) -> Self {
        Self::Postgres(PostgresConfig {
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
        })
    }
}

impl From<SqliteConnectOptions> for Config {
    fn from(db_opt: SqliteConnectOptions) -> Self {
        Self::Sqlite(SqliteConfig {
            db_opt,
            pool_opt: PoolOptions::default(),
            reset: false,
            migrations: vec![],
            no_migrations: false,
            pruner_cfg: None,
            archive: false,
            pool: None,
        })
    }
}

impl Config {
    /// Parse a connection string.
    ///
    /// If the string starts with `sqlite:`, it is parsed as a SQLite connection string. Otherwise,
    /// it is parsed as a PostgreSQL connection string.
    pub fn parse(s: &str) -> Result<Self, Error> {
        if s.starts_with("sqlite:") {
            Ok(SqliteConnectOptions::from_str(s)
                .map_err(|e| Error::msg(format!("invalid SQLite connection string: {e}")))?
                .into())
        } else {
            Ok(PgConnectOptions::from_str(s)
                .map_err(|e| Error::msg(format!("invalid PostgreSQL connection string: {e}")))?
                .into())
        }
    }
}

impl Config {
    /// Set the SQLite busy timeout.
    ///
    /// Only applicable to SQLite configs; ignored for Postgres.
    pub fn busy_timeout(mut self, timeout: Duration) -> Self {
        if let Self::Sqlite(ref mut cfg) = self {
            cfg.db_opt = cfg.db_opt.clone().busy_timeout(timeout);
        }
        self
    }

    /// Set the SQLite database file path.
    ///
    /// Only applicable to SQLite configs; ignored for Postgres.
    pub fn db_path(mut self, path: std::path::PathBuf) -> Self {
        if let Self::Sqlite(ref mut cfg) = self {
            cfg.db_opt = cfg.db_opt.clone().filename(path);
        }
        self
    }

    /// Set the hostname of the database server.
    ///
    /// The default is `localhost`.
    ///
    /// Only applicable to Postgres configs; ignored for SQLite.
    pub fn host(mut self, host: impl Into<String>) -> Self {
        if let Self::Postgres(ref mut cfg) = self {
            cfg.db_opt = cfg.db_opt.clone().host(&host.into());
        }
        self
    }

    /// Set the port on which to connect to the database.
    ///
    /// The default is 5432, the default Postgres port.
    ///
    /// Only applicable to Postgres configs; ignored for SQLite.
    pub fn port(mut self, port: u16) -> Self {
        if let Self::Postgres(ref mut cfg) = self {
            cfg.db_opt = cfg.db_opt.clone().port(port);
        }
        self
    }

    /// Set the DB user to connect as.
    ///
    /// Only applicable to Postgres configs; ignored for SQLite.
    pub fn user(mut self, user: &str) -> Self {
        if let Self::Postgres(ref mut cfg) = self {
            cfg.db_opt = cfg.db_opt.clone().username(user);
        }
        self
    }

    /// Set a password for connecting to the database.
    ///
    /// Only applicable to Postgres configs; ignored for SQLite.
    pub fn password(mut self, password: &str) -> Self {
        if let Self::Postgres(ref mut cfg) = self {
            cfg.db_opt = cfg.db_opt.clone().password(password);
        }
        self
    }

    /// Set the name of the database to connect to.
    ///
    /// Only applicable to Postgres configs; ignored for SQLite.
    pub fn database(mut self, database: &str) -> Self {
        if let Self::Postgres(ref mut cfg) = self {
            cfg.db_opt = cfg.db_opt.clone().database(database);
        }
        self
    }

    /// Use TLS for an encrypted connection to the database.
    ///
    /// Note that an encrypted connection may be established even if this option is not set, as long
    /// as both the client and server support it. This option merely causes connection to fail if an
    /// encrypted stream cannot be established.
    ///
    /// Only applicable to Postgres configs; ignored for SQLite.
    pub fn tls(mut self) -> Self {
        if let Self::Postgres(ref mut cfg) = self {
            cfg.db_opt = cfg.db_opt.clone().ssl_mode(PgSslMode::Require);
        }
        self
    }

    /// Set the name of the schema to use for queries.
    ///
    /// The default schema is named `hotshot` and is created via the default migrations.
    ///
    /// Only applicable to Postgres configs; ignored for SQLite.
    pub fn schema(mut self, schema: impl Into<String>) -> Self {
        if let Self::Postgres(ref mut cfg) = self {
            cfg.schema = schema.into();
        }
        self
    }

    /// Sets the database connection pool.
    /// This allows reusing an existing connection pool when building a new `SqlStorage` instance.
    pub fn pool(mut self, pool: SqlPool) -> Self {
        match &mut self {
            Self::Postgres(cfg) => cfg.pool = Some(pool),
            Self::Sqlite(cfg) => cfg.pool = Some(pool),
        }
        self
    }

    /// Reset the schema on connection.
    ///
    /// When this [`Config`] is used to [`connect`](SqlStorage::connect) a
    /// [`SqlDataSource`](crate::data_source::SqlDataSource), if this option is set, the relevant
    /// [`schema`](Self::schema) will first be dropped and then recreated, yielding a completely
    /// fresh instance of the query service.
    ///
    /// This is a particularly useful capability for development and staging environments. Still, it
    /// must be used with extreme caution, as using this will irrevocably delete any data pertaining
    /// to the query service in the database.
    pub fn reset_schema(mut self) -> Self {
        match &mut self {
            Self::Postgres(cfg) => cfg.reset = true,
            Self::Sqlite(cfg) => cfg.reset = true,
        }
        self
    }

    /// Add custom migrations to run when connecting to the database.
    pub fn migrations(mut self, migrations: impl IntoIterator<Item = Migration>) -> Self {
        match &mut self {
            Self::Postgres(cfg) => cfg.migrations.extend(migrations),
            Self::Sqlite(cfg) => cfg.migrations.extend(migrations),
        }
        self
    }

    /// Skip all migrations when connecting to the database.
    pub fn no_migrations(mut self) -> Self {
        match &mut self {
            Self::Postgres(cfg) => cfg.no_migrations = true,
            Self::Sqlite(cfg) => cfg.no_migrations = true,
        }
        self
    }

    /// Enable pruning with a given configuration.
    ///
    /// If [`archive`](Self::archive) was previously specified, this will override it.
    pub fn pruner_cfg(mut self, cfg: PrunerCfg) -> Result<Self, Error> {
        cfg.validate()?;
        match &mut self {
            Self::Postgres(c) => {
                c.pruner_cfg = Some(cfg);
                c.archive = false;
            },
            Self::Sqlite(c) => {
                c.pruner_cfg = Some(cfg);
                c.archive = false;
            },
        }
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
        match &mut self {
            Self::Postgres(cfg) => {
                cfg.pruner_cfg = None;
                cfg.archive = true;
            },
            Self::Sqlite(cfg) => {
                cfg.pruner_cfg = None;
                cfg.archive = true;
            },
        }
        self
    }

    /// Set the maximum idle time of a connection.
    ///
    /// Any connection which has been open and unused longer than this duration will be
    /// automatically closed to reduce load on the server.
    pub fn idle_connection_timeout(mut self, timeout: Duration) -> Self {
        match &mut self {
            Self::Postgres(cfg) => {
                cfg.pool_opt = cfg.pool_opt.clone().idle_timeout(Some(timeout));
                cfg.pool_opt_query = cfg.pool_opt_query.clone().idle_timeout(Some(timeout));
            },
            Self::Sqlite(cfg) => {
                cfg.pool_opt = cfg.pool_opt.clone().idle_timeout(Some(timeout));
            },
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
        match &mut self {
            Self::Postgres(cfg) => {
                cfg.pool_opt = cfg.pool_opt.clone().max_lifetime(Some(timeout));
                cfg.pool_opt_query = cfg.pool_opt_query.clone().max_lifetime(Some(timeout));
            },
            Self::Sqlite(cfg) => {
                cfg.pool_opt = cfg.pool_opt.clone().max_lifetime(Some(timeout));
            },
        }
        self
    }

    /// Set the minimum number of connections to maintain at any time.
    ///
    /// The data source will, to the best of its ability, maintain at least `min` open connections
    /// at all times. This can be used to reduce the latency hit of opening new connections when at
    /// least this many simultaneous connections are frequently needed.
    pub fn min_connections(mut self, min: u32) -> Self {
        match &mut self {
            Self::Postgres(cfg) => {
                cfg.pool_opt = cfg.pool_opt.clone().min_connections(min);
            },
            Self::Sqlite(cfg) => {
                cfg.pool_opt = cfg.pool_opt.clone().min_connections(min);
            },
        }
        self
    }

    /// Set the minimum number of connections for the query pool (Postgres only).
    pub fn query_min_connections(mut self, min: u32) -> Self {
        if let Self::Postgres(ref mut cfg) = self {
            cfg.pool_opt_query = cfg.pool_opt_query.clone().min_connections(min);
        }
        self
    }

    /// Set the maximum number of connections to maintain at any time.
    ///
    /// Once `max` connections are in use simultaneously, further attempts to acquire a connection
    /// (or begin a transaction) will block until one of the existing connections is released.
    pub fn max_connections(mut self, max: u32) -> Self {
        match &mut self {
            Self::Postgres(cfg) => {
                cfg.pool_opt = cfg.pool_opt.clone().max_connections(max);
            },
            Self::Sqlite(cfg) => {
                cfg.pool_opt = cfg.pool_opt.clone().max_connections(max);
            },
        }
        self
    }

    /// Set the maximum number of connections for the query pool (Postgres only).
    pub fn query_max_connections(mut self, max: u32) -> Self {
        if let Self::Postgres(ref mut cfg) = self {
            cfg.pool_opt_query = cfg.pool_opt_query.clone().max_connections(max);
        }
        self
    }

    /// Log at WARN level any time a SQL statement takes longer than `threshold`.
    ///
    /// The default threshold is 1s.
    pub fn slow_statement_threshold(mut self, threshold: Duration) -> Self {
        match &mut self {
            Self::Postgres(cfg) => {
                cfg.db_opt = cfg
                    .db_opt
                    .clone()
                    .log_slow_statements(LevelFilter::Warn, threshold);
            },
            Self::Sqlite(cfg) => {
                cfg.db_opt = cfg
                    .db_opt
                    .clone()
                    .log_slow_statements(LevelFilter::Warn, threshold);
            },
        }
        self
    }

    /// Set the maximum time a single SQL statement is allowed to run before being canceled.
    ///
    /// This helps prevent queries from running indefinitely even when the client is dropped.
    ///
    /// For Postgres, this sets the `statement_timeout` connection option.
    /// For SQLite, this is a no-op (not supported).
    pub fn statement_timeout(mut self, timeout: Duration) -> Self {
        if let Self::Postgres(ref mut cfg) = self {
            // Format duration as milliseconds.
            // PostgreSQL interprets values without units as milliseconds.
            let timeout_ms = timeout.as_millis();
            cfg.db_opt = cfg
                .db_opt
                .clone()
                .options([("statement_timeout", timeout_ms.to_string())]);
        }
        self
    }
}

/// Storage for the APIs provided in this crate, backed by a remote PostgreSQL or local SQLite database.
#[derive(Clone, Debug)]
pub struct SqlStorage {
    pool: SqlPool,
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
    pub fn pool(&self) -> SqlPool {
        self.pool.clone()
    }

    pub fn backend(&self) -> DbBackend {
        self.pool.backend()
    }

    /// Connect to a remote database.
    pub async fn connect(
        mut config: Config,
        connection_type: StorageConnectionType,
    ) -> Result<Self, Error> {
        let metrics = PrometheusMetrics::default();
        let pool_metrics = PoolMetrics::new(&*metrics.subgroup("sql".into()));

        match config {
            Config::Postgres(ref mut pg) => {
                Self::connect_postgres(pg, connection_type, metrics, pool_metrics).await
            },
            Config::Sqlite(ref mut sq) => Self::connect_sqlite(sq, metrics, pool_metrics).await,
        }
    }

    async fn connect_postgres(
        config: &mut PostgresConfig,
        connection_type: StorageConnectionType,
        metrics: PrometheusMetrics,
        pool_metrics: PoolMetrics,
    ) -> Result<Self, Error> {
        let pruner_cfg = config.pruner_cfg.clone();

        let pool_opt = match connection_type {
            StorageConnectionType::Sequencer => config.pool_opt.clone(),
            StorageConnectionType::Query => config.pool_opt_query.clone(),
        };

        // Re-use the same pool if present and return early.
        if connection_type == StorageConnectionType::Sequencer {
            if let Some(ref pool) = config.pool {
                return Ok(Self {
                    metrics,
                    pool_metrics,
                    pool: pool.clone(),
                    pruner_cfg,
                });
            }
        } else if config.pool.is_some() {
            tracing::info!("not reusing existing pool for query connection");
        }

        let schema = config.schema.clone();
        let pool_opt = pool_opt.after_connect(move |conn, _| {
            let schema = schema.clone();
            async move {
                sqlx::query(&format!("SET search_path TO {schema}"))
                    .execute(conn)
                    .await?;
                Ok(())
            }
            .boxed()
        });

        let pg_pool = pool_opt.connect_with(config.db_opt.clone()).await?;

        // Create or connect to the schema for this query service.
        let mut conn = pg_pool.acquire().await?;

        // Disable statement timeout for migrations, as they can take a long time.
        sqlx::query("SET statement_timeout = 0")
            .execute(conn.as_mut())
            .await?;

        if config.reset {
            sqlx::query(&format!("DROP SCHEMA IF EXISTS {} CASCADE", config.schema))
                .execute(conn.as_mut())
                .await?;
        }

        sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS {}", config.schema))
            .execute(conn.as_mut())
            .await?;

        // Get migrations and interleave with custom migrations, sorting by version number.
        validate_migrations(&mut config.migrations)?;
        let migrations = add_custom_migrations(
            default_migrations(DbBackend::Postgres),
            config.migrations.drain(..),
        )
        .collect::<Vec<_>>();

        // Get a migration runner. Depending on the config, we can either use this to actually run
        // the migrations or just check if the database is up to date.
        let runner = refinery::Runner::new(&migrations).set_grouped(true);

        let mut backend_conn = BackendPoolConnection::Postgres(conn);
        if config.no_migrations {
            // We've been asked not to run any migrations. Abort if the DB is not already up to
            // date.
            let last_applied = runner
                .get_last_applied_migration_async(&mut Migrator::from(&mut backend_conn))
                .await?;
            let last_expected = migrations.last();
            if last_applied.as_ref() != last_expected {
                return Err(Error::msg(format!(
                    "DB is out of date: last applied migration is {last_applied:?}, but expected \
                     {last_expected:?}"
                )));
            }
        } else {
            // Run migrations using refinery.
            match runner
                .run_async(&mut Migrator::from(&mut backend_conn))
                .await
            {
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
            // If running in archive mode, ensure the pruned height is set to 0, so the fetcher
            // will reconstruct previously pruned data.
            match &mut backend_conn {
                BackendPoolConnection::Postgres(c) => {
                    sqlx::query("DELETE FROM pruned_height WHERE id = 1")
                        .execute(c.as_mut())
                        .await?;
                },
                _ => unreachable!(),
            }
        }

        match backend_conn {
            BackendPoolConnection::Postgres(c) => c.close().await?,
            _ => unreachable!(),
        }

        Ok(Self {
            pool: SqlPool::Postgres(pg_pool),
            pool_metrics,
            metrics,
            pruner_cfg,
        })
    }

    async fn connect_sqlite(
        config: &mut SqliteConfig,
        metrics: PrometheusMetrics,
        pool_metrics: PoolMetrics,
    ) -> Result<Self, Error> {
        let pruner_cfg = config.pruner_cfg.clone();

        // Re-use the same pool if present and return early.
        if let Some(ref pool) = config.pool {
            return Ok(Self {
                metrics,
                pool_metrics,
                pool: pool.clone(),
                pruner_cfg,
            });
        }

        if config.reset {
            let _ = std::fs::remove_file(config.db_opt.get_filename());
        }

        let sq_pool = config
            .pool_opt
            .clone()
            .connect_with(config.db_opt.clone())
            .await?;

        let conn = sq_pool.acquire().await?;

        // Get migrations and interleave with custom migrations, sorting by version number.
        validate_migrations(&mut config.migrations)?;
        let migrations = add_custom_migrations(
            default_migrations(DbBackend::Sqlite),
            config.migrations.drain(..),
        )
        .collect::<Vec<_>>();

        // Get a migration runner. Depending on the config, we can either use this to actually run
        // the migrations or just check if the database is up to date.
        let runner = refinery::Runner::new(&migrations).set_grouped(true);

        let mut backend_conn = BackendPoolConnection::Sqlite(conn);
        if config.no_migrations {
            // We've been asked not to run any migrations. Abort if the DB is not already up to
            // date.
            let last_applied = runner
                .get_last_applied_migration_async(&mut Migrator::from(&mut backend_conn))
                .await?;
            let last_expected = migrations.last();
            if last_applied.as_ref() != last_expected {
                return Err(Error::msg(format!(
                    "DB is out of date: last applied migration is {last_applied:?}, but expected \
                     {last_expected:?}"
                )));
            }
        } else {
            // Run migrations using refinery.
            match runner
                .run_async(&mut Migrator::from(&mut backend_conn))
                .await
            {
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
            // If running in archive mode, ensure the pruned height is set to 0, so the fetcher
            // will reconstruct previously pruned data.
            match &mut backend_conn {
                BackendPoolConnection::Sqlite(c) => {
                    sqlx::query("DELETE FROM pruned_height WHERE id = 1")
                        .execute(c.as_mut())
                        .await?;
                },
                _ => unreachable!(),
            }
        }

        match backend_conn {
            BackendPoolConnection::Sqlite(c) => c.close().await?,
            _ => unreachable!(),
        }

        Ok(Self {
            pool: SqlPool::Sqlite(sq_pool),
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
        let result: Option<(Option<i64>,)> = with_backend!(tx, |inner| {
            sqlx::query_as("SELECT MIN(height) as height FROM header")
                .fetch_optional(inner.as_mut())
                .await
        })?;
        let Some((Some(height),)) = result else {
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
        let result: Option<(i64,)> = with_backend!(tx, |inner| {
            sqlx::query_as(
                "SELECT height FROM header
                  WHERE timestamp <= $1
                  ORDER BY timestamp DESC, height DESC
                  LIMIT 1",
            )
            .bind(timestamp)
            .fetch_optional(inner.as_mut())
            .await
        })?;
        let Some((height,)) = result else {
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

        let sql = match self.pool.backend() {
            DbBackend::Postgres => "SELECT pg_database_size(current_database())",
            DbBackend::Sqlite => {
                "SELECT( (SELECT page_count FROM pragma_page_count) * (SELECT * FROM \
                 pragma_page_size)) AS total_bytes"
            },
        };
        let (size,): (i64,) = with_backend!(tx, |inner| {
            sqlx::query_as(sql).fetch_one(inner.as_mut()).await
        })?;

        Ok(size as u64)
    }

    /// Trigger incremental vacuum to free up space in the SQLite database.
    /// Note: We don't vacuum the Postgres database, as there is no manual trigger for incremental
    /// vacuum, and a full vacuum can take a lot of time.
    async fn vacuum(&self) -> anyhow::Result<()> {
        if self.pool.backend() != DbBackend::Sqlite {
            return Ok(());
        }
        let config = self.get_pruning_config().ok_or(QueryError::Error {
            message: "Pruning config not found".to_string(),
        })?;
        let conn = self.pool.acquire().await?;
        let sql = format!(
            "PRAGMA incremental_vacuum({})",
            config.incremental_vacuum_pages()
        );
        match conn {
            BackendPoolConnection::Sqlite(mut c) => {
                sqlx::query(&sql).execute(c.as_mut()).await?;
                c.close().await?;
            },
            _ => unreachable!(),
        }
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
        let mut height = match pruner.pruned_height {
            Some(h) => h,
            None => {
                let Some(height) = self.get_minimum_height().await? else {
                    tracing::info!("database is empty, nothing to prune");
                    return Ok(None);
                };

                height
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

        if let Some(target_height) = target_height
            && height < target_height
        {
            height = min(height + batch_size, target_height);
            let mut tx = self.write().await?;
            tx.delete_batch(state_tables, height).await?;
            tx.commit().await.map_err(|e| QueryError::Error {
                message: format!("failed to commit {e}"),
            })?;
            pruner.pruned_height = Some(height);
            return Ok(Some(height));
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
                    && height < min_retention_height
                {
                    height = min(height + batch_size, min_retention_height);
                    let mut tx = self.write().await?;
                    tx.delete_batch(state_tables, height).await?;
                    tx.commit().await.map_err(|e| QueryError::Error {
                        message: format!("failed to commit {e}"),
                    })?;

                    self.vacuum().await?;

                    pruner.pruned_height = Some(height);

                    return Ok(Some(height));
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

#[async_trait]
pub trait MigrateTypes<Types: NodeType> {
    async fn migrate_types(&self, batch_size: u64) -> anyhow::Result<()>;
}

#[async_trait]
impl<Types: NodeType> MigrateTypes<Types> for SqlStorage {
    async fn migrate_types(&self, batch_size: u64) -> anyhow::Result<()> {
        let limit = batch_size;
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;

        // The table `types_migration` is populated in the SQL migration with `completed = false` and `migrated_rows = 0`
        // so fetch_one() would always return a row.
        // After each batch insert, it is updated with the number of rows migrated.
        // This is necessary to resume from the same point in case of a restart.
        let (is_migration_completed, mut offset): (bool, i64) = with_backend!(tx, |inner| {
            sqlx::query_as("SELECT completed, migrated_rows from types_migration WHERE id = 1 ")
                .fetch_one(inner.as_mut())
                .await
        })?;

        if is_migration_completed {
            tracing::info!("types migration already completed");
            return Ok(());
        }

        tracing::warn!(
            "migrating query service types storage. Offset={offset}, batch_size={limit}"
        );

        loop {
            let mut tx = self.read().await.map_err(|err| QueryError::Error {
                message: err.to_string(),
            })?;

            let rows: Vec<BackendRow> = QueryBuilder::new(self.pool.backend())
                .query(
                    "SELECT leaf, qc, common as vid_common, share as vid_share
                    FROM leaf INNER JOIN vid on leaf.height = vid.height
                    WHERE leaf.height >= $1 AND leaf.height < $2",
                )
                .bind(offset)
                .bind(offset + limit as i64)
                .fetch_all(&mut tx)
                .await?;

            drop(tx);

            if rows.is_empty() {
                break;
            }

            let mut leaf_rows = Vec::new();
            let mut vid_rows = Vec::new();

            for row in rows.iter() {
                let leaf1: serde_json::Value = row.try_get("leaf")?;
                let qc: serde_json::Value = row.try_get("qc")?;
                let leaf1: Leaf<Types> = serde_json::from_value(leaf1)?;
                let qc: QuorumCertificate<Types> = serde_json::from_value(qc)?;

                let leaf2: Leaf2<Types> = leaf1.into();
                let qc2: QuorumCertificate2<Types> = qc.to_qc2();

                let commit = leaf2.commit();

                let leaf2_json =
                    serde_json::to_value(leaf2.clone()).context("failed to serialize leaf2")?;
                let qc2_json = serde_json::to_value(qc2).context("failed to serialize QC2")?;

                let vid_common_bytes: Vec<u8> = row.try_get("vid_common")?;
                let vid_share_bytes: Option<Vec<u8>> = row.try_get("vid_share")?;

                let mut new_vid_share_bytes = None;

                if let Some(ref vid_share_bytes) = vid_share_bytes {
                    let vid_share: ADVZShare = bincode::deserialize(vid_share_bytes)
                        .context("failed to deserialize vid_share")?;
                    new_vid_share_bytes = Some(
                        bincode::serialize(&VidShare::V0(vid_share))
                            .context("failed to serialize vid_share")?,
                    );
                }

                let vid_common: ADVZCommon = bincode::deserialize(&vid_common_bytes)
                    .context("failed to deserialize vid_common")?;
                let new_vid_common_bytes = bincode::serialize(&VidCommon::V0(vid_common))
                    .context("failed to serialize vid_common")?;

                vid_rows.push((
                    leaf2.height() as i64,
                    new_vid_common_bytes,
                    new_vid_share_bytes,
                ));
                leaf_rows.push((
                    leaf2.height() as i64,
                    commit.to_string(),
                    leaf2.block_header().commit().to_string(),
                    leaf2_json,
                    qc2_json,
                ));
            }

            // Advance the `offset` to the highest `leaf.height` processed in this batch.
            // This ensures the next iteration starts from the next unseen leaf
            offset += limit as i64;

            let mut tx = self.write().await.map_err(|err| QueryError::Error {
                message: err.to_string(),
            })?;

            // migrate leaf2
            with_backend!(tx, |inner| {
                let mut query_builder = sqlx::QueryBuilder::new(
                    "INSERT INTO leaf2 (height, hash, block_hash, leaf, qc) ",
                );
                query_builder.push_values(&leaf_rows, |mut b, row| {
                    b.push_bind(row.0)
                        .push_bind(row.1.clone())
                        .push_bind(row.2.clone())
                        .push_bind(row.3.clone())
                        .push_bind(row.4.clone());
                });
                query_builder.push(" ON CONFLICT DO NOTHING");
                let query = query_builder.build();
                query.execute(inner.as_mut()).await.map(|_| ())
            })?;

            // update migrated_rows column with the offset
            tx.upsert(
                "types_migration",
                ["id", "completed", "migrated_rows"],
                ["id"],
                [(1_i64, false, offset)],
            )
            .await?;

            // migrate vid
            with_backend!(tx, |inner| {
                let mut query_builder =
                    sqlx::QueryBuilder::new("INSERT INTO vid2 (height, common, share) ");
                query_builder.push_values(&vid_rows, |mut b, row| {
                    b.push_bind(row.0)
                        .push_bind(row.1.clone())
                        .push_bind(row.2.clone());
                });
                query_builder.push(" ON CONFLICT DO NOTHING");
                let query = query_builder.build();
                query.execute(inner.as_mut()).await.map(|_| ())
            })?;

            tx.commit().await?;

            tracing::warn!("Migrated leaf and vid: offset={offset}");

            tracing::info!("offset={offset}");
            if rows.len() < limit as usize {
                break;
            }
        }

        let mut tx = self.write().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;

        tracing::warn!("query service types migration is completed!");

        tx.upsert(
            "types_migration",
            ["id", "completed", "migrated_rows"],
            ["id"],
            [(1_i64, true, offset)],
        )
        .await?;

        tracing::info!("updated types_migration table");

        tx.commit().await?;
        Ok(())
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

    use super::{Config, DbBackend};
    use crate::testing::sleep;

    #[derive(Debug)]
    pub enum TmpDb {
        Postgres {
            host: String,
            port: u16,
            container_id: String,
            persistent: bool,
        },
        Sqlite {
            db_path: std::path::PathBuf,
            persistent: bool,
        },
    }

    impl TmpDb {
        pub fn init_sqlite(persistent: bool) -> Self {
            let file = tempfile::Builder::new()
                .prefix("sqlite-")
                .suffix(".db")
                .tempfile()
                .unwrap();

            let (_, db_path) = file.keep().unwrap();

            Self::Sqlite {
                db_path,
                persistent,
            }
        }

        pub async fn init() -> Self {
            Self::init_postgres(false).await
        }

        pub async fn init_for(backend: DbBackend) -> Self {
            match backend {
                DbBackend::Postgres => Self::init_postgres(false).await,
                DbBackend::Sqlite => Self::init_sqlite(false),
            }
        }

        pub async fn persistent() -> Self {
            Self::init_postgres(true).await
        }

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
            let db = Self::Postgres {
                host,
                port,
                container_id: container_id.clone(),
                persistent,
            };

            db.wait_for_ready().await;
            db
        }

        pub fn host(&self) -> String {
            match self {
                Self::Postgres { host, .. } => host.clone(),
                Self::Sqlite { .. } => panic!("host() not applicable to SQLite"),
            }
        }

        pub fn port(&self) -> u16 {
            match self {
                Self::Postgres { port, .. } => *port,
                Self::Sqlite { .. } => panic!("port() not applicable to SQLite"),
            }
        }

        pub fn path(&self) -> std::path::PathBuf {
            match self {
                Self::Sqlite { db_path, .. } => db_path.clone(),
                Self::Postgres { .. } => panic!("path() not applicable to Postgres"),
            }
        }

        pub fn config(&self) -> Config {
            let mut cfg = match self {
                Self::Postgres { host, port, .. } => Config::postgres_default()
                    .user("postgres")
                    .password("password")
                    .host(host.clone())
                    .port(*port),
                Self::Sqlite { db_path, .. } => Config::sqlite_default().db_path(db_path.clone()),
            };

            let migration_sql = TestMerkleTreeMigration::create("test_tree", &cfg);
            cfg = cfg.migrations(vec![
                Migration::unapplied("V101__create_test_merkle_tree_table.sql", &migration_sql)
                    .unwrap(),
            ]);

            cfg
        }

        pub fn stop_postgres(&mut self) {
            if let Self::Postgres { container_id, .. } = self {
                tracing::info!(container = %container_id, "stopping postgres");
                let output = Command::new("docker")
                    .args(["stop", container_id.as_str()])
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "error killing postgres docker {}: {}",
                    container_id,
                    str::from_utf8(&output.stderr).unwrap()
                );
            }
        }

        pub async fn start_postgres(&mut self) {
            if let Self::Postgres { container_id, .. } = self {
                tracing::info!(container = %container_id, "resuming postgres");
                let output = Command::new("docker")
                    .args(["start", container_id.as_str()])
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "error starting postgres docker {}: {}",
                    container_id,
                    str::from_utf8(&output.stderr).unwrap()
                );

                self.wait_for_ready().await;
            }
        }

        async fn wait_for_ready(&self) {
            let Self::Postgres {
                host,
                port,
                container_id,
                ..
            } = self
            else {
                return;
            };
            let host = host.clone();
            let port = *port;
            let container_id = container_id.clone();

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
                        &container_id,
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
                while let Err(err) = TcpStream::connect(format!("{host}:{port}")).await {
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

    impl Drop for TmpDb {
        fn drop(&mut self) {
            match self {
                Self::Postgres { .. } => {
                    self.stop_postgres();
                },
                Self::Sqlite {
                    db_path,
                    persistent,
                } => {
                    if !*persistent {
                        let _ = std::fs::remove_file(db_path.clone());
                    }
                },
            }
        }
    }

    pub struct TestMerkleTreeMigration;

    impl TestMerkleTreeMigration {
        fn create(name: &str, cfg: &Config) -> String {
            let (bit_vec, binary, hash_pk, root_stored_column) = match cfg.backend() {
                super::DbBackend::Sqlite => (
                    "TEXT",
                    "BLOB",
                    "INTEGER PRIMARY KEY AUTOINCREMENT",
                    " (json_extract(data, '$.test_merkle_tree_root'))",
                ),
                super::DbBackend::Postgres => (
                    "BIT(8)",
                    "BYTEA",
                    "SERIAL PRIMARY KEY",
                    "(data->>'test_merkle_tree_root')",
                ),
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

    use committable::{Commitment, CommitmentBoundsArkless, Committable};
    use hotshot::traits::BlockPayload;
    use hotshot_example_types::{
        node_types::TEST_VERSIONS,
        state_types::{TestInstanceState, TestValidatedState},
    };
    use hotshot_types::{
        data::{QuorumProposal, ViewNumber},
        simple_vote::QuorumData,
        traits::{
            EncodeBytes,
            block_contents::{BlockHeader, GENESIS_VID_NUM_STORAGE_NODES},
        },
        vid::advz::advz_scheme,
    };
    use jf_advz::VidScheme;
    use jf_merkle_tree_compat::{
        MerkleTreeScheme, ToTraversalPath, UniversalMerkleTreeScheme, prelude::UniversalMerkleTree,
    };
    use rstest::rstest;
    use rstest_reuse::{self, apply, template};
    use tokio::time::sleep;

    use super::{testing::TmpDb, *};
    use crate::{
        availability::LeafQueryData,
        data_source::storage::{UpdateAvailabilityStorage, pruning::PrunedHeightStorage},
        merklized_state::{MerklizedState, UpdateStateData},
        testing::mocks::{MOCK_UPGRADE, MockHeader, MockMerkleTree, MockPayload, MockTypes},
    };

    #[template]
    #[rstest]
    #[case::postgres(DbBackend::Postgres)]
    #[case::sqlite(DbBackend::Sqlite)]
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    fn sql_backends(#[case] backend: DbBackend) {}

    #[apply(sql_backends)]
    async fn test_migrations(#[case] backend: DbBackend) {
        let db = TmpDb::init_for(backend).await;
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
    fn test_config_from_str_postgres() {
        let cfg = Config::parse("postgresql://user:password@host:8080").unwrap();
        match cfg {
            Config::Postgres(pg) => {
                assert_eq!(pg.db_opt.get_username(), "user");
                assert_eq!(pg.db_opt.get_host(), "host");
                assert_eq!(pg.db_opt.get_port(), 8080);
            },
            _ => panic!("expected Postgres config"),
        }
    }

    #[test]
    fn test_config_from_str_sqlite() {
        let cfg = Config::parse("sqlite://data.db").unwrap();
        match cfg {
            Config::Sqlite(sq) => {
                assert_eq!(sq.db_opt.get_filename().to_string_lossy(), "data.db");
            },
            _ => panic!("expected Sqlite config"),
        }
    }

    async fn vacuum(storage: &SqlStorage) {
        let sql = match storage.backend() {
            DbBackend::Sqlite => "PRAGMA incremental_vacuum(16000)",
            DbBackend::Postgres => "VACUUM",
        };
        let mut conn = storage.pool.acquire().await.unwrap();
        match &mut conn {
            BackendPoolConnection::Postgres(c) => {
                sqlx::query(sql).execute(c.as_mut()).await.unwrap();
            },
            BackendPoolConnection::Sqlite(c) => {
                sqlx::query(sql).execute(c.as_mut()).await.unwrap();
            },
        }
    }

    #[apply(sql_backends)]
    async fn test_target_period_pruning(#[case] backend: DbBackend) {
        let db = TmpDb::init_for(backend).await;
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
            tx.insert_leaf(leaf.clone()).await.unwrap();
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
        let mut tx = storage.read().await.unwrap();
        let (header_rows,): (i64,) = with_backend!(tx, |inner| {
            sqlx::query_as("select count(*) as count from header")
                .fetch_one(inner.as_mut())
                .await
        })
        .unwrap();
        // the table should be empty
        assert_eq!(header_rows, 0);

        // counting rows in leaf table.
        // Deleting rows from header table would delete rows in all the tables
        // as each of table implement "ON DELETE CASCADE" fk constraint with the header table.
        let (leaf_rows,): (i64,) = with_backend!(tx, |inner| {
            sqlx::query_as("select count(*) as count from leaf")
                .fetch_one(inner.as_mut())
                .await
        })
        .unwrap();
        drop(tx);
        // the table should be empty
        assert_eq!(leaf_rows, 0);

        assert!(
            usage_before_pruning > usage_after_pruning,
            " disk usage should decrease after pruning"
        )
    }

    #[apply(sql_backends)]
    async fn test_merklized_state_pruning(#[case] backend: DbBackend) {
        let db = TmpDb::init_for(backend).await;
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
                ["height", "hash", "payload_hash", "timestamp", "data"],
                ["height"],
                [(
                    block_height as i64,
                    format!("randomHash{block_height}"),
                    "t".to_string(),
                    0,
                    test_data,
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
        let (count,): (i64,) = with_backend!(tx, |inner| {
            sqlx::query_as(
                " SELECT count(*) FROM (SELECT count(*) as count FROM test_tree GROUP BY path \
                 having count(*) > 1) AS s",
            )
            .fetch_one(inner.as_mut())
            .await
        })
        .unwrap();

        tracing::info!("Number of nodes with multiple snapshots : {count}");
        assert!(count > 0);

        // This should delete all the nodes having height < 250 and is not the newest node with its position
        let mut tx = storage.write().await.unwrap();
        tx.delete_batch(vec!["test_tree".to_string()], 250)
            .await
            .unwrap();

        tx.commit().await.unwrap();
        let mut tx = storage.read().await.unwrap();
        let (count,): (i64,) = with_backend!(tx, |inner| {
            sqlx::query_as(
                "SELECT count(*) FROM (SELECT count(*) as count FROM test_tree GROUP BY path \
                 having count(*) > 1) AS s",
            )
            .fetch_one(inner.as_mut())
            .await
        })
        .unwrap();

        tracing::info!("Number of nodes with multiple snapshots : {count}");

        assert!(count == 0);
    }

    #[apply(sql_backends)]
    async fn test_minimum_retention_pruning(#[case] backend: DbBackend) {
        let db = TmpDb::init_for(backend).await;

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
            tx.insert_leaf(leaf.clone()).await.unwrap();
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
        let mut tx = storage.read().await.unwrap();
        let (header_rows,): (i64,) = with_backend!(tx, |inner| {
            sqlx::query_as("select count(*) as count from header")
                .fetch_one(inner.as_mut())
                .await
        })
        .unwrap();
        drop(tx);
        // the table should be empty
        assert_eq!(header_rows, 0);
    }

    #[apply(sql_backends)]
    async fn test_pruned_height_storage(#[case] backend: DbBackend) {
        let db = TmpDb::init_for(backend).await;
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

    #[apply(sql_backends)]
    async fn test_types_migration(#[case] backend: DbBackend) {
        let num_rows = 500;
        let db = TmpDb::init_for(backend).await;

        let storage = SqlStorage::connect(db.config(), StorageConnectionType::Query)
            .await
            .unwrap();

        for i in 0..num_rows {
            let view = ViewNumber::new(i);
            let validated_state = TestValidatedState::default();
            let instance_state = TestInstanceState::default();

            let (payload, metadata) = <MockPayload as BlockPayload<MockTypes>>::from_transactions(
                [],
                &validated_state,
                &instance_state,
            )
            .await
            .unwrap();

            let mut block_header = <MockHeader as BlockHeader<MockTypes>>::genesis(
                &instance_state,
                payload.clone(),
                &metadata,
                MOCK_UPGRADE.base,
            );

            block_header.block_number = i;

            let null_quorum_data = QuorumData {
                leaf_commit: Commitment::<Leaf<MockTypes>>::default_commitment_no_preimage(),
            };

            let mut qc = QuorumCertificate::new(
                null_quorum_data.clone(),
                null_quorum_data.commit(),
                view,
                None,
                std::marker::PhantomData,
            );

            let quorum_proposal = QuorumProposal {
                block_header,
                view_number: view,
                justify_qc: qc.clone(),
                upgrade_certificate: None,
                proposal_certificate: None,
            };

            let mut leaf = Leaf::from_quorum_proposal(&quorum_proposal);
            leaf.fill_block_payload(
                payload.clone(),
                GENESIS_VID_NUM_STORAGE_NODES,
                MOCK_UPGRADE.base,
            )
            .unwrap();
            qc.data.leaf_commit = <Leaf<MockTypes> as Committable>::commit(&leaf);

            let height = leaf.height() as i64;
            let hash = <Leaf<_> as Committable>::commit(&leaf).to_string();
            let header = leaf.block_header();

            let header_json = serde_json::to_value(header)
                .context("failed to serialize header")
                .unwrap();

            let payload_commitment =
                <MockHeader as BlockHeader<MockTypes>>::payload_commitment(header);
            let mut tx = storage.write().await.unwrap();

            tx.upsert(
                "header",
                ["height", "hash", "payload_hash", "data", "timestamp"],
                ["height"],
                [(
                    height,
                    leaf.block_header().commit().to_string(),
                    payload_commitment.to_string(),
                    header_json,
                    <MockHeader as BlockHeader<MockTypes>>::timestamp(leaf.block_header()) as i64,
                )],
            )
            .await
            .unwrap();

            let leaf_json = serde_json::to_value(leaf.clone()).expect("failed to serialize leaf");
            let qc_json = serde_json::to_value(qc).expect("failed to serialize QC");
            tx.upsert(
                "leaf",
                ["height", "hash", "block_hash", "leaf", "qc"],
                ["height"],
                [(
                    height,
                    hash,
                    header.commit().to_string(),
                    leaf_json,
                    qc_json,
                )],
            )
            .await
            .unwrap();

            let mut vid = advz_scheme(2);
            let disperse = vid.disperse(payload.encode()).unwrap();
            let common = disperse.common;

            let common_bytes = bincode::serialize(&common).unwrap();
            let share = disperse.shares[0].clone();
            let mut share_bytes = Some(bincode::serialize(&share).unwrap());

            // insert some nullable vid shares
            if i % 10 == 0 {
                share_bytes = None
            }

            tx.upsert(
                "vid",
                ["height", "common", "share"],
                ["height"],
                [(height, common_bytes, share_bytes)],
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
        }

        <SqlStorage as MigrateTypes<MockTypes>>::migrate_types(&storage, 50)
            .await
            .expect("failed to migrate");

        <SqlStorage as MigrateTypes<MockTypes>>::migrate_types(&storage, 50)
            .await
            .expect("failed to migrate");

        let mut tx = storage.read().await.unwrap();
        let (leaf_count,): (i64,) = with_backend!(tx, |inner| {
            sqlx::query_as("SELECT COUNT(*) from leaf2")
                .fetch_one(inner.as_mut())
                .await
        })
        .unwrap();

        let (vid_count,): (i64,) = with_backend!(tx, |inner| {
            sqlx::query_as("SELECT COUNT(*) from vid2")
                .fetch_one(inner.as_mut())
                .await
        })
        .unwrap();

        assert_eq!(leaf_count as u64, num_rows, "not all leaves migrated");
        assert_eq!(vid_count as u64, num_rows, "not all vid migrated");
    }

    #[apply(sql_backends)]
    async fn test_transaction_upsert_retries(#[case] backend: DbBackend) {
        let db = TmpDb::init_for(backend).await;
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
