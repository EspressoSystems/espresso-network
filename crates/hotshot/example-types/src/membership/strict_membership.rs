use std::{
    collections::HashSet,
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
    traits::{
        block_contents::BlockHeader,
        election::{Membership, MembershipSnapshot, NoStakeTableHash, NonEpochMembershipSnapshot},
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
}

impl<T, S> Membership<T> for StrictMembership<T, S>
where
    T: NodeType,
    S: TestStakeTable<T::SignatureKey, T::StateSignatureKey>,
{
    type Error = StrictMembershipError;
    type Snapshot = StrictEpochSnapshot<T, S>;
    type NonEpochSnapshot = StrictNonEpochSnapshot<T, S>;

    fn snapshot(&self, epoch: EpochNumber) -> Option<Self::Snapshot> {
        let inner = self.inner.read();
        if !inner.epochs.contains(&epoch) {
            return None;
        }
        let has_drb = inner.drbs.contains(&epoch);
        let first_epoch = inner.table.first_epoch().map(EpochNumber::new);
        Some(StrictEpochSnapshot::build(
            epoch,
            first_epoch,
            has_drb,
            inner.table.clone(),
        ))
    }

    fn non_epoch_snapshot(&self) -> Self::NonEpochSnapshot {
        StrictNonEpochSnapshot::build(self.inner.read().table.clone())
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

/// Per-epoch snapshot for `StrictMembership`.
///
/// Materializes the stake-table views at construction time so accessors can
/// return borrowed iterators.
pub struct StrictEpochSnapshot<T, S>
where
    T: NodeType,
    S: TestStakeTable<T::SignatureKey, T::StateSignatureKey>,
{
    epoch: EpochNumber,
    first_epoch: Option<EpochNumber>,
    has_drb: bool,
    stake_table: Vec<PeerConfig<T>>,
    da_stake_table: Vec<PeerConfig<T>>,
    committee_keys: Vec<T::SignatureKey>,
    da_committee_keys: Vec<T::SignatureKey>,
    table: S,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, S> StrictEpochSnapshot<T, S>
where
    T: NodeType,
    S: TestStakeTable<T::SignatureKey, T::StateSignatureKey>,
{
    fn build(
        epoch: EpochNumber,
        first_epoch: Option<EpochNumber>,
        has_drb: bool,
        table: S,
    ) -> Self {
        let stake_entries = table.stake_table(Some(*epoch));
        let da_entries = table.da_stake_table(Some(*epoch));
        let committee_keys = stake_entries
            .iter()
            .map(|e| e.signature_key.clone())
            .collect();
        let da_committee_keys = da_entries.iter().map(|e| e.signature_key.clone()).collect();
        let stake_table = stake_entries.into_iter().map(Into::into).collect();
        let da_stake_table = da_entries.into_iter().map(Into::into).collect();
        Self {
            epoch,
            first_epoch,
            has_drb,
            stake_table,
            da_stake_table,
            committee_keys,
            da_committee_keys,
            table,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, S> Clone for StrictEpochSnapshot<T, S>
where
    T: NodeType,
    S: TestStakeTable<T::SignatureKey, T::StateSignatureKey>,
{
    fn clone(&self) -> Self {
        Self {
            epoch: self.epoch,
            first_epoch: self.first_epoch,
            has_drb: self.has_drb,
            stake_table: self.stake_table.clone(),
            da_stake_table: self.da_stake_table.clone(),
            committee_keys: self.committee_keys.clone(),
            da_committee_keys: self.da_committee_keys.clone(),
            table: self.table.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, S> Debug for StrictEpochSnapshot<T, S>
where
    T: NodeType,
    S: TestStakeTable<T::SignatureKey, T::StateSignatureKey>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("StrictEpochSnapshot")
            .field("epoch", &self.epoch)
            .field("first_epoch", &self.first_epoch)
            .field("has_drb", &self.has_drb)
            .field("table", &self.table)
            .finish()
    }
}

impl<T, S> MembershipSnapshot<T> for StrictEpochSnapshot<T, S>
where
    T: NodeType,
    S: TestStakeTable<T::SignatureKey, T::StateSignatureKey>,
{
    type Error = StrictMembershipError;
    type StakeTableHash = NoStakeTableHash;

    fn epoch(&self) -> EpochNumber {
        self.epoch
    }

    fn first_epoch(&self) -> Option<EpochNumber> {
        self.first_epoch
    }

    fn has_drb(&self) -> bool {
        self.has_drb
    }

    fn stake_table(&self) -> impl ExactSizeIterator<Item = &PeerConfig<T>> + Send {
        self.stake_table.iter()
    }

    fn da_stake_table(&self) -> impl ExactSizeIterator<Item = &PeerConfig<T>> + Send {
        self.da_stake_table.iter()
    }

    fn committee_members(
        &self,
        _: ViewNumber,
    ) -> impl ExactSizeIterator<Item = &T::SignatureKey> + Send {
        self.committee_keys.iter()
    }

    fn da_committee_members(
        &self,
        _: ViewNumber,
    ) -> impl ExactSizeIterator<Item = &T::SignatureKey> + Send {
        self.da_committee_keys.iter()
    }

    fn stake(&self, key: &T::SignatureKey) -> Option<PeerConfig<T>> {
        self.table
            .stake(key.clone(), Some(*self.epoch))
            .map(Into::into)
    }

    fn da_stake(&self, key: &T::SignatureKey) -> Option<PeerConfig<T>> {
        self.table
            .da_stake(key.clone(), Some(*self.epoch))
            .map(Into::into)
    }

    fn has_stake(&self, key: &T::SignatureKey) -> bool {
        self.stake(key)
            .is_some_and(|x| x.stake_table_entry.stake() > U256::ZERO)
    }

    fn has_da_stake(&self, key: &T::SignatureKey) -> bool {
        self.da_stake(key)
            .is_some_and(|x| x.stake_table_entry.stake() > U256::ZERO)
    }

    fn lookup_leader(&self, view: ViewNumber) -> Result<T::SignatureKey, Self::Error> {
        Ok(self.table.lookup_leader(*view, Some(*self.epoch))?)
    }
}

/// Pre-epoch snapshot for `StrictMembership`. Materializes views at
/// construction so accessors can return borrowed iterators.
pub struct StrictNonEpochSnapshot<T, S>
where
    T: NodeType,
    S: TestStakeTable<T::SignatureKey, T::StateSignatureKey>,
{
    stake_table: Vec<PeerConfig<T>>,
    da_stake_table: Vec<PeerConfig<T>>,
    committee_keys: Vec<T::SignatureKey>,
    da_committee_keys: Vec<T::SignatureKey>,
    table: S,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, S> StrictNonEpochSnapshot<T, S>
where
    T: NodeType,
    S: TestStakeTable<T::SignatureKey, T::StateSignatureKey>,
{
    fn build(table: S) -> Self {
        let stake_entries = table.stake_table(None);
        let da_entries = table.da_stake_table(None);
        let committee_keys = stake_entries
            .iter()
            .map(|e| e.signature_key.clone())
            .collect();
        let da_committee_keys = da_entries.iter().map(|e| e.signature_key.clone()).collect();
        let stake_table = stake_entries.into_iter().map(Into::into).collect();
        let da_stake_table = da_entries.into_iter().map(Into::into).collect();
        Self {
            stake_table,
            da_stake_table,
            committee_keys,
            da_committee_keys,
            table,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, S> Clone for StrictNonEpochSnapshot<T, S>
where
    T: NodeType,
    S: TestStakeTable<T::SignatureKey, T::StateSignatureKey>,
{
    fn clone(&self) -> Self {
        Self {
            stake_table: self.stake_table.clone(),
            da_stake_table: self.da_stake_table.clone(),
            committee_keys: self.committee_keys.clone(),
            da_committee_keys: self.da_committee_keys.clone(),
            table: self.table.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, S> Debug for StrictNonEpochSnapshot<T, S>
where
    T: NodeType,
    S: TestStakeTable<T::SignatureKey, T::StateSignatureKey>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("StrictNonEpochSnapshot")
            .field("table", &self.table)
            .finish()
    }
}

impl<T, S> NonEpochMembershipSnapshot<T> for StrictNonEpochSnapshot<T, S>
where
    T: NodeType,
    S: TestStakeTable<T::SignatureKey, T::StateSignatureKey>,
{
    type Error = StrictMembershipError;

    fn stake_table(&self) -> impl ExactSizeIterator<Item = &PeerConfig<T>> + Send + '_ {
        self.stake_table.iter()
    }

    fn da_stake_table(&self) -> impl ExactSizeIterator<Item = &PeerConfig<T>> + Send + '_ {
        self.da_stake_table.iter()
    }

    fn committee_members(
        &self,
        _: ViewNumber,
    ) -> impl ExactSizeIterator<Item = &T::SignatureKey> + Send + '_ {
        self.committee_keys.iter()
    }

    fn da_committee_members(
        &self,
        _: ViewNumber,
    ) -> impl ExactSizeIterator<Item = &T::SignatureKey> + Send + '_ {
        self.da_committee_keys.iter()
    }

    fn stake(&self, key: &T::SignatureKey) -> Option<PeerConfig<T>> {
        self.table.stake(key.clone(), None).map(Into::into)
    }

    fn da_stake(&self, key: &T::SignatureKey) -> Option<PeerConfig<T>> {
        self.table.da_stake(key.clone(), None).map(Into::into)
    }

    fn has_stake(&self, key: &T::SignatureKey) -> bool {
        self.stake(key)
            .is_some_and(|x| x.stake_table_entry.stake() > U256::ZERO)
    }

    fn has_da_stake(&self, key: &T::SignatureKey) -> bool {
        self.da_stake(key)
            .is_some_and(|x| x.stake_table_entry.stake() > U256::ZERO)
    }

    fn lookup_leader(&self, view: ViewNumber) -> Result<T::SignatureKey, Self::Error> {
        Ok(self.table.lookup_leader(*view, None)?)
    }
}
