// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::collections::{BTreeMap, BTreeSet};

use alloy::primitives::U256;
use hotshot_types::{
    drb::{
        election::{generate_stake_cdf, select_randomized_leader, RandomizedCommittee},
        DrbResult,
    },
    stake_table::HSStakeTable,
    traits::{
        node_implementation::{NodeImplementation, NodeType},
        signature_key::{
            LCV1StateSignatureKey, LCV2StateSignatureKey, LCV3StateSignatureKey, SignatureKey,
            StakeTableEntryType, StateSignatureKey,
        },
    },
    PeerConfig,
};
use hotshot_utils::anytrace::*;

use crate::{
    membership::stake_table::{TestStakeTable, TestStakeTableEntry},
    storage_types::TestStorage,
};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct TwoStakeTables<
    PubKey: SignatureKey,
    StatePubKey: StateSignatureKey + LCV1StateSignatureKey + LCV2StateSignatureKey + LCV3StateSignatureKey,
> {
    quorum_1_members: BTreeMap<PubKey, TestStakeTableEntry<PubKey, StatePubKey>>,

    da_1_members: BTreeMap<PubKey, TestStakeTableEntry<PubKey, StatePubKey>>,

    quorum_2_members: BTreeMap<PubKey, TestStakeTableEntry<PubKey, StatePubKey>>,

    da_2_members: BTreeMap<PubKey, TestStakeTableEntry<PubKey, StatePubKey>>,

    first_epoch: Option<u64>,
}

impl<PubKey, StatePubKey> TestStakeTable<PubKey, StatePubKey>
    for TwoStakeTables<PubKey, StatePubKey>
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
            quorum_1_members: quorum_members
                .iter()
                .enumerate()
                .filter(|(idx, _)| idx % 2 == 0)
                .map(|(_, leader)| leader.clone())
                .map(|entry| (entry.signature_key.clone(), entry.clone()))
                .collect(),
            da_1_members: da_members
                .iter()
                .enumerate()
                .filter(|(idx, _)| idx % 2 == 0)
                .map(|(_, leader)| leader.clone())
                .map(|entry| (entry.signature_key.clone(), entry.clone()))
                .collect(),
            quorum_2_members: quorum_members
                .iter()
                .enumerate()
                .filter(|(idx, _)| idx % 2 == 1)
                .map(|(_, leader)| leader.clone())
                .map(|entry| (entry.signature_key.clone(), entry.clone()))
                .collect(),
            da_2_members: da_members
                .iter()
                .enumerate()
                .filter(|(idx, _)| idx % 2 == 1)
                .map(|(_, leader)| leader.clone())
                .map(|entry| (entry.signature_key.clone(), entry.clone()))
                .collect(),
            first_epoch: None,
        }
    }

    fn stake_table(&self, epoch: Option<u64>) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>> {
        let epoch = epoch.expect("epochs cannot be disabled with TwoStakeTables");
        if epoch != 0 && epoch % 2 == 0 {
            self.quorum_1_members.values().cloned().collect()
        } else {
            self.quorum_2_members.values().cloned().collect()
        }
    }

    fn da_stake_table(&self, epoch: Option<u64>) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>> {
        let epoch = epoch.expect("epochs cannot be disabled with TwoStakeTables");
        if epoch != 0 && epoch % 2 == 0 {
            self.da_1_members.values().cloned().collect()
        } else {
            self.da_2_members.values().cloned().collect()
        }
    }

    fn stake(
        &self,
        pub_key: PubKey,
        epoch: Option<u64>,
    ) -> Option<TestStakeTableEntry<PubKey, StatePubKey>> {
        let epoch = epoch.expect("epochs cannot be disabled with TwoStakeTables");
        if epoch != 0 && epoch % 2 == 0 {
            self.quorum_1_members.get(&pub_key).cloned()
        } else {
            self.quorum_2_members.get(&pub_key).cloned()
        }
    }

    fn da_stake(
        &self,
        pub_key: PubKey,
        epoch: Option<u64>,
    ) -> Option<TestStakeTableEntry<PubKey, StatePubKey>> {
        let epoch = epoch.expect("epochs cannot be disabled with TwoStakeTables");
        if epoch != 0 && epoch % 2 == 0 {
            self.da_1_members.get(&pub_key).cloned()
        } else {
            self.da_2_members.get(&pub_key).cloned()
        }
    }

    fn lookup_leader(&self, view_number: u64, epoch: Option<u64>) -> anyhow::Result<PubKey> {
        let epoch = epoch.expect("epochs cannot be disabled with TwoStakeTables");
        if epoch != 0 && epoch % 2 == 0 {
            let index = view_number as usize % self.quorum_1_members.len();
            let leader = self.quorum_1_members.values().collect::<Vec<_>>()[index].clone();
            Ok(leader.signature_key)
        } else {
            let index = view_number as usize % self.quorum_2_members.len();
            let leader = self.quorum_2_members.values().collect::<Vec<_>>()[index].clone();
            Ok(leader.signature_key)
        }
    }

    fn has_stake_table(&self, _epoch: u64) -> bool {
        true
    }

    fn has_randomized_stake_table(&self, _epoch: u64) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn add_drb_result(&mut self, _epoch: u64, _drb_result: DrbResult) {}

    fn set_first_epoch(&mut self, epoch: u64, _initial_drb_result: DrbResult) {
        self.first_epoch = Some(epoch);
    }

    fn first_epoch(&self) -> Option<u64> {
        self.first_epoch
    }
}
