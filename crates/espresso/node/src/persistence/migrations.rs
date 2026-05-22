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

                // Check if there is any work left in this window. If not, return None to
                // signal completion.
                let any: Option<(i64,)> = sqlx::query_as(concat!(
                    "SELECT created FROM ",
                    $legacy_table,
                    " WHERE created >= $1 AND created < $2 LIMIT 1"
                ))
                .bind(offset as i64)
                .bind(offset as i64 + batch_size)
                .fetch_optional(tx.as_mut())
                .await?;
                if any.is_none() {
                    return Ok(None);
                }

                // Move rows from legacy into the new _bigint table, translating both
                // `hash_id` and every element of `children` from legacy hash ids into
                // hash_bigint ids by joining on the hash `value`. This removes any
                // dependency on `BackfillHash` having preserved the original ids.
                sqlx::query(concat!(
                    "INSERT INTO ",
                    $new_table,
                    " (path, created, hash_id, children, children_bitvec, idx, entry)
                     SELECT
                         fmt.path,
                         fmt.created,
                         hb.id,
                         CASE WHEN fmt.children IS NULL THEN NULL
                              ELSE COALESCE((
                                  SELECT jsonb_agg(chb.id ORDER BY ord)
                                  FROM jsonb_array_elements_text(fmt.children)
                                       WITH ORDINALITY AS arr(child_id, ord)
                                  JOIN hash ch ON ch.id = arr.child_id::INT
                                  JOIN hash_bigint chb ON chb.value = ch.value
                              ), '[]'::jsonb)
                         END,
                         fmt.children_bitvec,
                         fmt.idx,
                         fmt.entry
                     FROM ",
                    $legacy_table,
                    " fmt
                     JOIN hash h ON h.id = fmt.hash_id
                     JOIN hash_bigint hb ON hb.value = h.value
                     WHERE fmt.created >= $1 AND fmt.created < $2
                     ON CONFLICT (path, created) DO NOTHING"
                ))
                .bind(offset as i64)
                .bind(offset as i64 + batch_size)
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

#[cfg(test)]
mod tests {
    use alloy::primitives::Address;
    use espresso_types::{FEE_MERKLE_TREE_HEIGHT, FeeAccount, FeeAmount, FeeMerkleTree, SeqTypes};
    use hotshot_query_service::{
        data_source::{
            Transaction as _, VersionedDataSource,
            sql::Config,
            storage::sql::{
                SqlStorage, StorageConnectionType, Transaction as SqlTransaction, Write,
                testing::TmpDb,
            },
        },
        merklized_state::UpdateStateData,
    };
    use jf_merkle_tree_compat::{
        LookupResult, MerkleTreeScheme, ToTraversalPath, UniversalMerkleTreeScheme,
    };

    use super::*;
    use crate::api::sql::impl_testable_data_source::tmp_options;

    async fn run_to_completion(
        backfill: &dyn DataBackfill,
        storage: &SqlStorage,
    ) -> anyhow::Result<()> {
        let mut offset = 0u64;
        loop {
            let mut tx = storage.write().await?;
            let next = backfill.run_batch(&mut tx, offset).await?;
            tx.commit().await?;
            match next {
                Some(o) => offset = o,
                None => return Ok(()),
            }
        }
    }

    async fn write_fee_merkle_proofs(
        tx: &mut SqlTransaction<Write>,
        tree: &FeeMerkleTree,
        accounts: &[FeeAccount],
        block_height: u64,
    ) {
        let proofs: Vec<_> = accounts
            .iter()
            .map(|a| {
                let proof = match tree.universal_lookup(a) {
                    LookupResult::Ok(_, p) => p,
                    LookupResult::NotFound(p) => p,
                    LookupResult::NotInMemory => panic!("account not in memory"),
                };
                let path =
                    <FeeAccount as ToTraversalPath<{ FeeMerkleTree::ARITY }>>::to_traversal_path(
                        a,
                        tree.height(),
                    );
                (proof, path)
            })
            .collect();
        UpdateStateData::<SeqTypes, FeeMerkleTree, { FeeMerkleTree::ARITY }>::insert_merkle_nodes_batch(
            tx,
            proofs,
            block_height,
        )
        .await
        .expect("insert_merkle_nodes_batch");
    }

    /// Regression test for the FK race between `BackfillHash` and live writes
    /// to `hash_bigint`.
    ///
    /// V1302 seeds the `hash_bigint(id)` sequence above `MAX(hash.id)` so new
    /// auto-ids cannot collide with legacy ids — but nothing protects the
    /// `value` UNIQUE constraint. Whenever a post-migration write inserts a
    /// value that also lives in legacy `hash` (the common case: empty-subtree
    /// hashes and unchanged branch hashes are byte-identical across blocks),
    /// the live row claims a new id, and the backfill's `INSERT (old_id, value)
    /// ON CONFLICT DO NOTHING` is silently dropped. The Merkle tree backfill
    /// then copies the legacy `hash_id` verbatim and the FK to `hash_bigint(id)`
    /// fires because that id was never inserted.
    ///
    /// This test exercises the real `UpdateStateData::insert_merkle_nodes_batch`
    /// path so the shared-hash overlap arises from realistic Merkle proofs.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn backfill_preserves_fk_when_live_write_shares_hash_value() {
        let db = TmpDb::init().await;
        let opt = tmp_options(&db);
        let cfg = Config::try_from(&opt).expect("config");
        let storage = SqlStorage::connect(cfg, StorageConnectionType::Query)
            .await
            .expect("connect");

        let mut tree = FeeMerkleTree::new(FEE_MERKLE_TREE_HEIGHT);
        let account = FeeAccount::from(Address::repeat_byte(0x42));
        tree.update(account, FeeAmount::from(100_u64)).unwrap();

        // Write a real Merkle proof for `account` into the *new* _bigint tables.
        let mut tx = storage.write().await.unwrap();
        write_fee_merkle_proofs(&mut tx, &tree, &[account], 1).await;
        tx.commit().await.unwrap();

        // Move every row from the _bigint tables back into the legacy tables to
        // simulate a database that was populated before V1302 ran. Then reset
        // the hash_bigint sequence the way V1302 itself does.
        let mut tx = storage.write().await.unwrap();
        sqlx::query("INSERT INTO hash (id, value) SELECT id::INT, value FROM hash_bigint")
            .execute(tx.as_mut())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO fee_merkle_tree (path, created, hash_id, children, children_bitvec, idx, \
             entry) SELECT path, created, hash_id::INT, children, children_bitvec::BIT(256), idx, \
             entry FROM fee_merkle_tree_bigint",
        )
        .execute(tx.as_mut())
        .await
        .unwrap();
        sqlx::query("TRUNCATE fee_merkle_tree_bigint, block_merkle_tree_bigint, hash_bigint")
            .execute(tx.as_mut())
            .await
            .unwrap();
        sqlx::query(
            "SELECT setval(pg_get_serial_sequence('hash_bigint', 'id'), GREATEST(COALESCE((SELECT \
             MAX(id) FROM hash), 1), 1))",
        )
        .execute(tx.as_mut())
        .await
        .unwrap();
        tx.commit().await.unwrap();

        // Live post-V1302 write at a new block height. The proof for the same
        // account shares almost every hash value with the legacy proof above,
        // so the live `batch_insert_hashes` calls collide on `value` with every
        // row BackfillHash is about to copy.
        let mut tx = storage.write().await.unwrap();
        write_fee_merkle_proofs(&mut tx, &tree, &[account], 2).await;
        tx.commit().await.unwrap();

        // Drive backfills directly so a failure surfaces immediately rather
        // than entering the registry's 5-minute retry loop.
        run_to_completion(&BackfillHash, &storage)
            .await
            .expect("BackfillHash failed");
        run_to_completion(&BackfillFeeMerkleTree, &storage)
            .await
            .expect("BackfillFeeMerkleTree failed (FK violation from dropped hash row)");

        let mut tx = storage.read().await.unwrap();
        let (n_legacy,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM fee_merkle_tree")
            .fetch_one(tx.as_mut())
            .await
            .unwrap();
        assert_eq!(n_legacy, 0, "legacy fee_merkle_tree rows were not migrated");

        let (n_heights,): (i64,) =
            sqlx::query_as("SELECT COUNT(DISTINCT created) FROM fee_merkle_tree_bigint")
                .fetch_one(tx.as_mut())
                .await
                .unwrap();
        assert_eq!(n_heights, 2, "expected rows at both heights");

        let (n_orphans,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM fee_merkle_tree_bigint fmt LEFT JOIN hash_bigint hb ON hb.id = \
             fmt.hash_id WHERE hb.id IS NULL",
        )
        .fetch_one(tx.as_mut())
        .await
        .unwrap();
        assert_eq!(n_orphans, 0, "fee_merkle_tree_bigint has dangling hash_id");

        let (n_orphan_children,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM ( SELECT child_id FROM fee_merkle_tree_bigint fmt, \
             jsonb_array_elements_text(fmt.children) AS arr(child_id) WHERE fmt.children IS NOT \
             NULL ) c LEFT JOIN hash_bigint hb ON hb.id = c.child_id::BIGINT WHERE hb.id IS NULL",
        )
        .fetch_one(tx.as_mut())
        .await
        .unwrap();
        assert_eq!(
            n_orphan_children, 0,
            "fee_merkle_tree_bigint.children has dangling hash_id"
        );
    }
}
