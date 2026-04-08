use hotshot::types::BLSPubKey;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    data::EpochNumber,
    message::Proposal as SignedProposal,
    traits::{block_contents::BlockHeader, signature_key::SignatureKey},
    utils::is_epoch_transition,
    vote::HasViewNumber,
};

use super::common::{
    assertions::{is_send_epoch_change, is_view_changed},
    utils::{ConsensusHarness, TEST_DRB_RESULT, TestData},
};
use crate::{
    consensus::{ConsensusInput, ConsensusOutput},
    helpers::proposal_commitment,
    message::{EpochChangeMessage, Proposal, ProposalMessage},
    tests::common::assertions::{
        any, count_matching, has_request_drb_for_epoch, is_proposal, is_request_block_and_header,
        is_vote1,
    },
};

const EPOCH_HEIGHT: u64 = 10;

/// Run views through the harness fully (proposal, block_reconstructed, cert1, cert2).
/// Pre-feeds DRB results before epoch transition views so handle_proposal accepts them.
async fn run_views_full(
    harness: &mut ConsensusHarness,
    test_data: &TestData,
    node_key: &BLSPubKey,
    range: std::ops::Range<usize>,
) {
    // Collect which epochs need DRB results and pre-feed them all up front.
    let mut drb_epochs = std::collections::BTreeSet::new();
    for i in range.clone() {
        let block_number =
            BlockHeader::<TestTypes>::block_number(&test_data.views[i].proposal.data.block_header);
        if is_epoch_transition(block_number, EPOCH_HEIGHT) {
            drb_epochs.insert(test_data.views[i].epoch_number + 1);
        }
    }
    for epoch in drb_epochs {
        harness
            .apply(ConsensusInput::DrbResult(epoch, TEST_DRB_RESULT))
            .await;
    }

    for i in range {
        harness
            .apply(test_data.views[i].proposal_input_consensus(node_key))
            .await;
        harness
            .apply(test_data.views[i].block_reconstructed_input())
            .await;
        harness.apply(test_data.views[i].cert1_input()).await;
        harness.apply(test_data.views[i].cert2_input()).await;
    }
}

/// When the last block of an epoch is decided (block_number % epoch_height == 0),
/// consensus should emit SendEpochChange. With epoch_height=10, block 10 is the
/// last block of epoch 1 and corresponds to views[9] (view 10).
#[tokio::test]
async fn test_epoch_change_sent_on_last_block() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    run_views_full(&mut harness, &test_data, &node_key, 0..10).await;

    assert!(
        any(harness.outputs(), is_send_epoch_change),
        "SendEpochChange should be emitted when the last block of an epoch is decided"
    );
}

/// Epoch change should not be emitted for blocks before the epoch boundary.
#[tokio::test]
async fn test_epoch_change_not_sent_mid_epoch() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new_with_epoch_height(10, EPOCH_HEIGHT).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Run through views 1-9 (blocks 1-9), no epoch boundary hit
    run_views_full(&mut harness, &test_data, &node_key, 0..9).await;

    assert!(
        !any(harness.outputs(), is_send_epoch_change),
        "SendEpochChange should NOT be emitted for mid-epoch blocks"
    );
}

/// A valid EpochChangeMessage should be accepted and produce a ViewChanged output.
#[tokio::test]
async fn test_handle_epoch_change_valid() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    run_views_full(&mut harness, &test_data, &node_key, 0..9).await;

    // Construct a valid EpochChangeMessage for view 10 (block 10, last block of epoch 1)
    let epoch_view = &test_data.views[9];
    let proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone();
    let epoch_change = EpochChangeMessage {
        cert1: epoch_view.cert1.clone(),
        cert2: epoch_view.cert2.clone(),
        proposal,
    };

    harness
        .apply(ConsensusInput::EpochChange(epoch_change))
        .await;

    assert!(
        any(harness.outputs(), is_view_changed),
        "Valid EpochChange should produce ViewChanged output"
    );
}

/// An EpochChangeMessage with mismatched cert1 and cert2 view numbers should be rejected.
#[tokio::test]
async fn test_handle_epoch_change_mismatched_views() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    run_views_full(&mut harness, &test_data, &node_key, 0..9).await;

    // Mix cert1 from view 9 with cert2 from view 10 — mismatched views
    let proposal: Proposal<TestTypes> = test_data.views[9].proposal.data.clone();
    let epoch_change = EpochChangeMessage {
        cert1: test_data.views[8].cert1.clone(),
        cert2: test_data.views[9].cert2.clone(),
        proposal,
    };

    let view_changed_before = count_matching(harness.outputs(), is_view_changed);

    harness
        .apply(ConsensusInput::EpochChange(epoch_change))
        .await;

    assert_eq!(
        view_changed_before,
        count_matching(harness.outputs(), is_view_changed),
        "Mismatched EpochChange should be rejected — no new ViewChanged"
    );
}

/// An EpochChangeMessage where the block is not the last block of the epoch
/// should be rejected (block_number % epoch_height != 0).
#[tokio::test]
async fn test_handle_epoch_change_wrong_block_number() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(11).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    run_views_full(&mut harness, &test_data, &node_key, 0..6).await;

    // Use view 6 (block 6) which is NOT the last block of the epoch
    let mid_view = &test_data.views[5];
    let proposal: Proposal<TestTypes> = mid_view.proposal.data.clone();
    let epoch_change = EpochChangeMessage {
        cert1: mid_view.cert1.clone(),
        cert2: mid_view.cert2.clone(),
        proposal,
    };

    let view_changed_before = count_matching(harness.outputs(), is_view_changed);

    harness
        .apply(ConsensusInput::EpochChange(epoch_change))
        .await;

    assert_eq!(
        view_changed_before,
        count_matching(harness.outputs(), is_view_changed),
        "EpochChange with wrong block number should be rejected"
    );
}

/// An EpochChangeMessage whose proposal commitment doesn't match cert1's
/// leaf_commit should be rejected.
#[tokio::test]
async fn test_handle_epoch_change_proposal_mismatch() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    run_views_full(&mut harness, &test_data, &node_key, 0..9).await;

    // Use cert1/cert2 from view 10 but proposal from view 9 — commitment mismatch
    let epoch_view = &test_data.views[9];
    let wrong_proposal: Proposal<TestTypes> = test_data.views[8].proposal.data.clone();
    let epoch_change = EpochChangeMessage {
        cert1: epoch_view.cert1.clone(),
        cert2: epoch_view.cert2.clone(),
        proposal: wrong_proposal,
    };

    let view_changed_before = count_matching(harness.outputs(), is_view_changed);

    harness
        .apply(ConsensusInput::EpochChange(epoch_change))
        .await;

    assert_eq!(
        view_changed_before,
        count_matching(harness.outputs(), is_view_changed),
        "EpochChange with mismatched proposal should be rejected"
    );
}

/// A stale EpochChangeMessage (cert1 view < locked_cert view) should be rejected.
#[tokio::test]
async fn test_handle_epoch_change_stale() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Run through all 10 views to advance the lock well past the epoch boundary
    run_views_full(&mut harness, &test_data, &node_key, 0..10).await;

    // Use views[5] (view 6, block 6) as a stale epoch change — locked_cert
    // is at view 10 after processing all views, so view 6 is behind the lock.
    let stale_view = &test_data.views[5];
    let proposal: Proposal<TestTypes> = stale_view.proposal.data.clone();
    let stale_epoch_change = EpochChangeMessage {
        cert1: stale_view.cert1.clone(),
        cert2: stale_view.cert2.clone(),
        proposal,
    };

    let view_changed_before = count_matching(harness.outputs(), is_view_changed);

    harness
        .apply(ConsensusInput::EpochChange(stale_epoch_change))
        .await;

    assert_eq!(
        view_changed_before,
        count_matching(harness.outputs(), is_view_changed),
        "Stale EpochChange should be rejected — no new ViewChanged"
    );
}

/// Verify that exactly one SendEpochChange is emitted when processing
/// views through a single epoch boundary.
#[tokio::test]
async fn test_epoch_change_emitted_exactly_once() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    run_views_full(&mut harness, &test_data, &node_key, 0..10).await;

    assert_eq!(
        count_matching(harness.outputs(), is_send_epoch_change),
        1,
        "Should emit exactly 1 SendEpochChange for 1 epoch boundary (block 10)"
    );
}

/// The EpochChangeMessage emitted by maybe_decide contains the correct cert1,
/// cert2, and proposal for the epoch boundary view.
#[tokio::test]
async fn test_epoch_change_message_contents() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    run_views_full(&mut harness, &test_data, &node_key, 0..10).await;

    let epoch_change_msg = harness
        .outputs()
        .iter()
        .find_map(|o| {
            if let ConsensusOutput::SendEpochChange(msg) = o {
                Some(msg.clone())
            } else {
                None
            }
        })
        .expect("SendEpochChange should have been emitted");

    // The epoch change should be for view 10 (the last block of epoch 1)
    let expected_view = test_data.views[9].view_number;
    assert_eq!(
        epoch_change_msg.cert1.view_number(),
        expected_view,
        "cert1 should be for the epoch boundary view"
    );
    assert_eq!(
        epoch_change_msg.cert2.view_number(),
        expected_view,
        "cert2 should be for the epoch boundary view"
    );

    // cert1 and cert2 should agree on the leaf commitment
    assert_eq!(
        epoch_change_msg.cert1.data.leaf_commit, epoch_change_msg.cert2.data.leaf_commit,
        "cert1 and cert2 should have the same leaf commitment"
    );

    // The proposal commitment should match the certificates
    let proposal_commit = proposal_commitment(&epoch_change_msg.proposal);
    assert_eq!(
        epoch_change_msg.cert1.data.leaf_commit, proposal_commit,
        "proposal commitment should match the certificates"
    );
}

/// When a valid EpochChangeMessage is received and the node is the leader
/// for the first view of the next epoch, it should request a block and header.
#[tokio::test]
async fn test_epoch_change_leader_proposes() {
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;
    let epoch_view = &test_data.views[9];

    let proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone();
    let epoch_change = EpochChangeMessage {
        cert1: epoch_view.cert1.clone(),
        cert2: epoch_view.cert2.clone(),
        proposal,
    };

    let mut harness = ConsensusHarness::new(1).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 1).0;

    run_views_full(&mut harness, &test_data, &node_key, 0..9).await;

    let req_before = count_matching(harness.outputs(), is_request_block_and_header);

    harness
        .apply(ConsensusInput::EpochChange(epoch_change.clone()))
        .await;

    let req_after = count_matching(harness.outputs(), is_request_block_and_header);

    assert!(
        req_after > req_before,
        "node should be the leader for the next epoch and request a block"
    );
    assert!(
        any(harness.outputs(), is_proposal),
        "node should send a proposal after requesting a block and header"
    );
}

/// Test that a node with no other information can vote on the first proposal of the next epoch, with just
/// the epoch change message and the proposal for the first view of the next epoch.
#[tokio::test]
async fn test_epoch_change_votes() {
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;
    let epoch_view = &test_data.views[9]; // view 10, last block of epoch 1

    let epoch_proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone();
    let epoch_change = EpochChangeMessage {
        cert1: epoch_view.cert1.clone(),
        cert2: epoch_view.cert2.clone(),
        proposal: epoch_proposal,
    };

    // Use node 0 (non-leader for the first view of epoch 2)
    let mut harness = ConsensusHarness::new(0).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Transition to epoch 2
    harness
        .apply(ConsensusInput::EpochChange(epoch_change))
        .await;

    // Build the proposal for the first view of epoch 2 with the
    // required next_epoch_justify_qc.
    let first_view = &test_data.views[10];
    let mut proposal: Proposal<TestTypes> = first_view.proposal.data.clone();
    proposal.epoch = EpochNumber::new(2);
    proposal.next_epoch_justify_qc = Some(epoch_view.cert2.clone());

    let signed_proposal = SignedProposal {
        data: proposal,
        signature: first_view.proposal.signature.clone(),
        _pd: std::marker::PhantomData,
    };

    let vid_share = first_view
        .vid_shares
        .iter()
        .find(|s| s.recipient_key == node_key)
        .expect("VID share not found for node")
        .clone();

    harness
        .apply(ConsensusInput::Proposal(ProposalMessage::validated(
            signed_proposal,
            vid_share,
        )))
        .await;

    assert!(
        any(harness.outputs(), is_vote1),
        "Node should send a vote after receiving epoch change and proposal for the new epoch"
    );
}

/// In the first two epochs, the leader should be able to propose during the
/// epoch transition window without a DRB result.  The DRB is only needed
/// starting from epoch 3 onward.  This test covers epoch 1.
#[tokio::test]
async fn test_first_epoch_leader_proposes_without_drb() {
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;
    // View 8 (block 8) is in the transition window: is_epoch_transition(8, 10) = true.
    // Node 8 is the leader for view 8 (8 % 10 = 8).
    let mut harness = ConsensusHarness::new(8).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 8).0;

    // Run views 1-7 WITHOUT pre-feeding any DRB results.
    // handle_proposal skips the DRB check for epoch 1 (genesis epoch),
    // so voting works. After processing view 7, the leader for view 8
    // should propose — but maybe_propose also needs to skip the DRB check.
    for i in 0..7 {
        harness
            .apply(test_data.views[i].proposal_input_consensus(&node_key))
            .await;
        harness
            .apply(test_data.views[i].block_reconstructed_input())
            .await;
        harness.apply(test_data.views[i].cert1_input()).await;
        harness.apply(test_data.views[i].cert2_input()).await;
    }

    assert!(
        any(harness.outputs(), is_proposal),
        "Leader should propose during the first epoch transition window without a DRB result"
    );
}

/// Epoch 2 also does not require a DRB result for proposing.  This test
/// runs through epoch 1, crosses the epoch boundary, advances into
/// epoch 2's transition window, and verifies the leader proposes without
/// needing a DRB result pre-fed.
#[tokio::test]
async fn test_second_epoch_leader_proposes_without_drb() {
    // We need at least 18 views: 10 for epoch 1, 1 epoch boundary, 7 into epoch 2.
    let test_data = TestData::new_with_epoch_height(19, EPOCH_HEIGHT).await;

    // View 18 (block 18) is in epoch 2's transition window:
    //   epoch_from_block_number(18, 10) = 2, is_epoch_transition(18, 10) = true.
    // Leader for view 18 = 18 % 10 = 8 → node 8.
    let mut harness = ConsensusHarness::new(8).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 8).0;

    // ---- Epoch 1 (views 1-10, blocks 1-10) ----
    // No DRB needed: handle_proposal and maybe_propose skip the check
    // for the genesis epoch.
    run_views_full(&mut harness, &test_data, &node_key, 0..9).await;

    // ---- Epoch boundary ----
    let epoch_view = &test_data.views[9];
    let epoch_proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone();
    let epoch_change = EpochChangeMessage {
        cert1: epoch_view.cert1.clone(),
        cert2: epoch_view.cert2.clone(),
        proposal: epoch_proposal,
    };
    harness
        .apply(ConsensusInput::EpochChange(epoch_change))
        .await;
    // Parent chain verification needs block_reconstructed for view 10.
    harness.apply(epoch_view.block_reconstructed_input()).await;

    // ---- First block of epoch 2 (view 11, block 11) ----
    // This proposal needs next_epoch_justify_qc set because it's the
    // first proposal after the epoch boundary.
    let first_e2_view = &test_data.views[10];
    let mut first_e2_proposal: Proposal<TestTypes> = first_e2_view.proposal.data.clone();
    first_e2_proposal.epoch = EpochNumber::new(2);
    first_e2_proposal.next_epoch_justify_qc = Some(epoch_view.cert2.clone());

    let signed = SignedProposal {
        data: first_e2_proposal,
        signature: first_e2_view.proposal.signature.clone(),
        _pd: std::marker::PhantomData,
    };
    let vid_share = first_e2_view
        .vid_shares
        .iter()
        .find(|s| s.recipient_key == node_key)
        .expect("VID share not found")
        .clone();
    harness
        .apply(ConsensusInput::Proposal(ProposalMessage::validated(
            signed, vid_share,
        )))
        .await;
    harness
        .apply(first_e2_view.block_reconstructed_input())
        .await;
    harness.apply(first_e2_view.cert1_input()).await;
    harness.apply(first_e2_view.cert2_input()).await;

    // ---- Epoch 2 views 12-16 (blocks 12-16, before transition window) ----
    for i in 11..16 {
        harness
            .apply(test_data.views[i].proposal_input_consensus(&node_key))
            .await;
        harness
            .apply(test_data.views[i].block_reconstructed_input())
            .await;
        harness.apply(test_data.views[i].cert1_input()).await;
        harness.apply(test_data.views[i].cert2_input()).await;
    }

    // ---- Epoch 2 transition window (block 17+) ----
    // No DRB pre-fed — epoch 2 should not require one.
    // Process view 17 (block 17, first block in transition window).
    // After cert1 for view 17, the leader for view 18 should propose.
    harness
        .apply(test_data.views[16].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[16].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[16].cert1_input()).await;
    harness.apply(test_data.views[16].cert2_input()).await;

    assert!(
        any(harness.outputs(), is_proposal),
        "Leader should propose during epoch 2 transition window without a DRB result"
    );
}

/// Starting from epoch 3, when a node receives a transition-window proposal
/// without a DRB result available, consensus should emit RequestDrbResult
/// for the next epoch.  Epochs 1 and 2 skip this check; epoch 3 is the
/// first epoch that requires a real DRB.
#[tokio::test]
async fn test_epoch3_transition_requests_drb_for_future_epoch() {
    let test_data = TestData::new_with_epoch_height(18, EPOCH_HEIGHT).await;

    let mut harness = ConsensusHarness::new(0).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // ---- Epoch 1 (views 1-9, blocks 1-9) ----
    run_views_full(&mut harness, &test_data, &node_key, 0..9).await;

    // ---- Epoch boundary ----
    let epoch_view = &test_data.views[9];
    let epoch_proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone();
    let epoch_change = EpochChangeMessage {
        cert1: epoch_view.cert1.clone(),
        cert2: epoch_view.cert2.clone(),
        proposal: epoch_proposal,
    };
    harness
        .apply(ConsensusInput::EpochChange(epoch_change))
        .await;
    harness.apply(epoch_view.block_reconstructed_input()).await;

    // ---- First block of epoch 2 (view 11) ----
    let first_e2_view = &test_data.views[10];
    let mut first_e2_proposal: Proposal<TestTypes> = first_e2_view.proposal.data.clone();
    first_e2_proposal.epoch = EpochNumber::new(2);
    first_e2_proposal.next_epoch_justify_qc = Some(epoch_view.cert2.clone());

    let signed = SignedProposal {
        data: first_e2_proposal,
        signature: first_e2_view.proposal.signature.clone(),
        _pd: std::marker::PhantomData,
    };
    let vid_share = first_e2_view
        .vid_shares
        .iter()
        .find(|s| s.recipient_key == node_key)
        .expect("VID share not found")
        .clone();
    harness
        .apply(ConsensusInput::Proposal(ProposalMessage::validated(
            signed, vid_share,
        )))
        .await;
    harness
        .apply(first_e2_view.block_reconstructed_input())
        .await;
    harness.apply(first_e2_view.cert1_input()).await;
    harness.apply(first_e2_view.cert2_input()).await;

    // ---- Epoch 2 views 12-16 (before transition window) ----
    for i in 11..16 {
        harness
            .apply(test_data.views[i].proposal_input_consensus(&node_key))
            .await;
        harness
            .apply(test_data.views[i].block_reconstructed_input())
            .await;
        harness.apply(test_data.views[i].cert1_input()).await;
        harness.apply(test_data.views[i].cert2_input()).await;
    }

    // Epoch 3's stake table and DRB result should already be registered
    // in the membership: when block 5 was decided (epoch root), the
    // harness called add_epoch_root and add_drb_result for the target
    // epoch (epoch 1 + 2 = 3).

    let v17 = &test_data.views[16];
    let mut v17_proposal: Proposal<TestTypes> = v17.proposal.data.clone();
    v17_proposal.epoch = EpochNumber::new(3);

    let signed = SignedProposal {
        data: v17_proposal,
        signature: v17.proposal.signature.clone(),
        _pd: std::marker::PhantomData,
    };
    let vid_share = v17
        .vid_shares
        .iter()
        .find(|s| s.recipient_key == node_key)
        .expect("VID share not found")
        .clone();
    harness
        .apply(ConsensusInput::Proposal(ProposalMessage::validated(
            signed, vid_share,
        )))
        .await;

    assert!(
        has_request_drb_for_epoch(harness.outputs(), EpochNumber::new(4)),
        "Consensus should request DRB for epoch 4 (current epoch 3 + 1) when processing a \
         transition-window proposal without a DRB result"
    );
}
