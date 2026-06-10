use std::{collections::HashSet, env, time::Duration};

use async_trait::async_trait;

use crate::data_source::{
    Transaction as _, VersionedDataSource,
    storage::sql::{SqlStorage, Transaction, Write},
};

const RETRY_INTERVAL: Duration = Duration::from_secs(300);

/// A background migration that copies or transforms data in batches.
#[async_trait]
pub trait DataBackfill: Send + Sync + 'static {
    fn name(&self) -> &'static str;

    /// Names of other [`DataBackfill`] migrations that must complete before this one starts.
    fn requires(&self) -> &'static [&'static str] {
        &[]
    }

    /// Number of rows to process per batch.
    fn batch_size(&self) -> usize {
        1_000
    }

    /// Number of batches between progress log lines.
    fn log_interval(&self) -> usize {
        10
    }

    /// How long to sleep between batches to avoid saturating the database.
    fn batch_delay(&self) -> Duration {
        Duration::from_millis(50)
    }

    /// Process one batch starting at `offset`.
    ///
    /// Returns `Some(next_offset)` to continue, or `None` when all rows have been processed.
    async fn run_batch(
        &self,
        tx: &mut Transaction<Write>,
        offset: u64,
    ) -> anyhow::Result<Option<u64>>;
}

/// An ordered registry of [`DataBackfill`] migrations.
pub struct MigrationRegistry {
    migrations: Vec<Box<dyn DataBackfill>>,
}

impl MigrationRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
        }
    }

    /// Append a migration. Migrations run in the order they are added.
    pub fn backfill(mut self, m: impl DataBackfill) -> Self {
        self.migrations.push(Box::new(m));
        self
    }

    /// Validate the registry.
    ///
    /// Checks that:
    /// - All migration names are unique.
    /// - Every name listed in `requires()` refers to a migration that appears earlier in the list.
    pub fn validate(&self) -> anyhow::Result<()> {
        let mut seen: HashSet<&'static str> = HashSet::new();
        for m in &self.migrations {
            let name = m.name();
            anyhow::ensure!(seen.insert(name), "duplicate migration name: {name}");
            for dep in m.requires() {
                anyhow::ensure!(
                    seen.contains(dep),
                    "migration {name} requires {dep} which either does not exist or appears later \
                     in the registry",
                );
            }
        }
        Ok(())
    }

    /// Iterate over the registered migrations in order.
    pub fn iter(&self) -> impl Iterator<Item = &dyn DataBackfill> {
        self.migrations.iter().map(|b| b.as_ref())
    }

    /// Drive all registered migrations to completion.
    ///
    /// Iterates over all migrations on each pass. Migrations whose prerequisites are not yet
    /// complete are skipped with a warning and retried after [`RETRY_INTERVAL`]. The loop exits
    /// once every migration has been marked complete.
    pub async fn run_all_migrations(self, db: SqlStorage) {
        if let Err(e) = self.validate() {
            tracing::error!(
                "deferred migration registry is invalid, skipping all backfills: {e:#}"
            );
            return;
        }

        loop {
            let mut pending = 0usize;

            for m in &self.migrations {
                let name = m.name();

                // Skip migrations that have already been marked complete.
                match Self::is_complete(&db, name).await {
                    Ok(true) => continue,
                    Ok(false) => {},
                    Err(e) => {
                        tracing::error!(name, "failed to check migration status: {e:#}");
                        pending += 1;
                        continue;
                    },
                }

                // Defer if any prerequisite is not yet complete.
                let mut deps_ready = true;
                for dep in m.requires() {
                    match Self::is_complete(&db, dep).await {
                        Ok(true) => {},
                        Ok(false) => {
                            tracing::warn!(
                                name,
                                dep,
                                "prerequisite not yet complete, will retry after \
                                 {RETRY_INTERVAL:?}"
                            );
                            deps_ready = false;
                        },
                        Err(e) => {
                            tracing::error!(name, dep, "failed to check prerequisite: {e:#}");
                            deps_ready = false;
                        },
                    }
                }
                if !deps_ready {
                    pending += 1;
                    continue;
                }

                // Run all batches for this migration.
                if let Err(e) = Self::run_migration(&db, m.as_ref()).await {
                    tracing::error!(name, "deferred migration failed: {e:#}");
                    pending += 1;
                }
            }

            if pending == 0 {
                tracing::warn!("all deferred migrations complete");
                break;
            }

            tokio::time::sleep(RETRY_INTERVAL).await;
        }
    }

    /// Run all batches for a single migration to completion.
    async fn run_migration(db: &SqlStorage, m: &dyn DataBackfill) -> anyhow::Result<()> {
        use anyhow::Context as _;

        let name = m.name();

        let mut offset = Self::init_and_get_offset(db, name)
            .await
            .context("failed to initialize migration")?;

        let delay = env::var("ESPRESSO_NODE_BACKFILL_BATCH_DELAY_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_millis)
            .unwrap_or_else(|| m.batch_delay());

        tracing::warn!(name, offset, ?delay, "starting deferred migration");

        let mut batch_count: usize = 0;

        loop {
            let mut tx = db
                .write()
                .await
                .with_context(|| format!("failed to open write transaction at offset {offset}"))?;

            let next = m
                .run_batch(&mut tx, offset)
                .await
                .with_context(|| format!("migration batch failed at offset {offset}"))?;

            sqlx::query(
                "UPDATE deferred_migrations SET last_offset = $1, completed_at = CASE WHEN $2 \
                 THEN CURRENT_TIMESTAMP ELSE completed_at END WHERE name = $3",
            )
            .bind(next.unwrap_or(offset) as i64)
            .bind(next.is_none())
            .bind(name)
            .execute(tx.as_mut())
            .await
            .context("failed to persist migration progress")?;

            tx.commit()
                .await
                .context("failed to commit migration batch")?;

            batch_count += 1;

            let Some(next_offset) = next else {
                tracing::warn!(name, batches = batch_count, "deferred migration complete");
                return Ok(());
            };
            offset = next_offset;

            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }

            if batch_count.is_multiple_of(m.log_interval()) {
                tracing::warn!(
                    name,
                    offset,
                    batches = batch_count,
                    "deferred migration progress"
                );
            }
        }
    }

    /// Returns `true` if the named migration has been marked complete in the database.
    /// Returns `false` if the row does not exist or `completed_at` is null.
    async fn is_complete(db: &SqlStorage, name: &str) -> anyhow::Result<bool> {
        let mut tx = db.read().await?;
        let row: Option<(bool,)> = sqlx::query_as(
            "SELECT completed_at IS NOT NULL FROM deferred_migrations WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(tx.as_mut())
        .await?;
        Ok(row.map(|(b,)| b).unwrap_or(false))
    }

    /// Insert a tracking row for `name` if one does not yet exist, then return the stored offset
    /// to resume from.
    async fn init_and_get_offset(db: &SqlStorage, name: &str) -> anyhow::Result<u64> {
        let mut tx = db.write().await?;

        sqlx::query(
            "INSERT INTO deferred_migrations (name, started_at, last_offset) VALUES ($1, \
             CURRENT_TIMESTAMP, 0) ON CONFLICT (name) DO NOTHING",
        )
        .bind(name)
        .execute(tx.as_mut())
        .await?;

        let (last_offset,): (i64,) = sqlx::query_as(
            "SELECT COALESCE(last_offset, 0) FROM deferred_migrations WHERE name = $1",
        )
        .bind(name)
        .fetch_one(tx.as_mut())
        .await?;

        tx.commit().await?;

        Ok(last_offset as u64)
    }
}

impl Default for MigrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A minimal DataBackfill that runs `total` batches (one unit of work each) then stops.
    struct CountBatches {
        name: &'static str,
        total: u64,
        deps: &'static [&'static str],
    }

    impl CountBatches {
        fn new(name: &'static str, total: u64) -> Self {
            Self {
                name,
                total,
                deps: &[],
            }
        }

        fn with_deps(name: &'static str, total: u64, deps: &'static [&'static str]) -> Self {
            Self { name, total, deps }
        }
    }

    #[async_trait]
    impl DataBackfill for CountBatches {
        fn name(&self) -> &'static str {
            self.name
        }

        fn requires(&self) -> &'static [&'static str] {
            self.deps
        }

        fn batch_size(&self) -> usize {
            1
        }

        async fn run_batch(
            &self,
            _tx: &mut Transaction<Write>,
            offset: u64,
        ) -> anyhow::Result<Option<u64>> {
            Ok((offset < self.total).then_some(offset + 1))
        }
    }

    // --- validate() unit tests (no database required) ---

    #[test]
    fn validate_empty() {
        MigrationRegistry::new().validate().unwrap();
    }

    #[test]
    fn validate_single() {
        MigrationRegistry::new()
            .backfill(CountBatches::new("a", 1))
            .validate()
            .unwrap();
    }

    #[test]
    fn validate_duplicate_name() {
        let err = MigrationRegistry::new()
            .backfill(CountBatches::new("foo", 1))
            .backfill(CountBatches::new("foo", 1))
            .validate()
            .unwrap_err();
        assert!(err.to_string().contains("duplicate"));
    }

    #[test]
    fn validate_unknown_dep() {
        let err = MigrationRegistry::new()
            .backfill(CountBatches::with_deps("bar", 1, &["nonexistent"]))
            .validate()
            .unwrap_err();
        assert!(err.to_string().contains("nonexistent"));
    }

    #[test]
    fn validate_dep_must_precede() {
        // "a" depends on "b" but "b" comes after "a" in the registry.
        let err = MigrationRegistry::new()
            .backfill(CountBatches::with_deps("a", 1, &["b"]))
            .backfill(CountBatches::new("b", 1))
            .validate()
            .unwrap_err();
        assert!(err.to_string().contains("b"));
    }

    #[test]
    fn validate_valid_dependency_chain() {
        MigrationRegistry::new()
            .backfill(CountBatches::new("a", 1))
            .backfill(CountBatches::with_deps("b", 1, &["a"]))
            .backfill(CountBatches::with_deps("c", 1, &["a", "b"]))
            .validate()
            .unwrap();
    }

    // --- Integration tests (require a real postgres database) ---

    #[cfg(not(feature = "embedded-db"))]
    mod db_tests {
        use super::*;
        use crate::data_source::storage::sql::{
            Migration, SqlStorage, StorageConnectionType, testing::TmpDb,
        };

        // The deferred_migrations table lives in espresso-node's migrations; for tests within
        // this crate we inject it as an extra migration.
        const CREATE_DEFERRED_MIGRATIONS: &str = "
            CREATE TABLE IF NOT EXISTS deferred_migrations (
                name         TEXT        PRIMARY KEY,
                started_at   TIMESTAMPTZ NOT NULL,
                completed_at TIMESTAMPTZ,
                last_offset  BIGINT
            );
        ";

        async fn setup() -> (TmpDb, SqlStorage) {
            let db = TmpDb::init().await;
            let storage = SqlStorage::connect(
                db.config().migrations(vec![
                    Migration::unapplied(
                        "V9990__deferred_migrations.sql",
                        CREATE_DEFERRED_MIGRATIONS,
                    )
                    .unwrap(),
                ]),
                StorageConnectionType::Query,
            )
            .await
            .unwrap();
            (db, storage)
        }

        async fn is_complete(storage: &SqlStorage, name: &str) -> bool {
            let mut tx = storage.read().await.unwrap();
            let row: Option<(bool,)> = sqlx::query_as(
                "SELECT completed_at IS NOT NULL FROM deferred_migrations WHERE name = $1",
            )
            .bind(name)
            .fetch_optional(tx.as_mut())
            .await
            .unwrap();
            row.map(|(b,)| b).unwrap_or(false)
        }

        async fn last_offset(storage: &SqlStorage, name: &str) -> Option<i64> {
            let mut tx = storage.read().await.unwrap();
            let row: Option<(i64,)> =
                sqlx::query_as("SELECT last_offset FROM deferred_migrations WHERE name = $1")
                    .bind(name)
                    .fetch_optional(tx.as_mut())
                    .await
                    .unwrap();
            row.map(|(o,)| o)
        }

        #[test_log::test(tokio::test(flavor = "multi_thread"))]
        async fn migration_runs_to_completion() {
            let (_db, storage) = setup().await;

            MigrationRegistry::new()
                .backfill(CountBatches::new("m", 5))
                .run_all_migrations(storage.clone())
                .await;

            assert!(is_complete(&storage, "m").await);
            assert_eq!(last_offset(&storage, "m").await, Some(5));
        }

        #[test_log::test(tokio::test(flavor = "multi_thread"))]
        async fn migration_resumes_from_checkpoint() {
            let (_db, storage) = setup().await;

            // Seed partial progress: already at offset 3 of 5.
            let mut tx = storage.write().await.unwrap();
            sqlx::query(
                "INSERT INTO deferred_migrations (name, started_at, last_offset) VALUES ($1, \
                 CURRENT_TIMESTAMP, $2)",
            )
            .bind("m")
            .bind(3i64)
            .execute(tx.as_mut())
            .await
            .unwrap();
            tx.commit().await.unwrap();

            MigrationRegistry::new()
                .backfill(CountBatches::new("m", 5))
                .run_all_migrations(storage.clone())
                .await;

            assert!(is_complete(&storage, "m").await);
            assert_eq!(last_offset(&storage, "m").await, Some(5));
        }

        #[test_log::test(tokio::test(flavor = "multi_thread"))]
        async fn completed_migration_is_skipped() {
            let (_db, storage) = setup().await;

            // Pre-mark the migration as complete at offset 99.
            let mut tx = storage.write().await.unwrap();
            sqlx::query(
                "INSERT INTO deferred_migrations (name, started_at, completed_at, last_offset) \
                 VALUES ($1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $2)",
            )
            .bind("m")
            .bind(99i64)
            .execute(tx.as_mut())
            .await
            .unwrap();
            tx.commit().await.unwrap();

            // Run a migration with total=0 — if it ran, it would reset the offset.
            MigrationRegistry::new()
                .backfill(CountBatches::new("m", 0))
                .run_all_migrations(storage.clone())
                .await;

            // Offset must not have changed.
            assert_eq!(last_offset(&storage, "m").await, Some(99));
        }

        #[test_log::test(tokio::test(flavor = "multi_thread"))]
        async fn dependency_ordering_respected() {
            let (_db, storage) = setup().await;

            MigrationRegistry::new()
                .backfill(CountBatches::new("first", 3))
                .backfill(CountBatches::with_deps("second", 3, &["first"]))
                .run_all_migrations(storage.clone())
                .await;

            assert!(is_complete(&storage, "first").await);
            assert!(is_complete(&storage, "second").await);
        }
    }
}
