//! Safety of the two-phase vote across nodes.
//!
//! A fork needs two quorums: one certifying a block at view `v`, another
//! certifying a block at a later view whose ancestry skips `v`. The second
//! quorum can only form from nodes that are not locked at `v`, and a node locks
//! at `v` exactly when it casts a phase-2 vote there. So the two quorums compete
//! for the same nodes, and whether they can both form comes down to whether a
//! node can serve both: voting phase-1 in a later view while unlocked, then
//! phase-2 in the earlier one once its block arrives. Nothing about when a
//! certificate and a block turn up orders them against the node's other votes,
//! so `Consensus::voted_for_branch_excluding` is what refuses the second vote.
//! This test builds the whole fork and stops at that refusal.
//!
//! The nodes here are separate `Consensus` instances fed by hand, which is what
//! a network does for them. Delivery order is the variable under test, so it is
//! controlled rather than left to a real network.

use hotshot::types::BLSPubKey;
use hotshot_example_types::{
    block_types::TestBlockHeader,
    node_types::{TEST_VERSIONS, TestTypes},
};
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    message::Proposal,
    stake_table::StakeTableEntries,
    traits::{block_contents::BlockHeader, signature_key::SignatureKey},
    vote::{Certificate, HasViewNumber, Vote},
};

use crate::{
    cert_verifier::ValidCert,
    consensus::{ConsensusInput, ConsensusOutput},
    helpers::{proposal_commitment, test_upgrade_lock},
    message::{Certificate1, Certificate2},
    tests::common::utils::{
        ConsensusHarness, SignerShare, TestData, assemble_cert, build_timeout_cert_signed_by,
        mock_membership, sign_vote_as,
    },
};

// ---------------------------------------------------------------------------
// Two conflicting quorums, assembled from real node behaviour
// ---------------------------------------------------------------------------

/// The certificate threshold for the 10-node test committee: `(10 * 2) / 3 + 1`.
const THRESHOLD: usize = 7;

/// Nodes whose signatures the adversary supplies. Node 6 leads view 6, so the
/// competing proposal is signed by a node the adversary controls.
const BYZANTINE: [u64; 3] = [6, 7, 8];

/// Honest nodes that never receive view 2's block, so they never lock there.
///
/// Node 2 is excluded: it leads view 2 and therefore holds that block whatever
/// the network does.
const NEVER_LOCKED: [u64; 3] = [0, 1, 3];

/// Honest nodes that receive view 2 in full and lock on it.
const LOCKED: [u64; 3] = [2, 4, 5];

/// The honest node that is unlocked when the competing proposal arrives and
/// receives view 2's block only afterwards.
const LATE: u64 = 9;

/// Who sent a timeout vote for view 5, and so backs the competing proposal's
/// view-change evidence.
///
/// The honest signers are nodes that left view 2 — 0, 1 and 3 on a timeout
/// certificate for it, node 2 by locking there — and then timed out through
/// views 3, 4 and 5. [`LATE`] is not among them because it never left view 2,
/// which the execution notes on the test below explain.
///
/// That exclusion is forced by the execution rather than chosen, and it is also
/// what keeps the scenario sharp: a timeout vote endorses no branch, so barring
/// the phase-2 vote on `timeout_view` would stop the nodes that timed out at
/// view 5 and let [`LATE`], which never timed out at all, walk straight past.
const TIMED_OUT_AT_5: [u64; 7] = [0, 1, 2, 3, 6, 7, 8];

/// One quorum, not two.
///
/// Seven honest nodes are run as separate consensus instances and fed by hand,
/// which is what a network does for them. Three roles:
///
/// - `LOCKED` receive view 2 complete. They vote phase-2 there and lock, so they
///   refuse the competing proposal.
/// - `NEVER_LOCKED` receive view 2's proposal and its phase-1 certificate but
///   never its block. Reconstruction is what a phase-2 vote requires, so they
///   never lock, and they accept the competing proposal.
/// - `LATE` is the interesting one. It is in the same state as `NEVER_LOCKED`
///   when the competing proposal arrives, so it votes phase-1 there — and then
///   view 2's block lands, which is everything it needs to vote phase-2 at view
///   2 as well and stand on both sides.
///
/// That second vote is refused. `LATE`'s phase-1 vote at view 6 is justified at
/// view 1, so the branch it endorsed holds no block for view 2, and a commit
/// vote there afterwards is the double count itself.
///
/// The scenario is built out to where that matters. Both sides are driven as far
/// as the nodes will take them: the competing branch collects its phase-1
/// certificate and then its commit certificate, since the nodes that voted
/// phase-1 at view 6 go on to vote phase-2 there. View 2 gets three phase-2
/// votes and stops one short of a commit, `LATE` no longer being counted twice.
/// Two quorums drawn from ten nodes meet in four: the three adversary nodes and
/// one honest one. Everything rests on that one node, and it now stands on a
/// single side.
///
/// So the competing branch wins and view 2's block is orphaned, which is what
/// safety asks for — orphaned rather than committed and then contradicted.
///
/// Nothing here is counted where it could be verified. The only fabricated
/// signatures are the adversary's, which is what an adversary can do anyway.
///
/// # What distinguishes `LATE`
///
/// Neither of the two properties that look like they would catch it. `LATE` has
/// not left view 2 when the second vote comes due, so a guard phrased as "vote
/// phase-2 only in the vote's own view" would admit it; and its timer never
/// fires, so `timeout_view` is still 0 and a bar of `view > timeout_view` would
/// admit it too. The test asserts both states where it reaches them, so
/// narrowing the scenario to one those guards would catch fails here rather than
/// passing quietly.
///
/// What is left is that it voted phase-1 in a *later* view while unlocked and
/// would then have voted phase-2 in an earlier one — one node in both quorums.
/// That is what `Consensus::voted_for_branch_excluding` refuses.
///
/// # What the execution requires
///
/// **View 1's certificate is withheld from `LATE` until late.** Its view-2 timer
/// starts when it enters view 2, which is when it locks view 1. Hold that back
/// until the rest of the network has produced the timeout certificate for view 5,
/// then deliver in one burst inside a single view duration: view 1 complete, view
/// 2's proposal and phase-1 certificate without the block, the competing
/// proposal, and last view 2's block. `LATE` need not have taken part in views 1
/// to 5 at all — view 2's phase-1 certificate forms from the six other honest
/// nodes and the adversary without its vote.
///
/// **`LATE` never receives a timeout certificate for view 2.** That retires the
/// view for VID reconstruction, so view 2's block would never arrive and the
/// second vote could not happen.
///
/// Both are permitted under asynchrony, and both are constraints on any execution
/// matching this scenario.
///
#[tokio::test]
async fn conflicting_quorums_cannot_both_reach_threshold() {
    let test_data = TestData::new(6).await;
    let committed = &test_data.views[1]; // view 2, parented at view 1
    let leader6 = test_data.views[5].leader_public_key;

    // The evidence for the gap, naming exactly who timed out at view 5 — see
    // `TIMED_OUT_AT_5` for why the late node must not be among them.
    assert!(
        !TIMED_OUT_AT_5.contains(&LATE),
        "the late node never reached view 5, so it cannot be a signer here"
    );
    let timeout_cert_5 = {
        let membership = mock_membership();
        let epoch_membership = membership
            .membership_for_epoch(Some(EpochNumber::genesis()))
            .expect("genesis membership");
        build_timeout_cert_signed_by(
            ViewNumber::new(5),
            EpochNumber::genesis(),
            &epoch_membership,
            &TIMED_OUT_AT_5,
        )
    };

    // The competing proposal must come from a node the adversary controls.
    assert!(
        BYZANTINE
            .iter()
            .any(|i| BLSPubKey::generated_from_seed_indexed([0; 32], *i).0 == leader6),
        "view 6's leader must be one of the adversary's nodes"
    );

    // View 6, parented at view 1, so it skips view 2 entirely. It reuses view
    // 6's payload so the honest nodes' VID shares still match, and carries a
    // timeout certificate for view 5 as the evidence for the gap.
    let forked = {
        let template = &test_data.views[5];
        let parent_leaf = test_data.views[0].proposal.data.clone().into();
        let mut proposal = template.proposal.data.clone();
        proposal.block_header = TestBlockHeader::new(
            &parent_leaf,
            template.proposal.data.block_header.payload_commitment,
            template
                .proposal
                .data
                .block_header
                .builder_commitment
                .clone(),
            template.proposal.data.block_header.metadata,
            TEST_VERSIONS.test.base,
        );
        proposal.justify_qc = test_data.views[0].cert1.clone();
        proposal.view_change_evidence = Some(timeout_cert_5.clone());
        proposal
    };
    let forked_signed = {
        let signature = <BLSPubKey as SignatureKey>::sign(
            &test_data.views[5].leader_private_key,
            proposal_commitment(&forked).as_ref(),
        )
        .expect("sign the competing proposal");
        Proposal::new(forked, signature)
    };

    let committed_commit = proposal_commitment(&committed.proposal.data);
    let forked_commit = proposal_commitment(&forked_signed.data);
    let fork_cert_parent_view = forked_signed.data.justify_qc.view_number();
    let mut voted_phase2_at_committed = Vec::new();
    let mut voted_phase1_at_fork = Vec::new();
    let mut committed_shares: Vec<SignerShare> = Vec::new();
    let mut fork_shares: Vec<SignerShare> = Vec::new();
    let mut committed_data = None;
    let mut fork_data = None;
    let mut harnesses = Vec::new();

    for node in NEVER_LOCKED.iter().chain(&LOCKED).chain([LATE].iter()) {
        let node = *node;
        let key = BLSPubKey::generated_from_seed_indexed([0; 32], node).0;
        let mut harness = ConsensusHarness::new(node).await;

        // View 1 in full: every honest node holds and locks it. The competing
        // proposal is parented here, which is why they can admit it at all.
        harness
            .apply_pair(test_data.views[0].proposal_input_consensus(&key))
            .await;
        harness
            .apply(test_data.views[0].block_reconstructed_input())
            .await;
        harness.apply(test_data.views[0].cert1_input()).await;

        // View 2: everyone gets the proposal and the phase-1 certificate. Only
        // `LOCKED` gets the block, so only `LOCKED` votes phase-2 and locks.
        harness
            .apply_pair(committed.proposal_input_consensus(&key))
            .await;
        if LOCKED.contains(&node) {
            harness.apply(committed.block_reconstructed_input()).await;
        }
        harness.apply(committed.cert1_input()).await;

        // A node without view 2's block is still in view 2: the phase-2 vote is
        // what advances the view, and a phase-1 certificate does not.
        if !LOCKED.contains(&node) {
            assert_eq!(
                harness.consensus.current_view(),
                ViewNumber::new(2),
                "a node without view 2's block has not left view 2"
            );
        }

        // The `NEVER_LOCKED` nodes sit in view 2 while the rest of the network
        // times out through views 2 to 5, so their timers fire. It changes
        // nothing — their phase-1 vote at view 6 needs only `6 > timeout_view` —
        // but it is what a real execution looks like.
        //
        // `LATE` is deliberately not given one. See the note on its timer in the
        // test's documentation: it enters view 2 only moments before the rest of
        // the burst, so its timer never fires and its `timeout_view` stays 0.
        if NEVER_LOCKED.contains(&node) {
            harness
                .apply(ConsensusInput::Timeout(
                    ViewNumber::new(2),
                    EpochNumber::genesis(),
                ))
                .await;
        }

        // The competing proposal reaches everyone.
        harness
            .apply(ConsensusInput::Proposal(
                leader6,
                crate::message::ProposalMessage::validated(forked_signed.clone()),
            ))
            .await;
        harness
            .apply(ConsensusInput::VidShare(
                test_data.views[5].vid_share_for(&key),
            ))
            .await;

        // View 2's block finally reaches the late node, after it has already
        // voted on the competing branch.
        if node == LATE {
            // Still in view 2. Casting a phase-1 vote does not advance the view,
            // and a received proposal's `view_change_evidence` is never applied
            // as an input — it is read only when building a proposal. So a guard
            // phrased as "vote phase-2 only while `current_view` is the vote's
            // view" would not stop what follows.
            assert_eq!(
                harness.consensus.current_view(),
                ViewNumber::new(2),
                "the late node is still in view 2 after voting phase-1 at view 6"
            );
            harness.apply(committed.block_reconstructed_input()).await;
        }

        // Take the signatures the node actually produced. The phase-1 vote is
        // parked until its action is persisted, which the harness does, so it
        // reaches the outbox like any other.
        for output in harness.outputs().iter() {
            match output {
                ConsensusOutput::SendVote1(v) if v.vote.data.leaf_commit == forked_commit => {
                    voted_phase1_at_fork.push(node);
                    fork_shares.push((node, v.vote.signature()));
                    fork_data.get_or_insert(v.vote.data);
                },
                ConsensusOutput::SendVote2(v) if v.data.leaf_commit == committed_commit => {
                    voted_phase2_at_committed.push(node);
                    committed_shares.push((node, v.signature()));
                    committed_data.get_or_insert_with(|| v.data.clone());
                },
                _ => {},
            }
        }
        harnesses.push((node, harness));
    }

    // The late node reached both sides and was admitted to one.
    assert_eq!(
        voted_phase1_at_fork,
        vec![0, 1, 3, 9],
        "the unlocked nodes and the late node vote phase-1 on the competing branch"
    );
    assert_eq!(
        voted_phase2_at_committed,
        vec![2, 4, 5],
        "only the locked nodes vote phase-2 at view 2: the late node has voted for a branch \
         without it"
    );

    // The adversary adds its own signatures over the same data, which is all it
    // has to do: the honest votes above are what it cannot forge.
    let committed_data = committed_data.expect("a phase-2 vote was cast at view 2");
    let fork_data = fork_data.expect("a phase-1 vote was cast at view 6");
    for node in BYZANTINE {
        committed_shares.push(sign_vote_as(
            node,
            committed_data.clone(),
            committed.view_number,
        ));
        fork_shares.push(sign_vote_as(node, fork_data, ViewNumber::new(6)));
    }
    assert_eq!(
        (committed_shares.len(), fork_shares.len()),
        (THRESHOLD - 1, THRESHOLD),
        "view 2 should fall exactly one signature short of a quorum, the competing branch not at \
         all"
    );

    let membership = mock_membership();
    let epoch_membership = membership
        .membership_for_epoch(Some(EpochNumber::genesis()))
        .expect("genesis membership");
    let entries = StakeTableEntries::from_iter(epoch_membership.stake_table()).0;
    let threshold = epoch_membership.success_threshold();

    // The same check the proposal validator applies to a justify QC. Below the
    // threshold there is nothing to check: `assemble_cert` will not build a
    // certificate that could never verify.
    let upgrade_lock = test_upgrade_lock::<TestTypes>();
    let committed_valid = committed_shares.len() >= THRESHOLD
        && assemble_cert::<_, Certificate2<TestTypes>>(
            committed_data,
            committed.view_number,
            &epoch_membership,
            &committed_shares,
        )
        .is_valid_cert(&entries, threshold, &upgrade_lock)
        .is_ok();
    let fork_cert: Certificate1<TestTypes> = assemble_cert(
        fork_data,
        ViewNumber::new(6),
        &epoch_membership,
        &fork_shares,
    );
    let fork_valid = fork_cert
        .is_valid_cert(&entries, threshold, &upgrade_lock)
        .is_ok();

    // The branches conflict: view 6's parent is view 1, so view 2 is not an
    // ancestor of it.
    assert_eq!(
        fork_cert_parent_view,
        ViewNumber::new(1),
        "the competing branch must skip the committed view"
    );

    // Drive the competing branch to a commit as well.
    //
    // `maybe_vote_2_and_update_lock` checks the vote mark, the certificate, the
    // proposal and reconstruction — not whether the branch extends the lock. So
    // the same nodes that voted phase-1 at view 6 vote phase-2 there once the
    // certificate and the block arrive, even the one now locked at view 2. That
    // commit is the one that stands, and it is only harmless because view 2
    // never reached one.
    let mut double_commit_shares: Vec<SignerShare> = Vec::new();
    let mut double_commit_data = None;
    for (node, harness) in harnesses.iter_mut() {
        if !voted_phase1_at_fork.contains(node) {
            continue;
        }
        let before = harness.outputs().len();
        harness
            .apply(ConsensusInput::Certificate1(ValidCert::new(
                fork_cert.clone(),
                EpochNumber::genesis(),
            )))
            .await;
        harness
            .apply(ConsensusInput::BlockReconstructed(
                ViewNumber::new(6),
                test_data.views[5].vid_commitment(),
            ))
            .await;
        for output in harness.outputs().iter().skip(before) {
            if let ConsensusOutput::SendVote2(v) = output
                && v.data.leaf_commit == forked_commit
            {
                double_commit_shares.push((*node, v.signature()));
                double_commit_data.get_or_insert_with(|| v.data.clone());
            }
        }
    }
    for node in BYZANTINE {
        if let Some(data) = double_commit_data.clone() {
            double_commit_shares.push(sign_vote_as(node, data, ViewNumber::new(6)));
        }
    }
    let double_commit_valid = double_commit_data.is_some_and(|data| {
        double_commit_shares.len() >= THRESHOLD
            && assemble_cert::<_, Certificate2<TestTypes>>(
                data,
                ViewNumber::new(6),
                &epoch_membership,
                &double_commit_shares,
            )
            .is_valid_cert(&entries, threshold, &upgrade_lock)
            .is_ok()
    });

    // The two blocks sit at the same height on branches that do not extend one
    // another, so certifying both is what a fork would look like.
    assert_eq!(
        (
            BlockHeader::<TestTypes>::block_number(&committed.proposal.data.block_header),
            BlockHeader::<TestTypes>::block_number(&forked_signed.data.block_header)
        ),
        (2, 2),
        "the two blocks should be at the same height"
    );

    // The competing branch got everything it could: a phase-1 certificate and
    // then a commit. Asserting it keeps the two below from passing because the
    // scenario quietly stopped short.
    assert!(
        fork_valid,
        "the competing branch's phase-1 certificate should verify"
    );
    assert!(
        double_commit_valid,
        "the competing branch should reach a commit certificate"
    );

    assert!(
        !(committed_valid && fork_valid),
        "both certificates verify: a phase-2 certificate at view 2 and a phase-1 certificate at \
         view 6 whose branch does not extend it"
    );
    assert!(
        !(committed_valid && double_commit_valid),
        "both branches commit: phase-2 certificates at view 2 and view 6 over different blocks at \
         height 2"
    );
}
