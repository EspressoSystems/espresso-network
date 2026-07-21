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

use crate::membership::stake_table::{TestCommitteeSchedule, TestStakeTable, TestStakeTableEntry};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
/// Static stake table that doesn't use DRB results for leader election
pub struct StaticStakeTable<
    PubKey: SignatureKey,
    StatePubKey: StateSignatureKey + LCV1StateSignatureKey + LCV2StateSignatureKey + LCV3StateSignatureKey,
> {
    quorum_members: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,

    da_members: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,

    quorum_committees: TestCommitteeSchedule<PubKey, StatePubKey>,

    first_epoch: Option<u64>,

    epochs: BTreeSet<u64>,

    drb_results: BTreeMap<u64, DrbResult>,

    da_committees: TestCommitteeSchedule<PubKey, StatePubKey>,
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
            quorum_members,
            da_members,
            quorum_committees: TestCommitteeSchedule::new(),
            first_epoch: None,
            epochs: BTreeSet::new(),
            drb_results: BTreeMap::new(),
            da_committees: TestCommitteeSchedule::new(),
        }
    }

    fn stake_table(&self, epoch: Option<u64>) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.quorum_committees
            .get(epoch)
            .unwrap_or_else(|| self.quorum_members.clone())
    }

    fn da_stake_table(&self, epoch: Option<u64>) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.da_committees
            .get(epoch)
            .unwrap_or(self.da_members.clone())
    }

    fn full_stake_table(&self) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.quorum_members.clone()
    }

    fn lookup_leader(&self, view_number: u64, epoch: Option<u64>) -> anyhow::Result<PubKey> {
        let committee = self.stake_table(epoch);
        let index = view_number as usize % committee.len();
        let leader = committee[index].clone();
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

    fn add_quorum_committee(
        &mut self,
        first_epoch: u64,
        committee: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,
    ) {
        self.quorum_committees.add(first_epoch, committee);
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::U256;
    use hotshot_types::{
        ValidatorConfig,
        signature_key::{BLSPubKey, SchnorrPubKey},
    };

    use super::*;
    use crate::node_types::TestTypes;

    fn entries(indices: &[u64]) -> Vec<TestStakeTableEntry<BLSPubKey, SchnorrPubKey>> {
        indices
            .iter()
            .map(|i| {
                let config: ValidatorConfig<TestTypes> =
                    ValidatorConfig::generated_from_seed_indexed(
                        [0u8; 32],
                        *i,
                        U256::from(1),
                        false,
                    );
                config.public_config().into()
            })
            .collect()
    }

    /// Epochs before the earliest scheduled first-epoch (and `None`) resolve
    /// to the default committee; later epochs resolve to the greatest
    /// scheduled first-epoch <= the requested epoch.
    #[test]
    fn quorum_schedule_resolution() {
        let default = entries(&[0, 1, 2, 3, 4]);
        let smaller = entries(&[0, 1, 2, 3]);
        let larger = entries(&[0, 1, 2, 3, 4, 5]);

        let mut table = StaticStakeTable::new(default.clone(), default.clone());
        table.add_quorum_committee(3, smaller.clone());
        table.add_quorum_committee(6, larger.clone());

        assert_eq!(table.stake_table(None), default);
        assert_eq!(table.stake_table(Some(1)), default);
        assert_eq!(table.stake_table(Some(2)), default);
        assert_eq!(table.stake_table(Some(3)), smaller);
        assert_eq!(table.stake_table(Some(5)), smaller);
        assert_eq!(table.stake_table(Some(6)), larger);
        assert_eq!(table.stake_table(Some(100)), larger);
    }

    /// The leader rotation uses the scheduled committee for the epoch, so a
    /// committee of a different size changes which key leads a given view.
    #[test]
    fn quorum_schedule_leader_rotation() {
        let default = entries(&[0, 1, 2, 3, 4]);
        let scheduled = entries(&[0, 1, 2]);

        let mut table = StaticStakeTable::new(default.clone(), default.clone());
        table.add_quorum_committee(3, scheduled.clone());

        assert_eq!(
            table.lookup_leader(4, Some(2)).unwrap(),
            default[4].signature_key
        );
        assert_eq!(
            table.lookup_leader(4, Some(3)).unwrap(),
            scheduled[4 % 3].signature_key
        );
    }

    /// The quorum and DA schedules are stored and resolved independently.
    #[test]
    fn quorum_and_da_schedules_independent() {
        let default = entries(&[0, 1, 2, 3, 4]);
        let quorum = entries(&[0, 1, 2, 3]);
        let da = entries(&[0, 1]);

        let mut table = StaticStakeTable::new(default.clone(), default.clone());
        table.add_quorum_committee(3, quorum.clone());
        table.add_da_committee(4, da.clone());

        assert_eq!(table.stake_table(Some(3)), quorum);
        assert_eq!(table.da_stake_table(Some(3)), default);
        assert_eq!(table.stake_table(Some(4)), quorum);
        assert_eq!(table.da_stake_table(Some(4)), da);
    }
}
