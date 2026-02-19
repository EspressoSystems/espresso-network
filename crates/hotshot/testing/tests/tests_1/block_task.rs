// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::sync::Arc;

use hotshot::tasks::task_state::CreateTaskState;
use hotshot_example_types::{
    block_types::{TestBlockPayload, TestTransaction},
    node_types::{MemoryImpl, TestConsecutiveLeaderTypes, TestTypes, TestVersions},
    state_types::{TestInstanceState, TestValidatedState},
};
use hotshot_task_impls::{block::BlockTaskState, events::HotShotEvent};
use hotshot_testing::helpers::build_system_handle;
use hotshot_types::{
    consensus::PayloadWithMetadata,
    data::{EpochNumber, VidCommitment, ViewNumber},
    traits::{node_implementation::ConsensusTime, BlockPayload},
};

/// BlockReconstructed saves the payload to consensus.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_block_reconstructed_saves_payload() {
    let handle = build_system_handle::<TestTypes, MemoryImpl, TestVersions>(2)
        .await
        .0;

    let mut block_state = BlockTaskState::<TestTypes, TestVersions>::create_from(&handle).await;

    let transactions = vec![TestTransaction::new(vec![1, 2, 3])];
    let (payload, metadata) = <TestBlockPayload as BlockPayload<TestTypes>>::from_transactions(
        transactions.clone(),
        &TestValidatedState::default(),
        &TestInstanceState::default(),
    )
    .await
    .unwrap();

    let view = ViewNumber::new(5);
    let vid_commitment = VidCommitment::default();

    // Create a broadcast channel pair for the task
    let (sender, receiver) = async_broadcast::broadcast(128);

    // Send BlockReconstructed event
    let event = Arc::new(HotShotEvent::BlockReconstructed(
        payload.clone(),
        metadata,
        vid_commitment,
        view,
    ));

    block_state
        .handle(event, sender.clone(), receiver.clone())
        .await
        .expect("handle should not error");

    // Verify the payload was saved to consensus
    let saved = block_state
        .consensus
        .read()
        .await
        .saved_payloads()
        .get(&view)
        .cloned();

    assert!(
        saved.is_some(),
        "Payload should be saved after BlockReconstructed"
    );
    let saved = saved.unwrap();
    assert_eq!(
        saved.payload, payload,
        "Saved payload should match the reconstructed block"
    );
}

/// BlockDirectlyRecv saves the payload to consensus.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_block_directly_recv_saves_payload() {
    let handle = build_system_handle::<TestTypes, MemoryImpl, TestVersions>(2)
        .await
        .0;

    let mut block_state = BlockTaskState::<TestTypes, TestVersions>::create_from(&handle).await;

    let transactions = vec![TestTransaction::new(vec![4, 5, 6])];
    let (payload, metadata) = <TestBlockPayload as BlockPayload<TestTypes>>::from_transactions(
        transactions.clone(),
        &TestValidatedState::default(),
        &TestInstanceState::default(),
    )
    .await
    .unwrap();

    let view = ViewNumber::new(5);

    // Create a broadcast channel pair for the task
    let (sender, receiver) = async_broadcast::broadcast(128);

    // Send BlockDirectRecv event
    let event = Arc::new(HotShotEvent::BlockDirectRecv(
        PayloadWithMetadata {
            payload: payload.clone(),
            metadata,
        },
        view,
    ));

    block_state
        .handle(event, sender.clone(), receiver.clone())
        .await
        .expect("handle should not error");

    // Verify the payload was saved to consensus
    let saved = block_state
        .consensus
        .read()
        .await
        .saved_payloads()
        .get(&view)
        .cloned();

    assert!(
        saved.is_some(),
        "Payload should be saved after BlockDirectlyRecv"
    );
    let saved = saved.unwrap();
    assert_eq!(
        saved.payload, payload,
        "Saved payload should match the directly received block"
    );
}

/// wait_for_previous_block via BlockDirectlyRecv, same leader for consecutive views.
/// Node 2 leads views 4 and 5 (TestConsecutiveLeaderTypes).
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_wait_for_previous_block_via_direct_recv() {
    let node_id = 2;
    let handle =
        build_system_handle::<TestConsecutiveLeaderTypes, MemoryImpl, TestVersions>(node_id)
            .await
            .0;

    let mut block_state =
        BlockTaskState::<TestConsecutiveLeaderTypes, TestVersions>::create_from(&handle).await;

    // Block payload for view 3 (previous view)
    let prev_transactions = vec![TestTransaction::new(vec![10, 20, 30])];
    let (prev_payload, prev_metadata) =
        <TestBlockPayload as BlockPayload<TestConsecutiveLeaderTypes>>::from_transactions(
            prev_transactions.clone(),
            &TestValidatedState::default(),
            &TestInstanceState::default(),
        )
        .await
        .unwrap();

    let (sender, receiver) = async_broadcast::broadcast(128);

    // Step 1: Receive block directly from previous leader for view 3
    let direct_recv_event = Arc::new(HotShotEvent::BlockDirectRecv(
        PayloadWithMetadata {
            payload: prev_payload.clone(),
            metadata: prev_metadata.clone(),
        },
        ViewNumber::new(3),
    ));
    block_state
        .handle(direct_recv_event, sender.clone(), receiver.clone())
        .await
        .expect("handle BlockDirectRecv should not error");

    // Verify the previous block is saved
    let saved_before = block_state
        .consensus
        .read()
        .await
        .saved_payloads()
        .get(&ViewNumber::new(3))
        .cloned();
    assert!(
        saved_before.is_some(),
        "Previous block should be in saved_payloads before leader builds"
    );

    // Step 2: ViewChange triggers leader flow; wait_for_previous_block finds saved payload
    let view_change_event = Arc::new(HotShotEvent::ViewChange(
        ViewNumber::new(4),
        Some(EpochNumber::new(1)),
    ));
    block_state
        .handle(view_change_event, sender.clone(), receiver.clone())
        .await
        .expect("handle ViewChange should not error");

    // Should emit BlockRecv (empty block fallback since no validated_state)
    let mut found_block_recv = false;
    while let Ok(event) = receiver.clone().try_recv() {
        if matches!(event.as_ref(), HotShotEvent::BlockRecv(_)) {
            found_block_recv = true;
            break;
        }
    }
    assert!(
        found_block_recv,
        "Leader should have emitted BlockRecv after processing ViewChange"
    );

    // The previous block at view 3 should still be in saved_payloads
    let saved_after = block_state
        .consensus
        .read()
        .await
        .saved_payloads()
        .get(&ViewNumber::new(3))
        .cloned();
    assert!(
        saved_after.is_some(),
        "Previous block should still be in saved_payloads after leader builds"
    );
    assert_eq!(
        saved_after.unwrap().payload,
        prev_payload,
        "Saved previous block payload should be unchanged"
    );
}

/// wait_for_previous_block via BlockReconstructed, same leader for consecutive views.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_wait_for_previous_block_via_reconstruction() {
    let node_id = 2;
    let handle =
        build_system_handle::<TestConsecutiveLeaderTypes, MemoryImpl, TestVersions>(node_id)
            .await
            .0;

    let mut block_state =
        BlockTaskState::<TestConsecutiveLeaderTypes, TestVersions>::create_from(&handle).await;

    let prev_transactions = vec![TestTransaction::new(vec![7, 8, 9])];
    let (prev_payload, prev_metadata) =
        <TestBlockPayload as BlockPayload<TestConsecutiveLeaderTypes>>::from_transactions(
            prev_transactions.clone(),
            &TestValidatedState::default(),
            &TestInstanceState::default(),
        )
        .await
        .unwrap();

    let (sender, receiver) = async_broadcast::broadcast(128);

    // Step 1: Send BlockReconstructed for view 3
    let reconstructed_event = Arc::new(HotShotEvent::BlockReconstructed(
        prev_payload.clone(),
        prev_metadata.clone(),
        VidCommitment::default(),
        ViewNumber::new(3),
    ));
    block_state
        .handle(reconstructed_event, sender.clone(), receiver.clone())
        .await
        .expect("handle BlockReconstructed should not error");

    // Step 2: Send ViewChange for view 4
    let view_change_event = Arc::new(HotShotEvent::ViewChange(
        ViewNumber::new(4),
        Some(EpochNumber::new(1)),
    ));
    block_state
        .handle(view_change_event, sender.clone(), receiver.clone())
        .await
        .expect("handle ViewChange should not error");

    // Verify a BlockRecv was emitted (empty block since no validated_state for view 3)
    let mut found_block_recv = false;
    while let Ok(event) = receiver.clone().try_recv() {
        if matches!(event.as_ref(), HotShotEvent::BlockRecv(_)) {
            found_block_recv = true;
            break;
        }
    }
    assert!(
        found_block_recv,
        "Leader should have emitted BlockRecv after processing ViewChange (via reconstructed \
         previous block path)"
    );

    // Previous block should be in saved_payloads
    let saved = block_state
        .consensus
        .read()
        .await
        .saved_payloads()
        .get(&ViewNumber::new(3))
        .cloned();
    assert!(saved.is_some(), "Reconstructed block should be saved");
    assert_eq!(saved.unwrap().payload, prev_payload);
}

/// wait_for_previous_block via BlockDirectlyRecv, different leaders for consecutive views.
/// Node 3 leads view 3, node 4 leads view 4 (TestTypes).
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_wait_for_previous_block_different_leader_direct_recv() {
    let node_id = 4;
    let handle = build_system_handle::<TestTypes, MemoryImpl, TestVersions>(node_id)
        .await
        .0;

    let mut block_state = BlockTaskState::<TestTypes, TestVersions>::create_from(&handle).await;

    let prev_transactions = vec![TestTransaction::new(vec![11, 22, 33])];
    let (prev_payload, prev_metadata) =
        <TestBlockPayload as BlockPayload<TestTypes>>::from_transactions(
            prev_transactions.clone(),
            &TestValidatedState::default(),
            &TestInstanceState::default(),
        )
        .await
        .unwrap();

    let (sender, receiver) = async_broadcast::broadcast(128);

    // Step 1: Receive block from previous leader (node 3) for view 3
    let direct_recv_event = Arc::new(HotShotEvent::BlockDirectRecv(
        PayloadWithMetadata {
            payload: prev_payload.clone(),
            metadata: prev_metadata.clone(),
        },
        ViewNumber::new(3),
    ));
    block_state
        .handle(direct_recv_event, sender.clone(), receiver.clone())
        .await
        .expect("handle BlockDirectRecv should not error");

    // Step 2: ViewChange for view 4
    let view_change_event = Arc::new(HotShotEvent::ViewChange(
        ViewNumber::new(4),
        Some(EpochNumber::new(1)),
    ));
    block_state
        .handle(view_change_event, sender.clone(), receiver.clone())
        .await
        .expect("handle ViewChange should not error");

    // Verify BlockRecv was emitted (empty block fallback since no validated_state)
    let mut found_block_recv = false;
    while let Ok(event) = receiver.clone().try_recv() {
        if matches!(event.as_ref(), HotShotEvent::BlockRecv(_)) {
            found_block_recv = true;
            break;
        }
    }
    assert!(
        found_block_recv,
        "Leader (node 4) should have emitted BlockRecv after receiving previous block from node 3"
    );

    // Previous block should still be saved
    let saved = block_state
        .consensus
        .read()
        .await
        .saved_payloads()
        .get(&ViewNumber::new(3))
        .cloned();
    assert!(
        saved.is_some(),
        "Previous block from node 3 should be saved"
    );
    assert_eq!(saved.unwrap().payload, prev_payload);
}

/// wait_for_previous_block via BlockReconstructed, different leaders for consecutive views.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_wait_for_previous_block_different_leader_reconstruction() {
    let node_id = 4;
    let handle = build_system_handle::<TestTypes, MemoryImpl, TestVersions>(node_id)
        .await
        .0;

    let mut block_state = BlockTaskState::<TestTypes, TestVersions>::create_from(&handle).await;

    let prev_transactions = vec![TestTransaction::new(vec![44, 55, 66])];
    let (prev_payload, prev_metadata) =
        <TestBlockPayload as BlockPayload<TestTypes>>::from_transactions(
            prev_transactions.clone(),
            &TestValidatedState::default(),
            &TestInstanceState::default(),
        )
        .await
        .unwrap();

    let (sender, receiver) = async_broadcast::broadcast(128);

    // Step 1: BlockReconstructed for view 3 (reconstructed from VID shares)
    let reconstructed_event = Arc::new(HotShotEvent::BlockReconstructed(
        prev_payload.clone(),
        prev_metadata.clone(),
        VidCommitment::default(),
        ViewNumber::new(3),
    ));
    block_state
        .handle(reconstructed_event, sender.clone(), receiver.clone())
        .await
        .expect("handle BlockReconstructed should not error");

    // Step 2: ViewChange for view 4 — node 4 is leader
    let view_change_event = Arc::new(HotShotEvent::ViewChange(
        ViewNumber::new(4),
        Some(EpochNumber::new(1)),
    ));
    block_state
        .handle(view_change_event, sender.clone(), receiver.clone())
        .await
        .expect("handle ViewChange should not error");

    let mut found_block_recv = false;
    while let Ok(event) = receiver.clone().try_recv() {
        if matches!(event.as_ref(), HotShotEvent::BlockRecv(_)) {
            found_block_recv = true;
            break;
        }
    }
    assert!(
        found_block_recv,
        "Leader (node 4) should have emitted BlockRecv after reconstructed previous block"
    );

    let saved = block_state
        .consensus
        .read()
        .await
        .saved_payloads()
        .get(&ViewNumber::new(3))
        .cloned();
    assert!(saved.is_some(), "Reconstructed block should be saved");
    assert_eq!(saved.unwrap().payload, prev_payload);
}

/// End-to-end test: VidTask emits BlockDirectSend → Network → BlockTask receives BlockDirectRecv.
/// Verifies:
/// 1. VidTask emits BlockDirectSend when processing BlockRecv (current leader sends to next leader)
/// 2. Network transmits the message to the next leader
/// 3. BlockTask receives BlockDirectRecv and saves payload to consensus
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_direct_block_end_to_end_vid_to_block_task() {
    use std::{collections::BTreeMap, time::Duration};

    use hotshot::traits::implementations::MemoryNetwork;
    use hotshot_task::task::{ConsensusTaskRegistry, Task};
    use hotshot_task_impls::{network::NetworkEventTaskState, vid::VidTaskState};
    use hotshot_testing::{
        test_builder::TestDescription, test_task::add_network_message_test_task,
    };
    use hotshot_types::{
        data::PackedBundle,
        epoch_membership::EpochMembershipCoordinator,
        message::UpgradeLock,
        traits::{election::Membership, BlockPayload, EncodeBytes},
    };
    use tokio::time::timeout;
    use vbs::version::StaticVersionType;

    // === Setup: Two nodes - current leader (node 2) and next leader (node 3) ===
    let builder: TestDescription<TestTypes, MemoryImpl, TestVersions> =
        TestDescription::default_multiple_rounds();
    let upgrade_lock = UpgradeLock::<TestTypes, TestVersions>::new();
    let launcher = builder.gen_launcher();

    // Current leader (node 2) - will send block directly to next leader
    let current_leader_id = 2u64;
    let current_leader_handle =
        build_system_handle::<TestTypes, MemoryImpl, TestVersions>(current_leader_id)
            .await
            .0;
    let current_leader_key = current_leader_handle.public_key();
    let current_leader_network =
        (launcher.resource_generators.channel_generator)(current_leader_id).await;
    let current_leader_storage = (launcher.resource_generators.storage)(current_leader_id);
    let config = (launcher.resource_generators.hotshot_config)(current_leader_id);
    let all_nodes = config.known_nodes_with_stake.clone();

    // Next leader (node 3) - will receive block directly
    let next_leader_id = 3u64;
    let next_leader_handle =
        build_system_handle::<TestTypes, MemoryImpl, TestVersions>(next_leader_id)
            .await
            .0;
    let next_leader_key = next_leader_handle.public_key();
    let next_leader_network =
        (launcher.resource_generators.channel_generator)(next_leader_id).await;

    // === Setup current leader's VidTask and NetworkEventTask ===
    let current_leader_membership = std::sync::Arc::new(async_lock::RwLock::new(
        <TestTypes as hotshot_types::traits::node_implementation::NodeType>::Membership::new::<
            MemoryImpl,
        >(
            all_nodes.clone(),
            all_nodes.clone(),
            current_leader_storage.clone(),
            current_leader_network.clone(),
            current_leader_key,
            config.epoch_height,
        ),
    ));
    let current_leader_coordinator = EpochMembershipCoordinator::new(
        current_leader_membership,
        config.epoch_height,
        &current_leader_storage,
    );

    let current_leader_consensus =
        hotshot_types::consensus::OuterConsensus::new(current_leader_handle.hotshot.consensus());

    // NetworkEventTask for current leader (to send messages)
    let current_leader_network_state: NetworkEventTaskState<
        TestTypes,
        TestVersions,
        MemoryNetwork<_>,
        _,
    > = NetworkEventTaskState {
        id: current_leader_id,
        network: current_leader_network.clone(),
        view: ViewNumber::new(0),
        epoch: None,
        membership_coordinator: current_leader_coordinator.clone(),
        upgrade_lock: upgrade_lock.clone(),
        storage: current_leader_storage.clone(),
        storage_metrics: current_leader_handle.storage_metrics(),
        consensus: current_leader_consensus.clone(),
        transmit_tasks: BTreeMap::new(),
        epoch_height: 0u64,
    };

    let (current_leader_tx, current_leader_rx) = async_broadcast::broadcast(10);
    let mut task_reg = ConsensusTaskRegistry::new();
    let network_task = Task::new(
        current_leader_network_state,
        current_leader_tx.clone(),
        current_leader_rx.clone(),
    );
    task_reg.run_task(network_task);

    // VidTask for current leader
    let mut vid_state =
        VidTaskState::<TestTypes, MemoryImpl, TestVersions>::create_from(&current_leader_handle)
            .await;

    // === Setup next leader's BlockTask and NetworkMessageTask ===
    let mut block_state =
        BlockTaskState::<TestTypes, TestVersions>::create_from(&next_leader_handle).await;

    // NetworkMessageTask for next leader (to receive messages)
    let (next_leader_internal_tx, mut next_leader_internal_rx) = async_broadcast::broadcast(10);
    let (next_leader_external_tx, _) = async_broadcast::broadcast(10);
    add_network_message_test_task(
        next_leader_internal_tx.clone(),
        next_leader_external_tx,
        upgrade_lock,
        next_leader_network,
        next_leader_key,
        next_leader_id,
    )
    .await;

    // === Create test payload with transactions ===
    let transactions = vec![
        TestTransaction::new(vec![1, 2, 3]),
        TestTransaction::new(vec![4, 5, 6]),
        TestTransaction::new(vec![7, 8, 9]),
    ];
    let (payload, metadata) = <TestBlockPayload as BlockPayload<TestTypes>>::from_transactions(
        transactions.clone(),
        &TestValidatedState::default(),
        &TestInstanceState::default(),
    )
    .await
    .unwrap();

    // View 2: current leader (node 2) is leader, next leader (node 3) is leader for view 3
    let view = ViewNumber::new(2);

    // === Step 1: VidTask processes BlockRecv and should emit BlockDirectSend ===
    let packed_bundle = PackedBundle::new(
        payload.encode(),
        metadata.clone(),
        view,
        None, // epoch_number
        vec1::vec1![hotshot_types::data::null_block::builder_fee::<TestTypes, TestVersions>(
            7,
            <TestVersions as hotshot_types::traits::node_implementation::Versions>::Base::VERSION,
        )
        .unwrap()],
    );

    // Send BlockRecv to VidTask - this should trigger BlockDirectSend emission
    vid_state
        .handle(
            std::sync::Arc::new(HotShotEvent::BlockRecv(packed_bundle)),
            current_leader_tx.clone(),
        )
        .await;

    // Wait briefly for network transmission
    tokio::time::sleep(Duration::from_millis(50)).await;

    // === Step 2: Verify next leader receives BlockDirectRecv ===
    let received_event = timeout(
        Duration::from_millis(500),
        next_leader_internal_rx.recv_direct(),
    )
    .await
    .expect("timed out waiting for BlockDirectRecv")
    .expect("channel closed");

    let (received_payload, received_view) = match received_event.as_ref() {
        HotShotEvent::BlockDirectRecv(payload, view) => (payload.clone(), *view),
        other => panic!(
            "Expected BlockDirectRecv, got {:?}",
            std::mem::discriminant(other)
        ),
    };

    assert_eq!(received_view, view, "View should match");
    assert_eq!(
        received_payload.payload, payload,
        "Payload should match sent payload"
    );
    assert_eq!(
        received_payload.metadata, metadata,
        "Metadata should match sent metadata"
    );

    // === Step 3: BlockTask processes BlockDirectRecv ===
    let (block_sender, block_receiver) = async_broadcast::broadcast(128);
    block_state
        .handle(received_event, block_sender.clone(), block_receiver.clone())
        .await
        .expect("BlockTask handle should not error");

    // === Step 4: Verify payload is saved to consensus ===
    let saved_payload = block_state
        .consensus
        .read()
        .await
        .saved_payloads()
        .get(&view)
        .cloned();

    assert!(
        saved_payload.is_some(),
        "Payload should be saved to consensus after BlockDirectRecv"
    );
    let saved = saved_payload.unwrap();
    assert_eq!(
        saved.payload, payload,
        "Saved payload should match the directly received block"
    );
    assert_eq!(
        saved.metadata, metadata,
        "Saved metadata should match the directly received block"
    );

    tracing::info!(
        "End-to-end test passed: VidTask -> Network -> BlockTask with {} transactions",
        transactions.len()
    );
}
