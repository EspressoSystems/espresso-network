// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Provides a number of tasks that run continuously

/// Provides trait to create task states from a `SystemContextHandle`
pub mod task_state;
use std::{collections::BTreeMap, fmt::Debug, sync::Arc, time::Duration};

use async_broadcast::RecvError;
use futures::future::{BoxFuture, FutureExt};
use hotshot_task::task::Task;
use hotshot_task_impls::{
    da::DaTaskState,
    events::HotShotEvent,
    network::{NetworkEventTaskState, NetworkMessageTaskState},
    request::NetworkRequestState,
    response::{NetworkResponseState, run_response_task},
    transactions::TransactionTaskState,
    upgrade::UpgradeTaskState,
    vid::VidTaskState,
    view_sync::ViewSyncTaskState,
};
use hotshot_types::{
    consensus::OuterConsensus,
    data::ViewNumber,
    message::{EXTERNAL_MESSAGE_VERSION, Message, MessageKind},
    traits::{
        network::ConnectedNetwork,
        node_implementation::{NodeImplementation, NodeType},
    },
};
use tokio::{spawn, time::sleep};
use vbs::version::Version;

use crate::{
    ConsensusApi, genesis_epoch_from_version, tasks::task_state::CreateTaskState,
    types::SystemContextHandle,
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
pub async fn add_request_network_task<TYPES: NodeType, I: NodeImplementation<TYPES>>(
    handle: &mut SystemContextHandle<TYPES, I>,
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
pub fn add_response_task<TYPES: NodeType, I: NodeImplementation<TYPES>>(
    handle: &mut SystemContextHandle<TYPES, I>,
) {
    let state = NetworkResponseState::<TYPES>::new(
        handle.hotshot.consensus(),
        handle.membership_coordinator.clone(),
        handle.public_key().clone(),
        handle.private_key().clone(),
        handle.hotshot.id,
        handle.hotshot.upgrade_lock.clone(),
    );
    handle.network_registry.register(run_response_task::<TYPES>(
        state,
        handle.internal_event_stream.1.activate_cloned(),
        handle.internal_event_stream.0.clone(),
    ));
}

/// Add a task which updates our queue length metric at a set interval
pub fn add_queue_len_task<TYPES: NodeType, I: NodeImplementation<TYPES>>(
    handle: &mut SystemContextHandle<TYPES, I>,
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
>(
    handle: &mut SystemContextHandle<TYPES, I>,
    channel: &Arc<NET>,
) {
    let upgrade_lock = handle.hotshot.upgrade_lock.clone();

    let network_state: NetworkMessageTaskState<TYPES> = NetworkMessageTaskState {
        internal_event_stream: handle.internal_event_stream.0.clone(),
        external_event_stream: handle.output_event_stream.0.clone(),
        public_key: handle.public_key().clone(),
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

                    // Deserialize the message and get the version
                    let (deserialized_message, version): (Message<TYPES>, Version) = match upgrade_lock.deserialize(&message) {
                        Ok(message) => message,
                        Err(e) => {
                            tracing::error!("Failed to deserialize message: {:?}", e);
                            continue;
                        }
                    };

                    // Special case: external messages (version 0.0). We want to make sure it is an external message
                    // and warn and continue otherwise.
                    if version == EXTERNAL_MESSAGE_VERSION
                        && !matches!(deserialized_message.kind, MessageKind::<TYPES>::External(_))
                    {
                        tracing::warn!("Received a non-external message with version 0.0");
                        continue;
                    }

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
    NET: ConnectedNetwork<TYPES::SignatureKey>,
>(
    handle: &mut SystemContextHandle<TYPES, I>,
    network: Arc<NET>,
) {
    let network_state: NetworkEventTaskState<_, _, _> = NetworkEventTaskState {
        network,
        view: ViewNumber::genesis(),
        epoch: genesis_epoch_from_version(handle.hotshot.upgrade_lock.upgrade().base),
        membership_coordinator: handle.membership_coordinator.clone(),
        storage: handle.storage(),
        storage_metrics: handle.storage_metrics(),
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
pub async fn add_consensus_tasks<TYPES: NodeType, I: NodeImplementation<TYPES>>(
    handle: &mut SystemContextHandle<TYPES, I>,
) {
    handle.add_task(ViewSyncTaskState::<TYPES>::create_from(handle).await);
    handle.add_task(VidTaskState::<TYPES, I>::create_from(handle).await);
    handle.add_task(DaTaskState::<TYPES, I>::create_from(handle).await);
    handle.add_task(TransactionTaskState::<TYPES>::create_from(handle).await);

    let upgrade = handle.hotshot.upgrade_lock.upgrade();

    // clear the loaded certificate if it's now outdated
    handle.hotshot.upgrade_lock.apply(|cert| {
        if cert
            .as_ref()
            .is_some_and(|c| upgrade.base >= c.data.new_version)
        {
            tracing::warn!("Discarding loaded upgrade certificate due to version configuration.");
            *cert = None
        }
    });

    // only spawn the upgrade task if we are actually configured to perform an upgrade.
    if upgrade.base < upgrade.target {
        tracing::warn!("Consensus was started with an upgrade configured. Spawning upgrade task.");
        handle.add_task(UpgradeTaskState::<TYPES>::create_from(handle).await);
    }

    {
        use hotshot_task_impls::{
            consensus::ConsensusTaskState, quorum_proposal::QuorumProposalTaskState,
            quorum_proposal_recv::QuorumProposalRecvTaskState, quorum_vote::QuorumVoteTaskState,
        };

        handle.add_task(QuorumProposalTaskState::<TYPES, I>::create_from(handle).await);
        handle.add_task(QuorumVoteTaskState::<TYPES, I>::create_from(handle).await);
        handle.add_task(QuorumProposalRecvTaskState::<TYPES, I>::create_from(handle).await);
        handle.add_task(ConsensusTaskState::<TYPES, I>::create_from(handle).await);
    }
    add_queue_len_task(handle);
}

/// Creates a monitor for shutdown events.
///
/// # Returns
/// A `BoxFuture<'static, ()>` that resolves when a `HotShotEvent::Shutdown` is detected.
///
/// # Usage
/// Use in `select!` macros or similar constructs for graceful shutdowns:
#[must_use]
pub fn create_shutdown_event_monitor<TYPES: NodeType, I: NodeImplementation<TYPES>>(
    handle: &SystemContextHandle<TYPES, I>,
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

/// adds tasks for sending/receiving messages to/from the network.
pub async fn add_network_tasks<TYPES: NodeType, I: NodeImplementation<TYPES>>(
    handle: &mut SystemContextHandle<TYPES, I>,
) {
    add_network_message_and_request_receiver_tasks(handle).await;

    add_network_event_tasks(handle);
}

/// Adds the `NetworkMessageTaskState` tasks and the request / receiver tasks.
pub async fn add_network_message_and_request_receiver_tasks<
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
>(
    handle: &mut SystemContextHandle<TYPES, I>,
) {
    let network = Arc::clone(&handle.network);

    add_network_message_task(handle, &network);

    add_request_network_task(handle).await;
    add_response_task(handle);
}

/// Adds the `NetworkEventTaskState` tasks.
pub fn add_network_event_tasks<TYPES: NodeType, I: NodeImplementation<TYPES>>(
    handle: &mut SystemContextHandle<TYPES, I>,
) {
    add_network_event_task(handle, Arc::clone(&handle.network));
}
