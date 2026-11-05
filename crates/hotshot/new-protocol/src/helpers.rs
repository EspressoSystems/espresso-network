use std::{fmt::Display, ops::RangeInclusive};

use committable::{Commitment, Committable};
use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    simple_certificate::{LightClientStateUpdateCertificateV2, check_qc_state_cert_correspondence},
    traits::node_implementation::NodeType,
};

use crate::message::Proposal;

/// A view and an epoch, ordered by view first.
///
/// Votes, proposals and certificates are counted once per epoch and view: at an
/// epoch boundary the outgoing committee may vote on its last block again in the
/// view the incoming committee proposes its first block in. With the view first,
/// the keys of a range of views form a range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ViewEpoch(pub ViewNumber, pub EpochNumber);

impl ViewEpoch {
    /// The first key at `view`.
    pub const fn start(view: ViewNumber) -> Self {
        Self(view, EpochNumber::new(0))
    }

    /// The keys at `view`, of every epoch.
    pub fn at_view(view: ViewNumber) -> RangeInclusive<Self> {
        Self::start(view)..=Self(view, EpochNumber::new(u64::MAX))
    }
}

impl Display for ViewEpoch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.1, self.0)
    }
}

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

/// An upgrade lock with [`TIMEOUT_EPOCH_VERSION`] already in effect, so
/// timeout votes and certificates take the form that binds the epoch.
#[cfg(test)]
pub fn test_timeout_epoch_lock<T: NodeType>() -> hotshot_types::message::UpgradeLock<T> {
    use versions::{TIMEOUT_EPOCH_VERSION, Upgrade};

    hotshot_types::message::UpgradeLock::new(Upgrade::trivial(TIMEOUT_EPOCH_VERSION))
}
