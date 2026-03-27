use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    traits::node_implementation::NodeType,
};

use crate::message::Proposal;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct BlockAndHeaderRequest<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub parent_proposal: Proposal<T>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct BlockRequest<T: NodeType> {
    pub view: ViewNumber,
    pub parent_proposal: Proposal<T>,
    pub epoch: EpochNumber,
}
