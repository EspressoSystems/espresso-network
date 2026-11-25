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

//! SQL storage backed by a PostgreSQL server.

use std::{collections::HashMap, time::Duration};

use anyhow::Context as _;
use futures::{FutureExt as _, stream::TryStreamExt};
use hotshot_types::traits::node_implementation::NodeType;
use sqlx::{
    Executor as _, Row as _,
    pool::{Pool, PoolOptions},
    postgres::{PgConnectOptions, PgRow, PgSslMode},
};

pub type Db = sqlx::Postgres;

#[path = "../sql.rs"]
mod imp;

use imp::queries::state::Node;
pub use imp::*;

use crate::{QueryError, QueryResult, include_migrations, merklized_state::MerklizedState};

/// Drop connections that come back to the pool with a transaction still open.
///
/// In sqlx 0.8.6, a `begin()` cancelled mid-flight can return its connection to the pool inside
/// a transaction that sqlx does not know about.
pub fn release_only_idle(pool: PoolOptions<sqlx::Postgres>) -> PoolOptions<sqlx::Postgres> {
    // TODO(abdul): remove once a sqlx release includes launchbadge/sqlx#4394.
    // With no transaction open, this query starts its own, so both timestamps are the same.
    // Keep `raw_sql`. A normal query is sent in several messages and Postgres moves
    // `statement_timestamp()` forward on each one, so the timestamps would always differ.
    pool.after_release(|conn, _| {
        async move {
            let row = conn
                .fetch_one(sqlx::raw_sql(
                    "SELECT transaction_timestamp() <> statement_timestamp()",
                ))
                .await?;
            let in_transaction: bool = row.try_get(0)?;
            Ok(!in_transaction)
        }
        .boxed()
    })
}

impl Backend for Db {
    type ConnectOptions = PgConnectOptions;

    const KIND: DbKind = DbKind::Postgres;
    const MAX_FN: &'static str = "GREATEST";

    fn default_connect_options() -> PgConnectOptions {
        PgConnectOptions::default()
            .username("postgres")
            .password("password")
            .host("localhost")
            .port(5432)
    }

    fn migrations() -> Vec<Migration> {
        include_migrations!("$CARGO_MANIFEST_DIR/migrations/postgres").collect()
    }

    fn delete_file(_opts: &PgConnectOptions) -> std::io::Result<()> {
        Ok(())
    }

    fn session_hooks(pool: PoolOptions<Db>, schema: String) -> PoolOptions<Db> {
        let pool = pool.after_connect(move |conn, _| {
            let schema = schema.clone();
            async move {
                query(&format!("SET search_path TO {schema}"))
                    .execute(conn)
                    .await?;
                Ok(())
            }
            .boxed()
        });
        release_only_idle(pool)
    }

    fn log_conflict_sessions(pool: Pool<Db>, op: &'static str) {
        tokio::spawn(async move {
            match sqlx::query(
                "SELECT pid, COALESCE(state, 'unknown') AS state, left(COALESCE(query, ''), 200) \
                 AS query FROM pg_stat_activity WHERE pid != pg_backend_pid() AND state IS \
                 DISTINCT FROM 'idle' AND usename = current_user",
            )
            .fetch_all(&pool)
            .await
            {
                Ok(rows) if rows.is_empty() => {
                    tracing::warn!(op, "serialization conflict: no other non-idle DB sessions");
                },
                Ok(rows) => {
                    for row in &rows {
                        let pid: i32 = row.try_get("pid").unwrap_or(-1);
                        let state: String = row.try_get("state").unwrap_or_default();
                        let query_text: String = row.try_get("query").unwrap_or_default();
                        tracing::warn!(
                            op,
                            pid,
                            state,
                            "serialization conflict: concurrent session: {query_text}",
                        );
                    }
                },
                Err(e) => {
                    tracing::error!(op, "failed to query pg_stat_activity: {e:#}");
                },
            }

            // Log SSI predicate locks held by all non-idle sessions
            match sqlx::query(
                "SELECT l.pid, l.locktype, CASE WHEN l.relation IS NOT NULL THEN c.relname ELSE \
                 NULL END AS relation, l.page, l.tuple, left(COALESCE(a.query, ''), 100) AS query \
                 FROM pg_locks l JOIN pg_stat_activity a ON a.pid = l.pid LEFT JOIN pg_class c ON \
                 c.oid = l.relation WHERE l.mode = 'SIReadLock' AND a.state IS DISTINCT FROM \
                 'idle' AND l.pid != pg_backend_pid() ORDER BY l.pid, c.relname",
            )
            .fetch_all(&pool)
            .await
            {
                Ok(rows) if rows.is_empty() => {
                    tracing::warn!(op, "serialization conflict: no SIReadLocks held");
                },
                Ok(rows) => {
                    for row in &rows {
                        let pid: i32 = row.try_get("pid").unwrap_or(-1);
                        let locktype: String = row.try_get("locktype").unwrap_or_default();
                        let relation: Option<String> = row.try_get("relation").unwrap_or(None);
                        let page: Option<i32> = row.try_get("page").unwrap_or(None);
                        let tuple: Option<i16> = row.try_get("tuple").unwrap_or(None);
                        let query_text: String = row.try_get("query").unwrap_or_default();
                        tracing::warn!(
                            op,
                            pid,
                            locktype,
                            relation,
                            page,
                            tuple,
                            "serialization conflict: SIReadLock: {query_text}",
                        );
                    }
                },
                Err(e) => {
                    tracing::warn!(op, "failed to query pg_locks: {e:#}");
                },
            }
        });
    }
}

impl StateQueries for Db {
    async fn hashes_by_id(
        conn: &mut sqlx::PgConnection,
        ids: Vec<i64>,
    ) -> QueryResult<HashMap<i64, Vec<u8>>> {
        Ok(
            sqlx::query_as("SELECT id, value FROM hash WHERE id = ANY($1::BIGINT[])")
                .bind(&ids)
                .fetch_all(conn)
                .await
                .map_err(|e| QueryError::Error {
                    message: format!("hash lookup failed: {e}"),
                })?
                .into_iter()
                .collect(),
        )
    }

    /// Batch insert hashes using UNNEST for large batches.
    async fn insert_hashes(
        tx: &mut Transaction<Write>,
        hashes: Vec<Vec<u8>>,
    ) -> QueryResult<HashMap<Vec<u8>, i64>> {
        if hashes.is_empty() {
            return Ok(HashMap::new());
        }

        // Use UNNEST-based batch insert (more efficient and avoids parameter limits).
        let sql = "INSERT INTO hash(value) SELECT * FROM UNNEST($1::bytea[]) ON CONFLICT (value) \
                   DO UPDATE SET value = EXCLUDED.value RETURNING value, id";

        let result: HashMap<Vec<u8>, i64> = sqlx::query_as(sql)
            .bind(&hashes)
            .fetch(tx.as_mut())
            .try_collect()
            .await
            .map_err(|e| QueryError::Error {
                message: format!("batch hash insert failed: {e}"),
            })?;

        Ok(result)
    }

    async fn upsert_nodes<Types, State, const ARITY: usize>(
        nodes: Vec<Node>,
        tx: &mut Transaction<Write>,
    ) -> anyhow::Result<()>
    where
        Types: NodeType,
        State: MerklizedState<Types, ARITY>,
    {
        if nodes.is_empty() {
            return Ok(());
        }
        let name = State::state_type();

        // Deduplicate nodes by (path, created) - keep the last occurrence
        // This is required because UNNEST + ON CONFLICT cannot handle duplicates in the same batch
        let mut deduped = HashMap::new();
        for node in nodes {
            deduped.insert((node.path.to_string(), node.created), node);
        }

        let mut paths = Vec::with_capacity(deduped.len());
        let mut createds = Vec::with_capacity(deduped.len());
        let mut hash_ids = Vec::with_capacity(deduped.len());
        let mut childrens = Vec::with_capacity(deduped.len());
        let mut children_bitvecs = Vec::with_capacity(deduped.len());
        let mut idxs = Vec::with_capacity(deduped.len());
        let mut entries = Vec::with_capacity(deduped.len());

        for node in deduped.into_values() {
            paths.push(node.path);
            createds.push(node.created);
            hash_ids.push(node.hash_id);
            childrens.push(node.children);
            children_bitvecs.push(node.children_bitvec);
            idxs.push(node.idx);
            entries.push(node.entry);
        }

        let sql = format!(
            r#"
            INSERT INTO "{name}" (path, created, hash_id, children, children_bitvec, idx, entry)
            SELECT * FROM UNNEST($1::jsonb[], $2::bigint[], $3::bigint[], $4::jsonb[], $5::bit varying[], $6::jsonb[], $7::jsonb[])
            ON CONFLICT (path, created) DO UPDATE SET
                hash_id = EXCLUDED.hash_id,
                children = EXCLUDED.children,
                children_bitvec = EXCLUDED.children_bitvec,
                idx = EXCLUDED.idx,
                entry = EXCLUDED.entry
            "#
        );

        sqlx::query(&sql)
            .bind(&paths)
            .bind(&createds)
            .bind(&hash_ids)
            .bind(&childrens)
            .bind(&children_bitvecs)
            .bind(&idxs)
            .bind(&entries)
            .execute(tx.as_mut())
            .await
            .context("batch upsert with UNNEST failed")?;

        Ok(())
    }

    fn node_from_row(row: PgRow) -> Node {
        Node {
            path: row.get_unchecked("path"),
            created: row.get_unchecked("created"),
            hash_id: row.get_unchecked("hash_id"),
            children: row.get_unchecked("children"),
            children_bitvec: row.get_unchecked("children_bitvec"),
            idx: row.get_unchecked("idx"),
            entry: row.get_unchecked("entry"),
        }
    }

    fn get_path_query<'q, Types, State, const ARITY: usize>(
        paths: Vec<serde_json::Value>,
        created: i64,
    ) -> QueryResult<(QueryBuilder<'q>, String)>
    where
        Types: NodeType,
        State: MerklizedState<Types, ARITY>,
    {
        let table = State::state_type();
        let mut query = QueryBuilder::default();
        let paths_param = query.bind(paths)?;
        let created_param = query.bind(created)?;
        // One LATERAL point-lookup per path in the array, instead of N UNION'd subqueries.
        // Postgres plans the inner subquery once and reuses it, collapsing N independent
        // planner decisions into one.
        let sql = format!(
            "SELECT n.* FROM unnest({paths_param}::jsonb[]) AS p(path), LATERAL (SELECT * FROM \
             {table} WHERE {table}.path = p.path AND {table}.created <= {created_param} ORDER BY \
             {table}.path DESC, {table}.created DESC LIMIT 1) AS n ORDER BY n.path DESC"
        );
        Ok((query, sql))
    }
}

impl From<PgConnectOptions> for Config {
    fn from(db_opt: PgConnectOptions) -> Self {
        Self::with_connect_options(db_opt)
    }
}

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

    pub fn connect_options(&self) -> &PgConnectOptions {
        &self.db_opt
    }

    /// Set the name of the schema to use for queries.
    ///
    /// The default schema is named `hotshot` and is created via the default migrations.
    pub fn schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = schema.into();
        self
    }

    /// Set the maximum time a single SQL statement is allowed to run before being canceled.
    ///
    /// This helps prevent queries from running indefinitely even when the client is dropped
    pub fn statement_timeout(mut self, timeout: Duration) -> Self {
        // Format duration as milliseconds
        // PostgreSQL interprets values without units as milliseconds
        let timeout_ms = timeout.as_millis();
        self.db_opt = self
            .db_opt
            .options([("statement_timeout", timeout_ms.to_string())]);
        self
    }
}

#[cfg(all(any(test, feature = "testing"), not(target_os = "windows")))]
pub(super) mod tmp_db {
    use std::{
        env,
        process::{Child, Command, Stdio},
        time::Duration,
    };

    use test_utils::reserve_tcp_port;
    use tokio::time::timeout;

    use super::{Config, testing::test_migrations};
    use crate::testing::sleep;

    #[derive(Debug)]
    pub struct TmpDb {
        postgres: TmpPostgres,
    }

    impl TmpDb {
        pub async fn init() -> Self {
            Self {
                postgres: TmpPostgres::init().await,
            }
        }

        pub fn host(&self) -> String {
            self.postgres.host()
        }

        pub fn port(&self) -> u16 {
            self.postgres.port()
        }

        pub fn config(&self) -> Config {
            Config::default()
                .user("postgres")
                .password("password")
                .host(self.host())
                .port(self.port())
                .migrations(test_migrations())
        }
    }

    /// A throwaway Postgres server with user `postgres` and trust auth, removed on drop.
    ///
    /// Available whichever database the query service is built for, for crates that keep data of
    /// their own in Postgres.
    #[derive(Debug)]
    pub struct TmpPostgres {
        host: String,
        port: u16,
        data_dir: std::path::PathBuf,
        postgres: Option<Child>,
    }

    impl TmpPostgres {
        pub async fn init() -> Self {
            let port = reserve_tcp_port().unwrap();
            let host = "127.0.0.1".to_string();

            // initdb requires an empty target; clear any dir left by a crashed run
            // that reused this (recycled) ephemeral port.
            let data_dir = env::temp_dir().join(format!("espresso-tmpdb-{port}"));
            let _ = std::fs::remove_dir_all(&data_dir);

            let output = Command::new("initdb")
                .arg("-D")
                .arg(&data_dir)
                .args(["-U", "postgres", "--auth=trust"])
                .output()
                .expect("initdb failed to run; is postgres installed and on PATH?");
            assert!(
                output.status.success(),
                "initdb failed for {data_dir:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            let mut db = Self {
                host,
                port,
                data_dir,
                postgres: None,
            };

            db.start().await;
            db
        }

        pub fn host(&self) -> String {
            self.host.clone()
        }

        pub fn port(&self) -> u16 {
            self.port
        }

        fn stop(&mut self) {
            let Some(mut postgres) = self.postgres.take() else {
                return;
            };
            tracing::info!(port = self.port, "stopping postgres");
            // Fast shutdown (SIGINT to the postmaster) releases the datadir before
            // Drop removes it; fall back to SIGKILL if pg_ctl is unavailable.
            let stopped = Command::new("pg_ctl")
                .arg("-D")
                .arg(&self.data_dir)
                .args(["stop", "-m", "fast", "-w"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|status| status.success())
                .unwrap_or(false);
            if !stopped {
                let _ = postgres.kill();
            }
            let _ = postgres.wait();
        }

        async fn start(&mut self) {
            tracing::info!(port = self.port, "starting postgres");
            let postgres = Command::new("postgres")
                .arg("-D")
                .arg(&self.data_dir)
                .args(["-p", &self.port.to_string()])
                .args(["-h", &self.host])
                // Keep the unix socket inside the data dir instead of a shared /tmp.
                .arg("-k")
                .arg(&self.data_dir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("failed to start postgres; is it installed and on PATH?");
            self.postgres = Some(postgres);

            self.wait_for_ready().await;
        }

        async fn wait_for_ready(&self) {
            let timeout_duration = Duration::from_secs(
                env::var("SQL_TMP_DB_CONNECT_TIMEOUT")
                    .unwrap_or("60".to_string())
                    .parse()
                    .expect("SQL_TMP_DB_CONNECT_TIMEOUT must be an integer number of seconds"),
            );

            if let Err(err) = timeout(timeout_duration, async {
                while !Command::new("pg_isready")
                    .args(["-h", &self.host])
                    .args(["-p", &self.port.to_string()])
                    .args(["-U", "postgres"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map(|status| status.success())
                    .unwrap_or(false)
                {
                    tracing::warn!("database is not ready");
                    sleep(Duration::from_secs(1)).await;
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

    impl Drop for TmpPostgres {
        fn drop(&mut self) {
            self.stop();
            let _ = std::fs::remove_dir_all(&self.data_dir);
        }
    }
}

#[cfg(all(test, not(target_os = "windows")))]
mod test {
    use std::str::FromStr;

    use futures::FutureExt as _;
    use sqlx::{Connection as _, Executor as _, Row as _};

    use super::{Config, SqlStorage, testing::TmpDb};
    use crate::data_source::storage::StorageConnectionType;

    #[test]
    fn test_config_from_str() {
        let cfg = Config::from_str("postgresql://user:password@host:8080").unwrap();
        assert_eq!(cfg.db_opt.get_username(), "user");
        assert_eq!(cfg.db_opt.get_host(), "host");
        assert_eq!(cfg.db_opt.get_port(), 8080);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_cancelled_begin_does_not_leak_transaction() {
        let db = TmpDb::init().await;
        let storage = SqlStorage::connect(
            db.config().max_connections(1),
            StorageConnectionType::Sequencer,
        )
        .await
        .unwrap();

        let mut conn = storage.pool().acquire().await.unwrap();
        assert!(
            conn.begin().now_or_never().is_none(),
            "BEGIN finished on the first poll, so nothing was cancelled"
        );
        drop(conn);

        let mut conn = storage.pool().acquire().await.unwrap();
        let row = conn
            .fetch_one(sqlx::raw_sql(
                "SELECT transaction_timestamp() <> statement_timestamp()",
            ))
            .await
            .unwrap();
        assert!(
            !row.get::<bool, _>(0),
            "the connection came back from the pool inside the cancelled transaction"
        );
    }
}
