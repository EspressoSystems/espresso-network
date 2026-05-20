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
        offset: u64,
    ) -> anyhow::Result<Option<u64>> {
        let rows: Vec<(i32, Vec<u8>)> =
            sqlx::query_as("SELECT id, value FROM hash ORDER BY id LIMIT $1 OFFSET $2")
                .bind(self.batch_size() as i64)
                .bind(offset as i64)
                .fetch_all(tx.as_mut())
                .await?;

        if rows.is_empty() {
            return Ok(None);
        }
        let n = rows.len();

        for (id, value) in rows {
            sqlx::query(
                "INSERT INTO hash_bigint (id, value) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(id as i64)
            .bind(&value)
            .execute(tx.as_mut())
            .await?;
        }

        if n < self.batch_size() {
            Ok(None)
        } else {
            Ok(Some(offset + n as u64))
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
                offset: u64,
            ) -> anyhow::Result<Option<u64>> {
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
                    " ORDER BY path, created LIMIT $1 OFFSET $2"
                ))
                .bind(self.batch_size() as i64)
                .bind(offset as i64)
                .fetch_all(tx.as_mut())
                .await?;

                if rows.is_empty() {
                    return Ok(None);
                }
                let n = rows.len();

                for (path, created, hash_id, children, children_bitvec, idx, entry) in rows {
                    sqlx::query(concat!(
                        "INSERT INTO ",
                        $new_table,
                        " (path, created, hash_id, children, children_bitvec, idx, entry) VALUES \
                         ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING"
                    ))
                    .bind(&path)
                    .bind(created)
                    .bind(hash_id)
                    .bind(&children)
                    .bind(&children_bitvec)
                    .bind(&idx)
                    .bind(&entry)
                    .execute(tx.as_mut())
                    .await?;
                }

                if n < self.batch_size() {
                    Ok(None)
                } else {
                    Ok(Some(offset + n as u64))
                }
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

pub fn hash_bigint_migrations() -> MigrationRegistry {
    MigrationRegistry::new()
        .backfill(BackfillHash)
        .backfill(BackfillFeeMerkleTree)
        .backfill(BackfillBlockMerkleTree)
}
