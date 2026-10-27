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

use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    future::IntoFuture,
    iter::once,
    ops::RangeBounds,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;
use async_trait::async_trait;
use derivative::Derivative;
use futures::future::{BoxFuture, FutureExt, join_all, try_join_all};
use hotshot_types::traits::{
    EncodeBytes, block_contents::BlockHeader, node_implementation::NodeType,
};
use parking_lot::Mutex;
use tokio::{spawn, time::sleep};

use super::{
    AvailabilityProvider, FetchRequest, Fetchable, Fetcher, Heights, Notifiers, RangedFetchable,
    Storable,
    header::{HeaderCallback, fetch_header_and_then},
};
use crate::{
    Header, Payload, QueryError, QueryResult,
    availability::{
        BlockId, BlockQueryData, PayloadMetadata, PayloadQueryData, QueryableHeader,
        QueryablePayload,
    },
    data_source::{
        VersionedDataSource,
        fetching::{header::fetch_header_range_and_then, leaf::RangeRequest},
        storage::{
            AvailabilityStorage, NodeStorage, UpdateAvailabilityStorage,
            blob::{BlobStore, StagedBlobs},
            pruning::PrunedHeightStorage,
        },
    },
    fetching::{
        self, Callback, NonEmptyRange,
        request::{self, BlockRangeRequest, PayloadRequest},
    },
    types::HeightIndexed,
};
pub(super) type PayloadFetcher<Types, S, P> =
    fetching::Fetcher<request::PayloadRequest, PayloadCallback<Types, S, P>>;

pub(super) type PayloadRangeFetcher<Types, S, P> =
    fetching::Fetcher<request::BlockRangeRequest, BlockRangeCallback<Types, S, P>>;

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
    Header<Types>: QueryableHeader<Types>,
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
        // VID common is missing whenever the payload is, and nothing else would fetch it for a
        // header a leaf stream stored.
        fetch_header_and_then(
            tx,
            req,
            [
                HeaderCallback::Payload {
                    fetcher: fetcher.clone(),
                },
                HeaderCallback::VidCommon { fetcher },
            ],
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
    Header<Types>: QueryableHeader<Types>,
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
    fn debug_name(&self) -> String {
        format!("block {}", self.height())
    }

    async fn notify(&self, notifiers: &Notifiers<Types>) {
        notifiers.block.notify(self).await;
    }

    async fn stage(
        &self,
        blobs: Option<&BlobStore>,
        leaf_only: bool,
    ) -> anyhow::Result<StagedBlobs> {
        stage_payloads(blobs, leaf_only, [self]).await
    }

    async fn store(
        &self,
        storage: &mut impl UpdateAvailabilityStorage<Types>,
        leaf_only: bool,
    ) -> anyhow::Result<()> {
        if leaf_only {
            return Ok(());
        }

        storage.insert_block(self).await
    }
}

/// Appends the non-empty payloads of `blocks` to the payload lane, concurrently so that they
/// share fsyncs. Empty payloads are stored inline.
async fn stage_payloads<'a, Types: NodeType>(
    blobs: Option<&BlobStore>,
    leaf_only: bool,
    blocks: impl IntoIterator<Item = &'a BlockQueryData<Types>>,
) -> anyhow::Result<StagedBlobs> {
    let Some(blobs) = blobs.filter(|_| !leaf_only) else {
        return Ok(StagedBlobs::new());
    };
    try_join_all(
        blocks
            .into_iter()
            .filter(|block| block.size() != 0)
            .map(|block| async move {
                let body = block.payload.encode().as_ref().to_vec();
                let loc = blobs.append_payload(block.height(), body).await?;
                Ok((block.height(), loc))
            }),
    )
    .await
}

/// Fetch the payload of `header` from a provider.
///
/// A block decided by this process without its payload may get it from the node's own consensus,
/// see [`Builder::with_local_payload_grace`](super::Builder::with_local_payload_grace). Its fetch
/// waits out the rest of the grace and is dropped if the payload was stored meanwhile. A request
/// for the block waits on the passive notifier, which storing the local payload triggers. Other
/// heights are fetched at once.
pub(super) fn fetch_block_with_header<Types, S, P>(
    fetcher: Arc<Fetcher<Types, S, P>>,
    header: Header<Types>,
) where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    if fetcher.payload_fetcher.is_none() {
        // If we're in light-weight mode, we don't need to fetch the block data.
        return;
    }
    let height = header.block_number();
    let wait = fetcher.local_grace.remaining(height);
    if wait.is_zero() {
        spawn_payload_fetch(&fetcher, header);
        return;
    }
    if !fetcher.local_grace.begin_wait(height) {
        return;
    }

    tracing::debug!(
        height,
        ?wait,
        "waiting for the local payload before fetching it"
    );
    spawn(async move {
        sleep(wait).await;
        fetcher.local_grace.end_wait(height);
        match fetcher.payload_stored(height).await {
            Ok(true) => tracing::debug!(height, "payload stored locally, not fetching it"),
            Ok(false) => spawn_payload_fetch(&fetcher, header),
            Err(err) => {
                tracing::warn!(height, "fetching payload: {err:#}");
                spawn_payload_fetch(&fetcher, header);
            },
        }
    });
}

/// When this process decided blocks whose payload was missing, and the heights waiting out the
/// grace for them.
#[derive(Debug)]
pub(super) struct LocalPayloadGrace {
    grace: Duration,
    decided: Mutex<HashMap<u64, Instant>>,
    waiting: Mutex<HashSet<u64>>,
}

impl LocalPayloadGrace {
    pub(super) fn new(grace: Duration) -> Self {
        Self {
            grace,
            decided: Default::default(),
            waiting: Default::default(),
        }
    }

    pub(super) fn note_decided(&self, height: u64) {
        if self.grace.is_zero() {
            return;
        }
        let mut decided = self.decided.lock();
        decided.retain(|_, at| at.elapsed() < self.grace);
        decided.insert(height, Instant::now());
    }

    /// What is left of the grace since the decide of `height`; zero for any other height.
    fn remaining(&self, height: u64) -> Duration {
        self.decided
            .lock()
            .get(&height)
            .map_or(Duration::ZERO, |at| self.grace.saturating_sub(at.elapsed()))
    }

    /// Whether the caller is the first to wait for `height`.
    fn begin_wait(&self, height: u64) -> bool {
        self.waiting.lock().insert(height)
    }

    fn end_wait(&self, height: u64) {
        self.waiting.lock().remove(&height);
    }

    #[cfg(test)]
    pub(super) fn waiting(&self) -> usize {
        self.waiting.lock().len()
    }
}

fn spawn_payload_fetch<Types, S, P>(fetcher: &Arc<Fetcher<Types, S, P>>, header: Header<Types>)
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    let Some(payload_fetcher) = fetcher.payload_fetcher.as_ref() else {
        return;
    };
    tracing::debug!(
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
        true,
    );
}

#[async_trait]
impl<Types> Fetchable<Types> for PayloadQueryData<Types>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
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
    Header<Types>: QueryableHeader<Types>,
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
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: 'static + VersionedDataSource,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    async fn run(self, payload: Payload<Types>) {
        tracing::debug!("fetched payload {:?}", self.header.payload_commitment());
        let height = self.header.block_number();
        let block = BlockQueryData::new(self.header, payload);
        // The node's own payload may have been stored while this fetch was in flight, and
        // storing rewrites the whole payload. Waiters still get notified: the block can have
        // become readable without a notification, e.g. through a payload shared with another
        // height.
        if block_stored(&self.fetcher, height).await {
            tracing::debug!(height, "fetched payload already stored");
            block.notify(&self.fetcher.notifiers).await;
            return;
        }
        self.fetcher.store_and_notify(&block).await;
    }

    fn height(&self) -> Option<u64> {
        Some(self.header.block_number())
    }
}

/// Whether the block at `height` is stored per [`Fetcher::payload_stored`] and its payload also
/// reads back, so a fetch still replaces an unreadable payload record. Only a fetched payload
/// pays for that read.
async fn block_stored<Types, S, P>(fetcher: &Fetcher<Types, S, P>, height: u64) -> bool
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    let checked = async {
        if !fetcher.payload_stored(height).await? {
            return Ok(false);
        }
        let mut tx = fetcher.read().await.context("opening read transaction")?;
        anyhow::Ok(tx.get_block(BlockId::Number(height as usize)).await.is_ok())
    };
    checked.await.unwrap_or_else(|err| {
        tracing::warn!(height, "checking whether block is stored: {err:#}");
        false
    })
}

#[async_trait]
impl<Types> Fetchable<Types> for PayloadMetadata<Types>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
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
    Header<Types>: QueryableHeader<Types>,
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

#[async_trait]
impl<Types> Fetchable<Types> for NonEmptyRange<BlockQueryData<Types>>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    type Request = RangeRequest;

    fn satisfies(&self, req: Self::Request) -> bool {
        req.is_satisfied(self)
    }

    async fn passive_fetch(
        notifiers: &Notifiers<Types>,
        req: Self::Request,
    ) -> BoxFuture<'static, Option<Self>> {
        let waits = join_all(req.into_iter().map(|i| {
            notifiers
                .block
                .wait_for(move |block| block.satisfies(BlockId::Number(i as usize)))
        }))
        .await;

        join_all(waits.into_iter().map(|wait| wait.into_future()))
            .map(|options| NonEmptyRange::new(options.into_iter().flatten()).ok())
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
        fetch_header_range_and_then(
            tx,
            req,
            [
                HeaderCallback::Payload {
                    fetcher: fetcher.clone(),
                },
                HeaderCallback::VidCommon { fetcher },
            ],
        )
        .await
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        let blocks = storage
            .get_block_range((req.start as usize)..(req.end as usize))
            .await?
            .into_iter()
            .collect::<QueryResult<Vec<_>>>()?;
        if blocks.len() != req.len() {
            tracing::debug!(
                ?req,
                len = blocks.len(),
                "database returned partial result, unable to load full range"
            );
            return Err(QueryError::Missing);
        }
        NonEmptyRange::new(blocks).map_err(|err| QueryError::Error {
            message: format!("expected contiguous range, but: {err:#}"),
        })
    }
}

impl<Types> Storable<Types> for NonEmptyRange<BlockQueryData<Types>>
where
    Types: NodeType,
{
    fn debug_name(&self) -> String {
        format!("block range {}..{}", self.start(), self.end())
    }

    async fn notify(&self, notifiers: &Notifiers<Types>) {
        for block in self {
            notifiers.block.notify(block).await;
        }
    }

    async fn stage(
        &self,
        blobs: Option<&BlobStore>,
        leaf_only: bool,
    ) -> anyhow::Result<StagedBlobs> {
        stage_payloads(blobs, leaf_only, self).await
    }

    async fn store(
        &self,
        storage: &mut impl UpdateAvailabilityStorage<Types>,
        leaf_only: bool,
    ) -> anyhow::Result<()> {
        if leaf_only {
            return Ok(());
        }

        storage.insert_block_range(self).await
    }
}

pub(super) fn fetch_block_range<Types, S, P>(
    fetcher: Arc<Fetcher<Types, S, P>>,
    start: u64,
    end: u64,
) where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    P: AvailabilityProvider<Types>,
{
    let Some(payload_range_fetcher) = fetcher.payload_range_fetcher.as_ref() else {
        // If we're in light-weight mode, we don't need to fetch the block data.
        return;
    };

    tracing::info!("spawned active fetch for payload range {start}..{end}");
    payload_range_fetcher.spawn_fetch(
        BlockRangeRequest { start, end },
        fetcher.provider.clone(),
        once(BlockRangeCallback {
            fetcher: fetcher.clone(),
        }),
        true,
    );
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
pub(super) struct BlockRangeCallback<Types: NodeType, S, P> {
    #[derivative(Debug = "ignore")]
    fetcher: Arc<Fetcher<Types, S, P>>,
}

impl<Types: NodeType, S, P> PartialEq for BlockRangeCallback<Types, S, P> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl<Types: NodeType, S, P> Eq for BlockRangeCallback<Types, S, P> {}

impl<Types: NodeType, S, P> Ord for BlockRangeCallback<Types, S, P> {
    fn cmp(&self, _: &Self) -> Ordering {
        // All callbacks for a given block range request do the same thing: just store the range.
        Ordering::Equal
    }
}

impl<Types: NodeType, S, P> PartialOrd for BlockRangeCallback<Types, S, P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Types: NodeType, S, P> Callback<NonEmptyRange<BlockQueryData<Types>>>
    for BlockRangeCallback<Types, S, P>
where
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: 'static + VersionedDataSource,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    P: AvailabilityProvider<Types>,
{
    async fn run(self, blocks: NonEmptyRange<BlockQueryData<Types>>) {
        tracing::info!("fetched blocks {}..{}", blocks.start(), blocks.end());
        self.fetcher.store_and_notify(&blocks).await;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_local_payload_grace() {
        let grace = LocalPayloadGrace::new(Duration::from_secs(60));
        assert_eq!(grace.remaining(1), Duration::ZERO);

        grace.note_decided(1);
        let remaining = grace.remaining(1);
        assert!(remaining > Duration::from_secs(59) && remaining <= Duration::from_secs(60));
        assert_eq!(grace.remaining(2), Duration::ZERO);

        assert!(grace.begin_wait(1));
        assert!(!grace.begin_wait(1));
        grace.end_wait(1);
        assert!(grace.begin_wait(1));
    }

    #[test]
    fn test_local_payload_grace_zero_records_nothing() {
        let grace = LocalPayloadGrace::new(Duration::ZERO);
        grace.note_decided(1);
        assert_eq!(grace.remaining(1), Duration::ZERO);
    }

    #[test]
    fn test_local_payload_grace_forgets_expired_decides() {
        let grace = LocalPayloadGrace::new(Duration::from_millis(10));
        grace.note_decided(1);
        std::thread::sleep(Duration::from_millis(20));
        grace.note_decided(2);
        assert_eq!(grace.decided.lock().len(), 1);
    }
}
