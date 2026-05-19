use std::time::Duration;

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
    fn log_frequency(&self) -> usize {
        10
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
        let mut seen: std::collections::HashSet<&'static str> = std::collections::HashSet::new();
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
                if !Self::run_migration(&db, m.as_ref()).await {
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

    /// Run all batches for a single migration to completion. Returns `true` if the migration
    /// finished successfully, `false` if it encountered an error.
    async fn run_migration(db: &SqlStorage, m: &dyn DataBackfill) -> bool {
        let name = m.name();

        let mut offset = match Self::init_and_get_offset(db, name).await {
            Ok(o) => o,
            Err(e) => {
                tracing::error!(name, "failed to initialize migration: {e:#}");
                return false;
            },
        };

        tracing::warn!(name, offset, "starting deferred migration");

        let mut batch_count: usize = 0;

        loop {
            let mut tx = match db.write().await {
                Ok(tx) => tx,
                Err(e) => {
                    tracing::error!(name, offset, "failed to open write transaction: {e:#}");
                    return false;
                },
            };

            let next = match m.run_batch(&mut tx, offset).await {
                Ok(next) => next,
                Err(e) => {
                    tracing::error!(name, offset, "migration batch failed: {e:#}");
                    return false;
                },
            };

            let done = next.is_none();
            let next_offset = next.unwrap_or(offset) as i64;

            if let Err(e) = sqlx::query(
                "UPDATE deferred_migrations SET last_offset = $1, completed_at = CASE WHEN $2 \
                 THEN CURRENT_TIMESTAMP ELSE completed_at END WHERE name = $3",
            )
            .bind(next_offset)
            .bind(done)
            .bind(name)
            .execute(tx.as_mut())
            .await
            {
                tracing::error!(name, "failed to persist migration progress: {e:#}");
                return false;
            }

            if let Err(e) = tx.commit().await {
                tracing::error!(name, "failed to commit migration batch: {e:#}");
                return false;
            }

            batch_count += 1;

            if done {
                tracing::warn!(name, batches = batch_count, "deferred migration complete");
                return true;
            }

            offset = next.unwrap() as u64;

            if batch_count.is_multiple_of(m.log_frequency()) {
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
