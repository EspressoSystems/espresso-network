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

//! SQL transactions
//!
//! A transaction encapsulates all the mutable functionality provided by the SQL database, and
//! allows for mutable operations to be combined into complex updates that affect the main database
//! atomically. A transaction also provides all the immutable query functionality of a regular
//! database connection, so that the updated state of the database can be queried midway through a
//! transaction.

use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
    time::Instant,
};

use anyhow::{bail, Context};
use ark_serialize::CanonicalSerialize;
use async_trait::async_trait;
use committable::Committable;
use derive_more::{Deref, DerefMut};
use futures::{future::Future, stream::TryStreamExt};
use hotshot_types::{
    data::VidShare,
    traits::{
        block_contents::BlockHeader,
        metrics::{Counter, Gauge, Histogram, Metrics},
        node_implementation::{ConsensusTime, NodeType},
        EncodeBytes,
    },
};
use itertools::Itertools;
use jf_merkle_tree::prelude::{MerkleNode, MerkleProof};
pub use sqlx::Executor;
use sqlx::{
    pool::Pool, query_builder::Separated, types::BitVec, Encode, FromRow, QueryBuilder, Type,
};

use super::{
    queries::{
        self,
        state::{build_hash_batch_insert, Node},
        DecodeError,
    },
    Database, Db,
};
use crate::{
    availability::{
        BlockQueryData, LeafQueryData, QueryableHeader, QueryablePayload, StateCertQueryData,
        VidCommonQueryData,
    },
    data_source::{
        storage::{pruning::PrunedHeightStorage, UpdateAvailabilityStorage},
        update,
    },
    merklized_state::{MerklizedState, UpdateStateData},
    types::HeightIndexed,
    Header, Payload, QueryError, QueryResult,
};

pub type Query<'q> = sqlx::query::Query<'q, Db, <Db as Database>::Arguments<'q>>;
pub type QueryAs<'q, T> = sqlx::query::QueryAs<'q, Db, T, <Db as Database>::Arguments<'q>>;

pub fn query(sql: &str) -> Query<'_> {
    sqlx::query(sql)
}

pub fn query_as<'q, T>(sql: &'q str) -> QueryAs<'q, T>
where
    T: for<'r> FromRow<'r, <Db as Database>::Row>,
{
    sqlx::query_as(sql)
}

/// Marker type indicating a transaction with read-write access to the database.
#[derive(Clone, Copy, Debug, Default)]
pub struct Write;

/// Marker type indicating a transaction with read-only access to the database.
#[derive(Clone, Copy, Debug, Default)]
pub struct Read;

/// Trait for marker types indicating what type of access a transaction has to the database.
pub trait TransactionMode: Send + Sync {
    fn begin(
        conn: &mut <Db as Database>::Connection,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    fn display() -> &'static str;
}

impl TransactionMode for Write {
    #[allow(unused_variables)]
    async fn begin(conn: &mut <Db as Database>::Connection) -> anyhow::Result<()> {
        // SQLite automatically sets the read/write mode of a transactions based on the statements
        // in it. However, there is still a good reason to explicitly enable write mode right from
        // the start: if a transaction first executes a read statement and then a write statement,
        // it will be upgraded from a read transaction to a write transaction. Because this involves
        // obtaining a different kind of lock while already holding one, it can cause a deadlock,
        // e.g.:
        // * Transaction A executes a read statement, obtaining a read lock
        // * Transaction B executes a write statement and begins waiting for a write lock
        // * Transaction A executes a write statement and begins waiting for a write lock
        //
        // Transaction A can never obtain its write lock because it must first wait for transaction
        // B to get a write lock, which cannot happen because B is in turn waiting for A to release
        // its read lock.
        //
        // This type of deadlock cannot happen if transaction A immediately starts as a write, since
        // it will then only ever try to acquire one type of lock (a write lock). By working with
        // this restriction (transactions are either readers or writers, but never upgradable), we
        // avoid deadlock, we more closely imitate the concurrency semantics of postgres, and we
        // take advantage of the SQLite busy timeout, which may allow a transaction to acquire a
        // lock and succeed (after a small delay), even when there was a conflicting transaction in
        // progress. Whereas a deadlock is always an automatic rollback.
        //
        // The proper way to begin a write transaction in SQLite is with `BEGIN IMMEDIATE`. However,
        // sqlx does not expose any way to customize the `BEGIN` statement that starts a
        // transaction. A serviceable workaround is to perform some write statement before performing
        // any read statement, ensuring that the first lock we acquire is exclusive. A write
        // statement that has no actual effect on the database is suitable for this purpose, hence
        // the `WHERE false`.
        #[cfg(feature = "embedded-db")]
        conn.execute("UPDATE pruned_height SET id = id WHERE false")
            .await?;

        // With Postgres things are much more straightforward: just tell Postgres we want a write
        // transaction immediately after opening it.
        #[cfg(not(feature = "embedded-db"))]
        conn.execute("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
            .await?;

        Ok(())
    }

    fn display() -> &'static str {
        "write"
    }
}

impl TransactionMode for Read {
    #[allow(unused_variables)]
    async fn begin(conn: &mut <Db as Database>::Connection) -> anyhow::Result<()> {
        // With Postgres, we explicitly set the transaction mode to specify that we want the
        // strongest possible consistency semantics in case of competing transactions
        // (SERIALIZABLE), and we want to wait until this is possible rather than failing
        // (DEFERRABLE).
        //
        // With SQLite, there is nothing to be done here, as SQLite automatically starts
        // transactions in read-only mode, and always has serializable concurrency unless we
        // explicitly opt in to dirty reads with a pragma.
        #[cfg(not(feature = "embedded-db"))]
        conn.execute("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE, READ ONLY, DEFERRABLE")
            .await?;

        Ok(())
    }

    fn display() -> &'static str {
        "read-only"
    }
}

#[derive(Clone, Copy, Debug)]
enum CloseType {
    Commit,
    Revert,
    Drop,
}

#[derive(Debug)]
struct TransactionMetricsGuard<Mode> {
    started_at: Instant,
    metrics: PoolMetrics,
    close_type: CloseType,
    _mode: PhantomData<Mode>,
}

impl<Mode: TransactionMode> TransactionMetricsGuard<Mode> {
    fn begin(metrics: PoolMetrics) -> Self {
        let started_at = Instant::now();
        tracing::trace!(mode = Mode::display(), ?started_at, "begin");
        metrics.open_transactions.update(1);

        Self {
            started_at,
            metrics,
            close_type: CloseType::Drop,
            _mode: Default::default(),
        }
    }

    fn set_closed(&mut self, t: CloseType) {
        self.close_type = t;
    }
}

impl<Mode> Drop for TransactionMetricsGuard<Mode> {
    fn drop(&mut self) {
        self.metrics
            .transaction_durations
            .add_point((self.started_at.elapsed().as_millis() as f64) / 1000.);
        self.metrics.open_transactions.update(-1);
        match self.close_type {
            CloseType::Commit => self.metrics.commits.add(1),
            CloseType::Revert => self.metrics.reverts.add(1),
            CloseType::Drop => self.metrics.drops.add(1),
        }
        tracing::trace!(started_at = ?self.started_at, reason = ?self.close_type, "close");
    }
}

/// An atomic SQL transaction.
#[derive(Debug, Deref, DerefMut)]
pub struct Transaction<Mode> {
    #[deref]
    #[deref_mut]
    inner: sqlx::Transaction<'static, Db>,
    metrics: TransactionMetricsGuard<Mode>,
}

impl<Mode: TransactionMode> Transaction<Mode> {
    pub(super) async fn new(pool: &Pool<Db>, metrics: PoolMetrics) -> anyhow::Result<Self> {
        let mut inner = pool.begin().await?;
        let metrics = TransactionMetricsGuard::begin(metrics);
        Mode::begin(inner.as_mut()).await?;
        Ok(Self { inner, metrics })
    }
}

impl<Mode: TransactionMode> update::Transaction for Transaction<Mode> {
    async fn commit(mut self) -> anyhow::Result<()> {
        self.inner.commit().await?;
        self.metrics.set_closed(CloseType::Commit);
        Ok(())
    }
    fn revert(mut self) -> impl Future + Send {
        async move {
            self.inner.rollback().await.unwrap();
            self.metrics.set_closed(CloseType::Revert);
        }
    }
}

/// A collection of parameters which can be bound to a SQL query.
///
/// This trait allows us to carry around hetergenous lists of parameters (e.g. tuples) and bind them
/// to a query at the last moment before executing. This means we can manipulate the parameters
/// independently of the query before executing it. For example, by requiring a trait bound of
/// `Params<'p> + Clone`, we get a list (or tuple) of parameters which can be cloned and then bound
/// to a query, which allows us to keep a copy of the parameters around in order to retry the query
/// if it fails.
///
/// # Lifetimes
///
/// A SQL [`Query`] with lifetime `'q` borrows from both it's SQL statement (`&'q str`) and its
/// parameters (bound via `bind<'q>`). Sometimes, though, it is necessary for the statement and its
/// parameters to have different (but overlapping) lifetimes. For example, the parameters might be
/// passed in and owned by the caller, while the query string is constructed in the callee and its
/// lifetime is limited to the callee scope. (See for example the [`upsert`](Transaction::upsert)
/// function which does exactly this.)
///
/// We could rectify this situation with a trait bound like `P: for<'q> Params<'q>`, meaning `P`
/// must be bindable to a query with a lifetime chosen by the callee. However, when `P` is an
/// associated type, such as an element of an iterator, as in
/// `<I as IntoIter>::Item: for<'q> Params<'q>`, [a current limitation](https://blog.rust-lang.org/2022/10/28/gats-stabilization.html#implied-static-requirement-from-higher-ranked-trait-bounds.)
/// in the Rust compiler then requires `P: 'static`, which we don't necessarily want: the caller
/// should be able to pass in a reference to avoid expensive cloning.
///
/// So, instead, we work around this by making it explicit in the [`Params`] trait that the lifetime
/// of the query we're binding to (`'q`) may be different than the lifetime of the parameters (`'p`)
/// as long as the parameters outlive the duration of the query (the `'p: 'q`) bound on the
/// [`bind`](Self::bind) function.
pub trait Params<'p> {
    fn bind<'q, 'r>(
        self,
        q: &'q mut Separated<'r, 'p, Db, &'static str>,
    ) -> &'q mut Separated<'r, 'p, Db, &'static str>
    where
        'p: 'r;
}

/// A collection of parameters with a statically known length.
///
/// This is a simple trick for enforcing at compile time that a list of parameters has a certain
/// length, such as matching the length of a list of column names. This can prevent easy mistakes
/// like leaving out a parameter. It is implemented for tuples up to length 8.
pub trait FixedLengthParams<'p, const N: usize>: Params<'p> {}

macro_rules! impl_tuple_params {
    ($n:literal, ($($t:ident,)+)) => {
        impl<'p,  $($t),+> Params<'p> for ($($t,)+)
        where $(
            $t: 'p +  Encode<'p, Db> + Type<Db>
        ),+{
            fn bind<'q, 'r>(self, q: &'q mut Separated<'r, 'p, Db, &'static str>) ->   &'q mut Separated<'r, 'p, Db, &'static str>
            where 'p: 'r,
            {
                #[allow(non_snake_case)]
                let ($($t,)+) = self;
                q $(
                    .push_bind($t)
                )+

            }
        }

        impl<'p, $($t),+> FixedLengthParams<'p, $n> for ($($t,)+)
        where $(
            $t: 'p + for<'q> Encode<'q, Db> + Type<Db>
        ),+ {
        }
    };
}

impl_tuple_params!(1, (T,));
impl_tuple_params!(2, (T1, T2,));
impl_tuple_params!(3, (T1, T2, T3,));
impl_tuple_params!(4, (T1, T2, T3, T4,));
impl_tuple_params!(5, (T1, T2, T3, T4, T5,));
impl_tuple_params!(6, (T1, T2, T3, T4, T5, T6,));
impl_tuple_params!(7, (T1, T2, T3, T4, T5, T6, T7,));
impl_tuple_params!(8, (T1, T2, T3, T4, T5, T6, T7, T8,));

pub fn build_where_in<'a, I>(
    query: &'a str,
    column: &'a str,
    values: I,
) -> QueryResult<(queries::QueryBuilder<'a>, String)>
where
    I: IntoIterator,
    I::Item: 'a + Encode<'a, Db> + Type<Db>,
{
    let mut builder = queries::QueryBuilder::default();
    let params = values
        .into_iter()
        .map(|v| Ok(format!("{} ", builder.bind(v)?)))
        .collect::<QueryResult<Vec<String>>>()?;

    if params.is_empty() {
        return Err(QueryError::Error {
            message: "failed to build WHERE IN query. No parameter found ".to_string(),
        });
    }

    let sql = format!(
        "{query} where {column} IN ({}) ",
        params.into_iter().join(",")
    );

    Ok((builder, sql))
}

/// Low-level, general database queries and mutation.
impl Transaction<Write> {
    pub async fn upsert<'p, const N: usize, R>(
        &mut self,
        table: &str,
        columns: [&str; N],
        pk: impl IntoIterator<Item = &str>,
        rows: R,
    ) -> anyhow::Result<()>
    where
        R: IntoIterator,
        R::Item: 'p + FixedLengthParams<'p, N>,
    {
        let set_columns = columns
            .iter()
            .map(|col| format!("{col} = excluded.{col}"))
            .join(",");
        let columns_str = columns
            .into_iter()
            .map(|col| format!("\"{col}\""))
            .join(",");
        let pk = pk.into_iter().join(",");

        let mut query_builder =
            QueryBuilder::new(format!("INSERT INTO \"{table}\" ({columns_str}) "));
        let mut num_rows = 0;

        query_builder.push_values(rows, |mut b, row| {
            num_rows += 1;
            row.bind(&mut b);
        });

        if num_rows == 0 {
            tracing::warn!("trying to upsert 0 rows, this has no effect");
            return Ok(());
        }
        query_builder.push(format!(" ON CONFLICT ({pk}) DO UPDATE SET {set_columns}"));

        let res = self.execute(query_builder.build()).await?;
        let stmt = query_builder.sql();
        let rows_modified = res.rows_affected() as usize;

        if rows_modified != num_rows {
            tracing::error!(
                stmt,
                "unexpected number of rows modified: expected {num_rows} but got {rows_modified}"
            );
        }
        Ok(())
    }
}

/// Query service specific mutations.
impl Transaction<Write> {
    /// Delete a batch of data for pruning.
    pub(super) async fn delete_batch(
        &mut self,
        state_tables: Vec<String>,
        height: u64,
    ) -> anyhow::Result<()> {
        self.execute(query("DELETE FROM header WHERE height <= $1").bind(height as i64))
            .await?;

        // prune merklized state tables
        // only delete nodes having created < h AND
        // is not the newest node with its position
        for state_table in state_tables {
            self.execute(
                query(&format!(
                    "
                DELETE FROM {state_table} WHERE (path, created) IN
                (SELECT path, created FROM 
                (SELECT path, created, 
                ROW_NUMBER() OVER (PARTITION BY path ORDER BY created DESC) as rank 
                FROM {state_table} WHERE created <= $1) ranked_nodes WHERE rank != 1)"
                ))
                .bind(height as i64),
            )
            .await?;
        }

        self.save_pruned_height(height).await?;
        Ok(())
    }

    /// Record the height of the latest pruned header.
    pub(super) async fn save_pruned_height(&mut self, height: u64) -> anyhow::Result<()> {
        // id is set to 1 so that there is only one row in the table.
        // height is updated if the row already exists.
        self.upsert(
            "pruned_height",
            ["id", "last_height"],
            ["id"],
            [(1i32, height as i64)],
        )
        .await
    }
}

impl<Types> UpdateAvailabilityStorage<Types> for Transaction<Write>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
    Header<Types>: QueryableHeader<Types>,
{
    async fn insert_leaf(&mut self, leaf: LeafQueryData<Types>) -> anyhow::Result<()> {
        let height = leaf.height();

        // Ignore the leaf if it is below the pruned height. This can happen if, for instance, the
        // fetcher is racing with the pruner.
        if let Some(pruned_height) = self.load_pruned_height().await? {
            if height <= pruned_height {
                tracing::info!(
                    height,
                    pruned_height,
                    "ignoring leaf which is already pruned"
                );
                return Ok(());
            }
        }

        // While we don't necessarily have the full block for this leaf yet, we can initialize the
        // header table with block metadata taken from the leaf.
        let header_json = serde_json::to_value(leaf.leaf().block_header())
            .context("failed to serialize header")?;
        self.upsert(
            "header",
            ["height", "hash", "payload_hash", "data", "timestamp"],
            ["height"],
            [(
                height as i64,
                leaf.block_hash().to_string(),
                leaf.leaf().block_header().payload_commitment().to_string(),
                header_json,
                leaf.leaf().block_header().timestamp() as i64,
            )],
        )
        .await?;

        // Similarly, we can initialize the payload table with a null payload, which can help us
        // distinguish between blocks that haven't been produced yet and blocks we haven't received
        // yet when answering queries.
        // We don't overwrite the payload if it already exists.
        // During epoch transition in PoS, the same height block is sent multiple times.
        // The first block may have the payload, but subsequent blocks might be missing it.
        // Overwriting would cause the payload to be lost since the block height is the same
        let query = query("INSERT INTO payload (height) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(height as i64);
        query.execute(self.as_mut()).await?;

        // Finally, we insert the leaf itself, which references the header row we created.
        // Serialize the full leaf and QC to JSON for easy storage.
        let leaf_json = serde_json::to_value(leaf.leaf()).context("failed to serialize leaf")?;
        let qc_json = serde_json::to_value(leaf.qc()).context("failed to serialize QC")?;
        self.upsert(
            "leaf2",
            ["height", "hash", "block_hash", "leaf", "qc"],
            ["height"],
            [(
                height as i64,
                leaf.hash().to_string(),
                leaf.block_hash().to_string(),
                leaf_json,
                qc_json,
            )],
        )
        .await?;

        Ok(())
    }

    async fn insert_block(&mut self, block: BlockQueryData<Types>) -> anyhow::Result<()> {
        let height = block.height();

        // Ignore the block if it is below the pruned height. This can happen if, for instance, the
        // fetcher is racing with the pruner.
        if let Some(pruned_height) = self.load_pruned_height().await? {
            if height <= pruned_height {
                tracing::info!(
                    height,
                    pruned_height,
                    "ignoring block which is already pruned"
                );
                return Ok(());
            }
        }

        // The header and payload tables should already have been initialized when we inserted the
        // corresponding leaf. All we have to do is add the payload itself and its size.
        let payload = block.payload.encode();

        self.upsert(
            "payload",
            ["height", "data", "size", "num_transactions"],
            ["height"],
            [(
                height as i64,
                payload.as_ref().to_vec(),
                block.size() as i32,
                block.num_transactions() as i32,
            )],
        )
        .await?;

        // Index the transactions in the block.
        let mut rows = vec![];
        for (txn_ix, txn) in block.enumerate() {
            let txn_ix =
                serde_json::to_value(&txn_ix).context("failed to serialize transaction index")?;
            rows.push((txn.commit().to_string(), height as i64, txn_ix));
        }
        if !rows.is_empty() {
            self.upsert(
                "transactions",
                ["hash", "block_height", "idx"],
                ["block_height", "idx"],
                rows,
            )
            .await?;
        }

        Ok(())
    }

    async fn insert_vid(
        &mut self,
        common: VidCommonQueryData<Types>,
        share: Option<VidShare>,
    ) -> anyhow::Result<()> {
        let height = common.height();

        // Ignore the object if it is below the pruned height. This can happen if, for instance, the
        // fetcher is racing with the pruner.
        if let Some(pruned_height) = self.load_pruned_height().await? {
            if height <= pruned_height {
                tracing::info!(
                    height,
                    pruned_height,
                    "ignoring VID common which is already pruned"
                );
                return Ok(());
            }
        }

        let common_data =
            bincode::serialize(common.common()).context("failed to serialize VID common data")?;
        if let Some(share) = share {
            let share_data = bincode::serialize(&share).context("failed to serialize VID share")?;
            self.upsert(
                "vid2",
                ["height", "common", "share"],
                ["height"],
                [(height as i64, common_data, share_data)],
            )
            .await
        } else {
            // Don't touch the `share` column at all if we don't have a share to insert. It's
            // possible that this column already exists, and we are just upserting the common data,
            // in which case we don't want to overwrite the share with NULL.
            self.upsert(
                "vid2",
                ["height", "common"],
                ["height"],
                [(height as i64, common_data)],
            )
            .await
        }
    }

    async fn insert_state_cert(
        &mut self,
        state_cert: StateCertQueryData<Types>,
    ) -> anyhow::Result<()> {
        let height = state_cert.height();

        // Ignore the object if it is below the pruned height. This can happen if, for instance, the
        // fetcher is racing with the pruner.
        if let Some(pruned_height) = self.load_pruned_height().await? {
            if height <= pruned_height {
                tracing::info!(
                    height,
                    pruned_height,
                    "ignoring state cert which is already pruned"
                );
                return Ok(());
            }
        }
        let epoch = state_cert.0.epoch.u64();
        let bytes = bincode::serialize(&state_cert.0).context("failed to serialize state cert")?;
        // Directly upsert the state cert to the finalized_state_cert table because
        // this is called only when the corresponding leaf is decided.
        self.upsert(
            "finalized_state_cert",
            ["epoch", "state_cert"],
            ["epoch"],
            [(epoch as i64, bytes)],
        )
        .await?;
        Ok(())
    }
}

#[async_trait]
impl<Types: NodeType, State: MerklizedState<Types, ARITY>, const ARITY: usize>
    UpdateStateData<Types, State, ARITY> for Transaction<Write>
{
    async fn set_last_state_height(&mut self, height: usize) -> anyhow::Result<()> {
        self.upsert(
            "last_merklized_state_height",
            ["id", "height"],
            ["id"],
            [(1i32, height as i64)],
        )
        .await?;

        Ok(())
    }

    async fn insert_merkle_nodes(
        &mut self,
        proof: MerkleProof<State::Entry, State::Key, State::T, ARITY>,
        traversal_path: Vec<usize>,
        block_number: u64,
    ) -> anyhow::Result<()> {
        let pos = proof.pos;
        let path = proof.proof;

        let name = State::state_type();
        let block_number = block_number as i64;

        let mut traversal_path = traversal_path.iter().map(|n| *n as i32);

        // All the nodes are collected here, They depend on the hash ids which are returned after
        // hashes are upserted in the db
        let mut nodes = Vec::new();
        let mut hashset = HashSet::new();

        for node in path.iter() {
            match node {
                MerkleNode::Empty => {
                    let index = serde_json::to_value(pos.clone())
                        .decode_error("malformed merkle position")?;
                    // The node path represents the sequence of nodes from the root down to a specific node.
                    // Therefore, the traversal path needs to be reversed
                    // The root node path is an empty array.
                    let node_path = traversal_path.clone().rev().collect();
                    nodes.push((
                        Node {
                            path: node_path,
                            idx: Some(index),
                            ..Default::default()
                        },
                        None,
                        [0_u8; 32].to_vec(),
                    ));
                    hashset.insert([0_u8; 32].to_vec());
                },
                MerkleNode::ForgettenSubtree { .. } => {
                    bail!("Node in the Merkle path contains a forgetten subtree");
                },
                MerkleNode::Leaf { value, pos, elem } => {
                    let mut leaf_commit = Vec::new();
                    // Serialize the leaf node hash value into a vector
                    value
                        .serialize_compressed(&mut leaf_commit)
                        .decode_error("malformed merkle leaf commitment")?;

                    let path = traversal_path.clone().rev().collect();

                    let index = serde_json::to_value(pos.clone())
                        .decode_error("malformed merkle position")?;
                    let entry =
                        serde_json::to_value(elem).decode_error("malformed merkle element")?;

                    nodes.push((
                        Node {
                            path,
                            idx: Some(index),
                            entry: Some(entry),
                            ..Default::default()
                        },
                        None,
                        leaf_commit.clone(),
                    ));

                    hashset.insert(leaf_commit);
                },
                MerkleNode::Branch { value, children } => {
                    // Get hash
                    let mut branch_hash = Vec::new();
                    value
                        .serialize_compressed(&mut branch_hash)
                        .decode_error("malformed merkle branch hash")?;

                    // We only insert the non-empty children in the children field of the table
                    // BitVec is used to separate out Empty children positions
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
                                value
                                    .serialize_compressed(&mut hash)
                                    .decode_error("malformed merkle node hash")?;

                                children_values.push(hash);
                                // Mark the entry as 1 in bitvec to indicate a non-empty child
                                children_bitvec.push(true);
                            },
                        }
                    }

                    // insert internal node
                    let path = traversal_path.clone().rev().collect();
                    nodes.push((
                        Node {
                            path,
                            children: None,
                            children_bitvec: Some(children_bitvec),
                            ..Default::default()
                        },
                        Some(children_values.clone()),
                        branch_hash.clone(),
                    ));
                    hashset.insert(branch_hash);
                    hashset.extend(children_values);
                },
            }

            // advance the traversal path for the internal nodes at each iteration
            // The final node would be the Root node where this iterator is exhausted
            traversal_path.next();
        }
        // We build a hashset to avoid duplicate entries
        let hashes = hashset.into_iter().collect::<Vec<Vec<u8>>>();

        // insert all the hashes into database
        // It returns all the ids inserted in the order they were inserted
        // We use the hash ids to insert all the nodes
        let (query, sql) = build_hash_batch_insert(&hashes)?;
        let nodes_hash_ids: HashMap<Vec<u8>, i32> = query
            .query_as(&sql)
            .fetch(self.as_mut())
            .try_collect()
            .await?;

        // Updates the node fields
        for (node, children, hash) in &mut nodes {
            node.created = block_number;
            node.hash_id = *nodes_hash_ids.get(&*hash).ok_or(QueryError::Error {
                message: "Missing node hash".to_string(),
            })?;

            if let Some(children) = children {
                let children_hashes = children
                    .iter()
                    .map(|c| nodes_hash_ids.get(c).copied())
                    .collect::<Option<Vec<i32>>>()
                    .ok_or(QueryError::Error {
                        message: "Missing child hash".to_string(),
                    })?;

                node.children = Some(children_hashes.into());
            }
        }

        Node::upsert(name, nodes.into_iter().map(|(n, ..)| n), self).await?;

        Ok(())
    }
}

#[async_trait]
impl<Mode: TransactionMode> PrunedHeightStorage for Transaction<Mode> {
    async fn load_pruned_height(&mut self) -> anyhow::Result<Option<u64>> {
        let Some((height,)) =
            query_as::<(i64,)>("SELECT last_height FROM pruned_height ORDER BY id DESC LIMIT 1")
                .fetch_optional(self.as_mut())
                .await?
        else {
            return Ok(None);
        };
        Ok(Some(height as u64))
    }
}

#[derive(Clone, Debug)]
pub(super) struct PoolMetrics {
    open_transactions: Box<dyn Gauge>,
    transaction_durations: Box<dyn Histogram>,
    commits: Box<dyn Counter>,
    reverts: Box<dyn Counter>,
    drops: Box<dyn Counter>,
}

impl PoolMetrics {
    pub(super) fn new(metrics: &(impl Metrics + ?Sized)) -> Self {
        Self {
            open_transactions: metrics.create_gauge("open_transactions".into(), None),
            transaction_durations: metrics
                .create_histogram("transaction_duration".into(), Some("s".into())),
            commits: metrics.create_counter("committed_transactions".into(), None),
            reverts: metrics.create_counter("reverted_transactions".into(), None),
            drops: metrics.create_counter("dropped_transactions".into(), None),
        }
    }
}
