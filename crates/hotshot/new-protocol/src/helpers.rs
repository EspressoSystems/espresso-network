use std::collections::{VecDeque, vec_deque};

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

#[derive(Debug)]
pub struct Outbox<T>(VecDeque<T>);

impl<T> Default for Outbox<T> {
    fn default() -> Self {
        Self(VecDeque::new())
    }
}

impl<T> Outbox<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn clear(&mut self) {
        self.0.clear()
    }

    pub fn push_back<U: Into<T>>(&mut self, item: U) {
        self.0.push_back(item.into())
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.0.pop_front()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.iter()
    }
}

impl<'a, T> IntoIterator for &'a Outbox<T> {
    type IntoIter = vec_deque::Iter<'a, T>;
    type Item = &'a T;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<T: PartialEq> Outbox<T> {
    pub fn contains(&self, item: &T) -> bool {
        self.0.contains(item)
    }
}
