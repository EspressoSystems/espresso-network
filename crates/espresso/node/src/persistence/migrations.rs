#![cfg(not(feature = "embedded-db"))]

use async_trait::async_trait;
use hotshot_query_service::{
    data_source::storage::sql::{Backfill, Transaction},
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
        tx: &mut Transaction<Backfill>,
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
                tx: &mut Transaction<Backfill>,
                // Cursor: the start of the `created` (block height) range for this batch.
                // Each batch covers [offset, offset + batch_size), using the index on `created`.
                offset: u64,
            ) -> anyhow::Result<Option<u64>> {
                let batch_size = self.batch_size() as i64;

                // Check if any rows remain at or beyond the current offset. Checking only the
                // current window would cause early termination if block heights have gaps larger
                // than batch_size; checking the open-ended tail means gaps are just a few fast
                // no-op iterations.
                let any: Option<(i64,)> = sqlx::query_as(concat!(
                    "SELECT created FROM ",
                    $legacy_table,
                    " WHERE created >= $1 LIMIT 1"
                ))
                .bind(offset as i64)
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
                         legacy_merkle_node.path,
                         legacy_merkle_node.created,
                         new_hash.id,
                         CASE WHEN legacy_merkle_node.children IS NULL THEN NULL
                              ELSE COALESCE((
                                  SELECT jsonb_agg(child_new_hash.id ORDER BY \
                     child_position)
                                  FROM \
                     jsonb_array_elements_text(legacy_merkle_node.children)
                                       WITH ORDINALITY AS child_elem(legacy_hash_id, \
                     child_position)
                                  JOIN hash AS child_legacy_hash
                                    ON child_legacy_hash.id = \
                     child_elem.legacy_hash_id::INT
                                  JOIN hash_bigint AS child_new_hash
                                    ON child_new_hash.value = child_legacy_hash.value
                              ), '[]'::jsonb)
                         END,
                         legacy_merkle_node.children_bitvec,
                         legacy_merkle_node.idx,
                         legacy_merkle_node.entry
                     FROM ",
                    $legacy_table,
                    " AS legacy_merkle_node
                     JOIN hash AS legacy_hash ON legacy_hash.id = \
                     legacy_merkle_node.hash_id
                     JOIN hash_bigint AS new_hash ON new_hash.value = legacy_hash.value
                     WHERE legacy_merkle_node.created >= $1 AND legacy_merkle_node.created \
                     < $2
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
        tx: &mut Transaction<Backfill>,
        _offset: u64,
    ) -> anyhow::Result<Option<u64>> {
        sqlx::query("TRUNCATE TABLE hash, fee_merkle_tree, block_merkle_tree")
            .execute(tx.as_mut())
            .await?;
        Ok(None)
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
            storage::{
                MerklizedStateStorage,
                sql::{
                    SqlStorage, StorageConnectionType, Transaction as SqlTransaction, Write,
                    testing::TmpDb,
                },
            },
        },
        merklized_state::{Snapshot, UpdateStateData},
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
            let mut tx = storage.backfill().await?;
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

    fn membership_proof(
        tree: &FeeMerkleTree,
        account: &FeeAccount,
    ) -> <FeeMerkleTree as MerkleTreeScheme>::MembershipProof {
        match tree.universal_lookup(account) {
            LookupResult::Ok(_, p) => p,
            _ => panic!("account not in tree"),
        }
    }

    /// Seed a `header` row carrying `commit` as its `fee_merkle_tree_root`.
    ///
    /// The root is read back by `snapshot_info` to verify the reconstructed proof.
    async fn insert_fee_header(
        tx: &mut SqlTransaction<Write>,
        height: i64,
        commit: <FeeMerkleTree as MerkleTreeScheme>::Commitment,
    ) {
        let data = serde_json::json!({
            "fee_merkle_tree_root": serde_json::to_value(commit).unwrap(),
            // Non-null filler for the other generated root column.
            "block_merkle_tree_root": "0",
        });
        sqlx::query(
            "INSERT INTO header (height, hash, payload_hash, timestamp, ns_table, data) VALUES \
             ($1, $2, $3, $4, $5, $6)",
        )
        .bind(height)
        .bind(format!("hash{height}"))
        .bind("payload")
        .bind(0i64)
        .bind("ns")
        .bind(data)
        .execute(tx.as_mut())
        .await
        .expect("insert header");
    }

    /// Regression test: reads during the backfill window must return the latest
    /// version of a node, not a stale one.
    ///
    /// Each Merkle node row is keyed `(path, created)` where `created` is the block
    /// height at which the node changed, so a single path accumulates one row per
    /// such height. The backfill moves rows by ascending `created`, so mid-migration
    /// a path's history is split: an older `created` lives in `*_bigint` while a
    /// newer `created` still lives in the legacy table.
    ///
    /// `get_path`'s legacy fallback keys on whether the path exists in `*_bigint` at
    /// all, not on `created`. With any older version present in `*_bigint` the path
    /// counts as "found", legacy is never consulted, and the snapshot read returns
    /// the stale older version. The reconstructed commitment then fails to match the
    /// header root and `get_path` errors (or, absent that check, returns wrong data).
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn read_during_backfill_returns_latest_version() {
        let db = TmpDb::init().await;
        let opt = tmp_options(&db);
        let cfg = Config::try_from(&opt).expect("config");
        let storage = SqlStorage::connect(cfg, StorageConnectionType::Query)
            .await
            .expect("connect");

        let account = FeeAccount::from(Address::repeat_byte(0x42));
        let mut tree = FeeMerkleTree::new(FEE_MERKLE_TREE_HEIGHT);

        // Height 1: account = 100. Live write goes to the *_bigint tables.
        tree.update(account, FeeAmount::from(100u64)).unwrap();
        let proof_v1 = membership_proof(&tree, &account);
        let commit_v1 = tree.commitment();
        let mut tx = storage.write().await.unwrap();
        write_fee_merkle_proofs(&mut tx, &tree, &[account], 1).await;
        tx.commit().await.unwrap();

        // Height 2: account = 200. Also written to the *_bigint tables.
        tree.update(account, FeeAmount::from(200u64)).unwrap();
        let proof_v2 = membership_proof(&tree, &account);
        let commit_v2 = tree.commitment();
        let mut tx = storage.write().await.unwrap();
        write_fee_merkle_proofs(&mut tx, &tree, &[account], 2).await;
        tx.commit().await.unwrap();

        // End-state of the mid-migration split, constructed in reverse: both
        // writes went to *_bigint above, now move the height-2 row across to
        // legacy. The natural flow is legacy → *_bigint by ascending `created`,
        // but the resulting (older in *_bigint, newer in legacy) shape is the
        // same. Also seed legacy `hash` so the legacy table's hash_id FK resolves.
        let mut tx = storage.write().await.unwrap();
        sqlx::query(
            "INSERT INTO hash (id, value) SELECT id::INT, value FROM hash_bigint ON CONFLICT DO \
             NOTHING",
        )
        .execute(tx.as_mut())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO fee_merkle_tree (path, created, hash_id, children, children_bitvec, idx, \
             entry) SELECT path, created, hash_id::INT, children, children_bitvec::BIT(256), idx, \
             entry FROM fee_merkle_tree_bigint WHERE created = 2",
        )
        .execute(tx.as_mut())
        .await
        .unwrap();
        sqlx::query("DELETE FROM fee_merkle_tree_bigint WHERE created = 2")
            .execute(tx.as_mut())
            .await
            .unwrap();
        tx.commit().await.unwrap();

        // Seed headers carrying each height's fee root, and mark height 2 decided.
        let mut tx = storage.write().await.unwrap();
        insert_fee_header(&mut tx, 1, commit_v1).await;
        insert_fee_header(&mut tx, 2, commit_v2).await;
        UpdateStateData::<SeqTypes, FeeMerkleTree, { FeeMerkleTree::ARITY }>::set_last_state_height(
            &mut tx, 2,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        // Control: height 1 is served from the *_bigint row and is correct.
        let mut tx = storage.read().await.unwrap();
        let got_v1 = tx
            .get_path(
                Snapshot::<SeqTypes, FeeMerkleTree, { FeeMerkleTree::ARITY }>::Index(1),
                account,
            )
            .await
            .expect("get_path at height 1");
        assert_eq!(got_v1, proof_v1, "height 1 should return the V1 proof");

        // Bug: height 2's latest node lives in legacy, but *_bigint still holds the
        // height-1 version of the same path, so the fallback returns it stale.
        let mut tx = storage.read().await.unwrap();
        let got_v2 = tx
            .get_path(
                Snapshot::<SeqTypes, FeeMerkleTree, { FeeMerkleTree::ARITY }>::Index(2),
                account,
            )
            .await
            .expect("get_path at height 2 (latest version is in the legacy table)");
        assert_eq!(
            got_v2, proof_v2,
            "height 2 must return the latest (V2) proof, not the stale V1 row from *_bigint"
        );
    }

    /// Regression test for the FK race between `BackfillHash` and live writes
    /// to `hash_bigint`.
    ///
    /// V1501 seeds the `hash_bigint(id)` sequence above `MAX(hash.id)` so new
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
        // simulate a database that was populated before V1501 ran. Then reset
        // the hash_bigint sequence the way V1501 itself does.
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

        // Live post-V1501 write at a new block height. The proof for the same
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
