use hotshot::types::BLSPubKey;
use hotshot_types::traits::signature_key::SignatureKey;

use super::{common::test_utils::TestData, *};
use crate::events::{Event, Update};

/// Integration: proposal accepted and Vote1 sent via real state validation.
#[tokio::test]
async fn test_vote1_genesis_parent() {
    let harness = TestHarness::new_with_state_manager(0).await;
    let test_data = TestData::new(2).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .send(test_data.views[0].proposal_update(&node_key))
        .await;

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let events = harness.shutdown().await;

    assert!(
        has_vote1(&events),
        "Vote1 should fire for view 1 with genesis parent"
    );
}

/// Integration: sequential views both produce Vote1 through real state validation.
#[tokio::test]
async fn test_sequential_vote1() {
    let harness = TestHarness::new_with_state_manager(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .send(test_data.views[0].proposal_update(&node_key))
        .await;
    harness
        .send(test_data.views[0].block_reconstructed_update())
        .await;

    harness
        .send(test_data.views[1].proposal_update(&node_key))
        .await;

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let events = harness.shutdown().await;

    assert_eq!(count_vote1(&events), 2, "Both views should produce Vote1");
}

/// Integration: full decide through real state validation.
#[tokio::test]
async fn test_single_view_decide() {
    let harness = TestHarness::new_with_state_manager(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .send(test_data.views[0].proposal_update(&node_key))
        .await;
    harness
        .send(test_data.views[0].block_reconstructed_update())
        .await;
    harness.send(test_data.views[0].cert1_update()).await;
    harness.send(test_data.views[0].cert2_update()).await;
    harness
        .send(test_data.views[1].proposal_update(&node_key))
        .await;
    harness
        .send(test_data.views[1].block_reconstructed_update())
        .await;

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let events = harness.shutdown().await;

    assert!(has_vote1(&events), "Vote1 should be sent");
    assert!(has_vote2(&events), "Vote2 should be sent");
    assert!(
        has_leaf_decided(&events),
        "Leaf should be decided after cert2"
    );
}

/// Integration: leader sends proposal via real state validation and header creation.
#[tokio::test]
async fn test_leader_sends_proposal() {
    let test_data = TestData::new(4).await;
    let leader_for_view_2 = test_data.views[1].leader_public_key;
    let leader_index = node_index_for_key(&leader_for_view_2);
    let harness = TestHarness::new_with_state_manager(leader_index).await;

    harness
        .send(test_data.views[0].proposal_update(&leader_for_view_2))
        .await;
    harness.send(test_data.views[0].cert1_update()).await;

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let events = harness.shutdown().await;

    assert!(
        has_request_block_and_header(&events),
        "Leader should request block and header"
    );
    assert!(
        has_proposal(&events),
        "Leader should send proposal via real state manager"
    );
}

/// Integration: multi-view chain with decisions flowing through real state validation.
#[tokio::test]
async fn test_multi_view_decide() {
    let harness = TestHarness::new_with_state_manager(0).await;
    let test_data = TestData::new(5).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    for view in &test_data.views {
        harness.send(view.proposal_update(&node_key)).await;
        harness.send(view.block_reconstructed_update()).await;
        harness.send(view.cert1_update()).await;
        harness.send(view.cert2_update()).await;
    }

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let events = harness.shutdown().await;

    let decide_count = events
        .iter()
        .filter(|e| matches!(e, Event::Update(Update::LeafDecided(_))))
        .count();
    assert!(
        decide_count == 5,
        "Multiple views should produce decisions, got {decide_count}"
    );
}
