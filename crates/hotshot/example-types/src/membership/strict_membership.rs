use std::{collections::HashSet, fmt, fmt::Debug, sync::Arc};

use alloy::primitives::U256;
use async_broadcast::Receiver;
use async_lock::RwLock;
use hotshot_types::{
    data::Leaf2,
    drb::DrbResult,
    event::Event,
    stake_table::HSStakeTable,
    traits::{
        election::{Membership, NoStakeTableHash},
        node_implementation::{ConsensusTime, NodeImplementation, NodeType},
        signature_key::StakeTableEntryType,
    },
    utils::transition_block_for_epoch,
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
    fetcher: Arc<RwLock<Leaf2Fetcher<TYPES>>>,
    epoch_height: u64,
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
    type StakeTableHash = NoStakeTableHash;
    type Storage = TestStorage<TYPES>;

    fn new<I: NodeImplementation<TYPES>>(
        quorum_members: Vec<hotshot_types::PeerConfig<TYPES>>,
        da_members: Vec<hotshot_types::PeerConfig<TYPES>>,
        storage: Self::Storage,
        network: Arc<<I as NodeImplementation<TYPES>>::Network>,
        public_key: TYPES::SignatureKey,
        epoch_height: u64,
    ) -> Self {
        let fetcher = Leaf2Fetcher::new::<I>(network, storage, public_key);

        Self {
            inner: TestStakeTable::new(
                quorum_members.into_iter().map(Into::into).collect(),
                da_members.into_iter().map(Into::into).collect(),
            ),
            epochs: HashSet::new(),
            drbs: HashSet::new(),
            fetcher: RwLock::new(fetcher).into(),
            epoch_height,
        }
    }

    async fn set_external_channel(&mut self, external_channel: Receiver<Event<TYPES>>) {
        self.fetcher
            .write()
            .await
            .set_external_channel(external_channel)
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
        self.assert_has_stake_table(epoch);

        self.stake(pub_key, epoch)
            .is_some_and(|x| x.stake_table_entry.stake() > U256::ZERO)
    }

    /// Check if a node has stake in the da committee
    fn has_da_stake(
        &self,
        pub_key: &<TYPES as NodeType>::SignatureKey,
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> bool {
        self.assert_has_stake_table(epoch);

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
        if !self.has_stake_table(epoch) {
            return Ok(false);
        }
        let has_randomized_stake_table = self.inner.has_randomized_stake_table(*epoch);

        if let Ok(result) = has_randomized_stake_table {
            assert_eq!(result, self.drbs.contains(&epoch));
        } else {
            assert!(!self.drbs.contains(&epoch));
        }

        has_randomized_stake_table
    }

    fn add_drb_result(&mut self, epoch: TYPES::Epoch, drb_result: hotshot_types::drb::DrbResult) {
        self.assert_has_stake_table(Some(epoch));

        self.drbs.insert(epoch);
        self.inner.add_drb_result(*epoch, drb_result);
    }

    fn first_epoch(&self) -> Option<TYPES::Epoch> {
        self.inner.first_epoch().map(TYPES::Epoch::new)
    }

    fn set_first_epoch(&mut self, epoch: TYPES::Epoch, initial_drb_result: DrbResult) {
        self.epochs.insert(epoch);
        self.epochs.insert(epoch + 1);

        self.drbs.insert(epoch);
        self.drbs.insert(epoch + 1);

        self.inner.set_first_epoch(*epoch, initial_drb_result);
    }

    async fn add_epoch_root(
        membership: Arc<RwLock<Self>>,
        epoch: TYPES::Epoch,
        _block_header: TYPES::BlockHeader,
    ) -> anyhow::Result<()> {
        let mut membership_writer = membership.write().await;

        membership_writer.epochs.insert(epoch);

        membership_writer.inner.add_epoch_root(*epoch);

        Ok(())
    }

    async fn get_epoch_root(
        membership: Arc<RwLock<Self>>,
        block_height: u64,
        _epoch: TYPES::Epoch,
    ) -> anyhow::Result<Leaf2<TYPES>> {
        let membership_reader = membership.read().await;

        for node in membership_reader.inner.full_stake_table() {
            if let Ok(leaf) = membership_reader
                .fetcher
                .read()
                .await
                .fetch_leaf(block_height, node.signature_key)
                .await
            {
                return Ok(leaf);
            }
        }

        anyhow::bail!("Failed to fetch epoch root from any peer");
    }

    async fn get_epoch_drb(
        membership: Arc<RwLock<Self>>,
        epoch: TYPES::Epoch,
    ) -> anyhow::Result<DrbResult> {
        let membership_reader = membership.read().await;
        if let Ok(drb_result) = membership_reader.inner.get_epoch_drb(*epoch) {
            Ok(drb_result)
        } else {
            let previous_epoch = match epoch.checked_sub(1) {
                Some(epoch) => epoch,
                None => {
                    anyhow::bail!("Missing initial DRB result for epoch {epoch:?}");
                },
            };

            let drb_block_height =
                transition_block_for_epoch(previous_epoch, membership_reader.epoch_height);

            let mut drb_leaf = None;

            for node in membership_reader.inner.full_stake_table() {
                if let Ok(leaf) = membership_reader
                    .fetcher
                    .read()
                    .await
                    .fetch_leaf(drb_block_height, node.signature_key)
                    .await
                {
                    drb_leaf = Some(leaf);
                    break;
                }
            }

            match drb_leaf {
                Some(leaf) => Ok(leaf.next_drb_result.expect(
                    "We fetched a leaf that is missing a DRB result. This should be impossible.",
                )),
                None => {
                    anyhow::bail!("Failed to fetch leaf from all nodes");
                },
            }
        }
    }
}
