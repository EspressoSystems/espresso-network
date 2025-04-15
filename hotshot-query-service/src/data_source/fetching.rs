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

use std::{
    cmp::{max, min},
    fmt::{Debug, Display},
    iter::repeat_with,
    marker::PhantomData,
    ops::{Bound, Range, RangeBounds},
    sync::Arc,
    time::Duration,
};

use anyhow::{bail, Context};
use async_lock::Semaphore;
use async_trait::async_trait;
use backoff::{backoff::Backoff, ExponentialBackoff, ExponentialBackoffBuilder};
use derivative::Derivative;
use futures::{
    channel::oneshot,
    future::{self, join_all, BoxFuture, Either, Future, FutureExt},
    stream::{self, BoxStream, StreamExt},
};
use hotshot_types::{
    data::VidShare,
    traits::{
        metrics::{Gauge, Metrics},
        node_implementation::NodeType,
    },
};
use jf_merkle_tree::{prelude::MerkleProof, MerkleTreeScheme};
use tagged_base64::TaggedBase64;
use tokio::{spawn, time::sleep};
use tracing::Instrument;

use super::{
    notifier::Notifier,
    storage::{
        pruning::{PruneStorage, PrunedHeightDataSource, PrunedHeightStorage},
        sql::MigrateTypes,
        Aggregate, AggregatesStorage, AvailabilityStorage, ExplorerStorage,
        MerklizedStateHeightStorage, MerklizedStateStorage, NodeStorage, UpdateAggregatesStorage,
        UpdateAvailabilityStorage,
    },
    Transaction, VersionedDataSource,
};
use crate::{
    availability::{
        AvailabilityDataSource, BlockId, BlockInfo, BlockQueryData, Fetch, FetchStream,
        HeaderQueryData, LeafId, LeafQueryData, PayloadMetadata, PayloadQueryData, QueryableHeader,
        QueryablePayload, StateCertQueryData, TransactionHash, TransactionQueryData,
        UpdateAvailabilityData, VidCommonMetadata, VidCommonQueryData,
    },
    explorer::{self, ExplorerDataSource},
    fetching::{
        self,
        request::{self, StateCertRequest},
        Provider,
    },
    merklized_state::{
        MerklizedState, MerklizedStateDataSource, MerklizedStateHeightPersistence, Snapshot,
    },
    metrics::PrometheusMetrics,
    node::{NodeDataSource, SyncStatus, TimeWindowQueryData, WindowStart},
    status::{HasMetrics, StatusDataSource},
    task::BackgroundTask,
    types::HeightIndexed,
    Header, Payload, QueryError, QueryResult,
};

mod block;
mod header;
mod leaf;
mod state_cert;
mod transaction;
mod vid;

use self::{
    block::PayloadFetcher,
    leaf::LeafFetcher,
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
    minor_scan_interval: Duration,
    major_scan_interval: usize,
    major_scan_offset: usize,
    proactive_range_chunk_size: Option<usize>,
    active_fetch_delay: Duration,
    chunk_fetch_delay: Duration,
    proactive_fetching: bool,
    aggregator: bool,
    aggregator_chunk_size: Option<usize>,
    types_migration_batch_size: u64,
    leaf_only: bool,
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
            // By default, we run minor proactive scans fairly frequently: once every minute. These
            // scans are cheap (more the more frequently they run) and can help us keep up with
            // the head of the chain even if our attached consensus instance is behind.
            minor_scan_interval: Duration::from_secs(60),
            // Major scans, on the other hand, are rather expensive and not especially important for
            // usability. We run them rarely, once every 60 minor scans, or once every hour by
            // default.
            major_scan_interval: 60,
            // Major scan offset can be used when starting multiple nodes at the same time, so they
            // don't all pause for a major scan together.
            major_scan_offset: 0,
            proactive_range_chunk_size: None,
            active_fetch_delay: Duration::from_millis(50),
            chunk_fetch_delay: Duration::from_millis(100),
            proactive_fetching: true,
            aggregator: true,
            aggregator_chunk_size: None,
            types_migration_batch_size: 10000,
            leaf_only: false,
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

    /// Set the offset (denominated in [minor scans](Self::with_minor_scan_interval)) before the
    /// first major proactive fetching scan.
    ///
    /// This is useful when starting multiple nodes at the same time: major proactive scans can have
    /// a measurable impact on the performance of the node for a brief time while the scan is
    /// running, so it may be desirable to prevent a group of nodes from all doing major scans at
    /// the same time. This can be achieved by giving each node a different `major_scan_offset`.
    ///
    /// See also [proactive fetching](self#proactive-fetching).
    pub fn with_major_scan_offset(mut self, offset: usize) -> Self {
        self.major_scan_offset = offset;
        self
    }

    /// Set the number of items to process at a time when scanning for proactive fetching.
    ///
    /// This is similar to [`Self::with_range_chunk_size`], but only affects the chunk size for
    /// proactive fetching scans, not for normal subscription streams. This can be useful to tune
    /// the proactive scanner to be more or less greedy with persistent storage resources.
    ///
    /// By default (i.e. if this method is not called) the proactive range chunk size will be set to
    /// whatever the normal range chunk size is.
    pub fn with_proactive_range_chunk_size(mut self, range_chunk_size: usize) -> Self {
        self.proactive_range_chunk_size = Some(range_chunk_size);
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

    /// Sets the batch size for the types migration.
    /// Determines how many `(leaf, vid)` rows are selected from the old types table
    /// and migrated at once.
    pub fn with_types_migration_batch_size(mut self, batch: u64) -> Self {
        self.types_migration_batch_size = batch;
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
    S: PruneStorage + VersionedDataSource + HasMetrics + MigrateTypes<Types> + 'static,
    for<'a> S::ReadOnly<'a>:
        AvailabilityStorage<Types> + PrunedHeightStorage + NodeStorage<Types> + AggregatesStorage,
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
    Payload<Types>: QueryablePayload<Types>,
    S: PruneStorage + Send + Sync + 'static,
{
    async fn new(storage: Arc<S>) -> Self {
        let cfg = storage.get_pruning_config();
        let Some(cfg) = cfg else {
            return Self {
                handle: None,
                _types: Default::default(),
            };
        };

        let future = async move {
            for i in 1.. {
                tracing::warn!("starting pruner run {i} ");
                Self::prune(storage.clone()).await;
                sleep(cfg.interval()).await;
            }
        };

        let task = BackgroundTask::spawn("pruner", future);

        Self {
            handle: Some(task),
            _types: Default::default(),
        }
    }

    async fn prune(storage: Arc<S>) {
        // We loop until the whole run pruner run is complete
        let mut pruner = S::Pruner::default();
        loop {
            match storage.prune(&mut pruner).await {
                Ok(Some(height)) => {
                    tracing::warn!("Pruned to height {height}");
                },
                Ok(None) => {
                    tracing::warn!("pruner run complete.");
                    break;
                },
                Err(e) => {
                    tracing::error!("pruner run failed: {e:?}");
                    break;
                },
            }
        }
    }
}

impl<Types, S, P> FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
    Header<Types>: QueryableHeader<Types>,
    S: VersionedDataSource + PruneStorage + HasMetrics + MigrateTypes<Types> + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types> + UpdateAggregatesStorage<Types>,
    for<'a> S::ReadOnly<'a>:
        AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage + AggregatesStorage,
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
        let minor_interval = builder.minor_scan_interval;
        let major_interval = builder.major_scan_interval;
        let major_offset = builder.major_scan_offset;
        let proactive_range_chunk_size = builder
            .proactive_range_chunk_size
            .unwrap_or(builder.range_chunk_size);
        let migration_batch_size = builder.types_migration_batch_size;
        let scanner_metrics = ScannerMetrics::new(builder.storage.metrics());
        let aggregator_metrics = AggregatorMetrics::new(builder.storage.metrics());

        let fetcher = Arc::new(Fetcher::new(builder).await?);

        // Migrate the old types to new PoS types
        // This is a one-time operation that should be done before starting the data source
        // It migrates leaf1 storage to leaf2
        // and vid to vid2
        fetcher.storage.migrate_types(migration_batch_size).await?;

        let scanner = if proactive_fetching && !leaf_only {
            Some(BackgroundTask::spawn(
                "proactive scanner",
                fetcher.clone().proactive_scan(
                    minor_interval,
                    major_interval,
                    major_offset,
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

        let pruner = Pruner::new(storage).await;
        let ds = Self {
            fetcher,
            scanner,
            pruner,
            aggregator,
        };

        Ok(ds)
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

    async fn get_transaction(
        &self,
        hash: TransactionHash<Types>,
    ) -> Fetch<TransactionQueryData<Types>> {
        self.fetcher.get(TransactionRequest::from(hash)).await
    }

    async fn get_state_cert(&self, epoch: u64) -> Fetch<StateCertQueryData<Types>> {
        self.fetcher.get(StateCertRequest::from(epoch)).await
    }
}

impl<Types, S, P> UpdateAvailabilityData<Types> for FetchingDataSource<Types, S, P>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    async fn append(&self, info: BlockInfo<Types>) -> anyhow::Result<()> {
        let height = info.height() as usize;
        let fetch_block = info.block.is_none();
        let fetch_vid = info.vid_common.is_none();

        // Trigger a fetch of the parent leaf, if we don't already have it.
        leaf::trigger_fetch_for_parent(&self.fetcher, &info.leaf);

        self.fetcher.store_and_notify(info).await;

        if fetch_block || fetch_vid {
            // If data related to this block is missing, try and fetch it. Do this in an async task:
            // we're triggering a fire-and-forget fetch; we don't need to block the caller on this.
            let fetcher = self.fetcher.clone();
            let span = tracing::info_span!("fetch missing data", height);
            spawn(
                async move {
                    tracing::info!(fetch_block, fetch_vid, "fetching missing data");
                    if fetch_block {
                        fetcher.get::<PayloadMetadata<Types>>(height).await;
                    }
                    if fetch_vid {
                        fetcher.get::<VidCommonMetadata<Types>>(height).await;
                    }
                }
                .instrument(span),
            );
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
    payload_fetcher: Option<Arc<PayloadFetcher<Types, S, P>>>,
    vid_common_fetcher: Option<Arc<VidCommonFetcher<Types, S, P>>>,
    range_chunk_size: usize,
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
    S: VersionedDataSource + Sync,
    for<'a> S::ReadOnly<'a>: PrunedHeightStorage + NodeStorage<Types>,
{
    pub async fn new(builder: Builder<Types, S, P>) -> anyhow::Result<Self> {
        let retry_semaphore = Arc::new(Semaphore::new(builder.rate_limit));
        let backoff = builder.backoff.build();

        let (payload_fetcher, vid_fetcher) = if builder.is_leaf_only() {
            (None, None)
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
            )
        };
        let leaf_fetcher = fetching::Fetcher::new(retry_semaphore.clone(), backoff.clone());

        let leaf_only = builder.leaf_only;

        Ok(Self {
            storage: Arc::new(builder.storage),
            notifiers: Default::default(),
            provider: Arc::new(builder.provider),
            leaf_fetcher: Arc::new(leaf_fetcher),
            payload_fetcher,
            vid_common_fetcher: vid_fetcher,
            range_chunk_size: builder.range_chunk_size,
            active_fetch_delay: builder.active_fetch_delay,
            chunk_fetch_delay: builder.chunk_fetch_delay,
            backoff,
            retry_semaphore,
            leaf_only,
        })
    }
}

impl<Types, S, P> Fetcher<Types, S, P>
where
    Types: NodeType,
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
        minor_interval: Duration,
        major_interval: usize,
        major_offset: usize,
        chunk_size: usize,
        metrics: ScannerMetrics,
    ) {
        let mut prev_height = 0;

        for i in 0.. {
            let major = i % major_interval == major_offset % major_interval;
            let span = tracing::warn_span!("proactive scan", i, major, prev_height);
            metrics.running.set(1);
            metrics.current_scan.set(i);
            metrics.current_is_major.set(major as usize);
            async {
                let mut backoff = minor_interval;
                let max_backoff = Duration::from_secs(60);
                metrics.backoff.set(backoff.as_secs() as usize);

                // We can't start the scan until we know the current block height and pruned height,
                // so we know which blocks to scan. Thus we retry until this succeeds.
                let heights = loop {
                    let mut tx = match self.read().await {
                        Ok(tx) => tx,
                        Err(err) => {
                            tracing::error!(
                                ?backoff,
                                "unable to start transaction for scan: {err:#}"
                            );
                            metrics.retries.update(1);
                            sleep(backoff).await;
                            backoff = min(2 * backoff, max_backoff);
                            metrics.backoff.set(backoff.as_secs() as usize);
                            continue;
                        },
                    };
                    let heights = match Heights::load(&mut tx).await {
                        Ok(heights) => heights,
                        Err(err) => {
                            tracing::error!(?backoff, "unable to load heights: {err:#}");
                            metrics.retries.update(1);
                            sleep(backoff).await;
                            backoff = min(2 * backoff, max_backoff);
                            metrics.backoff.set(backoff.as_secs() as usize);
                            continue;
                        },
                    };
                    metrics.retries.set(0);
                    break heights;
                };

                // Get the pruned height or default to 0 if it is not set. We will not look for
                // blocks older than the pruned height.
                let minimum_block_height = heights.pruned_height.unwrap_or(0) as usize;
                // Get the block height; we will look for any missing blocks up to `block_height`.
                let block_height = heights.height as usize;

                // In a major scan, we fetch all blocks between 0 and `block_height`. In minor scans
                // (much more frequent) we fetch blocks that are missing since the last scan.
                let start = if major {
                    // We log major scans at WARN level, since they happen infrequently and can have
                    // a measurable impact on performance while running. This also serves as a
                    // useful progress heartbeat.
                    tracing::warn!(
                        start = minimum_block_height,
                        block_height,
                        "starting major scan"
                    );

                    // If we're starting a major scan, reset the major scan counts of missing data
                    // as we're about to recompute them.
                    metrics.major_missing_blocks.set(0);
                    metrics.major_missing_vid.set(0);

                    minimum_block_height
                } else {
                    tracing::info!(start = prev_height, block_height, "starting minor scan");
                    prev_height
                };
                prev_height = block_height;
                metrics.current_start.set(start);
                metrics.current_end.set(block_height);
                metrics.scanned_blocks.set(0);
                metrics.scanned_vid.set(0);

                // Iterate over all blocks that we should have. Fetching the payload metadata is
                // enough to trigger an active fetch of the corresponding leaf and the full block if
                // they are missing, without loading the full payload from storage if we already
                // have it.
                //
                // We iterate in reverse order for two reasons:
                // 1. More recent blocks are more likely to be requested sooner.
                // 2. Leaves are inherently fetched in reverse order, because we cannot (actively)
                //    fetch a leaf until we have the subsequent leaf, which tells us what the hash
                //    of its parent should be. Thus, if we iterated forwards, we could get stuck any
                //    time we came upon two consecutive missing leaves.
                let mut blocks = self
                    .clone()
                    .get_range_with_chunk_size_rev::<PayloadMetadata<Types>>(
                        chunk_size,
                        Bound::Included(start),
                        block_height.saturating_sub(1),
                    );
                let mut missing_blocks = 0;
                while let Some(fetch) = blocks.next().await {
                    if fetch.is_pending() {
                        missing_blocks += 1;
                        // Wait for the block to be fetched. This slows down the scanner so that we
                        // don't waste memory generating more active fetch tasks then we can handle
                        // at a given time. Note that even with this await, all blocks within a
                        // chunk are fetched in parallel, so this does not block the next block in
                        // the chunk, only the next chunk until the current chunk completes.
                        fetch.await;
                    }
                    metrics.scanned_blocks.update(1);
                }
                metrics.add_missing_blocks(major, missing_blocks);

                // We have to trigger a separate fetch of the VID data, since this is fetched
                // independently of the block payload.
                let mut vid = self
                    .clone()
                    .get_range_with_chunk_size_rev::<VidCommonMetadata<Types>>(
                        chunk_size,
                        Bound::Included(start),
                        block_height.saturating_sub(1),
                    );
                let mut missing_vid = 0;
                while let Some(fetch) = vid.next().await {
                    if fetch.is_pending() {
                        missing_vid += 1;
                        // As above, limit the speed at which we spawn new fetches to the speed at
                        // which we can process them.
                        fetch.await;
                    }
                    metrics.scanned_vid.update(1);
                }
                metrics.add_missing_vid(major, missing_vid);

                tracing::info!("completed proactive scan, will scan again in {minor_interval:?}");

                // Reset metrics.
                metrics.running.set(0);
                if major {
                    // If we just completed a major scan, reset the incremental counts of missing
                    // data from minor scans, so the next round of minor scans can recompute/update
                    // them.
                    metrics.minor_missing_blocks.set(0);
                    metrics.minor_missing_vid.set(0);
                }

                sleep(minor_interval).await;
            }
            .instrument(span)
            .await;
        }
    }
}

impl<Types, S, P> Fetcher<Types, S, P>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types> + UpdateAggregatesStorage<Types>,
    for<'a> S::ReadOnly<'a>:
        AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage + AggregatesStorage,
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
    async fn store_and_notify<T>(&self, obj: T)
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
                "failed to store fetched {} {}: {err:#}",
                T::name(),
                obj.height()
            );

            let Some(delay) = backoff.next_backoff() else {
                break;
            };
            tracing::info!(?delay, "retrying failed operation");
            sleep(delay).await;
        }

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
}

#[derive(Debug)]
struct Notifiers<Types>
where
    Types: NodeType,
{
    block: Notifier<BlockQueryData<Types>>,
    leaf: Notifier<LeafQueryData<Types>>,
    vid_common: Notifier<VidCommonQueryData<Types>>,
    state_cert: Notifier<StateCertQueryData<Types>>,
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
            state_cert: Notifier::new(),
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
    S: VersionedDataSource + 'static,
    for<'a> S::ReadOnly<'a>: NodeStorage<Types>,
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
    ) -> QueryResult<usize> {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        tx.count_transactions_in_range(range).await
    }

    async fn payload_size_in_range(
        &self,
        range: impl RangeBounds<usize> + Send,
    ) -> QueryResult<usize> {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        tx.payload_size_in_range(range).await
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

    async fn sync_status(&self) -> QueryResult<SyncStatus> {
        let mut tx = self.read().await.map_err(|err| QueryError::Error {
            message: err.to_string(),
        })?;
        tx.sync_status().await
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
    crate::Transaction<Types>: explorer::traits::ExplorerTransaction,
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
    + Provider<Types, request::PayloadRequest>
    + Provider<Types, request::VidCommonRequest>
    + Sync
    + 'static
{
}
impl<Types: NodeType, P> AvailabilityProvider<Types> for P where
    P: Provider<Types, request::LeafRequest<Types>>
        + Provider<Types, request::PayloadRequest>
        + Provider<Types, request::VidCommonRequest>
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
trait Storable<Types: NodeType>: HeightIndexed + Clone {
    /// The name of this type of object, for debugging purposes.
    fn name() -> &'static str;

    /// Notify anyone waiting for this object that it has become available.
    fn notify(&self, notifiers: &Notifiers<Types>) -> impl Send + Future<Output = ()>;

    /// Store the object in the local database.
    fn store(
        self,
        storage: &mut (impl UpdateAvailabilityStorage<Types> + Send),
        leaf_only: bool,
    ) -> impl Send + Future<Output = anyhow::Result<()>>;
}

impl<Types: NodeType> Storable<Types> for BlockInfo<Types> {
    fn name() -> &'static str {
        "block info"
    }

    async fn notify(&self, notifiers: &Notifiers<Types>) {
        self.leaf.notify(notifiers).await;

        if let Some(block) = &self.block {
            block.notify(notifiers).await;
        }
        if let Some(vid) = &self.vid_common {
            vid.notify(notifiers).await;
        }
    }

    async fn store(
        self,
        storage: &mut (impl UpdateAvailabilityStorage<Types> + Send),
        leaf_only: bool,
    ) -> anyhow::Result<()> {
        self.leaf.store(storage, leaf_only).await?;

        if let Some(common) = self.vid_common {
            (common, self.vid_share).store(storage, leaf_only).await?;
        }

        if let Some(block) = self.block {
            block.store(storage, leaf_only).await?;
        }

        if let Some(state_cert) = self.state_cert {
            state_cert.store(storage, leaf_only).await?;
        }

        Ok(())
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

/// Break a range into fixed-size chunks, starting from the end and moving towards the start.
///
/// While the chunks are yielded in reverse order, from `end` to `start`, each individual chunk is
/// in the usual ascending order. That is, the first chunk ends with `end` and the last chunk starts
/// with `start`.
///
/// Note that unlike [`range_chunks_rev`], which accepts any range and yields an infinite iterator
/// if the range has no upper bound, this function requires there to be a defined upper bound,
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
    /// Whether the currently running scan is a major scan (1) or not (0).
    current_is_major: Box<dyn Gauge>,
    /// Current backoff delay for retries (s).
    backoff: Box<dyn Gauge>,
    /// Number of retries since last success.
    retries: Box<dyn Gauge>,
    /// Block height where the current scan started.
    current_start: Box<dyn Gauge>,
    /// Block height where the current scan will end.
    current_end: Box<dyn Gauge>,
    /// Number of blocks processed in the current scan.
    scanned_blocks: Box<dyn Gauge>,
    /// Number of VID entries processed in the current scan.
    scanned_vid: Box<dyn Gauge>,
    /// The number of missing blocks encountered in the last major scan.
    major_missing_blocks: Box<dyn Gauge>,
    /// The number of missing VID entries encountered in the last major scan.
    major_missing_vid: Box<dyn Gauge>,
    /// The number of missing blocks encountered _cumulatively_ in minor scans since the last major
    /// scan.
    minor_missing_blocks: Box<dyn Gauge>,
    /// The number of missing VID entries encountered _cumulatively_ in minor scans since the last
    /// major scan.
    minor_missing_vid: Box<dyn Gauge>,
}

impl ScannerMetrics {
    fn new(metrics: &PrometheusMetrics) -> Self {
        let group = metrics.subgroup("scanner".into());
        Self {
            running: group.create_gauge("running".into(), None),
            current_scan: group.create_gauge("current".into(), None),
            current_is_major: group.create_gauge("is_major".into(), None),
            backoff: group.create_gauge("backoff".into(), Some("s".into())),
            retries: group.create_gauge("retries".into(), None),
            current_start: group.create_gauge("start".into(), None),
            current_end: group.create_gauge("end".into(), None),
            scanned_blocks: group.create_gauge("scanned_blocks".into(), None),
            scanned_vid: group.create_gauge("scanned_vid".into(), None),
            major_missing_blocks: group.create_gauge("major_missing_blocks".into(), None),
            major_missing_vid: group.create_gauge("major_missing_vid".into(), None),
            minor_missing_blocks: group.create_gauge("minor_missing_blocks".into(), None),
            minor_missing_vid: group.create_gauge("minor_missing_vid".into(), None),
        }
    }

    fn add_missing_blocks(&self, major: bool, missing: usize) {
        if major {
            self.major_missing_blocks.set(missing);
        } else {
            self.minor_missing_blocks.update(missing as i64);
        }
    }

    fn add_missing_vid(&self, major: bool, missing: usize) {
        if major {
            self.major_missing_vid.set(missing);
        } else {
            self.minor_missing_vid.update(missing as i64);
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
    use super::*;

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
}
