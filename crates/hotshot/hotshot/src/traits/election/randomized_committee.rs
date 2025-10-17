// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::collections::{BTreeMap, BTreeSet};

use alloy::primitives::U256;
use anyhow::Context;
use hotshot_types::{
    da_committee::{DaCommittee, DaCommittees},
    drb::{
        election::{generate_stake_cdf, select_randomized_leader, RandomizedCommittee},
        DrbResult,
    },
    stake_table::HSStakeTable,
    traits::{
        election::{Membership, NoStakeTableHash},
        node_implementation::NodeType,
        signature_key::{SignatureKey, StakeTableEntryType},
    },
    PeerConfig,
};
use hotshot_utils::anytrace::*;

use crate::{Arc, RwLock};

#[derive(Clone, Debug)]

/// The static committee election
pub struct Committee<T: NodeType> {
    /// The nodes on the committee and their stake
    stake_table: HSStakeTable<T>,

    /// Stake tables randomized with the DRB, used (only) for leader election
    randomized_committee: RandomizedCommittee<<T::SignatureKey as SignatureKey>::StakeTableEntry>,

    /// The nodes on the committee and their stake, indexed by public key
    indexed_stake_table: BTreeMap<T::SignatureKey, PeerConfig<T>>,

    /// The non-epoch-based DA committee
    known_da_nodes: DaCommittee<T>,

    /// The first epoch which will be encountered. For testing, will panic if an epoch-carrying function is called
    /// when first_epoch is None or is Some greater than that epoch.
    first_epoch: Option<T::Epoch>,

    /// `DrbResult`s indexed by epoch
    drb_results: BTreeMap<T::Epoch, DrbResult>,

    /// DA committees, indexed by the first epoch in which they apply
    da_committees: DaCommittees<T>,
}

impl<TYPES: NodeType> Membership<TYPES> for Committee<TYPES> {
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
            .map(|config| {
                (
                    TYPES::SignatureKey::public_key(&config.stake_table_entry),
                    config.clone(),
                )
            })
            .collect();

        // We use a constant value of `[0u8; 32]` for the drb, since this is just meant to be used in tests
        let randomized_committee = generate_stake_cdf(
            eligible_leaders
                .clone()
                .into_iter()
                .map(|leader| leader.stake_table_entry)
                .collect::<Vec<_>>(),
            [0u8; 32],
        );

        Self {
            stake_table: members.into(),
            randomized_committee,
            indexed_stake_table,
            known_da_nodes: DaCommittee::new(da_members),
            first_epoch: None,
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
            .map(|x| TYPES::SignatureKey::public_key(&x.stake_table_entry))
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
            .map(|x| TYPES::SignatureKey::public_key(&x.stake_table_entry))
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

    /// Get the stake table entry for a public key
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
        let res = select_randomized_leader(&self.randomized_committee, *view_number);

        Ok(TYPES::SignatureKey::public_key(&res))
    }

    /// Get the total number of nodes in the committee
    fn total_nodes(&self, _epoch: Option<<TYPES as NodeType>::Epoch>) -> usize {
        self.stake_table.len()
    }
    /// Get the total number of nodes in the committee
    fn da_total_nodes(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> usize {
        self.get_da_committee(epoch).len()
    }
    /// Get the voting success threshold for the committee
    fn success_threshold(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> U256 {
        ((self.total_stake(epoch) * U256::from(2)) / U256::from(3)) + U256::from(1)
    }

    /// Get the voting success threshold for the committee
    fn da_success_threshold(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> U256 {
        ((self.total_da_stake(epoch) * U256::from(2)) / U256::from(3)) + U256::from(1)
    }

    /// Get the voting failure threshold for the committee
    fn failure_threshold(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> U256 {
        (self.total_stake(epoch) / U256::from(3)) + U256::from(1)
    }

    /// Get the voting upgrade threshold for the committee
    fn upgrade_threshold(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> U256 {
        let len = self.total_stake(epoch);

        U256::max(
            (len * U256::from(9)) / U256::from(10),
            ((len * U256::from(2)) / U256::from(3)) + U256::from(1),
        )
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

    fn set_first_epoch(&mut self, epoch: TYPES::Epoch, initial_drb_result: DrbResult) {
        self.first_epoch = Some(epoch);

        self.add_drb_result(epoch, initial_drb_result);
        self.add_drb_result(epoch + 1, initial_drb_result);
    }

    fn first_epoch(&self) -> Option<TYPES::Epoch> {
        self.first_epoch
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

    fn add_da_committee(&mut self, first_epoch: u64, da_committee: Vec<PeerConfig<TYPES>>) {
        self.da_committees.add(first_epoch, da_committee);
    }
}

impl<TYPES: NodeType> Committee<TYPES> {
    fn get_da_committee(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> &DaCommittee<TYPES> {
        self.da_committees
            .get(epoch)
            .unwrap_or(&self.known_da_nodes)
    }
}
