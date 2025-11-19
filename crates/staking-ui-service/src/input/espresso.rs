//! Input data from Espresso network.

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::Duration,
};

use alloy::primitives::U256;
use async_lock::{RwLock, RwLockUpgradableReadGuard};
use bitvec::vec::BitVec;
use derivative::Derivative;
use espresso_types::{Leaf2, PubKey, ValidatorMap};
use futures::{Stream, StreamExt};
use hotshot_types::{
    data::ViewNumber,
    drb::{
        DrbResult,
        election::{RandomizedCommittee, generate_stake_cdf, select_randomized_leader},
    },
    stake_table::StakeTableEntry,
    traits::{block_contents::BlockHeader, node_implementation::ConsensusTime},
    utils::{epoch_from_block_number, transition_block_for_epoch},
};
use tokio::time::sleep;
use tracing::instrument;

use crate::{
    Error, Result,
    error::ensure,
    types::{
        common::{ActiveNodeSetEntry, Address, ESPTokenAmount, EpochAndBlock, Ratio},
        global::{ActiveNodeSetDiff, ActiveNodeSetSnapshot, ActiveNodeSetUpdate},
    },
};

pub mod client;
pub mod testing;

#[derive(Clone, Debug)]
pub struct State<S, C> {
    /// Updates from the start of the current epoch to the latest Espresso block.
    ///
    /// This list will always consist of sequentially increasing Espresso blocks, so that
    /// `updates[i]` corresponds to block `updates[0].espresso_block.block + i`.
    ///
    /// It is possible that the list will be empty, but only immediately after startup. Even
    /// immediately after an epoch change, there will be one update in this list defining the node
    /// set for the new epoch.
    ///
    /// It is possible that the updates in this list will not reach all the way back to the start of
    /// the current epoch, but only within the first epoch after service startup.
    updates: Vec<ActiveNodeSetUpdate>,

    /// The latest Espresso block number.
    ///
    /// This always corresponds to the snapshot saved in persistent storage. It can be used to find
    /// the offset of other Espresso blocks in `updates`. If `updates` is not empty, then
    /// `updates.last().espresso_block.block == latest_espresso_block`, and the offset of other
    /// blocks can be determined relative to that.
    latest_espresso_block: u64,

    /// The view number of the `latest_espresso_block`.
    latest_espresso_view: ViewNumber,

    /// The number of blocks in an epoch.
    epoch_height: u64,

    /// Information about the current epoch.
    epoch: Arc<EpochState>,

    /// Persistent storage handle.
    storage: S,

    /// Espresso query service client.
    espresso: C,
}

impl<S: EspressoPersistence, C: EspressoClient> State<S, C> {
    pub async fn new(storage: S, espresso: C) -> Result<Self> {
        let current_epoch = espresso.wait_for_epochs().await;

        // Fetch the current epoch state.
        let epoch_height = espresso.epoch_height().await?;
        let epoch = EpochState::download(&espresso, epoch_height, current_epoch)
            .await
            .map_err(|err| err.context(format!("fetching current epoch {current_epoch}")))?;

        // Initialize active node storage to the most recent of:
        // * saved active node set
        // * start of current epoch
        let (latest_espresso_block, latest_espresso_view) = match storage.active_node_set().await? {
            Some(snapshot) if snapshot.espresso_block.epoch == current_epoch => {
                tracing::info!(
                    ?snapshot,
                    current_epoch,
                    "saved snapshot is from latest epoch, restoring state"
                );
                // Fetch the view number for this block.
                let leaf = espresso
                    .leaf(snapshot.espresso_block.block)
                    .await
                    .map_err(|err| {
                        err.context(format!(
                            "fetching snapshotted leaf {}",
                            snapshot.espresso_block.block
                        ))
                    })?;
                (snapshot.espresso_block.block, leaf.view_number())
            }
            snapshot => {
                tracing::info!(
                    current_epoch,
                    ?snapshot,
                    "stored snapshot is missing or out of date, will start from beginning of epoch"
                );

                // Set our state to the last block of the previous epoch, so that when we start up,
                // the first block we process is the first block of this epoch.
                let leaf = espresso.leaf(epoch.start_block(epoch_height) - 1).await?;
                // TODO fetch the reward balance of every account as of the start of this epoch, so
                // we can initialize our lifetime rewards storage.
                (leaf.height(), leaf.view_number())
            }
        };

        Ok(Self {
            latest_espresso_block,
            latest_espresso_view,
            epoch_height,
            epoch: Arc::new(epoch),
            updates: vec![],
            espresso,
            storage,
        })
    }

    /// The latest Espresso block number ingested.
    ///
    /// This is the same block which the latest [`active_node_set`](Self::active_node_set) snapshot
    /// corresponds to.
    pub fn latest_espresso_block(&self) -> u64 {
        self.latest_espresso_block
    }

    /// Get the active node set as of the latest Espresso block.
    pub async fn active_node_set(&self) -> Result<ActiveNodeSetSnapshot> {
        let active_nodes = self
            .storage
            .active_node_set()
            .await?
            .ok_or_else(Error::not_found)?;
        Ok(active_nodes.into_snapshot(self.epoch.start_block(self.epoch_height)))
    }

    /// Get the changes to the active node set that occurred in the requested Espresso block.
    pub fn active_node_set_update(&self, block: u64) -> Result<ActiveNodeSetUpdate> {
        ensure!(
            block <= self.latest_espresso_block,
            Error::not_found().context(format!(
                "Espresso block {block} is not available; latest available block is {}",
                self.latest_espresso_block
            ))
        );

        // Find the offset of the requested `block` in our list of `updates` (from the back of the
        // list, where `latest_espresso_block` lives).
        let offset = (self.latest_espresso_block - block) as usize;
        ensure!(
            offset < self.updates.len(),
            Error::gone().context(format!(
                "Espresso block {block} is not available; earliest available block is {}",
                self.updates
                    .first()
                    .map(|update| update.espresso_block.block)
                    .unwrap_or(self.latest_espresso_block)
            ))
        );

        Ok(self.updates[self.updates.len() - 1 - offset].clone())
    }

    /// Get the total amount of rewards ever earned by `account`, as of the requested Espresso
    /// block.
    pub async fn lifetime_rewards(&self, account: Address, block: u64) -> Result<ESPTokenAmount> {
        ensure!(
            block == self.latest_espresso_block,
            Error::not_found().context(format!(
                "lifetime rewards are only available for the latest Espresso block {}",
                self.latest_espresso_block
            ))
        );
        self.storage.lifetime_rewards(account).await
    }

    /// Asynchronously update the [`State`] based on inputs from Espresso.
    ///
    /// Unless there is some catastrophic error, this future will never resolve. It is best spawned
    /// as a background task.
    #[instrument(skip(state))]
    pub async fn update_task(state: Arc<RwLock<Self>>) -> Result<()> {
        let (espresso, next_block) = {
            let state = state.read().await;
            (state.espresso.clone(), state.latest_espresso_block + 1)
        };
        let mut leaves = espresso.leaves(next_block);
        while let Some((leaf, voters)) = leaves.next().await {
            // Retry on any errors until we have successfully handled this leaf.
            loop {
                let state = state.upgradable_read().await;
                if let Err(err) = Self::handle_leaf(state, &leaf, &voters).await {
                    tracing::error!(?leaf, ?voters, "error processing leaf: {err}");
                } else {
                    break;
                }

                // Back off before retrying.
                sleep(Duration::from_secs(1)).await;
            }
        }

        Err(Error::internal().context("Espresso block stream ended unexpectedly"))
    }

    #[instrument(skip(state))]
    async fn handle_leaf(
        state: RwLockUpgradableReadGuard<'_, Self>,
        leaf: &Leaf2,
        voters: &BitVec,
    ) -> Result<()> {
        tracing::debug!("received Espresso input");

        let height = leaf.height();
        let current_epoch = epoch_from_block_number(height, state.epoch_height);
        let mut diff = vec![];

        // Handle epoch change.
        let new_epoch = height % state.epoch_height == 1;
        let epoch = if new_epoch {
            // This is the first block in a new epoch, switch over.
            tracing::info!(current_epoch, "starting new epoch");
            let next = Arc::new(
                EpochState::download(&state.espresso, state.epoch_height, current_epoch)
                    .await
                    .map_err(|err| err.context(format!("fetching next epoch {current_epoch}")))?,
            );
            diff.push(ActiveNodeSetDiff::NewEpoch(next.active_nodes().collect()));
            next
        } else {
            state.epoch.clone()
        };

        // Sanity check we are interpreting this leaf using the correct epoch.
        ensure!(
            epoch.number() == current_epoch,
            Error::internal().context(format!(
                "internal inconsistency: have wrong epoch {} for leaf {height} in epoch \
                {current_epoch}",
                epoch.number()
            ))
        );

        // Update leader and voter statistics.
        let leader = epoch.leader(leaf.view_number());
        // Find the leaders of any views between the previous block and this one, which must have
        // failed their view.
        let mut failed_leaders = vec![];
        for view in (state.latest_espresso_view.u64() + 1)..leaf.view_number().u64() {
            let leader = epoch.leader(ViewNumber::new(view));
            failed_leaders.push(leader);
        }
        diff.push(ActiveNodeSetDiff::NewBlock {
            leader,
            failed_leaders,
            voters: voters.clone(),
        });

        let update = ActiveNodeSetUpdate {
            espresso_block: EpochAndBlock {
                block: height,
                epoch: current_epoch,
                timestamp: leaf.block_header().timestamp_millis(),
            },
            diff,
        };

        // Compute reward distribution for this block.
        let rewards = epoch.reward_distribution(leader);

        // Update state with the results of this block.
        let mut state = RwLockUpgradableReadGuard::upgrade(state).await;
        // Update storage first. This will succeed for fail atomically. We can then update the
        // in-memory state to match; since we hold an exclusive lock on the state, no one will see
        // the in-memory state before the update has been completed.
        state
            .storage
            .apply_update(update.clone(), rewards)
            .await
            .map_err(|err| err.context("updating storage"))?;
        // At this point, we have already updated storage and now are updating the in-memory state
        // object in place. We must not fail after this point, or we may drop the write lock while
        // the state is in a partially modified state. All the validation performed up to this point
        // should be sufficient to ensure that we will not fail after this.
        state.updates.push(update);
        state.latest_espresso_block = leaf.height();
        state.latest_espresso_view = leaf.view_number();
        state.epoch = epoch;
        if new_epoch {
            state.garbage_collect();
        }

        Ok(())
    }

    /// Garbage collect in-memory updates.
    ///
    /// Drops all updates which are not from the current epoch.
    fn garbage_collect(&mut self) {
        self.updates
            .retain(|update| update.espresso_block.epoch >= self.epoch.number());
    }
}

/// Stake table related data that remains static for an entire epoch.
///
/// This is necessary to derive some quantities, such as leader participation rates and reward
/// distributions.
#[derive(Debug, Derivative)]
#[derivative(PartialEq, Eq)]
struct EpochState {
    /// The number of this epoch.
    number: u64,

    /// The post-processed stake table, used for the leader election function.
    // It is safe to ignore this field in comparisons
    #[derivative(PartialEq(compare_with = "Self::committee_eq"))]
    committee: RandomizedCommittee<StakeTableEntry<PubKey>>,

    /// The active node set for this epoch.
    ///
    /// This is the same set of nodes that makes up `committee`, but while `committee` has been
    /// processed into, essentially, a generator function for a pseudo-random stream of nodes, with
    /// repeats, `active_nodes` just lists each node once in a consistent order, for surfacing to
    /// the UI.
    active_nodes: Vec<Address>,

    /// Reverse lookup, from public keys to indices within the `active_nodes` list.
    node_index: HashMap<PubKey, usize>,

    /// The delegators to each active node.
    #[allow(dead_code)]
    delegators: BTreeMap<Address, Vec<(Address, U256)>>,
}

impl EpochState {
    /// Construct the epoch state from a stake table and DRB result.
    fn new(number: u64, nodes: ValidatorMap, drb_result: DrbResult) -> Self {
        // Get active node addresses.
        let active_nodes = nodes.keys().copied().collect::<Vec<_>>();
        // Index active nodes by staking key.
        let node_index = nodes
            .values()
            .enumerate()
            .map(|(i, node)| (node.stake_table_key, i))
            .collect();

        // Pre-process a randomized committee for leader election.
        let entries = nodes
            .values()
            .map(|node| StakeTableEntry {
                stake_key: node.stake_table_key,
                stake_amount: node.stake,
            })
            .collect();
        let committee = generate_stake_cdf(entries, drb_result);

        // Index delegator lists by address.
        let delegators = nodes
            .into_values()
            .map(|node| (node.account, node.delegators.into_iter().collect()))
            .collect();

        Self {
            number,
            committee,
            active_nodes,
            node_index,
            delegators,
        }
    }

    /// Fetch the state for an epoch from an Espresso query service.
    #[instrument(skip(espresso))]
    async fn download(
        espresso: &impl EspressoClient,
        epoch_height: u64,
        epoch: u64,
    ) -> Result<Self> {
        // We require the first epoch at least to be completed, so we have a "previous" epoch from
        // which we can get the stake table and DRB result for this epoch.
        ensure!(
            epoch > 1,
            Error::internal().context("Espresso state must be started after epoch 1")
        );

        tracing::info!("fetching epoch state");
        let stake_table = espresso
            .stake_table_for_epoch(epoch)
            .await
            .map_err(|err| err.context("fetching stake table"))?;

        // Get the DRB result from the transition block of the previous epoch.
        let transition_block = transition_block_for_epoch(epoch - 1, epoch_height);
        let drb_leaf = espresso
            .leaf(transition_block)
            .await
            .map_err(|err| err.context(format!("fetching transition leaf {transition_block}")))?;
        // Sanity check that we actually got a block from the previous epoch.
        ensure!(
            drb_leaf.epoch(epoch_height).as_deref().copied() == Some(epoch - 1),
            Error::internal().context(format!(
                "transition leaf {drb_leaf:?} is not from expected epoch {} (epoch_height={epoch_height})",
                epoch - 1
            ))
        );
        let drb_result = drb_leaf.next_drb_result.ok_or_else(|| {
            Error::internal().context(format!(
                "transition block of epoch {} (height {transition_block}) does not have next epoch DRB result",
                epoch - 1,
            ))
        })?;

        Ok(Self::new(epoch, stake_table, drb_result))
    }

    /// The index of the leader for `view`.
    ///
    /// The caller must ensure that `view` is actually within the current epoch.
    fn leader(&self, view: ViewNumber) -> usize {
        let entry = select_randomized_leader(&self.committee, *view);
        self.node_index[&entry.stake_key]
    }

    /// Compute rewards to distribute to each account for a given block.
    fn reward_distribution(&self, _leader: usize) -> Vec<(Address, ESPTokenAmount)> {
        // TODO: actually compute rewards
        vec![]
    }

    /// The addresses of nodes in this epoch, in a consistent order.
    fn active_nodes(&self) -> impl Iterator<Item = Address> {
        self.active_nodes.iter().copied()
    }

    /// The block number of the first block in this epoch.
    fn start_block(&self, epoch_height: u64) -> u64 {
        (self.number - 1) * epoch_height + 1
    }

    /// The epoch number.
    fn number(&self) -> u64 {
        self.number
    }

    /// Compare randomized committees _in the context of an [`EpochState`].
    ///
    /// This comparison works by comparing the DRB results in each committee _only_. This works because
    /// the rest of the [`EpochState`] comparison also checks for equality of the node set, and the
    /// full state of the randomized committee is derived from the node set and the DRB result.
    fn committee_eq<T>(c1: &RandomizedCommittee<T>, c2: &RandomizedCommittee<T>) -> bool {
        c1.drb_result() == c2.drb_result()
    }
}

/// Persistent storage for data derived from Espresso.
pub trait EspressoPersistence {
    /// Get the latest persisted active node set.
    fn active_node_set(&self) -> impl Send + Future<Output = Result<Option<ActiveNodeSet>>>;

    /// Get the lifetime rewards earned by the requested account.
    fn lifetime_rewards(
        &self,
        account: Address,
    ) -> impl Send + Future<Output = Result<ESPTokenAmount>>;

    /// Apply changes to persistent storage after the given Espresso block.
    fn apply_update(
        &mut self,
        update: ActiveNodeSetUpdate,
        rewards: Vec<(Address, ESPTokenAmount)>,
    ) -> impl Send + Future<Output = Result<()>>;
}

/// State tracked for the active node set.
///
/// This represents the same information as an [`ActiveNodeSetSnapshot`], but internally it is
/// stored in a different format to support incremental updating. For example, we store both the
/// numerator and denominator of participation rates, rather than just the rate itself, so that we
/// can incrementally update rates just by incrementing the numerator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveNodeSet {
    /// The Espresso block at which this node set was active.
    pub espresso_block: EpochAndBlock,

    /// The nodes in the set, plus statistics.
    pub nodes: Vec<ActiveNode>,
}

impl ActiveNodeSet {
    /// Convert this node set into the public API representation of an active node set.
    pub fn into_snapshot(self, epoch_start_block: u64) -> ActiveNodeSetSnapshot {
        ActiveNodeSetSnapshot {
            espresso_block: self.espresso_block,
            nodes: self
                .nodes
                .into_iter()
                .map(|node| {
                    node.into_node_set_entry(
                        (self.espresso_block.block - epoch_start_block) as usize + 1,
                    )
                })
                .collect(),
        }
    }
}

/// State tracked for an individual active node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveNode {
    pub address: Address,
    pub proposals: usize,
    pub slots: usize,
    pub votes: usize,
}

impl ActiveNode {
    /// A node with a given address and empty statistics.
    pub fn new(address: Address) -> Self {
        Self {
            address,
            proposals: 0,
            slots: 0,
            votes: 0,
        }
    }

    /// Convert this node into the public API representation of an active node.
    pub fn into_node_set_entry(self, blocks_in_epoch: usize) -> ActiveNodeSetEntry {
        ActiveNodeSetEntry {
            address: self.address,
            voter_participation: Ratio::new(self.votes, blocks_in_epoch),
            leader_participation: Ratio::new(self.proposals, self.slots),
        }
    }
}

/// Interface for querying data from Espresso.
pub trait EspressoClient: Clone {
    /// Wait until epochs begin, in case the service is started before the POS upgrade.
    ///
    /// Eventually resolves with the current epoch number.
    fn wait_for_epochs(&self) -> impl Send + Future<Output = u64>;

    /// Get the configured epoch height, in blocks.
    fn epoch_height(&self) -> impl Send + Future<Output = Result<u64>>;

    /// Fetch the stake table for the requested epoch.
    fn stake_table_for_epoch(
        &self,
        epoch: u64,
    ) -> impl Send + Future<Output = Result<ValidatorMap>>;

    /// Fetch a leaf.
    fn leaf(&self, height: u64) -> impl Send + Future<Output = Result<Leaf2>>;

    /// Subscribe to leaves starting from the requested height.
    ///
    /// Each leaf is paired with the set of voters who signed it.
    fn leaves(&self, from: u64) -> impl Send + Unpin + Stream<Item = (Leaf2, BitVec)>;
}

#[cfg(test)]
mod test {
    use crate::input::espresso::testing::{MockEspressoClient, fake_drb_result};

    use super::*;

    use alloy::transports::http::reqwest::StatusCode;
    use testing::MemoryStorage;
    use tide_disco::Error;
    use tokio::task::spawn;

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_handle_leaf_node_stats() {
        let mut espresso = MockEspressoClient::new(3).await;
        let epoch_height = espresso.epoch_height();

        // Start somewhere in the middle of an epoch; this test is not about epoch changes.
        let blocks_in_epoch = 3;
        for _ in 0..(blocks_in_epoch - 1) {
            espresso.push_leaf(0, [false, false, false]).await;
        }
        let last_leaf = espresso.last_leaf().0;
        let epoch = *last_leaf.epoch(epoch_height).unwrap();

        let nodes = espresso
            .stake_table_for_epoch(epoch)
            .await
            .unwrap()
            .into_keys()
            .map(ActiveNode::new)
            .collect();
        let storage = MemoryStorage::new(
            ActiveNodeSet {
                espresso_block: EpochAndBlock {
                    epoch,
                    block: last_leaf.height(),
                    timestamp: last_leaf.block_header().timestamp_millis(),
                },
                nodes,
            },
            Default::default(),
        );

        // Create a leaf which skips a view from the previous leaf. We should thus get one
        // successful leader and one failed leader.
        let signers = [true, true, false];
        let (leaf, signers) = espresso.push_leaf(1, signers).await;
        let leaf = leaf.clone();
        let signers = signers.clone();

        let state = RwLock::new(State::new(storage, espresso).await.unwrap());
        {
            // Check preloaded state.
            let state = state.read().await;
            assert_eq!(state.latest_espresso_block(), leaf.height() - 1);
            assert_eq!(state.latest_espresso_view, leaf.view_number() - 2);
            assert_eq!(state.epoch.number(), epoch);
        }

        // Process the next leaf.
        State::handle_leaf(state.upgradable_read().await, &leaf, &signers)
            .await
            .unwrap();

        let state = state.read().await;

        // Check node statistics.
        let snapshot = state.active_node_set().await.unwrap();
        assert_eq!(snapshot.espresso_block.block, leaf.height());
        assert_eq!(snapshot.nodes.len(), 3);
        assert_eq!(state.latest_espresso_block(), leaf.height());

        tracing::info!(
            leader = state.epoch.leader(leaf.view_number()),
            failed_leader = state.epoch.leader(leaf.view_number() - 1),
            "checking leaders"
        );
        // We don't know exactly what the leader participation rate should be, because it depends on
        // whether the failed leader and successful leader were the same or different (depending on
        // randomness, you can have the same leader twice in a row). But we know that the _average_
        // participation of the failed and successful leader should be 1/2.
        assert_eq!(
            f32::from(snapshot.nodes[state.epoch.leader(leaf.view_number())].leader_participation)
                + f32::from(
                    snapshot.nodes[state.epoch.leader(leaf.view_number() - 1)].leader_participation
                ),
            1f32
        );

        tracing::info!("checking vote participation");
        assert_eq!(
            snapshot.nodes[0].voter_participation,
            (1f32 / (blocks_in_epoch as f32)).into()
        );
        assert_eq!(
            snapshot.nodes[1].voter_participation,
            (1f32 / (blocks_in_epoch as f32)).into()
        );
        assert_eq!(snapshot.nodes[2].voter_participation, 0f32.into());

        // Update should have been recorded.
        assert_eq!(
            state.active_node_set_update(leaf.height()).unwrap(),
            ActiveNodeSetUpdate {
                espresso_block: EpochAndBlock {
                    epoch: *leaf.epoch(epoch_height).unwrap(),
                    block: leaf.height(),
                    timestamp: leaf.block_header().timestamp_millis()
                },
                diff: vec![ActiveNodeSetDiff::NewBlock {
                    leader: state.epoch.leader(leaf.view_number()),
                    failed_leaders: vec![state.epoch.leader(leaf.view_number() - 1)],
                    voters: signers,
                }]
            }
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_handle_leaf_epoch_change() {
        let mut espresso = MockEspressoClient::new(3).await;
        let first_epoch = espresso.current_epoch() + 1;
        let epoch_height = espresso.epoch_height();
        let prev_stake_table = espresso
            .stake_table_for_epoch(first_epoch)
            .await
            .unwrap()
            .into_values()
            .map(|node| node.account)
            .collect::<Vec<_>>();
        let stake_table = espresso
            .stake_table_for_epoch(first_epoch + 1)
            .await
            .unwrap()
            .into_values()
            .map(|node| node.account)
            .collect();
        tracing::info!(
            ?prev_stake_table,
            ?stake_table,
            "test will transition stake table from epoch {} to {}",
            first_epoch,
            first_epoch + 1
        );

        // Add leaves until we get to the end of the epoch.
        let mut leaves = vec![];
        for _ in 0..epoch_height {
            let (leaf, signers) = espresso.push_leaf(0, [false, false, false]).await;
            leaves.push((leaf.clone(), signers.clone()));
        }

        // Add the first leaf of the next epoch.
        let (leaf, signers) = espresso.push_leaf(0, [true, true, true]).await;
        let last_leaf = leaf.clone();
        let last_signers = signers.clone();
        leaves.push((leaf.clone(), signers.clone()));

        let state = RwLock::new(
            State::new(MemoryStorage::default(), espresso)
                .await
                .unwrap(),
        );

        for (leaf, signers) in leaves {
            State::handle_leaf(state.upgradable_read().await, &leaf, &signers)
                .await
                .unwrap();
        }

        let state = state.read().await;
        let snapshot = state.active_node_set().await.unwrap();
        assert_eq!(state.latest_espresso_block(), last_leaf.height());
        assert_eq!(snapshot.espresso_block.block, last_leaf.height());
        assert_eq!(snapshot.espresso_block.epoch, 3);
        assert_eq!(
            snapshot
                .nodes
                .iter()
                .map(|node| node.address)
                .collect::<Vec<_>>(),
            stake_table
        );
        // Everybody participated in the first view of the new epoch.
        assert_eq!(snapshot.nodes[0].voter_participation, 1f32.into());
        assert_eq!(snapshot.nodes[1].voter_participation, 1f32.into());
        assert_eq!(snapshot.nodes[2].voter_participation, 1f32.into());

        // Epoch state has been updated.
        assert_eq!(
            state.epoch.number(),
            *last_leaf.epoch(epoch_height).unwrap()
        );
        assert_eq!(
            state.epoch.committee.drb_result(),
            fake_drb_result(state.epoch.number())
        );

        // Only events from the new epoch are retained.
        assert_eq!(
            state.active_node_set_update(last_leaf.height()).unwrap(),
            ActiveNodeSetUpdate {
                espresso_block: EpochAndBlock {
                    epoch: 3,
                    block: last_leaf.height(),
                    timestamp: last_leaf.block_header().timestamp_millis()
                },
                diff: vec![
                    ActiveNodeSetDiff::NewEpoch(stake_table),
                    ActiveNodeSetDiff::NewBlock {
                        leader: state.epoch.leader(last_leaf.view_number()),
                        failed_leaders: vec![],
                        voters: last_signers,
                    }
                ]
            }
        );
        let err = state
            .active_node_set_update(last_leaf.height() - 1)
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::GONE);
        let err = state
            .active_node_set_update(last_leaf.height() + 1)
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_update_task() {
        let mut espresso = MockEspressoClient::new(3).await;
        let epoch_height = espresso.epoch_height();

        // Start somewhere in the middle of an epoch; this test is not about epoch changes.
        let pre_blocks = 3;
        for _ in 0..pre_blocks {
            espresso.push_leaf(0, [false, false, false]).await;
        }
        let last_leaf = espresso.last_leaf().0;
        let epoch = *last_leaf.epoch(epoch_height).unwrap();

        let nodes = espresso
            .stake_table_for_epoch(epoch)
            .await
            .unwrap()
            .into_keys()
            .map(ActiveNode::new)
            .collect();
        let storage = MemoryStorage::new(
            ActiveNodeSet {
                espresso_block: EpochAndBlock {
                    epoch,
                    block: last_leaf.height(),
                    timestamp: last_leaf.block_header().timestamp_millis(),
                },
                nodes,
            },
            Default::default(),
        );

        // Push a number of future leaves.
        let mut leaves = vec![];
        for _ in 0..10 {
            let (leaf, signers) = espresso.push_leaf(0, [true, true, true]).await;
            leaves.push((leaf.clone(), signers.clone()));
        }

        let mut state = State::new(storage, espresso).await.unwrap();

        // Throw in a transient storage failure, the update task should handle this gracefully.
        state.storage.fail_next();

        // Start background task.
        let state = Arc::new(RwLock::new(state));
        let task = spawn(State::update_task(state.clone()));

        // Wait for all the updates.
        let state = loop {
            sleep(Duration::from_secs(1)).await;
            let state = state.read().await;
            if state.updates.len() < leaves.len() {
                tracing::info!(
                    leaves = leaves.len(),
                    updates = state.updates.len(),
                    "waiting for updates"
                );
                continue;
            }
            task.abort();
            task.await.ok();
            break state;
        };

        // Check snapshot consistency.
        let snapshot = state.active_node_set().await.unwrap();
        let last_leaf = &leaves.last().unwrap().0;
        assert_eq!(
            snapshot.espresso_block,
            EpochAndBlock {
                block: last_leaf.height(),
                epoch,
                timestamp: last_leaf.block_header().timestamp_millis(),
            }
        );
        for node in &snapshot.nodes {
            assert_eq!(
                f32::from(node.voter_participation),
                (leaves.len() as f32) / ((pre_blocks + leaves.len()) as f32)
            );
            assert_eq!(node.leader_participation, 1f32.into());
        }

        // We should have one diff for each leaf.
        for (leaf, voters) in leaves {
            let update = state.active_node_set_update(leaf.height()).unwrap();
            assert_eq!(
                update.espresso_block,
                EpochAndBlock {
                    block: leaf.height(),
                    epoch,
                    timestamp: leaf.block_header().timestamp_millis(),
                }
            );
            assert_eq!(
                update.diff,
                vec![ActiveNodeSetDiff::NewBlock {
                    leader: state.epoch.leader(leaf.view_number()),
                    failed_leaders: vec![],
                    voters,
                }]
            )
        }
    }

    async fn test_handle_leaf_failure_helper(
        setup_failure: impl FnOnce(&mut State<MemoryStorage, MockEspressoClient>),
    ) {
        let mut espresso = MockEspressoClient::new(3).await;
        let (leaf, signers) = espresso.push_leaf(0, [true, true, true]).await;
        let (leaf, signers) = (leaf.clone(), signers.clone());

        let mut state = State::new(MemoryStorage::default(), espresso)
            .await
            .unwrap();
        let pre_state = state.clone();
        let pre_storage = pre_state.storage.cmp_key().await;

        setup_failure(&mut state);
        let state = RwLock::new(state);
        State::handle_leaf(state.upgradable_read().await, &leaf, &signers)
            .await
            .unwrap_err();

        // The failed update did not change anything. [`State`] does not implement [`Eq`] (in
        // particular, comparing [`MemoryStorage`] is an async operation and does not use the [`Eq`]
        // trait) so we compare field by field. This pattern match ensures we remember every field.
        let State {
            updates: pre_updates,
            latest_espresso_block: pre_latest_espresso_block,
            latest_espresso_view: pre_latest_espresso_view,
            epoch_height: pre_epoch_height,
            epoch: pre_epoch,
            storage: _,
            espresso: _,
        } = pre_state;
        let State {
            updates,
            latest_espresso_block,
            latest_espresso_view,
            epoch_height,
            epoch,
            storage,
            espresso: _,
        } = state.into_inner();
        assert_eq!(pre_updates, updates);
        assert_eq!(pre_latest_espresso_block, latest_espresso_block);
        assert_eq!(pre_latest_espresso_view, latest_espresso_view);
        assert_eq!(pre_epoch_height, epoch_height);
        assert_eq!(epoch, pre_epoch);
        assert_eq!(pre_storage, storage.cmp_key().await);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_handle_leaf_failure_storage() {
        test_handle_leaf_failure_helper(|state| state.storage.fail_next()).await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_handle_leaf_failure_fetch_stake_table() {
        test_handle_leaf_failure_helper(|state| {
            state
                .espresso
                .delete_stake_table(state.espresso.current_epoch())
        })
        .await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_handle_leaf_failure_fetch_transition_leaf() {
        test_handle_leaf_failure_helper(|state| {
            state.espresso.delete_leaf(transition_block_for_epoch(
                state.epoch.number - 1,
                state.epoch_height,
            ))
        })
        .await;
    }
}
