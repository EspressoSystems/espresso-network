use std::{
    collections::{BTreeMap, HashMap},
    ops::Bound,
};

use crate::{
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
    PeerConfig,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaCommittee<TYPES: NodeType> {
    pub committee: Vec<PeerConfig<TYPES>>,
    pub indexed_committee: HashMap<TYPES::SignatureKey, PeerConfig<TYPES>>,
}

impl<TYPES: NodeType> DaCommittee<TYPES> {
    pub fn new(committee: Vec<PeerConfig<TYPES>>) -> Self {
        let indexed_committee: HashMap<TYPES::SignatureKey, _> = committee
            .iter()
            .map(|peer_config| {
                (
                    TYPES::SignatureKey::public_key(&peer_config.stake_table_entry),
                    peer_config.clone(),
                )
            })
            .collect();

        Self {
            committee,
            indexed_committee,
        }
    }

    pub fn len(&self) -> usize {
        self.committee.len()
    }
}

impl<TYPES: NodeType> std::hash::Hash for DaCommittee<TYPES> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.committee.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DaCommittees<TYPES: NodeType>(pub BTreeMap<u64, DaCommittee<TYPES>>);

impl<TYPES: NodeType> Default for DaCommittees<TYPES> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl<TYPES: NodeType> DaCommittees<TYPES> {
    pub fn add(&mut self, first_epoch: u64, committee: Vec<PeerConfig<TYPES>>) {
        self.0
            .insert(first_epoch, DaCommittee::<TYPES>::new(committee));
    }

    pub fn get(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> Option<&DaCommittee<TYPES>> {
        if let Some(e) = epoch {
            // returns the greatest key smaller than or equal to `e`
            self.0
                .range((Bound::Included(&0), Bound::Included(&*e)))
                .last()
                .map(|(_, committee)| committee)
        } else {
            None
        }
    }
}
