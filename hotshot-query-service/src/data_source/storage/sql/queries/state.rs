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

//! Merklized state storage implementation for a database query engine.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use ark_serialize::CanonicalDeserialize;
use async_trait::async_trait;
use futures::stream::TryStreamExt;
use hotshot_types::traits::node_implementation::NodeType;
use jf_merkle_tree::{
    prelude::{MerkleNode, MerkleProof},
    DigestAlgorithm, MerkleCommitment, ToTraversalPath,
};
use sqlx::types::{BitVec, JsonValue};

use super::{
    super::transaction::{query_as, Transaction, TransactionMode, Write},
    DecodeError, QueryBuilder,
};
use crate::{
    data_source::storage::{
        pruning::PrunedHeightStorage,
        sql::{build_where_in, sqlx::Row},
        MerklizedStateHeightStorage, MerklizedStateStorage,
    },
    merklized_state::{MerklizedState, Snapshot},
    QueryError, QueryResult,
};

#[async_trait]
impl<Mode, Types, State, const ARITY: usize> MerklizedStateStorage<Types, State, ARITY>
    for Transaction<Mode>
where
    Mode: TransactionMode,
    Types: NodeType,
    State: MerklizedState<Types, ARITY> + 'static,
{
    /// Retrieves a Merkle path from the database
    async fn get_path(
        &mut self,
        snapshot: Snapshot<Types, State, ARITY>,
        key: State::Key,
    ) -> QueryResult<MerkleProof<State::Entry, State::Key, State::T, ARITY>> {
        let state_type = State::state_type();
        let tree_height = State::tree_height();

        // Get the traversal path of the index
        let traversal_path = State::Key::to_traversal_path(&key, tree_height);
        let (created, merkle_commitment) = self.snapshot_info(snapshot).await?;

        // Get all the nodes in the path to the index.
        // Order by pos DESC is to return nodes from the leaf to the root
        let (query, sql) = build_get_path_query(state_type, traversal_path.clone(), created)?;
        let rows = query.query(&sql).fetch_all(self.as_mut()).await?;

        let nodes: Vec<Node> = rows.into_iter().map(|r| r.into()).collect();

        // insert all the hash ids to a hashset which is used to query later
        // HashSet is used to avoid duplicates
        let mut hash_ids = HashSet::new();
        for node in nodes.iter() {
            hash_ids.insert(node.hash_id);
            if let Some(children) = &node.children {
                let children: Vec<i32> =
                    serde_json::from_value(children.clone()).map_err(|e| QueryError::Error {
                        message: format!("Error deserializing 'children' into Vec<i32>: {e}"),
                    })?;
                hash_ids.extend(children);
            }
        }

        // Find all the hash values and create a hashmap
        // Hashmap will be used to get the hash value of the nodes children and the node itself.
        let hashes = if !hash_ids.is_empty() {
            let (query, sql) = build_where_in("SELECT id, value FROM hash", "id", hash_ids)?;
            query
                .query_as(&sql)
                .fetch(self.as_mut())
                .try_collect::<HashMap<i32, Vec<u8>>>()
                .await?
        } else {
            HashMap::new()
        };

        let mut proof_path = VecDeque::with_capacity(State::tree_height());
        for Node {
            hash_id,
            children,
            children_bitvec,
            idx,
            entry,
            ..
        } in nodes.iter()
        {
            {
                let value = hashes.get(hash_id).ok_or(QueryError::Error {
                    message: format!("node's value references non-existent hash {hash_id}"),
                })?;

                match (children, children_bitvec, idx, entry) {
                    // If the row has children then its a branch
                    (Some(children), Some(children_bitvec), None, None) => {
                        let children: Vec<i32> =
                            serde_json::from_value(children.clone()).map_err(|e| {
                                QueryError::Error {
                                    message: format!(
                                        "Error deserializing 'children' into Vec<i32>: {e}"
                                    ),
                                }
                            })?;
                        let mut children = children.iter();

                        // Reconstruct the Children MerkleNodes from storage.
                        // Children bit_vec is used to create forgotten  or empty node
                        let child_nodes = children_bitvec
                            .iter()
                            .map(|bit| {
                                if bit {
                                    let hash_id = children.next().ok_or(QueryError::Error {
                                        message: "node has fewer children than set bits".into(),
                                    })?;
                                    let value = hashes.get(hash_id).ok_or(QueryError::Error {
                                        message: format!(
                                            "node's child references non-existent hash {hash_id}"
                                        ),
                                    })?;
                                    Ok(Arc::new(MerkleNode::ForgettenSubtree {
                                        value: State::T::deserialize_compressed(value.as_slice())
                                            .decode_error("malformed merkle node value")?,
                                    }))
                                } else {
                                    Ok(Arc::new(MerkleNode::Empty))
                                }
                            })
                            .collect::<QueryResult<Vec<_>>>()?;
                        // Use the Children merkle nodes to reconstruct the branch node
                        proof_path.push_back(MerkleNode::Branch {
                            value: State::T::deserialize_compressed(value.as_slice())
                                .decode_error("malformed merkle node value")?,
                            children: child_nodes,
                        });
                    },
                    // If it has an entry, it's a leaf
                    (None, None, Some(index), Some(entry)) => {
                        proof_path.push_back(MerkleNode::Leaf {
                            value: State::T::deserialize_compressed(value.as_slice())
                                .decode_error("malformed merkle node value")?,
                            pos: serde_json::from_value(index.clone())
                                .decode_error("malformed merkle node index")?,
                            elem: serde_json::from_value(entry.clone())
                                .decode_error("malformed merkle element")?,
                        });
                    },
                    // Otherwise, it's empty.
                    (None, None, Some(_), None) => {
                        proof_path.push_back(MerkleNode::Empty);
                    },
                    _ => {
                        return Err(QueryError::Error {
                            message: "Invalid type of merkle node found".to_string(),
                        });
                    },
                }
            }
        }

        // Reconstruct the merkle commitment from the path
        let init = if let Some(MerkleNode::Leaf { value, .. }) = proof_path.front() {
            *value
        } else {
            // If the path ends in a branch (or, as a special case, if the path and thus the entire
            // tree is empty), we are looking up an entry that is not present in the tree. We always
            // store all the nodes on all the paths to all the entries in the tree, so the only
            // nodes we could be missing are empty nodes from unseen entries. Thus, we can
            // reconstruct what the path should be by prepending empty nodes.
            while proof_path.len() <= State::tree_height() {
                proof_path.push_front(MerkleNode::Empty);
            }
            State::T::default()
        };
        let commitment_from_path = traversal_path
            .iter()
            .zip(proof_path.iter().skip(1))
            .try_fold(init, |val, (branch, node)| -> QueryResult<State::T> {
                match node {
                    MerkleNode::Branch { value: _, children } => {
                        let data = children
                            .iter()
                            .map(|node| match node.as_ref() {
                                MerkleNode::ForgettenSubtree { value } => Ok(*value),
                                MerkleNode::Empty => Ok(State::T::default()),
                                _ => Err(QueryError::Error {
                                    message: "Invalid child node".to_string(),
                                }),
                            })
                            .collect::<QueryResult<Vec<_>>>()?;

                        if data[*branch] != val {
                            // This can only happen if data is missing: we have an old version of
                            // one of the nodes in the path, which is why it is not matching up with
                            // its parent.
                            tracing::warn!(
                                ?key,
                                parent = ?data[*branch],
                                child = ?val,
                                branch = %*branch,
                                %created,
                                %merkle_commitment,
                                "missing data in merklized state; parent-child mismatch",
                            );
                            return Err(QueryError::Missing);
                        }

                        State::Digest::digest(&data).map_err(|err| QueryError::Error {
                            message: format!("failed to update digest: {err:#}"),
                        })
                    },
                    MerkleNode::Empty => Ok(init),
                    _ => Err(QueryError::Error {
                        message: "Invalid type of Node in the proof".to_string(),
                    }),
                }
            })?;

        if commitment_from_path != merkle_commitment.digest() {
            return Err(QueryError::Error {
                message:
                    format!("Commitment calculated from merkle path ({commitment_from_path:?}) does not match the commitment in the header ({:?})", merkle_commitment.digest()),
            });
        }

        Ok(MerkleProof {
            pos: key,
            proof: proof_path.into(),
        })
    }
}

#[async_trait]
impl<Mode: TransactionMode> MerklizedStateHeightStorage for Transaction<Mode> {
    async fn get_last_state_height(&mut self) -> QueryResult<usize> {
        let Some((height,)) = query_as::<(i64,)>("SELECT height from last_merklized_state_height")
            .fetch_optional(self.as_mut())
            .await?
        else {
            return Ok(0);
        };
        Ok(height as usize)
    }
}

impl<Mode: TransactionMode> Transaction<Mode> {
    /// Get information identifying a [`Snapshot`].
    ///
    /// If the given snapshot is known to the database, this function returns
    /// * The block height at which the snapshot was created
    /// * A digest of the Merkle commitment to the snapshotted state
    async fn snapshot_info<Types, State, const ARITY: usize>(
        &mut self,
        snapshot: Snapshot<Types, State, ARITY>,
    ) -> QueryResult<(i64, State::Commit)>
    where
        Types: NodeType,
        State: MerklizedState<Types, ARITY>,
    {
        let header_state_commitment_field = State::header_state_commitment_field();

        let (created, commit) = match snapshot {
            Snapshot::Commit(commit) => {
                // Get the block height using the merkle commitment. It is possible that multiple
                // headers will have the same state commitment. In this case we don't care which
                // height we get, since any query against equivalent states will yield equivalent
                // results, regardless of which block the state is from. Thus, we can make this
                // query fast with `LIMIT 1` and no `ORDER BY`.
                let (height,) = query_as(&format!(
                    "SELECT height
                       FROM header
                      WHERE {header_state_commitment_field} = $1
                      LIMIT 1"
                ))
                .bind(commit.to_string())
                .fetch_one(self.as_mut())
                .await?;

                (height, commit)
            },
            Snapshot::Index(created) => {
                let created = created as i64;
                let (commit,) = query_as::<(String,)>(&format!(
                    "SELECT {header_state_commitment_field} AS root_commitment
                       FROM header
                      WHERE height = $1
                      LIMIT 1"
                ))
                .bind(created)
                .fetch_one(self.as_mut())
                .await?;
                let commit = serde_json::from_value(commit.into())
                    .decode_error("malformed state commitment")?;
                (created, commit)
            },
        };

        // Make sure the requested snapshot is up to date.
        let height = self.get_last_state_height().await?;

        if height < (created as usize) {
            return Err(QueryError::NotFound);
        }

        let pruned_height = self
            .load_pruned_height()
            .await
            .map_err(|e| QueryError::Error {
                message: format!("failed to load pruned height: {e}"),
            })?;

        if pruned_height.is_some_and(|h| height <= h as usize) {
            return Err(QueryError::NotFound);
        }

        Ok((created, commit))
    }
}

// TODO: create a generic upsert function with retries that returns the column
pub(crate) fn build_hash_batch_insert(
    hashes: &[Vec<u8>],
) -> QueryResult<(QueryBuilder<'_>, String)> {
    let mut query = QueryBuilder::default();
    let params = hashes
        .iter()
        .map(|hash| Ok(format!("({})", query.bind(hash)?)))
        .collect::<QueryResult<Vec<String>>>()?;
    let sql = format!(
        "INSERT INTO hash(value) values {} ON CONFLICT (value) DO UPDATE SET value = EXCLUDED.value returning value, id",
        params.join(",")
    );
    Ok((query, sql))
}

// Represents a row in a state table
#[derive(Debug, Default, Clone)]
pub(crate) struct Node {
    pub(crate) path: JsonValue,
    pub(crate) created: i64,
    pub(crate) hash_id: i32,
    pub(crate) children: Option<JsonValue>,
    pub(crate) children_bitvec: Option<BitVec>,
    pub(crate) idx: Option<JsonValue>,
    pub(crate) entry: Option<JsonValue>,
}

#[cfg(feature = "embedded-db")]
impl From<sqlx::sqlite::SqliteRow> for Node {
    fn from(row: sqlx::sqlite::SqliteRow) -> Self {
        let bit_string: Option<String> = row.get_unchecked("children_bitvec");
        let children_bitvec: Option<BitVec> =
            bit_string.map(|b| b.chars().map(|c| c == '1').collect());

        Self {
            path: row.get_unchecked("path"),
            created: row.get_unchecked("created"),
            hash_id: row.get_unchecked("hash_id"),
            children: row.get_unchecked("children"),
            children_bitvec,
            idx: row.get_unchecked("idx"),
            entry: row.get_unchecked("entry"),
        }
    }
}

#[cfg(not(feature = "embedded-db"))]
impl From<sqlx::postgres::PgRow> for Node {
    fn from(row: sqlx::postgres::PgRow) -> Self {
        Self {
            path: row.get_unchecked("path"),
            created: row.get_unchecked("created"),
            hash_id: row.get_unchecked("hash_id"),
            children: row.get_unchecked("children"),
            children_bitvec: row.get_unchecked("children_bitvec"),
            idx: row.get_unchecked("idx"),
            entry: row.get_unchecked("entry"),
        }
    }
}

impl Node {
    pub(crate) async fn upsert(
        name: &str,
        nodes: impl IntoIterator<Item = Self>,
        tx: &mut Transaction<Write>,
    ) -> anyhow::Result<()> {
        tx.upsert(
            name,
            [
                "path",
                "created",
                "hash_id",
                "children",
                "children_bitvec",
                "idx",
                "entry",
            ],
            ["path", "created"],
            nodes.into_iter().map(|n| {
                #[cfg(feature = "embedded-db")]
                let children_bitvec: Option<String> = n
                    .children_bitvec
                    .clone()
                    .map(|b| b.iter().map(|bit| if bit { '1' } else { '0' }).collect());

                #[cfg(not(feature = "embedded-db"))]
                let children_bitvec = n.children_bitvec.clone();

                (
                    n.path.clone(),
                    n.created,
                    n.hash_id,
                    n.children.clone(),
                    children_bitvec,
                    n.idx.clone(),
                    n.entry.clone(),
                )
            }),
        )
        .await
    }
}

fn build_get_path_query<'q>(
    table: &'static str,
    traversal_path: Vec<usize>,
    created: i64,
) -> QueryResult<(QueryBuilder<'q>, String)> {
    let mut query = QueryBuilder::default();
    let mut traversal_path = traversal_path.into_iter().map(|x| x as i32);

    // We iterate through the path vector skipping the first element after each iteration
    let len = traversal_path.len();
    let mut sub_queries = Vec::new();

    query.bind(created)?;

    for _ in 0..=len {
        let path = traversal_path.clone().rev().collect::<Vec<_>>();
        let path: serde_json::Value = path.into();
        let node_path = query.bind(path)?;

        let sub_query = format!(
            "SELECT * FROM (SELECT * FROM {table} WHERE path = {node_path} AND created <= $1 ORDER BY created DESC LIMIT 1)",
        );

        sub_queries.push(sub_query);
        traversal_path.next();
    }

    let mut sql: String = sub_queries.join(" UNION ");

    sql = format!("SELECT * FROM ({sql}) as t ");

    // PostgreSQL already orders JSON arrays by length, so no additional function is needed
    // For SQLite, `length()` is used to sort by length.
    if cfg!(feature = "embedded-db") {
        sql.push_str("ORDER BY length(t.path) DESC");
    } else {
        sql.push_str("ORDER BY t.path DESC");
    }

    Ok((query, sql))
}

#[cfg(test)]
mod test {
    use futures::stream::StreamExt;
    use jf_merkle_tree::{
        universal_merkle_tree::UniversalMerkleTree, LookupResult, MerkleTreeScheme,
        UniversalMerkleTreeScheme,
    };
    use rand::{seq::IteratorRandom, RngCore};

    use super::*;
    use crate::{
        data_source::{
            storage::sql::{testing::TmpDb, *},
            VersionedDataSource,
        },
        merklized_state::UpdateStateData,
        testing::{
            mocks::{MockMerkleTree, MockTypes},
            setup_test,
        },
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn test_merklized_state_storage() {
        // In this test we insert some entries into the tree and update the database
        // Each entry's merkle path is compared with the path from the tree
        setup_test();

        let db = TmpDb::init().await;
        let storage = SqlStorage::connect(db.config()).await.unwrap();

        // define a test tree
        let mut test_tree: UniversalMerkleTree<_, _, _, 8, _> =
            MockMerkleTree::new(MockMerkleTree::tree_height());
        let block_height = 1;

        // insert some entries into the tree and the header table
        // Header table is used the get_path query to check if the header exists for the block height.
        let mut tx = storage.write().await.unwrap();
        for i in 0..27 {
            test_tree.update(i, i).unwrap();

            // data field of the header
            let test_data = serde_json::json!({ MockMerkleTree::header_state_commitment_field() : serde_json::to_value(test_tree.commitment()).unwrap()});
            tx.upsert(
                "header",
                ["height", "hash", "payload_hash", "timestamp", "data"],
                ["height"],
                [(
                    block_height as i64,
                    format!("randomHash{i}"),
                    "t".to_string(),
                    0,
                    test_data,
                )],
            )
            .await
            .unwrap();
            // proof for the index from the tree
            let (_, proof) = test_tree.lookup(i).expect_ok().unwrap();
            // traversal path for the index.
            let traversal_path =
                <usize as ToTraversalPath<8>>::to_traversal_path(&i, test_tree.height());

            UpdateStateData::<_, MockMerkleTree, 8>::insert_merkle_nodes(
                &mut tx,
                proof.clone(),
                traversal_path.clone(),
                block_height as u64,
            )
            .await
            .expect("failed to insert nodes");
        }
        // update saved state height
        UpdateStateData::<_, MockMerkleTree, 8>::set_last_state_height(&mut tx, block_height)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        //Get the path and check if it matches the lookup
        for i in 0..27 {
            // Query the path for the index
            let mut tx = storage.read().await.unwrap();
            let merkle_path = tx
                .get_path(
                    Snapshot::<_, MockMerkleTree, 8>::Index(block_height as u64),
                    i,
                )
                .await
                .unwrap();

            let (_, proof) = test_tree.lookup(i).expect_ok().unwrap();

            tracing::info!("merkle path {:?}", merkle_path);

            // merkle path from the storage should match the path from test tree
            assert_eq!(merkle_path, proof.clone(), "merkle paths mismatch");
        }

        // Get the proof of index 0 with bh = 1
        let (_, proof_bh_1) = test_tree.lookup(0).expect_ok().unwrap();
        // Inserting Index 0 again with created (bh) = 2
        // Our database should then have 2 versions of this leaf node
        // Update the node so that proof is also updated
        test_tree.update(0, 99).unwrap();
        // Also update the merkle commitment in the header

        // data field of the header
        let mut tx = storage.write().await.unwrap();
        let test_data = serde_json::json!({ MockMerkleTree::header_state_commitment_field() : serde_json::to_value(test_tree.commitment()).unwrap()});
        tx.upsert(
            "header",
            ["height", "hash", "payload_hash", "timestamp", "data"],
            ["height"],
            [(
                2i64,
                "randomstring".to_string(),
                "t".to_string(),
                0,
                test_data,
            )],
        )
        .await
        .unwrap();
        let (_, proof_bh_2) = test_tree.lookup(0).expect_ok().unwrap();
        // traversal path for the index.
        let traversal_path =
            <usize as ToTraversalPath<8>>::to_traversal_path(&0, test_tree.height());
        // Update storage to insert a new version of this code

        UpdateStateData::<_, MockMerkleTree, 8>::insert_merkle_nodes(
            &mut tx,
            proof_bh_2.clone(),
            traversal_path.clone(),
            2,
        )
        .await
        .expect("failed to insert nodes");
        // update saved state height
        UpdateStateData::<_, MockMerkleTree, 8>::set_last_state_height(&mut tx, 2)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let node_path = traversal_path
            .into_iter()
            .rev()
            .map(|n| n as i32)
            .collect::<Vec<_>>();

        // Find all the nodes of Index 0 in table
        let mut tx = storage.read().await.unwrap();
        let rows = query("SELECT * from test_tree where path = $1 ORDER BY created")
            .bind(serde_json::to_value(node_path).unwrap())
            .fetch(tx.as_mut());

        let nodes: Vec<Node> = rows.map(|res| res.unwrap().into()).collect().await;
        // There should be only 2 versions of this node
        assert!(nodes.len() == 2, "incorrect number of nodes");
        assert_eq!(nodes[0].created, 1, "wrong block height");
        assert_eq!(nodes[1].created, 2, "wrong block height");

        // Now we can have two snapshots for Index 0
        // One with created = 1 and other with 2
        // Query snapshot with created = 2

        let path_with_bh_2 = storage
            .read()
            .await
            .unwrap()
            .get_path(Snapshot::<_, MockMerkleTree, 8>::Index(2), 0)
            .await
            .unwrap();

        assert_eq!(path_with_bh_2, proof_bh_2);
        let path_with_bh_1 = storage
            .read()
            .await
            .unwrap()
            .get_path(Snapshot::<_, MockMerkleTree, 8>::Index(1), 0)
            .await
            .unwrap();
        assert_eq!(path_with_bh_1, proof_bh_1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_merklized_state_non_membership_proof() {
        // This test updates the Merkle tree with a new entry and inserts the corresponding Merkle nodes into the database with created = 1.
        // A Merkle node is then deleted from the tree.
        // The database is then updated to reflect the deletion of the entry with a created (block height) of 2
        // As the leaf node becomes a non-member, we do a universal lookup to obtain its non-membership proof path.
        // It is expected that the path retrieved from the tree matches the path obtained from the database.
        setup_test();

        let db = TmpDb::init().await;
        let storage = SqlStorage::connect(db.config()).await.unwrap();

        // define a test tree
        let mut test_tree = MockMerkleTree::new(MockMerkleTree::tree_height());
        let block_height = 1;
        //insert an entry into the tree
        test_tree.update(0, 0).unwrap();
        let commitment = test_tree.commitment();

        let test_data = serde_json::json!({ MockMerkleTree::header_state_commitment_field() : serde_json::to_value(commitment).unwrap()});
        // insert the header with merkle commitment
        let mut tx = storage.write().await.unwrap();
        tx.upsert(
            "header",
            ["height", "hash", "payload_hash", "timestamp", "data"],
            ["height"],
            [(
                block_height as i64,
                "randomString".to_string(),
                "t".to_string(),
                0,
                test_data,
            )],
        )
        .await
        .unwrap();
        // proof for the index from the tree
        let (_, proof_before_remove) = test_tree.lookup(0).expect_ok().unwrap();
        // traversal path for the index.
        let traversal_path =
            <usize as ToTraversalPath<8>>::to_traversal_path(&0, test_tree.height());
        // insert merkle nodes
        UpdateStateData::<_, MockMerkleTree, 8>::insert_merkle_nodes(
            &mut tx,
            proof_before_remove.clone(),
            traversal_path.clone(),
            block_height as u64,
        )
        .await
        .expect("failed to insert nodes");
        // update saved state height
        UpdateStateData::<_, MockMerkleTree, 8>::set_last_state_height(&mut tx, block_height)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        // the path from the db and and tree should match
        let merkle_path = storage
            .read()
            .await
            .unwrap()
            .get_path(
                Snapshot::<_, MockMerkleTree, 8>::Index(block_height as u64),
                0,
            )
            .await
            .unwrap();

        // merkle path from the storage should match the path from test tree
        assert_eq!(
            merkle_path,
            proof_before_remove.clone(),
            "merkle paths mismatch"
        );

        //Deleting the index 0
        test_tree.remove(0).expect("failed to delete index 0 ");

        // Update the database with the proof
        // Created = 2 in this case
        let proof_after_remove = test_tree.universal_lookup(0).expect_not_found().unwrap();

        let mut tx = storage.write().await.unwrap();
        UpdateStateData::<_, MockMerkleTree, 8>::insert_merkle_nodes(
            &mut tx,
            proof_after_remove.clone(),
            traversal_path.clone(),
            2_u64,
        )
        .await
        .expect("failed to insert nodes");
        // Insert the new header
        tx.upsert(
                "header",
                ["height", "hash", "payload_hash", "timestamp", "data"],
                ["height"],
                [(
                    2i64,
                    "randomString2".to_string(),
                    "t".to_string(),
                    0,
                    serde_json::json!({ MockMerkleTree::header_state_commitment_field() : serde_json::to_value(test_tree.commitment()).unwrap()}),
                )],
            )
            .await
            .unwrap();
        // update saved state height
        UpdateStateData::<_, MockMerkleTree, 8>::set_last_state_height(&mut tx, 2)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        // Get non membership proof
        let non_membership_path = storage
            .read()
            .await
            .unwrap()
            .get_path(Snapshot::<_, MockMerkleTree, 8>::Index(2_u64), 0)
            .await
            .unwrap();
        // Assert that the paths from the db and the tree are equal
        assert_eq!(
            non_membership_path, proof_after_remove,
            "merkle paths dont match"
        );

        // Query the membership proof i.e proof with created = 1
        // This proof should be equal to the proof before deletion
        // Assert that the paths from the db and the tree are equal

        let proof_bh_1 = storage
            .read()
            .await
            .unwrap()
            .get_path(Snapshot::<_, MockMerkleTree, 8>::Index(1_u64), 0)
            .await
            .unwrap();
        assert_eq!(proof_bh_1, proof_before_remove, "merkle paths dont match");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_merklized_state_non_membership_proof_unseen_entry() {
        setup_test();

        let db = TmpDb::init().await;
        let storage = SqlStorage::connect(db.config()).await.unwrap();

        // define a test tree
        let mut test_tree = MockMerkleTree::new(MockMerkleTree::tree_height());

        // For each case (where the root is empty, a leaf, and a branch) test getting a
        // non-membership proof for an entry node the database has never seen.
        for i in 0..=2 {
            tracing::info!(i, ?test_tree, "testing non-membership proof");
            let mut tx = storage.write().await.unwrap();

            // Insert a dummy header
            tx.upsert(
                "header",
                ["height", "hash", "payload_hash", "timestamp", "data"],
                ["height"],
                [(
                    i as i64,
                    format!("hash{i}"),
                    "t".to_string(),
                    0,
                    serde_json::json!({ MockMerkleTree::header_state_commitment_field() : serde_json::to_value(test_tree.commitment()).unwrap()})
                )],
            )
            .await
            .unwrap();
            // update saved state height
            UpdateStateData::<_, MockMerkleTree, 8>::set_last_state_height(&mut tx, i)
                .await
                .unwrap();
            tx.commit().await.unwrap();

            // get a non-membership proof for a never-before-seen node.
            let proof = storage
                .read()
                .await
                .unwrap()
                .get_path(
                    Snapshot::<MockTypes, MockMerkleTree, 8>::Index(i as u64),
                    100,
                )
                .await
                .unwrap();
            assert_eq!(proof.elem(), None);
            assert!(test_tree.non_membership_verify(100, proof).unwrap());

            // insert an additional node into the tree.
            test_tree.update(i, i).unwrap();
            let (_, proof) = test_tree.lookup(i).expect_ok().unwrap();
            let traversal_path = ToTraversalPath::<8>::to_traversal_path(&i, test_tree.height());
            let mut tx = storage.write().await.unwrap();
            UpdateStateData::<_, MockMerkleTree, 8>::insert_merkle_nodes(
                &mut tx,
                proof,
                traversal_path,
                (i + 1) as u64,
            )
            .await
            .expect("failed to insert nodes");
            tx.commit().await.unwrap();
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_merklized_storage_with_commit() {
        // This test insert a merkle path into the database and queries the path using the merkle commitment
        setup_test();

        let db = TmpDb::init().await;
        let storage = SqlStorage::connect(db.config()).await.unwrap();

        // define a test tree
        let mut test_tree = MockMerkleTree::new(MockMerkleTree::tree_height());
        let block_height = 1;
        //insert an entry into the tree
        test_tree.update(0, 0).unwrap();
        let commitment = test_tree.commitment();

        let test_data = serde_json::json!({ MockMerkleTree::header_state_commitment_field() : serde_json::to_value(commitment).unwrap()});
        // insert the header with merkle commitment
        let mut tx = storage.write().await.unwrap();
        tx.upsert(
            "header",
            ["height", "hash", "payload_hash", "timestamp", "data"],
            ["height"],
            [(
                block_height as i64,
                "randomString".to_string(),
                "t".to_string(),
                0,
                test_data,
            )],
        )
        .await
        .unwrap();
        // proof for the index from the tree
        let (_, proof) = test_tree.lookup(0).expect_ok().unwrap();
        // traversal path for the index.
        let traversal_path =
            <usize as ToTraversalPath<8>>::to_traversal_path(&0, test_tree.height());
        // insert merkle nodes
        UpdateStateData::<_, MockMerkleTree, 8>::insert_merkle_nodes(
            &mut tx,
            proof.clone(),
            traversal_path.clone(),
            block_height as u64,
        )
        .await
        .expect("failed to insert nodes");
        // update saved state height
        UpdateStateData::<_, MockMerkleTree, 8>::set_last_state_height(&mut tx, block_height)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let merkle_proof = storage
            .read()
            .await
            .unwrap()
            .get_path(Snapshot::<_, MockMerkleTree, 8>::Commit(commitment), 0)
            .await
            .unwrap();

        let (_, proof) = test_tree.lookup(0).expect_ok().unwrap();

        assert_eq!(merkle_proof, proof.clone(), "merkle paths mismatch");
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn test_merklized_state_missing_state() {
        // This test checks that header commitment matches the root hash.
        // For this, the header merkle root commitment field is not updated, which should result in an error
        // The full merkle path verification is also done by recomputing the root hash
        // An index and its corresponding merkle nodes with created (bh) = 1 are inserted.
        // The entry of the index is updated, and the updated nodes are inserted with created (bh) = 2.
        // A node which is in the traversal path with bh = 2 is deleted, so the get_path should return an error as an older version of one of the nodes is used.
        setup_test();

        let db = TmpDb::init().await;
        let storage = SqlStorage::connect(db.config()).await.unwrap();

        // define a test tree
        let mut test_tree = MockMerkleTree::new(MockMerkleTree::tree_height());
        let block_height = 1;
        //insert an entry into the tree

        let mut tx = storage.write().await.unwrap();
        for i in 0..27 {
            test_tree.update(i, i).unwrap();
            let test_data = serde_json::json!({ MockMerkleTree::header_state_commitment_field() : serde_json::to_value(test_tree.commitment()).unwrap()});
            // insert the header with merkle commitment
            tx.upsert(
                "header",
                ["height", "hash", "payload_hash", "timestamp", "data"],
                ["height"],
                [(
                    block_height as i64,
                    format!("rarndomString{i}"),
                    "t".to_string(),
                    0,
                    test_data,
                )],
            )
            .await
            .unwrap();
            // proof for the index from the tree
            let (_, proof) = test_tree.lookup(i).expect_ok().unwrap();
            // traversal path for the index.
            let traversal_path =
                <usize as ToTraversalPath<8>>::to_traversal_path(&i, test_tree.height());
            // insert merkle nodes
            UpdateStateData::<_, MockMerkleTree, 8>::insert_merkle_nodes(
                &mut tx,
                proof.clone(),
                traversal_path.clone(),
                block_height as u64,
            )
            .await
            .expect("failed to insert nodes");
            // update saved state height
            UpdateStateData::<_, MockMerkleTree, 8>::set_last_state_height(&mut tx, block_height)
                .await
                .unwrap();
        }

        test_tree.update(1, 100).unwrap();
        //insert updated merkle path without updating the header
        let traversal_path =
            <usize as ToTraversalPath<8>>::to_traversal_path(&1, test_tree.height());
        let (_, proof) = test_tree.lookup(1).expect_ok().unwrap();

        // insert merkle nodes
        UpdateStateData::<_, MockMerkleTree, 8>::insert_merkle_nodes(
            &mut tx,
            proof.clone(),
            traversal_path.clone(),
            block_height as u64,
        )
        .await
        .expect("failed to insert nodes");
        tx.commit().await.unwrap();

        let merkle_path = storage
            .read()
            .await
            .unwrap()
            .get_path(
                Snapshot::<_, MockMerkleTree, 8>::Index(block_height as u64),
                1,
            )
            .await;
        assert!(merkle_path.is_err());

        let test_data = serde_json::json!({ MockMerkleTree::header_state_commitment_field() : serde_json::to_value(test_tree.commitment()).unwrap()});
        // insert the header with merkle commitment
        let mut tx = storage.write().await.unwrap();
        tx.upsert(
            "header",
            ["height", "hash", "payload_hash", "timestamp", "data"],
            ["height"],
            [(
                block_height as i64,
                "randomStringgg".to_string(),
                "t".to_string(),
                0,
                test_data,
            )],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        // Querying the path again
        let merkle_proof = storage
            .read()
            .await
            .unwrap()
            .get_path(
                Snapshot::<_, MockMerkleTree, 8>::Index(block_height as u64),
                1,
            )
            .await
            .unwrap();
        assert_eq!(merkle_proof, proof, "path dont match");

        // Update the tree again for index 0 with created (bh) = 2
        // Delete one of the node in the traversal path
        test_tree.update(1, 200).unwrap();

        let (_, proof) = test_tree.lookup(1).expect_ok().unwrap();
        let test_data = serde_json::json!({ MockMerkleTree::header_state_commitment_field() : serde_json::to_value(test_tree.commitment()).unwrap()});

        // insert the header with merkle commitment
        let mut tx = storage.write().await.unwrap();
        tx.upsert(
            "header",
            ["height", "hash", "payload_hash", "timestamp", "data"],
            ["height"],
            [(
                2i64,
                "randomHashString".to_string(),
                "t".to_string(),
                0,
                test_data,
            )],
        )
        .await
        .unwrap();
        UpdateStateData::<_, MockMerkleTree, 8>::insert_merkle_nodes(
            &mut tx,
            proof.clone(),
            traversal_path.clone(),
            2_u64,
        )
        .await
        .expect("failed to insert nodes");

        // Deleting one internal node
        let node_path = traversal_path
            .iter()
            .skip(1)
            .rev()
            .map(|n| *n as i32)
            .collect::<Vec<_>>();
        tx.execute(
            query(&format!(
                "DELETE FROM {} WHERE created = 2 and path = $1",
                MockMerkleTree::state_type()
            ))
            .bind(serde_json::to_value(node_path).unwrap()),
        )
        .await
        .expect("failed to delete internal node");
        tx.commit().await.unwrap();

        let merkle_path = storage
            .read()
            .await
            .unwrap()
            .get_path(Snapshot::<_, MockMerkleTree, 8>::Index(2_u64), 1)
            .await;

        assert!(merkle_path.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_merklized_state_snapshot() {
        setup_test();

        let db = TmpDb::init().await;
        let storage = SqlStorage::connect(db.config()).await.unwrap();

        // Define a test tree
        let mut test_tree = MockMerkleTree::new(MockMerkleTree::tree_height());

        // We will sample random keys as u32. This is a value that is not a valid u32 and thus is a
        // key we will never insert into the tree.
        const RESERVED_KEY: usize = (u32::MAX as usize) + 1;

        // Randomly insert and delete some entries. For each entry we insert, we also keep track of
        // whether the entry should be in the tree using a HashMap.
        #[tracing::instrument(skip(tree, expected))]
        fn randomize(tree: &mut MockMerkleTree, expected: &mut HashMap<usize, Option<usize>>) {
            let mut rng = rand::thread_rng();
            tracing::info!("randomizing tree");

            for _ in 0..50 {
                // We flip a coin to decide whether to insert or delete, unless the tree is empty,
                // in which case we can only insert.
                if !expected.values().any(|v| v.is_some()) || rng.next_u32() % 2 == 0 {
                    // Insert.
                    let key = rng.next_u32() as usize;
                    let val = rng.next_u32() as usize;
                    tracing::info!(key, val, "inserting");

                    tree.update(key, val).unwrap();
                    expected.insert(key, Some(val));
                } else {
                    // Delete.
                    let key = expected
                        .iter()
                        .filter_map(|(k, v)| if v.is_some() { Some(k) } else { None })
                        .choose(&mut rng)
                        .unwrap();
                    tracing::info!(key, "deleting");

                    tree.remove(key).unwrap();
                    expected.insert(*key, None);
                }
            }
        }

        // Commit the tree to storage.
        #[tracing::instrument(skip(storage, tree, expected))]
        async fn store(
            storage: &SqlStorage,
            tree: &MockMerkleTree,
            expected: &HashMap<usize, Option<usize>>,
            block_height: u64,
        ) {
            tracing::info!("persisting tree");
            let mut tx = storage.write().await.unwrap();

            for key in expected.keys() {
                let proof = match tree.universal_lookup(key) {
                    LookupResult::Ok(_, proof) => proof,
                    LookupResult::NotFound(proof) => proof,
                    LookupResult::NotInMemory => panic!("failed to find key {key}"),
                };
                let traversal_path = ToTraversalPath::<8>::to_traversal_path(key, tree.height());
                UpdateStateData::<_, MockMerkleTree, 8>::insert_merkle_nodes(
                    &mut tx,
                    proof,
                    traversal_path,
                    block_height,
                )
                .await
                .unwrap();
            }
            // insert the header with merkle commitment
            tx
            .upsert("header", ["height", "hash", "payload_hash", "timestamp", "data"], ["height"],
                [(
                    block_height as i64,
                    format!("hash{block_height}"),
                    "hash".to_string(),
                    0i64,
                    serde_json::json!({ MockMerkleTree::header_state_commitment_field() : serde_json::to_value(tree.commitment()).unwrap()}),
                )],
            )
            .await
            .unwrap();
            UpdateStateData::<MockTypes, MockMerkleTree, 8>::set_last_state_height(
                &mut tx,
                block_height as usize,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
        }

        #[tracing::instrument(skip(storage, tree, expected))]
        async fn validate(
            storage: &SqlStorage,
            tree: &MockMerkleTree,
            expected: &HashMap<usize, Option<usize>>,
            block_height: u64,
        ) {
            tracing::info!("validating snapshot");

            // Check that we can get a correct path for each key that we touched.
            let snapshot = Snapshot::<_, MockMerkleTree, 8>::Index(block_height);

            for (key, val) in expected {
                let proof = match tree.universal_lookup(key) {
                    LookupResult::Ok(_, proof) => proof,
                    LookupResult::NotFound(proof) => proof,
                    LookupResult::NotInMemory => panic!("failed to find key {key}"),
                };
                assert_eq!(
                    proof,
                    storage
                        .read()
                        .await
                        .unwrap()
                        .get_path(snapshot, *key)
                        .await
                        .unwrap()
                );
                assert_eq!(val.as_ref(), proof.elem());
                // Check path is valid for test_tree
                if val.is_some() {
                    MockMerkleTree::verify(tree.commitment().digest(), key, proof)
                        .unwrap()
                        .unwrap();
                } else {
                    assert!(tree.non_membership_verify(key, proof).unwrap());
                }
            }

            // Check that we can even get a non-membership proof for a key that we never touched.
            let proof = match tree.universal_lookup(RESERVED_KEY) {
                LookupResult::Ok(_, proof) => proof,
                LookupResult::NotFound(proof) => proof,
                LookupResult::NotInMemory => panic!("failed to find reserved key {RESERVED_KEY}"),
            };
            assert_eq!(
                proof,
                storage
                    .read()
                    .await
                    .unwrap()
                    .get_path(snapshot, RESERVED_KEY)
                    .await
                    .unwrap()
            );
            assert_eq!(proof.elem(), None);
            // Check path is valid for test_tree
            assert!(tree.non_membership_verify(RESERVED_KEY, proof).unwrap());
        }

        // Create a randomized Merkle tree.
        let mut expected = HashMap::<usize, Option<usize>>::new();
        randomize(&mut test_tree, &mut expected);

        // Commit the randomized tree to storage.
        store(&storage, &test_tree, &expected, 1).await;
        validate(&storage, &test_tree, &expected, 1).await;

        // Make random edits and commit another snapshot.
        let mut expected2 = expected.clone();
        let mut test_tree2 = test_tree.clone();
        randomize(&mut test_tree2, &mut expected2);
        store(&storage, &test_tree2, &expected2, 2).await;
        validate(&storage, &test_tree2, &expected2, 2).await;

        // Ensure the original snapshot is still valid.
        validate(&storage, &test_tree, &expected, 1).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_merklized_state_missing_leaf() {
        // Check that if a leaf is missing but its ancestors are present/key is in the tree, we
        // catch it rather than interpreting the entry as an empty node by default. Note that this
        // scenario should be impossible in normal usage, since we never store or delete partial
        // paths. But we should never return an invalid proof even in extreme cases like database
        // corruption.
        setup_test();

        for tree_size in 1..=3 {
            let db = TmpDb::init().await;
            let storage = SqlStorage::connect(db.config()).await.unwrap();

            // Define a test tree
            let mut test_tree = MockMerkleTree::new(MockMerkleTree::tree_height());
            for i in 0..tree_size {
                test_tree.update(i, i).unwrap();
            }

            let mut tx = storage.write().await.unwrap();

            // Insert a header with the tree commitment.
            tx.upsert(
                "header",
                ["height", "hash", "payload_hash", "timestamp", "data"],
                ["height"],
                [(
                    0i64,
                    "hash".to_string(),
                    "hash".to_string(),
                    0,
                    serde_json::json!({ MockMerkleTree::header_state_commitment_field() : serde_json::to_value(test_tree.commitment()).unwrap()}),
                )],
            )
            .await
            .unwrap();

            // Insert Merkle nodes.
            for i in 0..tree_size {
                let proof = test_tree.lookup(i).expect_ok().unwrap().1;
                let traversal_path =
                    ToTraversalPath::<8>::to_traversal_path(&i, test_tree.height());
                UpdateStateData::<_, MockMerkleTree, 8>::insert_merkle_nodes(
                    &mut tx,
                    proof,
                    traversal_path,
                    0,
                )
                .await
                .unwrap();
            }
            UpdateStateData::<_, MockMerkleTree, 8>::set_last_state_height(&mut tx, 0)
                .await
                .unwrap();
            tx.commit().await.unwrap();

            // Test that we can get all the entries.
            let snapshot = Snapshot::<MockTypes, MockMerkleTree, 8>::Index(0);
            for i in 0..tree_size {
                let proof = test_tree.lookup(i).expect_ok().unwrap().1;
                assert_eq!(
                    proof,
                    storage
                        .read()
                        .await
                        .unwrap()
                        .get_path(snapshot, i)
                        .await
                        .unwrap()
                );
                assert_eq!(*proof.elem().unwrap(), i);
            }

            // Now delete the leaf node for the last entry we inserted, corrupting the database.
            let index = serde_json::to_value(tree_size - 1).unwrap();
            let mut tx = storage.write().await.unwrap();

            tx.execute(
                query(&format!(
                    "DELETE FROM {} WHERE idx = $1",
                    MockMerkleTree::state_type()
                ))
                .bind(serde_json::to_value(index).unwrap()),
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();

            // Test that we can still get the entries we didn't delete.
            for i in 0..tree_size - 1 {
                let proof = test_tree.lookup(i).expect_ok().unwrap().1;
                assert_eq!(
                    proof,
                    storage
                        .read()
                        .await
                        .unwrap()
                        .get_path(snapshot, i)
                        .await
                        .unwrap()
                );
                assert_eq!(*proof.elem().unwrap(), i);
            }

            // Looking up the entry we deleted fails, rather than return an invalid path.
            let err = storage
                .read()
                .await
                .unwrap()
                .get_path(snapshot, tree_size - 1)
                .await
                .unwrap_err();
            assert!(matches!(err, QueryError::Missing));
        }
    }
}
