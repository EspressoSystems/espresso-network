//! Types that make up the API specific to an individual wallet.

use super::common::{Delegation, ESPTokenAmount, L1BlockInfo, PendingWithdrawal, Withdrawal};
use serde::{Deserialize, Serialize};

/// A complete snapshot of a user's wallet state, at a point in time.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct WalletSnapshot {
    /// Nodes that this user is delegating to.
    pub nodes: im::Vector<Delegation>,

    /// Stake that has been undelegated but not yet withdrawn.
    pub pending_undelegations: im::Vector<PendingWithdrawal>,

    /// Stake previously delegated to nodes that have exited.
    pub pending_exits: im::Vector<PendingWithdrawal>,

    /// Total amount of rewards ever claimed from the contract.
    pub claimed_rewards: ESPTokenAmount,

    /// The block at which this snapshot was taken.
    pub l1_block: L1BlockInfo,
}

/// A change to the state of a wallet.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct WalletUpdate {
    /// The block at which this change occurred.
    pub l1_block: L1BlockInfo,

    /// The actual changes to the state.
    pub diff: Vec<WalletDiff>,
}

/// A single update to the state of a wallet.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum WalletDiff {
    /// A change in the total amount of claimed rewards for this wallet.
    ClaimedRewards(u64),

    /// A new delegation by this wallet to a specific node.
    DelegatedToNode(Delegation),

    /// An undelegation by this wallet from a specific node.
    UndelegatedFromNode(PendingWithdrawal),

    /// A node that this wallet is delegated to unregistered.
    NodeExited(PendingWithdrawal),

    /// A successful withdrawal of previously undelegated stake.
    UndelegationWithdrawal(Withdrawal),

    /// A successful withdrawal of stake previously delegated to a node that exited.
    NodeExitWithdrawal(Withdrawal),
}
