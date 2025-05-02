// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Provides a number of tasks that run continuously

/// Provides trait to create task states from a `SystemContextHandle`
pub mod task_state;
use std::{collections::BTreeMap, fmt::Debug, num::NonZeroUsize, sync::Arc, time::Duration};

use async_broadcast::{broadcast, RecvError};
use async_lock::RwLock;
use async_trait::async_trait;
use futures::{
    future::{BoxFuture, FutureExt},
    stream, StreamExt,
};
use hotshot_task::task::Task;
#[cfg(feature = "rewind")]
use hotshot_task_impls::rewind::RewindTaskState;
use hotshot_task_impls::{
    da::DaTaskState,
    events::HotShotEvent,
    network::{NetworkEventTaskState, NetworkMessageTaskState},
    request::NetworkRequestState,
    response::{run_response_task, NetworkResponseState},
    transactions::TransactionTaskState,
    upgrade::UpgradeTaskState,
    vid::VidTaskState,
    view_sync::ViewSyncTaskState,
};
use hotshot_types::{
    consensus::{Consensus, OuterConsensus},
    constants::EVENT_CHANNEL_SIZE,
    message::{Message, UpgradeLock},
    traits::{
        network::ConnectedNetwork,
        node_implementation::{ConsensusTime, NodeImplementation, NodeType},
    },
};
use tokio::{spawn, time::sleep};
use vbs::version::StaticVersionType;

use crate::{
    genesis_epoch_from_version, tasks::task_state::CreateTaskState, types::SystemContextHandle,
    ConsensusApi, ConsensusMetricsValue, ConsensusTaskRegistry, EpochMembershipCoordinator,
    HotShotConfig, HotShotInitializer, MarketplaceConfig, NetworkTaskRegistry, SignatureKey,
    StateSignatureKey, SystemContext, Versions,
};

/// event for global event stream
#[derive(Clone, Debug)]
pub enum GlobalEvent {
    /// shut everything down
    Shutdown,
    /// dummy (TODO delete later)
    Dummy,
}

/// Add tasks for network requests and responses
pub async fn add_request_network_task<
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    V: Versions,
>(
    handle: &mut SystemContextHandle<TYPES, I, V>,
) {
    let state = NetworkRequestState::<TYPES, I>::create_from(handle).await;

    let task = Task::new(
        state,
        handle.internal_event_stream.0.clone(),
        handle.internal_event_stream.1.activate_cloned(),
    );
    handle.consensus_registry.run_task(task);
}

/// Add a task which responds to requests on the network.
pub fn add_response_task<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>(
    handle: &mut SystemContextHandle<TYPES, I, V>,
) {
    let state = NetworkResponseState::<TYPES, V>::new(
        handle.hotshot.consensus(),
        handle.membership_coordinator.clone(),
        handle.public_key().clone(),
        handle.private_key().clone(),
        handle.hotshot.id,
        handle.hotshot.upgrade_lock.clone(),
    );
    handle
        .network_registry
        .register(run_response_task::<TYPES, V>(
            state,
            handle.internal_event_stream.1.activate_cloned(),
            handle.internal_event_stream.0.clone(),
        ));
}

/// Add a task which updates our queue length metric at a set interval
pub fn add_queue_len_task<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>(
    handle: &mut SystemContextHandle<TYPES, I, V>,
) {
    let consensus = handle.hotshot.consensus();
    let rx = handle.internal_event_stream.1.clone();
    let shutdown_signal = create_shutdown_event_monitor(handle).fuse();
    let task_handle = spawn(async move {
        futures::pin_mut!(shutdown_signal);
        loop {
            futures::select! {
                () = shutdown_signal => {
                    return;
                },
                () = sleep(Duration::from_millis(500)).fuse() => {
                    consensus.read().await.metrics.internal_event_queue_len.set(rx.len());
                }
            }
        }
    });
    handle.network_registry.register(task_handle);
}

/// Add the network task to handle messages and publish events.
#[allow(clippy::missing_panics_doc)]
pub fn add_network_message_task<
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    NET: ConnectedNetwork<TYPES::SignatureKey>,
    V: Versions,
>(
    handle: &mut SystemContextHandle<TYPES, I, V>,
    channel: &Arc<NET>,
) {
    let upgrade_lock = handle.hotshot.upgrade_lock.clone();

    let network_state: NetworkMessageTaskState<TYPES, V> = NetworkMessageTaskState {
        internal_event_stream: handle.internal_event_stream.0.clone(),
        external_event_stream: handle.output_event_stream.0.clone(),
        public_key: handle.public_key().clone(),
        transactions_cache: lru::LruCache::new(NonZeroUsize::new(100_000).unwrap()),
        upgrade_lock: upgrade_lock.clone(),
        id: handle.hotshot.id,
    };

    let network = Arc::clone(channel);
    let mut state = network_state.clone();
    let shutdown_signal = create_shutdown_event_monitor(handle).fuse();
    let task_handle = spawn(async move {
        futures::pin_mut!(shutdown_signal);

        loop {
            // Wait for one of the following to resolve:
            futures::select! {
                // Wait for a shutdown signal
                () = shutdown_signal => {
                    tracing::error!("Shutting down network message task");
                    return;
                }

                // Wait for a message from the network
                message = network.recv_message().fuse() => {
                    // Make sure the message did not fail
                    let Ok(message) = message else {
                        continue;
                    };

                    // Deserialize the message
                    let deserialized_message: Message<TYPES> = match upgrade_lock.deserialize(&message).await {
                        Ok(message) => message,
                        Err(e) => {
                            tracing::error!("Failed to deserialize message: {:?}", e);
                            continue;
                        }
                    };

                    // Handle the message
                    state.handle_message(deserialized_message).await;
                }
            }
        }
    });
    handle.network_registry.register(task_handle);
}

/// Add the network task to handle events and send messages.
pub fn add_network_event_task<
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    V: Versions,
    NET: ConnectedNetwork<TYPES::SignatureKey>,
>(
    handle: &mut SystemContextHandle<TYPES, I, V>,
    network: Arc<NET>,
) {
    let network_state: NetworkEventTaskState<_, V, _, _> = NetworkEventTaskState {
        network,
        view: TYPES::View::genesis(),
        epoch: genesis_epoch_from_version::<V, TYPES>(),
        membership_coordinator: handle.membership_coordinator.clone(),
        storage: Arc::clone(&handle.storage()),
        consensus: OuterConsensus::new(handle.consensus()),
        upgrade_lock: handle.hotshot.upgrade_lock.clone(),
        transmit_tasks: BTreeMap::new(),
        epoch_height: handle.epoch_height,
        id: handle.hotshot.id,
    };
    let task = Task::new(
        network_state,
        handle.internal_event_stream.0.clone(),
        handle.internal_event_stream.1.activate_cloned(),
    );
    handle.consensus_registry.run_task(task);
}

/// Adds consensus-related tasks to a `SystemContextHandle`.
pub async fn add_consensus_tasks<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>(
    handle: &mut SystemContextHandle<TYPES, I, V>,
) {
    handle.add_task(ViewSyncTaskState::<TYPES, V>::create_from(handle).await);
    handle.add_task(VidTaskState::<TYPES, I, V>::create_from(handle).await);
    handle.add_task(DaTaskState::<TYPES, I, V>::create_from(handle).await);
    handle.add_task(TransactionTaskState::<TYPES, I, V>::create_from(handle).await);

    {
        let mut upgrade_certificate_lock = handle
            .hotshot
            .upgrade_lock
            .decided_upgrade_certificate
            .write()
            .await;

        // clear the loaded certificate if it's now outdated
        if upgrade_certificate_lock
            .as_ref()
            .is_some_and(|cert| V::Base::VERSION >= cert.data.new_version)
        {
            tracing::warn!("Discarding loaded upgrade certificate due to version configuration.");
            *upgrade_certificate_lock = None;
        }
    }

    // only spawn the upgrade task if we are actually configured to perform an upgrade.
    if V::Base::VERSION < V::Upgrade::VERSION {
        tracing::warn!("Consensus was started with an upgrade configured. Spawning upgrade task.");
        handle.add_task(UpgradeTaskState::<TYPES, V>::create_from(handle).await);
    }

    {
        use hotshot_task_impls::{
            consensus::ConsensusTaskState, quorum_proposal::QuorumProposalTaskState,
            quorum_proposal_recv::QuorumProposalRecvTaskState, quorum_vote::QuorumVoteTaskState,
        };

        handle.add_task(QuorumProposalTaskState::<TYPES, I, V>::create_from(handle).await);
        handle.add_task(QuorumVoteTaskState::<TYPES, I, V>::create_from(handle).await);
        handle.add_task(QuorumProposalRecvTaskState::<TYPES, I, V>::create_from(handle).await);
        handle.add_task(ConsensusTaskState::<TYPES, I, V>::create_from(handle).await);
    }
    add_queue_len_task(handle);
    #[cfg(feature = "rewind")]
    handle.add_task(RewindTaskState::<TYPES>::create_from(&handle).await);
}

/// Creates a monitor for shutdown events.
///
/// # Returns
/// A `BoxFuture<'static, ()>` that resolves when a `HotShotEvent::Shutdown` is detected.
///
/// # Usage
/// Use in `select!` macros or similar constructs for graceful shutdowns:
#[must_use]
pub fn create_shutdown_event_monitor<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>(
    handle: &SystemContextHandle<TYPES, I, V>,
) -> BoxFuture<'static, ()> {
    // Activate the cloned internal event stream
    let mut event_stream = handle.internal_event_stream.1.activate_cloned();

    // Create a future that completes when the `HotShotEvent::Shutdown` is received
    async move {
        loop {
            match event_stream.recv_direct().await {
                Ok(event) => {
                    if matches!(event.as_ref(), HotShotEvent::Shutdown) {
                        return;
                    }
                },
                Err(RecvError::Closed) => {
                    return;
                },
                Err(e) => {
                    tracing::error!("Shutdown event monitor channel recv error: {}", e);
                },
            }
        }
    }
    .boxed()
}

#[async_trait]
/// Trait for intercepting and modifying messages between the network and consensus layers.
///
/// Consensus <-> [Byzantine logic layer] <-> Network
pub trait EventTransformerState<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>
where
    Self: std::fmt::Debug + Send + Sync + 'static,
{
    /// modify incoming messages from the network
    async fn recv_handler(&mut self, event: &HotShotEvent<TYPES>) -> Vec<HotShotEvent<TYPES>>;

    /// modify outgoing messages from the network
    async fn send_handler(
        &mut self,
        event: &HotShotEvent<TYPES>,
        public_key: &TYPES::SignatureKey,
        private_key: &<TYPES::SignatureKey as SignatureKey>::PrivateKey,
        upgrade_lock: &UpgradeLock<TYPES, V>,
        consensus: Arc<RwLock<Consensus<TYPES>>>,
    ) -> Vec<HotShotEvent<TYPES>>;

    #[allow(clippy::too_many_arguments)]
    /// Creates a `SystemContextHandle` with the given even transformer
    async fn spawn_handle(
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
    ) -> SystemContextHandle<TYPES, I, V> {
        let epoch_height = config.epoch_height;

        let hotshot = SystemContext::new(
            public_key,
            private_key,
            state_private_key,
            nonce,
            config,
            memberships.clone(),
            network,
            initializer,
            metrics,
            storage,
            marketplace_config,
        )
        .await;
        let consensus_registry = ConsensusTaskRegistry::new();
        let network_registry = NetworkTaskRegistry::new();

        let output_event_stream = hotshot.external_event_stream.clone();
        let internal_event_stream = hotshot.internal_event_stream.clone();

        let mut handle = SystemContextHandle {
            consensus_registry,
            network_registry,
            output_event_stream: output_event_stream.clone(),
            internal_event_stream: internal_event_stream.clone(),
            hotshot: Arc::clone(&hotshot),
            storage: Arc::clone(&hotshot.storage),
            network: Arc::clone(&hotshot.network),
            membership_coordinator: memberships.clone(),
            epoch_height,
        };

        add_consensus_tasks::<TYPES, I, V>(&mut handle).await;
        self.add_network_tasks(&mut handle).await;

        handle
    }

    /// Add byzantine network tasks with the trait
    #[allow(clippy::too_many_lines)]
    async fn add_network_tasks(&'static mut self, handle: &mut SystemContextHandle<TYPES, I, V>) {
        // channels between the task spawned in this function and the network tasks.
        // with this, we can control exactly what events the network tasks see.

        // channel to the network task
        let (sender_to_network, network_task_receiver) = broadcast(EVENT_CHANNEL_SIZE);
        // channel from the network task
        let (network_task_sender, receiver_from_network) = broadcast(EVENT_CHANNEL_SIZE);
        // create a copy of the original receiver
        let (original_sender, original_receiver) = (
            handle.internal_event_stream.0.clone(),
            handle.internal_event_stream.1.activate_cloned(),
        );

        // replace the internal event stream with the one we just created,
        // so that the network tasks are spawned with our channel.
        let mut internal_event_stream = (
            network_task_sender.clone(),
            network_task_receiver.clone().deactivate(),
        );
        std::mem::swap(
            &mut internal_event_stream,
            &mut handle.internal_event_stream,
        );

        // spawn the network tasks with our newly-created channel
        add_network_message_and_request_receiver_tasks(handle).await;
        self.add_network_event_tasks(handle);

        std::mem::swap(
            &mut internal_event_stream,
            &mut handle.internal_event_stream,
        );

        let state_in = Arc::new(RwLock::new(self));
        let state_out = Arc::clone(&state_in);
        // spawn a task to listen on the (original) internal event stream,
        // and broadcast the transformed events to the replacement event stream we just created.
        let shutdown_signal = create_shutdown_event_monitor(handle).fuse();
        let public_key = handle.public_key().clone();
        let private_key = handle.private_key().clone();
        let upgrade_lock = handle.hotshot.upgrade_lock.clone();
        let consensus = Arc::clone(&handle.hotshot.consensus());
        let send_handle = spawn(async move {
            futures::pin_mut!(shutdown_signal);

            let recv_stream = stream::unfold(original_receiver, |mut recv| async move {
                match recv.recv().await {
                    Ok(event) => Some((Ok(event), recv)),
                    Err(async_broadcast::RecvError::Closed) => None,
                    Err(e) => Some((Err(e), recv)),
                }
            })
            .boxed();

            let fused_recv_stream = recv_stream.fuse();
            futures::pin_mut!(fused_recv_stream);

            loop {
                futures::select! {
                    () = shutdown_signal => {
                        tracing::error!("Shutting down relay send task");
                        let _ = sender_to_network.broadcast(HotShotEvent::<TYPES>::Shutdown.into()).await;
                        return;
                    }
                    event = fused_recv_stream.next() => {
                        match event {
                            Some(Ok(msg)) => {
                                let mut state = state_out.write().await;
                                let mut results = state.send_handler(
                                    &msg,
                                    &public_key,
                                    &private_key,
                                    &upgrade_lock,
                                    Arc::clone(&consensus)
                                ).await;
                                results.reverse();
                                while let Some(event) = results.pop() {
                                    let _ = sender_to_network.broadcast(event.into()).await;
                                }
                            }
                            Some(Err(e)) => {
                                tracing::error!("Relay Task, send_handle, Error receiving event: {:?}", e);
                            }
                            None => {
                                tracing::info!("Relay Task, send_handle, Event stream closed");
                                return;
                            }
                        }
                    }
                }
            }
        });

        // spawn a task to listen on the newly created event stream,
        // and broadcast the transformed events to the original internal event stream
        let shutdown_signal = create_shutdown_event_monitor(handle).fuse();
        let recv_handle = spawn(async move {
            futures::pin_mut!(shutdown_signal);

            let network_recv_stream =
                stream::unfold(receiver_from_network, |mut recv| async move {
                    match recv.recv().await {
                        Ok(event) => Some((Ok(event), recv)),
                        Err(async_broadcast::RecvError::Closed) => None,
                        Err(e) => Some((Err(e), recv)),
                    }
                });

            let fused_network_recv_stream = network_recv_stream.boxed().fuse();
            futures::pin_mut!(fused_network_recv_stream);

            loop {
                futures::select! {
                    () = shutdown_signal => {
                        tracing::error!("Shutting down relay receive task");
                        return;
                    }
                    event = fused_network_recv_stream.next() => {
                        match event {
                            Some(Ok(msg)) => {
                                let mut state = state_in.write().await;
                                let mut results = state.recv_handler(&msg).await;
                                results.reverse();
                                while let Some(event) = results.pop() {
                                    let _ = original_sender.broadcast(event.into()).await;
                                }
                            }
                            Some(Err(e)) => {
                                tracing::error!("Relay Task, recv_handle, Error receiving event from network: {:?}", e);
                            }
                            None => {
                                tracing::info!("Relay Task, recv_handle, Network event stream closed");
                                return;
                            }
                        }
                    }
                }
            }
        });

        handle.network_registry.register(send_handle);
        handle.network_registry.register(recv_handle);
    }

    /// Adds the `NetworkEventTaskState` tasks possibly modifying them as well.
    fn add_network_event_tasks(&self, handle: &mut SystemContextHandle<TYPES, I, V>) {
        let network = Arc::clone(&handle.network);

        self.add_network_event_task(handle, network);
    }

    /// Adds a `NetworkEventTaskState` task. Can be reimplemented to modify its behaviour.
    fn add_network_event_task(
        &self,
        handle: &mut SystemContextHandle<TYPES, I, V>,
        channel: Arc<<I as NodeImplementation<TYPES>>::Network>,
    ) {
        add_network_event_task(handle, channel);
    }
}

/// adds tasks for sending/receiving messages to/from the network.
pub async fn add_network_tasks<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>(
    handle: &mut SystemContextHandle<TYPES, I, V>,
) {
    add_network_message_and_request_receiver_tasks(handle).await;

    add_network_event_tasks(handle);
}

/// Adds the `NetworkMessageTaskState` tasks and the request / receiver tasks.
pub async fn add_network_message_and_request_receiver_tasks<
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    V: Versions,
>(
    handle: &mut SystemContextHandle<TYPES, I, V>,
) {
    let network = Arc::clone(&handle.network);

    add_network_message_task(handle, &network);

    add_request_network_task(handle).await;
    add_response_task(handle);
}

/// Adds the `NetworkEventTaskState` tasks.
pub fn add_network_event_tasks<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>(
    handle: &mut SystemContextHandle<TYPES, I, V>,
) {
    add_network_event_task(handle, Arc::clone(&handle.network));
}
