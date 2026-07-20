use std::sync::Arc;

use hotshot::{traits::ValidatedState, types::BLSPubKey};
use hotshot_example_types::{
    block_types::TestBlockHeader,
    node_types::{TEST_VERSIONS, TestTypes},
    state_types::TestValidatedState,
};
use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    traits::signature_key::SignatureKey,
};

use super::common::utils::TestData;
use crate::{
    consensus::{ConsensusInput, ConsensusOutput},
    coordinator::GcScope,
    helpers::proposal_commitment,
    message::Proposal,
    outbox::Outbox,
    state::StateResponse,
    storage::{ActionKind, StorageOutput},
    tests::common::{
        assertions::{
            any, count_matching, decides_view, is_leaf_decided, is_persist_proposal, is_proposal,
            is_proposal_for_view, is_record_action, is_request_block_and_header, is_request_state,
            is_send_cert2, is_send_timeout_cert, is_send_timeout_vote, is_view_changed, is_vote1,
            is_vote2, node_index_for_key,
        },
        utils::{ConsensusHarness, MockBlock, state_verified_input},
    },
};

/// Fresh consensus with no locked_cert accepts any proposal (genesis safety).
#[tokio::test]
async fn test_safety_genesis_no_lock() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(2).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;

    assert!(
        any(harness.outputs(), is_request_state),
        "Proposal should be accepted with no locked cert"
    );
}

/// All inputs are processed regardless of timeout_view, but vote1 is
/// suppressed for views <= timeout_view.
#[tokio::test]
async fn test_timeout_filters_vote1_not_processing() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(6).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Set timeout at view 3
    harness
        .apply(ConsensusInput::Timeout(
            ViewNumber::new(3),
            EpochNumber::genesis(),
        ))
        .await;

    // Send stale proposal (view 2, which is <= timeout_view 3).
    // It is still processed (state validation requested) but vote1 is suppressed.
    harness
        .apply_pair(test_data.views[1].proposal_input_consensus(&node_key))
        .await;

    // Send fresh proposal (view 4, which is > timeout_view 3)
    harness
        .apply_pair(test_data.views[3].proposal_input_consensus(&node_key))
        .await;

    // Both proposals are processed.
    assert_eq!(2, count_matching(harness.outputs(), is_request_state));

    // No vote1 for any view — the stale view is suppressed and the fresh
    // view lacks block reconstruction.
    assert!(
        !any(harness.outputs(), is_vote1),
        "Vote1 should not fire for a timed-out view"
    );
}

/// Vote1 fires for sequential views when all preconditions are met.
#[tokio::test]
async fn test_vote1_for_sequential_views() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    harness
        .apply_pair(test_data.views[1].proposal_input_consensus(&node_key))
        .await;

    assert_eq!(
        count_matching(harness.outputs(), is_vote1),
        2,
        "Vote1 should fire for sequential views"
    );
}

/// A re-seeded parent proposal (not re-received over the network) lets the node
/// vote1 on the child built on it, mirroring `Coordinator::maker`.
#[tokio::test]
async fn test_vote1_with_seeded_parent_proposal() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Restart: re-seed the validated view-1 proposal and mark its block reconstructed.
    harness
        .consensus
        .seed_proposals([test_data.views[0].proposal.data.clone()]);
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    // The view-2 proposal arrives and builds on the re-seeded parent.
    harness
        .apply_pair(test_data.views[1].proposal_input_consensus(&node_key))
        .await;

    assert!(
        any(harness.outputs(), is_vote1),
        "vote1 should fire on the child using the re-seeded parent proposal"
    );
}

/// A restored lock certifying the parent lets the node vote1 without
/// re-reconstructing the parent block (nothing in `blocks_reconstructed`).
#[tokio::test]
async fn test_vote1_parent_reconstruction_from_lock() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Restart: re-seed the parent proposal and restore the lock that certifies
    // it, but do NOT mark its block reconstructed.
    harness
        .consensus
        .seed_proposals([test_data.views[0].proposal.data.clone()]);
    harness
        .consensus
        .seed_locked_cert(test_data.views[0].cert1.clone());

    harness
        .apply_pair(test_data.views[1].proposal_input_consensus(&node_key))
        .await;

    assert!(
        any(harness.outputs(), is_vote1),
        "vote1 should fire when the restored lock certifies the parent block"
    );
}

/// Control for `test_vote1_with_seeded_parent_proposal`: parent block
/// reconstructed but proposal absent, so `maybe_vote_1` bails and no vote fires.
#[tokio::test]
async fn test_vote1_blocked_without_parent_proposal() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    harness
        .apply_pair(test_data.views[1].proposal_input_consensus(&node_key))
        .await;

    assert!(
        !any(harness.outputs(), is_vote1),
        "vote1 must not fire when the parent proposal is unavailable"
    );
}

/// Vote1 fires for view 1 (genesis parent) — parent checks are skipped.
#[tokio::test]
async fn test_vote1_genesis_parent() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(2).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
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
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    harness
        .apply_pair(test_data.views[1].proposal_input_consensus(&node_key))
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
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    harness
        .apply_pair(test_data.views[1].proposal_input_consensus(&node_key))
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
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    harness
        .apply_pair(test_data.views[1].proposal_input_consensus(&node_key))
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

/// Obtaining a Cert2 broadcasts it once so peers that missed the vote2s can
/// still decide, and a re-delivered Cert2 for the same view is not re-broadcast.
#[tokio::test]
async fn test_cert2_broadcast_once() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;
    harness
        .apply_pair(test_data.views[1].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[1].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[1].cert1_input()).await;
    harness.apply(test_data.views[1].cert2_input()).await;

    assert_eq!(
        count_matching(harness.outputs(), is_send_cert2),
        1,
        "obtaining a cert2 should broadcast it exactly once"
    );

    harness.apply(test_data.views[1].cert2_input()).await;
    assert_eq!(
        count_matching(harness.outputs(), is_send_cert2),
        1,
        "a re-delivered cert2 must not be re-broadcast"
    );

    // View 1 decided as an ancestor of view 2, so its cert2 was never
    // assembled locally.
    harness.apply(test_data.views[0].cert2_input()).await;
    assert_eq!(
        count_matching(harness.outputs(), is_send_cert2),
        1,
        "a cert2 for an already-decided view must not be relayed"
    );
}

/// A Cert2 arriving long after the node advanced past its view must still
/// decide: view-change GC must not drop the decide inputs of undecided views.
#[tokio::test]
async fn test_late_cert2_decides_after_view_change_gc() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(5).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Drive views 1..=4 forward with Cert1 only, so nothing decides.
    for view in &test_data.views[0..4] {
        harness
            .apply_pair(view.proposal_input_consensus(&node_key))
            .await;
        harness.apply(view.block_reconstructed_input()).await;
        harness.apply(view.cert1_input()).await;
    }
    assert!(
        !any(harness.outputs(), is_leaf_decided),
        "no leaf should decide before any cert2 arrives"
    );

    // The view-change GC the coordinator runs as the node advances.
    harness
        .consensus
        .gc(GcScope::Local(test_data.views[3].view_number));

    harness.apply(test_data.views[0].cert2_input()).await;

    assert!(
        any(harness.outputs(), is_leaf_decided),
        "a late cert2 must still decide — GC must not drop the info needed to decide"
    );
}

/// A newer view decides *without* its full ancestor chain (a gap); a late
/// Cert2 for the skipped view must still decide it, without moving the
/// decided watermark backward.
#[tokio::test]
async fn test_gap_fill_decide_of_older_view() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(5).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Decide view 4 without ever delivering views 1-3, leaving a gap.
    harness
        .apply_pair(test_data.views[3].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[3].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[3].cert1_input()).await;
    harness.apply(test_data.views[3].cert2_input()).await;

    assert_eq!(
        count_matching(harness.outputs(), is_leaf_decided),
        1,
        "view 4 should decide"
    );
    assert!(
        any(harness.outputs(), |o| decides_view(o, 4)),
        "the first decide should be for view 4"
    );
    assert!(
        !any(harness.outputs(), |o| decides_view(o, 3)),
        "view 3 must not be decided yet — it is a gap"
    );

    // The gap view's proposal was received live before the node advanced past
    // it; its Cert1 and Cert2 arrive late.
    harness
        .consensus
        .seed_proposals([test_data.views[2].proposal.data.clone()]);
    harness.apply(test_data.views[2].cert1_input()).await;
    harness.apply(test_data.views[2].cert2_input()).await;

    assert!(
        any(harness.outputs(), |o| decides_view(o, 3)),
        "the late cert2 for the gap view (3) must decide it"
    );
    assert_eq!(
        harness.consensus.last_decided_view(),
        test_data.views[3].view_number,
        "a gap-fill decide of an older view must not move the watermark backward"
    );
    assert_eq!(
        count_matching(harness.outputs(), is_send_cert2),
        2,
        "the late cert2 for the still-undecided gap view must be relayed (once for view 4, once \
         for the gap view 3)"
    );
}

/// Duplicate votes are prevented — only one Vote1 per view.
#[tokio::test]
async fn test_no_duplicate_vote1() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(2).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
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
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    harness
        .apply_pair(test_data.views[1].proposal_input_consensus(&node_key))
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
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    // Send proposal for view 2 — but bypass the harness auto-response
    // by directly applying the proposal input, then manually sending
    // StateValidationFailed instead of letting the harness auto-respond.
    // We need to call consensus.apply directly to avoid auto StateVerified.
    let (proposal_input, vid_share_input) = test_data.views[1].proposal_input_consensus(&node_key);
    let mut outbox = Outbox::new();
    harness.consensus.apply(proposal_input, &mut outbox);
    harness.consensus.apply(vid_share_input, &mut outbox);
    harness.collected.extend(outbox.take());

    // Send StateVerificationFailed — removes proposal
    let proposal: Proposal<TestTypes> = test_data.views[1].proposal.data.clone();
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
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    harness
        .apply_pair(test_data.views[1].proposal_input_consensus(&node_key))
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
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    // View 2: proposal + cert1, but NO block_reconstructed for view 2
    harness
        .apply_pair(test_data.views[1].proposal_input_consensus(&node_key))
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
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    // View 2: proposal + cert1 first (no block_reconstructed yet)
    harness
        .apply_pair(test_data.views[1].proposal_input_consensus(&node_key))
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
            .apply_pair(view.proposal_input_consensus(&node_key))
            .await;
        harness.apply(view.block_reconstructed_input()).await;
        harness.apply(view.cert1_input()).await;
        harness.apply(view.cert2_input()).await;
    }

    assert!(count_matching(harness.outputs(), is_leaf_decided) >= 2);
}

/// After a timeout, inputs for the timed-out view are still processed (so
/// the node can decide the leaf via cert2), but vote1 is suppressed.
#[tokio::test]
async fn test_timeout_prevents_vote1_but_allows_vote2() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Process view 1 to establish state.
    harness
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;

    let vote1_before = count_matching(harness.outputs(), is_vote1);

    // Timeout view 2 BEFORE the proposal arrives.
    harness
        .apply(ConsensusInput::Timeout(
            test_data.views[1].view_number,
            test_data.views[1].epoch_number,
        ))
        .await;
    assert!(
        any(harness.outputs(), is_send_timeout_vote),
        "Timeout should emit timeout vote"
    );

    // Now send the proposal and block reconstruction for view 2.
    // The proposal is still stored (inputs are processed), but vote1 is
    // suppressed because view 2 <= timeout_view.
    harness
        .apply_pair(test_data.views[1].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[1].block_reconstructed_input())
        .await;

    assert_eq!(
        vote1_before,
        count_matching(harness.outputs(), is_vote1),
        "Vote1 should not fire for a timed-out view"
    );

    // Send cert1 for view 2 — still processed, enabling vote2.
    harness.apply(test_data.views[1].cert1_input()).await;

    assert!(
        any(harness.outputs(), is_vote2),
        "Vote2 should still fire for a timed-out view when cert1 arrives"
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
        .apply_pair(test_data.views[0].proposal_input_consensus(&leader_for_view_2))
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

/// Regression: `maybe_propose` must refuse to propose when
/// `self.proposals[parent_view]` has drifted from
/// `parent_cert.data.leaf_commit`.
///
/// `self.proposals` is keyed by view and can be overwritten (e.g. byzantine
/// leader sends two safe proposals at the same view, the cert formed for the
/// first but a later overwrite landed in the proposals map). In that case the
/// canonical header was built off the cert's leaf — but if `maybe_propose`
/// derived the header lookup key from `self.proposals[parent_view]`, it could
/// pick up a header built for a different parent (same view, different
/// height) and emit a proposal whose `justify_qc` says height `h` but whose
/// own `block_header` says height `h+2`, exactly matching the
/// `Invalid Height: parent_height=X, proposal_height=X+2` production error.
#[tokio::test]
async fn test_propose_refuses_when_stored_proposal_differs_from_cert() {
    let test_data = TestData::new(4).await;
    let leader_for_view_2 = test_data.views[1].leader_public_key;
    let leader_index = node_index_for_key(&leader_for_view_2);
    let mut harness = ConsensusHarness::new(leader_index).await;

    // Standard happy-path setup through view 1: receive proposal, which
    // triggers RequestBlockAndHeader for view 2 (auto-built by the harness
    // against the canonical view-1 proposal as parent).
    harness
        .apply_pair(test_data.views[0].proposal_input_consensus(&leader_for_view_2))
        .await;

    // Sanity: header for view 2 has been built off the canonical view-1
    // parent. Without the fix and with `proposals[1]` overwritten below, the
    // header lookup would still find a header (the canonical one) keyed by
    // a different commitment than what `maybe_propose` would compute — so
    // either we'd lose the header (silent stall) or worse, find a header
    // we built for the byzantine parent. The check guards both.

    // Inject a synthetic "byzantine" proposal at view 1 — same view but a
    // different leaf commit (taken from view 2's proposal, with its view
    // number rewritten to 1). This simulates the case where a second safe
    // proposal at view 1 overwrites `self.proposals[1]` after the cert had
    // already formed for the first.
    let mut byzantine = test_data.views[1].proposal.data.clone();
    byzantine.view_number = test_data.views[0].view_number;
    let canonical_commit = proposal_commitment(&test_data.views[0].proposal.data);
    let byzantine_commit = proposal_commitment(&byzantine);
    assert_ne!(
        canonical_commit, byzantine_commit,
        "test setup: byzantine proposal must have a different leaf commit"
    );
    harness
        .consensus
        .force_set_proposal(test_data.views[0].view_number, byzantine.clone());

    // Plant a header keyed by the byzantine commit at view 2. Without this
    // step, even the buggy code would silently fail to find a header and
    // return early — so the test wouldn't differentiate the fix from the
    // bug. With this header present, the buggy code WOULD look it up
    // (via `proposal_commitment(proposals[1]) = byzantine_commit`), find
    // it, and emit a wrong-height proposal. The fix's commitment check
    // catches the mismatch before the lookup.
    let mock = MockBlock::new();
    let byzantine_parent_leaf: Leaf2<TestTypes> = byzantine.into();
    let byzantine_built_header = TestBlockHeader::new(
        &byzantine_parent_leaf,
        mock.payload_commitment,
        mock.builder_commitment,
        mock.metadata,
        TEST_VERSIONS.test.base,
    );
    harness
        .apply(ConsensusInput::HeaderCreated(
            test_data.views[1].view_number,
            byzantine_commit,
            byzantine_built_header,
        ))
        .await;

    // cert1 for view 1 (over the canonical proposal) — this is what
    // `maybe_propose(view=2)` will use as `parent_cert`. Its leaf_commit no
    // longer matches `proposals[1]`.
    harness.apply(test_data.views[0].cert1_input()).await;

    assert!(
        !any(harness.outputs(), is_proposal),
        "Leader must NOT propose when proposals[parent_view] disagrees with \
         parent_cert.leaf_commit"
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
        .apply_pair(test_data.views[0].proposal_input_consensus(&leader_for_view_3))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[0].cert1_input()).await;

    // Now send timeout cert for view 2 — triggers proposal for view 3
    harness.apply(test_data.views[1].timeout_cert_input()).await;

    assert!(
        any(harness.outputs(), is_send_timeout_cert),
        "Timeout certificate should be forwarded"
    );
    assert!(
        any(harness.outputs(), is_request_block_and_header),
        "Leader should request block and header after timeout"
    );
    assert!(
        any(harness.outputs(), is_proposal),
        "Leader should send proposal with timeout view change evidence"
    );
}

/// A timeout-backed proposal chains only from the locked cert, never from a
/// Cert1 of the timed-out view.
#[tokio::test]
async fn test_timeout_proposal_chains_from_lock_not_timed_out_cert1() {
    let test_data = TestData::new(5).await;
    let leader_for_view_3 = test_data.views[2].leader_public_key;
    let leader_index = node_index_for_key(&leader_for_view_3);
    let mut harness = ConsensusHarness::new(leader_index).await;

    // Lock on cert1(1).
    harness
        .apply_pair(test_data.views[0].proposal_input_consensus(&leader_for_view_3))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[0].cert1_input()).await;

    // View 2 arrives via fetch (no optimistic header request); its cert1
    // forms but the block is never reconstructed, so the lock stays at 1.
    harness
        .apply(ConsensusInput::FetchedProposal(
            test_data.views[1].proposal_message(),
        ))
        .await;
    harness.apply(test_data.views[1].cert1_input()).await;
    assert!(
        harness
            .consensus
            .cert1_at(ViewNumber::new(2))
            .is_some_and(|_| harness.consensus.locked_view() == Some(ViewNumber::new(1))),
        "setup: cert1(2) must be held while the lock stays at view 1"
    );
    assert!(
        !any(harness.outputs(), |o| is_proposal_for_view(o, 3)),
        "no header for a view-2 parent yet, so view 3 must not be proposed"
    );

    harness.apply(test_data.views[1].timeout_cert_input()).await;

    let justify_views: Vec<u64> = harness
        .outputs()
        .iter()
        .filter_map(|output| match output {
            ConsensusOutput::SendProposal(p) if *p.data.view_number == 3 => {
                Some(*p.data.justify_qc.view_number)
            },
            _ => None,
        })
        .collect();
    assert_eq!(
        justify_views,
        vec![1],
        "timeout-backed proposal must chain from the locked cert1(1), not the timed-out view's \
         cert1(2)"
    );
}

/// Cutover exception: a bridged legacy QC higher than the lock is adopted
/// as the lock and proposing is retried on it, re-requesting the header for
/// the new lock.
#[tokio::test]
async fn test_bridged_legacy_qc_adopts_lock_and_reproposes() {
    let test_data = TestData::new(5).await;
    let leader_for_view_3 = test_data.views[2].leader_public_key;
    let leader_index = node_index_for_key(&leader_for_view_3);
    let mut harness = ConsensusHarness::new(leader_index).await;

    // Lock on cert1(1); hold view 2's proposal but no cert for it.
    harness
        .apply_pair(test_data.views[0].proposal_input_consensus(&leader_for_view_3))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[0].cert1_input()).await;
    harness
        .apply(ConsensusInput::FetchedProposal(
            test_data.views[1].proposal_message(),
        ))
        .await;

    // Manual outbox from here: the TC's header request stays unfulfilled, so
    // the leader has not proposed view 3 when the legacy QC arrives.
    let mut outbox = Outbox::new();
    harness
        .consensus
        .apply(test_data.views[1].timeout_cert_input(), &mut outbox);
    assert!(
        !outbox
            .iter()
            .any(|o| matches!(o, ConsensusOutput::PersistProposal(_))),
        "view 3 must not be proposed while the header is outstanding"
    );

    harness
        .consensus
        .register_legacy_qc(&test_data.views[1].cert1);
    assert_eq!(harness.consensus.locked_view(), Some(ViewNumber::new(2)));

    // The TC-time header request (parent view 1) completes; its HeaderCreated
    // retriggers maybe_propose, which must re-request for the adopted lock.
    let old_parent = test_data.views[0].proposal_message().proposal.data;
    let old_commitment = proposal_commitment(&old_parent);
    let old_leaf: Leaf2<TestTypes> = old_parent.into();
    let stale_block = MockBlock::new();
    let stale_header = TestBlockHeader::new(
        &old_leaf,
        stale_block.payload_commitment,
        stale_block.builder_commitment,
        stale_block.metadata,
        TEST_VERSIONS.test.base,
    );
    harness.consensus.apply(
        ConsensusInput::HeaderCreated(ViewNumber::new(3), old_commitment, stale_header),
        &mut outbox,
    );
    let request_epoch = outbox
        .iter()
        .find_map(|o| match o {
            ConsensusOutput::RequestBlockAndHeader(r)
                if r.parent_proposal.view_number == ViewNumber::new(2) =>
            {
                Some(r.epoch)
            },
            _ => None,
        })
        .expect("header must be re-requested for the adopted lock's parent");

    // Fulfill the re-request.
    let parent_proposal = test_data.views[1].proposal_message().proposal.data;
    let parent_commitment = proposal_commitment(&parent_proposal);
    let mock_block = MockBlock::new();
    let parent_leaf: Leaf2<TestTypes> = parent_proposal.into();
    let header = TestBlockHeader::new(
        &parent_leaf,
        mock_block.payload_commitment,
        mock_block.builder_commitment,
        mock_block.metadata,
        TEST_VERSIONS.test.base,
    );
    harness.consensus.apply(
        ConsensusInput::BlockBuilt {
            view: ViewNumber::new(3),
            epoch: request_epoch,
            payload: mock_block.block,
            metadata: mock_block.metadata,
            payload_commitment: mock_block.payload_commitment,
        },
        &mut outbox,
    );
    harness.consensus.apply(
        ConsensusInput::HeaderCreated(ViewNumber::new(3), parent_commitment, header),
        &mut outbox,
    );

    let proposals: Vec<(u64, bool)> = outbox
        .iter()
        .filter_map(|output| match output {
            ConsensusOutput::PersistProposal(p) if *p.data.view_number == 3 => Some((
                *p.data.justify_qc.view_number,
                p.data.view_change_evidence.is_some(),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(
        proposals,
        vec![(2, true)],
        "re-proposal must justify the adopted legacy QC and carry the TC"
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
        .apply_pair(test_data.views[0].proposal_input_consensus(&non_leader_key))
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
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[0].cert1_input()).await;

    // Process view 2 fully → locked_qc = view 2
    harness
        .apply_pair(test_data.views[1].proposal_input_consensus(&node_key))
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
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
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
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[0].cert1_input()).await;

    // Process view 2 proposal + block_reconstructed (need parent data for view 3)
    harness
        .apply_pair(test_data.views[1].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[1].block_reconstructed_input())
        .await;

    harness
        .apply(ConsensusInput::Timeout(
            test_data.views[1].view_number,
            test_data.views[1].epoch_number,
        ))
        .await;
    assert!(
        any(harness.outputs(), is_send_timeout_vote),
        "Timeout should emit timeout vote"
    );

    // Receive timeout cert for view 2 → view advances to 3
    harness.apply(test_data.views[1].timeout_cert_input()).await;
    assert!(
        any(harness.outputs(), is_send_timeout_cert),
        "Timeout certificate should be forwarded"
    );

    let vote1_before = count_matching(harness.outputs(), is_vote1);

    // Process proposal for view 3. Its justify_qc is at view 2, which is
    // >= locked_qc at view 1 (liveness passes). Parent data for view 2 exists.
    harness
        .apply_pair(test_data.views[2].proposal_input_consensus(&node_key))
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
            .apply_pair(test_data.views[i].proposal_input_consensus(&node_key))
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
        .apply_pair(test_data.views[2].proposal_input_consensus(&node_key))
        .await;

    assert_eq!(
        count_matching(harness.outputs(), is_vote1),
        vote1_count,
        "No new Vote1 — proposal with justify_qc below lock rejected by safety rule"
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
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;
    harness
        .apply_pair(test_data.views[1].proposal_input_consensus(&node_key))
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

/// Regression: after restarting from a persisted decided anchor, the first
/// decide must not re-decide the anchor leaf. The `maybe_decide` chain-walk
/// follows `justify_qc` back while `parent_view > last_decided_view`; if a
/// restart leaves `last_decided_view` at genesis instead of the anchor view,
/// the walk re-includes the anchor — which production logs as a spurious
/// "duplicate decided leaf" warning the first decide after every restart.
#[tokio::test]
async fn test_restart_does_not_redecide_anchor() {
    let test_data = TestData::new(4).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Treat view index 1 as the persisted decided anchor; index 2 is the first
    // view decided after the restart and its `justify_qc` certifies the anchor.
    let anchor = &test_data.views[1];
    let decider = &test_data.views[2];
    let anchor_view = anchor.view_number;
    assert_ne!(
        anchor_view,
        ViewNumber::genesis(),
        "anchor must be a non-genesis view to exercise the chain-walk bound"
    );

    let mut harness =
        ConsensusHarness::restarted_from(0, anchor.proposal.data.clone(), anchor.cert1.clone(), [])
            .await;
    assert_eq!(
        harness.consensus.last_decided_view(),
        anchor_view,
        "restart must seed last_decided_view from the anchor, not genesis"
    );

    // Drive the first decide after the restart at the next view.
    harness
        .apply_pair(decider.proposal_input_consensus(&node_key))
        .await;
    harness.apply(decider.block_reconstructed_input()).await;
    harness.apply(decider.cert1_input()).await;
    harness.apply(decider.cert2_input()).await;

    let decided_views: Vec<ViewNumber> = harness
        .outputs()
        .iter()
        .filter_map(|o| match o {
            ConsensusOutput::LeafDecided { leaves, .. } => Some(leaves),
            _ => None,
        })
        .flatten()
        .map(|leaf| leaf.view_number())
        .collect();

    assert!(
        !decided_views.is_empty(),
        "the view after the anchor should decide"
    );
    assert!(
        decided_views.iter().all(|view| *view > anchor_view),
        "the decide re-included the anchor view {anchor_view:?}; decided {decided_views:?}"
    );
}

/// Certificates re-delivered after a restart for views this node already
/// acted in must not re-record the view's Vote action or re-cast its phase-2
/// vote once the Cert2 is known; a late Cert2 must still *decide* the view.
#[tokio::test]
async fn test_no_revote2_for_restart_barred_views() {
    let test_data = TestData::new(4).await;
    // Decided up to view 2 (the anchor) and voted in view 3 before going down.
    let anchor = &test_data.views[1];
    let voted = &test_data.views[2];
    let anchor_view = anchor.view_number;

    let mut harness = ConsensusHarness::restarted_from(
        0,
        anchor.proposal.data.clone(),
        anchor.cert1.clone(),
        [voted.proposal.data.clone()],
    )
    .await;
    // Raise the action bar to the voted view, as `Coordinator::maker` does.
    harness
        .consensus
        .resume_from_restart(anchor_view, anchor_view + 1, voted.view_number);

    // A peer relays the missed Cert2s before re-broadcasting the Cert1s.
    harness.apply(anchor.cert2_input()).await;
    harness.apply(anchor.cert1_input()).await;
    harness.apply(voted.cert2_input()).await;
    harness.apply(voted.cert1_input()).await;

    assert!(
        any(harness.outputs(), |o| decides_view(o, *voted.view_number)),
        "a late cert2 must still decide the undecided barred view"
    );
    assert!(
        !any(harness.outputs(), is_vote2),
        "must not re-cast a phase-2 vote in a view acted in before the restart"
    );
    assert_eq!(
        count_matching(harness.outputs(), is_record_action),
        0,
        "no action may be re-recorded for views acted in before the restart"
    );
}

/// If Cert1 formed before a crash but no Cert2 did, the restarted node must
/// re-cast its (identical) phase-2 vote — without the barred quorum's re-votes
/// the Cert2 can never form — while not re-recording the Vote action.
#[tokio::test]
async fn test_revote2_after_restart_without_cert2() {
    let test_data = TestData::new(4).await;
    let anchor = &test_data.views[1];
    let voted = &test_data.views[2];
    let anchor_view = anchor.view_number;

    let mut harness = ConsensusHarness::restarted_from(
        0,
        anchor.proposal.data.clone(),
        anchor.cert1.clone(),
        [voted.proposal.data.clone()],
    )
    .await;
    harness
        .consensus
        .resume_from_restart(anchor_view, anchor_view + 1, voted.view_number);

    // A peer re-broadcasts Cert1 for the voted view; no Cert2 exists anywhere.
    harness.apply(voted.cert1_input()).await;

    assert_eq!(
        count_matching(harness.outputs(), is_vote2),
        1,
        "the phase-2 vote must be re-cast after restart when no cert2 exists"
    );
    assert_eq!(
        count_matching(harness.outputs(), is_record_action),
        0,
        "re-casting the vote must not re-record the view's Vote action"
    );
}

/// A replayed Cert1+Cert2 pair for a view below the restart anchor (decided
/// in the previous run) must not re-decide it: the floor is pinned there.
#[tokio::test]
async fn test_no_redecide_below_restart_anchor() {
    let test_data = TestData::new(4).await;
    let decided = &test_data.views[1];
    let anchor = &test_data.views[2];
    let anchor_view = anchor.view_number;

    let mut harness =
        ConsensusHarness::restarted_from(0, anchor.proposal.data.clone(), anchor.cert1.clone(), [])
            .await;
    // Make the pre-anchor proposal available so only the floor stands between
    // the replayed certificates and a duplicate decide.
    harness
        .consensus
        .seed_proposals([decided.proposal.data.clone()]);
    harness
        .consensus
        .resume_from_restart(anchor_view, anchor_view + 1, anchor_view);

    harness.apply(decided.cert1_input()).await;
    harness.apply(decided.cert2_input()).await;

    assert!(
        !any(harness.outputs(), is_leaf_decided),
        "a replayed certificate pair below the restart anchor must not re-decide"
    );
}

/// A timeout for a view below `current_view` is ignored entirely: no
/// timeout vote is signed or broadcast. Regression test for restarted
/// nodes being dragged back to long-past views (e.g. the protocol-upgrade
/// cutover) by joining stale timeouts via `TimeoutOneHonest`.
#[tokio::test]
async fn test_stale_timeout_ignored() {
    let mut harness = ConsensusHarness::new(0).await;

    harness
        .consensus
        .set_view(ViewNumber::new(5), EpochNumber::genesis());

    // Timeout for view 2 (< current view 5) must not produce a vote.
    harness
        .apply(ConsensusInput::Timeout(
            ViewNumber::new(2),
            EpochNumber::genesis(),
        ))
        .await;
    assert!(
        !any(harness.outputs(), is_send_timeout_vote),
        "must not sign a timeout vote for a view below the current view"
    );

    // Timeout at the current view still produces a vote.
    harness
        .apply(ConsensusInput::Timeout(
            ViewNumber::new(5),
            EpochNumber::genesis(),
        ))
        .await;
    assert!(
        any(harness.outputs(), is_send_timeout_vote),
        "timeout at the current view must still produce a vote"
    );
}

/// A timeout certificate advancing into a view below `current_view` is
/// ignored: no `ViewChanged` is emitted and the certificate is not
/// rebroadcast. Regression test: stale TCs (e.g. formed around the
/// protocol-upgrade cutover by restarted nodes) must not drag a caught-up
/// node's view backwards.
#[tokio::test]
async fn test_stale_timeout_certificate_ignored() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(6).await;

    harness
        .consensus
        .set_view(ViewNumber::new(5), EpochNumber::genesis());

    // TC certifying view 2 advances into view 3 (< current view 5) — ignored.
    harness.apply(test_data.views[1].timeout_cert_input()).await;

    assert!(
        !any(harness.outputs(), is_view_changed),
        "stale timeout certificate must not change the view"
    );
    assert!(
        !any(harness.outputs(), is_send_timeout_cert),
        "stale timeout certificate must not be rebroadcast"
    );

    // A TC certifying the current view (advancing into view 6) still works.
    harness.apply(test_data.views[4].timeout_cert_input()).await;
    assert!(
        any(harness.outputs(), is_view_changed),
        "timeout certificate over the current view must still advance the view"
    );
    assert!(
        any(harness.outputs(), is_send_timeout_cert),
        "timeout certificate over the current view must still be forwarded"
    );
}

/// Vote1 is held until the Vote action and the proposal are durably stored,
/// and the action record is requested exactly once.
#[tokio::test]
async fn test_vote1_gated_on_storage() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(2).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;
    let view = ViewNumber::new(1);
    let commit = proposal_commitment(&test_data.views[0].proposal.data);
    let consensus = &mut harness.consensus;
    let mut outbox = Outbox::new();

    let (proposal_input, vid_share_input) = test_data.views[0].proposal_input_consensus(&node_key);
    consensus.apply(proposal_input, &mut outbox);
    consensus.apply(vid_share_input, &mut outbox);
    consensus.apply(
        state_verified_input(&test_data.views[0].proposal.data, view),
        &mut outbox,
    );

    assert!(!any(&outbox, is_vote1), "vote1 must wait for storage");
    assert_eq!(count_matching(&outbox, is_record_action), 1);

    consensus.apply(
        ConsensusInput::Stored(StorageOutput::Action(view, ActionKind::Vote)),
        &mut outbox,
    );
    assert!(!any(&outbox, is_vote1), "vote1 must wait for the proposal");

    consensus.apply(
        ConsensusInput::Stored(StorageOutput::Proposal(view, commit)),
        &mut outbox,
    );
    assert_eq!(count_matching(&outbox, is_vote1), 1);

    consensus.apply(
        ConsensusInput::Stored(StorageOutput::Proposal(view, commit)),
        &mut outbox,
    );
    assert_eq!(
        count_matching(&outbox, is_vote1),
        1,
        "re-delivered confirmations must not double-send"
    );
}

/// The lock update and view change fire as soon as cert1 is valid, but
/// vote2 is held until the node's own VID share is durably stored.  Vote1
/// and vote2 for the same view share a single action record.
#[tokio::test]
async fn test_vote2_gated_on_vid_storage() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;
    let view = ViewNumber::new(1);
    let commit = proposal_commitment(&test_data.views[0].proposal.data);
    let consensus = &mut harness.consensus;
    let mut outbox = Outbox::new();

    let (proposal_input, vid_share_input) = test_data.views[0].proposal_input_consensus(&node_key);
    consensus.apply(proposal_input, &mut outbox);
    consensus.apply(vid_share_input, &mut outbox);
    consensus.apply(
        state_verified_input(&test_data.views[0].proposal.data, view),
        &mut outbox,
    );
    consensus.apply(
        ConsensusInput::Stored(StorageOutput::Action(view, ActionKind::Vote)),
        &mut outbox,
    );
    consensus.apply(
        ConsensusInput::Stored(StorageOutput::Proposal(view, commit)),
        &mut outbox,
    );
    assert_eq!(count_matching(&outbox, is_vote1), 1);

    consensus.apply(test_data.views[0].block_reconstructed_input(), &mut outbox);
    consensus.apply(test_data.views[0].cert1_input(), &mut outbox);

    assert!(any(&outbox, is_view_changed), "lock update is not gated");
    assert!(!any(&outbox, is_vote2), "vote2 must wait for the VID share");

    consensus.apply(
        ConsensusInput::Stored(StorageOutput::Vid(view)),
        &mut outbox,
    );
    assert!(
        !any(&outbox, is_vote2),
        "vote2 must wait for the locked QC to be persisted"
    );

    consensus.apply(
        ConsensusInput::Stored(StorageOutput::HighQc(view)),
        &mut outbox,
    );
    assert_eq!(count_matching(&outbox, is_vote2), 1);
    assert_eq!(
        count_matching(&outbox, is_record_action),
        1,
        "vote1 and vote2 share one action record per view"
    );
}

/// A pending vote1 is dropped if its view times out before the storage
/// confirmations arrive.
#[tokio::test]
async fn test_pending_vote1_dropped_on_timeout() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(2).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;
    let view = ViewNumber::new(1);
    let commit = proposal_commitment(&test_data.views[0].proposal.data);
    let consensus = &mut harness.consensus;
    let mut outbox = Outbox::new();

    let (proposal_input, vid_share_input) = test_data.views[0].proposal_input_consensus(&node_key);
    consensus.apply(proposal_input, &mut outbox);
    consensus.apply(vid_share_input, &mut outbox);
    consensus.apply(
        state_verified_input(&test_data.views[0].proposal.data, view),
        &mut outbox,
    );
    assert_eq!(count_matching(&outbox, is_record_action), 1);

    consensus.apply(
        ConsensusInput::Timeout(view, EpochNumber::genesis()),
        &mut outbox,
    );
    consensus.apply(
        ConsensusInput::Stored(StorageOutput::Action(view, ActionKind::Vote)),
        &mut outbox,
    );
    consensus.apply(
        ConsensusInput::Stored(StorageOutput::Proposal(view, commit)),
        &mut outbox,
    );
    assert!(
        !any(&outbox, is_vote1),
        "vote1 for a timed-out view must not be released"
    );
}

/// SendProposal is released only after the proposal is persisted and the
/// Propose action is recorded.
#[tokio::test]
async fn test_proposal_release_follows_storage() {
    let test_data = TestData::new(4).await;
    let leader_for_view_2 = test_data.views[1].leader_public_key;
    let leader_index = node_index_for_key(&leader_for_view_2);
    let mut harness = ConsensusHarness::new(leader_index).await;

    harness
        .apply_pair(test_data.views[0].proposal_input_consensus(&leader_for_view_2))
        .await;
    harness.apply(test_data.views[0].cert1_input()).await;

    let outputs: Vec<_> = harness.outputs().iter().cloned().collect();
    let send = outputs.iter().position(is_proposal).expect("proposal sent");
    let persist = outputs
        .iter()
        .position(is_persist_proposal)
        .expect("proposal persisted");
    let action = outputs
        .iter()
        .position(|o| matches!(o, ConsensusOutput::RecordAction(_, _, ActionKind::Propose)))
        .expect("propose action recorded");
    assert!(persist < send, "proposal must be persisted before sending");
    assert!(
        action < send,
        "propose action must be recorded before sending"
    );
}

/// Seeded proposals land in `self.proposals` and surface as undecided leaves.
#[tokio::test]
async fn test_seed_proposals_populates_undecided_chain() {
    let test_data = TestData::new(4).await;
    let mut harness = ConsensusHarness::new(0).await;

    harness
        .consensus
        .seed_proposals(test_data.views.iter().map(|v| v.proposal.data.clone()));

    for view in &test_data.views {
        let seeded = harness
            .consensus
            .proposal_at(view.view_number)
            .expect("seeded proposal available");
        assert_eq!(
            proposal_commitment(seeded),
            proposal_commitment(&view.proposal.data),
            "seeded proposal at view {} differs from the persisted proposal",
            view.view_number
        );
    }

    let undecided: Vec<_> = harness
        .consensus
        .undecided_leaves()
        .map(|leaf| leaf.view_number())
        .collect();
    assert_eq!(
        undecided,
        test_data
            .views
            .iter()
            .map(|v| v.view_number)
            .collect::<Vec<_>>(),
        "every seeded proposal should surface as an undecided leaf"
    );
}
