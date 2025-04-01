// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::collections::{BTreeMap, BTreeSet};

use alloy::primitives::U256;
use hotshot_types::{
    drb::DrbResult,
    traits::{
        election::Membership,
        node_implementation::NodeType,
        signature_key::{SignatureKey, StakeTableEntryType},
    },
    PeerConfig,
};
use hotshot_utils::anytrace::Result;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]

/// The static committee election
pub struct StaticCommitteeLeaderForTwoViews<T: NodeType> {
    /// The nodes eligible for leadership.
    /// NOTE: This is currently a hack because the DA leader needs to be the quorum
    /// leader but without voting rights.
    eligible_leaders: Vec<PeerConfig<T>>,

    /// The nodes on the committee and their stake
    stake_table: Vec<PeerConfig<T>>,

    /// The nodes on the committee and their stake
    da_stake_table: Vec<PeerConfig<T>>,

    /// The nodes on the committee and their stake, indexed by public key
    indexed_stake_table: BTreeMap<T::SignatureKey, PeerConfig<T>>,

    /// The nodes on the committee and their stake, indexed by public key
    indexed_da_stake_table: BTreeMap<T::SignatureKey, PeerConfig<T>>,
}

impl<TYPES: NodeType> Membership<TYPES> for StaticCommitteeLeaderForTwoViews<TYPES> {
    type Error = hotshot_utils::anytrace::Error;
    /// Create a new election
    fn new(committee_members: Vec<PeerConfig<TYPES>>, da_members: Vec<PeerConfig<TYPES>>) -> Self {
        // For each eligible leader, get the stake table entry
        let eligible_leaders: Vec<PeerConfig<TYPES>> = committee_members
            .iter()
            .filter(|&member| member.stake_table_entry.stake() > U256::ZERO)
            .cloned()
            .collect();

        // For each member, get the stake table entry
        let members: Vec<PeerConfig<TYPES>> = committee_members
            .iter()
            .filter(|&member| member.stake_table_entry.stake() > U256::ZERO)
            .cloned()
            .collect();

        // For each member, get the stake table entry
        let da_members: Vec<PeerConfig<TYPES>> = da_members
            .iter()
            .filter(|&member| member.stake_table_entry.stake() > U256::ZERO)
            .cloned()
            .collect();

        // Index the stake table by public key
        let indexed_stake_table: BTreeMap<TYPES::SignatureKey, PeerConfig<TYPES>> = members
            .iter()
            .map(|member| {
                (
                    TYPES::SignatureKey::public_key(&member.stake_table_entry),
                    member.clone(),
                )
            })
            .collect();

        // Index the stake table by public key
        let indexed_da_stake_table: BTreeMap<TYPES::SignatureKey, PeerConfig<TYPES>> = da_members
            .iter()
            .map(|member| {
                (
                    TYPES::SignatureKey::public_key(&member.stake_table_entry),
                    member.clone(),
                )
            })
            .collect();

        Self {
            eligible_leaders,
            stake_table: members,
            da_stake_table: da_members,
            indexed_stake_table,
            indexed_da_stake_table,
        }
    }

    /// Get the stake table for the current view
    fn stake_table(&self, _epoch: Option<<TYPES as NodeType>::Epoch>) -> Vec<PeerConfig<TYPES>> {
        self.stake_table.clone()
    }

    /// Get the stake table for the current view
    fn da_stake_table(&self, _epoch: Option<<TYPES as NodeType>::Epoch>) -> Vec<PeerConfig<TYPES>> {
        self.da_stake_table.clone()
    }

    /// Get all members of the committee for the current view
    fn committee_members(
        &self,
        _view_number: <TYPES as NodeType>::View,
        _epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> BTreeSet<<TYPES as NodeType>::SignatureKey> {
        self.stake_table
            .iter()
            .map(|sc| TYPES::SignatureKey::public_key(&sc.stake_table_entry))
            .collect()
    }

    /// Get all members of the committee for the current view
    fn da_committee_members(
        &self,
        _view_number: <TYPES as NodeType>::View,
        _epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> BTreeSet<<TYPES as NodeType>::SignatureKey> {
        self.da_stake_table
            .iter()
            .map(|da| TYPES::SignatureKey::public_key(&da.stake_table_entry))
            .collect()
    }

    /// Get the stake table entry for a public key
    fn stake(
        &self,
        pub_key: &<TYPES as NodeType>::SignatureKey,
        _epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> Option<PeerConfig<TYPES>> {
        // Only return the stake if it is above zero
        self.indexed_stake_table.get(pub_key).cloned()
    }

    /// Get DA the stake table entry for a public key
    fn da_stake(
        &self,
        pub_key: &<TYPES as NodeType>::SignatureKey,
        _epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> Option<PeerConfig<TYPES>> {
        // Only return the stake if it is above zero
        self.indexed_da_stake_table.get(pub_key).cloned()
    }

    /// Check if a node has stake in the committee
    fn has_stake(
        &self,
        pub_key: &<TYPES as NodeType>::SignatureKey,
        _epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> bool {
        self.indexed_stake_table
            .get(pub_key)
            .is_some_and(|x| x.stake_table_entry.stake() > U256::ZERO)
    }

    /// Check if a node has stake in the committee
    fn has_da_stake(
        &self,
        pub_key: &<TYPES as NodeType>::SignatureKey,
        _epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> bool {
        self.indexed_da_stake_table
            .get(pub_key)
            .is_some_and(|x| x.stake_table_entry.stake() > U256::ZERO)
    }

    /// Index the vector of public keys with the current view number
    fn lookup_leader(
        &self,
        view_number: <TYPES as NodeType>::View,
        _epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> Result<TYPES::SignatureKey> {
        let index =
            usize::try_from((*view_number / 2) % self.eligible_leaders.len() as u64).unwrap();
        let res = self.eligible_leaders[index].clone();

        Ok(TYPES::SignatureKey::public_key(&res.stake_table_entry))
    }

    /// Get the total number of nodes in the committee
    fn total_nodes(&self, _epoch: Option<<TYPES as NodeType>::Epoch>) -> usize {
        self.stake_table.len()
    }

    /// Get the total number of DA nodes in the committee
    fn da_total_nodes(&self, _epoch: Option<<TYPES as NodeType>::Epoch>) -> usize {
        self.da_stake_table.len()
    }

    /// Get the voting success threshold for the committee
    fn success_threshold(&self, _epoch: Option<<TYPES as NodeType>::Epoch>) -> U256 {
        U256::from(((self.stake_table.len() as u64 * 2) / 3) + 1)
    }

    /// Get the voting success threshold for the committee
    fn da_success_threshold(&self, _epoch: Option<<TYPES as NodeType>::Epoch>) -> U256 {
        U256::from(((self.da_stake_table.len() as u64 * 2) / 3) + 1)
    }

    /// Get the voting failure threshold for the committee
    fn failure_threshold(&self, _epoch: Option<<TYPES as NodeType>::Epoch>) -> U256 {
        U256::from(((self.stake_table.len() as u64) / 3) + 1)
    }

    /// Get the voting upgrade threshold for the committee
    fn upgrade_threshold(&self, _epoch: Option<<TYPES as NodeType>::Epoch>) -> U256 {
        U256::from(((self.stake_table.len() as u64 * 9) / 10) + 1)
    }
    fn has_stake_table(&self, _epoch: TYPES::Epoch) -> bool {
        true
    }
    fn has_randomized_stake_table(&self, _epoch: TYPES::Epoch) -> bool {
        true
    }

    fn add_drb_result(&mut self, _epoch: <TYPES as NodeType>::Epoch, _drb_result: DrbResult) {}

    fn set_first_epoch(&mut self, _epoch: TYPES::Epoch, _initial_drb_result: DrbResult) {}
}
