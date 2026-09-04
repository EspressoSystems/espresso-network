use committable::{Commitment, Committable};
use hotshot_types::{
    data::Leaf2, simple_certificate::LightClientStateUpdateCertificateV2,
    traits::node_implementation::NodeType, utils::is_epoch_root,
};

use crate::message::Proposal;

pub fn proposal_commitment<T: NodeType>(proposal: &Proposal<T>) -> Commitment<Leaf2<T>> {
    let leaf: Leaf2<T> = proposal.clone().into();
    leaf.commit()
}

/// The proposal's `state_cert`, if `Validator::state_cert` actually checked it.
///
/// The validator skips the field unless the parent QC sits at an epoch root, and
/// the proposal signature does not cover it (`Leaf2` discards it), so anyone
/// relaying a proposal can substitute it. Every store site must gate on this, or
/// it files away a certificate nobody verified, under an epoch of the sender's
/// choosing. Keep the condition identical to the validator's.
pub fn validated_state_cert<T: NodeType>(
    proposal: &Proposal<T>,
    epoch_height: u64,
) -> Option<&LightClientStateUpdateCertificateV2<T>> {
    let parent_block = proposal.justify_qc.data.block_number?;
    if !is_epoch_root(parent_block, epoch_height) {
        return None;
    }
    proposal.state_cert.as_ref()
}

#[cfg(test)]
pub fn test_upgrade_lock<T: NodeType>() -> hotshot_types::message::UpgradeLock<T> {
    use versions::{NEW_PROTOCOL_VERSION, Upgrade};

    hotshot_types::message::UpgradeLock::new(Upgrade::trivial(NEW_PROTOCOL_VERSION))
}
