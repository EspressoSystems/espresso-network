// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.
use std::{
    collections::{BTreeMap, BTreeSet},
    marker::PhantomData,
};

use alloy::primitives::U256;
use anyhow::Context;
use hotshot_types::{
    da_committee::{DaCommittee, DaCommittees},
    drb::DrbResult,
    stake_table::HSStakeTable,
    traits::{
        election::{Membership, NoStakeTableHash},
        node_implementation::{ConsensusTime, NodeType},
        signature_key::{SignatureKey, StakeTableEntryType},
    },
    PeerConfig,
};
use hotshot_utils::anytrace::Result;
use rand::{rngs::StdRng, Rng};
use tracing::error;

use crate::{traits::election::helpers::QuorumFilterConfig, Arc, RwLock};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
/// The static committee election
pub struct RandomizedCommitteeMembers<
    T: NodeType,
    CONFIG: QuorumFilterConfig,
    DaConfig: QuorumFilterConfig,
> {
    /// The nodes eligible for leadership.
    /// NOTE: This is currently a hack because the DA leader needs to be the quorum
    /// leader but without voting rights.
    eligible_leaders: Vec<PeerConfig<T>>,

    /// The nodes on the committee and their stake
    stake_table: Vec<PeerConfig<T>>,

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

    /// Phantom
    _pd: PhantomData<CONFIG>,

    /// Phantom
    _da_pd: PhantomData<DaConfig>,
}

impl<TYPES: NodeType, CONFIG: QuorumFilterConfig, DaConfig: QuorumFilterConfig>
    RandomizedCommitteeMembers<TYPES, CONFIG, DaConfig>
{
    /// Creates a set of indices into the stake_table which reference the nodes selected for this epoch's committee
    fn make_quorum_filter(&self, epoch: <TYPES as NodeType>::Epoch) -> BTreeSet<usize> {
        CONFIG::execute(epoch.u64(), self.stake_table.len())
    }

    /// Creates a set of indices into the da_stake_table which reference the nodes selected for this epoch's da committee
    fn make_da_quorum_filter(&self, epoch: <TYPES as NodeType>::Epoch) -> BTreeSet<usize> {
        DaConfig::execute(epoch.u64(), self.get_da_committee(Some(epoch)).len())
    }

    /// Writes the offsets used for the quorum filter and da_quorum filter to stdout
    fn debug_display_offsets(&self) {
        /// Ensures that the quorum filters are only displayed once
        static START: std::sync::Once = std::sync::Once::new();

        START.call_once(|| {
            error!(
                "{} offsets for Quorum filter:",
                std::any::type_name::<CONFIG>()
            );
            for epoch in 1..=10 {
                error!(
                    "  epoch {epoch}: {:?}",
                    self.make_quorum_filter(<TYPES as NodeType>::Epoch::new(epoch))
                );
            }

            error!(
                "{} offsets for DA Quorum filter:",
                std::any::type_name::<DaConfig>()
            );
            for epoch in 1..=10 {
                error!(
                    "  epoch {epoch}: {:?}",
                    self.make_da_quorum_filter(<TYPES as NodeType>::Epoch::new(epoch))
                );
            }
        });
    }

    fn get_da_committee(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> &DaCommittee<TYPES> {
        self.da_committees
            .get(epoch)
            .unwrap_or(&self.known_da_nodes)
    }
}

impl<TYPES: NodeType, CONFIG: QuorumFilterConfig, DaConfig: QuorumFilterConfig> Membership<TYPES>
    for RandomizedCommitteeMembers<TYPES, CONFIG, DaConfig>
{
    type Error = hotshot_utils::anytrace::Error;
    type StakeTableHash = NoStakeTableHash;
    /// Create a new election
    fn new(committee_members: Vec<PeerConfig<TYPES>>, da_members: Vec<PeerConfig<TYPES>>) -> Self {
        // For each eligible leader, get the stake table entry
        let eligible_leaders = committee_members
            .iter()
            .filter(|&member| member.stake_table_entry.stake() > U256::ZERO)
            .cloned()
            .collect();

        // For each member, get the stake table entry
        let members: Vec<PeerConfig<TYPES>> = committee_members
            .iter()
            .filter(|&entry| entry.stake_table_entry.stake() > U256::ZERO)
            .cloned()
            .collect();

        // For each da member, get the stake table entry
        let da_members: Vec<PeerConfig<TYPES>> = da_members
            .iter()
            .filter(|&entry| entry.stake_table_entry.stake() > U256::ZERO)
            .cloned()
            .collect();

        // Index the stake table by public key
        let indexed_stake_table = members
            .iter()
            .map(|entry| {
                (
                    TYPES::SignatureKey::public_key(&entry.stake_table_entry),
                    entry.clone(),
                )
            })
            .collect();

        let s = Self {
            eligible_leaders,
            stake_table: members,
            indexed_stake_table,
            known_da_nodes: DaCommittee::new(da_members),
            first_epoch: None,
            drb_results: BTreeMap::new(),
            da_committees: DaCommittees::default(),
            _pd: PhantomData,
            _da_pd: PhantomData,
        };

        s.debug_display_offsets();

        s
    }

    /// Get the stake table for the current view
    fn stake_table(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> HSStakeTable<TYPES> {
        if let Some(epoch) = epoch {
            let filter = self.make_quorum_filter(epoch);
            //self.stake_table.clone()s
            self.stake_table
                .iter()
                .enumerate()
                .filter(|(idx, _)| filter.contains(idx))
                .map(|(_, v)| v.clone())
                .collect()
        } else {
            self.stake_table.clone()
        }
        .into()
    }

    /// Get the da stake table for the current view
    fn da_stake_table(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> HSStakeTable<TYPES> {
        if let Some(epoch) = epoch {
            let filter = self.make_da_quorum_filter(epoch);
            //self.stake_table.clone()s
            self.get_da_committee(Some(epoch))
                .committee
                .iter()
                .enumerate()
                .filter(|(idx, _)| filter.contains(idx))
                .map(|(_, v)| v.clone())
                .collect()
        } else {
            self.get_da_committee(epoch).committee.clone()
        }
        .into()
    }

    /// Get all members of the committee for the current view
    fn committee_members(
        &self,
        _view_number: <TYPES as NodeType>::View,
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> BTreeSet<<TYPES as NodeType>::SignatureKey> {
        if let Some(epoch) = epoch {
            let filter = self.make_quorum_filter(epoch);
            self.stake_table
                .iter()
                .enumerate()
                .filter(|(idx, _)| filter.contains(idx))
                .map(|(_, v)| TYPES::SignatureKey::public_key(&v.stake_table_entry))
                .collect()
        } else {
            self.stake_table
                .iter()
                .map(|config| TYPES::SignatureKey::public_key(&config.stake_table_entry))
                .collect()
        }
    }

    /// Get all members of the committee for the current view
    fn da_committee_members(
        &self,
        _view_number: <TYPES as NodeType>::View,
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> BTreeSet<<TYPES as NodeType>::SignatureKey> {
        if let Some(epoch) = epoch {
            let filter = self.make_da_quorum_filter(epoch);
            self.get_da_committee(Some(epoch))
                .committee
                .iter()
                .enumerate()
                .filter(|(idx, _)| filter.contains(idx))
                .map(|(_, v)| TYPES::SignatureKey::public_key(&v.stake_table_entry))
                .collect()
        } else {
            self.get_da_committee(epoch)
                .committee
                .iter()
                .map(|config| TYPES::SignatureKey::public_key(&config.stake_table_entry))
                .collect()
        }
    }
    /// Get the stake table entry for a public key
    fn stake(
        &self,
        pub_key: &<TYPES as NodeType>::SignatureKey,
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> Option<PeerConfig<TYPES>> {
        if let Some(epoch) = epoch {
            let filter = self.make_quorum_filter(epoch);
            let actual_members: BTreeSet<_> = self
                .stake_table
                .iter()
                .enumerate()
                .filter(|(idx, _)| filter.contains(idx))
                .map(|(_, v)| TYPES::SignatureKey::public_key(&v.stake_table_entry))
                .collect();

            if actual_members.contains(pub_key) {
                // Only return the stake if it is above zero
                self.indexed_stake_table.get(pub_key).cloned()
            } else {
                // Skip members which aren't included based on the quorum filter
                None
            }
        } else {
            self.indexed_stake_table.get(pub_key).cloned()
        }
    }

    /// Get the da stake table entry for a public key
    fn da_stake(
        &self,
        pub_key: &<TYPES as NodeType>::SignatureKey,
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> Option<PeerConfig<TYPES>> {
        let da_committee = self.get_da_committee(epoch);
        if let Some(epoch) = epoch {
            let filter = self.make_da_quorum_filter(epoch);
            let actual_members: BTreeSet<_> = da_committee
                .committee
                .iter()
                .enumerate()
                .filter(|(idx, _)| filter.contains(idx))
                .map(|(_, v)| TYPES::SignatureKey::public_key(&v.stake_table_entry))
                .collect();

            if actual_members.contains(pub_key) {
                // Only return the stake if it is above zero
                da_committee.indexed_committee.get(pub_key).cloned()
            } else {
                // Skip members which aren't included based on the quorum filter
                None
            }
        } else {
            da_committee.indexed_committee.get(pub_key).cloned()
        }
    }

    /// Check if a node has stake in the committee
    fn has_stake(
        &self,
        pub_key: &<TYPES as NodeType>::SignatureKey,
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> bool {
        if let Some(epoch) = epoch {
            let filter = self.make_quorum_filter(epoch);
            let actual_members: BTreeSet<_> = self
                .stake_table
                .iter()
                .enumerate()
                .filter(|(idx, _)| filter.contains(idx))
                .map(|(_, v)| TYPES::SignatureKey::public_key(&v.stake_table_entry))
                .collect();

            if actual_members.contains(pub_key) {
                self.indexed_stake_table
                    .get(pub_key)
                    .is_some_and(|x| x.stake_table_entry.stake() > U256::ZERO)
            } else {
                // Skip members which aren't included based on the quorum filter
                false
            }
        } else {
            self.indexed_stake_table
                .get(pub_key)
                .is_some_and(|x| x.stake_table_entry.stake() > U256::ZERO)
        }
    }

    /// Check if a node has stake in the committee
    fn has_da_stake(
        &self,
        pub_key: &<TYPES as NodeType>::SignatureKey,
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> bool {
        let da_committee = self.get_da_committee(epoch);
        if let Some(epoch) = epoch {
            let filter = self.make_da_quorum_filter(epoch);
            let actual_members: BTreeSet<_> = da_committee
                .committee
                .iter()
                .enumerate()
                .filter(|(idx, _)| filter.contains(idx))
                .map(|(_, v)| TYPES::SignatureKey::public_key(&v.stake_table_entry))
                .collect();

            if actual_members.contains(pub_key) {
                da_committee
                    .indexed_committee
                    .get(pub_key)
                    .is_some_and(|x| x.stake_table_entry.stake() > U256::ZERO)
            } else {
                // Skip members which aren't included based on the quorum filter
                false
            }
        } else {
            da_committee
                .indexed_committee
                .get(pub_key)
                .is_some_and(|x| x.stake_table_entry.stake() > U256::ZERO)
        }
    }

    /// Index the vector of public keys with the current view number
    fn lookup_leader(
        &self,
        view_number: <TYPES as NodeType>::View,
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> Result<TYPES::SignatureKey> {
        if let Some(epoch) = epoch {
            let filter = self.make_quorum_filter(epoch);
            let leader_vec: Vec<_> = self
                .stake_table
                .iter()
                .enumerate()
                .filter(|(idx, _)| filter.contains(idx))
                .map(|(idx, v)| (idx, v.clone()))
                .collect();

            let mut rng: StdRng = rand::SeedableRng::seed_from_u64(*view_number);

            let randomized_view_number: u64 = rng.gen_range(0..=u64::MAX);
            #[allow(clippy::cast_possible_truncation)]
            let index = randomized_view_number as usize % leader_vec.len();

            let res = leader_vec[index].clone();

            tracing::debug!(
                "RandomizedCommitteeMembers lookup_leader, view_number: {view_number}, epoch: \
                 {epoch}, leader: {}",
                res.0
            );

            Ok(TYPES::SignatureKey::public_key(&res.1.stake_table_entry))
        } else {
            let mut rng: StdRng = rand::SeedableRng::seed_from_u64(*view_number);

            let randomized_view_number: u64 = rng.gen_range(0..=u64::MAX);
            #[allow(clippy::cast_possible_truncation)]
            let index = randomized_view_number as usize % self.eligible_leaders.len();

            let res = self.eligible_leaders[index].clone();

            Ok(TYPES::SignatureKey::public_key(&res.stake_table_entry))
        }
    }

    /// Get the total number of nodes in the committee
    fn total_nodes(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> usize {
        if let Some(epoch) = epoch {
            self.make_quorum_filter(epoch).len()
        } else {
            self.stake_table.len()
        }
    }

    /// Get the total number of nodes in the committee
    fn da_total_nodes(&self, epoch: Option<<TYPES as NodeType>::Epoch>) -> usize {
        if let Some(epoch) = epoch {
            self.make_da_quorum_filter(epoch).len()
        } else {
            self.get_da_committee(epoch).len()
        }
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
