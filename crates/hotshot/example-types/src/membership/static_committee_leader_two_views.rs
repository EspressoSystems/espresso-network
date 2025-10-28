// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
};

use anyhow::Context;
use hotshot_types::{
    drb::DrbResult,
    traits::signature_key::{
        LCV1StateSignatureKey, LCV2StateSignatureKey, LCV3StateSignatureKey, SignatureKey,
        StateSignatureKey,
    },
};

use crate::membership::stake_table::{TestDaCommittees, TestStakeTable, TestStakeTableEntry};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
/// Static stake table that doesn't use DRB results for leader election, where every leader leads for 2 views
pub struct StaticStakeTableLeaderForTwoViews<
    PubKey: SignatureKey,
    StatePubKey: StateSignatureKey + LCV1StateSignatureKey + LCV2StateSignatureKey + LCV3StateSignatureKey,
> {
    quorum_members: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,

    da_members: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,

    epochs: BTreeSet<u64>,

    drb_results: BTreeMap<u64, DrbResult>,

    first_epoch: Option<u64>,

    da_committees: TestDaCommittees<PubKey, StatePubKey>,
}

impl<PubKey, StatePubKey> TestStakeTable<PubKey, StatePubKey>
    for StaticStakeTableLeaderForTwoViews<PubKey, StatePubKey>
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
            quorum_members,
            da_members,
            first_epoch: None,
            epochs: BTreeSet::new(),
            drb_results: BTreeMap::new(),
            da_committees: TestDaCommittees::new(),
        }
    }

    fn stake_table(&self, _epoch: Option<u64>) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.quorum_members.clone()
    }

    fn da_stake_table(&self, epoch: Option<u64>) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.da_committees
            .get(epoch)
            .unwrap_or(self.da_members.clone())
    }

    fn full_stake_table(&self) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.quorum_members.clone()
    }

    fn lookup_leader(&self, view_number: u64, _epoch: Option<u64>) -> anyhow::Result<PubKey> {
        let index = (view_number / 2) as usize % self.quorum_members.len();
        let leader = self.quorum_members[index].clone();
        Ok(leader.signature_key)
    }

    fn has_stake_table(&self, epoch: u64) -> bool {
        self.epochs.contains(&epoch)
    }

    fn has_randomized_stake_table(&self, epoch: u64) -> anyhow::Result<bool> {
        Ok(self.drb_results.contains_key(&epoch))
    }

    fn add_epoch_root(&mut self, epoch: u64) {
        self.epochs.insert(epoch);
    }

    fn add_drb_result(&mut self, epoch: u64, drb_result: DrbResult) {
        self.drb_results.insert(epoch, drb_result);
    }

    fn set_first_epoch(&mut self, epoch: u64, initial_drb_result: DrbResult) {
        self.first_epoch = Some(epoch);

        self.add_epoch_root(epoch);
        self.add_epoch_root(epoch + 1);

        self.add_drb_result(epoch, initial_drb_result);
        self.add_drb_result(epoch + 1, initial_drb_result);
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

    fn add_da_committee(
        &mut self,
        first_epoch: u64,
        committee: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,
    ) {
        self.da_committees.add(first_epoch, committee);
    }
}
