//! The state of the chain at recent heights, for consensus nodes catching up.
//!
//! The API service follows its store in height order, as a consensus node follows the chain: it
//! applies each block's header to the state of its parent and checks the result against the roots
//! the header commits to, and it hands each epoch root and each DRB result to membership, which
//! needs both to know the leaders whose rewards the state pays at the end of an epoch. It keeps the
//! states of the newest heights, which is what a node that is behind asks for: it fetches the state
//! an API service has and applies the headers after it itself.
//!
//! The [`ChainFollower`] alone writes the states; the [`ChainStates`] it hands out read them. Every
//! [`SNAPSHOT_INTERVAL`] heights it stores a snapshot of what the next blocks need, so a restarted
//! API service resumes there instead of applying the chain from genesis again. A store pruned past
//! the heights the follower needs, without a snapshot to start after them, cannot be followed.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use anyhow::{Context, bail, ensure};
use bytes::Bytes;
use chain_store::{AppWrite, ChainStore};
use committable::Committable;
use consensus_node::null_catchup::NullStateCatchup;
use either::Either;
use espresso_types::{
    BlockMerkleTree, ChainConfig, Leaf2, NodeState, ValidatedState,
    v0_3::RewardAmount,
    v0_4::{REWARD_MERKLE_TREE_V2_HEIGHT, RewardAccountV2, RewardMerkleTreeV2},
};
use hotshot_types::{
    traits::states::ValidatedState as _,
    utils::{is_epoch_root, is_transition_block},
};
use jf_merkle_tree_compat::{ForgetableMerkleTreeScheme, MerkleTreeScheme};
use serde::{Deserialize, Serialize};
use tokio::{sync::watch, time::sleep};

/// How many of the newest heights the state is kept for.
const KEPT_HEIGHTS: u64 = 256;

/// How long to wait before applying a block again that could not be applied, for instance because
/// membership does not have the stake table of its epoch yet.
const RETRY_DELAY: Duration = Duration::from_secs(1);

/// How many heights apart snapshots are stored. The store must keep more heights than this, or it
/// prunes the snapshot's height before the next one is stored.
pub const SNAPSHOT_INTERVAL: u64 = 64;

/// The application table of the chain store the snapshot is kept in, and its key.
const TABLE: &str = "chain-state";
const SNAPSHOT: &[u8] = b"snapshot";

type BlocksFrontier = <BlockMerkleTree as MerkleTreeScheme>::MembershipProof;

/// Creates the follower of `store` and the reader of the states it keeps.
pub fn follower(store: ChainStore, node: NodeState) -> (ChainFollower, ChainStates) {
    let (states, rx) = watch::channel(BTreeMap::new());
    (
        ChainFollower {
            store,
            node,
            states,
        },
        ChainStates(rx),
    )
}

/// The states of the newest heights the API service has applied.
#[derive(Clone, Debug)]
pub struct ChainStates(watch::Receiver<BTreeMap<u64, Arc<ValidatedState>>>);

impl ChainStates {
    /// The state after the block at `height`, if it is among the newest kept.
    pub fn at(&self, height: u64) -> Option<Arc<ValidatedState>> {
        self.0.borrow().get(&height).cloned()
    }

    /// The newest height whose state is kept.
    pub fn newest(&self) -> Option<u64> {
        self.0.borrow().keys().next_back().copied()
    }

    /// The newest state kept, with its height.
    pub fn latest(&self) -> Option<(u64, Arc<ValidatedState>)> {
        let states = self.0.borrow();
        let (height, state) = states.last_key_value()?;
        Some((*height, state.clone()))
    }
}

/// Applies the blocks of the store in height order, and keeps the newest states.
#[derive(Debug)]
pub struct ChainFollower {
    store: ChainStore,
    node: NodeState,
    states: watch::Sender<BTreeMap<u64, Arc<ValidatedState>>>,
}

impl ChainFollower {
    /// Follows the store from the snapshot, or from genesis without one. Returns when the store's
    /// changes end, or fails if the store no longer holds the heights to start from.
    pub async fn run(self) -> anyhow::Result<()> {
        let mut changes = self.store.changes();
        let (mut parent, mut height) = match self.start().await? {
            Some((leaf, state)) => {
                let height = leaf.height();
                tracing::info!(height, "resuming from the stored snapshot");
                self.keep(height, Arc::new(state.clone()));
                (Some((leaf, state)), height + 1)
            },
            None => (None, 0),
        };
        loop {
            changes.borrow_and_update();
            let item = match self.store.item(height).await {
                Ok(Some(item)) => item,
                Ok(None) => {
                    let floor = self.store.floor().await?;
                    if floor > height {
                        bail!("the store pruned height {height} before it was applied");
                    }
                    // Not stored yet; the ingester or the gap filler stores it.
                    if changes.changed().await.is_err() {
                        return Ok(());
                    }
                    continue;
                },
                Err(err) => {
                    tracing::warn!(height, "reading the store failed: {err:#}");
                    sleep(RETRY_DELAY).await;
                    continue;
                },
            };
            let state = match &parent {
                None => Ok(ValidatedState::genesis(&self.node).0),
                Some((parent_leaf, parent_state)) => {
                    apply(&self.node, parent_state, parent_leaf, &item.leaf).await
                },
            };
            let state = match state {
                Ok(state) => state,
                Err(err) => {
                    tracing::warn!(height, "applying the block failed: {err:#}");
                    sleep(RETRY_DELAY).await;
                    continue;
                },
            };
            if let Err(err) = track_epoch(&self.node, &item.leaf).await {
                tracing::warn!(height, "membership did not take the block: {err:#}");
                sleep(RETRY_DELAY).await;
                continue;
            }
            self.keep(height, Arc::new(state.clone()));
            if height % SNAPSHOT_INTERVAL == 0
                && let Err(err) = self.snapshot(height, &state).await
            {
                tracing::warn!(height, "storing the snapshot failed: {err:#}");
            }
            parent = Some((item.leaf, state));
            height += 1;
        }
    }

    /// The leaf and state to continue after: the snapshot's, or none to start at genesis.
    async fn start(&self) -> anyhow::Result<Option<(Leaf2, ValidatedState)>> {
        match self.resume().await {
            Ok(Some(start)) => return Ok(Some(start)),
            Ok(None) => {},
            Err(err) => tracing::warn!("ignoring the stored snapshot: {err:#}"),
        }
        let floor = self.store.floor().await?;
        if floor > 0 {
            bail!(
                "the store keeps heights from {floor} on and no usable snapshot of the state; it \
                 cannot be followed from genesis"
            );
        }
        Ok(None)
    }

    /// The leaf and state of the stored snapshot, checked against the leaf's header.
    async fn resume(&self) -> anyhow::Result<Option<(Leaf2, ValidatedState)>> {
        let Some(bytes) = self.store.app_get(TABLE, SNAPSHOT).await? else {
            return Ok(None);
        };
        let snapshot: Snapshot = bincode::deserialize(&bytes).context("decoding the snapshot")?;
        let item = self
            .store
            .item(snapshot.height)
            .await?
            .with_context(|| format!("height {} is not stored", snapshot.height))?;
        let header = item.leaf.block_header();
        let mut state = ValidatedState::from_header(header);

        if let Some(last) = state.block_merkle_tree.num_leaves().checked_sub(1) {
            let frontier = snapshot.frontier.context("no frontier of the block tree")?;
            let elem = *frontier.elem().context("a frontier without its element")?;
            state
                .block_merkle_tree
                .remember(last, elem, &frontier)
                .context("the frontier does not prove against the block root")?;
        }

        ensure!(
            snapshot.chain_config.commit() == header.chain_config().commit(),
            "the chain config is not the header's"
        );
        state.chain_config = snapshot.chain_config.into();

        // The fee tree does not change under the new protocol.
        let genesis = ValidatedState::genesis(&self.node).0;
        ensure!(
            genesis.fee_merkle_tree.commitment() == header.fee_merkle_tree_root(),
            "the genesis fee tree is not the header's"
        );
        state.fee_merkle_tree = genesis.fee_merkle_tree;

        if let Either::Right(root) = header.reward_merkle_tree_root() {
            let tree =
                RewardMerkleTreeV2::from_kv_set(REWARD_MERKLE_TREE_V2_HEIGHT, snapshot.rewards)
                    .context("rebuilding the reward tree")?;
            ensure!(
                tree.commitment() == root,
                "the reward tree is not the header's"
            );
            state.reward_merkle_tree_v2 = tree;
        }
        Ok(Some((item.leaf, state)))
    }

    /// Stores what the blocks after `height` need of `state`, the state after it.
    async fn snapshot(&self, height: u64, state: &ValidatedState) -> anyhow::Result<()> {
        let frontier = match state.block_merkle_tree.num_leaves().checked_sub(1) {
            Some(last) => {
                let (_, frontier) = state
                    .block_merkle_tree
                    .lookup(last)
                    .expect_ok()
                    .map_err(|_| anyhow::anyhow!("the block tree forgot its frontier"))?;
                Some(frontier)
            },
            None => None,
        };
        let chain_config = state
            .chain_config
            .resolve()
            .context("the chain config is not resolved")?;
        let tree = &state.reward_merkle_tree_v2;
        let rewards: Vec<_> = tree
            .iter()
            .map(|(account, amount)| (*account, *amount))
            .collect();
        ensure!(
            rewards.len() as u64 == tree.num_leaves(),
            "the reward tree is missing accounts"
        );
        let snapshot = Snapshot {
            height,
            frontier,
            chain_config,
            rewards,
        };
        self.store
            .app_write(vec![AppWrite::Put {
                table: TABLE.into(),
                key: Bytes::from_static(SNAPSHOT),
                value: bincode::serialize(&snapshot)?.into(),
            }])
            .await
    }

    fn keep(&self, height: u64, state: Arc<ValidatedState>) {
        self.states.send_modify(|states| {
            states.insert(height, state);
            while states.len() as u64 > KEPT_HEIGHTS {
                states.pop_first();
            }
        });
    }
}

/// What the blocks after `height` need of the state after it, besides the roots its header
/// holds: the frontier of the block tree, unless it is empty, the chain config and every reward
/// balance.
#[derive(Serialize, Deserialize)]
struct Snapshot {
    height: u64,
    frontier: Option<BlocksFrontier>,
    chain_config: ChainConfig,
    rewards: Vec<(RewardAccountV2, RewardAmount)>,
}

/// The state after `leaf`, its parent's being `parent_state`, checked against the roots its header
/// commits to. Of the block tree it keeps only the frontier, which is all the next block needs.
async fn apply(
    node: &NodeState,
    parent_state: &ValidatedState,
    parent_leaf: &Leaf2,
    leaf: &Leaf2,
) -> anyhow::Result<ValidatedState> {
    let header = leaf.block_header();
    let mut parent_state = parent_state.clone();
    // A block of a newer version runs under its own chain config; the genesis file need not
    // record every upgrade the chain went through.
    if header.version() > parent_leaf.block_header().version() {
        parent_state.chain_config = header.chain_config();
    }
    let (mut state, ..) = parent_state
        .apply_header(
            node,
            &NullStateCatchup::default(),
            parent_leaf,
            header,
            header.version(),
            leaf.view_number(),
        )
        .await?;
    let height = header.height();
    ensure!(
        state.chain_config.commit() == header.chain_config().commit(),
        "the chain config after block {height} does not match its header"
    );
    ensure!(
        state.block_merkle_tree.commitment() == header.block_merkle_tree_root(),
        "the block tree after block {height} does not match its header"
    );
    ensure!(
        state.fee_merkle_tree.commitment() == header.fee_merkle_tree_root(),
        "the fee tree after block {height} does not match its header"
    );
    let rewards_match = match header.reward_merkle_tree_root() {
        Either::Left(root) => state.reward_merkle_tree_v1.commitment() == root,
        Either::Right(root) => state.reward_merkle_tree_v2.commitment() == root,
    };
    ensure!(
        rewards_match,
        "the reward tree after block {height} does not match its header"
    );
    if let Some(before_last) = state.block_merkle_tree.num_leaves().checked_sub(2) {
        // Only the leaf just appended can still be unforgotten before it.
        let _ = state.block_merkle_tree.forget(before_last);
    }
    Ok(state)
}

/// Hands membership what `leaf` tells it: an epoch root gives the stake table of the epoch two
/// later, and a transition block the DRB result of the next epoch.
async fn track_epoch(node: &NodeState, leaf: &Leaf2) -> anyhow::Result<()> {
    let epoch_height = node.epoch_height.unwrap_or_default();
    let height = leaf.height();
    let Some(epoch) = leaf.epoch(epoch_height) else {
        return Ok(());
    };
    let membership = &node.coordinator;
    let has_stake_table = |epoch| {
        use hotshot_types::traits::election::Membership as _;
        membership.membership().snapshot(epoch).is_some()
    };
    if is_epoch_root(height, epoch_height) && !has_stake_table(epoch + 2) {
        membership
            .add_epoch_root(leaf.block_header().clone())
            .await
            .map_err(|err| anyhow::anyhow!("{err}"))
            .with_context(|| format!("deriving the stake table of epoch {}", epoch + 2))?;
    }
    if is_transition_block(height, epoch_height)
        && let Some(drb) = leaf.next_drb_result
    {
        membership.supply_drb(epoch + 1, drb);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chain_store::{Item, Parts, mem::MemBackend};
    use hotshot_types::simple_certificate::QuorumCertificate2;
    use jf_merkle_tree_compat::AppendableMerkleTreeScheme;
    use versions::{NEW_PROTOCOL_VERSION, Upgrade};

    use super::*;

    /// A store holding the genesis block, and the state after it.
    async fn genesis(node: &NodeState) -> (ChainStore, ValidatedState) {
        let state = ValidatedState::genesis(node).0;
        let leaf = Leaf2::genesis(&state, node, NEW_PROTOCOL_VERSION).await;
        stored(leaf, state, node).await
    }

    /// A store holding the genesis block as if a block came before it, so that the block tree
    /// after it is not empty, and that state.
    async fn after_a_block(node: &NodeState) -> (ChainStore, ValidatedState) {
        let mut state = ValidatedState::genesis(node).0;
        let mut leaf = Leaf2::genesis(&state, node, NEW_PROTOCOL_VERSION).await;
        state
            .block_merkle_tree
            .push(leaf.block_header().commit())
            .unwrap();
        *leaf.block_header_mut().block_merkle_tree_root_mut() =
            state.block_merkle_tree.commitment();
        stored(leaf, state, node).await
    }

    async fn stored(
        leaf: Leaf2,
        state: ValidatedState,
        node: &NodeState,
    ) -> (ChainStore, ValidatedState) {
        let upgrade = Upgrade::trivial(NEW_PROTOCOL_VERSION);
        let item = Item {
            leaf,
            cert1: QuorumCertificate2::genesis(&state, node, upgrade).await,
        };
        let store = ChainStore::new(MemBackend::default());
        store.put(&item, &Parts::default()).await.unwrap();
        (store, state)
    }

    /// A restarted follower resumes from the snapshot with the state it stored.
    #[tokio::test]
    async fn resumes_from_the_snapshot() {
        let node = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        for (store, state) in [genesis(&node).await, after_a_block(&node).await] {
            resumes(store, state, node.clone()).await;
        }
    }

    async fn resumes(store: ChainStore, state: ValidatedState, node: NodeState) {
        let (first, _) = follower(store.clone(), node.clone());
        assert!(first.start().await.unwrap().is_none());

        first.snapshot(0, &state).await.unwrap();
        let (restarted, _) = follower(store, node);
        let (leaf, resumed) = restarted.start().await.unwrap().expect("a snapshot");
        assert_eq!(leaf.height(), 0);
        assert_eq!(
            resumed.block_merkle_tree.commitment(),
            state.block_merkle_tree.commitment()
        );
        assert_eq!(
            resumed.reward_merkle_tree_v2.commitment(),
            state.reward_merkle_tree_v2.commitment()
        );
        assert_eq!(resumed.chain_config.commit(), state.chain_config.commit());
        assert_eq!(
            resumed.fee_merkle_tree.num_leaves(),
            state.fee_merkle_tree.num_leaves()
        );
    }

    /// A follower that the store prunes past stops instead of waiting for a height it will not
    /// get.
    #[tokio::test]
    async fn stops_when_pruned_past() {
        let node = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        let (store, _) = genesis(&node).await;
        let (follower, states) = follower(store.clone(), node);
        let mut run = tokio::spawn(follower.run());
        tokio::time::timeout(Duration::from_secs(10), async {
            while states.newest() != Some(0) {
                sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("genesis is applied");
        store.prune_below(2).await.unwrap();
        tokio::time::timeout(Duration::from_secs(10), &mut run)
            .await
            .expect("the follower stops")
            .unwrap()
            .expect_err("it was pruned past");
    }

    /// A store pruned past genesis, without a snapshot, cannot be followed.
    #[tokio::test]
    async fn refuses_a_pruned_store_without_a_snapshot() {
        let node = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        let (store, _) = genesis(&node).await;
        store.prune_below(1).await.unwrap();
        let (pruned, _) = follower(store, node);
        pruned.start().await.expect_err("nothing to start from");
    }
}
