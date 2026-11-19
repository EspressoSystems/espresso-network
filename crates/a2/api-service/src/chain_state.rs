//! The state of the chain at recent heights, for consensus nodes catching up.
//!
//! The API service follows its store from genesis, in height order, as a consensus node follows the
//! chain: it applies each block's header to the state of its parent and checks the result against
//! the roots the header commits to, and it hands each epoch root and each DRB result to membership,
//! which needs both to know the leaders whose rewards the state pays at the end of an epoch. It
//! keeps the states of the newest heights, which is what a node that is behind asks for: it fetches
//! the state an API service has and applies the headers after it itself.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use anyhow::{Context, ensure};
use chain_store::ChainStore;
use consensus_node::null_catchup::NullStateCatchup;
use either::Either;
use espresso_types::{Leaf2, NodeState, ValidatedState};
use hotshot_types::{
    traits::states::ValidatedState as _,
    utils::{is_epoch_root, is_transition_block},
};
use jf_merkle_tree_compat::MerkleTreeScheme;
use parking_lot::RwLock;
use tokio::time::sleep;

/// How many of the newest heights the state is kept for.
const KEPT_HEIGHTS: u64 = 256;

/// How long to wait before applying a block again that could not be applied, for instance because
/// membership does not have the stake table of its epoch yet.
const RETRY_DELAY: Duration = Duration::from_secs(1);

/// The states of the newest heights the API service has applied.
#[derive(Debug, Default)]
pub struct ChainState {
    states: RwLock<BTreeMap<u64, Arc<ValidatedState>>>,
}

impl ChainState {
    /// The state after the block at `height`, if it is among the newest kept.
    pub fn at(&self, height: u64) -> Option<Arc<ValidatedState>> {
        self.states.read().get(&height).cloned()
    }

    /// The newest height whose state is kept.
    pub fn newest(&self) -> Option<u64> {
        self.states.read().keys().next_back().copied()
    }

    /// The newest state kept, with its height.
    pub fn latest(&self) -> Option<(u64, Arc<ValidatedState>)> {
        let states = self.states.read();
        let (height, state) = states.last_key_value()?;
        Some((*height, state.clone()))
    }

    fn keep(&self, height: u64, state: Arc<ValidatedState>) {
        let mut states = self.states.write();
        states.insert(height, state);
        while states.len() as u64 > KEPT_HEIGHTS {
            states.pop_first();
        }
    }
}

/// Follows `store` from genesis, applying each block to `chain`. Never returns.
pub async fn follow(chain: Arc<ChainState>, store: ChainStore, node: NodeState) {
    let mut changes = store.changes();
    let mut parent: Option<(Leaf2, Arc<ValidatedState>)> = None;
    let mut height = 0;
    loop {
        changes.borrow_and_update();
        let item = match store.item(height).await {
            Ok(Some(item)) => item,
            Ok(None) => {
                // Not stored yet; the ingester or the gap filler stores it.
                if changes.changed().await.is_err() {
                    return;
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
            None => Ok(ValidatedState::genesis(&node).0),
            Some((parent_leaf, parent_state)) => {
                apply(&node, parent_state, parent_leaf, &item.leaf).await
            },
        };
        let state = match state {
            Ok(state) => Arc::new(state),
            Err(err) => {
                tracing::warn!(height, "applying the block failed: {err:#}");
                sleep(RETRY_DELAY).await;
                continue;
            },
        };
        if let Err(err) = track_epoch(&node, &item.leaf).await {
            tracing::warn!(height, "membership did not take the block: {err:#}");
            sleep(RETRY_DELAY).await;
            continue;
        }
        chain.keep(height, state.clone());
        parent = Some((item.leaf, state));
        height += 1;
    }
}

/// The state after `leaf`, its parent's being `parent_state`, checked against the roots its header
/// commits to.
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
    let (state, ..) = parent_state
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
