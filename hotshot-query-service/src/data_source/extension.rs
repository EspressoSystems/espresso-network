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

use std::ops::{Bound, RangeBounds};

use async_trait::async_trait;
use hotshot_types::{
    data::VidShare,
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
};
use jf_merkle_tree::prelude::MerkleProof;
use tagged_base64::TaggedBase64;

use super::VersionedDataSource;
use crate::{
    availability::{
        AvailabilityDataSource, BlockId, BlockInfo, BlockQueryData, Fetch, FetchStream, LeafId,
        LeafQueryData, PayloadMetadata, PayloadQueryData, QueryablePayload, StateCertQueryData,
        TransactionHash, TransactionQueryData, UpdateAvailabilityData, VidCommonMetadata,
        VidCommonQueryData,
    },
    data_source::storage::pruning::PrunedHeightDataSource,
    explorer::{self, ExplorerDataSource, ExplorerHeader, ExplorerTransaction},
    merklized_state::{
        MerklizedState, MerklizedStateDataSource, MerklizedStateHeightPersistence, Snapshot,
        UpdateStateData,
    },
    metrics::PrometheusMetrics,
    node::{NodeDataSource, SyncStatus, TimeWindowQueryData, WindowStart},
    status::{HasMetrics, StatusDataSource},
    Header, Payload, QueryResult, Transaction,
};
/// Wrapper to add extensibility to an existing data source.
///
/// [`ExtensibleDataSource`] adds app-specific data to any existing data source. It implements all
/// the data source traits defined in this crate as long as the underlying data source does so,
/// which means it can be used as state for instantiating the APIs defined in this crate. At the
/// same time, it provides access to an application-defined state type, which means it can also be
/// used to implement application-specific endpoints.
///
/// [`ExtensibleDataSource`] implements `AsRef<U>` and `AsMut<U>` for some user-defined type `U`, so
/// your API extensions can always access application-specific state from [`ExtensibleDataSource`].
/// We can use this to complete the [UTXO example](crate#extension) by extending our data source
/// with an index to look up transactions by the UTXOs they contain:
///
/// ```
/// # use async_trait::async_trait;
/// # use hotshot_query_service::availability::{AvailabilityDataSource, TransactionIndex};
/// # use hotshot_query_service::data_source::ExtensibleDataSource;
/// # use hotshot_query_service::testing::mocks::MockTypes as AppTypes;
/// # use std::collections::HashMap;
/// # #[async_trait]
/// # trait UtxoDataSource: AvailabilityDataSource<AppTypes> {
/// #   async fn find_utxo(&self, utxo: u64) -> Option<(usize, TransactionIndex<AppTypes>, usize)>;
/// # }
/// type UtxoIndex = HashMap<u64, (usize, TransactionIndex<AppTypes>, usize)>;
///
/// #[async_trait]
/// impl<UnderlyingDataSource> UtxoDataSource for
///     ExtensibleDataSource<UnderlyingDataSource, UtxoIndex>
/// where
///     UnderlyingDataSource: AvailabilityDataSource<AppTypes> + Send + Sync,
/// {
///     async fn find_utxo(&self, utxo: u64) -> Option<(usize, TransactionIndex<AppTypes>, usize)> {
///         self.as_ref().get(&utxo).cloned()
///     }
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ExtensibleDataSource<D, U> {
    data_source: D,
    user_data: U,
}

impl<D, U> ExtensibleDataSource<D, U> {
    pub fn new(data_source: D, user_data: U) -> Self {
        Self {
            data_source,
            user_data,
        }
    }

    /// Access the underlying data source.
    ///
    /// This functionality is provided as an inherent method rather than an implementation of the
    /// [`AsRef`] trait so that `self.as_ref()` unambiguously returns `&U`, helping with type
    /// inference.
    pub fn inner(&self) -> &D {
        &self.data_source
    }

    /// Mutably access the underlying data source.
    ///
    /// This functionality is provided as an inherent method rather than an implementation of the
    /// [`AsMut`] trait so that `self.as_mut()` unambiguously returns `&U`, helping with type
    /// inference.
    pub fn inner_mut(&mut self) -> &mut D {
        &mut self.data_source
    }
}

impl<D, U> AsRef<U> for ExtensibleDataSource<D, U> {
    fn as_ref(&self) -> &U {
        &self.user_data
    }
}

impl<D, U> AsMut<U> for ExtensibleDataSource<D, U> {
    fn as_mut(&mut self) -> &mut U {
        &mut self.user_data
    }
}

impl<D, U> VersionedDataSource for ExtensibleDataSource<D, U>
where
    D: VersionedDataSource + Send,
    U: Send + Sync,
{
    type Transaction<'a>
        = D::Transaction<'a>
    where
        Self: 'a;

    type ReadOnly<'a>
        = D::ReadOnly<'a>
    where
        Self: 'a;

    async fn write(&self) -> anyhow::Result<Self::Transaction<'_>> {
        self.data_source.write().await
    }

    async fn read(&self) -> anyhow::Result<Self::ReadOnly<'_>> {
        self.data_source.read().await
    }
}

#[async_trait]
impl<D, U> PrunedHeightDataSource for ExtensibleDataSource<D, U>
where
    D: PrunedHeightDataSource + Send + Sync,
    U: Send + Sync,
{
    async fn load_pruned_height(&self) -> anyhow::Result<Option<u64>> {
        self.data_source.load_pruned_height().await
    }
}

#[async_trait]
impl<D, U, Types> AvailabilityDataSource<Types> for ExtensibleDataSource<D, U>
where
    D: AvailabilityDataSource<Types> + Send + Sync,
    U: Send + Sync,
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    async fn get_leaf<ID>(&self, id: ID) -> Fetch<LeafQueryData<Types>>
    where
        ID: Into<LeafId<Types>> + Send + Sync,
    {
        self.data_source.get_leaf(id).await
    }

    async fn get_header<ID>(&self, id: ID) -> Fetch<Header<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync,
    {
        self.data_source.get_header(id).await
    }

    async fn get_block<ID>(&self, id: ID) -> Fetch<BlockQueryData<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync,
    {
        self.data_source.get_block(id).await
    }
    async fn get_payload<ID>(&self, id: ID) -> Fetch<PayloadQueryData<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync,
    {
        self.data_source.get_payload(id).await
    }
    async fn get_payload_metadata<ID>(&self, id: ID) -> Fetch<PayloadMetadata<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync,
    {
        self.data_source.get_payload_metadata(id).await
    }
    async fn get_vid_common<ID>(&self, id: ID) -> Fetch<VidCommonQueryData<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync,
    {
        self.data_source.get_vid_common(id).await
    }
    async fn get_vid_common_metadata<ID>(&self, id: ID) -> Fetch<VidCommonMetadata<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync,
    {
        self.data_source.get_vid_common_metadata(id).await
    }
    async fn get_leaf_range<R>(&self, range: R) -> FetchStream<LeafQueryData<Types>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.data_source.get_leaf_range(range).await
    }
    async fn get_block_range<R>(&self, range: R) -> FetchStream<BlockQueryData<Types>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.data_source.get_block_range(range).await
    }

    async fn get_header_range<R>(&self, range: R) -> FetchStream<Header<Types>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.data_source.get_header_range(range).await
    }
    async fn get_payload_range<R>(&self, range: R) -> FetchStream<PayloadQueryData<Types>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.data_source.get_payload_range(range).await
    }
    async fn get_payload_metadata_range<R>(&self, range: R) -> FetchStream<PayloadMetadata<Types>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.data_source.get_payload_metadata_range(range).await
    }
    async fn get_vid_common_range<R>(&self, range: R) -> FetchStream<VidCommonQueryData<Types>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.data_source.get_vid_common_range(range).await
    }
    async fn get_vid_common_metadata_range<R>(
        &self,
        range: R,
    ) -> FetchStream<VidCommonMetadata<Types>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.data_source.get_vid_common_metadata_range(range).await
    }

    async fn get_leaf_range_rev(
        &self,
        start: Bound<usize>,
        end: usize,
    ) -> FetchStream<LeafQueryData<Types>> {
        self.data_source.get_leaf_range_rev(start, end).await
    }
    async fn get_block_range_rev(
        &self,
        start: Bound<usize>,
        end: usize,
    ) -> FetchStream<BlockQueryData<Types>> {
        self.data_source.get_block_range_rev(start, end).await
    }
    async fn get_payload_range_rev(
        &self,
        start: Bound<usize>,
        end: usize,
    ) -> FetchStream<PayloadQueryData<Types>> {
        self.data_source.get_payload_range_rev(start, end).await
    }
    async fn get_payload_metadata_range_rev(
        &self,
        start: Bound<usize>,
        end: usize,
    ) -> FetchStream<PayloadMetadata<Types>> {
        self.data_source
            .get_payload_metadata_range_rev(start, end)
            .await
    }
    async fn get_vid_common_range_rev(
        &self,
        start: Bound<usize>,
        end: usize,
    ) -> FetchStream<VidCommonQueryData<Types>> {
        self.data_source.get_vid_common_range_rev(start, end).await
    }
    async fn get_vid_common_metadata_range_rev(
        &self,
        start: Bound<usize>,
        end: usize,
    ) -> FetchStream<VidCommonMetadata<Types>> {
        self.data_source
            .get_vid_common_metadata_range_rev(start, end)
            .await
    }
    async fn get_transaction(
        &self,
        hash: TransactionHash<Types>,
    ) -> Fetch<TransactionQueryData<Types>> {
        self.data_source.get_transaction(hash).await
    }
    async fn get_state_cert(&self, epoch: u64) -> Fetch<StateCertQueryData<Types>> {
        self.data_source.get_state_cert(epoch).await
    }
}

impl<D, U, Types> UpdateAvailabilityData<Types> for ExtensibleDataSource<D, U>
where
    D: UpdateAvailabilityData<Types> + Send + Sync,
    U: Send + Sync,
    Types: NodeType,
{
    async fn append(&self, info: BlockInfo<Types>) -> anyhow::Result<()> {
        self.data_source.append(info).await
    }
}

#[async_trait]
impl<D, U, Types> NodeDataSource<Types> for ExtensibleDataSource<D, U>
where
    D: NodeDataSource<Types> + Send + Sync,
    U: Send + Sync,
    Types: NodeType,
{
    async fn block_height(&self) -> QueryResult<usize> {
        self.data_source.block_height().await
    }
    async fn count_transactions_in_range(
        &self,
        range: impl RangeBounds<usize> + Send,
    ) -> QueryResult<usize> {
        self.data_source.count_transactions_in_range(range).await
    }
    async fn payload_size_in_range(
        &self,
        range: impl RangeBounds<usize> + Send,
    ) -> QueryResult<usize> {
        self.data_source.payload_size_in_range(range).await
    }
    async fn vid_share<ID>(&self, id: ID) -> QueryResult<VidShare>
    where
        ID: Into<BlockId<Types>> + Send + Sync,
    {
        self.data_source.vid_share(id).await
    }
    async fn sync_status(&self) -> QueryResult<SyncStatus> {
        self.data_source.sync_status().await
    }
    async fn get_header_window(
        &self,
        start: impl Into<WindowStart<Types>> + Send + Sync,
        end: u64,
        limit: usize,
    ) -> QueryResult<TimeWindowQueryData<Header<Types>>> {
        self.data_source.get_header_window(start, end, limit).await
    }
}

impl<D, U> HasMetrics for ExtensibleDataSource<D, U>
where
    D: HasMetrics,
{
    fn metrics(&self) -> &PrometheusMetrics {
        self.data_source.metrics()
    }
}

#[async_trait]
impl<D, U> StatusDataSource for ExtensibleDataSource<D, U>
where
    D: StatusDataSource + Send + Sync,
    U: Send + Sync,
{
    async fn block_height(&self) -> QueryResult<usize> {
        self.data_source.block_height().await
    }
}

#[async_trait]
impl<D, U, Types, State, const ARITY: usize> MerklizedStateDataSource<Types, State, ARITY>
    for ExtensibleDataSource<D, U>
where
    D: MerklizedStateDataSource<Types, State, ARITY> + Sync,
    U: Send + Sync,
    Types: NodeType,
    State: MerklizedState<Types, ARITY>,
{
    async fn get_path(
        &self,
        snapshot: Snapshot<Types, State, ARITY>,
        key: State::Key,
    ) -> QueryResult<MerkleProof<State::Entry, State::Key, State::T, ARITY>> {
        self.data_source.get_path(snapshot, key).await
    }
}

#[async_trait]
impl<D, U> MerklizedStateHeightPersistence for ExtensibleDataSource<D, U>
where
    D: MerklizedStateHeightPersistence + Sync,
    U: Send + Sync,
{
    async fn get_last_state_height(&self) -> QueryResult<usize> {
        self.data_source.get_last_state_height().await
    }
}

#[async_trait]
impl<D, U, Types, State, const ARITY: usize> UpdateStateData<Types, State, ARITY>
    for ExtensibleDataSource<D, U>
where
    D: UpdateStateData<Types, State, ARITY> + Send + Sync,
    U: Send + Sync,
    State: MerklizedState<Types, ARITY>,
    Types: NodeType,
{
    async fn set_last_state_height(&mut self, height: usize) -> anyhow::Result<()> {
        self.data_source.set_last_state_height(height).await
    }

    async fn insert_merkle_nodes(
        &mut self,
        path: MerkleProof<State::Entry, State::Key, State::T, ARITY>,
        traversal_path: Vec<usize>,
        block_number: u64,
    ) -> anyhow::Result<()> {
        self.data_source
            .insert_merkle_nodes(path, traversal_path, block_number)
            .await
    }
}

#[async_trait]
impl<D, U, Types> ExplorerDataSource<Types> for ExtensibleDataSource<D, U>
where
    D: ExplorerDataSource<Types> + Sync,
    U: Send + Sync,
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
    Header<Types>: ExplorerHeader<Types> + BlockHeader<Types>,
    Transaction<Types>: ExplorerTransaction,
{
    async fn get_block_detail(
        &self,
        request: explorer::query_data::BlockIdentifier<Types>,
    ) -> Result<explorer::query_data::BlockDetail<Types>, explorer::query_data::GetBlockDetailError>
    {
        self.data_source.get_block_detail(request).await
    }

    async fn get_block_summaries(
        &self,
        request: explorer::query_data::GetBlockSummariesRequest<Types>,
    ) -> Result<
        Vec<explorer::query_data::BlockSummary<Types>>,
        explorer::query_data::GetBlockSummariesError,
    > {
        self.data_source.get_block_summaries(request).await
    }

    async fn get_transaction_detail(
        &self,
        request: explorer::query_data::TransactionIdentifier<Types>,
    ) -> Result<
        explorer::query_data::TransactionDetailResponse<Types>,
        explorer::query_data::GetTransactionDetailError,
    > {
        self.data_source.get_transaction_detail(request).await
    }

    async fn get_transaction_summaries(
        &self,
        request: explorer::query_data::GetTransactionSummariesRequest<Types>,
    ) -> Result<
        Vec<explorer::query_data::TransactionSummary<Types>>,
        explorer::query_data::GetTransactionSummariesError,
    > {
        self.data_source.get_transaction_summaries(request).await
    }

    async fn get_explorer_summary(
        &self,
    ) -> Result<
        explorer::query_data::ExplorerSummary<Types>,
        explorer::query_data::GetExplorerSummaryError,
    > {
        self.data_source.get_explorer_summary().await
    }

    async fn get_search_results(
        &self,
        query: TaggedBase64,
    ) -> Result<
        explorer::query_data::SearchResult<Types>,
        explorer::query_data::GetSearchResultsError,
    > {
        self.data_source.get_search_results(query).await
    }
}

#[cfg(any(test, feature = "testing"))]
mod impl_testable_data_source {
    use hotshot::types::Event;

    use super::*;
    use crate::{
        data_source::UpdateDataSource,
        testing::{
            consensus::{DataSourceLifeCycle, TestableDataSource},
            mocks::MockTypes,
        },
    };

    #[async_trait]
    impl<D, U> DataSourceLifeCycle for ExtensibleDataSource<D, U>
    where
        D: TestableDataSource + UpdateDataSource<MockTypes>,
        U: Clone + Default + Send + Sync + 'static,
    {
        type Storage = D::Storage;

        async fn create(node_id: usize) -> Self::Storage {
            D::create(node_id).await
        }

        async fn connect(storage: &Self::Storage) -> Self {
            Self::new(D::connect(storage).await, Default::default())
        }

        async fn reset(storage: &Self::Storage) -> Self {
            Self::new(D::reset(storage).await, Default::default())
        }

        async fn handle_event(&self, event: &Event<MockTypes>) {
            self.update(event).await.unwrap();
        }
    }
}

#[cfg(test)]
mod test {
    use super::ExtensibleDataSource;
    use crate::testing::consensus::MockDataSource;
    // For some reason this is the only way to import the macro defined in another module of this
    // crate.
    use crate::*;

    instantiate_data_source_tests!(ExtensibleDataSource<MockDataSource, ()>);
}
