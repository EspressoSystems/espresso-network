use hotshot::types::BLSPubKey;
use hotshot_types::traits::signature_key::SignatureKey;

use super::common::{
    test_utils::{TestData, mock_membership},
    *,
};
use crate::{
    consensus::Consensus,
    events::{Action, ConsensusOutput},
    helpers::Outbox,
};

/// Create a Consensus instance for a given node index.
async fn make_consensus(
    node_index: u64,
) -> Consensus<hotshot_example_types::node_types::TestTypes> {
    let membership = mock_membership().await;
    let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
    Consensus::new(membership, public_key, private_key)
}

/// Integration: proposal accepted and Vote1 sent.
#[tokio::test]
async fn test_vote1_genesis_parent() {
    let mut consensus = make_consensus(0).await;
    let mut outbox = Outbox::new();
    let test_data = TestData::new(2).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    consensus
        .apply(test_data.views[0].proposal_event(&node_key), &mut outbox)
        .await;
    // Simulate state verification completing.
    consensus
        .apply(test_data.views[0].state_verified_event(), &mut outbox)
        .await;

    assert!(
        has_vote1(&outbox),
        "Vote1 should fire for view 1 with genesis parent"
    );
}

/// Integration: sequential views both produce Vote1.
#[tokio::test]
async fn test_sequential_vote1() {
    let mut consensus = make_consensus(0).await;
    let mut outbox = Outbox::new();
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    consensus
        .apply(test_data.views[0].proposal_event(&node_key), &mut outbox)
        .await;
    consensus
        .apply(test_data.views[0].state_verified_event(), &mut outbox)
        .await;
    consensus
        .apply(test_data.views[0].block_reconstructed_event(), &mut outbox)
        .await;

    consensus
        .apply(test_data.views[1].proposal_event(&node_key), &mut outbox)
        .await;
    consensus
        .apply(test_data.views[1].state_verified_event(), &mut outbox)
        .await;

    assert_eq!(count_vote1(&outbox), 2, "Both views should produce Vote1");
}

/// Integration: full decide flow.
#[tokio::test]
async fn test_single_view_decide() {
    let mut consensus = make_consensus(0).await;
    let mut outbox = Outbox::new();
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    consensus
        .apply(test_data.views[0].proposal_event(&node_key), &mut outbox)
        .await;
    consensus
        .apply(test_data.views[0].state_verified_event(), &mut outbox)
        .await;
    consensus
        .apply(test_data.views[0].block_reconstructed_event(), &mut outbox)
        .await;
    consensus
        .apply(test_data.views[0].cert1_event(), &mut outbox)
        .await;
    consensus
        .apply(test_data.views[0].cert2_event(), &mut outbox)
        .await;

    consensus
        .apply(test_data.views[1].proposal_event(&node_key), &mut outbox)
        .await;
    consensus
        .apply(test_data.views[1].state_verified_event(), &mut outbox)
        .await;
    consensus
        .apply(test_data.views[1].block_reconstructed_event(), &mut outbox)
        .await;

    assert!(has_vote1(&outbox), "Vote1 should be sent");
    assert!(has_vote2(&outbox), "Vote2 should be sent");
    assert!(
        has_leaf_decided(&outbox),
        "Leaf should be decided after cert2"
    );
}

/// Integration: leader sends proposal.
#[tokio::test]
async fn test_leader_sends_proposal() {
    let test_data = TestData::new(4).await;
    let leader_for_view_2 = test_data.views[1].leader_public_key;
    let leader_index = node_index_for_key(&leader_for_view_2);
    let mut consensus = make_consensus(leader_index).await;
    let mut outbox = Outbox::new();

    consensus
        .apply(
            test_data.views[0].proposal_event(&leader_for_view_2),
            &mut outbox,
        )
        .await;
    consensus
        .apply(test_data.views[0].state_verified_event(), &mut outbox)
        .await;
    consensus
        .apply(test_data.views[0].cert1_event(), &mut outbox)
        .await;

    assert!(
        has_request_block_and_header(&outbox),
        "Leader should request block and header"
    );
}

/// Integration: multi-view chain with decisions.
#[tokio::test]
async fn test_multi_view_decide() {
    let mut consensus = make_consensus(0).await;
    let mut outbox = Outbox::new();
    let test_data = TestData::new(5).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    for view in &test_data.views {
        consensus
            .apply(view.proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(view.state_verified_event(), &mut outbox)
            .await;
        consensus
            .apply(view.block_reconstructed_event(), &mut outbox)
            .await;
        consensus.apply(view.cert1_event(), &mut outbox).await;
        consensus.apply(view.cert2_event(), &mut outbox).await;
    }

    let decide_count = outbox
        .iter()
        .filter(|e| {
            matches!(
                e,
                ConsensusOutput::Event(crate::events::Event::LeafDecided(_))
            )
        })
        .count();
    assert!(
        decide_count == 5,
        "Multiple views should produce decisions, got {decide_count}"
    );
}
