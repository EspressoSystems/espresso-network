use committable::{Commitment, Committable};
use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    utils::epoch_from_block_number,
    vote::HasViewNumber,
};

use crate::message::Proposal;

pub fn proposal_commitment<T: NodeType>(proposal: &Proposal<T>) -> Commitment<Leaf2<T>> {
    let leaf: Leaf2<T> = proposal.clone().into();
    leaf.commit()
}

/// The proposal's epoch must be the one its block number falls in.
pub(crate) fn epoch_matches_height<T: NodeType>(
    proposal: &Proposal<T>,
    epoch_height: u64,
) -> Result<(), EpochMismatch> {
    // Epochs are disabled, so no block number names an epoch.
    if epoch_height == 0 {
        return Ok(());
    }
    let block_number = proposal.block_header.block_number();
    let expected = epoch_of_block(block_number, epoch_height);
    if proposal.epoch != expected {
        return Err(EpochMismatch {
            view: proposal.view_number(),
            block_number,
            expected,
            claimed: proposal.epoch,
        });
    }
    Ok(())
}

/// A proposal claims an epoch other than the one its block number falls in.
#[derive(Copy, Clone, Debug, thiserror::Error)]
#[error(
    "proposal at view {view} claims epoch {claimed}, but block number {block_number} falls in \
     epoch {expected}"
)]
pub struct EpochMismatch {
    pub view: ViewNumber,
    pub block_number: u64,
    pub expected: EpochNumber,
    pub claimed: EpochNumber,
}

/// The epoch the block at `block_number` falls in.
///
/// Only meaningful with epochs enabled; `epoch_height` of 0 yields
/// [`EpochNumber`] 0, which names no committee.
pub(crate) fn epoch_of_block(block_number: u64, epoch_height: u64) -> EpochNumber {
    EpochNumber::new(epoch_from_block_number(block_number, epoch_height))
}

#[cfg(test)]
pub fn test_upgrade_lock<T: NodeType>() -> hotshot_types::message::UpgradeLock<T> {
    use versions::{NEW_PROTOCOL_VERSION, Upgrade};

    hotshot_types::message::UpgradeLock::new(Upgrade::trivial(NEW_PROTOCOL_VERSION))
}
