#![cfg(not(feature = "embedded-db"))]

use async_trait::async_trait;
use hotshot_query_service::{
    data_source::storage::sql::{Transaction, Write},
    migration::{DataBackfill, MigrationRegistry},
};

pub struct BackfillHash;

#[async_trait]
impl DataBackfill for BackfillHash {
    fn name(&self) -> &'static str {
        "hash_bigint_backfill_hash"
    }

    async fn run_batch(
        &self,
        tx: &mut Transaction<Write>,
        // Reused as last-seen id (keyset cursor), not a row count.
        // Initial value 0 is safe because auto-increment ids start at 1.
        offset: u64,
    ) -> anyhow::Result<Option<u64>> {
        let rows: Vec<(i32, Vec<u8>)> =
            sqlx::query_as("SELECT id, value FROM hash WHERE id > $1 ORDER BY id LIMIT $2")
                .bind(offset as i64)
                .bind(self.batch_size() as i64)
                .fetch_all(tx.as_mut())
                .await?;

        if rows.is_empty() {
            return Ok(None);
        }
        let n = rows.len();
        let last_id = rows.last().expect("non-empty").0 as u64;

        let (ids, values): (Vec<i64>, Vec<Vec<u8>>) = rows
            .into_iter()
            .map(|(id, value)| (id as i64, value))
            .unzip();

        sqlx::query(
            "INSERT INTO hash_bigint (id, value)
             SELECT * FROM UNNEST($1::bigint[], $2::bytea[])
             ON CONFLICT DO NOTHING",
        )
        .bind(&ids)
        .bind(&values)
        .execute(tx.as_mut())
        .await?;

        if n < self.batch_size() {
            Ok(None)
        } else {
            Ok(Some(last_id))
        }
    }
}

macro_rules! merkle_tree_backfill {
    ($struct_name:ident, $migration_name:literal, $legacy_table:literal, $new_table:literal) => {
        pub struct $struct_name;

        #[async_trait]
        impl DataBackfill for $struct_name {
            fn name(&self) -> &'static str {
                $migration_name
            }

            fn requires(&self) -> &'static [&'static str] {
                &["hash_bigint_backfill_hash"]
            }

            async fn run_batch(
                &self,
                tx: &mut Transaction<Write>,
                // Cursor: the start of the `created` (block height) range for this batch.
                // Each batch covers [offset, offset + batch_size), using the index on `created`.
                offset: u64,
            ) -> anyhow::Result<Option<u64>> {
                let batch_size = self.batch_size() as i64;
                let rows: Vec<(
                    serde_json::Value,
                    i64,
                    i64,
                    Option<serde_json::Value>,
                    Option<sqlx::types::BitVec>,
                    Option<serde_json::Value>,
                    Option<serde_json::Value>,
                )> = sqlx::query_as(concat!(
                    "SELECT path, created, hash_id::BIGINT, children, children_bitvec, idx, entry \
                     FROM ",
                    $legacy_table,
                    " WHERE created >= $1 AND created < $2 ORDER BY created, path"
                ))
                .bind(offset as i64)
                .bind(offset as i64 + batch_size)
                .fetch_all(tx.as_mut())
                .await?;

                if rows.is_empty() {
                    return Ok(None);
                }
                let n = rows.len();

                let mut paths = Vec::with_capacity(n);
                let mut createds = Vec::with_capacity(n);
                let mut hash_ids = Vec::with_capacity(n);
                let mut childrens = Vec::with_capacity(n);
                let mut children_bitvecs = Vec::with_capacity(n);
                let mut idxs = Vec::with_capacity(n);
                let mut entries = Vec::with_capacity(n);

                for (path, created, hash_id, children, children_bitvec, idx, entry) in rows {
                    paths.push(path);
                    createds.push(created);
                    hash_ids.push(hash_id);
                    childrens.push(children);
                    children_bitvecs.push(children_bitvec);
                    idxs.push(idx);
                    entries.push(entry);
                }

                sqlx::query(concat!(
                    "INSERT INTO ",
                    $new_table,
                    " (path, created, hash_id, children, children_bitvec, idx, entry)
                     SELECT * FROM UNNEST($1::jsonb[], $2::bigint[], $3::bigint[], \
                     $4::jsonb[], $5::bit varying[], $6::jsonb[], $7::jsonb[])
                     ON CONFLICT DO NOTHING"
                ))
                .bind(&paths)
                .bind(&createds)
                .bind(&hash_ids)
                .bind(&childrens)
                .bind(&children_bitvecs)
                .bind(&idxs)
                .bind(&entries)
                .execute(tx.as_mut())
                .await?;

                // Delete moved rows from the legacy table in the same transaction so that
                // storage stays roughly flat during the migration (move, not copy).
                sqlx::query(concat!(
                    "DELETE FROM ",
                    $legacy_table,
                    " WHERE created >= $1 AND created < $2"
                ))
                .bind(offset as i64)
                .bind(offset as i64 + batch_size)
                .execute(tx.as_mut())
                .await?;

                Ok(Some(offset + self.batch_size() as u64))
            }
        }
    };
}

merkle_tree_backfill!(
    BackfillFeeMerkleTree,
    "hash_bigint_backfill_fee_merkle_tree",
    "fee_merkle_tree",
    "fee_merkle_tree_bigint"
);
merkle_tree_backfill!(
    BackfillBlockMerkleTree,
    "hash_bigint_backfill_block_merkle_tree",
    "block_merkle_tree",
    "block_merkle_tree_bigint"
);

pub struct CleanupLegacyHashTable;

#[async_trait]
impl DataBackfill for CleanupLegacyHashTable {
    fn name(&self) -> &'static str {
        "hash_bigint_cleanup_legacy_hash"
    }

    fn requires(&self) -> &'static [&'static str] {
        &[
            "hash_bigint_backfill_fee_merkle_tree",
            "hash_bigint_backfill_block_merkle_tree",
        ]
    }

    async fn run_batch(
        &self,
        tx: &mut Transaction<Write>,
        // Keyset cursor: last deleted id (0 on first batch).
        offset: u64,
    ) -> anyhow::Result<Option<u64>> {
        // Both merkle tree tables are now empty so there are no FK references to hash.id.
        let deleted: Vec<(i64,)> = sqlx::query_as(
            "DELETE FROM hash WHERE id IN (
                SELECT id FROM hash WHERE id > $1 ORDER BY id LIMIT $2
             ) RETURNING id",
        )
        .bind(offset as i64)
        .bind(self.batch_size() as i64)
        .fetch_all(tx.as_mut())
        .await?;

        Ok(deleted.last().map(|(id,)| *id as u64))
    }
}

pub fn hash_bigint_migrations() -> MigrationRegistry {
    MigrationRegistry::new()
        .backfill(BackfillHash)
        .backfill(BackfillFeeMerkleTree)
        .backfill(BackfillBlockMerkleTree)
        .backfill(CleanupLegacyHashTable)
}
