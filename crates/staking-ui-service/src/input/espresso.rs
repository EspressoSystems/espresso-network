//! Input data from Espresso network.

use std::{collections::HashMap, sync::Arc, time::Duration};

use async_lock::{RwLock, RwLockUpgradableReadGuard, RwLockWriteGuard};
use bitvec::vec::BitVec;
use derivative::Derivative;
use espresso_types::{
    Leaf2, PubKey, ValidatorMap,
    v0::RewardDistributor,
    v0_3::{RewardAmount, Validator},
};
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
        common::{ActiveNodeSetEntry, Address, ESPTokenAmount, EpochAndBlock},
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

    /// The latest Espresso block.
    ///
    /// This always corresponds to the snapshot saved in persistent storage. It can be used to find
    /// the offset of other Espresso blocks in `updates`. If `last_block` is not [`None`], then
    /// `updates.last().espresso_block.block == last_block.number`, and the offset of other blocks
    /// can be determined relative to that.
    last_block: Option<BlockState>,

    /// The number of blocks in an epoch.
    epoch_height: u64,

    /// Persistent storage handle.
    storage: S,

    /// Espresso query service client.
    espresso: C,
}

impl<S: EspressoPersistence, C: EspressoClient> State<S, C> {
    /// Fetches all reward accounts and their balances from the Espresso API at the given
    /// block height, then saves them to the database.
    ///
    /// Called once at startup to populate the initial reward state.
    async fn sync_lifetime_rewards(state: &RwLock<Self>, block_height: u64) -> Result<()> {
        tracing::info!("Synchronizing lifetime rewards from Espresso API at block {block_height}");

        let espresso = { state.read().await.espresso.clone() };

        let reward_balances = espresso.fetch_all_reward_accounts(block_height).await?;

        if !reward_balances.is_empty() {
            let mut state = state.write().await;
            state
                .storage
                .initialize_lifetime_rewards(reward_balances)
                .await
                .map_err(|e| {
                    Error::internal().context(format!(
                        "synchronizing lifetime rewards from Espresso API: {e}",
                    ))
                })?;
        }

        Ok(())
    }

    pub async fn new(storage: S, espresso: C) -> Result<Self> {
        let epoch_height = espresso.epoch_height().await?;
        Ok(Self {
            last_block: None,
            updates: vec![],
            espresso,
            storage,
            epoch_height,
        })
    }

    /// The latest Espresso block number ingested.
    ///
    /// This is the same block which the latest [`active_node_set`](Self::active_node_set) snapshot
    /// corresponds to.
    pub fn latest_espresso_block(&self) -> Result<u64> {
        Ok(self.last_block()?.number)
    }

    /// Get the active node set as of the latest Espresso block.
    pub async fn active_node_set(&self) -> Result<ActiveNodeSetSnapshot> {
        let epoch = &self.last_block()?.epoch;
        let active_nodes = self
            .storage
            .active_node_set()
            .await?
            .ok_or_else(Error::not_found)?;
        Ok(active_nodes.into_snapshot(epoch.start_block(self.epoch_height)))
    }

    /// Get the changes to the active node set that occurred in the requested Espresso block.
    pub fn active_node_set_update(&self, block: u64) -> Result<ActiveNodeSetUpdate> {
        let last_block = self.last_block()?;
        ensure!(
            block <= last_block.number,
            Error::not_found().context(format!(
                "Espresso block {block} is not available; latest available block is {}",
                last_block.number
            ))
        );

        // Find the offset of the requested `block` in our list of `updates` (from the back of the
        // list, where `last_block` lives).
        let offset = (last_block.number - block) as usize;
        ensure!(
            offset < self.updates.len(),
            Error::gone().context(format!(
                "Espresso block {block} is not available; earliest available block is {}",
                self.updates
                    .first()
                    .map(|update| update.espresso_block.block)
                    .unwrap_or(last_block.number)
            ))
        );

        Ok(self.updates[self.updates.len() - 1 - offset].clone())
    }

    /// Get the total amount of rewards ever earned by `account`, as of the requested Espresso
    /// block.
    pub async fn lifetime_rewards(&self, account: Address, block: u64) -> Result<ESPTokenAmount> {
        let last_block = self.last_block()?;

        // Make sure the requested block is the one we have a state snapshot for.
        if block > last_block.number {
            return Err(Error::not_found().context(format!(
                "Espresso block {block} is not available; latest available block is {}",
                last_block.number
            )));
        }
        if block < last_block.number {
            return Err(Error::gone().context(format!(
                "rewards for Espresso block {block} are not available; earliest available block is {}",
                last_block.number
            )));
        }

        self.storage.lifetime_rewards(account).await
    }

    /// Asynchronously update the [`State`] based on inputs from Espresso.
    ///
    /// Unless there is some catastrophic error, this future will never resolve. It is best spawned
    /// as a background task.
    #[instrument(skip(state))]
    pub async fn update_task(state: Arc<RwLock<Self>>) -> Result<()> {
        // Retry until we initialize successfully.
        let next_block = loop {
            match Self::update_task_initial_block(&state).await {
                Ok(block) => break block,
                Err(err) => {
                    tracing::error!("error finding initial Espresso block: {err:#}");
                    sleep(Duration::from_secs(1)).await;
                }
            }
        };
        let espresso = { state.read().await.espresso.clone() };
        let mut leaves = espresso.leaves(next_block);
        while let Some((leaf, voters)) = leaves.next().await {
            let state = state.upgradable_read().await;

            // Calculate the effects of this new leaf, retrying on errors.
            let (block, update, rewards) = loop {
                match state.leaf_effects(&leaf, &voters).await {
                    Ok(effects) => break effects,
                    Err(err) => {
                        tracing::error!(?leaf, ?voters, "error computing leaf effects: {err:#}");
                        sleep(Duration::from_secs(1)).await;
                    }
                }
            };

            // Apply effects to state, retying on errors.
            let mut state_read = state;
            loop {
                let mut state = RwLockUpgradableReadGuard::upgrade(state_read).await;
                if let Err(err) = state
                    .apply_effects(block.clone(), update.clone(), rewards.clone())
                    .await
                {
                    tracing::error!(?leaf, ?voters, "error applying leaf effects: {err}");
                } else {
                    break;
                }

                // Release exclusive lock and back off before retrying.
                state_read = RwLockWriteGuard::downgrade_to_upgradable(state);
                sleep(Duration::from_secs(1)).await;
            }
        }

        Err(Error::internal().context("Espresso block stream ended unexpectedly"))
    }

    async fn update_task_initial_block(state: &RwLock<Self>) -> Result<u64> {
        let (espresso, snapshot, epoch_height) = {
            let state = state.read().await;
            (
                state.espresso.clone(),
                state.storage.active_node_set().await?,
                state.epoch_height,
            )
        };
        let current_epoch = espresso.wait_for_epochs().await;

        // Initialize to the most recent of:
        // * saved active node set
        // * start of current epoch
        let next_espresso_block = match snapshot {
            Some(snapshot) if snapshot.espresso_block.epoch == current_epoch => {
                tracing::info!(
                    ?snapshot,
                    current_epoch,
                    "saved snapshot is from latest epoch, restoring state"
                );
                snapshot.espresso_block.block + 1
            }
            snapshot => {
                tracing::info!(
                    current_epoch,
                    ?snapshot,
                    "stored snapshot is missing or out of date, will start from beginning of epoch"
                );

                // Fetch the reward balance of every account as of the start of this epoch, so
                // we can initialize our lifetime rewards storage.
                let epoch_start = epoch_start_block(current_epoch, epoch_height);

                let sync_block = epoch_start - 1;
                let already_synced = snapshot
                    .as_ref()
                    .is_some_and(|s| s.espresso_block.block == sync_block);

                if !already_synced {
                    Self::sync_lifetime_rewards(state, sync_block).await?;
                }

                // Set our state to the first block of this epoch.
                epoch_start
            }
        };

        Ok(next_espresso_block)
    }

    /// Interpret a new Espresso leaf.
    ///
    /// Returns the corresponding block state along with changes to the active node set and reward
    /// accounts.
    #[instrument(skip(self))]
    async fn leaf_effects(
        &self,
        leaf: &Leaf2,
        voters: &BitVec,
    ) -> Result<(BlockState, ActiveNodeSetUpdate, RewardDistribution)> {
        tracing::debug!("received Espresso input");
        let height = leaf.height();
        let current_epoch = epoch_from_block_number(height, self.epoch_height);
        let mut diff = vec![];

        // Get the previous block state. We don't need the entire block state (which might be
        // missing if this is the first leaf we're processing after startup) but we need at least:
        // * the previous view number, to account for failed views
        // * the previous [`EpochState`], if we have it, so we can avoid unnecessarily downloading
        //   it again
        let (prev_view, prev_epoch) = match &self.last_block {
            Some(prev) => (prev.view, Some(prev.epoch.clone())),
            None => {
                // Fetch the view number of the parent leaf.
                let parent =
                    self.espresso.leaf(height - 1).await.map_err(|err| {
                        err.context(format!("fetching parent leaf {}", height - 1))
                    })?;
                (parent.view_number().u64(), None)
            }
        };

        // Download epoch state if necessary.
        let epoch = match prev_epoch {
            Some(epoch) if epoch.number() == current_epoch => epoch,
            _ => Arc::new(
                EpochState::download(&self.espresso, self.epoch_height, current_epoch)
                    .await
                    .map_err(|err| err.context(format!("fetching next epoch {current_epoch}")))?,
            ),
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

        // Emit an event for the first block in the epoch.
        let new_epoch = height % self.epoch_height == 1;
        if new_epoch {
            tracing::info!(current_epoch, "starting new epoch");
            diff.push(ActiveNodeSetDiff::NewEpoch(epoch.active_nodes().collect()));
        }

        // Update leader and voter statistics.
        let leader = epoch.leader(leaf.view_number());
        // Find the leaders of any views between the previous block and this one, which must have
        // failed their view.
        let mut failed_leaders = vec![];
        for view in (prev_view + 1)..leaf.view_number().u64() {
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
        let rewards = epoch
            .reward_distribution(leader, &self.espresso)
            .await
            .map_err(|err| err.context("computing reward distribution"))?;

        let block = BlockState {
            number: height,
            view: leaf.view_number().u64(),
            epoch,
        };
        Ok((block, update, rewards))
    }

    /// Update the [`State`] by applying changes returned from [`leaf_effects`](Self::leaf_effects).
    #[instrument(skip(self))]
    async fn apply_effects(
        &mut self,
        block: BlockState,
        update: ActiveNodeSetUpdate,
        rewards: RewardDistribution,
    ) -> Result<()> {
        // Determine if the epoch is changing; if so we will garbage collect.
        let new_epoch = match &self.last_block {
            Some(last_block) if last_block.epoch.number != block.epoch.number => {
                tracing::debug!(
                    last_block.epoch.number,
                    block.epoch.number,
                    "epoch changed, GC will run"
                );
                true
            }
            _ => false,
        };

        // Update storage first. This will succeed or fail atomically. We can then update the
        // in-memory state to match; since we hold an exclusive lock on the state, no one will see
        // the in-memory state before the update has been completed.
        self.storage
            .apply_update(update.clone(), rewards)
            .await
            .map_err(|err| err.context("updating storage"))?;

        // At this point, we have already updated storage and now are updating the in-memory state
        // object in place. We must not fail after this point, or we may drop the write lock while
        // the state is in a partially modified state. All the validation performed up to this point
        // should be sufficient to ensure that we will not fail after this.
        if new_epoch {
            self.garbage_collect(block.epoch.number);
        }
        self.updates.push(update);
        self.last_block = Some(block);

        Ok(())
    }

    /// Get the latest Espresso block.
    ///
    /// Returns a `503 Service Unavailable` error if the Espresso block has not yet been initialized
    /// by the update task.
    fn last_block(&self) -> Result<&BlockState> {
        self.last_block.as_ref().ok_or_else(Error::not_initialized)
    }

    /// Garbage collect in-memory updates.
    ///
    /// Drops all updates which are not from the current epoch.
    fn garbage_collect(&mut self, epoch: u64) {
        self.updates
            .retain(|update| update.espresso_block.epoch >= epoch);
    }
}

/// State that changes every block.
#[derive(Clone, Debug, PartialEq, Eq)]
struct BlockState {
    /// The block number.
    number: u64,

    /// The view number of the block.
    view: u64,

    /// The epoch this block belongs to.
    epoch: Arc<EpochState>,
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

    /// The block reward amount for this epoch
    block_reward: ESPTokenAmount,

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

    /// The full validator information for each active node
    validators: Vec<Validator<PubKey>>,
}

impl EpochState {
    /// Construct the epoch state from a stake table and DRB result.
    fn new(
        number: u64,
        nodes: ValidatorMap,
        drb_result: DrbResult,
        block_reward: ESPTokenAmount,
    ) -> Self {
        // Get active node addresses.
        let active_nodes = nodes.keys().copied().collect::<Vec<_>>();
        let validators: Vec<_> = nodes.values().cloned().collect();

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

        Self {
            number,
            block_reward,
            committee,
            active_nodes,
            node_index,
            validators,
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

        let block_reward = espresso.block_reward(epoch).await?;
        let state = Self::new(epoch, stake_table, drb_result, block_reward);

        Ok(state)
    }

    /// The index of the leader for `view`.
    ///
    /// The caller must ensure that `view` is actually within the current epoch.
    fn leader(&self, view: ViewNumber) -> usize {
        let entry = select_randomized_leader(&self.committee, *view);
        self.node_index[&entry.stake_key]
    }

    /// Compute rewards to distribute to each account for a given block.
    async fn reward_distribution<C: EspressoClient>(
        &self,
        leader: usize,
        _client: &C,
    ) -> Result<Vec<(Address, ESPTokenAmount)>> {
        if self.block_reward.is_zero() {
            tracing::warn!(
                "Block reward is zero for epoch={}, skipping reward distribution",
                self.number
            );
            return Ok(vec![]);
        }

        let validator = self
            .validators
            .get(leader)
            .ok_or_else(|| {
                Error::internal().context(format!(
                    "leader index {leader} out of bounds (validators len: {})",
                    self.validators.len()
                ))
            })?
            .clone();

        let distributor = RewardDistributor::new(
            validator,
            RewardAmount(self.block_reward),
            RewardAmount::from(0u64), // total_distributed not needed for compute_rewards
        );

        let computed_rewards = distributor.compute_rewards().map_err(|err| {
            Error::internal().context(format!("failed to compute rewards: {err}"))
        })?;

        Ok(computed_rewards
            .all_rewards()
            .into_iter()
            .map(|(addr, reward)| (addr, reward.0))
            .collect())
    }

    /// The addresses of nodes in this epoch, in a consistent order.
    fn active_nodes(&self) -> impl Iterator<Item = Address> {
        self.active_nodes.iter().copied()
    }

    /// The block number of the first block in this epoch.
    fn start_block(&self, epoch_height: u64) -> u64 {
        epoch_start_block(self.number, epoch_height)
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

fn epoch_start_block(epoch: u64, epoch_height: u64) -> u64 {
    (epoch - 1) * epoch_height + 1
}

/// List of accounts receiving rewards and the amounts they are receiving.
pub type RewardDistribution = Vec<(Address, ESPTokenAmount)>;

/// Persistent storage for data derived from Espresso.
pub trait EspressoPersistence {
    /// Get the latest persisted active node set.
    fn active_node_set(&self) -> impl Send + Future<Output = Result<Option<ActiveNodeSet>>>;

    /// Get the lifetime rewards earned by the requested account.
    fn lifetime_rewards(
        &self,
        account: Address,
    ) -> impl Send + Future<Output = Result<ESPTokenAmount>>;

    /// Initialize lifetime rewards
    ///
    /// This is used at startup to populate the lifetime_rewards table with balances
    /// from the Espresso API.
    fn initialize_lifetime_rewards(
        &mut self,
        rewards: Vec<(Address, ESPTokenAmount)>,
    ) -> impl Send + Future<Output = Result<()>>;

    /// Apply changes to persistent storage after the given Espresso block.
    fn apply_update(
        &mut self,
        update: ActiveNodeSetUpdate,
        rewards: RewardDistribution,
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
        let blocks_in_epoch = self
            .espresso_block
            .block
            .checked_sub(epoch_start_block)
            .unwrap_or_else(|| {
                panic!(
                    "Block {} (epoch {}) < epoch_start_block {epoch_start_block}",
                    self.espresso_block.block, self.espresso_block.epoch
                )
            }) as usize
            + 1;

        ActiveNodeSetSnapshot {
            espresso_block: self.espresso_block,
            nodes: self
                .nodes
                .into_iter()
                .map(|node| node.into_node_set_entry(blocks_in_epoch))
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
            votes: self.votes as u64,
            eligible_votes: blocks_in_epoch as u64,
            proposals: self.proposals as u64,
            slots: self.slots as u64,
        }
    }
}

/// Interface for querying data from Espresso.
pub trait EspressoClient: Clone + Send + Sync {
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

    /// Get the block reward amount for a given epoch.
    fn block_reward(&self, epoch: u64) -> impl Send + Future<Output = Result<ESPTokenAmount>>;

    /// Subscribe to leaves starting from the requested height.
    ///
    /// Each leaf is paired with the set of voters who signed it.
    fn leaves(&self, from: u64) -> impl Send + Unpin + Stream<Item = (Leaf2, BitVec)>;

    /// Fetch all reward accounts and their balances at a specific block height.
    ///
    /// This uses the `reward-amounts` endpoint which is more efficient than
    /// querying individual balances when syncing all accounts.
    fn fetch_all_reward_accounts(
        &self,
        block: u64,
    ) -> impl Send + Future<Output = Result<Vec<(Address, ESPTokenAmount)>>>;
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::input::espresso::testing::{MockEspressoClient, fake_drb_result};

    use crate::input::espresso::client::{QueryServiceClient, QueryServiceOptions};
    use crate::persistence::sql::{Persistence, PersistenceOptions};
    use alloy::primitives::U256;
    use surf_disco::Client as HttpClient;
    use tempfile::NamedTempFile;
    use testing::start_pos_network;
    use vbs::version::StaticVersion;

    use alloy::transports::http::reqwest::StatusCode;
    use async_lock::Semaphore;
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

        // Process the next leaf.
        handle_leaf(state.upgradable_read().await, &leaf, &signers)
            .await
            .unwrap();

        let state = state.read().await;

        // Check node statistics.
        let snapshot = state.active_node_set().await.unwrap();
        assert_eq!(snapshot.espresso_block.block, leaf.height());
        assert_eq!(snapshot.nodes.len(), 3);
        assert_eq!(state.latest_espresso_block().unwrap(), leaf.height());

        let epoch = &state.last_block().unwrap().epoch;
        tracing::info!(
            leader = epoch.leader(leaf.view_number()),
            failed_leader = epoch.leader(leaf.view_number() - 1),
            "checking leaders"
        );
        // Exactly which nodes succeeded and failed depends on the leader election function, but we
        // know there should be a total of one successful proposal out of 2 possible slots.
        assert_eq!(
            snapshot
                .nodes
                .iter()
                .map(|node| node.proposals)
                .sum::<u64>(),
            1
        );
        assert_eq!(snapshot.nodes.iter().map(|node| node.slots).sum::<u64>(), 2);

        tracing::info!("checking vote participation");
        assert_eq!(snapshot.nodes[0].votes, 1);
        assert_eq!(snapshot.nodes[0].eligible_votes, blocks_in_epoch);
        assert_eq!(snapshot.nodes[1].votes, 1);
        assert_eq!(snapshot.nodes[1].eligible_votes, blocks_in_epoch);
        assert_eq!(snapshot.nodes[2].votes, 0);
        assert_eq!(snapshot.nodes[2].eligible_votes, blocks_in_epoch);

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
                    leader: epoch.leader(leaf.view_number()),
                    failed_leaders: vec![epoch.leader(leaf.view_number() - 1)],
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
            handle_leaf(state.upgradable_read().await, &leaf, &signers)
                .await
                .unwrap();
        }

        let state = state.read().await;
        let snapshot = state.active_node_set().await.unwrap();
        assert_eq!(state.latest_espresso_block().unwrap(), last_leaf.height());
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
        assert_eq!(snapshot.nodes[0].votes, 1);
        assert_eq!(snapshot.nodes[1].votes, 1);
        assert_eq!(snapshot.nodes[2].votes, 1);

        // Epoch state has been updated.
        let epoch = &state.last_block().unwrap().epoch;
        assert_eq!(epoch.number(), *last_leaf.epoch(epoch_height).unwrap());
        assert_eq!(
            epoch.committee.drb_result(),
            fake_drb_result(epoch.number())
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
                        leader: epoch.leader(last_leaf.view_number()),
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
            assert_eq!(node.votes, leaves.len() as u64);
            assert_eq!(node.eligible_votes, (pre_blocks + leaves.len()) as u64);
            assert_eq!(node.slots, node.proposals);
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
                    leader: state.last_block().unwrap().epoch.leader(leaf.view_number()),
                    failed_leaders: vec![],
                    voters,
                }]
            )
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_rewards() {
        let mut espresso = MockEspressoClient::new(3).await;
        let (leaf, signers) = espresso.push_leaf(0, [true, true, true]).await;
        let (leaf, signers) = (leaf.clone(), signers.clone());

        let state = RwLock::new(
            State::new(MemoryStorage::default(), espresso)
                .await
                .unwrap(),
        );
        handle_leaf(state.upgradable_read().await, &leaf, &signers)
            .await
            .unwrap();
        let state = state.into_inner();

        // Check empty account.
        assert_eq!(
            state
                .lifetime_rewards(Address::random(), leaf.height())
                .await
                .unwrap(),
            ESPTokenAmount::ZERO
        );

        // Check old block.
        assert_eq!(
            state
                .lifetime_rewards(Address::random(), leaf.height() - 1)
                .await
                .unwrap_err()
                .status(),
            StatusCode::GONE
        );

        // Check new block.
        assert_eq!(
            state
                .lifetime_rewards(Address::random(), leaf.height() + 1)
                .await
                .unwrap_err()
                .status(),
            StatusCode::NOT_FOUND
        );
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
        let err = handle_leaf(state.upgradable_read().await, &leaf, &signers)
            .await
            .unwrap_err();
        tracing::info!("task failed successfully: {err:#}");

        // The failed update did not change anything. [`State`] does not implement [`Eq`] (in
        // particular, comparing [`MemoryStorage`] is an async operation and does not use the [`Eq`]
        // trait) so we compare field by field. This pattern match ensures we remember every field.
        let State {
            updates: pre_updates,
            last_block: pre_last_block,
            epoch_height: pre_epoch_height,
            storage: _,
            espresso: _,
        } = pre_state;
        let State {
            updates,
            last_block,
            epoch_height,
            storage,
            espresso: _,
        } = state.into_inner();
        assert_eq!(pre_updates, updates);
        assert_eq!(pre_last_block, last_block);
        assert_eq!(pre_epoch_height, epoch_height);
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
                state.espresso.current_epoch() - 1,
                state.epoch_height,
            ))
        })
        .await;
    }

    async fn wait_for_state_and_rewards(
        state_lock: &Arc<RwLock<State<Persistence, QueryServiceClient>>>,
        http_client: &HttpClient<crate::Error, StaticVersion<0, 1>>,
        target_epoch: u64,
        epoch_height: u64,
    ) -> u64 {
        let min_blocks_to_wait = target_epoch * epoch_height;
        tracing::info!(
            "Waiting for blocks up to epoch {target_epoch} (block {min_blocks_to_wait})"
        );

        loop {
            sleep(Duration::from_secs(1)).await;
            let state = state_lock.read().await;
            let latest_block = state.latest_espresso_block().unwrap();
            tracing::info!("Latest Espresso block processed: {latest_block}");

            if latest_block >= min_blocks_to_wait {
                break;
            }
        }

        let final_state = state_lock.read().await;
        let active_node_snapshot = final_state
            .active_node_set()
            .await
            .expect("Failed to get active node set");
        let latest_espresso_block = active_node_snapshot.espresso_block.block;

        loop {
            sleep(Duration::from_secs(1)).await;

            match http_client
                .get::<u64>("reward-state-v2/block-height")
                .send()
                .await
            {
                Ok(block) if block >= latest_espresso_block => {
                    tracing::info!("Reward-state is at block {block}");
                    break latest_espresso_block;
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("Failed to get reward-state block height, will retry: {e}");
                }
            }
        }
    }

    async fn assert_reward_balances(
        state_lock: &Arc<RwLock<State<Persistence, QueryServiceClient>>>,
        http_client: &HttpClient<crate::Error, StaticVersion<0, 1>>,
        latest_espresso_block: u64,
    ) {
        let state = state_lock.read().await;

        let active_node_snapshot = state
            .active_node_set()
            .await
            .expect("Failed to get active node set");

        assert!(
            !active_node_snapshot.nodes.is_empty(),
            "Should have active nodes"
        );

        for (idx, node) in active_node_snapshot.nodes.iter().enumerate() {
            let stored_rewards = state
                .storage
                .lifetime_rewards(node.address)
                .await
                .unwrap_or_default();

            let espresso_reward_balance: U256 = http_client
                .get::<Option<RewardAmount>>(&format!(
                    "reward-state-v2/reward-balance/{}/{}",
                    latest_espresso_block, node.address
                ))
                .send()
                .await
                .expect("Failed to get reward balance")
                .map(|amount| amount.0)
                .unwrap_or(U256::from(0));

            tracing::info!(
                "Node {idx}: stored_rewards={stored_rewards} WEI, espresso_api={espresso_reward_balance} WEI at block {latest_espresso_block}"
            );

            assert_eq!(
                U256::from(stored_rewards),
                espresso_reward_balance,
                "Node {idx} reward balance mismatch: stored={stored_rewards}, espresso_api={espresso_reward_balance}"
            );
        }
    }
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_wallet_block_rewards_e2e_test() {
        let espresso_port = portpicker::pick_unused_port().expect("No free ports");
        let (network, _storage, _deployment) = start_pos_network(espresso_port).await;

        tracing::info!("Started Espresso network on port {espresso_port}");

        let espresso_url = format!("http://localhost:{espresso_port}");
        let espresso_client =
            QueryServiceClient::new(QueryServiceOptions::new(espresso_url.parse().unwrap()))
                .await
                .expect("Failed to create Espresso client");

        let current_epoch = espresso_client.wait_for_epochs().await;
        tracing::info!("Epochs started, current epoch: {current_epoch}");

        let http_client: HttpClient<crate::Error, StaticVersion<0, 1>> =
            HttpClient::new(espresso_url.parse().unwrap());
        http_client.connect(None).await;

        let temp_db = NamedTempFile::new().expect("Failed to create temp file");
        let temp_db_path = temp_db.path().to_path_buf();

        let epoch_height = {
            let espresso_storage = Persistence::new(&PersistenceOptions {
                path: temp_db_path.clone(),
                max_connections: 5,
            })
            .await
            .expect("Failed to create SQL persistence");
            let espresso_state = State::new(espresso_storage.clone(), espresso_client.clone())
                .await
                .expect("Failed to initialize Espresso state");
            espresso_state.epoch_height
        };

        // Test at current_epoch
        {
            tracing::info!("Testing at epoch {}", current_epoch);
            let espresso_storage = Persistence::new(&PersistenceOptions {
                path: temp_db_path.clone(),
                max_connections: 5,
            })
            .await
            .expect("Failed to create SQL persistence");
            let espresso_state = State::new(espresso_storage.clone(), espresso_client.clone())
                .await
                .expect("Failed to initialize Espresso state");

            let state_lock = Arc::new(RwLock::new(espresso_state));
            let update_task = spawn(State::update_task(state_lock.clone()));

            let latest_block =
                wait_for_state_and_rewards(&state_lock, &http_client, current_epoch, epoch_height)
                    .await;

            tracing::info!("Reached latest block {latest_block}");
            update_task.abort();

            assert_reward_balances(&state_lock, &http_client, latest_block).await;
        }

        // Test at current_epoch + 3
        {
            tracing::info!("Testing at epoch {}", current_epoch + 3);
            let espresso_storage = Persistence::new(&PersistenceOptions {
                path: temp_db_path.clone(),
                max_connections: 5,
            })
            .await
            .expect("Failed to create SQL persistence");
            let espresso_state = State::new(espresso_storage.clone(), espresso_client.clone())
                .await
                .expect("Failed to initialize Espresso state");

            let state_lock = Arc::new(RwLock::new(espresso_state));
            let update_task = spawn(State::update_task(state_lock.clone()));

            let latest_block = wait_for_state_and_rewards(
                &state_lock,
                &http_client,
                current_epoch + 3,
                epoch_height,
            )
            .await;

            tracing::info!("Reached latest block {latest_block}");
            update_task.abort();

            assert_reward_balances(&state_lock, &http_client, latest_block).await;
        }

        // Test at current_epoch + 3 again
        {
            tracing::info!("Testing at epoch {} (repeat)", current_epoch + 3);
            let espresso_storage = Persistence::new(&PersistenceOptions {
                path: temp_db_path.clone(),
                max_connections: 5,
            })
            .await
            .expect("Failed to create SQL persistence");
            let espresso_state = State::new(espresso_storage.clone(), espresso_client.clone())
                .await
                .expect("Failed to initialize Espresso state");

            let state_lock = Arc::new(RwLock::new(espresso_state));
            let update_task = spawn(State::update_task(state_lock.clone()));

            let latest_block = wait_for_state_and_rewards(
                &state_lock,
                &http_client,
                current_epoch + 3,
                epoch_height,
            )
            .await;

            tracing::info!("Reached latest block {latest_block}");
            update_task.abort();

            assert_reward_balances(&state_lock, &http_client, latest_block).await;
        }

        // Test at current_epoch + 4
        {
            tracing::info!("Testing at epoch {}", current_epoch + 4);
            let espresso_storage = Persistence::new(&PersistenceOptions {
                path: temp_db_path.clone(),
                max_connections: 5,
            })
            .await
            .expect("Failed to create SQL persistence");
            let espresso_state = State::new(espresso_storage.clone(), espresso_client.clone())
                .await
                .expect("Failed to initialize Espresso state");

            let state_lock = Arc::new(RwLock::new(espresso_state));
            let update_task = spawn(State::update_task(state_lock.clone()));

            let latest_block = wait_for_state_and_rewards(
                &state_lock,
                &http_client,
                current_epoch + 4,
                epoch_height,
            )
            .await;

            tracing::info!("Reached latest block {latest_block}");
            update_task.abort();

            assert_reward_balances(&state_lock, &http_client, latest_block).await;
        }

        // Test at current_epoch + 4 with new storage
        {
            tracing::info!("Testing at epoch {} (with new storage)", current_epoch + 4);
            let temp_db2 = NamedTempFile::new().expect("Failed to create temp file");
            let espresso_storage = Persistence::new(&PersistenceOptions {
                path: temp_db2.path().to_path_buf(),
                max_connections: 5,
            })
            .await
            .expect("Failed to create SQL persistence");
            let espresso_state = State::new(espresso_storage.clone(), espresso_client.clone())
                .await
                .expect("Failed to initialize Espresso state");

            let state_lock = Arc::new(RwLock::new(espresso_state));
            let update_task = spawn(State::update_task(state_lock.clone()));

            let latest_block = wait_for_state_and_rewards(
                &state_lock,
                &http_client,
                current_epoch + 4,
                epoch_height,
            )
            .await;

            tracing::info!("Reached latest block {latest_block}");
            update_task.abort();

            assert_reward_balances(&state_lock, &http_client, latest_block).await;
        }

        drop(network);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_wait_for_epochs() {
        let mut espresso = MockEspressoClient::new(3).await;
        let leaf = espresso.push_leaf(0, [true, true, true]).await.0;
        let leaf = leaf.clone();
        let espresso = WaitForEpochsClient::new(espresso);

        let state = Arc::new(RwLock::new(
            State::new(MemoryStorage::default(), espresso.clone())
                .await
                .unwrap(),
        ));
        let task = spawn(State::update_task(state.clone()));

        // Prior to epochs being enabled, the APIs are not available.
        sleep(Duration::from_secs(1)).await;
        {
            let state = state.read().await;
            assert_eq!(
                state.latest_espresso_block().unwrap_err().status(),
                StatusCode::SERVICE_UNAVAILABLE
            );
            assert_eq!(
                state.active_node_set().await.unwrap_err().status(),
                StatusCode::SERVICE_UNAVAILABLE
            );
            assert_eq!(
                state
                    .active_node_set_update(leaf.height())
                    .unwrap_err()
                    .status(),
                StatusCode::SERVICE_UNAVAILABLE
            );
            assert_eq!(
                state
                    .lifetime_rewards(Address::random(), leaf.height())
                    .await
                    .unwrap_err()
                    .status(),
                StatusCode::SERVICE_UNAVAILABLE
            );
        }

        // After enabling epochs, we begin processing Espresso blocks and populating state.
        espresso.start_epochs();
        loop {
            let state = state.read().await;
            let last_block = state.latest_espresso_block();
            if let Ok(block) = last_block
                && block == leaf.height()
            {
                break;
            }
            tracing::info!(
                leaf.height = leaf.height(),
                ?last_block,
                "waiting for state to catch up"
            );
            sleep(Duration::from_secs(1)).await;
        }

        // Now all the APIs are usable.
        {
            let state = state.read().await;
            assert_eq!(
                state.active_node_set().await.unwrap().espresso_block.block,
                leaf.height()
            );
            assert_eq!(
                state
                    .active_node_set_update(leaf.height())
                    .unwrap()
                    .espresso_block
                    .block,
                leaf.height()
            );
            assert_eq!(
                state
                    .lifetime_rewards(Address::random(), leaf.height())
                    .await
                    .unwrap(),
                ESPTokenAmount::ZERO
            );
        }

        task.abort();
        task.await.ok();
    }

    /// A client wrapper which does not start epochs until signalled.
    #[derive(Clone, Debug)]
    struct WaitForEpochsClient<C> {
        inner: Arc<C>,
        wait_for_epochs: Arc<Semaphore>,
    }

    impl<C> WaitForEpochsClient<C> {
        fn new(inner: C) -> Self {
            Self {
                inner: Arc::new(inner),
                // Start with a limit of 0 in the semaphore, so `wait_for_epochs` will block until
                // signalled.
                wait_for_epochs: Arc::new(Semaphore::new(0)),
            }
        }

        fn start_epochs(&self) {
            // Increase the semaphore limit so waiters in `wait_for_epochs` will be able to acquire
            // it.
            self.wait_for_epochs.add_permits(1);
        }
    }

    impl<C> EspressoClient for WaitForEpochsClient<C>
    where
        C: EspressoClient,
    {
        async fn wait_for_epochs(&self) -> u64 {
            // Wait until the semaphore is signalled.
            {
                self.wait_for_epochs.acquire().await;
            }
            self.inner.wait_for_epochs().await
        }

        async fn epoch_height(&self) -> Result<u64> {
            self.inner.epoch_height().await
        }

        async fn leaf(&self, height: u64) -> Result<Leaf2> {
            self.inner.leaf(height).await
        }

        async fn stake_table_for_epoch(&self, epoch: u64) -> Result<ValidatorMap> {
            self.inner.stake_table_for_epoch(epoch).await
        }

        async fn block_reward(&self, epoch: u64) -> Result<ESPTokenAmount> {
            self.inner.block_reward(epoch).await
        }

        async fn fetch_all_reward_accounts(
            &self,
            block: u64,
        ) -> Result<Vec<(Address, ESPTokenAmount)>> {
            self.inner.fetch_all_reward_accounts(block).await
        }

        fn leaves(&self, from: u64) -> impl Send + Unpin + Stream<Item = (Leaf2, BitVec)> {
            self.inner.leaves(from)
        }
    }

    async fn handle_leaf<S, C>(
        state: RwLockUpgradableReadGuard<'_, State<S, C>>,
        leaf: &Leaf2,
        signers: &BitVec,
    ) -> Result<()>
    where
        S: EspressoPersistence,
        C: EspressoClient,
    {
        let (block, update, rewards) = state.leaf_effects(leaf, signers).await?;
        let mut state = RwLockUpgradableReadGuard::upgrade(state).await;
        state.apply_effects(block, update, rewards).await
    }
}
