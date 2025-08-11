// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::collections::BTreeMap;

use anyhow::Context;
use hotshot_types::{
    drb::{
        election::{generate_stake_cdf, select_randomized_leader, RandomizedCommittee},
        DrbResult,
    },
    traits::signature_key::{
        LCV1StateSignatureKey, LCV2StateSignatureKey, LCV3StateSignatureKey, SignatureKey,
        StateSignatureKey,
    },
};

use crate::membership::stake_table::{TestStakeTable, TestStakeTableEntry};

#[derive(Clone, Debug)]

/// The randomized stake table election
pub struct RandomizedStakeTable<
    PubKey: SignatureKey,
    StatePubKey: StateSignatureKey + LCV1StateSignatureKey + LCV2StateSignatureKey + LCV3StateSignatureKey,
> {
    quorum_members: BTreeMap<PubKey, TestStakeTableEntry<PubKey, StatePubKey>>,

    da_members: BTreeMap<PubKey, TestStakeTableEntry<PubKey, StatePubKey>>,

    first_epoch: Option<u64>,

    drb_results: BTreeMap<u64, DrbResult>,

    /// Stake tables randomized with the DRB, used (only) for leader election
    randomized_committee: RandomizedCommittee<<PubKey as SignatureKey>::StakeTableEntry>,
}

impl<PubKey, StatePubKey> TestStakeTable<PubKey, StatePubKey>
    for RandomizedStakeTable<PubKey, StatePubKey>
where
    PubKey: SignatureKey,
    StatePubKey:
        StateSignatureKey + LCV1StateSignatureKey + LCV2StateSignatureKey + LCV3StateSignatureKey,
{
    fn new(
        quorum_members: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,
        da_members: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,
    ) -> Self {
        // We use a constant value of `[0u8; 32]` for the drb, since this is just meant to be used in tests
        let randomized_committee = generate_stake_cdf(
            quorum_members
                .clone()
                .into_iter()
                .map(|entry| entry.stake_table_entry)
                .collect::<Vec<_>>(),
            [0u8; 32],
        );

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
            randomized_committee,
            drb_results: BTreeMap::new(),
        }
    }

    fn stake_table(&self, _epoch: Option<u64>) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.quorum_members.values().cloned().collect()
    }

    fn da_stake_table(&self, _epoch: Option<u64>) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.da_members.values().cloned().collect()
    }

    fn stake(
        &self,
        pub_key: PubKey,
        _epoch: Option<u64>,
    ) -> Option<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.quorum_members.get(&pub_key).cloned()
    }

    fn da_stake(
        &self,
        pub_key: PubKey,
        _epoch: Option<u64>,
    ) -> Option<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.da_members.get(&pub_key).cloned()
    }

    fn lookup_leader(&self, view_number: u64, _epoch: Option<u64>) -> anyhow::Result<PubKey> {
        let res = select_randomized_leader(&self.randomized_committee, view_number);

        Ok(PubKey::public_key(&res))
    }

    fn has_stake_table(&self, _epoch: u64) -> bool {
        true
    }

    fn has_randomized_stake_table(&self, epoch: u64) -> anyhow::Result<bool> {
        Ok(self.drb_results.contains_key(&epoch))
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
