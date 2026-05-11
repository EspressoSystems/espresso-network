//! Deferred migration: widen `hash.id` and the `hash_id` foreign-key columns in all
//! merkle-tree tables from INT (i32) to BIGINT (i64).
//!
//! The expand SQL migration (`V1302__hash_id_bigint_expand.sql`) adds the new
//! `id_big` / `hash_id_big` columns and replaces the exhausted `hash_id_seq` with
//! a sentinel sequence. This module provides the three trait implementations that
//! complete the migration in the background:
//!
//! 1. [`HashTableBackfill`] — fills `hash.id_big = hash.id` for every existing row.
//! 2. [`MerkleHashIdBackfill`] — fills `*.hash_id_big = *.hash_id::bigint` across
//!    all four merkle tree tables; starts only after `HashTableBackfill` is done.
//! 3. [`IndexHashIdBig`] — creates `UNIQUE INDEX CONCURRENTLY` on `hash.id_big` so
//!    the cleanup can promote it to a primary key.
//! 4. [`RestoreHashConstraints`] — cleanup migration that drops the old columns,
//!    renames the new columns, and restores the primary key and FK constraints.

use async_trait::async_trait;
use hotshot_query_service::{
    data_source::storage::sql::{SqlStorage, Transaction, Write},
    migration::{
        CleanupMigration, DataBackfill, DeferredSchemaChange, DualReadAdapter, MigrationMeta,
    },
};

// ---------------------------------------------------------------------------
// DualReadAdapter
// ---------------------------------------------------------------------------

/// Converts between the old INT hash ID (i32) and the new BIGINT hash ID (i64).
pub struct HashIdAdapter;

impl DualReadAdapter for HashIdAdapter {
    type View = i64;
    type Legacy = i32;
    type New = i64;

    fn view_from_legacy(legacy: i32) -> i64 {
        legacy as i64
    }

    fn view_from_new(new: i64) -> i64 {
        new
    }

    fn legacy_to_new(legacy: i32) -> i64 {
        legacy as i64
    }
}

// ---------------------------------------------------------------------------
// Backfill 1: hash table
// ---------------------------------------------------------------------------

/// Fills `hash.id_big = hash.id` for all existing rows.
pub struct HashTableBackfill;

impl MigrationMeta for HashTableBackfill {
    fn name(&self) -> &'static str {
        "hash_id_bigint_hash_table"
    }

    fn order(&self) -> u32 {
        1
    }
}

#[async_trait]
impl DataBackfill for HashTableBackfill {
    type Adapter = HashIdAdapter;

    fn batch_size(&self) -> usize {
        10_000
    }

    async fn migrate_batch(
        &self,
        tx: &mut Transaction<Write>,
        _offset: u64,
    ) -> anyhow::Result<Option<u64>> {
        // Update rows where id_big is still NULL (i.e. old rows that haven't been
        // backfilled yet). Positive id values are the real legacy IDs; negative values
        // are sentinel placeholders written by new inserts during the migration window
        // and do not need to be copied into id_big.
        let n: u64 = sqlx::query_scalar(
            "WITH batch AS (
                SELECT id FROM hash
                WHERE id_big IS NULL AND id > 0
                ORDER BY id
                LIMIT $1
            )
            UPDATE hash SET id_big = hash.id
            FROM batch
            WHERE hash.id = batch.id",
        )
        .bind(self.batch_size() as i64)
        .fetch_one(tx.as_mut())
        .await
        .map(|n: i64| n as u64)?;

        if n == 0 { Ok(None) } else { Ok(Some(n)) }
    }
}

// ---------------------------------------------------------------------------
// Backfill 2: merkle tree tables
// ---------------------------------------------------------------------------

/// Fills `hash_id_big = hash_id::bigint` for all existing rows in the four merkle
/// tree tables. Must run after [`HashTableBackfill`] completes.
pub struct MerkleHashIdBackfill;

impl MigrationMeta for MerkleHashIdBackfill {
    fn name(&self) -> &'static str {
        "hash_id_bigint_merkle_tables"
    }

    fn order(&self) -> u32 {
        2
    }
}

#[async_trait]
impl DataBackfill for MerkleHashIdBackfill {
    type Adapter = HashIdAdapter;

    fn batch_size(&self) -> usize {
        10_000
    }

    async fn migrate_batch(
        &self,
        tx: &mut Transaction<Write>,
        _offset: u64,
    ) -> anyhow::Result<Option<u64>> {
        let mut total: i64 = 0;

        for table in &[
            "fee_merkle_tree",
            "block_merkle_tree",
            "reward_merkle_tree",
            "reward_merkle_tree_v2",
        ] {
            // Use a CTE to limit each table to batch_size rows per call.
            let n: i64 = sqlx::query_scalar(&format!(
                "WITH batch AS (
                    SELECT path, created FROM \"{table}\"
                    WHERE hash_id_big IS NULL
                    LIMIT $1
                )
                UPDATE \"{table}\" t
                SET hash_id_big = t.hash_id::bigint
                FROM batch
                WHERE t.path = batch.path AND t.created = batch.created",
            ))
            .bind(self.batch_size() as i64)
            .fetch_one(tx.as_mut())
            .await?;

            total += n;
        }

        if total == 0 {
            Ok(None)
        } else {
            Ok(Some(total as u64))
        }
    }
}

// ---------------------------------------------------------------------------
// Deferred schema change: concurrent unique index
// ---------------------------------------------------------------------------

/// Creates `UNIQUE INDEX CONCURRENTLY` on `hash.id_big` so the cleanup can
/// promote it to a primary key with a zero-downtime `ADD PRIMARY KEY USING INDEX`.
pub struct IndexHashIdBig;

impl MigrationMeta for IndexHashIdBig {
    fn name(&self) -> &'static str {
        "hash_id_bigint_index"
    }

    fn order(&self) -> u32 {
        1
    }
}

#[async_trait]
impl DeferredSchemaChange for IndexHashIdBig {
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
pub struct RestoreHashConstraints;

impl MigrationMeta for RestoreHashConstraints {
    fn name(&self) -> &'static str {
        "hash_id_bigint_cleanup"
    }

    fn order(&self) -> u32 {
        1
    }
}

#[async_trait]
impl CleanupMigration for RestoreHashConstraints {
    fn requires(&self) -> &'static [&'static str] {
        &[
            "hash_id_bigint_hash_table",
            "hash_id_bigint_merkle_tables",
            "hash_id_bigint_index",
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

        // Drop sentinel sequence and the old id default.
        sqlx::query("ALTER TABLE hash ALTER COLUMN id DROP DEFAULT")
            .execute(tx.as_mut())
            .await?;
        sqlx::query("DROP SEQUENCE IF EXISTS hash_id_sentinel_seq")
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
impl DeferredSchemaTest for IndexHashIdBig {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_laws() {
        HashIdAdapter::assert_adapter_laws();
    }
}
