//! The scenario files `lake exe sim --scenarios` writes.
//!
//! Blocks and certificates are named abstractly, by the identity the simulated
//! network gave a block, and the runner builds the real objects
//! (`super::world`). Block `1` is the anchor every run starts from.

use std::path::Path;

use serde::{Deserialize, de::IgnoredAny};

/// The run a scenario comes from, and the node it is for. The rest of the run's
/// parameters are in the file for a reader; the runner needs only these.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    /// The node, numbered from 1.
    pub node: u64,
    pub nodes: u64,
    pub epoch_height: u64,
    /// The leaders whose proposals break the rules.
    #[serde(default)]
    pub misbehaving: Vec<u64>,
    /// The committees, each from the epoch it names, its members in the order
    /// they lead. Empty for one committee of all the nodes.
    #[serde(default)]
    pub committees: Vec<(u64, Vec<u64>)>,
}

/// A certificate over a block.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
pub struct Cert {
    pub block: u64,
    pub epoch: u64,
    pub height: u64,
    pub view: u64,
}

/// A timeout certificate.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub struct TimeoutCert {
    pub epoch: u64,
    pub view: u64,
    pub lock: Cert,
}

/// A block, by its proposal.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub struct Block {
    pub id: u64,
    /// Whether the block passes the validity check.
    #[serde(default = "valid")]
    pub valid: bool,
    pub view: u64,
    pub epoch: u64,
    pub height: u64,
    pub payload: u64,
    pub parent: Cert,
    pub evidence: Option<TimeoutCert>,
}

fn valid() -> bool {
    true
}

/// An input, as the specification has it (`NewProtocol.Input`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Input {
    BlockReconstructed {
        view: u64,
        payload: u64,
    },
    Certificate1(Cert),
    Certificate2(Cert),
    EpochChange {
        cert1: Cert,
        cert2: Cert,
        block: Block,
    },
    /// Answered by the harness, so not read.
    HeaderBuilt(IgnoredAny),
    Proposal {
        sender: u64,
        block: Block,
        share: bool,
    },
    Revote {
        sender: u64,
        cert: Cert,
        view: u64,
        evidence: Option<TimeoutCert>,
    },
    /// Answered by the harness, so not read.
    BlockValidated(IgnoredAny),
    Timeout(u64),
    TimeoutCertificate(TimeoutCert),
    TimeoutOneHonest(u64),
}

/// One input. Its time and whether it was the node's timer are in the file for a
/// reader; the runner replays the inputs in order.
#[derive(Clone, Debug, Deserialize)]
pub struct Step {
    pub input: Input,
}

#[derive(Deserialize)]
struct HeaderLine {
    scenario: Header,
}

#[derive(Deserialize)]
struct BlocksLine {
    blocks: Vec<Block>,
}

/// A scenario: the run, every block the network built, and the node's inputs.
#[derive(Clone, Debug)]
pub struct Scenario {
    pub header: Header,
    pub blocks: Vec<Block>,
    pub steps: Vec<Step>,
}

impl Scenario {
    pub fn read(path: &Path) -> Self {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let mut lines = text.lines().filter(|l| !l.trim().is_empty());
        let parse_err =
            |what: &str, e: serde_json::Error| -> ! { panic!("{}: {what}: {e}", path.display()) };
        let header: HeaderLine = serde_json::from_str(lines.next().expect("a header line"))
            .unwrap_or_else(|e| parse_err("header", e));
        let blocks: BlocksLine = serde_json::from_str(lines.next().expect("a block line"))
            .unwrap_or_else(|e| parse_err("blocks", e));
        let steps = lines
            .map(|l| serde_json::from_str(l).unwrap_or_else(|e| parse_err("step", e)))
            .collect();
        Self {
            header: header.scenario,
            blocks: blocks.blocks,
            steps,
        }
    }
}
