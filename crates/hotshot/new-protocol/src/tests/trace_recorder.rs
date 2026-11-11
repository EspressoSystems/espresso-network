//! What the recorder writes of a run beyond its inputs and outputs.
//!
//! A checker reads a certificate as held from the step that receives it, and
//! leaves a run out, or reads it by fewer rules, on the notes the recorder
//! writes. These tests record a run to a file and read the trace back.

use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    signature_key::BLSPubKey,
    simple_certificate::TimeoutEvidence,
    traits::signature_key::SignatureKey,
};
use serde_json::{Value, json};
use versions::{NEW_PROTOCOL_VERSION, TIMEOUT_EPOCH_VERSION, Version};

use super::common::utils::{
    ConsensusHarness, TestData, build_timeout_cert3_with_lock, carrying_timeout_cert, trace_steps,
};
use crate::{
    cert_verifier::ValidCert,
    consensus::{ConsensusInput, ConsensusOutput},
    helpers::pin_test_version,
    message::{ReVote, ReVoteMessage},
    storage::ActionKind,
    trace::BEFORE_CERTIFICATE_RULE,
};

const EPOCH_HEIGHT: u64 = 10;

/// A node at `version` that records its run to a file of its own. The rest of
/// the test builds its data at that version too.
async fn recorded_node(version: Version) -> (ConsensusHarness, tempfile::TempDir) {
    pin_test_version(version);
    let dir = tempfile::tempdir().unwrap();
    let harness = ConsensusHarness::new_with_epoch_height(0, EPOCH_HEIGHT)
        .await
        .record_to(&dir.path().join("trace.jsonl"));
    (harness, dir)
}

/// The trace of a node that receives nothing but a timeout certificate of
/// `epoch` whose signers are locked on the block of view 1, in epoch 1, and
/// whether the node kept that lock's certificate.
async fn trace_of_a_timeout_certificate(epoch: EpochNumber) -> (String, bool) {
    let (mut harness, _dir) = recorded_node(TIMEOUT_EPOCH_VERSION).await;
    let test_data = TestData::new_with_epoch_height(1, EPOCH_HEIGHT).await;
    // The certificate reaches consensus as already verified, so any committee
    // can sign it. The mock membership has none for a later epoch.
    let membership = harness
        .membership_coordinator
        .membership_for_epoch(Some(EpochNumber::new(2)))
        .unwrap();
    let tc = TimeoutEvidence::V3(build_timeout_cert3_with_lock(
        ViewNumber::new(25),
        epoch,
        Some(test_data.views[0].cert1.clone()),
        &membership,
    ));
    harness
        .apply(ConsensusInput::TimeoutCertificate(ValidCert::new(
            tc, epoch,
        )))
        .await;
    let kept = harness.consensus.cert1_at(ViewNumber::new(1)).is_some();
    (harness.recorded(), kept)
}

/// A timeout certificate's lock of its own epoch or the one before is
/// verified and kept, so it is written as a `Certificate1` received.
#[tokio::test]
async fn recent_lock_of_a_timeout_certificate_is_written_as_received() {
    let (trace, kept) = trace_of_a_timeout_certificate(EpochNumber::new(2)).await;
    assert!(kept, "setup: the node keeps the lock's certificate");
    assert_eq!(trace_steps(&trace, "timeoutCertificate"), 1);
    assert_eq!(trace_steps(&trace, "certificate1"), 1);
}

/// An earlier lock's certificate is not verified and not kept, so no
/// `Certificate1` is written for it.
#[tokio::test]
async fn stale_lock_of_a_timeout_certificate_is_not_written_as_received() {
    let (trace, kept) = trace_of_a_timeout_certificate(EpochNumber::new(3)).await;
    assert!(
        !kept,
        "setup: the node does not keep the lock's certificate"
    );
    assert_eq!(trace_steps(&trace, "timeoutCertificate"), 1);
    assert_eq!(trace_steps(&trace, "certificate1"), 0);
}

/// The trace of a node of `epoch` that receives a re-vote request of epoch 1
/// for view 3, carrying the certificate of view 1 and the timeout certificate
/// of view 2.
async fn trace_of_a_revote(epoch: EpochNumber) -> String {
    let (mut harness, _dir) = recorded_node(TIMEOUT_EPOCH_VERSION).await;
    let test_data = TestData::new_with_epoch_height(1, EPOCH_HEIGHT).await;
    harness.set_view(ViewNumber::new(1), epoch);
    let membership = harness
        .membership_coordinator
        .membership_for_epoch(Some(EpochNumber::new(1)))
        .unwrap();
    let revote = ReVote {
        view: ViewNumber::new(3),
        epoch: EpochNumber::new(1),
        cert1: test_data.views[0].cert1.clone(),
        timeout: Some(build_timeout_cert3_with_lock(
            ViewNumber::new(2),
            EpochNumber::new(1),
            None,
            &membership,
        )),
    };
    let (_, key) = BLSPubKey::generated_from_seed_indexed([0; 32], 1);
    harness
        .apply(ConsensusInput::ReVote(
            ReVoteMessage::new(revote, &key).unwrap(),
        ))
        .await;
    let taken = epoch == EpochNumber::new(1);
    assert_eq!(
        harness.consensus.cert1_at(ViewNumber::new(1)).is_some(),
        taken,
        "setup: the node keeps the request's certificate unless it refuses the request"
    );
    assert_eq!(
        harness
            .consensus
            .timeout_cert_at(ViewNumber::new(3))
            .is_some(),
        taken,
        "setup: and likewise its timeout certificate"
    );
    harness.recorded()
}

/// The certificates a re-vote request carries are written as received when the
/// node takes the request.
#[tokio::test]
async fn revote_taken_has_its_certificates_recorded() {
    let trace = trace_of_a_revote(EpochNumber::new(1)).await;
    assert_eq!(trace_steps(&trace, "certificate1"), 1);
    assert_eq!(trace_steps(&trace, "timeoutCertificate"), 1);
}

/// A node refuses a re-vote request of an epoch earlier than its own and
/// keeps none of the certificates it carries, so none is written as received.
#[tokio::test]
async fn revote_refused_has_none_of_its_certificates_recorded() {
    let trace = trace_of_a_revote(EpochNumber::new(2)).await;
    assert_eq!(
        trace_steps(&trace, "revote"),
        1,
        "setup: the request is recorded"
    );
    assert_eq!(trace_steps(&trace, "certificate1"), 0);
    assert_eq!(trace_steps(&trace, "timeoutCertificate"), 0);
}

/// The trace of a node at `version` that receives the proposal of view 1.
async fn trace_of_a_proposal(version: Version) -> String {
    let (mut harness, _dir) = recorded_node(version).await;
    let test_data = TestData::new_with_epoch_height(1, EPOCH_HEIGHT).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;
    harness
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness.recorded()
}

/// A proposal the node takes has its timeout certificate written as received.
#[tokio::test]
async fn taken_proposal_has_its_timeout_certificate_recorded() {
    let (mut harness, _dir) = recorded_node(TIMEOUT_EPOCH_VERSION).await;
    let test_data = TestData::new_with_epoch_height(1, EPOCH_HEIGHT).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;
    let (proposal, share) = test_data.views[0].proposal_input_consensus(&node_key);
    let membership = harness
        .membership_coordinator
        .membership_for_epoch(Some(EpochNumber::new(1)))
        .unwrap();
    let tc = TimeoutEvidence::V3(build_timeout_cert3_with_lock(
        ViewNumber::genesis(),
        EpochNumber::new(1),
        None,
        &membership,
    ));
    harness
        .apply_pair((carrying_timeout_cert(proposal, tc), share))
        .await;
    assert!(
        harness
            .consensus
            .proposals()
            .live(ViewNumber::new(1))
            .is_some(),
        "setup: the node took the proposal"
    );
    assert!(
        harness
            .consensus
            .timeout_cert_at(ViewNumber::new(1))
            .is_some(),
        "setup: and kept its timeout certificate"
    );
    let trace = harness.recorded();
    assert_eq!(trace_steps(&trace, "timeoutCertificate"), 1);
}

/// A node at a view before the certificate rule marks the run, even with no
/// timeout in it.
#[tokio::test]
async fn input_before_the_certificate_rule_marks_the_run() {
    let trace = trace_of_a_proposal(NEW_PROTOCOL_VERSION).await;
    assert!(trace.lines().any(|l| l == BEFORE_CERTIFICATE_RULE));
}

/// A run under the certificate rule is not marked, and one without a restart
/// has none noted.
#[tokio::test]
async fn run_under_the_certificate_rule_without_a_restart_has_neither_note() {
    let trace = trace_of_a_proposal(TIMEOUT_EPOCH_VERSION).await;
    assert_eq!(
        trace_steps(&trace, "proposal"),
        2,
        "setup: the run was recorded"
    );
    assert!(!trace.lines().any(|l| l == BEFORE_CERTIFICATE_RULE));
    assert!(!trace.lines().any(|l| l.starts_with("# placed")));
}

/// A node resumed as after a restart has that noted as a placement before its
/// next step, with the last view it is barred from, which need not be the
/// last one it acted in.
#[tokio::test]
async fn restart_is_noted_as_a_placement() {
    let (mut harness, _dir) = recorded_node(TIMEOUT_EPOCH_VERSION).await;
    let test_data = TestData::new_with_epoch_height(1, EPOCH_HEIGHT).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;
    harness
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness.resume_from_restart(
        ViewNumber::genesis(),
        ViewNumber::new(10),
        ViewNumber::new(3),
    );
    harness.apply(test_data.views[0].cert1_input()).await;
    let trace = harness.recorded();
    let lines: Vec<&str> = trace.lines().collect();
    let note = lines
        .iter()
        .position(|l| l.starts_with("# placed restart after view 9 (at step "))
        .expect("the restart is noted with the view it bars up to");
    assert_eq!(
        trace_steps(&lines[..note].join("\n"), "certificate1"),
        1,
        "setup: the proposal's parent certificate was received before the restart"
    );
    assert_eq!(
        trace_steps(&lines[note..].join("\n"), "certificate1"),
        1,
        "the note comes before the step taken after the restart"
    );
}

/// The trace of a node that takes the proposal of view 1 and then two timers
/// for that view, its timeout vote held back for storage throughout.
async fn trace_of_two_timers() -> String {
    let (mut harness, _dir) = recorded_node(TIMEOUT_EPOCH_VERSION).await;
    let test_data = TestData::new_with_epoch_height(1, EPOCH_HEIGHT).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;
    harness
        .apply_pair(test_data.views[0].proposal_input_consensus(&node_key))
        .await;
    harness.defer_action(ViewNumber::new(1), ActionKind::Timeout);
    harness
        .apply(ConsensusInput::Timeout(ViewNumber::new(1)))
        .await;
    harness
        .apply(ConsensusInput::TimeoutOneHonest(ViewNumber::new(1)))
        .await;
    assert!(
        !harness
            .outputs()
            .iter()
            .any(|o| matches!(o, ConsensusOutput::SendTimeoutVote(..))),
        "setup: the timeout vote is still held back"
    );
    harness.recorded()
}

/// The lines of `trace` that are steps or dropped inputs, as JSON.
fn events(trace: &str) -> Vec<Value> {
    trace
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| serde_json::from_str(l).expect("a line of a trace is JSON"))
        .collect()
}

/// What a line of a trace says beyond its input and outputs.
fn notes(event: &Value) -> &Value {
    event
        .get("consensus")
        .or_else(|| event.get("dropped"))
        .map(|body| &body["notes"])
        .expect("a line is a step or a dropped input")
}

/// The actions of `kind` for view 1 that `event` says were held back, each
/// as the writes it waits for.
fn held_back(event: &Value, kind: &str) -> Vec<Value> {
    notes(event)["held"]
        .as_array()
        .expect("the actions held back are a list")
        .iter()
        .filter(|held| held["kind"] == kind && held["view"] == 1 && held["epoch"] == 1)
        .map(|held| held["waits"].clone())
        .collect()
}

/// A certificate written as received just before the message that carried it
/// is marked as taken with it. A vote held back for storage is written in the
/// step that held it back, with the writes it waits for.
#[tokio::test]
async fn joined_steps_and_held_back_votes_are_written() {
    let trace = trace_of_a_proposal(TIMEOUT_EPOCH_VERSION).await;
    let events = events(&trace);
    let joined = events
        .iter()
        .position(|e| notes(e)["joined"] == true)
        .expect("the parent's certificate is taken with the proposal");
    assert!(events[joined]["consensus"]["input"]["certificate1"].is_object());
    assert!(events[joined + 1]["consensus"]["input"]["proposal"].is_object());
    assert_eq!(notes(&events[joined + 1])["joined"], false);

    let held: Vec<Value> = events.iter().flat_map(|e| held_back(e, "vote1")).collect();
    assert_eq!(
        held,
        [json!([{ "action": { "v": 1, "kind": "vote" } }])],
        "the vote1 waits for its action to be stored: the proposal already is"
    );
}

/// Storage confirming a write is a line of its own, with what it confirmed
/// and what the node released on it.
#[tokio::test]
async fn storage_confirmation_is_a_line_with_what_it_released() {
    let trace = trace_of_a_proposal(TIMEOUT_EPOCH_VERSION).await;
    let events = events(&trace);
    let confirmed = |write: &str| {
        events
            .iter()
            .filter(|e| e["dropped"]["kind"]["stored"]["w"][write]["v"] == 1)
            .collect::<Vec<_>>()
    };
    let votes = |event: &Value| {
        event["dropped"]["output"]
            .as_array()
            .expect("the outputs are a list")
            .iter()
            .filter(|o| o["send"]["m"]["vote1"].is_object())
            .count()
    };
    let action = confirmed("action");
    let proposal = confirmed("proposal");
    assert_eq!((action.len(), proposal.len()), (1, 1));
    assert_eq!(
        action[0]["dropped"]["kind"]["stored"]["w"]["action"]["kind"],
        "vote"
    );
    assert!(proposal[0]["dropped"]["kind"]["stored"]["w"]["proposal"]["c"].is_string());
    assert_eq!(
        votes(action[0]) + votes(proposal[0]),
        1,
        "the vote1 is released on the confirmation of the last write it waits for"
    );
}

/// A timer for a view whose timeout vote is held back already is answered by
/// that vote, so the vote is written as held back at each timer's step.
#[tokio::test]
async fn timeout_vote_held_back_is_written_at_every_timer() {
    let trace = trace_of_two_timers().await;
    let events = events(&trace);
    let timers: Vec<&Value> = events
        .iter()
        .filter(|e| {
            let input = &e["consensus"]["input"];
            input["timeout"].is_object() || input["timeoutOneHonest"].is_object()
        })
        .collect();
    assert_eq!(timers.len(), 2);
    for timer in timers {
        assert_eq!(
            held_back(timer, "timeout"),
            [json!([{ "action": { "v": 1, "kind": "timeout" } }])]
        );
    }
}
