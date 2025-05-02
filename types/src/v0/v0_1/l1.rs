use alloy::{
    network::Ethereum,
    primitives::{B256, U256},
    providers::{
        fillers::{FillProvider, JoinFill, RecommendedFillers},
        Identity, RootProvider,
    },
    transports::http::{Client, Http},
};
use alloy_compat::ethers_serde;
use async_broadcast::{InactiveReceiver, Sender};
use clap::Parser;
use derive_more::Deref;
use hotshot_types::traits::metrics::{Counter, Gauge, Metrics, NoMetrics};
use lru::LruCache;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{
    num::NonZeroUsize,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    sync::{Mutex, Notify},
    task::JoinHandle,
};
use url::Url;

use crate::v0::utils::parse_duration;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Hash, PartialEq, Eq)]
pub struct L1BlockInfo {
    pub number: u64,
    #[serde(with = "ethers_serde::u256")]
    pub timestamp: U256,
    #[serde(with = "ethers_serde::b256")]
    pub hash: B256,
}

#[derive(Clone, Copy, Debug, PartialOrd, Ord, Hash, PartialEq, Eq)]
pub(crate) struct L1BlockInfoWithParent {
    pub(crate) info: L1BlockInfo,
    pub(crate) parent_hash: B256,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Hash, PartialEq, Eq)]
pub struct L1Snapshot {
    /// The relevant snapshot of the L1 includes a reference to the current head of the L1 chain.
    ///
    /// Note that the L1 head is subject to changing due to a reorg. However, no reorg will change
    /// the _number_ of this block in the chain: L1 block numbers will always be sequentially
    /// increasing. Therefore, the sequencer does not have to worry about reorgs invalidating this
    /// snapshot.
    pub head: u64,

    /// The snapshot also includes information about the latest finalized L1 block.
    ///
    /// Since this block is finalized (ie cannot be reorged) we can include specific information
    /// about the particular block, such as its hash and timestamp.
    ///
    /// This block may be `None` in the rare case where Espresso has started shortly after the
    /// genesis of the L1, and the L1 has yet to finalize a block. In all other cases it will be
    /// `Some`.
    pub finalized: Option<L1BlockInfo>,
}

/// Configuration for an L1 client.
#[derive(Clone, Debug, Parser)]
pub struct L1ClientOptions {
    /// Delay when retrying failed L1 queries.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_L1_RETRY_DELAY",
        default_value = "1s",
        value_parser = parse_duration,
    )]
    pub l1_retry_delay: Duration,

    /// Request rate when polling L1.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_L1_POLLING_INTERVAL",
        default_value = "7s",
        value_parser = parse_duration,
    )]
    pub l1_polling_interval: Duration,

    /// Maximum number of L1 blocks to keep in cache at once.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_L1_BLOCKS_CACHE_SIZE",
        default_value = "100"
    )]
    pub l1_blocks_cache_size: NonZeroUsize,

    /// Number of L1 events to buffer before discarding.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_L1_EVENTS_CHANNEL_CAPACITY",
        default_value = "100"
    )]
    pub l1_events_channel_capacity: usize,

    /// Maximum number of L1 blocks that can be scanned for events in a single query.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_L1_EVENTS_MAX_BLOCK_RANGE",
        default_value = "10000"
    )]
    pub l1_events_max_block_range: u64,

    /// Maximum time to wait for new heads before considering a stream invalid and reconnecting.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_L1_SUBSCRIPTION_TIMEOUT",
        default_value = "1m",
        value_parser = parse_duration,
    )]
    pub subscription_timeout: Duration,

    /// Fail over to another provider if the current provider fails twice within this window.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_L1_FREQUENT_FAILURE_TOLERANCE",
        default_value = "1m",
        value_parser = parse_duration,
    )]
    pub l1_frequent_failure_tolerance: Duration,

    /// Fail over to another provider if the current provider fails many times in a row, within any
    /// time window.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_L1_CONSECUTIVE_FAILURE_TOLERANCE",
        default_value = "10"
    )]
    pub l1_consecutive_failure_tolerance: usize,

    /// Amount of time to wait after receiving a 429 response before making more L1 RPC requests.
    ///
    /// If not set, the general l1-retry-delay will be used.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_L1_RATE_LIMIT_DELAY",
        value_parser = parse_duration,
    )]
    pub l1_rate_limit_delay: Option<Duration>,

    /// Separate provider to use for subscription feeds.
    ///
    /// Typically this would be a WebSockets endpoint while the main provider uses HTTP.
    #[clap(long, env = "ESPRESSO_SEQUENCER_L1_WS_PROVIDER", value_delimiter = ',')]
    pub l1_ws_provider: Option<Vec<Url>>,

    /// Interval at which the background update loop polls the L1 stake table contract for new events
    /// and updates local persistence.
    ///
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_L1_STAKE_TABLE_UPDATE_INTERVAL",
        default_value = "60m",
        value_parser = parse_duration,
    )]
    pub stake_table_update_interval: Duration,

    /// A block range which is expected to contain the finalized heads of all L1 provider chains.
    ///
    /// If specified, it is assumed that if a block `n` is known to be finalized according to a
    /// certain provider, then any block less than `n - L1_FINALIZED_SAFETY_MARGIN` is finalized
    /// _according to any provider_. In other words, if we fail over from one provider to another,
    /// the second provider will never be lagging the first by more than this margin.
    ///
    /// This allows us to quickly query for very old finalized blocks by number. Without this
    /// assumption, we always need to verify that a block is finalized by fetching all blocks in a
    /// hash chain between the known finalized block and the desired block, recomputing and checking
    /// the hashes. This is fine and good for blocks very near the finalized head, but for
    /// extremely old blocks it is prohibitively expensive, and these old blocks are extremely
    /// unlikely to be unfinalized anyways.
    #[clap(long, env = "ESPRESSO_SEQUENCER_L1_FINALIZED_SAFETY_MARGIN")]
    pub l1_finalized_safety_margin: Option<u64>,

    #[clap(skip = Arc::<Box<dyn Metrics>>::new(Box::new(NoMetrics)))]
    pub metrics: Arc<Box<dyn Metrics>>,
}

/// Type alias for alloy provider
pub type L1Provider = FillProvider<
    JoinFill<Identity, <Ethereum as RecommendedFillers>::RecommendedFillers>,
    RootProvider,
>;

#[derive(Clone, Debug, Deref)]
/// An Ethereum provider and configuration to interact with the L1.
///
/// This client runs asynchronously, updating an in-memory snapshot of the relevant L1 information
/// each time a new L1 block is published. The main advantage of this is that we can update the L1
/// state at the pace of the L1, instead of the much faster pace of HotShot consensus.This makes it
/// easy to use a subscription instead of polling for new blocks, vastly reducing the number of L1
/// RPC calls we make.
pub struct L1Client {
    /// The alloy provider used for L1 communication with wallet and default fillers
    #[deref]
    pub provider: L1Provider,
    /// Actual transport used in `self.provider`
    /// i.e. the `t` variable in `ProviderBuilder::new().on_client(RpcClient::new(t, is_local))`
    pub transport: SwitchingTransport,
    /// Shared state updated by an asynchronous task which polls the L1.
    pub(crate) state: Arc<Mutex<L1State>>,
    /// Channel used by the async update task to send events to clients.
    pub(crate) sender: Sender<L1Event>,
    /// Receiver for events from the async update task.
    pub(crate) receiver: InactiveReceiver<L1Event>,
    /// Async task which updates the shared state.
    pub(crate) update_task: Arc<L1UpdateTask>,
}

/// In-memory view of the L1 state, updated asynchronously.
#[derive(Debug)]
pub(crate) struct L1State {
    pub(crate) snapshot: L1Snapshot,
    pub(crate) finalized: LruCache<u64, L1BlockInfoWithParent>,
    pub(crate) last_finalized: Option<u64>,
}

#[derive(Clone, Debug)]
pub(crate) enum L1Event {
    NewHead { head: u64 },
    NewFinalized { finalized: L1BlockInfoWithParent },
}

#[derive(Debug, Default)]
pub(crate) struct L1UpdateTask(pub(crate) Mutex<Option<JoinHandle<()>>>);

#[derive(Clone, Debug)]
pub(crate) struct L1ClientMetrics {
    pub(crate) head: Arc<dyn Gauge>,
    pub(crate) finalized: Arc<dyn Gauge>,
    pub(crate) reconnects: Arc<dyn Counter>,
    pub(crate) failovers: Arc<dyn Counter>,
    pub(crate) failures: Arc<Vec<Box<dyn Counter>>>,
}

/// An RPC client with multiple remote (HTTP) providers.
///
/// This client utilizes one RPC provider at a time, but if it detects that the provider is in a
/// failing state, it will automatically switch to the next provider in its list.
#[derive(Clone, Debug)]
pub struct SwitchingTransport {
    /// The transport currently being used by the client
    pub(crate) current_transport: Arc<RwLock<SingleTransport>>,
    /// The list of configured HTTP URLs to use for RPC requests
    pub(crate) urls: Arc<Vec<Url>>,
    pub(crate) opt: Arc<L1ClientOptions>,
    pub(crate) metrics: L1ClientMetrics,
    pub(crate) switch_notify: Arc<Notify>,
}

/// The state of the current provider being used by a [`SwitchingTransport`].
/// This is cloneable and returns a reference to the same underlying data.
#[derive(Debug, Clone)]
pub(crate) struct SingleTransport {
    pub(crate) generation: usize,
    pub(crate) client: Http<Client>,
    pub(crate) status: Arc<RwLock<SingleTransportStatus>>,
}

/// The status of a single transport
#[derive(Debug, Default)]
pub(crate) struct SingleTransportStatus {
    pub(crate) last_failure: Option<Instant>,
    pub(crate) consecutive_failures: usize,
    pub(crate) rate_limited_until: Option<Instant>,
    /// Whether or not this current transport is being shut down (switching to the next transport)
    pub(crate) shutting_down: bool,
}
