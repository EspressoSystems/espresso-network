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

use anyhow::Context;
use ark_serialize::CanonicalDeserialize;
use async_trait::async_trait;
use futures::stream::TryStreamExt;
use hotshot_types::traits::node_implementation::NodeType;
use jf_merkle_tree_compat::{
    prelude::{MerkleNode, MerkleProof},
    DigestAlgorithm, MerkleCommitment, ToTraversalPath,
};
use sqlx::types::{BitVec, JsonValue};

use super::{
    super::{
        db::{with_backend, DbBackend},
        transaction::{Transaction, TransactionMode, Write},
    },
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
    async fn get_path(
        &mut self,
        snapshot: Snapshot<Types, State, ARITY>,
        key: State::Key,
    ) -> QueryResult<MerkleProof<State::Entry, State::Key, State::T, ARITY>> {
        let state_type = State::state_type();
        let tree_height = State::tree_height();

        let traversal_path = State::Key::to_traversal_path(&key, tree_height);
        let (created, merkle_commitment) = self.snapshot_info(snapshot).await?;

        let backend = self.backend();
        let (query, sql) =
            build_get_path_query(state_type, traversal_path.clone(), created, backend)?;
        let rows = query.query(&sql).fetch_all(self).await?;

        let nodes: Vec<Node> = rows.into_iter().map(Node::from_backend_row).collect();

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

        let hashes = if !hash_ids.is_empty() {
            let (query, sql) =
                build_where_in("SELECT id, value FROM hash", "id", hash_ids, backend)?;
            query
                .query_as::<(i32, Vec<u8>)>(&sql)
                .fetch(self)
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
                        proof_path.push_back(MerkleNode::Branch {
                            value: State::T::deserialize_compressed(value.as_slice())
                                .decode_error("malformed merkle node value")?,
                            children: child_nodes,
                        });
                    },
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

        let init = if let Some(MerkleNode::Leaf { value, .. }) = proof_path.front() {
            *value
        } else {
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
                message: format!(
                    "Commitment calculated from merkle path ({commitment_from_path:?}) does not \
                     match the commitment in the header ({:?})",
                    merkle_commitment.digest()
                ),
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
        let result: Option<(i64,)> = with_backend!(self, |tx| {
            sqlx::query_as("SELECT height from last_merklized_state_height")
                .fetch_optional(tx.as_mut())
                .await
        })?;
        let Some((height,)) = result else {
            return Ok(0);
        };
        Ok(height as usize)
    }
}

impl<Mode: TransactionMode> Transaction<Mode> {
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
                let sql = format!(
                    "SELECT height
                       FROM header
                      WHERE {header_state_commitment_field} = $1
                      LIMIT 1"
                );
                let (height,): (i64,) = with_backend!(self, |tx| {
                    sqlx::query_as(&sql)
                        .bind(commit.to_string())
                        .fetch_one(tx.as_mut())
                        .await
                })?;

                (height, commit)
            },
            Snapshot::Index(created) => {
                let created = created as i64;
                let sql = format!(
                    "SELECT {header_state_commitment_field} AS root_commitment
                       FROM header
                      WHERE height = $1
                      LIMIT 1"
                );
                let (commit_str,): (String,) = with_backend!(self, |tx| {
                    sqlx::query_as(&sql)
                        .bind(created)
                        .fetch_one(tx.as_mut())
                        .await
                })?;
                let commit = serde_json::from_value(commit_str.into())
                    .decode_error("malformed state commitment")?;
                (created, commit)
            },
        };

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

pub(crate) fn build_hash_batch_insert(
    hashes: &[Vec<u8>],
    backend: DbBackend,
) -> QueryResult<(QueryBuilder<'_>, String)> {
    let mut query = QueryBuilder::new(backend);
    let params = hashes
        .iter()
        .map(|hash| Ok(format!("({})", query.bind(hash)?)))
        .collect::<QueryResult<Vec<String>>>()?;
    let sql = format!(
        "INSERT INTO hash(value) values {} ON CONFLICT (value) DO UPDATE SET value = \
         EXCLUDED.value returning value, id",
        params.join(",")
    );
    Ok((query, sql))
}

pub(crate) async fn batch_insert_hashes(
    hashes: Vec<Vec<u8>>,
    tx: &mut Transaction<Write>,
) -> QueryResult<HashMap<Vec<u8>, i32>> {
    if hashes.is_empty() {
        return Ok(HashMap::new());
    }

    let sql = "INSERT INTO hash(value) SELECT * FROM UNNEST($1::bytea[]) ON CONFLICT (value) DO \
               UPDATE SET value = EXCLUDED.value RETURNING value, id";

    let result: HashMap<Vec<u8>, i32> = match &mut tx.inner {
        super::super::db::BackendTransaction::Postgres(inner) => sqlx::query_as(sql)
            .bind(&hashes)
            .fetch(inner.as_mut())
            .try_collect()
            .await
            .map_err(|e| QueryError::Error {
                message: format!("batch hash insert failed: {e}"),
            })?,
        super::super::db::BackendTransaction::Sqlite(_) => {
            return Err(QueryError::Error {
                message: "batch_insert_hashes with UNNEST is only supported on Postgres"
                    .to_string(),
            });
        },
    };

    Ok(result)
}

pub(crate) type ProofWithPath<Entry, Key, T, const ARITY: usize> =
    (MerkleProof<Entry, Key, T, ARITY>, Vec<usize>);

pub(crate) fn collect_nodes_from_proofs<Entry, Key, T, const ARITY: usize>(
    proofs: &[ProofWithPath<Entry, Key, T, ARITY>],
) -> QueryResult<(Vec<NodeWithHashes>, HashSet<Vec<u8>>)>
where
    Entry: jf_merkle_tree_compat::Element + serde::Serialize,
    Key: jf_merkle_tree_compat::Index + serde::Serialize,
    T: jf_merkle_tree_compat::NodeValue,
{
    let mut nodes = Vec::new();
    let mut hashes = HashSet::new();

    for (proof, traversal_path) in proofs {
        let pos = &proof.pos;
        let path = &proof.proof;
        let mut trav_path = traversal_path.iter().map(|n| *n as i32);

        for node in path.iter() {
            match node {
                MerkleNode::Empty => {
                    let index =
                        serde_json::to_value(pos.clone()).map_err(|e| QueryError::Error {
                            message: format!("malformed merkle position: {e}"),
                        })?;
                    let node_path: Vec<i32> = trav_path.clone().rev().collect();
                    nodes.push((
                        Node {
                            path: node_path.into(),
                            idx: Some(index),
                            ..Default::default()
                        },
                        None,
                        [0_u8; 32].to_vec(),
                    ));
                    hashes.insert([0_u8; 32].to_vec());
                },
                MerkleNode::ForgettenSubtree { .. } => {
                    return Err(QueryError::Error {
                        message: "Node in the Merkle path contains a forgotten subtree".into(),
                    });
                },
                MerkleNode::Leaf { value, pos, elem } => {
                    let mut leaf_commit = Vec::new();
                    value.serialize_compressed(&mut leaf_commit).map_err(|e| {
                        QueryError::Error {
                            message: format!("malformed merkle leaf commitment: {e}"),
                        }
                    })?;

                    let node_path: Vec<i32> = trav_path.clone().rev().collect();

                    let index =
                        serde_json::to_value(pos.clone()).map_err(|e| QueryError::Error {
                            message: format!("malformed merkle position: {e}"),
                        })?;
                    let entry = serde_json::to_value(elem).map_err(|e| QueryError::Error {
                        message: format!("malformed merkle element: {e}"),
                    })?;

                    nodes.push((
                        Node {
                            path: node_path.into(),
                            idx: Some(index),
                            entry: Some(entry),
                            ..Default::default()
                        },
                        None,
                        leaf_commit.clone(),
                    ));

                    hashes.insert(leaf_commit);
                },
                MerkleNode::Branch { value, children } => {
                    let mut branch_hash = Vec::new();
                    value.serialize_compressed(&mut branch_hash).map_err(|e| {
                        QueryError::Error {
                            message: format!("malformed merkle branch hash: {e}"),
                        }
                    })?;

                    let mut children_bitvec = BitVec::new();
                    let mut children_values = Vec::new();
                    for child in children {
                        let child = child.as_ref();
                        match child {
                            MerkleNode::Empty => {
                                children_bitvec.push(false);
                            },
                            MerkleNode::Branch { value, .. }
                            | MerkleNode::Leaf { value, .. }
                            | MerkleNode::ForgettenSubtree { value } => {
                                let mut hash = Vec::new();
                                value.serialize_compressed(&mut hash).map_err(|e| {
                                    QueryError::Error {
                                        message: format!("malformed merkle node hash: {e}"),
                                    }
                                })?;

                                children_values.push(hash);
                                children_bitvec.push(true);
                            },
                        }
                    }

                    let node_path: Vec<i32> = trav_path.clone().rev().collect();
                    nodes.push((
                        Node {
                            path: node_path.into(),
                            children: None,
                            children_bitvec: Some(children_bitvec),
                            ..Default::default()
                        },
                        Some(children_values.clone()),
                        branch_hash.clone(),
                    ));
                    hashes.insert(branch_hash);
                    hashes.extend(children_values);
                },
            }

            trav_path.next();
        }
    }

    Ok((nodes, hashes))
}

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

pub(crate) type NodeWithHashes = (Node, Option<Vec<Vec<u8>>>, Vec<u8>);

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
    pub(crate) fn from_backend_row(row: super::BackendRow) -> Self {
        match row {
            super::BackendRow::Postgres(row) => Self::from(row),
            super::BackendRow::Sqlite(row) => Self::from(row),
        }
    }

    pub(crate) async fn upsert(
        name: &str,
        nodes: impl IntoIterator<Item = Self>,
        tx: &mut Transaction<Write>,
    ) -> anyhow::Result<()> {
        let nodes: Vec<_> = nodes.into_iter().collect();

        match tx.backend() {
            DbBackend::Postgres => Self::upsert_batch_unnest(name, nodes, tx).await,
            DbBackend::Sqlite => {
                for node_chunk in nodes.chunks(20) {
                    let rows: Vec<_> = node_chunk
                        .iter()
                        .map(|n| {
                            let children_bitvec: Option<String> = n
                                .children_bitvec
                                .clone()
                                .map(|b| b.iter().map(|bit| if bit { '1' } else { '0' }).collect());

                            (
                                n.path.clone(),
                                n.created,
                                n.hash_id,
                                n.children.clone(),
                                children_bitvec,
                                n.idx.clone(),
                                n.entry.clone(),
                            )
                        })
                        .collect();

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
                        rows,
                    )
                    .await?;
                }
                Ok(())
            },
        }
    }

    async fn upsert_batch_unnest(
        name: &str,
        nodes: Vec<Self>,
        tx: &mut Transaction<Write>,
    ) -> anyhow::Result<()> {
        if nodes.is_empty() {
            return Ok(());
        }

        let mut deduped = HashMap::new();
        for node in nodes {
            deduped.insert((node.path.to_string(), node.created), node);
        }

        let mut paths = Vec::with_capacity(deduped.len());
        let mut createds = Vec::with_capacity(deduped.len());
        let mut hash_ids = Vec::with_capacity(deduped.len());
        let mut childrens = Vec::with_capacity(deduped.len());
        let mut children_bitvecs = Vec::with_capacity(deduped.len());
        let mut idxs = Vec::with_capacity(deduped.len());
        let mut entries = Vec::with_capacity(deduped.len());

        for node in deduped.into_values() {
            paths.push(node.path);
            createds.push(node.created);
            hash_ids.push(node.hash_id);
            childrens.push(node.children);
            children_bitvecs.push(node.children_bitvec);
            idxs.push(node.idx);
            entries.push(node.entry);
        }

        let sql = format!(
            r#"
            INSERT INTO "{name}" (path, created, hash_id, children, children_bitvec, idx, entry)
            SELECT * FROM UNNEST($1::jsonb[], $2::bigint[], $3::int[], $4::jsonb[], $5::bit varying[], $6::jsonb[], $7::jsonb[])
            ON CONFLICT (path, created) DO UPDATE SET
                hash_id = EXCLUDED.hash_id,
                children = EXCLUDED.children,
                children_bitvec = EXCLUDED.children_bitvec,
                idx = EXCLUDED.idx,
                entry = EXCLUDED.entry
            "#
        );

        match &mut tx.inner {
            super::super::db::BackendTransaction::Postgres(inner) => {
                sqlx::query(&sql)
                    .bind(&paths)
                    .bind(&createds)
                    .bind(&hash_ids)
                    .bind(&childrens)
                    .bind(&children_bitvecs)
                    .bind(&idxs)
                    .bind(&entries)
                    .execute(inner.as_mut())
                    .await
                    .context("batch upsert with UNNEST failed")?;
            },
            super::super::db::BackendTransaction::Sqlite(_) => {
                anyhow::bail!("upsert_batch_unnest is only supported on Postgres");
            },
        }

        Ok(())
    }
}

fn build_get_path_query<'q>(
    table: &'static str,
    traversal_path: Vec<usize>,
    created: i64,
    backend: DbBackend,
) -> QueryResult<(QueryBuilder<'q>, String)> {
    let mut query = QueryBuilder::new(backend);
    let mut traversal_path = traversal_path.into_iter().map(|x| x as i32);

    let len = traversal_path.len();
    let mut sub_queries = Vec::new();

    query.bind(created)?;

    for _ in 0..=len {
        let path = traversal_path.clone().rev().collect::<Vec<_>>();
        let path: serde_json::Value = path.into();
        let node_path = query.bind(path)?;

        let sub_query = format!(
            "SELECT * FROM (SELECT * FROM {table} WHERE path = {node_path} AND created <= $1 \
             ORDER BY created DESC LIMIT 1) AS latest_node",
        );

        sub_queries.push(sub_query);
        traversal_path.next();
    }

    let mut sql: String = sub_queries.join(" UNION ");

    sql = format!("SELECT * FROM ({sql}) as t ");

    match backend {
        DbBackend::Sqlite => sql.push_str("ORDER BY length(t.path) DESC"),
        DbBackend::Postgres => sql.push_str("ORDER BY t.path DESC"),
    }

    Ok((query, sql))
}

#[cfg(test)]
mod test {
    use futures::stream::StreamExt;
    use jf_merkle_tree_compat::{
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
        testing::mocks::{MockMerkleTree, MockTypes},
    };

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_merklized_state_storage() {
        let db = TmpDb::init().await;
        let storage = SqlStorage::connect(db.config(), StorageConnectionType::Query)
            .await
            .unwrap();

        let mut test_tree: UniversalMerkleTree<_, _, _, 8, _> =
            MockMerkleTree::new(MockMerkleTree::tree_height());
        let block_height = 1;

        let mut tx = storage.write().await.unwrap();
        for i in 0..27 {
            test_tree.update(i, i).unwrap();

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
            let (_, proof) = test_tree.lookup(i).expect_ok().unwrap();
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
        UpdateStateData::<_, MockMerkleTree, 8>::set_last_state_height(&mut tx, block_height)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        for i in 0..27 {
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

            assert_eq!(merkle_path, proof.clone(), "merkle paths mismatch");
        }

        let (_, proof_bh_1) = test_tree.lookup(0).expect_ok().unwrap();
        test_tree.update(0, 99).unwrap();

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
        let traversal_path =
            <usize as ToTraversalPath<8>>::to_traversal_path(&0, test_tree.height());

        UpdateStateData::<_, MockMerkleTree, 8>::insert_merkle_nodes(
            &mut tx,
            proof_bh_2.clone(),
            traversal_path.clone(),
            2,
        )
        .await
        .expect("failed to insert nodes");
        UpdateStateData::<_, MockMerkleTree, 8>::set_last_state_height(&mut tx, 2)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let node_path = traversal_path
            .into_iter()
            .rev()
            .map(|n| n as i32)
            .collect::<Vec<_>>();

        let mut tx = storage.read().await.unwrap();
        let nodes: Vec<Node> = with_backend!(tx, |inner_tx| {
            sqlx::query("SELECT * from test_tree where path = $1 ORDER BY created")
                .bind(serde_json::to_value(&node_path).unwrap())
                .fetch(inner_tx.as_mut())
                .map(|res| Node::from(res.unwrap()))
                .collect()
                .await
        });
        assert!(nodes.len() == 2, "incorrect number of nodes");
        assert_eq!(nodes[0].created, 1, "wrong block height");
        assert_eq!(nodes[1].created, 2, "wrong block height");

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

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_merklized_state_non_membership_proof() {
        let db = TmpDb::init().await;
        let storage = SqlStorage::connect(db.config(), StorageConnectionType::Query)
            .await
            .unwrap();

        let mut test_tree = MockMerkleTree::new(MockMerkleTree::tree_height());
        let block_height = 1;
        test_tree.update(0, 0).unwrap();
        let commitment = test_tree.commitment();

        let test_data = serde_json::json!({ MockMerkleTree::header_state_commitment_field() : serde_json::to_value(commitment).unwrap()});
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
        let (_, proof_before_remove) = test_tree.lookup(0).expect_ok().unwrap();
        let traversal_path =
            <usize as ToTraversalPath<8>>::to_traversal_path(&0, test_tree.height());
        UpdateStateData::<_, MockMerkleTree, 8>::insert_merkle_nodes(
            &mut tx,
            proof_before_remove.clone(),
            traversal_path.clone(),
            block_height as u64,
        )
        .await
        .expect("failed to insert nodes");
        UpdateStateData::<_, MockMerkleTree, 8>::set_last_state_height(&mut tx, block_height)
            .await
            .unwrap();
        tx.commit().await.unwrap();
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

        assert_eq!(
            merkle_path,
            proof_before_remove.clone(),
            "merkle paths mismatch"
        );

        test_tree.remove(0).expect("failed to delete index 0 ");

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
        UpdateStateData::<_, MockMerkleTree, 8>::set_last_state_height(&mut tx, 2)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let non_membership_path = storage
            .read()
            .await
            .unwrap()
            .get_path(Snapshot::<_, MockMerkleTree, 8>::Index(2_u64), 0)
            .await
            .unwrap();
        assert_eq!(
            non_membership_path, proof_after_remove,
            "merkle paths dont match"
        );

        let proof_bh_1 = storage
            .read()
            .await
            .unwrap()
            .get_path(Snapshot::<_, MockMerkleTree, 8>::Index(1_u64), 0)
            .await
            .unwrap();
        assert_eq!(proof_bh_1, proof_before_remove, "merkle paths dont match");
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_merklized_state_non_membership_proof_unseen_entry() {
        let db = TmpDb::init().await;
        let storage = SqlStorage::connect(db.config(), StorageConnectionType::Query)
            .await
            .unwrap();

        let mut test_tree = MockMerkleTree::new(MockMerkleTree::tree_height());

        for i in 0..=2 {
            tracing::info!(i, ?test_tree, "testing non-membership proof");
            let mut tx = storage.write().await.unwrap();

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
            UpdateStateData::<_, MockMerkleTree, 8>::set_last_state_height(&mut tx, i)
                .await
                .unwrap();
            tx.commit().await.unwrap();

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

            assert!(
                MockMerkleTree::non_membership_verify(test_tree.commitment(), 100, proof).unwrap()
            );

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

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_merklized_storage_with_commit() {
        let db = TmpDb::init().await;
        let storage = SqlStorage::connect(db.config(), StorageConnectionType::Query)
            .await
            .unwrap();

        let mut test_tree = MockMerkleTree::new(MockMerkleTree::tree_height());
        let block_height = 1;
        test_tree.update(0, 0).unwrap();
        let commitment = test_tree.commitment();

        let test_data = serde_json::json!({ MockMerkleTree::header_state_commitment_field() : serde_json::to_value(commitment).unwrap()});
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
        let (_, proof) = test_tree.lookup(0).expect_ok().unwrap();
        let traversal_path =
            <usize as ToTraversalPath<8>>::to_traversal_path(&0, test_tree.height());
        UpdateStateData::<_, MockMerkleTree, 8>::insert_merkle_nodes(
            &mut tx,
            proof.clone(),
            traversal_path.clone(),
            block_height as u64,
        )
        .await
        .expect("failed to insert nodes");
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
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_merklized_state_missing_state() {
        let db = TmpDb::init().await;
        let storage = SqlStorage::connect(db.config(), StorageConnectionType::Query)
            .await
            .unwrap();

        let mut test_tree = MockMerkleTree::new(MockMerkleTree::tree_height());
        let block_height = 1;

        let mut tx = storage.write().await.unwrap();
        for i in 0..27 {
            test_tree.update(i, i).unwrap();
            let test_data = serde_json::json!({ MockMerkleTree::header_state_commitment_field() : serde_json::to_value(test_tree.commitment()).unwrap()});
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
            let (_, proof) = test_tree.lookup(i).expect_ok().unwrap();
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
            UpdateStateData::<_, MockMerkleTree, 8>::set_last_state_height(&mut tx, block_height)
                .await
                .unwrap();
        }

        test_tree.update(1, 100).unwrap();
        let traversal_path =
            <usize as ToTraversalPath<8>>::to_traversal_path(&1, test_tree.height());
        let (_, proof) = test_tree.lookup(1).expect_ok().unwrap();

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

        test_tree.update(1, 200).unwrap();

        let (_, proof) = test_tree.lookup(1).expect_ok().unwrap();
        let test_data = serde_json::json!({ MockMerkleTree::header_state_commitment_field() : serde_json::to_value(test_tree.commitment()).unwrap()});

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

        let node_path = traversal_path
            .iter()
            .skip(1)
            .rev()
            .map(|n| *n as i32)
            .collect::<Vec<_>>();
        with_backend!(tx, |inner_tx| {
            sqlx::query(&format!(
                "DELETE FROM {} WHERE created = 2 and path = $1",
                MockMerkleTree::state_type()
            ))
            .bind(serde_json::to_value(node_path).unwrap())
            .execute(inner_tx.as_mut())
            .await
            .map(|_| ())
        })
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

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_merklized_state_snapshot() {
        let db = TmpDb::init().await;
        let storage = SqlStorage::connect(db.config(), StorageConnectionType::Query)
            .await
            .unwrap();

        let mut test_tree = MockMerkleTree::new(MockMerkleTree::tree_height());

        const RESERVED_KEY: usize = (u32::MAX as usize) + 1;

        #[tracing::instrument(skip(tree, expected))]
        fn randomize(tree: &mut MockMerkleTree, expected: &mut HashMap<usize, Option<usize>>) {
            let mut rng = rand::thread_rng();
            tracing::info!("randomizing tree");

            for _ in 0..50 {
                if !expected.values().any(|v| v.is_some()) || rng.next_u32().is_multiple_of(2) {
                    let key = rng.next_u32() as usize;
                    let val = rng.next_u32() as usize;
                    tracing::info!(key, val, "inserting");

                    tree.update(key, val).unwrap();
                    expected.insert(key, Some(val));
                } else {
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
                if val.is_some() {
                    MockMerkleTree::verify(tree.commitment(), key, proof)
                        .unwrap()
                        .unwrap();
                } else {
                    assert!(
                        MockMerkleTree::non_membership_verify(tree.commitment(), key, proof)
                            .unwrap()
                    );
                }
            }

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
            assert!(
                MockMerkleTree::non_membership_verify(tree.commitment(), RESERVED_KEY, proof)
                    .unwrap()
            );
        }

        let mut expected = HashMap::<usize, Option<usize>>::new();
        randomize(&mut test_tree, &mut expected);

        store(&storage, &test_tree, &expected, 1).await;
        validate(&storage, &test_tree, &expected, 1).await;

        let mut expected2 = expected.clone();
        let mut test_tree2 = test_tree.clone();
        randomize(&mut test_tree2, &mut expected2);
        store(&storage, &test_tree2, &expected2, 2).await;
        validate(&storage, &test_tree2, &expected2, 2).await;

        validate(&storage, &test_tree, &expected, 1).await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_merklized_state_missing_leaf() {
        for tree_size in 1..=3 {
            let db = TmpDb::init().await;
            let storage = SqlStorage::connect(db.config(), StorageConnectionType::Query)
                .await
                .unwrap();

            let mut test_tree = MockMerkleTree::new(MockMerkleTree::tree_height());
            for i in 0..tree_size {
                test_tree.update(i, i).unwrap();
            }

            let mut tx = storage.write().await.unwrap();

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

            let index = serde_json::to_value(tree_size - 1).unwrap();
            let mut tx = storage.write().await.unwrap();

            with_backend!(tx, |inner_tx| {
                sqlx::query(&format!(
                    "DELETE FROM {} WHERE idx = $1",
                    MockMerkleTree::state_type()
                ))
                .bind(serde_json::to_value(index).unwrap())
                .execute(inner_tx.as_mut())
                .await
                .map(|_| ())
            })
            .unwrap();
            tx.commit().await.unwrap();

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
