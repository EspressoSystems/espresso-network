// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    cmp::max,
    collections::{BTreeMap, BTreeSet},
};

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
use hotshot_utils::anytrace::*;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
/// The static committee election
pub struct StaticCommittee<T: NodeType> {
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

    /// The first epoch which will be encountered. For testing, will panic if an epoch-carrying function is called
    /// when first_epoch is None or is Some greater than that epoch.
    first_epoch: Option<T::Epoch>,
}

impl<TYPES: NodeType> StaticCommittee<TYPES> {
    fn check_first_epoch(&self, epoch: Option<<TYPES as NodeType>::Epoch>) {
        if let Some(epoch) = epoch {
            if let Some(first_epoch) = self.first_epoch {
                assert!(first_epoch <= epoch, "Called a method in StaticCommittee where first_epoch={first_epoch:} but epoch={epoch}");
            } else {
                panic!("Called a method in StaticCommittee with non-None epoch={epoch}, but set_first_epoch was not yet called");
            }
        }
    }
}

impl<TYPES: NodeType> Membership<TYPES> for StaticCommittee<TYPES> {
    type Error = hotshot_utils::anytrace::Error;
    /// Create a new election
    fn new(committee_members: Vec<PeerConfig<TYPES>>, da_members: Vec<PeerConfig<TYPES>>) -> Self {
        // For each eligible leader, get the stake table entry
        let eligible_leaders: Vec<PeerConfig<TYPES>> = committee_members
            .clone()
            .into_iter()
            .filter(|member| member.stake_table_entry.stake() > U256::ZERO)
            .collect();

        // For each member, get the stake table entry
        let members: Vec<PeerConfig<TYPES>> = committee_members
            .into_iter()
            .filter(|member| member.stake_table_entry.stake() > U256::ZERO)
            .collect();

        // For each member, get the stake table entry
        let da_members: Vec<PeerConfig<TYPES>> = da_members
            .into_iter()
            .filter(|member| member.stake_table_entry.stake() > U256::ZERO)
            .collect();

        // Index the stake table by public key
        let indexed_stake_table: BTreeMap<TYPES::SignatureKey, _> = members
            .iter()
            .map(|entry| {
                (
                    TYPES::SignatureKey::public_key(&entry.stake_table_entry),
                    entry.clone(),
                )
            })
            .collect();

        // Index the stake table by public key
        let indexed_da_stake_table: BTreeMap<TYPES::SignatureKey, _> = da_members
            .iter()
            .map(|entry| {
                (
                    TYPES::SignatureKey::public_key(&entry.stake_table_entry),
                    entry.clone(),
                )
            })
            .collect();

        Self {
            eligible_leaders,
            stake_table: members,
            da_stake_table: da_members,
            indexed_stake_table,
            indexed_da_stake_table,
            first_epoch: None,
        }
    }

    /// Get the stake table for the current view
    fn stake_table(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> Vec<PeerConfig<TYPES>> {
        self.check_first_epoch(epoch);
        self.stake_table.clone()
    }

    /// Get the stake table for the current view
    fn da_stake_table(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> Vec<PeerConfig<TYPES>> {
        self.check_first_epoch(epoch);
        self.da_stake_table.clone()
    }

    /// Get all members of the committee for the current view
    fn committee_members(
        &self,
        _view_number: <TYPES as NodeType>::View,
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> BTreeSet<<TYPES as NodeType>::SignatureKey> {
        self.check_first_epoch(epoch);
        self.stake_table
            .iter()
            .map(|sc| TYPES::SignatureKey::public_key(&sc.stake_table_entry))
            .collect()
    }

    /// Get all members of the committee for the current view
    fn da_committee_members(
        &self,
        _view_number: <TYPES as NodeType>::View,
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> BTreeSet<<TYPES as NodeType>::SignatureKey> {
        self.check_first_epoch(epoch);
        self.da_stake_table
            .iter()
            .map(|da| TYPES::SignatureKey::public_key(&da.stake_table_entry))
            .collect()
    }

    /// Get the stake table entry for a public key
    fn stake(
        &self,
        pub_key: &<TYPES as NodeType>::SignatureKey,
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> Option<PeerConfig<TYPES>> {
        self.check_first_epoch(epoch);
        // Only return the stake if it is above zero
        self.indexed_stake_table.get(pub_key).cloned()
    }

    /// Get the DA stake table entry for a public key
    fn da_stake(
        &self,
        pub_key: &<TYPES as NodeType>::SignatureKey,
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> Option<PeerConfig<TYPES>> {
        self.check_first_epoch(epoch);
        // Only return the stake if it is above zero
        self.indexed_da_stake_table.get(pub_key).cloned()
    }

    /// Check if a node has stake in the committee
    fn has_stake(
        &self,
        pub_key: &<TYPES as NodeType>::SignatureKey,
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> bool {
        self.check_first_epoch(epoch);
        self.indexed_stake_table
            .get(pub_key)
            .is_some_and(|x| x.stake_table_entry.stake() > U256::ZERO)
    }

    /// Check if a node has stake in the committee
    fn has_da_stake(
        &self,
        pub_key: &<TYPES as NodeType>::SignatureKey,
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> bool {
        self.check_first_epoch(epoch);
        self.indexed_da_stake_table
            .get(pub_key)
            .is_some_and(|x| x.stake_table_entry.stake() > U256::ZERO)
    }

    /// Index the vector of public keys with the current view number
    fn lookup_leader(
        &self,
        view_number: <TYPES as NodeType>::View,
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> Result<TYPES::SignatureKey> {
        self.check_first_epoch(epoch);
        #[allow(clippy::cast_possible_truncation)]
        let index = *view_number as usize % self.eligible_leaders.len();
        let res = self.eligible_leaders[index].clone();
        Ok(TYPES::SignatureKey::public_key(&res.stake_table_entry))
    }

    /// Get the total number of nodes in the committee
    fn total_nodes(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> usize {
        self.check_first_epoch(epoch);
        self.stake_table.len()
    }

    /// Get the total number of DA nodes in the committee
    fn da_total_nodes(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> usize {
        self.check_first_epoch(epoch);
        self.da_stake_table.len()
    }

    /// Get the voting success threshold for the committee
    fn success_threshold(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> U256 {
        self.check_first_epoch(epoch);
        U256::from(((self.stake_table.len() as u64 * 2) / 3) + 1)
    }

    /// Get the voting success threshold for the committee
    fn da_success_threshold(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> U256 {
        self.check_first_epoch(epoch);
        U256::from(((self.da_stake_table.len() as u64 * 2) / 3) + 1)
    }

    /// Get the voting failure threshold for the committee
    fn failure_threshold(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> U256 {
        self.check_first_epoch(epoch);
        U256::from(((self.stake_table.len() as u64) / 3) + 1)
    }

    /// Get the voting upgrade threshold for the committee
    fn upgrade_threshold(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> U256 {
        self.check_first_epoch(epoch);
        let len = self.stake_table.len();
        U256::from(max((len as u64 * 9) / 10, ((len as u64 * 2) / 3) + 1))
    }
    fn has_stake_table(&self, _epoch: TYPES::Epoch) -> bool {
        true
    }
    fn has_randomized_stake_table(&self, _epoch: TYPES::Epoch) -> bool {
        true
    }

    fn add_drb_result(&mut self, _epoch: <TYPES as NodeType>::Epoch, _drb_result: DrbResult) {}

    fn set_first_epoch(&mut self, epoch: TYPES::Epoch, _initial_drb_result: DrbResult) {
        self.first_epoch = Some(epoch);
    }
}
