use committable::{Commitment, Committable};
use hotshot_types::{
    data::{Leaf2, QuorumProposal2, QuorumProposalWrapper},
    message::UpgradeLock,
    traits::node_implementation::NodeType,
};
use versions::{Upgrade, VID2_UPGRADE_VERSION};

pub fn proposal_commitment<TYPES: NodeType>(
    proposal: &QuorumProposal2<TYPES>,
) -> Commitment<Leaf2<TYPES>> {
    let wrapper = QuorumProposalWrapper::from(proposal.clone());
    Leaf2::from_quorum_proposal(&wrapper).commit()
}

pub fn upgrade_lock<TYPES: NodeType>() -> UpgradeLock<TYPES> {
    UpgradeLock::new(Upgrade::trivial(VID2_UPGRADE_VERSION))
}
