//! Scenarios from the Lean simulator, replayed to the Rust node.
//!
//! `lake exe sim --scenarios <dir>` runs the reference machine on simulated
//! networks, each with faults, committees, timing and delays drawn at random,
//! and writes, for each node, the inputs it was given in order (`format`).
//! Replaying one gives the Rust node a run in the shape the specification
//! describes: real certificates, timeouts, re-votes and epoch changes, arriving
//! as a network that meets the specification's delivery assumptions would
//! deliver them. The test asserts nothing about what the node does. Its recorded
//! trace is checked against the specification afterwards
//! (`just test-lean-scenarios`), as every other test's is.
//!
//! The scenario is the node's environment, not a script of what it must do: the
//! simulated node ran the reference machine, and the Rust node may act
//! differently where the rules let it. Where it proposes, its own proposal
//! stands for the simulated one if it extends the same parent (`World::adopt_own`).
//! A simulated proposal of the node's that the Rust node did not make is not
//! delivered to it: no network could deliver a proposal under a node's key that
//! the node never signed. Certificates over that block still arrive, as they
//! would from the nodes that voted for it.
//!
//! Inputs the Rust node gets from its own modules are answered by the harness as
//! in the other tests: the header a leader asks for, the validity report, storage
//! confirmations, DRB results. The scenario's `headerBuilt` and `blockValidated`
//! inputs are therefore skipped. A `Cert1` over an epoch root arrives with its
//! light-client state certificate, as the node's state-vote tally delivers it.

mod format;
mod world;

use std::path::{Path, PathBuf};

use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    simple_certificate::TimeoutEvidence,
};

use self::{
    format::{Input, Scenario},
    world::{Cache, World, scheduled_membership},
};
use crate::{
    cert_verifier::ValidCert,
    consensus::{ConsensusInput, ConsensusOutput},
    helpers::{proposal_commitment, test_timeout_epoch_lock},
    message::{EpochChangeMessage, ProposalMessage, ReVote, ReVoteMessage},
    proposal::ProposalValidator,
    tests::common::{
        coordinator_builder::{build_genesis_cert1, build_genesis_proposal},
        utils::{ConsensusHarness, STAKE_TABLE_CAPACITY},
    },
};

/// Replay every scenario in the directory `NP_SCENARIOS` names.
///
/// Does nothing when it is unset, so a plain `cargo test` passes over it. The
/// scenarios are shared out over a thread per core, a run's nodes to the same
/// thread: they name the same blocks, so they share most of what is built.
#[test]
fn scenarios_replay() {
    let Ok(dir) = std::env::var("NP_SCENARIOS") else {
        return;
    };
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {dir}: {e}"))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no scenarios in {dir}");
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let mut shares = vec![Vec::new(); threads];
    for (i, run) in paths.chunk_by(|a, b| run_of(a) == run_of(b)).enumerate() {
        shares[i % threads].extend_from_slice(run);
    }
    std::thread::scope(|s| {
        for share in &shares {
            s.spawn(move || replay_all(share));
        }
    });
}

/// The run a scenario file is of: its name without the node.
fn run_of(path: &Path) -> &str {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    stem.rsplit_once("-node").map_or(stem, |(run, _)| run)
}

/// Replay `paths` in order on this thread.
fn replay_all(paths: &[PathBuf]) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime");
    runtime.block_on(async {
        let mut cache = Cache::default();
        for path in paths {
            cache = replay(path, cache).await;
        }
    });
}

/// Replay one scenario to a fresh node, recording its trace under the file's name.
///
/// Takes and gives back what earlier scenarios built, for later ones to reuse.
async fn replay(path: &Path, cache: Cache) -> Cache {
    crate::logging::init_test_logging();
    let scenario = Scenario::read(path);
    let h = &scenario.header;
    assert!(
        h.nodes as usize <= STAKE_TABLE_CAPACITY,
        "{}: the harness's stake tables hold {STAKE_TABLE_CAPACITY} nodes",
        path.display()
    );
    let label = format!(
        "scenario-{}",
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
    );
    let me = h.node;
    let membership = scheduled_membership(h.nodes, h.epoch_height, &h.committees);
    let mut harness = ConsensusHarness::with_membership(
        me - 1,
        h.epoch_height,
        test_timeout_epoch_lock(),
        membership,
    )
    .await
    .record_as(&label);
    // Started as `Coordinator::maker` starts a node: locked on the anchor.
    let genesis_leaf = harness.consensus.last_decided_leaf().clone();
    harness.consensus.seed_parent(
        build_genesis_cert1(&genesis_leaf),
        build_genesis_proposal(&genesis_leaf),
        std::iter::empty(),
    );
    let mut world = World::new(
        h.nodes,
        h.epoch_height,
        &h.committees,
        &scenario.blocks,
        cache,
    )
    .await;
    let my_key = world.key(me).0;
    let mut seen_outputs = 0;
    for step in &scenario.steps {
        // The node's own proposals since the last step stand for the simulated ones.
        let own: Vec<_> = harness
            .outputs()
            .iter()
            .skip(seen_outputs)
            .filter_map(|o| match o {
                ConsensusOutput::SendProposal(p) => Some(p.clone()),
                _ => None,
            })
            .collect();
        seen_outputs = harness.outputs().len();
        for p in own {
            let view = p.data.view_number;
            if let Some(id) = scenario
                .blocks
                .iter()
                .find(|b| b.view == *view && b.epoch == *p.data.epoch)
                .filter(|b| world.block_leader(b.id) == Some(my_key))
                .map(|b| b.id)
                && !world.adopt_own(id, p).await
            {
                // The node built on another parent than the simulation's block:
                // the simulated block, under this node's key, stands for it.
                tracing::debug!(%view, "own proposal not adopted");
            }
        }
        if let Some(inputs) = consensus_input(&mut world, &my_key, &step.input).await {
            deliver(&mut harness, &world, &scenario.header, inputs).await;
        }
    }
    world.into_cache()
}

/// Hand one of the scenario's inputs to the node, as its coordinator would.
///
/// A proposal from a misbehaving leader goes through the node's proposal
/// validation first; one it refuses never reaches consensus, so it is recorded
/// as taken, with its share, and nothing done. A block that is not valid fails
/// its state validation.
async fn deliver(
    harness: &mut ConsensusHarness,
    world: &World,
    header: &format::Header,
    inputs: Vec<ConsensusInput<TestTypes>>,
) {
    if let Some(ConsensusInput::Proposal(sender, message)) = inputs.first() {
        let commitment = proposal_commitment(&message.proposal.data);
        if let Some(id) = world.block_with_commitment(&commitment)
            && !world.is_valid(id)
        {
            harness.invalidate(commitment);
        }
        if world
            .node_of(sender)
            .is_some_and(|n| header.misbehaving.contains(&n))
            && !valid_proposal(harness, message).await
        {
            let share = inputs.iter().find_map(|i| match i {
                ConsensusInput::VidShare(share) => Some(share.clone()),
                _ => None,
            });
            harness.record_refused(*sender, message.clone(), share);
            return;
        }
    }
    for input in inputs {
        harness.apply(input).await;
    }
}

/// Whether the node's proposal validation accepts `message`.
async fn valid_proposal(
    harness: &ConsensusHarness,
    message: &ProposalMessage<TestTypes, crate::message::Validated>,
) -> bool {
    let mut validator = ProposalValidator::new(
        harness.membership_coordinator.clone(),
        *harness.consensus.epoch_height,
        test_timeout_epoch_lock(),
        None,
    );
    validator.validate(ProposalMessage::unchecked(message.proposal.clone()));
    matches!(validator.next().await, Some(Ok(_)))
}

/// The inputs the Rust node takes for one of the scenario's.
async fn consensus_input(
    world: &mut World,
    me: &hotshot_types::signature_key::BLSPubKey,
    input: &Input,
) -> Option<Vec<ConsensusInput<hotshot_example_types::node_types::TestTypes>>> {
    let inputs = match *input {
        Input::Proposal {
            sender,
            block,
            share,
        } => {
            if world.node_of(me) == Some(sender) && !world.is_own(block.id) {
                return None;
            }
            let built = world.block(block.id).await;
            assert_eq!(
                world.node_of(&built.leader),
                Some(sender),
                "the scenario's leader of view {} is the harness's",
                block.view
            );
            let mut v = vec![ConsensusInput::Proposal(
                built.leader,
                ProposalMessage::validated(built.proposal.clone()),
            )];
            if share {
                v.push(ConsensusInput::VidShare(built.share_for(me)));
            }
            v
        },
        Input::Certificate1(c) => {
            let cert1 = ValidCert::new(world.cert1(c).await, EpochNumber::new(c.epoch));
            match world.state_cert(c).await {
                Some(state_cert) => {
                    vec![ConsensusInput::EpochRootCertificates { cert1, state_cert }]
                },
                None => vec![ConsensusInput::Certificate1(cert1)],
            }
        },
        Input::Certificate2(c) => {
            world.saw_cert2(c);
            vec![ConsensusInput::Certificate2(ValidCert::new(
                world.cert2(c).await,
                EpochNumber::new(c.epoch),
            ))]
        },
        Input::EpochChange {
            cert1,
            cert2,
            block,
        } => {
            world.saw_cert2(cert2);
            let c1 = world.cert1(cert1).await;
            let c2 = world.cert2(cert2).await;
            let built = world.block(block.id).await;
            vec![ConsensusInput::EpochChange(EpochChangeMessage::validated(
                c1,
                c2,
                built.proposal.data.clone(),
            ))]
        },
        Input::BlockReconstructed { view, payload } => {
            let id = world
                .block_with_payload(view, payload)
                .unwrap_or_else(|| panic!("a block at view {view} with payload {payload}"));
            let built = world.block(id).await;
            vec![ConsensusInput::BlockReconstructed {
                view: ViewNumber::new(view),
                payload_commitment: built.vid.payload_commitment,
                payload: built.payload.clone(),
            }]
        },
        Input::Revote {
            sender,
            cert,
            view,
            evidence,
        } => {
            let cert1 = world.cert1(cert).await;
            let timeout = match evidence {
                Some(tc) => Some(world.timeout_cert(tc).await),
                None => None,
            };
            let revote = ReVote {
                view: ViewNumber::new(view),
                epoch: EpochNumber::new(cert.epoch),
                cert1,
                timeout,
            };
            let key = world.key(sender).1;
            vec![ConsensusInput::ReVote(
                ReVoteMessage::new(revote, &key).expect("the leader signs the request"),
            )]
        },
        Input::Timeout(v) => vec![ConsensusInput::Timeout(ViewNumber::new(v))],
        Input::TimeoutOneHonest(v) => vec![ConsensusInput::TimeoutOneHonest(ViewNumber::new(v))],
        Input::TimeoutCertificate(tc) => vec![ConsensusInput::TimeoutCertificate(ValidCert::new(
            TimeoutEvidence::V3(world.timeout_cert(tc).await),
            EpochNumber::new(tc.epoch),
        ))],
        Input::HeaderBuilt(_) | Input::BlockValidated(_) => return None,
    };
    Some(inputs)
}
