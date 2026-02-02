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

    // Send BlockDirectlyRecv event
    let event = Arc::new(HotShotEvent::BlockDirectlyRecv(
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
    let direct_recv_event = Arc::new(HotShotEvent::BlockDirectlyRecv(
        PayloadWithMetadata {
            payload: prev_payload.clone(),
            metadata: prev_metadata.clone(),
        },
        ViewNumber::new(3),
    ));
    block_state
        .handle(direct_recv_event, sender.clone(), receiver.clone())
        .await
        .expect("handle BlockDirectlyRecv should not error");

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
    let direct_recv_event = Arc::new(HotShotEvent::BlockDirectlyRecv(
        PayloadWithMetadata {
            payload: prev_payload.clone(),
            metadata: prev_metadata.clone(),
        },
        ViewNumber::new(3),
    ));
    block_state
        .handle(direct_recv_event, sender.clone(), receiver.clone())
        .await
        .expect("handle BlockDirectlyRecv should not error");

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
