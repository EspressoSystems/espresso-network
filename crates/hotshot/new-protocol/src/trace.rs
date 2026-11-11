//! Recording a run for the Lean specification to check.
//!
//! The rules of `lean/new-protocol-spec` read a node's history: the inputs it
//! took and the outputs it emitted. This module writes that history as a trace;
//! the `check` binary of `lean/new-protocol-diff` decides on it whether the node
//! obeyed every rule, and acted on what it owed in the step it owed it in.
//!
//! # What does not survive the translation
//!
//! The model covers the core protocol and epoch changes. The DRB and the storage
//! handshake have no counterpart. An input carrying one is written as a line
//! of its own, a dropped input, with what it is and what the node emitted on
//! it: a node waiting on one can owe for a reason that is not a bug, and a
//! checker has to see that it acted there (see [Lines](#lines)).
//!
//! A certificate that arrives inside another message and that consensus keeps
//! is recorded as received on its own, just before that message. Consensus
//! takes both at once, so the message's step has all the outputs, and the
//! certificate's step is marked as taken with it. A proposal not yet paired
//! with our share, and a fetched block, are recorded as a `proposal` without a
//! share: what the node then holds is the proposal alone. Light-client
//! certification has no counterpart either: an epoch root's state certificate
//! is left out, and only the `Cert1` it arrives with is recorded. The step of
//! a proposal carrying one the node did not hold says so, since a leader
//! proposing on an epoch root waits for it.
//!
//! One dropped input is worth watching for:
//! [`ConsensusInput::StateValidationFailed`]. The model has no way to un-admit a
//! block, so it goes on owing a vote for a block we condemned.
//!
//! Pruning is not recorded either. The specification says nothing about what a
//! node keeps, so there is nothing to check it against.
//!
//! # Identities
//!
//! A block's identity travels as the text of its commitment. The Lean side does
//! not interpret it — every rule there only ever *compares* identities — so all
//! that is asked is that distinct commitments give distinct text.
//!
//! # Lines
//!
//! A line is a step, an input the model has with the outputs it drew, or a
//! dropped input with its outputs. A checker reads a run as steps, not as
//! time: what the node owes after a step it must have acted on in that step,
//! or held back for storage. The model saw nothing between a step and a
//! dropped input after it, so a checker reads the dropped input's outputs as
//! part of that step, after its own.
//!
//! Each line also says what consensus held back there until storage confirms
//! a write, with the writes it waits for: that is where consensus decided on
//! an output it releases later. A dropped input that is such a confirmation
//! says which write it confirmed. With both, a checker can tell an action
//! released by the write it waited for from one the node took at a dropped
//! input for no reason, and an action still under way from one never released.
//! A line says as well which DRB results consensus asked for.

use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, HashMap},
    fs::create_dir_all,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use committable::Committable;
use hotshot_types::{
    data::{EpochNumber, Leaf2, VidCommitment, VidDisperseShare2, ViewNumber},
    simple_certificate::{TimeoutCertificate3, TimeoutEvidence},
    simple_vote::HasEpoch,
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    utils::epoch_from_block_number,
    vote::HasViewNumber,
};

use crate::{
    consensus::{Consensus, ConsensusInput, ConsensusOutput, DECIDE_BUFFER, recorded::HeldKind},
    helpers::{ViewEpoch, proposal_commitment},
    message::{CatchupEvidence, Certificate1, Certificate2, Proposal, ReVote, TimeoutVote},
    storage::{ActionKind, StorageOutput},
};

/// Where traces are written, if anywhere.
const TRACE_DIR: &str = "NP_TRACE_DIR";

/// Stands in for the anchor's certificate in a lock until the trace is written.
///
/// A test may not give the recorder the anchor's certificate, and a timeout
/// vote can name it before any block does. The first block on the anchor names
/// it, and the trace is held until the recorder drops, so the placeholder is
/// filled in then.
const ANCHOR_LOCK: &str = "\"<anchor lock>\"";

/// The note saying a run was, at least in part, before the certificate rule.
///
/// Before it a timeout vote and certificate carry no lock, and a node votes by
/// the rules it replaced, which the model does not have. A checker leaves such
/// a run out.
pub(crate) const BEFORE_CERTIFICATE_RULE: &str = "# before the certificate rule";

/// The note saying the test that recorded the run failed.
///
/// A failing test has left the path it was written for, often because it was
/// written for another version, so its run says nothing about whether the node
/// conforms. A checker leaves such a run out rather than report its breaks.
///
/// The note is best-effort: it is written when the recorder drops while its
/// thread panics. A test that drops the recorder before it fails, fails by
/// returning an error, or panics on another thread leaves no note.
const TEST_FAILED: &str = "# test failed";

/// The note saying no certificate for the anchor was named, so the locks that
/// stand for it are written as `null`.
///
/// The model's lock is a certificate, so a checker cannot read such a trace.
/// The note says why, rather than leaving only the decoding error.
const ANCHOR_UNKNOWN: &str = "# anchor lock unknown: no certificate for the anchor was named";

/// Distinguishes recorders sharing a label.
///
/// A test may build several nodes, and the model is a specification of *one*, so
/// each gets its own trace. Without this they would share a filename and their
/// writes would interleave into nonsense.
static NEXT_TRACE: AtomicUsize = AtomicUsize::new(0);

/// A trace being collected, or nothing at all when the environment is unset.
///
/// Recording is off unless `NP_TRACE_DIR` names a directory, so a normal test
/// run pays for a discriminant check per step and nothing else.
///
/// The whole trace is held in memory and written once, when the recorder drops.
/// Nothing is written as the run proceeds, for two reasons. A syscall per step
/// slowed the suite fourfold, which was enough for a view timer to fire and a
/// restart test to fail, and instrumentation that changes what it observes is
/// worse than none. And a test builds a recorder per node, so a file held open
/// for the length of a run costs a descriptor that the mesh's own sockets are
/// already competing for.
pub struct Recorder {
    /// Where to write on drop, or nothing when there is nothing to write to.
    path: Option<PathBuf>,
    /// The trace so far, one line per element.
    lines: Vec<String>,
    /// Whether the identity line has been written; see [`Recorder::preamble`].
    identified: bool,
    /// The numbers of the blocks consensus built a header for, by view. A view
    /// can have more than one, on different parents.
    headers: BTreeMap<ViewNumber, BTreeSet<u64>>,
    /// Epochs and views whose leader has been written; see [`Recorder::leader`].
    led: BTreeMap<(EpochNumber, ViewNumber), String>,
    /// Steps written so far, which is the index the next one will have.
    ///
    /// A test's placement of the node is noted with this number.
    steps: usize,
    /// Blocks to an epoch on this network, as [`Recorder::preamble`] was told.
    ///
    /// A decided leaf carries no epoch of its own, so this is what turns its
    /// block number into one.
    epoch_height: u64,
    /// The anchor's certificate, as [`Recorder::preamble`] was given it or the
    /// first block on the anchor named it.
    ///
    /// The lock of a node locked on nothing but the anchor. Rust leaves that
    /// lock out of a timeout vote or certificate; the model names the anchor's
    /// certificate.
    anchor_lock: Option<String>,
    /// Whether the run was, at least in part, before the certificate rule: it
    /// took an input at a view before it, or had a timeout vote or certificate
    /// that signs no lock. The model has none such.
    before_certificate_rule: Cell<bool>,
    /// A certificate for the anchor to name when the run gives none, as
    /// [`Recorder::fallback_anchor`] was told.
    fallback_anchor: Option<String>,
    /// The timeout evidence of each proposal the run had, by its identity.
    ///
    /// A decided leaf no longer carries it, and the model's decided block does.
    evidence: RefCell<HashMap<String, String>>,
}

/// What a step is read against of consensus as it was before the input; see
/// [`Recorder::before`].
#[derive(Default)]
pub struct Before {
    /// Whether consensus refuses the input whole, keeping none of the
    /// certificates it carries.
    refused: bool,
    /// What consensus held back for storage, by kind, view and epoch, each
    /// with the writes it waited for.
    held_back: BTreeMap<(HeldKind, ViewEpoch), Vec<String>>,
    /// The epochs consensus held a state certificate for.
    state_cert_epochs: BTreeSet<EpochNumber>,
}

impl Recorder {
    /// A recorder collecting a trace for `$NP_TRACE_DIR/<label>.jsonl`, or an inert one.
    pub fn new(label: &str) -> Self {
        let Ok(dir) = std::env::var(TRACE_DIR) else {
            return Self::writing_to(None);
        };
        let dir = PathBuf::from(dir);
        if let Err(err) = create_dir_all(&dir) {
            eprintln!("trace: cannot create {}: {err}", dir.display());
            return Self::writing_to(None);
        }
        // A label may name a test, and test names contain `::`.
        let name = label.replace("::", "-");
        let seq = NEXT_TRACE.fetch_add(1, Ordering::Relaxed);
        Self::writing_to(Some(dir.join(format!("{name}.{seq}.jsonl"))))
    }

    /// A recorder writing its trace to `path`, or an inert one if there is none.
    pub fn writing_to(path: Option<PathBuf>) -> Self {
        Self {
            identified: path.is_none(),
            path,
            lines: Vec::new(),
            headers: BTreeMap::new(),
            led: BTreeMap::new(),
            steps: 0,
            epoch_height: 0,
            anchor_lock: None,
            before_certificate_rule: Cell::new(false),
            fallback_anchor: None,
            evidence: RefCell::new(HashMap::new()),
        }
    }

    /// The file the trace is written to, unless the recorder is inert.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// A recorder labelled by the running test, which is this thread's name.
    pub fn for_current_test() -> Self {
        Self::new(std::thread::current().name().unwrap_or("trace"))
    }

    /// Write the identity line, once, before the first step.
    ///
    /// A checker needs two things the steps do not carry: which node this is, and
    /// the block its chain is anchored at. Neither can be recovered from the
    /// steps in general — a run that casts no vote never names its own key, and
    /// one that starts above genesis never cites the anchor — so a checker left
    /// to infer them has to guess, and a wrong guess looks like a fault.
    ///
    /// Written on the comment channel: a reader that only knows about steps skips
    /// it, as it already skips the dropped-input notes.
    ///
    /// Call before applying the input, so a first step that decides cannot move
    /// the anchor out from under the line describing where the run began.
    ///
    /// `anchor_cert` is the certificate the node holds at the genesis view. The
    /// first proposal names it as its parent, and a fresh node is locked on it,
    /// so the rules have to start from that certificate rather than from one
    /// made up from the anchor leaf.
    pub fn preamble<T: NodeType>(
        &mut self,
        node: &T::SignatureKey,
        anchor: &Leaf2<T>,
        anchor_cert: Option<&Certificate1<T>>,
        epoch_height: u64,
    ) {
        if self.identified {
            return;
        }
        self.identified = true;
        self.epoch_height = epoch_height;
        self.anchor_lock = anchor_cert.map(cert1_json_raw);
        if self.path.is_none() {
            return;
        }
        let mut fields = vec![
            ("node", ident(node)),
            ("anchor", ident(&anchor.commit())),
            // After genesis only for a node restored from storage, which the
            // model does not cover.
            ("anchorView", view_json(anchor.view_number())),
            ("decideBuffer", DECIDE_BUFFER.to_string()),
            ("epochHeight", epoch_height.to_string()),
        ];
        if let Some(cert) = &self.anchor_lock {
            fields.push(("anchorCert", cert.clone()));
        }
        let line = obj(&fields);
        self.lines.push(format!("# trace {line}"));
    }

    /// Name a certificate for the anchor, for a run that never names one.
    ///
    /// A run whose node is never handed the anchor's certificate and never
    /// sees a block on the anchor can still sign it as a timeout lock. The
    /// checker then needs some certificate for the anchor, and this one is
    /// written into the identity line.
    pub fn fallback_anchor<T: NodeType>(&mut self, cert: &Certificate1<T>) {
        if self.path.is_some() && self.fallback_anchor.is_none() {
            self.fallback_anchor = Some(cert1_json_raw(cert));
        }
    }

    /// Record that the node holds `proposal` without having received it, as a
    /// test that seeds a node's state directly does.
    ///
    /// Written as a step receiving the proposal alone, which is the most the node
    /// could have been given: holding it, not having voted on it.
    pub fn held<T: NodeType>(&mut self, proposal: &Proposal<T>) {
        if self.path.is_none() {
            return;
        }
        let input = bare_proposal_json(NO_SENDER, self.proposal_json(proposal));
        self.push_step(input, Vec::new(), no_notes());
    }

    /// Record that the node holds the payload of `proposal` without having
    /// reconstructed it, as a test that restores a lock on it does.
    ///
    /// A node locks only on a block whose payload it holds, so a restored lock
    /// stands for a payload the node had before it restarted. That holds only
    /// if the payload was persisted with the lock, which a real restart does
    /// not ensure: the two are written by separate storage tasks (see
    /// `Consensus::parent_reconstructed`).
    pub fn payload_held<T: NodeType>(&mut self, proposal: &Proposal<T>) {
        if self.path.is_none() {
            return;
        }
        let input = tagged(
            "blockReconstructed",
            obj(&[
                ("v", view_json(proposal.view_number)),
                ("c", payload_json(&proposal.block_header)),
            ]),
        );
        self.push_step(input, Vec::new(), no_notes());
    }

    /// Record that the node holds `cert`, as a test that restores a lock on it
    /// does.
    pub fn cert1_held<T: NodeType>(&mut self, cert: &Certificate1<T>) {
        if self.path.is_none() {
            return;
        }
        self.push_step(certificate1_json(cert), Vec::new(), no_notes());
    }

    /// Record that a test placed the node at `view` in `epoch` directly.
    ///
    /// No input puts a node there, so the view and epoch a checker derives from
    /// the steps are not the node's. Written on the comment channel with the
    /// step it precedes, so a checker can leave the rules that read the view and
    /// epoch unchecked rather than report the placement as a fault.
    pub fn placed(&mut self, view: ViewNumber, epoch: EpochNumber) {
        if self.path.is_none() {
            return;
        }
        self.lines.push(format!(
            "# placed at view {view} epoch {epoch} (at step {})",
            self.steps
        ));
    }

    /// Record that a test restored the node's lock on `view` directly.
    ///
    /// The steps cannot show how the node came to hold the lock, nor what it did
    /// on it before the restart, so what a checker derives from them as owed is
    /// not what the node owes. Written like [`Recorder::placed`], so a checker
    /// can leave the rules that read what the node owes unchecked.
    pub fn lock_restored(&mut self, view: ViewNumber) {
        if self.path.is_none() {
            return;
        }
        self.lines.push(format!(
            "# placed lock at view {view} (at step {})",
            self.steps
        ));
    }

    /// Record that a test resumed the node as after a restart, which bars it
    /// from acting again in the views up to `view`.
    ///
    /// The steps do not show what the node did before the restart, so what a
    /// checker derives from them as owed in those views is not what the node
    /// owes. Written like [`Recorder::placed`], so a checker can leave the
    /// rules that read what the node owes unchecked.
    pub fn restarted(&mut self, view: ViewNumber) {
        if self.path.is_none() {
            return;
        }
        self.lines.push(format!(
            "# placed restart after view {view} (at step {})",
            self.steps
        ));
    }

    /// Record that a test replaced the node's proposal for `view` directly.
    ///
    /// No input does that, so the blocks a checker derives from the steps are not
    /// the node's. Written like [`Recorder::placed`], so a checker can leave the
    /// rules that read what the node holds unchecked.
    pub fn forced(&mut self, view: ViewNumber) {
        if self.path.is_none() {
            return;
        }
        self.lines.push(format!(
            "# forced proposal at view {view} (at step {})",
            self.steps
        ));
    }

    /// Whether anything is being recorded.
    ///
    /// Lets a caller skip work that only a trace needs — and, for the leader of a
    /// view, skip a lookup whose failure should stop a recorded run but has no
    /// business stopping an ordinary one.
    pub fn recording(&self) -> bool {
        self.path.is_some()
    }

    /// Name the leader of `view` in `epoch`, once, or say that it cannot be named.
    ///
    /// Every view has a leader, and the model takes the schedule as a parameter,
    /// so a trace that does not say who leads leaves a checker with no way to tell
    /// a proposal this node was entitled to make from one it was not: assuming
    /// this node leads everywhere makes the leader clause of
    /// `ProposalJustified` unfalsifiable.
    ///
    /// `None` is written as `unknown` rather than left out. A stake table that
    /// cannot answer for an epoch it does not have leaves consensus unable to
    /// claim leadership either, and the rules, which then find no leader, agree
    /// with it; but silence would not say whether the leader was unknown or merely
    /// unrecorded.
    ///
    /// The leader is named per epoch: at a boundary the outgoing committee's
    /// leader may ask for a re-vote in the view the incoming committee's leader
    /// proposes in. An `unknown` is written again once the leader is known.
    ///
    /// Written on the comment channel, like the identity line, so a reader that
    /// only knows about steps skips it.
    pub fn leader<T: NodeType>(
        &mut self,
        view: ViewNumber,
        epoch: EpochNumber,
        leader: Option<&T::SignatureKey>,
    ) {
        let known = self
            .led
            .get(&(epoch, view))
            .is_some_and(|named| named != "unknown");
        if self.path.is_none() || known {
            return;
        }
        let named = match leader {
            Some(key) => ident(key),
            None => "unknown".to_string(),
        };
        if self.led.get(&(epoch, view)) == Some(&named) {
            return;
        }
        self.lines.push(format!("# leader {view} {epoch} {named}"));
        self.led.insert((epoch, view), named);
    }

    /// What the step for `input` is read against of `consensus` as it is
    /// before the input, which may change it.
    ///
    /// Consensus refuses whole, keeping none of the certificates it carries, a
    /// re-vote request at or before the decide floor, or of an epoch earlier
    /// than the node's, as `handle_revote` decides it. What consensus holds
    /// back for storage, and the state certificates it holds, are compared
    /// with what it has after the input.
    pub fn before<T: NodeType>(
        &self,
        consensus: &Consensus<T>,
        input: &ConsensusInput<T>,
    ) -> Before {
        if self.path.is_none() {
            return Before::default();
        }
        Before {
            refused: matches!(
                input,
                ConsensusInput::ReVote(message)
                    if message.revote.view <= consensus.decide_floor()
                        || message.revote.epoch < consensus.current_epoch()
            ),
            held_back: consensus
                .held_back()
                .iter()
                .map(|held| {
                    let waits = held.waits.iter().map(write_json).collect();
                    ((held.kind, held.key), waits)
                })
                .collect(),
            state_cert_epochs: consensus.state_cert_epochs(),
        }
    }

    /// Record one step: the input taken, and what it drew, on `consensus` as
    /// it is after the input. `before` is what [`Recorder::before`] took of
    /// it before the input.
    pub fn record<'a, T: NodeType + 'a>(
        &mut self,
        consensus: &Consensus<T>,
        before: Before,
        input: &ConsensusInput<T>,
        outputs: impl Iterator<Item = &'a ConsensusOutput<T>>,
    ) {
        if self.path.is_none() {
            return;
        }
        let outputs: Vec<_> = outputs.collect();
        let mut steps: Vec<(String, Vec<String>)> = Vec::new();
        // The step, of those written here, at which a state certificate the
        // node did not hold arrived.
        let mut state_cert = None;
        self.learn_anchor_lock(input, &outputs);

        // A node at a view before the certificate rule, or an input for one,
        // puts the run outside the model. These two inputs belong to no view.
        let input_view = match input {
            ConsensusInput::DrbResult(..) | ConsensusInput::UpgradeCertificateFormed(..) => None,
            _ => Some(input.view_number()),
        };
        let upgrade_lock = consensus.upgrade_lock();
        if std::iter::once(consensus.current_view())
            .chain(input_view)
            .any(|view| !upgrade_lock.certificate_rule(view))
        {
            self.before_certificate_rule.set(true);
        }

        // The model takes a proposal already paired with our share, and pairs
        // nothing itself. Rather than reassemble the pair here — where a bug
        // would look like a fault — take it from the output that reports
        // the pairing, as a step of its own. The parent's `Cert1`, the timeout
        // certificate and its lock, and the `Cert2` a proposal opening an
        // epoch comes with, are kept on pairing, so each is received just
        // before. Only a timeout certificate's recent lock is verified, and
        // only a recent one is kept.
        //
        // Consensus may refuse the proposal right after pairing it, and then
        // keeps none of its certificates. A proposal it took is the one it
        // holds as paired with this node's share. A duplicate of that one is
        // refused too, but its certificates were kept with the first copy.
        for output in &outputs {
            if let ConsensusOutput::ProposalPaired {
                proposal,
                vid_share,
            } = output
            {
                let commit = proposal_commitment(&proposal.data);
                let taken = consensus
                    .proposals()
                    .live(proposal.data.view_number)
                    .is_some_and(|live| proposal_commitment(live) == commit);
                if taken {
                    steps.push((certificate1_json(&proposal.data.justify_qc), Vec::new()));
                    if let Some(tc) = &proposal.data.view_change_evidence {
                        if let Some(lock) = tc.timeout_lock().recent() {
                            steps.push((certificate1_json(lock), Vec::new()));
                        }
                        steps.push((
                            tagged(
                                "timeoutCertificate",
                                obj(&[("c", self.timeout_cert_json(tc))]),
                            ),
                            Vec::new(),
                        ));
                    }
                    if let Some(cert2) = &proposal.data.next_epoch_justify_qc {
                        steps.push((certificate2_json(cert2), Vec::new()));
                    }
                }
                let key = (proposal.data.epoch, proposal.data.view_number);
                let sender = self.led.get(&key).cloned();
                steps.push((
                    paired_proposal_json(self.proposal_json(&proposal.data), vid_share, sender),
                    Vec::new(),
                ));
                // A leader proposing on an epoch root needs the state
                // certificate, which may first arrive with this proposal.
                if proposal.data.state_cert.as_ref().is_some_and(|sc| {
                    !before.state_cert_epochs.contains(&sc.epoch)
                        && consensus.state_cert_epochs().contains(&sc.epoch)
                }) {
                    state_cert = Some(steps.len() - 1);
                }
            }
        }

        // A certificate carried inside another message is not held by
        // receiving the message (`History.HasCert1`). Consensus keeps the
        // lock of a timeout certificate, and the timeout certificate and the
        // certificate of a re-vote request, so each is received just before
        // the input carrying it. It keeps none of a re-vote request it refuses.
        if !before.refused {
            for carried in self.carried_steps(input) {
                steps.push((carried, Vec::new()));
            }
        }

        // The input first: a block decided in the same step is written with the
        // timeout evidence of the proposal this input carries.
        let taken = self.input_json(input);
        let emitted: Vec<String> = outputs
            .iter()
            .copied()
            .filter_map(|o| self.output_json(o))
            .collect();

        // An output held back is released on a later input, but consensus
        // decided on it here, for a checker to attribute the release to. Each
        // is written with the writes it waits for, and again if it comes to
        // wait for another. A timer for a view whose timeout vote is held
        // back already is answered by that vote, so the vote is written again.
        let timer = match input {
            ConsensusInput::Timeout(view) | ConsensusInput::TimeoutOneHonest(view) => Some(*view),
            _ => None,
        };
        let held: Vec<String> = consensus
            .held_back()
            .iter()
            .filter_map(|held| {
                let waits: Vec<String> = held.waits.iter().map(write_json).collect();
                let known = before
                    .held_back
                    .get(&(held.kind, held.key))
                    .is_some_and(|waited| waits.iter().all(|write| waited.contains(write)));
                let answers = held.kind == HeldKind::Timeout && Some(held.key.0) == timer;
                (!known || answers).then_some((held, waits))
            })
            .map(|(held, waits)| {
                obj(&[
                    ("kind", quoted(held.kind.name())),
                    ("view", view_json(held.key.0)),
                    ("epoch", (*held.key.1).to_string()),
                    ("waits", arr(&waits)),
                ])
            })
            .collect();
        // A DRB result consensus asks for, with the block it wants it for: a
        // proposal it took here, or the header it built to propose in its
        // view. A checker reads from the block whether its vote or proposal
        // waits for the result.
        if let ConsensusInput::HeaderCreated(view, _, header) = input {
            self.headers
                .entry(*view)
                .or_default()
                .insert(header.block_number());
        }
        let wanted_for: Vec<(EpochNumber, ViewNumber, u64)> = outputs
            .iter()
            .filter_map(|output| match output {
                ConsensusOutput::ProposalPaired { proposal, .. } => Some((
                    proposal.data.epoch,
                    proposal.data.view_number,
                    proposal.data.block_header.block_number(),
                )),
                _ => None,
            })
            // Consensus proposes in its view and the next, on the header built
            // for either.
            .chain(
                [consensus.current_view(), consensus.current_view() + 1]
                    .into_iter()
                    .flat_map(|view| {
                        let epoch_height = self.epoch_height;
                        let blocks = self.headers.get(&view).into_iter().flatten();
                        blocks.map(move |block| {
                            let epoch = epoch_from_block_number(*block, epoch_height);
                            (EpochNumber::new(epoch), view, *block)
                        })
                    }),
            )
            .collect();
        let drb_requested: Vec<String> = outputs
            .iter()
            .filter_map(|output| match output {
                ConsensusOutput::RequestDrbResult(epoch) => Some(*epoch),
                _ => None,
            })
            .flat_map(|drb| {
                let request = |view: String, block: String| {
                    obj(&[
                        ("epoch", (*drb).to_string()),
                        ("view", view),
                        ("block", block),
                    ])
                };
                let named: Vec<String> = wanted_for
                    .iter()
                    .filter(|(epoch, ..)| *epoch + 1 == drb)
                    .map(|(_, view, block)| request(view_json(*view), block.to_string()))
                    .collect();
                if named.is_empty() {
                    vec![request("null".to_string(), "null".to_string())]
                } else {
                    named
                }
            })
            .collect();

        // An epoch root's `Cert1` arrives with its state certificate, which a
        // leader proposing on it waits for.
        if let ConsensusInput::EpochRootCertificates { state_cert: sc, .. } = input
            && !before.state_cert_epochs.contains(&sc.epoch)
            && consensus.state_cert_epochs().contains(&sc.epoch)
        {
            state_cert = Some(steps.len());
        }

        match taken {
            Ok(json) => steps.push((json, emitted)),
            // The model has no such input. A step written for it, the proposal
            // it paired, takes its outputs. Otherwise it is written as a
            // dropped input with them: a trace that discarded them would show
            // a node that never voted, since the votes a node holds back until
            // storage confirms a write are released on exactly these inputs.
            Err(dropped) => match steps.last_mut() {
                Some((_, outputs)) => outputs.extend(emitted),
                None => {
                    let notes = notes_json(false, &held, &drb_requested, false);
                    self.lines.push(tagged(
                        "dropped",
                        obj(&[
                            ("kind", dropped),
                            ("output", arr(&emitted)),
                            ("notes", notes),
                        ]),
                    ));
                    return;
                },
            },
        }

        // Consensus took every step written here in one go, so all its outputs,
        // and what it held back, are on the last. A checker reads the earlier
        // ones as part of it.
        let last = steps.len().saturating_sub(1);
        for (i, (input, outputs)) in steps.into_iter().enumerate() {
            let notes = if i < last {
                notes_json(true, &[], &[], state_cert == Some(i))
            } else {
                notes_json(false, &held, &drb_requested, state_cert == Some(i))
            };
            self.push_step(input, outputs, notes);
        }
    }

    /// Record a proposal the node took and refused before consensus saw it, with
    /// the share it came with if any: a step that emits nothing.
    pub fn refused<T: NodeType>(
        &mut self,
        sender: &T::SignatureKey,
        proposal: &Proposal<T>,
        share: Option<&VidDisperseShare2<T>>,
    ) {
        if self.path.is_none() {
            return;
        }
        let body = self.proposal_json(proposal);
        let input = match share {
            Some(share) => paired_proposal_json(body, share, Some(ident(sender))),
            None => bare_proposal_json(&ident(sender), body),
        };
        self.push_step(input, Vec::new(), no_notes());
    }

    /// Write a step with what it says beyond its input and outputs.
    fn push_step(&mut self, input: String, outputs: Vec<String>, notes: String) {
        self.lines.push(tagged(
            "consensus",
            obj(&[
                ("input", input),
                ("output", arr(&outputs)),
                ("notes", notes),
            ]),
        ));
        self.steps += 1;
    }

    /// Take the anchor's certificate from the first block on the anchor, if
    /// it is not known yet.
    fn learn_anchor_lock<T: NodeType>(
        &mut self,
        input: &ConsensusInput<T>,
        outputs: &[&ConsensusOutput<T>],
    ) {
        if self.anchor_lock.is_some() {
            return;
        }
        let received = match input {
            ConsensusInput::Proposal(_, message) | ConsensusInput::FetchedProposal(message) => {
                Some(&message.proposal.data)
            },
            ConsensusInput::EpochChange(change) => Some(&change.proposal),
            _ => None,
        };
        let made = outputs.iter().filter_map(|output| match output {
            ConsensusOutput::SendProposal(signed) => Some(&signed.data),
            ConsensusOutput::ProposalPaired { proposal, .. } => Some(&proposal.data),
            _ => None,
        });
        self.anchor_lock = received
            .into_iter()
            .chain(made)
            .map(|proposal| &proposal.justify_qc)
            .find(|cert| cert.view_number() == ViewNumber::genesis())
            .map(cert1_json_raw);
    }
}

/// Write the trace, and say what could not be written.
///
/// A run that recorded nothing leaves no file: a test that never reached the
/// consensus core has no trace to speak of, and an empty one would only have to
/// be skipped by every reader.
///
/// Outputs still held back are noted rather than lost in silence: a run that
/// wrote no step has no step for them to join, and a checker would otherwise
/// pass a node whose actions it never saw.
impl Drop for Recorder {
    fn drop(&mut self) {
        let Some(path) = self.path.as_ref() else {
            return;
        };
        if self.lines.is_empty() {
            return;
        }
        // A test fails by panicking, and the recorder drops while it unwinds.
        if std::thread::panicking() {
            self.lines.push(TEST_FAILED.to_string());
        }
        if self.before_certificate_rule.get() {
            self.lines
                .insert(1.min(self.lines.len()), BEFORE_CERTIFICATE_RULE.to_string());
        }
        let mut text = self.lines.join("\n");
        text.push('\n');
        let text = fill_anchor_lock(
            &text,
            self.anchor_lock.as_deref(),
            self.fallback_anchor.as_deref(),
        );
        if let Err(err) = std::fs::write(path, text) {
            eprintln!("trace: cannot write {}: {err}", path.display());
        }
    }
}

/// `text` with the placeholder for the anchor's certificate filled in.
///
/// With no certificate named for the anchor, `fallback` is used and written into
/// the identity line, so the checker holds it too. With neither, the locks are
/// written as `null` and the trace says why.
fn fill_anchor_lock(text: &str, anchor: Option<&str>, fallback: Option<&str>) -> String {
    if !text.contains(ANCHOR_LOCK) {
        return text.to_string();
    }
    match (anchor, fallback) {
        (Some(anchor), _) => text.replace(ANCHOR_LOCK, anchor),
        (None, Some(fallback)) => text
            .replacen(
                "# trace {",
                &format!("# trace {{\"anchorCert\":{fallback},"),
                1,
            )
            .replace(ANCHOR_LOCK, fallback),
        (None, None) => format!("{}{ANCHOR_UNKNOWN}\n", text.replace(ANCHOR_LOCK, "null")),
    }
}

/// What a line says beyond its input and outputs: whether the step was taken
/// with the next, what consensus held back, the DRB results it asked for, and
/// whether a state certificate it did not hold arrived.
fn notes_json(joined: bool, held: &[String], drb_requested: &[String], state_cert: bool) -> String {
    obj(&[
        ("joined", joined.to_string()),
        ("held", arr(held)),
        ("drbRequested", arr(drb_requested)),
        ("stateCert", state_cert.to_string()),
    ])
}

/// The notes of a line that says nothing beyond its input and outputs.
fn no_notes() -> String {
    notes_json(false, &[], &[], false)
}

/// A write storage confirms, as the checker's `Write`.
fn write_json<T: NodeType>(write: &StorageOutput<T>) -> String {
    match write {
        StorageOutput::Proposal(view, commit) => tagged(
            "proposal",
            obj(&[("v", view_json(*view)), ("c", ident(commit))]),
        ),
        StorageOutput::Vid(view) => tagged("share", obj(&[("v", view_json(*view))])),
        StorageOutput::Action(view, kind) => {
            let kind = match kind {
                ActionKind::Vote => "vote",
                ActionKind::Propose => "propose",
                ActionKind::Timeout => "timeout",
            };
            tagged(
                "action",
                obj(&[("v", view_json(*view)), ("kind", quoted(kind))]),
            )
        },
        StorageOutput::HighQc(lock) => tagged(
            "lock",
            obj(&[
                ("v", view_json(lock.view)),
                ("e", (*lock.epoch).to_string()),
            ]),
        ),
    }
}

/// An input the model has no counterpart for, as the checker's `Dropped`, by
/// the name of its variant.
fn other_json(name: &str) -> String {
    tagged("other", obj(&[("name", quoted(name))]))
}

/// A proposal and this node's share for it, as the model's `Input.proposal`.
///
/// The sender is the leader named for the proposal's view. The model does not
/// carry signatures, so the sender is what authenticates a proposal: a vote1 is
/// owed only on one the leader sent. Consensus admitted this one because the
/// leader signed it, and the leader named for the view is the key it checked.
/// When no leader was named, the share's recipient stands in.
fn paired_proposal_json<T: NodeType>(
    proposal: String,
    share: &VidDisperseShare2<T>,
    leader: Option<String>,
) -> String {
    let sender = match leader {
        Some(named) if named != "unknown" => named,
        _ => ident(&share.recipient_key),
    };
    tagged(
        "proposal",
        obj(&[
            ("sender", sender),
            ("p", proposal),
            (
                "share",
                obj(&[
                    ("view", view_json(share.view_number)),
                    ("payloadCommit", ident(&share.payload_commitment)),
                ]),
            ),
        ]),
    )
}

/// The sender of a proposal whose sender the recorder does not know.
///
/// A seeded proposal, or a fetched one whose view's leader was not named. Such a
/// proposal comes without its share, and no rule reads the sender of one: a
/// vote1 needs the share.
const NO_SENDER: &str = "\"\"";

/// A proposal without this node's share, as the model's `Input.proposal`.
fn bare_proposal_json(sender: &str, proposal: String) -> String {
    tagged(
        "proposal",
        obj(&[
            ("sender", sender.to_string()),
            ("p", proposal),
            ("share", "null".to_string()),
        ]),
    )
}

impl Recorder {
    /// One input, as the model's `Input`.
    ///
    /// An input the model has no counterpart for is returned as the checker's
    /// `Dropped`.
    fn input_json<T: NodeType>(&self, input: &ConsensusInput<T>) -> Result<String, String> {
        Ok(match input {
            ConsensusInput::BlockReconstructed {
                view,
                payload_commitment: commit,
                ..
            }
            | ConsensusInput::VidDisperseCreated(view, commit) => tagged(
                "blockReconstructed",
                obj(&[("v", view_json(*view)), ("c", ident(commit))]),
            ),
            ConsensusInput::Certificate1(cert) | ConsensusInput::AdvanceView(cert) => {
                certificate1_json(cert)
            },
            // The pairing is epoch machinery, but the certificate is an ordinary
            // `Cert1` and this is the only way consensus receives it at an epoch
            // root — dropping the variant would lose a certificate the model wants.
            ConsensusInput::EpochRootCertificates { cert1, .. } => certificate1_json(cert1),
            ConsensusInput::Certificate2(cert) => certificate2_json(cert),
            ConsensusInput::HeaderCreated(view, parent, header) => tagged(
                "headerBuilt",
                obj(&[
                    ("v", view_json(*view)),
                    ("parent", ident(parent)),
                    ("h", header_json::<T>(header)),
                ]),
            ),
            ConsensusInput::StateValidated(response) => tagged(
                "blockValidated",
                obj(&[
                    ("v", view_json(response.view)),
                    ("h", ident(&response.commitment)),
                ]),
            ),
            ConsensusInput::Timeout(view) => tagged("timeout", obj(&[("v", view_json(*view))])),
            ConsensusInput::TimeoutOneHonest(view) => {
                tagged("timeoutOneHonest", obj(&[("v", view_json(*view))]))
            },
            ConsensusInput::TimeoutCertificate(cert) => tagged(
                "timeoutCertificate",
                obj(&[("c", self.timeout_cert_json(cert))]),
            ),
            // A proposal paired with our share is emitted from `ProposalPaired`
            // instead of reassembled here. Until then, and for a fetched proposal,
            // what the node holds is the proposal alone. A fetched proposal names
            // the leader of its view as its sender, which signed it.
            ConsensusInput::Proposal(sender, message) => {
                bare_proposal_json(&ident(sender), self.proposal_json(&message.proposal.data))
            },
            ConsensusInput::FetchedProposal(message) => {
                let proposal = &message.proposal.data;
                let sender = self
                    .led
                    .get(&(proposal.epoch, proposal.view_number))
                    .filter(|named| *named != "unknown")
                    .map_or(NO_SENDER, String::as_str);
                bare_proposal_json(sender, self.proposal_json(proposal))
            },
            ConsensusInput::VidShare(..) => return Err(other_json("VidShare")),
            ConsensusInput::StateValidationFailed(response) => {
                return Err(tagged(
                    "stateValidationFailed",
                    obj(&[
                        ("v", view_json(response.view)),
                        ("h", ident(&response.commitment)),
                    ]),
                ));
            },
            ConsensusInput::BlockBuilt { view, .. } => {
                return Err(tagged("blockBuilt", obj(&[("v", view_json(*view))])));
            },
            ConsensusInput::Stored(write) => {
                return Err(tagged("stored", obj(&[("w", write_json(write))])));
            },
            ConsensusInput::EpochChange(epoch_change) => tagged(
                "epochChange",
                obj(&[
                    ("c1", cert1_json_raw(&epoch_change.cert1)),
                    ("c2", cert2_json_raw(&epoch_change.cert2)),
                    ("p", self.proposal_json(&epoch_change.proposal)),
                ]),
            ),
            ConsensusInput::DrbResult(epoch, _) => {
                return Err(tagged(
                    "drbResult",
                    obj(&[("epoch", (**epoch).to_string())]),
                ));
            },
            ConsensusInput::ReVote(message) => {
                let revote = &message.revote;
                // Unnamed when the node's membership does not have the request's
                // epoch, as for a fetched proposal.
                let sender = self
                    .led
                    .get(&(revote.epoch, revote.view))
                    .filter(|named| *named != "unknown")
                    .map_or(NO_SENDER, String::as_str)
                    .to_string();
                tagged(
                    "revote",
                    obj(&[("sender", sender), ("r", self.revote_json(revote))]),
                )
            },
            ConsensusInput::UpgradeCertificateFormed(..) => {
                return Err(other_json("UpgradeCertificateFormed"));
            },
        })
    }

    /// One output, as the model's `Output`, or nothing when the model has no such output.
    fn output_json<T: NodeType>(&self, output: &ConsensusOutput<T>) -> Option<String> {
        let message = match output {
            ConsensusOutput::SendVote1(vote) => tagged(
                "vote1",
                obj(&[(
                    "v",
                    obj(&[
                        (
                            "data",
                            obj(&[
                                ("blockHash", ident(&vote.vote.data.leaf_commit)),
                                ("epoch", epoch_json(HasEpoch::epoch(&vote.vote.data))),
                                (
                                    "blockNumber",
                                    vote.vote.data.block_number.unwrap_or(0).to_string(),
                                ),
                            ]),
                        ),
                        ("view", view_json(vote.vote.view_number)),
                        ("signer", ident(&vote.vote.signature.0)),
                    ]),
                )]),
            ),
            ConsensusOutput::SendVote2(vote) => tagged(
                "vote2",
                obj(&[(
                    "v",
                    obj(&[
                        (
                            "data",
                            obj(&[
                                ("blockHash", ident(&vote.data.leaf_commit)),
                                ("epoch", epoch_json(HasEpoch::epoch(&vote.data))),
                                ("blockNumber", vote.data.block_number.to_string()),
                            ]),
                        ),
                        ("view", view_json(vote.view_number)),
                        ("signer", ident(&vote.signature.0)),
                    ]),
                )]),
            ),
            ConsensusOutput::SendProposal(signed) => {
                tagged("proposal", obj(&[("p", self.proposal_json(&signed.data))]))
            },
            ConsensusOutput::SendReVote(message) => {
                tagged("revote", obj(&[("r", self.revote_json(&message.revote))]))
            },
            ConsensusOutput::SendTimeoutVote(vote, evidence) => tagged(
                "timeoutVote",
                obj(&[
                    (
                        "v",
                        obj(&[
                            (
                                "data",
                                obj(&[
                                    ("epoch", epoch_json(vote.epoch())),
                                    ("lock", self.timeout_vote_lock_json(vote)),
                                ]),
                            ),
                            ("view", view_json(vote.view_number())),
                            ("signer", ident(&vote.signing_key())),
                        ]),
                    ),
                    ("e", self.evidence_json(evidence.as_ref())),
                ]),
            ),
            ConsensusOutput::SendTimeoutCertificate(cert, view, _) => tagged(
                "timeoutCert",
                obj(&[("c", self.timeout_cert_json(cert)), ("v", view_json(*view))]),
            ),
            ConsensusOutput::SendCertificate1(cert) => {
                tagged("cert1", obj(&[("c", cert1_json_raw(cert))]))
            },
            ConsensusOutput::SendCertificate2(cert) => {
                tagged("cert2", obj(&[("c", cert2_json_raw(cert))]))
            },
            ConsensusOutput::SendEpochChange(epoch_change) => tagged(
                "epochChange",
                obj(&[
                    ("c1", cert1_json_raw(&epoch_change.cert1)),
                    ("c2", cert2_json_raw(&epoch_change.cert2)),
                    ("p", self.proposal_json(&epoch_change.proposal)),
                ]),
            ),
            // A decide is not a message; it leaves through the other arm of `Output`.
            ConsensusOutput::LeafDecided {
                leaves,
                cert1,
                cert2,
                ..
            } => {
                // The model's decide carries a `Cert2`; without one there is nothing
                // to compare, and the views will arrive with a later decide anyway.
                let cert2 = cert2.as_ref()?;
                let blocks: Vec<String> = leaves
                    .iter()
                    .map(|l| leaf_json(l, self.epoch_height, &self.evidence.borrow()))
                    .collect();
                return Some(tagged(
                    "decided",
                    obj(&[
                        ("blocks", arr(&blocks)),
                        ("c1", cert1_json_raw(cert1)),
                        ("c2", cert2_json_raw(cert2)),
                    ]),
                ));
            },
            // Reported as the model's paired-proposal *input*, above.
            ConsensusOutput::ProposalPaired { .. } => return None,
            // Everything else the model does not emit: the requests it has no
            // outputs for, the storage handshake, the epoch machinery, and the
            // notifications (`LockUpdated`, `ViewChanged`, `ViewTimedOut`).
            _ => return None,
        };
        Some(tagged("send", obj(&[("m", message)])))
    }

    /// A proposal we sent or received.
    fn proposal_json<T: NodeType>(&self, proposal: &Proposal<T>) -> String {
        let identity = ident(&crate::helpers::proposal_commitment(proposal));
        let evidence = match &proposal.view_change_evidence {
            Some(tc) => self.timeout_cert_json(tc),
            None => "null".to_string(),
        };
        self.evidence
            .borrow_mut()
            .insert(identity.clone(), evidence.clone());
        obj(&[
            ("blockHeader", header_json::<T>(&proposal.block_header)),
            ("viewNumber", view_json(proposal.view_number)),
            ("epoch", proposal.epoch.to_string()),
            ("parentCert", cert1_json_raw(&proposal.justify_qc)),
            ("timeoutEvidence", evidence),
            ("identity", identity),
        ])
    }

    /// A re-vote request, as the model's `RevoteRequest`.
    fn revote_json<T: NodeType>(&self, revote: &ReVote<T>) -> String {
        obj(&[
            ("cert", cert1_json_raw(&revote.cert1)),
            ("view", view_json(revote.view)),
            (
                "timeoutEvidence",
                match &revote.timeout {
                    Some(tc) => self.timeout_cert3_json(tc),
                    None => "null".to_string(),
                },
            ),
        ])
    }

    fn evidence_json<T: NodeType>(&self, evidence: Option<&CatchupEvidence<T>>) -> String {
        match evidence {
            None => "null".to_string(),
            Some(CatchupEvidence::Qc(qc)) => tagged("cert1", obj(&[("cert", cert1_json_raw(qc))])),
            Some(CatchupEvidence::Tc(tc)) => {
                self.before_certificate_rule.set(true);
                tagged(
                    "timeout",
                    obj(&[(
                        "cert",
                        self.timeout_data_json::<T>(tc.epoch(), tc.view_number(), None),
                    )]),
                )
            },
            Some(CatchupEvidence::Tc3(tc)) => {
                tagged("timeout", obj(&[("cert", self.timeout_cert3_json(tc))]))
            },
        }
    }

    /// A timeout certificate: the epoch whose committee formed it, its lock,
    /// and the view it certifies.
    ///
    /// A certificate whose signers did not sign their locks is outside the
    /// model; it is written with the anchor's, and the run is marked.
    fn timeout_cert_json<T: NodeType>(&self, cert: &TimeoutEvidence<T>) -> String {
        if !cert.binds_epoch() {
            self.before_certificate_rule.set(true);
        }
        self.timeout_data_json(
            cert.epoch(),
            cert.view_number(),
            cert.lock().map(|(_, lock)| lock),
        )
    }

    fn timeout_cert3_json<T: NodeType>(&self, cert: &TimeoutCertificate3<T>) -> String {
        self.timeout_data_json(Some(cert.epoch), cert.view_number, cert.lock_cert())
    }

    fn timeout_data_json<T: NodeType>(
        &self,
        epoch: Option<EpochNumber>,
        view: ViewNumber,
        lock: Option<&Certificate1<T>>,
    ) -> String {
        obj(&[
            (
                "data",
                obj(&[("epoch", epoch_json(epoch)), ("lock", self.lock_json(lock))]),
            ),
            ("view", view_json(view)),
        ])
    }

    /// The lock a timeout vote signs. One that signs none is outside the
    /// model; it is written with the anchor's, and the run is marked.
    fn timeout_vote_lock_json<T: NodeType>(&self, vote: &TimeoutVote<T>) -> String {
        match vote {
            TimeoutVote::V2(_) => {
                self.before_certificate_rule.set(true);
                self.lock_json::<T>(None)
            },
            TimeoutVote::V3(ballot) => self.lock_json(ballot.lock()),
        }
    }

    /// A timeout lock: its certificate, or the anchor's for a node locked on
    /// nothing but the anchor.
    fn lock_json<T: NodeType>(&self, lock: Option<&Certificate1<T>>) -> String {
        match lock {
            Some(cert) => cert1_json_raw(cert),
            None => self
                .anchor_lock
                .clone()
                .unwrap_or_else(|| ANCHOR_LOCK.to_string()),
        }
    }

    /// The certificates `input` carries that consensus keeps, as inputs of
    /// their own: a timeout certificate's lock, and a re-vote request's
    /// timeout certificate, its lock and the request's certificate. Only a
    /// recent lock is verified, and only a recent one is kept.
    fn carried_steps<T: NodeType>(&self, input: &ConsensusInput<T>) -> Vec<String> {
        match input {
            ConsensusInput::TimeoutCertificate(cert) => cert
                .timeout_lock()
                .recent()
                .map(certificate1_json)
                .into_iter()
                .collect(),
            ConsensusInput::ReVote(message) => {
                let revote = &message.revote;
                let mut steps = Vec::new();
                if let Some(tc) = &revote.timeout {
                    steps.extend(tc.timeout_lock().recent().map(certificate1_json));
                    steps.push(tagged(
                        "timeoutCertificate",
                        obj(&[("c", self.timeout_cert3_json(tc))]),
                    ));
                }
                steps.push(certificate1_json(&revote.cert1));
                steps
            },
            _ => Vec::new(),
        }
    }
}

fn certificate1_json<T: NodeType>(cert: &Certificate1<T>) -> String {
    tagged("certificate1", obj(&[("c", cert1_json_raw(cert))]))
}

fn certificate2_json<T: NodeType>(cert: &Certificate2<T>) -> String {
    tagged("certificate2", obj(&[("c", cert2_json_raw(cert))]))
}

/// A block header, as the model reads one: the payload it commits to, and the
/// height the epoch arithmetic runs on.
fn header_json<T: NodeType>(header: &T::BlockHeader) -> String {
    obj(&[
        ("payloadCommit", payload_json(header)),
        ("blockNumber", header.block_number().to_string()),
    ])
}

/// A decided leaf, as the model's `Block`.
///
/// The model compares a decided block with the proposal the node holds, field by
/// field. A leaf does not carry the proposal's timeout evidence, so it is taken
/// from the proposal of the same identity the run had, as `evidence` maps them.
fn leaf_json<T: NodeType>(
    leaf: &Leaf2<T>,
    epoch_height: u64,
    evidence: &HashMap<String, String>,
) -> String {
    let identity = ident(&leaf.commit());
    let timeout_evidence = evidence
        .get(&identity)
        .cloned()
        .unwrap_or_else(|| "null".to_string());
    obj(&[
        ("blockHeader", header_json::<T>(leaf.block_header())),
        ("viewNumber", view_json(leaf.view_number())),
        (
            "epoch",
            leaf.epoch(epoch_height)
                .map_or_else(|| "0".to_string(), |e| e.to_string()),
        ),
        ("parentCert", cert1_json_raw(leaf.justify_qc())),
        ("timeoutEvidence", timeout_evidence),
        ("identity", identity),
    ])
}

fn payload_json<H: BlockHeader<T>, T: NodeType>(header: &H) -> String {
    match header.payload_commitment() {
        VidCommitment::V2(commit) => ident(&commit),
        _ => "null".to_string(),
    }
}

/// `{"data": {"blockHash": …, "epoch": …, "blockNumber": …}, "view": …}` for a `Cert1`.
fn cert1_json_raw<T: NodeType>(cert: &Certificate1<T>) -> String {
    obj(&[
        (
            "data",
            obj(&[
                ("blockHash", ident(&cert.data.leaf_commit)),
                ("epoch", epoch_json(cert.epoch())),
                (
                    "blockNumber",
                    cert.data.block_number.unwrap_or(0).to_string(),
                ),
            ]),
        ),
        ("view", view_json(cert.view_number())),
    ])
}

fn cert2_json_raw<T: NodeType>(cert: &Certificate2<T>) -> String {
    obj(&[
        (
            "data",
            obj(&[
                ("blockHash", ident(&cert.data.leaf_commit)),
                ("epoch", epoch_json(cert.epoch())),
                ("blockNumber", cert.data.block_number.to_string()),
            ]),
        ),
        ("view", view_json(cert.view_number())),
    ])
}

/// The epoch a certificate or vote names.
///
/// `None` goes out as zero, which is how the model reads a run with no epochs.
/// On a network that does have them, a missing epoch is a real difference and
/// shows up as one rather than being smoothed over.
fn epoch_json(epoch: Option<EpochNumber>) -> String {
    epoch.map_or(0, |e| *e).to_string()
}

/// A view number is a number.
fn view_json(view: ViewNumber) -> String {
    (*view).to_string()
}

/// Anything identified travels as its own text: compared, never interpreted.
///
/// Commitments, payload commitments and keys all render as tagged base64; the
/// text is escaped anyway, these `Display` impls not being ours.
fn ident<D: std::fmt::Display>(value: &D) -> String {
    quoted(&value.to_string())
}

/// JSON, assembled by hand: the crate has no JSON writer and these shapes are
/// small and fixed.
fn obj(fields: &[(&str, String)]) -> String {
    let body: Vec<String> = fields
        .iter()
        .map(|(key, value)| format!("\"{key}\":{value}"))
        .collect();
    format!("{{{}}}", body.join(","))
}

fn arr(items: &[String]) -> String {
    format!("[{}]", items.join(","))
}

/// A JSON string.
///
/// Escaped rather than trusted: the text comes from `Display` impls this module
/// does not own, and a stray quote or newline would corrupt a record silently —
/// which is exactly how the interleaving bug above first showed itself.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn tagged(tag: &str, body: String) -> String {
    obj(&[(tag, body)])
}

#[cfg(test)]
mod tests {
    use super::{ANCHOR_LOCK, ANCHOR_UNKNOWN, fill_anchor_lock};

    /// A trace whose identity line names no anchor, and whose step has a lock
    /// standing for the anchor's certificate.
    fn trace() -> String {
        format!("# trace {{\"node\":1}}\n{{\"lock\":{ANCHOR_LOCK}}}\n")
    }

    #[test]
    fn the_anchor_named_fills_the_lock() {
        assert_eq!(
            fill_anchor_lock(&trace(), Some("\"A\""), Some("\"F\"")),
            "# trace {\"node\":1}\n{\"lock\":\"A\"}\n"
        );
    }

    #[test]
    fn the_fallback_fills_the_lock_and_the_identity_line() {
        assert_eq!(
            fill_anchor_lock(&trace(), None, Some("\"F\"")),
            "# trace {\"anchorCert\":\"F\",\"node\":1}\n{\"lock\":\"F\"}\n"
        );
    }

    #[test]
    fn with_neither_the_lock_is_null_and_the_trace_says_why() {
        assert_eq!(
            fill_anchor_lock(&trace(), None, None),
            format!("# trace {{\"node\":1}}\n{{\"lock\":null}}\n{ANCHOR_UNKNOWN}\n")
        );
    }

    #[test]
    fn a_trace_without_the_placeholder_is_unchanged() {
        let text = "# trace {\"node\":1}\n{\"lock\":\"L\"}\n";
        assert_eq!(fill_anchor_lock(text, None, Some("\"F\"")), text);
        assert_eq!(fill_anchor_lock(text, None, None), text);
    }
}
