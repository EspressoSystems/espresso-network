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

#![cfg(any(test, feature = "testing"))]

use std::{ops::RangeBounds, sync::Arc};

use async_lock::Mutex;
use async_trait::async_trait;
use futures::future::Future;
use hotshot_types::{data::VidShare, traits::node_implementation::NodeType};
use vec1::Vec1;

use super::{
    pruning::{PruneStorage, PrunedHeightStorage, PrunerCfg, PrunerConfig},
    sql::MigrateTypes,
    Aggregate, AggregatesStorage, AvailabilityStorage, NodeStorage, UpdateAggregatesStorage,
    UpdateAvailabilityStorage,
};
use crate::{
    availability::{
        BlockId, BlockQueryData, LeafId, LeafQueryData, PayloadQueryData, QueryablePayload,
        StateCertQueryData, TransactionHash, TransactionQueryData, VidCommonQueryData,
    },
    data_source::{
        storage::{PayloadMetadata, VidCommonMetadata},
        update, VersionedDataSource,
    },
    metrics::PrometheusMetrics,
    node::{SyncStatus, TimeWindowQueryData, WindowStart},
    status::HasMetrics,
    Header, Payload, QueryError, QueryResult,
};

/// A specific action that can be targeted to inject an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailableAction {
    // TODO currently we implement failable actions for the availability methods, but if needed we
    // can always add more variants for other actions.
    GetHeader,
    GetLeaf,
    GetLeaves,
    GetBlock,
    GetPayload,
    GetPayloadMetadata,
    GetVidCommon,
    GetVidCommonMetadata,
    GetHeaderRange,
    GetLeafRange,
    GetBlockRange,
    GetPayloadRange,
    GetPayloadMetadataRange,
    GetVidCommonRange,
    GetVidCommonMetadataRange,
    GetTransaction,
    FirstAvailableLeaf,
    GetStateCert,

    /// Target any action for failure.
    Any,
}

impl FailableAction {
    /// Should `self` being targeted for failure cause `action` to fail?
    fn matches(self, action: Self) -> bool {
        // Fail if this is the action specifically targeted for failure or if we are failing any
        // action right now.
        self == action || self == Self::Any
    }
}

#[derive(Clone, Copy, Debug, Default)]
enum FailureMode {
    #[default]
    Never,
    Once(FailableAction),
    Always(FailableAction),
}

impl FailureMode {
    fn maybe_fail(&mut self, action: FailableAction) -> QueryResult<()> {
        match self {
            Self::Once(fail_action) if fail_action.matches(action) => {
                *self = Self::Never;
            },
            Self::Always(fail_action) if fail_action.matches(action) => {},
            _ => return Ok(()),
        }

        Err(QueryError::Error {
            message: "injected error".into(),
        })
    }
}

#[derive(Debug, Default)]
struct Failure {
    on_read: FailureMode,
    on_write: FailureMode,
    on_commit: FailureMode,
    on_begin_writable: FailureMode,
    on_begin_read_only: FailureMode,
}

/// Storage wrapper for error injection.
#[derive(Clone, Debug)]
pub struct FailStorage<S> {
    inner: S,
    failure: Arc<Mutex<Failure>>,
}

impl<S> From<S> for FailStorage<S> {
    fn from(inner: S) -> Self {
        Self {
            inner,
            failure: Default::default(),
        }
    }
}

impl<S> FailStorage<S> {
    pub async fn fail_reads(&self, action: FailableAction) {
        self.failure.lock().await.on_read = FailureMode::Always(action);
    }

    pub async fn fail_writes(&self, action: FailableAction) {
        self.failure.lock().await.on_write = FailureMode::Always(action);
    }

    pub async fn fail_commits(&self, action: FailableAction) {
        self.failure.lock().await.on_commit = FailureMode::Always(action);
    }

    pub async fn fail_begins_writable(&self, action: FailableAction) {
        self.failure.lock().await.on_begin_writable = FailureMode::Always(action);
    }

    pub async fn fail_begins_read_only(&self, action: FailableAction) {
        self.failure.lock().await.on_begin_read_only = FailureMode::Always(action);
    }

    pub async fn fail(&self, action: FailableAction) {
        let mut failure = self.failure.lock().await;
        failure.on_read = FailureMode::Always(action);
        failure.on_write = FailureMode::Always(action);
        failure.on_commit = FailureMode::Always(action);
        failure.on_begin_writable = FailureMode::Always(action);
        failure.on_begin_read_only = FailureMode::Always(action);
    }

    pub async fn pass_reads(&self) {
        self.failure.lock().await.on_read = FailureMode::Never;
    }

    pub async fn pass_writes(&self) {
        self.failure.lock().await.on_write = FailureMode::Never;
    }

    pub async fn pass_commits(&self) {
        self.failure.lock().await.on_commit = FailureMode::Never;
    }

    pub async fn pass_begins_writable(&self) {
        self.failure.lock().await.on_begin_writable = FailureMode::Never;
    }

    pub async fn pass_begins_read_only(&self) {
        self.failure.lock().await.on_begin_read_only = FailureMode::Never;
    }

    pub async fn pass(&self) {
        let mut failure = self.failure.lock().await;
        failure.on_read = FailureMode::Never;
        failure.on_write = FailureMode::Never;
        failure.on_commit = FailureMode::Never;
        failure.on_begin_writable = FailureMode::Never;
        failure.on_begin_read_only = FailureMode::Never;
    }

    pub async fn fail_one_read(&self, action: FailableAction) {
        self.failure.lock().await.on_read = FailureMode::Once(action);
    }

    pub async fn fail_one_write(&self, action: FailableAction) {
        self.failure.lock().await.on_write = FailureMode::Once(action);
    }

    pub async fn fail_one_commit(&self, action: FailableAction) {
        self.failure.lock().await.on_commit = FailureMode::Once(action);
    }

    pub async fn fail_one_begin_writable(&self, action: FailableAction) {
        self.failure.lock().await.on_begin_writable = FailureMode::Once(action);
    }

    pub async fn fail_one_begin_read_only(&self, action: FailableAction) {
        self.failure.lock().await.on_begin_read_only = FailureMode::Once(action);
    }
}

impl<S> VersionedDataSource for FailStorage<S>
where
    S: VersionedDataSource,
{
    type Transaction<'a>
        = Transaction<S::Transaction<'a>>
    where
        Self: 'a;
    type ReadOnly<'a>
        = Transaction<S::ReadOnly<'a>>
    where
        Self: 'a;

    async fn write(&self) -> anyhow::Result<<Self as VersionedDataSource>::Transaction<'_>> {
        self.failure
            .lock()
            .await
            .on_begin_writable
            .maybe_fail(FailableAction::Any)?;
        Ok(Transaction {
            inner: self.inner.write().await?,
            failure: self.failure.clone(),
        })
    }

    async fn read(&self) -> anyhow::Result<<Self as VersionedDataSource>::ReadOnly<'_>> {
        self.failure
            .lock()
            .await
            .on_begin_read_only
            .maybe_fail(FailableAction::Any)?;
        Ok(Transaction {
            inner: self.inner.read().await?,
            failure: self.failure.clone(),
        })
    }
}

impl<S> PrunerConfig for FailStorage<S>
where
    S: PrunerConfig,
{
    fn set_pruning_config(&mut self, cfg: PrunerCfg) {
        self.inner.set_pruning_config(cfg);
    }

    fn get_pruning_config(&self) -> Option<PrunerCfg> {
        self.inner.get_pruning_config()
    }
}

#[async_trait]
impl<S, Types: NodeType> MigrateTypes<Types> for FailStorage<S>
where
    S: MigrateTypes<Types> + Sync,
{
    async fn migrate_types(&self, _batch_size: u64) -> anyhow::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl<S> PruneStorage for FailStorage<S>
where
    S: PruneStorage + Sync,
{
    type Pruner = S::Pruner;

    async fn get_disk_usage(&self) -> anyhow::Result<u64> {
        self.inner.get_disk_usage().await
    }

    async fn prune(&self, pruner: &mut Self::Pruner) -> anyhow::Result<Option<u64>> {
        self.inner.prune(pruner).await
    }
}

impl<S> HasMetrics for FailStorage<S>
where
    S: HasMetrics,
{
    fn metrics(&self) -> &PrometheusMetrics {
        self.inner.metrics()
    }
}

#[derive(Debug)]
pub struct Transaction<T> {
    inner: T,
    failure: Arc<Mutex<Failure>>,
}

impl<T> Transaction<T> {
    async fn maybe_fail_read(&self, action: FailableAction) -> QueryResult<()> {
        self.failure.lock().await.on_read.maybe_fail(action)
    }

    async fn maybe_fail_write(&self, action: FailableAction) -> QueryResult<()> {
        self.failure.lock().await.on_write.maybe_fail(action)
    }

    async fn maybe_fail_commit(&self, action: FailableAction) -> QueryResult<()> {
        self.failure.lock().await.on_commit.maybe_fail(action)
    }
}

impl<T> update::Transaction for Transaction<T>
where
    T: update::Transaction,
{
    async fn commit(self) -> anyhow::Result<()> {
        self.maybe_fail_commit(FailableAction::Any).await?;
        self.inner.commit().await
    }

    fn revert(self) -> impl Future + Send {
        self.inner.revert()
    }
}

#[async_trait]
impl<Types, T> AvailabilityStorage<Types> for Transaction<T>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
    T: AvailabilityStorage<Types>,
{
    async fn get_leaf(&mut self, id: LeafId<Types>) -> QueryResult<LeafQueryData<Types>> {
        self.maybe_fail_read(FailableAction::GetLeaf).await?;
        self.inner.get_leaf(id).await
    }

    async fn get_leaves(&mut self, height: u64) -> QueryResult<Vec1<LeafQueryData<Types>>> {
        self.maybe_fail_read(FailableAction::GetLeaves).await?;
        self.inner.get_leaves(height).await
    }

    async fn get_block(&mut self, id: BlockId<Types>) -> QueryResult<BlockQueryData<Types>> {
        self.maybe_fail_read(FailableAction::GetBlock).await?;
        self.inner.get_block(id).await
    }

    async fn get_header(&mut self, id: BlockId<Types>) -> QueryResult<Header<Types>> {
        self.maybe_fail_read(FailableAction::GetHeader).await?;
        self.inner.get_header(id).await
    }

    async fn get_payload(&mut self, id: BlockId<Types>) -> QueryResult<PayloadQueryData<Types>> {
        self.maybe_fail_read(FailableAction::GetPayload).await?;
        self.inner.get_payload(id).await
    }

    async fn get_payload_metadata(
        &mut self,
        id: BlockId<Types>,
    ) -> QueryResult<PayloadMetadata<Types>> {
        self.maybe_fail_read(FailableAction::GetPayloadMetadata)
            .await?;
        self.inner.get_payload_metadata(id).await
    }

    async fn get_vid_common(
        &mut self,
        id: BlockId<Types>,
    ) -> QueryResult<VidCommonQueryData<Types>> {
        self.maybe_fail_read(FailableAction::GetVidCommon).await?;
        self.inner.get_vid_common(id).await
    }

    async fn get_vid_common_metadata(
        &mut self,
        id: BlockId<Types>,
    ) -> QueryResult<VidCommonMetadata<Types>> {
        self.maybe_fail_read(FailableAction::GetVidCommonMetadata)
            .await?;
        self.inner.get_vid_common_metadata(id).await
    }

    async fn get_leaf_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<LeafQueryData<Types>>>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.maybe_fail_read(FailableAction::GetLeafRange).await?;
        self.inner.get_leaf_range(range).await
    }

    async fn get_block_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<BlockQueryData<Types>>>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.maybe_fail_read(FailableAction::GetBlockRange).await?;
        self.inner.get_block_range(range).await
    }

    async fn get_payload_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<PayloadQueryData<Types>>>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.maybe_fail_read(FailableAction::GetPayloadRange)
            .await?;
        self.inner.get_payload_range(range).await
    }

    async fn get_payload_metadata_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<PayloadMetadata<Types>>>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.maybe_fail_read(FailableAction::GetPayloadMetadataRange)
            .await?;
        self.inner.get_payload_metadata_range(range).await
    }

    async fn get_vid_common_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<VidCommonQueryData<Types>>>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.maybe_fail_read(FailableAction::GetVidCommonRange)
            .await?;
        self.inner.get_vid_common_range(range).await
    }

    async fn get_vid_common_metadata_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<VidCommonMetadata<Types>>>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.maybe_fail_read(FailableAction::GetVidCommonMetadataRange)
            .await?;
        self.inner.get_vid_common_metadata_range(range).await
    }

    async fn get_transaction(
        &mut self,
        hash: TransactionHash<Types>,
    ) -> QueryResult<TransactionQueryData<Types>> {
        self.maybe_fail_read(FailableAction::GetTransaction).await?;
        self.inner.get_transaction(hash).await
    }

    async fn first_available_leaf(&mut self, from: u64) -> QueryResult<LeafQueryData<Types>> {
        self.maybe_fail_read(FailableAction::FirstAvailableLeaf)
            .await?;
        self.inner.first_available_leaf(from).await
    }

    async fn get_state_cert(&mut self, epoch: u64) -> QueryResult<StateCertQueryData<Types>> {
        self.maybe_fail_read(FailableAction::GetStateCert).await?;
        self.inner.get_state_cert(epoch).await
    }
}

impl<Types, T> UpdateAvailabilityStorage<Types> for Transaction<T>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
    T: UpdateAvailabilityStorage<Types> + Send + Sync,
{
    async fn insert_leaf(&mut self, leaf: LeafQueryData<Types>) -> anyhow::Result<()> {
        self.maybe_fail_write(FailableAction::Any).await?;
        self.inner.insert_leaf(leaf).await
    }

    async fn insert_block(&mut self, block: BlockQueryData<Types>) -> anyhow::Result<()> {
        self.maybe_fail_write(FailableAction::Any).await?;
        self.inner.insert_block(block).await
    }

    async fn insert_vid(
        &mut self,
        common: VidCommonQueryData<Types>,
        share: Option<VidShare>,
    ) -> anyhow::Result<()> {
        self.maybe_fail_write(FailableAction::Any).await?;
        self.inner.insert_vid(common, share).await
    }

    async fn insert_state_cert(
        &mut self,
        state_cert: StateCertQueryData<Types>,
    ) -> anyhow::Result<()> {
        self.maybe_fail_write(FailableAction::Any).await?;
        self.inner.insert_state_cert(state_cert).await
    }
}

#[async_trait]
impl<T> PrunedHeightStorage for Transaction<T>
where
    T: PrunedHeightStorage + Send + Sync,
{
    async fn load_pruned_height(&mut self) -> anyhow::Result<Option<u64>> {
        self.maybe_fail_read(FailableAction::Any).await?;
        self.inner.load_pruned_height().await
    }
}

#[async_trait]
impl<Types, T> NodeStorage<Types> for Transaction<T>
where
    Types: NodeType,
    T: NodeStorage<Types> + Send + Sync,
{
    async fn block_height(&mut self) -> QueryResult<usize> {
        self.maybe_fail_read(FailableAction::Any).await?;
        self.inner.block_height().await
    }

    async fn count_transactions_in_range(
        &mut self,
        range: impl RangeBounds<usize> + Send,
    ) -> QueryResult<usize> {
        self.maybe_fail_read(FailableAction::Any).await?;
        self.inner.count_transactions_in_range(range).await
    }

    async fn payload_size_in_range(
        &mut self,
        range: impl RangeBounds<usize> + Send,
    ) -> QueryResult<usize> {
        self.maybe_fail_read(FailableAction::Any).await?;
        self.inner.payload_size_in_range(range).await
    }

    async fn vid_share<ID>(&mut self, id: ID) -> QueryResult<VidShare>
    where
        ID: Into<BlockId<Types>> + Send + Sync,
    {
        self.maybe_fail_read(FailableAction::Any).await?;
        self.inner.vid_share(id).await
    }

    async fn sync_status(&mut self) -> QueryResult<SyncStatus> {
        self.maybe_fail_read(FailableAction::Any).await?;
        self.inner.sync_status().await
    }

    async fn get_header_window(
        &mut self,
        start: impl Into<WindowStart<Types>> + Send + Sync,
        end: u64,
        limit: usize,
    ) -> QueryResult<TimeWindowQueryData<Header<Types>>> {
        self.maybe_fail_read(FailableAction::Any).await?;
        self.inner.get_header_window(start, end, limit).await
    }
}

impl<T> AggregatesStorage for Transaction<T>
where
    T: AggregatesStorage + Send + Sync,
{
    async fn aggregates_height(&mut self) -> anyhow::Result<usize> {
        self.maybe_fail_read(FailableAction::Any).await?;
        self.inner.aggregates_height().await
    }

    async fn load_prev_aggregate(&mut self) -> anyhow::Result<Option<Aggregate>> {
        self.maybe_fail_read(FailableAction::Any).await?;
        self.inner.load_prev_aggregate().await
    }
}

impl<T, Types> UpdateAggregatesStorage<Types> for Transaction<T>
where
    Types: NodeType,
    T: UpdateAggregatesStorage<Types> + Send + Sync,
{
    async fn update_aggregates(
        &mut self,
        prev: Aggregate,
        blocks: &[PayloadMetadata<Types>],
    ) -> anyhow::Result<Aggregate> {
        self.maybe_fail_write(FailableAction::Any).await?;
        self.inner.update_aggregates(prev, blocks).await
    }
}
