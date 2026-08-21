//! Liveness when one honest node misses a certified view's payload.
//!
//! A certificate needs 7 of 10 signatures, so with three nodes silent it needs
//! every honest one. This test puts a single honest node one payload behind:
//! it holds view 1's proposal, cast a vote1 there, holds view 1's cert1, and
//! misses only the VID share broadcasts that would reconstruct the block.
//! That is enough to stop every later view from certifying anything.
//!
//! The stall is a fixed point. The six other honest nodes lock view 1, so a
//! proposal they will vote for must be parented there, and their locks move
//! only on a new cert1 — which needs seven votes and gets six,
//! because voting requires the parent's block reconstructed and the behind
//! node cannot supply that for view 1. Timeout certificates still form — the
//! behind node answers timeouts like anyone else — so views keep advancing
//! while nothing certifies, and each later view repeats the previous one:
//! parent pinned at view 1, six votes where seven are needed.
//!
//! Two facts outside this harness would make the fixed point permanent on
//! their own. Nothing retransmits the missed broadcasts: Cliquenet retries a
//! send until it is ACKed, but the `Coordinator` drops the un-ACKed copies two
//! views behind the frontier, so entering view 3 — on the timeout certificate
//! the behind node co-signs — deletes the last copies of view 1's share
//! broadcasts network-wide. And the proposal fetch cannot help: the behind
//! node has the proposal; what it lacks is the payload.
//!
//! The payload fetch is the exit. Once the network has moved past a certified
//! view this node cannot act on, consensus asks for what is missing, and the
//! coordinator turns the request into a fetch: the whole payload, unicast by
//! one peer drawn from the view's committee, never a broadcast. A peer that
//! does not hold it, or whose payload does not fit a message, says so and the
//! next round draws someone else. The bytes are believed only if they
//! re-commit to what our own proposal names, and then come back as an
//! ordinary `BlockReconstructed`. The first test walks the stall up to the
//! emitted request; the second walks through the exit.

use hotshot::types::BLSPubKey;
use hotshot_example_types::{block_types::TestBlockHeader, node_types::TEST_VERSIONS};
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    message::Proposal,
    traits::signature_key::SignatureKey,
    vote::HasViewNumber,
};

use crate::{
    cert_verifier::ValidCert,
    consensus::{ConsensusInput, ConsensusOutput},
    helpers::proposal_commitment,
    message::CatchupEvidence,
    tests::common::utils::{
        ConsensusHarness, TestData, build_timeout_cert_signed_by, mock_membership,
    },
};

/// The certificate threshold for the 10-node test committee.
const THRESHOLD: usize = 7;

/// Certification stalls for good once one honest node misses one payload.
///
/// Roles, chosen around the leader schedule so that the delivered proposals
/// come from nodes that are up:
///
/// - three nodes are *silent* — crashed, or withholding, which a node cannot
///   tell apart. They contribute no votes, so every certificate needs all
///   seven honest signatures.
/// - one honest node is *behind*: it takes part in view 1 — proposal, VID
///   share, vote1 — but the other voters' share broadcasts never reach
///   it, so it cannot reconstruct view 1's block. Everything else reaches it.
/// - the remaining six honest nodes are *current*: they hold view 1 complete,
///   vote phase-2 there and lock it.
///
/// The run then drives the two shapes every later view must take:
///
/// 1. **View 2, the direct parent.** Its proposal extends view 1. The six
///    current nodes vote; the behind node holds the proposal, the share and
///    the phase-1 certificate of view 1, and still must refuse — voting needs
///    the parent reconstructed. Six votes, seven needed.
/// 2. **View 3, the timeout path.** View 2 times out; the timeout certificate
///    forms from all seven honest votes, the behind node's included — a
///    timeout vote needs no data. View 3's leader proposes on its lock:
///    parent view 1 again, the certificate as evidence. Same count, same
///    refusal, same shortfall.
///
/// Afterwards nothing has moved by itself: the six current nodes are still
/// locked at view 1, so view 4 and every view after it would replay view 3
/// verbatim. The behind node did everything asked of a correct node — it
/// voted where it could, answered the timeout, advanced on the certificate —
/// and, from the moment the network moved past view 1, asked for the one
/// thing it lacks: the test pins the `RequestMissingPayload` for view 1,
/// emitted by the behind node alone. `fetched_payload_restores_certification`
/// continues from here.
///
/// The catchup evidence attached to the current nodes' timeout votes names
/// view 1's certificate — the pointer to the cure travels every timeout
/// round, and the payload fetch is what finally acts on it.
#[tokio::test]
async fn one_missed_payload_stalls_certification() {
    let test_data = TestData::new(3).await;
    let epoch = EpochNumber::genesis();

    let index_of = |key: &BLSPubKey| -> u64 {
        (0..10u64)
            .find(|i| BLSPubKey::generated_from_seed_indexed([0; 32], *i).0 == *key)
            .expect("every leader is a committee member")
    };
    let leaders: Vec<u64> = test_data
        .views
        .iter()
        .map(|v| index_of(&v.leader_public_key))
        .collect();

    let behind: u64 = (0..10)
        .find(|i| !leaders.contains(i))
        .expect("ten nodes, at most three lead views 1-3");
    let silent: Vec<u64> = (0..10)
        .filter(|i| !leaders.contains(i) && *i != behind)
        .take(3)
        .collect();
    let current: Vec<u64> = (0..10)
        .filter(|i| *i != behind && !silent.contains(i))
        .collect();
    assert_eq!(
        (silent.len(), current.len() + 1),
        (3, THRESHOLD),
        "seven honest nodes: every certificate needs all of them"
    );
    let honest: Vec<u64> = current.iter().copied().chain([behind]).collect();

    // View 2's timeout certificate carries all seven honest signatures; six
    // would not reach the threshold. The signatures are fabricated here for
    // assembly only — the run below shows every signer actually emits the
    // vote, the behind node's asserted by name.
    let timeout_cert_2 = {
        let membership = mock_membership();
        let epoch_membership = membership
            .membership_for_epoch(Some(epoch))
            .expect("genesis membership");
        build_timeout_cert_signed_by(ViewNumber::new(2), epoch, &epoch_membership, &honest)
    };

    // View 3's proposal as its leader must build it after view 2 timed out:
    // parented at its lock — view 1 — with the timeout certificate as
    // evidence. It reuses view 3's payload so the honest nodes' VID shares
    // still match.
    let stalled_signed = {
        let template = &test_data.views[2];
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
        proposal.view_change_evidence = Some(timeout_cert_2.clone());
        let signature = <BLSPubKey as SignatureKey>::sign(
            &template.leader_private_key,
            proposal_commitment(&proposal).as_ref(),
        )
        .expect("sign view 3's proposal");
        Proposal::new(proposal, signature)
    };
    let leader_3 = test_data.views[2].leader_public_key;

    let commit_view_1 = proposal_commitment(&test_data.views[0].proposal.data);
    let commit_view_2 = proposal_commitment(&test_data.views[1].proposal.data);
    let commit_view_3 = proposal_commitment(&stalled_signed.data);

    let mut voted_phase1_at_1 = Vec::new();
    let mut voted_phase2_at_1 = Vec::new();
    let mut voted_phase1_at_2 = Vec::new();
    let mut voted_phase1_at_3 = Vec::new();
    let mut timed_out_at_2 = Vec::new();
    let mut requested_payload_at_1 = Vec::new();

    for node in honest.iter().copied() {
        let key = BLSPubKey::generated_from_seed_indexed([0; 32], node).0;
        let mut harness = ConsensusHarness::new(node).await;

        // View 1: everyone takes part. Only the current nodes get the share
        // broadcasts that reconstruct the block; the behind node gets
        // everything else, the phase-1 certificate included.
        harness
            .apply_pair(test_data.views[0].proposal_input_consensus(&key))
            .await;
        if current.contains(&node) {
            harness
                .apply(test_data.views[0].block_reconstructed_input())
                .await;
        }
        harness.apply(test_data.views[0].cert1_input()).await;

        // The phase-2 vote is what locks and advances; without reconstruction
        // the behind node has done neither, and still sits where it started.
        if node == behind {
            assert_eq!(harness.consensus.current_view(), ViewNumber::genesis());
            assert_eq!(harness.consensus.locked_view(), None);
        }

        // View 2, parented at view 1.
        harness
            .apply_pair(test_data.views[1].proposal_input_consensus(&key))
            .await;

        // View 2 times out. The current nodes' timers fire; the behind node,
        // still in view 1, joins on the evidence of others' timeouts — the
        // vote that makes the timeout certificate reach its threshold.
        if current.contains(&node) {
            harness
                .apply(ConsensusInput::Timeout(ViewNumber::new(2), epoch))
                .await;
        } else {
            harness
                .apply(ConsensusInput::TimeoutOneHonest(ViewNumber::new(2), epoch))
                .await;
        }

        // View 3, parented at view 1 on the timeout certificate.
        harness
            .apply(ConsensusInput::Proposal(
                leader_3,
                crate::message::ProposalMessage::validated(stalled_signed.clone()),
            ))
            .await;
        harness
            .apply(ConsensusInput::VidShare(
                test_data.views[2].vid_share_for(&key),
            ))
            .await;

        // The certificate reaches the behind node and moves it into view 3.
        // In the coordinator this view change is what garbage-collects the
        // transport's un-ACKed copies of view 1's broadcasts — the node's own
        // timeout vote helped destroy the data it is missing.
        if node == behind {
            harness
                .apply(ConsensusInput::TimeoutCertificate(ValidCert::new(
                    timeout_cert_2.clone(),
                    epoch,
                )))
                .await;
            assert_eq!(harness.consensus.current_view(), ViewNumber::new(3));
        }

        for output in harness.outputs().iter() {
            match output {
                ConsensusOutput::SendVote1(v) if v.vote.data.leaf_commit == commit_view_1 => {
                    voted_phase1_at_1.push(node);
                },
                ConsensusOutput::SendVote1(v) if v.vote.data.leaf_commit == commit_view_2 => {
                    voted_phase1_at_2.push(node);
                },
                ConsensusOutput::SendVote1(v) if v.vote.data.leaf_commit == commit_view_3 => {
                    voted_phase1_at_3.push(node);
                },
                ConsensusOutput::SendVote2(v) if v.data.leaf_commit == commit_view_1 => {
                    voted_phase2_at_1.push(node);
                },
                ConsensusOutput::SendTimeoutVote(v, evidence) => {
                    assert_eq!(v.view_number(), ViewNumber::new(2));
                    timed_out_at_2.push(node);
                    // A current node's timeout vote advertises view 1's
                    // certificate — the network keeps telling itself where
                    // the highest lock is, every timeout round. The behind
                    // node has nothing to advertise, and no one has a way to
                    // hand it the payload the advertisement points at.
                    if current.contains(&node) {
                        match evidence {
                            Some(CatchupEvidence::Qc(qc)) => {
                                assert_eq!(qc.view_number(), ViewNumber::new(1));
                            },
                            other => panic!("expected view 1's QC as evidence, got {other:?}"),
                        }
                    } else {
                        assert_eq!(evidence, &None);
                    }
                },
                // Every honest node holds every proposal named here, so the
                // proposal fetch must stay silent; the payload fetch is what
                // the deficit calls for.
                ConsensusOutput::RequestMissingProposal { view, .. } => {
                    panic!("node {node} requested proposal for view {view}, which it holds");
                },
                ConsensusOutput::RequestMissingPayload { view, .. } => {
                    assert_eq!(
                        *view,
                        ViewNumber::new(1),
                        "only view 1's payload is missing"
                    );
                    requested_payload_at_1.push(node);
                },
                _ => {},
            }
        }

        // The locks are the fixed point: they move only on a new phase-1
        // certificate, and none formed. Every view after 3 therefore replays
        // view 3 — a proposal parented at view 1, six votes, no certificate.
        if current.contains(&node) {
            assert_eq!(harness.consensus.locked_view(), Some(ViewNumber::new(1)));
        } else {
            assert_eq!(harness.consensus.locked_view(), None);
        }
    }

    let mut expected_all = honest.clone();
    expected_all.sort_unstable();
    let sorted = |mut v: Vec<u64>| {
        v.sort_unstable();
        v
    };

    // View 1 certified because everyone honest voted — the behind node's
    // deficit begins only after its own vote, with the broadcasts it missed.
    assert_eq!(
        sorted(voted_phase1_at_1),
        expected_all,
        "all seven honest nodes vote phase-1 at view 1, the behind node included"
    );
    assert_eq!(
        sorted(voted_phase2_at_1),
        current,
        "only the current nodes reconstruct view 1, vote phase-2 and lock"
    );
    assert_eq!(
        sorted(timed_out_at_2),
        expected_all,
        "the timeout certificate forms from all seven honest votes; six would not reach the \
         threshold"
    );

    // Both shapes a view can take fall one vote short, and for the same
    // reason: a phase-1 vote needs the parent's block, and the parent is
    // always view 1.
    assert_eq!(
        sorted(voted_phase1_at_2),
        current,
        "view 2: the behind node holds the proposal, the share and view 1's certificate, and \
         still cannot vote — its copy of view 1's block never reconstructed"
    );
    assert_eq!(
        sorted(voted_phase1_at_3),
        current,
        "view 3: the timeout path pins the parent at the leaders' locks, which is view 1 again"
    );
    assert!(
        current.len() < THRESHOLD,
        "six votes where seven are needed: no view certifies, and without the payload fetch the \
         state this run ends in is the state every later view starts from"
    );

    // The exit: once the network moved past view 1 — the behind node's own
    // timeout join, then the timeout certificate — it asked for the payload.
    // The current nodes ask for nothing: a certificate for a reconstructed
    // view needs no fetching, so the healthy path stays silent.
    requested_payload_at_1.dedup();
    assert_eq!(
        requested_payload_at_1,
        vec![behind],
        "exactly the behind node requests view 1's payload"
    );
}

/// The payload fetch turns the stall of
/// [`one_missed_payload_stalls_certification`] back into progress.
///
/// Same behind node, same deficit: it took part in view 1 but missed the
/// share broadcasts, so it holds the proposal and view 1's cert1 and cannot
/// reconstruct. The network times out view 2 and the node joins — the moment
/// the network has provably moved past a certified view it cannot act on,
/// and the moment consensus emits the `RequestMissingPayload` the coordinator
/// turns into a fetch of the whole payload from one peer, which ends in the
/// `BlockReconstructed` input this test injects. View 3's proposal — parented
/// at view 1 on the timeout certificate, the shape every view of the stall
/// takes — arrives and is refused like before.
#[tokio::test]
async fn fetched_payload_restores_certification() {
    let test_data = TestData::new(3).await;
    let epoch = EpochNumber::genesis();

    let leaders: Vec<u64> = test_data
        .views
        .iter()
        .map(|v| {
            (0..10u64)
                .find(|i| {
                    BLSPubKey::generated_from_seed_indexed([0; 32], *i).0 == v.leader_public_key
                })
                .expect("every leader is a committee member")
        })
        .collect();
    let behind: u64 = (0..10)
        .find(|i| !leaders.contains(i))
        .expect("ten nodes, at most three lead views 1-3");
    let honest: Vec<u64> = (0..10)
        .filter(|i| leaders.contains(i) || *i == behind)
        .chain((0..10).filter(|i| !leaders.contains(i) && *i != behind))
        .take(THRESHOLD)
        .collect();

    let timeout_cert_2 = {
        let membership = mock_membership();
        let epoch_membership = membership
            .membership_for_epoch(Some(epoch))
            .expect("genesis membership");
        build_timeout_cert_signed_by(ViewNumber::new(2), epoch, &epoch_membership, &honest)
    };
    let stalled_signed = {
        let template = &test_data.views[2];
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
        proposal.view_change_evidence = Some(timeout_cert_2.clone());
        let signature = <BLSPubKey as SignatureKey>::sign(
            &template.leader_private_key,
            proposal_commitment(&proposal).as_ref(),
        )
        .expect("sign view 3's proposal");
        Proposal::new(proposal, signature)
    };
    let commit_view_3 = proposal_commitment(&stalled_signed.data);

    let key = BLSPubKey::generated_from_seed_indexed([0; 32], behind).0;
    let mut harness = ConsensusHarness::new(behind).await;

    // The stall prefix: view 1 without its share broadcasts, view 2's
    // proposal, the network timing out view 2, view 3 parented at view 1.
    harness
        .apply_pair(test_data.views[0].proposal_input_consensus(&key))
        .await;
    harness.apply(test_data.views[0].cert1_input()).await;
    harness
        .apply_pair(test_data.views[1].proposal_input_consensus(&key))
        .await;
    harness
        .apply(ConsensusInput::TimeoutOneHonest(ViewNumber::new(2), epoch))
        .await;
    harness
        .apply(ConsensusInput::Proposal(
            test_data.views[2].leader_public_key,
            crate::message::ProposalMessage::validated(stalled_signed.clone()),
        ))
        .await;
    harness
        .apply(ConsensusInput::VidShare(
            test_data.views[2].vid_share_for(&key),
        ))
        .await;

    let requested = harness.outputs().iter().any(|o| {
        matches!(o, ConsensusOutput::RequestMissingPayload { view, .. }
            if *view == ViewNumber::new(1))
    });
    assert!(requested, "the payload of view 1 was requested");
    let voted_at_3 = |harness: &ConsensusHarness| {
        harness.outputs().iter().any(|o| {
            matches!(o, ConsensusOutput::SendVote1(v) if v.vote.data.leaf_commit == commit_view_3)
        })
    };
    assert!(
        !voted_at_3(&harness),
        "no vote at view 3 while view 1 is unreconstructed"
    );

    // What the coordinator injects once a peer's response re-committed to the
    // proposal's payload commitment.
    harness
        .apply(ConsensusInput::BlockReconstructed(
            ViewNumber::new(1),
            test_data.views[0].vid_commitment(),
        ))
        .await;

    assert!(
        voted_at_3(&harness),
        "with view 1 reconstructed, the vote at view 3 goes out — the seventh vote, and \
         certification resumes"
    );
    let commit_view_1 = proposal_commitment(&test_data.views[0].proposal.data);
    assert!(
        harness.outputs().iter().any(|o| {
            matches!(o, ConsensusOutput::SendVote2(v) if v.data.leaf_commit == commit_view_1)
        }),
        "reconstruction also completes the phase-2 vote at view 1 itself — held back only by the \
         missing block — so the pinned view can commit directly, not just as an ancestor"
    );
}
