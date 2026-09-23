use std::{marker::PhantomData, time::Duration};

use committable::{Commitment, CommitmentBoundsArkless, Committable};
use hotshot::types::{BLSPrivKey, BLSPubKey};
use hotshot_example_types::node_types::TestTypes;
use hotshot_testing::helpers::build_cert;
use hotshot_types::{
    data::{EpochNumber, VidCommitment2, ViewNumber},
    epoch_membership::EpochMembership,
    simple_certificate::{TimeoutCertificate3, TimeoutEvidence},
    simple_vote::{HasEpoch, QuorumData2, TimeoutData3, TimeoutVote3},
    stake_table::StakeTableEntries,
    traits::signature_key::SignatureKey,
    vote::HasViewNumber,
};

use super::common::{
    harness::TestHarness,
    utils::{TestData, build_cert1, build_cert2, build_timeout_cert3},
};
use crate::{
    consensus::{ConsensusInput, ConsensusOutput},
    coordinator::EPOCH_CHANGE_LOOKAHEAD,
    helpers::test_timeout_epoch_lock,
    message::{
        CatchupEvidence, Certificate1, ConsensusMessage, EpochChangeMessage, Message, MessageType,
        Proposal, Validated,
    },
    tests::common::assertions::{
        any, count_matching, is_block_built, is_block_reconstructed, is_cert1, is_cert2,
        is_drb_result, is_header_created, is_header_created_for_view, is_leaf_decided, is_proposal,
        is_proposal_for_view, is_request_block_and_header, is_request_vid_disperse, is_send_cert1,
        is_send_epoch_change, is_send_timeout_vote, is_state_validated, is_timeout,
        is_timeout_cert, is_timeout_one_honest, is_vid_disperse, is_view_changed, is_vote1,
        is_vote2, node_index_for_key,
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
    for fragment in test_view.vid_share_inputs(node_key) {
        harness.message(fragment);
    }
    harness.message(test_view.proposal_input());

    for i in 0..THRESHOLD {
        harness.message(test_view.vote1_input(i));
        harness.message(test_view.vid_share_broadcast_input(i))
    }

    harness
        .process_until(|inputs| {
            assert!(!any(inputs, is_timeout));
            any(inputs, is_cert1)
                && any(inputs, is_block_reconstructed)
                && any(inputs, is_state_validated)
        })
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
        harness.message(test_view.vote2_input(i));
    }
    harness
        .process_until(|inputs| {
            assert!(!any(inputs, is_timeout));
            any(inputs, is_cert2)
        })
        .await;
}

/// Send enough timeout votes to form a TimeoutCertificate.
async fn send_timeout_votes(
    harness: &mut TestHarness,
    test_data: &TestData,
    view_idx: usize,
    evidence: Option<CatchupEvidence<TestTypes>>,
) {
    let test_view = &test_data.views[view_idx];
    for i in 0..THRESHOLD {
        harness.message(test_view.timeout_vote_input(i, evidence.clone()));
    }
    harness
        .process_until(|inputs| {
            assert!(!any(inputs, is_timeout));
            any(inputs, is_timeout_cert)
        })
        .await;
    harness.apply_and_process(test_view.timeout_cert_input());
}

/// How long a negative assertion waits for something that must not happen.
///
/// Every test that uses it follows with a positive control, so a window too
/// short to reach the event cannot make the assertion pass silently.
const NO_CERT_WINDOW: Duration = Duration::from_secs(2);

/// A timeout vote in the form its version does not call for is not tallied.
///
/// The form check sits in front of both collectors. Without it the wrong form
/// would accumulate into a certificate that consensus then refuses, spending
/// the votes of a view on a certificate nothing can use.
#[tokio::test]
async fn test_timeout_votes_in_the_wrong_form_are_not_tallied() {
    let test_data = TestData::new(2).await;
    let timed_out = &test_data.views[0];

    // Votes that leave the epoch unbound, once the version binds it.
    let mut upgraded = TestHarness::new_with_upgrade_lock(0, test_timeout_epoch_lock()).await;
    for i in 0..THRESHOLD {
        upgraded.message(timed_out.timeout_vote_input(i, None));
    }
    let inputs = upgraded.process_for(NO_CERT_WINDOW).await;
    assert!(
        !any(&inputs, is_timeout_cert),
        "unbound votes must not certify a view whose version binds the epoch"
    );
    for i in 0..THRESHOLD {
        upgraded.message(timed_out.timeout_vote3_input(i, None));
    }
    upgraded
        .process_until(|inputs| any(inputs, is_timeout_cert))
        .await;

    // ...and votes that bind it before the version does.
    let mut current = TestHarness::new(0).await;
    for i in 0..THRESHOLD {
        current.message(timed_out.timeout_vote3_input(i, None));
    }
    let inputs = current.process_for(NO_CERT_WINDOW).await;
    assert!(
        !any(&inputs, is_timeout_cert),
        "epoch binding votes must not certify a view whose version does not bind it"
    );
    for i in 0..THRESHOLD {
        current.message(timed_out.timeout_vote_input(i, None));
    }
    current
        .process_until(|inputs| any(inputs, is_timeout_cert))
        .await;
}

/// Timeout votes that bind an epoch outside the admissible window are not
/// tallied. The epoch they bind is a committee this node has no business
/// certifying a view under, however well the signatures check out.
#[tokio::test]
async fn test_timeout_votes_from_an_inadmissible_epoch_are_not_tallied() {
    let test_data = TestData::new(2).await;
    let timed_out = &test_data.views[0];
    let mut harness = TestHarness::new_with_upgrade_lock(0, test_timeout_epoch_lock()).await;

    // The node is in the genesis epoch, so this is past the lookahead. It is
    // registered, so the votes binding it resolve a committee and only the
    // window check stands between them and a certificate.
    let far = timed_out.epoch_number + (EPOCH_CHANGE_LOOKAHEAD + 1);
    harness
        .membership()
        .membership()
        .register_epoch(far, [0u8; 32]);
    for i in 0..THRESHOLD {
        harness.message(timed_out.timeout_vote3_input_for_epoch(i, far, None));
    }
    let inputs = harness.process_for(NO_CERT_WINDOW).await;
    assert!(
        !any(&inputs, is_timeout_cert),
        "votes binding an epoch outside the window must not certify the view"
    );

    for i in 0..THRESHOLD {
        harness.message(timed_out.timeout_vote3_input(i, None));
    }
    harness
        .process_until(|inputs| any(inputs, is_timeout_cert))
        .await;
}

/// A timeout certificate whose signed data names another view than the one it
/// is for is not delivered to consensus.
///
/// The certificate here is signed by a real quorum, so the signature check
/// passes and only the view check stands between it and the node. Delivered,
/// it would advance the node past a view its signers never attested to.
#[tokio::test]
async fn test_timeout_certificate_naming_another_view_is_not_delivered() {
    let test_data = TestData::new(2).await;
    let timed_out = &test_data.views[0];
    let mut harness = TestHarness::new_with_upgrade_lock(0, test_timeout_epoch_lock()).await;
    let membership = harness
        .membership()
        .membership_for_epoch(Some(timed_out.epoch_number))
        .expect("the genesis epoch resolves");
    let message = |named: ViewNumber| Message::<TestTypes, Validated> {
        sender: timed_out.leader_public_key,
        message_type: MessageType::Consensus(ConsensusMessage::TimeoutCertificate3(
            timeout_cert_naming(
                timed_out.view_number,
                named,
                timed_out.epoch_number,
                &membership,
                &timed_out.leader_public_key,
                &timed_out.leader_private_key,
            ),
        )),
    };

    harness.message(message(timed_out.view_number + 1));
    let inputs = harness.process_for(NO_CERT_WINDOW).await;
    assert!(
        !any(&inputs, is_timeout_cert),
        "a certificate naming another view must not reach consensus"
    );

    harness.message(message(timed_out.view_number));
    harness
        .process_until(|inputs| any(inputs, is_timeout_cert))
        .await;
}

/// The same holds for a timeout certificate a proposal carries as evidence,
/// which is checked by `TimeoutEvidence::is_valid_cert` rather than by the
/// coordinator's intake.
#[tokio::test]
async fn test_timeout_evidence_naming_another_view_is_invalid() {
    let test_data = TestData::new(2).await;
    let timed_out = &test_data.views[0];
    let harness = TestHarness::new_with_upgrade_lock(0, test_timeout_epoch_lock()).await;
    let membership = harness
        .membership()
        .membership_for_epoch(Some(timed_out.epoch_number))
        .expect("the genesis epoch resolves");
    let entries = StakeTableEntries::<TestTypes>::from_iter(membership.stake_table()).0;
    let threshold = membership.success_threshold();
    let lock = test_timeout_epoch_lock();
    let evidence = |named: ViewNumber| {
        TimeoutEvidence::V3(timeout_cert_naming(
            timed_out.view_number,
            named,
            timed_out.epoch_number,
            &membership,
            &timed_out.leader_public_key,
            &timed_out.leader_private_key,
        ))
    };

    assert!(
        evidence(timed_out.view_number)
            .is_valid_cert(&entries, threshold, &lock)
            .is_ok(),
        "a certificate naming its own view is valid"
    );
    assert!(
        evidence(timed_out.view_number + 1)
            .is_valid_cert(&entries, threshold, &lock)
            .is_err(),
        "a certificate naming another view is not, although its signatures check out"
    );
}

/// An unsigned `HighQc` claiming view 0 does not move a fresh node.
///
/// A view-0 `Cert1` passes the verifier unsigned, because the genesis QC is
/// unsigned and honest nodes send it as catchup evidence. A fresh node is still
/// at view 0, so without a further check `handle_advance_view` would adopt
/// whatever epoch the certificate names, as long as that epoch agreed with the
/// block number it also names. Consensus accepts it only if it is the genesis
/// QC.
#[tokio::test]
async fn test_unsigned_view_zero_high_qc_does_not_move_the_epoch() {
    let mut harness = TestHarness::new(0).await;
    assert_eq!(
        harness.current_view(),
        ViewNumber::genesis(),
        "setup: a fresh node"
    );

    let forged = EpochNumber::genesis() + 1;
    harness
        .membership()
        .membership()
        .register_epoch(forged, [0u8; 32]);
    // Block 11 is in epoch 2 at an epoch height of 10, so the pair is well formed.
    let data = QuorumData2::<TestTypes> {
        leaf_commit: Commitment::default_commitment_no_preimage(),
        epoch: Some(forged),
        block_number: Some(11),
    };
    let cert = Certificate1::<TestTypes>::new(
        data,
        data.commit(),
        ViewNumber::genesis(),
        None,
        PhantomData,
    );
    harness.message(Message::<TestTypes, Validated> {
        sender: BLSPubKey::generated_from_seed_indexed([0u8; 32], 1).0,
        message_type: MessageType::Consensus(ConsensusMessage::HighQc(cert)),
    });
    harness.process_for(NO_CERT_WINDOW).await;

    assert_eq!(
        harness.coordinator().consensus().current_epoch(),
        Some(EpochNumber::genesis()),
        "an unsigned view-0 certificate must not move the epoch"
    );
    assert_eq!(
        harness.current_view(),
        ViewNumber::genesis(),
        "nor the view"
    );
}

/// An unsigned timeout certificate claiming view 0 does not move a fresh
/// node's epoch.
///
/// Same exemption as for the `HighQc`, and worse: a timeout certificate's
/// epoch is bound to no block height, so nothing constrains it at all, and a
/// fresh node enters view 1 on it.
#[tokio::test]
async fn test_unsigned_view_zero_timeout_certificate_does_not_move_the_epoch() {
    let mut harness = TestHarness::new_with_upgrade_lock(0, test_timeout_epoch_lock()).await;
    assert_eq!(
        harness.current_view(),
        ViewNumber::genesis(),
        "setup: a fresh node"
    );

    let forged = EpochNumber::genesis() + 1;
    harness
        .membership()
        .membership()
        .register_epoch(forged, [0u8; 32]);
    let data = TimeoutData3 {
        view: ViewNumber::genesis(),
        epoch: forged,
    };
    let cert = TimeoutCertificate3::<TestTypes>::new(
        data.clone(),
        data.commit(),
        ViewNumber::genesis(),
        None,
        PhantomData,
    );
    harness.message(Message::<TestTypes, Validated> {
        sender: BLSPubKey::generated_from_seed_indexed([0u8; 32], 1).0,
        message_type: MessageType::Consensus(ConsensusMessage::TimeoutCertificate3(cert)),
    });
    let inputs = harness.process_for(NO_CERT_WINDOW).await;

    assert_eq!(
        harness.coordinator().consensus().current_epoch(),
        Some(EpochNumber::genesis()),
        "an unsigned view-0 timeout certificate must not move the epoch"
    );
    assert!(!any(&inputs, is_timeout_cert), "nor reach consensus at all");
}

/// A forged view-0 `Cert1` does not stop the genesis QC advancing the view.
///
/// The forgery names the genesis epoch, so it and the genesis QC share the
/// `advance` verifier's completion key. Were it verified and only then refused
/// in consensus, it would retire that key and the genesis QC, which a fresh
/// peer offers as catchup evidence, would be dropped as already completed.
#[tokio::test]
async fn test_forged_view_zero_high_qc_does_not_shadow_the_genesis_qc() {
    let mut harness = TestHarness::new(0).await;
    let genesis = harness.seed_genesis();
    let epoch = EpochNumber::genesis();
    let high_qc = |qc: Certificate1<TestTypes>, signer: u64| Message::<TestTypes, Validated> {
        sender: BLSPubKey::generated_from_seed_indexed([0u8; 32], signer).0,
        message_type: MessageType::Consensus(ConsensusMessage::HighQc(qc)),
    };

    // Well formed, in the genesis epoch, and not the genesis QC.
    let data = QuorumData2::<TestTypes> {
        leaf_commit: Commitment::default_commitment_no_preimage(),
        epoch: Some(epoch),
        block_number: Some(5),
    };
    let forged = Certificate1::<TestTypes>::new(
        data,
        data.commit(),
        ViewNumber::genesis(),
        None,
        PhantomData,
    );
    harness.message(high_qc(forged, 1));
    harness.process_for(NO_CERT_WINDOW).await;
    assert_eq!(
        harness.current_view(),
        ViewNumber::genesis(),
        "the forgery must not move the view"
    );

    harness.message(high_qc(genesis, 2));
    harness.process_for(NO_CERT_WINDOW).await;
    assert_eq!(
        harness.current_view(),
        ViewNumber::new(1),
        "the genesis QC still advances the view after the forgery"
    );
}

/// A timeout certificate for view 0 that a quorum really signed is delivered.
///
/// Signatures are now checked at the genesis view for timeout certificates,
/// where `is_valid_cert` skips them. A genuine one has them and still gets
/// through, so a view that times out at genesis can be left.
#[tokio::test]
async fn test_signed_view_zero_timeout_certificate_is_delivered() {
    let test_data = TestData::new(2).await;
    let signer = &test_data.views[0];
    let mut harness = TestHarness::new_with_upgrade_lock(0, test_timeout_epoch_lock()).await;
    let epoch = EpochNumber::genesis();
    let membership = harness
        .membership()
        .membership_for_epoch(Some(epoch))
        .expect("the genesis epoch resolves");
    let cert = timeout_cert_naming(
        ViewNumber::genesis(),
        ViewNumber::genesis(),
        epoch,
        &membership,
        &signer.leader_public_key,
        &signer.leader_private_key,
    );

    harness.message(Message::<TestTypes, Validated> {
        sender: signer.leader_public_key,
        message_type: MessageType::Consensus(ConsensusMessage::TimeoutCertificate3(cert)),
    });
    harness
        .process_until(|inputs| any(inputs, is_timeout_cert))
        .await;
    assert_eq!(
        harness.current_view(),
        ViewNumber::new(1),
        "the certificate moves the node into view 1"
    );
}

/// A timeout certificate a quorum really signed, for `view`, whose data names
/// `named`.
///
/// `build_timeout_cert3` always sets the two equal. These tests need them
/// apart with the signatures still valid, so that the view check is the only
/// thing that can reject the certificate.
fn timeout_cert_naming(
    view: ViewNumber,
    named: ViewNumber,
    epoch: EpochNumber,
    membership: &EpochMembership<TestTypes>,
    public_key: &BLSPubKey,
    private_key: &BLSPrivKey,
) -> TimeoutCertificate3<TestTypes> {
    build_cert::<TestTypes, TimeoutData3, TimeoutVote3<TestTypes>, TimeoutCertificate3<TestTypes>>(
        TimeoutData3 { view: named, epoch },
        membership,
        view,
        public_key,
        private_key,
        &test_timeout_epoch_lock(),
    )
}

/// Timeout votes that do not bind their epoch are tallied together, whatever
/// epoch they name.
///
/// Their signature covers only the view, so the epoch field names no
/// committee and the vote says no more than that its signer gave up on the
/// view. Splitting the tally by that field would let an epoch boundary, where
/// honest nodes disagree about the epoch, leave both sides short of a
/// threshold their votes together reach.
#[tokio::test]
async fn test_unbound_timeout_votes_pool_across_the_epochs_they_name() {
    let test_data = TestData::new(2).await;
    let timed_out = &test_data.views[0];
    let mut harness = TestHarness::new(0).await;

    // Put the node in an epoch neither vote names, so the epoch the
    // certificate ends up with can only have come from the node itself.
    let ours = timed_out.epoch_number + 2;
    harness
        .membership()
        .membership()
        .register_epoch(ours, [0u8; 32]);
    harness.set_view(timed_out.view_number, ours);

    // Split as an epoch boundary splits them: neither named epoch has enough
    // votes on its own.
    let next = timed_out.epoch_number + 1;
    for i in 0..THRESHOLD {
        let named = if i % 2 == 0 {
            timed_out.epoch_number
        } else {
            next
        };
        harness.message(timed_out.timeout_vote_input_for_epoch(i, named, None));
    }

    harness
        .process_until(|inputs| any(inputs, is_timeout_cert))
        .await;

    let certs: Vec<_> = harness
        .outputs()
        .iter()
        .filter_map(|o| match o {
            ConsensusOutput::SendTimeoutCertificate(cert, _, epoch) => Some((cert.clone(), *epoch)),
            _ => None,
        })
        .collect();
    let [(cert, epoch)] = certs.as_slice() else {
        panic!("expected one timeout certificate, got {certs:?}");
    };
    assert!(!cert.binds_epoch());
    assert_eq!(
        HasEpoch::epoch(cert),
        Some(ours),
        "the certificate names the epoch of the node that formed it"
    );
    assert_eq!(*epoch, ours);
}

/// A timeout vote carries its sender's catchup evidence, and that evidence is
/// applied even when the vote itself is inadmissible.
///
/// A node that is behind binds an epoch the receiver has left, so gating the
/// evidence on the vote being tallyable would deny catchup to exactly the
/// nodes that need it. The view analogue is
/// `test_timeout_vote_evidence_overrides_distance_check`.
#[tokio::test]
async fn test_timeout_vote_evidence_overrides_the_epoch_check() {
    let test_data = TestData::new(3).await;
    let mut harness = TestHarness::new_with_upgrade_lock(0, test_timeout_epoch_lock()).await;
    let timed_out = &test_data.views[1];
    let epoch = timed_out.epoch_number;
    let far = epoch + (EPOCH_CHANGE_LOOKAHEAD + 1);

    let membership = harness
        .membership()
        .membership_for_epoch(Some(epoch))
        .expect("the genesis epoch resolves");
    let evidence = build_timeout_cert3(
        test_data.views[0].view_number,
        epoch,
        &membership,
        &test_data.views[0].leader_public_key,
        &test_data.views[0].leader_private_key,
    );

    // A peer times out view 2 binding an epoch past the lookahead, attaching
    // the timeout certificate for view 1.
    harness.message(timed_out.timeout_vote3_input_for_epoch(
        1,
        far,
        Some(CatchupEvidence::from(&evidence)),
    ));

    harness
        .process_until(|inputs| any(inputs, is_timeout_cert))
        .await;

    assert_eq!(
        *harness.current_view(),
        2,
        "the attached timeout certificate should advance us to view 2"
    );
}

/// Under the timeout epoch version the whole timeout path takes the epoch
/// binding form: the votes are collected by the binding tally, the
/// certificate it forms passes the form check in consensus, and the view
/// advances on it.
#[tokio::test]
async fn test_epoch_binding_timeout_votes_form_a_certificate() {
    let test_data = TestData::new(2).await;
    let mut harness = TestHarness::new_with_upgrade_lock(0, test_timeout_epoch_lock()).await;
    let test_view = &test_data.views[0];

    for i in 0..THRESHOLD {
        harness.message(test_view.timeout_vote3_input(i, None));
    }
    harness
        .process_until(|inputs| any(inputs, is_timeout_cert))
        .await;

    let certs: Vec<_> = harness
        .outputs()
        .iter()
        .filter_map(|o| match o {
            ConsensusOutput::SendTimeoutCertificate(cert, view, epoch) => {
                Some((cert.clone(), *view, *epoch))
            },
            _ => None,
        })
        .collect();
    let [(cert, view, epoch)] = certs.as_slice() else {
        panic!("expected one timeout certificate, got {certs:?}");
    };
    assert!(cert.binds_epoch(), "the certificate must bind its epoch");
    assert_eq!(*view, test_view.view_number + 1);
    assert_eq!(*epoch, test_view.epoch_number);
    assert!(
        any(harness.outputs(), is_view_changed),
        "the certificate must advance the view"
    );
}

/// Integration: sequential views both produce Vote1 through real state validation.
#[tokio::test]
async fn test_sequential_vote1() {
    let test_data = TestData::new(3).await;
    let mut harness = TestHarness::new(0).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    for fragment in test_data.views[0].vid_share_inputs(&node_key) {
        harness.message(fragment);
    }
    harness.message(test_data.views[0].proposal_input());
    harness.apply_and_process(test_data.views[0].block_reconstructed_input());

    for fragment in test_data.views[1].vid_share_inputs(&node_key) {
        harness.message(fragment);
    }
    harness.message(test_data.views[1].proposal_input());

    harness
        .process_until(|inputs| {
            assert!(!any(inputs, is_timeout));
            count_matching(inputs, is_state_validated) >= 2
        })
        .await;
    harness
        .process_until_output(|outputs| count_matching(outputs, is_vote1) >= 2)
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

    harness
        .process_until_output(|outputs| any(outputs, is_vote1) && any(outputs, is_vote2))
        .await;
}

/// Full decide path: Certificate1, Certificate2, and block reconstruction
/// from VID shares lead to a leaf decision.
#[tokio::test]
async fn test_full_decide() {
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

    harness
        .process_until_output(|outputs| any(outputs, is_vote1) && any(outputs, is_vote2))
        .await;

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

/// Leader sends a proposal after Certificate1 is formed.
/// SendProposal in the output proves the full leader path:
/// cert1 formation → block/header request → VID disperse → proposal sent.
#[tokio::test]
async fn test_leader_proposal() {
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

    for fragment in test_view.vid_share_inputs(&leader_for_view_2) {
        harness.message(fragment);
    }
    harness.message(test_view.proposal_input());

    for i in 0..THRESHOLD {
        harness.message(test_view.vote1_input(i));
        harness.message(test_view.vid_share_broadcast_input(i))
    }

    harness
        .process_until(|inputs| {
            assert!(!any(inputs, is_timeout));
            any(inputs, is_cert1)
                && any(inputs, is_block_reconstructed)
                && any(inputs, is_state_validated)
                && any(inputs, is_block_built)
                && any(inputs, is_header_created)
                && any(inputs, is_vid_disperse)
        })
        .await;

    // SendProposal proves the CPU VidDisperseTask computed the VID
    // disperse — consensus cannot send a proposal without it.
    harness
        .process_until_output(|outputs| any(outputs, is_proposal))
        .await;
}

/// Multi-view chain: certificates are formed for each view, leading to
/// multiple decisions.
#[tokio::test]
async fn test_multi_view_decide() {
    // Block 5 is an epoch root at epoch_height=10 (the harness default), so
    // TestData must also use epoch_height=10 for the generator to attach the
    // state_cert/state_votes that the coordinator's routing now requires.
    let test_data = TestData::new_with_epoch_height(5, 10).await;
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
    let lock = Some(CatchupEvidence::Qc(test_data.views[0].cert1.clone()));
    send_timeout_votes(&mut harness, &test_data, 1, lock).await;

    assert!(
        any(harness.outputs(), is_view_changed),
        "View should advance after timeout certificate"
    );
}

/// Full leader path after timeout: establish lock → timer fires for
/// view 2 (Timeout) → timeout votes form TC → leader proposes for
/// view 3 before the view 3 timer fires.
#[tokio::test]
async fn test_leader_proposes_after_timeout() {
    let test_data = TestData::new(5).await;
    // Timeout cert for view 2 advances to view 3; we need to be leader of view 3
    let leader_for_view_3 = test_data.views[2].leader_public_key;
    let leader_index = node_index_for_key(&leader_for_view_3);
    // Timer must be long enough for the empty-block throttle sleep
    // (BlockBuilder sleeps 500ms when its buffer is empty), VID disperse,
    // and header creation to all complete before the view 3 timer fires.
    // It must also be short enough to actually fire for view 2 during
    // the test in a reasonable amount of time.
    let mut harness =
        TestHarness::new_with_timer(leader_index, std::time::Duration::from_millis(2000)).await;

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
    let lock = Some(CatchupEvidence::Qc(test_data.views[0].cert1.clone()));
    send_timeout_votes(&mut harness, &test_data, 1, lock).await;

    harness
        .process_until(|inputs| {
            assert!(!any(inputs, is_timeout));
            any(inputs, is_vid_disperse)
                && any(inputs, is_block_built)
                && any(inputs, is_header_created)
        })
        .await;

    assert!(
        any(harness.outputs(), is_request_block_and_header),
        "Leader should request block and header after TC"
    );

    assert!(
        any(harness.outputs(), is_request_vid_disperse),
        "Leader should request VID disperse after TC"
    );
    // Proposal with timeout view change evidence, released once stored.
    harness
        .process_until_output(|outputs| any(outputs, is_proposal))
        .await;
}

// ---------------------------------------------------------------------------
// Epoch change integration tests
// ---------------------------------------------------------------------------

const EPOCH_HEIGHT: u64 = 10;

/// Helper to build an EpochChangeMessage from test data at the epoch boundary view.
fn epoch_change_message(test_data: &TestData) -> EpochChangeMessage<TestTypes, Validated> {
    let epoch_view = &test_data.views[9]; // view 10, last block of epoch 1
    let proposal: Proposal<TestTypes> = epoch_view.proposal.data.clone();
    EpochChangeMessage::validated(epoch_view.cert1.clone(), epoch_view.cert2.clone(), proposal)
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
        any(harness.outputs(), is_send_epoch_change),
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
    harness.apply_and_process(ConsensusInput::EpochChange(epoch_change));

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
    harness.apply_and_process(ConsensusInput::EpochChange(epoch_change));

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
    let epoch_change = EpochChangeMessage::validated(
        boundary_view.cert1.clone(),
        boundary_view.cert2.clone(),
        proposal,
    );
    harness.apply_and_process(ConsensusInput::EpochChange(epoch_change));

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

    // ---- Epoch 1 views 1-5 ----
    // Block 5 decision triggers epoch root → DRB for epoch 3 is computed.
    run_views_integration(&mut harness, &test_data, &leader_key, 0..5).await;

    // Wait for the DRB result from the EpochManager to arrive.
    harness
        .process_until(|inputs| {
            assert!(!any(inputs, is_timeout));
            any(inputs, is_drb_result)
        })
        .await;

    // ---- Epoch 1 views 6-10 ----
    run_views_integration(&mut harness, &test_data, &leader_key, 5..10).await;

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
    // The proposal is emitted only once the block builder finishes (it sleeps
    // before producing an empty block) and the resulting header for view 28 is
    // applied
    harness
        .process_until(|inputs| any(inputs, |i| is_header_created_for_view(i, 28)))
        .await;
    // Leader proposes view 28 in epoch 3's transition window using the DRB
    // result computed in epoch 1.
    harness
        .process_until_output(|outputs| any(outputs, |o| is_proposal_for_view(o, 28)))
        .await;
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

    // ---- Epoch 1 views 1-5 ----
    // Block 5 decision triggers epoch root → DRB for epoch 3 is computed.
    run_views_integration(&mut harness, &test_data, &node_key, 0..5).await;

    // Wait for the DRB result from the EpochManager to arrive.
    harness
        .process_until(|inputs| {
            assert!(!any(inputs, is_timeout));
            any(inputs, is_drb_result)
        })
        .await;

    // ---- Epoch 1 views 6-10 ----
    run_views_integration(&mut harness, &test_data, &node_key, 5..10).await;

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

    // The node votes on the epoch 3 transition-window proposal with the
    // computed DRB result.
    harness
        .process_until_output(|outputs| count_matching(outputs, is_vote1) > vote_count_before)
        .await;
}

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
    let lock = Some(CatchupEvidence::Qc(test_data.views[0].cert1.clone()));
    for i in 0..ONE_HONEST_THRESHOLD {
        harness.message(test_view.timeout_vote_input(i, lock.clone()));
    }

    harness
        .process_until(|inputs| {
            assert!(!any(inputs, is_timeout));
            any(inputs, is_timeout_one_honest)
        })
        .await;

    assert!(
        any(harness.outputs(), is_send_timeout_vote),
        "f+1 timeout votes should trigger TimeoutOneHonest"
    );
}

/// A peer's timeout vote carries its locked QC.
///
/// A node that is behind adopts that QC's view (+ 1).
#[tokio::test]
async fn test_timeout_vote_lock_advances_view() {
    use crate::consensus::ConsensusOutput;

    let view_changed_to = |outputs, v| {
        count_matching(
            outputs,
            |o| matches!(o, ConsensusOutput::ViewChanged(view, _) if **view == v),
        )
    };

    let test_data = TestData::new(3).await;
    let mut harness = TestHarness::new(0).await;

    // The node starts at genesis. A peer times out view 2 while carrying a
    // locked QC for view 1 (`views[0].cert1`). Adopting that QC moves us into
    // view 2.
    let high_qc = CatchupEvidence::Qc(test_data.views[0].cert1.clone());
    harness.message(test_data.views[1].timeout_vote_input(1, Some(high_qc.clone())));

    harness
        .process_until_output(|outputs| {
            any(
                outputs,
                |o| matches!(o, ConsensusOutput::ViewChanged(view, _) if **view == 2),
            )
        })
        .await;

    assert_eq!(
        view_changed_to(harness.outputs(), 2),
        1,
        "adopting the locked QC from a timeout vote should advance us to view 2"
    );

    // A later timeout vote for view 3 carrying the same (now stale) view-1 lock
    // would not advance us, so no further view change is emitted.
    let view_changes_before = count_matching(harness.outputs(), is_view_changed);
    harness.message(test_data.views[2].timeout_vote_input(2, Some(high_qc)));
    assert_eq!(
        count_matching(harness.outputs(), is_view_changed),
        view_changes_before,
        "a lock below our current view must not advance us"
    );
}

/// A peer's timeout vote carries its latest timeout certificate.
///
/// A node that fell behind on views that only advanced via timeouts (so no
/// newer QC exists anywhere) adopts the certificate and re-arms its timer,
/// so its next local timeout is for the adopted view.
#[tokio::test]
async fn test_timeout_vote_tc_advances_view() {
    let test_data = TestData::new(3).await;
    let mut harness = TestHarness::new_with_timer(0, Duration::from_millis(250)).await;

    // The node starts at genesis. A peer times out view 3, attaching the
    // timeout certificate for view 2 that formed without us.
    let tc = CatchupEvidence::from(&test_data.views[1].timeout_cert);
    harness.message(test_data.views[2].timeout_vote_input(1, Some(tc)));

    harness
        .process_until(|inputs| any(inputs, is_timeout_cert))
        .await;

    assert_eq!(
        *harness.current_view(),
        3,
        "the attached timeout certificate should advance us to view 3"
    );

    harness
        .process_until(|inputs| {
            inputs
                .iter()
                .any(|i| matches!(i, ConsensusInput::Timeout(view) if **view == 3))
        })
        .await;
}

/// A stuck node's view timer keeps re-firing for the same view (and
/// re-broadcasting its timeout vote) as long as no progress is made. A
/// single lost timeout-vote broadcast must not deadlock the network forever.
#[tokio::test]
async fn test_timer_refires_for_same_view_without_progress() {
    let mut harness = TestHarness::new_with_timer(0, Duration::from_millis(100)).await;

    let first_view = harness
        .process_until(|inputs| any(inputs, is_timeout))
        .await
        .into_iter()
        .find_map(|i| match i {
            ConsensusInput::Timeout(view) => Some(view),
            _ => None,
        })
        .expect("timer should fire");

    let timeout_votes_before = count_matching(harness.outputs(), is_send_timeout_vote);

    let second_view = tokio::time::timeout(
        Duration::from_secs(5),
        harness.process_until(|inputs| any(inputs, is_timeout)),
    )
    .await
    .expect("timer should re-fire for the same view within one timeout period")
    .into_iter()
    .find_map(|i| match i {
        ConsensusInput::Timeout(view) => Some(view),
        _ => None,
    })
    .expect("timer should re-fire for the same view when no progress is made");

    assert_eq!(
        second_view, first_view,
        "the timer should keep re-firing for the same view until it advances"
    );
    assert_eq!(
        count_matching(harness.outputs(), is_send_timeout_vote),
        timeout_votes_before + 1,
        "the re-fired timeout should trigger another timeout vote broadcast"
    );
}

/// Catchup evidence takes precedence over the "too far ahead" distance
/// check: the vote itself is dropped, but the attached certificate still
/// advances a node that fell far behind.
#[tokio::test]
async fn test_timeout_vote_evidence_overrides_distance_check() {
    let test_data = TestData::new(32).await;
    let mut harness = TestHarness::new(0).await;

    // The node starts at genesis (view 1). A peer times out view 32 — more
    // than 30 views ahead, so the vote is dropped — attaching the timeout
    // certificate for view 31.
    let tc = CatchupEvidence::from(&test_data.views[30].timeout_cert);
    harness.message(test_data.views[31].timeout_vote_input(1, Some(tc)));

    harness
        .process_until(|inputs| any(inputs, is_timeout_cert))
        .await;

    assert_eq!(
        *harness.current_view(),
        32,
        "the attached timeout certificate should advance us to view 32"
    );
}

/// A stale timeout vote is answered with the responder's newest certificate,
/// so a node stuck behind receives the evidence it needs to advance.
#[tokio::test]
async fn test_stale_timeout_vote_answered_with_catchup_evidence() {
    let test_data = TestData::new(3).await;
    let mut harness = TestHarness::new(0).await;

    // Advance to view 3 via the network's timeout certificates.
    harness.apply_and_process(test_data.views[0].timeout_cert_input());
    harness.apply_and_process(test_data.views[1].timeout_cert_input());
    assert_eq!(*harness.current_view(), 3);

    let evidence = harness
        .coordinator()
        .catchup_evidence()
        .expect("a node past genesis has catchup evidence");
    assert!(
        matches!(
            &evidence,
            ConsensusMessage::TimeoutCertificate(tc) if *tc.view_number() == 2
        ),
        "expected the view-2 timeout certificate, got {evidence:?}"
    );

    // The stale vote itself is only answered (unicast to its sender), never
    // tallied: it must not regress or otherwise disturb consensus state.
    harness.message(test_data.views[0].timeout_vote_input(1, None));
    assert_eq!(*harness.current_view(), 3);
}

/// A VID share fragment authorises its sender through an epoch the sender
/// chose, over a signature covering only the payload commitment. Neither a
/// stream from a node that does not lead the view nor one naming an epoch
/// outside the admissible window may displace the honest leader's: the node
/// must still assemble its own share, vote, and broadcast the real share.
///
/// The forged streams are shaped as squats -- a single namespace, so they
/// would complete on their first fragment -- which under a view-keyed
/// accumulator would retire the view and strand the honest fragments.
#[tokio::test]
async fn test_forged_vid_fragments_do_not_block_the_vote() {
    let test_data = TestData::new(1).await;
    let mut harness = TestHarness::new(0).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;
    let view = &test_data.views[0];

    // Shape a squat: claim a single namespace so the stream completes at once,
    // and a commitment of its own so an admitted forgery is distinguishable
    // from the honest share.
    let squat = |fragment: &mut Message<TestTypes, Validated>| {
        if let MessageType::Consensus(ConsensusMessage::VidShareFragment(f)) =
            &mut fragment.message_type
        {
            f.data.num_namespaces = 1;
            f.data.namespaces.truncate(1);
            f.data.payload_commitment = VidCommitment2::default();
        }
    };

    // A node that does not lead this view, relaying the leader's signature
    // (it covers only the payload commitment, so it is public).
    let impostor = (1..10u64)
        .map(|i| BLSPubKey::generated_from_seed_indexed([0; 32], i).0)
        .find(|key| *key != view.leader_public_key)
        .expect("a non-leader among the committee");
    for mut fragment in view.vid_share_inputs(&node_key) {
        fragment.sender = impostor;
        squat(&mut fragment);
        harness.message(fragment);
    }

    // The view's real leader, but naming an epoch outside the window.
    for mut fragment in view.vid_share_inputs(&node_key) {
        squat(&mut fragment);
        if let MessageType::Consensus(ConsensusMessage::VidShareFragment(f)) =
            &mut fragment.message_type
        {
            f.data.epoch = Some(EpochNumber::new(999));
            f.data.target_epoch = Some(EpochNumber::new(999));
        }
        harness.message(fragment);
    }

    harness.message(view.proposal_input());
    for fragment in view.vid_share_inputs(&node_key) {
        harness.message(fragment);
    }

    // If either forged stream had taken the view, the honest share would never
    // assemble and no vote1 would ever be produced.
    tokio::time::timeout(
        Duration::from_secs(5),
        harness.process_until_output(|outputs| any(outputs, is_vote1)),
    )
    .await
    .expect("forged fragments must not stop the honest share assembling");

    // And the share it broadcasts is its own, not a forgery.
    let broadcast = harness
        .outputs()
        .iter()
        .find_map(|output| match output {
            ConsensusOutput::BroadcastVidShare(share) => Some(share.clone()),
            _ => None,
        })
        .expect("the node broadcasts its share alongside vote1");
    assert_eq!(broadcast, view.vid_share_for(&node_key));
}

/// A signed `Cert1` whose epoch is not the one its block number falls in is
/// not delivered, and does not stop a well-formed one for the same view.
#[tokio::test]
async fn test_malformed_certificate1_is_not_delivered() {
    let signers = TestData::new(2).await;
    let mut harness = TestHarness::new(0).await;
    let (view, epoch) = (ViewNumber::new(1), EpochNumber::genesis());
    let membership = harness
        .membership()
        .membership_for_epoch(Some(epoch))
        .expect("the genesis epoch resolves");
    let cert = |block| {
        let signer = &signers.views[0];
        build_cert1(
            Commitment::default_commitment_no_preimage(),
            epoch,
            block,
            &membership,
            view,
            &signer.leader_public_key,
            &signer.leader_private_key,
        )
    };

    let message = |cert, sender: &BLSPubKey| Message::<TestTypes, Validated> {
        sender: *sender,
        message_type: MessageType::Consensus(ConsensusMessage::Certificate1(cert, *sender)),
    };

    harness.message(message(
        cert(MALFORMED_BLOCK),
        &signers.views[0].leader_public_key,
    ));
    let inputs = harness.process_for(NO_CERT_WINDOW).await;
    assert!(
        !any(&inputs, is_cert1),
        "a malformed cert1 must not be delivered"
    );

    harness.message(message(
        cert(WELL_FORMED_BLOCK),
        &signers.views[1].leader_public_key,
    ));
    harness.process_until(|inputs| any(inputs, is_cert1)).await;
}

/// A signed `Cert2` whose epoch is not the one its block number falls in is
/// not delivered, and does not stop a well-formed one for the same view.
#[tokio::test]
async fn test_malformed_certificate2_is_not_delivered() {
    let signers = TestData::new(2).await;
    let mut harness = TestHarness::new(0).await;
    let (view, epoch) = (ViewNumber::new(1), EpochNumber::genesis());
    let membership = harness
        .membership()
        .membership_for_epoch(Some(epoch))
        .expect("the genesis epoch resolves");
    let cert = |block| {
        let signer = &signers.views[0];
        build_cert2(
            Commitment::default_commitment_no_preimage(),
            epoch,
            block,
            &membership,
            view,
            &signer.leader_public_key,
            &signer.leader_private_key,
        )
    };
    let message = |cert, sender: &BLSPubKey| Message::<TestTypes, Validated> {
        sender: *sender,
        message_type: MessageType::Consensus(ConsensusMessage::Certificate2(cert, *sender)),
    };

    harness.message(message(
        cert(MALFORMED_BLOCK),
        &signers.views[0].leader_public_key,
    ));
    let inputs = harness.process_for(NO_CERT_WINDOW).await;
    assert!(
        !any(&inputs, is_cert2),
        "a malformed cert2 must not be delivered"
    );

    harness.message(message(
        cert(WELL_FORMED_BLOCK),
        &signers.views[1].leader_public_key,
    ));
    harness.process_until(|inputs| any(inputs, is_cert2)).await;
}

/// A signed, malformed `Cert1` sent as a high QC moves neither the view nor
/// the epoch, and does not stop a well-formed one for the same view.
///
/// The malformed one names the next epoch for a block of the genesis epoch, so
/// accepting it would move the epoch cursor as well as the view.
#[tokio::test]
async fn test_malformed_high_qc_moves_nothing() {
    let signers = TestData::new(2).await;
    let mut harness = TestHarness::new(0).await;
    let view = ViewNumber::new(1);
    let cert = |epoch: EpochNumber, block| {
        let membership = harness
            .membership()
            .membership_for_epoch(Some(epoch))
            .expect("the epoch resolves");
        let signer = &signers.views[0];
        build_cert1(
            Commitment::default_commitment_no_preimage(),
            epoch,
            block,
            &membership,
            view,
            &signer.leader_public_key,
            &signer.leader_private_key,
        )
    };
    let malformed = cert(EpochNumber::new(2), WELL_FORMED_BLOCK);
    let well_formed = cert(EpochNumber::genesis(), WELL_FORMED_BLOCK);
    let message = |qc, sender: &BLSPubKey| Message::<TestTypes, Validated> {
        sender: *sender,
        message_type: MessageType::Consensus(ConsensusMessage::HighQc(qc)),
    };
    let current = |harness: &TestHarness| {
        let consensus = harness.coordinator().consensus();
        (consensus.current_view(), consensus.current_epoch())
    };
    let start = current(&harness);

    harness.message(message(malformed, &signers.views[0].leader_public_key));
    harness.process_for(NO_CERT_WINDOW).await;
    assert_eq!(
        current(&harness),
        start,
        "a malformed cert1 must not move the view or the epoch"
    );

    harness.message(message(well_formed, &signers.views[1].leader_public_key));
    harness
        .process_until(|inputs| any(inputs, |i| matches!(i, ConsensusInput::AdvanceView(_))))
        .await;
    assert_eq!(
        harness.current_view(),
        view + 1,
        "the well-formed cert1 for the same view advances past it"
    );
}

/// A block in the second epoch, at the harness's epoch height of 10, so a
/// certificate naming the genesis epoch for it is malformed while its epoch
/// still resolves.
const MALFORMED_BLOCK: u64 = 11;

/// A block in the genesis epoch.
const WELL_FORMED_BLOCK: u64 = 2;
