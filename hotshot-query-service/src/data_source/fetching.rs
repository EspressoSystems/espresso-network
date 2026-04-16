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
//! Proactive fetching is implemented by a background task which performs periodic scans of the
//! database, identifying and retrieving missing objects. This task is generally low priority, since
//! missing objects are rare, and it will take care not to monopolize resources that could be used
//! to serve requests.
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

use std::{
    cmp::{max, min},
    fmt::{Debug, Display},
    iter::repeat_with,
    marker::PhantomData,
    ops::{Bound, Range, RangeBounds},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, bail};
use async_lock::Semaphore;
use async_trait::async_trait;
use backoff::{ExponentialBackoff, ExponentialBackoffBuilder, backoff::Backoff};
use chrono::{DateTime, Utc};
use derivative::Derivative;
use futures::{
    channel::oneshot,
    future::{self, BoxFuture, Either, Future, FutureExt, join_all},
    stream::{self, BoxStream, StreamExt},
};
use hotshot_types::{
    data::VidShare,
    simple_certificate::CertificatePair,
    traits::{
        metrics::{Counter, Gauge, Histogram, Metrics},
        node_implementation::NodeType,
    },
};
use jf_merkle_tree_compat::{MerkleTreeScheme, prelude::MerkleProof};
use tagged_base64::TaggedBase64;
use tokio::{spawn, sync::Mutex, time::sleep};
use tracing::Instrument;

use super::{
    Transaction, VersionedDataSource,
    notifier::Notifier,
    storage::{
        Aggregate, AggregatesStorage, AvailabilityStorage, ExplorerStorage,
        MerklizedStateHeightStorage, MerklizedStateStorage, NodeStorage, UpdateAggregatesStorage,
        UpdateAvailabilityStorage,
        pruning::{PruneStorage, PrunedHeightDataSource, PrunedHeightStorage},
    },
};
use crate::{
    Header, Payload, QueryError, QueryResult,
    availability::{
        AvailabilityDataSource, BlockId, BlockInfo, BlockQueryData, BlockWithTransaction, Fetch,
        FetchStream, HeaderQueryData, LeafId, LeafQueryData, NamespaceId, PayloadMetadata,
        PayloadQueryData, QueryableHeader, QueryablePayload, TransactionHash,
        UpdateAvailabilityData, VidCommonMetadata, VidCommonQueryData,
    },
    data_source::fetching::{leaf::RangeRequest, vid::VidCommonRangeFetcher},
    explorer::{self, ExplorerDataSource},
    fetching::{self, NonEmptyRange, Provider, request},
    merklized_state::{
        MerklizedState, MerklizedStateDataSource, MerklizedStateHeightPersistence, Snapshot,
    },
    metrics::PrometheusMetrics,
    node::{
        NodeDataSource, SyncStatus, SyncStatusQueryData, SyncStatusRange, TimeWindowQueryData,
        WindowStart,
    },
    status::{HasMetrics, StatusDataSource},
    task::BackgroundTask,
    types::HeightIndexed,
};

mod block;
mod header;
mod leaf;
mod transaction;
mod vid;

use self::{
    block::{PayloadFetcher, PayloadRangeFetcher},
    leaf::{LeafFetcher, LeafRangeFetcher},
    transaction::TransactionRequest,
    vid::{VidCommonFetcher, VidCommonRequest},
};

/// Builder for [`FetchingDataSource`] with configuration.
pub struct Builder<Types, S, P> {
    storage: S,
    provider: P,
    backoff: ExponentialBackoffBuilder,
    rate_limit: usize,
    range_chunk_size: usize,
    proactive_interval: Duration,
    proactive_range_chunk_size: usize,
    sync_status_chunk_size: usize,
    active_fetch_delay: Duration,
    chunk_fetch_delay: Duration,
    proactive_fetching: bool,
    aggregator: bool,
    aggregator_chunk_size: Option<usize>,
    leaf_only: bool,
    sync_status_ttl: Duration,
    _types: PhantomData<Types>,
}

impl<Types, S, P> Builder<Types, S, P> {
    /// Construct a new builder with the given storage and fetcher and the default options.
    pub fn new(storage: S, provider: P) -> Self {
        let mut default_backoff = ExponentialBackoffBuilder::default();
        default_backoff
            .with_initial_interval(Duration::from_secs(1))
            .with_multiplier(2.)
            .with_max_interval(Duration::from_secs(32))
            .with_max_elapsed_time(Some(Duration::from_secs(64)));

        Self {
            storage,
            provider,
            backoff: default_backoff,
            rate_limit: 32,
            range_chunk_size: 25,
            proactive_interval: Duration::from_hours(8),
            proactive_range_chunk_size: 100,
            sync_status_chunk_size: 100_000,
            active_fetch_delay: Duration::from_millis(50),
            chunk_fetch_delay: Duration::from_millis(100),
            proactive_fetching: true,
            aggregator: true,
            aggregator_chunk_size: None,
            leaf_only: false,
            sync_status_ttl: Duration::from_mins(5),
            _types: Default::default(),
        }
    }

    pub fn leaf_only(mut self) -> Self {
        self.leaf_only = true;
        self
    }

    /// Set the minimum delay between retries of failed operations.
    pub fn with_min_retry_interval(mut self, interval: Duration) -> Self {
        self.backoff.with_initial_interval(interval);
        self
    }

    /// Set the maximum delay between retries of failed operations.
    pub fn with_max_retry_interval(mut self, interval: Duration) -> Self {
        self.backoff.with_max_interval(interval);
        self
    }

    /// Set the multiplier for exponential backoff when retrying failed requests.
    pub fn with_retry_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff.with_multiplier(multiplier);
        self
    }

    /// Set the randomization factor for randomized backoff when retrying failed requests.
    pub fn with_retry_randomization_factor(mut self, factor: f64) -> Self {
        self.backoff.with_randomization_factor(factor);
        self
    }

    /// Set the maximum time to retry failed operations before giving up.
    pub fn with_retry_timeout(mut self, timeout: Duration) -> Self {
        self.backoff.with_max_elapsed_time(Some(timeout));
        self
    }

    /// Set the maximum number of simultaneous fetches.
    pub fn with_rate_limit(mut self, with_rate_limit: usize) -> Self {
        self.rate_limit = with_rate_limit;
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

    /// Set the time interval between proactive fetching scans.
    ///
    /// See [proactive fetching](self#proactive-fetching).
    pub fn with_proactive_interval(mut self, interval: Duration) -> Self {
        self.proactive_interval = interval;
        self
    }

    /// Set the number of items to process at a time when scanning for proactive fetching.
    ///
    /// This is similar to [`Self::with_range_chunk_size`], but only affects the chunk size for
    /// proactive fetching scans, not for normal subscription streams. This can be useful to tune
    /// the proactive scanner to be more or less greedy with persistent storage resources.
    pub fn with_proactive_range_chunk_size(mut self, range_chunk_size: usize) -> Self {
        self.proactive_range_chunk_size = range_chunk_size;
        self
    }

    /// Set the number of items to process in a single transaction when scanning the database for
    /// missing objects.
    pub fn with_sync_status_chunk_size(mut self, chunk_size: usize) -> Self {
        self.sync_status_chunk_size = chunk_size;
        self
    }

    /// Duration to cache sync status results for.
    ///
    /// Computing the sync status is expensive, and it typically doesn't change that quickly. Thus,
    /// it makes sense to cache the results whenever we do compute it, and return those cached
    /// results if they are not too old.
    pub fn with_sync_status_ttl(mut self, ttl: Duration) -> Self {
        self.sync_status_ttl = ttl;
        self
    }

    /// Add a delay between active fetches in proactive scans.
    ///
    /// This can be used to limit the rate at which this query service makes requests to other query
    /// services during proactive scans. This is useful if the query service has a lot of blocks to
    /// catch up on, as without a delay, scanning can be extremely burdensome on the peer.
    pub fn with_active_fetch_delay(mut self, active_fetch_delay: Duration) -> Self {
        self.active_fetch_delay = active_fetch_delay;
        self
    }

    /// Adds a delay between chunk fetches during proactive scans.
    ///
    /// In a proactive scan, we retrieve a range of objects from a provider or local storage (e.g., a database).
    /// Without a delay between fetching these chunks, the process can become very CPU-intensive, especially
    /// when chunks are retrieved from local storage. While there is already a delay for active fetches
    /// (`active_fetch_delay`), situations may arise when subscribed to an old stream that fetches most of the data
    /// from local storage.
    ///
    /// This additional delay helps to limit constant maximum CPU usage
    /// and ensures that local storage remains accessible to all processes,
    /// not just the proactive scanner.
    pub fn with_chunk_fetch_delay(mut self, chunk_fetch_delay: Duration) -> Self {
        self.chunk_fetch_delay = chunk_fetch_delay;
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

    /// Run without an aggregator.
    ///
    /// This can reduce load on the CPU and the database, but it will cause aggregate statistics
    /// (such as transaction counts) not to update.
    pub fn disable_aggregator(mut self) -> Self {
        self.aggregator = false;
        self
    }

    /// Set the number of items to process at a time when computing aggregate statistics.
    ///
    /// This is similar to [`Self::with_range_chunk_size`], but only affects the chunk size for
    /// the aggregator task, not for normal subscription streams. This can be useful to tune
    /// the aggregator to be more or less greedy with persistent storage resources.
    ///
    /// By default (i.e. if this method is not called) the proactive range chunk size will be set to
    /// whatever the normal range chunk size is.
    pub fn with_aggregator_chunk_size(mut self, chunk_size: usize) -> Self {
        self.aggregator_chunk_size = Some(chunk_size);
        self
    }

    pub fn is_leaf_only(&self) -> bool {
        self.leaf_only
    }
}

impl<Types, S, P> Builder<Types, S, P>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
    Header<Types>: QueryableHeader<Types>,
    S: PruneStorage + VersionedDataSource + HasMetrics + 'static,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types>
        + PrunedHeightStorage
        + NodeStorage<Types>
        + AggregatesStorage<Types>,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types> + UpdateAggregatesStorage<Types>,
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
    // The proactive scanner task. This is only saved here so that we can cancel it on drop.
    scanner: Option<BackgroundTask>,
    // The aggregator task, which derives aggregate statistics from a block stream.
    aggregator: Option<BackgroundTask>,
    pruner: Pruner<Types, S>,
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""), Debug(bound = "S: Debug,   "))]
pub struct Pruner<Types, S>
where
    Types: NodeType,
{
    handle: Option<BackgroundTask>,
    _types: PhantomData<(Types, S)>,
}

impl<Types, S> Pruner<Types, S>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: PruneStorage + Send + Sync + 'static,
{
    async fn new(storage: Arc<S>, backoff: ExponentialBackoff) -> Self {
        let cfg = storage.get_pruning_config();
        let Some(cfg) = cfg else {
            return Self {
                handle: None,
                _types: Default::default(),
            };
        };

        let future = async move {
            for i in 1.. {
                // Delay before we start the pruner run to avoid a useless and expensive prune
                // immediately on startup.
                sleep(cfg.interval()).await;

                tracing::warn!("starting pruner run {i} ");
                Self::prune(storage.clone(), &backoff).await;
            }
        };

        let task = BackgroundTask::spawn("pruner", future);

        Self {
            handle: Some(task),
            _types: Default::default(),
        }
    }

    async fn prune(storage: Arc<S>, backoff: &ExponentialBackoff) {
        // We loop until the whole run pruner run is complete
        let mut pruner = S::Pruner::default();
        'run: loop {
            let mut backoff = backoff.clone();
            backoff.reset();
            'batch: loop {
                match storage.prune(&mut pruner).await {
                    Ok(Some(height)) => {
                        tracing::warn!("Pruned to height {height}");
                        break 'batch;
                    },
                    Ok(None) => {
                        tracing::warn!("pruner run complete.");
                        break 'run;
                    },
                    Err(e) => {
                        tracing::warn!("error pruning batch: {e:#}");
                        if let Some(delay) = backoff.next_backoff() {
                            sleep(delay).await;
                        } else {
                            tracing::error!("pruning run failed after too many errors: {e:#}");
                            break 'run;
                        }
                    },
                }
            }
        }
    }
}

impl<Types, S, P> FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
    Header<Types>: QueryableHeader<Types>,
    S: VersionedDataSource + PruneStorage + HasMetrics + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types> + UpdateAggregatesStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types>
        + NodeStorage<Types>
        + PrunedHeightStorage
        + AggregatesStorage<Types>,
    P: AvailabilityProvider<Types>,
{
    /// Build a [`FetchingDataSource`] with the given `storage` and `provider`.
    pub fn builder(storage: S, provider: P) -> Builder<Types, S, P> {
        Builder::new(storage, provider)
    }

    async fn new(builder: Builder<Types, S, P>) -> anyhow::Result<Self> {
        let leaf_only = builder.is_leaf_only();
        let aggregator = builder.aggregator;
        let aggregator_chunk_size = builder
            .aggregator_chunk_size
            .unwrap_or(builder.range_chunk_size);
        let proactive_fetching = builder.proactive_fetching;
        let proactive_interval = builder.proactive_interval;
        let proactive_range_chunk_size = builder.proactive_range_chunk_size;
        let backoff = builder.backoff.build();
        let scanner_metrics = ScannerMetrics::new(builder.storage.metrics());
        let aggregator_metrics = AggregatorMetrics::new(builder.storage.metrics());

        let fetcher = Arc::new(Fetcher::new(builder).await?);
        let scanner = if proactive_fetching && !leaf_only {
            Some(BackgroundTask::spawn(
                "proactive scanner",
                fetcher.clone().proactive_scan(
                    proactive_interval,
                    proactive_range_chunk_size,
                    scanner_metrics,
                ),
            ))
        } else {
            None
        };

        let aggregator = if aggregator && !leaf_only {
            Some(BackgroundTask::spawn(
                "aggregator",
                fetcher
                    .clone()
                    .aggregate(aggregator_chunk_size, aggregator_metrics),
            ))
        } else {
            None
        };

        let storage = fetcher.storage.clone();

        let pruner = Pruner::new(storage, backoff).await;
        let ds = Self {
            fetcher,
            scanner,
            pruner,
            aggregator,
        };

        Ok(ds)
    }

    /// Get a copy of the (shared) inner storage
    pub fn inner(&self) -> Arc<S> {
        self.fetcher.storage.clone()
    }
}

impl<Types, S, P> AsRef<S> for FetchingDataSource<Types, S, P>
where
    Types: NodeType,
{
    fn as_ref(&self) -> &S {
        &self.fetcher.storage
    }
}

impl<Types, S, P> HasMetrics for FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    S: HasMetrics,
{
    fn metrics(&self) -> &PrometheusMetrics {
        self.as_ref().metrics()
    }
}

#[async_trait]
impl<Types, S, P> StatusDataSource for FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: VersionedDataSource + HasMetrics + Send + Sync + 'static,
    for<'a> S::ReadOnly<'a>: NodeStorage<Types>,
    P: Send + Sync,
{
    async fn block_height(&self) -> QueryResult<usize> {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        tx.block_height().await
    }
}

#[async_trait]
impl<Types, S, P> PrunedHeightDataSource for FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    S: VersionedDataSource + HasMetrics + Send + Sync + 'static,
    for<'a> S::ReadOnly<'a>: PrunedHeightStorage,
    P: Send + Sync,
{
    async fn load_pruned_height(&self) -> anyhow::Result<Option<u64>> {
        let mut tx = self.read().await?;
        tx.load_pruned_height().await
    }
}

#[async_trait]
impl<Types, S, P> AvailabilityDataSource<Types> for FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    async fn get_leaf<ID>(&self, id: ID) -> Fetch<LeafQueryData<Types>>
    where
        ID: Into<LeafId<Types>> + Send + Sync,
    {
        self.fetcher.get(id.into()).await
    }

    async fn get_header<ID>(&self, id: ID) -> Fetch<Header<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync,
    {
        self.fetcher
            .get::<HeaderQueryData<_>>(id.into())
            .await
            .map(|h| h.header)
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

    async fn get_payload_metadata<ID>(&self, id: ID) -> Fetch<PayloadMetadata<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync,
    {
        self.fetcher.get(id.into()).await
    }

    async fn get_vid_common<ID>(&self, id: ID) -> Fetch<VidCommonQueryData<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync,
    {
        self.fetcher.get(VidCommonRequest::from(id.into())).await
    }

    async fn get_vid_common_metadata<ID>(&self, id: ID) -> Fetch<VidCommonMetadata<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync,
    {
        self.fetcher.get(VidCommonRequest::from(id.into())).await
    }

    async fn get_leaf_range<R>(&self, range: R) -> FetchStream<LeafQueryData<Types>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.fetcher.clone().get_range(range)
    }

    async fn get_block_range<R>(&self, range: R) -> FetchStream<BlockQueryData<Types>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.fetcher.clone().get_range(range)
    }

    async fn get_header_range<R>(&self, range: R) -> FetchStream<Header<Types>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        let leaves: FetchStream<LeafQueryData<Types>> = self.fetcher.clone().get_range(range);

        leaves
            .map(|fetch| fetch.map(|leaf| leaf.leaf.block_header().clone()))
            .boxed()
    }

    async fn get_payload_range<R>(&self, range: R) -> FetchStream<PayloadQueryData<Types>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.fetcher.clone().get_range(range)
    }

    async fn get_payload_metadata_range<R>(&self, range: R) -> FetchStream<PayloadMetadata<Types>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.fetcher.clone().get_range(range)
    }

    async fn get_vid_common_range<R>(&self, range: R) -> FetchStream<VidCommonQueryData<Types>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.fetcher.clone().get_range(range)
    }

    async fn get_vid_common_metadata_range<R>(
        &self,
        range: R,
    ) -> FetchStream<VidCommonMetadata<Types>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        self.fetcher.clone().get_range(range)
    }

    async fn get_leaf_range_rev(
        &self,
        start: Bound<usize>,
        end: usize,
    ) -> FetchStream<LeafQueryData<Types>> {
        self.fetcher.clone().get_range_rev(start, end)
    }

    async fn get_block_range_rev(
        &self,
        start: Bound<usize>,
        end: usize,
    ) -> FetchStream<BlockQueryData<Types>> {
        self.fetcher.clone().get_range_rev(start, end)
    }

    async fn get_payload_range_rev(
        &self,
        start: Bound<usize>,
        end: usize,
    ) -> FetchStream<PayloadQueryData<Types>> {
        self.fetcher.clone().get_range_rev(start, end)
    }

    async fn get_payload_metadata_range_rev(
        &self,
        start: Bound<usize>,
        end: usize,
    ) -> FetchStream<PayloadMetadata<Types>> {
        self.fetcher.clone().get_range_rev(start, end)
    }

    async fn get_vid_common_range_rev(
        &self,
        start: Bound<usize>,
        end: usize,
    ) -> FetchStream<VidCommonQueryData<Types>> {
        self.fetcher.clone().get_range_rev(start, end)
    }

    async fn get_vid_common_metadata_range_rev(
        &self,
        start: Bound<usize>,
        end: usize,
    ) -> FetchStream<VidCommonMetadata<Types>> {
        self.fetcher.clone().get_range_rev(start, end)
    }

    async fn get_block_containing_transaction(
        &self,
        h: TransactionHash<Types>,
    ) -> Fetch<BlockWithTransaction<Types>> {
        self.fetcher.clone().get(TransactionRequest::from(h)).await
    }
}

impl<Types, S, P> UpdateAvailabilityData<Types> for FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    async fn append(&self, info: BlockInfo<Types>) -> anyhow::Result<()> {
        let height = info.height() as usize;

        // Save the new decided leaf.
        self.fetcher
            .store(&(info.leaf.clone(), info.qc_chain))
            .await;

        // Trigger a fetch of the parent leaf, if we don't already have it.
        leaf::trigger_fetch_for_parent(&self.fetcher, &info.leaf);

        // Store and notify the block data and VID common, if available. Spawn a fetch to retrieve
        // it, if not.
        //
        // Note a special case here: if the data was not available in the decide event, but _is_
        // available locally in the database, without having to spawn a fetch for it, we _must_
        // notify now. Thus, we must pattern match to distinguish `Fetch::Ready`/`Fetch::Pending`.
        //
        // Why? As soon as we inserted the leaf, the corresponding object may become available, if
        // we already had an identical payload/VID common in the database, from a different block.
        // Then calling `get()` will not spawn a fetch/notification, and existing fetches waiting
        // for the newly decided object to arrive will miss it. Thus, if `get()` returned a `Ready`
        // object, it is our responsibility, as the task processing newly decided objects, to make
        // sure those fetches get notified.
        let block = match info.block {
            Some(block) => Some(block),
            None => match self.fetcher.get::<BlockQueryData<Types>>(height).await {
                Fetch::Ready(block) => Some(block),
                Fetch::Pending(fut) => {
                    let span = tracing::info_span!("fetch missing block", height);
                    spawn(
                        async move {
                            tracing::info!("fetching missing block");
                            fut.await;
                        }
                        .instrument(span),
                    );
                    None
                },
            },
        };
        if let Some(block) = &block {
            self.fetcher.store(block).await;
        }
        let vid = match info.vid_common {
            Some(vid) => Some(vid),
            None => match self.fetcher.get::<VidCommonQueryData<Types>>(height).await {
                Fetch::Ready(vid) => Some(vid),
                Fetch::Pending(fut) => {
                    let span = tracing::info_span!("fetch missing VID common", height);
                    spawn(
                        async move {
                            tracing::info!("fetching missing VID common");
                            fut.await;
                        }
                        .instrument(span),
                    );
                    None
                },
            },
        };
        if let Some(vid) = &vid {
            self.fetcher.store(&(vid.clone(), info.vid_share)).await;
        }

        // Send notifications for the new objects after storing all of them. This ensures that as
        // soon as a fetch for any of these objects resolves, the corresponding data will
        // immediately be available. This isn't strictly required for correctness; after all,
        // objects can generally be fetched as asynchronously as we want. But this is the most
        // intuitive behavior to provide when possible.
        info.leaf.notify(&self.fetcher.notifiers).await;
        if let Some(block) = &block {
            block.notify(&self.fetcher.notifiers).await;
        }
        if let Some(vid) = &vid {
            vid.notify(&self.fetcher.notifiers).await;
        }

        Ok(())
    }
}

impl<Types, S, P> VersionedDataSource for FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    S: VersionedDataSource + Send + Sync,
    P: Send + Sync,
{
    type Transaction<'a>
        = S::Transaction<'a>
    where
        Self: 'a;
    type ReadOnly<'a>
        = S::ReadOnly<'a>
    where
        Self: 'a;

    async fn write(&self) -> anyhow::Result<Self::Transaction<'_>> {
        self.fetcher.write().await
    }

    async fn read(&self) -> anyhow::Result<Self::ReadOnly<'_>> {
        self.fetcher.read().await
    }
}

/// Asynchronous retrieval and storage of [`Fetchable`] resources.
#[derive(Debug)]
struct Fetcher<Types, S, P>
where
    Types: NodeType,
{
    storage: Arc<S>,
    notifiers: Notifiers<Types>,
    provider: Arc<P>,
    leaf_fetcher: Arc<LeafFetcher<Types, S, P>>,
    leaf_range_fetcher: Arc<LeafRangeFetcher<Types, S, P>>,
    payload_fetcher: Option<Arc<PayloadFetcher<Types, S, P>>>,
    payload_range_fetcher: Option<Arc<PayloadRangeFetcher<Types, S, P>>>,
    vid_common_fetcher: Option<Arc<VidCommonFetcher<Types, S, P>>>,
    vid_common_range_fetcher: Option<Arc<VidCommonRangeFetcher<Types, S, P>>>,
    range_chunk_size: usize,
    sync_status_chunk_size: usize,
    // Duration to sleep after each active fetch,
    active_fetch_delay: Duration,
    // Duration to sleep after each chunk fetched
    chunk_fetch_delay: Duration,
    // Exponential backoff when retrying failed operations.
    backoff: ExponentialBackoff,
    // Semaphore limiting the number of simultaneous DB accesses we can have from tasks spawned to
    // retry failed loads.
    retry_semaphore: Arc<Semaphore>,
    leaf_only: bool,
    sync_status_metrics: SyncStatusMetrics,
    sync_status: Mutex<CachedSyncStatus>,
}

impl<Types, S, P> VersionedDataSource for Fetcher<Types, S, P>
where
    Types: NodeType,
    S: VersionedDataSource + Send + Sync,
    P: Send + Sync,
{
    type Transaction<'a>
        = S::Transaction<'a>
    where
        Self: 'a;
    type ReadOnly<'a>
        = S::ReadOnly<'a>
    where
        Self: 'a;

    async fn write(&self) -> anyhow::Result<Self::Transaction<'_>> {
        self.storage.write().await
    }

    async fn read(&self) -> anyhow::Result<Self::ReadOnly<'_>> {
        self.storage.read().await
    }
}

impl<Types, S, P> Fetcher<Types, S, P>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: VersionedDataSource + HasMetrics + Sync,
    for<'a> S::ReadOnly<'a>: PrunedHeightStorage + NodeStorage<Types>,
{
    pub async fn new(builder: Builder<Types, S, P>) -> anyhow::Result<Self> {
        let retry_semaphore = Arc::new(Semaphore::new(builder.rate_limit));
        let backoff = builder.backoff.build();

        let (payload_fetcher, payload_range_fetcher, vid_common_fetcher, vid_common_range_fetcher) =
            if builder.is_leaf_only() {
                (None, None, None, None)
            } else {
                (
                    Some(Arc::new(fetching::Fetcher::new(
                        retry_semaphore.clone(),
                        backoff.clone(),
                    ))),
                    Some(Arc::new(fetching::Fetcher::new(
                        retry_semaphore.clone(),
                        backoff.clone(),
                    ))),
                    Some(Arc::new(fetching::Fetcher::new(
                        retry_semaphore.clone(),
                        backoff.clone(),
                    ))),
                    Some(Arc::new(fetching::Fetcher::new(
                        retry_semaphore.clone(),
                        backoff.clone(),
                    ))),
                )
            };
        let leaf_fetcher = fetching::Fetcher::new(retry_semaphore.clone(), backoff.clone());
        let leaf_range_fetcher = fetching::Fetcher::new(retry_semaphore.clone(), backoff.clone());

        let leaf_only = builder.leaf_only;
        let sync_status_metrics =
            SyncStatusMetrics::new(builder.storage.metrics(), builder.sync_status_chunk_size);

        Ok(Self {
            storage: Arc::new(builder.storage),
            notifiers: Default::default(),
            provider: Arc::new(builder.provider),
            leaf_fetcher: Arc::new(leaf_fetcher),
            leaf_range_fetcher: Arc::new(leaf_range_fetcher),
            payload_fetcher,
            payload_range_fetcher,
            vid_common_fetcher,
            vid_common_range_fetcher,
            range_chunk_size: builder.range_chunk_size,
            sync_status_chunk_size: builder.sync_status_chunk_size,
            active_fetch_delay: builder.active_fetch_delay,
            chunk_fetch_delay: builder.chunk_fetch_delay,
            backoff,
            retry_semaphore,
            leaf_only,
            sync_status_metrics,
            sync_status: Mutex::new(CachedSyncStatus::new(builder.sync_status_ttl)),
        })
    }
}

impl<Types, S, P> Fetcher<Types, S, P>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    async fn get<T>(self: &Arc<Self>, req: impl Into<T::Request> + Send) -> Fetch<T>
    where
        T: Fetchable<Types>,
    {
        let req = req.into();

        // Subscribe to notifications before we check storage for the requested object. This ensures
        // that this operation will always eventually succeed as long as the requested object
        // actually exists (or will exist). We will either find it in our local storage and succeed
        // immediately, or (if it exists) someone will *later* come and add it to storage, at which
        // point we will get a notification causing this passive fetch to resolve.
        //
        // Note the "someone" who later fetches the object and adds it to storage may be an active
        // fetch triggered by this very requests, in cases where that is possible, but it need not
        // be.
        let passive_fetch = T::passive_fetch(&self.notifiers, req).await;

        match self.try_get(req).await {
            Ok(Some(obj)) => return Fetch::Ready(obj),
            Ok(None) => return passive(req, passive_fetch),
            Err(err) => {
                tracing::warn!(
                    ?req,
                    "unable to fetch object; spawning a task to retry: {err:#}"
                );
            },
        }

        // We'll use this channel to get the object back if we successfully load it on retry.
        let (send, recv) = oneshot::channel();

        let fetcher = self.clone();
        let mut backoff = fetcher.backoff.clone();
        let span = tracing::warn_span!("get retry", ?req);
        spawn(
            async move {
                backoff.reset();
                let mut delay = backoff.next_backoff().unwrap_or(Duration::from_secs(1));
                loop {
                    let res = {
                        // Limit the number of simultaneous retry tasks hitting the database. When
                        // the database is down, we might have a lot of these tasks running, and if
                        // they all hit the DB at once, they are only going to make things worse.
                        let _guard = fetcher.retry_semaphore.acquire().await;
                        fetcher.try_get(req).await
                    };
                    match res {
                        Ok(Some(obj)) => {
                            // If the object was immediately available after all, signal the
                            // original fetch. We probably just temporarily couldn't access it due
                            // to database errors.
                            tracing::info!(?req, "object was ready after retries");
                            send.send(obj).ok();
                            break;
                        },
                        Ok(None) => {
                            // The object was not immediately available after all, but we have
                            // successfully spawned a fetch for it if possible. The spawned fetch
                            // will notify the original request once it completes.
                            tracing::info!(?req, "spawned fetch after retries");
                            break;
                        },
                        Err(err) => {
                            tracing::warn!(
                                ?req,
                                ?delay,
                                "unable to fetch object, will retry: {err:#}"
                            );
                            sleep(delay).await;
                            if let Some(next_delay) = backoff.next_backoff() {
                                delay = next_delay;
                            }
                        },
                    }
                }
            }
            .instrument(span),
        );

        // Wait for the object to be fetched, either from the local database on retry or from
        // another provider eventually.
        passive(req, select_some(passive_fetch, recv.map(Result::ok)))
    }

    /// Try to get an object from local storage or initialize a fetch if it is missing.
    ///
    /// There are three possible scenarios in this function, indicated by the return type:
    /// * `Ok(Some(obj))`: the requested object was available locally and successfully retrieved
    ///   from the database; no fetch was spawned
    /// * `Ok(None)`: the requested object was not available locally, but a fetch was successfully
    ///   spawned if possible (in other words, if a fetch was not spawned, it was determined that
    ///   the requested object is not fetchable)
    /// * `Err(_)`: it could not be determined whether the object was available locally or whether
    ///   it could be fetched; no fetch was spawned even though the object may be fetchable
    async fn try_get<T>(self: &Arc<Self>, req: T::Request) -> anyhow::Result<Option<T>>
    where
        T: Fetchable<Types>,
    {
        let mut tx = self.read().await.context("opening read transaction")?;
        match T::load(&mut tx, req).await {
            Ok(t) => Ok(Some(t)),
            Err(QueryError::Missing | QueryError::NotFound) => {
                // We successfully queried the database, but the object wasn't there. Try to
                // fetch it.
                tracing::debug!(?req, "object missing from local storage, will try to fetch");
                self.fetch::<T>(&mut tx, req).await?;
                Ok(None)
            },
            Err(err) => {
                // An error occurred while querying the database. We don't know if we need to fetch
                // the object or not. Return an error so we can try again.
                bail!("failed to fetch resource {req:?} from local storage: {err:#}");
            },
        }
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
        let chunk_fetch_delay = self.chunk_fetch_delay;
        let active_fetch_delay = self.active_fetch_delay;

        stream::iter(range_chunks(range, chunk_size))
            .then(move |chunk| {
                let self_clone = self.clone();
                async move {
                    {
                        let chunk = self_clone.get_chunk(chunk).await;

                        // Introduce a delay (`chunk_fetch_delay`) between fetching chunks. This
                        // helps to limit constant high CPU usage when fetching long range of data,
                        // especially for older streams that fetch most of the data from local
                        // storage.
                        sleep(chunk_fetch_delay).await;
                        stream::iter(chunk)
                    }
                }
            })
            .flatten()
            .then(move |f| async move {
                match f {
                    // Introduce a delay (`active_fetch_delay`) for active fetches to reduce load on
                    // the catchup provider. The delay applies between pending fetches, not between
                    // chunks.
                    Fetch::Pending(_) => sleep(active_fetch_delay).await,
                    Fetch::Ready(_) => (),
                };
                f
            })
            .boxed()
    }

    /// Same as [`Self::get_range`], but yields objects in reverse order by height.
    ///
    /// Note that unlike [`Self::get_range`], which accepts any range and yields an infinite stream
    /// if the range has no upper bound, this function requires there to be a defined upper bound,
    /// otherwise we don't know where the reversed stream should _start_. The `end` bound given here
    /// is inclusive; i.e. the first item yielded by the stream will have height `end`.
    fn get_range_rev<T>(
        self: Arc<Self>,
        start: Bound<usize>,
        end: usize,
    ) -> BoxStream<'static, Fetch<T>>
    where
        T: RangedFetchable<Types>,
    {
        let chunk_size = self.range_chunk_size;
        self.get_range_with_chunk_size_rev(chunk_size, start, end)
    }

    /// Same as [`Self::get_range_rev`], but uses the given chunk size instead of the default.
    fn get_range_with_chunk_size_rev<T>(
        self: Arc<Self>,
        chunk_size: usize,
        start: Bound<usize>,
        end: usize,
    ) -> BoxStream<'static, Fetch<T>>
    where
        T: RangedFetchable<Types>,
    {
        let chunk_fetch_delay = self.chunk_fetch_delay;
        let active_fetch_delay = self.active_fetch_delay;

        stream::iter(range_chunks_rev(start, end, chunk_size))
            .then(move |chunk| {
                let self_clone = self.clone();
                async move {
                    {
                        let chunk = self_clone.get_chunk(chunk).await;

                        // Introduce a delay (`chunk_fetch_delay`) between fetching chunks. This
                        // helps to limit constant high CPU usage when fetching long range of data,
                        // especially for older streams that fetch most of the data from local
                        // storage
                        sleep(chunk_fetch_delay).await;
                        stream::iter(chunk.into_iter().rev())
                    }
                }
            })
            .flatten()
            .then(move |f| async move {
                match f {
                    // Introduce a delay (`active_fetch_delay`) for active fetches to reduce load on
                    // the catchup provider. The delay applies between pending fetches, not between
                    // chunks.
                    Fetch::Pending(_) => sleep(active_fetch_delay).await,
                    Fetch::Ready(_) => (),
                };
                f
            })
            .boxed()
    }

    /// Get a range of objects from local storage or a provider.
    ///
    /// This method is similar to `get_range`, except that:
    /// * It fetches all desired objects together, as a single chunk
    /// * It loads the object or triggers fetches right now rather than providing a lazy stream
    ///   which only fetches objects when polled.
    async fn get_chunk<T>(self: &Arc<Self>, chunk: Range<usize>) -> Vec<Fetch<T>>
    where
        T: RangedFetchable<Types>,
    {
        // Subscribe to notifications first. As in [`get`](Self::get), this ensures we won't miss
        // any notifications sent in between checking local storage and triggering a fetch if
        // necessary.
        let passive_fetches = join_all(
            chunk
                .clone()
                .map(|i| T::passive_fetch(&self.notifiers, i.into())),
        )
        .await;

        match self.try_get_chunk(&chunk).await {
            Ok(objs) => {
                // Convert to fetches. Objects which are not immediately available (`None` in the
                // chunk) become passive fetches awaiting a notification of availability.
                return objs
                    .into_iter()
                    .zip(passive_fetches)
                    .enumerate()
                    .map(move |(i, (obj, passive_fetch))| match obj {
                        Some(obj) => Fetch::Ready(obj),
                        None => passive(T::Request::from(chunk.start + i), passive_fetch),
                    })
                    .collect();
            },
            Err(err) => {
                tracing::warn!(
                    ?chunk,
                    "unable to fetch chunk; spawning a task to retry: {err:#}"
                );
            },
        }

        // We'll use these channels to get the objects back that we successfully load on retry.
        let (send, recv): (Vec<_>, Vec<_>) =
            repeat_with(oneshot::channel).take(chunk.len()).unzip();

        {
            let fetcher = self.clone();
            let mut backoff = fetcher.backoff.clone();
            let chunk = chunk.clone();
            let span = tracing::warn_span!("get_chunk retry", ?chunk);
            spawn(
                async move {
                    backoff.reset();
                    let mut delay = backoff.next_backoff().unwrap_or(Duration::from_secs(1));
                    loop {
                        let res = {
                            // Limit the number of simultaneous retry tasks hitting the database.
                            // When the database is down, we might have a lot of these tasks
                            // running, and if they all hit the DB at once, they are only going to
                            // make things worse.
                            let _guard = fetcher.retry_semaphore.acquire().await;
                            fetcher.try_get_chunk(&chunk).await
                        };
                        match res {
                            Ok(objs) => {
                                for (i, (obj, sender)) in objs.into_iter().zip(send).enumerate() {
                                    if let Some(obj) = obj {
                                        // If the object was immediately available after all, signal
                                        // the original fetch. We probably just temporarily couldn't
                                        // access it due to database errors.
                                        tracing::info!(?chunk, i, "object was ready after retries");
                                        sender.send(obj).ok();
                                    } else {
                                        // The object was not immediately available after all, but
                                        // we have successfully spawned a fetch for it if possible.
                                        // The spawned fetch will notify the original request once
                                        // it completes.
                                        tracing::info!(?chunk, i, "spawned fetch after retries");
                                    }
                                }
                                break;
                            },
                            Err(err) => {
                                tracing::warn!(
                                    ?chunk,
                                    ?delay,
                                    "unable to fetch chunk, will retry: {err:#}"
                                );
                                sleep(delay).await;
                                if let Some(next_delay) = backoff.next_backoff() {
                                    delay = next_delay;
                                }
                            },
                        }
                    }
                }
                .instrument(span),
            );
        }

        // Wait for the objects to be fetched, either from the local database on retry or from
        // another provider eventually.
        passive_fetches
            .into_iter()
            .zip(recv)
            .enumerate()
            .map(move |(i, (passive_fetch, recv))| {
                passive(
                    T::Request::from(chunk.start + i),
                    select_some(passive_fetch, recv.map(Result::ok)),
                )
            })
            .collect()
    }

    /// Try to get a range of objects from local storage, initializing fetches if any are missing.
    ///
    /// If this function succeeded, then for each object in the requested range, either:
    /// * the object was available locally, and corresponds to `Some(_)` object in the result
    /// * the object was not available locally (and corresponds to `None` in the result), but a
    ///   fetch was successfully spawned if possible (in other words, if a fetch was not spawned, it
    ///   was determined that the requested object is not fetchable)
    ///
    /// This function will fail if it could not be determined which objects in the requested range
    /// are available locally, or if, for any missing object, it could not be determined whether
    /// that object is fetchable. In this case, there may be no fetch spawned for certain objects in
    /// the requested range, even if those objects are actually fetchable.
    async fn try_get_chunk<T>(
        self: &Arc<Self>,
        chunk: &Range<usize>,
    ) -> anyhow::Result<Vec<Option<T>>>
    where
        T: RangedFetchable<Types>,
    {
        let mut tx = self.read().await.context("opening read transaction")?;
        let ts = T::load_range(&mut tx, chunk.clone())
            .await
            .context(format!("when fetching items in range {chunk:?}"))?;

        // Log and discard error information; we want a list of Option where None indicates an
        // object that needs to be fetched. Note that we don't use `FetchRequest::might_exist` to
        // silence the logs here when an object is missing that is not expected to exist at all.
        // When objects are not expected to exist, `load_range` should just return a truncated list
        // rather than returning `Err` objects, so if there are errors in here they are unexpected
        // and we do want to log them.
        let ts = ts.into_iter().filter_map(ResultExt::ok_or_trace);

        // Kick off a fetch for each missing object.
        let mut results = Vec::with_capacity(chunk.len());
        for t in ts {
            // Fetch missing objects that should come before `t`.
            while chunk.start + results.len() < t.height() as usize {
                tracing::debug!(
                    "item {} in chunk not available, will be fetched",
                    results.len()
                );
                self.fetch::<T>(&mut tx, (chunk.start + results.len()).into())
                    .await?;
                results.push(None);
            }

            results.push(Some(t));
        }
        // Fetch missing objects from the end of the range.
        while results.len() < chunk.len() {
            self.fetch::<T>(&mut tx, (chunk.start + results.len()).into())
                .await?;
            results.push(None);
        }

        Ok(results)
    }

    /// Spawn an active fetch for the requested object, if possible.
    ///
    /// On success, either an active fetch for `req` has been spawned, or it has been determined
    /// that `req` is not fetchable. Fails if it cannot be determined (e.g. due to errors in the
    /// local database) whether `req` is fetchable or not.
    async fn fetch<T>(
        self: &Arc<Self>,
        tx: &mut <Self as VersionedDataSource>::ReadOnly<'_>,
        req: T::Request,
    ) -> anyhow::Result<()>
    where
        T: Fetchable<Types>,
    {
        tracing::debug!("fetching resource {req:?}");

        // Trigger an active fetch from a remote provider if possible.
        let heights = Heights::load(tx)
            .await
            .context("failed to load heights; cannot definitively say object might exist")?;
        if req.might_exist(heights) {
            T::active_fetch(tx, self.clone(), req).await?;
        } else {
            tracing::debug!("not fetching object {req:?} that cannot exist at {heights:?}");
        }
        Ok(())
    }

    /// Proactively search for and retrieve missing objects.
    ///
    /// This function will proactively identify and retrieve blocks and leaves which are missing
    /// from storage. It will run until cancelled, thus, it is meant to be spawned as a background
    /// task rather than called synchronously.
    async fn proactive_scan(
        self: Arc<Self>,
        interval: Duration,
        chunk_size: usize,
        metrics: ScannerMetrics,
    ) {
        for i in 0.. {
            let span = tracing::warn_span!("proactive scan", i);
            metrics.running.set(1);
            metrics.current_scan.set(i);
            async {
                let sync_status = {
                    match self.sync_status().await {
                        Ok(st) => st,
                        Err(err) => {
                            tracing::warn!(
                                "unable to load sync status, scan will be skipped: {err:#}"
                            );
                            return;
                        },
                    }
                };
                tracing::info!(?sync_status, "starting scan");
                metrics.missing_blocks.set(sync_status.blocks.missing);
                metrics.missing_vid.set(sync_status.vid_common.missing);

                // Fetch missing blocks. This will also trigger a fetch for the corresponding
                // missing leaves.
                for range in sync_status.blocks.ranges {
                    metrics.scanned_blocks.set(range.start);
                    if range.status != SyncStatus::Missing {
                        metrics.scanned_blocks.set(range.end);
                        continue;
                    }

                    tracing::info!(?range, "fetching missing block range");

                    // Break the range into manageable, aligned chunks (which improves cacheability
                    // for the upstream server).
                    //
                    // We iterate in reverse order because leaves are inherently fetched in reverse,
                    // since we cannot (actively) fetch a leaf until we have the subsequent leaf,
                    // which tells us what the hash of its parent should be.
                    for chunk in range_chunks_aligned_rev(
                        Bound::Included(range.start),
                        range.end - 1,
                        chunk_size,
                    ) {
                        tracing::info!(?chunk, "fetching missing block chunk");

                        // Fetching the payload metadata is enough to trigger an active fetch of the
                        // corresponding leaf and the full block if they are missing.
                        self.get::<NonEmptyRange<BlockQueryData<Types>>>(RangeRequest {
                            start: chunk.start as u64,
                            end: chunk.end as u64,
                        })
                        .await
                        .await;

                        metrics
                            .missing_blocks
                            .update((chunk.start as i64) - (chunk.end as i64));
                        metrics.scanned_blocks.set(chunk.end);
                    }
                }

                // Do the same for VID.
                for range in sync_status.vid_common.ranges {
                    metrics.scanned_vid.set(range.start);
                    if range.status != SyncStatus::Missing {
                        metrics.scanned_vid.set(range.end);
                        continue;
                    }

                    tracing::info!(?range, "fetching missing VID range");
                    for chunk in range_chunks_aligned_rev(
                        Bound::Included(range.start),
                        range.end - 1,
                        chunk_size,
                    ) {
                        tracing::info!(?chunk, "fetching missing VID chunk");
                        self.get::<NonEmptyRange<VidCommonQueryData<Types>>>(RangeRequest {
                            start: chunk.start as u64,
                            end: chunk.end as u64,
                        })
                        .await
                        .await;

                        metrics
                            .missing_vid
                            .update((chunk.start as i64) - (chunk.end as i64));
                        metrics.scanned_vid.set(chunk.end);
                    }
                }

                tracing::info!("completed proactive scan, will scan again in {interval:?}");

                // Reset metrics.
                metrics.running.set(0);
            }
            .instrument(span)
            .await;

            sleep(interval).await;
        }
    }
}

impl<Types, S, P> Fetcher<Types, S, P>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::ReadOnly<'a>: NodeStorage<Types> + PrunedHeightStorage,
    P: Send + Sync,
{
    async fn sync_status(&self) -> anyhow::Result<SyncStatusQueryData> {
        // Check the cache first. This prevents the expensive sync_status queries from being run too
        // often, and also ensures that if two tasks try to get the sync status at the same time,
        // only one will actually compute it; the other will find the cache populated by the time it
        // gets a lock on the mutex.
        let mut cache = self.sync_status.lock().await;
        if let Some(sync_status) = cache.try_get() {
            return Ok(sync_status.clone());
        }
        tracing::debug!("updating sync status");

        let heights = {
            let mut tx = self
                .read()
                .await
                .context("opening transaction to load heights")?;
            Heights::load(&mut tx).await.context("loading heights")?
        };

        let mut res = SyncStatusQueryData {
            pruned_height: heights.pruned_height.map(|h| h as usize),
            ..Default::default()
        };
        let start = if let Some(height) = res.pruned_height {
            // Add an initial range for pruned data.
            let range = SyncStatusRange {
                status: SyncStatus::Pruned,
                start: 0,
                end: height + 1,
            };
            res.blocks.ranges.push(range);
            res.leaves.ranges.push(range);
            res.vid_common.ranges.push(range);

            height + 1
        } else {
            0
        };

        // Break the range into manageable chunks, so we don't hold any one database transaction
        // open for too long.
        for chunk in range_chunks(
            start..(heights.height as usize),
            self.sync_status_chunk_size,
        ) {
            tracing::debug!(chunk.start, chunk.end, "checking sync status in sub-range");
            let metrics = self.sync_status_metrics.start_range(&chunk);
            let mut tx = self
                .read()
                .await
                .context("opening transaction to sync status range")?;
            let range_status = tx
                .sync_status_for_range(chunk.start, chunk.end)
                .await
                .context(format!("checking sync status in sub-range {chunk:?}"))?;
            tracing::debug!(
                chunk.start,
                chunk.end,
                ?range_status,
                "found sync status for range"
            );

            res.blocks.extend(range_status.blocks);
            res.leaves.extend(range_status.leaves);
            res.vid_common.extend(range_status.vid_common);
            metrics.end();
        }

        cache.update(res.clone());
        Ok(res)
    }
}

impl<Types, S, P> Fetcher<Types, S, P>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types> + UpdateAggregatesStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types>
        + NodeStorage<Types>
        + PrunedHeightStorage
        + AggregatesStorage<Types>,
    P: AvailabilityProvider<Types>,
{
    #[tracing::instrument(skip_all)]
    async fn aggregate(self: Arc<Self>, chunk_size: usize, metrics: AggregatorMetrics) {
        loop {
            let prev_aggregate = loop {
                let mut tx = match self.read().await {
                    Ok(tx) => tx,
                    Err(err) => {
                        tracing::error!("unable to open read tx: {err:#}");
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    },
                };
                match tx.load_prev_aggregate().await {
                    Ok(agg) => break agg,
                    Err(err) => {
                        tracing::error!("unable to load previous aggregate: {err:#}");
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    },
                }
            };

            let (start, mut prev_aggregate) = match prev_aggregate {
                Some(aggregate) => (aggregate.height as usize + 1, aggregate),
                None => (0, Aggregate::default()),
            };

            tracing::info!(start, "starting aggregator");
            metrics.height.set(start);

            let mut blocks = self
                .clone()
                .get_range_with_chunk_size::<_, PayloadMetadata<Types>>(chunk_size, start..)
                .then(Fetch::resolve)
                .ready_chunks(chunk_size)
                .boxed();
            while let Some(chunk) = blocks.next().await {
                let Some(last) = chunk.last() else {
                    // This is not supposed to happen, but if the chunk is empty, just skip it.
                    tracing::warn!("ready_chunks returned an empty chunk");
                    continue;
                };
                let height = last.height();
                let num_blocks = chunk.len();
                tracing::debug!(
                    num_blocks,
                    height,
                    "updating aggregate statistics for chunk"
                );
                loop {
                    let res = async {
                        let mut tx = self.write().await.context("opening transaction")?;
                        let aggregate =
                            tx.update_aggregates(prev_aggregate.clone(), &chunk).await?;
                        tx.commit().await.context("committing transaction")?;
                        prev_aggregate = aggregate;
                        anyhow::Result::<_>::Ok(())
                    }
                    .await;
                    match res {
                        Ok(()) => {
                            break;
                        },
                        Err(err) => {
                            tracing::warn!(
                                num_blocks,
                                height,
                                "failed to update aggregates for chunk: {err:#}"
                            );
                            sleep(Duration::from_secs(1)).await;
                        },
                    }
                }
                metrics.height.set(height as usize);
            }
            tracing::warn!("aggregator block stream ended unexpectedly; will restart");
        }
    }
}

impl<Types, S, P> Fetcher<Types, S, P>
where
    Types: NodeType,
    S: VersionedDataSource,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
{
    /// Store an object and notify anyone waiting on this object that it is available.
    async fn store_and_notify<T>(&self, obj: &T)
    where
        T: Storable<Types>,
    {
        self.store(obj).await;

        // Send a notification about the newly received object. It is important that we do this
        // _after_ our attempt to store the object in local storage, otherwise there is a potential
        // missed notification deadlock:
        // * we send the notification
        // * a task calls [`get`](Self::get) or [`get_chunk`](Self::get_chunk), finds that the
        //   requested object is not in storage, and begins waiting for a notification
        // * we store the object. This ensures that no other task will be triggered to fetch it,
        //   which means no one will ever notify the waiting task.
        //
        // Note that we send the notification regardless of whether the store actually succeeded or
        // not. This is to avoid _another_ subtle deadlock: if we failed to notify just because we
        // failed to store, some fetches might not resolve, even though the object in question has
        // actually been fetched. This should actually be ok, because as long as the object is not
        // in storage, eventually some other task will come along and fetch, store, and notify about
        // it. However, this is certainly not ideal, since we could resolve those pending fetches
        // right now, and it causes bigger problems when the fetch that fails to resolve is the
        // proactive scanner task, who is often the one that would eventually come along and
        // re-fetch the object.
        //
        // The key thing to note is that it does no harm to notify even if we fail to store: at best
        // we wake some tasks up sooner; at worst, anyone who misses the notification still
        // satisfies the invariant that we only wait on notifications for objects which are not in
        // storage, and eventually some other task will come along, find the object missing from
        // storage, and re-fetch it.
        obj.notify(&self.notifiers).await;
    }

    async fn store<T>(&self, obj: &T)
    where
        T: Storable<Types>,
    {
        let try_store = || async {
            let mut tx = self.storage.write().await?;
            obj.clone().store(&mut tx, self.leaf_only).await?;
            tx.commit().await
        };

        // Store the object in local storage, so we can avoid fetching it in the future.
        let mut backoff = self.backoff.clone();
        backoff.reset();
        loop {
            let Err(err) = try_store().await else {
                break;
            };
            // It is unfortunate if this fails, but we can still proceed by notifying with the
            // object that we fetched, keeping it in memory. Log the error, retry a few times, and
            // eventually move on.
            tracing::warn!(
                obj = obj.debug_name(),
                "failed to store fetched object: {err:#}"
            );

            let Some(delay) = backoff.next_backoff() else {
                break;
            };
            tracing::info!(?delay, "retrying failed operation");
            sleep(delay).await;
        }
    }
}

#[derive(Debug)]
struct Notifiers<Types>
where
    Types: NodeType,
{
    block: Notifier<BlockQueryData<Types>>,
    leaf: Notifier<LeafQueryData<Types>>,
    vid_common: Notifier<VidCommonQueryData<Types>>,
}

impl<Types> Default for Notifiers<Types>
where
    Types: NodeType,
{
    fn default() -> Self {
        Self {
            block: Notifier::new(),
            leaf: Notifier::new(),
            vid_common: Notifier::new(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Heights {
    height: u64,
    pruned_height: Option<u64>,
}

impl Heights {
    async fn load<Types, T>(tx: &mut T) -> anyhow::Result<Self>
    where
        Types: NodeType,
        Header<Types>: QueryableHeader<Types>,
        T: NodeStorage<Types> + PrunedHeightStorage + Send,
    {
        let height = tx.block_height().await.context("loading block height")? as u64;
        let pruned_height = tx
            .load_pruned_height()
            .await
            .context("loading pruned height")?;
        Ok(Self {
            height,
            pruned_height,
        })
    }

    fn might_exist(self, h: u64) -> bool {
        h < self.height && self.pruned_height.is_none_or(|ph| h > ph)
    }
}

#[async_trait]
impl<Types, S, P, State, const ARITY: usize> MerklizedStateDataSource<Types, State, ARITY>
    for FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    S: VersionedDataSource + 'static,
    for<'a> S::ReadOnly<'a>: MerklizedStateStorage<Types, State, ARITY>,
    P: Send + Sync,
    State: MerklizedState<Types, ARITY> + 'static,
    <State as MerkleTreeScheme>::Commitment: Send,
{
    async fn get_path(
        &self,
        snapshot: Snapshot<Types, State, ARITY>,
        key: State::Key,
    ) -> QueryResult<MerkleProof<State::Entry, State::Key, State::T, ARITY>> {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        tx.get_path(snapshot, key).await
    }
}

#[async_trait]
impl<Types, S, P> MerklizedStateHeightPersistence for FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::ReadOnly<'a>: MerklizedStateHeightStorage,
    P: Send + Sync,
{
    async fn get_last_state_height(&self) -> QueryResult<usize> {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        tx.get_last_state_height().await
    }
}

#[async_trait]
impl<Types, S, P> NodeDataSource<Types> for FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::ReadOnly<'a>: NodeStorage<Types> + PrunedHeightStorage,
    P: Send + Sync,
{
    async fn block_height(&self) -> QueryResult<usize> {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        tx.block_height().await
    }

    async fn count_transactions_in_range(
        &self,
        range: impl RangeBounds<usize> + Send,
        namespace: Option<NamespaceId<Types>>,
    ) -> QueryResult<usize> {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        tx.count_transactions_in_range(range, namespace).await
    }

    async fn payload_size_in_range(
        &self,
        range: impl RangeBounds<usize> + Send,
        namespace: Option<NamespaceId<Types>>,
    ) -> QueryResult<usize> {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        tx.payload_size_in_range(range, namespace).await
    }

    async fn vid_share<ID>(&self, id: ID) -> QueryResult<VidShare>
    where
        ID: Into<BlockId<Types>> + Send + Sync,
    {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        tx.vid_share(id).await
    }

    async fn sync_status(&self) -> QueryResult<SyncStatusQueryData> {
        self.fetcher
            .sync_status()
            .await
            .map_err(|err| QueryError::Error {
                message: format!("{err:#}"),
            })
    }

    async fn get_header_window(
        &self,
        start: impl Into<WindowStart<Types>> + Send + Sync,
        end: u64,
        limit: usize,
    ) -> QueryResult<TimeWindowQueryData<Header<Types>>> {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        tx.get_header_window(start, end, limit).await
    }
}

#[async_trait]
impl<Types, S, P> ExplorerDataSource<Types> for FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
    Header<Types>: QueryableHeader<Types> + explorer::traits::ExplorerHeader<Types>,
    crate::Transaction<Types>: explorer::traits::ExplorerTransaction<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::ReadOnly<'a>: ExplorerStorage<Types>,
    P: Send + Sync,
{
    async fn get_block_summaries(
        &self,
        request: explorer::query_data::GetBlockSummariesRequest<Types>,
    ) -> Result<
        Vec<explorer::query_data::BlockSummary<Types>>,
        explorer::query_data::GetBlockSummariesError,
    > {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        tx.get_block_summaries(request).await
    }

    async fn get_block_detail(
        &self,
        request: explorer::query_data::BlockIdentifier<Types>,
    ) -> Result<explorer::query_data::BlockDetail<Types>, explorer::query_data::GetBlockDetailError>
    {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        tx.get_block_detail(request).await
    }

    async fn get_transaction_summaries(
        &self,
        request: explorer::query_data::GetTransactionSummariesRequest<Types>,
    ) -> Result<
        Vec<explorer::query_data::TransactionSummary<Types>>,
        explorer::query_data::GetTransactionSummariesError,
    > {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        tx.get_transaction_summaries(request).await
    }

    async fn get_transaction_detail(
        &self,
        request: explorer::query_data::TransactionIdentifier<Types>,
    ) -> Result<
        explorer::query_data::TransactionDetailResponse<Types>,
        explorer::query_data::GetTransactionDetailError,
    > {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        tx.get_transaction_detail(request).await
    }

    async fn get_explorer_summary(
        &self,
    ) -> Result<
        explorer::query_data::ExplorerSummary<Types>,
        explorer::query_data::GetExplorerSummaryError,
    > {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        tx.get_explorer_summary().await
    }

    async fn get_search_results(
        &self,
        query: TaggedBase64,
    ) -> Result<
        explorer::query_data::SearchResult<Types>,
        explorer::query_data::GetSearchResultsError,
    > {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        tx.get_search_results(query).await
    }
}

/// A provider which can be used as a fetcher by the availability service.
pub trait AvailabilityProvider<Types: NodeType>:
    Provider<Types, request::LeafRequest<Types>>
    + Provider<Types, request::LeafRangeRequest<Types>>
    + Provider<Types, request::PayloadRequest>
    + Provider<Types, request::BlockRangeRequest>
    + Provider<Types, request::VidCommonRequest>
    + Provider<Types, request::VidCommonRangeRequest>
    + Sync
    + 'static
{
}
impl<Types: NodeType, P> AvailabilityProvider<Types> for P where
    P: Provider<Types, request::LeafRequest<Types>>
        + Provider<Types, request::LeafRangeRequest<Types>>
        + Provider<Types, request::PayloadRequest>
        + Provider<Types, request::BlockRangeRequest>
        + Provider<Types, request::VidCommonRequest>
        + Provider<Types, request::VidCommonRangeRequest>
        + Sync
        + 'static
{
}

trait FetchRequest: Copy + Debug + Send + Sync + 'static {
    /// Indicate whether it is possible this object could exist.
    ///
    /// This can filter out requests quickly for objects that cannot possibly exist, such as
    /// requests for objects with a height greater than the current block height. Not only does this
    /// let us fail faster for such requests (without touching storage at all), it also helps keep
    /// logging quieter when we fail to fetch an object because the user made a bad request, while
    /// still being fairly loud when we fail to fetch an object that might have really existed.
    ///
    /// This method is conservative: it returns `true` if it cannot tell whether the given object
    /// could exist or not.
    fn might_exist(self, _heights: Heights) -> bool {
        true
    }
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
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    /// A succinct specification of the object to be fetched.
    type Request: FetchRequest;

    /// Does this object satisfy the given request?
    fn satisfies(&self, req: Self::Request) -> bool;

    /// Spawn a task to fetch the object from a remote provider, if possible.
    ///
    /// An active fetch will only be triggered if:
    /// * There is not already an active fetch in progress for the same object
    /// * The requested object is known to exist. For example, we will fetch a leaf by height but
    ///   not by hash, since we can't guarantee that a leaf with an arbitrary hash exists. Note that
    ///   this function assumes `req.might_exist()` has already been checked before calling it, and
    ///   so may do unnecessary work if the caller does not ensure this.
    ///
    /// If we do trigger an active fetch for an object, any passive listeners for the object will be
    /// notified once it has been retrieved. If we do not trigger an active fetch for an object,
    /// this function does nothing. In either case, as long as the requested object does in fact
    /// exist, we will eventually receive it passively, since we will eventually receive all blocks
    /// and leaves that are ever produced. Active fetching merely helps us receive certain objects
    /// sooner.
    ///
    /// This function fails if it _might_ be possible to actively fetch the requested object, but we
    /// were unable to do so (e.g. due to errors in the database).
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
        P: AvailabilityProvider<Types>;

    /// Wait for someone else to fetch the object.
    async fn passive_fetch(notifiers: &Notifiers<Types>, req: Self::Request) -> PassiveFetch<Self>;

    /// Load an object from local storage.
    ///
    /// This function assumes `req.might_exist()` has already been checked before calling it, and so
    /// may do unnecessary work if the caller does not ensure this.
    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>;
}

type PassiveFetch<T> = BoxFuture<'static, Option<T>>;

#[async_trait]
trait RangedFetchable<Types>: Fetchable<Types, Request = Self::RangedRequest> + HeightIndexed
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    type RangedRequest: FetchRequest + From<usize> + Send;

    /// Load a range of these objects from local storage.
    async fn load_range<S, R>(storage: &mut S, range: R) -> QueryResult<Vec<QueryResult<Self>>>
    where
        S: AvailabilityStorage<Types>,
        R: RangeBounds<usize> + Send + 'static;
}

/// An object which can be stored in the database.
trait Storable<Types: NodeType>: Clone {
    /// The name of this object, for debugging purposes.
    fn debug_name(&self) -> String;

    /// Notify anyone waiting for this object that it has become available.
    fn notify(&self, notifiers: &Notifiers<Types>) -> impl Send + Future<Output = ()>;

    /// Store the object in the local database.
    fn store(
        &self,
        storage: &mut impl UpdateAvailabilityStorage<Types>,
        leaf_only: bool,
    ) -> impl Send + Future<Output = anyhow::Result<()>>;
}

impl<Types: NodeType> Storable<Types>
    for (LeafQueryData<Types>, Option<[CertificatePair<Types>; 2]>)
{
    fn debug_name(&self) -> String {
        format!("leaf {} with QC chain", self.0.height())
    }

    async fn notify(&self, notifiers: &Notifiers<Types>) {
        self.0.notify(notifiers).await;
    }

    async fn store(
        &self,
        storage: &mut impl UpdateAvailabilityStorage<Types>,
        _leaf_only: bool,
    ) -> anyhow::Result<()> {
        storage
            .insert_leaf_with_qc_chain(&self.0, self.1.clone())
            .await
    }
}

/// Break a range into fixed-size chunks.
fn range_chunks<R>(range: R, chunk_size: usize) -> impl Iterator<Item = Range<usize>>
where
    R: RangeBounds<usize>,
{
    // Transform range to explicit start (inclusive) and end (exclusive) bounds.
    let Range { mut start, end } = range_to_bounds(range);
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

/// Break a range into fixed-alignment chunks.
///
/// Each chunk is of size `alignment`, and starts on a multiple of `alignment`, with the possible
/// exception of the first chunk (which may be misaligned and small) and the last (which may be
/// small).
#[allow(dead_code)]
fn range_chunks_aligned<R>(range: R, alignment: usize) -> impl Iterator<Item = Range<usize>>
where
    R: RangeBounds<usize>,
{
    // Transform range to explicit start (inclusive) and end (exclusive) bounds.
    let Range { mut start, end } = range_to_bounds(range);

    // If necessary, generate a partial first chunk to force the remaining chunks into alignment.
    let first = if start.is_multiple_of(alignment) {
        None
    } else {
        // The partial first chunk ends at the next multiple of the alignment, or at the end of the
        // overall range, whichever comes first.
        let chunk_end = min(start.next_multiple_of(alignment), end);
        let chunk = start..chunk_end;

        // Start the series of aligned chunks at the end of the partial first chunk.
        start = chunk_end;
        Some(chunk)
    };

    first.into_iter().chain(range_chunks(start..end, alignment))
}

/// Transform a range to explicit start (inclusive) and end (exclusive) bounds.
fn range_to_bounds(range: impl RangeBounds<usize>) -> Range<usize> {
    let start = match range.start_bound() {
        Bound::Included(i) => *i,
        Bound::Excluded(i) => *i + 1,
        Bound::Unbounded => 0,
    };
    let end = match range.end_bound() {
        Bound::Included(i) => *i + 1,
        Bound::Excluded(i) => *i,
        Bound::Unbounded => usize::MAX,
    };
    Range { start, end }
}

/// Break a range into fixed-size chunks, starting from the end and moving towards the start.
///
/// While the chunks are yielded in reverse order, from `end` to `start`, each individual chunk is
/// in the usual ascending order. That is, the first chunk ends with `end` and the last chunk starts
/// with `start`.
///
/// Note that unlike [`range_chunks`], which accepts any range and yields an infinite iterator if
/// the range has no upper bound, this function requires there to be a defined upper bound,
/// otherwise we don't know where the reversed iterator should _start_. The `end` bound given here
/// is inclusive; i.e. the end of the first chunk yielded by the stream will be exactly `end`.
fn range_chunks_rev(
    start: Bound<usize>,
    end: usize,
    chunk_size: usize,
) -> impl Iterator<Item = Range<usize>> {
    // Transform the start bound to be inclusive.
    let start = match start {
        Bound::Included(i) => i,
        Bound::Excluded(i) => i + 1,
        Bound::Unbounded => 0,
    };
    // Transform the end bound to be exclusive.
    let mut end = end + 1;

    std::iter::from_fn(move || {
        let chunk_start = max(start, end.saturating_sub(chunk_size));
        if end <= chunk_start {
            return None;
        }

        let chunk = chunk_start..end;
        end = chunk_start;
        Some(chunk)
    })
}

/// Break a range into fixed-alignment chunks, starting from the end and moving towards the start.
///
/// Each chunk is of size `alignment`, and starts on a multiple of `alignment` (that is, the lower
/// bound an _exclusive_ upper bound of each chunk are multiples of `alignment`), with the possible
/// exception of the first chunk (the last chunk in numerical order, which may be small) and the
/// last (which may be misaligned and small).
///
/// While the chunks are yielded in reverse order, from `end` to `start`, each individual chunk is
/// in the usual ascending order. That is, the first chunk ends with `end` and the last chunk starts
/// with `start`.
///
/// Note that unlike [`range_chunks_aligned`], which accepts any range and yields an infinite
/// iterator if the range has no upper bound, this function requires there to be a defined upper
/// bound, otherwise we don't know where the reversed iterator should _start_. The `end` bound given
/// here is inclusive; i.e. the end of the first chunk yielded by the stream will be exactly `end`.
fn range_chunks_aligned_rev(
    start: Bound<usize>,
    end: usize,
    alignment: usize,
) -> impl Iterator<Item = Range<usize>> {
    // Transform the start bound to be inclusive.
    let start = match start {
        Bound::Included(i) => i,
        Bound::Excluded(i) => i + 1,
        Bound::Unbounded => 0,
    };
    // Transform the end bound to be exclusive.
    let mut end = end + 1;

    // If necessary, generate a partial first chunk to force the remaining chunks into alignment.
    let first = if end.is_multiple_of(alignment) {
        None
    } else {
        // The partial first chunk starts at the previous multiple of the alignment, or at the start
        // of the overall range, whichever comes first.
        let next_multiple = end.next_multiple_of(alignment);
        let prev_multiple = next_multiple - alignment;
        let chunk_start = max(prev_multiple, start);
        let chunk = chunk_start..end;

        // Start the reverse series of aligned chunks at the start of the partial first chunk.
        end = chunk_start;
        Some(chunk)
    };

    first
        .into_iter()
        .chain(range_chunks_rev(Bound::Included(start), end - 1, alignment))
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
                tracing::info!(
                    "error loading resource from local storage, will try to fetch: {err:#}"
                );
                None
            },
        }
    }
}

#[derive(Debug)]
struct ScannerMetrics {
    /// Whether a scan is currently running (1) or not (0).
    running: Box<dyn Gauge>,
    /// The current number that is running.
    current_scan: Box<dyn Gauge>,
    /// Number of blocks processed in the current scan.
    scanned_blocks: Box<dyn Gauge>,
    /// Number of VID entries processed in the current scan.
    scanned_vid: Box<dyn Gauge>,
    /// The number of missing blocks discovered and not yet resolved in the current scan.
    missing_blocks: Box<dyn Gauge>,
    /// The number of missing VID entries discovered and not yet resolved in the current scan.
    missing_vid: Box<dyn Gauge>,
}

impl ScannerMetrics {
    fn new(metrics: &PrometheusMetrics) -> Self {
        let group = metrics.subgroup("scanner".into());
        Self {
            running: group.create_gauge("running".into(), None),
            current_scan: group.create_gauge("current".into(), None),
            scanned_blocks: group.create_gauge("scanned_blocks".into(), None),
            scanned_vid: group.create_gauge("scanned_vid".into(), None),
            missing_blocks: group.create_gauge("missing_blocks".into(), None),
            missing_vid: group.create_gauge("missing_vid".into(), None),
        }
    }
}

#[derive(Debug)]
struct AggregatorMetrics {
    /// The block height for which aggregate statistics are currently available.
    height: Box<dyn Gauge>,
}

impl AggregatorMetrics {
    fn new(metrics: &PrometheusMetrics) -> Self {
        let group = metrics.subgroup("aggregator".into());
        Self {
            height: group.create_gauge("height".into(), None),
        }
    }
}

#[derive(Debug)]
struct SyncStatusMetrics {
    current_range_start: Box<dyn Gauge>,
    current_range_end: Box<dyn Gauge>,
    current_start_time: Box<dyn Gauge>,
    avg_rate: Box<dyn Histogram>,
    ranges_scanned: Box<dyn Counter>,
    running: Box<dyn Gauge>,
}

impl SyncStatusMetrics {
    fn new(metrics: &PrometheusMetrics, size: usize) -> Self {
        let group = metrics.subgroup("sync_status".into());
        group.create_gauge("range_size".into(), None).set(size);

        Self {
            current_range_start: group.create_gauge("current_range_start".into(), None),
            current_range_end: group.create_gauge("current_range_end".into(), None),
            current_start_time: group
                .create_gauge("current_range_start_time".into(), Some("s".into())),
            avg_rate: group
                .create_histogram("avg_time_per_block_scanned".into(), Some("ms".into())),
            ranges_scanned: group.create_counter("ranges_scanned".into(), None),
            running: group.create_gauge("running".into(), None),
        }
    }

    fn start_range(&self, range: &Range<usize>) -> SyncStatusRangeMetrics<'_> {
        let start = Utc::now();
        self.current_range_start.set(range.start);
        self.current_range_end.set(range.end);
        self.current_start_time.set(start.timestamp() as usize);
        self.running.set(1);
        SyncStatusRangeMetrics {
            size: range.end - range.start,
            start,
            metrics: self,
        }
    }
}

#[must_use]
#[derive(Debug)]
struct SyncStatusRangeMetrics<'a> {
    size: usize,
    start: DateTime<Utc>,
    metrics: &'a SyncStatusMetrics,
}

impl<'a> SyncStatusRangeMetrics<'a> {
    fn end(self) {
        let elapsed = Utc::now() - self.start;
        self.metrics
            .avg_rate
            .add_point((elapsed.num_milliseconds() as f64) / (self.size as f64));
        self.metrics.ranges_scanned.add(1);
        self.metrics.running.set(0);
    }
}

#[derive(Debug)]
struct CachedSyncStatus {
    last_updated: Instant,
    ttl: Duration,
    cached: Option<SyncStatusQueryData>,
}

impl CachedSyncStatus {
    fn new(ttl: Duration) -> Self {
        Self {
            last_updated: Instant::now(),
            ttl,
            cached: None,
        }
    }

    /// Return the cached sync status, if present and fresh.
    fn try_get(&self) -> Option<&SyncStatusQueryData> {
        if self.last_updated.elapsed() > self.ttl {
            // Cached value is stale.
            return None;
        }
        self.cached.as_ref()
    }

    /// Refresh the cache with an updated value.
    fn update(&mut self, value: SyncStatusQueryData) {
        self.last_updated = Instant::now();
        self.cached = Some(value);
    }
}

/// Turn a fallible passive fetch future into an infallible "fetch".
///
/// Basically, we ignore failures due to a channel sender being dropped, which should never happen.
fn passive<T>(
    req: impl Debug + Send + 'static,
    fut: impl Future<Output = Option<T>> + Send + 'static,
) -> Fetch<T>
where
    T: Send + 'static,
{
    Fetch::Pending(
        fut.then(move |opt| async move {
            match opt {
                Some(t) => t,
                None => {
                    // If `passive_fetch` returns `None`, it means the notifier was dropped without
                    // ever sending a notification. In this case, the correct behavior is actually
                    // to block forever (unless the `Fetch` itself is dropped), since the semantics
                    // of `Fetch` are to never fail. This is analogous to fetching an object which
                    // doesn't actually exist: the `Fetch` will never return.
                    //
                    // However, for ease of debugging, and since this is never expected to happen in
                    // normal usage, we panic instead. This should only happen in two cases:
                    // * The server was shut down (dropping the notifier) without cleaning up some
                    //   background tasks. This will not affect runtime behavior, but should be
                    //   fixed if it happens.
                    // * There is a very unexpected runtime bug resulting in the notifier being
                    //   dropped. If this happens, things are very broken in any case, and it is
                    //   better to panic loudly than simply block forever.
                    panic!("notifier dropped without satisfying request {req:?}");
                },
            }
        })
        .boxed(),
    )
}

/// Get the result of the first future to return `Some`, if either do.
async fn select_some<T>(
    a: impl Future<Output = Option<T>> + Unpin,
    b: impl Future<Output = Option<T>> + Unpin,
) -> Option<T> {
    match future::select(a, b).await {
        // If the first future resolves with `Some`, immediately return the result.
        Either::Left((Some(a), _)) => Some(a),
        Either::Right((Some(b), _)) => Some(b),

        // If the first future resolves with `None`, wait for the result of the second future.
        Either::Left((None, b)) => b.await,
        Either::Right((None, a)) => a.await,
    }
}

#[cfg(test)]
mod test {
    use hotshot_example_types::node_types::TEST_VERSIONS;

    use super::*;
    use crate::{
        data_source::{
            sql::testing::TmpDb,
            storage::{SqlStorage, StorageConnectionType},
        },
        fetching::provider::NoFetching,
        testing::{consensus::MockSqlDataSource, mocks::MockTypes},
    };

    #[test]
    fn test_range_chunks() {
        // Inclusive bounds, partial last chunk.
        assert_eq!(
            range_chunks(0..=4, 2).collect::<Vec<_>>(),
            [0..2, 2..4, 4..5]
        );

        // Inclusive bounds, complete last chunk.
        assert_eq!(
            range_chunks(0..=5, 2).collect::<Vec<_>>(),
            [0..2, 2..4, 4..6]
        );

        // Exclusive bounds, partial last chunk.
        assert_eq!(
            range_chunks(0..5, 2).collect::<Vec<_>>(),
            [0..2, 2..4, 4..5]
        );

        // Exclusive bounds, complete last chunk.
        assert_eq!(
            range_chunks(0..6, 2).collect::<Vec<_>>(),
            [0..2, 2..4, 4..6]
        );

        // Unbounded.
        assert_eq!(
            range_chunks(0.., 2).take(5).collect::<Vec<_>>(),
            [0..2, 2..4, 4..6, 6..8, 8..10]
        );
    }

    #[test]
    fn test_range_chunks_aligned() {
        #![allow(clippy::single_range_in_vec_init)]

        // Aligned first chunk, partial last chunk.
        assert_eq!(
            range_chunks_aligned(2..5, 2).collect::<Vec<_>>(),
            [2..4, 4..5]
        );

        // Misaligned first chunk, complete last chunk.
        assert_eq!(
            range_chunks_aligned(1..4, 2).collect::<Vec<_>>(),
            [1..2, 2..4]
        );

        // Incomplete chunk.
        assert_eq!(range_chunks_aligned(1..3, 10).collect::<Vec<_>>(), [1..3]);

        // Unbounded.
        assert_eq!(
            range_chunks_aligned(1.., 2).take(5).collect::<Vec<_>>(),
            [1..2, 2..4, 4..6, 6..8, 8..10]
        );
    }

    #[test]
    fn test_range_chunks_rev() {
        // Inclusive bounds, partial last chunk.
        assert_eq!(
            range_chunks_rev(Bound::Included(0), 4, 2).collect::<Vec<_>>(),
            [3..5, 1..3, 0..1]
        );

        // Inclusive bounds, complete last chunk.
        assert_eq!(
            range_chunks_rev(Bound::Included(0), 5, 2).collect::<Vec<_>>(),
            [4..6, 2..4, 0..2]
        );

        // Exclusive bounds, partial last chunk.
        assert_eq!(
            range_chunks_rev(Bound::Excluded(0), 5, 2).collect::<Vec<_>>(),
            [4..6, 2..4, 1..2]
        );

        // Exclusive bounds, complete last chunk.
        assert_eq!(
            range_chunks_rev(Bound::Excluded(0), 4, 2).collect::<Vec<_>>(),
            [3..5, 1..3]
        );
    }

    #[test]
    fn test_range_chunks_aligned_rev() {
        #![allow(clippy::single_range_in_vec_init)]

        // Aligned first chunk, partial last chunk.
        assert_eq!(
            range_chunks_aligned_rev(Bound::Included(1), 3, 2).collect::<Vec<_>>(),
            [2..4, 1..2]
        );

        // Misaligned first chunk, complete last chunk.
        assert_eq!(
            range_chunks_aligned_rev(Bound::Included(0), 2, 2).collect::<Vec<_>>(),
            [2..3, 0..2]
        );

        // Incomplete chunk.
        assert_eq!(
            range_chunks_aligned_rev(Bound::Excluded(0), 3, 10).collect::<Vec<_>>(),
            [1..4]
        );
    }

    async fn test_sync_status(chunk_size: usize, present_ranges: &[(usize, usize)]) {
        let block_height = present_ranges.last().unwrap().1;
        let storage = TmpDb::init().await;
        let db = SqlStorage::connect(storage.config(), StorageConnectionType::Query)
            .await
            .unwrap();
        let ds = MockSqlDataSource::builder(db, NoFetching)
            .with_sync_status_chunk_size(chunk_size)
            .with_sync_status_ttl(Duration::ZERO)
            .build()
            .await
            .unwrap();

        // Generate some mock leaves to insert.
        let mut leaves: Vec<LeafQueryData<MockTypes>> = vec![
            LeafQueryData::<MockTypes>::genesis(
                &Default::default(),
                &Default::default(),
                TEST_VERSIONS.test,
            )
            .await,
        ];
        for i in 1..block_height {
            let mut leaf = leaves[i - 1].clone();
            leaf.leaf.block_header_mut().block_number = i as u64;
            leaves.push(leaf);
        }

        // Set up.
        {
            let mut tx = ds.write().await.unwrap();

            for &(start, end) in present_ranges {
                for leaf in &leaves[start..end] {
                    tracing::info!(height = leaf.height(), "insert leaf");
                    tx.insert_leaf(leaf).await.unwrap();
                }
            }

            if present_ranges[0].0 > 0 {
                tx.save_pruned_height((present_ranges[0].0 - 1) as u64)
                    .await
                    .unwrap();
            }

            tx.commit().await.unwrap();
        }

        let sync_status = ds.sync_status().await.unwrap().leaves;

        // Verify missing.
        let present: usize = present_ranges.iter().map(|(start, end)| end - start).sum();
        assert_eq!(
            sync_status.missing,
            block_height - present - present_ranges[0].0
        );

        // Verify ranges.
        let mut ranges = sync_status.ranges.into_iter();
        let mut prev = 0;
        for &(start, end) in present_ranges {
            if start != prev {
                let range = ranges.next().unwrap();
                assert_eq!(
                    range,
                    SyncStatusRange {
                        start: prev,
                        end: start,
                        status: if prev == 0 {
                            SyncStatus::Pruned
                        } else {
                            SyncStatus::Missing
                        },
                    }
                );
            }
            let range = ranges.next().unwrap();
            assert_eq!(
                range,
                SyncStatusRange {
                    start,
                    end,
                    status: SyncStatus::Present,
                }
            );
            prev = end;
        }

        if prev != block_height {
            let range = ranges.next().unwrap();
            assert_eq!(
                range,
                SyncStatusRange {
                    start: prev,
                    end: block_height,
                    status: SyncStatus::Missing,
                }
            );
        }

        assert_eq!(ranges.next(), None);
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_sync_status_multiple_chunks() {
        test_sync_status(10, &[(0, 1), (3, 5), (8, 10)]).await;
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_sync_status_multiple_chunks_present_range_overlapping_chunk() {
        test_sync_status(5, &[(1, 4)]).await;
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_sync_status_multiple_chunks_missing_range_overlapping_chunk() {
        test_sync_status(5, &[(0, 1), (4, 5)]).await;
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_load_range_incomplete() {
        let storage = TmpDb::init().await;
        let db = SqlStorage::connect(storage.config(), StorageConnectionType::Query)
            .await
            .unwrap();
        {
            let mut tx = db.write().await.unwrap();
            tx.insert_leaf(
                &LeafQueryData::<MockTypes>::genesis(
                    &Default::default(),
                    &Default::default(),
                    TEST_VERSIONS.test,
                )
                .await,
            )
            .await
            .unwrap();
            tx.insert_block(
                &BlockQueryData::<MockTypes>::genesis(
                    &Default::default(),
                    &Default::default(),
                    TEST_VERSIONS.test.base,
                )
                .await,
            )
            .await
            .unwrap();
            tx.insert_vid(
                &VidCommonQueryData::<MockTypes>::genesis(
                    &Default::default(),
                    &Default::default(),
                    TEST_VERSIONS.test.base,
                )
                .await,
                None,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
        }

        let mut tx = db.read().await.unwrap();
        let req = RangeRequest { start: 0, end: 100 };

        let err = <NonEmptyRange<BlockQueryData<MockTypes>>>::load(&mut tx, req)
            .await
            .unwrap_err();
        tracing::info!("loading partial block range failed as expected: {err:#}");
        assert!(matches!(err, QueryError::Missing));

        let err =
            <NonEmptyRange<LeafQueryData<MockTypes>> as Fetchable<MockTypes>>::load(&mut tx, req)
                .await
                .unwrap_err();
        tracing::info!("loading partial leaf range failed as expected: {err:#}");
        assert!(matches!(err, QueryError::Missing));

        let err = <NonEmptyRange<VidCommonQueryData<MockTypes>>>::load(&mut tx, req)
            .await
            .unwrap_err();
        tracing::info!("loading partial VID common range failed as expected: {err:#}");
        assert!(matches!(err, QueryError::Missing));
    }
}
