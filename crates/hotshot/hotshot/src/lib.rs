// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Provides a generic rust implementation of the `HotShot` BFT protocol
//!

// Documentation module
#[cfg(feature = "docs")]
pub mod documentation;
use committable::Committable;
use futures::future::{select, Either};
use hotshot_types::{
    drb::{DrbResult, INITIAL_DRB_RESULT},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    simple_certificate::LightClientStateUpdateCertificate,
    traits::{
        block_contents::BlockHeader, election::Membership, network::BroadcastDelay,
        node_implementation::Versions, signature_key::StateSignatureKey,
    },
    utils::epoch_from_block_number,
};
use rand::Rng;
use url::Url;
use vbs::version::StaticVersionType;

/// Contains traits consumed by [`SystemContext`]
pub mod traits;
/// Contains types used by the crate
pub mod types;

pub mod tasks;
use hotshot_types::data::QuorumProposalWrapper;

/// Contains helper functions for the crate
pub mod helpers;

use std::{
    collections::{BTreeMap, HashMap},
    num::NonZeroUsize,
    sync::Arc,
    time::Duration,
};

use async_broadcast::{broadcast, InactiveReceiver, Receiver, Sender};
use async_lock::RwLock;
use async_trait::async_trait;
use futures::join;
use hotshot_task::task::{ConsensusTaskRegistry, NetworkTaskRegistry};
use hotshot_task_impls::{events::HotShotEvent, helpers::broadcast_event};
// Internal
/// Reexport error type
pub use hotshot_types::error::HotShotError;
use hotshot_types::{
    consensus::{
        Consensus, ConsensusMetricsValue, OuterConsensus, PayloadWithMetadata, VidShares, View,
        ViewInner,
    },
    constants::{EVENT_CHANNEL_SIZE, EXTERNAL_EVENT_CHANNEL_SIZE},
    data::Leaf2,
    event::{EventType, LeafInfo},
    message::{DataMessage, Message, MessageKind, Proposal},
    simple_certificate::{NextEpochQuorumCertificate2, QuorumCertificate2, UpgradeCertificate},
    traits::{
        consensus_api::ConsensusApi,
        network::ConnectedNetwork,
        node_implementation::{ConsensusTime, NodeType},
        signature_key::SignatureKey,
        states::ValidatedState,
    },
    utils::genesis_epoch_from_version,
    HotShotConfig,
};
/// Reexport rand crate
pub use rand;
use tokio::{spawn, time::sleep};
use tracing::{debug, instrument, trace};

// -- Rexports
// External
use crate::{
    tasks::{add_consensus_tasks, add_network_tasks},
    traits::NodeImplementation,
    types::{Event, SystemContextHandle},
};

/// Length, in bytes, of a 512 bit hash
pub const H_512: usize = 64;
/// Length, in bytes, of a 256 bit hash
pub const H_256: usize = 32;

#[derive(Clone)]
/// Wrapper for all marketplace config that needs to be passed when creating a new instance of HotShot
pub struct MarketplaceConfig<TYPES: NodeType, I: NodeImplementation<TYPES>> {
    /// auction results provider
    pub auction_results_provider: Arc<I::AuctionResultsProvider>,
    /// fallback builder
    pub fallback_builder_url: Url,
}

/// Holds the state needed to participate in `HotShot` consensus
pub struct SystemContext<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> {
    /// The public key of this node
    public_key: TYPES::SignatureKey,

    /// The private key of this node
    private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,

    /// The private key to sign the light client state
    state_private_key: <TYPES::StateSignatureKey as StateSignatureKey>::StatePrivateKey,

    /// Configuration items for this hotshot instance
    pub config: HotShotConfig<TYPES>,

    /// The underlying network
    pub network: Arc<I::Network>,

    /// Memberships used by consensus
    pub membership_coordinator: EpochMembershipCoordinator<TYPES>,

    /// the metrics that the implementor is using.
    metrics: Arc<ConsensusMetricsValue>,

    /// The hotstuff implementation
    consensus: OuterConsensus<TYPES>,

    /// Immutable instance state
    instance_state: Arc<TYPES::InstanceState>,

    /// The view to enter when first starting consensus
    start_view: TYPES::View,

    /// The epoch to enter when first starting consensus
    start_epoch: Option<TYPES::Epoch>,

    /// Access to the output event stream.
    output_event_stream: (Sender<Event<TYPES>>, InactiveReceiver<Event<TYPES>>),

    /// External event stream for communication with the application.
    pub(crate) external_event_stream: (Sender<Event<TYPES>>, InactiveReceiver<Event<TYPES>>),

    /// Anchored leaf provided by the initializer.
    anchored_leaf: Leaf2<TYPES>,

    /// access to the internal event stream, in case we need to, say, shut something down
    #[allow(clippy::type_complexity)]
    internal_event_stream: (
        Sender<Arc<HotShotEvent<TYPES>>>,
        InactiveReceiver<Arc<HotShotEvent<TYPES>>>,
    ),

    /// uid for instrumentation
    pub id: u64,

    /// Reference to the internal storage for consensus datum.
    pub storage: I::Storage,

    /// shared lock for upgrade information
    pub upgrade_lock: UpgradeLock<TYPES, V>,

    /// Marketplace config for this instance of HotShot
    pub marketplace_config: MarketplaceConfig<TYPES, I>,
}
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> Clone
    for SystemContext<TYPES, I, V>
{
    #![allow(deprecated)]
    fn clone(&self) -> Self {
        Self {
            public_key: self.public_key.clone(),
            private_key: self.private_key.clone(),
            state_private_key: self.state_private_key.clone(),
            config: self.config.clone(),
            network: Arc::clone(&self.network),
            membership_coordinator: self.membership_coordinator.clone(),
            metrics: Arc::clone(&self.metrics),
            consensus: self.consensus.clone(),
            instance_state: Arc::clone(&self.instance_state),
            start_view: self.start_view,
            start_epoch: self.start_epoch,
            output_event_stream: self.output_event_stream.clone(),
            external_event_stream: self.external_event_stream.clone(),
            anchored_leaf: self.anchored_leaf.clone(),
            internal_event_stream: self.internal_event_stream.clone(),
            id: self.id,
            storage: self.storage.clone(),
            upgrade_lock: self.upgrade_lock.clone(),
            marketplace_config: self.marketplace_config.clone(),
        }
    }
}

impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> SystemContext<TYPES, I, V> {
    #![allow(deprecated)]
    /// Creates a new [`Arc<SystemContext>`] with the given configuration options.
    ///
    /// To do a full initialization, use `fn init` instead, which will set up background tasks as
    /// well.
    ///
    /// Use this instead of `init` if you want to start the tasks manually
    ///
    /// # Panics
    ///
    /// Panics if storage migration fails.
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        public_key: TYPES::SignatureKey,
        private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,
        state_private_key: <TYPES::StateSignatureKey as StateSignatureKey>::StatePrivateKey,
        nonce: u64,
        config: HotShotConfig<TYPES>,
        memberships: EpochMembershipCoordinator<TYPES>,
        network: Arc<I::Network>,
        initializer: HotShotInitializer<TYPES>,
        metrics: ConsensusMetricsValue,
        storage: I::Storage,
        marketplace_config: MarketplaceConfig<TYPES, I>,
    ) -> Arc<Self> {
        let internal_chan = broadcast(EVENT_CHANNEL_SIZE);
        let external_chan = broadcast(EXTERNAL_EVENT_CHANNEL_SIZE);

        Self::new_from_channels(
            public_key,
            private_key,
            state_private_key,
            nonce,
            config,
            memberships,
            network,
            initializer,
            metrics,
            storage,
            marketplace_config,
            internal_chan,
            external_chan,
        )
        .await
    }

    /// Creates a new [`Arc<SystemContext>`] with the given configuration options.
    ///
    /// To do a full initialization, use `fn init` instead, which will set up background tasks as
    /// well.
    ///
    /// Use this function if you want to use some preexisting channels and to spin up the tasks
    /// and start consensus manually.  Mostly useful for tests
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub async fn new_from_channels(
        public_key: TYPES::SignatureKey,
        private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,
        state_private_key: <TYPES::StateSignatureKey as StateSignatureKey>::StatePrivateKey,
        nonce: u64,
        config: HotShotConfig<TYPES>,
        membership_coordinator: EpochMembershipCoordinator<TYPES>,
        network: Arc<I::Network>,
        initializer: HotShotInitializer<TYPES>,
        metrics: ConsensusMetricsValue,
        storage: I::Storage,
        marketplace_config: MarketplaceConfig<TYPES, I>,
        internal_channel: (
            Sender<Arc<HotShotEvent<TYPES>>>,
            Receiver<Arc<HotShotEvent<TYPES>>>,
        ),
        external_channel: (Sender<Event<TYPES>>, Receiver<Event<TYPES>>),
    ) -> Arc<Self> {
        debug!("Creating a new hotshot");

        tracing::warn!("Starting consensus with HotShotConfig:\n\n {:?}", config);

        let consensus_metrics = Arc::new(metrics);
        let anchored_leaf = initializer.anchor_leaf;
        let instance_state = initializer.instance_state;

        let (internal_tx, mut internal_rx) = internal_channel;
        let (mut external_tx, mut external_rx) = external_channel;

        tracing::warn!(
            "Starting consensus with versions:\n\n Base: {:?}\nUpgrade: {:?}.",
            V::Base::VERSION,
            V::Upgrade::VERSION,
        );
        tracing::warn!(
            "Loading previously decided upgrade certificate from storage: {:?}",
            initializer.decided_upgrade_certificate
        );

        let upgrade_lock =
            UpgradeLock::<TYPES, V>::from_certificate(&initializer.decided_upgrade_certificate);

        // Allow overflow on the external channel, otherwise sending to it may block.
        external_rx.set_overflow(true);

        // Allow overflow on the internal channel as well. We don't want to block consensus if we
        // have a slow receiver
        internal_rx.set_overflow(true);

        // Get the validated state from the initializer or construct an incomplete one from the
        // block header.
        let validated_state = initializer.anchor_state;

        load_start_epoch_info(
            membership_coordinator.membership(),
            &initializer.start_epoch_info,
            config.epoch_height,
            config.epoch_start_block,
        )
        .await;

        // #3967 REVIEW NOTE: Should this actually be Some()? How do we know?
        let epoch = initializer.high_qc.data.block_number.map(|block_number| {
            TYPES::Epoch::new(epoch_from_block_number(
                block_number + 1,
                config.epoch_height,
            ))
        });

        // Insert the validated state to state map.
        let mut validated_state_map = BTreeMap::default();
        validated_state_map.insert(
            anchored_leaf.view_number(),
            View {
                view_inner: ViewInner::Leaf {
                    leaf: anchored_leaf.commit(),
                    state: Arc::clone(&validated_state),
                    delta: initializer.anchor_state_delta,
                    epoch,
                },
            },
        );
        for (view_num, inner) in initializer.undecided_state {
            validated_state_map.insert(view_num, inner);
        }

        let mut saved_leaves = HashMap::new();
        let mut saved_payloads = BTreeMap::new();
        saved_leaves.insert(anchored_leaf.commit(), anchored_leaf.clone());

        for (_, leaf) in initializer.undecided_leaves {
            saved_leaves.insert(leaf.commit(), leaf.clone());
        }
        if let Some(payload) = anchored_leaf.block_payload() {
            let metadata = anchored_leaf.block_header().metadata().clone();
            saved_payloads.insert(
                anchored_leaf.view_number(),
                Arc::new(PayloadWithMetadata { payload, metadata }),
            );
        }

        let consensus = Consensus::new(
            validated_state_map,
            Some(initializer.saved_vid_shares),
            anchored_leaf.view_number(),
            epoch,
            anchored_leaf.view_number(),
            anchored_leaf.view_number(),
            initializer.last_actioned_view,
            initializer.saved_proposals,
            saved_leaves,
            saved_payloads,
            initializer.high_qc,
            initializer.next_epoch_high_qc,
            Arc::clone(&consensus_metrics),
            config.epoch_height,
            initializer.state_cert,
        );

        let consensus = Arc::new(RwLock::new(consensus));

        // This makes it so we won't block on broadcasting if there is not a receiver
        // Our own copy of the receiver is inactive so it doesn't count.
        external_tx.set_await_active(false);

        let inner: Arc<SystemContext<TYPES, I, V>> = Arc::new(SystemContext {
            id: nonce,
            consensus: OuterConsensus::new(consensus),
            instance_state: Arc::new(instance_state),
            public_key,
            private_key,
            state_private_key,
            config,
            start_view: initializer.start_view,
            start_epoch: initializer.start_epoch,
            network,
            membership_coordinator,
            metrics: Arc::clone(&consensus_metrics),
            internal_event_stream: (internal_tx, internal_rx.deactivate()),
            output_event_stream: (external_tx.clone(), external_rx.clone().deactivate()),
            external_event_stream: (external_tx, external_rx.deactivate()),
            anchored_leaf: anchored_leaf.clone(),
            storage,
            upgrade_lock,
            marketplace_config,
        });

        inner
    }

    /// "Starts" consensus by sending a `Qc2Formed`, `ViewChange` events
    ///
    /// # Panics
    /// Panics if sending genesis fails
    #[instrument(skip_all, target = "SystemContext", fields(id = self.id))]
    pub async fn start_consensus(&self) {
        #[cfg(all(feature = "rewind", not(debug_assertions)))]
        compile_error!("Cannot run rewind in production builds!");

        debug!("Starting Consensus");
        let consensus = self.consensus.read().await;

        #[allow(clippy::panic)]
        self.internal_event_stream
            .0
            .broadcast_direct(Arc::new(HotShotEvent::ViewChange(
                self.start_view,
                self.start_epoch,
            )))
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "Genesis Broadcast failed; event = ViewChange({:?})",
                    self.start_view
                )
            });

        // Clone the event stream that we send the timeout event to
        let event_stream = self.internal_event_stream.0.clone();
        let next_view_timeout = self.config.next_view_timeout;
        let start_view = self.start_view;
        let start_epoch = self.start_epoch;

        // Spawn a task that will sleep for the next view timeout and then send a timeout event
        // if not cancelled
        spawn({
            async move {
                sleep(Duration::from_millis(next_view_timeout)).await;
                broadcast_event(
                    Arc::new(HotShotEvent::Timeout(
                        start_view + 1,
                        start_epoch.map(|x| x + 1),
                    )),
                    &event_stream,
                )
                .await;
            }
        });
        #[allow(clippy::panic)]
        self.internal_event_stream
            .0
            .broadcast_direct(Arc::new(HotShotEvent::Qc2Formed(either::Left(
                consensus.high_qc().clone(),
            ))))
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "Genesis Broadcast failed; event = Qc2Formed(either::Left({:?}))",
                    consensus.high_qc()
                )
            });

        {
            // Some applications seem to expect a leaf decide event for the genesis leaf,
            // which contains only that leaf and nothing else.
            if self.anchored_leaf.view_number() == TYPES::View::genesis() {
                let (validated_state, state_delta) =
                    TYPES::ValidatedState::genesis(&self.instance_state);

                let qc = Arc::new(
                    QuorumCertificate2::genesis::<V>(
                        &validated_state,
                        self.instance_state.as_ref(),
                    )
                    .await,
                );

                broadcast_event(
                    Event {
                        view_number: self.anchored_leaf.view_number(),
                        event: EventType::Decide {
                            leaf_chain: Arc::new(vec![LeafInfo::new(
                                self.anchored_leaf.clone(),
                                Arc::new(validated_state),
                                Some(Arc::new(state_delta)),
                                None,
                                None,
                            )]),
                            qc,
                            block_size: None,
                        },
                    },
                    &self.external_event_stream.0,
                )
                .await;
            }
        }
    }

    /// Emit an external event
    async fn send_external_event(&self, event: Event<TYPES>) {
        debug!(?event, "send_external_event");
        broadcast_event(event, &self.external_event_stream.0).await;
    }

    /// Publishes a transaction asynchronously to the network.
    ///
    /// # Errors
    ///
    /// Always returns Ok; does not return an error if the transaction couldn't be published to the network
    #[instrument(skip(self), err, target = "SystemContext", fields(id = self.id))]
    pub async fn publish_transaction_async(
        &self,
        transaction: TYPES::Transaction,
    ) -> Result<(), HotShotError<TYPES>> {
        trace!("Adding transaction to our own queue");

        let api = self.clone();

        let consensus_reader = api.consensus.read().await;
        let view_number = consensus_reader.cur_view();
        let epoch = consensus_reader.cur_epoch();
        drop(consensus_reader);

        // Wrap up a message
        let message_kind: DataMessage<TYPES> =
            DataMessage::SubmitTransaction(transaction.clone(), view_number);
        let message = Message {
            sender: api.public_key.clone(),
            kind: MessageKind::from(message_kind),
        };

        let serialized_message = self.upgrade_lock.serialize(&message).await.map_err(|err| {
            HotShotError::FailedToSerialize(format!("failed to serialize transaction: {err}"))
        })?;

        let membership = match api.membership_coordinator.membership_for_epoch(epoch).await {
            Ok(m) => m,
            Err(e) => return Err(HotShotError::InvalidState(e.message)),
        };

        spawn(async move {
            let memberships_da_committee_members = membership
                .da_committee_members(view_number)
                .await
                .iter()
                .cloned()
                .collect();

            join! {
                // TODO We should have a function that can return a network error if there is one
                // but first we'd need to ensure our network implementations can support that
                // (and not hang instead)

                // version <0, 1> currently fixed; this is the same as VERSION_0_1,
                // and will be updated to be part of SystemContext. I wanted to use associated
                // constants in NodeType, but that seems to be unavailable in the current Rust.
                api
                    .network.da_broadcast_message(
                        serialized_message,
                        memberships_da_committee_members,
                        BroadcastDelay::None,
                    ),
                api
                    .send_external_event(Event {
                        view_number,
                        event: EventType::Transactions {
                            transactions: vec![transaction],
                        },
                    }),
            }
        });
        Ok(())
    }

    /// Returns a copy of the consensus struct
    #[must_use]
    pub fn consensus(&self) -> Arc<RwLock<Consensus<TYPES>>> {
        Arc::clone(&self.consensus.inner_consensus)
    }

    /// Returns a copy of the instance state
    pub fn instance_state(&self) -> Arc<TYPES::InstanceState> {
        Arc::clone(&self.instance_state)
    }

    /// Returns a copy of the last decided leaf
    /// # Panics
    /// Panics if internal leaf for consensus is inconsistent
    #[instrument(skip_all, target = "SystemContext", fields(id = self.id))]
    pub async fn decided_leaf(&self) -> Leaf2<TYPES> {
        self.consensus.read().await.decided_leaf()
    }

    /// [Non-blocking] instantly returns a copy of the last decided leaf if
    /// it is available to be read. If not, we return `None`.
    ///
    /// # Panics
    /// Panics if internal state for consensus is inconsistent
    #[must_use]
    #[instrument(skip_all, target = "SystemContext", fields(id = self.id))]
    pub fn try_decided_leaf(&self) -> Option<Leaf2<TYPES>> {
        self.consensus.try_read().map(|guard| guard.decided_leaf())
    }

    /// Returns the last decided validated state.
    ///
    /// # Panics
    /// Panics if internal state for consensus is inconsistent
    #[instrument(skip_all, target = "SystemContext", fields(id = self.id))]
    pub async fn decided_state(&self) -> Arc<TYPES::ValidatedState> {
        Arc::clone(&self.consensus.read().await.decided_state())
    }

    /// Get the validated state from a given `view`.
    ///
    /// Returns the requested state, if the [`SystemContext`] is tracking this view. Consensus
    /// tracks views that have not yet been decided but could be in the future. This function may
    /// return [`None`] if the requested view has already been decided (but see
    /// [`decided_state`](Self::decided_state)) or if there is no path for the requested
    /// view to ever be decided.
    #[instrument(skip_all, target = "SystemContext", fields(id = self.id))]
    pub async fn state(&self, view: TYPES::View) -> Option<Arc<TYPES::ValidatedState>> {
        self.consensus.read().await.state(view).cloned()
    }

    /// Initializes a new [`SystemContext`] and does the work of setting up all the background tasks
    ///
    /// Assumes networking implementation is already primed.
    ///
    /// Underlying `HotShot` instance starts out paused, and must be unpaused
    ///
    /// Upon encountering an unrecoverable error, such as a failure to send to a broadcast channel,
    /// the `HotShot` instance will log the error and shut down.
    ///
    /// To construct a [`SystemContext`] without setting up tasks, use `fn new` instead.
    /// # Errors
    ///
    /// Can throw an error if `Self::new` fails.
    #[allow(clippy::too_many_arguments)]
    pub async fn init(
        public_key: TYPES::SignatureKey,
        private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,
        state_private_key: <TYPES::StateSignatureKey as StateSignatureKey>::StatePrivateKey,
        node_id: u64,
        config: HotShotConfig<TYPES>,
        memberships: EpochMembershipCoordinator<TYPES>,
        network: Arc<I::Network>,
        initializer: HotShotInitializer<TYPES>,
        metrics: ConsensusMetricsValue,
        storage: I::Storage,
        marketplace_config: MarketplaceConfig<TYPES, I>,
    ) -> Result<
        (
            SystemContextHandle<TYPES, I, V>,
            Sender<Arc<HotShotEvent<TYPES>>>,
            Receiver<Arc<HotShotEvent<TYPES>>>,
        ),
        HotShotError<TYPES>,
    > {
        let hotshot = Self::new(
            public_key,
            private_key,
            state_private_key,
            node_id,
            config,
            memberships,
            network,
            initializer,
            metrics,
            storage,
            marketplace_config,
        )
        .await;
        let handle = Arc::clone(&hotshot).run_tasks().await;
        let (tx, rx) = hotshot.internal_event_stream.clone();

        Ok((handle, tx, rx.activate()))
    }
    /// return the timeout for a view for `self`
    #[must_use]
    pub fn next_view_timeout(&self) -> u64 {
        self.config.next_view_timeout
    }
}

impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> SystemContext<TYPES, I, V> {
    /// Spawn all tasks that operate on [`SystemContextHandle`].
    ///
    /// For a list of which tasks are being spawned, see this module's documentation.
    pub async fn run_tasks(&self) -> SystemContextHandle<TYPES, I, V> {
        let consensus_registry = ConsensusTaskRegistry::new();
        let network_registry = NetworkTaskRegistry::new();

        let output_event_stream = self.external_event_stream.clone();
        let internal_event_stream = self.internal_event_stream.clone();

        let mut handle = SystemContextHandle {
            consensus_registry,
            network_registry,
            output_event_stream: output_event_stream.clone(),
            internal_event_stream: internal_event_stream.clone(),
            hotshot: self.clone().into(),
            storage: self.storage.clone(),
            network: Arc::clone(&self.network),
            membership_coordinator: self.membership_coordinator.clone(),
            epoch_height: self.config.epoch_height,
        };

        add_network_tasks::<TYPES, I, V>(&mut handle).await;
        add_consensus_tasks::<TYPES, I, V>(&mut handle).await;

        handle
    }
}

/// An async broadcast channel
type Channel<S> = (Sender<Arc<S>>, Receiver<Arc<S>>);

#[async_trait]
/// Trait for handling messages for a node with a twin copy of consensus
pub trait TwinsHandlerState<TYPES, I, V>
where
    Self: std::fmt::Debug + Send + Sync,
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    V: Versions,
{
    /// Handle a message sent to the twin from the network task, forwarding it to one of the two twins.
    async fn send_handler(
        &mut self,
        event: &HotShotEvent<TYPES>,
    ) -> Vec<Either<HotShotEvent<TYPES>, HotShotEvent<TYPES>>>;

    /// Handle a message from either twin, forwarding it to the network task
    async fn recv_handler(
        &mut self,
        event: &Either<HotShotEvent<TYPES>, HotShotEvent<TYPES>>,
    ) -> Vec<HotShotEvent<TYPES>>;

    /// Fuse two channels into a single channel
    ///
    /// Note: the channels are fused using two async loops, whose `JoinHandle`s are dropped.
    fn fuse_channels(
        &'static mut self,
        left: Channel<HotShotEvent<TYPES>>,
        right: Channel<HotShotEvent<TYPES>>,
    ) -> Channel<HotShotEvent<TYPES>> {
        let send_state = Arc::new(RwLock::new(self));
        let recv_state = Arc::clone(&send_state);

        let (left_sender, mut left_receiver) = (left.0, left.1);
        let (right_sender, mut right_receiver) = (right.0, right.1);

        // channel to the network task
        let (sender_to_network, network_task_receiver) = broadcast(EVENT_CHANNEL_SIZE);
        // channel from the network task
        let (network_task_sender, mut receiver_from_network): Channel<HotShotEvent<TYPES>> =
            broadcast(EVENT_CHANNEL_SIZE);

        let _recv_loop_handle = spawn(async move {
            loop {
                let msg = match select(left_receiver.recv(), right_receiver.recv()).await {
                    Either::Left(msg) => Either::Left(msg.0.unwrap().as_ref().clone()),
                    Either::Right(msg) => Either::Right(msg.0.unwrap().as_ref().clone()),
                };

                let mut state = recv_state.write().await;
                let mut result = state.recv_handler(&msg).await;

                while let Some(event) = result.pop() {
                    let _ = sender_to_network.broadcast(event.into()).await;
                }
            }
        });

        let _send_loop_handle = spawn(async move {
            loop {
                if let Ok(msg) = receiver_from_network.recv().await {
                    let mut state = send_state.write().await;

                    let mut result = state.send_handler(&msg).await;

                    while let Some(event) = result.pop() {
                        match event {
                            Either::Left(msg) => {
                                let _ = left_sender.broadcast(msg.into()).await;
                            },
                            Either::Right(msg) => {
                                let _ = right_sender.broadcast(msg.into()).await;
                            },
                        }
                    }
                }
            }
        });

        (network_task_sender, network_task_receiver)
    }

    #[allow(clippy::too_many_arguments)]
    /// Spawn all tasks that operate on [`SystemContextHandle`].
    ///
    /// For a list of which tasks are being spawned, see this module's documentation.
    async fn spawn_twin_handles(
        &'static mut self,
        public_key: TYPES::SignatureKey,
        private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,
        state_private_key: <TYPES::StateSignatureKey as StateSignatureKey>::StatePrivateKey,
        nonce: u64,
        config: HotShotConfig<TYPES>,
        memberships: EpochMembershipCoordinator<TYPES>,
        network: Arc<I::Network>,
        initializer: HotShotInitializer<TYPES>,
        metrics: ConsensusMetricsValue,
        storage: I::Storage,
        marketplace_config: MarketplaceConfig<TYPES, I>,
    ) -> (
        SystemContextHandle<TYPES, I, V>,
        SystemContextHandle<TYPES, I, V>,
    ) {
        let epoch_height = config.epoch_height;
        let left_system_context = SystemContext::new(
            public_key.clone(),
            private_key.clone(),
            state_private_key.clone(),
            nonce,
            config.clone(),
            memberships.clone(),
            Arc::clone(&network),
            initializer.clone(),
            metrics.clone(),
            storage.clone(),
            marketplace_config.clone(),
        )
        .await;
        let right_system_context = SystemContext::new(
            public_key,
            private_key,
            state_private_key,
            nonce,
            config,
            memberships,
            network,
            initializer,
            metrics,
            storage,
            marketplace_config,
        )
        .await;

        // create registries for both handles
        let left_consensus_registry = ConsensusTaskRegistry::new();
        let left_network_registry = NetworkTaskRegistry::new();

        let right_consensus_registry = ConsensusTaskRegistry::new();
        let right_network_registry = NetworkTaskRegistry::new();

        // create external channels for both handles
        let (left_external_sender, left_external_receiver) = broadcast(EXTERNAL_EVENT_CHANNEL_SIZE);
        let left_external_event_stream =
            (left_external_sender, left_external_receiver.deactivate());

        let (right_external_sender, right_external_receiver) =
            broadcast(EXTERNAL_EVENT_CHANNEL_SIZE);
        let right_external_event_stream =
            (right_external_sender, right_external_receiver.deactivate());

        // create internal channels for both handles
        let (left_internal_sender, left_internal_receiver) = broadcast(EVENT_CHANNEL_SIZE);
        let left_internal_event_stream = (
            left_internal_sender.clone(),
            left_internal_receiver.clone().deactivate(),
        );

        let (right_internal_sender, right_internal_receiver) = broadcast(EVENT_CHANNEL_SIZE);
        let right_internal_event_stream = (
            right_internal_sender.clone(),
            right_internal_receiver.clone().deactivate(),
        );

        // create each handle
        let mut left_handle = SystemContextHandle {
            consensus_registry: left_consensus_registry,
            network_registry: left_network_registry,
            output_event_stream: left_external_event_stream.clone(),
            internal_event_stream: left_internal_event_stream.clone(),
            hotshot: Arc::clone(&left_system_context),
            storage: left_system_context.storage.clone(),
            network: Arc::clone(&left_system_context.network),
            membership_coordinator: left_system_context.membership_coordinator.clone(),
            epoch_height,
        };

        let mut right_handle = SystemContextHandle {
            consensus_registry: right_consensus_registry,
            network_registry: right_network_registry,
            output_event_stream: right_external_event_stream.clone(),
            internal_event_stream: right_internal_event_stream.clone(),
            hotshot: Arc::clone(&right_system_context),
            storage: right_system_context.storage.clone(),
            network: Arc::clone(&right_system_context.network),
            membership_coordinator: right_system_context.membership_coordinator.clone(),
            epoch_height,
        };

        // add consensus tasks to each handle, using their individual internal event streams
        add_consensus_tasks::<TYPES, I, V>(&mut left_handle).await;
        add_consensus_tasks::<TYPES, I, V>(&mut right_handle).await;

        // fuse the event streams from both handles before initializing the network tasks
        let fused_internal_event_stream = self.fuse_channels(
            (left_internal_sender, left_internal_receiver),
            (right_internal_sender, right_internal_receiver),
        );

        // swap out the event stream on the left handle
        left_handle.internal_event_stream = (
            fused_internal_event_stream.0,
            fused_internal_event_stream.1.deactivate(),
        );

        // add the network tasks to the left handle. note: because the left handle has the fused event stream, the network tasks on the left handle will handle messages from both handles.
        add_network_tasks::<TYPES, I, V>(&mut left_handle).await;

        // revert to the original event stream on the left handle, for any applications that want to listen to it
        left_handle.internal_event_stream = left_internal_event_stream.clone();

        (left_handle, right_handle)
    }
}

#[derive(Debug)]
/// A `TwinsHandlerState` that randomly forwards a message to either twin,
/// and returns messages from both.
pub struct RandomTwinsHandler;

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> TwinsHandlerState<TYPES, I, V>
    for RandomTwinsHandler
{
    async fn send_handler(
        &mut self,
        event: &HotShotEvent<TYPES>,
    ) -> Vec<Either<HotShotEvent<TYPES>, HotShotEvent<TYPES>>> {
        let random: bool = rand::thread_rng().gen();

        #[allow(clippy::match_bool)]
        match random {
            true => vec![Either::Left(event.clone())],
            false => vec![Either::Right(event.clone())],
        }
    }

    async fn recv_handler(
        &mut self,
        event: &Either<HotShotEvent<TYPES>, HotShotEvent<TYPES>>,
    ) -> Vec<HotShotEvent<TYPES>> {
        match event {
            Either::Left(msg) | Either::Right(msg) => vec![msg.clone()],
        }
    }
}

#[derive(Debug)]
/// A `TwinsHandlerState` that forwards each message to both twins,
/// and returns messages from each of them.
pub struct DoubleTwinsHandler;

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> TwinsHandlerState<TYPES, I, V>
    for DoubleTwinsHandler
{
    async fn send_handler(
        &mut self,
        event: &HotShotEvent<TYPES>,
    ) -> Vec<Either<HotShotEvent<TYPES>, HotShotEvent<TYPES>>> {
        vec![Either::Left(event.clone()), Either::Right(event.clone())]
    }

    async fn recv_handler(
        &mut self,
        event: &Either<HotShotEvent<TYPES>, HotShotEvent<TYPES>>,
    ) -> Vec<HotShotEvent<TYPES>> {
        match event {
            Either::Left(msg) | Either::Right(msg) => vec![msg.clone()],
        }
    }
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> ConsensusApi<TYPES, I>
    for SystemContextHandle<TYPES, I, V>
{
    fn total_nodes(&self) -> NonZeroUsize {
        self.hotshot.config.num_nodes_with_stake
    }

    fn builder_timeout(&self) -> Duration {
        self.hotshot.config.builder_timeout
    }

    async fn send_event(&self, event: Event<TYPES>) {
        debug!(?event, "send_event");
        broadcast_event(event, &self.hotshot.external_event_stream.0).await;
    }

    fn public_key(&self) -> &TYPES::SignatureKey {
        &self.hotshot.public_key
    }

    fn private_key(&self) -> &<TYPES::SignatureKey as SignatureKey>::PrivateKey {
        &self.hotshot.private_key
    }

    fn state_private_key(
        &self,
    ) -> &<TYPES::StateSignatureKey as StateSignatureKey>::StatePrivateKey {
        &self.hotshot.state_private_key
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InitializerEpochInfo<TYPES: NodeType> {
    pub epoch: TYPES::Epoch,
    pub drb_result: DrbResult,
    // pub stake_table: Option<StakeTable>, // TODO: Figure out how to connect this up
    pub block_header: Option<TYPES::BlockHeader>,
}

#[derive(Clone)]
/// initializer struct for creating starting block
pub struct HotShotInitializer<TYPES: NodeType> {
    /// Instance-level state.
    pub instance_state: TYPES::InstanceState,

    /// Epoch height
    pub epoch_height: u64,

    /// Epoch start block
    pub epoch_start_block: u64,

    /// the anchor leaf for the hotshot initializer
    pub anchor_leaf: Leaf2<TYPES>,

    /// ValidatedState for the anchor leaf
    pub anchor_state: Arc<TYPES::ValidatedState>,

    /// ValidatedState::Delta for the anchor leaf, optional.
    pub anchor_state_delta: Option<Arc<<TYPES::ValidatedState as ValidatedState<TYPES>>::Delta>>,

    /// Starting view number that should be equivalent to the view the node shut down with last.
    pub start_view: TYPES::View,

    /// The view we last performed an action in.  An action is proposing or voting for
    /// either the quorum or DA.
    pub last_actioned_view: TYPES::View,

    /// Starting epoch number that should be equivalent to the epoch the node shut down with last.
    pub start_epoch: Option<TYPES::Epoch>,

    /// Highest QC that was seen, for genesis it's the genesis QC.  It should be for a view greater
    /// than `inner`s view number for the non genesis case because we must have seen higher QCs
    /// to decide on the leaf.
    pub high_qc: QuorumCertificate2<TYPES>,

    /// Next epoch highest QC that was seen. This is needed to propose during epoch transition after restart.
    pub next_epoch_high_qc: Option<NextEpochQuorumCertificate2<TYPES>>,

    /// Proposals we have sent out to provide to others for catchup
    pub saved_proposals: BTreeMap<TYPES::View, Proposal<TYPES, QuorumProposalWrapper<TYPES>>>,

    /// Previously decided upgrade certificate; this is necessary if an upgrade has happened and we are not restarting with the new version
    pub decided_upgrade_certificate: Option<UpgradeCertificate<TYPES>>,

    /// Undecided leaves that were seen, but not yet decided on.  These allow a restarting node
    /// to vote and propose right away if they didn't miss anything while down.
    pub undecided_leaves: BTreeMap<TYPES::View, Leaf2<TYPES>>,

    /// Not yet decided state
    pub undecided_state: BTreeMap<TYPES::View, View<TYPES>>,

    /// Saved VID shares
    pub saved_vid_shares: VidShares<TYPES>,

    /// The last formed light client state update certificate if there's any
    pub state_cert: Option<LightClientStateUpdateCertificate<TYPES>>,

    /// Saved epoch information. This must be sorted ascending by epoch.
    pub start_epoch_info: Vec<InitializerEpochInfo<TYPES>>,
}

impl<TYPES: NodeType> HotShotInitializer<TYPES> {
    /// initialize from genesis
    /// # Errors
    /// If we are unable to apply the genesis block to the default state
    pub async fn from_genesis<V: Versions>(
        instance_state: TYPES::InstanceState,
        epoch_height: u64,
        epoch_start_block: u64,
        start_epoch_info: Vec<InitializerEpochInfo<TYPES>>,
    ) -> Result<Self, HotShotError<TYPES>> {
        let (validated_state, state_delta) = TYPES::ValidatedState::genesis(&instance_state);
        let high_qc = QuorumCertificate2::genesis::<V>(&validated_state, &instance_state).await;

        Ok(Self {
            anchor_leaf: Leaf2::genesis::<V>(&validated_state, &instance_state).await,
            anchor_state: Arc::new(validated_state),
            anchor_state_delta: Some(Arc::new(state_delta)),
            start_view: TYPES::View::new(0),
            start_epoch: genesis_epoch_from_version::<V, TYPES>(),
            last_actioned_view: TYPES::View::new(0),
            saved_proposals: BTreeMap::new(),
            high_qc,
            next_epoch_high_qc: None,
            decided_upgrade_certificate: None,
            undecided_leaves: BTreeMap::new(),
            undecided_state: BTreeMap::new(),
            instance_state,
            saved_vid_shares: BTreeMap::new(),
            epoch_height,
            state_cert: None,
            epoch_start_block,
            start_epoch_info,
        })
    }

    /// Use saved proposals to update undecided leaves and state
    #[must_use]
    pub fn update_undecided(self) -> Self {
        let mut undecided_leaves = self.undecided_leaves.clone();
        let mut undecided_state = self.undecided_state.clone();

        for proposal in self.saved_proposals.values() {
            // skip proposals unless they're newer than the anchor leaf
            if proposal.data.view_number() <= self.anchor_leaf.view_number() {
                continue;
            }

            undecided_leaves.insert(
                proposal.data.view_number(),
                Leaf2::from_quorum_proposal(&proposal.data),
            );
        }

        for leaf in undecided_leaves.values() {
            let view_inner = ViewInner::Leaf {
                leaf: leaf.commit(),
                state: Arc::new(TYPES::ValidatedState::from_header(leaf.block_header())),
                delta: None,
                epoch: leaf.epoch(self.epoch_height),
            };
            let view = View { view_inner };

            undecided_state.insert(leaf.view_number(), view);
        }

        Self {
            undecided_leaves,
            undecided_state,
            ..self
        }
    }

    /// Create a `HotShotInitializer` from the given information.
    ///
    /// This function uses the anchor leaf to set the initial validated state,
    /// and populates `undecided_leaves` and `undecided_state` using `saved_proposals`.
    ///
    /// If you are able to or would prefer to set these yourself,
    /// you should use the `HotShotInitializer` constructor directly.
    #[allow(clippy::too_many_arguments)]
    pub fn load(
        instance_state: TYPES::InstanceState,
        epoch_height: u64,
        epoch_start_block: u64,
        start_epoch_info: Vec<InitializerEpochInfo<TYPES>>,
        anchor_leaf: Leaf2<TYPES>,
        (start_view, start_epoch): (TYPES::View, Option<TYPES::Epoch>),
        (high_qc, next_epoch_high_qc): (
            QuorumCertificate2<TYPES>,
            Option<NextEpochQuorumCertificate2<TYPES>>,
        ),
        saved_proposals: BTreeMap<TYPES::View, Proposal<TYPES, QuorumProposalWrapper<TYPES>>>,
        saved_vid_shares: VidShares<TYPES>,
        decided_upgrade_certificate: Option<UpgradeCertificate<TYPES>>,
        state_cert: Option<LightClientStateUpdateCertificate<TYPES>>,
    ) -> Self {
        let anchor_state = Arc::new(TYPES::ValidatedState::from_header(
            anchor_leaf.block_header(),
        ));
        let anchor_state_delta = None;

        let initializer = Self {
            instance_state,
            epoch_height,
            epoch_start_block,
            anchor_leaf,
            anchor_state,
            anchor_state_delta,
            high_qc,
            start_view,
            start_epoch,
            last_actioned_view: start_view,
            saved_proposals,
            saved_vid_shares,
            next_epoch_high_qc,
            decided_upgrade_certificate,
            undecided_leaves: BTreeMap::new(),
            undecided_state: BTreeMap::new(),
            state_cert,
            start_epoch_info,
        };

        initializer.update_undecided()
    }
}

async fn load_start_epoch_info<TYPES: NodeType>(
    membership: &Arc<RwLock<TYPES::Membership>>,
    start_epoch_info: &Vec<InitializerEpochInfo<TYPES>>,
    epoch_height: u64,
    epoch_start_block: u64,
) {
    let first_epoch_number =
        TYPES::Epoch::new(epoch_from_block_number(epoch_start_block, epoch_height));

    tracing::warn!("Calling set_first_epoch for epoch {:?}", first_epoch_number);
    membership
        .write()
        .await
        .set_first_epoch(first_epoch_number, INITIAL_DRB_RESULT);

    for epoch_info in start_epoch_info {
        if let Some(block_header) = &epoch_info.block_header {
            tracing::info!("Calling add_epoch_root for epoch {:?}", epoch_info.epoch);
            let write_callback = {
                let membership_reader = membership.read().await;
                membership_reader
                    .add_epoch_root(epoch_info.epoch, block_header.clone())
                    .await
            };

            if let Some(write_callback) = write_callback {
                let mut membership_writer = membership.write().await;
                write_callback(&mut *membership_writer);
            }
        }
    }

    for epoch_info in start_epoch_info {
        tracing::info!("Calling add_drb_result for epoch {:?}", epoch_info.epoch);
        membership
            .write()
            .await
            .add_drb_result(epoch_info.epoch, epoch_info.drb_result);
    }
}
