//! The epoch boundary under the certificate rule, and its recovery by re-vote.
//!
//! The last block `L` of epoch 1 is at view 10. The first block of epoch 2
//! needs a `Certificate2` over `L`. Under the certificate rule a node never
//! votes2 at a view it has sent a timeout vote for, so if `L`'s `Certificate1`
//! arrives after the nodes timed view 10 out, that `Certificate2` can never
//! form at view 10 (problem 3 of `epoch-boundary-stall.typ`). The leader of a
//! later view of epoch 1 asks for a re-vote instead: the committee votes1 and
//! then votes2 on `L`'s own vote data again, at that view, and the resulting
//! `Certificate2` commits `L`.

use std::collections::BTreeSet;

use hotshot::types::{BLSPubKey, SignatureKey};
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    message::UpgradeLock,
    simple_certificate::TimeoutEvidence,
    simple_vote::{HasEpoch, LockView},
    traits::block_contents::BlockHeader,
    utils::is_epoch_transition,
    vote::HasViewNumber,
};

use super::common::utils::{
    ConsensusHarness, TEST_DRB_RESULT, TestData, build_cert1, build_cert2,
    build_timeout_cert3_with_lock,
};
use crate::{
    cert_verifier::ValidCert,
    consensus::{ConsensusInput, ConsensusOutput, certificate_rule_admits},
    helpers::{proposal_commitment, test_timeout_epoch_lock},
    message::{
        Certificate1, Certificate2, EpochChangeMessage, ProposalMessage, ReVote, ReVoteMessage,
        TimeoutVote,
    },
};

const EPOCH_HEIGHT: u64 = 10;
/// Index into `TestData::views` of view 10, the last block of epoch 1.
const BOUNDARY: usize = 9;

fn view(v: u64) -> ViewNumber {
    ViewNumber::new(v)
}

fn epoch(e: u64) -> EpochNumber {
    EpochNumber::new(e)
}

/// The index of the test key that leads `view` in `epoch`.
fn leader_index(harness: &ConsensusHarness, view: ViewNumber, epoch: EpochNumber) -> u64 {
    let leader = harness
        .consensus
        .leader_of(view, epoch)
        .expect("the view has a leader");
    (0..10)
        .find(|i| BLSPubKey::generated_from_seed_indexed([0u8; 32], *i).0 == leader)
        .expect("the leader is a test key")
}

/// A node, run under the certificate rule, through views 1..=9 of epoch 1,
/// with the boundary block at view 10 proposed, voted on and paired with its
/// payload, but without its `Certificate1`.
async fn node_before_the_boundary_certificate(
    node_index: u64,
    test_data: &TestData,
    lock: UpgradeLock<TestTypes>,
) -> ConsensusHarness {
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], node_index).0;
    let mut harness = ConsensusHarness::new_with_upgrade_lock(node_index, EPOCH_HEIGHT, lock).await;
    let mut drb_epochs = BTreeSet::new();
    for v in &test_data.views[..=BOUNDARY] {
        let block = BlockHeader::<TestTypes>::block_number(&v.proposal.data.block_header);
        if is_epoch_transition(block, EPOCH_HEIGHT) {
            drb_epochs.insert(v.epoch_number + 1);
        }
    }
    for e in drb_epochs {
        harness
            .apply(ConsensusInput::DrbResult(e, TEST_DRB_RESULT))
            .await;
    }
    for v in &test_data.views[..BOUNDARY] {
        harness
            .apply_pair(v.proposal_input_consensus(&node_key))
            .await;
        harness.apply(v.block_reconstructed_input()).await;
        harness.apply(v.cert1_input()).await;
        harness.apply(v.cert2_input()).await;
    }
    let boundary = &test_data.views[BOUNDARY];
    harness
        .apply_pair(boundary.proposal_input_consensus(&node_key))
        .await;
    harness.apply(boundary.block_reconstructed_input()).await;
    harness
}

fn vote2_views(harness: &ConsensusHarness) -> Vec<ViewNumber> {
    harness
        .outputs()
        .iter()
        .filter_map(|o| match o {
            ConsensusOutput::SendVote2(vote) => Some(vote.view_number()),
            _ => None,
        })
        .collect()
}

fn vote1_at(
    harness: &ConsensusHarness,
    v: ViewNumber,
) -> Option<&crate::message::Vote1<TestTypes>> {
    harness.outputs().iter().find_map(|o| match o {
        ConsensusOutput::SendVote1(vote) if vote.vote.view_number() == v => Some(vote),
        _ => None,
    })
}

fn last_timeout_vote(harness: &ConsensusHarness) -> Option<&TimeoutVote<TestTypes>> {
    harness
        .outputs()
        .iter()
        .filter_map(|o| match o {
            ConsensusOutput::SendTimeoutVote(vote, ..) => Some(vote),
            _ => None,
        })
        .last()
}

/// The timeout certificate of epoch 1 for `timed_out`, its signers all locked
/// on `lock`.
fn timeout_cert(
    harness: &ConsensusHarness,
    timed_out: ViewNumber,
    lock: Option<Certificate1<TestTypes>>,
) -> TimeoutEvidence<TestTypes> {
    timeout_cert_of(harness, epoch(1), timed_out, lock)
}

/// The timeout certificate of `e` for `timed_out`, its signers all locked on
/// `lock`.
fn timeout_cert_of(
    harness: &ConsensusHarness,
    e: EpochNumber,
    timed_out: ViewNumber,
    lock: Option<Certificate1<TestTypes>>,
) -> TimeoutEvidence<TestTypes> {
    let membership = harness
        .membership_coordinator
        .membership_for_epoch(Some(e))
        .unwrap();
    TimeoutEvidence::V3(build_timeout_cert3_with_lock(
        timed_out,
        e,
        lock,
        &membership,
    ))
}

fn timeout_cert_input(tc: TimeoutEvidence<TestTypes>) -> ConsensusInput<TestTypes> {
    let e = HasEpoch::epoch(&tc).expect("a lock-carrying certificate names its epoch");
    ConsensusInput::TimeoutCertificate(ValidCert::new(tc, e))
}

/// Problem 3: a `Certificate1` arriving after the timeout leaves the boundary
/// block without a commit at its own view, and a re-vote commits it.
#[tokio::test]
async fn late_certificate1_at_the_boundary_is_committed_by_a_revote() {
    let test_data = TestData::new_with_epoch_height(BOUNDARY + 1, EPOCH_HEIGHT).await;
    let boundary = &test_data.views[BOUNDARY];
    let leaf = proposal_commitment(&boundary.proposal.data);
    let mut harness =
        node_before_the_boundary_certificate(0, &test_data, test_timeout_epoch_lock()).await;
    assert!(
        vote1_at(&harness, view(10)).is_some(),
        "setup: the node votes1 for the boundary block"
    );

    // Everyone times view 10 out before the block's certificate arrives.
    harness.apply(ConsensusInput::Timeout(view(10))).await;
    let vote = last_timeout_vote(&harness).expect("a timeout vote");
    let TimeoutVote::V3(vote) = vote else {
        panic!("the certificate rule signs the lock into the timeout vote");
    };
    assert_eq!(
        vote.vote().data.lock,
        Some(LockView::of(&test_data.views[BOUNDARY - 1].cert1)),
        "the timeout vote names the lock the node holds"
    );
    let lock_before = test_data.views[BOUNDARY - 1].cert1.clone();
    harness
        .apply(timeout_cert_input(timeout_cert(
            &harness,
            view(10),
            Some(lock_before),
        )))
        .await;
    assert_eq!(harness.consensus.current_view(), view(11));

    // The certificate arrives late: the node locks on the block but must not
    // vote2 at a view it timed out.
    harness.apply(boundary.cert1_input()).await;
    assert_eq!(
        harness.consensus.lock_view().map(|lock| lock.view),
        Some(view(10)),
        "the node locks on the late certificate"
    );
    assert!(
        !vote2_views(&harness).contains(&view(10)),
        "no vote2 at a view the node timed out"
    );

    // View 11 fails as well; the leader of view 12 asks for a re-vote.
    harness.apply(ConsensusInput::Timeout(view(11))).await;
    let tc11 = timeout_cert(&harness, view(11), Some(boundary.cert1.clone()));
    let TimeoutEvidence::V3(timeout) = tc11 else {
        unreachable!()
    };
    let leader = leader_index(&harness, view(12), epoch(1));
    let (_, leader_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], leader);
    let revote = ReVoteMessage::new(
        ReVote {
            view: view(12),
            epoch: epoch(1),
            cert1: boundary.cert1.clone(),
            timeout: Some(timeout),
        },
        &leader_key,
    )
    .unwrap();
    harness.apply(ConsensusInput::ReVote(revote)).await;

    let revote1 = vote1_at(&harness, view(12)).expect("the node votes1 on the re-vote");
    assert_eq!(
        revote1.vote.data, boundary.cert1.data,
        "on the block's own vote data"
    );

    // The re-vote's Certificate1 locks the node at view 12, and it votes2 there.
    let membership = harness
        .membership_coordinator
        .membership_for_epoch(Some(epoch(1)))
        .unwrap();
    let (pk, sk) = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0);
    let block = BlockHeader::<TestTypes>::block_number(&boundary.proposal.data.block_header);
    let cert1_12 = build_cert1(leaf, epoch(1), block, &membership, view(12), &pk, &sk);
    harness
        .apply(ConsensusInput::Certificate1(ValidCert::new(
            cert1_12.clone(),
            epoch(1),
        )))
        .await;
    assert_eq!(
        harness.consensus.lock_view().map(|lock| lock.view),
        Some(view(12))
    );
    assert!(
        vote2_views(&harness).contains(&view(12)),
        "the node votes2 on the re-vote"
    );

    // Its Certificate2 commits the boundary block, and the epoch changes.
    let cert2_12 = build_cert2(leaf, epoch(1), block, &membership, view(12), &pk, &sk);
    harness
        .apply(ConsensusInput::Certificate2(ValidCert::new(
            cert2_12.clone(),
            epoch(1),
        )))
        .await;
    let decided = harness.outputs().iter().find_map(|o| match o {
        ConsensusOutput::LeafDecided { leaves, cert2, .. }
            if leaves.first().is_some_and(|l| l.view_number() == view(10)) =>
        {
            Some(cert2.clone())
        },
        _ => None,
    });
    assert_eq!(
        decided.expect("the boundary block is decided"),
        Some(cert2_12.clone()),
        "by the re-vote's commit"
    );
    let epoch_change = harness
        .outputs()
        .iter()
        .find_map(|o| match o {
            ConsensusOutput::SendEpochChange(change) => Some(change.clone()),
            _ => None,
        })
        .expect("the epoch change is sent");
    assert_eq!(
        epoch_change.cert1.view_number(),
        view(10),
        "with the block's own Cert1"
    );
    assert_eq!(epoch_change.cert2, cert2_12, "and the re-vote's Cert2");
    assert!(epoch_change.well_formed(EPOCH_HEIGHT).is_ok());
}

/// The leader side: a leader of epoch 1 entering a view by timeout, locked on
/// the boundary block without a commit of it, sends a re-vote request in
/// place of a proposal.
#[tokio::test]
async fn leader_requests_a_revote_for_an_uncommitted_boundary_block() {
    let test_data = TestData::new_with_epoch_height(BOUNDARY + 1, EPOCH_HEIGHT).await;
    let boundary = &test_data.views[BOUNDARY];
    let probe =
        ConsensusHarness::new_with_upgrade_lock(0, EPOCH_HEIGHT, test_timeout_epoch_lock()).await;
    let leader = leader_index(&probe, view(12), epoch(1));
    let mut harness =
        node_before_the_boundary_certificate(leader, &test_data, test_timeout_epoch_lock()).await;

    harness.apply(ConsensusInput::Timeout(view(10))).await;
    let lock_before = test_data.views[BOUNDARY - 1].cert1.clone();
    harness
        .apply(timeout_cert_input(timeout_cert(
            &harness,
            view(10),
            Some(lock_before),
        )))
        .await;
    harness.apply(boundary.cert1_input()).await;
    harness.apply(ConsensusInput::Timeout(view(11))).await;
    harness
        .apply(timeout_cert_input(timeout_cert(
            &harness,
            view(11),
            Some(boundary.cert1.clone()),
        )))
        .await;

    let request = harness
        .outputs()
        .iter()
        .find_map(|o| match o {
            ConsensusOutput::SendReVote(request) => Some(request.clone()),
            _ => None,
        })
        .expect("the leader sends a re-vote request");
    assert_eq!(request.revote.view, view(12));
    assert_eq!(request.revote.epoch, epoch(1));
    assert_eq!(request.revote.cert1, boundary.cert1);
    assert!(request.revote.well_formed(EPOCH_HEIGHT).is_ok());
    let (leader_key, _) = BLSPubKey::generated_from_seed_indexed([0u8; 32], leader);
    assert!(request.signed_by(&leader_key));
    assert!(
        !harness.outputs().iter().any(|o| matches!(
            o,
            ConsensusOutput::SendProposal(p) if p.data.view_number == view(12)
        )),
        "and no proposal for the same view"
    );
}

/// The leader of the view after the boundary block asks for a re-vote as soon
/// as it can lock on the block, without waiting for the view to time out.
#[tokio::test]
async fn leader_requests_a_revote_in_the_view_after_the_boundary() {
    let test_data = TestData::new_with_epoch_height(BOUNDARY + 1, EPOCH_HEIGHT).await;
    let boundary = &test_data.views[BOUNDARY];
    let probe =
        ConsensusHarness::new_with_upgrade_lock(0, EPOCH_HEIGHT, test_timeout_epoch_lock()).await;
    let leader = leader_index(&probe, view(11), epoch(1));
    let mut harness =
        node_before_the_boundary_certificate(leader, &test_data, test_timeout_epoch_lock()).await;
    harness.apply(boundary.cert1_input()).await;

    let request = harness
        .outputs()
        .iter()
        .find_map(|o| match o {
            ConsensusOutput::SendReVote(request) => Some(request.clone()),
            _ => None,
        })
        .expect("the leader sends a re-vote request");
    assert_eq!(request.revote.view, view(11));
    assert_eq!(request.revote.epoch, epoch(1));
    assert_eq!(request.revote.cert1, boundary.cert1);
    assert!(request.revote.timeout.is_none());
    assert!(request.revote.well_formed(EPOCH_HEIGHT).is_ok());
}

/// The outgoing committee's re-vote and the incoming committee's first block
/// share the view after the boundary block, and a node votes1 on each: votes
/// are counted per epoch and view.
#[tokio::test]
async fn revote_and_first_block_share_a_view() {
    let test_data = TestData::new_with_epoch_height(BOUNDARY + 2, EPOCH_HEIGHT).await;
    let boundary = &test_data.views[BOUNDARY];
    let first = &test_data.views[BOUNDARY + 1];
    let mut harness =
        node_before_the_boundary_certificate(0, &test_data, test_timeout_epoch_lock()).await;
    harness.apply(boundary.cert1_input()).await;

    let leader = leader_index(&harness, view(11), epoch(1));
    let (_, leader_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], leader);
    let revote = ReVoteMessage::new(
        ReVote {
            view: view(11),
            epoch: epoch(1),
            cert1: boundary.cert1.clone(),
            timeout: None,
        },
        &leader_key,
    )
    .unwrap();
    harness.apply(ConsensusInput::ReVote(revote)).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;
    harness
        .apply_pair(first.proposal_input_consensus(&node_key))
        .await;

    let epochs: Vec<EpochNumber> = harness
        .outputs()
        .iter()
        .filter_map(|o| match o {
            ConsensusOutput::SendVote1(vote) if vote.vote.view_number() == view(11) => {
                Some(vote.vote.data.epoch.expect("an epoch"))
            },
            _ => None,
        })
        .collect();
    assert_eq!(epochs, [epoch(1), epoch(2)]);
}

/// Rule B.3: after a timeout, the parent is checked against the lock the
/// timeout certificate carries, not against the voter's own lock.
#[tokio::test]
async fn certificate_rule_checks_the_timeout_certificates_lock() {
    let test_data = TestData::new_with_epoch_height(6, EPOCH_HEIGHT).await;
    let harness =
        ConsensusHarness::new_with_upgrade_lock(0, EPOCH_HEIGHT, test_timeout_epoch_lock()).await;
    let parent = &test_data.views[3].cert1;

    let no_later = timeout_cert(&harness, view(6), Some(parent.clone()));
    assert!(
        certificate_rule_admits(parent, &no_later, epoch(1)),
        "a parent no earlier than the certificate's lock is admitted"
    );

    let later = timeout_cert(&harness, view(6), Some(test_data.views[4].cert1.clone()));
    assert!(
        !certificate_rule_admits(parent, &later, epoch(1)),
        "a parent earlier than the certificate's lock is refused"
    );
    assert!(
        !certificate_rule_admits(&test_data.views[4].cert1, &later, epoch(2)),
        "and so is any proposal of another epoch than the certificate's"
    );

    // A parent over the lock's block passes at any view: a re-vote certifies
    // the block again later, and the next epoch names the block's own cert.
    let mut revoted = test_data.views[4].cert1.clone();
    revoted.view_number = view(5);
    let revote_lock = timeout_cert(&harness, view(6), Some(revoted));
    assert!(certificate_rule_admits(
        &test_data.views[4].cert1,
        &revote_lock,
        epoch(1)
    ));
}

/// Rule B.1 across a restart: a node restarted after recording a timeout vote
/// does not vote2 at that view.
#[tokio::test]
async fn restarted_node_does_not_vote2_at_a_view_it_timed_out() {
    let test_data = TestData::new_with_epoch_height(3, EPOCH_HEIGHT).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;
    let mut harness =
        ConsensusHarness::new_with_upgrade_lock(0, EPOCH_HEIGHT, test_timeout_epoch_lock()).await;
    harness
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness
        .apply(test_data.views[0].block_reconstructed_input())
        .await;
    // The node timed view 1 out before it went down; its timeout action is
    // the last one recorded.
    harness
        .consensus
        .resume_from_restart(ViewNumber::genesis(), view(1), view(1));
    harness.apply(test_data.views[0].cert1_input()).await;
    assert_eq!(
        harness.consensus.lock_view().map(|lock| lock.view),
        Some(view(1))
    );
    assert!(
        !vote2_views(&harness).contains(&view(1)),
        "no vote2 at a view timed out before the restart"
    );
}

/// Rule 4: a node does not vote for an epoch earlier than its own, so a
/// re-vote of epoch 1 cannot take its vote1 in a view of epoch 2.
#[tokio::test]
async fn no_revote_once_in_the_next_epoch() {
    let test_data = TestData::new_with_epoch_height(BOUNDARY + 1, EPOCH_HEIGHT).await;
    let boundary = &test_data.views[BOUNDARY];
    let mut harness =
        node_before_the_boundary_certificate(0, &test_data, test_timeout_epoch_lock()).await;
    harness.apply(boundary.cert1_input()).await;
    harness
        .apply(ConsensusInput::EpochChange(EpochChangeMessage::validated(
            boundary.cert1.clone(),
            boundary.cert2.clone(),
            boundary.proposal.data.clone(),
        )))
        .await;
    assert_eq!(harness.consensus.current_epoch(), Some(epoch(2)));

    let TimeoutEvidence::V3(timeout) =
        timeout_cert(&harness, view(13), Some(boundary.cert1.clone()))
    else {
        unreachable!()
    };
    let leader = leader_index(&harness, view(14), epoch(1));
    let (_, leader_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], leader);
    let revote = ReVoteMessage::new(
        ReVote {
            view: view(14),
            epoch: epoch(1),
            cert1: boundary.cert1.clone(),
            timeout: Some(timeout),
        },
        &leader_key,
    )
    .unwrap();
    harness.apply(ConsensusInput::ReVote(revote)).await;
    assert!(
        vote1_at(&harness, view(14)).is_none(),
        "a node of epoch 2 does not vote on a re-vote of epoch 1"
    );
}

/// The boundary block's `Certificate1` or `Certificate2` at a re-vote's `view`.
fn revote_certs(
    harness: &ConsensusHarness,
    test_data: &TestData,
    v: ViewNumber,
) -> (Certificate1<TestTypes>, Certificate2<TestTypes>) {
    let boundary = &test_data.views[BOUNDARY];
    let leaf = proposal_commitment(&boundary.proposal.data);
    let block = BlockHeader::<TestTypes>::block_number(&boundary.proposal.data.block_header);
    let membership = harness
        .membership_coordinator
        .membership_for_epoch(Some(epoch(1)))
        .unwrap();
    let (pk, sk) = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0);
    (
        build_cert1(leaf, epoch(1), block, &membership, v, &pk, &sk),
        build_cert2(leaf, epoch(1), block, &membership, v, &pk, &sk),
    )
}

/// A node at the start of epoch 2: the boundary block committed at its own
/// view, the epoch changed, and the first block of epoch 2 (view 11) proposed
/// and paired, with its payload.
async fn node_at_the_first_block(node_index: u64, test_data: &TestData) -> ConsensusHarness {
    let boundary = &test_data.views[BOUNDARY];
    let mut harness =
        node_before_the_boundary_certificate(node_index, test_data, test_timeout_epoch_lock())
            .await;
    harness.apply(boundary.cert1_input()).await;
    harness
        .apply(ConsensusInput::EpochChange(EpochChangeMessage::validated(
            boundary.cert1.clone(),
            boundary.cert2.clone(),
            boundary.proposal.data.clone(),
        )))
        .await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], node_index).0;
    let first = &test_data.views[BOUNDARY + 1];
    harness
        .apply_pair(first.proposal_input_consensus(&node_key))
        .await;
    harness.apply(first.block_reconstructed_input()).await;
    harness
}

/// A second first block of epoch 2, built on the boundary block's own
/// certificate at view 10 with a commit of it from a re-vote at view 20, is
/// malformed at view 21 without a timeout certificate.
///
/// Such a block would follow a re-vote by nodes of epoch 1 that never saw the
/// boundary block's first commit, and skip the committed first block. A
/// timeout certificate for the view before is what compares its parent with
/// the locks of the nodes that committed.
#[tokio::test]
async fn a_first_block_skipping_views_without_a_timeout_is_malformed() {
    let test_data = TestData::new_with_epoch_height(BOUNDARY + 2, EPOCH_HEIGHT).await;
    let harness =
        ConsensusHarness::new_with_upgrade_lock(0, EPOCH_HEIGHT, test_timeout_epoch_lock()).await;
    let (_, cert2_20) = revote_certs(&harness, &test_data, view(20));
    let mut forked = test_data.views[BOUNDARY + 1].proposal.data.clone();
    forked.view_number = view(21);
    forked.next_epoch_justify_qc = Some(cert2_20);
    assert!(matches!(
        crate::proposal::well_formed(&forked, EPOCH_HEIGHT),
        Err(crate::proposal::MalformedProposal::ViewChangeEvidenceMissing(v)) if v == view(21)
    ));
}

/// A certificate of the next epoch supersedes the outgoing epoch's re-vote
/// certificate at the same view, whichever arrives first.
#[tokio::test]
async fn later_epochs_certificate_supersedes_a_revotes_at_the_same_view() {
    let test_data = TestData::new_with_epoch_height(BOUNDARY + 2, EPOCH_HEIGHT).await;
    let first = &test_data.views[BOUNDARY + 1];
    for revote_first in [true, false] {
        let mut harness = node_at_the_first_block(0, &test_data).await;
        let (mut revote_cert, _) = revote_certs(&harness, &test_data, view(11));
        revote_cert.view_number = view(11);
        let inputs = [
            ConsensusInput::Certificate1(ValidCert::new(revote_cert, epoch(1))),
            first.cert1_input(),
        ];
        let inputs = if revote_first {
            inputs
        } else {
            let [a, b] = inputs;
            [b, a]
        };
        for input in inputs {
            harness.apply(input).await;
        }
        assert_eq!(
            harness.consensus.cert1_at(view(11)),
            Some(&first.cert1),
            "the epoch 2 certificate is kept (re-vote first: {revote_first})"
        );
        assert_eq!(
            harness.consensus.lock_view().map(|lock| lock.view),
            Some(view(11))
        );
    }
}

/// Rule 4 for vote2: a node in epoch 2 does not vote2 on a re-vote of epoch 1.
#[tokio::test]
async fn no_vote2_on_a_revote_once_in_the_next_epoch() {
    let test_data = TestData::new_with_epoch_height(BOUNDARY + 2, EPOCH_HEIGHT).await;
    let mut harness = node_at_the_first_block(0, &test_data).await;
    assert_eq!(harness.consensus.current_epoch(), Some(epoch(2)));
    let (cert1_20, _) = revote_certs(&harness, &test_data, view(20));
    harness
        .apply(ConsensusInput::Certificate1(ValidCert::new(
            cert1_20,
            epoch(1),
        )))
        .await;
    assert!(
        !vote2_views(&harness).contains(&view(20)),
        "no vote2 for an epoch earlier than the node's own"
    );
}

/// A node restarted with a lock of a later epoch than its decided anchor is in
/// its lock's epoch.
#[tokio::test]
async fn restored_lock_sets_the_epoch() {
    let test_data = TestData::new_with_epoch_height(BOUNDARY + 2, EPOCH_HEIGHT).await;
    let mut harness =
        ConsensusHarness::new_with_upgrade_lock(0, EPOCH_HEIGHT, test_timeout_epoch_lock()).await;
    assert_eq!(harness.consensus.current_epoch(), Some(epoch(1)));
    harness
        .consensus
        .seed_locked_cert(test_data.views[BOUNDARY + 1].cert1.clone());
    assert_eq!(harness.consensus.current_epoch(), Some(epoch(2)));
}

/// The leader of the view after a re-vote's commit proposes the first block
/// of epoch 2 once that view times out, knowing the commit only from the
/// epoch change.
///
/// The first block names the boundary block's own certificate, and a commit
/// at a later view than that certificate skips views: only a timeout
/// certificate for the view before covers them.
#[tokio::test]
async fn first_block_after_a_revote_is_proposed() {
    let test_data = TestData::new_with_epoch_height(BOUNDARY + 1, EPOCH_HEIGHT).await;
    let boundary = &test_data.views[BOUNDARY];
    let probe =
        ConsensusHarness::new_with_upgrade_lock(0, EPOCH_HEIGHT, test_timeout_epoch_lock()).await;
    let leader = leader_index(&probe, view(12), epoch(2));
    let mut harness =
        node_before_the_boundary_certificate(leader, &test_data, test_timeout_epoch_lock()).await;
    harness.apply(boundary.cert1_input()).await;
    let (_, cert2_11) = revote_certs(&harness, &test_data, view(11));
    harness
        .apply(ConsensusInput::EpochChange(EpochChangeMessage::validated(
            boundary.cert1.clone(),
            cert2_11.clone(),
            boundary.proposal.data.clone(),
        )))
        .await;
    assert!(
        !harness.outputs().iter().any(|o| matches!(
            o,
            ConsensusOutput::SendProposal(p) if p.data.view_number == view(12)
        )),
        "no first block in the view after the commit without a timeout"
    );

    let tc = timeout_cert_of(&harness, epoch(2), view(11), Some(boundary.cert1.clone()));
    harness.apply(timeout_cert_input(tc)).await;
    let proposal = harness
        .outputs()
        .iter()
        .find_map(|o| match o {
            ConsensusOutput::SendProposal(p) if p.data.view_number == view(12) => {
                Some(p.data.clone())
            },
            _ => None,
        })
        .expect("the leader proposes the first block of epoch 2");
    assert_eq!(proposal.epoch, epoch(2));
    assert_eq!(
        proposal.justify_qc, boundary.cert1,
        "it names the boundary block's own certificate"
    );
    assert_eq!(proposal.next_epoch_justify_qc, Some(cert2_11));
    assert!(proposal.view_change_evidence.is_some());
    assert!(crate::proposal::well_formed(&proposal, EPOCH_HEIGHT).is_ok());
}

/// A fetch by a re-vote certificate's view is answered with the block it
/// certifies, which a node holding only the certificate cannot ask for by
/// its own view.
#[tokio::test]
async fn fetch_by_a_revote_view_returns_the_boundary_block() {
    let test_data = TestData::new_with_epoch_height(BOUNDARY + 1, EPOCH_HEIGHT).await;
    let boundary = &test_data.views[BOUNDARY];
    let mut harness =
        node_before_the_boundary_certificate(0, &test_data, test_timeout_epoch_lock()).await;
    let (cert1_12, _) = revote_certs(&harness, &test_data, view(12));
    harness
        .apply(ConsensusInput::Certificate1(ValidCert::new(
            cert1_12,
            epoch(1),
        )))
        .await;
    let served = harness.consensus.signed_proposals_for_fetch(&view(12));
    assert_eq!(
        served.iter().map(|p| &p.data).collect::<Vec<_>>(),
        [&boundary.proposal.data],
        "the boundary block is served for the re-vote's view"
    );
    assert!(
        harness
            .consensus
            .signed_proposals_for_fetch(&view(13))
            .is_empty()
    );
}

/// A re-vote's commit at a view where the next epoch has a block of its own
/// does not keep that block from being decided.
#[tokio::test]
async fn revote_commit_at_a_view_does_not_hide_the_next_epochs_block_there() {
    let test_data = TestData::new_with_epoch_height(BOUNDARY + 2, EPOCH_HEIGHT).await;
    let first = &test_data.views[BOUNDARY + 1];
    let mut harness = node_at_the_first_block(0, &test_data).await;
    // A late re-vote of epoch 1 commits the boundary block, already decided,
    // again at view 11, where the first block of epoch 2 is.
    let (_, cert2_11) = revote_certs(&harness, &test_data, view(11));
    harness
        .apply(ConsensusInput::Certificate2(ValidCert::new(
            cert2_11,
            epoch(1),
        )))
        .await;
    harness.apply(first.cert1_input()).await;
    harness.apply(first.cert2_input()).await;
    assert!(
        harness.outputs().iter().any(|o| matches!(
            o,
            ConsensusOutput::LeafDecided { leaves, .. }
                if leaves.first().is_some_and(|l| l.view_number() == view(11))
        )),
        "the first block of epoch 2 is decided"
    );
}

/// A node holding the boundary block and a re-vote's `Certificate2` over it,
/// but neither `Certificate1`, decides the block once its own `Certificate1`
/// arrives, and sends the epoch change.
#[tokio::test]
async fn own_certificate1_after_a_revotes_certificate2_decides_the_boundary_block() {
    let test_data = TestData::new_with_epoch_height(BOUNDARY + 1, EPOCH_HEIGHT).await;
    let boundary = &test_data.views[BOUNDARY];
    let mut harness =
        node_before_the_boundary_certificate(0, &test_data, test_timeout_epoch_lock()).await;
    let decided_boundary = |harness: &ConsensusHarness| {
        harness.outputs().iter().any(|o| {
            matches!(
                o,
                ConsensusOutput::LeafDecided { leaves, .. }
                    if leaves.first().is_some_and(|l| l.view_number() == view(10))
            )
        })
    };

    let (_, cert2_12) = revote_certs(&harness, &test_data, view(12));
    harness
        .apply(ConsensusInput::Certificate2(ValidCert::new(
            cert2_12,
            epoch(1),
        )))
        .await;
    assert!(!decided_boundary(&harness), "no Certificate1 is held yet");

    harness.apply(boundary.cert1_input()).await;
    assert!(decided_boundary(&harness), "the boundary block is decided");
    assert!(
        harness
            .outputs()
            .iter()
            .any(|o| matches!(o, ConsensusOutput::SendEpochChange(_))),
        "the epoch change is sent"
    );
}

/// A node lacking both the re-vote's block and its payload fetches both.
#[tokio::test]
async fn revote_fetches_the_block_and_then_its_payload() {
    let test_data = TestData::new_with_epoch_height(BOUNDARY + 1, EPOCH_HEIGHT).await;
    let boundary = &test_data.views[BOUNDARY];
    // Not the leader of view 10, which would hold the payload it built.
    const NODE: u64 = 5;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], NODE).0;
    let mut harness =
        ConsensusHarness::new_with_upgrade_lock(NODE, EPOCH_HEIGHT, test_timeout_epoch_lock())
            .await;
    let mut drb_epochs = BTreeSet::new();
    for v in &test_data.views[..=BOUNDARY] {
        let block = BlockHeader::<TestTypes>::block_number(&v.proposal.data.block_header);
        if is_epoch_transition(block, EPOCH_HEIGHT) {
            drb_epochs.insert(v.epoch_number + 1);
        }
    }
    for e in drb_epochs {
        harness
            .apply(ConsensusInput::DrbResult(e, TEST_DRB_RESULT))
            .await;
    }
    for v in &test_data.views[..BOUNDARY] {
        harness
            .apply_pair(v.proposal_input_consensus(&node_key))
            .await;
        harness.apply(v.block_reconstructed_input()).await;
        harness.apply(v.cert1_input()).await;
        harness.apply(v.cert2_input()).await;
    }
    // The node never saw the boundary block. A re-vote on it arrives.
    let TimeoutEvidence::V3(timeout) =
        timeout_cert(&harness, view(11), Some(boundary.cert1.clone()))
    else {
        unreachable!()
    };
    let leader = leader_index(&harness, view(12), epoch(1));
    let (_, leader_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], leader);
    let revote = ReVoteMessage::new(
        ReVote {
            view: view(12),
            epoch: epoch(1),
            cert1: boundary.cert1.clone(),
            timeout: Some(timeout),
        },
        &leader_key,
    )
    .unwrap();
    harness.apply(ConsensusInput::ReVote(revote)).await;
    let requested_block = harness.outputs().iter().any(
        |o| matches!(o, ConsensusOutput::RequestMissingProposal { view: v, .. } if *v == view(10)),
    );
    assert!(requested_block, "the node fetches the block");

    harness
        .apply(ConsensusInput::FetchedProposal(boundary.proposal_message()))
        .await;
    let requested_payload = harness.outputs().iter().any(
        |o| matches!(o, ConsensusOutput::RequestMissingPayload { view: v, .. } if *v == view(10)),
    );
    assert!(requested_payload, "and then its payload");

    harness.apply(boundary.block_reconstructed_input()).await;
    assert!(
        vote1_at(&harness, view(12)).is_some(),
        "and votes once it has both"
    );
}

/// The leader of the view after a re-vote's commit does not propose the first
/// block of epoch 2 on a timeout certificate of epoch 1, which voters of
/// epoch 2 would refuse, nor without one, which would skip views.
#[tokio::test]
async fn first_block_after_a_revote_drops_a_timeout_certificate_of_the_old_epoch() {
    let test_data = TestData::new_with_epoch_height(BOUNDARY + 1, EPOCH_HEIGHT).await;
    let boundary = &test_data.views[BOUNDARY];
    let probe =
        ConsensusHarness::new_with_upgrade_lock(0, EPOCH_HEIGHT, test_timeout_epoch_lock()).await;
    let leader = leader_index(&probe, view(13), epoch(2));
    // The leader lacks the boundary block's payload, so on the timeout
    // certificate it can neither re-vote nor build on its earlier lock, which
    // the certificate's lock refuses: it has not acted in view 13 when the
    // commit arrives.
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], leader).0;
    let mut harness =
        ConsensusHarness::new_with_upgrade_lock(leader, EPOCH_HEIGHT, test_timeout_epoch_lock())
            .await;
    for v in &test_data.views[..BOUNDARY] {
        let block = BlockHeader::<TestTypes>::block_number(&v.proposal.data.block_header);
        if is_epoch_transition(block, EPOCH_HEIGHT) {
            harness
                .apply(ConsensusInput::DrbResult(
                    v.epoch_number + 1,
                    TEST_DRB_RESULT,
                ))
                .await;
        }
        harness
            .apply_pair(v.proposal_input_consensus(&node_key))
            .await;
        harness.apply(v.block_reconstructed_input()).await;
        harness.apply(v.cert1_input()).await;
        harness.apply(v.cert2_input()).await;
    }
    harness
        .apply(ConsensusInput::DrbResult(epoch(2), TEST_DRB_RESULT))
        .await;
    harness
        .apply_pair(boundary.proposal_input_consensus(&node_key))
        .await;
    harness.apply(boundary.cert1_input()).await;
    harness
        .apply(timeout_cert_input(timeout_cert(
            &harness,
            view(12),
            Some(boundary.cert1.clone()),
        )))
        .await;
    assert!(
        !harness.outputs().iter().any(|o| match o {
            ConsensusOutput::SendReVote(r) => r.revote.view == view(13),
            ConsensusOutput::SendProposal(p) => p.data.view_number == view(13),
            _ => false,
        }),
        "setup: the leader has not acted in view 13"
    );
    let (_, cert2_12) = revote_certs(&harness, &test_data, view(12));
    harness
        .apply(ConsensusInput::EpochChange(EpochChangeMessage::validated(
            boundary.cert1.clone(),
            cert2_12.clone(),
            boundary.proposal.data.clone(),
        )))
        .await;
    assert!(
        !harness.outputs().iter().any(|o| matches!(
            o,
            ConsensusOutput::SendProposal(p) if p.data.view_number == view(13)
        )),
        "the leader does not propose in view 13"
    );
}

/// After a timeout, a node locked in epoch 2 still votes for a first block on
/// the boundary block whose timeout certificate admits it: the certificate's
/// lock, not the voter's own, decides there. A lock released to the node
/// late cannot then make it refuse honest proposals.
#[tokio::test]
async fn late_lock_of_the_next_epoch_does_not_refuse_an_admitted_first_block() {
    let test_data = TestData::new_with_epoch_height(BOUNDARY + 2, EPOCH_HEIGHT).await;
    let boundary = &test_data.views[BOUNDARY];
    let first = &test_data.views[BOUNDARY + 1];
    let mut harness = node_at_the_first_block(0, &test_data).await;
    harness.apply(first.cert1_input()).await;
    assert_eq!(
        harness.consensus.lock_view().map(|lock| lock.view),
        Some(view(11))
    );

    // View 11 timed out under epoch 2 with the boundary block as the lock.
    let membership = harness
        .membership_coordinator
        .membership_for_epoch(Some(epoch(2)))
        .unwrap();
    let tc = TimeoutEvidence::V3(build_timeout_cert3_with_lock(
        view(11),
        epoch(2),
        Some(boundary.cert1.clone()),
        &membership,
    ));
    let mut again = first.proposal.clone();
    again.data.view_number = view(12);
    again.data.view_change_evidence = Some(tc.clone());
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;
    let mut share = first.vid_share_for(&node_key);
    share.view_number = view(12);
    harness
        .apply(ConsensusInput::TimeoutCertificate(ValidCert::new(
            tc,
            epoch(2),
        )))
        .await;
    harness
        .apply_pair((
            ConsensusInput::Proposal(first.leader_public_key, ProposalMessage::validated(again)),
            ConsensusInput::VidShare(share),
        ))
        .await;
    assert!(
        vote1_at(&harness, view(12)).is_some(),
        "the timeout certificate's lock admits the boundary block"
    );
}
