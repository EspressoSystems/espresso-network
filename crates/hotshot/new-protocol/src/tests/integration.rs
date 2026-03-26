use hotshot::types::BLSPubKey;
use hotshot_types::traits::signature_key::SignatureKey;

use super::common::{
    assertions::{
        has_leaf_decided, has_proposal, has_request_block_and_header, has_timeout_cert,
        has_view_changed, has_vote1, node_index_for_key,
    },
    harness::TestHarness,
    utils::TestData,
};
use crate::{
    events::{ConsensusOutput, Event},
    tests::common::assertions::{
        has_block_reconstructed, has_cert1, has_request_vid_disperse, has_vid_disperse, has_vote2,
    },
};

/// Threshold for SuccessThreshold with 10 nodes of stake 1: (10*2)/3 + 1 = 7.
const THRESHOLD: u64 = 7;

/// Send a proposal and enough Vote1 messages to form Certificate1.
///
/// Sends the proposal first (providing metadata for VID reconstruction),
/// then Vote1 messages from `THRESHOLD` validators. Our own node's vote
/// from consensus goes through `Action::SendVote1` which the mock
/// coordinator doesn't forward to the CPU task, so we need all THRESHOLD
/// votes to come from external Vote1 messages.
async fn send_proposal_and_vote1s(
    harness: &TestHarness,
    test_data: &TestData,
    view_idx: usize,
    node_key: &BLSPubKey,
) {
    let test_view = &test_data.views[view_idx];
    harness.send(test_view.proposal_input(node_key)).await;

    for i in 0..THRESHOLD {
        harness.send(test_view.vote1_input(i)).await;
    }
}

/// Send enough Vote2 messages to form Certificate2.
async fn send_vote2s(harness: &TestHarness, test_data: &TestData, view_idx: usize) {
    let test_view = &test_data.views[view_idx];
    for i in 0..THRESHOLD {
        harness.send(test_view.vote2_input(i)).await;
    }
}

/// CPU tasks form Certificate1 from accumulated Vote1 messages, enabling
/// consensus to continue (verified by Vote1 emission for subsequent views).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cert1_formed_and_vote2_sent() {
    let harness = TestHarness::new_with_cpu_tasks(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // View 1: proposal + Vote1 messages → CPU forms cert1 + reconstructs block
    send_proposal_and_vote1s(&harness, &test_data, 0, &node_key).await;

    // Wait for CPU tasks to process votes
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let events = harness.shutdown().await;

    assert!(has_vote1(&events), "Vote1 should be sent for proposal");
    assert!(has_vote2(&events), "Vote2 should be sent for proposal");
    assert!(has_cert1(&events), "Certificate1 should be formed");
}

/// Full decide path: CPU tasks form Certificate1, Certificate2, and
/// reconstruct blocks from VID shares, leading to a leaf decision.
/// Block reconstruction is exercised because consensus requires
/// BlockReconstructed (produced by the CPU VidShareTask) before it
/// can proceed to the decide step.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_full_decide_via_cpu_tasks() {
    let harness = TestHarness::new_with_cpu_tasks(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // View 1: send proposal + Vote1s
    //   CPU VoteCollectionTask: accumulates QuorumVote2s → forms cert1
    //   CPU VidShareTask: accumulates VID shares → reconstructs block
    send_proposal_and_vote1s(&harness, &test_data, 0, &node_key).await;

    // Send Vote2s for view 1 → CPU forms cert2
    send_vote2s(&harness, &test_data, 0).await;

    // View 2: full round to trigger decision on view 1
    send_proposal_and_vote1s(&harness, &test_data, 1, &node_key).await;
    send_vote2s(&harness, &test_data, 1).await;

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let events = harness.shutdown().await;

    assert!(has_vote1(&events), "Vote1 should be sent");
    assert!(has_vote2(&events), "Vote2 should be sent");
    // LeafDecided proves the full pipeline: cert1 formation, block
    // reconstruction from VID shares, cert2 formation, and decision.
    assert!(
        has_leaf_decided(&events),
        "Leaf should be decided — requires CPU block reconstruction + cert formation"
    );
}

/// Leader sends a proposal after CPU tasks form Certificate1.
/// The proposal requires VID disperse, which is computed by the CPU
/// VidDisperseTask (the mock coordinator forwards RequestVidDisperse
/// to the CPU task when cpu_tx is set). SendProposal in the output
/// proves the full leader path: cert1 formation → block/header request
/// → VID disperse via CPU → proposal sent.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_leader_proposal_via_cpu_tasks() {
    let test_data = TestData::new(4).await;
    let leader_for_view_2 = test_data.views[1].leader_public_key;
    let leader_index = node_index_for_key(&leader_for_view_2);
    let harness = TestHarness::new_with_cpu_tasks(leader_index).await;

    // View 1: send proposal + Vote1s → CPU forms cert1 → leader proposes for view 2
    // The leader proposal path requires:
    //   1. cert1 formed by CPU VoteCollectionTask
    //   2. block + header built (handled inline by mock)
    //   3. VID disperse computed by CPU VidDisperseTask
    send_proposal_and_vote1s(&harness, &test_data, 0, &leader_for_view_2).await;

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let events = harness.shutdown().await;

    assert!(
        has_request_block_and_header(&events),
        "Leader should request block and header after CPU forms cert1"
    );
    assert!(
        has_request_vid_disperse(&events),
        "Leader should request VID disperse after CPU forms cert1"
    );
    assert!(
        has_vid_disperse(&events),
        "Leader should request VID disperse after CPU forms cert1"
    );
    assert!(
        has_cert1(&events),
        "Leader should form cert1 (requires CPU VID disperse)"
    );
    assert!(
        has_block_reconstructed(&events),
        "Leader should reconstruct block after CPU forms cert1"
    );

    // SendProposal proves the CPU VidDisperseTask computed the VID
    // disperse — consensus cannot send a proposal without it.
    assert!(
        has_proposal(&events),
        "Leader should send proposal (requires CPU VID disperse)"
    );
}

/// Multi-view chain: CPU tasks form certificates for each view, leading to
/// multiple decisions.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_multi_view_decide_via_cpu_tasks() {
    let harness = TestHarness::new_with_cpu_tasks(0).await;
    let test_data = TestData::new(5).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    for i in 0..test_data.views.len() {
        send_proposal_and_vote1s(&harness, &test_data, i, &node_key).await;
        send_vote2s(&harness, &test_data, i).await;
    }

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let events = harness.shutdown().await;

    let decide_count = events
        .iter()
        .filter(|e| matches!(e, ConsensusOutput::Event(Event::LeafDecided(_))))
        .count();
    assert!(
        decide_count >= 2,
        "Multiple views should produce decisions, got {decide_count}"
    );
}

/// Send enough timeout votes for a view.
async fn send_timeout_votes(harness: &TestHarness, test_data: &TestData, view_idx: usize) {
    let test_view = &test_data.views[view_idx];
    for i in 0..THRESHOLD {
        harness.send(test_view.timeout_vote_input(i)).await;
    }
}

/// Timeout votes are collected by the CPU VoteCollector and form a
/// TimeoutCertificate, which advances the view.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_timeout_votes_form_tc() {
    let harness = TestHarness::new_with_cpu_tasks(0).await;
    let test_data = TestData::new(4).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Process view 1 to establish locked_qc (needed for TC handling)
    send_proposal_and_vote1s(&harness, &test_data, 0, &node_key).await;
    // Send timeout votes for view 2 → CPU timeout collector forms TC
    send_timeout_votes(&harness, &test_data, 1).await;

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let events = harness.shutdown().await;

    assert!(
        has_timeout_cert(&events),
        "TimeoutCertificate should be formed from timeout votes"
    );
    assert!(
        has_view_changed(&events),
        "View should advance after timeout certificate"
    );
}

/// Full leader path after timeout via CPU tasks: establish lock → timer
/// fires for view 2 (Timeout) → timeout votes form TC → leader proposes
/// for view 3 before the view 3 timer fires.
///
/// The 200ms timer is long enough for VID disperse to complete (~50-100ms)
/// but short enough to actually fire during the test, proving the timeout
/// mechanism does not interfere with the leader's proposal path.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_leader_proposes_after_timeout_via_cpu_tasks() {
    let test_data = TestData::new(5).await;
    // Timeout cert for view 2 advances to view 3; we need to be leader of view 3
    let leader_for_view_3 = test_data.views[2].leader_public_key;
    let leader_index = node_index_for_key(&leader_for_view_3);
    // Timer must be long enough for VID to complete (so the view 3 timeout
    // doesn't kill the in-progress proposal), but short enough to actually
    // fire for view 2 during the test.
    let harness = TestHarness::new_with_cpu_tasks_and_timer(
        leader_index,
        std::time::Duration::from_millis(400),
    )
    .await;

    // View 1: process fully to establish locked_qc
    send_proposal_and_vote1s(&harness, &test_data, 0, &leader_for_view_3).await;

    // Wait for cert1 formation + block reconstruction → locked_qc set,
    // and for the view 2 timer to fire (Timeout(2) sets timeout_view=2).
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Send timeout votes for view 2 → CPU timeout collector forms TC
    // → consensus handles TC → leader of view 3 requests block/header → proposes.
    // The TC input view is 3 (cert.view+1), which passes the stale filter
    // (3 > timeout_view=2). After ViewChanged(3) resets the timer, the leader
    // must complete VID disperse before the 500ms timer fires for view 3.
    send_timeout_votes(&harness, &test_data, 1).await;

    // Wait for TC formation, block/header creation, VID disperse, and proposal
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let events = harness.shutdown().await;

    // Verify the timeout actually fired (not avoided)
    let has_timeout = events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Event(Event::Timeout(_))));
    assert!(has_timeout, "Timer should have fired (Timeout event)");

    // TODO eventually check a timeout vote was sent

    assert!(
        has_timeout_cert(&events),
        "TimeoutCertificate should be formed from timeout votes"
    );
    assert!(
        has_request_block_and_header(&events),
        "Leader should request block and header after TC"
    );
    assert!(
        has_proposal(&events),
        "Leader should send proposal with timeout view change evidence"
    );
}
