use std::{
    collections::{BTreeSet, HashSet},
    fmt::{self, Debug},
    sync::Arc,
};

use alloy::primitives::U256;
use anyhow::anyhow;
use async_broadcast::Receiver;
use async_lock::RwLock as AsyncRwLock;
use hotshot_types::{
    PeerConfig,
    data::{BlockNumber, EpochNumber, Leaf2, ViewNumber},
    drb::DrbResult,
    event::Event,
    stake_table::HSStakeTable,
    traits::{
        block_contents::BlockHeader,
        election::{Membership, NoStakeTableHash},
        leaf_fetcher_network::LeafFetcherNetwork,
        node_implementation::NodeType,
        signature_key::StakeTableEntryType,
    },
    utils::{epoch_from_block_number, root_block_in_epoch, transition_block_for_epoch},
};
use parking_lot::RwLock;

use crate::{
    membership::{TestableMembership, fetcher::Leaf2Fetcher, stake_table::TestStakeTable},
    storage_types::TestStorage,
};

#[derive(Clone)]
pub struct StrictMembership<T, S>
where
    T: NodeType,
    S: TestStakeTable<T::SignatureKey, T::StateSignatureKey>,
{
    inner: Arc<RwLock<Inner<T, S>>>,
    epoch_height: BlockNumber,
}

struct Inner<T: NodeType, S> {
    table: S,
    epochs: HashSet<EpochNumber>,
    drbs: HashSet<EpochNumber>,
    fetcher: Option<Arc<AsyncRwLock<Leaf2Fetcher<T>>>>,
}

impl<T, S> Debug for StrictMembership<T, S>
where
    T: NodeType,
    S: TestStakeTable<T::SignatureKey, T::StateSignatureKey>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let inner = self.inner.read();
        f.debug_struct("StrictMembership")
            .field("table", &inner.table)
            .field("epochs", &inner.epochs)
            .field("drbs", &inner.drbs)
            .finish()
    }
}

impl<T, S> TestableMembership<T> for StrictMembership<T, S>
where
    T: NodeType,
    S: TestStakeTable<T::SignatureKey, T::StateSignatureKey>,
{
    fn new(
        quorum_members: Vec<PeerConfig<T>>,
        da_members: Vec<PeerConfig<T>>,
        _public_key: T::SignatureKey,
        epoch_height: u64,
    ) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                table: TestStakeTable::new(
                    quorum_members.into_iter().map(Into::into).collect(),
                    da_members.into_iter().map(Into::into).collect(),
                ),
                epochs: HashSet::new(),
                drbs: HashSet::new(),
                fetcher: None,
            })),
            epoch_height: epoch_height.into(),
        }
    }

    fn set_leaf_fetcher(
        &self,
        network: Arc<dyn LeafFetcherNetwork<T>>,
        storage: TestStorage<T>,
        public_key: T::SignatureKey,
        channel: Receiver<Event<T>>,
    ) {
        let mut fetcher = Leaf2Fetcher::new(network, storage, public_key);
        fetcher.set_external_channel(channel);
        self.inner.write().fetcher = Some(Arc::new(AsyncRwLock::new(fetcher)));
    }
}

impl<T: NodeType, S> Inner<T, S> {
    fn assert_has_stake_table(&self, epoch: Option<EpochNumber>) {
        let Some(epoch) = epoch else {
            return;
        };
        assert!(
            self.epochs.contains(&epoch),
            "Failed stake table check for epoch {epoch}"
        );
    }

    fn assert_has_randomized_stake_table(&self, epoch: Option<EpochNumber>) {
        let Some(epoch) = epoch else {
            return;
        };
        assert!(
            self.drbs.contains(&epoch),
            "Failed drb check for epoch {epoch}"
        );
    }
}

impl<T, S> Membership<T> for StrictMembership<T, S>
where
    T: NodeType,
    S: TestStakeTable<T::SignatureKey, T::StateSignatureKey>,
{
    type StakeTableHash = NoStakeTableHash;
    type Error = StrictMembershipError;

    fn stake_table(&self, epoch: Option<EpochNumber>) -> HSStakeTable<T> {
        let inner = self.inner.read();
        inner.assert_has_stake_table(epoch);
        let peer_configs = inner
            .table
            .stake_table(epoch.map(|e| *e))
            .into_iter()
            .map(Into::into)
            .collect();
        HSStakeTable(peer_configs)
    }

    fn da_stake_table(&self, epoch: Option<EpochNumber>) -> HSStakeTable<T> {
        let inner = self.inner.read();
        inner.assert_has_stake_table(epoch);
        let peer_configs = inner
            .table
            .da_stake_table(epoch.map(|e| *e))
            .into_iter()
            .map(Into::into)
            .collect();
        HSStakeTable(peer_configs)
    }

    fn committee_members(
        &self,
        _: ViewNumber,
        e: Option<EpochNumber>,
    ) -> BTreeSet<T::SignatureKey> {
        let inner = self.inner.read();
        inner.assert_has_stake_table(e);
        inner
            .table
            .stake_table(e.map(|e| *e))
            .into_iter()
            .map(|entry| entry.signature_key)
            .collect()
    }

    fn da_committee_members(
        &self,
        _: ViewNumber,
        e: Option<EpochNumber>,
    ) -> BTreeSet<T::SignatureKey> {
        let inner = self.inner.read();
        inner.assert_has_stake_table(e);
        inner
            .table
            .da_stake_table(e.map(|e| *e))
            .into_iter()
            .map(|entry| entry.signature_key)
            .collect()
    }

    fn stake(&self, k: &T::SignatureKey, e: Option<EpochNumber>) -> Option<PeerConfig<T>> {
        let inner = self.inner.read();
        inner.assert_has_stake_table(e);
        inner.table.stake(k.clone(), e.map(|e| *e)).map(Into::into)
    }

    fn da_stake(&self, k: &T::SignatureKey, e: Option<EpochNumber>) -> Option<PeerConfig<T>> {
        let inner = self.inner.read();
        inner.assert_has_stake_table(e);
        inner
            .table
            .da_stake(k.clone(), e.map(|e| *e))
            .map(Into::into)
    }

    fn has_stake(&self, k: &T::SignatureKey, e: Option<EpochNumber>) -> bool {
        self.stake(k, e)
            .is_some_and(|x| x.stake_table_entry.stake() > U256::ZERO)
    }

    fn has_da_stake(&self, k: &T::SignatureKey, e: Option<EpochNumber>) -> bool {
        self.da_stake(k, e)
            .is_some_and(|x| x.stake_table_entry.stake() > U256::ZERO)
    }

    fn lookup_leader(
        &self,
        v: ViewNumber,
        e: Option<EpochNumber>,
    ) -> Result<T::SignatureKey, Self::Error> {
        let inner = self.inner.read();
        inner.assert_has_randomized_stake_table(e);
        Ok(inner.table.lookup_leader(*v, e.map(|e| *e))?)
    }

    fn total_nodes(&self, e: Option<EpochNumber>) -> usize {
        let inner = self.inner.read();
        inner.assert_has_stake_table(e);
        inner.table.stake_table(e.map(|e| *e)).len()
    }

    fn da_total_nodes(&self, e: Option<EpochNumber>) -> usize {
        let inner = self.inner.read();
        inner.assert_has_stake_table(e);
        inner.table.da_stake_table(e.map(|e| *e)).len()
    }

    fn has_stake_table(&self, e: EpochNumber) -> bool {
        let inner = self.inner.read();
        let has_stake_table = inner.table.has_stake_table(*e);
        assert_eq!(has_stake_table, inner.epochs.contains(&e));
        has_stake_table
    }

    fn has_randomized_stake_table(&self, e: EpochNumber) -> Result<bool, Self::Error> {
        if !self.has_stake_table(e) {
            return Ok(false);
        }
        let inner = self.inner.read();
        let has_randomized_stake_table = inner.table.has_randomized_stake_table(*e);
        if let Ok(result) = has_randomized_stake_table {
            assert_eq!(result, inner.drbs.contains(&e));
        } else {
            assert!(!inner.drbs.contains(&e));
        }
        Ok(has_randomized_stake_table?)
    }

    fn add_drb_result(&self, e: EpochNumber, drb: DrbResult) {
        let mut inner = self.inner.write();
        inner.assert_has_stake_table(Some(e));
        inner.drbs.insert(e);
        inner.table.add_drb_result(*e, drb);
    }

    fn first_epoch(&self) -> Option<EpochNumber> {
        self.inner.read().table.first_epoch().map(EpochNumber::new)
    }

    fn set_first_epoch(&self, e: EpochNumber, initial_drb_result: DrbResult) {
        let mut inner = self.inner.write();
        inner.epochs.insert(e);
        inner.epochs.insert(e + 1);

        inner.drbs.insert(e);
        inner.drbs.insert(e + 1);

        inner.table.set_first_epoch(*e, initial_drb_result);
    }

    async fn add_epoch_root(&self, hdr: T::BlockHeader) -> Result<(), Self::Error> {
        let epoch = epoch_from_block_number(hdr.block_number(), *self.epoch_height) + 2;

        let mut inner = self.inner.write();
        inner.epochs.insert(EpochNumber::new(epoch));
        inner.table.add_epoch_root(epoch);

        Ok(())
    }

    async fn get_epoch_root(&self, e: EpochNumber) -> Result<Leaf2<T>, Self::Error> {
        let block_height = root_block_in_epoch(*e, *self.epoch_height);

        let (stake_table, fetcher) = {
            let inner = self.inner.read();
            let table = inner.table.stake_table(Some(*e));
            let fetcher = inner
                .fetcher
                .clone()
                .expect("get_epoch_root called before set_leaf_fetcher_network");
            (table, fetcher)
        };

        for node in stake_table {
            if let Ok(leaf) = fetcher
                .read()
                .await
                .fetch_leaf(block_height, node.signature_key)
                .await
            {
                return Ok(leaf);
            }
        }

        Err(anyhow!("Failed to fetch epoch root from any peer").into())
    }

    async fn get_epoch_drb(&self, e: EpochNumber) -> Result<DrbResult, Self::Error> {
        let epoch_height = self.epoch_height;

        let (epoch_drb, fetcher) = {
            let state = self.inner.read();
            let drb = state.table.get_epoch_drb(*e);
            let fetcher = state.fetcher.clone();
            (drb, fetcher)
        };

        if let Ok(drb_result) = epoch_drb {
            Ok(drb_result)
        } else {
            let previous_epoch = match e.checked_sub(1) {
                Some(epoch) => epoch,
                None => {
                    return Err(anyhow!("Missing initial DRB result for epoch {e:?}").into());
                },
            };

            let drb_block_height = transition_block_for_epoch(previous_epoch, *epoch_height);
            let stake_table = self.inner.read().table.stake_table(Some(previous_epoch));
            let fetcher = fetcher.expect("get_epoch_drb called before set_leaf_fetcher_network");

            let mut drb_leaf = None;

            for node in stake_table {
                if let Ok(leaf) = fetcher
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
                None => Err(anyhow!(
                    "Failed to fetch leaf from all nodes. Height: {drb_block_height}"
                )
                .into()),
            }
        }
    }

    fn add_da_committee(&self, first_epoch: EpochNumber, committee: Vec<PeerConfig<T>>) {
        self.inner.write().table.add_da_committee(
            *first_epoch,
            committee.into_iter().map(Into::into).collect(),
        );
    }
}

#[derive(Debug, thiserror::Error)]
#[error("strict membership error: {0}")]
pub struct StrictMembershipError(#[from] anyhow::Error);
