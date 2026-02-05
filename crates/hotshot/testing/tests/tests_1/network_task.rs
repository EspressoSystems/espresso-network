// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use async_broadcast::Sender;
use async_lock::RwLock;
use hotshot::traits::implementations::MemoryNetwork;
use hotshot_example_types::node_types::{MemoryImpl, TestTypes, TestVersions};
use hotshot_task::task::{ConsensusTaskRegistry, Task};
use hotshot_task_impls::{events::HotShotEvent, network::NetworkEventTaskState};
use hotshot_testing::{
    helpers::build_system_handle, test_builder::TestDescription,
    test_task::add_network_message_test_task, view_generator::TestViewGenerator,
};
use hotshot_types::{
    consensus::OuterConsensus,
    data::ViewNumber,
    message::UpgradeLock,
    traits::{
        election::Membership,
        node_implementation::{ConsensusTime, NodeType},
    },
};
use tokio::time::timeout;

// Test that the event task sends a message, and the message task receives it
// and emits the proper event
#[cfg(test)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
#[allow(clippy::too_many_lines)]
async fn test_network_task() {
    use std::collections::BTreeMap;

    use futures::StreamExt;
    use hotshot_types::epoch_membership::EpochMembershipCoordinator;

    let builder: TestDescription<TestTypes, MemoryImpl, TestVersions> =
        TestDescription::default_multiple_rounds();
    let upgrade_lock = UpgradeLock::<TestTypes, TestVersions>::new();
    let node_id = 1;
    let (handle, _, _, node_key_map) =
        build_system_handle::<TestTypes, MemoryImpl, TestVersions>(node_id).await;
    let launcher = builder.gen_launcher();

    let network = (launcher.resource_generators.channel_generator)(node_id).await;

    let storage = (launcher.resource_generators.storage)(node_id);
    let consensus = OuterConsensus::new(handle.hotshot.consensus());
    let config = (launcher.resource_generators.hotshot_config)(node_id);
    let validator_config = (launcher.resource_generators.validator_config)(node_id);
    let public_key = validator_config.public_key;

    let all_nodes = config.known_nodes_with_stake.clone();

    let membership = Arc::new(RwLock::new(<TestTypes as NodeType>::Membership::new::<
        MemoryImpl,
    >(
        all_nodes.clone(),
        all_nodes,
        storage.clone(),
        network.clone(),
        public_key,
        config.epoch_height,
    )));
    let coordinator =
        EpochMembershipCoordinator::new(membership, config.epoch_height, &storage.clone());
    let network_state: NetworkEventTaskState<TestTypes, TestVersions, MemoryNetwork<_>, _> =
        NetworkEventTaskState {
            id: node_id,
            network: network.clone(),
            view: ViewNumber::new(0),
            epoch: None,
            membership_coordinator: coordinator.clone(),
            upgrade_lock: upgrade_lock.clone(),
            storage,
            storage_metrics: handle.storage_metrics(),
            consensus,
            transmit_tasks: BTreeMap::new(),
            epoch_height: 0u64,
        };
    let (tx, rx) = async_broadcast::broadcast(10);
    let mut task_reg = ConsensusTaskRegistry::new();

    let task = Task::new(network_state, tx.clone(), rx);
    task_reg.run_task(task);

    let mut generator = TestViewGenerator::<TestVersions>::generate(coordinator, node_key_map);
    let view = generator.next().await.unwrap();

    let (out_tx_internal, mut out_rx_internal) = async_broadcast::broadcast(10);
    let (out_tx_external, _) = async_broadcast::broadcast(10);
    add_network_message_test_task(
        out_tx_internal.clone(),
        out_tx_external.clone(),
        upgrade_lock,
        network.clone(),
        public_key,
        node_id,
    )
    .await;

    tx.broadcast_direct(Arc::new(HotShotEvent::QuorumProposalSend(
        view.quorum_proposal,
        public_key,
    )))
    .await
    .unwrap();
    let res: Arc<HotShotEvent<TestTypes>> =
        timeout(Duration::from_millis(100), out_rx_internal.recv_direct())
            .await
            .expect("timed out waiting for response")
            .expect("channel closed");
    assert!(matches!(
        res.as_ref(),
        HotShotEvent::QuorumProposalRecv(_, _)
    ));
}

#[cfg(test)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_network_external_mnessages() {
    use hotshot::types::EventType;
    use hotshot_testing::helpers::build_system_handle_from_launcher;
    use hotshot_types::message::RecipientList;

    let builder: TestDescription<TestTypes, MemoryImpl, TestVersions> =
        TestDescription::default_multiple_rounds();

    let launcher = builder.gen_launcher();

    let mut handles = vec![];
    let mut event_streams = vec![];
    for i in 0..launcher.metadata.test_config.num_nodes_with_stake.into() {
        let handle = build_system_handle_from_launcher::<TestTypes, MemoryImpl, TestVersions>(
            i.try_into().unwrap(),
            &launcher,
        )
        .await
        .0;
        event_streams.push(handle.event_stream_known_impl());
        handles.push(handle);
    }

    // Send a message from 1 -> 2
    handles[1]
        .send_external_message(vec![1, 2], RecipientList::Direct(handles[2].public_key()))
        .await
        .unwrap();
    let event = tokio::time::timeout(Duration::from_millis(100), event_streams[2].recv())
        .await
        .unwrap()
        .unwrap()
        .event;

    // check that 2 received the message
    assert!(matches!(
        event,
        EventType::ExternalMessageReceived {
            sender,
            data,
        } if sender == handles[1].public_key() && data == vec![1, 2]
    ));

    // Send a message from 2 -> 1
    handles[2]
        .send_external_message(vec![2, 1], RecipientList::Direct(handles[1].public_key()))
        .await
        .unwrap();
    let event = tokio::time::timeout(Duration::from_millis(100), event_streams[1].recv())
        .await
        .unwrap()
        .unwrap()
        .event;

    // check that 1 received the message
    assert!(matches!(
        event,
        EventType::ExternalMessageReceived {
            sender,
            data,
        } if sender == handles[2].public_key() && data == vec![2,1]
    ));

    // Check broadcast works
    handles[0]
        .send_external_message(vec![0, 0, 0], RecipientList::Broadcast)
        .await
        .unwrap();
    // All other nodes get the broadcast
    for stream in event_streams.iter_mut().skip(1) {
        let event = tokio::time::timeout(Duration::from_millis(100), stream.recv())
            .await
            .unwrap()
            .unwrap()
            .event;
        assert!(matches!(
            event,
            EventType::ExternalMessageReceived {
                sender,
                data,
            } if sender == handles[0].public_key() && data == vec![0,0,0]
        ));
    }
    // No event on 0 even after short sleep
    tokio::time::sleep(Duration::from_millis(2)).await;
    assert!(event_streams[0].is_empty());
}

#[cfg(test)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_network_storage_fail() {
    use std::collections::BTreeMap;

    use futures::StreamExt;
    use hotshot_types::epoch_membership::EpochMembershipCoordinator;

    let builder: TestDescription<TestTypes, MemoryImpl, TestVersions> =
        TestDescription::default_multiple_rounds();
    let node_id = 1;
    let (handle, _, _, node_key_map) =
        build_system_handle::<TestTypes, MemoryImpl, TestVersions>(node_id).await;
    let launcher = builder.gen_launcher();

    let network = (launcher.resource_generators.channel_generator)(node_id).await;

    let consensus = OuterConsensus::new(handle.hotshot.consensus());
    let storage = (launcher.resource_generators.storage)(node_id);
    storage.should_return_err.store(true, Ordering::Relaxed);
    let config = (launcher.resource_generators.hotshot_config)(node_id);
    let validator_config = (launcher.resource_generators.validator_config)(node_id);
    let public_key = validator_config.public_key;
    let all_nodes = config.known_nodes_with_stake.clone();
    let upgrade_lock = UpgradeLock::<TestTypes, TestVersions>::new();

    let membership = Arc::new(RwLock::new(<TestTypes as NodeType>::Membership::new::<
        MemoryImpl,
    >(
        all_nodes.clone(),
        all_nodes,
        storage.clone(),
        network.clone(),
        public_key,
        config.epoch_height,
    )));
    let coordinator =
        EpochMembershipCoordinator::new(membership, config.epoch_height, &storage.clone());
    let network_state: NetworkEventTaskState<TestTypes, TestVersions, MemoryNetwork<_>, _> =
        NetworkEventTaskState {
            id: node_id,
            network: network.clone(),
            view: ViewNumber::new(0),
            epoch: None,
            membership_coordinator: coordinator.clone(),
            upgrade_lock: upgrade_lock.clone(),
            storage,
            storage_metrics: handle.storage_metrics(),
            consensus,
            transmit_tasks: BTreeMap::new(),
            epoch_height: 0u64,
        };
    let (tx, rx) = async_broadcast::broadcast(10);
    let mut task_reg = ConsensusTaskRegistry::new();

    let task = Task::new(network_state, tx.clone(), rx);
    task_reg.run_task(task);

    let mut generator = TestViewGenerator::<TestVersions>::generate(coordinator, node_key_map);
    let view = generator.next().await.unwrap();

    let (out_tx_internal, mut out_rx_internal): (Sender<Arc<HotShotEvent<TestTypes>>>, _) =
        async_broadcast::broadcast(10);
    let (out_tx_external, _) = async_broadcast::broadcast(10);
    add_network_message_test_task(
        out_tx_internal.clone(),
        out_tx_external.clone(),
        upgrade_lock,
        network.clone(),
        public_key,
        node_id,
    )
    .await;

    tx.broadcast_direct(Arc::new(HotShotEvent::QuorumProposalSend(
        view.quorum_proposal,
        public_key,
    )))
    .await
    .unwrap();
    let res = timeout(Duration::from_millis(100), out_rx_internal.recv_direct()).await;
    assert!(res.is_err());
}

/// Test that BlockDirectSend is transmitted over the network and received as BlockDirectRecv.
/// This verifies the direct block communication path between consecutive leaders.
#[cfg(test)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_block_direct_send_recv() {
    use std::collections::BTreeMap;

    use hotshot_example_types::{
        block_types::{TestBlockPayload, TestTransaction},
        state_types::{TestInstanceState, TestValidatedState},
    };
    use hotshot_types::{
        consensus::PayloadWithMetadata, epoch_membership::EpochMembershipCoordinator,
        traits::BlockPayload,
    };

    let builder: TestDescription<TestTypes, MemoryImpl, TestVersions> =
        TestDescription::default_multiple_rounds();
    let upgrade_lock = UpgradeLock::<TestTypes, TestVersions>::new();

    // Node 1 is the sender (current leader)
    let sender_node_id = 1;
    let (sender_handle, _, _, _) =
        build_system_handle::<TestTypes, MemoryImpl, TestVersions>(sender_node_id).await;
    let launcher = builder.gen_launcher();

    let sender_network = (launcher.resource_generators.channel_generator)(sender_node_id).await;
    let sender_storage = (launcher.resource_generators.storage)(sender_node_id);
    let sender_consensus = OuterConsensus::new(sender_handle.hotshot.consensus());
    let config = (launcher.resource_generators.hotshot_config)(sender_node_id);
    let sender_validator_config = (launcher.resource_generators.validator_config)(sender_node_id);
    let sender_public_key = sender_validator_config.public_key;

    // Node 2 is the receiver (next leader)
    let receiver_node_id = 2;
    let (_receiver_handle, _, _, _) =
        build_system_handle::<TestTypes, MemoryImpl, TestVersions>(receiver_node_id).await;
    let receiver_network = (launcher.resource_generators.channel_generator)(receiver_node_id).await;
    let receiver_validator_config =
        (launcher.resource_generators.validator_config)(receiver_node_id);
    let receiver_public_key = receiver_validator_config.public_key;

    let all_nodes = config.known_nodes_with_stake.clone();

    // Set up sender's membership and network state
    let sender_membership = Arc::new(RwLock::new(
        <TestTypes as NodeType>::Membership::new::<MemoryImpl>(
            all_nodes.clone(),
            all_nodes.clone(),
            sender_storage.clone(),
            sender_network.clone(),
            sender_public_key,
            config.epoch_height,
        ),
    ));
    let sender_coordinator = EpochMembershipCoordinator::new(
        sender_membership,
        config.epoch_height,
        &sender_storage.clone(),
    );

    let sender_network_state: NetworkEventTaskState<TestTypes, TestVersions, MemoryNetwork<_>, _> =
        NetworkEventTaskState {
            id: sender_node_id,
            network: sender_network.clone(),
            view: ViewNumber::new(0),
            epoch: None,
            membership_coordinator: sender_coordinator.clone(),
            upgrade_lock: upgrade_lock.clone(),
            storage: sender_storage,
            storage_metrics: sender_handle.storage_metrics(),
            consensus: sender_consensus,
            transmit_tasks: BTreeMap::new(),
            epoch_height: 0u64,
        };

    let (sender_tx, sender_rx) = async_broadcast::broadcast(10);
    let mut task_reg = ConsensusTaskRegistry::new();

    let sender_task = Task::new(sender_network_state, sender_tx.clone(), sender_rx);
    task_reg.run_task(sender_task);

    // Set up receiver's network message task
    let (receiver_internal_tx, mut receiver_internal_rx) = async_broadcast::broadcast(10);
    let (receiver_external_tx, _) = async_broadcast::broadcast(10);
    add_network_message_test_task(
        receiver_internal_tx.clone(),
        receiver_external_tx.clone(),
        upgrade_lock,
        receiver_network.clone(),
        receiver_public_key,
        receiver_node_id,
    )
    .await;

    // Create a test payload
    let transactions = vec![TestTransaction::new(vec![1, 2, 3])];
    let (payload, metadata) = <TestBlockPayload as BlockPayload<TestTypes>>::from_transactions(
        transactions,
        &TestValidatedState::default(),
        &TestInstanceState::default(),
    )
    .await
    .unwrap();

    let view = ViewNumber::new(5);

    // Send BlockDirectSend event from sender node
    sender_tx
        .broadcast_direct(Arc::new(HotShotEvent::BlockDirectSend(
            PayloadWithMetadata {
                payload: payload.clone(),
                metadata: metadata.clone(),
            },
            view,
            sender_public_key,
            receiver_public_key,
        )))
        .await
        .unwrap();

    // Verify receiver gets BlockDirectRecv event
    let res: Arc<HotShotEvent<TestTypes>> =
        timeout(Duration::from_millis(500), receiver_internal_rx.recv_direct())
            .await
            .expect("timed out waiting for BlockDirectRecv")
            .expect("channel closed");

    match res.as_ref() {
        HotShotEvent::BlockDirectRecv(received_payload, received_view) => {
            assert_eq!(
                *received_view, view,
                "Received view should match sent view"
            );
            assert_eq!(
                received_payload.payload, payload,
                "Received payload should match sent payload"
            );
            assert_eq!(
                received_payload.metadata, metadata,
                "Received metadata should match sent metadata"
            );
        },
        other => panic!(
            "Expected BlockDirectRecv event, got {:?}",
            std::mem::discriminant(other)
        ),
    }
}
