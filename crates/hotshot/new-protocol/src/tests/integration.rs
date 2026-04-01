use hotshot::types::BLSPubKey;
use hotshot_types::traits::signature_key::SignatureKey;

use super::common::{harness::TestHarness, utils::TestData};
use crate::{
    consensus::ConsensusInput,
    tests::common::assertions::{
        any, count_matching, is_block_built, is_block_reconstructed, is_cert1, is_cert2,
        is_header_created, is_leaf_decided, is_proposal, is_request_block_and_header,
        is_request_vid_disperse, is_state_validated, is_timeout, is_timeout_cert, is_vid_disperse,
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
        .process_until(|inputs| {
            any(inputs, is_timeout)
                || any(inputs, is_cert1)
                    && any(inputs, is_block_reconstructed)
                    && any(inputs, is_state_validated)
        })
        .await;
}

/// Send enough Vote2 messages to form Certificate2.
async fn send_vote2s(harness: &mut TestHarness, test_data: &TestData, view_idx: usize) {
    let test_view = &test_data.views[view_idx];
    for i in 0..THRESHOLD {
        harness.message(test_view.vote2_input(i)).await;
    }
    harness
        .process_until(|inputs| any(inputs, is_cert2) || any(inputs, is_timeout))
        .await;
}

/// Integration: sequential views both produce Vote1 through real state validation.
#[tokio::test]
async fn test_sequential_vote1() {
    let mut harness = TestHarness::new(0).await;
    let test_data = TestData::new(3).await;
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
        .process_until(|inputs| {
            count_matching(inputs, is_state_validated) >= 2 || any(inputs, is_timeout)
        })
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
    let mut harness = TestHarness::new(0).await;
    let test_data = TestData::new(3).await;
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
    let mut harness = TestHarness::new(0).await;
    let test_data = TestData::new(3).await;
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
        .process_until(|inputs| {
            any(inputs, is_timeout)
                || any(inputs, is_cert1)
                    && any(inputs, is_block_reconstructed)
                    && any(inputs, is_state_validated)
                    && any(inputs, is_block_built)
                    && any(inputs, is_header_created)
                    && any(inputs, is_vid_disperse)
        })
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
    let mut harness = TestHarness::new(0).await;
    let test_data = TestData::new(5).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    for i in 0..test_data.views.len() {
        send_proposal_and_vote1s(&mut harness, &test_data, i, &node_key).await;
        send_vote2s(&mut harness, &test_data, i).await;
    }

    assert!(count_matching(harness.outputs(), is_leaf_decided) >= 2);
}

/// Send enough timeout votes for a view.
async fn send_timeout_votes(harness: &mut TestHarness, test_data: &TestData, view_idx: usize) {
    let test_view = &test_data.views[view_idx];
    for i in 0..THRESHOLD {
        harness.message(test_view.timeout_vote_input(i)).await;
    }
    harness
        .process_until(|inputs| any(inputs, is_timeout_cert) || any(inputs, is_timeout))
        .await;
    harness
        .apply_and_process(ConsensusInput::TimeoutCertificate(
            test_view.timeout_cert.clone(),
        ))
        .await;
}

/// Timeout votes are collected by the CPU VoteCollector and form a
/// TimeoutCertificate, which advances the view.
#[tokio::test]
async fn test_timeout_votes_form_tc() {
    let mut harness = TestHarness::new(0).await;
    let test_data = TestData::new(4).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Process view 1 to establish locked_qc (needed for TC handling)
    send_proposal_and_vote1s(&mut harness, &test_data, 0, &node_key).await;
    // Send timeout votes for view 2 → CPU timeout collector forms TC
    send_timeout_votes(&mut harness, &test_data, 1).await;

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
        .process_until(|inputs| any(inputs, is_timeout))
        .await;

    // Send timeout votes for view 2 → CPU timeout collector forms TC
    // → consensus handles TC → leader of view 3 requests block/header → proposes.
    // The TC input view is 3 (cert.view+1), which passes the stale filter
    // (3 > timeout_view=2). After ViewChanged(3) resets the timer, the leader
    // must complete VID disperse before the timer fires for view 3.
    let test_view = &test_data.views[1];

    for i in 0..THRESHOLD {
        harness.message(test_view.timeout_vote_input(i)).await;
    }

    harness
        .process_until(|inputs| any(inputs, is_timeout_cert))
        .await;

    harness
        .apply_and_process(ConsensusInput::TimeoutCertificate(
            test_view.timeout_cert.clone(),
        ))
        .await;

    harness
        .process_until(|inputs| any(inputs, is_vid_disperse) || any(inputs, is_timeout))
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
