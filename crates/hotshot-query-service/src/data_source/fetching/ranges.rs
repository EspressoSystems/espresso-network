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
    cert2::fetch_cert2_with_header, header::HeaderCallback, leaf::RangeRequest,
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
        request::{
            BlockRangesRequest, BlockRangesResponse, LeafRangesRequest, VidCommonRangesRequest,
        },
    },
    types::HeightIndexed,
};

pub(super) type LeafRangesFetcher<Types, S, P> =
    fetching::Fetcher<LeafRangesRequest, LeafRangesCallback<Types, S, P>>;
pub(super) type BlockRangesFetcher<Types, S, P> =
    fetching::Fetcher<BlockRangesRequest, StoreRanges<Types, S, P>>;
pub(super) type VidCommonRangesFetcher<Types, S, P> =
    fetching::Fetcher<VidCommonRangesRequest, StoreRanges<Types, S, P>>;

/// The heights to fetch, as a set of half-open ranges.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct RangesRequest(pub(super) Vec<Range<u64>>);

impl RangesRequest {
    fn heights(&self) -> impl Iterator<Item = u64> + '_ {
        self.0.iter().flat_map(|range| range.clone())
    }
}

impl FetchRequest for RangesRequest {
    fn might_exist(self, heights: Heights) -> bool {
        self.0.iter().all(|range| {
            heights.pruned_height.is_none_or(|h| h < range.start) && range.end <= heights.height
        })
    }
}

/// The objects a [`RangesRequest`] asked for.
#[derive(Clone, Debug)]
pub(super) struct Ranges<T>(pub(super) Vec<T>);

impl<T: HeightIndexed> Ranges<T> {
    /// Does this cover every height the request asked for?
    ///
    /// Fetched answers are checked against this before they resolve a request, so a peer that
    /// answers with only part of what it was asked for does not end the fetch.
    fn satisfies(&self, req: &RangesRequest) -> bool {
        let heights = self
            .0
            .iter()
            .map(|obj| obj.height())
            .collect::<HashSet<_>>();
        req.heights().all(|height| heights.contains(&height))
    }
}

/// The objects the notifiers delivered, or [`None`] if they do not cover the whole request.
///
/// A notifier is dropped only at shutdown, and then yields no object. Returning [`None`] makes the
/// passive fetch panic like every other object's does, rather than quietly resolving a partial
/// answer as if it were whole.
fn complete<T: HeightIndexed>(objs: Vec<Option<T>>, req: &RangesRequest) -> Option<Ranges<T>> {
    let ranges = Ranges(objs.into_iter().flatten().collect::<Vec<_>>());
    ranges.satisfies(req).then_some(ranges)
}

/// Stores fetched derived objects.
#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
pub(super) struct StoreRanges<Types: NodeType, S, P> {
    #[derivative(Debug = "ignore")]
    pub(super) fetcher: Arc<Fetcher<Types, S, P>>,
}

impl<Types: NodeType, S, P> PartialEq for StoreRanges<Types, S, P> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl<Types: NodeType, S, P> Eq for StoreRanges<Types, S, P> {}

impl<Types: NodeType, S, P> Ord for StoreRanges<Types, S, P> {
    fn cmp(&self, _other: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl<Types: NodeType, S, P> PartialOrd for StoreRanges<Types, S, P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Types, S, P> Callback<BlockRangesResponse<Types>> for StoreRanges<Types, S, P>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    async fn run(self, response: BlockRangesResponse<Types>) {
        // VID goes in first: block notifications are what resolve the block fetch, and the VID
        // scan that follows checks storage. Blocks first would let that scan run while these VID
        // writes are still in flight, and refetch what is already in hand.
        self.fetcher.store_runs(response.vid_common).await;
        self.fetcher.store_runs(response.blocks).await;
    }
}

impl<Types, S, P> Callback<Vec<VidCommonQueryData<Types>>> for StoreRanges<Types, S, P>
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

/// Stores fetched leaves, and continues on to whatever needed them.
#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
pub(super) enum LeafRangesCallback<Types: NodeType, S, P> {
    /// Store the leaves and backfill their cert2s.
    Store {
        #[derivative(Debug = "ignore")]
        fetcher: Arc<Fetcher<Types, S, P>>,
    },
    /// Fetch the blocks stored against these leaves.
    Blocks {
        #[derivative(Debug = "ignore")]
        fetcher: Arc<Fetcher<Types, S, P>>,
        req: RangesRequest,
    },
    /// Fetch the VID common stored against these leaves.
    VidCommon {
        #[derivative(Debug = "ignore")]
        fetcher: Arc<Fetcher<Types, S, P>>,
        req: RangesRequest,
    },
}

impl<Types: NodeType, S, P> PartialEq for LeafRangesCallback<Types, S, P> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl<Types: NodeType, S, P> Eq for LeafRangesCallback<Types, S, P> {}

impl<Types: NodeType, S, P> PartialOrd for LeafRangesCallback<Types, S, P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Types: NodeType, S, P> Ord for LeafRangesCallback<Types, S, P> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Store first, so the headers are in place before the derived fetches run. The request is
        // left out: every callback on one fetch carries the request that is that fetch's key.
        fn rank<Types: NodeType, S, P>(cb: &LeafRangesCallback<Types, S, P>) -> u8 {
            match cb {
                LeafRangesCallback::Store { .. } => 0,
                LeafRangesCallback::Blocks { .. } => 1,
                LeafRangesCallback::VidCommon { .. } => 2,
            }
        }
        rank(self).cmp(&rank(other))
    }
}

impl<Types, S, P> Callback<Vec<LeafQueryData<Types>>> for LeafRangesCallback<Types, S, P>
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
            Self::Blocks { fetcher, req } => fetch_block_ranges(fetcher, req),
            Self::VidCommon { fetcher, req } => fetch_vid_common_ranges(fetcher, req),
        }
    }
}

/// Fetch the leaves for `req`, then run `then`.
fn fetch_leaf_ranges_and_then<Types, S, P>(
    fetcher: Arc<Fetcher<Types, S, P>>,
    req: RangesRequest,
    then: Option<LeafRangesCallback<Types, S, P>>,
) where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    let store = LeafRangesCallback::Store {
        fetcher: fetcher.clone(),
    };
    fetcher.leaf_ranges_fetcher.clone().spawn_fetch(
        LeafRangesRequest(req.0),
        fetcher.provider.clone(),
        std::iter::once(store).chain(then),
        false,
    );
}

fn fetch_block_ranges<Types, S, P>(fetcher: Arc<Fetcher<Types, S, P>>, req: RangesRequest)
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
    let Some(block_fetcher) = &fetcher.block_ranges_fetcher else {
        return;
    };
    block_fetcher.clone().spawn_fetch(
        BlockRangesRequest(req.0),
        fetcher.provider.clone(),
        [StoreRanges {
            fetcher: fetcher.clone(),
        }],
        false,
    );
}

fn fetch_vid_common_ranges<Types, S, P>(fetcher: Arc<Fetcher<Types, S, P>>, req: RangesRequest)
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    let Some(vid_fetcher) = &fetcher.vid_common_ranges_fetcher else {
        return;
    };
    vid_fetcher.clone().spawn_fetch(
        VidCommonRangesRequest(req.0),
        fetcher.provider.clone(),
        [StoreRanges {
            fetcher: fetcher.clone(),
        }],
        false,
    );
}

/// Load the objects for `req` from storage, or [`QueryError::Missing`] if any height is absent.
///
/// Storage returns the rows it has, so an absent height is a short result rather than an error.
/// Without this check a request for missing heights would look complete and never be fetched.
fn load_ranges<T: HeightIndexed>(req: &RangesRequest, objs: Vec<T>) -> QueryResult<Ranges<T>> {
    let objs = Ranges(objs);
    if objs.satisfies(req) {
        Ok(objs)
    } else {
        Err(QueryError::Missing)
    }
}

#[async_trait]
impl<Types> Fetchable<Types> for Ranges<LeafQueryData<Types>>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    type Request = RangesRequest;

    fn satisfies(&self, req: Self::Request) -> bool {
        Ranges::satisfies(self, &req)
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
        fetch_leaf_ranges_and_then(fetcher, req, None);
        Ok(())
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        load_ranges(&req, storage.get_leaf_ranges(&req.0).await?)
    }
}

#[async_trait]
impl<Types> Fetchable<Types> for Ranges<BlockQueryData<Types>>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    type Request = RangesRequest;

    fn satisfies(&self, req: Self::Request) -> bool {
        Ranges::satisfies(self, &req)
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
        match <Ranges<LeafQueryData<Types>>>::load(tx, req.clone()).await {
            Ok(leaves) => {
                // A leaf fetch is what carries the cert2 backfill, and none runs when the leaves
                // are already stored, so request it here as `fetch_header_range_and_then` does.
                for leaf in &leaves.0 {
                    fetch_cert2_with_header(&fetcher, leaf.leaf().block_header());
                }
                fetch_block_ranges(fetcher, req)
            },
            Err(QueryError::Missing | QueryError::NotFound) => fetch_leaf_ranges_and_then(
                fetcher.clone(),
                req.clone(),
                Some(LeafRangesCallback::Blocks { fetcher, req }),
            ),
            Err(QueryError::Error { message }) => {
                anyhow::bail!("failed to load leaves for ranges {req:?}: {message}")
            },
        }
        Ok(())
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        load_ranges(&req, storage.get_block_ranges(&req.0).await?)
    }
}

#[async_trait]
impl<Types> Fetchable<Types> for Ranges<VidCommonQueryData<Types>>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    type Request = RangesRequest;

    fn satisfies(&self, req: Self::Request) -> bool {
        Ranges::satisfies(self, &req)
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

        match <Ranges<LeafQueryData<Types>>>::load(tx, req.clone()).await {
            Ok(leaves) => {
                // As in the block path: no leaf fetch runs for leaves that are already stored, so
                // nothing else would request their cert2.
                for leaf in &leaves.0 {
                    fetch_cert2_with_header(&fetcher, leaf.leaf().block_header());
                }
                fetch_vid_common_ranges(fetcher, req)
            },
            Err(QueryError::Missing | QueryError::NotFound) => fetch_leaf_ranges_and_then(
                fetcher.clone(),
                req.clone(),
                Some(LeafRangesCallback::VidCommon { fetcher, req }),
            ),
            Err(QueryError::Error { message }) => {
                anyhow::bail!("failed to load leaves for ranges {req:?}: {message}")
            },
        }
        Ok(())
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        load_ranges(&req, storage.get_vid_common_ranges(&req.0).await?)
    }
}
