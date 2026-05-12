//! Deferred migration: widen `hash.id` and the `hash_id` foreign-key columns in all
//! merkle-tree tables from INT (i32) to BIGINT (i64).
//!
//! The expand SQL migration (`V1302__hash_id_bigint_expand.sql`) adds the new
//! `id_big` / `hash_id_big` columns and replaces the exhausted `hash_id_seq` with
//! a placeholder sequence. This module provides the four trait implementations that
//! complete the migration in the background:
//!
//! 1. [`BackfillIds`] — fills `hash.id_big = hash.id` for every existing row.
//! 2. [`BackfillRefs`] — fills `*.hash_id_big = *.hash_id::bigint` across
//!    all four merkle tree tables; starts only after [`BackfillIds`] is done.
//! 3. [`CreateIndex`] — creates `UNIQUE INDEX CONCURRENTLY` on `hash.id_big` so
//!    the cleanup can promote it to a primary key.
//! 4. [`Cleanup`] — drops the old columns, renames the new columns, and restores
//!    the primary key and FK constraints.

use async_trait::async_trait;
use hotshot_query_service::{
    data_source::storage::sql::{SqlStorage, Transaction, Write},
    migration::{
        CleanupMigration, DataBackfill, DeferredSchemaChange, DualReadAdapter, MigrationMeta,
    },
};

pub struct HashIdAdapter;

impl DualReadAdapter for HashIdAdapter {
    type View = i64;
    type Legacy = i32;
    type New = i64;

    fn view_from_legacy(legacy: i32) -> anyhow::Result<i64> {
        Ok(legacy as i64)
    }

    fn view_from_new(new: i64) -> anyhow::Result<i64> {
        Ok(new)
    }

    fn legacy_to_new(legacy: i32) -> anyhow::Result<i64> {
        Ok(legacy as i64)
    }
}

pub struct BackfillIds;

impl MigrationMeta for BackfillIds {
    fn name(&self) -> &'static str {
        "hash_id_bigint_backfill_ids"
    }

    fn order(&self) -> u32 {
        1
    }
}

#[async_trait]
impl DataBackfill for BackfillIds {
    type Adapter = HashIdAdapter;

    fn batch_size(&self) -> usize {
        10_000
    }

    async fn migrate_batch(
        &self,
        tx: &mut Transaction<Write>,
        offset: u64,
    ) -> anyhow::Result<Option<u64>> {
        // offset is the last id processed (exclusive lower bound for the next batch).
        // Positive ids are real legacy IDs; negative ids are placeholder values written
        // by new inserts during the migration window and do not need to be backfilled.
        let next: Option<i64> = sqlx::query_scalar(
            "WITH batch AS (
                SELECT id FROM hash
                WHERE id > $1 AND id > 0 AND id_big IS NULL
                ORDER BY id
                LIMIT $2
            ),
            updated AS (
                UPDATE hash SET id_big = id
                FROM batch
                WHERE hash.id = batch.id
                RETURNING hash.id
            )
            SELECT MAX(id) FROM updated",
        )
        .bind(offset as i64)
        .bind(self.batch_size() as i64)
        .fetch_one(tx.as_mut())
        .await?;

        Ok(next.map(|id| id as u64))
    }
}

pub struct BackfillRefs;

impl MigrationMeta for BackfillRefs {
    fn name(&self) -> &'static str {
        "hash_id_bigint_backfill_refs"
    }

    fn order(&self) -> u32 {
        2
    }
}

#[async_trait]
impl DataBackfill for BackfillRefs {
    type Adapter = HashIdAdapter;

    fn batch_size(&self) -> usize {
        10_000
    }

    async fn migrate_batch(
        &self,
        tx: &mut Transaction<Write>,
        offset: u64,
    ) -> anyhow::Result<Option<u64>> {
        let mut max_created: Option<i64> = None;

        for table in &[
            "fee_merkle_tree",
            "block_merkle_tree",
            "reward_merkle_tree",
            "reward_merkle_tree_v2",
        ] {
            let next: Option<i64> = sqlx::query_scalar(&format!(
                "WITH batch AS (
                    SELECT path, created FROM \"{table}\"
                    WHERE hash_id_big IS NULL AND created > $1
                    ORDER BY created
                    LIMIT $2
                ),
                updated AS (
                    UPDATE \"{table}\" t
                    SET hash_id_big = t.hash_id::bigint
                    FROM batch
                    WHERE t.path = batch.path AND t.created = batch.created
                    RETURNING t.created
                )
                SELECT MAX(created) FROM updated",
            ))
            .bind(offset as i64)
            .bind(self.batch_size() as i64)
            .fetch_one(tx.as_mut())
            .await?;

            if let Some(c) = next {
                max_created = Some(max_created.unwrap_or(i64::MIN).max(c));
            }
        }

        Ok(max_created.map(|c| c as u64))
    }
}

pub struct CreateIndex;

impl MigrationMeta for CreateIndex {
    fn name(&self) -> &'static str {
        "hash_id_bigint_create_index"
    }

    fn order(&self) -> u32 {
        1
    }
}

#[async_trait]
impl DeferredSchemaChange for CreateIndex {
    async fn run(&self, storage: &SqlStorage) -> anyhow::Result<()> {
        sqlx::query(
            "CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS hash_id_big_unique ON hash(id_big)",
        )
        .execute(&storage.pool())
        .await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Cleanup migration
// ---------------------------------------------------------------------------

/// Drops the old INT columns, promotes `id_big` / `hash_id_big` to primary key
/// and FK columns, and renames them back to `id` / `hash_id`. After this migration
/// the schema is identical to the original — just with BIGINT.
pub struct Cleanup;

impl MigrationMeta for Cleanup {
    fn name(&self) -> &'static str {
        "hash_id_bigint_cleanup"
    }

    fn order(&self) -> u32 {
        1
    }
}

#[async_trait]
impl CleanupMigration for Cleanup {
    fn requires(&self) -> &'static [&'static str] {
        &[
            "hash_id_bigint_backfill_ids",
            "hash_id_bigint_backfill_refs",
            "hash_id_bigint_create_index",
        ]
    }

    async fn run(&self, tx: &mut Transaction<Write>) -> anyhow::Result<()> {
        // Promote id_big to NOT NULL and use the concurrent index as primary key.
        sqlx::query("ALTER TABLE hash ALTER COLUMN id_big SET NOT NULL")
            .execute(tx.as_mut())
            .await?;
        sqlx::query(
            "ALTER TABLE hash ADD CONSTRAINT hash_pkey PRIMARY KEY USING INDEX hash_id_big_unique",
        )
        .execute(tx.as_mut())
        .await?;

        // Set NOT NULL on the new merkle FK columns.
        for table in &[
            "fee_merkle_tree",
            "block_merkle_tree",
            "reward_merkle_tree",
            "reward_merkle_tree_v2",
        ] {
            sqlx::query(&format!(
                "ALTER TABLE \"{table}\" ALTER COLUMN hash_id_big SET NOT NULL"
            ))
            .execute(tx.as_mut())
            .await?;
        }

        // Re-add FK constraints pointing to the new PK.
        for table in &[
            "fee_merkle_tree",
            "block_merkle_tree",
            "reward_merkle_tree",
            "reward_merkle_tree_v2",
        ] {
            sqlx::query(&format!(
                "ALTER TABLE \"{table}\" ADD CONSTRAINT {table}_hash_id_big_fkey FOREIGN KEY \
                 (hash_id_big) REFERENCES hash(id_big)"
            ))
            .execute(tx.as_mut())
            .await?;
        }

        // Drop placeholder sequence and the old id default.
        sqlx::query("ALTER TABLE hash ALTER COLUMN id DROP DEFAULT")
            .execute(tx.as_mut())
            .await?;
        sqlx::query("DROP SEQUENCE IF EXISTS hash_id_placeholder_seq")
            .execute(tx.as_mut())
            .await?;

        // Drop old INT columns.
        sqlx::query("ALTER TABLE hash DROP COLUMN id")
            .execute(tx.as_mut())
            .await?;
        for table in &[
            "fee_merkle_tree",
            "block_merkle_tree",
            "reward_merkle_tree",
            "reward_merkle_tree_v2",
        ] {
            sqlx::query(&format!("ALTER TABLE \"{table}\" DROP COLUMN hash_id"))
                .execute(tx.as_mut())
                .await?;
        }

        // Rename new columns to canonical names.
        sqlx::query("ALTER TABLE hash RENAME COLUMN id_big TO id")
            .execute(tx.as_mut())
            .await?;
        for table in &[
            "fee_merkle_tree",
            "block_merkle_tree",
            "reward_merkle_tree",
            "reward_merkle_tree_v2",
        ] {
            sqlx::query(&format!(
                "ALTER TABLE \"{table}\" RENAME COLUMN hash_id_big TO hash_id"
            ))
            .execute(tx.as_mut())
            .await?;
        }

        // Transfer sequence ownership to the renamed column.
        sqlx::query("ALTER SEQUENCE hash_id_big_seq OWNED BY hash.id")
            .execute(tx.as_mut())
            .await?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Test support
// ---------------------------------------------------------------------------

#[cfg(any(test, feature = "testing"))]
use hotshot_query_service::testing::migration::{AdapterTest, DeferredSchemaTest};

#[cfg(any(test, feature = "testing"))]
impl AdapterTest for HashIdAdapter {
    fn equivalent_pair() -> (i32, i64) {
        (1_i32, 1_i64)
    }
}

#[cfg(any(test, feature = "testing"))]
impl DeferredSchemaTest for CreateIndex {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_laws() {
        HashIdAdapter::assert_adapter_laws();
    }
}
