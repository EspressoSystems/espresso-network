use std::{collections::BTreeMap, ops::RangeBounds};

use committable::Commitment;
use hotshot_types::{
    data::{Leaf2, ViewNumber},
    traits::node_implementation::NodeType,
};

use crate::message::Proposal;

/// The proposals a node holds, by view and commitment.
///
/// A view normally holds one. An equivocating leader can send different
/// proposals for its view to different nodes, and at an epoch boundary two
/// leaders can propose for one view, so a node can hold several. Every
/// certificate names its proposal by commitment, and whatever a node does on a
/// certificate's behalf looks the proposal up by that commitment. So no proposal
/// ever has to give way to another, whatever order they arrive in.
///
/// `live` is, per view, the proposal the node paired with its own VID share.
pub struct Proposals<T: NodeType> {
    live: BTreeMap<ViewNumber, Commitment<Leaf2<T>>>,
    all: BTreeMap<ViewNumber, BTreeMap<Commitment<Leaf2<T>>, Proposal<T>>>,
}

impl<T: NodeType> Proposals<T> {
    pub fn new() -> Self {
        Self {
            live: BTreeMap::new(),
            all: BTreeMap::new(),
        }
    }

    pub fn live(&self, v: ViewNumber) -> Option<&Proposal<T>> {
        self.get(v, *self.live.get(&v)?)
    }

    pub fn at(&self, v: ViewNumber) -> impl Iterator<Item = &Proposal<T>> {
        self.all.get(&v).into_iter().flat_map(|a| a.values())
    }

    pub fn get(&self, v: ViewNumber, c: Commitment<Leaf2<T>>) -> Option<&Proposal<T>> {
        self.all.get(&v)?.get(&c)
    }

    pub fn contains(&self, v: ViewNumber, c: Commitment<Leaf2<T>>) -> bool {
        self.get(v, c).is_some()
    }

    pub fn range<R>(&self, r: R) -> impl Iterator<Item = &Proposal<T>>
    where
        R: RangeBounds<ViewNumber>,
    {
        self.all.range(r).flat_map(|(_, a)| a.values())
    }

    pub fn last(&self) -> Option<&Proposal<T>> {
        let (&view, all) = self.all.last_key_value()?;
        self.live(view).or_else(|| all.values().next())
    }

    pub fn insert_under(&mut self, p: Proposal<T>, c: Commitment<Leaf2<T>>) {
        self.all.entry(p.view_number).or_default().insert(c, p);
    }

    pub fn set_live(&mut self, v: ViewNumber, c: Commitment<Leaf2<T>>) {
        self.live.insert(v, c);
    }

    pub fn retain_from(&mut self, view: ViewNumber) {
        self.all = self.all.split_off(&view);
        self.live = self.live.split_off(&view);
    }

    #[cfg(test)]
    pub fn insert(&mut self, p: Proposal<T>) -> Commitment<Leaf2<T>> {
        let c = crate::helpers::proposal_commitment(&p);
        self.insert_under(p, c);
        c
    }

    #[cfg(test)]
    pub(crate) fn replace_view(&mut self, proposal: Proposal<T>) {
        let view = proposal.view_number;
        self.all.remove(&view);
        self.live.remove(&view);
        self.insert(proposal);
    }
}
