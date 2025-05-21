// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

#![cfg(feature = "file-system-data-source")]

use std::{
    collections::{
        hash_map::{Entry, HashMap},
        BTreeMap,
    },
    hash::Hash,
    ops::{Bound, Deref, RangeBounds},
    path::Path,
};

use async_lock::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use async_trait::async_trait;
use atomic_store::{AtomicStore, AtomicStoreLoader, PersistenceError};
use committable::Committable;
use futures::future::Future;
use hotshot_types::{
    data::{VidCommitment, VidShare},
    traits::{
        block_contents::BlockHeader,
        node_implementation::{ConsensusTime, NodeType},
    },
};
use serde::{de::DeserializeOwned, Serialize};
use snafu::OptionExt;

use super::{
    ledger_log::{Iter, LedgerLog},
    pruning::{PruneStorage, PrunedHeightStorage, PrunerConfig},
    sql::MigrateTypes,
    Aggregate, AggregatesStorage, AvailabilityStorage, NodeStorage, PayloadMetadata,
    UpdateAggregatesStorage, UpdateAvailabilityStorage, VidCommonMetadata,
};
use crate::{
    availability::{
        data_source::{BlockId, LeafId},
        query_data::{
            BlockHash, BlockQueryData, LeafHash, LeafQueryData, PayloadQueryData, QueryableHeader,
            QueryablePayload, TransactionHash, TransactionQueryData, VidCommonQueryData,
        },
        StateCertQueryData,
    },
    data_source::{update, VersionedDataSource},
    metrics::PrometheusMetrics,
    node::{SyncStatus, TimeWindowQueryData, WindowStart},
    status::HasMetrics,
    types::HeightIndexed,
    ErrorSnafu, Header, MissingSnafu, NotFoundSnafu, Payload, QueryError, QueryResult,
};

const CACHED_LEAVES_COUNT: usize = 100;
const CACHED_BLOCKS_COUNT: usize = 100;
const CACHED_VID_COMMON_COUNT: usize = 100;
const CACHED_STATE_CERT_COUNT: usize = 5;

#[derive(custom_debug::Debug)]
pub struct FileSystemStorageInner<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    index_by_leaf_hash: HashMap<LeafHash<Types>, u64>,
    index_by_block_hash: HashMap<BlockHash<Types>, u64>,
    index_by_payload_hash: HashMap<VidCommitment, u64>,
    index_by_txn_hash: HashMap<TransactionHash<Types>, u64>,
    index_by_time: BTreeMap<u64, Vec<u64>>,
    num_transactions: usize,
    payload_size: usize,
    #[debug(skip)]
    top_storage: Option<AtomicStore>,
    leaf_storage: LedgerLog<LeafQueryData<Types>>,
    block_storage: LedgerLog<BlockQueryData<Types>>,
    vid_storage: LedgerLog<(VidCommonQueryData<Types>, Option<VidShare>)>,
    state_cert_storage: LedgerLog<StateCertQueryData<Types>>,
}

impl<Types> FileSystemStorageInner<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    fn get_block_index(&self, id: BlockId<Types>) -> QueryResult<usize> {
        match id {
            BlockId::Number(n) => Ok(n),
            BlockId::Hash(h) => {
                Ok(*self.index_by_block_hash.get(&h).context(NotFoundSnafu)? as usize)
            },
            BlockId::PayloadHash(h) => {
                Ok(*self.index_by_payload_hash.get(&h).context(NotFoundSnafu)? as usize)
            },
        }
    }

    fn get_block(&self, id: BlockId<Types>) -> QueryResult<BlockQueryData<Types>> {
        self.block_storage
            .iter()
            .nth(self.get_block_index(id)?)
            .context(NotFoundSnafu)?
            .context(MissingSnafu)
    }

    fn get_header(&self, id: BlockId<Types>) -> QueryResult<Header<Types>> {
        self.get_block(id).map(|block| block.header)
    }

    fn get_block_range<R>(&self, range: R) -> QueryResult<Vec<QueryResult<BlockQueryData<Types>>>>
    where
        R: RangeBounds<usize> + Send,
    {
        Ok(range_iter(self.block_storage.iter(), range).collect())
    }
}

/// Storage for the APIs provided in this crate, backed by a remote PostgreSQL database.
#[derive(Debug)]
pub struct FileSystemStorage<Types: NodeType>
where
    Payload<Types>: QueryablePayload<Types>,
{
    inner: RwLock<FileSystemStorageInner<Types>>,
    metrics: PrometheusMetrics,
}

impl<Types: NodeType> PrunerConfig for FileSystemStorage<Types> where
    Payload<Types>: QueryablePayload<Types>
{
}
impl<Types: NodeType> PruneStorage for FileSystemStorage<Types>
where
    Payload<Types>: QueryablePayload<Types>,
{
    type Pruner = ();
}

#[async_trait]
impl<Types: NodeType> MigrateTypes<Types> for FileSystemStorage<Types>
where
    Payload<Types>: QueryablePayload<Types>,
{
    async fn migrate_types(&self, _batch_size: u64) -> anyhow::Result<()> {
        Ok(())
    }
}

impl<Types: NodeType> FileSystemStorage<Types>
where
    Payload<Types>: QueryablePayload<Types>,
    Header<Types>: QueryableHeader<Types>,
{
    /// Create a new [FileSystemStorage] with storage at `path`.
    ///
    /// If there is already data at `path`, it will be archived.
    ///
    /// The [FileSystemStorage] will manage its own persistence synchronization.
    pub async fn create(path: &Path) -> Result<Self, PersistenceError> {
        let mut loader = AtomicStoreLoader::create(path, "hotshot_data_source")?;
        loader.retain_archives(1);
        let data_source = Self::create_with_store(&mut loader).await?;
        data_source.inner.write().await.top_storage = Some(AtomicStore::open(loader)?);
        Ok(data_source)
    }

    /// Open an existing [FileSystemStorage] from storage at `path`.
    ///
    /// If there is no data at `path`, a new store will be created.
    ///
    /// The [FileSystemStorage] will manage its own persistence synchronization.
    pub async fn open(path: &Path) -> Result<Self, PersistenceError> {
        let mut loader = AtomicStoreLoader::load(path, "hotshot_data_source")?;
        loader.retain_archives(1);
        let data_source = Self::open_with_store(&mut loader).await?;
        data_source.inner.write().await.top_storage = Some(AtomicStore::open(loader)?);
        Ok(data_source)
    }

    /// Create a new [FileSystemStorage] using a persistent storage loader.
    ///
    /// If there is existing data corresponding to the [FileSystemStorage] data structures, it will
    /// be archived.
    ///
    /// The [FileSystemStorage] will register its persistent data structures with `loader`. The
    /// caller is responsible for creating an [AtomicStore] from `loader` and managing
    /// synchronization of the store.
    pub async fn create_with_store(
        loader: &mut AtomicStoreLoader,
    ) -> Result<Self, PersistenceError> {
        Ok(Self {
            inner: RwLock::new(FileSystemStorageInner {
                index_by_leaf_hash: Default::default(),
                index_by_block_hash: Default::default(),
                index_by_payload_hash: Default::default(),
                index_by_txn_hash: Default::default(),
                index_by_time: Default::default(),
                num_transactions: 0,
                payload_size: 0,
                top_storage: None,
                leaf_storage: LedgerLog::create(loader, "leaves", CACHED_LEAVES_COUNT)?,
                block_storage: LedgerLog::create(loader, "blocks", CACHED_BLOCKS_COUNT)?,
                vid_storage: LedgerLog::create(loader, "vid_common", CACHED_VID_COMMON_COUNT)?,
                state_cert_storage: LedgerLog::create(
                    loader,
                    "state_cert",
                    CACHED_STATE_CERT_COUNT,
                )?,
            }),
            metrics: Default::default(),
        })
    }

    /// Open an existing [FileSystemStorage] using a persistent storage loader.
    ///
    /// If there is no existing data corresponding to the [FileSystemStorage] data structures, a new
    /// store will be created.
    ///
    /// The [FileSystemStorage] will register its persistent data structures with `loader`. The
    /// caller is responsible for creating an [AtomicStore] from `loader` and managing
    /// synchronization of the store.
    pub async fn open_with_store(loader: &mut AtomicStoreLoader) -> Result<Self, PersistenceError> {
        let leaf_storage =
            LedgerLog::<LeafQueryData<Types>>::open(loader, "leaves", CACHED_LEAVES_COUNT)?;
        let block_storage =
            LedgerLog::<BlockQueryData<Types>>::open(loader, "blocks", CACHED_BLOCKS_COUNT)?;
        let vid_storage = LedgerLog::<(VidCommonQueryData<Types>, Option<VidShare>)>::open(
            loader,
            "vid_common",
            CACHED_VID_COMMON_COUNT,
        )?;
        let state_cert_storage = LedgerLog::<StateCertQueryData<Types>>::open(
            loader,
            "state_cert",
            CACHED_STATE_CERT_COUNT,
        )?;

        let mut index_by_block_hash = HashMap::new();
        let mut index_by_payload_hash = HashMap::new();
        let mut index_by_time = BTreeMap::<u64, Vec<u64>>::new();
        let index_by_leaf_hash = leaf_storage
            .iter()
            .flatten()
            .map(|leaf| {
                update_index_by_hash(&mut index_by_block_hash, leaf.block_hash(), leaf.height());
                update_index_by_hash(
                    &mut index_by_payload_hash,
                    leaf.payload_hash(),
                    leaf.height(),
                );
                index_by_time
                    .entry(leaf.header().timestamp())
                    .or_default()
                    .push(leaf.height());
                (leaf.hash(), leaf.height())
            })
            .collect();

        let mut index_by_txn_hash = HashMap::new();
        let mut num_transactions = 0;
        let mut payload_size = 0;
        for block in block_storage.iter().flatten() {
            num_transactions += block.len();
            payload_size += block.size() as usize;

            let height = block.height();
            for (_, txn) in block.enumerate() {
                update_index_by_hash(&mut index_by_txn_hash, txn.commit(), height);
            }
        }

        Ok(Self {
            inner: RwLock::new(FileSystemStorageInner {
                index_by_leaf_hash,
                index_by_block_hash,
                index_by_payload_hash,
                index_by_txn_hash,
                index_by_time,
                num_transactions,
                payload_size,
                leaf_storage,
                block_storage,
                vid_storage,
                state_cert_storage,
                top_storage: None,
            }),
            metrics: Default::default(),
        })
    }

    /// Advance the version of the persistent store without committing changes to persistent state.
    pub async fn skip_version(&self) -> Result<(), PersistenceError> {
        let mut inner = self.inner.write().await;
        inner.leaf_storage.skip_version()?;
        inner.block_storage.skip_version()?;
        inner.vid_storage.skip_version()?;
        inner.state_cert_storage.skip_version()?;
        if let Some(store) = &mut inner.top_storage {
            store.commit_version()?;
        }
        Ok(())
    }
}

pub trait Revert {
    fn revert(&mut self);
}

impl<Types> Revert for RwLockWriteGuard<'_, FileSystemStorageInner<Types>>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    fn revert(&mut self) {
        self.leaf_storage.revert_version().unwrap();
        self.block_storage.revert_version().unwrap();
        self.vid_storage.revert_version().unwrap();
        self.state_cert_storage.revert_version().unwrap();
    }
}

impl<Types> Revert for RwLockReadGuard<'_, FileSystemStorageInner<Types>>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    fn revert(&mut self) {
        // Nothing to revert for a read-only transaction.
    }
}

#[derive(Debug)]
pub struct Transaction<T: Revert> {
    inner: T,
}

impl<T: Revert> Drop for Transaction<T> {
    fn drop(&mut self) {
        self.inner.revert();
    }
}
impl<Types> update::Transaction for Transaction<RwLockWriteGuard<'_, FileSystemStorageInner<Types>>>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    async fn commit(mut self) -> anyhow::Result<()> {
        self.inner.leaf_storage.commit_version().await?;
        self.inner.block_storage.commit_version().await?;
        self.inner.vid_storage.commit_version().await?;
        self.inner.state_cert_storage.commit_version().await?;
        if let Some(store) = &mut self.inner.top_storage {
            store.commit_version()?;
        }
        Ok(())
    }

    fn revert(self) -> impl Future + Send {
        // The revert is handled when `self` is dropped.
        async move {}
    }
}

impl<Types> update::Transaction for Transaction<RwLockReadGuard<'_, FileSystemStorageInner<Types>>>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    async fn commit(self) -> anyhow::Result<()> {
        // Nothing to commit for a read-only transaction.
        Ok(())
    }

    fn revert(self) -> impl Future + Send {
        // The revert is handled when `self` is dropped.
        async move {}
    }
}

impl<Types: NodeType> VersionedDataSource for FileSystemStorage<Types>
where
    Payload<Types>: QueryablePayload<Types>,
{
    type Transaction<'a>
        = Transaction<RwLockWriteGuard<'a, FileSystemStorageInner<Types>>>
    where
        Self: 'a;
    type ReadOnly<'a>
        = Transaction<RwLockReadGuard<'a, FileSystemStorageInner<Types>>>
    where
        Self: 'a;

    async fn write(&self) -> anyhow::Result<Self::Transaction<'_>> {
        Ok(Transaction {
            inner: self.inner.write().await,
        })
    }

    async fn read(&self) -> anyhow::Result<Self::ReadOnly<'_>> {
        Ok(Transaction {
            inner: self.inner.read().await,
        })
    }
}
fn range_iter<T>(
    mut iter: Iter<'_, T>,
    range: impl RangeBounds<usize>,
) -> impl '_ + Iterator<Item = QueryResult<T>>
where
    T: Clone + Serialize + DeserializeOwned,
{
    let start = range.start_bound().cloned();
    let end = range.end_bound().cloned();

    // Advance the underlying iterator to the start of the range.
    let pos = match start {
        Bound::Included(n) => {
            if n > 0 {
                iter.nth(n - 1);
            }
            n
        },
        Bound::Excluded(n) => {
            iter.nth(n);
            n + 1
        },
        Bound::Unbounded => 0,
    };

    itertools::unfold((iter, end, pos), |(iter, end, pos)| {
        // Check if we have reached the end of the range.
        let reached_end = match end {
            Bound::Included(n) => pos > n,
            Bound::Excluded(n) => pos >= n,
            Bound::Unbounded => false,
        };
        if reached_end {
            return None;
        }
        let opt = iter.next()?;
        *pos += 1;
        Some(opt.context(MissingSnafu))
    })
}

#[async_trait]
impl<Types, T> AvailabilityStorage<Types> for Transaction<T>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
    Header<Types>: QueryableHeader<Types>,
    T: Revert + Deref<Target = FileSystemStorageInner<Types>> + Send + Sync,
{
    async fn get_leaf(&mut self, id: LeafId<Types>) -> QueryResult<LeafQueryData<Types>> {
        let n = match id {
            LeafId::Number(n) => n,
            LeafId::Hash(h) => *self
                .inner
                .index_by_leaf_hash
                .get(&h)
                .context(NotFoundSnafu)? as usize,
        };
        self.inner
            .leaf_storage
            .iter()
            .nth(n)
            .context(NotFoundSnafu)?
            .context(MissingSnafu)
    }

    async fn get_block(&mut self, id: BlockId<Types>) -> QueryResult<BlockQueryData<Types>> {
        self.inner.get_block(id)
    }

    async fn get_header(&mut self, id: BlockId<Types>) -> QueryResult<Header<Types>> {
        self.inner.get_header(id)
    }

    async fn get_payload(&mut self, id: BlockId<Types>) -> QueryResult<PayloadQueryData<Types>> {
        self.get_block(id).await.map(PayloadQueryData::from)
    }

    async fn get_payload_metadata(
        &mut self,
        id: BlockId<Types>,
    ) -> QueryResult<PayloadMetadata<Types>> {
        self.get_block(id).await.map(PayloadMetadata::from)
    }

    async fn get_vid_common(
        &mut self,
        id: BlockId<Types>,
    ) -> QueryResult<VidCommonQueryData<Types>> {
        Ok(self
            .inner
            .vid_storage
            .iter()
            .nth(self.inner.get_block_index(id)?)
            .context(NotFoundSnafu)?
            .context(MissingSnafu)?
            .0)
    }

    async fn get_vid_common_metadata(
        &mut self,
        id: BlockId<Types>,
    ) -> QueryResult<VidCommonMetadata<Types>> {
        self.get_vid_common(id).await.map(VidCommonMetadata::from)
    }

    async fn get_leaf_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<LeafQueryData<Types>>>>
    where
        R: RangeBounds<usize> + Send,
    {
        Ok(range_iter(self.inner.leaf_storage.iter(), range).collect())
    }

    async fn get_block_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<BlockQueryData<Types>>>>
    where
        R: RangeBounds<usize> + Send,
    {
        self.inner.get_block_range(range)
    }

    async fn get_payload_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<PayloadQueryData<Types>>>>
    where
        R: RangeBounds<usize> + Send,
    {
        Ok(range_iter(self.inner.block_storage.iter(), range)
            .map(|res| res.map(PayloadQueryData::from))
            .collect())
    }

    async fn get_payload_metadata_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<PayloadMetadata<Types>>>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        Ok(range_iter(self.inner.block_storage.iter(), range)
            .map(|res| res.map(PayloadMetadata::from))
            .collect())
    }

    async fn get_vid_common_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<VidCommonQueryData<Types>>>>
    where
        R: RangeBounds<usize> + Send,
    {
        Ok(range_iter(self.inner.vid_storage.iter(), range)
            .map(|res| res.map(|(common, _)| common))
            .collect())
    }

    async fn get_vid_common_metadata_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<VidCommonMetadata<Types>>>>
    where
        R: RangeBounds<usize> + Send,
    {
        Ok(range_iter(self.inner.vid_storage.iter(), range)
            .map(|res| res.map(|(common, _)| common.into()))
            .collect())
    }

    async fn get_transaction(
        &mut self,
        hash: TransactionHash<Types>,
    ) -> QueryResult<TransactionQueryData<Types>> {
        let height = self
            .inner
            .index_by_txn_hash
            .get(&hash)
            .context(NotFoundSnafu)?;
        let block = self.inner.get_block((*height as usize).into())?;
        TransactionQueryData::with_hash(&block, hash).context(ErrorSnafu {
            message: format!(
                "transaction index inconsistent: block {height} contains no transaction {hash}"
            ),
        })
    }

    async fn first_available_leaf(&mut self, from: u64) -> QueryResult<LeafQueryData<Types>> {
        // The file system backend doesn't index by whether a leaf is present, so we can't
        // efficiently seek to the first leaf with height >= `from`. Our best effort is to return
        // `from` itself if we can, or fail.
        self.get_leaf((from as usize).into()).await
    }

    async fn get_state_cert(&mut self, epoch: u64) -> QueryResult<StateCertQueryData<Types>> {
        self.inner
            .state_cert_storage
            .iter()
            .nth(epoch as usize)
            .context(NotFoundSnafu)?
            .context(MissingSnafu)
    }
}

impl<Types: NodeType> UpdateAvailabilityStorage<Types>
    for Transaction<RwLockWriteGuard<'_, FileSystemStorageInner<Types>>>
where
    Payload<Types>: QueryablePayload<Types>,
    Header<Types>: QueryableHeader<Types>,
{
    async fn insert_leaf(&mut self, leaf: LeafQueryData<Types>) -> anyhow::Result<()> {
        self.inner
            .leaf_storage
            .insert(leaf.height() as usize, leaf.clone())?;
        self.inner
            .index_by_leaf_hash
            .insert(leaf.hash(), leaf.height());
        update_index_by_hash(
            &mut self.inner.index_by_block_hash,
            leaf.block_hash(),
            leaf.height(),
        );
        update_index_by_hash(
            &mut self.inner.index_by_payload_hash,
            leaf.payload_hash(),
            leaf.height(),
        );
        self.inner
            .index_by_time
            .entry(leaf.header().timestamp())
            .or_default()
            .push(leaf.height());
        Ok(())
    }

    async fn insert_block(&mut self, block: BlockQueryData<Types>) -> anyhow::Result<()> {
        if !self
            .inner
            .block_storage
            .insert(block.height() as usize, block.clone())?
        {
            // The block was already present.
            return Ok(());
        }
        self.inner.num_transactions += block.len();
        self.inner.payload_size += block.size() as usize;
        for (_, txn) in block.enumerate() {
            update_index_by_hash(
                &mut self.inner.index_by_txn_hash,
                txn.commit(),
                block.height(),
            );
        }
        Ok(())
    }

    async fn insert_vid(
        &mut self,
        common: VidCommonQueryData<Types>,
        share: Option<VidShare>,
    ) -> anyhow::Result<()> {
        self.inner
            .vid_storage
            .insert(common.height() as usize, (common, share))?;
        Ok(())
    }

    async fn insert_state_cert(
        &mut self,
        state_cert: StateCertQueryData<Types>,
    ) -> anyhow::Result<()> {
        self.inner
            .state_cert_storage
            .insert(state_cert.0.epoch.u64() as usize, state_cert)?;
        Ok(())
    }
}

/// Update an index mapping hashes of objects to their positions in the ledger.
///
/// This function will insert the mapping from `hash` to `pos` into `index`, _unless_ there is
/// already an entry for `hash` at an earlier position in the ledger.
fn update_index_by_hash<H: Eq + Hash, P: Ord>(index: &mut HashMap<H, P>, hash: H, pos: P) {
    match index.entry(hash) {
        Entry::Occupied(mut e) => {
            if &pos < e.get() {
                // Overwrite the existing entry if the new object was sequenced first.
                e.insert(pos);
            }
        },
        Entry::Vacant(e) => {
            e.insert(pos);
        },
    }
}

#[async_trait]
impl<Types, T> NodeStorage<Types> for Transaction<T>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
    Header<Types>: QueryableHeader<Types>,
    T: Revert + Deref<Target = FileSystemStorageInner<Types>> + Send,
{
    async fn block_height(&mut self) -> QueryResult<usize> {
        Ok(self.inner.leaf_storage.iter().len())
    }

    async fn count_transactions_in_range(
        &mut self,
        range: impl RangeBounds<usize> + Send,
    ) -> QueryResult<usize> {
        if !matches!(range.start_bound(), Bound::Unbounded | Bound::Included(0))
            || !matches!(range.end_bound(), Bound::Unbounded)
        {
            return Err(QueryError::Error {
                message: "partial aggregates are not supported with file system backend".into(),
            });
        }

        Ok(self.inner.num_transactions)
    }

    async fn payload_size_in_range(
        &mut self,
        range: impl RangeBounds<usize> + Send,
    ) -> QueryResult<usize> {
        if !matches!(range.start_bound(), Bound::Unbounded | Bound::Included(0))
            || !matches!(range.end_bound(), Bound::Unbounded)
        {
            return Err(QueryError::Error {
                message: "partial aggregates are not supported with file system backend".into(),
            });
        }

        Ok(self.inner.payload_size)
    }

    async fn vid_share<ID>(&mut self, id: ID) -> QueryResult<VidShare>
    where
        ID: Into<BlockId<Types>> + Send + Sync,
    {
        self.inner
            .vid_storage
            .iter()
            .nth(self.inner.get_block_index(id.into())?)
            .context(NotFoundSnafu)?
            .context(MissingSnafu)?
            .1
            .context(MissingSnafu)
    }

    async fn sync_status(&mut self) -> QueryResult<SyncStatus> {
        let height = self.inner.leaf_storage.iter().len();

        // The number of missing VID common is just the number of completely missing VID
        // entries, since every entry we have is guaranteed to have the common data.
        let missing_vid = self.inner.vid_storage.missing(height);
        // Missing shares includes the completely missing VID entries, plus any entry which
        // is _not_ messing but which has a null share.
        let null_vid_shares: usize = self
            .inner
            .vid_storage
            .iter()
            .map(|res| if matches!(res, Some((_, None))) { 1 } else { 0 })
            .sum();
        Ok(SyncStatus {
            missing_blocks: self.inner.block_storage.missing(height),
            missing_leaves: self.inner.leaf_storage.missing(height),
            missing_vid_common: missing_vid,
            missing_vid_shares: missing_vid + null_vid_shares,
            pruned_height: None,
        })
    }

    async fn get_header_window(
        &mut self,
        start: impl Into<WindowStart<Types>> + Send + Sync,
        end: u64,
        limit: usize,
    ) -> QueryResult<TimeWindowQueryData<Header<Types>>> {
        let first_block = match start.into() {
            WindowStart::Height(h) => h,
            WindowStart::Hash(h) => self.inner.get_header(h.into())?.block_number(),
            WindowStart::Time(t) => {
                // Find the minimum timestamp which is at least `t`, and all the blocks with
                // that timestamp.
                let blocks = self
                    .inner
                    .index_by_time
                    .range(t..)
                    .next()
                    .context(NotFoundSnafu)?
                    .1;
                // Multiple blocks can have the same timestamp (when truncated to seconds);
                // we want the first one. It is an invariant that any timestamp which has an
                // entry in `index_by_time` has a non-empty list associated with it, so this
                // indexing is safe.
                blocks[0]
            },
        } as usize;

        let mut res = TimeWindowQueryData::default();

        // Include the block just before the start of the window, if there is one.
        if first_block > 0 {
            res.prev = Some(self.inner.get_header((first_block - 1).into())?);
        }

        // Add blocks to the window, starting from `first_block`, until we reach the end of
        // the requested time window.
        for block in self.inner.get_block_range(first_block..)? {
            let header = block?.header().clone();
            if header.timestamp() >= end {
                res.next = Some(header);
                break;
            }
            res.window.push(header);
            if res.window.len() >= limit {
                break;
            }
        }

        Ok(res)
    }
}

impl<T: Revert + Send> AggregatesStorage for Transaction<T> {
    async fn aggregates_height(&mut self) -> anyhow::Result<usize> {
        Ok(0)
    }

    async fn load_prev_aggregate(&mut self) -> anyhow::Result<Option<Aggregate>> {
        Ok(None)
    }
}

impl<Types, T: Revert + Send> UpdateAggregatesStorage<Types> for Transaction<T>
where
    Types: NodeType,
{
    async fn update_aggregates(
        &mut self,
        _prev: Aggregate,
        _blocks: &[PayloadMetadata<Types>],
    ) -> anyhow::Result<Aggregate> {
        Ok(Aggregate::default())
    }
}

impl<T: Revert> PrunedHeightStorage for Transaction<T> {}

impl<Types> HasMetrics for FileSystemStorage<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    fn metrics(&self) -> &PrometheusMetrics {
        &self.metrics
    }
}
