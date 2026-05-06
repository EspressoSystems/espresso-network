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

use std::{collections::HashMap, marker::PhantomData, time::Instant};

use anyhow::{Context, bail};
use async_trait::async_trait;
use committable::Committable;
use derive_more::{Deref, DerefMut};
use futures::future::Future;
#[cfg(feature = "embedded-db")]
use futures::stream::TryStreamExt;
use hotshot_types::{
    data::VidShare,
    simple_certificate::CertificatePair,
    traits::{
        EncodeBytes,
        block_contents::BlockHeader,
        metrics::{Counter, Gauge, Histogram, Metrics},
        node_implementation::NodeType,
    },
};
use itertools::Itertools;
use jf_merkle_tree_compat::prelude::MerkleProof;
pub use sqlx::Executor;
use sqlx::{Encode, Execute, FromRow, QueryBuilder, Type, pool::Pool, query_builder::Separated};
use tracing::instrument;

#[cfg(not(feature = "embedded-db"))]
use super::queries::state::batch_insert_hashes;
#[cfg(feature = "embedded-db")]
use super::queries::state::build_hash_batch_insert;
use super::{
    Database, Db,
    queries::{
        self,
        state::{Node, collect_nodes_from_proofs},
    },
};
use crate::{
    Header, Payload, QueryError, QueryResult,
    availability::{
        BlockQueryData, Certificate2, LeafQueryData, QueryableHeader, QueryablePayload,
        VidCommonQueryData,
    },
    data_source::{
        storage::{NodeStorage, UpdateAvailabilityStorage, pruning::PrunedHeightStorage},
        update,
    },
    merklized_state::{MerklizedState, UpdateStateData},
    types::HeightIndexed,
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

/// Marker type indicating a transaction used for pruning deletes.
///
/// On Postgres this uses READ COMMITTED isolation instead of SERIALIZABLE to avoid predicate lock
/// conflicts between pruning DELETE and consensus INSERT operations.
#[derive(Clone, Copy, Debug, Default)]
pub struct Prune;

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

impl TransactionMode for Prune {
    #[allow(unused_variables)]
    async fn begin(conn: &mut <Db as Database>::Connection) -> anyhow::Result<()> {
        // SQLite: same as Write -- acquire an exclusive lock immediately to avoid deadlocks.
        #[cfg(feature = "embedded-db")]
        conn.execute("UPDATE pruned_height SET id = id WHERE false")
            .await?;

        // Postgres: use READ COMMITTED to avoid predicate lock conflicts between pruning
        // DELETE and concurrent consensus INSERT operations. Pruning does not need SERIALIZABLE
        // guarantees since it only removes old data that is no longer read by consensus.
        #[cfg(not(feature = "embedded-db"))]
        conn.execute("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
            .await?;

        Ok(())
    }

    fn display() -> &'static str {
        "prune"
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

        let columns_str = columns.iter().map(|col| format!("\"{col}\"")).join(",");

        let pk = pk.into_iter().join(",");

        let rows: Vec<_> = rows.into_iter().collect();
        let num_rows = rows.len();

        if num_rows == 0 {
            tracing::warn!("trying to upsert 0 rows into {table}, this has no effect");
            return Ok(());
        }

        let mut query_builder =
            QueryBuilder::new(format!("INSERT INTO \"{table}\" ({columns_str}) "));
        query_builder.push_values(rows, |mut b, row| {
            row.bind(&mut b);
        });
        query_builder.push(format!(" ON CONFLICT ({pk}) DO UPDATE SET {set_columns}"));

        let query = query_builder.build();
        let statement = query.sql();

        let res = self.execute(query).await.inspect_err(|err| {
            tracing::error!(statement, "error in statement execution: {err:#}");
        })?;
        let rows_modified = res.rows_affected() as usize;
        if rows_modified != num_rows {
            let error = format!(
                "unexpected number of rows modified: expected {num_rows}, got {rows_modified}. \
                 query: {statement}"
            );
            tracing::error!(error);
            bail!(error);
        }
        Ok(())
    }
}

/// Pruning mutations, run under READ COMMITTED isolation on Postgres.
impl Transaction<Prune> {
    /// Delete a batch of data for pruning.
    ///
    /// Payloads/vid_common are GC'd after header deletion using NOT EXISTS. Under READ
    /// COMMITTED, if a concurrent insert holds a lock on a payload row, the DELETE waits
    /// and re-evaluates with a fresh snapshot after the insert commits. If the payload was
    /// already deleted, the inserting SERIALIZABLE transaction gets a serialization error
    /// and retries, recreating the payload.
    #[instrument(skip(self))]
    pub(super) async fn delete_batch(&mut self, height: u64) -> anyhow::Result<()> {
        let res = query("DELETE FROM transactions WHERE block_height <= $1")
            .bind(height as i64)
            .execute(self.as_mut())
            .await
            .context("deleting transactions")?;
        tracing::debug!(rows_affected = res.rows_affected(), "pruned transactions");

        let res = query("DELETE FROM leaf2 WHERE height <= $1")
            .bind(height as i64)
            .execute(self.as_mut())
            .await
            .context("deleting leaf2")?;
        tracing::debug!(rows_affected = res.rows_affected(), "pruned leaf2");

        let res = query("DELETE FROM header WHERE height <= $1")
            .bind(height as i64)
            .execute(self.as_mut())
            .await
            .context("deleting headers")?;
        tracing::debug!(rows_affected = res.rows_affected(), "pruned headers");

        let res = query(
            "DELETE FROM payload AS p
             WHERE NOT EXISTS (
                SELECT 1 FROM header AS h
                WHERE h.payload_hash = p.hash AND h.ns_table = p.ns_table
             )",
        )
        .execute(self.as_mut())
        .await
        .context("garbage collecting payloads")?;
        tracing::debug!(
            rows_affected = res.rows_affected(),
            "garbage collected payloads"
        );

        let res = query(
            "DELETE FROM vid_common AS v
             WHERE NOT EXISTS (
                SELECT 1 FROM header AS h
                WHERE h.payload_hash = v.hash
             )",
        )
        .execute(self.as_mut())
        .await
        .context("garbage collecting VID common")?;
        tracing::debug!(
            rows_affected = res.rows_affected(),
            "garbage collected VID common"
        );

        Ok(())
    }

    /// Prune merklized state tables.
    ///
    /// Only deletes nodes having `created <= height` that are not the newest node at their position.
    #[instrument(skip(self))]
    pub(super) async fn delete_state_batch(
        &mut self,
        state_tables: Vec<String>,
        height: u64,
    ) -> anyhow::Result<()> {
        for state_table in state_tables {
            self.execute(
                query(&format!(
                    "
                DELETE FROM {state_table}
                WHERE {state_table}.created <= $1
                  AND EXISTS (
                    SELECT 1 FROM {state_table} AS t2
                    WHERE t2.path = {state_table}.path
                      AND t2.created > {state_table}.created
                      AND t2.created <= $1
                  )"
                ))
                .bind(height as i64),
            )
            .await?;
        }

        Ok(())
    }
}

/// Query service specific mutations.
impl Transaction<Write> {
    /// Record the height of the latest pruned header.
    pub(crate) async fn save_pruned_height(&mut self, height: u64) -> anyhow::Result<()> {
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
    async fn insert_qc_chain(
        &mut self,
        height: u64,
        qc_chain: Option<[CertificatePair<Types>; 2]>,
    ) -> anyhow::Result<()> {
        let block_height = NodeStorage::<Types>::block_height(self).await? as u64;
        if height + 1 >= block_height {
            // If this QC chain is for the latest leaf we know about, store it so that we can prove
            // to clients that the corresponding leaf is finalized. (If it is not the latest leaf,
            // this is unnecessary, since we can prove it is an ancestor of some later, finalized
            // leaf.)
            let qcs = serde_json::to_value(&qc_chain)?;
            self.upsert("latest_qc_chain", ["id", "qcs"], ["id"], [(1i32, qcs)])
                .await
                .context("inserting QC chain")?;
        }

        Ok(())
    }

    async fn insert_cert2(
        &mut self,
        height: u64,
        cert2: Certificate2<Types>,
    ) -> anyhow::Result<()> {
        let cert2_json = serde_json::to_value(&cert2)?;
        self.upsert(
            "cert2",
            ["height", "data"],
            ["height"],
            [(height as i64, cert2_json)],
        )
        .await
        .context("inserting cert2")?;
        Ok(())
    }

    async fn insert_leaf_range<'a>(
        &mut self,
        leaves: impl Send + IntoIterator<IntoIter: Send, Item = &'a LeafQueryData<Types>>,
    ) -> anyhow::Result<()> {
        let leaves = leaves.into_iter();

        // Ignore leaves below the pruned height.
        let pruned_height = self.load_pruned_height().await?;
        let leaves = leaves.skip_while(|leaf| pruned_height.is_some_and(|h| leaf.height() <= h));

        // While we don't necessarily have the full block for these leaves yet, we can initialize
        // the header and leaf tables with block metadata taken from the leaves.
        let (header_rows, leaf_rows): (Vec<_>, Vec<_>) = leaves
            .map(|leaf| {
                let header_json = serde_json::to_value(leaf.leaf().block_header())
                    .context("failed to serialize header")?;
                let header_row = (
                    leaf.height() as i64,
                    leaf.block_hash().to_string(),
                    leaf.leaf().block_header().payload_commitment().to_string(),
                    leaf.leaf().block_header().ns_table(),
                    header_json,
                    leaf.leaf().block_header().timestamp() as i64,
                );

                let leaf_json =
                    serde_json::to_value(leaf.leaf()).context("failed to serialize leaf")?;
                let qc_json = serde_json::to_value(leaf.qc()).context("failed to serialize QC")?;
                let leaf_row = (
                    leaf.height() as i64,
                    leaf.hash().to_string(),
                    leaf.block_hash().to_string(),
                    leaf_json,
                    qc_json,
                );

                anyhow::Ok((header_row, leaf_row))
            })
            .process_results(|iter| iter.unzip())?;

        self.upsert(
            "header",
            [
                "height",
                "hash",
                "payload_hash",
                "ns_table",
                "data",
                "timestamp",
            ],
            ["height"],
            header_rows,
        )
        .await
        .context("inserting headers")?;

        // Insert the leaves themselves, which reference the header rows we created.
        self.upsert(
            "leaf2",
            ["height", "hash", "block_hash", "leaf", "qc"],
            ["height"],
            leaf_rows,
        )
        .await
        .context("inserting leaves")?;

        Ok(())
    }

    async fn insert_block_range<'a>(
        &mut self,
        blocks: impl Send + IntoIterator<IntoIter: Send, Item = &'a BlockQueryData<Types>>,
    ) -> anyhow::Result<()> {
        let blocks = blocks.into_iter();

        // Ignore blocks below the pruned height.
        let pruned_height = self.load_pruned_height().await?;
        let blocks = blocks.skip_while(|block| pruned_height.is_some_and(|h| block.height() <= h));

        let (payload_rows, tx_rows): (Vec<_>, Vec<_>) = blocks
            .map(|block| {
                let payload_row = (
                    block.payload_hash().to_string(),
                    block.header().ns_table(),
                    block.size() as i32,
                    block.num_transactions() as i32,
                    block.payload.encode().as_ref().to_vec(),
                );

                let tx_rows = block.enumerate().map(|(txn_ix, txn)| {
                    let ns_id = block.header().namespace_id(&txn_ix.ns_index).unwrap();
                    (
                        txn.commit().to_string(),
                        block.height() as i64,
                        txn_ix.ns_index.into(),
                        ns_id.into(),
                        txn_ix.position as i64,
                    )
                });

                (payload_row, tx_rows)
            })
            .unzip();
        let tx_rows = tx_rows.into_iter().flatten().collect::<Vec<_>>();

        // Multiple blocks in the range might have the same payload. We must filter out such
        // duplicates, because SQL does not allow conflicting rows in a single upsert statement.
        let payload_rows = payload_rows
            .into_iter()
            .unique_by(|(hash, ns_table, ..)| (hash.clone(), ns_table.clone()));

        self.upsert(
            "payload",
            ["hash", "ns_table", "size", "num_transactions", "data"],
            ["hash", "ns_table"],
            payload_rows,
        )
        .await
        .context("inserting payloads")?;

        // Index the transactions and namespaces in the block.
        if !tx_rows.is_empty() {
            self.upsert(
                "transactions",
                ["hash", "block_height", "ns_index", "ns_id", "position"],
                ["block_height", "ns_id", "position"],
                tx_rows,
            )
            .await
            .context("inserting transactions")?;
        }

        Ok(())
    }

    async fn insert_vid_range<'a>(
        &mut self,
        vid: impl Send
        + IntoIterator<
            IntoIter: Send,
            Item = (&'a VidCommonQueryData<Types>, Option<&'a VidShare>),
        >,
    ) -> anyhow::Result<()> {
        let vid = vid.into_iter();

        // Ignore objects below the pruned height.
        let pruned_height = self.load_pruned_height().await?;
        let vid = vid.skip_while(|(common, _)| pruned_height.is_some_and(|h| common.height() <= h));

        let (common_rows, share_rows): (Vec<_>, Vec<_>) = vid
            .map(|(common, share)| {
                let common_data = bincode::serialize(common.common())
                    .context("failed to serialize VID common data")?;
                let common_row = (common.payload_hash().to_string(), common_data);

                let share_row = if let Some(share) = share {
                    let share_data =
                        bincode::serialize(&share).context("failed to serialize VID share")?;
                    Some((common.height() as i64, share_data))
                } else {
                    None
                };

                anyhow::Ok((common_row, share_row))
            })
            .process_results(|iter| iter.unzip())?;
        let share_rows = share_rows.into_iter().flatten().collect::<Vec<_>>();

        // Multiple blocks in the range might have the same VID common. We must filter out such
        // duplicates, because SQL does not allow conflicting rows in a single upsert statement.
        let common_rows = common_rows.into_iter().unique_by(|(hash, ..)| hash.clone());

        self.upsert("vid_common", ["hash", "data"], ["hash"], common_rows)
            .await
            .context("inserting VID common")?;

        if !share_rows.is_empty() {
            let mut q = QueryBuilder::new("WITH rows (height, share) AS (");
            q.push_values(share_rows, |mut q, (height, share)| {
                q.push_bind(height).push_bind(share);
            });
            q.push(
                ") UPDATE header SET vid_share = rows.share
                FROM rows
                WHERE header.height = rows.height",
            );
            q.build()
                .execute(self.as_mut())
                .await
                .context("inserting VID shares")?;
        }

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
        let proofs = vec![(proof, traversal_path)];
        UpdateStateData::<Types, State, ARITY>::insert_merkle_nodes_batch(
            self,
            proofs,
            block_number,
        )
        .await
    }

    async fn insert_merkle_nodes_batch(
        &mut self,
        proofs: Vec<(
            MerkleProof<State::Entry, State::Key, State::T, ARITY>,
            Vec<usize>,
        )>,
        block_number: u64,
    ) -> anyhow::Result<()> {
        if proofs.is_empty() {
            return Ok(());
        }

        let name = State::state_type();
        let block_number = block_number as i64;

        let (mut all_nodes, all_hashes) = collect_nodes_from_proofs(&proofs)?;
        let hashes: Vec<Vec<u8>> = all_hashes.into_iter().collect();

        #[cfg(not(feature = "embedded-db"))]
        let nodes_hash_ids: HashMap<Vec<u8>, i32> = batch_insert_hashes(hashes, self).await?;

        #[cfg(feature = "embedded-db")]
        let nodes_hash_ids: HashMap<Vec<u8>, i32> = {
            let mut hash_ids: HashMap<Vec<u8>, i32> = HashMap::with_capacity(hashes.len());
            for hash_chunk in hashes.chunks(20) {
                let (query, sql) = build_hash_batch_insert(hash_chunk)?;
                let chunk_ids: HashMap<Vec<u8>, i32> = query
                    .query_as(&sql)
                    .fetch(self.as_mut())
                    .try_collect()
                    .await?;
                hash_ids.extend(chunk_ids);
            }
            hash_ids
        };

        for (node, children, hash) in &mut all_nodes {
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

        Node::upsert(name, all_nodes.into_iter().map(|(n, ..)| n), self).await?;

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
