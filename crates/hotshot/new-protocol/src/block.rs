use hotshot_types::{
    data::{EpochNumber, QuorumProposal2, ViewNumber},
    traits::node_implementation::NodeType,
};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct BlockAndHeaderRequest<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub parent_proposal: QuorumProposal2<T>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct BlockRequest<T: NodeType> {
    pub view: ViewNumber,
    pub parent_proposal: QuorumProposal2<T>,
    pub epoch: EpochNumber,
}
