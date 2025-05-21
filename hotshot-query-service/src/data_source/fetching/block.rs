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

//! [`Fetchable`] implementation for [`BlockQueryData`] and [`PayloadQueryData`].

use std::{cmp::Ordering, future::IntoFuture, iter::once, ops::RangeBounds, sync::Arc};

use async_trait::async_trait;
use derivative::Derivative;
use derive_more::From;
use futures::future::{BoxFuture, FutureExt};
use hotshot_types::traits::{block_contents::BlockHeader, node_implementation::NodeType};

use super::{
    header::{fetch_header_and_then, HeaderCallback},
    AvailabilityProvider, FetchRequest, Fetchable, Fetcher, Heights, Notifiers, RangedFetchable,
    Storable,
};
use crate::{
    availability::{BlockId, BlockQueryData, PayloadMetadata, PayloadQueryData, QueryablePayload},
    data_source::{
        storage::{
            pruning::PrunedHeightStorage, AvailabilityStorage, NodeStorage,
            UpdateAvailabilityStorage,
        },
        VersionedDataSource,
    },
    fetching::{
        self,
        request::{self, PayloadRequest},
        Callback,
    },
    types::HeightIndexed,
    Header, Payload, QueryResult,
};
pub(super) type PayloadFetcher<Types, S, P> =
    fetching::Fetcher<request::PayloadRequest, PayloadCallback<Types, S, P>>;

impl<Types> FetchRequest for BlockId<Types>
where
    Types: NodeType,
{
    fn might_exist(self, heights: Heights) -> bool {
        if let BlockId::Number(n) = self {
            heights.might_exist(n as u64)
        } else {
            true
        }
    }
}

#[async_trait]
impl<Types> Fetchable<Types> for BlockQueryData<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    type Request = BlockId<Types>;

    fn satisfies(&self, req: Self::Request) -> bool {
        match req {
            BlockId::Number(n) => self.height() == n as u64,
            BlockId::Hash(h) => self.hash() == h,
            BlockId::PayloadHash(h) => self.payload_hash() == h,
        }
    }

    async fn passive_fetch(
        notifiers: &Notifiers<Types>,
        req: Self::Request,
    ) -> BoxFuture<'static, Option<Self>> {
        notifiers
            .block
            .wait_for(move |block| block.satisfies(req))
            .await
            .into_future()
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
        fetch_header_and_then(
            tx,
            req,
            HeaderCallback::Payload {
                fetcher: fetcher.clone(),
            },
        )
        .await
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        storage.get_block(req).await
    }
}

#[async_trait]
impl<Types> RangedFetchable<Types> for BlockQueryData<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    type RangedRequest = BlockId<Types>;

    async fn load_range<S, R>(storage: &mut S, range: R) -> QueryResult<Vec<QueryResult<Self>>>
    where
        S: AvailabilityStorage<Types>,
        R: RangeBounds<usize> + Send + 'static,
    {
        storage.get_block_range(range).await
    }
}

impl<Types> Storable<Types> for BlockQueryData<Types>
where
    Types: NodeType,
{
    fn name() -> &'static str {
        "block"
    }

    async fn notify(&self, notifiers: &Notifiers<Types>) {
        notifiers.block.notify(self).await;
    }

    async fn store(
        self,
        storage: &mut (impl UpdateAvailabilityStorage<Types> + Send),
        leaf_only: bool,
    ) -> anyhow::Result<()> {
        if leaf_only {
            return Ok(());
        }

        storage.insert_block(self).await
    }
}

pub(super) fn fetch_block_with_header<Types, S, P>(
    fetcher: Arc<Fetcher<Types, S, P>>,
    header: Header<Types>,
) where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    P: AvailabilityProvider<Types>,
{
    let Some(payload_fetcher) = fetcher.payload_fetcher.as_ref() else {
        // If we're in light-weight mode, we don't need to fetch the VID common data.
        return;
    };

    // Now that we have the header, we only need to retrieve the payload.
    tracing::info!(
        "spawned active fetch for payload {:?} (height {})",
        header.payload_commitment(),
        header.block_number()
    );
    payload_fetcher.spawn_fetch(
        PayloadRequest(header.payload_commitment()),
        fetcher.provider.clone(),
        once(PayloadCallback {
            header,
            fetcher: fetcher.clone(),
        }),
    );
}

#[async_trait]
impl<Types> Fetchable<Types> for PayloadQueryData<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    type Request = BlockId<Types>;

    fn satisfies(&self, req: Self::Request) -> bool {
        match req {
            BlockId::Number(n) => self.height() == n as u64,
            BlockId::Hash(h) => self.block_hash() == h,
            BlockId::PayloadHash(h) => self.hash() == h,
        }
    }

    async fn passive_fetch(
        notifiers: &Notifiers<Types>,
        req: Self::Request,
    ) -> BoxFuture<'static, Option<Self>> {
        notifiers
            .block
            .wait_for(move |block| block.satisfies(req))
            .await
            .into_future()
            .map(|block| block.map(PayloadQueryData::from))
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
        // We don't have storage for the payload alone, only the whole block. So if we need to fetch
        // the payload, we just fetch the whole block (which may end up fetching only the payload,
        // if that's all that's needed to complete the block).
        BlockQueryData::active_fetch(tx, fetcher, req).await
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        storage.get_payload(req).await
    }
}

#[async_trait]
impl<Types> RangedFetchable<Types> for PayloadQueryData<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    type RangedRequest = BlockId<Types>;

    async fn load_range<S, R>(storage: &mut S, range: R) -> QueryResult<Vec<QueryResult<Self>>>
    where
        S: AvailabilityStorage<Types>,
        R: RangeBounds<usize> + Send + 'static,
    {
        storage.get_payload_range(range).await
    }
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
pub(super) struct PayloadCallback<Types: NodeType, S, P> {
    header: Header<Types>,
    #[derivative(Debug = "ignore")]
    fetcher: Arc<Fetcher<Types, S, P>>,
}

impl<Types: NodeType, S, P> PartialEq for PayloadCallback<Types, S, P> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl<Types: NodeType, S, P> Eq for PayloadCallback<Types, S, P> {}

impl<Types: NodeType, S, P> Ord for PayloadCallback<Types, S, P> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.header.block_number().cmp(&other.header.block_number())
    }
}

impl<Types: NodeType, S, P> PartialOrd for PayloadCallback<Types, S, P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Types: NodeType, S, P> Callback<Payload<Types>> for PayloadCallback<Types, S, P>
where
    Payload<Types>: QueryablePayload<Types>,
    S: 'static + VersionedDataSource,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    P: AvailabilityProvider<Types>,
{
    async fn run(self, payload: Payload<Types>) {
        tracing::info!("fetched payload {:?}", self.header.payload_commitment());
        let block = BlockQueryData::new(self.header, payload);
        self.fetcher.store_and_notify(block).await;
    }
}

#[async_trait]
impl<Types> Fetchable<Types> for PayloadMetadata<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    type Request = BlockId<Types>;

    fn satisfies(&self, req: Self::Request) -> bool {
        match req {
            BlockId::Number(n) => self.height == n as u64,
            BlockId::Hash(h) => self.block_hash == h,
            BlockId::PayloadHash(h) => self.hash == h,
        }
    }

    async fn passive_fetch(
        notifiers: &Notifiers<Types>,
        req: Self::Request,
    ) -> BoxFuture<'static, Option<Self>> {
        notifiers
            .block
            .wait_for(move |block| block.satisfies(req))
            .await
            .into_future()
            .map(|opt| opt.map(Self::from))
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
        // Trigger the full block to be fetched. This will be enough to satisfy this request for the
        // payload summary.
        BlockQueryData::active_fetch(tx, fetcher, req).await
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        storage.get_payload_metadata(req).await
    }
}

#[async_trait]
impl<Types> RangedFetchable<Types> for PayloadMetadata<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    type RangedRequest = BlockId<Types>;

    async fn load_range<S, R>(storage: &mut S, range: R) -> QueryResult<Vec<QueryResult<Self>>>
    where
        S: AvailabilityStorage<Types>,
        R: RangeBounds<usize> + Send + 'static,
    {
        storage.get_payload_metadata_range(range).await
    }
}
