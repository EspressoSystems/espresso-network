use committable::{Commitment, Committable};
use hotshot_types::{
    data::Leaf2,
    simple_certificate::{
        LightClientStateUpdateCertificateV2, check_qc_state_cert_correspondence,
    },
    traits::node_implementation::NodeType,
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
/// choosing.
///
/// Re-running the correspondence check binds the certificate's epoch and view to
/// the QC here rather than trusting that the validator did it, so the epoch the
/// store sites key by cannot be chosen by the sender. Callers must still pass a
/// validated proposal: the threshold signature check only happens there.
pub fn validated_state_cert<T: NodeType>(
    proposal: &Proposal<T>,
    epoch_height: u64,
) -> Option<&LightClientStateUpdateCertificateV2<T>> {
    let state_cert = proposal.state_cert.as_ref()?;
    check_qc_state_cert_correspondence(&proposal.justify_qc, state_cert, epoch_height)
        .then_some(state_cert)
}

#[cfg(test)]
pub fn test_upgrade_lock<T: NodeType>() -> hotshot_types::message::UpgradeLock<T> {
    use versions::{NEW_PROTOCOL_VERSION, Upgrade};

    hotshot_types::message::UpgradeLock::new(Upgrade::trivial(NEW_PROTOCOL_VERSION))
}
