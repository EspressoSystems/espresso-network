use committable::{Commitment, Committable};
use hotshot_types::{data::Leaf2, traits::node_implementation::NodeType};

use crate::message::Proposal;

pub fn proposal_commitment<T: NodeType>(proposal: &Proposal<T>) -> Commitment<Leaf2<T>> {
    let leaf: Leaf2<T> = proposal.clone().into();
    leaf.commit()
}

#[cfg(test)]
pub fn test_upgrade_lock<T: NodeType>() -> hotshot_types::message::UpgradeLock<T> {
    use versions::{CLIQUENET_VERSION, Upgrade};

    hotshot_types::message::UpgradeLock::new(Upgrade::trivial(CLIQUENET_VERSION))
}
