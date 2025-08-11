// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.
use std::{
    collections::{BTreeMap, BTreeSet},
    marker::PhantomData,
};

use hotshot_types::{
    drb::DrbResult,
    traits::signature_key::{
        LCV1StateSignatureKey, LCV2StateSignatureKey, LCV3StateSignatureKey, SignatureKey,
        StateSignatureKey,
    },
};
use rand::{rngs::StdRng, Rng};
use tracing::error;

use crate::membership::{
    helpers::QuorumFilterConfig,
    stake_table::{TestStakeTable, TestStakeTableEntry},
};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RandomizedCommitteeMembers<
    PubKey: SignatureKey,
    StatePubKey: StateSignatureKey + LCV1StateSignatureKey + LCV2StateSignatureKey + LCV3StateSignatureKey,
    QuorumConfig: QuorumFilterConfig,
    DaConfig: QuorumFilterConfig,
> {
    quorum_members: BTreeMap<PubKey, TestStakeTableEntry<PubKey, StatePubKey>>,

    da_members: BTreeMap<PubKey, TestStakeTableEntry<PubKey, StatePubKey>>,

    first_epoch: Option<u64>,

    _quorum_pd: PhantomData<QuorumConfig>,

    _da_pd: PhantomData<DaConfig>,
}

impl<
        PubKey: SignatureKey,
        StatePubKey: StateSignatureKey + LCV1StateSignatureKey + LCV2StateSignatureKey + LCV3StateSignatureKey,
        QuorumConfig: QuorumFilterConfig,
        DaConfig: QuorumFilterConfig,
    > RandomizedCommitteeMembers<PubKey, StatePubKey, QuorumConfig, DaConfig>
{
    /// Creates a set of indices into the stake_table which reference the nodes selected for this epoch's committee
    fn make_quorum_filter(&self, epoch: u64) -> BTreeSet<usize> {
        QuorumConfig::execute(epoch, self.quorum_members.len())
    }

    /// Creates a set of indices into the da_stake_table which reference the nodes selected for this epoch's da committee
    fn make_da_quorum_filter(&self, epoch: u64) -> BTreeSet<usize> {
        DaConfig::execute(epoch, self.da_members.len())
    }

    /// Writes the offsets used for the quorum filter and da_quorum filter to stdout
    fn debug_display_offsets(&self) {
        /// Ensures that the quorum filters are only displayed once
        static START: std::sync::Once = std::sync::Once::new();

        START.call_once(|| {
            error!(
                "{} offsets for Quorum filter:",
                std::any::type_name::<QuorumConfig>()
            );
            for epoch in 1..=10 {
                error!("  epoch {epoch}: {:?}", self.make_quorum_filter(epoch));
            }

            error!(
                "{} offsets for DA Quorum filter:",
                std::any::type_name::<DaConfig>()
            );
            for epoch in 1..=10 {
                error!("  epoch {epoch}: {:?}", self.make_da_quorum_filter(epoch));
            }
        });
    }
}

impl<
        PubKey: SignatureKey,
        StatePubKey: StateSignatureKey + LCV1StateSignatureKey + LCV2StateSignatureKey + LCV3StateSignatureKey,
        QuorumConfig: QuorumFilterConfig,
        DaConfig: QuorumFilterConfig,
    > TestStakeTable<PubKey, StatePubKey>
    for RandomizedCommitteeMembers<PubKey, StatePubKey, QuorumConfig, DaConfig>
{
    fn new(
        quorum_members: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,
        da_members: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,
    ) -> Self {
        let result = Self {
            quorum_members: quorum_members
                .iter()
                .map(|entry| (entry.signature_key.clone(), entry.clone()))
                .collect(),
            da_members: da_members
                .iter()
                .map(|entry| (entry.signature_key.clone(), entry.clone()))
                .collect(),
            first_epoch: None,
            _quorum_pd: PhantomData,
            _da_pd: PhantomData,
        };

        result.debug_display_offsets();

        result
    }

    fn stake_table(&self, epoch: Option<u64>) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>> {
        if let Some(epoch) = epoch {
            let filter = self.make_quorum_filter(epoch);
            self.quorum_members
                .values()
                .cloned()
                .enumerate()
                .filter(|(idx, _)| filter.contains(idx))
                .map(|(_, v)| v.clone())
                .collect()
        } else {
            self.quorum_members.values().cloned().collect()
        }
    }

    fn da_stake_table(&self, epoch: Option<u64>) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>> {
        if let Some(epoch) = epoch {
            let filter = self.make_da_quorum_filter(epoch);
            self.da_members
                .values()
                .cloned()
                .enumerate()
                .filter(|(idx, _)| filter.contains(idx))
                .map(|(_, v)| v.clone())
                .collect()
        } else {
            self.da_members.values().cloned().collect()
        }
    }

    fn stake(
        &self,
        pub_key: PubKey,
        epoch: Option<u64>,
    ) -> Option<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.stake_table(epoch)
            .iter()
            .find(|entry| entry.signature_key == pub_key)
            .cloned()
    }

    fn da_stake(
        &self,
        pub_key: PubKey,
        epoch: Option<u64>,
    ) -> Option<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.da_stake_table(epoch)
            .iter()
            .find(|entry| entry.signature_key == pub_key)
            .cloned()
    }

    fn lookup_leader(&self, view_number: u64, epoch: Option<u64>) -> anyhow::Result<PubKey> {
        let stake_table = self.stake_table(epoch);
        let mut rng: StdRng = rand::SeedableRng::seed_from_u64(view_number);

        let randomized_view_number: u64 = rng.gen_range(0..=u64::MAX);
        let index = randomized_view_number as usize % stake_table.len();
        let leader = stake_table[index].clone();

        tracing::debug!(
            "RandomizedCommitteeMembers lookup_leader, view_number: {view_number}, epoch: \
             {epoch:?}, leader: {leader:?}",
        );

        Ok(leader.signature_key)
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
