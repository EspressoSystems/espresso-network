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
