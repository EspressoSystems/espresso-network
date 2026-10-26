use core::fmt::Debug;
use std::{ops::Range, sync::Arc, time::Duration};

use anyhow::{Context, bail, ensure};
use async_lock::Mutex;
use committable::Committable;
use either::Either;
use espresso_types::{
    BlockMerkleTree, EpochRewardsCalculator, FeeAccount, FeeMerkleTree, Leaf2, ValidatedState,
    traits::StateCatchup,
    v0_3::{ChainConfig, RewardMerkleTreeV1},
    v0_4::{Delta, PermittedRewardMerkleTreeV2},
};
use futures::{StreamExt, future::Future};
use hotshot_query_service::{
    availability::{self, AvailabilityDataSource},
    data_source::{Transaction, VersionedDataSource, storage::pruning::PrunedHeightDataSource},
    merklized_state::{MerklizedStateHeightPersistence, UpdateStateData},
    status::StatusDataSource,
    types::HeightIndexed,
};
use hotshot_types::{traits::ValidatedState as HotShotState, utils::is_last_block};
use jf_merkle_tree_compat::{
    LookupResult, MerkleTreeScheme, ToTraversalPath, UniversalMerkleTreeScheme,
};
use tokio::time::{sleep, timeout};
use vbs::version::Version;
use versions::{DRB_AND_HEADER_UPGRADE_VERSION, EPOCH_REWARD_VERSION, EPOCH_VERSION};

use crate::{
    NodeState, SeqTypes,
    api::{LightClientProvider, RewardMerkleTreeDataSource},
    catchup::{CatchupStorage, SingleAttemptCatchup},
    persistence::ChainConfigPersistence,
};

pub(crate) async fn compute_state_update(
    parent_state: &ValidatedState,
    instance: &NodeState,
    peers: &impl StateCatchup,
    parent_leaf: &Leaf2,
    proposed_leaf: &Leaf2,
) -> anyhow::Result<(ValidatedState, Delta)> {
    let header = proposed_leaf.block_header();

    let mut parent_state = parent_state.clone();

    // if the protocol has been upgraded, the new chain_config should be used
    // as the base chain config for the call to `apply_header`. This mirrors the
    // `apply_upgrade` step at the start of `apply_header`
    //
    // We need to do this here because this loop may need to process historical upgrades
    // that are no longer recorded in our genesis file. but it's safe, because this loop
    // only handles decided leaves
    if proposed_leaf.block_header().version() > parent_leaf.block_header().version() {
        parent_state.chain_config = proposed_leaf.block_header().chain_config()
    }

    let (state, delta, total_rewards_distributed) = parent_state
        .apply_header(
            instance,
            peers,
            parent_leaf,
            header,
            header.version(),
            proposed_leaf.view_number(),
        )
        .await?;

    // Check internal consistency.
    ensure!(
        state.chain_config.commit() == header.chain_config().commit(),
        "internal error! in-memory chain config {:?} does not match header {:?}",
        state.chain_config,
        header.chain_config(),
    );
    ensure!(
        state.block_merkle_tree.commitment() == header.block_merkle_tree_root(),
        "internal error! in-memory block tree {} does not match header {}",
        state.block_merkle_tree.commitment(),
        header.block_merkle_tree_root()
    );
    ensure!(
        state.fee_merkle_tree.commitment() == header.fee_merkle_tree_root(),
        "internal error! in-memory fee tree {} does not match header {}",
        state.fee_merkle_tree.commitment(),
        header.fee_merkle_tree_root()
    );

    match header.reward_merkle_tree_root() {
        Either::Left(v1_root) => {
            ensure!(
                state.reward_merkle_tree_v1.commitment() == v1_root,
                "internal error! in-memory v1 reward tree {} does not match header {}",
                state.reward_merkle_tree_v1.commitment(),
                v1_root
            )
        },
        Either::Right(v2_root) => {
            ensure!(
                state.reward_merkle_tree_v2.commitment() == v2_root,
                "internal error! in-memory v2 reward tree {} does not match header {}",
                state.reward_merkle_tree_v2.commitment(),
                v2_root
            )
        },
    }

    if header.version() >= DRB_AND_HEADER_UPGRADE_VERSION {
        let Some(actual_total) = total_rewards_distributed else {
            bail!(
                "internal error! total_rewards_distributed is None for version {:?}",
                header.version()
            );
        };

        let Some(proposed_total) = header.total_reward_distributed() else {
            bail!(
                "internal error! proposed header.total_reward_distributed() is None for version \
                 {:?}",
                header.version()
            );
        };

        ensure!(
            proposed_total == actual_total,
            "Total rewards mismatch: proposed header has {proposed_total} but actual total is \
             {actual_total}",
        );
    }

    Ok((state, delta))
}

async fn store_state_update(
    tx: &mut impl SequencerStateUpdate,
    block_number: u64,
    _version: Version,
    state: &ValidatedState,
    delta: &Delta,
) -> anyhow::Result<()> {
    let ValidatedState {
        fee_merkle_tree,
        block_merkle_tree,
        ..
    } = state;
    let Delta { fees_delta, .. } = delta;

    // Collect fee merkle tree proofs for batch insertion
    let fee_proofs: Vec<_> = fees_delta
        .iter()
        .map(|delta| {
            let proof = match fee_merkle_tree.universal_lookup(*delta) {
                LookupResult::Ok(_, proof) => proof,
                LookupResult::NotFound(proof) => proof,
                LookupResult::NotInMemory => bail!("missing merkle path for fee account {delta}"),
            };
            let path = FeeAccount::to_traversal_path(delta, fee_merkle_tree.height());
            Ok((proof, path))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    tracing::debug!(count = fee_proofs.len(), "inserting fee accounts in batch");
    UpdateStateData::<SeqTypes, FeeMerkleTree, { FeeMerkleTree::ARITY }>::insert_merkle_nodes_batch(
        tx,
        fee_proofs,
        block_number,
    )
    .await
    .context("failed to store fee merkle nodes")?;

    store_block_frontier(tx, block_number, block_merkle_tree).await
}

/// Store the frontier of `block_merkle_tree`, the block tree at `block_number`, as that snapshot's
/// block tree nodes.
async fn store_block_frontier(
    tx: &mut impl SequencerStateUpdate,
    block_number: u64,
    block_merkle_tree: &BlockMerkleTree,
) -> anyhow::Result<()> {
    let (_, proof) = block_merkle_tree
        .lookup(block_number - 1)
        .expect_ok()
        .context("getting blocks frontier")?;
    let path = <u64 as ToTraversalPath<{ BlockMerkleTree::ARITY }>>::to_traversal_path(
        &(block_number - 1),
        block_merkle_tree.height(),
    );

    tracing::debug!("inserting blocks frontier");
    UpdateStateData::<SeqTypes, BlockMerkleTree, { BlockMerkleTree::ARITY }>::insert_merkle_nodes(
        tx,
        proof,
        path,
        block_number,
    )
    .await
    .context("failed to store block merkle nodes")
}

#[tracing::instrument(
    skip_all,
    fields(
        node_id = instance.node_id,
        view = ?parent_leaf.view_number(),
        height = parent_leaf.height(),
    ),
)]
async fn update_state_storage<T>(
    parent_state: &ValidatedState,
    storage: &Arc<T>,
    instance: &NodeState,
    peers: &impl StateCatchup,
    parent_leaf: &Leaf2,
    proposed_leaf: &Leaf2,
) -> anyhow::Result<ValidatedState>
where
    T: SequencerStateDataSource,
    for<'a> T::Transaction<'a>: SequencerStateUpdate,
{
    let parent_chain_config = parent_state.chain_config;
    let block_number = proposed_leaf.height();
    let version = proposed_leaf.block_header().version();

    let (state, delta) =
        compute_state_update(parent_state, instance, peers, parent_leaf, proposed_leaf)
            .await
            .context("computing state update")?;

    let has_changed_accounts = version > EPOCH_VERSION && !delta.rewards_delta.is_empty();
    // For EPOCH_REWARD_VERSION+ we must persist the reward tree at every epoch
    // boundary, even when no rewards were distributed. During a V4→V5 upgrade
    // the first post upgrade epoch boundary skips rewards (the previous epoch's
    // header is pre-V5), leaving rewards_delta empty. Without saving here, the
    // tree would be missing from storage and catchup requests from peers or
    // subsequent epoch reward calculations would fail.
    //
    // Example: V4→V5 upgrade at block 9756, epoch_height=3000.
    // At block 12000 (first epoch boundary post upgrade), handle_epoch_rewards
    // skips rewards because the previous epoch's boundary (block 9000) is pre-V5,
    // so rewards_delta is empty. Without saving here, the tree is never persisted
    // at height 12000. Later at block 15000, the epoch 4 reward calculation calls
    // fetch_reward_merkle_tree_v2(height=12000) to catch up missing accounts and
    // fails because no tree exists in storage at that height.
    let is_epoch_boundary = version >= EPOCH_REWARD_VERSION
        && is_last_block(
            block_number,
            instance
                .epoch_height
                .expect("epoch_height should be set for version > V3"),
        );

    if has_changed_accounts || is_epoch_boundary {
        storage
            .save_and_gc_reward_tree_v2(
                instance,
                block_number,
                version,
                &state.reward_merkle_tree_v2,
            )
            .await
            .context("failed to save and gc reward merkle tree v2")?;
    }

    storage
        .persist_reward_proofs(instance, block_number, version)
        .await
        .context("failed to persist reward proofs")?;

    // The nodes and the new head commit together, so no node version ever exists above the head:
    // the head snapshot is the newest version of everything it touches, which is what keeps it
    // intact under pruning and lets the loop resume from it after a crash.
    tracing::debug!("storing state update");
    let mut tx = storage
        .write()
        .await
        .context("opening transaction for state update")?;

    store_state_update(&mut tx, block_number, version, &state, &delta).await?;

    if parent_chain_config != state.chain_config {
        let cf = state
            .chain_config
            .resolve()
            .context("failed to resolve to chain config")?;

        tx.insert_chain_config(cf).await?;
    }

    tracing::debug!(block_number, "updating state height");
    UpdateStateData::<SeqTypes, _, { BlockMerkleTree::ARITY }>::set_last_state_height(
        &mut tx,
        block_number as usize,
    )
    .await
    .context("setting state height")?;

    tx.commit().await?;

    Ok(state)
}

async fn store_genesis_state<S>(
    storage: &S,
    chain_config: ChainConfig,
    state: &ValidatedState,
) -> anyhow::Result<()>
where
    S: SequencerStateDataSource,
    for<'a> S::Transaction<'a>: SequencerStateUpdate,
{
    ensure!(
        state.block_merkle_tree.num_leaves() == 0,
        "genesis state with non-empty block tree is unsupported"
    );

    let mut tx = storage
        .write()
        .await
        .context("starting transaction for genesis state")?;

    // Insert fee merkle tree nodes
    for (account, _) in state.fee_merkle_tree.iter() {
        let proof = match state.fee_merkle_tree.universal_lookup(account) {
            LookupResult::Ok(_, proof) => proof,
            LookupResult::NotFound(proof) => proof,
            LookupResult::NotInMemory => bail!("missing merkle path for fee account {account}"),
        };
        let path: Vec<usize> =
            <FeeAccount as ToTraversalPath<{ FeeMerkleTree::ARITY }>>::to_traversal_path(
                account,
                state.fee_merkle_tree.height(),
            );

        UpdateStateData::<SeqTypes, FeeMerkleTree, { FeeMerkleTree::ARITY }>::insert_merkle_nodes(
            &mut tx, proof, path, 0,
        )
        .await
        .context("failed to store fee merkle nodes")?;
    }

    tx.insert_chain_config(chain_config).await?;

    tx.commit().await?;

    // Store the genesis reward tree at height 0 so catchup can find it.
    storage
        .save_reward_tree_v2(0, &state.reward_merkle_tree_v2)
        .await
        .context("storing genesis reward merkle tree")?;

    Ok(())
}

/// How long the loop waits on the local leaf stream before checking whether the data pruner has
/// passed the height it is waiting for.
const PRUNED_LEAF_CHECK_INTERVAL: Duration = Duration::from_secs(10);

/// How long the loop waits before it retries a step of its start that failed, such as a try to
/// start from a recent epoch boundary.
const BOUNDARY_RETRY_INTERVAL: Duration = Duration::from_secs(10);

/// How long the loop waits on a local catchup read. Consensus cuts local reads short, so that it
/// can still vote in time. The loop has no such deadline, while peers may have pruned the heights
/// it reads, so a slow read cut short on every try would stall it.
const LOCAL_CATCHUP_TIMEOUT: Duration = Duration::from_secs(60);

/// Where the state loop starts when it has no state of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EmptyStateStart {
    /// From a snapshot at a recent epoch boundary, from peers, for a pruned node, which serves
    /// recent state only. Fast, but the trees stay incomplete for good: they have no paths for the
    /// blocks below the boundary, and no fee account the loop has not touched since.
    RecentBoundary,
    /// From genesis, on leaves from peers, for archive nodes and nodes without a pruner. Slow, but
    /// the only way to complete trees.
    Genesis,
}

#[tracing::instrument(skip_all)]
pub(crate) async fn update_state_storage_loop<T>(
    storage: Arc<T>,
    instance: impl Future<Output = NodeState>,
    provider: Arc<LightClientProvider>,
    empty_state_start: EmptyStateStart,
) -> anyhow::Result<()>
where
    T: SequencerStateDataSource,
    for<'a> T::Transaction<'a>: SequencerStateUpdate,
{
    let mut instance = instance.await;
    // Use a separate rewards calculator for the state loop so it doesn't
    // interfere with consensus, which may be on a very different epoch.
    instance.epoch_rewards_calculator = Arc::new(Mutex::new(EpochRewardsCalculator::new()));
    // The loop can wait longer on its own database than consensus can.
    if let Some(catchup) = instance
        .state_catchup
        .with_local_timeout(LOCAL_CATCHUP_TIMEOUT)
    {
        instance.state_catchup = catchup;
    }

    // Resume from the newest snapshot in storage. The loop cannot start anywhere else:
    // `from_header` gives it a bare state, and its catchup fills in the block frontier and any fee
    // account it has not seen from the parent's snapshot in this database, which exists only up to
    // the head. The state pruner in turn never advances to the head, so that snapshot stays
    // readable however far behind the chain the loop has fallen.
    let mut last_height = retry_until_ok("loading the state head", || async {
        Ok(storage.get_last_state_height().await? as u64)
    })
    .await;
    // With no state of its own on a pruned database, as on a fresh pruned node, the loop would
    // replay the whole chain from genesis, mostly on leaves from peers. A pruned node serves recent
    // state only, so give it a recent snapshot to resume from instead.
    if last_height == 0
        && empty_state_start == EmptyStateStart::RecentBoundary
        && let Some(boundary) = seed_boundary_snapshot(&storage, &instance, &provider).await
    {
        last_height = boundary;
    }
    if last_height > 0 && empty_state_start == EmptyStateStart::Genesis {
        report_incomplete_trees(&*storage).await;
    }
    let current_height = storage.block_height().await.ok();
    tracing::info!(
        node_id = instance.node_id,
        last_height,
        ?current_height,
        "updating state storage"
    );

    let parent_leaf = leaf_at(&storage, &provider, last_height).await;
    let mut parent_state = ValidatedState::from_header(parent_leaf.block_header());

    // Seed the parent's reward tree from storage.
    //
    // `from_header` starts the reward tree empty, and the epoch boundary only
    // repopulates it when the previous epoch actually distributed rewards. If the
    // previous epoch predates rewards (e.g. the first boundary after a V4→V5
    // upgrade, where the previous epoch is pre-V5), `handle_epoch_rewards` returns
    // zero and passes the empty tree through unchanged. So after a restart in such
    // an epoch the tree stays empty until the next boundary, where the save in
    // `update_state_storage` fails as it serializes the tree as full key-value pairs
    // and bails because the accounts are missing.
    //
    // Load the parent's tree from storage to avoid this. Doing it once is enough
    // every later iteration carries the tree forward in `parent_state`.
    if parent_leaf.block_header().version() > EPOCH_VERSION && parent_leaf.height() > 0 {
        parent_state.reward_merkle_tree_v2 =
            retry_until_ok("loading the parent's reward tree", || {
                load_parent_reward_tree(&*storage, &parent_leaf)
            })
            .await
            .tree;
    }

    if last_height == 0 {
        // If the last height is 0, we need to insert the genesis state, since this state is
        // never the result of a state update and thus is not inserted in the loop below.
        tracing::info!("storing genesis merklized state");
        // Every write is an upsert, so a retry after a partial store is safe.
        retry_until_ok("storing the genesis state", || {
            store_genesis_state(&*storage, instance.chain_config, &instance.genesis_state)
        })
        .await;
    }

    follow_leaves(&storage, &instance, &provider, parent_leaf, parent_state).await;
    Ok(())
}

/// Archive nodes and nodes without a pruner are meant to have complete trees, which a state that
/// started from a boundary snapshot, as on a node that was pruned before, never gets: the loop only
/// moves forward. So say so.
async fn report_incomplete_trees<T>(storage: &T)
where
    T: SequencerStateDataSource,
{
    match storage.load_state_start_height().await {
        Ok(Some(start)) => tracing::error!(
            start,
            "merklized state started from a boundary snapshot, not genesis, so its trees are \
             incomplete"
        ),
        Ok(None) => {},
        Err(err) => tracing::warn!(
            err = %format_args!("{err:#}"),
            "failed to load the state start height"
        ),
    }
}

/// Run `f` until it succeeds. An error would end the loop for good, possibly after a seed that
/// waited minutes for peers, so the steps of its start retry, as the leaves it applies do.
async fn retry_until_ok<R, Fut>(step: &str, mut f: impl FnMut() -> Fut) -> R
where
    Fut: Future<Output = anyhow::Result<R>>,
{
    loop {
        match f().await {
            Ok(res) => return res,
            Err(err) => tracing::error!(
                step,
                err = %format_args!("{err:#}"),
                "state loop start step failed, retrying"
            ),
        }
        sleep(BOUNDARY_RETRY_INTERVAL).await;
    }
}

/// The reward tree of `parent_leaf`, from storage.
async fn load_parent_reward_tree<T>(
    storage: &T,
    parent_leaf: &Leaf2,
) -> anyhow::Result<PermittedRewardMerkleTreeV2>
where
    T: SequencerStateDataSource,
{
    // The tree is only written at epoch boundaries, so the parent height usually has no row; fall
    // back to the most recent tree at or below it.
    let tree = match storage
        .load_reward_merkle_tree_v2(parent_leaf.height())
        .await
    {
        Ok(tree) => tree,
        Err(_) => storage
            .load_latest_reward_merkle_tree_v2(parent_leaf.height())
            .await
            .context("failed to load RewardMerkleTreeV2 for the previous height")?,
    };

    // The fallback returns the latest tree at or below the parent height, which is the parent's
    // tree only if their roots match (they do within an epoch). Verify against the root the parent
    // header commits to.
    let parent_reward_root = parent_leaf
        .block_header()
        .reward_merkle_tree_root()
        .right()
        .context("V5+ parent header must have a v2 reward root")?;
    ensure!(
        tree.tree.commitment() == parent_reward_root,
        "loaded reward tree root {} does not match parent header {parent_reward_root}",
        tree.tree.commitment(),
    );
    Ok(tree)
}

/// Store a snapshot at a recent epoch boundary for a loop with no state of its own, and return the
/// boundary, which the loop then resumes from as after a restart.
///
/// The boundary is the newest one at or below the local tip, once the node holds a block above the
/// data pruned height, or above genesis before the data pruner has run; a fresh node holds none
/// until consensus decides one. It has to be recent: the reward tree is stored only at boundaries,
/// and every node garbage collects all but the last few, so peers have no tree for an older
/// snapshot.
///
/// The snapshot holds the block frontier and the reward tree, fetched from peers and checked
/// against the boundary header. Peers serve fee accounts only by address, and the loop cannot know
/// them all, so the fee tree starts empty and the catchup in [`apply_leaf`] fetches each account
/// from peers when the loop first needs it. From V0.6 on no block reads one, so it stays empty.
///
/// Returns `None`, which leaves the loop to replay from genesis, when there is no boundary yet, or
/// when it predates epoch rewards and so has no reward tree stored at it.
async fn seed_boundary_snapshot<T>(
    storage: &Arc<T>,
    instance: &NodeState,
    provider: &LightClientProvider,
) -> Option<u64>
where
    T: SequencerStateDataSource,
    for<'a> T::Transaction<'a>: SequencerStateUpdate,
{
    let epoch_height = instance.epoch_height.filter(|height| *height > 0)?;
    let pruned = retry_until_ok("loading the data pruned height", || {
        storage.load_pruned_height()
    })
    .await
    .unwrap_or(0);
    // Each request is sent once, because this loop retries: peers never answer a request for a
    // boundary they have pruned, and the next try then picks the boundary again from the tip. Each
    // try passes its number on, so that peers get longer to answer.
    let mut retry = 0;
    let (leaf, state) = loop {
        let tip = local_tip_above(&**storage, pruned).await;
        let boundary = tip - tip % epoch_height;
        if boundary == 0 {
            return None;
        }
        let leaf = leaf_at(storage, provider, boundary).await;
        if leaf.block_header().version() < EPOCH_REWARD_VERSION {
            return None;
        }
        let catchup = SingleAttemptCatchup::new(instance.state_catchup.clone(), retry);
        match fetch_boundary_state(&catchup, instance, &leaf).await {
            Ok(state) => break (leaf, state),
            // Peers may not have reached the boundary yet, or, if the local tip is stale, may
            // have pruned its frontier or garbage collected its reward tree. Try again from
            // wherever the tip is then.
            Err(err) => {
                tracing::warn!(
                    boundary,
                    retry,
                    err = %format_args!("{err:#}"),
                    "fetching the state at the epoch boundary failed, retrying"
                );
                retry += 1;
                sleep(BOUNDARY_RETRY_INTERVAL).await;
            },
        }
    };
    let height = leaf.height();
    // Every write is an upsert, so a retry after a partial store is safe.
    retry_until_ok(
        &format!("storing the boundary snapshot at height {height}"),
        || store_boundary_snapshot(&**storage, height, &state),
    )
    .await;
    tracing::info!(
        height,
        pruned,
        "state loop starts from a snapshot at a recent epoch boundary"
    );
    Some(height)
}

/// The height of the newest block in local storage, once it is above `pruned`.
async fn local_tip_above<T>(storage: &T, pruned: u64) -> u64
where
    T: SequencerStateDataSource,
{
    loop {
        match storage.block_height().await {
            Ok(height) if height as u64 > pruned + 1 => return height as u64 - 1,
            Ok(height) => tracing::info!(
                height,
                pruned,
                "no block above the data pruned height yet; waiting for one"
            ),
            Err(err) => tracing::warn!(
                err = %format_args!("{err:#}"),
                "failed to load the local block height"
            ),
        }
        sleep(BOUNDARY_RETRY_INTERVAL).await;
    }
}

/// The state at the epoch boundary `leaf`, from `catchup`: the block frontier and the whole reward
/// tree, each checked against its root in the boundary header.
async fn fetch_boundary_state(
    catchup: &impl StateCatchup,
    instance: &NodeState,
    leaf: &Leaf2,
) -> anyhow::Result<ValidatedState> {
    let header = leaf.block_header();
    let mut state = ValidatedState::from_header(header);
    catchup
        .remember_blocks_merkle_tree(
            instance,
            leaf.height(),
            leaf.view_number(),
            &mut state.block_merkle_tree,
        )
        .await
        .context("fetching the block frontier")?;
    let reward_root = header
        .reward_merkle_tree_root()
        .right()
        .context("V5+ header must have a v2 reward root")?;
    state.reward_merkle_tree_v2 = catchup
        .fetch_reward_merkle_tree_v2(
            leaf.height(),
            leaf.view_number(),
            reward_root,
            Default::default(),
        )
        .await
        .context("fetching the reward tree")?
        .tree;
    Ok(state)
}

/// Store `state`, the state at the epoch boundary `height`, as the loop's head snapshot. The loop
/// keeps nothing of `state`: on every start it rebuilds the head's state from the header and fills
/// it from this database, so the head needs what a state update stores:
/// - The reward tree. The loop reads it from here only, so it goes first: a head without it would
///   fail every start.
/// - The block frontier. Without it, the loop would fetch it from peers again, and block proofs at
///   the head would fail.
/// - The chain config. The loop takes it from the header, but only the loop stores it, and chain
///   config catchup reads it from here.
///
/// No state is stored below the boundary, so the state pruned height goes just below it: queries
/// for lower heights then answer that the state is pruned, as on a node that pruned it. The
/// boundary is also recorded as where the state starts, so that a node meant to have complete
/// trees can tell that it does not.
async fn store_boundary_snapshot<T>(
    storage: &T,
    height: u64,
    state: &ValidatedState,
) -> anyhow::Result<()>
where
    T: SequencerStateDataSource,
    for<'a> T::Transaction<'a>: SequencerStateUpdate,
{
    storage
        .save_reward_tree_v2(height, &state.reward_merkle_tree_v2)
        .await
        .context("storing the boundary reward tree")?;

    let mut tx = storage
        .write()
        .await
        .context("opening transaction for the boundary snapshot")?;
    store_block_frontier(&mut tx, height, &state.block_merkle_tree).await?;
    if let Some(chain_config) = state.chain_config.resolve() {
        tx.insert_chain_config(chain_config).await?;
    }
    UpdateStateData::<SeqTypes, _, { BlockMerkleTree::ARITY }>::set_last_state_height(
        &mut tx,
        height as usize,
    )
    .await
    .context("setting state height")?;
    UpdateStateData::<SeqTypes, _, { BlockMerkleTree::ARITY }>::set_state_pruned_height(
        &mut tx,
        height - 1,
    )
    .await
    .context("setting state pruned height")?;
    UpdateStateData::<SeqTypes, _, { BlockMerkleTree::ARITY }>::set_state_start_height(
        &mut tx, height,
    )
    .await
    .context("setting state start height")?;
    tx.commit().await
}

/// Apply every leaf above `parent_leaf` as it arrives. Leaves come from the local stream, except
/// those the data pruner has already deleted: the availability layer never fetches a leaf at or
/// below the data pruned height, so the stream would wait for one forever. Those come from peers,
/// in runs. Returns when the stream ends, or when a leaf from peers does not extend the parent.
async fn follow_leaves<T>(
    storage: &Arc<T>,
    instance: &NodeState,
    provider: &LightClientProvider,
    mut parent_leaf: Leaf2,
    mut parent_state: ValidatedState,
) where
    T: SequencerStateDataSource,
    for<'a> T::Transaction<'a>: SequencerStateUpdate,
{
    // Peers serve at most this many leaves per range request.
    let limit = availability::Options::default().small_object_range_limit as u64;
    let mut next_height = parent_leaf.height() + 1;
    loop {
        while let Some(pruned) = pruned_past(&**storage, next_height).await {
            let end = (next_height + limit).min(pruned + 1);
            tracing::info!(
                next_height,
                end,
                pruned,
                "fetching leaves below the data pruned height from peers"
            );
            for leaf in fetch_pruned_leaves(provider, next_height..end).await {
                // The leaves are verified, so they are the decided leaves at these heights, and a
                // refetch returns them again. A mismatch means the loop's parent is wrong, which
                // no retry fixes.
                if leaf.parent_commitment() != parent_leaf.commit() {
                    tracing::error!(
                        height = next_height,
                        parent = %parent_leaf.commit(),
                        leaf_parent = %leaf.parent_commitment(),
                        "fetched leaf does not extend the state loop's parent; stopping the loop"
                    );
                    return;
                }
                apply_leaf(storage, instance, &mut parent_leaf, &mut parent_state, leaf).await;
                next_height += 1;
            }
        }

        let mut leaves = storage.subscribe_leaves(next_height as usize).await;
        loop {
            match timeout(PRUNED_LEAF_CHECK_INTERVAL, leaves.next()).await {
                Ok(Some(leaf)) => {
                    next_height = leaf.height() + 1;
                    apply_leaf(
                        storage,
                        instance,
                        &mut parent_leaf,
                        &mut parent_state,
                        leaf.leaf().clone(),
                    )
                    .await;
                },
                Ok(None) => return,
                // Still waiting. If the data pruner has passed this height in the meantime, the
                // stream will never deliver it.
                Err(_) => {
                    if pruned_past(&**storage, next_height).await.is_some() {
                        break;
                    }
                },
            }
        }
    }
}

/// The data pruned height when it is at or above `height`. The leaf and header there are out of
/// local reach: the pruner deleted them, and the availability layer never fetches below its
/// marker. A failed read counts as not pruned; every caller asks again before long, and the loop
/// must not end over a transient database error.
async fn pruned_past<T>(storage: &T, height: u64) -> Option<u64>
where
    T: SequencerStateDataSource,
{
    match storage.load_pruned_height().await {
        Ok(pruned) => pruned.filter(|pruned| height <= *pruned),
        Err(err) => {
            tracing::warn!(height, "failed to load the data pruned height: {err:#}");
            None
        },
    }
}

/// The leaf at `height`, from local storage, or from peers once the data pruner has deleted it.
async fn leaf_at<T>(storage: &Arc<T>, provider: &LightClientProvider, height: u64) -> Leaf2
where
    T: SequencerStateDataSource,
{
    loop {
        if let Some(pruned) = pruned_past(&**storage, height).await {
            tracing::warn!(
                height,
                pruned,
                "leaf is at or below the data pruned height; fetching it from peers"
            );
            return fetch_pruned_leaves(provider, height..height + 1)
                .await
                .pop()
                .expect("a one-height range yields exactly one leaf");
        }
        let local = AvailabilityDataSource::get_leaf(&**storage, height as usize).await;
        // On a timeout the data pruner may have passed this height in the meantime, so check
        // again before waiting further.
        if let Ok(leaf) = timeout(PRUNED_LEAF_CHECK_INTERVAL, local).await {
            return leaf.leaf().clone();
        }
        tracing::info!(
            height,
            "leaf is not in local storage yet; still waiting for it"
        );
    }
}

/// The leaves at `heights`, which the data pruned height says are gone, from the query module's
/// peers through its light client, retrying until they arrive. The light client verifies leaves
/// of every protocol version, and one proof covers the run: it proves the last leaf final and
/// chains the rest to it. The loop needs each leaf once, so the leaves are stored neither in this
/// database, where the pruner would only delete them again, nor in the light client's cache.
async fn fetch_pruned_leaves(provider: &LightClientProvider, heights: Range<u64>) -> Vec<Leaf2> {
    let start = heights.start as usize;
    let end = heights.end as usize;
    let mut delay = Duration::from_secs(1);
    loop {
        if let Some(light_client) = provider.light_client().await {
            match light_client
                .fetch_leaves_in_range_uncached(start, end)
                .await
            {
                Ok(leaves) => return leaves.into_iter().map(|leaf| leaf.leaf().clone()).collect(),
                Err(err) => tracing::warn!(
                    ?heights,
                    ?delay,
                    err = %format_args!("{err:#}"),
                    "fetching leaves failed, retrying"
                ),
            }
        }
        sleep(delay).await;
        delay = (delay * 2).min(Duration::from_secs(60));
    }
}

/// Apply `leaf` to the merklized state, retrying until it is stored, then make it the parent.
async fn apply_leaf<T>(
    storage: &Arc<T>,
    instance: &NodeState,
    parent_leaf: &mut Leaf2,
    parent_state: &mut ValidatedState,
    leaf: Leaf2,
) where
    T: SequencerStateDataSource,
    for<'a> T::Transaction<'a>: SequencerStateUpdate,
{
    // The node's catchup races this database, through the node's SQL persistence, against peers,
    // which serve what the parent snapshot lacks: a loop that started from a boundary snapshot
    // holds only the fee accounts it has touched since. The parent snapshot is the state head,
    // which this database always keeps, while peers may have pruned it. When the data pruner has
    // deleted the parent's leaf and header, the local read finds the snapshot by the roots in the
    // parent state. Each request is sent once, because the loop below retries: peers never answer
    // a request for a height they have pruned, and the next try then reads this database again.
    // Each try passes its number on, so that peers get longer to answer, as they would on retries
    // of the request itself.
    for retry in 0.. {
        tracing::debug!(
            height = leaf.height(),
            node_id = instance.node_id,
            ?leaf,
            "updating persistent merklized state"
        );
        match update_state_storage(
            parent_state,
            storage,
            instance,
            &SingleAttemptCatchup::new(instance.state_catchup.clone(), retry),
            parent_leaf,
            &leaf,
        )
        .await
        {
            Ok(state) => {
                *parent_leaf = leaf;
                *parent_state = state;
                return;
            },
            Err(err) => {
                tracing::error!(height = leaf.height(), "failed to update state: {err:#}");
                // If we fail, delay for a second and retry.
                sleep(Duration::from_secs(1)).await;
            },
        }
    }
}

pub(crate) trait SequencerStateDataSource:
    'static
    + Debug
    + AvailabilityDataSource<SeqTypes>
    + StatusDataSource
    + VersionedDataSource
    + CatchupStorage
    + RewardMerkleTreeDataSource
    + PrunedHeightDataSource
    + MerklizedStateHeightPersistence
{
}

impl<T> SequencerStateDataSource for T where
    T: 'static
        + Debug
        + AvailabilityDataSource<SeqTypes>
        + StatusDataSource
        + VersionedDataSource
        + CatchupStorage
        + RewardMerkleTreeDataSource
        + PrunedHeightDataSource
        + MerklizedStateHeightPersistence
{
}

pub(crate) trait SequencerStateUpdate:
    Transaction
    + UpdateStateData<SeqTypes, FeeMerkleTree, { FeeMerkleTree::ARITY }>
    + UpdateStateData<SeqTypes, BlockMerkleTree, { BlockMerkleTree::ARITY }>
    + UpdateStateData<SeqTypes, RewardMerkleTreeV1, { RewardMerkleTreeV1::ARITY }>
    + ChainConfigPersistence
{
}

impl<T> SequencerStateUpdate for T where
    T: Transaction
        + UpdateStateData<SeqTypes, FeeMerkleTree, { FeeMerkleTree::ARITY }>
        + UpdateStateData<SeqTypes, BlockMerkleTree, { BlockMerkleTree::ARITY }>
        + UpdateStateData<SeqTypes, RewardMerkleTreeV1, { RewardMerkleTreeV1::ARITY }>
        + ChainConfigPersistence
{
}
