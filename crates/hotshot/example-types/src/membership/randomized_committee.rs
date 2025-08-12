// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::collections::{BTreeMap, BTreeSet};

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
    quorum_members: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,

    da_members: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,

    first_epoch: Option<u64>,

    epochs: BTreeSet<u64>,

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
            quorum_members: quorum_members,
            da_members: da_members,
            first_epoch: None,
            randomized_committee,
            epochs: BTreeSet::new(),
            drb_results: BTreeMap::new(),
        }
    }

    fn stake_table(&self, _epoch: Option<u64>) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.quorum_members.clone()
    }

    fn da_stake_table(&self, _epoch: Option<u64>) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.da_members.clone()
    }

    fn lookup_leader(&self, view_number: u64, _epoch: Option<u64>) -> anyhow::Result<PubKey> {
        let res = select_randomized_leader(&self.randomized_committee, view_number);

        Ok(PubKey::public_key(&res))
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
}
