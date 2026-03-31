use committable::{Commitment, Committable};
use hotshot_types::{
    data::{Leaf2, QuorumProposal2, QuorumProposalWrapper},
    message::UpgradeLock,
    traits::node_implementation::NodeType,
};
use versions::{Upgrade, VID2_UPGRADE_VERSION};

pub fn proposal_commitment<T: NodeType>(proposal: &QuorumProposal2<T>) -> Commitment<Leaf2<T>> {
    let wrapper = QuorumProposalWrapper::from(proposal.clone());
    Leaf2::from_quorum_proposal(&wrapper).commit()
}

// TODO: Remove this and use the actual upgrade lock
pub fn upgrade_lock<T: NodeType>() -> UpgradeLock<T> {
    UpgradeLock::new(Upgrade::trivial(VID2_UPGRADE_VERSION))
}
