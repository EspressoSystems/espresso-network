//! Recording a run for the Lean reference machine to replay.
//!
//! `lean/new-protocol-impl` is a machine proved to satisfy the specification in
//! `lean/new-protocol-spec`. It is executable, so it can be driven with the
//! inputs this implementation took and its conclusions compared against ours.
//! This module writes the trace; `lean/new-protocol-diff` reads it.
//!
//! # What the comparison is
//!
//! Not per-step output equality. The Lean machine is *eager* — after recording
//! an input it does everything the new state owes — while this implementation
//! defers, acting on particular views on particular inputs. Both satisfy the
//! specification; their per-step outputs differ anyway. What is compared is
//! containment on the views each side voted, proposed and decided in: everything
//! we did by step `n`, the machine did by step `n`. See the Lean side's
//! `NewProtocolDiff.Replay` for why that is the right relation.
//!
//! # What does not survive the translation
//!
//! The model covers the core protocol only, so epochs, the DRB, light-client
//! certification, the storage handshake and proposal fetching have no
//! counterpart. Inputs carrying them are dropped, each leaving a `#` comment
//! naming what went — visible in the trace rather than silent, because a dropped
//! input can make the machine look behind us for a reason that is not a bug.
//! Two are worth watching for, both of which make the machine look *ahead* of us
//! rather than behind — which the comparison reports but tolerates:
//! [`ConsensusInput::FetchedProposal`], since the machine has no way to hold a
//! fetched block; and [`ConsensusInput::StateValidationFailed`], since the model
//! has no way to un-admit one, so the machine goes on voting for a block we
//! condemned. A first corpus should come from tests that do neither.
//!
//! Pruning is not recorded either. We prune inside a consensus step; the model
//! makes it a separate transition, so a faithful trace would have to say where
//! ours happened. Until it does, `GcSpec` goes untested and the machine keeps
//! more state than we do — which shows up as the machine being ahead, which the
//! comparison tolerates.
//!
//! # Identities
//!
//! A block's identity travels as the text of its commitment. The Lean side does
//! not interpret it — every rule there only ever *compares* identities — so all
//! that is asked is that distinct commitments give distinct text.

use std::{
    collections::BTreeSet,
    fs::create_dir_all,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use committable::Committable;
use hotshot_types::{
    data::{Leaf2, VidCommitment, VidDisperseShare2, ViewNumber},
    simple_certificate::TimeoutCertificate2,
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    vote::HasViewNumber,
};

use crate::{
    consensus::{ConsensusInput, ConsensusOutput, DECIDE_BUFFER},
    message::{CatchupEvidence, Certificate1, Certificate2, Proposal},
};

/// Where traces are written, if anywhere.
const TRACE_DIR: &str = "NP_TRACE_DIR";

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
    /// Outputs from steps the model has no input for, waiting for a step to ride on.
    ///
    /// See [`Recorder::record`] for why they wait rather than being dropped.
    pending: Vec<String>,
    /// Views whose leader has been written; see [`Recorder::leader`].
    led: BTreeSet<ViewNumber>,
}

impl Recorder {
    /// A recorder collecting a trace for `$NP_TRACE_DIR/<label>.jsonl`, or an inert one.
    pub fn new(label: &str) -> Self {
        let inert = Self {
            path: None,
            lines: Vec::new(),
            identified: true,
            pending: Vec::new(),
            led: BTreeSet::new(),
        };
        let Ok(dir) = std::env::var(TRACE_DIR) else {
            return inert;
        };
        let dir = PathBuf::from(dir);
        if let Err(err) = create_dir_all(&dir) {
            eprintln!("trace: cannot create {}: {err}", dir.display());
            return inert;
        }
        // A label may name a test, and test names contain `::`.
        let name = label.replace("::", "-");
        let seq = NEXT_TRACE.fetch_add(1, Ordering::Relaxed);
        Self {
            path: Some(dir.join(format!("{name}.{seq}.jsonl"))),
            lines: Vec::new(),
            identified: false,
            pending: Vec::new(),
            led: BTreeSet::new(),
        }
    }

    /// A recorder labelled by the running test, which is this thread's name.
    pub fn for_current_test() -> Self {
        Self::new(std::thread::current().name().unwrap_or("trace"))
    }

    /// Write the identity line, once, before the first step.
    ///
    /// A replay needs two things the steps do not carry: which node this is, and
    /// the block its chain is anchored at. Neither can be recovered from the
    /// steps in general — a run that casts no vote never names its own key, and
    /// one that starts above genesis never cites the anchor — so a replay left to
    /// infer them has to guess, and a wrong guess looks like a disagreement.
    ///
    /// Written on the comment channel: a reader that only knows about steps skips
    /// it, as it already skips the dropped-input notes.
    ///
    /// Call before applying the input, so a first step that decides cannot move
    /// the anchor out from under the line describing where the run began.
    pub fn preamble<T: NodeType>(&mut self, node: &T::SignatureKey, anchor: &Leaf2<T>) {
        if self.identified {
            return;
        }
        self.identified = true;
        if self.path.is_none() {
            return;
        }
        let line = obj(&[
            ("node", ident(node)),
            ("anchor", ident(&anchor.commit())),
            ("decideBuffer", DECIDE_BUFFER.to_string()),
        ]);
        self.lines.push(format!("# trace {line}"));
    }

    /// Name the leader of `view`, once.
    ///
    /// Every view has a leader, and a trace that omits them leaves a replay with
    /// no way to tell a proposal this node was entitled to make from one it was
    /// not: the model takes the schedule as a parameter, so a replay that cannot
    /// read it has to assume something, and assuming this node leads everywhere
    /// makes the leader clause of `ProposalJustification` unfalsifiable.
    ///
    /// Written on the comment channel, like the identity line, so a reader that
    /// only knows about steps skips it.
    pub fn leader<T: NodeType>(&mut self, view: ViewNumber, leader: &T::SignatureKey) {
        if self.path.is_none() || !self.led.insert(view) {
            return;
        }
        self.lines
            .push(format!("# leader {} {}", view, ident(leader)));
    }

    /// Record one step: the input taken, and what it drew.
    pub fn record<'a, T: NodeType + 'a>(
        &mut self,
        input: &ConsensusInput<T>,
        outputs: impl Iterator<Item = &'a ConsensusOutput<T>>,
    ) {
        if self.path.is_none() {
            return;
        }
        let outputs: Vec<_> = outputs.collect();
        let mut lines: Vec<String> = Vec::new();

        // The model takes a proposal already paired with our share, and pairs
        // nothing itself. Rather than reassemble the pair here — where a bug
        // would look like a divergence — take it from the output that reports
        // the pairing, as a step of its own.
        for output in &outputs {
            if let ConsensusOutput::ProposalPaired {
                proposal,
                vid_share,
            } = output
            {
                lines.push(step(paired_proposal_json(&proposal.data, vid_share), &[]));
            }
        }

        let emitted: Vec<String> = outputs.iter().copied().filter_map(output_json).collect();
        match input_json(input) {
            Ok(json) => {
                // Anything held back rides along now. A recorded action reported
                // later than it happened is the safe direction: the replay asks
                // that the machine acted at or before the recording did, so a
                // delay can only make the recording easier to contain.
                let mut all = std::mem::take(&mut self.pending);
                all.extend(emitted);
                lines.push(step(json, &all));
            },
            Err(dropped) => {
                // The step cannot be written, because the model has no such
                // input. Its outputs are the run's all the same, and a trace
                // that discarded them would show a node that never voted: the
                // votes a real node parks until storage confirms them are
                // released on exactly these steps.
                lines.push(format!("# dropped input: {dropped}"));
                self.pending.extend(emitted);
            },
        }

        self.lines.extend(lines);
    }
}

/// Write the trace, and say what could not be written.
///
/// A run that recorded nothing leaves no file: a test that never reached the
/// consensus core has no trace to speak of, and an empty one would only have to
/// be skipped by every reader.
///
/// Outputs still held back are noted rather than lost in silence. A run whose
/// last steps all carry inputs the model has no counterpart for ends with actions
/// that never found a step to ride on. A reader comparing marks would otherwise
/// see the machine ahead and have no way to know why.
impl Drop for Recorder {
    fn drop(&mut self) {
        let Some(path) = self.path.as_ref() else {
            return;
        };
        if self.lines.is_empty() {
            return;
        }
        let held = self.pending.len();
        if held > 0 {
            self.lines
                .push(format!("# {held} outputs had no later step to ride on"));
        }
        let mut text = self.lines.join("\n");
        text.push('\n');
        if let Err(err) = std::fs::write(path, text) {
            eprintln!("trace: cannot write {}: {err}", path.display());
        }
    }
}

/// One `consensus` step of a trace.
fn step(input: String, outputs: &[String]) -> String {
    tagged(
        "consensus",
        obj(&[("input", input), ("output", arr(outputs))]),
    )
}

/// A proposal and this node's share for it, as the model's `Input.proposal`.
///
/// The sender is the share's recipient, which is us: the model carries a sender
/// but no rule reads it, so any key that identifies the step will do.
fn paired_proposal_json<T: NodeType>(
    proposal: &Proposal<T>,
    share: &VidDisperseShare2<T>,
) -> String {
    tagged(
        "proposal",
        obj(&[
            ("sender", ident(&share.recipient_key)),
            ("p", proposal_json(proposal)),
            (
                "vid",
                obj(&[
                    ("view", view_json(share.view_number)),
                    ("payloadCommit", ident(&share.payload_commitment)),
                ]),
            ),
        ]),
    )
}

/// The name of an input the model has no counterpart for.
struct Dropped(&'static str);

impl std::fmt::Display for Dropped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// One input, as the model's `Input`.
fn input_json<T: NodeType>(input: &ConsensusInput<T>) -> Result<String, Dropped> {
    Ok(match input {
        ConsensusInput::BlockReconstructed(view, commit)
        | ConsensusInput::VidDisperseCreated(view, commit) => tagged(
            "blockReconstructed",
            obj(&[("v", view_json(*view)), ("c", ident(commit))]),
        ),
        ConsensusInput::Certificate1(cert) => {
            tagged("certificate1", obj(&[("c", cert1_json_raw(cert))]))
        },
        // The pairing is epoch machinery, but the certificate is an ordinary
        // `Cert1` and this is the only way consensus receives it at an epoch
        // root — dropping the variant would lose a certificate the model wants.
        ConsensusInput::EpochRootCertificates { cert1, .. } => {
            tagged("certificate1", obj(&[("c", cert1_json_raw(cert1))]))
        },
        ConsensusInput::Certificate2(cert) => {
            tagged("certificate2", obj(&[("c", cert2_json_raw(cert))]))
        },
        ConsensusInput::AdvanceView(cert) => {
            tagged("advanceView", obj(&[("c", cert1_json_raw(cert))]))
        },
        ConsensusInput::HeaderCreated(view, parent, header) => tagged(
            "headerBuilt",
            obj(&[
                ("v", view_json(*view)),
                ("parent", ident(parent)),
                ("h", obj(&[("payloadCommit", payload_json(header))])),
            ]),
        ),
        ConsensusInput::StateValidated(response) => tagged(
            "blockValidated",
            obj(&[
                ("v", view_json(response.view)),
                ("h", ident(&response.commitment)),
            ]),
        ),
        ConsensusInput::Timeout(view, _) => tagged("timeout", obj(&[("v", view_json(*view))])),
        ConsensusInput::TimeoutOneHonest(view, _) => {
            tagged("timeoutOneHonest", obj(&[("v", view_json(*view))]))
        },
        ConsensusInput::TimeoutCertificate(cert) => tagged(
            "timeoutCertificate",
            obj(&[("c", timeout_cert_json_raw(cert))]),
        ),
        // The model receives a proposal already paired with our share, so the
        // pair is emitted from `ProposalPaired` instead of reassembled here.
        ConsensusInput::Proposal(..) => return Err(Dropped("Proposal (paired later)")),
        ConsensusInput::VidShare(..) => return Err(Dropped("VidShare (paired later)")),
        // Outside the model's scope; see the module docs.
        ConsensusInput::FetchedProposal(..) => return Err(Dropped("FetchedProposal")),
        ConsensusInput::StateValidationFailed(..) => return Err(Dropped("StateValidationFailed")),
        ConsensusInput::BlockBuilt { .. } => return Err(Dropped("BlockBuilt")),
        ConsensusInput::Stored(..) => return Err(Dropped("Stored")),
        ConsensusInput::DrbResult(..) => return Err(Dropped("DrbResult")),
        ConsensusInput::EpochChange(..) => return Err(Dropped("EpochChange")),
    })
}

/// One output, as the model's `Output`, or nothing when the model has no such output.
fn output_json<T: NodeType>(output: &ConsensusOutput<T>) -> Option<String> {
    let message = match output {
        ConsensusOutput::SendVote1(vote) => tagged(
            "vote1",
            obj(&[(
                "v",
                obj(&[
                    (
                        "data",
                        obj(&[("blockHash", ident(&vote.vote.data.leaf_commit))]),
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
                    ("data", obj(&[("blockHash", ident(&vote.data.leaf_commit))])),
                    ("view", view_json(vote.view_number)),
                    ("signer", ident(&vote.signature.0)),
                ]),
            )]),
        ),
        ConsensusOutput::SendProposal(signed) => {
            tagged("proposal", obj(&[("p", proposal_json(&signed.data))]))
        },
        ConsensusOutput::SendTimeoutVote(vote, evidence) => tagged(
            "timeoutVote",
            obj(&[
                (
                    "v",
                    obj(&[
                        ("data", obj(&[])),
                        ("view", view_json(vote.view_number)),
                        ("signer", ident(&vote.signature.0)),
                    ]),
                ),
                ("e", evidence_json(evidence.as_ref())),
            ]),
        ),
        ConsensusOutput::SendTimeoutCertificate(cert, view, _) => tagged(
            "timeoutCert",
            obj(&[("c", timeout_cert_json_raw(cert)), ("v", view_json(*view))]),
        ),
        ConsensusOutput::SendCertificate1(cert) => {
            tagged("cert1", obj(&[("c", cert1_json_raw(cert))]))
        },
        ConsensusOutput::SendCertificate2(cert) => {
            tagged("cert2", obj(&[("c", cert2_json_raw(cert))]))
        },
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
            let blocks: Vec<String> = leaves.iter().map(leaf_json).collect();
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

/// A proposal we sent, or one a decide delivered.
fn proposal_json<T: NodeType>(proposal: &Proposal<T>) -> String {
    obj(&[
        (
            "blockHeader",
            obj(&[("payloadCommit", payload_json(&proposal.block_header))]),
        ),
        ("viewNumber", view_json(proposal.view_number)),
        ("parentCert", cert1_json_raw(&proposal.justify_qc)),
        (
            "timeoutEvidence",
            match &proposal.view_change_evidence {
                Some(tc) => timeout_cert_json_raw(tc),
                None => "null".to_string(),
            },
        ),
        (
            "identity",
            ident(&crate::helpers::proposal_commitment(proposal)),
        ),
    ])
}

/// A decided leaf, as the model's `Block`.
///
/// Only the view numbers are compared, so the remaining fields are filled as
/// faithfully as a leaf allows and no further.
fn leaf_json<T: NodeType>(leaf: &Leaf2<T>) -> String {
    obj(&[
        (
            "blockHeader",
            obj(&[("payloadCommit", payload_json(leaf.block_header()))]),
        ),
        ("viewNumber", view_json(leaf.view_number())),
        ("parentCert", cert1_json_raw(&leaf.justify_qc())),
        ("timeoutEvidence", "null".to_string()),
        ("identity", ident(&leaf.commit())),
    ])
}

fn evidence_json<T: NodeType>(evidence: Option<&CatchupEvidence<T>>) -> String {
    match evidence {
        None => "null".to_string(),
        Some(CatchupEvidence::Qc(qc)) => tagged("cert1", obj(&[("cert", cert1_json_raw(qc))])),
        Some(CatchupEvidence::Tc(tc)) => {
            tagged("timeout", obj(&[("cert", timeout_cert_json_raw(tc))]))
        },
    }
}

fn payload_json<H: BlockHeader<T>, T: NodeType>(header: &H) -> String {
    match header.payload_commitment() {
        VidCommitment::V2(commit) => ident(&commit),
        _ => "null".to_string(),
    }
}

/// `{"data": {"blockHash": …}, "view": …}` for a `Cert1`.
fn cert1_json_raw<T: NodeType>(cert: &Certificate1<T>) -> String {
    obj(&[
        ("data", obj(&[("blockHash", ident(&cert.data.leaf_commit))])),
        ("view", view_json(cert.view_number())),
    ])
}

fn cert2_json_raw<T: NodeType>(cert: &Certificate2<T>) -> String {
    obj(&[
        ("data", obj(&[("blockHash", ident(&cert.data.leaf_commit))])),
        ("view", view_json(cert.view_number())),
    ])
}

/// The model's timeout certificate carries no data; only the view it certifies.
fn timeout_cert_json_raw<T: NodeType>(cert: &TimeoutCertificate2<T>) -> String {
    obj(&[("data", obj(&[])), ("view", view_json(cert.view_number()))])
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
