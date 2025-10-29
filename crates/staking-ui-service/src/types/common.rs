//! Primitive types.

use serde::{Deserialize, Serialize};

pub use alloy::primitives::{Address, BlockHash, U256};
pub use tagged_base64::TaggedBase64;

/// An amount of Espresso tokens in WEI.
pub type ESPTokenAmount = U256;

/// A Unix timestamps in seconds since epoch.
pub type Timestamp = u64;

/// A ratio between 0 and 1.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct Ratio(f32);

impl Ratio {
    pub fn new(num: usize, den: usize) -> Self {
        Self((num as f32) / (den as f32))
    }
}

/// An entry in the full node set.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NodeSetEntry {
    /// Node's Ethereum address.
    pub address: Address,

    /// The key used for the node for signing consensus messages.
    pub staking_key: TaggedBase64,

    /// Total stake currently attributed to the node.
    pub stake: ESPTokenAmount,

    /// How much commission the node charges.
    pub commission: Ratio,
}

/// Information about an L1 block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct L1BlockInfo {
    /// The block number
    pub number: u64,

    /// The hash of this block (useful for detecting reorgs)
    pub hash: BlockHash,

    /// The timestamp of this block.
    pub timestamp: Timestamp,
}

/// Minimal information needed to identify an L1 block and check for reorgs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct L1BlockId {
    /// The block number.
    pub number: u64,

    /// The hash of this block.
    pub hash: BlockHash,

    /// The parent of this block, used for reorg detection.
    pub parent: BlockHash,
}

/// Information about the exiting of a node from the node set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct NodeExit {
    /// The exiting node.
    pub address: Address,

    /// The Espresso epoch and block at which this exit will be realized.
    pub exit_epoch_and_block: EpochAndBlock,

    /// The timestamp for the exit escrow delay time.
    pub exit_time: Timestamp,
}

/// Information about the current "time" on the Espresso chain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct EpochAndBlock {
    /// The current epoch of the Espresso chain
    pub epoch: u64,

    /// The current block of the Espresso chain
    pub block: u64,

    /// The timestamp of the last block
    pub timestamp: Timestamp,
}

/// An entry in the active node set.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct ActiveNodeSetEntry {
    /// The node's address.
    pub address: Address,

    /// The node's voter participation in the current epoch.
    pub voter_participation: Ratio,

    /// The node's leader participation in the current epoch.
    pub leader_participation: Ratio,
}

/// A general participation percentage change for a node.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct ParticipationChange {
    /// The address of the node whose participation percentage is changing.
    pub address: Address,

    /// The new participation ratio.
    pub ratio: Ratio,
}

/// A single delegation from a particular user to a particular node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Delegation {
    /// The user delegating.
    pub delegator: Address,

    /// The node being delegated to.
    pub node: Address,

    /// Amount of stake delegated by this user to this node.
    pub amount: ESPTokenAmount,

    /// The estimated time that this delegation will start accumulating awards.
    pub effective: EpochAndBlock,
}

/// A withdrawal of stake that is waiting to be claimed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct PendingWithdrawal {
    /// The owner of the pending stake.
    pub delegator: Address,

    /// The node which was previously delegated to, which stake is now being withdrawn.
    pub node: Address,

    /// The amount of stake pending withdrawal.
    pub amount: ESPTokenAmount,

    /// The timestamp recorded for the exit escrow time.
    ///
    /// Any attempts to withdrawal before this will fail.
    pub available_time: Timestamp,
}

/// A completed withdrawal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Withdrawal {
    /// The owner of the withdrawn stake.
    pub delegator: Address,

    /// The node which was previously delegated to, which stake is now withdrawn.
    pub node: Address,

    /// The amount of stake.
    pub amount: ESPTokenAmount,
}
