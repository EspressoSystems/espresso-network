use committable::{Commitment, Committable};
use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    simple_certificate::Certificate2,
    simple_vote::HasEpoch,
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    utils::{epoch_from_block_number, is_last_block},
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

/// The justify QC must certify the block before this proposal, in the epoch that
/// block's height falls in.
///
/// A certificate's epoch selects the committee whose stake table and threshold
/// its signatures are weighed against. Unlike the proposal's own epoch, it is
/// covered by those signatures, so a mismatch is not something a proposer can
/// produce by relabelling a genuine certificate.
///
/// Returns that epoch, which the QC checks resolve their membership through.
pub(crate) fn justify_qc_matches_parent<T: NodeType>(
    proposal: &Proposal<T>,
    epoch_height: u64,
) -> Result<EpochNumber, JustifyQcMismatch> {
    let view = proposal.view_number();
    let Some(claimed_epoch) = proposal.justify_qc.epoch() else {
        return Err(JustifyQcMismatch::MissingEpoch(view));
    };
    // Epochs are disabled, so no block number names an epoch.
    if epoch_height == 0 {
        return Ok(claimed_epoch);
    }
    let parent_block = proposal.block_header.block_number().saturating_sub(1);
    let expected_epoch = epoch_of_block(parent_block, epoch_height);
    if claimed_epoch != expected_epoch {
        return Err(JustifyQcMismatch::Epoch {
            view,
            parent_block,
            expected: expected_epoch,
            claimed: claimed_epoch,
        });
    }
    // A certificate names its epoch and block number together, so requiring one
    // here is what the epoch above already established.
    let Some(claimed_block) = proposal.justify_qc.data.block_number else {
        return Err(JustifyQcMismatch::MissingBlockNumber(view));
    };
    if claimed_block != parent_block {
        return Err(JustifyQcMismatch::BlockNumber {
            view,
            expected: parent_block,
            claimed: claimed_block,
        });
    }
    Ok(claimed_epoch)
}

/// A proposal's justify QC does not certify the block before it.
#[derive(Copy, Clone, Debug, thiserror::Error)]
pub enum JustifyQcMismatch {
    #[error("justify_qc of proposal at view {0} names no epoch")]
    MissingEpoch(ViewNumber),

    #[error("justify_qc of proposal at view {0} names no block number")]
    MissingBlockNumber(ViewNumber),

    #[error(
        "justify_qc of proposal at view {view} claims epoch {claimed}, but its parent block \
         {parent_block} falls in epoch {expected}"
    )]
    Epoch {
        view: ViewNumber,
        parent_block: u64,
        expected: EpochNumber,
        claimed: EpochNumber,
    },

    #[error(
        "justify_qc of proposal at view {view} certifies block {claimed}, not its parent block \
         {expected}"
    )]
    BlockNumber {
        view: ViewNumber,
        expected: u64,
        claimed: u64,
    },
}

/// The first proposal of an epoch must carry the boundary block's Cert2, which
/// certifies the same block as its justify QC.
///
/// Returns the certificate to verify the signatures on, or `None` when the
/// proposal does not follow a boundary block and carries none.
///
/// `justify_qc_epoch` is the justify QC's own, as returned by
/// [`justify_qc_matches_parent`]. The certificate names the same view, epoch
/// and block as that QC because both certify the same leaf, and the leaf
/// commitment covers neither, so agreement otherwise rests on an honest signer
/// being in the quorum that formed it.
pub(crate) fn next_epoch_justify_qc_matches_parent<T: NodeType>(
    proposal: &Proposal<T>,
    epoch_height: u64,
    justify_qc_epoch: EpochNumber,
) -> Result<Option<&Certificate2<T>>, NextEpochJustifyQcMismatch> {
    let parent_block = proposal.block_header.block_number().saturating_sub(1);
    if !is_last_block(parent_block, epoch_height) {
        return Ok(None);
    }
    let view = proposal.view_number();
    let Some(cert2) = proposal.next_epoch_justify_qc.as_ref() else {
        return Err(NextEpochJustifyQcMismatch::Missing(view));
    };
    if cert2.data.leaf_commit != proposal.justify_qc.data.leaf_commit {
        return Err(NextEpochJustifyQcMismatch::LeafCommit(view));
    }
    let parent_view = proposal.justify_qc.view_number();
    if cert2.view_number() != parent_view
        || cert2.data.epoch != justify_qc_epoch
        || cert2.data.block_number != parent_block
    {
        return Err(NextEpochJustifyQcMismatch::Parent {
            view,
            claimed_view: cert2.view_number(),
            claimed_epoch: cert2.data.epoch,
            claimed_block: cert2.data.block_number,
            parent_view,
            parent_epoch: justify_qc_epoch,
            parent_block,
        });
    }
    Ok(Some(cert2))
}

/// A proposal's `next_epoch_justify_qc` does not certify the block below it.
#[derive(Copy, Clone, Debug, thiserror::Error)]
pub enum NextEpochJustifyQcMismatch {
    #[error("first proposal of an epoch at view {0} is missing next_epoch_justify_qc")]
    Missing(ViewNumber),

    #[error(
        "next_epoch_justify_qc of proposal at view {0} certifies another leaf than its justify_qc"
    )]
    LeafCommit(ViewNumber),

    #[error(
        "next_epoch_justify_qc of proposal at view {view} certifies view {claimed_view} of epoch \
         {claimed_epoch} at block {claimed_block}, not view {parent_view} of epoch {parent_epoch} \
         at block {parent_block}"
    )]
    Parent {
        view: ViewNumber,
        claimed_view: ViewNumber,
        claimed_epoch: EpochNumber,
        claimed_block: u64,
        parent_view: ViewNumber,
        parent_epoch: EpochNumber,
        parent_block: u64,
    },
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
