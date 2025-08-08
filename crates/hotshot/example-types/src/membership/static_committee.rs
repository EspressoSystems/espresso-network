// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.


use std::fmt::Debug;
use crate::membership::stake_table::TestStakeTableEntry;
use std::collections::BTreeMap;
use crate::membership::stake_table::TestStakeTable;
use hotshot_types::traits::signature_key::StakeTableEntryType;
use alloy::primitives::U256;
use hotshot_types::{
    drb::DrbResult,
    traits::{
        node_implementation::NodeType,
        signature_key::{
            LCV1StateSignatureKey, LCV2StateSignatureKey, LCV3StateSignatureKey, SignatureKey,
            StateSignatureKey,
        },
    },
    PeerConfig,
};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
/// Static stake table that doesn't use DRB results for leader election
pub struct StaticStakeTable<
    PubKey: SignatureKey,
    StatePubKey: StateSignatureKey + LCV1StateSignatureKey + LCV2StateSignatureKey + LCV3StateSignatureKey,
> {
    quorum_members: BTreeMap<PubKey, TestStakeTableEntry<PubKey, StatePubKey>>,

    da_members: BTreeMap<PubKey, TestStakeTableEntry<PubKey, StatePubKey>>,

    first_epoch: Option<u64>,
}

impl<PubKey, StatePubKey> TestStakeTable<PubKey, StatePubKey>
    for StaticStakeTable<PubKey, StatePubKey>
where
    PubKey: SignatureKey,
    StatePubKey:
        StateSignatureKey + LCV1StateSignatureKey + LCV2StateSignatureKey + LCV3StateSignatureKey,
{
    fn new(
        quorum_members: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,
        da_members: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,
    ) -> Self {
        Self {
            quorum_members: quorum_members
                .iter()
                .map(|entry| (entry.signature_key.clone(), entry.clone()))
                .collect(),
            da_members: da_members
                .iter()
                .map(|entry| (entry.signature_key.clone(), entry.clone()))
                .collect(),
                first_epoch: None,
        }
    }

    fn stake_table(&self, epoch: Option<u64>) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.quorum_members.values().cloned().collect()
    }

    fn da_stake_table(&self, epoch: Option<u64>) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.da_members.values().cloned().collect()
    }

    fn stake(
        &self,
        pub_key: PubKey,
        epoch: Option<u64>,
    ) -> Option<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.quorum_members.get(&pub_key).cloned()
    }

    fn da_stake(
        &self,
        pub_key: PubKey,
        epoch: Option<u64>,
    ) -> Option<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.da_members.get(&pub_key).cloned()
    }

    fn total_stake(&self, epoch: Option<u64>) -> U256 {
        self.quorum_members.values().fold(U256::ZERO, |acc, entry| {
            acc + entry.stake_table_entry.stake()
        })
    }

    fn lookup_leader(&self, view_number: u64, epoch: Option<u64>) -> anyhow::Result<PubKey> {
        let index = view_number as usize % self.quorum_members.len();
        let leader = self.quorum_members.values().collect::<Vec<_>>()[index].clone();
        Ok(leader.signature_key)
    }

    fn has_stake_table(&self, epoch: u64) -> bool {
        true
    }

    fn has_randomized_stake_table(&self, epoch: u64) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn add_drb_result(&mut self, epoch: u64, drb_result: DrbResult) {}

    fn set_first_epoch(&mut self, epoch: u64, initial_drb_result: DrbResult) {
        self.first_epoch = Some(epoch);
    }

    fn first_epoch(&self) -> Option<u64> {
        self.first_epoch
    }
}
