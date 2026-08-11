//! Intake admission bounds.
//!
//! Every epoch a message claims is read off the wire to decide which committee
//! would verify that message, so it reaches a stake-table lookup before anything
//! about the message has been checked. Resolving an epoch costs work linear in
//! the epoch number, so intake bounds the claim first. These tests pin that
//! boundary: the same message is admitted at the ceiling and dropped one epoch
//! past it.

use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    vote::HasViewNumber,
};

use super::common::{harness::TestHarness, utils::TestData};
use crate::{
    coordinator::{EPOCH_CHANGE_LOOKAHEAD, MAX_VIEWS_AHEAD},
    message::{CatchupEvidence, ConsensusMessage, Message, MessageType, Validated},
};

/// The harness has entered no epoch yet, so intake compares against
/// `EpochNumber::genesis()`, which is 1 rather than 0.
fn at_ceiling() -> EpochNumber {
    EpochNumber::genesis() + EPOCH_CHANGE_LOOKAHEAD
}

fn past_ceiling() -> EpochNumber {
    at_ceiling() + 1
}

/// Rewrite every epoch this message claims, so the same message can be offered
/// at the ceiling and one past it.
fn set_claimed_epoch(message: &mut Message<TestTypes, Validated>, epoch: EpochNumber) {
    let MessageType::Consensus(consensus) = &mut message.message_type else {
        panic!("test only builds consensus messages");
    };
    match consensus {
        ConsensusMessage::Proposal(p) => {
            p.proposal.data.epoch = epoch;
            p.proposal.data.justify_qc.data.epoch = Some(epoch);
        },
        ConsensusMessage::Vote1(v) => {
            v.vote.data.epoch = Some(epoch);
            if let Some(state_vote) = v.state_vote.as_mut() {
                state_vote.epoch = epoch;
            }
        },
        ConsensusMessage::Vote2(v) => v.data.epoch = epoch,
        ConsensusMessage::Certificate1(c, _) => c.data.epoch = Some(epoch),
        ConsensusMessage::Certificate2(c, _) => c.data.epoch = epoch,
        ConsensusMessage::TimeoutVote(m) => {
            m.vote.data.epoch = Some(epoch);
            match m.evidence.as_mut() {
                Some(CatchupEvidence::Qc(qc)) => qc.data.epoch = Some(epoch),
                Some(CatchupEvidence::Tc(tc)) => tc.data.epoch = Some(epoch),
                None => {},
            }
        },
        ConsensusMessage::TimeoutCertificate(c) => c.data.epoch = Some(epoch),
        ConsensusMessage::HighQc(c) => c.data.epoch = Some(epoch),
        other => panic!("test does not build {other:?}"),
    }
}

/// One message of every kind that carries an epoch into a stake-table lookup,
/// paired with a label for assertion messages.
async fn epoch_bearing_messages() -> Vec<(&'static str, Message<TestTypes, Validated>)> {
    let data = TestData::new(1).await;
    let view = &data.views[0];
    let key = view.leader_public_key;
    let consensus = |m| Message {
        sender: key,
        message_type: MessageType::Consensus(m),
    };
    vec![
        ("proposal", view.proposal_input()),
        ("vote1", view.vote1_input(0)),
        ("vote2", view.vote2_input(0)),
        ("timeout_vote", view.timeout_vote_input(0, None)),
        (
            "timeout_vote_with_evidence",
            view.timeout_vote_input(0, Some(CatchupEvidence::Qc(view.cert1.clone()))),
        ),
        (
            "certificate1",
            consensus(ConsensusMessage::Certificate1(view.cert1.clone(), key)),
        ),
        (
            "certificate2",
            consensus(ConsensusMessage::Certificate2(view.cert2.clone(), key)),
        ),
        (
            "timeout_certificate",
            consensus(ConsensusMessage::TimeoutCertificate(
                view.timeout_cert.clone(),
            )),
        ),
        (
            "high_qc",
            consensus(ConsensusMessage::HighQc(view.cert1.clone())),
        ),
    ]
}

/// A message claiming an epoch past the lookahead ceiling is dropped at intake:
/// no subsystem is left holding work for it.
#[tokio::test]
async fn intake_drops_messages_claiming_an_epoch_past_the_ceiling() {
    for (label, mut message) in epoch_bearing_messages().await {
        let mut harness = TestHarness::new(0).await;
        assert_eq!(
            harness.coordinator().pending_intake_work(),
            0,
            "{label}: harness should start idle"
        );
        let epoch = past_ceiling();
        set_claimed_epoch(&mut message, epoch);
        harness.message(message);
        assert_eq!(
            harness.coordinator().pending_intake_work(),
            0,
            "{label}: message claiming epoch {epoch} was admitted past the ceiling"
        );
    }
}

/// The same message at the ceiling is admitted, so the bound discriminates
/// rather than dropping everything.
#[tokio::test]
async fn intake_admits_messages_claiming_an_epoch_at_the_ceiling() {
    for (label, mut message) in epoch_bearing_messages().await {
        let mut harness = TestHarness::new(0).await;
        let epoch = at_ceiling();
        set_claimed_epoch(&mut message, epoch);
        harness.message(message);
        assert!(
            harness.coordinator().pending_intake_work() > 0,
            "{label}: message claiming epoch {epoch} should be admitted at the ceiling"
        );
    }
}

/// A proposal is bounded in view like every other view-keyed message.
#[tokio::test]
async fn intake_drops_proposals_too_far_ahead_in_view() {
    let data = TestData::new(1).await;
    let mut message = data.views[0].proposal_input();
    let MessageType::Consensus(ConsensusMessage::Proposal(p)) = &mut message.message_type else {
        unreachable!("proposal_input builds a proposal")
    };

    let mut harness = TestHarness::new(0).await;
    let far = harness.current_view() + *MAX_VIEWS_AHEAD + 1;
    p.proposal.data.view_number = far;
    assert_eq!(message.view_number(), far);

    harness.message(message);
    assert_eq!(
        harness.coordinator().pending_intake_work(),
        0,
        "proposal {far} views ahead was admitted"
    );
}

/// A proposal within the view bound is still admitted.
#[tokio::test]
async fn intake_admits_proposals_within_the_view_bound() {
    let data = TestData::new(1).await;
    let message = data.views[0].proposal_input();
    assert!(
        message.view_number() <= ViewNumber::genesis() + *MAX_VIEWS_AHEAD,
        "test data should be within the view bound"
    );

    let mut harness = TestHarness::new(0).await;
    harness.message(message);
    assert!(
        harness.coordinator().pending_intake_work() > 0,
        "proposal within the view bound should be admitted"
    );
}
