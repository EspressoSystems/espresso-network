use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    message::Proposal as SignedProposal,
    signature_key::BLSPubKey,
    traits::signature_key::SignatureKey,
    vote::HasViewNumber,
};

use super::common::{
    assertions::{is_send_epoch_change, is_view_changed},
    utils::{ConsensusHarness, TestData, epoch_length},
};
use crate::{
    consensus::{ConsensusInput, ConsensusOutput},
    helpers::proposal_commitment,
    message::{EpochChangeError, EpochChangeMessage, Proposal, ProposalMessage},
    proposal::MalformedProposal,
    tests::common::assertions::{
        any, count_matching, has_request_drb_for_epoch, is_leaf_decided, is_proposal,
        is_request_block_and_header, is_vote1, is_vote1_for_view,
    },
};

const EPOCH_HEIGHT: u64 = 10;

/// Run views through the harness fully (proposal, block_reconstructed, cert1, cert2).
async fn run_views_full(
    harness: &mut ConsensusHarness,
    test_data: &TestData,
    node_key: &BLSPubKey,
    range: std::ops::Range<usize>,
) {
    for i in range {
        harness
            .apply_pair(test_data.views[i].proposal_input_consensus(node_key))
            .await;
        harness
            .apply(test_data.views[i].block_reconstructed_input())
            .await;
        harness.apply(test_data.views[i].cert1_input()).await;
        harness.apply(test_data.views[i].cert2_input()).await;
    }
}

/// Cross from epoch 1 into epoch 2: the epoch change on view 10's block, then
/// view 11, the first block of epoch 2.
async fn cross_into_epoch_2(
    harness: &mut ConsensusHarness,
    test_data: &TestData,
    node_key: &BLSPubKey,
) {
    let epoch_view = &test_data.views[9];
    let epoch_proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone();
    let epoch_change = EpochChangeMessage::validated(
        epoch_view.cert1.clone(),
        epoch_view.cert2.clone(),
        epoch_proposal,
    );
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
        .find(|s| s.recipient_key == *node_key)
        .expect("VID share not found")
        .clone();
    harness
        .apply_pair((
            ConsensusInput::Proposal(
                first_e2_view.leader_public_key,
                ProposalMessage::validated(signed),
            ),
            ConsensusInput::VidShare(vid_share),
        ))
        .await;
    harness
        .apply(first_e2_view.block_reconstructed_input())
        .await;
    harness.apply(first_e2_view.cert1_input()).await;
    harness.apply(first_e2_view.cert2_input()).await;
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

/// The epoch change a node sent when it decided an epoch's last block is the one
/// it answers a timeout vote of that epoch with: a node still voting in the epoch
/// has not learnt it ended, and the change sent once may have been lost.
#[tokio::test]
async fn test_epoch_change_ending_matches_the_one_sent() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    run_views_full(&mut harness, &test_data, &node_key, 0..10).await;

    let sent = harness
        .outputs()
        .iter()
        .find_map(|o| match o {
            ConsensusOutput::SendEpochChange(change) => Some(change.clone()),
            _ => None,
        })
        .expect("setup: the epoch change is sent when epoch 1's last block is decided");
    let epoch = test_data.views[9].epoch_number;
    let answer = harness
        .consensus
        .epoch_change_ending(epoch)
        .expect("a node that decided the epoch's last block holds its epoch change");
    assert_eq!(answer.cert1, sent.cert1);
    assert_eq!(answer.cert2, sent.cert2);
    assert_eq!(answer.proposal, sent.proposal);
    assert!(
        harness.consensus.epoch_change_ending(epoch + 1).is_none(),
        "no epoch change is held for an epoch that has not ended"
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
    let epoch_change =
        EpochChangeMessage::validated(epoch_view.cert1.clone(), epoch_view.cert2.clone(), proposal);

    harness
        .apply(ConsensusInput::EpochChange(epoch_change))
        .await;

    assert!(
        any(harness.outputs(), is_view_changed),
        "Valid EpochChange should produce ViewChanged output"
    );
    assert!(
        any(harness.outputs(), |output| matches!(
            output,
            ConsensusOutput::ProposalCertified(certified)
                if certified.view_number == epoch_view.view_number
        )),
        "the boundary proposal is announced so the reconstructor can pin its view"
    );
}

/// An EpochChangeMessage with mismatched cert1 and cert2 view numbers is not
/// well-formed.
#[tokio::test]
async fn test_epoch_change_mismatched_views_not_well_formed() {
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;

    // Mix cert1 from view 9 with cert2 from view 10 — mismatched views
    let proposal: Proposal<TestTypes> = test_data.views[9].proposal.data.clone();
    let epoch_change = EpochChangeMessage::validated(
        test_data.views[8].cert1.clone(),
        test_data.views[9].cert2.clone(),
        proposal,
    );

    assert!(matches!(
        epoch_change.well_formed(epoch_length(EPOCH_HEIGHT)),
        Err(EpochChangeError::CertificateMismatch)
    ));
}

/// Certificates from different epochs are not well-formed. Each certificate
/// can be individually crypto-valid (adjacent epochs often share a stake
/// table), so this must be caught structurally before verification.
#[tokio::test]
async fn test_epoch_change_cross_epoch_certificates_not_well_formed() {
    let test_data = TestData::new_with_epoch_height(21, EPOCH_HEIGHT).await;

    // cert1 from the epoch-1 boundary (view 10), cert2 from the epoch-2
    // boundary (view 20); both are genuine boundary certificates.
    let proposal: Proposal<TestTypes> = test_data.views[9].proposal.data.clone();
    let epoch_change = EpochChangeMessage::validated(
        test_data.views[9].cert1.clone(),
        test_data.views[19].cert2.clone(),
        proposal,
    );

    assert!(matches!(
        epoch_change.well_formed(epoch_length(EPOCH_HEIGHT)),
        Err(EpochChangeError::CertificateMismatch)
    ));
}

/// An EpochChangeMessage whose block is not the last block of the epoch
/// (block_number % epoch_height != 0) is not well-formed.
#[tokio::test]
async fn test_epoch_change_wrong_block_number_not_well_formed() {
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;

    // Use view 6 (block 6) which is NOT the last block of the epoch
    let mid_view = &test_data.views[5];
    let proposal: Proposal<TestTypes> = mid_view.proposal.data.clone();
    let epoch_change =
        EpochChangeMessage::validated(mid_view.cert1.clone(), mid_view.cert2.clone(), proposal);

    assert!(matches!(
        epoch_change.well_formed(epoch_length(EPOCH_HEIGHT)),
        Err(EpochChangeError::NotLastBlock)
    ));
}

/// An EpochChangeMessage whose proposal commitment doesn't match cert1's
/// leaf_commit is not well-formed.
#[tokio::test]
async fn test_epoch_change_proposal_mismatch_not_well_formed() {
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;

    // Use cert1/cert2 from view 10 but proposal from view 9 — commitment mismatch
    let epoch_view = &test_data.views[9];
    let wrong_proposal: Proposal<TestTypes> = test_data.views[8].proposal.data.clone();
    let epoch_change = EpochChangeMessage::validated(
        epoch_view.cert1.clone(),
        epoch_view.cert2.clone(),
        wrong_proposal,
    );

    assert!(matches!(
        epoch_change.well_formed(epoch_length(EPOCH_HEIGHT)),
        Err(EpochChangeError::ProposalMismatch)
    ));
}

/// A leaf commitment does not cover `Proposal::epoch`, so pinning the embedded
/// proposal to cert1's leaf commitment leaves its epoch free. An otherwise
/// genuine message whose proposal claims an epoch its block number does not
/// fall in is not well-formed.
#[tokio::test]
async fn test_epoch_change_proposal_epoch_mismatch_not_well_formed() {
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;

    // View 10 is the epoch-1 boundary, so this is a genuine epoch change.
    let epoch_view = &test_data.views[9];
    let mut proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone();
    let genuine = EpochChangeMessage::validated(
        epoch_view.cert1.clone(),
        epoch_view.cert2.clone(),
        proposal.clone(),
    );
    assert!(genuine.well_formed(epoch_length(EPOCH_HEIGHT)).is_ok());

    proposal.epoch += 1;
    assert_eq!(
        proposal_commitment(&proposal),
        epoch_view.cert1.data.leaf_commit,
        "bumping the epoch must leave the commitment intact, or this proves nothing",
    );

    let epoch_change =
        EpochChangeMessage::validated(epoch_view.cert1.clone(), epoch_view.cert2.clone(), proposal);

    assert!(matches!(
        epoch_change.well_formed(epoch_length(EPOCH_HEIGHT)),
        Err(EpochChangeError::Proposal(MalformedProposal::Epoch { .. }))
    ));
}

/// The embedded proposal is always the last block of an epoch, whose parent
/// (the block before it) is never an epoch root, so nothing may attach a
/// state_cert to it, not even a genuine one lifted from elsewhere in the
/// chain: the field is not covered by any signature this message carries.
#[tokio::test]
async fn test_epoch_change_state_cert_unexpected_not_well_formed() {
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;

    let epoch_view = &test_data.views[9];
    let genuine_state_cert = test_data
        .views
        .iter()
        .find_map(|v| v.proposal.data.state_cert.clone())
        .expect("fixture precondition: chain carries a genuine state_cert");

    let mut proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone();
    assert!(
        proposal.state_cert.is_none(),
        "fixture precondition: the epoch-change proposal's own parent is not an epoch root"
    );
    proposal.state_cert = Some(genuine_state_cert);
    assert_eq!(
        proposal_commitment(&proposal),
        epoch_view.cert1.data.leaf_commit,
        "attaching state_cert must leave the commitment intact, or this proves nothing",
    );

    let epoch_change =
        EpochChangeMessage::validated(epoch_view.cert1.clone(), epoch_view.cert2.clone(), proposal);

    assert!(matches!(
        epoch_change.well_formed(epoch_length(EPOCH_HEIGHT)),
        Err(EpochChangeError::Proposal(
            MalformedProposal::StateCertUnexpected(_)
        ))
    ));
}

/// A certificate names its own view, which the leaf commitment does not cover.
/// Certificates that agree with each other but sit at a view other than the
/// proposal's are not well-formed, even though they certify the proposal's leaf.
#[tokio::test]
async fn test_epoch_change_certificates_at_another_view_not_well_formed() {
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;

    let epoch_view = &test_data.views[9];
    let proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone();
    let mut cert1 = epoch_view.cert1.clone();
    let mut cert2 = epoch_view.cert2.clone();
    let elsewhere = epoch_view.cert1.view_number() + 1;
    cert1.view_number = elsewhere;
    cert2.view_number = elsewhere;

    let epoch_change = EpochChangeMessage::validated(cert1, cert2, proposal);

    assert!(matches!(
        epoch_change.well_formed(epoch_length(EPOCH_HEIGHT)),
        Err(EpochChangeError::ProposalCertificateMismatch)
    ));
}

/// The block number a certificate names is not covered by the leaf commitment
/// either, and it is the one the boundary and epoch checks read, while the
/// proposal's epoch is checked against the proposal's own height. Certificates
/// for another epoch's boundary block pass both of those checks on their own.
#[tokio::test]
async fn test_epoch_change_certificates_for_another_block_not_well_formed() {
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;

    let epoch_view = &test_data.views[9];
    let proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone();
    let mut cert1 = epoch_view.cert1.clone();
    let mut cert2 = epoch_view.cert2.clone();
    // The last block of epoch 2 rather than of epoch 1, so the certificates
    // remain internally consistent.
    let other_boundary = 2 * EPOCH_HEIGHT;
    cert1.data.block_number = Some(other_boundary);
    cert1.data.epoch = Some(EpochNumber::new(2));
    cert2.data.block_number = other_boundary;
    cert2.data.epoch = EpochNumber::new(2);

    let epoch_change = EpochChangeMessage::validated(cert1, cert2, proposal);

    assert!(matches!(
        epoch_change.well_formed(epoch_length(EPOCH_HEIGHT)),
        Err(EpochChangeError::ProposalCertificateMismatch)
    ));
}

/// Both certificates certify one leaf, so they name one block. cert2's epoch is
/// not optional, so the epoch comparison above it has already established that
/// cert1 names an epoch, and a certificate that names one names a block too.
#[tokio::test]
async fn test_epoch_change_certificates_for_different_blocks_not_well_formed() {
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;

    let epoch_view = &test_data.views[9];
    let proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone();
    let mut cert1 = epoch_view.cert1.clone();
    cert1.data.block_number = Some(epoch_view.cert2.data.block_number + 1);

    let epoch_change =
        EpochChangeMessage::validated(cert1, epoch_view.cert2.clone(), proposal.clone());

    assert!(matches!(
        epoch_change.well_formed(epoch_length(EPOCH_HEIGHT)),
        Err(EpochChangeError::CertificateMismatch)
    ));

    let mut without_block_number = epoch_view.cert1.clone();
    without_block_number.data.block_number = None;
    let epoch_change =
        EpochChangeMessage::validated(without_block_number, epoch_view.cert2.clone(), proposal);
    assert!(matches!(
        epoch_change.well_formed(epoch_length(EPOCH_HEIGHT)),
        Err(EpochChangeError::CertificateMismatch)
    ));
}

/// The embedded proposal's justify QC must certify the block below it, as it
/// must in a proposal that arrives on its own. Reaching this needs certificates
/// over the tampered leaf, which is what a quorum that certified it would leave
/// behind.
#[tokio::test]
async fn test_epoch_change_proposal_justify_qc_mismatch_not_well_formed() {
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;

    let epoch_view = &test_data.views[9];
    let mut proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone();
    // The parent is block 9, which falls in epoch 1.
    proposal.justify_qc.data.epoch = Some(EpochNumber::new(2));
    let leaf_commit = proposal_commitment(&proposal);
    let mut cert1 = epoch_view.cert1.clone();
    let mut cert2 = epoch_view.cert2.clone();
    cert1.data.leaf_commit = leaf_commit;
    cert2.data.leaf_commit = leaf_commit;

    let epoch_change = EpochChangeMessage::validated(cert1, cert2, proposal);

    assert!(matches!(
        epoch_change.well_formed(epoch_length(EPOCH_HEIGHT)),
        Err(EpochChangeError::Proposal(
            MalformedProposal::JustifyQcEpoch { .. }
        ))
    ));
}

/// The embedded proposal must follow the view its justify QC certifies, as it
/// must when it arrives on its own. The walks over stored proposals descend on
/// that ordering, and `handle_epoch_change` stores this proposal.
#[tokio::test]
async fn test_epoch_change_proposal_view_order_not_well_formed() {
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;

    let epoch_view = &test_data.views[9];
    let mut proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone();
    // Still the parent's epoch and block number, so only the ordering is wrong.
    proposal.justify_qc.view_number = proposal.view_number;
    let leaf_commit = proposal_commitment(&proposal);
    let mut cert1 = epoch_view.cert1.clone();
    let mut cert2 = epoch_view.cert2.clone();
    cert1.data.leaf_commit = leaf_commit;
    cert2.data.leaf_commit = leaf_commit;

    let epoch_change = EpochChangeMessage::validated(cert1, cert2, proposal);

    assert!(matches!(
        epoch_change.well_formed(epoch_length(EPOCH_HEIGHT)),
        Err(EpochChangeError::Proposal(
            MalformedProposal::ParentNotEarlier { .. }
        ))
    ));
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
    let stale_epoch_change =
        EpochChangeMessage::validated(stale_view.cert1.clone(), stale_view.cert2.clone(), proposal);

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

/// An otherwise-valid EpochChangeMessage for an epoch boundary the node has
/// already crossed is rejected, even when the lock does not catch it (here:
/// no locked cert at all). Regression test: replayed boundary messages must
/// not emit a backwards `ViewChanged` and regress a caught-up node to a
/// long-past view.
#[tokio::test]
async fn test_handle_epoch_change_replay_of_crossed_boundary() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;

    // Simulate a node already deep in epoch 3 — e.g. freshly restarted and
    // seeded from its anchor — with no locked cert (genesis safety), so the
    // locked-cert staleness check cannot reject the replay.
    harness.set_view(ViewNumber::new(25), EpochNumber::new(3));

    // Replay the epoch 1 → 2 boundary (view 10, block 10).
    let epoch_view = &test_data.views[9];
    let proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone();
    let epoch_change =
        EpochChangeMessage::validated(epoch_view.cert1.clone(), epoch_view.cert2.clone(), proposal);

    harness
        .apply(ConsensusInput::EpochChange(epoch_change))
        .await;

    assert!(
        !any(harness.outputs(), is_view_changed),
        "replayed epoch change for a crossed boundary must not change the view"
    );

    // The guard stops the cursors, not the data. This message is the only one
    // carrying the boundary block together with both its certificates, so a
    // node that never saw the boundary keeps them whatever the cursors do.
    let boundary_view = epoch_view.view_number;
    assert!(
        harness.consensus.proposal_at(boundary_view).is_some(),
        "the boundary proposal is kept"
    );
    assert!(
        harness.consensus.cert1_at(boundary_view).is_some(),
        "the boundary cert1 is kept"
    );
    assert!(
        harness.consensus.cert2_at(boundary_view).is_some(),
        "the boundary cert2 is kept"
    );
    assert!(
        any(harness.outputs(), is_leaf_decided),
        "and the view they certify can then decide"
    );
}

/// The locked-certificate guard keeps the boundary data too.
///
/// The other guard's test reaches it with a node that has already crossed;
/// this one reaches it with a lock past the boundary and nothing held at the
/// boundary view itself, which is the case where keeping the data matters.
#[tokio::test]
async fn test_handle_epoch_change_behind_the_lock_keeps_the_data() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;

    // A lock past the boundary, as a restart would restore it, without any of
    // the views in between having been run.
    harness
        .consensus
        .seed_locked_cert(test_data.views[10].cert1.clone());

    let epoch_view = &test_data.views[9];
    let boundary_view = epoch_view.view_number;
    assert!(
        harness.consensus.cert2_at(boundary_view).is_none(),
        "the boundary view starts empty"
    );

    let epoch_change = EpochChangeMessage::validated(
        epoch_view.cert1.clone(),
        epoch_view.cert2.clone(),
        epoch_view.proposal.data.clone(),
    );
    harness
        .apply(ConsensusInput::EpochChange(epoch_change))
        .await;

    assert!(
        !any(harness.outputs(), is_view_changed),
        "a lock newer than the epoch change must not move the view"
    );
    assert!(
        harness.consensus.proposal_at(boundary_view).is_some(),
        "the boundary proposal is kept"
    );
    assert!(
        harness.consensus.cert1_at(boundary_view).is_some(),
        "the boundary cert1 is kept"
    );
    assert!(
        harness.consensus.cert2_at(boundary_view).is_some(),
        "the boundary cert2 is kept"
    );
}

/// Locking the boundary block after crossing does not take the node back.
///
/// The block carries the epoch that ended, and the lock reaches it only after
/// the epoch change has moved the node on, within the same step: `apply` runs
/// `maybe_vote_2_and_update_lock` for the view the epoch change names. Writing
/// the block's own epoch there would undo the crossing, and the node would go
/// on signing timeout votes for the committee that has handed over.
#[tokio::test]
async fn test_locking_the_boundary_block_keeps_the_new_epoch() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new_with_epoch_height(11, EPOCH_HEIGHT).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    run_views_full(&mut harness, &test_data, &node_key, 0..9).await;

    // The boundary block's proposal and payload, but not its `Cert1`: the
    // lagging node an epoch change exists to carry across.
    let boundary = &test_data.views[9];
    harness
        .apply_pair(boundary.proposal_input_consensus(&node_key))
        .await;
    harness.apply(boundary.block_reconstructed_input()).await;

    let epoch_change = EpochChangeMessage::validated(
        boundary.cert1.clone(),
        boundary.cert2.clone(),
        boundary.proposal.data.clone(),
    );
    harness
        .apply(ConsensusInput::EpochChange(epoch_change))
        .await;

    assert_eq!(
        harness.consensus.lock_view().map(|lock| lock.view),
        Some(boundary.view_number),
        "the lock moved onto the boundary block in the same step"
    );
    assert_eq!(
        harness.consensus.current_epoch(),
        boundary.epoch_number + 1,
        "and the node stays in the epoch the epoch change entered"
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
    let epoch_change =
        EpochChangeMessage::validated(epoch_view.cert1.clone(), epoch_view.cert2.clone(), proposal);

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
    let epoch_change = EpochChangeMessage::validated(
        epoch_view.cert1.clone(),
        epoch_view.cert2.clone(),
        epoch_proposal,
    );

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
        .apply_pair((
            ConsensusInput::Proposal(
                first_view.leader_public_key,
                ProposalMessage::validated(signed_proposal),
            ),
            ConsensusInput::VidShare(vid_share),
        ))
        .await;

    assert!(
        any(harness.outputs(), is_vote1),
        "Node should send a vote after receiving epoch change and proposal for the new epoch"
    );
}

/// In the first epoch, the leader should be able to propose during the
/// epoch transition window without a DRB result: the membership seeds the
/// results of the first two epochs, so a transition proposal carries one
/// only from epoch 2 onward, for epoch 3.
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
    for view in &test_data.views[..7] {
        harness
            .apply_pair(view.proposal_input_consensus(&node_key))
            .await;
        harness.apply(view.block_reconstructed_input()).await;
        harness.apply(view.cert1_input()).await;
        harness.apply(view.cert2_input()).await;
    }

    assert!(
        any(harness.outputs(), is_proposal),
        "Leader should propose during the first epoch transition window without a DRB result"
    );
}

/// A node in epoch 2's transition window needs the DRB result of epoch 3,
/// which the transition proposals carry. This test runs through epoch 1,
/// crosses the epoch boundary and advances into epoch 2's transition window.
/// The result is the one computed when epoch 1's root was decided, and the
/// test verifies the node votes for a transition proposal carrying it and,
/// as the next leader, proposes with it.
#[tokio::test]
async fn test_second_epoch_node_votes_and_proposes_with_computed_drb() {
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

    cross_into_epoch_2(&mut harness, &test_data, &node_key).await;

    // ---- Epoch 2 views 12-16 (blocks 12-16, before transition window) ----
    for view in &test_data.views[11..16] {
        harness
            .apply_pair(view.proposal_input_consensus(&node_key))
            .await;
        harness.apply(view.block_reconstructed_input()).await;
        harness.apply(view.cert1_input()).await;
        harness.apply(view.cert2_input()).await;
    }

    // ---- Epoch 2 transition window (block 17+) ----
    // The node holds epoch 3's DRB result since it decided block 5, epoch 1's
    // root, so it never has to request it.
    // Process view 17 (block 17, first block in transition window).
    // After cert1 for view 17, the leader for view 18 should propose.
    harness
        .apply_pair(test_data.views[16].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[16].block_reconstructed_input())
        .await;
    harness.apply(test_data.views[16].cert1_input()).await;
    harness.apply(test_data.views[16].cert2_input()).await;

    let drb = test_data.views[16].proposal.data.next_drb_result;
    assert!(drb.is_some(), "view 17's proposal carries epoch 3's DRB");
    assert!(
        !has_request_drb_for_epoch(harness.outputs(), EpochNumber::new(3)),
        "the node holds epoch 3's DRB result from the decided epoch root"
    );
    assert!(
        any(harness.outputs(), |o| is_vote1_for_view(o, 17)),
        "the node votes for a transition proposal carrying the computed DRB result"
    );
    assert!(
        any(harness.outputs(), |o| matches!(
            o,
            ConsensusOutput::SendProposal(p)
                if p.data.view_number == ViewNumber::new(18) && p.data.next_drb_result == drb
        )),
        "the leader of view 18 proposes with the computed DRB result"
    );
}

/// When a node receives a transition-window proposal of epoch 2 or later
/// without holding the next epoch's DRB result, consensus should emit
/// RequestDrbResult for the next epoch. Here the node is in epoch 2, block
/// 17's proposal is relabelled into epoch 3, and the node has not decided
/// epoch 2's root, which epoch 4's result is computed from.
#[tokio::test]
async fn test_epoch3_transition_requests_drb_for_future_epoch() {
    let test_data = TestData::new_with_epoch_height(18, EPOCH_HEIGHT).await;

    let mut harness = ConsensusHarness::new(0).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // ---- Epoch 1 (views 1-9, blocks 1-9) ----
    run_views_full(&mut harness, &test_data, &node_key, 0..9).await;

    cross_into_epoch_2(&mut harness, &test_data, &node_key).await;

    // ---- Epoch 2 views 12-16 (before transition window) ----
    // No Cert2 from view 15 on, so block 15, epoch 2's root, stays undecided
    // and the harness has no DRB result for epoch 4.
    for (i, view) in (11..).zip(&test_data.views[11..16]) {
        harness
            .apply_pair(view.proposal_input_consensus(&node_key))
            .await;
        harness.apply(view.block_reconstructed_input()).await;
        harness.apply(view.cert1_input()).await;
        if i < 14 {
            harness.apply(view.cert2_input()).await;
        }
    }

    // Epoch 3's stake table and DRB result are registered in the membership
    // since block 5 was decided (epoch 1's root). Epoch 4's are not: block 15,
    // epoch 2's root, is undecided.

    let v17 = &test_data.views[16];
    let mut v17_proposal: Proposal<TestTypes> = v17.proposal.data.clone();
    v17_proposal.epoch = EpochNumber::new(3);

    let signed = SignedProposal {
        data: v17_proposal,
        signature: v17.proposal.signature.clone(),
        _pd: std::marker::PhantomData,
    };
    let mut vid_share = v17
        .vid_shares
        .iter()
        .find(|s| s.recipient_key == node_key)
        .expect("VID share not found")
        .clone();
    // The proposal is relabelled into epoch 3 to reach the transition window,
    // so its dispersal is too: a share pairs with a proposal only in the epoch
    // the proposal names.
    vid_share.epoch = Some(EpochNumber::new(3));
    vid_share.target_epoch = Some(EpochNumber::new(3));
    harness
        .apply_pair((
            ConsensusInput::Proposal(v17.leader_public_key, ProposalMessage::validated(signed)),
            ConsensusInput::VidShare(vid_share),
        ))
        .await;

    assert!(
        has_request_drb_for_epoch(harness.outputs(), EpochNumber::new(4)),
        "Consensus should request DRB for epoch 4 (current epoch 3 + 1) when processing a \
         transition-window proposal without a DRB result"
    );
}

/// A node restarted above epoch 1's root never decides it, so it does not
/// hold epoch 3's DRB result when epoch 2's transition proposals arrive. It
/// requests the result, withholds its vote, and votes once it is given the
/// result the proposal carries.
#[tokio::test]
async fn test_restarted_node_requests_drb_and_votes_once_given_it() {
    let test_data = TestData::new_with_epoch_height(17, EPOCH_HEIGHT).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    // Decided up to view 6, past block 5, epoch 1's root.
    let anchor = &test_data.views[5];
    let mut harness =
        ConsensusHarness::restarted_from(0, anchor.proposal.data.clone(), anchor.cert1.clone(), [])
            .await;

    run_views_full(&mut harness, &test_data, &node_key, 6..9).await;
    cross_into_epoch_2(&mut harness, &test_data, &node_key).await;
    run_views_full(&mut harness, &test_data, &node_key, 11..16).await;

    let v17 = &test_data.views[16];
    harness
        .apply_pair(v17.proposal_input_consensus(&node_key))
        .await;
    harness.apply(v17.block_reconstructed_input()).await;

    assert!(
        has_request_drb_for_epoch(harness.outputs(), EpochNumber::new(3)),
        "the node requests epoch 3's DRB result, which it has no root for"
    );
    assert!(
        !any(harness.outputs(), |o| is_vote1_for_view(o, 17)),
        "the vote waits for the DRB result"
    );

    let drb = v17.proposal.data.next_drb_result.expect("a DRB result");
    harness
        .apply(ConsensusInput::DrbResult(EpochNumber::new(3), drb))
        .await;
    assert!(
        any(harness.outputs(), |o| is_vote1_for_view(o, 17)),
        "the node votes once it has the DRB result the proposal carries"
    );
}
