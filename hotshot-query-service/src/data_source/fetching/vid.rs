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

//! [`Fetchable`] implementation for [`VidCommonQueryData`].

use std::{cmp::Ordering, future::IntoFuture, iter::once, ops::RangeBounds, sync::Arc};

use async_trait::async_trait;
use derivative::Derivative;
use derive_more::From;
use futures::future::{BoxFuture, FutureExt};
use hotshot_types::{
    data::{VidCommon, VidShare},
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
};

use super::{
    header::{fetch_header_and_then, HeaderCallback},
    AvailabilityProvider, FetchRequest, Fetchable, Fetcher, Heights, Notifiers, RangedFetchable,
    Storable,
};
use crate::{
    availability::{
        BlockId, QueryableHeader, QueryablePayload, VidCommonMetadata, VidCommonQueryData,
    },
    data_source::{
        storage::{
            pruning::PrunedHeightStorage, AvailabilityStorage, NodeStorage,
            UpdateAvailabilityStorage,
        },
        VersionedDataSource,
    },
    fetching::{self, request, Callback},
    types::HeightIndexed,
    Header, Payload, QueryResult,
};

pub(super) type VidCommonFetcher<Types, S, P> =
    fetching::Fetcher<request::VidCommonRequest, VidCommonCallback<Types, S, P>>;

#[derive(Clone, Copy, Debug, From)]
pub(super) struct VidCommonRequest<Types: NodeType>(BlockId<Types>);

impl<Types: NodeType> From<usize> for VidCommonRequest<Types> {
    fn from(n: usize) -> Self {
        Self(n.into())
    }
}

impl<Types> FetchRequest for VidCommonRequest<Types>
where
    Types: NodeType,
{
    fn might_exist(self, heights: Heights) -> bool {
        self.0.might_exist(heights)
    }
}

#[async_trait]
impl<Types> Fetchable<Types> for VidCommonQueryData<Types>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    type Request = VidCommonRequest<Types>;

    fn satisfies(&self, req: Self::Request) -> bool {
        match req.0 {
            BlockId::Number(n) => self.height() == n as u64,
            BlockId::Hash(h) => self.block_hash() == h,
            BlockId::PayloadHash(h) => self.payload_hash() == h,
        }
    }

    async fn passive_fetch(
        notifiers: &Notifiers<Types>,
        req: Self::Request,
    ) -> BoxFuture<'static, Option<Self>> {
        notifiers
            .vid_common
            .wait_for(move |vid| vid.satisfies(req))
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
            req.0,
            HeaderCallback::VidCommon {
                fetcher: fetcher.clone(),
            },
        )
        .await
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        storage.get_vid_common(req.0).await
    }
}

#[async_trait]
impl<Types> RangedFetchable<Types> for VidCommonQueryData<Types>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    type RangedRequest = VidCommonRequest<Types>;

    async fn load_range<S, R>(storage: &mut S, range: R) -> QueryResult<Vec<QueryResult<Self>>>
    where
        S: AvailabilityStorage<Types>,
        R: RangeBounds<usize> + Send + 'static,
    {
        storage.get_vid_common_range(range).await
    }
}

impl<Types> Storable<Types> for VidCommonQueryData<Types>
where
    Types: NodeType,
{
    fn name() -> &'static str {
        "VID common"
    }

    async fn notify(&self, notifiers: &Notifiers<Types>) {
        notifiers.vid_common.notify(self).await;
    }

    async fn store(
        self,
        storage: &mut (impl UpdateAvailabilityStorage<Types> + Send),
        _leaf_only: bool,
    ) -> anyhow::Result<()> {
        storage.insert_vid(self, None).await
    }
}

impl<Types> Storable<Types> for (VidCommonQueryData<Types>, Option<VidShare>)
where
    Types: NodeType,
{
    fn name() -> &'static str {
        "VID data"
    }

    async fn notify(&self, notifiers: &Notifiers<Types>) {
        notifiers.vid_common.notify(&self.0).await;
    }

    async fn store(
        self,
        storage: &mut (impl UpdateAvailabilityStorage<Types> + Send),
        _leaf_only: bool,
    ) -> anyhow::Result<()> {
        storage.insert_vid(self.0, self.1).await
    }
}

pub(super) fn fetch_vid_common_with_header<Types, S, P>(
    fetcher: Arc<Fetcher<Types, S, P>>,
    header: Header<Types>,
) where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    P: AvailabilityProvider<Types>,
{
    let Some(vid_fetcher) = fetcher.vid_common_fetcher.as_ref() else {
        tracing::info!("not fetching vid because of leaf only mode");
        return;
    };

    // Now that we have the header, we only need to retrieve the VID common data.
    tracing::info!(
        "spawned active fetch for VID common {:?} (height {})",
        header.payload_commitment(),
        header.block_number()
    );
    vid_fetcher.spawn_fetch(
        request::VidCommonRequest(header.payload_commitment()),
        fetcher.provider.clone(),
        once(VidCommonCallback {
            header,
            fetcher: fetcher.clone(),
        }),
    );
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
pub(super) struct VidCommonCallback<Types: NodeType, S, P> {
    header: Header<Types>,
    #[derivative(Debug = "ignore")]
    fetcher: Arc<Fetcher<Types, S, P>>,
}

impl<Types: NodeType, S, P> PartialEq for VidCommonCallback<Types, S, P> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl<Types: NodeType, S, P> Eq for VidCommonCallback<Types, S, P> {}

impl<Types: NodeType, S, P> Ord for VidCommonCallback<Types, S, P> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.header.block_number().cmp(&other.header.block_number())
    }
}

impl<Types: NodeType, S, P> PartialOrd for VidCommonCallback<Types, S, P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Types: NodeType, S, P> Callback<VidCommon> for VidCommonCallback<Types, S, P>
where
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    P: AvailabilityProvider<Types>,
{
    async fn run(self, common: VidCommon) {
        let common = VidCommonQueryData::new(self.header, common);
        self.fetcher.store_and_notify(common).await;
    }
}

#[async_trait]
impl<Types> Fetchable<Types> for VidCommonMetadata<Types>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    type Request = VidCommonRequest<Types>;

    fn satisfies(&self, req: Self::Request) -> bool {
        match req.0 {
            BlockId::Number(n) => self.height == n as u64,
            BlockId::Hash(h) => self.block_hash == h,
            BlockId::PayloadHash(h) => self.payload_hash == h,
        }
    }

    async fn passive_fetch(
        notifiers: &Notifiers<Types>,
        req: Self::Request,
    ) -> BoxFuture<'static, Option<Self>> {
        notifiers
            .vid_common
            .wait_for(move |vid| vid.satisfies(req))
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
        // Do not fetch if we are in leaf only mode
        if fetcher.leaf_only {
            return Ok(());
        }
        // Trigger the full VID object to be fetched. This will be enough to satisfy this request
        // for the summary.
        VidCommonQueryData::active_fetch(tx, fetcher, req).await
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        storage.get_vid_common_metadata(req.0).await
    }
}

#[async_trait]
impl<Types> RangedFetchable<Types> for VidCommonMetadata<Types>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    type RangedRequest = VidCommonRequest<Types>;

    async fn load_range<S, R>(storage: &mut S, range: R) -> QueryResult<Vec<QueryResult<Self>>>
    where
        S: AvailabilityStorage<Types>,
        R: RangeBounds<usize> + Send + 'static,
    {
        storage.get_vid_common_metadata_range(range).await
    }
}
