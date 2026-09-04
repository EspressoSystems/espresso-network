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

//! Fetching objects for many height ranges at once.
//!
//! The ranges the scanner is missing are often short and scattered, and fetching each one costs a
//! round trip. These requests carry the whole set at once. They resolve like any other fetch, so a
//! peer that cannot serve them is not a dead end: the provider falls back to fetching each range on
//! its own.

use std::{cmp::Ordering, collections::HashSet, ops::Range, sync::Arc};

use async_trait::async_trait;
use derivative::Derivative;
use futures::future::{BoxFuture, FutureExt, join_all};
use hotshot_types::traits::node_implementation::NodeType;

use super::{
    AvailabilityProvider, FetchRequest, Fetchable, Fetcher, Heights, Notifiers,
    header::HeaderCallback, leaf::RangeRequest,
};
use crate::{
    Header, Payload, QueryError, QueryResult,
    availability::{
        BlockId, BlockQueryData, LeafId, LeafQueryData, QueryableHeader, QueryablePayload,
        VidCommonQueryData,
    },
    data_source::{
        VersionedDataSource,
        storage::{
            AvailabilityStorage, NodeStorage, UpdateAvailabilityStorage,
            pruning::PrunedHeightStorage,
        },
    },
    fetching::{
        self, Callback, NonEmptyRange,
        request::{BlockBatchRequest, BlockBatchResponse, LeafBatchRequest, VidCommonBatchRequest},
    },
    types::HeightIndexed,
};

pub(super) type LeafBatchFetcher<Types, S, P> =
    fetching::Fetcher<LeafBatchRequest, LeafBatchCallback<Types, S, P>>;
pub(super) type BlockBatchFetcher<Types, S, P> =
    fetching::Fetcher<BlockBatchRequest, StoreBatch<Types, S, P>>;
pub(super) type VidCommonBatchFetcher<Types, S, P> =
    fetching::Fetcher<VidCommonBatchRequest, StoreBatch<Types, S, P>>;

/// The heights to fetch, as a set of half-open ranges.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct BatchRequest(pub(super) Vec<Range<u64>>);

impl BatchRequest {
    fn heights(&self) -> impl Iterator<Item = u64> + '_ {
        self.0.iter().flat_map(|range| range.clone())
    }
}

impl FetchRequest for BatchRequest {
    fn might_exist(self, heights: Heights) -> bool {
        self.0.iter().all(|range| {
            heights.pruned_height.is_none_or(|h| h < range.start) && range.end <= heights.height
        })
    }
}

/// The objects a [`BatchRequest`] asked for.
#[derive(Clone, Debug)]
pub(super) struct Batch<T>(pub(super) Vec<T>);

impl<T: HeightIndexed> Batch<T> {
    /// Does this cover every height the request asked for?
    ///
    /// Fetched batches are checked against this before they resolve a request, so a peer that
    /// answers with only part of what it was asked for does not end the fetch.
    fn satisfies(&self, req: &BatchRequest) -> bool {
        let heights = self
            .0
            .iter()
            .map(|obj| obj.height())
            .collect::<HashSet<_>>();
        req.heights().all(|height| heights.contains(&height))
    }
}

/// The batch the notifiers delivered, or [`None`] if it does not cover the whole request.
///
/// A notifier is dropped only at shutdown, and then yields no object. Returning [`None`] makes the
/// passive fetch panic like every other object's does, rather than quietly resolving a partial
/// batch as if it were whole.
fn complete<T: HeightIndexed>(objs: Vec<Option<T>>, req: &BatchRequest) -> Option<Batch<T>> {
    let batch = Batch(objs.into_iter().flatten().collect::<Vec<_>>());
    batch.satisfies(req).then_some(batch)
}

/// Stores a fetched batch of derived objects.
#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
pub(super) struct StoreBatch<Types: NodeType, S, P> {
    #[derivative(Debug = "ignore")]
    pub(super) fetcher: Arc<Fetcher<Types, S, P>>,
}

impl<Types: NodeType, S, P> PartialEq for StoreBatch<Types, S, P> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl<Types: NodeType, S, P> Eq for StoreBatch<Types, S, P> {}

impl<Types: NodeType, S, P> Ord for StoreBatch<Types, S, P> {
    fn cmp(&self, _other: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl<Types: NodeType, S, P> PartialOrd for StoreBatch<Types, S, P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Types, S, P> Callback<BlockBatchResponse<Types>> for StoreBatch<Types, S, P>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    async fn run(self, batch: BlockBatchResponse<Types>) {
        // VID goes in first: block notifications are what resolve the block batch, and the VID
        // scan that follows checks storage. Blocks first would let that scan run while these VID
        // writes are still in flight, and refetch what is already in hand.
        self.fetcher.store_runs(batch.vid_common).await;
        self.fetcher.store_runs(batch.blocks).await;
    }
}

impl<Types, S, P> Callback<Vec<VidCommonQueryData<Types>>> for StoreBatch<Types, S, P>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    async fn run(self, common: Vec<VidCommonQueryData<Types>>) {
        self.fetcher.store_runs(common).await;
    }
}

/// Stores a fetched batch of leaves, and continues on to whatever needed them.
#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
pub(super) enum LeafBatchCallback<Types: NodeType, S, P> {
    /// Store the leaves and backfill their cert2s.
    Store {
        #[derivative(Debug = "ignore")]
        fetcher: Arc<Fetcher<Types, S, P>>,
    },
    /// Fetch the blocks stored against these leaves.
    Blocks {
        #[derivative(Debug = "ignore")]
        fetcher: Arc<Fetcher<Types, S, P>>,
        req: BatchRequest,
    },
    /// Fetch the VID common stored against these leaves.
    VidCommon {
        #[derivative(Debug = "ignore")]
        fetcher: Arc<Fetcher<Types, S, P>>,
        req: BatchRequest,
    },
}

impl<Types: NodeType, S, P> PartialEq for LeafBatchCallback<Types, S, P> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl<Types: NodeType, S, P> Eq for LeafBatchCallback<Types, S, P> {}

impl<Types: NodeType, S, P> PartialOrd for LeafBatchCallback<Types, S, P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Types: NodeType, S, P> Ord for LeafBatchCallback<Types, S, P> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Store first, so the headers are in place before the derived fetches run. The request is
        // left out: every callback on one fetch carries the request that is that fetch's key.
        fn rank<Types: NodeType, S, P>(cb: &LeafBatchCallback<Types, S, P>) -> u8 {
            match cb {
                LeafBatchCallback::Store { .. } => 0,
                LeafBatchCallback::Blocks { .. } => 1,
                LeafBatchCallback::VidCommon { .. } => 2,
            }
        }
        rank(self).cmp(&rank(other))
    }
}

impl<Types, S, P> Callback<Vec<LeafQueryData<Types>>> for LeafBatchCallback<Types, S, P>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    async fn run(self, leaves: Vec<LeafQueryData<Types>>) {
        match self {
            Self::Store { fetcher } => {
                // Storing a leaf skips the callbacks a leaf fetch would have run, so chain the
                // cert2 backfill onto it the way `fetch_header_range_and_then` does.
                for run in fetcher.store_runs(leaves).await {
                    HeaderCallback::Cert2 {
                        fetcher: fetcher.clone(),
                    }
                    .run_range(&run);
                }
            },
            Self::Blocks { fetcher, req } => fetch_block_batch(fetcher, req),
            Self::VidCommon { fetcher, req } => fetch_vid_common_batch(fetcher, req),
        }
    }
}

/// Fetch the leaves for `req`, then run `then`.
fn fetch_leaf_batch_and_then<Types, S, P>(
    fetcher: Arc<Fetcher<Types, S, P>>,
    req: BatchRequest,
    then: Option<LeafBatchCallback<Types, S, P>>,
) where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    let store = LeafBatchCallback::Store {
        fetcher: fetcher.clone(),
    };
    fetcher.leaf_batch_fetcher.clone().spawn_fetch(
        LeafBatchRequest(req.0),
        fetcher.provider.clone(),
        std::iter::once(store).chain(then),
        false,
    );
}

fn fetch_block_batch<Types, S, P>(fetcher: Arc<Fetcher<Types, S, P>>, req: BatchRequest)
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    // Not fetched in leaf-only mode, where derived data is not stored.
    let Some(block_fetcher) = &fetcher.block_batch_fetcher else {
        return;
    };
    block_fetcher.clone().spawn_fetch(
        BlockBatchRequest(req.0),
        fetcher.provider.clone(),
        [StoreBatch {
            fetcher: fetcher.clone(),
        }],
        false,
    );
}

fn fetch_vid_common_batch<Types, S, P>(fetcher: Arc<Fetcher<Types, S, P>>, req: BatchRequest)
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    let Some(vid_fetcher) = &fetcher.vid_common_batch_fetcher else {
        return;
    };
    vid_fetcher.clone().spawn_fetch(
        VidCommonBatchRequest(req.0),
        fetcher.provider.clone(),
        [StoreBatch {
            fetcher: fetcher.clone(),
        }],
        false,
    );
}

/// Load the objects for `req` from storage, or [`QueryError::Missing`] if any height is absent.
///
/// Storage returns the rows it has, so an absent height is a short result rather than an error.
/// Without this check a batch of missing heights would look complete and never be fetched.
fn load_batch<T: HeightIndexed>(req: &BatchRequest, objs: Vec<T>) -> QueryResult<Batch<T>> {
    let objs = Batch(objs);
    if objs.satisfies(req) {
        Ok(objs)
    } else {
        Err(QueryError::Missing)
    }
}

#[async_trait]
impl<Types> Fetchable<Types> for Batch<LeafQueryData<Types>>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    type Request = BatchRequest;

    fn satisfies(&self, req: Self::Request) -> bool {
        Batch::satisfies(self, &req)
    }

    async fn passive_fetch(
        notifiers: &Notifiers<Types>,
        req: Self::Request,
    ) -> BoxFuture<'static, Option<Self>> {
        let waits = join_all(req.heights().map(|i| {
            notifiers
                .leaf
                .wait_for(move |leaf| leaf.satisfies(LeafId::Number(i as usize)))
        }))
        .await;

        join_all(waits.into_iter().map(|wait| wait.into_future()))
            .map(move |objs| complete(objs, &req))
            .boxed()
    }

    async fn active_fetch<S, P>(
        tx: &mut impl AvailabilityStorage<Types>,
        fetcher: Arc<Fetcher<Types, S, P>>,
        req: Self::Request,
    ) -> anyhow::Result<()>
    where
        S: VersionedDataSource + 'static,
        for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
        for<'a> S::ReadOnly<'a>:
            AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
        P: AvailabilityProvider<Types>,
    {
        // One range is just a range fetch, and that endpoint is a cacheable GET.
        if let [range] = req.0.as_slice() {
            let range = RangeRequest {
                start: range.start,
                end: range.end,
            };
            return <NonEmptyRange<LeafQueryData<Types>>>::active_fetch(tx, fetcher, range).await;
        }
        fetch_leaf_batch_and_then(fetcher, req, None);
        Ok(())
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        load_batch(&req, storage.get_leaf_batch(&req.0).await?)
    }
}

#[async_trait]
impl<Types> Fetchable<Types> for Batch<BlockQueryData<Types>>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    type Request = BatchRequest;

    fn satisfies(&self, req: Self::Request) -> bool {
        Batch::satisfies(self, &req)
    }

    async fn passive_fetch(
        notifiers: &Notifiers<Types>,
        req: Self::Request,
    ) -> BoxFuture<'static, Option<Self>> {
        let waits = join_all(req.heights().map(|i| {
            notifiers
                .block
                .wait_for(move |block| block.satisfies(BlockId::Number(i as usize)))
        }))
        .await;

        join_all(waits.into_iter().map(|wait| wait.into_future()))
            .map(move |objs| complete(objs, &req))
            .boxed()
    }

    /// Fetch the leaves first if they are missing, exactly as the range fetch does: blocks are
    /// stored against the headers the leaves carry.
    async fn active_fetch<S, P>(
        tx: &mut impl AvailabilityStorage<Types>,
        fetcher: Arc<Fetcher<Types, S, P>>,
        req: Self::Request,
    ) -> anyhow::Result<()>
    where
        S: VersionedDataSource + 'static,
        for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
        for<'a> S::ReadOnly<'a>:
            AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
        P: AvailabilityProvider<Types>,
    {
        // No single-range shortcut here, unlike leaves and VID: the plain range fetch drops the
        // VID common that rides along with the blocks, which would leave the VID scan
        // re-downloading every payload. A provider that can keep it shortcuts a single range
        // itself, on the cacheable GET.
        match <Batch<LeafQueryData<Types>>>::load(tx, req.clone()).await {
            Ok(_) => fetch_block_batch(fetcher, req),
            Err(QueryError::Missing | QueryError::NotFound) => fetch_leaf_batch_and_then(
                fetcher.clone(),
                req.clone(),
                Some(LeafBatchCallback::Blocks { fetcher, req }),
            ),
            Err(QueryError::Error { message }) => {
                anyhow::bail!("failed to load leaves for batch {req:?}: {message}")
            },
        }
        Ok(())
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        load_batch(&req, storage.get_block_batch(&req.0).await?)
    }
}

#[async_trait]
impl<Types> Fetchable<Types> for Batch<VidCommonQueryData<Types>>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    type Request = BatchRequest;

    fn satisfies(&self, req: Self::Request) -> bool {
        Batch::satisfies(self, &req)
    }

    async fn passive_fetch(
        notifiers: &Notifiers<Types>,
        req: Self::Request,
    ) -> BoxFuture<'static, Option<Self>> {
        let waits = join_all(req.heights().map(|i| {
            notifiers
                .vid_common
                .wait_for(move |common| common.height() == i)
        }))
        .await;

        join_all(waits.into_iter().map(|wait| wait.into_future()))
            .map(move |objs| complete(objs, &req))
            .boxed()
    }

    async fn active_fetch<S, P>(
        tx: &mut impl AvailabilityStorage<Types>,
        fetcher: Arc<Fetcher<Types, S, P>>,
        req: Self::Request,
    ) -> anyhow::Result<()>
    where
        S: VersionedDataSource + 'static,
        for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
        for<'a> S::ReadOnly<'a>:
            AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
        P: AvailabilityProvider<Types>,
    {
        // One range is just a range fetch, and that endpoint is a cacheable GET.
        if let [range] = req.0.as_slice() {
            let range = RangeRequest {
                start: range.start,
                end: range.end,
            };
            return <NonEmptyRange<VidCommonQueryData<Types>>>::active_fetch(tx, fetcher, range)
                .await;
        }

        match <Batch<LeafQueryData<Types>>>::load(tx, req.clone()).await {
            Ok(_) => fetch_vid_common_batch(fetcher, req),
            Err(QueryError::Missing | QueryError::NotFound) => fetch_leaf_batch_and_then(
                fetcher.clone(),
                req.clone(),
                Some(LeafBatchCallback::VidCommon { fetcher, req }),
            ),
            Err(QueryError::Error { message }) => {
                anyhow::bail!("failed to load leaves for batch {req:?}: {message}")
            },
        }
        Ok(())
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        load_batch(&req, storage.get_vid_common_batch(&req.0).await?)
    }
}
