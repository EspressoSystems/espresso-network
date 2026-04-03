use std::sync::Arc;

use hotshot::{traits::ValidatedState, types::BLSPubKey};
use hotshot_example_types::{node_types::TestTypes, state_types::TestValidatedState};
use hotshot_types::{data::ViewNumber, traits::signature_key::SignatureKey};

use super::common::utils::TestData;
use crate::{
    consensus::ConsensusInput,
    helpers::proposal_commitment,
    message::Proposal,
    outbox::Outbox,
    state::StateResponse,
    tests::common::{
        assertions::{
            any, count_matching, is_leaf_decided, is_proposal, is_request_block_and_header,
            is_request_state, is_send_cert1, is_send_cert2, is_vote1, is_vote2, node_index_for_key,
        },
        utils::ConsensusHarness,
    },
};

/// Fresh consensus with no locked_cert accepts any proposal (genesis safety).
#[tokio::test]
async fn test_safety_genesis_no_lock() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(2).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply(test_data.views[0].proposal_input_consensus(&node_key))
        .await;

    assert!(
        any(harness.outputs(), is_request_state),
        "Proposal should be accepted with no locked cert"
    );
}

/// Events with view <= timeout_view are silently dropped.
#[tokio::test]
async fn test_timeout_filters_stale_events() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(6).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Set timeout at view 3
    harness
        .apply(ConsensusInput::Timeout(ViewNumber::new(3)))
        .await;

    // Send stale proposal (view 2, which is <= timeout_view 3)
    harness
        .apply(test_data.views[1].proposal_input_consensus(&node_key))
        .await;

    // Send fresh proposal (view 4, which is > timeout_view 3)
    harness
        .apply(test_data.views[3].proposal_input_consensus(&node_key))
        .await;

    assert_eq!(1, count_matching(harness.outputs(), is_request_state))
}

/// Vote1 fires for sequential views when all preconditions are met.
#[tokio::test]
async fn test_vote1_for_sequential_views() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    harness
        .apply(test_data.views[1].proposal_input_consensus(&node_key))
        .await;

    assert_eq!(
        count_matching(harness.outputs(), is_vote1),
        2,
        "Vote1 should fire for sequential views"
    );
}

/// Vote1 fires for view 1 (genesis parent) — parent checks are skipped.
#[tokio::test]
async fn test_vote1_genesis_parent() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(2).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply(test_data.views[0].proposal_input_consensus(&node_key))
        .await;

    assert!(
        any(harness.outputs(), is_vote1),
        "Vote1 should fire for view 1 with genesis parent"
    );
}

/// Vote2 requires Certificate1 + BlockReconstructed + Proposal.
/// Without Certificate1, no Vote2 is sent.
#[tokio::test]
async fn test_vote2_missing_cert1() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    harness
        .apply(test_data.views[1].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[1].block_reconstructed_input())
        .await;

    assert!(
        !any(harness.outputs(), is_vote2),
        "Vote2 should not be sent without Certificate1"
    );
}

/// Vote2 is sent when Certificate1 arrives after proposal.
#[tokio::test]
async fn test_vote2_with_cert1() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    harness
        .apply(test_data.views[1].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[1].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[1].cert1_input()).await;

    assert!(
        any(harness.outputs(), is_vote2),
        "Vote2 should be sent when cert1 is present"
    );
}

/// Full single-view decision: proposal → vote1, cert1 → vote2, cert2 → decide.
#[tokio::test]
async fn test_single_view_decide() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    harness
        .apply(test_data.views[1].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[1].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[1].cert1_input()).await;
    harness.apply(test_data.views[1].cert2_input()).await;

    assert!(any(harness.outputs(), is_vote1), "Vote1 should be sent");
    assert!(any(harness.outputs(), is_vote2), "Vote2 should be sent");
    assert!(
        any(harness.outputs(), is_leaf_decided),
        "Leaf should be decided after cert2"
    );
}

/// Duplicate votes are prevented — only one Vote1 per view.
#[tokio::test]
async fn test_no_duplicate_vote1() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(2).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[0].cert1_input()).await;

    assert_eq!(
        count_matching(harness.outputs(), is_vote1),
        1,
        "Should only send one Vote1 per view"
    );
}

/// Duplicate votes are prevented — only one Vote2 per view.
#[tokio::test]
async fn test_no_duplicate_vote2() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    harness
        .apply(test_data.views[1].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[1].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[1].cert1_input()).await;
    harness.apply(test_data.views[1].cert2_input()).await;

    assert_eq!(
        count_matching(harness.outputs(), is_vote2),
        1,
        "Should only send one Vote2 per view"
    );
}

/// StateValidationFailed with matching commitment removes proposal and vid_share.
#[tokio::test]
async fn test_state_validation_failed_removes_proposal() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    // Send proposal for view 2 — but bypass the harness auto-response
    // by directly applying the proposal input, then manually sending
    // StateValidationFailed instead of letting the harness auto-respond.
    // We need to call consensus.apply directly to avoid auto StateVerified.
    let proposal_input = test_data.views[1].proposal_input_consensus(&node_key);
    let mut outbox = Outbox::new();
    harness.consensus.apply(proposal_input, &mut outbox).await;
    harness.collected.extend(outbox.take());

    // Send StateVerificationFailed — removes proposal
    let proposal: Proposal<TestTypes> = test_data.views[1].proposal.data.clone().into();
    harness
        .apply(ConsensusInput::StateValidationFailed(StateResponse {
            view: test_data.views[1].view_number,
            commitment: proposal_commitment(&proposal),
            state: Arc::new(
                <TestValidatedState as ValidatedState<TestTypes>>::from_header(
                    &proposal.block_header,
                ),
            ),
            delta: None,
        }))
        .await;

    // Now send cert1 + block_reconstructed — vote2 should NOT fire
    harness
        .apply(test_data.views[1].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[1].cert1_input()).await;

    assert!(
        !any(harness.outputs(), is_vote2),
        "Vote2 should not fire after proposal removed by StateVerificationFailed"
    );
}

/// Without Certificate2, no decision is made even with all other data.
#[tokio::test]
async fn test_decide_requires_cert2() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    harness
        .apply(test_data.views[1].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[1].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[1].cert1_input()).await;
    // No cert2 sent

    assert!(any(harness.outputs(), is_vote2), "Vote2 should still fire");
    assert!(
        !any(harness.outputs(), is_leaf_decided),
        "No decision without Certificate2"
    );
}

/// Vote2 requires BlockReconstructed for the current view.
#[tokio::test]
async fn test_vote2_missing_block_reconstructed() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    // View 2: proposal + cert1, but NO block_reconstructed for view 2
    harness
        .apply(test_data.views[1].proposal_input_consensus(&node_key))
        .await;
    harness.apply(test_data.views[1].cert1_input()).await;

    assert!(
        !any(harness.outputs(), is_vote2),
        "Vote2 should not fire without BlockReconstructed"
    );
}

/// BlockReconstructed arriving after cert1 triggers vote2.
#[tokio::test]
async fn test_vote2_block_reconstructed_arrives_late() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    // View 2: proposal + cert1 first (no block_reconstructed yet)
    harness
        .apply(test_data.views[1].proposal_input_consensus(&node_key))
        .await;
    harness.apply(test_data.views[1].cert1_input()).await;

    // Now send block_reconstructed — should trigger vote2
    harness
        .apply(test_data.views[1].block_reconstructed_input())
        .await;

    assert!(
        any(harness.outputs(), is_vote2),
        "Vote2 should fire when BlockReconstructed arrives late"
    );
}

/// Multi-view chain: consecutive views each get decided when cert2 arrives.
#[tokio::test]
async fn test_multi_view_chain_decide() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(5).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    for view in &test_data.views {
        harness
            .apply(view.proposal_input_consensus(&node_key))
            .await;
        harness.apply(view.block_reconstructed_input()).await;
        harness.apply(view.cert1_input()).await;
        harness.apply(view.cert2_input()).await;
    }

    assert!(count_matching(harness.outputs(), is_leaf_decided) >= 2);
}

/// Timeout event sets timeout_view and prevents processing of that view.
#[tokio::test]
async fn test_timeout_prevents_voting() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    harness
        .apply(test_data.views[1].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[1].block_reconstructed_input())
        .await;

    // Timeout view 2 — now cert1 for view 2 should be dropped
    harness
        .apply(ConsensusInput::Timeout(test_data.views[1].view_number))
        .await;

    // Send cert1 for view 2 — should be stale
    harness.apply(test_data.views[1].cert1_input()).await;

    assert!(
        !any(harness.outputs(), is_vote2),
        "Vote2 should not fire after timeout for that view"
    );
}

/// Leader sends a proposal for view N+1 after receiving proposal for view N
/// and cert1 for view N.
#[tokio::test]
async fn test_leader_sends_proposal() {
    let test_data = TestData::new(4).await;
    let leader_for_view_2 = test_data.views[1].leader_public_key;
    let leader_index = node_index_for_key(&leader_for_view_2);
    let mut harness = ConsensusHarness::new(leader_index).await;

    harness
        .apply(test_data.views[0].proposal_input_consensus(&leader_for_view_2))
        .await;
    harness.apply(test_data.views[0].cert1_input()).await;

    assert!(
        any(harness.outputs(), is_request_block_and_header),
        "Leader should request block and header for the next view"
    );
    assert!(
        any(harness.outputs(), is_proposal),
        "Leader should send a proposal when it has cert1, header, block, and vid_disperse"
    );
}

/// Leader sends a proposal after a timeout using the locked cert and
/// view change evidence.
#[tokio::test]
async fn test_leader_proposes_after_timeout() {
    let test_data = TestData::new(5).await;
    let leader_for_view_3 = test_data.views[2].leader_public_key;
    let leader_index = node_index_for_key(&leader_for_view_3);
    let mut harness = ConsensusHarness::new(leader_index).await;

    // Build up locked_cert: process view 1 so cert1 sets locked_cert
    harness
        .apply(test_data.views[0].proposal_input_consensus(&leader_for_view_3))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[0].cert1_input()).await;

    // Now send timeout cert for view 2 — triggers proposal for view 3
    harness.apply(test_data.views[1].timeout_cert_input()).await;

    assert!(
        any(harness.outputs(), is_request_block_and_header),
        "Leader should request block and header after timeout"
    );
    assert!(
        any(harness.outputs(), is_proposal),
        "Leader should send proposal with timeout view change evidence"
    );
}

/// Non-leader node does NOT send a proposal.
#[tokio::test]
async fn test_non_leader_does_not_propose() {
    let test_data = TestData::new(4).await;
    let leader_for_view_2 = test_data.views[1].leader_public_key;
    let leader_index = node_index_for_key(&leader_for_view_2);
    let non_leader_index = if leader_index == 0 { 1 } else { 0 };
    let non_leader_key = BLSPubKey::generated_from_seed_indexed([0; 32], non_leader_index).0;
    let mut harness = ConsensusHarness::new(non_leader_index).await;

    harness
        .apply(test_data.views[0].proposal_input_consensus(&non_leader_key))
        .await;
    harness.apply(test_data.views[0].cert1_input()).await;

    assert!(
        !any(harness.outputs(), is_proposal),
        "Non-leader should NOT send a proposal"
    );
}

/// After advancing the lock, a proposal whose justify_qc references a view
/// below the locked QC (and with a different commitment) is rejected by the
/// safety rule, preventing vote1.
#[tokio::test]
async fn test_safety_rejects_proposal_below_lock() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(4).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Process view 1 fully: proposal + block_reconstructed + cert1 → locked_qc = view 1
    harness
        .apply(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[0].cert1_input()).await;

    // Process view 2 fully → locked_qc = view 2
    harness
        .apply(test_data.views[1].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[1].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[1].cert1_input()).await;

    let state_requests_before = count_matching(harness.outputs(), is_request_state);

    // Re-send view 1's proposal. Its justify_qc references genesis (view 0),
    // while locked_qc is at view 2.
    // Liveness: 0 > 2 = false
    // Safety:   genesis commitment != cert1(view 2) commitment = false
    // → Proposal rejected by is_safe, no RequestState emitted.
    harness
        .apply(test_data.views[0].proposal_input_consensus(&node_key))
        .await;

    let state_requests_after = count_matching(harness.outputs(), is_request_state);

    assert_eq!(
        state_requests_before, state_requests_after,
        "Proposal below lock should be rejected — no new RequestState"
    );
}

/// After receiving a timeout certificate, new proposals for higher views are
/// accepted and voted on — demonstrating the protocol continues.
#[tokio::test]
async fn test_vote_after_timeout_cert() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(5).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Process view 1 fully → locked_qc = view 1
    harness
        .apply(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[0].cert1_input()).await;

    // Process view 2 proposal + block_reconstructed (need parent data for view 3)
    harness
        .apply(test_data.views[1].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[1].block_reconstructed_input())
        .await;

    // Receive timeout cert for view 2 → view advances to 3
    harness.apply(test_data.views[1].timeout_cert_input()).await;

    let vote1_before = count_matching(harness.outputs(), is_vote1);

    // Process proposal for view 3. Its justify_qc is at view 2, which is
    // >= locked_qc at view 1 (liveness passes). Parent data for view 2 exists.
    harness
        .apply(test_data.views[2].proposal_input_consensus(&node_key))
        .await;

    assert!(
        count_matching(harness.outputs(), is_vote1) > vote1_before,
        "Vote1 should fire for proposal after timeout certificate"
    );
}

/// After a timeout certificate advances the view, a proposal whose
/// justify_qc references a view below the lock is rejected by the
/// safety rule. This simulates a leader with a stale lock proposing
/// after a timeout — the replica's higher lock prevents voting.
///
/// The proposal's view (3) passes the stale filter (timeout_view is 0
/// because no local Timeout fired), but is_safe rejects it because
/// justify_qc.view (2) < locked_qc.view (3) and the commitments differ.
#[tokio::test]
async fn test_no_vote_after_timeout_for_proposal_below_lock() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(6).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Process views 1-3 fully → locked_qc = cert1 for view 3
    for i in 0..3 {
        harness
            .apply(test_data.views[i].proposal_input_consensus(&node_key))
            .await;
        harness
            .apply(test_data.views[i].block_reconstructed_input())
            .await;
        harness.apply(test_data.views[i].cert1_input()).await;
    }

    // Receive timeout cert for view 3 → view advances to 4.
    // No local Timeout fires, so timeout_view stays at 0 and proposals
    // are filtered only by the safety rule, not the stale filter.
    harness.apply(test_data.views[2].timeout_cert_input()).await;

    let vote1_count = count_matching(harness.outputs(), is_vote1);

    // Re-send view 3's proposal. Its justify_qc is cert1 for view 2.
    // Stale filter: view 3 > timeout_view (0) → passes.
    // Safety:  justify_qc.view (2) > locked_qc.view (3) → false (liveness fails)
    //          justify_qc commitment ≠ locked_qc commitment → false (safety fails)
    // → Proposal rejected by is_safe.
    harness
        .apply(test_data.views[2].proposal_input_consensus(&node_key))
        .await;

    assert_eq!(
        count_matching(harness.outputs(), is_vote1),
        vote1_count,
        "No new Vote1 — proposal with justify_qc below lock rejected by safety rule"
    );
}

/// Valid certificates are re-broadcast exactly once; duplicates are suppressed.
#[tokio::test]
async fn test_certificate_forwarding_and_dedup() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Build up state: two views with proposals + block reconstructed
    harness
        .apply(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;
    harness
        .apply(test_data.views[1].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[1].block_reconstructed_input())
        .await;

    // Receive cert1 → should forward, then duplicate → suppressed
    harness.apply(test_data.views[1].cert1_input()).await;
    harness.apply(test_data.views[1].cert1_input()).await;

    assert_eq!(
        count_matching(harness.outputs(), is_send_cert1),
        1,
        "cert1: first receive forwards, duplicate suppressed"
    );

    // Receive cert2 → should forward, then duplicate → suppressed
    harness.apply(test_data.views[1].cert2_input()).await;
    harness.apply(test_data.views[1].cert2_input()).await;

    assert_eq!(
        count_matching(harness.outputs(), is_send_cert2),
        1,
        "cert2: first receive forwards, duplicate suppressed"
    );
}

/// Cert2 for a view that is already decided is ignored.
#[tokio::test]
async fn test_decide_not_repeated_for_same_view() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Full round for view 2
    harness
        .apply(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;
    harness
        .apply(test_data.views[1].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[1].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[1].cert1_input()).await;
    harness.apply(test_data.views[1].cert2_input()).await;

    // Send cert2 again for same view — should not produce another decide
    harness.apply(test_data.views[1].cert2_input()).await;

    assert_eq!(1, count_matching(harness.outputs(), is_leaf_decided));
}
