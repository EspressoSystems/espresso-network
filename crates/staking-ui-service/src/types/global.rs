//! Types that make up the global state API.

use std::sync::Arc;

use super::common::{ActiveNodeSetEntry, EpochAndBlock, L1BlockInfo, NodeExit, NodeSetEntry};
use alloy::primitives::Address;
use bitvec::vec::BitVec;
use serde::{Deserialize, Serialize};

/// A snapshot of the full node set, according to the staking contract, at a point in time.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct FullNodeSetSnapshot {
    /// The list of registered nodes.
    pub nodes: Vec<NodeSetEntry>,

    /// The block at which this snapshot was taken
    pub l1_block: L1BlockInfo,
}

/// A change to the full node set.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct FullNodeSetUpdate {
    /// The block at which this change occurred.
    pub l1_block: L1BlockInfo,

    /// The actual changes to the set.
    pub diff: Vec<FullNodeSetDiff>,
}

/// A single update to the full node set.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum FullNodeSetDiff {
    /// A change in information about a specific node.
    ///
    /// This can indicate an update to any and all of the data for a specific node, or the addition
    /// of a new node.
    NodeUpdate(Arc<NodeSetEntry>),

    /// A node leaving the set.
    NodeExit(NodeExit),
}

/// A snapshot of the active node set, according to the Espresso, at a point in time.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ActiveNodeSetSnapshot {
    /// The block at which this snapshot was taken.
    pub espresso_block: EpochAndBlock,

    /// The list of active nodes.
    pub nodes: Vec<ActiveNodeSetEntry>,
}

/// A change to the active node set.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ActiveNodeSetUpdate {
    /// The block at which this change occurred.
    pub espresso_block: EpochAndBlock,

    /// The actual changes to the set.
    pub diff: Vec<ActiveNodeSetDiff>,
}

/// A single update to the active node set.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum ActiveNodeSetDiff {
    /// Update sent out every Espresso block.
    NewBlock {
        /// The index of the leader who produced this block.
        ///
        /// This causes the indicated node's leader participation rate to be increased, the
        /// numerator and denominator both increasing by one.
        leader: usize,

        /// The indices of the leaders of any failed views between the last block and this one.
        ///
        /// This causes the indicated nodes' leader participation rates to be decreased, the
        /// denominator increasing by one for each time a node appears in this list. Note that it is
        /// possible, though not especially likely, for the same index to appear more than once in
        /// the list (hence why we use an explicit list and not a bitset).
        failed_leaders: Vec<usize>,

        /// A bitmap defining the set of nodes which voted on this block.
        ///
        /// This can be used by the client to compute the replica participation rate of each active
        /// node. The participation rate for a node that is active in an epoch is defined as
        /// `# of votes in epoch / # of blocks in epoch`. The former can be obtained by counting the
        /// number of times the node appears in the bitmaps in these events. The latter can be
        /// computed by `current block - epoch start block`.
        ///
        /// The bitmap aligns with the current active node set: bit `i` is set if and only if active
        /// node `i` voted on this block.
        voters: BitVec,
    },

    /// Upon entering a new epoch, replace the current active node set with an entirely new set.
    NewEpoch(Vec<Address>),
}
