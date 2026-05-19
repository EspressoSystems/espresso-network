// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the Espresso Sequencer.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

//! Background DataBackfill migrations for the espresso node.
//!
//! All migrations here are postgres-only: they back-fill data from the original tables (`hash`,
//! `fee_merkle_tree`, `block_merkle_tree`) — kept intact by migration V1302 as read fallbacks —
//! into the new BIGINT-keyed tables.

use async_trait::async_trait;
use hotshot_query_service::{
    data_source::storage::sql::{Transaction, Write},
    migration::{DataBackfill, MigrationRegistry},
};

// ---------------------------------------------------------------------------
// hash table backfill
// ---------------------------------------------------------------------------

#[cfg(not(feature = "embedded-db"))]
pub struct BackfillHash;

#[cfg(not(feature = "embedded-db"))]
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
            sqlx::query("INSERT INTO hash_bigint (id, value) VALUES ($1, $2) ON CONFLICT DO NOTHING")
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

// ---------------------------------------------------------------------------
// Merkle-tree table backfills (macro-generated)
// ---------------------------------------------------------------------------

macro_rules! merkle_tree_backfill {
    ($struct_name:ident, $migration_name:literal, $legacy_table:literal, $new_table:literal) => {
        #[cfg(not(feature = "embedded-db"))]
        pub struct $struct_name;

        #[cfg(not(feature = "embedded-db"))]
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

// ---------------------------------------------------------------------------
// Registry constructor
// ---------------------------------------------------------------------------

/// Build the [`MigrationRegistry`] for the hash INT → BIGINT backfill.
#[cfg(not(feature = "embedded-db"))]
pub fn hash_bigint_migrations() -> MigrationRegistry {
    MigrationRegistry::new()
        .backfill(BackfillHash)
        .backfill(BackfillFeeMerkleTree)
        .backfill(BackfillBlockMerkleTree)
}
