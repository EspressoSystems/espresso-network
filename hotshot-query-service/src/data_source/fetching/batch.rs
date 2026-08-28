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

use std::{ops::Range, sync::Arc};

use async_trait::async_trait;
use backoff::backoff::Backoff;
use futures::future::{BoxFuture, FutureExt, join_all};
use hotshot_types::traits::node_implementation::NodeType;

use super::{
    AvailabilityProvider, FetchRequest, Fetchable, Fetcher, Heights, Notifiers,
    header::HeaderCallback,
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
    fetching::request::{BlockBatchRequest, LeafBatchRequest, VidCommonBatchRequest},
    types::HeightIndexed,
};

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
        let heights = self.0.iter().map(|obj| obj.height()).collect::<Vec<_>>();
        req.heights().all(|height| heights.contains(&height))
    }
}

/// Fetch a batch from a provider, retrying until it is complete, then store what came back.
///
/// Retrying is safe because the scanner only asks for heights the chain has already produced, and
/// because the provider falls back to per-range fetches against peers without the batch endpoints.
///
/// `$needs_leaves` fetches the leaves for the same heights first, the way
/// [`fetch_header_range_and_then`](super::header::fetch_header_range_and_then) does for a range:
/// derived objects are stored against the headers the leaves carry, and the cert2 backfill chains
/// off that leaf fetch.
macro_rules! spawn_batch_fetch {
    ($fetcher:expr, $req:expr, $provider_req:expr, $store:expr, $needs_leaves:expr) => {{
        let fetcher = $fetcher;
        let req = $req;
        tokio::spawn(async move {
            if $needs_leaves {
                fetcher
                    .get::<Batch<LeafQueryData<Types>>>(req.clone())
                    .await
                    .await;
            }

            let mut backoff = fetcher.backoff.clone();
            backoff.reset();
            loop {
                let permit = fetcher.retry_semaphore.acquire().await;
                if let Some(objs) = fetcher.provider.fetch($provider_req(req.0.clone())).await {
                    drop(permit);
                    let objs = Batch(objs);
                    if objs.satisfies(&req) {
                        $store(fetcher.clone(), objs).await;
                        return;
                    }
                    tracing::info!(
                        ?req,
                        fetched = objs.0.len(),
                        "batch was answered incompletely, will retry"
                    );
                } else {
                    drop(permit);
                }

                let delay = backoff
                    .next_backoff()
                    .unwrap_or(std::time::Duration::from_secs(32));
                tracing::warn!(?req, ?delay, "failed to fetch batch, will retry");
                tokio::time::sleep(delay).await;
            }
        });
    }};
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
            .map(|objs| Some(Batch(objs.into_iter().flatten().collect())))
            .boxed()
    }

    async fn active_fetch<S, P>(
        _tx: &mut impl AvailabilityStorage<Types>,
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
        spawn_batch_fetch!(
            fetcher,
            req,
            LeafBatchRequest,
            |fetcher: Arc<Fetcher<Types, S, P>>, objs: Batch<LeafQueryData<Types>>| async move {
                // Leaves fetched this way skip the callbacks a leaf fetch would have run, so chain
                // the cert2 backfill onto them the same way the range path does.
                for run in fetcher.store_runs(objs.0).await {
                    HeaderCallback::Cert2 {
                        fetcher: fetcher.clone(),
                    }
                    .run_range(&run);
                }
            },
            false
        );
        Ok(())
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        let mut objs = vec![];
        for range in &req.0 {
            objs.extend(
                storage
                    .get_leaf_range(range.start as usize..range.end as usize)
                    .await?
                    .into_iter()
                    .collect::<QueryResult<Vec<_>>>()?,
            );
        }

        // Storage returns the rows it has, so an absent height is a short result rather than an
        // error. Without this the batch would look complete and never be fetched.
        let objs = Batch(objs);
        if !objs.satisfies(&req) {
            return Err(QueryError::Missing);
        }
        Ok(objs)
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
            .map(|objs| Some(Batch(objs.into_iter().flatten().collect())))
            .boxed()
    }

    async fn active_fetch<S, P>(
        _tx: &mut impl AvailabilityStorage<Types>,
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
        spawn_batch_fetch!(
            fetcher,
            req,
            BlockBatchRequest,
            |fetcher: Arc<Fetcher<Types, S, P>>, objs: Batch<BlockQueryData<Types>>| async move {
                fetcher.store_runs(objs.0).await;
            },
            true
        );
        Ok(())
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        let mut objs = vec![];
        for range in &req.0 {
            objs.extend(
                storage
                    .get_block_range(range.start as usize..range.end as usize)
                    .await?
                    .into_iter()
                    .collect::<QueryResult<Vec<_>>>()?,
            );
        }

        // Storage returns the rows it has, so an absent height is a short result rather than an
        // error. Without this the batch would look complete and never be fetched.
        let objs = Batch(objs);
        if !objs.satisfies(&req) {
            return Err(QueryError::Missing);
        }
        Ok(objs)
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
            .map(|objs| Some(Batch(objs.into_iter().flatten().collect())))
            .boxed()
    }

    async fn active_fetch<S, P>(
        _tx: &mut impl AvailabilityStorage<Types>,
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
        spawn_batch_fetch!(
            fetcher,
            req,
            VidCommonBatchRequest,
            |fetcher: Arc<Fetcher<Types, S, P>>, objs: Batch<VidCommonQueryData<Types>>| async move {
                fetcher.store_runs(objs.0).await;
            },
            true
        );
        Ok(())
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        let mut objs = vec![];
        for range in &req.0 {
            objs.extend(
                storage
                    .get_vid_common_range(range.start as usize..range.end as usize)
                    .await?
                    .into_iter()
                    .collect::<QueryResult<Vec<_>>>()?,
            );
        }

        // Storage returns the rows it has, so an absent height is a short result rather than an
        // error. Without this the batch would look complete and never be fetched.
        let objs = Batch(objs);
        if !objs.satisfies(&req) {
            return Err(QueryError::Missing);
        }
        Ok(objs)
    }
}
