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
}

impl Default for MigrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}
