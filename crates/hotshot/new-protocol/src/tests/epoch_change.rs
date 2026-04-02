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
    assertions::{count_epoch_change, has_epoch_change, is_view_changed},
    utils::{ConsensusHarness, TEST_DRB_RESULT, TestData},
};
use crate::{
    consensus::{ConsensusInput, ConsensusOutput},
    helpers::proposal_commitment,
    message::{EpochChangeMessage, Proposal, ProposalMessage},
    outbox::Outbox,
    tests::common::assertions::{any, is_proposal, is_vote1},
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
        let block_number = BlockHeader::<TestTypes>::block_number(
            &test_data.views[i].proposal.data.proposal.block_header,
        );
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
        has_epoch_change(harness.outputs()),
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
        !has_epoch_change(harness.outputs()),
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
    let proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone().into();
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
    let proposal: Proposal<TestTypes> = test_data.views[9].proposal.data.clone().into();
    let epoch_change = EpochChangeMessage {
        cert1: test_data.views[8].cert1.clone(),
        cert2: test_data.views[9].cert2.clone(),
        proposal,
    };

    let view_changed_before = count_view_changed(harness.outputs());

    harness
        .apply(ConsensusInput::EpochChange(epoch_change))
        .await;

    assert_eq!(
        view_changed_before,
        count_view_changed(harness.outputs()),
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
    let proposal: Proposal<TestTypes> = mid_view.proposal.data.clone().into();
    let epoch_change = EpochChangeMessage {
        cert1: mid_view.cert1.clone(),
        cert2: mid_view.cert2.clone(),
        proposal,
    };

    let view_changed_before = count_view_changed(harness.outputs());

    harness
        .apply(ConsensusInput::EpochChange(epoch_change))
        .await;

    assert_eq!(
        view_changed_before,
        count_view_changed(harness.outputs()),
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
    let wrong_proposal: Proposal<TestTypes> = test_data.views[8].proposal.data.clone().into();
    let epoch_change = EpochChangeMessage {
        cert1: epoch_view.cert1.clone(),
        cert2: epoch_view.cert2.clone(),
        proposal: wrong_proposal,
    };

    let view_changed_before = count_view_changed(harness.outputs());

    harness
        .apply(ConsensusInput::EpochChange(epoch_change))
        .await;

    assert_eq!(
        view_changed_before,
        count_view_changed(harness.outputs()),
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
    let proposal: Proposal<TestTypes> = stale_view.proposal.data.clone().into();
    let stale_epoch_change = EpochChangeMessage {
        cert1: stale_view.cert1.clone(),
        cert2: stale_view.cert2.clone(),
        proposal,
    };

    let view_changed_before = count_view_changed(harness.outputs());

    harness
        .apply(ConsensusInput::EpochChange(stale_epoch_change))
        .await;

    assert_eq!(
        view_changed_before,
        count_view_changed(harness.outputs()),
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
        count_epoch_change(harness.outputs()),
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

    let proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone().into();
    let epoch_change = EpochChangeMessage {
        cert1: epoch_view.cert1.clone(),
        cert2: epoch_view.cert2.clone(),
        proposal,
    };

    let mut harness = ConsensusHarness::new(1).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 1).0;

    run_views_full(&mut harness, &test_data, &node_key, 0..9).await;

    let req_before = harness
        .outputs()
        .iter()
        .filter(|o| matches!(o, ConsensusOutput::RequestBlockAndHeader(_)))
        .count();

    harness
        .apply(ConsensusInput::EpochChange(epoch_change.clone()))
        .await;

    let req_after = harness
        .outputs()
        .iter()
        .filter(|o| matches!(o, ConsensusOutput::RequestBlockAndHeader(_)))
        .count();
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

    let epoch_proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone().into();
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
    let mut proposal: Proposal<TestTypes> = first_view.proposal.data.clone().into();
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

fn count_view_changed(outputs: &Outbox<ConsensusOutput<TestTypes>>) -> usize {
    outputs
        .iter()
        .filter(|o| matches!(o, ConsensusOutput::ViewChanged(..)))
        .count()
}
