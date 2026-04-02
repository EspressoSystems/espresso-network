use committable::{Commitment, Committable};
use hotshot_types::{data::Leaf2, message::UpgradeLock, traits::node_implementation::NodeType};
use versions::{Upgrade, VID2_UPGRADE_VERSION};

use crate::message::Proposal;

pub fn proposal_commitment<T: NodeType>(proposal: &Proposal<T>) -> Commitment<Leaf2<T>> {
    let leaf: Leaf2<T> = proposal.clone().into();
    leaf.commit()
}

// TODO: Remove this and use the actual upgrade lock
pub fn upgrade_lock<T: NodeType>() -> UpgradeLock<T> {
    UpgradeLock::new(Upgrade::trivial(VID2_UPGRADE_VERSION))
}
