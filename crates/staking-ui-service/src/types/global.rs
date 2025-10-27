//! Types that make up the global state API.

use super::common::{
    ActiveNodeSetEntry, EpochAndBlock, L1BlockInfo, NodeExit, NodeSetEntry, ParticipationChange,
};
use serde::{Deserialize, Serialize};

/// A snapshot of the full node set, according to the staking contract, at a point in time.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct FullNodeSetSnapshot {
    /// The list of registered nodes.
    pub nodes: Vec<NodeSetEntry>,

    /// The block at which this snapshot was taken
    pub l1_block: L1BlockInfo,
}

/// A change to the full node set.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct FullNodeSetUpdate {
    /// The block at which this change occurred.
    pub l1_block: L1BlockInfo,

    /// The actual changes to the set.
    pub diff: Vec<FullNodeSetDiff>,
}

/// A single update to the full node set.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum FullNodeSetDiff {
    /// A change in information about a specific node.
    ///
    /// This can indicate an update to any and all of the data for a specific node, or the addition
    /// of a new node.
    NodeUpdate(NodeSetEntry),

    /// A node leaving the set.
    NodeExit(NodeExit),
}

/// A snapshot of the active node set, according to the Espresso, at a point in time.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ActiveNodeSetSnapshot {
    /// The block at which this snapshot was taken.
    pub espresso_block: EpochAndBlock,

    /// The list of active nodes.
    pub nodes: Vec<ActiveNodeSetEntry>,
}

/// A change to the active node set.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ActiveNodeSetUpdate {
    /// The block at which this change occurred.
    pub espresso_block: EpochAndBlock,

    /// The actual changes to the set.
    pub diff: Vec<ActiveNodeSetDiff>,
}

/// A single update to the active node set.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum ActiveNodeSetDiff {
    /// An update to the leader participation change of the relevant nodes.
    LeaderParticipationChange(Vec<ParticipationChange>),

    /// An update to the voter participation change of the relevant nodes.
    VoterParticipationChange(Vec<ParticipationChange>),

    /// Upon entering a new epoch, replace the current active node set with an entirely new set.
    NewEpoch(Vec<ActiveNodeSetEntry>),
}
