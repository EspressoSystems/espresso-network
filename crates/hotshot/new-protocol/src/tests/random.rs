//! Conformance under a randomly scheduled input stream.
//!
//! The other tests here fix a delivery order and assert an outcome. This one
//! fixes neither: it feeds one node a seeded random permutation of well-formed
//! inputs and asserts nothing about what the node does. What checks the run is
//! the recorded trace, replayed against the Lean reference machine — every
//! action the node took must be one the machine could have taken from the same
//! inputs, which is the whole of conformance.
//!
//! The randomness is in the *schedule*, not in the bytes. Consensus consumes
//! values the verification layer has already accepted (`ValidCert`, a validated
//! `Proposal`), so random bytes would be refused before consensus saw them: that
//! would exercise `cert_verifier` and record a trace of nothing. What is worth
//! shuffling is arrival order — a `Cert1` landing after its view is abandoned, a
//! block reconstructed twice, a timeout racing the proposal it would abandon —
//! and every one of those is a permutation of inputs `TestData` already builds.
//!
//! Inputs the specification does not model are deliberately not generated.
//! They would each turn a trace into `out-of-scope`, which is neither evidence
//! of conformance nor of its absence.

use hotshot::types::BLSPubKey;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{data::EpochNumber, traits::signature_key::SignatureKey};
use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};

use crate::{
    consensus::{ConsensusInput, ConsensusOutput, DECIDE_BUFFER},
    tests::common::utils::{ConsensusHarness, TestData},
};

/// An epoch longer than any run, so no boundary is crossed.
const EPOCH_HEIGHT: u64 = 10_000;

/// Views of material to draw from.
const VIEWS: usize = 5 * DECIDE_BUFFER as usize;

/// Repeated arrivals, as a fraction of the bag.
///
/// A quarter of the inputs arrive a second time, which is where duplicates and
/// late arrivals come from. Stated as a fraction rather than a step count because
/// the two are coupled: an absolute cap below the size of the bag silently drops
/// the repeats altogether, which is what happened when this was `60` steps and
/// the bag held five hundred.
const REPEAT_FRACTION: usize = 4;

/// What the node did, by kind of action.
///
/// A run that conforms while doing nothing conforms trivially, so a trace is
/// only evidence if the node acted. Counting is how this test knows the schedule
/// reached the behaviour rather than merely being accepted.
#[derive(Default, Debug)]
struct Reached {
    vote1: usize,
    vote2: usize,
    proposal: usize,
    decided: usize,
}

impl Reached {
    fn tally<'a>(outputs: impl Iterator<Item = &'a ConsensusOutput<TestTypes>>) -> Self {
        let mut r = Self::default();
        for o in outputs {
            match o {
                ConsensusOutput::SendVote1(_) => r.vote1 += 1,
                ConsensusOutput::SendVote2(_) => r.vote2 += 1,
                ConsensusOutput::SendProposal(_) => r.proposal += 1,
                ConsensusOutput::LeafDecided { .. } => r.decided += 1,
                _ => {},
            }
        }
        r
    }
}

/// One seeded run: a random schedule over `data`, applied to one node.
///
/// The schedule is a shuffled bag rather than independent draws. Drawing each
/// step uniformly looks more random and is much weaker: a vote needs a view's
/// proposal, its share, its payload and its certificate all to have arrived, and
/// independent draws over views assemble that set almost never. A bag holding
/// every view's every input, shuffled, arrives in an arbitrary order but does
/// arrive, so the interesting orders are the ones being tested rather than the
/// question of whether anything happens at all.
///
/// A second, partial copy is appended for repeats: the same input twice, and a
/// late arrival after the view has moved on.
async fn run_seed(seed: u64) -> Reached {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut harness = ConsensusHarness::new_with_epoch_height(0, EPOCH_HEIGHT).await;
    let data = TestData::new_with_epoch_height(VIEWS, EPOCH_HEIGHT).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

    let mut bag: Vec<(usize, u8)> = (0..VIEWS)
        .flat_map(|v| (0..5u8).map(move |k| (v, k)))
        .chain((0..VIEWS).filter(|_| rng.gen_bool(0.25)).map(|v| (v, 5u8)))
        .collect();

    bag.shuffle(&mut rng);

    let mut repeats = bag.clone();
    repeats.shuffle(&mut rng);
    bag.extend(repeats.into_iter().take(bag.len() / REPEAT_FRACTION));

    for (view, kind) in bag {
        let v = &data.views[view];
        match kind {
            0 => {
                harness
                    .apply_pair(v.proposal_input_consensus(&node_key))
                    .await
            },
            1 => harness.apply(v.block_reconstructed_input()).await,
            2 => harness.apply(v.cert1_input()).await,
            3 => harness.apply(v.cert2_input()).await,
            4 => harness.apply(v.timeout_cert_input()).await,
            _ => {
                harness
                    .apply(ConsensusInput::Timeout(
                        v.view_number,
                        EpochNumber::genesis(),
                    ))
                    .await
            },
        }
    }

    Reached::tally(harness.outputs().iter())
}

#[tokio::test]
async fn random_schedules_conform() {
    let mut total = Reached::default();
    for seed in 0..8u64 {
        let reached = run_seed(seed).await;
        assert!(
            reached.vote1 + reached.vote2 + reached.proposal + reached.decided > 0,
            "seed {seed} drew no action at all: {reached:?}"
        );
        total.vote1 += reached.vote1;
        total.vote2 += reached.vote2;
        total.proposal += reached.proposal;
        total.decided += reached.decided;
    }
    // Floors, not expectations: the seeds are fixed, so these counts are
    // deterministic, and they sit well under what the schedule actually reaches.
    // What they catch is a generator that has quietly stopped producing votable
    // material — which is how a conformance run goes green while checking almost
    // nothing.
    assert!(total.vote1 >= 5, "too few vote1s to be evidence: {total:?}");
    assert!(total.vote2 >= 3, "too few vote2s to be evidence: {total:?}");
    // Proposing needs this node to lead one of the hundred views and to hold a
    // header for it, so it is the scarcest of the four and the first to vanish if
    // the schedule stops assembling views. It reached ten when this was written.
    assert!(
        total.proposal >= 3,
        "too few proposals to be evidence: {total:?}"
    );
    assert!(
        total.decided >= 10,
        "too few decides to be evidence: {total:?}"
    );
}
