//! Timeout votes for the view after an epoch boundary.
//!
//! The last block of epoch `e` is certified at view `v`. Every node enters
//! view `v + 1` labelled with epoch `e`, because `maybe_vote_2_and_update_lock`
//! (and `handle_advance_view`) take the label from the block just certified.
//! Only the `EpochChange` message relabels the view with `e + 1`
//! (`handle_epoch_change`). The coordinator arms the view timer with whichever
//! label it holds, and `handle_timeout` signs that label into the vote.
//!
//! `TimeoutData2::commit` covers the view alone, so votes labelled `e` and
//! `e + 1` for the same view aggregate into one certificate. `TimeoutData3::commit`
//! covers the epoch too, so they land in separate buckets of the accumulator
//! and each bucket needs `2f + 1` on its own. With at least `f + 1` stake on
//! each label, neither does, and the view stays timed out.

use std::{collections::BTreeSet, time::Duration};

use committable::Committable;
use hotshot::types::BLSPubKey;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    message::UpgradeLock,
    simple_certificate::{TimeoutCertificate2, TimeoutCertificate3},
    simple_vote::{TimeoutVote2, TimeoutVote3},
    traits::{block_contents::BlockHeader, signature_key::SignatureKey},
    utils::is_epoch_transition,
    vote::HasViewNumber,
};
use versions::{TIMEOUT_EPOCH_VERSION, Upgrade};

use super::common::{
    runner::{DroppedMessage, NodeAction, NodeChange, TestError, TestRunner},
    utils::{ConsensusHarness, TEST_DRB_RESULT, TestData, mock_membership},
};
use crate::{
    consensus::{ConsensusInput, ConsensusOutput},
    helpers::test_upgrade_lock,
    message::{EpochChangeMessage, TimeoutVote},
    vote::{SimpleTally, VoteCollector},
};

/// Validators in the mock membership, each with stake 1.
const NUM_NODES: u64 = 10;
const EPOCH_HEIGHT: u64 = 10;
/// Index into `TestData::views` of view 10, the last block of epoch 1.
const BOUNDARY: usize = 9;
/// The view after the boundary, whose timeout is under test.
const NEXT_VIEW: u64 = 11;
/// Nodes that never see the `EpochChange`. With 10 nodes of stake 1 the
/// success threshold is 7, so 4 votes on one label leave the other 6 short.
const CERT1_ONLY_NODES: u64 = 4;

/// How long to wait for a certificate that must form.
const CERT_TIMEOUT: Duration = Duration::from_secs(10);
/// How long to wait to confirm no certificate forms.
const NO_CERT_TIMEOUT: Duration = Duration::from_millis(500);

fn epoch(e: u64) -> EpochNumber {
    EpochNumber::new(e)
}

fn epoch_bound_lock() -> UpgradeLock<TestTypes> {
    UpgradeLock::new(Upgrade::trivial(TIMEOUT_EPOCH_VERSION))
}

/// Run one node through views 1..=10 under `lock`, optionally deliver the
/// `EpochChange` for view 10, then time out view 11.
///
/// Returns the epoch label the node's timer would hold when it fires (the
/// last `ViewChanged` it emitted) and the timeout vote it broadcasts.
async fn boundary_timeout_vote(
    node_index: u64,
    test_data: &TestData,
    lock: UpgradeLock<TestTypes>,
    saw_epoch_change: bool,
) -> (EpochNumber, TimeoutVote<TestTypes>) {
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], node_index).0;
    let mut harness = ConsensusHarness::new_with_lock(node_index, EPOCH_HEIGHT, lock).await;

    // Pre-feed the DRB results the transition proposals need, as the other
    // epoch-change tests do.
    let mut drb_epochs = BTreeSet::new();
    for view in &test_data.views[..=BOUNDARY] {
        let block = BlockHeader::<TestTypes>::block_number(&view.proposal.data.block_header);
        if is_epoch_transition(block, EPOCH_HEIGHT) {
            drb_epochs.insert(view.epoch_number + 1);
        }
    }
    for e in drb_epochs {
        harness
            .apply(ConsensusInput::DrbResult(e, TEST_DRB_RESULT))
            .await;
    }

    let boundary = &test_data.views[BOUNDARY];
    for view in &test_data.views[..=BOUNDARY] {
        harness
            .apply_pair(view.proposal_input_consensus(&node_key))
            .await;
        harness.apply(view.block_reconstructed_input()).await;
        harness.apply(view.cert1_input()).await;
        // The boundary block's Cert2 travels inside the EpochChange below.
        if view.view_number < boundary.view_number {
            harness.apply(view.cert2_input()).await;
        }
    }

    if saw_epoch_change {
        let epoch_change = EpochChangeMessage::validated(
            boundary.cert1.clone(),
            boundary.cert2.clone(),
            boundary.proposal.data.clone(),
        );
        harness
            .apply(ConsensusInput::EpochChange(epoch_change))
            .await;
    }

    assert_eq!(
        harness.consensus.current_view(),
        ViewNumber::new(NEXT_VIEW),
        "node {node_index} should be in the view after the boundary"
    );
    let label = harness
        .outputs()
        .iter()
        .filter_map(|o| match o {
            ConsensusOutput::ViewChanged(view, e) if **view == NEXT_VIEW => Some(*e),
            _ => None,
        })
        .last()
        .expect("a ViewChanged into the view after the boundary");

    harness
        .apply(ConsensusInput::Timeout(ViewNumber::new(NEXT_VIEW), label))
        .await;
    let vote = harness
        .outputs()
        .iter()
        .filter_map(|o| match o {
            ConsensusOutput::SendTimeoutVote(vote, _) => Some(vote.clone()),
            _ => None,
        })
        .last()
        .expect("the timeout should produce a vote");
    assert_eq!(vote.view_number(), ViewNumber::new(NEXT_VIEW));
    (label, vote)
}

/// Timeout votes for view 11 from every node: the first `CERT1_ONLY_NODES`
/// never see the `EpochChange`, the rest do.
async fn split_boundary_votes(
    test_data: &TestData,
    lock: &UpgradeLock<TestTypes>,
) -> Vec<TimeoutVote<TestTypes>> {
    let mut votes = Vec::new();
    for node in 0..NUM_NODES {
        let saw_epoch_change = node >= CERT1_ONLY_NODES;
        let (label, vote) =
            boundary_timeout_vote(node, test_data, lock.clone(), saw_epoch_change).await;
        let expected = if saw_epoch_change { epoch(2) } else { epoch(1) };
        assert_eq!(
            label, expected,
            "node {node} labels view {NEXT_VIEW} from what it saw at the boundary"
        );
        votes.push(vote);
    }
    votes
}

fn v3_votes(votes: Vec<TimeoutVote<TestTypes>>) -> Vec<TimeoutVote3<TestTypes>> {
    votes
        .into_iter()
        .map(|v| match v {
            TimeoutVote::V3(vote) => vote,
            TimeoutVote::V2(_) => panic!("epoch-binding lock must produce V3 votes"),
        })
        .collect()
}

fn v2_votes(votes: Vec<TimeoutVote<TestTypes>>) -> Vec<TimeoutVote2<TestTypes>> {
    votes
        .into_iter()
        .map(|v| match v {
            TimeoutVote::V2(vote) => vote,
            TimeoutVote::V3(_) => panic!("pre-upgrade lock must produce V2 votes"),
        })
        .collect()
}

type TimeoutCollector3 = VoteCollector<
    TestTypes,
    SimpleTally<TestTypes, TimeoutVote3<TestTypes>, TimeoutCertificate3<TestTypes>>,
>;
type TimeoutCollector2 = VoteCollector<
    TestTypes,
    SimpleTally<TestTypes, TimeoutVote2<TestTypes>, TimeoutCertificate2<TestTypes>>,
>;

/// Honest nodes on both sides of the boundary sign different commitments
/// for the same view, and the coordinator's V3 timeout collector cannot
/// aggregate them: 4 votes labelled epoch 1 and 6 labelled epoch 2 form no
/// certificate although all 10 validators timed out view 11.
#[tokio::test]
async fn v3_timeout_votes_split_at_epoch_boundary_form_no_certificate() {
    let lock = epoch_bound_lock();
    let test_data = TestData::new_with_epoch_height(BOUNDARY + 2, EPOCH_HEIGHT).await;

    let votes = v3_votes(split_boundary_votes(&test_data, &lock).await);
    assert_eq!(votes.len() as u64, NUM_NODES);

    let (cert1_only, epoch_changed) = votes.split_at(CERT1_ONLY_NODES as usize);
    let commitment_of = |v: &TimeoutVote3<TestTypes>| v.data.commit();
    assert!(
        cert1_only.iter().all(|v| v.data.epoch == epoch(1))
            && epoch_changed.iter().all(|v| v.data.epoch == epoch(2)),
        "the label each node signed is the one it entered the view with"
    );
    assert!(
        cert1_only
            .iter()
            .all(|v| commitment_of(v) == commitment_of(&cert1_only[0]))
            && epoch_changed
                .iter()
                .all(|v| commitment_of(v) == commitment_of(&epoch_changed[0])),
        "votes with the same label share a commitment"
    );
    assert_ne!(
        commitment_of(&cert1_only[0]),
        commitment_of(&epoch_changed[0]),
        "TimeoutData3 puts the label into the signed commitment"
    );

    let mut collector = TimeoutCollector3::new(mock_membership(), lock.clone());
    for vote in votes {
        collector.accumulate_vote(vote);
    }
    // The tally for view 11 stays open: neither bucket reaches 7 of 10.
    let outcome = tokio::time::timeout(NO_CERT_TIMEOUT, collector.next()).await;
    assert!(
        outcome.is_err(),
        "10 honest timeout votes for view {NEXT_VIEW} formed a certificate: {outcome:?}"
    );

    // Control: the same 10 nodes, all of which saw the EpochChange, form one.
    let mut control = TimeoutCollector3::new(mock_membership(), lock.clone());
    for node in 0..NUM_NODES {
        let (label, vote) = boundary_timeout_vote(node, &test_data, lock.clone(), true).await;
        assert_eq!(label, epoch(2));
        for vote in v3_votes(vec![vote]) {
            control.accumulate_vote(vote);
        }
    }
    let cert = tokio::time::timeout(CERT_TIMEOUT, control.next())
        .await
        .expect("unanimous labels form a certificate in time")
        .expect("collector still running");
    assert_eq!(cert.view_number(), ViewNumber::new(NEXT_VIEW));
    assert_eq!(cert.epoch(), epoch(2));
}

/// The same split under the pre-upgrade form: the nodes disagree on the
/// label exactly as above, but `TimeoutData2::commit` ignores it, so the
/// V2 collector aggregates all 10 votes into a certificate.
#[tokio::test]
async fn v2_timeout_votes_split_at_epoch_boundary_form_a_certificate() {
    let lock = test_upgrade_lock();
    let test_data = TestData::new_with_epoch_height(BOUNDARY + 2, EPOCH_HEIGHT).await;

    let votes = v2_votes(split_boundary_votes(&test_data, &lock).await);
    assert_eq!(votes.len() as u64, NUM_NODES);

    let (cert1_only, epoch_changed) = votes.split_at(CERT1_ONLY_NODES as usize);
    assert!(
        cert1_only.iter().all(|v| v.data.epoch == Some(epoch(1)))
            && epoch_changed.iter().all(|v| v.data.epoch == Some(epoch(2))),
        "the disagreement on the label exists under V2 too"
    );
    assert_eq!(
        cert1_only[0].data.commit(),
        epoch_changed[0].data.commit(),
        "TimeoutData2 leaves the label out of the commitment"
    );

    let mut collector = TimeoutCollector2::new(mock_membership(), lock);
    for vote in votes {
        collector.accumulate_vote(vote);
    }
    let cert = tokio::time::timeout(CERT_TIMEOUT, collector.next())
        .await
        .expect("mixed labels aggregate under V2")
        .expect("collector still running");
    assert_eq!(cert.view_number(), ViewNumber::new(NEXT_VIEW));
}

// ---------------------------------------------------------------------------
// The same split between real coordinators over cliquenet
// ---------------------------------------------------------------------------

/// Views are `view % 10`-led, so node 1 leads view 1 and view 11. It goes
/// down once block 1 is decided: every view up to the boundary then has a
/// live leader, block 10, the last of epoch 1, is proposed at view 10, and
/// view 11 times out because its leader is gone.
const LEAVING_NODE: usize = 1;
/// Nodes that never receive the boundary block's Vote2s, Cert2 relays or
/// EpochChange, so they enter view 11 still labelled with epoch 1. Three of
/// the nine live nodes: the six that do relabel are one short of the
/// threshold of 7, and so are these three.
const CERT1_ONLY_PEERS: [usize; 3] = [7, 8, 9];
/// Enough leaves to prove the network crossed the boundary: views 1..=10
/// and 12..=15, the last four led by nodes that saw the EpochChange.
const TARGET_DECISIONS: usize = 14;

/// Ten real coordinators over cliquenet, node 1 leaving after view 1, three
/// nodes blind to the boundary block's second round.
fn boundary_split_network(lock: UpgradeLock<TestTypes>, max_runtime: Duration) -> TestRunner {
    let boundary = ViewNumber::new(NEXT_VIEW - 1);
    let blind = BTreeSet::from([
        DroppedMessage::Vote2(boundary),
        DroppedMessage::Certificate2(boundary),
        DroppedMessage::EpochChange(boundary),
    ]);
    TestRunner::builder()
        .num_nodes(NUM_NODES as usize)
        .epoch_height(EPOCH_HEIGHT)
        .view_timeout(Duration::from_secs(2))
        .target_decisions(TARGET_DECISIONS)
        .max_runtime(max_runtime)
        .node_changes(vec![(
            1,
            vec![NodeChange {
                idx: LEAVING_NODE,
                action: NodeAction::Shutdown,
            }],
        )])
        .expected_failed_views(BTreeSet::from([ViewNumber::new(NEXT_VIEW)]))
        .dropped_inbound(
            CERT1_ONLY_PEERS
                .iter()
                .map(|&node| (node, blind.clone()))
                .collect(),
        )
        .upgrade_lock(lock)
        .build()
}

/// Under the 0.7 lock the split is a stall: every live node reaches the
/// boundary, every live node keeps timing out view 11, and no node decides
/// past view 10 because no timeout certificate can form.
#[tokio::test(flavor = "multi_thread")]
async fn v3_epoch_boundary_split_stalls_over_cliquenet() {
    let err = boundary_split_network(epoch_bound_lock(), Duration::from_secs(60))
        .run()
        .await
        .expect_err("the network must not reach its decision target");
    let TestError::Timeout { progress } = err else {
        panic!("the run failed for another reason: {err}");
    };

    let boundary = ViewNumber::new(NEXT_VIEW - 1);
    let next = ViewNumber::new(NEXT_VIEW);
    for node in progress.iter().filter(|node| !node.down) {
        // The blind nodes lack the boundary block's Cert2 and stop one
        // decide short of it; everyone else decides it. Nobody goes further.
        let expected_highest = if CERT1_ONLY_PEERS.contains(&node.idx) {
            boundary - 1
        } else {
            boundary
        };
        assert_eq!(
            node.highest_view,
            Some(expected_highest),
            "node {} did not stop at the boundary: {node:?}",
            node.idx
        );
        assert!(
            node.timed_out_views.contains(&next),
            "node {} never timed out view {NEXT_VIEW}: {node:?}",
            node.idx
        );
    }
}

/// The identical network under the 0.6 lock: the mixed labels aggregate
/// into one timeout certificate for view 11 and the run reaches its target.
#[tokio::test(flavor = "multi_thread")]
async fn v2_epoch_boundary_split_recovers_over_cliquenet() {
    boundary_split_network(test_upgrade_lock(), Duration::from_secs(120))
        .run()
        .await
        .expect("the network recovers from the split under the old form");
}
