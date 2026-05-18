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

//! Background data migration infrastructure.
//!
//! A [`DataBackfill`] is a migration that runs asynchronously after node startup, copying rows in
//! batches while consensus proceeds uninterrupted. Progress is persisted in the
//! `deferred_migrations` table so that a restart resumes from where it left off.

use async_trait::async_trait;

use crate::data_source::storage::sql::{Transaction, Write};

#[cfg(not(feature = "embedded-db"))]
use crate::data_source::{Transaction as _, VersionedDataSource, storage::sql::SqlStorage};

/// A background migration that copies or transforms data in batches.
#[async_trait]
pub trait DataBackfill: Send + Sync + 'static {
    /// Globally unique name, persisted in `deferred_migrations`.
    fn name(&self) -> &'static str;

    /// Names of other [`DataBackfill`] migrations that must complete before this one starts.
    /// The runner checks `deferred_migrations` for completion before proceeding.
    fn requires(&self) -> &'static [&'static str] {
        &[]
    }

    /// Number of rows to process per batch.
    fn batch_size(&self) -> usize {
        1_000
    }

    /// Process one batch starting at `offset`.
    ///
    /// Returns `Some(next_offset)` to continue, or `None` when all rows have been processed.
    /// Must be idempotent — may be called again at the same offset after a restart.
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

    /// Run all registered migrations sequentially against `db`.
    ///
    /// Designed to be passed to `tokio::spawn`; logs errors rather than propagating them so that
    /// a failing backfill does not crash the node.  Progress is persisted in the
    /// `deferred_migrations` table so restarts resume from the last committed offset.
    #[cfg(not(feature = "embedded-db"))]
    pub async fn run(self, db: SqlStorage) {
        if let Err(e) = self.validate() {
            tracing::error!("deferred migration registry is invalid, skipping all backfills: {e:#}");
            return;
        }
        for m in &self.migrations {
            Self::run_one(&db, m.as_ref()).await;
        }
    }

    /// Run a single backfill migration, resuming from the last persisted offset.
    #[cfg(not(feature = "embedded-db"))]
    async fn run_one(db: &SqlStorage, m: &dyn DataBackfill) {
        let name = m.name();

        let offset = match Self::init_or_get_offset(db, name).await {
            Ok(None) => {
                tracing::debug!(name, "deferred migration already complete");
                return;
            }
            Ok(Some(o)) => o,
            Err(e) => {
                tracing::error!(name, "failed to initialize deferred migration: {e:#}");
                return;
            }
        };

        for dep in m.requires() {
            match Self::check_complete(db, dep).await {
                Ok(true) => {}
                Ok(false) => {
                    tracing::warn!(
                        name,
                        dep,
                        "prerequisite deferred migration not complete, skipping"
                    );
                    return;
                }
                Err(e) => {
                    tracing::error!(name, dep, "failed to check prerequisite: {e:#}");
                    return;
                }
            }
        }

        tracing::info!(name, offset, "starting deferred migration");
        let mut offset = offset;

        loop {
            let mut tx = match db.write().await {
                Ok(tx) => tx,
                Err(e) => {
                    tracing::error!(name, "failed to open write transaction: {e:#}");
                    return;
                }
            };

            let next = match m.run_batch(&mut tx, offset).await {
                Ok(next) => next,
                Err(e) => {
                    tracing::error!(name, offset, "deferred migration batch failed: {e:#}");
                    return;
                }
            };

            let done = next.is_none();
            let next_offset = next.unwrap_or(offset) as i64;

            let update = sqlx::query(
                "UPDATE deferred_migrations \
                 SET last_offset = $1, \
                     completed_at = CASE WHEN $2 THEN NOW() ELSE completed_at END \
                 WHERE name = $3",
            )
            .bind(next_offset)
            .bind(done)
            .bind(name)
            .execute(tx.as_mut())
            .await;

            if let Err(e) = update {
                tracing::error!(name, "failed to persist migration progress: {e:#}");
                return;
            }

            if let Err(e) = tx.commit().await {
                tracing::error!(name, "failed to commit migration batch: {e:#}");
                return;
            }

            if done {
                tracing::info!(name, "deferred migration complete");
                break;
            }
            offset = next.unwrap() as u64;
        }
    }

    /// Insert a row into `deferred_migrations` if one does not already exist, then return the
    /// current offset.  Returns `None` if the migration is already marked complete.
    #[cfg(not(feature = "embedded-db"))]
    async fn init_or_get_offset(db: &SqlStorage, name: &str) -> anyhow::Result<Option<u64>> {
        let mut tx = db.write().await?;

        sqlx::query(
            "INSERT INTO deferred_migrations (name, started_at, last_offset) \
             VALUES ($1, NOW(), 0) ON CONFLICT (name) DO NOTHING",
        )
        .bind(name)
        .execute(tx.as_mut())
        .await?;

        let (completed, last_offset): (bool, i64) = sqlx::query_as(
            "SELECT completed_at IS NOT NULL, COALESCE(last_offset, 0) \
             FROM deferred_migrations WHERE name = $1",
        )
        .bind(name)
        .fetch_one(tx.as_mut())
        .await?;

        tx.commit().await?;

        if completed {
            Ok(None)
        } else {
            Ok(Some(last_offset as u64))
        }
    }

    /// Return `true` iff the named migration exists in `deferred_migrations` and is complete.
    #[cfg(not(feature = "embedded-db"))]
    async fn check_complete(db: &SqlStorage, name: &str) -> anyhow::Result<bool> {
        let mut tx = db.read().await?;
        let row: Option<(bool,)> = sqlx::query_as(
            "SELECT completed_at IS NOT NULL FROM deferred_migrations WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(tx.as_mut())
        .await?;
        Ok(row.map(|(b,)| b).unwrap_or(false))
    }
}

impl Default for MigrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}
