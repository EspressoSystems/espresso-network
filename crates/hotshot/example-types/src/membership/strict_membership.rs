use std::{collections::HashSet, fmt, fmt::Debug, sync::Arc};

use alloy::primitives::U256;
use hotshot_types::{
    stake_table::HSStakeTable,
    traits::{
        election::Membership,
        node_implementation::{NodeImplementation, NodeType},
        signature_key::StakeTableEntryType,
    },
};

use crate::{
    membership::{fetcher::Leaf2Fetcher, stake_table::TestStakeTable},
    storage_types::TestStorage,
};

#[derive(Clone)]
pub struct StrictMembership<
    TYPES: NodeType,
    StakeTable: TestStakeTable<TYPES::SignatureKey, TYPES::StateSignatureKey>,
> {
    inner: StakeTable,
    epochs: HashSet<TYPES::Epoch>,
    drbs: HashSet<TYPES::Epoch>,
    fetcher: Arc<Leaf2Fetcher<TYPES>>,
}

impl<TYPES, StakeTable> Debug for StrictMembership<TYPES, StakeTable>
where
    TYPES: NodeType,
    StakeTable: TestStakeTable<TYPES::SignatureKey, TYPES::StateSignatureKey>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("StrictMembership")
            .field("inner", &self.inner)
            .field("epochs", &self.epochs)
            .field("drbs", &self.drbs)
            .finish()
    }
}

impl<
        TYPES: NodeType,
        StakeTable: TestStakeTable<TYPES::SignatureKey, TYPES::StateSignatureKey>,
    > StrictMembership<TYPES, StakeTable>
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
}

impl<
        TYPES: NodeType,
        StakeTable: TestStakeTable<TYPES::SignatureKey, TYPES::StateSignatureKey>,
    > Membership<TYPES> for StrictMembership<TYPES, StakeTable>
{
    type Error = anyhow::Error;
    type Storage = TestStorage<TYPES>;

    fn new<I: NodeImplementation<TYPES>>(
        quorum_members: Vec<hotshot_types::PeerConfig<TYPES>>,
        da_members: Vec<hotshot_types::PeerConfig<TYPES>>,
        storage: Self::Storage,
        network: Arc<<I as NodeImplementation<TYPES>>::Network>,
        public_key: TYPES::SignatureKey,
    ) -> Self {
        let fetcher = Leaf2Fetcher::new::<I>(network, storage, public_key);

        Self {
            inner: TestStakeTable::new(
                quorum_members.into_iter().map(Into::into).collect(),
                da_members.into_iter().map(Into::into).collect(),
            ),
            epochs: HashSet::new(),
            drbs: HashSet::new(),
            fetcher: fetcher.into(),
        }
    }

    fn stake_table(&self, epoch: Option<TYPES::Epoch>) -> HSStakeTable<TYPES> {
        self.assert_has_stake_table(epoch);
        let peer_configs = self
            .inner
            .stake_table(epoch.map(|e| *e))
            .into_iter()
            .map(Into::into)
            .collect();
        HSStakeTable(peer_configs)
    }

    fn da_stake_table(&self, epoch: Option<TYPES::Epoch>) -> HSStakeTable<TYPES> {
        self.assert_has_stake_table(epoch);
        let peer_configs = self
            .inner
            .da_stake_table(epoch.map(|e| *e))
            .into_iter()
            .map(Into::into)
            .collect();
        HSStakeTable(peer_configs)
    }

    fn committee_members(
        &self,
        _view_number: TYPES::View,
        epoch: Option<TYPES::Epoch>,
    ) -> std::collections::BTreeSet<TYPES::SignatureKey> {
        self.assert_has_stake_table(epoch);
        self.inner
            .stake_table(epoch.map(|e| *e))
            .into_iter()
            .map(|entry| entry.signature_key)
            .collect()
    }

    fn da_committee_members(
        &self,
        _view_number: TYPES::View,
        epoch: Option<TYPES::Epoch>,
    ) -> std::collections::BTreeSet<TYPES::SignatureKey> {
        self.assert_has_stake_table(epoch);
        self.inner
            .da_stake_table(epoch.map(|e| *e))
            .into_iter()
            .map(|entry| entry.signature_key)
            .collect()
    }

    fn stake(
        &self,
        pub_key: &TYPES::SignatureKey,
        epoch: Option<TYPES::Epoch>,
    ) -> Option<hotshot_types::PeerConfig<TYPES>> {
        self.assert_has_stake_table(epoch);
        self.inner
            .stake(pub_key.clone(), epoch.map(|e| *e))
            .map(Into::into)
    }

    fn da_stake(
        &self,
        pub_key: &TYPES::SignatureKey,
        epoch: Option<TYPES::Epoch>,
    ) -> Option<hotshot_types::PeerConfig<TYPES>> {
        self.assert_has_stake_table(epoch);
        self.inner
            .da_stake(pub_key.clone(), epoch.map(|e| *e))
            .map(Into::into)
    }

    /// Check if a node has stake in the committee
    fn has_stake(
        &self,
        pub_key: &<TYPES as NodeType>::SignatureKey,
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> bool {
        self.stake(pub_key, epoch)
            .is_some_and(|x| x.stake_table_entry.stake() > U256::ZERO)
    }

    /// Check if a node has stake in the da committee
    fn has_da_stake(
        &self,
        pub_key: &<TYPES as NodeType>::SignatureKey,
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> bool {
        self.da_stake(pub_key, epoch)
            .is_some_and(|x| x.stake_table_entry.stake() > U256::ZERO)
    }

    fn lookup_leader(
        &self,
        view: TYPES::View,
        epoch: Option<TYPES::Epoch>,
    ) -> anyhow::Result<TYPES::SignatureKey> {
        self.assert_has_randomized_stake_table(epoch);
        self.inner.lookup_leader(*view, epoch.map(|e| *e))
    }

    fn total_nodes(&self, epoch: Option<TYPES::Epoch>) -> usize {
        self.assert_has_stake_table(epoch);
        self.inner.stake_table(epoch.map(|e| *e)).len()
    }

    fn da_total_nodes(&self, epoch: Option<TYPES::Epoch>) -> usize {
        self.assert_has_stake_table(epoch);
        self.inner.da_stake_table(epoch.map(|e| *e)).len()
    }

    fn has_stake_table(&self, epoch: TYPES::Epoch) -> bool {
        let has_stake_table = self.inner.has_stake_table(*epoch);

        assert_eq!(has_stake_table, self.epochs.contains(&epoch));

        has_stake_table
    }

    fn has_randomized_stake_table(&self, epoch: TYPES::Epoch) -> anyhow::Result<bool> {
        let has_randomized_stake_table = self.inner.has_randomized_stake_table(*epoch);

        if let Ok(result) = has_randomized_stake_table {
            assert_eq!(result, self.drbs.contains(&epoch));
        } else {
            assert!(!self.drbs.contains(&epoch));
        }

        has_randomized_stake_table
    }

    fn add_drb_result(&mut self, epoch: TYPES::Epoch, drb_result: hotshot_types::drb::DrbResult) {
        self.drbs.insert(epoch);
        self.inner.add_drb_result(*epoch, drb_result);
    }
}
