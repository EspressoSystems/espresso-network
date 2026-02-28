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

use anyhow::{bail, Context};
use async_trait::async_trait;
use committable::Committable;
use futures::{future::Future, stream::TryStreamExt};
use hotshot_types::{
    data::VidShare,
    simple_certificate::CertificatePair,
    traits::{
        block_contents::BlockHeader,
        metrics::{Counter, Gauge, Histogram, Metrics},
        node_implementation::NodeType,
        EncodeBytes,
    },
};
use itertools::Itertools;
use jf_merkle_tree_compat::prelude::MerkleProof;
use sqlx::{query_builder::Separated, Database, Encode, Execute, Executor, QueryBuilder, Type};

use super::{
    db::{with_backend, BackendTransaction, DbBackend, SqlPool},
    queries::{
        self,
        state::{batch_insert_hashes, build_hash_batch_insert, collect_nodes_from_proofs, Node},
    },
};
use crate::{
    availability::{
        BlockQueryData, LeafQueryData, QueryableHeader, QueryablePayload, VidCommonQueryData,
    },
    data_source::{
        storage::{pruning::PrunedHeightStorage, NodeStorage, UpdateAvailabilityStorage},
        update,
    },
    merklized_state::{MerklizedState, UpdateStateData},
    types::HeightIndexed,
    Header, Payload, QueryError, QueryResult,
};

/// Marker type indicating a transaction with read-write access to the database.
#[derive(Clone, Copy, Debug, Default)]
pub struct Write;

/// Marker type indicating a transaction with read-only access to the database.
#[derive(Clone, Copy, Debug, Default)]
pub struct Read;

/// Trait for marker types indicating what type of access a transaction has to the database.
pub trait TransactionMode: Send + Sync {
    fn begin(tx: &mut BackendTransaction) -> impl Future<Output = anyhow::Result<()>> + Send;

    fn display() -> &'static str;
}

impl TransactionMode for Write {
    async fn begin(tx: &mut BackendTransaction) -> anyhow::Result<()> {
        match tx {
            BackendTransaction::Postgres(inner) => {
                inner
                    .as_mut()
                    .execute("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
                    .await?;
            },
            BackendTransaction::Sqlite(inner) => {
                // SQLite automatically sets the read/write mode of a transaction based on the
                // statements in it. However, we explicitly enable write mode from the start to
                // avoid deadlocks from lock upgrades. The proper way is `BEGIN IMMEDIATE`, but
                // sqlx doesn't expose that. A serviceable workaround is performing a write
                // statement first.
                inner
                    .as_mut()
                    .execute("UPDATE pruned_height SET id = id WHERE false")
                    .await?;
            },
        }
        Ok(())
    }

    fn display() -> &'static str {
        "write"
    }
}

impl TransactionMode for Read {
    async fn begin(tx: &mut BackendTransaction) -> anyhow::Result<()> {
        match tx {
            BackendTransaction::Postgres(inner) => {
                inner
                    .as_mut()
                    .execute("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE, READ ONLY, DEFERRABLE")
                    .await?;
            },
            BackendTransaction::Sqlite(_) => {},
        }
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
#[derive(Debug)]
pub struct Transaction<Mode> {
    pub inner: BackendTransaction,
    metrics: TransactionMetricsGuard<Mode>,
}

impl<Mode> Transaction<Mode> {
    pub fn backend(&self) -> DbBackend {
        self.inner.backend()
    }

    pub async fn execute(
        &mut self,
        q: super::queries::BackendQuery<'_>,
    ) -> Result<u64, sqlx::Error> {
        q.execute(self).await
    }

    pub async fn fetch_one(
        &mut self,
        q: super::queries::BackendQuery<'_>,
    ) -> Result<super::queries::BackendRow, sqlx::Error> {
        q.fetch_one(self).await
    }

    pub async fn fetch_optional(
        &mut self,
        q: super::queries::BackendQuery<'_>,
    ) -> Result<Option<super::queries::BackendRow>, sqlx::Error> {
        q.fetch_optional(self).await
    }

    pub async fn fetch_all(
        &mut self,
        q: super::queries::BackendQuery<'_>,
    ) -> Result<Vec<super::queries::BackendRow>, sqlx::Error> {
        q.fetch_all(self).await
    }

    pub fn query<'a>(&self, sql: &'a str) -> super::queries::BackendQuery<'a> {
        super::queries::query(self.backend(), sql)
    }

    pub fn query_as<'a, T>(&self, sql: &'a str) -> super::queries::BackendQueryAs<'a, T>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        super::queries::query_as(self.backend(), sql)
    }
}

impl<Mode: TransactionMode> Transaction<Mode> {
    pub(super) async fn new(pool: &SqlPool, metrics: PoolMetrics) -> anyhow::Result<Self> {
        let mut inner = pool.begin().await?;
        let metrics = TransactionMetricsGuard::begin(metrics);
        Mode::begin(&mut inner).await?;
        Ok(Self { inner, metrics })
    }
}

impl<Mode: TransactionMode> update::Transaction for Transaction<Mode> {
    async fn commit(self) -> anyhow::Result<()> {
        let Self { inner, mut metrics } = self;
        inner.commit().await?;
        metrics.set_closed(CloseType::Commit);
        Ok(())
    }
    fn revert(self) -> impl Future + Send {
        async move {
            let Self { inner, mut metrics } = self;
            inner.rollback().await.unwrap();
            metrics.set_closed(CloseType::Revert);
        }
    }
}

/// A collection of parameters which can be bound to a SQL query.
///
/// Generic over `DB: Database` so it works with both Postgres and SQLite inside `with_backend!`.
pub trait Params<'p, DB: Database> {
    fn bind<'q, 'r>(
        self,
        q: &'q mut Separated<'r, 'p, DB, &'static str>,
    ) -> &'q mut Separated<'r, 'p, DB, &'static str>
    where
        'p: 'r;
}

/// A collection of parameters with a statically known length.
pub trait FixedLengthParams<'p, DB: Database, const N: usize>: Params<'p, DB> {}

macro_rules! impl_tuple_params {
    ($n:literal, ($($t:ident,)+)) => {
        impl<'p, DB: Database, $($t),+> Params<'p, DB> for ($($t,)+)
        where $(
            $t: 'p + Encode<'p, DB> + Type<DB>
        ),+ {
            fn bind<'q, 'r>(self, q: &'q mut Separated<'r, 'p, DB, &'static str>) -> &'q mut Separated<'r, 'p, DB, &'static str>
            where 'p: 'r,
            {
                #[allow(non_snake_case)]
                let ($($t,)+) = self;
                q $(
                    .push_bind($t)
                )+

            }
        }

        impl<'p, DB: Database, $($t),+> FixedLengthParams<'p, DB, $n> for ($($t,)+)
        where $(
            $t: 'p + for<'q> Encode<'q, DB> + Type<DB>
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
    backend: DbBackend,
) -> QueryResult<(queries::QueryBuilder<'a>, String)>
where
    I: IntoIterator,
    I::Item: 'a
        + Encode<'a, sqlx::Postgres>
        + Type<sqlx::Postgres>
        + Encode<'a, sqlx::Sqlite>
        + Type<sqlx::Sqlite>,
{
    let mut builder = queries::QueryBuilder::new(backend);
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
        R::Item:
            'p + FixedLengthParams<'p, sqlx::Postgres, N> + FixedLengthParams<'p, sqlx::Sqlite, N>,
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

        with_backend!(self, |tx| {
            let mut query_builder =
                QueryBuilder::new(format!("INSERT INTO \"{table}\" ({columns_str}) "));
            query_builder.push_values(rows, |mut b, row| {
                row.bind(&mut b);
            });
            query_builder.push(format!(" ON CONFLICT ({pk}) DO UPDATE SET {set_columns}"));

            let query = query_builder.build();
            let statement = query.sql();

            let res = query.execute(tx.as_mut()).await.inspect_err(|err| {
                tracing::error!(statement, "error in statement execution: {err:#}");
            })?;
            let rows_modified = res.rows_affected() as usize;
            if rows_modified != num_rows {
                let error = format!(
                    "unexpected number of rows modified: expected {num_rows}, got \
                     {rows_modified}. query: {statement}"
                );
                tracing::error!(error);
                bail!(error);
            }
            Ok(())
        })
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
        with_backend!(self, |tx| {
            sqlx::query("DELETE FROM header WHERE height <= $1")
                .bind(height as i64)
                .execute(tx.as_mut())
                .await
                .map(|_| ())
        })?;

        for state_table in state_tables {
            with_backend!(self, |tx| {
                sqlx::query(&format!(
                    "
                DELETE FROM {state_table} WHERE (path, created) IN
                (SELECT path, created FROM
                (SELECT path, created,
                ROW_NUMBER() OVER (PARTITION BY path ORDER BY created DESC) as rank
                FROM {state_table} WHERE created <= $1) ranked_nodes WHERE rank != 1)"
                ))
                .bind(height as i64)
                .execute(tx.as_mut())
                .await
                .map(|_| ())
            })?;
        }

        self.save_pruned_height(height).await?;
        Ok(())
    }

    /// Record the height of the latest pruned header.
    pub(super) async fn save_pruned_height(&mut self, height: u64) -> anyhow::Result<()> {
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
    async fn insert_leaf_with_qc_chain(
        &mut self,
        leaf: LeafQueryData<Types>,
        qc_chain: Option<[CertificatePair<Types>; 2]>,
    ) -> anyhow::Result<()> {
        let height = leaf.height();

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

        with_backend!(self, |tx| {
            sqlx::query("INSERT INTO payload (height) VALUES ($1) ON CONFLICT DO NOTHING")
                .bind(height as i64)
                .execute(tx.as_mut())
                .await
                .map(|_| ())
        })?;

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

        let block_height = NodeStorage::<Types>::block_height(self).await? as u64;
        if height + 1 >= block_height {
            let qcs = serde_json::to_value(&qc_chain)?;
            self.upsert("latest_qc_chain", ["id", "qcs"], ["id"], [(1i32, qcs)])
                .await?;
        }

        Ok(())
    }

    async fn insert_block(&mut self, block: BlockQueryData<Types>) -> anyhow::Result<()> {
        let height = block.height();

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

        let mut rows = vec![];
        for (txn_ix, txn) in block.enumerate() {
            let ns_id = block.header().namespace_id(&txn_ix.ns_index).unwrap();
            rows.push((
                txn.commit().to_string(),
                height as i64,
                txn_ix.ns_index.into(),
                ns_id.into(),
                txn_ix.position as i64,
            ));
        }
        if !rows.is_empty() {
            self.upsert(
                "transactions",
                ["hash", "block_height", "ns_index", "ns_id", "position"],
                ["block_height", "ns_id", "position"],
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
            self.upsert(
                "vid2",
                ["height", "common"],
                ["height"],
                [(height as i64, common_data)],
            )
            .await
        }
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

        let nodes_hash_ids: HashMap<Vec<u8>, i32> = match self.inner.backend() {
            DbBackend::Postgres => batch_insert_hashes(hashes, self).await?,
            DbBackend::Sqlite => {
                let mut hash_ids: HashMap<Vec<u8>, i32> = HashMap::with_capacity(hashes.len());
                for hash_chunk in hashes.chunks(20) {
                    let (query, sql) = build_hash_batch_insert(hash_chunk, DbBackend::Sqlite)?;
                    let chunk_ids: HashMap<Vec<u8>, i32> =
                        query.query_as(&sql).fetch(self).try_collect().await?;
                    hash_ids.extend(chunk_ids);
                }
                hash_ids
            },
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
        let result: Option<(i64,)> = with_backend!(self, |tx| {
            sqlx::query_as("SELECT last_height FROM pruned_height ORDER BY id DESC LIMIT 1")
                .fetch_optional(tx.as_mut())
                .await
        })?;
        let Some((height,)) = result else {
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
