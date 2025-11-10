//! Input data from L1.

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use alloy::primitives::BlockHash;
use async_lock::{RwLock, RwLockUpgradableReadGuard, RwLockWriteGuard};
use derive_more::{Deref, DerefMut};
use espresso_types::{PubKey, v0_3::COMMISSION_BASIS_POINTS};
use futures::stream::{Stream, StreamExt};
use hotshot_contract_adapter::sol_types::{
    RewardClaim::RewardClaimEvents,
    StakeTableV2::{StakeTableV2Events, ValidatorRegistered, ValidatorRegisteredV2},
};
use hotshot_types::light_client::StateVerKey;
use tracing::instrument;

use crate::{
    error::{Error, Result, ensure},
    types::{
        common::{
            Address, Delegation, ESPTokenAmount, L1BlockId, L1BlockInfo, NodeExit, NodeSetEntry,
            PendingWithdrawal, Ratio, Timestamp,
        },
        global::{FullNodeSetDiff, FullNodeSetSnapshot, FullNodeSetUpdate},
        wallet::{WalletDiff, WalletSnapshot, WalletUpdate},
    },
};

pub mod options;
pub mod provider;
mod rpc_stream;
pub mod switching_transport;

pub use rpc_stream::RpcStream;
pub mod testing;

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
    pub async fn new(storage: S, genesis: Snapshot) -> Result<Self> {
        let state = match storage.finalized_snapshot().await? {
            Some(snapshot) => {
                tracing::info!(?snapshot.block, "starting from saved snapshot");
                snapshot
            }
            None => {
                tracing::info!(?genesis.block, "starting from genesis");
                storage.save_genesis(genesis.clone()).await?;
                genesis
            }
        };

        let finalized = BlockData {
            state,

            // The updates are the difference between this snapshot and the previous one; for the
            // initial snapshot, there is no such difference.
            node_set_update: None,
            wallets_update: None,
        };
        let blocks_by_hash = [(finalized.block().hash(), finalized.block().number())]
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
        self.blocks[self.blocks.len() - 1].block().id()
    }

    /// Get the hashes identifying a particular L1 block.
    pub fn l1_block(&self, number: u64) -> Result<L1BlockId> {
        Ok(self.block(number)?.block().id())
    }

    /// Get a snapshot of the full node set at the given L1 block.
    ///
    /// # Performance
    ///
    /// This method returns a full node set, plus information about the L1 block where that node set
    /// was snapshotted. This can be converted into an API-facing [`FullNodeSetSnapshot`] using
    /// [`NodeSet::into_snapshot`]. This conversion is not performed within this method because it
    /// may be relatively expensive, while this method as-is is very efficient. It is recommended to
    /// call this method, then drop the reference or lock on the [`State`] before performing the
    /// conversion.
    pub fn full_node_set(&self, hash: BlockHash) -> Result<(NodeSet, L1BlockInfo)> {
        let number = self.block_number(hash)?;
        let block = self.block(number)?;
        Ok((block.state.node_set.clone(), block.block().info()))
    }

    /// Get the update applied to the full node set due to the given L1 block.
    pub fn full_node_set_update(&self, hash: BlockHash) -> Result<FullNodeSetUpdate> {
        let number = self.block_number(hash)?;
        let block = self.block(number)?;
        let diff = block.node_set_update.clone().ok_or_else(Error::gone)?;
        let update = FullNodeSetUpdate {
            l1_block: block.block().info(),
            diff,
        };
        Ok(update)
    }

    /// Get a snapshot of the requested wallet state at the given L1 block.
    ///
    /// # Performance
    ///
    /// This method returns a full wallet snapshot, plus information about the L1 block where the
    /// snapshot was taken. This can be converted into an API-facing [`WalletSnapshot`] using
    /// [`Wallet::into_snapshot`]. This conversion is not performed within this method because it
    /// may be relatively expensive, while this method as-is is very efficient. It is recommended to
    /// call this method, then drop the reference or lock on the [`State`] before performing the
    /// conversion.
    pub fn wallet(&self, address: Address, hash: BlockHash) -> Result<(Wallet, L1BlockInfo)> {
        let number = self.block_number(hash)?;
        let block = self.block(number)?;
        let wallet = block
            .state
            .wallets
            .get(&address)
            .ok_or_else(|| Error::not_found().context(format!("unknown account {address}")))?
            .clone();
        Ok((wallet, block.block().info()))
    }

    /// Get the update applied to the requested wallet due to the given L1 block.
    pub fn wallet_update(&self, address: Address, hash: BlockHash) -> Result<WalletUpdate> {
        let number = self.block_number(hash)?;
        let block = self.block(number)?;

        // Check that this account even exists.
        ensure!(
            block.state.wallets.contains_key(&address),
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
            l1_block: block.block().info(),
            diff,
        })
    }

    /// Get the block with the requested block number.
    fn block(&self, number: u64) -> Result<&BlockData> {
        let finalized = self.blocks[0].block().number();
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
    pub async fn subscribe(
        state: Arc<RwLock<Self>>,
        mut stream: impl ResettableStream,
    ) -> Result<()> {
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
        Err(Error::internal().context("L1 block stream ended unexpectedly"))
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
        let prev = state.blocks[state.blocks.len() - 1].block();
        if head.block.number != prev.number() + 1 || head.block.parent != prev.hash() {
            tracing::warn!(?head, ?prev, "new head does not extend previous head");
            let mut state = RwLockUpgradableReadGuard::upgrade(state).await;
            state.reorg(stream).await;
            return Ok(());
        }

        // Handle a new finalized block if there is one.
        let old_finalized = state.blocks[0].block().number();
        let new_finalized = head.finalized;
        ensure!(
            new_finalized.number <= head.block.number,
            Error::internal().context(format!(
                "stream yielded a finalized block from the future: \
                    finalized {new_finalized:?}, head {:?}",
                head.block
            ))
        );
        if new_finalized.number > old_finalized {
            let mut write_state = RwLockUpgradableReadGuard::upgrade(state).await;

            // Make sure the hash of this new finalized block matches what we have in our
            // unfinalized state, if not there has somehow been a reorg of this (presumably pretty
            // old) block, and we need to go back and re-process it.
            let offset = new_finalized.number - old_finalized;
            let expected_hash = write_state.blocks[offset as usize].block().hash();
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
            .insert(new_block.block().hash(), new_block.block().number());
        state.blocks.push(new_block);

        Ok(())
    }

    /// Reset the state back to the last persisted finalized state.
    async fn reorg(&mut self, stream: &mut impl ResettableStream) {
        tracing::warn!("reorg detected, resetting to finalized state");
        stream.reset(self.blocks[0].block().number()).await;
        self.blocks.truncate(1);
        self.blocks_by_hash = [(
            self.blocks[0].block().hash(),
            self.blocks[0].block().number(),
        )]
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
            if block.block().number() > finalized {
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
            .apply_events(finalized_info.block(), nodes_set_diff, wallets_diff)
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
        let offset = (finalized - self.blocks[0].block().number()) as usize;
        self.blocks.rotate_left(offset);
        // Remove the old blocks, and also their entries in `blocks_by_hash`.
        for _ in 0..offset {
            let block = self.blocks.pop().unwrap();
            self.blocks_by_hash.remove(&block.block().hash());
        }
    }
}

/// Snapshots and updates for each L1 block.
#[derive(Clone, Debug, PartialEq)]
struct BlockData {
    state: Snapshot,

    /// The change to the full node set between the previous L1 block and this one.
    ///
    /// This may be [`None`] when the previous L1 block has been garbage collected.
    node_set_update: Option<Vec<FullNodeSetDiff>>,

    /// The changes to each wallet between the previous L1 block and this one.
    ///
    /// Unlike [`Wallets`] snapshots, we don't benefit from using an immutable map here, as updates
    /// are always unique to a block, not partially shared with previous blocks. However, the good
    /// news is that only a few if any wallets will have non-trivial updates in any given block, so
    /// the size of this set should be small in practice. We use a [`BTreeMap`] to further compress
    /// the size of this set, as B-Trees should generally have a better memory footprint than hash
    /// maps.
    ///
    /// If a wallet was not updated in a given block, it will not have an entry here, but a trivial
    /// [`WalletUpdate`] can always be generated for any account by combining the [`L1BlockInfo`]
    /// with an empty [`WalletDiff`] list.
    ///
    /// This may be [`None`] when the previous L1 block has been garbage collected.
    wallets_update: Option<BTreeMap<Address, Vec<WalletDiff>>>,
}

impl BlockData {
    /// Get the [`BlockData`] that follows from this one after applying the next L1 block.
    fn next(&self, input: &BlockInput) -> Self {
        // Cloning entire state in order to apply updates and get the new state. This is cheap
        // thanks to the magic of immutable data structures.
        let mut state = self.state.clone();
        let (node_set_update, wallets_update) = state.apply(input);
        Self {
            state,
            node_set_update: Some(node_set_update),
            wallets_update: Some(wallets_update),
        }
    }

    fn block(&self) -> L1BlockSnapshot {
        self.state.block
    }
}

/// Data we capture about each L1 block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct L1BlockSnapshot {
    /// The L1 block.
    pub id: L1BlockId,

    /// The L1 block timestamp.
    pub timestamp: Timestamp,

    /// The exit escrow period in the stake table contract as of this L1 block.
    pub exit_escrow_period: u64,
}

impl L1BlockSnapshot {
    pub fn id(&self) -> L1BlockId {
        self.id
    }

    pub fn number(&self) -> u64 {
        self.id.number
    }

    pub fn hash(&self) -> BlockHash {
        self.id.hash
    }

    pub fn parent(&self) -> BlockHash {
        self.id.parent
    }

    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }

    pub fn info(&self) -> L1BlockInfo {
        L1BlockInfo {
            number: self.id.number,
            hash: self.id.hash,
            timestamp: self.timestamp,
        }
    }
}

/// A snapshot of the L1-derived state after a certain L1 block.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    /// The L1 block.
    pub block: L1BlockSnapshot,

    /// The full node set as of this L1 block.
    pub node_set: NodeSet,

    /// The state of each wallet as of this L1 block.
    pub wallets: Wallets,
}

impl Snapshot {
    /// An empty snapshot starting at the given L1 block.
    pub fn empty(block: L1BlockSnapshot) -> Self {
        Self::new(block, Default::default(), Default::default())
    }

    /// A snapshot from an L1 block snapshot and snapshots of the node set and wallet state.
    pub fn new(block: L1BlockSnapshot, node_set: NodeSet, wallets: Wallets) -> Self {
        Self {
            block,
            node_set,
            wallets,
        }
    }

    /// Transform the state snapshot by applying a new L1 block.
    ///
    /// Returns the updates that were made while applying the block.
    fn apply(
        &mut self,
        input: &BlockInput,
    ) -> (Vec<FullNodeSetDiff>, BTreeMap<Address, Vec<WalletDiff>>) {
        // Keep track of changes we apply.
        let mut node_set_update = vec![];
        let mut wallets_update = BTreeMap::<Address, Vec<WalletDiff>>::default();

        // Update the L1 block information.
        self.block = L1BlockSnapshot {
            id: input.block,
            timestamp: input.timestamp,
            // The exit escrow period doesn't change unless we get an event saying so.
            exit_escrow_period: self.block.exit_escrow_period,
        };

        for event in &input.events {
            // Convert contract events into either node set or wallet updates, and apply updates
            // to state snapshots.
            let (nodes_diff, wallets_diff) = self.handle_event(input, event);
            for diff in nodes_diff {
                self.node_set.apply(&diff);
                node_set_update.push(diff);
            }
            for (address, diff) in wallets_diff {
                self.wallets.apply(address, &diff);
                wallets_update.entry(address).or_default().push(diff);
            }
        }

        (node_set_update, wallets_update)
    }

    /// Handle a single L1 event from a given L1 block.
    ///
    /// Updates the block state accordingly and returns the diffs that should be applied to the rest
    /// of the state.
    #[tracing::instrument(skip(self))]
    fn handle_event(
        &mut self,
        input: &BlockInput,
        event: &L1Event,
    ) -> (Vec<FullNodeSetDiff>, Vec<(Address, WalletDiff)>) {
        tracing::debug!("processing L1 event");
        match event {
            L1Event::StakeTable(ev) => match ev.as_ref() {
                StakeTableV2Events::ValidatorRegisteredV2(ev) => {
                    if let Err(err) = ev.authenticate() {
                        // The contract doesn't check all the signatures, so it's possible that an
                        // invalid event got through. We can safely just ignore it as the consensus
                        // protocol will do the same.
                        tracing::warn!("got invalid ValidatorRegisteredV2 event: {err:#}");
                        return Default::default();
                    }
                    (vec![FullNodeSetDiff::NodeUpdate(ev.into())], vec![])
                }
                StakeTableV2Events::ValidatorRegistered(ev) => {
                    tracing::warn!("received legacy ValidatorRegistered event");
                    (vec![FullNodeSetDiff::NodeUpdate(ev.into())], vec![])
                }
                StakeTableV2Events::ExitEscrowPeriodUpdated(ev) => {
                    // This should be quite rare, we can be a little loud about it.
                    tracing::warn!(
                        old = self.block.exit_escrow_period,
                        new = ev.newExitEscrowPeriod,
                        "updating exit escrow period",
                    );
                    self.block.exit_escrow_period = ev.newExitEscrowPeriod;

                    // Apart from changing our per-block state, this event does not have any effect
                    // on the node set or the wallets.
                    (vec![], vec![])
                }
                StakeTableV2Events::ValidatorExit(ev) => {
                    if !self.node_set.contains_key(&ev.validator) {
                        // This should not happen, but if we somehow see a validator exit event for
                        // a non-existent validator:
                        // * don't generate an invalid diff to pass along to the front end
                        // * complain loudly because it probably means our node set is out of sync
                        //   with the contract
                        tracing::error!(%ev.validator, "got ValidatorExit event for non-existent validator");
                        return Default::default();
                    }

                    let diff = FullNodeSetDiff::NodeExit(NodeExit {
                        address: ev.validator,
                        exit_time: input.timestamp + self.block.exit_escrow_period,
                    });

                    // TODO we should also emit a `WalletDiff::NodeExited` for each wallet that is
                    // delegated to the exiting validator.
                    (vec![diff], vec![])
                }
                StakeTableV2Events::ConsensusKeysUpdated(_) => {
                    tracing::error!(
                        "not implemented event handling for StakeTableV2Events::ConsensusKeysUpdated"
                    );
                    (vec![], vec![])
                }
                StakeTableV2Events::ConsensusKeysUpdatedV2(_) => {
                    tracing::error!(
                        "not implemented event handling for StakeTableV2Events::ConsensusKeysUpdatedV2"
                    );
                    (vec![], vec![])
                }
                StakeTableV2Events::CommissionUpdated(_) => {
                    tracing::error!(
                        "not implemented event handling for StakeTableV2Events::CommissionUpdated"
                    );
                    (vec![], vec![])
                }
                StakeTableV2Events::Delegated(_) => {
                    tracing::error!(
                        "not implemented event handling for StakeTableV2Events::Delegated"
                    );
                    (vec![], vec![])
                }
                StakeTableV2Events::Undelegated(_) => {
                    tracing::error!(
                        "not implemented event handling for StakeTableV2Events::Undelegated"
                    );
                    (vec![], vec![])
                }
                StakeTableV2Events::Withdrawal(_) => {
                    tracing::error!(
                        "not implemented event handling for StakeTableV2Events::Withdrawal"
                    );
                    (vec![], vec![])
                }
                // These events are not relevant to this service. We still list them out explicitly
                // (rather than matching on _) so that it is clear that we are not missing any
                // important events, and if new event types are added, the compiler will force us to
                // handle them explicitly.
                StakeTableV2Events::MaxCommissionIncreaseUpdated(_)
                | StakeTableV2Events::MinCommissionUpdateIntervalUpdated(_)
                | StakeTableV2Events::OwnershipTransferred(_)
                | StakeTableV2Events::Paused(_)
                | StakeTableV2Events::Unpaused(_)
                | StakeTableV2Events::Initialized(_)
                | StakeTableV2Events::RoleAdminChanged(_)
                | StakeTableV2Events::RoleGranted(_)
                | StakeTableV2Events::RoleRevoked(_)
                | StakeTableV2Events::Upgraded(_) => {
                    tracing::debug!("skipping irrelevant event");
                    (vec![], vec![])
                }
            },
            L1Event::Reward(_) => {
                // TODO
                (vec![], vec![])
            }
        }
    }
}

/// The state of every wallet.
///
/// Note that while this set can, in concept, become very large, the use of immutable data
/// structures means that the memory for all entries is shared with the previous block's
/// snapshot, excepting only those wallets whose state has actually changed in this L1 block.
/// This will be a small number of wallets (often 0!) in practice.
#[derive(Clone, Debug, Default, PartialEq, Deref, DerefMut)]
pub struct Wallets(im::HashMap<Address, Wallet>);

impl Wallets {
    /// Mutate the state by applying a diff to the indicated account.
    pub fn apply(&mut self, _address: Address, _diff: &WalletDiff) {
        // TODO
    }
}

/// State tracked for each wallet.
///
/// This is a persistent data structure, meaning the [`Clone`] implementation is very cheap, even
/// for large wallets (e.g. those with many delegations), and partial changes can be made in a lazy,
/// copy-on-write fashion. This is important for performance when dealing with many large wallets
/// and managing different state snapshots for different points in history.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Wallet {
    /// Nodes that this user is delegating to.
    pub nodes: im::Vector<Delegation>,

    /// Stake that has been undelegated but not yet withdrawn.
    pub pending_undelegations: im::Vector<PendingWithdrawal>,

    /// Stake previously delegated to nodes that have exited.
    pub pending_exits: im::Vector<PendingWithdrawal>,

    /// Total amount of rewards ever claimed from the contract.
    pub claimed_rewards: ESPTokenAmount,
}

impl Wallet {
    /// Convert this wallet into the public API representation of a wallet snapshot.
    pub fn into_snapshot(self, l1_block: L1BlockInfo) -> WalletSnapshot {
        WalletSnapshot {
            nodes: self.nodes.into_iter().collect(),
            pending_undelegations: self.pending_undelegations.into_iter().collect(),
            pending_exits: self.pending_exits.into_iter().collect(),
            claimed_rewards: self.claimed_rewards,
            l1_block,
        }
    }

    /// Mutate this wallet by applying a diff.
    pub fn apply(&mut self, _diff: &WalletDiff) {
        // TODO
    }
}

/// State tracked for the full node set.
///
/// This is a persistent data structure, meaning the [`Clone`] implementation is very cheap, even
/// for large sets, and partial changes can be made in a lazy, copy-on-write fashion. This is
/// important for performance when managing different state snapshots for different points in
/// history.
#[derive(Clone, Debug, Default, PartialEq, Deref, DerefMut)]
pub struct NodeSet(im::OrdMap<Address, NodeSetEntry>);

impl NodeSet {
    /// Convert this node set into the public API representation of a node set snapshot.
    pub fn into_snapshot(self, l1_block: L1BlockInfo) -> FullNodeSetSnapshot {
        FullNodeSetSnapshot {
            nodes: self.0.values().cloned().collect(),
            l1_block,
        }
    }

    /// Mutate this node set by applying a diff.
    pub fn apply(&mut self, diff: &FullNodeSetDiff) {
        match diff {
            FullNodeSetDiff::NodeUpdate(node) => {
                self.insert(node.address, node.clone());
            }
            FullNodeSetDiff::NodeExit(node) => {
                self.remove(&node.address);
            }
        }
    }

    /// Add a node to the set.
    pub fn push(&mut self, node: NodeSetEntry) {
        self.0.insert(node.address, node);
    }
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
    StakeTable(Arc<StakeTableV2Events>),
}

impl From<RewardClaimEvents> for L1Event {
    fn from(event: RewardClaimEvents) -> Self {
        Self::Reward(Arc::new(event))
    }
}

impl From<StakeTableV2Events> for L1Event {
    fn from(event: StakeTableV2Events) -> Self {
        Self::StakeTable(Arc::new(event))
    }
}

impl From<&ValidatorRegisteredV2> for NodeSetEntry {
    fn from(e: &ValidatorRegisteredV2) -> Self {
        // Downgrade the event: apart from authentication, the V2 event contains the same
        // information as the V1 event, and can be converted to a node the same way.
        let e = ValidatorRegistered {
            account: e.account,
            blsVk: e.blsVK,
            commission: e.commission,
            schnorrVk: e.schnorrVK,
        };
        (&e).into()
    }
}

impl From<&ValidatorRegistered> for NodeSetEntry {
    fn from(e: &ValidatorRegistered) -> Self {
        NodeSetEntry {
            address: e.account,
            staking_key: PubKey::from(e.blsVk).into(),
            state_key: StateVerKey::from(e.schnorrVk).into(),
            commission: Ratio::new(e.commission as usize, COMMISSION_BASIS_POINTS as usize),
            // All nodes start with 0 stake, a separate delegation event will be generated when
            // someone delegates non-zero stake to a node.
            stake: ESPTokenAmount::ZERO,
        }
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

/// Persistent storage for the L1 data.
pub trait L1Persistence: Send {
    /// Fetch the latest persisted snapshot.
    fn finalized_snapshot(&self) -> impl Send + Future<Output = Result<Option<Snapshot>>>;

    /// Apply changes to persistent storage up to the specified L1 block.
    fn apply_events(
        &self,
        block: L1BlockSnapshot,
        node_set_diff: impl IntoIterator<Item = FullNodeSetDiff> + Send,
        wallets_diff: impl IntoIterator<Item = (Address, WalletDiff)> + Send,
    ) -> impl Send + Future<Output = Result<()>>;

    /// Save an initial snapshot to a previously empty database.
    fn save_genesis(&self, snapshot: Snapshot) -> impl Send + Future<Output = Result<()>>;
}

#[cfg(test)]
mod test {
    use std::time::{Duration, Instant};

    use espresso_types::{StakeTableState, v0_3::StakeTableEvent};
    use hotshot_contract_adapter::sol_types::StakeTableV2::{
        ExitEscrowPeriodUpdated, ValidatorExit,
    };
    use tide_disco::{Error as _, StatusCode};

    use super::{
        testing::{FailStorage, MemoryStorage, VecStream, block_id, make_node},
        *,
    };

    use crate::{
        input::l1::testing::{
            EventGenerator, InputGenerator, block_snapshot, subscribe_until,
            validator_registered_event,
        },
        types::common::{Delegation, Ratio},
    };

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
                .insert(block.block().hash(), block.block().number());
            state.blocks.push(block);
        }
        state
    }

    /// Check consistency between the [`NodeSet`] used by this service and the [`StakeTableState`]
    /// used by consensus.
    fn check_stake_table_consistency(nodes: &NodeSet, stake_table: &StakeTableState) {
        tracing::debug!("checking state consistency");
        let validators = stake_table.validators();
        assert_eq!(validators.len(), nodes.len());
        for node in nodes.values() {
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

    #[test_log::test]
    fn test_gc() {
        for finalized in 0..3 {
            tracing::info!(finalized, "test garbage collection");
            let blocks = (0..3).map(BlockData::empty).collect::<Vec<_>>();
            let mut state = from_blocks::<MemoryStorage>(blocks.clone());

            state.garbage_collect(finalized as u64);

            assert_eq!(&state.blocks, &blocks[finalized..]);
            assert_eq!(state.blocks.len(), state.blocks_by_hash.len());
            for block in &state.blocks {
                assert_eq!(
                    state.blocks_by_hash[&block.block().hash()],
                    block.block().number()
                );
            }
        }
    }

    #[test_log::test]
    fn test_api_l1_block() {
        let block = BlockData::empty(1);
        let state = from_blocks::<MemoryStorage>([block.clone()]);

        // Query for old block.
        let err = state.l1_block(0).unwrap_err();
        assert_eq!(err.status(), StatusCode::GONE);

        // Query for future block.
        let err = state.l1_block(2).unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        // Query for known block.
        assert_eq!(state.l1_block(1).unwrap(), block.block().id());
        assert_eq!(state.latest_l1_block(), block.block().id());
    }

    #[test_log::test]
    fn test_api_node_set() {
        let mut finalized = BlockData::empty(0);
        finalized.node_set_update = None;

        let mut block = BlockData::empty(1);
        let node = make_node(0);
        block.state.node_set.push(node.clone());
        block.node_set_update = Some(vec![FullNodeSetDiff::NodeUpdate(node)]);

        let state = from_blocks::<MemoryStorage>([finalized.clone(), block.clone()]);

        // Query for unknown block.
        let unknown = BlockData::empty(2).block().hash();
        let err = state.full_node_set(unknown).unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
        let err = state.full_node_set_update(unknown).unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        // Query for deleted update.
        let err = state
            .full_node_set_update(finalized.block().hash())
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::GONE);

        // Query for known block.
        assert_eq!(
            state.full_node_set(block.block().hash()).unwrap(),
            (block.state.node_set.clone(), block.block().info())
        );
        let update = state.full_node_set_update(block.block().hash()).unwrap();
        assert_eq!(&update.diff, block.node_set_update.as_ref().unwrap());
        assert_eq!(update.l1_block, block.block().info());
    }

    #[test_log::test]
    fn test_api_wallet() {
        let mut finalized = BlockData::empty(0);
        finalized.wallets_update = None;

        let address = Address::random();
        let delegation = Delegation {
            delegator: address,
            node: Address::random(),
            amount: Default::default(),
        };
        let wallet = Wallet {
            nodes: vec![delegation].into(),
            ..Default::default()
        };

        let mut block = BlockData::empty(1);
        block.state.wallets.insert(address, wallet.clone());
        block
            .wallets_update
            .as_mut()
            .unwrap()
            .insert(address, vec![WalletDiff::DelegatedToNode(delegation)]);
        // Let `address` be known even in the finalized snapshot, so we can test queries for a known
        // wallet in a block whose update field has been deleted.
        finalized.state.wallets = block.state.wallets.clone();
        // Insert a second wallet that is not updated by this block, so we can test queries for
        // updates for a known wallet with no non-trivial update.
        let not_updated = Address::random();
        let not_updated_wallet = Wallet::default();
        block
            .state
            .wallets
            .insert(not_updated, not_updated_wallet.clone());

        let state = from_blocks::<MemoryStorage>([finalized.clone(), block.clone()]);

        // Query for unknown block.
        let unknown = BlockData::empty(2).block().hash();
        let err = state.wallet(address, unknown).unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
        let err = state.wallet_update(address, unknown).unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        // Query for unknown address.
        let err = state
            .wallet(Address::random(), block.block().hash())
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
        let err = state
            .wallet_update(Address::random(), block.block().hash())
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        // Query for known address with no updates.
        assert_eq!(
            state.wallet(not_updated, block.block().hash()).unwrap(),
            (not_updated_wallet, block.block().info())
        );
        assert_eq!(
            state
                .wallet_update(not_updated, block.block().hash())
                .unwrap(),
            WalletUpdate {
                l1_block: block.block().info(),
                diff: vec![]
            }
        );

        // Query for deleted update.
        let err = state
            .wallet_update(address, finalized.block().hash())
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::GONE);

        // Query for known wallet.
        assert_eq!(
            state.wallet(address, block.block().hash()).unwrap(),
            (wallet, block.block().info())
        );
        let update = state.wallet_update(address, block.block().hash()).unwrap();
        assert_eq!(&update.diff, &[WalletDiff::DelegatedToNode(delegation)]);
        assert_eq!(update.l1_block, block.block().info());
    }

    #[test_log::test]
    fn test_replay_consistency() {
        let mut block = BlockData::empty(0);
        let mut state = StakeTableState::default();
        let mut num_events = 0;

        let inputs = InputGenerator::from_events(EventGenerator::stake_table_events()).take(100);
        for input in inputs {
            tracing::info!(?input, "apply input");
            num_events += input.events.len();

            // Compute the full node set snapshot as the staking UI service would do it.
            let start = Instant::now();
            block = block.next(&input);
            tracing::debug!(elapsed = ?start.elapsed(), "updated BlockData");

            // Compute the Espresso validator set as the protocol does it.
            let start = Instant::now();
            for event in &input.events {
                if let L1Event::StakeTable(e) = event {
                    let Ok(stake_table_event) = e.as_ref().clone().try_into() else {
                        tracing::info!(?e, "skipping GCL-irrelevant contract event");
                        continue;
                    };
                    state.apply_event(stake_table_event).unwrap().unwrap();
                }
            }
            tracing::debug!(
                elapsed = ?start.elapsed(),
                events = input.events.len(),
                "updated StakeTableState",
            );

            check_stake_table_consistency(&block.state.node_set, &state);
        }

        tracing::info!(
            "complete replay, processed {num_events} events and ended with {} nodes",
            block.state.node_set.len()
        );
    }

    #[test_log::test]
    fn test_large_state() {
        let mut block = BlockData::empty(0);

        // Realistically large state: 500 registered validators, 10000 delegators, each delegating
        // to 10 different nodes.
        for i in 0..500 {
            block.state.node_set.push(make_node(i));
        }
        for i in 0..10_000 {
            let delegator = Address::random();
            let mut wallet = Wallet::default();
            for j in 0..10 {
                let node = *block
                    .state
                    .node_set
                    .keys()
                    .nth((i + j) % block.state.node_set.len())
                    .unwrap();
                let delegation = Delegation {
                    delegator,
                    node,
                    amount: 1.try_into().unwrap(),
                };
                wallet.nodes.push_back(delegation);
            }
            block.state.wallets.insert(delegator, wallet);
        }

        // Apply random events. It should take on average no more than 12 seconds (although in
        // practice it should be much less), since that is how long we have to process an L1 block
        // in the real world.
        let inputs = InputGenerator::default().take(100);
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
        let inputs = (1..3).map(BlockInput::empty).collect::<Vec<_>>();
        let stream = &mut VecStream::default();

        // Start with just the finalized state.
        let state = RwLock::new(from_blocks::<MemoryStorage>([BlockData::empty(0)]));

        // Apply blocks without finalizing a new block.
        for input in &inputs {
            State::handle_new_head(state.upgradable_read().await, stream, input)
                .await
                .unwrap();
        }

        // Apply a finalized block.
        let mut finalized = BlockInput::empty(3);
        finalized.finalized = block_id(2);
        State::handle_new_head(state.upgradable_read().await, stream, &finalized)
            .await
            .unwrap();

        // Check that the new block has been added and state has been garbage collected.
        let state = state.read().await;
        assert_eq!(state.blocks.len(), 2);
        assert_eq!(state.blocks[0].block(), block_snapshot(2));
        assert_eq!(state.blocks[1].block(), block_snapshot(3));
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
                .block
                .id(),
            finalized.finalized
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_finalize_storage_failure() {
        let inputs = (1..3).map(BlockInput::empty).collect::<Vec<_>>();
        let stream = &mut VecStream::default();

        // Start with just the finalized state.
        let state = RwLock::new(from_blocks::<FailStorage>([BlockData::empty(0)]));

        // Apply blocks without finalizing a new block.
        for input in &inputs {
            State::handle_new_head(state.upgradable_read().await, stream, input)
                .await
                .unwrap();
        }

        // Apply a finalized block. Storing the finalized snapshot will fail, and on failure the
        // state should not be modified.
        let initial_state = { state.read().await.clone() };
        let mut finalized = BlockInput::empty(3);
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
            stream.push(BlockInput::empty(i));
        }

        // Start with just the finalized state.
        let state = Arc::new(RwLock::new(from_blocks::<MemoryStorage>([
            BlockData::empty(0),
        ])));

        // Process the updates in `stream`.
        let state = subscribe_until(&state, stream, |state| state.blocks.len() >= 3).await;
        assert_eq!(state.blocks.len(), 3);
        assert_eq!(state.blocks_by_hash.len(), 3);
        for i in 0..3 {
            assert_eq!(state.blocks[i as usize].block(), block_snapshot(i));
            assert_eq!(state.blocks_by_hash[&block_id(i).hash], i);
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_subscribe_reorg_head() {
        let inputs = (1..5).map(BlockInput::empty).collect::<Vec<_>>();

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
        let state = Arc::new(RwLock::new(from_blocks::<MemoryStorage>([
            BlockData::empty(0),
        ])));

        // Process the updates in `stream`.
        let state = subscribe_until(&state, stream, |state| state.blocks.len() >= 5).await;
        assert_eq!(state.blocks.len(), 5);
        assert_eq!(state.blocks_by_hash.len(), 5);
        for i in 0..5 {
            assert_eq!(state.blocks[i as usize].block(), block_snapshot(i));
            assert_eq!(state.blocks_by_hash[&block_id(i).hash], i);
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_subscribe_reorg_finalized() {
        let mut inputs = (1..5).map(BlockInput::empty).collect::<Vec<_>>();

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
        let state = Arc::new(RwLock::new(from_blocks::<MemoryStorage>([
            BlockData::empty(0),
        ])));

        // Process the updates in `stream`.
        let state = subscribe_until(&state, stream, |state| state.blocks.len() >= 4).await;

        // We should have garbage collected block 0 after block 1 became finalized.
        assert_eq!(state.blocks.len(), 4);
        assert_eq!(state.blocks_by_hash.len(), 4);
        for (input, block) in inputs.iter().zip(&state.blocks) {
            assert_eq!(block.block().id(), input.block);
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
                .block
                .id(),
            inputs[0].block
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_restart() {
        let mut stream = VecStream::infinite();
        stream.push(BlockInput::empty(1));
        // Second block finalizes the first block.
        let mut block_2 = BlockInput::empty(2);
        block_2.finalized = block_id(1);
        stream.push(block_2);

        // Start up and run until block 1 is finalized.
        let genesis = Snapshot::empty(block_snapshot(0));
        let storage = MemoryStorage::default();
        let state = Arc::new(RwLock::new(
            State::new(storage.clone(), genesis.clone()).await.unwrap(),
        ));
        // Initializing the state should cause a genesis snapshot to be saved.
        assert_eq!(
            storage.finalized_snapshot().await.unwrap().unwrap(),
            genesis
        );
        let state = subscribe_until(&state, stream, |state| {
            state.blocks[0].block().number() == 1
        })
        .await;
        assert_eq!(state.blocks.len(), 2);
        assert_eq!(
            storage.finalized_snapshot().await.unwrap().unwrap().block,
            block_snapshot(1)
        );
        drop(state);

        // Restart and check that we reload the finalized snapshot (and don't use the genesis, for
        // which we will pass in some nonsense).
        let genesis = Snapshot::empty(block_snapshot(1000));
        let state = State::new(storage.clone(), genesis).await.unwrap();
        assert_eq!(state.blocks.len(), 1);
        assert_eq!(state.blocks[0].block(), block_snapshot(1));

        // Subscribe to new blocks starting from where we left off (with block 1 being finalized,
        // the next block would be block 2).
        let mut stream = VecStream::infinite();
        stream.push(BlockInput::empty(2));
        let state = Arc::new(RwLock::new(state));
        let state = subscribe_until(&state, stream, |state| state.blocks.len() >= 2).await;
        assert_eq!(state.blocks.len(), 2);
        assert_eq!(state.blocks[0].block(), block_snapshot(1));
        assert_eq!(state.blocks[1].block(), block_snapshot(2));
    }

    /// Helper for testing event handling.
    ///
    /// Pass in a single event or an event sequence. Get back the state after applying those events
    /// to the empty state. Perform whatever validation you want on the result.
    ///
    /// Automatically validates
    /// * L1 data is snapshotted correctly
    /// * the result of applying the each event is consistent with applying it to a
    ///   [`StakeTableState`].
    /// * if an event is invalid, the state is not modified and no updates are recorded.
    fn test_events(events: impl IntoIterator<Item = StakeTableV2Events>) -> BlockData {
        let mut curr = BlockData::empty(0);
        let mut stake_table = StakeTableState::default();

        for (i, event) in events.into_iter().enumerate() {
            let number = i as u64 + 1;
            let next = curr.next(&BlockInput::empty(number).with_event(event.clone()));
            assert_eq!(next.block().id(), block_id(number));

            // Calling `next` should always populate the update fields.
            let node_set_update = next.node_set_update.as_ref().unwrap();
            let wallets_update = next.wallets_update.as_ref().unwrap();

            if let Ok(stake_table_event) = StakeTableEvent::try_from(event) {
                // This event affects the stake table. Check consistency between `next` and the
                // consensus protocol's [`StakeTableState`].
                if !matches!(stake_table.apply_event(stake_table_event), Ok(Ok(_))) {
                    // This event was invalid and should not have changed the stake table.
                    assert_eq!(next.state.node_set, curr.state.node_set);
                    assert!(node_set_update.is_empty());
                    assert!(wallets_update.is_empty());
                }
                check_stake_table_consistency(&next.state.node_set, &stake_table);
            }

            curr = next;
        }

        curr
    }

    #[test_log::test]
    fn test_event_validator_registered_v2_valid() {
        let event = validator_registered_event(rand::thread_rng());
        let block = test_events([StakeTableV2Events::ValidatorRegisteredV2(event.clone())]);
        let expected = NodeSetEntry::from(&event);
        assert_eq!(block.state.node_set.len(), 1);
        assert_eq!(block.state.node_set[&event.account], expected);
        assert_eq!(
            block.node_set_update.unwrap(),
            [FullNodeSetDiff::NodeUpdate(expected)]
        );
    }

    #[test_log::test]
    fn test_event_validator_registered_v2_invalid_bls() {
        let mut event = validator_registered_event(rand::thread_rng());

        // Change the signature so it doesn't match the public key.
        event.blsSig = validator_registered_event(rand::thread_rng()).blsSig;

        let block = test_events([StakeTableV2Events::ValidatorRegisteredV2(event)]);
        assert_eq!(block.state.node_set.len(), 0);
    }

    #[test_log::test]
    fn test_event_validator_registered_v2_invalid_schnorr() {
        let mut event = validator_registered_event(rand::thread_rng());

        // Change the signature so it doesn't match the public key.
        event.schnorrSig = validator_registered_event(rand::thread_rng()).schnorrSig;

        let block = test_events([StakeTableV2Events::ValidatorRegisteredV2(event)]);
        assert_eq!(block.state.node_set.len(), 0);
    }

    #[test_log::test]
    fn test_event_validator_exit_valid() {
        let node = validator_registered_event(rand::thread_rng());
        let next = test_events([
            StakeTableV2Events::ValidatorRegisteredV2(node.clone()),
            StakeTableV2Events::ValidatorExit(ValidatorExit {
                validator: node.account,
            }),
        ]);
        assert_eq!(next.state.node_set.len(), 0);
        assert_eq!(
            next.node_set_update.as_ref().unwrap(),
            &[FullNodeSetDiff::NodeExit(NodeExit {
                address: node.account,
                exit_time: next.block().timestamp() + next.block().exit_escrow_period
            })]
        );
    }

    #[test_log::test]
    fn test_event_validator_exit_invalid_not_found() {
        test_events([StakeTableV2Events::ValidatorExit(ValidatorExit {
            validator: Address::random(),
        })]);
    }

    #[test_log::test]
    fn test_exit_escrow_period_updated() {
        let block = test_events([StakeTableV2Events::ExitEscrowPeriodUpdated(
            ExitEscrowPeriodUpdated {
                newExitEscrowPeriod: 12345,
            },
        )]);
        assert_eq!(block.block().exit_escrow_period, 12345);
    }

    #[test_log::test]
    fn test_exit_escrow_period_updated_and_validator_exit_in_same_block() {
        let genesis = BlockData::empty(0);
        let node = validator_registered_event(rand::thread_rng());

        let input = BlockInput::empty(1)
            .with_event(StakeTableV2Events::ValidatorRegisteredV2(node.clone()))
            .with_event(StakeTableV2Events::ExitEscrowPeriodUpdated(
                ExitEscrowPeriodUpdated {
                    newExitEscrowPeriod: genesis.block().exit_escrow_period + 100,
                },
            ))
            .with_event(StakeTableV2Events::ValidatorExit(ValidatorExit {
                validator: node.account,
            }));
        let block = genesis.next(&input);
        assert!(block.state.node_set.is_empty());
        assert_eq!(
            block.block().exit_escrow_period,
            genesis.block().exit_escrow_period + 100
        );
        assert_eq!(
            block.node_set_update.as_ref().unwrap(),
            &[
                FullNodeSetDiff::NodeUpdate(NodeSetEntry::from(&node)),
                FullNodeSetDiff::NodeExit(NodeExit {
                    address: node.account,
                    exit_time: block.block().timestamp() + block.block().exit_escrow_period
                })
            ]
        );
    }
}
