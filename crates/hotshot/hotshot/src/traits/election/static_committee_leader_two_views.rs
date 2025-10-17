// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::collections::{BTreeMap, BTreeSet};

use alloy::primitives::U256;
use anyhow::Context;
use hotshot_types::{
    da_committee::{DaCommittee, DaCommittees},
    drb::DrbResult,
    stake_table::HSStakeTable,
    traits::{
        election::{Membership, NoStakeTableHash},
        node_implementation::NodeType,
        signature_key::{SignatureKey, StakeTableEntryType},
    },
    PeerConfig,
};
use hotshot_utils::anytrace::Result;

use crate::{Arc, RwLock};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]

/// The static committee election
pub struct StaticCommitteeLeaderForTwoViews<T: NodeType> {
    /// The nodes eligible for leadership.
    /// NOTE: This is currently a hack because the DA leader needs to be the quorum
    /// leader but without voting rights.
    eligible_leaders: Vec<PeerConfig<T>>,

    /// The nodes on the committee and their stake
    stake_table: HSStakeTable<T>,

    /// The nodes on the committee and their stake, indexed by public key
    indexed_stake_table: BTreeMap<T::SignatureKey, PeerConfig<T>>,

    /// The non-epoch-based DA committee
    known_da_nodes: DaCommittee<T>,

    /// `DrbResult`s indexed by epoch
    drb_results: BTreeMap<T::Epoch, DrbResult>,

    /// DA committees, indexed by the first epoch in which they apply
    da_committees: DaCommittees<T>,
}

impl<TYPES: NodeType> Membership<TYPES> for StaticCommitteeLeaderForTwoViews<TYPES> {
    type Error = hotshot_utils::anytrace::Error;
    type StakeTableHash = NoStakeTableHash;
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

        Self {
            eligible_leaders,
            stake_table: members.into(),
            indexed_stake_table,
            known_da_nodes: DaCommittee::new(da_members),
            drb_results: BTreeMap::new(),
            da_committees: DaCommittees::default(),
        }
    }

    /// Get the stake table for the current view
    fn stake_table(&self, _epoch: Option<<TYPES as NodeType>::Epoch>) -> HSStakeTable<TYPES> {
        self.stake_table.clone()
    }

    /// Get the stake table for the current view
    fn da_stake_table(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> HSStakeTable<TYPES> {
        self.get_da_committee(epoch).committee.clone().into()
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
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> BTreeSet<<TYPES as NodeType>::SignatureKey> {
        self.get_da_committee(epoch)
            .committee
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
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> Option<PeerConfig<TYPES>> {
        // Only return the stake if it is above zero
        self.get_da_committee(epoch)
            .indexed_committee
            .get(pub_key)
            .cloned()
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
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> bool {
        self.get_da_committee(epoch)
            .indexed_committee
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
    fn da_total_nodes(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> usize {
        self.get_da_committee(epoch).len()
    }

    /// Get the voting success threshold for the committee
    fn success_threshold(&self, _epoch: Option<<TYPES as NodeType>::Epoch>) -> U256 {
        U256::from(((self.stake_table.len() as u64 * 2) / 3) + 1)
    }

    /// Get the voting success threshold for the committee
    fn da_success_threshold(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> U256 {
        U256::from(((self.da_total_nodes(epoch) as u64 * 2) / 3) + 1)
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
    fn has_randomized_stake_table(&self, _epoch: TYPES::Epoch) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn add_drb_result(&mut self, epoch: <TYPES as NodeType>::Epoch, drb_result: DrbResult) {
        self.drb_results.insert(epoch, drb_result);
    }

    async fn get_epoch_drb(
        membership: Arc<RwLock<Self>>,
        epoch: TYPES::Epoch,
    ) -> anyhow::Result<DrbResult> {
        let membership_reader = membership.read().await;

        membership_reader
            .drb_results
            .get(&epoch)
            .context("DRB result missing")
            .copied()
    }

    fn set_first_epoch(&mut self, epoch: TYPES::Epoch, initial_drb_result: DrbResult) {
        self.add_drb_result(epoch, initial_drb_result);
        self.add_drb_result(epoch + 1, initial_drb_result);
    }

    fn add_da_committee(&mut self, first_epoch: u64, da_committee: Vec<PeerConfig<TYPES>>) {
        self.da_committees.add(first_epoch, da_committee);
    }
}

impl<TYPES: NodeType> StaticCommitteeLeaderForTwoViews<TYPES> {
    fn get_da_committee(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> &DaCommittee<TYPES> {
        self.da_committees
            .get(epoch)
            .unwrap_or(&self.known_da_nodes)
    }
}
