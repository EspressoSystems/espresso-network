use std::{collections::HashSet, marker::PhantomData, sync::Arc, time::Duration};

use alloy::primitives::U256;
use anyhow::Ok;
use async_lock::RwLock;
use hotshot_types::{
    data::Leaf2,
    drb::DrbResult,
    stake_table::HSStakeTable,
    traits::{
        election::Membership,
        node_implementation::{NodeType, Versions},
        signature_key::SignatureKey,
    },
    PeerConfig,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DummyCatchupCommittee<TYPES: NodeType, V: Versions, InnerTypes: NodeType> {
    inner: InnerTypes::Membership,
    epochs: HashSet<TYPES::Epoch>,
    drbs: HashSet<TYPES::Epoch>,
    _phantom: PhantomData<V>,
}

impl<TYPES: NodeType, V: Versions, InnerTypes: NodeType>
    DummyCatchupCommittee<TYPES, V, InnerTypes>
{
    fn assert_has_stake_table(&self, epoch: Option<TYPES::Epoch>) {
        let Some(epoch) = epoch else {
            return;
        };
        assert!(
            self.epochs.contains(&epoch),
            "Failed stake table check for epoch {epoch}"
        );
    }
    fn assert_has_randomized_stake_table(&self, epoch: Option<TYPES::Epoch>) {
        let Some(epoch) = epoch else {
            return;
        };
        assert!(
            self.drbs.contains(&epoch),
            "Failed drb check for epoch {epoch}"
        );
    }

    fn convert_peer_config<FromTypes, IntoTypes>(
        peer_config: PeerConfig<FromTypes>,
    ) -> PeerConfig<IntoTypes>
    where
        FromTypes: NodeType,
        IntoTypes: NodeType,
        <IntoTypes::SignatureKey as SignatureKey>::StakeTableEntry:
            From<<FromTypes::SignatureKey as SignatureKey>::StakeTableEntry>,
        IntoTypes::StateSignatureKey: From<FromTypes::StateSignatureKey>,
    {
        PeerConfig {
            stake_table_entry: peer_config.stake_table_entry.into(),
            state_ver_key: Into::<IntoTypes::StateSignatureKey>::into(peer_config.state_ver_key),
        }
    }
}

impl<TYPES: NodeType, V: Versions, InnerTypes: NodeType> Membership<TYPES>
    for DummyCatchupCommittee<TYPES, V, InnerTypes>
where
    TYPES::BlockHeader: Default,
    TYPES::InstanceState: Default,
    InnerTypes::Epoch: From<TYPES::Epoch>,
    TYPES::Epoch: From<InnerTypes::Epoch>,
    InnerTypes::View: From<TYPES::View>,
    TYPES::SignatureKey: From<InnerTypes::SignatureKey>,
    for<'a> &'a InnerTypes::SignatureKey: From<&'a TYPES::SignatureKey>,
    <InnerTypes::SignatureKey as SignatureKey>::StakeTableEntry:
        From<<TYPES::SignatureKey as SignatureKey>::StakeTableEntry>,
    InnerTypes::StateSignatureKey: From<TYPES::StateSignatureKey>,
    <TYPES::SignatureKey as SignatureKey>::StakeTableEntry:
        From<<InnerTypes::SignatureKey as SignatureKey>::StakeTableEntry>,
    TYPES::StateSignatureKey: From<InnerTypes::StateSignatureKey>,
{
    type Error = <InnerTypes::Membership as Membership<InnerTypes>>::Error;

    fn new(
        // Note: eligible_leaders is currently a haMemck because the DA leader == the quorum leader
        // but they should not have voting power.
        stake_committee_members: Vec<hotshot_types::PeerConfig<TYPES>>,
        da_committee_members: Vec<hotshot_types::PeerConfig<TYPES>>,
    ) -> Self {
        Self {
            inner: Membership::new(
                stake_committee_members
                    .into_iter()
                    .map(Self::convert_peer_config)
                    .collect(),
                da_committee_members
                    .into_iter()
                    .map(Self::convert_peer_config)
                    .collect(),
            ),
            epochs: HashSet::new(),
            drbs: HashSet::new(),
            _phantom: PhantomData,
        }
    }

    fn stake_table(&self, epoch: Option<TYPES::Epoch>) -> HSStakeTable<TYPES> {
        self.assert_has_stake_table(epoch);
        let peer_configs = self.inner.stake_table(epoch.map(Into::into)).0;
        HSStakeTable(
            peer_configs
                .into_iter()
                .map(Self::convert_peer_config)
                .collect(),
        )
    }

    fn da_stake_table(&self, epoch: Option<TYPES::Epoch>) -> HSStakeTable<TYPES> {
        self.assert_has_stake_table(epoch);
        let peer_configs = self.inner.da_stake_table(epoch.map(Into::into)).0;
        HSStakeTable(
            peer_configs
                .into_iter()
                .map(Self::convert_peer_config)
                .collect(),
        )
    }

    fn committee_members(
        &self,
        view_number: TYPES::View,
        epoch: Option<TYPES::Epoch>,
    ) -> std::collections::BTreeSet<TYPES::SignatureKey> {
        self.assert_has_stake_table(epoch);
        self.inner
            .committee_members(view_number.into(), epoch.map(Into::into))
            .into_iter()
            .map(Into::<TYPES::SignatureKey>::into)
            .collect()
    }

    fn da_committee_members(
        &self,
        view_number: TYPES::View,
        epoch: Option<TYPES::Epoch>,
    ) -> std::collections::BTreeSet<TYPES::SignatureKey> {
        self.assert_has_stake_table(epoch);
        self.inner
            .da_committee_members(view_number.into(), epoch.map(Into::into))
            .into_iter()
            .map(Into::<TYPES::SignatureKey>::into)
            .collect()
    }

    fn stake(
        &self,
        pub_key: &TYPES::SignatureKey,
        epoch: Option<TYPES::Epoch>,
    ) -> Option<hotshot_types::PeerConfig<TYPES>> {
        self.assert_has_stake_table(epoch);
        self.inner
            .stake(pub_key.into(), epoch.map(Into::into))
            .map(Self::convert_peer_config)
    }

    fn da_stake(
        &self,
        pub_key: &TYPES::SignatureKey,
        epoch: Option<TYPES::Epoch>,
    ) -> Option<hotshot_types::PeerConfig<TYPES>> {
        self.assert_has_stake_table(epoch);
        self.inner
            .da_stake(pub_key.into(), epoch.map(Into::into))
            .map(Self::convert_peer_config)
    }

    fn has_stake(&self, pub_key: &TYPES::SignatureKey, epoch: Option<TYPES::Epoch>) -> bool {
        self.assert_has_stake_table(epoch);
        self.inner.has_stake(pub_key.into(), epoch.map(Into::into))
    }

    fn has_da_stake(&self, pub_key: &TYPES::SignatureKey, epoch: Option<TYPES::Epoch>) -> bool {
        self.assert_has_stake_table(epoch);
        self.inner
            .has_da_stake(pub_key.into(), epoch.map(Into::into))
    }

    fn lookup_leader(
        &self,
        view: TYPES::View,
        epoch: Option<TYPES::Epoch>,
    ) -> std::result::Result<TYPES::SignatureKey, Self::Error> {
        self.assert_has_randomized_stake_table(epoch);
        self.inner
            .lookup_leader(view.into(), epoch.map(Into::into))
            .map(Into::<TYPES::SignatureKey>::into)
    }

    fn total_nodes(&self, epoch: Option<TYPES::Epoch>) -> usize {
        self.assert_has_stake_table(epoch);
        self.inner.total_nodes(epoch.map(Into::into))
    }

    fn da_total_nodes(&self, epoch: Option<TYPES::Epoch>) -> usize {
        self.assert_has_stake_table(epoch);
        self.inner.da_total_nodes(epoch.map(Into::into))
    }

    fn success_threshold(&self, epoch: Option<TYPES::Epoch>) -> U256 {
        self.assert_has_stake_table(epoch);
        self.inner.success_threshold(epoch.map(Into::into))
    }

    fn da_success_threshold(&self, epoch: Option<TYPES::Epoch>) -> U256 {
        self.assert_has_stake_table(epoch);
        self.inner.da_success_threshold(epoch.map(Into::into))
    }

    fn failure_threshold(&self, epoch: Option<TYPES::Epoch>) -> U256 {
        self.assert_has_stake_table(epoch);
        self.inner.failure_threshold(epoch.map(Into::into))
    }

    fn upgrade_threshold(&self, epoch: Option<TYPES::Epoch>) -> U256 {
        self.assert_has_stake_table(epoch);
        self.inner.upgrade_threshold(epoch.map(Into::into))
    }

    fn has_stake_table(&self, epoch: TYPES::Epoch) -> bool {
        self.epochs.contains(&epoch)
    }

    fn has_randomized_stake_table(&self, epoch: TYPES::Epoch) -> anyhow::Result<bool> {
        Ok(self.drbs.contains(&epoch))
    }

    async fn get_epoch_root(
        _membership: Arc<RwLock<Self>>,
        _block_height: u64,
        _epoch: TYPES::Epoch,
    ) -> anyhow::Result<Leaf2<TYPES>> {
        tokio::time::sleep(Duration::from_millis(10)).await;
        let leaf = Leaf2::genesis::<V>(
            &TYPES::ValidatedState::default(),
            &TYPES::InstanceState::default(),
        )
        .await;
        Ok(leaf)
    }

    async fn get_epoch_drb(
        _membership: Arc<RwLock<Self>>,
        _epoch: TYPES::Epoch,
    ) -> anyhow::Result<DrbResult> {
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(DrbResult::default())
    }

    fn add_drb_result(&mut self, epoch: TYPES::Epoch, drb_result: hotshot_types::drb::DrbResult) {
        self.drbs.insert(epoch);
        self.inner.add_drb_result(epoch.into(), drb_result);
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
        self.inner.set_first_epoch(epoch.into(), initial_drb_result);
    }

    async fn add_epoch_root(
        membership: Arc<RwLock<Self>>,
        epoch: TYPES::Epoch,
        _block_header: TYPES::BlockHeader,
    ) -> anyhow::Result<()> {
        let mut membership_writer = membership.write().await;
        tracing::error!("Adding epoch root for {epoch}");
        membership_writer.epochs.insert(epoch);
        Ok(())
    }

    fn first_epoch(&self) -> Option<TYPES::Epoch> {
        self.inner.first_epoch().map(Into::into)
    }
}
