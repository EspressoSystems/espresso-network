// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Networking Implementation that has a primary and a fallback network.  If the primary
//! Errors we will use the backup to send or receive
use std::{
    collections::{BTreeMap, HashMap},
    future::Future,
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use async_broadcast::{broadcast, InactiveReceiver, Sender};
use async_lock::RwLock;
use async_trait::async_trait;
use futures::{join, select, FutureExt};
#[cfg(feature = "hotshot-testing")]
use hotshot_types::traits::network::{
    AsyncGenerator, NetworkReliability, TestableNetworkingImplementation,
};
use hotshot_types::{
    boxed_sync,
    constants::{
        COMBINED_NETWORK_CACHE_SIZE, COMBINED_NETWORK_DELAY_DURATION,
        COMBINED_NETWORK_MIN_PRIMARY_FAILURES, COMBINED_NETWORK_PRIMARY_CHECK_INTERVAL,
    },
    data::ViewNumber,
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        network::{BroadcastDelay, ConnectedNetwork, Topic},
        node_implementation::NodeType,
    },
    BoxSyncFuture,
};
use lru::LruCache;
use parking_lot::RwLock as PlRwLock;
use tokio::{spawn, sync::mpsc::error::TrySendError, time::sleep};
use tracing::{debug, info, warn};

use super::{push_cdn_network::PushCdnNetwork, NetworkError};
use crate::traits::implementations::Libp2pNetwork;

/// Thread-safe ref counted lock to a map of channels to the delayed tasks
type DelayedTasksChannelsMap = Arc<RwLock<BTreeMap<u64, (Sender<()>, InactiveReceiver<()>)>>>;

/// A communication channel with 2 networks, where we can fall back to the slower network if the
/// primary fails
#[derive(Clone)]
pub struct CombinedNetworks<TYPES: NodeType> {
    /// The two networks we'll use for send/recv
    networks: Arc<UnderlyingCombinedNetworks<TYPES>>,

    /// The message deduplication cache that we use to prevent duplicate messages between
    /// sources
    message_deduplication_cache: Arc<PlRwLock<MessageDeduplicationCache>>,

    /// How many times primary failed to deliver
    primary_fail_counter: Arc<AtomicU64>,

    /// Whether primary is considered down
    primary_down: Arc<AtomicBool>,

    /// How long to delay
    delay_duration: Arc<RwLock<Duration>>,

    /// Channels to the delayed tasks
    delayed_tasks_channels: DelayedTasksChannelsMap,

    /// How many times messages were sent on secondary without delay because primary is down
    no_delay_counter: Arc<AtomicU64>,
}

impl<TYPES: NodeType> CombinedNetworks<TYPES> {
    /// Constructor
    ///
    /// # Panics
    ///
    /// Panics if `COMBINED_NETWORK_CACHE_SIZE` is 0
    #[must_use]
    pub fn new(
        primary_network: PushCdnNetwork<TYPES::SignatureKey>,
        secondary_network: Libp2pNetwork<TYPES>,
        delay_duration: Option<Duration>,
    ) -> Self {
        // Create networks from the ones passed in
        let networks = Arc::from(UnderlyingCombinedNetworks(
            primary_network,
            secondary_network,
        ));

        Self {
            networks,
            message_deduplication_cache: Arc::new(PlRwLock::new(MessageDeduplicationCache::new())),
            primary_fail_counter: Arc::new(AtomicU64::new(0)),
            primary_down: Arc::new(AtomicBool::new(false)),
            delay_duration: Arc::new(RwLock::new(
                delay_duration.unwrap_or(Duration::from_millis(COMBINED_NETWORK_DELAY_DURATION)),
            )),
            delayed_tasks_channels: Arc::default(),
            no_delay_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get a ref to the primary network
    #[must_use]
    pub fn primary(&self) -> &PushCdnNetwork<TYPES::SignatureKey> {
        &self.networks.0
    }

    /// Get a ref to the backup network
    #[must_use]
    pub fn secondary(&self) -> &Libp2pNetwork<TYPES> {
        &self.networks.1
    }

    /// a helper function to send messages through both networks (possibly delayed)
    async fn send_both_networks(
        &self,
        _message: Vec<u8>,
        primary_future: impl Future<Output = Result<(), NetworkError>> + Send + 'static,
        secondary_future: impl Future<Output = Result<(), NetworkError>> + Send + 'static,
        broadcast_delay: BroadcastDelay,
    ) -> Result<(), NetworkError> {
        // A local variable used to decide whether to delay this message or not
        let mut primary_failed = false;
        if self.primary_down.load(Ordering::Relaxed) {
            // If the primary is considered down, we don't want to delay
            primary_failed = true;
        } else if self.primary_fail_counter.load(Ordering::Relaxed)
            > COMBINED_NETWORK_MIN_PRIMARY_FAILURES
        {
            // If the primary failed more than `COMBINED_NETWORK_MIN_PRIMARY_FAILURES` times,
            // we don't want to delay this message, and from now on we consider the primary as down
            info!(
                "View progression is slower than normally, stop delaying messages on the secondary"
            );
            self.primary_down.store(true, Ordering::Relaxed);
            primary_failed = true;
        }

        // Always send on the primary network
        if let Err(e) = primary_future.await {
            // If the primary failed right away, we don't want to delay this message
            warn!("Error on primary network: {}", e);
            self.primary_fail_counter.fetch_add(1, Ordering::Relaxed);
            primary_failed = true;
        };

        if let (BroadcastDelay::View(view), false) = (broadcast_delay, primary_failed) {
            // We are delaying this message
            let duration = *self.delay_duration.read().await;
            let primary_down = Arc::clone(&self.primary_down);
            let primary_fail_counter = Arc::clone(&self.primary_fail_counter);
            // Each delayed task gets its own receiver clone to get a signal cancelling all tasks
            // related to the given view.
            let mut receiver = self
                .delayed_tasks_channels
                .write()
                .await
                .entry(view)
                .or_insert_with(|| {
                    let (s, r) = broadcast(1);
                    (s, r.deactivate())
                })
                .1
                .activate_cloned();
            // Spawn a task that sleeps for `duration` and then sends the message if it wasn't cancelled
            spawn(async move {
                sleep(duration).await;
                if receiver.try_recv().is_ok() {
                    // The task has been cancelled because the view progressed, it means the primary is working fine
                    debug!(
                        "Not sending on secondary after delay, task was canceled in view update"
                    );
                    match primary_fail_counter.load(Ordering::Relaxed) {
                        0u64 => {
                            // The primary fail counter reached 0, the primary is now considered up
                            primary_down.store(false, Ordering::Relaxed);
                            debug!("primary_fail_counter reached zero, primary_down set to false");
                        },
                        c => {
                            // Decrement the primary fail counter
                            primary_fail_counter.store(c - 1, Ordering::Relaxed);
                            debug!("primary_fail_counter set to {:?}", c - 1);
                        },
                    }
                    return Ok(());
                }
                // The task hasn't been cancelled, the primary probably failed.
                // Increment the primary fail counter and send the message.
                debug!("Sending on secondary after delay, message possibly has not reached recipient on primary");
                primary_fail_counter.fetch_add(1, Ordering::Relaxed);
                secondary_future.await
            });
            Ok(())
        } else {
            // We will send without delay
            if self.primary_down.load(Ordering::Relaxed) {
                // If the primary is considered down, we want to periodically delay sending
                // on the secondary to check whether the primary is able to deliver.
                // This message will be sent without delay but the next might be delayed.
                match self.no_delay_counter.load(Ordering::Relaxed) {
                    c if c < COMBINED_NETWORK_PRIMARY_CHECK_INTERVAL => {
                        // Just increment the 'no delay counter'
                        self.no_delay_counter.store(c + 1, Ordering::Relaxed);
                    },
                    _ => {
                        // The 'no delay counter' reached the threshold
                        debug!(
                            "Sent on secondary without delay more than {} times,\
                            try delaying to check primary",
                            COMBINED_NETWORK_PRIMARY_CHECK_INTERVAL
                        );
                        // Reset the 'no delay counter'
                        self.no_delay_counter.store(0u64, Ordering::Relaxed);
                        // The primary is not considered down for the moment
                        self.primary_down.store(false, Ordering::Relaxed);
                        // The primary fail counter is set just below the threshold to delay the next message
                        self.primary_fail_counter
                            .store(COMBINED_NETWORK_MIN_PRIMARY_FAILURES, Ordering::Relaxed);
                    },
                }
            }
            // Send the message
            secondary_future.await
        }
    }
}

/// Wrapper for the tuple of `PushCdnNetwork` and `Libp2pNetwork`
/// We need this so we can impl `TestableNetworkingImplementation`
/// on the tuple
#[derive(Clone)]
pub struct UnderlyingCombinedNetworks<TYPES: NodeType>(
    pub PushCdnNetwork<TYPES::SignatureKey>,
    pub Libp2pNetwork<TYPES>,
);

#[cfg(feature = "hotshot-testing")]
impl<TYPES: NodeType> TestableNetworkingImplementation<TYPES> for CombinedNetworks<TYPES> {
    fn generator(
        expected_node_count: usize,
        num_bootstrap: usize,
        network_id: usize,
        da_committee_size: usize,
        reliability_config: Option<Box<dyn NetworkReliability>>,
        secondary_network_delay: Duration,
    ) -> AsyncGenerator<Arc<Self>> {
        let generators = (
            <PushCdnNetwork<TYPES::SignatureKey> as TestableNetworkingImplementation<TYPES>>::generator(
                expected_node_count,
                num_bootstrap,
                network_id,
                da_committee_size,
                None,
                Duration::default(),
            ),
            <Libp2pNetwork<TYPES> as TestableNetworkingImplementation<TYPES>>::generator(
                expected_node_count,
                num_bootstrap,
                network_id,
                da_committee_size,
                reliability_config,
                Duration::default(),
            )
        );
        Box::pin(move |node_id| {
            let gen0 = generators.0(node_id);
            let gen1 = generators.1(node_id);

            Box::pin(async move {
                // Generate the CDN network
                let cdn = gen0.await;
                let cdn = Arc::<PushCdnNetwork<TYPES::SignatureKey>>::into_inner(cdn).unwrap();

                // Generate the p2p network
                let p2p = gen1.await;

                // Combine the two
                let underlying_combined = UnderlyingCombinedNetworks(
                    cdn.clone(),
                    Arc::<Libp2pNetwork<TYPES>>::unwrap_or_clone(p2p),
                );

                // Create a new message deduplication cache
                let message_deduplication_cache =
                    Arc::new(PlRwLock::new(MessageDeduplicationCache::new()));

                // Combine the two networks with the same cache
                let combined_network = Self {
                    networks: Arc::new(underlying_combined),
                    primary_fail_counter: Arc::new(AtomicU64::new(0)),
                    primary_down: Arc::new(AtomicBool::new(false)),
                    message_deduplication_cache: Arc::clone(&message_deduplication_cache),
                    delay_duration: Arc::new(RwLock::new(secondary_network_delay)),
                    delayed_tasks_channels: Arc::default(),
                    no_delay_counter: Arc::new(AtomicU64::new(0)),
                };

                Arc::new(combined_network)
            })
        })
    }

    /// Get the number of messages in-flight.
    ///
    /// Some implementations will not be able to tell how many messages there are in-flight. These implementations should return `None`.
    fn in_flight_message_count(&self) -> Option<usize> {
        None
    }
}

#[async_trait]
impl<TYPES: NodeType> ConnectedNetwork<TYPES::SignatureKey> for CombinedNetworks<TYPES> {
    fn pause(&self) {
        self.networks.0.pause();
    }

    fn resume(&self) {
        self.networks.0.resume();
    }

    async fn wait_for_ready(&self) {
        join!(
            self.primary().wait_for_ready(),
            self.secondary().wait_for_ready()
        );
    }

    fn shut_down<'a, 'b>(&'a self) -> BoxSyncFuture<'b, ()>
    where
        'a: 'b,
        Self: 'b,
    {
        let closure = async move {
            join!(self.primary().shut_down(), self.secondary().shut_down());
        };
        boxed_sync(closure)
    }

    async fn broadcast_message(
        &self,
        message: Vec<u8>,
        topic: Topic,
        broadcast_delay: BroadcastDelay,
    ) -> Result<(), NetworkError> {
        let primary = self.primary().clone();
        let secondary = self.secondary().clone();
        let primary_message = message.clone();
        let secondary_message = message.clone();
        let topic_clone = topic.clone();
        self.send_both_networks(
            message,
            async move {
                primary
                    .broadcast_message(primary_message, topic_clone, BroadcastDelay::None)
                    .await
            },
            async move {
                secondary
                    .broadcast_message(secondary_message, topic, BroadcastDelay::None)
                    .await
            },
            broadcast_delay,
        )
        .await
    }

    async fn da_broadcast_message(
        &self,
        message: Vec<u8>,
        recipients: Vec<TYPES::SignatureKey>,
        broadcast_delay: BroadcastDelay,
    ) -> Result<(), NetworkError> {
        let primary = self.primary().clone();
        let secondary = self.secondary().clone();
        let primary_message = message.clone();
        let secondary_message = message.clone();
        let primary_recipients = recipients.clone();
        self.send_both_networks(
            message,
            async move {
                primary
                    .da_broadcast_message(primary_message, primary_recipients, BroadcastDelay::None)
                    .await
            },
            async move {
                secondary
                    .da_broadcast_message(secondary_message, recipients, BroadcastDelay::None)
                    .await
            },
            broadcast_delay,
        )
        .await
    }

    async fn direct_message(
        &self,
        message: Vec<u8>,
        recipient: TYPES::SignatureKey,
    ) -> Result<(), NetworkError> {
        let primary = self.primary().clone();
        let secondary = self.secondary().clone();
        let primary_message = message.clone();
        let secondary_message = message.clone();
        let primary_recipient = recipient.clone();
        self.send_both_networks(
            message,
            async move {
                primary
                    .direct_message(primary_message, primary_recipient)
                    .await
            },
            async move { secondary.direct_message(secondary_message, recipient).await },
            BroadcastDelay::None,
        )
        .await
    }

    async fn vid_broadcast_message(
        &self,
        messages: HashMap<TYPES::SignatureKey, Vec<u8>>,
    ) -> Result<(), NetworkError> {
        self.networks.0.vid_broadcast_message(messages).await
    }

    /// Receive one or many messages from the underlying network.
    ///
    /// # Errors
    /// Does not error
    async fn recv_message(&self) -> Result<Vec<u8>, NetworkError> {
        loop {
            // Receive from both networks
            let mut primary_fut = self.primary().recv_message().fuse();
            let mut secondary_fut = self.secondary().recv_message().fuse();

            // Wait for one to return a message
            let (message, from_primary) = select! {
                p = primary_fut => (p?, true),
                s = secondary_fut => (s?, false),
            };

            // See if we should process the message or not based on the cache
            if self
                .message_deduplication_cache
                .write()
                .is_unique(&message, from_primary)
            {
                break Ok(message);
            }
        }
    }

    fn queue_node_lookup(
        &self,
        view_number: ViewNumber,
        pk: TYPES::SignatureKey,
    ) -> Result<(), TrySendError<Option<(ViewNumber, TYPES::SignatureKey)>>> {
        self.primary().queue_node_lookup(view_number, pk.clone())?;
        self.secondary().queue_node_lookup(view_number, pk)
    }

    async fn update_view<'a, T>(
        &'a self,
        view: u64,
        epoch: Option<u64>,
        membership: EpochMembershipCoordinator<T>,
    ) where
        T: NodeType<SignatureKey = TYPES::SignatureKey> + 'a,
    {
        let delayed_tasks_channels = Arc::clone(&self.delayed_tasks_channels);
        spawn(async move {
            let mut map_lock = delayed_tasks_channels.write().await;
            while let Some((first_view, _)) = map_lock.first_key_value() {
                // Broadcast a cancelling signal to all the tasks related to each view older than the new one
                if *first_view < view {
                    if let Some((_, (sender, _))) = map_lock.pop_first() {
                        let _ = sender.try_broadcast(());
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        });
        // Run `update_view` logic for the libp2p network
        self.networks
            .1
            .update_view::<T>(view, epoch, membership)
            .await;
    }

    fn is_primary_down(&self) -> bool {
        self.primary_down.load(Ordering::Relaxed)
    }
}

struct MessageDeduplicationCache {
    /// Last n seen messages from the primary network (to prevent processing a duplicate
    /// received from the secondary network)
    primary_message_cache: LruCache<blake3::Hash, ()>,

    /// Last n seen messages from the secondary network (to prevent processing a duplicate
    /// received from the primary network)
    secondary_message_cache: LruCache<blake3::Hash, ()>,
}

impl MessageDeduplicationCache {
    /// Create a new, empty message cache
    fn new() -> Self {
        Self {
            primary_message_cache: LruCache::new(
                NonZeroUsize::new(COMBINED_NETWORK_CACHE_SIZE).unwrap(),
            ),
            secondary_message_cache: LruCache::new(
                NonZeroUsize::new(COMBINED_NETWORK_CACHE_SIZE).unwrap(),
            ),
        }
    }

    /// Determine if a message is unique between two sources
    fn is_unique(&mut self, message: &[u8], from_primary: bool) -> bool {
        // Calculate the hash of the message
        let message_hash = blake3::hash(message);

        // Determine which cache to use based on the source of the message
        let (this_cache, other_cache) = if from_primary {
            (
                &mut self.primary_message_cache,
                &mut self.secondary_message_cache,
            )
        } else {
            (
                &mut self.secondary_message_cache,
                &mut self.primary_message_cache,
            )
        };

        // Check if we've seen this message from the other source. We want to use `pop` because we
        // still want to process the message if it gets received again from the same source.
        if other_cache.pop(&message_hash).is_some() {
            // We've seen this message from the other source, don't process it again
            false
        } else {
            // First time seeing from this source or already processed duplicate
            this_cache.put(message_hash, ());
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_deduplication() {
        let message = b"hello";

        // Process one message from both. Only the first one should be unique
        let mut cache = MessageDeduplicationCache::new();
        assert!(cache.is_unique(message, true));
        assert!(!cache.is_unique(message, false));

        // Since we've already received it once on both, it should continue to be unique
        // on the second receive
        assert!(cache.is_unique(message, true));
        assert!(!cache.is_unique(message, false));

        // Try both of the above tests the other way around
        assert!(cache.is_unique(message, false));
        assert!(!cache.is_unique(message, true));
        assert!(cache.is_unique(message, false));
        assert!(!cache.is_unique(message, true));

        // The same message from the same source a few times should always be treated as unique
        assert!(cache.is_unique(message, true));
        assert!(cache.is_unique(message, true));
        assert!(cache.is_unique(message, true));
        assert!(!cache.is_unique(message, false));
        assert!(cache.is_unique(message, false));
    }
}
