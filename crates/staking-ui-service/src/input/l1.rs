//! Input data from L1.

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use alloy::primitives::BlockHash;
use async_lock::{RwLock, RwLockUpgradableReadGuard, RwLockWriteGuard};
use espresso_types::v0_3::StakeTableEvent;
use futures::stream::{Stream, StreamExt};
use hotshot_contract_adapter::sol_types::RewardClaim::RewardClaimEvents;
use tracing::instrument;

use crate::{
    error::{Error, Result, ensure},
    types::{
        common::{Address, L1BlockId, L1BlockInfo, Timestamp},
        global::{FullNodeSetDiff, FullNodeSetSnapshot, FullNodeSetUpdate},
        wallet::{WalletDiff, WalletSnapshot, WalletUpdate},
    },
};

pub mod options;
mod rpc_stream;
pub mod switching_transport;

pub use rpc_stream::RpcStream;

/// In-memory state populated by the L1 input source.
#[derive(Clone, Debug)]
pub struct State<S> {
    /// Information about each L1 block since the finalized one.
    ///
    /// This forms a chain of updates and snapshots dating back to the persisted/finalized snapshot.
    /// These blocks are kept in order, corresponding 1:1 with L1 blocks. As such, `blocks[i]`
    /// corresponds to the state after L1 block `finalized + i` (where the finalized block is
    /// always `blocks[0].block.number`).
    ///
    /// This list always contains at least one entry: `blocks[0]` is the finalized state. This is
    /// either loaded from persistent storage on startup, or initialized to a genesis state.
    blocks: Vec<BlockData>,

    /// Maps L1 block hash to block number for all blocks in `blocks`.
    blocks_by_hash: HashMap<BlockHash, u64>,

    /// Persistent storage handle.
    storage: S,
}

impl<S: L1Persistence> State<S> {
    /// Create a new L1 input data source.
    ///
    /// Previously saved state will be loaded from `storage` if possible. If there is no previous
    /// state in storage, the given genesis state will be used.
    pub async fn new(storage: S, genesis: PersistentSnapshot) -> Result<Self> {
        let snapshot = match storage.finalized_snapshot().await? {
            Some(snapshot) => snapshot,
            None => {
                storage.save_genesis(genesis.clone()).await?;
                genesis
            }
        };

        let finalized = BlockData {
            block: snapshot.block,
            timestamp: snapshot.timestamp,
            node_set: snapshot.node_set,
            wallets: snapshot.wallets,

            // The updates are the difference between this snapshot and the previous one; for the
            // initial snapshot, there is no such difference.
            node_set_update: None,
            wallets_update: None,
        };
        let blocks_by_hash = [(finalized.block.hash, finalized.block.number)]
            .into_iter()
            .collect();
        let blocks = vec![finalized];
        Ok(Self {
            blocks,
            blocks_by_hash,
            storage,
        })
    }

    /// Get the latest known L1 block.
    pub fn latest_l1_block(&self) -> L1BlockId {
        self.blocks[self.blocks.len() - 1].block
    }

    /// Get the hashes identifying a particular L1 block.
    pub fn l1_block(&self, number: u64) -> Result<L1BlockId> {
        Ok(self.block(number)?.block)
    }

    /// Get a snapshot of the full node set at the given L1 block.
    pub fn full_node_set(&self, hash: BlockHash) -> Result<FullNodeSetSnapshot> {
        let number = self.block_number(hash)?;
        Ok(self.block(number)?.node_set.clone())
    }

    /// Get the update applied to the full node set due to the given L1 block.
    pub fn full_node_set_update(&self, hash: BlockHash) -> Result<FullNodeSetUpdate> {
        let number = self.block_number(hash)?;
        let block = self.block(number)?;
        let diff = block.node_set_update.clone().ok_or_else(Error::gone)?;
        let update = FullNodeSetUpdate {
            l1_block: block.block_info(),
            diff,
        };
        Ok(update)
    }

    /// Get a snapshot of the requested wallet state at the given L1 block.
    pub fn wallet(&self, address: Address, hash: BlockHash) -> Result<WalletSnapshot> {
        let number = self.block_number(hash)?;
        let block = self.block(number)?;
        Ok(block
            .wallets
            .get(&address)
            .ok_or_else(|| Error::not_found().context(format!("unknown account {address}")))?
            .clone())
    }

    /// Get the update applied to the requested wallet due to the given L1 block.
    pub fn wallet_update(&self, address: Address, hash: BlockHash) -> Result<WalletUpdate> {
        let number = self.block_number(hash)?;
        let block = self.block(number)?;

        // Check that this account even exists.
        ensure!(
            block.wallets.contains_key(&address),
            Error::not_found().context(format!("unknown account {address}"))
        );

        // Get the update. If there was none for this block, generate a trivial one.
        let diff = block
            .wallets_update
            .as_ref()
            .ok_or_else(Error::gone)?
            .get(&address)
            .cloned()
            .unwrap_or_default();
        Ok(WalletUpdate {
            l1_block: block.block_info(),
            diff,
        })
    }

    /// Get the block with the requested block number.
    fn block(&self, number: u64) -> Result<&BlockData> {
        let finalized = self.blocks[0].block.number;
        ensure!(
            number >= finalized,
            Error::gone().context(format!(
                "requested L1 block {number}; earliest available is {finalized}"
            ))
        );

        let offset = (number - finalized) as usize;
        ensure!(
            offset < self.blocks.len(),
            Error::not_found().context(format!(
                "requested L1 block {number}; latest available is {}",
                finalized + (offset as u64)
            ))
        );

        Ok(&self.blocks[offset])
    }

    /// Get the block number with the requested hash.
    ///
    /// This function will succeed if and only if the resulting block number is in our in-memory
    /// state (i.e. if [`Self::block`] would also succeed).
    fn block_number(&self, hash: BlockHash) -> Result<u64> {
        self.blocks_by_hash
            .get(&hash)
            .copied()
            .ok_or_else(|| Error::not_found().context(format!("unknown block hash {hash}")))
    }

    /// Subscribe to a stream of L1 events and populate `state` accordingly.
    ///
    /// Unless there is some catastrophic error, this future will never resolve. It is best spawned
    /// as a background task.
    #[instrument(skip(state, stream))]
    pub async fn subscribe(state: Arc<RwLock<Self>>, mut stream: impl ResettableStream) {
        while let Some(block) = stream.next().await {
            // Retry on any errors until we have successfully handled this input.
            loop {
                let state = state.upgradable_read().await;
                if let Err(err) = Self::handle_new_head(state, &mut stream, &block).await {
                    tracing::error!(?block, "error processing block input: {err}");
                } else {
                    break;
                }
            }
        }
        tracing::error!("L1 block stream ended unexpectedly");
    }

    /// Handle a single block input from L1.
    ///
    /// Returns an error if some transient error occurred which prevented the block from being
    /// processed (e.g. error reaching database). If a reorg occurs, the `state` and `stream` are
    /// reset appropriately, but the result will be [`Ok`], indicating that the next (old) block
    /// should be consumed from the stream.
    #[instrument(skip(state, stream))]
    async fn handle_new_head(
        mut state: RwLockUpgradableReadGuard<'_, Self>,
        stream: &mut impl ResettableStream,
        head: &BlockInput,
    ) -> Result<()> {
        tracing::debug!("received L1 input");

        // Check that the new block extends the last block; if not there's been a reorg.
        let prev = state.blocks[state.blocks.len() - 1].block;
        if head.block.number != prev.number + 1 || head.block.parent != prev.hash {
            tracing::warn!(?head, ?prev, "new head does not extend previous head");
            let mut state = RwLockUpgradableReadGuard::upgrade(state).await;
            state.reorg(stream).await;
            return Ok(());
        }

        // Handle a new finalized block if there is one.
        let old_finalized = state.blocks[0].block;
        let new_finalized = head.finalized;
        ensure!(
            new_finalized.number <= head.block.number,
            Error::internal().context(format!(
                "stream yielded a finalized block from the future: \
                    finalized {new_finalized:?}, head {:?}",
                head.block
            ))
        );
        if new_finalized.number > old_finalized.number {
            let mut write_state = RwLockUpgradableReadGuard::upgrade(state).await;

            // Make sure the hash of this new finalized block matches what we have in our
            // unfinalized state, if not there has somehow been a reorg of this (presumably pretty
            // old) block, and we need to go back and re-process it.
            let offset = new_finalized.number - old_finalized.number;
            let expected_hash = write_state.blocks[offset as usize].block.hash;
            if new_finalized.hash != expected_hash {
                tracing::warn!(
                    ?new_finalized,
                    %expected_hash,
                    "block finalized with different hash than originally seen"
                );
                write_state.reorg(stream).await;
                return Ok(());
            }

            // Now it is safe to finalize this new finalized block.
            write_state.finalize(new_finalized.number).await?;

            // Drop the write lock while we proceed to process the new block like normal and compute
            // the new state snapshots, as this might be slow.
            state = RwLockWriteGuard::downgrade_to_upgradable(write_state);
        }

        // Convert the input event into new updates to our state.
        let new_block = state.blocks[state.blocks.len() - 1].next(head);

        // Update state. As soon as we take this write lock, we are updating the state object in
        // place. We must not fail after this point, or we may drop the lock while the state is in a
        // partially modified state. All the validation performed up to this point should be
        // sufficient to ensure that we will not fail after this.
        let mut state = RwLockUpgradableReadGuard::upgrade(state).await;
        state
            .blocks_by_hash
            .insert(new_block.block.hash, new_block.block.number);
        state.blocks.push(new_block);

        Ok(())
    }

    /// Reset the state back to the last persisted finalized state.
    async fn reorg(&mut self, stream: &mut impl ResettableStream) {
        tracing::warn!("reorg detected, resetting to finalized state");
        stream.reset(self.blocks[0].block.number).await;
        self.blocks.truncate(1);
        self.blocks_by_hash = [(self.blocks[0].block.hash, self.blocks[0].block.number)]
            .into_iter()
            .collect();
    }

    /// Handle a new finalized block.
    ///
    /// The caller must ensure that `finalized` is in the range `0..self.blocks.len()`, and that the
    /// corresponding block is indeed finalized.
    async fn finalize(&mut self, finalized: u64) -> Result<()> {
        tracing::info!(finalized, "new finalized block");

        // Collect events up to the new finalized block, to write to persistent storage.
        let mut nodes_set_diff = vec![];
        let mut wallets_diff = vec![];
        for block in &self.blocks[1..] {
            if block.block.number > finalized {
                break;
            }

            // We have an invaraint that the update fields of any unfinalized block are not
            // [`None`]. Since we are explicitly skipping the finalized block
            // (`&self.blocks[1..]` above) we can safely unwrap here.
            nodes_set_diff.extend(block.node_set_update.clone().unwrap());
            for (address, diffs) in block.wallets_update.as_ref().unwrap() {
                wallets_diff.extend(diffs.iter().map(|diff| (*address, diff.clone())));
            }
        }

        let finalized_info = self.block(finalized)?;
        self.storage
            .apply_events(
                finalized_info.block,
                finalized_info.timestamp,
                nodes_set_diff,
                wallets_diff,
            )
            .await?;
        self.garbage_collect(finalized);
        Ok(())
    }

    /// Garbage collect in-memory blocks.
    ///
    /// Brings the new `finalized` block to the front of the `blocks` list, deleting all blocks
    /// before it.
    ///
    /// The caller must ensure that `finalized` is in the range `0..self.blocks.len()`.
    fn garbage_collect(&mut self, finalized: u64) {
        // Bring the new finalized block to the front of the list, shifting all now-old blocks to
        // the end.
        let offset = (finalized - self.blocks[0].block.number) as usize;
        self.blocks.rotate_left(offset);
        // Remove the old blocks, and also their entries in `blocks_by_hash`.
        for _ in 0..offset {
            let block = self.blocks.pop().unwrap();
            self.blocks_by_hash.remove(&block.block.hash);
        }
    }
}

/// Snapshots and updates for each L1 block.
#[derive(Clone, Debug, PartialEq)]
struct BlockData {
    /// The L1 block.
    block: L1BlockId,

    /// The L1 block timestamp.
    timestamp: Timestamp,

    /// The full node set as of this L1 block.
    node_set: FullNodeSetSnapshot,

    /// The change to the full node set between the previous L1 block and this one.
    ///
    /// This may be [`None`] when the previous L1 block has been garbage collected.
    node_set_update: Option<Vec<FullNodeSetDiff>>,

    /// The state of each wallet as of this L1 block.
    wallets: WalletsSnapshot,

    /// The changes to each wallet between the previous L1 block and this one.
    ///
    /// Unlike wallet snapshots, we don't benefit from using an immutable map here, as updates are
    /// always unique to a block, not partially shared with previous blocks. However, the good news
    /// is that only a few if any wallets will have non-trivial updates in any given block, so the
    /// size of this set should be small in practice. We use a [`BTreeMap`] to further compress the
    /// size of this set, as B-Trees should generally have a better memory footprint than hash maps.
    ///
    /// If a wallet was not updated in a given block, it will not have an entry here, but a trivial
    /// [`WalletUpdate`] can always be generated for any account by combining the [`L1BlockInfo`]
    /// with an empty [`WalletDiff`] list.
    ///
    /// This may be [`None`] when the previous L1 block has been garbage collected.
    wallets_update: Option<BTreeMap<Address, Vec<WalletDiff>>>,
}

/// A snapshot of the latest state of every wallet.
///
/// Note that while this set can, in concept, become very large, the use of immutable data
/// structures means that the memory for all entries is shared with the previous block's
/// snapshot, excepting only those wallets whose state has actually changed in this L1 block.
/// This will be a small number of wallets (often 0!) in practice.
pub type WalletsSnapshot = im::HashMap<Address, WalletSnapshot>;

impl BlockData {
    fn block_info(&self) -> L1BlockInfo {
        L1BlockInfo {
            number: self.block.number,
            hash: self.block.hash,
            timestamp: self.timestamp,
        }
    }

    fn next(&self, input: &BlockInput) -> Self {
        #![allow(unused_mut)]

        // Cloning entire state in order to apply updates and get the new state. This is cheap
        // thanks to the magic of immutable data structures.
        let mut node_set = self.node_set.clone();
        let mut wallets = self.wallets.clone();

        // Keep track of changes we apply.
        let mut node_set_update = vec![];
        let mut wallets_update = BTreeMap::<Address, Vec<WalletDiff>>::default();

        for event in &input.events {
            // Convert contract events into either node set or wallet updates, and apply updates
            // to state snapshots.
            let (nodes_diff, wallets_diff) = event.diffs();
            for diff in nodes_diff {
                apply_node_set_diff(&mut node_set, &diff);
                node_set_update.push(diff);
            }
            for (address, diff) in wallets_diff {
                apply_wallet_diff(&mut wallets, address, &diff);
                wallets_update.entry(address).or_default().push(diff);
            }
        }

        Self {
            block: input.block,
            timestamp: input.timestamp,
            node_set,
            node_set_update: Some(node_set_update),
            wallets,
            wallets_update: Some(wallets_update),
        }
    }
}

fn apply_node_set_diff(_snapshot: &mut FullNodeSetSnapshot, _diff: &FullNodeSetDiff) {
    // TODO
}

fn apply_wallet_diff(_snapshot: &mut WalletsSnapshot, _address: Address, _diff: &WalletDiff) {
    // TODO
}

/// The minimal data we need in order to ingest a new L1 block.
#[derive(Clone, Debug)]
pub struct BlockInput {
    /// The L1 block.
    pub block: L1BlockId,

    /// The latest finalized block, as of the time of this block being produced as the head.
    pub finalized: L1BlockId,

    /// The timestamp of this L1 block.
    pub timestamp: Timestamp,

    /// Relevant events emitted by this block.
    pub events: Vec<L1Event>,
}

/// The set of L1 events that we care about.
#[derive(Clone, Debug)]
pub enum L1Event {
    /// An event emitted by the reward claim contract.
    Reward(Arc<RewardClaimEvents>),

    /// An event emitted by the stake table contract.
    StakeTable(Arc<StakeTableEvent>),
}

impl L1Event {
    /// Extract changes to our state snapshot caused by this event.
    pub fn diffs(&self) -> (Vec<FullNodeSetDiff>, Vec<(Address, WalletDiff)>) {
        // TODO
        (vec![], vec![])
    }
}

/// A stream of information incoming from the L1.
///
/// This stream is special in that it has an interface for resetting back to the finalized block.
/// This is used to handle reorgs.
///
/// This interface is also unique in that it outputs just the information that the L1 input source
/// needs, namely minimal information about the L1 block itself, plus the parsed event logs that we
/// care about.
///
/// Note that both the stream interface and the [`reset`](Self::reset) function are infallible. It
/// is expected that the underlying implementation has its own way of retrying and/or reconnecting
/// when errors occur, so that, in theory, the interface we consume is that of a never-ending stream
/// of L1 blocks.
pub trait ResettableStream: Unpin + Stream<Item = BlockInput> {
    /// Reset a stream to the state just after block number `number`.
    ///
    /// Typically, `number` will be the number of the latest known finalized block, and this is used
    /// to reset back to the finalized state.
    ///
    /// The first call to `next()` after calling this function should yield the L1 block _after_
    /// `number`, i.e. `number + 1`.
    fn reset(&mut self, number: u64) -> impl Send + Future<Output = ()>;
}

/// The information which must be stored in persistent storage.
#[derive(Clone, Debug, PartialEq)]
pub struct PersistentSnapshot {
    pub block: L1BlockId,
    pub timestamp: Timestamp,
    pub node_set: FullNodeSetSnapshot,
    pub wallets: WalletsSnapshot,
}

impl PersistentSnapshot {
    /// An empty genesis snapshot starting at the given L1 block.
    pub fn genesis(block: L1BlockId, timestamp: Timestamp) -> Self {
        Self {
            block,
            timestamp,
            node_set: FullNodeSetSnapshot {
                nodes: Default::default(),
                l1_block: L1BlockInfo {
                    number: block.number,
                    hash: block.hash,
                    timestamp,
                },
            },
            wallets: Default::default(),
        }
    }
}

/// Persistent storage for the L1 data.
pub trait L1Persistence: Send {
    /// Fetch the latest persisted snapshot.
    fn finalized_snapshot(&self)
    -> impl Send + Future<Output = Result<Option<PersistentSnapshot>>>;

    /// Apply changes to persistent storage up to the specified L1 block.
    fn apply_events(
        &self,
        block: L1BlockId,
        timestamp: Timestamp,
        node_set_diff: impl IntoIterator<Item = FullNodeSetDiff> + Send,
        wallets_diff: impl IntoIterator<Item = (Address, WalletDiff)> + Send,
    ) -> impl Send + Future<Output = Result<()>>;

    /// Save an initial snapshot to a previously empty database.
    fn save_genesis(&self, snapshot: PersistentSnapshot)
    -> impl Send + Future<Output = Result<()>>;
}

#[cfg(test)]
mod test {
    use std::{
        pin::Pin,
        task::{Context, Poll},
        time::{Duration, Instant},
    };

    use alloy::primitives::keccak256;
    use espresso_types::v0::validators_from_l1_events;
    use tagged_base64::TaggedBase64;
    use tide_disco::{Error as _, StatusCode};
    use tokio::{task::spawn, time::sleep};

    use crate::types::common::{Delegation, NodeSetEntry, Ratio};

    use super::*;

    /// Easy-setup storage that just uses memory.
    #[derive(Clone, Debug, Default)]
    struct MemoryStorage {
        snapshot: Arc<RwLock<Option<PersistentSnapshot>>>,
    }

    impl L1Persistence for MemoryStorage {
        async fn finalized_snapshot(&self) -> Result<Option<PersistentSnapshot>> {
            Ok(self.snapshot.read().await.clone())
        }

        async fn save_genesis(&self, snapshot: PersistentSnapshot) -> Result<()> {
            *self.snapshot.write().await = Some(snapshot);
            Ok(())
        }

        async fn apply_events(
            &self,
            block: L1BlockId,
            timestamp: Timestamp,
            node_set_diff: impl IntoIterator<Item = FullNodeSetDiff> + Send,
            wallets_diff: impl IntoIterator<Item = (Address, WalletDiff)> + Send,
        ) -> Result<()> {
            let mut lock = self.snapshot.write().await;
            let snapshot = lock.get_or_insert(PersistentSnapshot::genesis(block_id(0), 0));

            for diff in node_set_diff {
                apply_node_set_diff(&mut snapshot.node_set, &diff);
            }
            for (address, diff) in wallets_diff {
                apply_wallet_diff(&mut snapshot.wallets, address, &diff);
            }
            snapshot.block = block;
            snapshot.timestamp = timestamp;

            Ok(())
        }
    }

    /// Storage that always fails.
    #[derive(Clone, Copy, Debug, Default)]
    struct FailStorage;

    impl L1Persistence for FailStorage {
        async fn finalized_snapshot(&self) -> Result<Option<PersistentSnapshot>> {
            Err(Error::catch_all(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FailStorage".into(),
            ))
        }

        async fn save_genesis(&self, _snapshot: PersistentSnapshot) -> Result<()> {
            Err(Error::catch_all(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FailStorage".into(),
            ))
        }

        async fn apply_events(
            &self,
            _block: L1BlockId,
            _timestamp: Timestamp,
            _node_set_diff: impl IntoIterator<Item = FullNodeSetDiff> + Send,
            _wallets_diff: impl IntoIterator<Item = (Address, WalletDiff)> + Send,
        ) -> Result<()> {
            Err(Error::catch_all(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FailStorage".into(),
            ))
        }
    }

    /// Resettable stream that yields a predefined list of inputs.
    #[derive(Clone, Debug)]
    struct VecStream {
        inputs: Vec<BlockInput>,
        reorg: Option<Vec<BlockInput>>,
        pos: usize,
        panic_at_end: bool,
    }

    impl Default for VecStream {
        fn default() -> Self {
            Self {
                inputs: vec![],
                reorg: None,
                pos: 0,
                panic_at_end: true,
            }
        }
    }

    impl VecStream {
        /// Emulate an infinite stream.
        ///
        /// The resulting stream will block indefinitely when it reaches the end of its predefined
        /// input sequence.
        fn infinite() -> Self {
            Self {
                panic_at_end: false,
                ..Default::default()
            }
        }

        /// Append a new L1 block input to be yielded by the stream.
        fn push(&mut self, input: BlockInput) {
            self.inputs.push(input);
        }

        /// Provide an alternative sequence of inputs to yield after the stream is reset.
        fn with_reorg(mut self, inputs: Vec<BlockInput>) -> Self {
            self.reorg = Some(inputs);
            self
        }
    }

    impl Stream for VecStream {
        type Item = BlockInput;

        fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            if self.pos >= self.inputs.len() {
                if self.panic_at_end {
                    // Most tests expect to never hit the end of the predefined input sequence. If
                    // we do, panic and fail the test.
                    panic!("reached end of predefined input stream");
                } else {
                    // In some cases, we want to emulate the realistic behavior of an L1 stream,
                    // which is blocking when no more blocks are readily available.
                    tracing::warn!("reached end of predefined input stream, blocking indefinitely");
                    return Poll::Pending;
                }
            }

            let poll = Poll::Ready(Some(self.inputs[self.pos].clone()));
            self.pos += 1;
            poll
        }
    }

    impl ResettableStream for VecStream {
        async fn reset(&mut self, number: u64) {
            tracing::info!(number, "reset");
            self.pos = self
                .inputs
                .iter()
                .position(|input| input.block.number == number + 1)
                .unwrap_or_else(|| panic!("cannot reset to unknown block height {number}"));
            if let Some(reorg) = self.reorg.take() {
                self.inputs = reorg;
            }
        }
    }

    /// Generate a block ID for testing.
    fn block_id(number: u64) -> L1BlockId {
        let parent = keccak256(number.saturating_sub(1).to_le_bytes());
        let hash = keccak256(number.to_le_bytes());
        L1BlockId {
            number,
            hash,
            parent,
        }
    }

    /// Generate a test L1 block with no staking-related data.
    fn empty_block(number: u64) -> BlockData {
        let block = block_id(number);
        let timestamp = 12 * number;
        BlockData {
            block,
            timestamp,
            node_set: FullNodeSetSnapshot {
                nodes: Default::default(),
                l1_block: L1BlockInfo {
                    number,
                    hash: block.hash,
                    timestamp,
                },
            },
            node_set_update: Some(Default::default()),
            wallets: Default::default(),
            wallets_update: Some(Default::default()),
        }
    }

    /// Generate a test [`State`] from a list of L1 blocks.
    fn from_blocks<S: Default>(blocks: impl IntoIterator<Item = BlockData>) -> State<S> {
        let mut state = State::<S> {
            blocks: Default::default(),
            blocks_by_hash: Default::default(),
            storage: S::default(),
        };
        for block in blocks {
            state
                .blocks_by_hash
                .insert(block.block.hash, block.block.number);
            state.blocks.push(block);
        }
        state
    }

    /// Generate an arbitrary node for testing.
    fn make_node(i: usize) -> NodeSetEntry {
        let address = Address::random();
        let staking_key = TaggedBase64::new("KEY", &i.to_le_bytes()).unwrap();
        NodeSetEntry {
            address,
            staking_key,
            stake: i.try_into().unwrap(),
            commission: Ratio::new(5, 100),
        }
    }

    /// Generate an empty wallet snapshot for testing.
    fn empty_wallet(l1_block: L1BlockInfo) -> WalletSnapshot {
        WalletSnapshot {
            l1_block,
            nodes: Default::default(),
            pending_exits: Default::default(),
            pending_undelegations: Default::default(),
            claimed_rewards: Default::default(),
        }
    }

    /// Generate a random L1 input for testing.
    fn random_block_input(number: u64) -> BlockInput {
        BlockInput {
            block: block_id(number),
            finalized: block_id(0),
            timestamp: 12 * number,
            events: vec![], // TODO generate random events
        }
    }

    #[test_log::test]
    fn test_gc() {
        for finalized in 0..3 {
            tracing::info!(finalized, "test garbage collection");
            let blocks = (0..3).map(empty_block).collect::<Vec<_>>();
            let mut state = from_blocks::<MemoryStorage>(blocks.clone());

            state.garbage_collect(finalized as u64);

            assert_eq!(&state.blocks, &blocks[finalized..]);
            assert_eq!(state.blocks.len(), state.blocks_by_hash.len());
            for block in &state.blocks {
                assert_eq!(state.blocks_by_hash[&block.block.hash], block.block.number);
            }
        }
    }

    #[test_log::test]
    fn test_api_l1_block() {
        let block = empty_block(1);
        let state = from_blocks::<MemoryStorage>([block.clone()]);

        // Query for old block.
        let err = state.l1_block(0).unwrap_err();
        assert_eq!(err.status(), StatusCode::GONE);

        // Query for future block.
        let err = state.l1_block(2).unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        // Query for known block.
        assert_eq!(state.l1_block(1).unwrap(), block.block);
        assert_eq!(state.latest_l1_block(), block.block);
    }

    #[test_log::test]
    fn test_api_node_set() {
        let mut finalized = empty_block(0);
        finalized.node_set_update = None;

        let mut block = empty_block(1);
        let node = make_node(0);
        block.node_set.nodes.push_back(node.clone());
        block.node_set_update = Some(vec![FullNodeSetDiff::NodeUpdate(node)]);

        let state = from_blocks::<MemoryStorage>([finalized.clone(), block.clone()]);

        // Query for unknown block.
        let unknown = empty_block(2).block.hash;
        let err = state.full_node_set(unknown).unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
        let err = state.full_node_set_update(unknown).unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        // Query for deleted update.
        let err = state
            .full_node_set_update(finalized.block.hash)
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::GONE);

        // Query for known block.
        assert_eq!(
            state.full_node_set(block.block.hash).unwrap(),
            block.node_set
        );
        let update = state.full_node_set_update(block.block.hash).unwrap();
        assert_eq!(&update.diff, block.node_set_update.as_ref().unwrap());
        assert_eq!(update.l1_block, block.block_info());
    }

    #[test_log::test]
    fn test_api_wallet() {
        let mut finalized = empty_block(0);
        finalized.wallets_update = None;

        let address = Address::random();
        let delegation = Delegation {
            delegator: address,
            node: Address::random(),
            amount: Default::default(),
            effective: Default::default(),
        };
        let wallet = WalletSnapshot {
            nodes: vec![delegation].into(),
            ..empty_wallet(finalized.block_info())
        };

        let mut block = empty_block(1);
        block.wallets.insert(address, wallet.clone());
        block
            .wallets_update
            .as_mut()
            .unwrap()
            .insert(address, vec![WalletDiff::DelegatedToNode(delegation)]);
        // Let `address` be known even in the finalized snapshot, so we can test queries for a known
        // wallet in a block whose update field has been deleted.
        finalized.wallets = block.wallets.clone();
        // Insert a second wallet that is not updated by this block, so we can test queries for
        // updates for a known wallet with no non-trivial update.
        let not_updated = Address::random();
        let not_updated_wallet = empty_wallet(finalized.block_info());
        block
            .wallets
            .insert(not_updated, not_updated_wallet.clone());

        let state = from_blocks::<MemoryStorage>([finalized.clone(), block.clone()]);

        // Query for unknown block.
        let unknown = empty_block(2).block.hash;
        let err = state.wallet(address, unknown).unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
        let err = state.wallet_update(address, unknown).unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        // Query for unknown address.
        let err = state
            .wallet(Address::random(), block.block.hash)
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
        let err = state
            .wallet_update(Address::random(), block.block.hash)
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        // Query for known address with no updates.
        assert_eq!(
            state.wallet(not_updated, block.block.hash).unwrap(),
            not_updated_wallet
        );
        assert_eq!(
            state.wallet_update(not_updated, block.block.hash).unwrap(),
            WalletUpdate {
                l1_block: block.block_info(),
                diff: vec![]
            }
        );

        // Query for deleted update.
        let err = state
            .wallet_update(address, finalized.block.hash)
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::GONE);

        // Query for known wallet.
        assert_eq!(state.wallet(address, block.block.hash).unwrap(), wallet);
        let update = state.wallet_update(address, block.block.hash).unwrap();
        assert_eq!(&update.diff, &[WalletDiff::DelegatedToNode(delegation)]);
        assert_eq!(update.l1_block, block.block_info());
    }

    #[test_log::test]
    fn test_replay_consistency() {
        let mut block = empty_block(0);
        let mut events = vec![];

        let inputs = (0..100).map(random_block_input);
        for input in inputs {
            // Compute the full node set snapshot as the staking UI service would do it.
            block = block.next(&input);

            // Compute the Espresso validator set as the protocol does it.
            events.extend(input.events.iter().filter_map(|event| match event {
                L1Event::StakeTable(ev) => Some(ev.as_ref().clone()),
                _ => None,
            }));
            let validators = validators_from_l1_events(events.iter().cloned()).unwrap().0;
            assert_eq!(validators.len(), block.node_set.nodes.len());
            for node in &block.node_set.nodes {
                let validator = &validators[&node.address];
                assert_eq!(node.address, validator.account);
                assert_eq!(
                    node.commission,
                    Ratio::new(validator.commission.into(), 10_000),
                );
                assert_eq!(node.stake, validator.stake);
                assert_eq!(node.staking_key, validator.stake_table_key.into());
            }
        }
    }

    #[test_log::test]
    fn test_large_state() {
        let mut block = empty_block(0);

        // Realistically large state: 500 registered validators, 10000 delegators, each delegating
        // to 10 different nodes.
        for i in 0..500 {
            block.node_set.nodes.push_back(make_node(i));
        }
        for i in 0..10_000 {
            let delegator = Address::random();
            let mut wallet = empty_wallet(block.block_info());
            for j in 0..10 {
                let node = block.node_set.nodes[(i + j) % block.node_set.nodes.len()].address;
                let delegation = Delegation {
                    delegator,
                    node,
                    amount: 1.try_into().unwrap(),
                    effective: Default::default(),
                };
                wallet.nodes.push_back(delegation);
            }
            block.wallets.insert(delegator, wallet);
        }

        // Apply random events. It should take on average no more than 12 seconds (although in
        // practice it should be much less), since that is how long we have to process an L1 block
        // in the real world.
        let inputs = (0..100).map(random_block_input);
        let start = Instant::now();
        for input in inputs {
            block = block.next(&input);
        }
        let elapsed = start.elapsed();
        tracing::info!(?elapsed, avg_duration = ?elapsed / 100, "processed 100 inputs");
        assert!(elapsed / 100 < Duration::from_secs(12));
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_finalize_success() {
        let inputs = (1..3).map(random_block_input).collect::<Vec<_>>();
        let stream = &mut VecStream::default();

        // Start with just the finalized state.
        let state = RwLock::new(from_blocks::<MemoryStorage>([empty_block(0)]));

        // Apply blocks without finalizing a new block.
        for input in &inputs {
            State::handle_new_head(state.upgradable_read().await, stream, input)
                .await
                .unwrap();
        }

        // Apply a finalized block.
        let mut finalized = random_block_input(3);
        finalized.finalized = block_id(2);
        State::handle_new_head(state.upgradable_read().await, stream, &finalized)
            .await
            .unwrap();

        // Check that the new block has been added and state has been garbage collected.
        let state = state.read().await;
        assert_eq!(state.blocks.len(), 2);
        assert_eq!(state.blocks[0].block, block_id(2));
        assert_eq!(state.blocks[1].block, block_id(3));
        assert_eq!(state.blocks_by_hash.len(), 2);
        assert_eq!(
            state.blocks_by_hash[&finalized.finalized.hash],
            finalized.finalized.number
        );
        assert_eq!(
            state.blocks_by_hash[&finalized.block.hash],
            finalized.block.number
        );

        // Check that the finalized snapshot has been persisted.
        assert_eq!(
            state
                .storage
                .finalized_snapshot()
                .await
                .unwrap()
                .unwrap()
                .block,
            finalized.finalized
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_finalize_storage_failure() {
        let inputs = (1..3).map(random_block_input).collect::<Vec<_>>();
        let stream = &mut VecStream::default();

        // Start with just the finalized state.
        let state = RwLock::new(from_blocks::<FailStorage>([empty_block(0)]));

        // Apply blocks without finalizing a new block.
        for input in &inputs {
            State::handle_new_head(state.upgradable_read().await, stream, input)
                .await
                .unwrap();
        }

        // Apply a finalized block. Storing the finalized snapshot will fail, and on failure the
        // state should not be modified.
        let initial_state = { state.read().await.clone() };
        let mut finalized = random_block_input(3);
        finalized.finalized = block_id(2);
        State::handle_new_head(state.upgradable_read().await, stream, &finalized)
            .await
            .unwrap_err();
        let final_state = state.read().await;
        assert_eq!(initial_state.blocks, final_state.blocks);
        assert_eq!(initial_state.blocks_by_hash, final_state.blocks_by_hash);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_subscribe_happy_path() {
        let mut stream = VecStream::infinite();
        for i in 1..3 {
            stream.push(random_block_input(i));
        }

        // Start with just the finalized state.
        let state = Arc::new(RwLock::new(from_blocks::<MemoryStorage>([empty_block(0)])));

        // Process the updates in `stream`.
        let task = spawn(State::subscribe(state.clone(), stream));

        // Wait for the processing task to catch up.
        let state = loop {
            sleep(Duration::from_millis(100)).await;
            let state = state.read().await;
            if state.blocks.len() >= 3 {
                task.abort();
                break state;
            }
        };
        assert_eq!(state.blocks.len(), 3);
        assert_eq!(state.blocks_by_hash.len(), 3);
        for i in 0..3 {
            assert_eq!(state.blocks[i as usize].block, block_id(i));
            assert_eq!(state.blocks_by_hash[&block_id(i).hash], i);
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_subscribe_reorg_head() {
        let inputs = (1..5).map(random_block_input).collect::<Vec<_>>();

        let mut stream = VecStream::infinite();
        stream.push(inputs[0].clone());
        // For the second input, push something with the wrong hash, so that on the following input,
        // we will detect the reorg.
        let mut uncle = inputs[1].clone();
        uncle.block.hash = block_id(1000).hash;
        stream.push(uncle);
        // One more input will trigger the reorg handling when we find that the parent hash of this
        // block does not match the hash of the previous block.
        stream.push(inputs[2].clone());

        // Provide the correct sequence of blocks after reorging.
        stream = stream.with_reorg(inputs);

        // Start with just the finalized state.
        let state = Arc::new(RwLock::new(from_blocks::<MemoryStorage>([empty_block(0)])));

        // Process the updates in `stream`.
        let task = spawn(State::subscribe(state.clone(), stream));

        // Wait for the processing task to catch up.
        let state = loop {
            sleep(Duration::from_millis(100)).await;
            let state = state.read().await;
            if state.blocks.len() >= 5 {
                task.abort();
                break state;
            }
        };
        assert_eq!(state.blocks.len(), 5);
        assert_eq!(state.blocks_by_hash.len(), 5);
        for i in 0..5 {
            assert_eq!(state.blocks[i as usize].block, block_id(i));
            assert_eq!(state.blocks_by_hash[&block_id(i).hash], i);
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_subscribe_reorg_finalized() {
        let mut inputs = (1..5).map(random_block_input).collect::<Vec<_>>();

        let mut stream = VecStream::infinite();
        stream.push(inputs[0].clone());
        stream.push(inputs[1].clone());
        // The last input causes the first input to become finalized, but with a different hash than
        // we originally saw.
        inputs[2].finalized.number = 1;
        let finalized_hash = block_id(1000).hash;
        inputs[2].finalized.hash = finalized_hash;
        stream.push(inputs[2].clone());

        // This will causes us to reorg back to the original finalized block 0, after which we
        // produce a block stream consistent with the now-finalized hash.
        inputs[0].block.hash = finalized_hash;
        inputs[1].block.parent = finalized_hash;
        stream = stream.with_reorg(inputs.clone());

        // Start with just the genesis state.
        let state = Arc::new(RwLock::new(from_blocks::<MemoryStorage>([empty_block(0)])));

        // Process the updates in `stream`.
        let task = spawn(State::subscribe(state.clone(), stream));

        // Wait for the processing task to catch up.
        let state = loop {
            sleep(Duration::from_millis(100)).await;
            let state = state.read().await;
            if state.blocks.len() >= 4 {
                task.abort();
                break state;
            }
        };

        // We should have garbage collected block 0 after block 1 became finalized.
        assert_eq!(state.blocks.len(), 4);
        assert_eq!(state.blocks_by_hash.len(), 4);
        for (input, block) in inputs.iter().zip(&state.blocks) {
            assert_eq!(block.block, input.block);
            assert_eq!(state.blocks_by_hash[&input.block.hash], input.block.number);
        }

        // Block 1 should have become finalized.
        assert_eq!(
            state
                .storage
                .finalized_snapshot()
                .await
                .unwrap()
                .unwrap()
                .block,
            inputs[0].block
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_restart() {
        let mut stream = VecStream::infinite();
        stream.push(random_block_input(1));
        // Second block finalizes the first block.
        let mut block_2 = random_block_input(2);
        block_2.finalized = block_id(1);
        stream.push(block_2);

        // Start up and run until block 1 is finalized.
        let genesis = PersistentSnapshot::genesis(block_id(0), 0);
        let storage = MemoryStorage::default();
        let state = Arc::new(RwLock::new(
            State::new(storage.clone(), genesis.clone()).await.unwrap(),
        ));
        // Initializing the state should cause a genesis snapshot to be saved.
        assert_eq!(
            storage.finalized_snapshot().await.unwrap().unwrap(),
            genesis
        );
        let task = spawn(State::subscribe(state.clone(), stream));
        let state = loop {
            sleep(Duration::from_millis(100)).await;
            let state = state.read().await;
            if state.blocks[0].block.number == 1 {
                task.abort();
                break state;
            }
        };
        assert_eq!(state.blocks.len(), 2);
        assert_eq!(
            storage.finalized_snapshot().await.unwrap().unwrap().block,
            block_id(1)
        );
        drop(state);

        // Restart and check that we reload the finalized snapshot (and don't use the genesis, for
        // which we will pass in some nonsense).
        let genesis = PersistentSnapshot::genesis(block_id(1000), 12_000);
        let state = State::new(storage.clone(), genesis).await.unwrap();
        assert_eq!(state.blocks.len(), 1);
        assert_eq!(state.blocks[0].block, block_id(1));

        // Subscribe to new blocks starting from where we left off (with block 1 being finalized,
        // the next block would be block 2).
        let mut stream = VecStream::infinite();
        stream.push(random_block_input(2));
        let state = Arc::new(RwLock::new(state));
        let task = spawn(State::subscribe(state.clone(), stream));
        let state = loop {
            sleep(Duration::from_millis(100)).await;
            let state = state.read().await;
            if state.blocks.len() >= 2 {
                task.abort();
                break state;
            }
        };
        assert_eq!(state.blocks.len(), 2);
        assert_eq!(state.blocks[0].block, block_id(1));
        assert_eq!(state.blocks[1].block, block_id(2));
    }
}
