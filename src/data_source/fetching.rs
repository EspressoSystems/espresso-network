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

//! Asynchronous retrieval of missing data.
//!
//! [`FetchingDataSource`] combines a local storage implementation with a remote data availability
//! provider to create a data sources which caches data locally, but which is capable of fetching
//! missing data from a remote source, either proactively or on demand.
//!
//! This implementation supports three kinds of data fetching.
//!
//! # Proactive Fetching
//!
//! Proactive fetching means actively scanning the local database for missing objects and
//! proactively retrieving them from a remote provider, even if those objects have yet to be
//! requested by a client. Doing this increases the chance of success and decreases latency when a
//! client does eventually ask for those objects. This is also the mechanism by which a query
//! service joining a network late, or having been offline for some time, is able to catch up with
//! the events on the network that it missed.
//!
//! The current implementation of proactive fetching is meant to be the simplest effective algorithm
//! which still gives us a reasonable range of configuration options for experimentation. It is
//! subject to change as we learn about the behavior of proactive fetching in a realistic system.
//!
//! Proactive fetching is currently implemented by a background task which performs periodic scans
//! of the database, identifying and retrieving missing objects. This task is generally low
//! priority, since missing objects are rare, and it will take care not to monopolize resources that
//! could be used to serve requests. To reduce load and to optimize for the common case where blocks
//! are usually not missing once they have already been retrieved, we distinguish between _major_
//! and _minor_ scans.
//!
//! Minor scans are lightweight and can run very frequently. They will only look for missing
//! blocks among blocks that are new since the previous scan. Thus, the more frequently minor
//! scans run, the less work they have to do. This allows them to run frequently, giving low
//! latency for retrieval of newly produced blocks that we failed to receive initially. Between
//! each minor scan, the task will sleep for [a configurable
//! duration](Builder::with_minor_scan_interval) to wait for new blocks to be produced and give
//! other tasks full access to all shared resources.
//!
//! Every `n`th scan (`n` is [configurable](Builder::with_major_scan_interval)) is a major scan.
//! These scan all blocks from 0, which guarantees that we will eventually retrieve all blocks, even
//! if for some reason we have lost a block that we previously had (due to storage failures and
//! corruptions, or simple bugs in this software). These scans are rather expensive (although they
//! will release control of shared resources many times during the duration of the scan), but
//! because it is rather unlikely that a major scan will discover any missing blocks that the next
//! minor scan would have missed, it is ok if major scans run very infrequently.
//!
//! # Active Fetching
//!
//! Active fetching means reaching out to a remote data availability provider to retrieve a missing
//! resource, upon receiving a request for that resource from a client. Not every request for a
//! missing resource triggers an active fetch. To avoid spamming peers with requests for missing
//! data, we only actively fetch resources that are known to exist somewhere. This means we can
//! actively fetch leaves and headers when we are requested a leaf or header by height, whose height
//! is less than the current chain height. We can fetch a block when the corresponding header exists
//! (corresponding based on height, hash, or payload hash) or can be actively fetched.
//!
//! # Passive Fetching
//!
//! For requests that cannot be actively fetched (for example, a block requested by hash, where we
//! do not have a header proving that a block with that hash exists), we use passive fetching. This
//! essentially means waiting passively until the query service receives an object that satisfies
//! the request. This object may be received because it was actively fetched in responsive to a
//! different request for the same object, one that permitted an active fetch. Or it may have been
//! fetched [proactively](#proactive-fetching).

use super::{notifier::Notifier, storage::AvailabilityStorage, VersionedDataSource};
use crate::{
    availability::{
        AvailabilityDataSource, BlockId, BlockQueryData, Fetch, LeafId, LeafQueryData,
        PayloadQueryData, QueryablePayload, TransactionHash, TransactionIndex,
        UpdateAvailabilityData,
    },
    fetching::{
        self,
        request::{self, PayloadRequest},
        Callback, Provider,
    },
    metrics::PrometheusMetrics,
    node::{NodeDataSource, UpdateNodeData},
    status::StatusDataSource,
    task::BackgroundTask,
    Header, NotFoundSnafu, Payload, QueryResult, SignatureKey,
};
use anyhow::Context;
use async_std::{
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
    task::sleep,
};
use async_trait::async_trait;
use derivative::Derivative;
use derive_more::{Display, From};
use futures::{
    future::{join_all, BoxFuture, FutureExt},
    stream::{self, BoxStream, Stream, StreamExt},
};
use hotshot_types::traits::{block_contents::BlockHeader, node_implementation::NodeType};
use snafu::ensure;
use std::{
    cmp::{min, Ordering},
    fmt::{Debug, Display},
    future::IntoFuture,
    iter::once,
    marker::PhantomData,
    ops::{Bound, Deref, DerefMut, Range, RangeBounds},
    time::Duration,
};

/// Builder for [`FetchingDataSource`] with configuration.
pub struct Builder<Types, S, P> {
    storage: S,
    provider: P,
    retry_delay: Option<Duration>,
    range_chunk_size: usize,
    minor_scan_interval: Duration,
    major_scan_interval: usize,
    proactive_range_chunk_size: Option<usize>,
    proactive_fetching: bool,
    _types: PhantomData<Types>,
}

impl<Types, S, P> Builder<Types, S, P> {
    /// Construct a new builder with the given storage and fetcher and the default options.
    pub fn new(storage: S, provider: P) -> Self {
        Self {
            storage,
            provider,
            retry_delay: None,
            range_chunk_size: 25,
            // By default, we run minor proactive scans fairly frequently: once every minute. These
            // scans are cheap (moreso the more frequently they run) and can help us keep up with
            // the head of the chain even if our attached consensus instance is behind.
            minor_scan_interval: Duration::from_secs(60),
            // Major scans, on the other hand, are rather expensive and not especially important for
            // usability. We run them rarely, once every 60 minor scans, or once every hour by
            // default.
            major_scan_interval: 60,
            proactive_range_chunk_size: None,
            proactive_fetching: true,
            _types: Default::default(),
        }
    }

    /// Set the maximum delay between retries of fetches.
    pub fn with_retry_delay(mut self, retry_delay: Duration) -> Self {
        self.retry_delay = Some(retry_delay);
        self
    }

    /// Set the number of items to process at a time when loading a range or stream.
    ///
    /// This determines:
    /// * The number of objects to load from storage in a single request
    /// * The number of objects to buffer in memory per request/stream
    /// * The number of concurrent notification subscriptions per request/stream
    pub fn with_range_chunk_size(mut self, range_chunk_size: usize) -> Self {
        self.range_chunk_size = range_chunk_size;
        self
    }

    /// Set the time interval between minor proactive fetching scans.
    ///
    /// See [proactive fetching](self#proactive-fetching).
    pub fn with_minor_scan_interval(mut self, interval: Duration) -> Self {
        self.minor_scan_interval = interval;
        self
    }

    /// Set the interval (denominated in [minor scans](Self::with_minor_scan_interval)) between
    /// major proactive fetching scans.
    ///
    /// See [proactive fetching](self#proactive-fetching).
    pub fn with_major_scan_interval(mut self, interval: usize) -> Self {
        self.major_scan_interval = interval;
        self
    }

    /// Set the number of items to process at a time when scanning for proactive fetching.
    ///
    /// This is similar to [`Self::with_range_chunk_size`], but only affects the chunk size for
    /// proactive fetching scans, not for normal subscription streams. This can be useful to tune
    /// the proactive scanner to be more or less greedy with the lock on persistent storage.
    ///
    /// By default (i.e. if this method is not called) the proactive range chunk size will be set to
    /// whatever the normal range chunk size is.
    pub fn with_proactive_range_chunk_size(mut self, range_chunk_size: usize) -> Self {
        self.proactive_range_chunk_size = Some(range_chunk_size);
        self
    }

    /// Run without [proactive fetching](self#proactive-fetching).
    ///
    /// This can reduce load on the CPU and the database, but increases the probability that
    /// requests will fail due to missing resources. If resources are constrained, it is recommended
    /// to run with rare proactive fetching (see
    /// [`with_major_scan_interval`](Self::with_major_scan_interval),
    /// [`with_minor_scan_interval`](Self::with_minor_scan_interval)), rather than disabling it
    /// entirely.
    pub fn disable_proactive_fetching(mut self) -> Self {
        self.proactive_fetching = false;
        self
    }
}

impl<Types, S, P> Builder<Types, S, P>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload,
    S: NodeDataSource<Types>
        + UpdateNodeData<Types>
        + AvailabilityStorage<Types>
        + VersionedDataSource
        + 'static,
    P: AvailabilityProvider<Types>,
{
    /// Build a [`FetchingDataSource`] with these options.
    pub async fn build(self) -> anyhow::Result<FetchingDataSource<Types, S, P>> {
        FetchingDataSource::new(self).await
    }
}

/// The most basic kind of data source.
///
/// A data source is constructed modularly by combining a [storage](super::storage) implementation
/// with a [Fetcher](crate::fetching::Fetcher). The former allows the query service to store the
/// data it has persistently in an easily accessible storage medium, such as the local file system
/// or a database. This allows it to answer queries efficiently and to maintain its state across
/// restarts. The latter allows the query service to fetch data that is missing from its storage
/// from an external data availability provider, such as the Tiramisu DA network or another instance
/// of the query service.
///
/// These two components of a data source are combined in [`FetchingDataSource`], which is the
/// lowest level kind of data source available. It simply uses the storage implementation to fetch
/// data when available, and fills in everything else using the fetcher. Various kinds of data
/// sources can be constructed out of [`FetchingDataSource`] by changing the storage and fetcher
/// implementations used, and more complex data sources can be built on top using data source
/// combinators.
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Debug(bound = "S: Debug, P: Debug"))]
pub struct FetchingDataSource<Types, S, P>
where
    Types: NodeType,
{
    // The fetcher manages retrieval of resources from both local storage and a remote provider. It
    // encapsulates the data which may need to be shared with a long-lived task or future that
    // implements the asynchronous fetching of a particular object. This is why it gets its own
    // type, wrapped in an [`Arc`] for easy, efficient cloning.
    fetcher: Arc<Fetcher<Types, S, P>>,
    // The rest of the data we need for implementing data source traits but not for fetching.
    metrics: PrometheusMetrics,
    // The proactive scanner task. This is only saved here so that we can cancel it on drop.
    scanner: Option<BackgroundTask>,
}

impl<Types, S, P> FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload,
    S: NodeDataSource<Types>
        + UpdateNodeData<Types>
        + AvailabilityStorage<Types>
        + VersionedDataSource
        + 'static,
    P: AvailabilityProvider<Types>,
{
    /// Build a [`FetchingDataSource`] with the given `storage` and `provider`.
    pub fn builder(storage: S, provider: P) -> Builder<Types, S, P> {
        Builder::new(storage, provider)
    }

    async fn new(builder: Builder<Types, S, P>) -> anyhow::Result<Self> {
        let proactive_fetching = builder.proactive_fetching;
        let minor_interval = builder.minor_scan_interval;
        let major_interval = builder.major_scan_interval;
        let proactive_range_chunk_size = builder
            .proactive_range_chunk_size
            .unwrap_or(builder.range_chunk_size);

        let fetcher = Arc::new(Fetcher::new(builder).await?);
        let scanner = if proactive_fetching {
            Some(BackgroundTask::spawn(
                "proactive scanner",
                fetcher.clone().proactive_scan(
                    minor_interval,
                    major_interval,
                    proactive_range_chunk_size,
                ),
            ))
        } else {
            None
        };

        let mut ds = Self {
            fetcher,
            scanner,
            metrics: Default::default(),
        };

        if NodeDataSource::block_height(&ds).await? == 0 {
            // HotShot doesn't emit an event for the genesis block, so we need to manually ensure it
            // is present.
            ds.insert_genesis().await?;
        }

        Ok(ds)
    }

    async fn insert_genesis(&mut self) -> anyhow::Result<()> {
        UpdateAvailabilityData::insert_leaf(self, LeafQueryData::genesis()).await?;
        UpdateNodeData::insert_leaf(self, LeafQueryData::genesis()).await?;
        self.insert_block(BlockQueryData::genesis()).await?;
        self.commit().await?;
        Ok(())
    }
}

#[derive(From)]
pub struct StorageReadGuard<'a, Types, S>
where
    Types: NodeType,
{
    inner: RwLockReadGuard<'a, NotifyStorage<Types, S>>,
}

impl<'a, Types, S> Deref for StorageReadGuard<'a, Types, S>
where
    Types: NodeType,
{
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.inner.storage
    }
}

#[derive(From)]
pub struct StorageWriteGuard<'a, Types, S>
where
    Types: NodeType,
{
    inner: RwLockWriteGuard<'a, NotifyStorage<Types, S>>,
}

impl<'a, Types, S> Deref for StorageWriteGuard<'a, Types, S>
where
    Types: NodeType,
{
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.inner.storage
    }
}

impl<'a, Types, S> DerefMut for StorageWriteGuard<'a, Types, S>
where
    Types: NodeType,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner.storage
    }
}

impl<Types, S, P> FetchingDataSource<Types, S, P>
where
    Types: NodeType,
{
    /// Obtain direct, read-only access to the underlying local storage.
    pub async fn storage(&self) -> StorageReadGuard<Types, S> {
        self.fetcher.storage.read().await.into()
    }

    /// Obtain direct, mutable access the underlying local storage.
    pub async fn storage_mut(&self) -> StorageWriteGuard<Types, S> {
        self.fetcher.storage.write().await.into()
    }
}

#[async_trait]
impl<Types, S, P> StatusDataSource for FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    S: NodeDataSource<Types> + Send + Sync,
    P: Send + Sync,
{
    async fn block_height(&self) -> QueryResult<usize> {
        NodeDataSource::block_height(self).await
    }

    fn metrics(&self) -> &PrometheusMetrics {
        &self.metrics
    }
}

#[async_trait]
impl<Types, S, P> AvailabilityDataSource<Types> for FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload,
    S: AvailabilityStorage<Types> + 'static,
    P: AvailabilityProvider<Types>,
{
    type LeafRange<R> = BoxStream<'static, Fetch<LeafQueryData<Types>>>
    where
        R: RangeBounds<usize> + Send;
    type BlockRange<R> = BoxStream<'static, Fetch<BlockQueryData<Types>>>
    where
        R: RangeBounds<usize> + Send;
    type PayloadRange<R> = BoxStream<'static, Fetch<PayloadQueryData<Types>>>
    where
        R: RangeBounds<usize> + Send;

    async fn get_leaf<ID>(&self, id: ID) -> Fetch<LeafQueryData<Types>>
    where
        ID: Into<LeafId<Types>> + Send + Sync,
    {
        self.fetcher.get(id.into()).await
    }

    async fn get_block<ID>(&self, id: ID) -> Fetch<BlockQueryData<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync,
    {
        self.fetcher.get(id.into()).await
    }

    async fn get_payload<ID>(&self, id: ID) -> Fetch<PayloadQueryData<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync,
    {
        self.fetcher.get(id.into()).await
    }

    async fn get_leaf_range<R>(&self, range: R) -> Self::LeafRange<R>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.fetcher.clone().get_range(range)
    }

    async fn get_block_range<R>(&self, range: R) -> Self::BlockRange<R>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.fetcher.clone().get_range(range)
    }

    async fn get_payload_range<R>(&self, range: R) -> Self::PayloadRange<R>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.fetcher.clone().get_range(range)
    }

    async fn get_block_with_transaction(
        &self,
        hash: TransactionHash<Types>,
    ) -> Fetch<(BlockQueryData<Types>, TransactionIndex<Types>)> {
        self.fetcher.get(hash).await
    }
}

#[async_trait]
impl<Types, S, P> UpdateAvailabilityData<Types> for FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload,
    S: UpdateAvailabilityData<Types> + Send + Sync,
    P: Send + Sync,
{
    type Error = S::Error;

    async fn insert_leaf(&mut self, leaf: LeafQueryData<Types>) -> Result<(), Self::Error> {
        self.fetcher.storage.write().await.insert_leaf(leaf).await
    }

    async fn insert_block(&mut self, block: BlockQueryData<Types>) -> Result<(), Self::Error> {
        self.fetcher.storage.write().await.insert_block(block).await
    }
}

#[async_trait]
impl<Types, S, P> NodeDataSource<Types> for FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    S: NodeDataSource<Types> + Send + Sync,
    P: Send + Sync,
{
    async fn block_height(&self) -> QueryResult<usize> {
        Ok(self.fetcher.storage.read().await.height as usize)
    }

    async fn get_proposals(
        &self,
        proposer: &SignatureKey<Types>,
        limit: Option<usize>,
    ) -> QueryResult<Vec<LeafQueryData<Types>>> {
        self.storage().await.get_proposals(proposer, limit).await
    }

    async fn count_proposals(&self, proposer: &SignatureKey<Types>) -> QueryResult<usize> {
        self.storage().await.count_proposals(proposer).await
    }
}

#[async_trait]
impl<Types, S, P> UpdateNodeData<Types> for FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    S: UpdateNodeData<Types> + Send + Sync,
    P: Send + Sync,
{
    type Error = S::Error;

    async fn insert_leaf(&mut self, leaf: LeafQueryData<Types>) -> Result<(), Self::Error> {
        self.fetcher
            .storage
            .write()
            .await
            .storage
            .insert_leaf(leaf)
            .await
    }
}

#[async_trait]
impl<Types, S, P> VersionedDataSource for FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    S: VersionedDataSource + Send + Sync,
    P: Send + Sync,
{
    type Error = S::Error;

    async fn commit(&mut self) -> Result<(), Self::Error> {
        self.fetcher.storage.write().await.commit().await
    }

    async fn revert(&mut self) {
        self.fetcher.storage.write().await.revert().await
    }
}

/// Asynchronous retrieval and storage of [`Fetchable`] resources.
#[derive(Debug)]
struct Fetcher<Types, S, P>
where
    Types: NodeType,
{
    storage: RwLock<NotifyStorage<Types, S>>,
    provider: Arc<P>,
    payload_fetcher: Arc<fetching::Fetcher<request::PayloadRequest, PayloadCallback<Types, S, P>>>,
    leaf_fetcher: Arc<fetching::Fetcher<request::LeafRequest, LeafCallback<Types, S, P>>>,
    range_chunk_size: usize,
}

#[derive(Debug)]
struct NotifyStorage<Types, S>
where
    Types: NodeType,
{
    storage: S,
    height: u64,
    // The block height at the last commit, which we will roll back to on `revert`.
    committed_height: u64,
    block_notifier: Notifier<BlockQueryData<Types>>,
    leaf_notifier: Notifier<LeafQueryData<Types>>,
}

impl<Types, S> NotifyStorage<Types, S>
where
    Types: NodeType,
    S: UpdateAvailabilityData<Types>,
{
    async fn insert_leaf(&mut self, leaf: LeafQueryData<Types>) -> Result<(), S::Error> {
        self.leaf_notifier.notify(&leaf);
        if leaf.height() + 1 > self.height {
            self.height = leaf.height() + 1;
        }
        self.storage.insert_leaf(leaf).await
    }

    async fn insert_block(&mut self, block: BlockQueryData<Types>) -> Result<(), S::Error> {
        self.block_notifier.notify(&block);
        if block.height() + 1 > self.height {
            self.height = block.height() + 1;
        }
        self.storage.insert_block(block).await
    }
}

impl<Types, S> NotifyStorage<Types, S>
where
    Types: NodeType,
    S: VersionedDataSource,
{
    async fn commit(&mut self) -> Result<(), S::Error> {
        self.storage.commit().await?;
        self.committed_height = self.height;
        Ok(())
    }

    async fn revert(&mut self) {
        self.storage.revert().await;
        self.height = self.committed_height;
    }
}

impl<Types, S, P> Fetcher<Types, S, P>
where
    Types: NodeType,
    S: NodeDataSource<Types>,
{
    async fn new(builder: Builder<Types, S, P>) -> anyhow::Result<Self> {
        let mut payload_fetcher = fetching::Fetcher::default();
        let mut leaf_fetcher = fetching::Fetcher::default();
        if let Some(delay) = builder.retry_delay {
            payload_fetcher = payload_fetcher.with_retry_delay(delay);
            leaf_fetcher = leaf_fetcher.with_retry_delay(delay);
        }

        let height = builder.storage.block_height().await? as u64;
        Ok(Self {
            storage: RwLock::new(NotifyStorage {
                height,
                committed_height: height,
                storage: builder.storage,
                block_notifier: Notifier::new(),
                leaf_notifier: Notifier::new(),
            }),
            provider: Arc::new(builder.provider),
            payload_fetcher: Arc::new(payload_fetcher),
            leaf_fetcher: Arc::new(leaf_fetcher),
            range_chunk_size: builder.range_chunk_size,
        })
    }
}

impl<Types, S, P> Fetcher<Types, S, P>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload,
    S: AvailabilityStorage<Types> + 'static,
    P: AvailabilityProvider<Types>,
{
    async fn get<T, R>(self: &Arc<Self>, req: R) -> Fetch<T>
    where
        T: Fetchable<Types>,
        R: Into<T::Request> + Send,
    {
        let req = req.into();
        // Hold a read lock on `storage` while we run `ok_or_fetch`. This means that no
        // notifications are sent in between checking local storage and triggering a fetch if
        // necessary, since sending notifications requires a write lock. Hence, we will not miss a
        // notification.
        let storage = self.storage.read().await;
        self.ok_or_fetch(&storage, req, T::load(&storage, req).await)
            .await
    }

    /// Get a range of objects from local storage or a provider.
    ///
    /// Convert a finite stream of fallible local storage lookups into a (possibly infinite) stream
    /// of infallible fetches. Objects in `range` are loaded from local storage. Any gaps or missing
    /// objects are filled by fetching from a provider. Items in the resulting stream are futures
    /// that will never fail to produce a resource, although they may block indefinitely if the
    /// resource needs to be fetched.
    ///
    /// Objects are loaded and fetched in chunks, which strikes a good balance of limiting the total
    /// number of storage and network requests, while also keeping the amount of simultaneous
    /// resource consumption bounded.
    fn get_range<R, T>(self: Arc<Self>, range: R) -> BoxStream<'static, Fetch<T>>
    where
        R: RangeBounds<usize> + Send + 'static,
        T: RangedFetchable<Types>,
    {
        let chunk_size = self.range_chunk_size;
        self.get_range_with_chunk_size(chunk_size, range)
    }

    /// Same as [`Self::get_range`], but uses the given chunk size instead of the default.
    fn get_range_with_chunk_size<R, T>(
        self: Arc<Self>,
        chunk_size: usize,
        range: R,
    ) -> BoxStream<'static, Fetch<T>>
    where
        R: RangeBounds<usize> + Send + 'static,
        T: RangedFetchable<Types>,
    {
        stream::iter(range_chunks(range, chunk_size))
            .then(move |chunk| self.clone().get_chunk(chunk))
            .flatten()
            .boxed()
    }

    /// Get a range of objects from local storage or a provider.
    ///
    /// This method is similar to `get_range`, except that:
    /// * It fetches all desired objects together, as a single chunk
    /// * It loads the object or triggers fetches right now rather than providing a lazy stream
    ///   which only fetches objects when polled.
    async fn get_chunk<T>(self: Arc<Self>, chunk: Range<usize>) -> impl Stream<Item = Fetch<T>>
    where
        T: RangedFetchable<Types>,
    {
        let storage = self.storage.read().await;
        let ts = T::load_range(&storage, chunk.clone())
            .await
            .context(format!("when fetching items in range {chunk:?}"))
            .ok_or_trace()
            .unwrap_or_default();
        // Log and discard error information; we want a list of Option where None indicates an
        // object that needs to be fetched.
        let ts = ts.into_iter().map(ResultExt::ok_or_trace);
        // Storage may return fewer objects than asked for if we hit the end of the current chain.
        // Pad out to the end of the chunk with None, indicating that objects we don't have yet must
        // be fetched.
        if ts.len() < chunk.len() {
            tracing::info!(
                "items {}-{} in chunk are not available, will be fetched",
                ts.len(),
                chunk.len()
            );
        }
        let padding = std::iter::repeat(None).take(chunk.len() - ts.len());
        let ts = ts.chain(padding);
        // Kick off a fetch for each missing object.
        let fetcher = &self;
        let ts = ts
            .enumerate()
            .map(|(i, opt)| fetcher.some_or_fetch(&storage, chunk.start + i, opt));

        // We `join_all` here because we want this iterator to be evaluated eagerly for two reasons:
        // 1. It borrows from `self`, which is local to this future. This avoids having to clone
        //    `self` for every entry, instead we clone it for every chunk.
        // 2. We evaluate all the `some_or_fetch` calls eagerly, so the fetches are triggered as
        //    soon as we evaluate the chunk. This ensures we don't miss any notifications, since we
        //    load from storage and subscribe to notifications for missing objects all while we have
        //    a read lock on `self.storage`. No notifications can be sent during this time since
        //    sending a notification requires a write lock.
        stream::iter(join_all(ts).await)
    }

    async fn ok_or_fetch<R, T>(
        self: &Arc<Self>,
        storage: &RwLockReadGuard<'_, NotifyStorage<Types, S>>,
        req: R,
        res: QueryResult<T>,
    ) -> Fetch<T>
    where
        R: Into<T::Request> + Send,
        T: Fetchable<Types> + Send + Sync + 'static,
    {
        let req = req.into();
        self.some_or_fetch(
            storage,
            req,
            res.context(format!("req: {req:?}")).ok_or_trace(),
        )
        .await
    }

    async fn some_or_fetch<R, T>(
        self: &Arc<Self>,
        storage: &RwLockReadGuard<'_, NotifyStorage<Types, S>>,
        req: R,
        res: Option<T>,
    ) -> Fetch<T>
    where
        R: Into<T::Request> + Send,
        T: Fetchable<Types> + Send + Sync + 'static,
    {
        match res {
            Some(t) => Fetch::Ready(t),
            None => self.fetch(storage, req).await,
        }
    }

    async fn fetch<R, T>(
        self: &Arc<Self>,
        storage: &RwLockReadGuard<'_, NotifyStorage<Types, S>>,
        req: R,
    ) -> Fetch<T>
    where
        R: Into<T::Request>,
        T: Fetchable<Types>,
    {
        let req = req.into();
        tracing::debug!("fetching resource {req:?}");

        // Subscribe to notifications so we are alerted when we get the resource.
        let fut = T::passive_fetch(&**storage, req)
            .await
            .then(move |opt| async move {
                match opt {
                    Some(t) => t,
                    None => {
                        // If `passive_fetch` returns `None`, it means the notifier was dropped
                        // without ever sending a notification. In this case, the correct behavior
                        // is actually to block forever (unless the `Fetch` itself is dropped),
                        // since the semantics of `Fetch` are to never fail. This is analogous to
                        // fetching an object which doesn't actually exist: the `Fetch` will never
                        // return.
                        //
                        // However, for ease of debugging, and since this is never expected to
                        // happen in normal usage, we panic instead. This should only happen in two
                        // cases:
                        // * The server was shut down (dropping the notifier) without cleaning up
                        //   some background tasks. This will not affect runtime behavior, but
                        //   should be fixed if it happens.
                        // * There is a very unexpected runtime bug resulting in the notifier being
                        //   dropped. If this happens, things are very broken in any case, and it is
                        //   better to panic loudly than simply block forever.
                        panic!("notifier dropped without satisfying request {req:?}");
                    }
                }
            });

        // Trigger an active fetch from a remote provider if possible.
        //
        // We pass in the lock we have on `storage`, so that `active_fetch` is allowed to read from
        // storage without needing to recursively acquire a read lock. This is important because
        // `active_fetch` sometimes needs to read from storage in order to decide whether it is
        // appropriate to spawn an active fetching task.
        //
        // Ordinarily, taking a read-only lock recursively from the same task would be fine,
        // but because async_std::RwLock is "write-preferring" (attempts to acquire a read lock
        // block if there are concurrent attempts to acquire a write lock) we can have the following
        // deadlock:
        // * This task acquires `storage` in the caller of this function
        // * Another task completes a fetch and attempts to acquire a write lock on storage to save
        //   the fetched resource. This blocks since we have a read lock.
        // * This task (in `active_fetch`) attempts to acquire a read lock on storage. This blocks
        //   due to the concurrent request for a write lock. However as long as we are blocked, we
        //   can never release the _original_ read lock we had on `storage`, and so we can never
        //   unblock the task waiting for a write lock, which in turn means we can never become
        //   unblocked ourselves.
        T::active_fetch(self.clone(), storage, req).await;

        // Wait for the object to arrive.
        Fetch::Pending(fut.boxed())
    }

    /// Proactively search for and retrieve missing objects.
    ///
    /// This function will proactively identify and retrieve blocks and leaves which are missing
    /// from storage. It will run until cancelled, thus, it is meant to be spawned as a background
    /// task rather than called synchronously.
    async fn proactive_scan(
        self: Arc<Self>,
        minor_interval: Duration,
        major_interval: usize,
        chunk_size: usize,
    ) {
        let mut prev_height = 0;

        for i in 0.. {
            // Get the block height; we will look for any missing blocks up to `block_height`.
            let block_height = { self.storage.read().await.height } as usize;

            // In a major scan, we fetch all blocks between 0 and `block_height`. In minor scans
            // (much more frequent) we fetch blocks that are missing since the last scan.
            let start = if i % major_interval == 0 {
                0
            } else {
                prev_height
            };
            tracing::info!("starting proactive scan {i} of blocks from {start}-{block_height}");
            prev_height = block_height;

            // Iterate over all blocks that we should have. Merely iterating over the `Fetch`es
            // without awaiting them is enough to trigger active fetches of missing blocks, since
            // we always trigger an active fetch when fetching by block number. Moreover, fetching
            // the block is enough to trigger an active fetch of the corresponding leaf if it too is
            // missing, so this one loop takes care of all resources that might be missing.
            //
            // The chunking behavior of `get_range` automatically ensures that, no matter how big
            // the range is, we will release the read lock on storage every `chunk_size` items, so
            // we don't starve out would-be writers.
            let mut blocks = self
                .clone()
                .get_range_with_chunk_size::<_, BlockQueryData<Types>>(
                    chunk_size,
                    start..block_height,
                );
            while blocks.next().await.is_some() {}

            tracing::info!("completed proactive scan {i} of blocks from {start}-{block_height}, will scan again in {minor_interval:?}");
            sleep(minor_interval).await;
        }
    }
}

/// A provider which can be used as a fetcher by the availability service.
pub trait AvailabilityProvider<Types: NodeType>:
    Provider<Types, request::LeafRequest> + Provider<Types, request::PayloadRequest> + Sync + 'static
{
}
impl<Types: NodeType, P> AvailabilityProvider<Types> for P where
    P: Provider<Types, request::LeafRequest>
        + Provider<Types, request::PayloadRequest>
        + Sync
        + 'static
{
}

/// Objects which can be fetched from a remote DA provider and cached in local storage.
///
/// This trait lets us abstract over leaves, blocks, and other types that can be fetched. Thus, the
/// logistics of fetching are shared between all objects, and only the low-level particulars are
/// type-specific.
#[async_trait]
trait Fetchable<Types>: Clone + Send + Sync + 'static
where
    Types: NodeType,
    Payload<Types>: QueryablePayload,
{
    /// A succinct specification of the object to be fetched.
    type Request: Copy + Debug + Send + Sync + 'static;

    /// Does this object satisfy the given request?
    fn satisfies(&self, req: Self::Request) -> bool;

    /// Spawn a task to fetch the object from a remote provider, if possible.
    ///
    /// An active fetch will only be triggered if:
    /// * There is not already an active fetch in progress for the same object
    /// * The requested object is known to exist. For example, we will trigger a fetch of a block
    ///   with a height less than the current block height, but not greater, since the latter might
    ///   not exist yet, and we should receive it passively once it is produced. Or, we will fetch a
    ///   leaf by height but not by hash, since we can't guarantee that a leaf with an arbitrary
    ///   hash exists.
    ///
    /// If we do trigger an active fetch for an object, the provided callback will be called if and
    /// when the fetch completes successfully. The callback should be responsible for notifying any
    /// passive listeners that the object has been retrieved. If we do not trigger an active fetch
    /// for an object, this function does nothing.
    ///
    /// In either case, as long as the requested object does in fact exist, we will eventually
    /// receive it passively, since we will eventually receive all blocks and leaves that are ever
    /// produced. Active fetching merely helps us receive certain objects sooner.
    async fn active_fetch<S, P>(
        fetcher: Arc<Fetcher<Types, S, P>>,
        storage: &RwLockReadGuard<'_, NotifyStorage<Types, S>>,
        req: Self::Request,
    ) where
        S: AvailabilityStorage<Types> + 'static,
        P: AvailabilityProvider<Types>;

    /// Wait for someone else to fetch the object.
    async fn passive_fetch<S>(
        storage: &NotifyStorage<Types, S>,
        req: Self::Request,
    ) -> BoxFuture<'static, Option<Self>>
    where
        S: AvailabilityStorage<Types>;

    /// Load an object from local storage.
    async fn load<S>(storage: &NotifyStorage<Types, S>, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>;
}

#[async_trait]
trait RangedFetchable<Types>: Fetchable<Types, Request = Self::RangedRequest>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload,
{
    type RangedRequest: From<usize> + Send;

    /// Load a range of these objects from local storage.
    async fn load_range<S, R>(
        storage: &NotifyStorage<Types, S>,
        range: R,
    ) -> QueryResult<Vec<QueryResult<Self>>>
    where
        S: AvailabilityStorage<Types>,
        R: RangeBounds<usize> + Send + 'static;
}

#[async_trait]
impl<Types> Fetchable<Types> for LeafQueryData<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload,
{
    type Request = LeafId<Types>;

    fn satisfies(&self, req: Self::Request) -> bool {
        match req {
            LeafId::Number(n) => self.height() == n as u64,
            LeafId::Hash(h) => self.hash() == h,
        }
    }

    async fn passive_fetch<S>(
        storage: &NotifyStorage<Types, S>,
        req: Self::Request,
    ) -> BoxFuture<'static, Option<Self>>
    where
        S: AvailabilityStorage<Types>,
    {
        storage
            .leaf_notifier
            .wait_for(move |leaf| leaf.satisfies(req))
            .await
            .into_future()
            .boxed()
    }

    async fn active_fetch<S, P>(
        fetcher: Arc<Fetcher<Types, S, P>>,
        storage: &RwLockReadGuard<'_, NotifyStorage<Types, S>>,
        req: Self::Request,
    ) where
        S: AvailabilityStorage<Types> + 'static,
        P: AvailabilityProvider<Types>,
    {
        fetch_leaf_with_callbacks(fetcher, storage, req, None)
    }

    async fn load<S>(storage: &NotifyStorage<Types, S>, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        // Fail quickly, without touching storage, if the requested height is greater than the
        // current height. In this case, we know we don't have the leaf.
        if let LeafId::Number(n) = req {
            ensure!((n as u64) < storage.height, NotFoundSnafu);
        }
        storage.storage.get_leaf(req).await
    }
}

fn fetch_leaf_with_callbacks<Types, S, P, I>(
    fetcher: Arc<Fetcher<Types, S, P>>,
    storage: &RwLockReadGuard<'_, NotifyStorage<Types, S>>,
    req: LeafId<Types>,
    callbacks: I,
) where
    Types: NodeType,
    Payload<Types>: QueryablePayload,
    S: AvailabilityStorage<Types> + 'static,
    P: AvailabilityProvider<Types>,
    I: IntoIterator<Item = LeafCallback<Types, S, P>> + Send + 'static,
    I::IntoIter: Send,
{
    match req {
        LeafId::Number(n) => {
            let height = storage.height as usize;
            if n >= height {
                // If the requested leaf has yet to be produced, based on the current block height,
                // there is no point in requesting it. We will receive it passively once it is
                // created.
                tracing::info!("not fetching leaf {n} because height is only {height}");
                return;
            }

            let fetcher = fetcher.clone();
            fetcher.leaf_fetcher.clone().spawn_fetch(
                n.into(),
                fetcher.provider.clone(),
                once(LeafCallback::Leaf { fetcher }).chain(callbacks),
            );
        }
        LeafId::Hash(h) => {
            // We don't actively fetch leaves when requested by hash, because we have no way of
            // knowing whether a leaf with such a hash actually exists, and we don't want to bother
            // peers with requests for non-existant leaves.
            tracing::info!("not fetching unknown leaf {h}");
        }
    }
}

async fn store_leaf<Types, S>(
    storage: &mut NotifyStorage<Types, S>,
    leaf: LeafQueryData<Types>,
) -> anyhow::Result<()>
where
    Types: NodeType,
    S: UpdateAvailabilityData<Types> + VersionedDataSource,
{
    tracing::info!("storing leaf");
    storage.insert_leaf(leaf).await?;
    storage.commit().await?;
    Ok(())
}

#[async_trait]
impl<Types> RangedFetchable<Types> for LeafQueryData<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload,
{
    type RangedRequest = LeafId<Types>;

    async fn load_range<S, R>(
        storage: &NotifyStorage<Types, S>,
        range: R,
    ) -> QueryResult<Vec<QueryResult<Self>>>
    where
        S: AvailabilityStorage<Types>,
        R: RangeBounds<usize> + Send + 'static,
    {
        // Return quickly, without touching storage, if the start of the requested range is greater
        // than the current height. In this case, we know storage would return an empty range.
        match range.start_bound() {
            Bound::Included(n) if (*n as u64) >= storage.height => return Ok(vec![]),
            Bound::Excluded(n) if (*n as u64) + 1 >= storage.height => return Ok(vec![]),
            _ => {}
        }
        storage.storage.get_leaf_range(range).await
    }
}

/// A request to fetch a block.
///
/// Blocks can be requested either directly by their [`BlockId`], or indirectly, by requesting a
/// block containing a particular transaction.
#[derive(Derivative, From, Display)]
#[derivative(Ord = "feature_allow_slow_enum")]
#[derivative(
    Copy(bound = ""),
    Debug(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = ""),
    Ord(bound = ""),
    Hash(bound = "")
)]
pub enum BlockRequest<Types>
where
    Types: NodeType,
{
    Id(BlockId<Types>),
    WithTransaction(TransactionHash<Types>),
}

impl<Types> From<usize> for BlockRequest<Types>
where
    Types: NodeType,
{
    fn from(i: usize) -> Self {
        Self::Id(i.into())
    }
}

impl<Types> Clone for BlockRequest<Types>
where
    Types: NodeType,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<Types> PartialOrd for BlockRequest<Types>
where
    Types: NodeType,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[async_trait]
impl<Types> Fetchable<Types> for BlockQueryData<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload,
{
    type Request = BlockRequest<Types>;

    fn satisfies(&self, req: Self::Request) -> bool {
        match req {
            BlockRequest::Id(BlockId::Number(n)) => self.height() == n as u64,
            BlockRequest::Id(BlockId::Hash(h)) => self.hash() == h,
            BlockRequest::Id(BlockId::PayloadHash(h)) => self.payload_hash() == h,
            BlockRequest::WithTransaction(h) => self.transaction_by_hash(h).is_some(),
        }
    }

    async fn passive_fetch<S>(
        storage: &NotifyStorage<Types, S>,
        req: Self::Request,
    ) -> BoxFuture<'static, Option<Self>>
    where
        S: AvailabilityStorage<Types>,
    {
        storage
            .block_notifier
            .wait_for(move |block| block.satisfies(req))
            .await
            .into_future()
            .boxed()
    }

    async fn active_fetch<S, P>(
        fetcher: Arc<Fetcher<Types, S, P>>,
        storage: &RwLockReadGuard<'_, NotifyStorage<Types, S>>,
        req: Self::Request,
    ) where
        S: AvailabilityStorage<Types> + 'static,
        P: AvailabilityProvider<Types>,
    {
        match req {
            BlockRequest::Id(id) => {
                // First, check if at least the header is available in local storage. If it is, we
                // benefit two ways:
                // 1. We know for sure the corresponding block exists, so we can unconditionally
                //    trigger an active fetch without unnecessarily bothering our peers.
                // 2. We only need to fetch the payload, not the full block. Not only is this
                //    marginally less data to download, there are some providers that may only be
                //    able to provide payloads, not full blocks, such as HotShot DA committee
                //    members.
                if let Some(header) = load_header(&**storage, id)
                    .await
                    .context(format!("loading header for block {id}"))
                    .ok_or_trace()
                {
                    fetch_block_with_header(fetcher, header);
                    return;
                }

                // If the header is _not_ present, we may still be able to fetch the block, but we
                // need to fetch the header (in fact, the entire leaf) first. This is because we
                // have an invariant that we should not store a block payload in the database unless
                // we already have its corresponding header and leaf.
                match id {
                    BlockId::Number(n) => {
                        fetch_leaf_with_callbacks(
                            fetcher.clone(),
                            storage,
                            n.into(),
                            [LeafCallback::Block { fetcher }],
                        );
                    }
                    BlockId::Hash(h) => {
                        // Given only the hash, we cannot tell if the corresonding leaf actually
                        // exists, since we don't have a corresponding header. Therefore, we will
                        // not spawn an active fetch.
                        tracing::info!("not fetching unknown block {h}");
                        return;
                    }
                    BlockId::PayloadHash(h) => {
                        // Same as above, we don't fetch a block with a payload that is not known to
                        // exist.
                        tracing::info!("not fetching block with unknown payload {h}");
                        return;
                    }
                }
            }
            BlockRequest::WithTransaction(h) => {
                // We don't actively fetch blocks when requested by transaction, because without the
                // block payload, we have no way of knowing whether a block with such a transaction
                // actually exists, and we don't want to bother peers with requests for non-existant
                // blocks.
                tracing::info!("not fetching block with unknown transaction {h}");
            }
        }
    }

    async fn load<S>(storage: &NotifyStorage<Types, S>, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        match req {
            BlockRequest::Id(id) => {
                // Fail quickly, without touching storage, if the requested height is greater than
                // the current height. In this case, we know we don't have the block.
                if let BlockId::Number(n) = id {
                    ensure!((n as u64) < storage.height, NotFoundSnafu);
                }
                storage.storage.get_block(id).await
            }
            BlockRequest::WithTransaction(h) => {
                Ok(storage.storage.get_block_with_transaction(h).await?.0)
            }
        }
    }
}

#[async_trait]
impl<Types> RangedFetchable<Types> for BlockQueryData<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload,
{
    type RangedRequest = BlockRequest<Types>;

    async fn load_range<S, R>(
        storage: &NotifyStorage<Types, S>,
        range: R,
    ) -> QueryResult<Vec<QueryResult<Self>>>
    where
        S: AvailabilityStorage<Types>,
        R: RangeBounds<usize> + Send + 'static,
    {
        // Return quickly, without touching storage, if the start of the requested range is greater
        // than the current height. In this case, we know storage would return an empty range.
        match range.start_bound() {
            Bound::Included(n) if (*n as u64) >= storage.height => return Ok(vec![]),
            Bound::Excluded(n) if (*n as u64 + 1) >= storage.height => return Ok(vec![]),
            _ => {}
        }
        storage.storage.get_block_range(range).await
    }
}

fn fetch_block_with_header<Types, S, P>(fetcher: Arc<Fetcher<Types, S, P>>, header: Header<Types>)
where
    Types: NodeType,
    Payload<Types>: QueryablePayload,
    S: AvailabilityStorage<Types> + 'static,
    P: AvailabilityProvider<Types>,
{
    // Now that we have the header, we only need to retrieve the payload.
    tracing::info!(
        "spawned active fetch for payload {:?} (height {})",
        header.payload_commitment(),
        header.block_number()
    );
    fetcher.payload_fetcher.spawn_fetch(
        PayloadRequest(header.payload_commitment()),
        fetcher.provider.clone(),
        once(PayloadCallback {
            header,
            fetcher: fetcher.clone(),
        }),
    );
}

async fn store_block<Types, S>(
    storage: &mut NotifyStorage<Types, S>,
    block: BlockQueryData<Types>,
) -> anyhow::Result<()>
where
    Types: NodeType,
    S: UpdateAvailabilityData<Types> + VersionedDataSource,
{
    storage.insert_block(block).await?;
    storage.commit().await?;
    Ok(())
}

async fn load_header<Types, S>(
    storage: &NotifyStorage<Types, S>,
    id: BlockId<Types>,
) -> QueryResult<Header<Types>>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload,
    S: AvailabilityStorage<Types>,
{
    // Fail quickly, without touching storage, if the requested height is greater than the current
    // height. In this case, we know we don't have the header.
    if let BlockId::Number(n) = id {
        ensure!((n as u64) < storage.height, NotFoundSnafu);
    }
    storage.storage.get_header(id).await
}

#[async_trait]
impl<Types> Fetchable<Types> for (BlockQueryData<Types>, TransactionIndex<Types>)
where
    Types: NodeType,
    Payload<Types>: QueryablePayload,
{
    type Request = TransactionHash<Types>;

    fn satisfies(&self, req: Self::Request) -> bool {
        self.0.transaction_by_hash(req).is_some()
    }

    async fn passive_fetch<S>(
        storage: &NotifyStorage<Types, S>,
        req: Self::Request,
    ) -> BoxFuture<'static, Option<Self>>
    where
        S: AvailabilityStorage<Types>,
    {
        let wait_block = storage
            .block_notifier
            .wait_for(move |block| block.satisfies(req.into()))
            .await;

        async move {
            let block = wait_block.await?;

            // This `unwrap` is safe, `wait_for` only returns blocks which satisfy the request, and
            // in this case that means the block must contain the requested transaction.
            let ix = block.transaction_by_hash(req).unwrap();

            Some((block, ix))
        }
        .boxed()
    }

    async fn active_fetch<S, P>(
        _fetcher: Arc<Fetcher<Types, S, P>>,
        _storage: &RwLockReadGuard<'_, NotifyStorage<Types, S>>,
        req: Self::Request,
    ) where
        S: AvailabilityStorage<Types> + 'static,
        P: AvailabilityProvider<Types>,
    {
        // We don't actively fetch blocks when requested by transaction, because without the block
        // payload, we have no way of knowing whether a block with such a transaction actually
        // exists, and we don't want to bother peers with requests for non-existant blocks.
        tracing::info!("not fetching block with unknown transaction {req}");
    }

    async fn load<S>(storage: &NotifyStorage<Types, S>, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        storage.storage.get_block_with_transaction(req).await
    }
}

#[async_trait]
impl<Types> Fetchable<Types> for PayloadQueryData<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload,
{
    type Request = BlockId<Types>;

    fn satisfies(&self, req: Self::Request) -> bool {
        match req {
            BlockId::Number(n) => self.height() == n as u64,
            BlockId::Hash(h) => self.block_hash() == h,
            BlockId::PayloadHash(h) => self.hash() == h,
        }
    }

    async fn passive_fetch<S>(
        storage: &NotifyStorage<Types, S>,
        req: Self::Request,
    ) -> BoxFuture<'static, Option<Self>>
    where
        S: AvailabilityStorage<Types>,
    {
        storage
            .block_notifier
            .wait_for(move |block| block.satisfies(req.into()))
            .await
            .into_future()
            .map(|block| block.map(PayloadQueryData::from))
            .boxed()
    }

    async fn active_fetch<S, P>(
        fetcher: Arc<Fetcher<Types, S, P>>,
        storage: &RwLockReadGuard<'_, NotifyStorage<Types, S>>,
        req: Self::Request,
    ) where
        S: AvailabilityStorage<Types> + 'static,
        P: AvailabilityProvider<Types>,
    {
        // We don't have storage for the payload alone, only the whole block. So if we need to fetch
        // the payload, we just fetch the whole block (which may end up fetching only the payload,
        // if that's all that's needed to complete the block).
        BlockQueryData::active_fetch(fetcher, storage, req.into()).await
    }

    async fn load<S>(storage: &NotifyStorage<Types, S>, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        // Fail quickly, without touching storage, if the requested height is greater than the
        // current height. In this case, we know we don't have the payload.
        if let BlockId::Number(n) = req {
            ensure!((n as u64) < storage.height, NotFoundSnafu);
        }
        storage.storage.get_payload(req).await
    }
}

#[async_trait]
impl<Types> RangedFetchable<Types> for PayloadQueryData<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload,
{
    type RangedRequest = BlockId<Types>;

    async fn load_range<S, R>(
        storage: &NotifyStorage<Types, S>,
        range: R,
    ) -> QueryResult<Vec<QueryResult<Self>>>
    where
        S: AvailabilityStorage<Types>,
        R: RangeBounds<usize> + Send + 'static,
    {
        // Return quickly, without touching storage, if the start of the requested range is greater
        // than the current height. In this case, we know storage would return an empty range.
        match range.start_bound() {
            Bound::Included(n) if (*n as u64) >= storage.height => return Ok(vec![]),
            Bound::Excluded(n) if (*n as u64 + 1) >= storage.height => return Ok(vec![]),
            _ => {}
        }
        storage.storage.get_payload_range(range).await
    }
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
struct PayloadCallback<Types: NodeType, S, P> {
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
    Payload<Types>: QueryablePayload,
    S: AvailabilityStorage<Types>,
    P: AvailabilityProvider<Types>,
{
    async fn run(self, payload: Payload<Types>) {
        tracing::info!("fetched payload {:?}", self.header.payload_commitment());
        let block = BlockQueryData::new(self.header, payload);

        // Store the block in local storage, so we can avoid fetching it in the future.
        {
            let mut storage = self.fetcher.storage.write().await;
            if let Err(err) = store_block(&mut *storage, block.clone()).await {
                // It is unfortunate if this fails, but we can still proceed by returning
                // the block that we fetched, keeping it in memory. Simply log the error and
                // move on.
                tracing::warn!("failed to store fetched block {}: {err}", block.height());
            }
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
enum LeafCallback<Types: NodeType, S, P> {
    /// Callback when fetching the leaf for its own sake.
    Leaf {
        #[derivative(Debug = "ignore")]
        fetcher: Arc<Fetcher<Types, S, P>>,
    },
    /// Callback when fetching the leaf in order to then look up a block.
    Block {
        #[derivative(Debug = "ignore")]
        fetcher: Arc<Fetcher<Types, S, P>>,
    },
}

impl<Types: NodeType, S, P> PartialEq for LeafCallback<Types, S, P> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl<Types: NodeType, S, P> Eq for LeafCallback<Types, S, P> {}

impl<Types: NodeType, S, P> Ord for LeafCallback<Types, S, P> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            // Store leaves in the database before storing blocks.
            (Self::Leaf { .. }, Self::Block { .. }) => Ordering::Less,
            (Self::Block { .. }, Self::Leaf { .. }) => Ordering::Greater,
            _ => Ordering::Equal,
        }
    }
}

impl<Types: NodeType, S, P> PartialOrd for LeafCallback<Types, S, P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Types: NodeType, S, P> Callback<LeafQueryData<Types>> for LeafCallback<Types, S, P>
where
    Payload<Types>: QueryablePayload,
    S: AvailabilityStorage<Types> + 'static,
    P: AvailabilityProvider<Types>,
{
    async fn run(self, leaf: LeafQueryData<Types>) {
        match self {
            Self::Leaf { fetcher } => {
                let height = leaf.height();
                tracing::info!("fetched leaf {height}");
                let mut storage = fetcher.storage.write().await;
                if let Err(err) = store_leaf(&mut *storage, leaf).await {
                    // It is unfortunate if this fails, but we can still proceed by
                    // returning the leaf that we fetched, keeping it in memory.
                    // Simply log the error and move on.
                    tracing::warn!("failed to store fetched leaf {height}: {err}");
                }
            }
            Self::Block { fetcher } => {
                tracing::info!("fetched leaf {}, will now fetch payload", leaf.height());
                fetch_block_with_header(fetcher, leaf.leaf.block_header);
            }
        }
    }
}

/// Break a range into fixed-size chunks.
fn range_chunks<R>(range: R, chunk_size: usize) -> impl Iterator<Item = Range<usize>>
where
    R: RangeBounds<usize>,
{
    // Transform range to explicit start (inclusive) and end (exclusive) bounds.
    let mut start = match range.start_bound() {
        Bound::Included(i) => *i,
        Bound::Excluded(i) => *i + 1,
        Bound::Unbounded => 0,
    };
    let end = match range.end_bound() {
        Bound::Included(i) => *i + 1,
        Bound::Excluded(i) => *i,
        Bound::Unbounded => usize::MAX,
    };
    std::iter::from_fn(move || {
        let chunk_end = min(start + chunk_size, end);
        if chunk_end == start {
            return None;
        }

        let chunk = start..chunk_end;
        start = chunk_end;
        Some(chunk)
    })
}

trait ResultExt<T, E> {
    fn ok_or_trace(self) -> Option<T>
    where
        E: Display;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn ok_or_trace(self) -> Option<T>
    where
        E: Display,
    {
        match self {
            Ok(t) => Some(t),
            Err(err) => {
                tracing::warn!(
                    "error loading resource from local storage, will try to fetch: {err:#}"
                );
                None
            }
        }
    }
}
