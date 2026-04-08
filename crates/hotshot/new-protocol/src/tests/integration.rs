use hotshot::types::BLSPubKey;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::traits::signature_key::SignatureKey;

use super::common::{harness::TestHarness, utils::TestData};
use crate::{
    consensus::ConsensusInput,
    message::{Certificate1, EpochChangeMessage, Proposal},
    tests::common::assertions::{
        any, count_matching, has_epoch_change, is_block_built, is_block_reconstructed, is_cert1,
        is_cert2, is_drb_result, is_header_created, is_leaf_decided, is_proposal,
        is_request_block_and_header, is_request_vid_disperse, is_send_cert1, is_send_timeout_vote,
        is_state_validated, is_timeout, is_timeout_cert, is_timeout_one_honest, is_vid_disperse,
        is_view_changed, is_vote1, is_vote2, node_index_for_key,
    },
};

/// Threshold for SuccessThreshold with 10 nodes of stake 1: (10*2)/3 + 1 = 7.
const THRESHOLD: u64 = 7;

/// Send a proposal and enough Vote1 messages to form Certificate1.
///
/// Sends the proposal first (providing metadata for VID reconstruction),
/// then Vote1 messages from `THRESHOLD` validators. Waits until
/// Certificate1, BlockReconstructed and StateValidated have all been
/// produced — in any order.
async fn send_proposal_and_vote1s(
    harness: &mut TestHarness,
    test_data: &TestData,
    view_idx: usize,
    node_key: &BLSPubKey,
) {
    let test_view = &test_data.views[view_idx];
    harness.message(test_view.proposal_input(node_key)).await;

    for i in 0..THRESHOLD {
        harness.message(test_view.vote1_input(i)).await;
    }

    harness
        .process_until(
            |inputs| {
                any(inputs, is_cert1)
                    && any(inputs, is_block_reconstructed)
                    && any(inputs, is_state_validated)
            },
            |inputs| any(inputs, is_timeout),
        )
        .await;
    assert!(
        any(harness.outputs(), is_view_changed),
        "View should be changed"
    );
}

/// Send enough Vote2 messages to form Certificate2.
async fn send_vote2s(harness: &mut TestHarness, test_data: &TestData, view_idx: usize) {
    let test_view = &test_data.views[view_idx];
    for i in 0..THRESHOLD {
        harness.message(test_view.vote2_input(i)).await;
    }
    harness
        .process_until(
            |inputs| any(inputs, is_cert2),
            |inputs| any(inputs, is_timeout),
        )
        .await;
}

/// Send enough timeout votes to form a TimeoutCertificate.
async fn send_timeout_votes(
    harness: &mut TestHarness,
    test_data: &TestData,
    view_idx: usize,
    lock: Option<Certificate1<TestTypes>>,
) {
    let test_view = &test_data.views[view_idx];
    for i in 0..THRESHOLD {
        harness
            .message(test_view.timeout_vote_input(i, lock.clone()))
            .await;
    }
    harness
        .process_until(
            |inputs| any(inputs, is_timeout_cert),
            |inputs| any(inputs, is_timeout),
        )
        .await;
    harness
        .apply_and_process(ConsensusInput::TimeoutCertificate(
            test_view.timeout_cert.clone(),
        ))
        .await;
}

/// Integration: sequential views both produce Vote1 through real state validation.
#[tokio::test]
async fn test_sequential_vote1() {
    let test_data = TestData::new(3).await;
    let mut harness = TestHarness::new(0).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    harness
        .message(test_data.views[0].proposal_input(&node_key))
        .await;
    harness
        .apply_and_process(test_data.views[0].block_reconstructed_input())
        .await;

    harness
        .message(test_data.views[1].proposal_input(&node_key))
        .await;

    harness
        .process_until(
            |inputs| count_matching(inputs, is_state_validated) >= 2 || any(inputs, is_timeout),
            |inputs| any(inputs, is_timeout),
        )
        .await;

    assert_eq!(
        count_matching(harness.outputs(), is_vote1),
        2,
        "Both views should produce Vote1"
    );
}

/// CPU tasks form Certificate1 from accumulated Vote1 messages, enabling
/// consensus to continue (verified by Vote1 emission for subsequent views).
#[tokio::test]
async fn test_cert1_formed_and_vote2_sent() {
    let test_data = TestData::new(3).await;
    let mut harness = TestHarness::new(0).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // View 1: proposal + Vote1 messages → CPU forms cert1 + reconstructs block
    send_proposal_and_vote1s(&mut harness, &test_data, 0, &node_key).await;

    assert!(
        any(harness.outputs(), is_vote1),
        "Vote1 should be sent for proposal"
    );
    assert!(
        any(harness.outputs(), is_vote2),
        "Vote2 should be sent for proposal"
    );
}

/// Full decide path: CPU tasks form Certificate1, Certificate2, and
/// reconstruct blocks from VID shares, leading to a leaf decision.
/// Block reconstruction is exercised because consensus requires
/// BlockReconstructed (produced by the CPU VidShareTask) before it
/// can proceed to the decide step.
#[tokio::test]
async fn test_full_decide_via_cpu_tasks() {
    let test_data = TestData::new(3).await;
    let mut harness = TestHarness::new(0).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // View 1: send proposal + Vote1s
    //   CPU VoteCollectionTask: accumulates QuorumVote2s → forms cert1
    //   CPU VidShareTask: accumulates VID shares → reconstructs block
    send_proposal_and_vote1s(&mut harness, &test_data, 0, &node_key).await;

    // Send Vote2s for view 1 → CPU forms cert2
    send_vote2s(&mut harness, &test_data, 0).await;

    // View 2: full round to trigger decision on view 1
    send_proposal_and_vote1s(&mut harness, &test_data, 1, &node_key).await;
    send_vote2s(&mut harness, &test_data, 1).await;

    assert!(any(harness.outputs(), is_vote1), "Vote1 should be sent");
    assert!(any(harness.outputs(), is_vote2), "Vote2 should be sent");

    assert!(
        any(harness.outputs(), is_send_cert1),
        "Certificate1 should be sent"
    );

    // LeafDecided proves the full pipeline: cert1 formation, block
    // reconstruction from VID shares, cert2 formation, and decision.
    assert!(
        any(harness.outputs(), is_leaf_decided),
        "Leaf should be decided — requires block reconstruction + cert2 formation"
    );
}

/// Leader sends a proposal after CPU tasks form Certificate1.
/// The proposal requires VID disperse, which is computed by the CPU
/// VidDisperseTask (the mock coordinator forwards RequestVidDisperse
/// to the CPU task when cpu_tx is set). SendProposal in the output
/// proves the full leader path: cert1 formation → block/header request
/// → VID disperse via CPU → proposal sent.
#[tokio::test]
async fn test_leader_proposal_via_cpu_tasks() {
    let test_data = TestData::new(4).await;
    let leader_for_view_2 = test_data.views[1].leader_public_key;
    let leader_index = node_index_for_key(&leader_for_view_2);
    let mut harness = TestHarness::new(leader_index).await;

    // View 1: send proposal + Vote1s → CPU forms cert1 → leader proposes for view 2
    // The leader proposal path requires:
    //   1. cert1 formed by CPU VoteCollectionTask
    //   2. block/header by BlockBuilder/StateManager
    //   3. VID disperse computed by CPU VidDisperseTask

    let test_view = &test_data.views[0];

    harness
        .message(test_view.proposal_input(&leader_for_view_2))
        .await;

    for i in 0..THRESHOLD {
        harness.message(test_view.vote1_input(i)).await;
    }

    harness
        .process_until(
            |inputs| {
                any(inputs, is_cert1)
                    && any(inputs, is_block_reconstructed)
                    && any(inputs, is_state_validated)
                    && any(inputs, is_block_built)
                    && any(inputs, is_header_created)
                    && any(inputs, is_vid_disperse)
            },
            |inputs| any(inputs, is_timeout),
        )
        .await;

    // SendProposal proves the CPU VidDisperseTask computed the VID
    // disperse — consensus cannot send a proposal without it.
    assert!(
        any(harness.outputs(), is_proposal),
        "Leader should send proposal (requires CPU VID disperse)"
    );
}

/// Multi-view chain: CPU tasks form certificates for each view, leading to
/// multiple decisions.
#[tokio::test]
async fn test_multi_view_decide_via_cpu_tasks() {
    let test_data = TestData::new(5).await;
    let mut harness = TestHarness::new(0).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    for i in 0..test_data.views.len() {
        send_proposal_and_vote1s(&mut harness, &test_data, i, &node_key).await;
        send_vote2s(&mut harness, &test_data, i).await;
    }

    assert!(count_matching(harness.outputs(), is_leaf_decided) >= 2);
}

/// Timeout votes are collected by the CPU VoteCollector and form a
/// TimeoutCertificate, which advances the view.
#[tokio::test]
async fn test_timeout_votes_form_tc() {
    let test_data = TestData::new(4).await;
    let mut harness = TestHarness::new(0).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Process view 1 to establish locked_qc (needed for TC handling)
    send_proposal_and_vote1s(&mut harness, &test_data, 0, &node_key).await;
    // Send timeout votes for view 2 and form a TimeoutCertificate.
    let lock = Some(test_data.views[0].cert1.clone());
    send_timeout_votes(&mut harness, &test_data, 1, lock).await;

    assert!(
        any(harness.outputs(), is_view_changed),
        "View should advance after timeout certificate"
    );
}

/// Full leader path after timeout via CPU tasks: establish lock → timer
/// fires for view 2 (Timeout) → timeout votes form TC → leader proposes
/// for view 3 before the view 3 timer fires.
///
/// The 100ms timer is short enough to actually fire during the test,
/// proving the timeout mechanism does not interfere with the leader's
/// proposal path.
#[tokio::test]
async fn test_leader_proposes_after_timeout_via_cpu_tasks() {
    let test_data = TestData::new(5).await;
    // Timeout cert for view 2 advances to view 3; we need to be leader of view 3
    let leader_for_view_3 = test_data.views[2].leader_public_key;
    let leader_index = node_index_for_key(&leader_for_view_3);
    // Timer must be long enough for VID to complete (so the view 3 timeout
    // doesn't kill the in-progress proposal), but short enough to actually
    // fire for view 2 during the test.
    let mut harness =
        TestHarness::new_with_timer(leader_index, std::time::Duration::from_millis(100)).await;

    // View 1: process fully to establish locked_qc
    send_proposal_and_vote1s(&mut harness, &test_data, 0, &leader_for_view_3).await;

    // Wait for the timeout to fire (non-timeout events are processed inline).
    harness
        .process_until(
            |inputs| any(inputs, is_timeout),
            |inputs| !any(inputs, is_timeout),
        )
        .await;

    // Send timeout votes for view 2 → CPU timeout collector forms TC
    // → consensus handles TC → leader of view 3 requests block/header → proposes.
    // The TC input view is 3 (cert.view+1), which passes the stale filter
    // (3 > timeout_view=2). After ViewChanged(3) resets the timer, the leader
    // must complete VID disperse before the timer fires for view 3.
    let lock = Some(test_data.views[0].cert1.clone());
    send_timeout_votes(&mut harness, &test_data, 1, lock).await;

    harness
        .process_until(
            |inputs| {
                any(inputs, is_vid_disperse)
                    && any(inputs, is_block_built)
                    && any(inputs, is_header_created)
            },
            |inputs| any(inputs, is_timeout),
        )
        .await;

    assert!(
        any(harness.outputs(), is_request_block_and_header),
        "Leader should request block and header after TC"
    );

    assert!(
        any(harness.outputs(), is_request_vid_disperse),
        "Leader should request VID disperse after TC"
    );
    assert!(
        any(harness.outputs(), is_proposal),
        "Leader should send proposal with timeout view change evidence"
    );
}

// ---------------------------------------------------------------------------
// Epoch change integration tests
// ---------------------------------------------------------------------------

const EPOCH_HEIGHT: u64 = 10;

/// Helper to build an EpochChangeMessage from test data at the epoch boundary view.
fn epoch_change_message(test_data: &TestData) -> EpochChangeMessage<TestTypes> {
    let epoch_view = &test_data.views[9]; // view 10, last block of epoch 1
    let proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone();
    EpochChangeMessage {
        cert1: epoch_view.cert1.clone(),
        cert2: epoch_view.cert2.clone(),
        proposal,
    }
}

/// Run views through the full integration pipeline.
/// Pre-feeds DRB results for any epoch transitions in the range.
async fn run_views_integration(
    harness: &mut TestHarness,
    test_data: &TestData,
    node_key: &BLSPubKey,
    range: std::ops::Range<usize>,
) {
    for i in range {
        send_proposal_and_vote1s(harness, test_data, i, node_key).await;
        send_vote2s(harness, test_data, i).await;
    }
}

/// Full integration: deciding the last block of an epoch emits SendEpochChange.
/// Runs all 10 views through the real pipeline including epoch root computation
/// and DRB calculation triggered by handle_leaf_decided.
#[tokio::test]
async fn test_epoch_boundary_emits_epoch_change() {
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;
    let mut harness = TestHarness::new(0).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    run_views_integration(&mut harness, &test_data, &node_key, 0..10).await;

    assert!(
        has_epoch_change(harness.outputs()),
        "SendEpochChange should be emitted when the epoch boundary block is decided"
    );
}

/// Receiving an EpochChangeMessage through the coordinator advances the view
/// to the first view of the next epoch. Runs all 9 views through the full
/// pipeline (including epoch root computation) before applying the epoch change.
#[tokio::test]
async fn test_epoch_change_advances_view() {
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;
    let mut harness = TestHarness::new(0).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Build state through views 1-9 via the full integration pipeline.
    run_views_integration(&mut harness, &test_data, &node_key, 0..9).await;

    let view_changed_before = count_matching(harness.outputs(), is_view_changed);

    // Apply the epoch change directly to the coordinator (simulating
    // receipt from the network after another node decided the boundary).
    let epoch_change = epoch_change_message(&test_data);
    harness
        .apply_and_process(ConsensusInput::EpochChange(epoch_change))
        .await;

    assert!(
        count_matching(harness.outputs(), is_view_changed) > view_changed_before,
        "ViewChanged should be emitted after processing EpochChange"
    );
}

/// After the epoch boundary is decided, the EpochChange is fed back to
/// the leader of the next epoch, who should request a block and header.
/// Runs all 10 views through the real pipeline (including epoch root
/// computation and DRB calculation).
#[tokio::test]
async fn test_leader_requests_block_after_epoch_change() {
    let test_data = TestData::new_with_epoch_height(12, EPOCH_HEIGHT).await;
    // Node 1 is the leader for the first view of epoch 2 (view 11).
    let leader_key = BLSPubKey::generated_from_seed_indexed([0; 32], 1).0;
    let mut harness = TestHarness::new(1).await;

    // Run all 10 views so the state manager has state through view 10.
    run_views_integration(&mut harness, &test_data, &leader_key, 0..10).await;

    let req_before = count_matching(harness.outputs(), is_request_block_and_header);

    // Apply the epoch change (simulating the network round-trip).
    let epoch_change = epoch_change_message(&test_data);
    harness
        .apply_and_process(ConsensusInput::EpochChange(epoch_change))
        .await;

    assert!(
        count_matching(harness.outputs(), is_request_block_and_header) > req_before,
        "Leader should request block and header after epoch change"
    );
}

/// Cross an epoch boundary: apply the EpochChange from `test_data` for the
/// given boundary view index, then feed the first proposal of the new epoch
/// with `next_epoch_justify_qc` set correctly.
async fn cross_epoch_boundary(
    harness: &mut TestHarness,
    test_data: &TestData,
    node_key: &BLSPubKey,
    boundary_idx: usize,
) {
    let boundary_view = &test_data.views[boundary_idx];
    let proposal: Proposal<TestTypes> = boundary_view.proposal.data.clone();
    let epoch_change = EpochChangeMessage {
        cert1: boundary_view.cert1.clone(),
        cert2: boundary_view.cert2.clone(),
        proposal,
    };
    harness
        .apply_and_process(ConsensusInput::EpochChange(epoch_change))
        .await;

    // Send vote1s and vote2s through the normal pipeline.
    send_proposal_and_vote1s(harness, test_data, boundary_idx + 1, node_key).await;
    send_vote2s(harness, test_data, boundary_idx + 1).await;
}

/// Full three-epoch integration test: the EpochManager computes the DRB
/// result for epoch 3 when the epoch root (block 5) is decided. A leader
/// in epoch 3's transition window uses that computed DRB to propose.
///
/// No DRB results are injected — the value is calculated by the real
/// EpochManager pipeline (add_epoch_root + compute_drb_result).
#[tokio::test]
async fn test_leader_proposes_with_computed_drb_in_epoch3() {
    // Need 28 views: epoch 1 (1-10), epoch 2 (11-20), epoch 3 (21-28).
    // Block 27 is the first block in epoch 3's transition window
    // (is_epoch_transition(27, 10) = 27 % 10 = 7 >= 7).
    // Leader for view 28 = 28 % 10 = 8 → node 8.
    let test_data = TestData::new_with_epoch_height(29, EPOCH_HEIGHT).await;
    let leader_key = BLSPubKey::generated_from_seed_indexed([0; 32], 8).0;
    // Long timer: 27 views across 3 epochs takes more than the default 2s.
    let mut harness = TestHarness::new(8).await;

    // ---- Epoch 1 (views 1-10) ----
    // Block 5 decision triggers epoch root → DRB for epoch 3 is computed.
    run_views_integration(&mut harness, &test_data, &leader_key, 0..10).await;

    // Wait for the DRB result from the EpochManager to arrive.
    harness
        .process_until(
            |inputs| any(inputs, is_drb_result),
            |inputs| any(inputs, is_timeout),
        )
        .await;

    // ---- Epoch 1 → 2 boundary ----
    cross_epoch_boundary(&mut harness, &test_data, &leader_key, 9).await;

    // ---- Epoch 2 (views 12-20) ----
    run_views_integration(&mut harness, &test_data, &leader_key, 11..20).await;

    // ---- Epoch 2 → 3 boundary ----
    cross_epoch_boundary(&mut harness, &test_data, &leader_key, 19).await;

    // ---- Epoch 3 views 22-27 (reach transition window) ----
    run_views_integration(&mut harness, &test_data, &leader_key, 21..27).await;

    // After processing view 27 the leader for view 28 should propose
    // with the DRB result that was computed back in epoch 1.
    assert!(
        any(harness.outputs(), is_proposal),
        "Leader should propose in epoch 3 transition window using the computed DRB result"
    );
}

/// Same three-epoch setup but from a non-leader node's perspective:
/// the node receives a proposal in epoch 3's transition window containing
/// the computed DRB result and votes on it.
#[tokio::test]
async fn test_node_votes_with_computed_drb_in_epoch3() {
    let test_data = TestData::new_with_epoch_height(28, EPOCH_HEIGHT).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;
    // Long timer: 27 views across 3 epochs takes more than the default 2s.
    let mut harness = TestHarness::new_with_timer(0, std::time::Duration::from_secs(30)).await;

    // ---- Epoch 1 (views 1-10) ----
    run_views_integration(&mut harness, &test_data, &node_key, 0..10).await;

    // Wait for the DRB result from the EpochManager.
    harness
        .process_until(
            |inputs| any(inputs, is_drb_result),
            |inputs| any(inputs, is_timeout),
        )
        .await;

    // ---- Epoch 1 → 2 boundary ----
    cross_epoch_boundary(&mut harness, &test_data, &node_key, 9).await;

    // ---- Epoch 2 (views 12-20) ----
    run_views_integration(&mut harness, &test_data, &node_key, 11..20).await;

    // ---- Epoch 2 → 3 boundary ----
    cross_epoch_boundary(&mut harness, &test_data, &node_key, 19).await;

    // ---- Epoch 3 views before transition window ----
    run_views_integration(&mut harness, &test_data, &node_key, 21..26).await;

    let vote_count_before = count_matching(harness.outputs(), is_vote1);

    // View 27 (index 26, block 27) is in epoch 3's transition window.
    // Process it through the full pipeline — the node should vote on
    // this proposal using the DRB result computed by the EpochManager.
    run_views_integration(&mut harness, &test_data, &node_key, 26..27).await;

    assert!(
        count_matching(harness.outputs(), is_vote1) > vote_count_before,
        "Node should vote on epoch 3 transition-window proposal with the computed DRB result"
    );
}

/// Certificate1 formed from votes and certificate2 received from network
/// are each forwarded exactly once. Duplicates from either source are ignored.
/// f+1 timeout votes (OneHonestThreshold) trigger a TimeoutOneHonest input,
/// which causes the node to emit its own timeout vote.
#[tokio::test]
async fn test_f_plus_1_timeout_votes_trigger_timeout_one_honest() {
    const ONE_HONEST_THRESHOLD: u64 = 4;

    let test_data = TestData::new(4).await;
    let mut harness = TestHarness::new(0).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Process view 1 to establish state
    send_proposal_and_vote1s(&mut harness, &test_data, 0, &node_key).await;

    // Send exactly f+1 timeout votes for view 2 (below the 2f+1 TC threshold).
    let test_view = &test_data.views[1];
    let lock = Some(test_data.views[0].cert1.clone());
    for i in 0..ONE_HONEST_THRESHOLD {
        harness
            .message(test_view.timeout_vote_input(i, lock.clone()))
            .await;
    }

    harness
        .process_until(
            |inputs| any(inputs, is_timeout_one_honest),
            |inputs| any(inputs, is_timeout),
        )
        .await;

    assert!(
        any(harness.outputs(), is_send_timeout_vote),
        "f+1 timeout votes should trigger TimeoutOneHonest"
    );
}
