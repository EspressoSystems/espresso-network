// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::collections::{BTreeMap, BTreeSet};

use alloy::primitives::U256;
use anyhow::Context;
use hotshot_types::{
    drb::DrbResult,
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

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
/// The static committee election
pub struct StaticCommittee<T: NodeType> {
    /// The nodes eligible for leadership.
    /// NOTE: This is currently a hack because the DA leader needs to be the quorum
    /// leader but without voting rights.
    eligible_leaders: Vec<PeerConfig<T>>,

    /// The nodes on the committee and their stake
    stake_table: HSStakeTable<T>,

    /// The nodes on the committee and their stake
    da_stake_table: HSStakeTable<T>,

    /// The nodes on the committee and their stake, indexed by public key
    indexed_stake_table: BTreeMap<T::SignatureKey, PeerConfig<T>>,

    /// The nodes on the committee and their stake, indexed by public key
    indexed_da_stake_table: BTreeMap<T::SignatureKey, PeerConfig<T>>,

    /// The first epoch which will be encountered. For testing, will panic if an epoch-carrying function is called
    /// when first_epoch is None or is Some greater than that epoch.
    first_epoch: Option<T::Epoch>,

    /// `DrbResult`s indexed by epoch
    drb_results: BTreeMap<T::Epoch, DrbResult>,
}

impl<TYPES: NodeType> StaticCommittee<TYPES> {
    fn check_first_epoch(&self, epoch: Option<<TYPES as NodeType>::Epoch>) {
        if let Some(epoch) = epoch {
            if let Some(first_epoch) = self.first_epoch {
                assert!(
                    first_epoch <= epoch,
                    "Called a method in StaticCommittee where first_epoch={first_epoch:} but \
                     epoch={epoch}"
                );
            } else {
                panic!(
                    "Called a method in StaticCommittee with non-None epoch={epoch}, but \
                     set_first_epoch was not yet called"
                );
            }
        }
    }
}

impl<TYPES: NodeType> Membership<TYPES> for StaticCommittee<TYPES> {
    type Error = hotshot_utils::anytrace::Error;
    type StakeTableHash = NoStakeTableHash;
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
            stake_table: members.into(),
            da_stake_table: da_members.into(),
            indexed_stake_table,
            indexed_da_stake_table,
            first_epoch: None,
            drb_results: BTreeMap::new(),
        }
    }

    /// Get the stake table for the current view
    fn stake_table(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> HSStakeTable<TYPES> {
        self.check_first_epoch(epoch);
        self.stake_table.clone()
    }

    /// Get the stake table for the current view
    fn da_stake_table(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> HSStakeTable<TYPES> {
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
        if self.eligible_leaders.is_empty() {
            return Err(Error {
                level: Level::Unspecified,
                message: "No eligible leaders configured".to_string(),
            });
        }
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
        ((self.total_stake(epoch) * U256::from(2)) / U256::from(3)) + U256::from(1)
    }

    /// Get the voting success threshold for the committee
    fn da_success_threshold(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> U256 {
        self.check_first_epoch(epoch);
        ((self.total_da_stake(epoch) * U256::from(2)) / U256::from(3)) + U256::from(1)
    }

    /// Get the voting failure threshold for the committee
    fn failure_threshold(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> U256 {
        self.check_first_epoch(epoch);
        (self.total_stake(epoch) / U256::from(3)) + U256::from(1)
    }

    /// Get the voting upgrade threshold for the committee
    fn upgrade_threshold(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> U256 {
        self.check_first_epoch(epoch);
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
}
