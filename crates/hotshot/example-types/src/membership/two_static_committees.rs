// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::collections::BTreeMap;

use anyhow::Context;
use hotshot_types::{
    drb::DrbResult,
    traits::signature_key::{
        LCV1StateSignatureKey, LCV2StateSignatureKey, LCV3StateSignatureKey, SignatureKey,
        StateSignatureKey,
    },
};

use crate::membership::stake_table::{TestStakeTable, TestStakeTableEntry};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct TwoStakeTables<
    PubKey: SignatureKey,
    StatePubKey: StateSignatureKey + LCV1StateSignatureKey + LCV2StateSignatureKey + LCV3StateSignatureKey,
> {
    quorum_1_members: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,

    da_1_members: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,

    quorum_2_members: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,

    da_2_members: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,

    drb_results: BTreeMap<u64, DrbResult>,

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
                .map(|(_, entry)| entry.clone())
                .collect(),
            da_1_members: da_members
                .iter()
                .enumerate()
                .filter(|(idx, _)| idx % 2 == 0)
                .map(|(_, entry)| entry.clone())
                .collect(),
            quorum_2_members: quorum_members
                .iter()
                .enumerate()
                .filter(|(idx, _)| idx % 2 == 1)
                .map(|(_, entry)| entry.clone())
                .collect(),
            da_2_members: da_members
                .iter()
                .enumerate()
                .filter(|(idx, _)| idx % 2 == 1)
                .map(|(_, entry)| entry.clone())
                .collect(),
            first_epoch: None,
            drb_results: BTreeMap::new(),
        }
    }

    fn stake_table(&self, epoch: Option<u64>) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>> {
        let epoch = epoch.expect("epochs cannot be disabled with TwoStakeTables");
        if epoch != 0 && epoch % 2 == 0 {
            self.quorum_1_members.clone()
        } else {
            self.quorum_2_members.clone()
        }
    }

    fn da_stake_table(&self, epoch: Option<u64>) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>> {
        let epoch = epoch.expect("epochs cannot be disabled with TwoStakeTables");
        if epoch != 0 && epoch % 2 == 0 {
            self.da_1_members.clone()
        } else {
            self.da_2_members.clone()
        }
    }

    fn lookup_leader(&self, view_number: u64, epoch: Option<u64>) -> anyhow::Result<PubKey> {
        let stake_table = self.stake_table(epoch);

        let index = view_number as usize % stake_table.len();
        let leader = stake_table[index].clone();

        Ok(leader.signature_key)
    }

    fn has_stake_table(&self, _epoch: u64) -> bool {
        true
    }

    fn has_randomized_stake_table(&self, _epoch: u64) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn add_drb_result(&mut self, epoch: u64, drb_result: DrbResult) {
        self.drb_results.insert(epoch, drb_result);
    }

    fn set_first_epoch(&mut self, epoch: u64, initial_drb_result: DrbResult) {
        self.first_epoch = Some(epoch);

        self.drb_results.insert(epoch, initial_drb_result);
        self.drb_results.insert(epoch + 1, initial_drb_result);
    }

    fn get_epoch_drb(&self, epoch: u64) -> anyhow::Result<DrbResult> {
        self.drb_results
            .get(&epoch)
            .context("DRB result missing")
            .copied()
    }

    fn first_epoch(&self) -> Option<u64> {
        self.first_epoch
    }
}
