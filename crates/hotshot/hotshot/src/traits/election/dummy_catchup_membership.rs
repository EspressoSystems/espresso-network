use std::{collections::HashSet, sync::Arc, time::Duration};

use alloy::primitives::U256;
use anyhow::Ok;
use async_lock::RwLock;
use hotshot_types::{
    data::Leaf2,
    drb::DrbResult,
    stake_table::HSStakeTable,
    traits::{election::Membership, node_implementation::NodeType},
};

use super::static_committee::StaticCommittee;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DummyCatchupCommittee<TYPES: NodeType> {
    inner: StaticCommittee<TYPES>,
    epochs: HashSet<TYPES::Epoch>,
    drbs: HashSet<TYPES::Epoch>,
}

impl<TYPES: NodeType> DummyCatchupCommittee<TYPES> {
    fn assert_has_stake_table(&self, epoch: Option<TYPES::Epoch>) {
        let Some(epoch) = epoch else {
            return;
        };
        assert!(
            self.epochs.contains(&epoch),
            "Failed epoch check for epoch {epoch}"
        );
    }
    fn assert_has_randomized_stake_table(&self, epoch: Option<TYPES::Epoch>) {
        let Some(epoch) = epoch else {
            return;
        };
        assert!(
            self.drbs.contains(&epoch),
            "Failed epoch check for epoch {epoch}"
        );
    }
}

impl<TYPES: NodeType> Membership<TYPES> for DummyCatchupCommittee<TYPES>
where
    TYPES::BlockHeader: Default,
{
    type Error = hotshot_utils::anytrace::Error;

    fn new(
        // Note: eligible_leaders is currently a haMemck because the DA leader == the quorum leader
        // but they should not have voting power.
        stake_committee_members: Vec<hotshot_types::PeerConfig<TYPES>>,
        da_committee_members: Vec<hotshot_types::PeerConfig<TYPES>>,
    ) -> Self {
        Self {
            inner: StaticCommittee::new(stake_committee_members, da_committee_members),
            epochs: HashSet::new(),
            drbs: HashSet::new(),
        }
    }

    fn stake_table(&self, epoch: Option<TYPES::Epoch>) -> HSStakeTable<TYPES> {
        self.assert_has_stake_table(epoch);
        self.inner.stake_table(epoch)
    }

    fn da_stake_table(&self, epoch: Option<TYPES::Epoch>) -> HSStakeTable<TYPES> {
        self.assert_has_stake_table(epoch);
        self.inner.da_stake_table(epoch)
    }

    fn committee_members(
        &self,
        view_number: TYPES::View,
        epoch: Option<TYPES::Epoch>,
    ) -> std::collections::BTreeSet<TYPES::SignatureKey> {
        self.assert_has_stake_table(epoch);
        self.inner.committee_members(view_number, epoch)
    }

    fn da_committee_members(
        &self,
        view_number: TYPES::View,
        epoch: Option<TYPES::Epoch>,
    ) -> std::collections::BTreeSet<TYPES::SignatureKey> {
        self.assert_has_stake_table(epoch);
        self.inner.da_committee_members(view_number, epoch)
    }

    fn stake(
        &self,
        pub_key: &TYPES::SignatureKey,
        epoch: Option<TYPES::Epoch>,
    ) -> Option<hotshot_types::PeerConfig<TYPES>> {
        self.assert_has_stake_table(epoch);
        self.inner.stake(pub_key, epoch)
    }

    fn da_stake(
        &self,
        pub_key: &TYPES::SignatureKey,
        epoch: Option<TYPES::Epoch>,
    ) -> Option<hotshot_types::PeerConfig<TYPES>> {
        self.assert_has_stake_table(epoch);
        self.inner.da_stake(pub_key, epoch)
    }

    fn has_stake(&self, pub_key: &TYPES::SignatureKey, epoch: Option<TYPES::Epoch>) -> bool {
        self.assert_has_stake_table(epoch);
        self.inner.has_stake(pub_key, epoch)
    }

    fn has_da_stake(&self, pub_key: &TYPES::SignatureKey, epoch: Option<TYPES::Epoch>) -> bool {
        self.assert_has_stake_table(epoch);
        self.inner.has_da_stake(pub_key, epoch)
    }

    fn lookup_leader(
        &self,
        view: TYPES::View,
        epoch: Option<TYPES::Epoch>,
    ) -> std::result::Result<TYPES::SignatureKey, Self::Error> {
        self.assert_has_randomized_stake_table(epoch);
        self.inner.lookup_leader(view, epoch)
    }

    fn total_nodes(&self, epoch: Option<TYPES::Epoch>) -> usize {
        self.assert_has_stake_table(epoch);
        self.inner.total_nodes(epoch)
    }

    fn da_total_nodes(&self, epoch: Option<TYPES::Epoch>) -> usize {
        self.assert_has_stake_table(epoch);
        self.inner.da_total_nodes(epoch)
    }

    fn success_threshold(&self, epoch: Option<TYPES::Epoch>) -> U256 {
        self.assert_has_stake_table(epoch);
        self.inner.success_threshold(epoch)
    }

    fn da_success_threshold(&self, epoch: Option<TYPES::Epoch>) -> U256 {
        self.assert_has_stake_table(epoch);
        self.inner.da_success_threshold(epoch)
    }

    fn failure_threshold(&self, epoch: Option<TYPES::Epoch>) -> U256 {
        self.assert_has_stake_table(epoch);
        self.inner.failure_threshold(epoch)
    }

    fn upgrade_threshold(&self, epoch: Option<TYPES::Epoch>) -> U256 {
        self.assert_has_stake_table(epoch);
        self.inner.upgrade_threshold(epoch)
    }

    fn has_stake_table(&self, epoch: TYPES::Epoch) -> bool {
        self.assert_has_stake_table(Some(epoch));
        self.epochs.contains(&epoch)
    }

    fn has_randomized_stake_table(&self, epoch: TYPES::Epoch) -> bool {
        self.assert_has_randomized_stake_table(Some(epoch));
        self.drbs.contains(&epoch)
    }

    async fn get_epoch_root(
        _membership: Arc<RwLock<Self>>,
        _block_height: u64,
        _epoch: TYPES::Epoch,
    ) -> anyhow::Result<Leaf2<TYPES>> {
        tokio::time::sleep(Duration::from_secs(1)).await;
        anyhow::bail!("Not implemented");
    }

    async fn get_epoch_drb(
        _membership: Arc<RwLock<Self>>,
        _block_height: u64,
        _epoch: TYPES::Epoch,
    ) -> anyhow::Result<DrbResult> {
        tokio::time::sleep(Duration::from_secs(1)).await;
        Ok(DrbResult::default())
    }

    fn add_drb_result(&mut self, epoch: TYPES::Epoch, drb_result: hotshot_types::drb::DrbResult) {
        self.drbs.insert(epoch);
        self.inner.add_drb_result(epoch, drb_result);
    }

    fn set_first_epoch(
        &mut self,
        epoch: TYPES::Epoch,
        initial_drb_result: hotshot_types::drb::DrbResult,
    ) {
        self.epochs.insert(epoch);
        self.epochs.insert(epoch + 1);
        self.drbs.insert(epoch);
        self.drbs.insert(epoch + 1);
        self.inner.set_first_epoch(epoch, initial_drb_result);
    }

    #[allow(refining_impl_trait)]
    async fn add_epoch_root(
        &self,
        epoch: TYPES::Epoch,
        _block_header: TYPES::BlockHeader,
    ) -> Option<Box<dyn FnOnce(&mut Self) + Send>> {
        Some(Box::new(move |mem: &mut Self| {
            tracing::error!("Adding epoch root for {epoch}");
            mem.epochs.insert(epoch);
        }))
    }
}
