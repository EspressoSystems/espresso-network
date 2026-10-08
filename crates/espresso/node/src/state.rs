use core::fmt::Debug;
use std::{sync::Arc, time::Duration};

use anyhow::{Context, bail, ensure};
use async_lock::Mutex;
use committable::{Commitment, Committable};
use either::Either;
use espresso_types::{
    BackoffParams, BlockMerkleTree, EpochRewardsCalculator, FeeAccount, FeeMerkleTree, Leaf2,
    ValidatedState,
    traits::StateCatchup,
    v0_3::{ChainConfig, RewardMerkleTreeV1},
    v0_4::Delta,
};
use futures::{FutureExt, StreamExt, future::Future};
use hotshot::traits::ValidatedState as HotShotState;
use hotshot_query_service::{
    availability::AvailabilityDataSource,
    data_source::{Transaction, VersionedDataSource, storage::pruning::PrunedHeightDataSource},
    merklized_state::{MerklizedStateHeightPersistence, UpdateStateData},
    status::StatusDataSource,
    types::HeightIndexed,
};
use hotshot_types::utils::is_last_block;
use jf_merkle_tree_compat::{
    LookupResult, MerkleTreeScheme, ToTraversalPath, UniversalMerkleTreeScheme,
};
use tokio::time::{sleep, timeout};
use vbs::version::Version;
use versions::{DRB_AND_HEADER_UPGRADE_VERSION, EPOCH_REWARD_VERSION, EPOCH_VERSION};

use crate::{
    NodeState, SeqTypes,
    api::{RewardMerkleTreeDataSource, RewardMerkleTreeV2Data},
    catchup::CatchupStorage,
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

    // Insert block merkle tree nodes
    let (_, proof) = block_merkle_tree
        .lookup(block_number - 1)
        .expect_ok()
        .context("getting blocks frontier")?;
    let path = <u64 as ToTraversalPath<{ BlockMerkleTree::ARITY }>>::to_traversal_path(
        &(block_number - 1),
        block_merkle_tree.height(),
    );

    {
        tracing::debug!("inserting blocks frontier");
        UpdateStateData::<SeqTypes, BlockMerkleTree, { BlockMerkleTree::ARITY }>::insert_merkle_nodes(
            tx,
            proof,
            path,
            block_number,
        )
        .await
        .context("failed to store block merkle nodes")?;
    }

    Ok(())
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
    let tree_data: RewardMerkleTreeV2Data = (&state.reward_merkle_tree_v2)
        .try_into()
        .context("serializing genesis reward tree")?;
    let tree_bytes = bincode::serialize(&tree_data).context("serializing genesis reward tree")?;
    storage
        .persist_tree(0, tree_bytes)
        .await
        .context("storing genesis reward merkle tree")?;

    Ok(())
}

/// How long the loop waits on the local leaf stream before checking whether the data pruner has
/// passed the height it is waiting for.
const PRUNED_LEAF_CHECK_INTERVAL: Duration = Duration::from_secs(10);

/// How many leaves fetched from peers go by between progress log lines.
const PRUNED_LEAF_PROGRESS_EVERY: u64 = 1000;

#[tracing::instrument(skip_all)]
pub(crate) async fn update_state_storage_loop<T>(
    storage: Arc<T>,
    instance: impl Future<Output = NodeState>,
) -> anyhow::Result<()>
where
    T: SequencerStateDataSource,
    for<'a> T::Transaction<'a>: SequencerStateUpdate,
{
    let mut instance = instance.await;
    // Use a separate rewards calculator for the state loop so it doesn't
    // interfere with consensus, which may be on a very different epoch.
    instance.epoch_rewards_calculator = Arc::new(Mutex::new(EpochRewardsCalculator::new()));

    // Resume from the newest snapshot in storage. The loop cannot start anywhere else:
    // `from_header` gives it a bare state, and its catchup fills in the block frontier and any fee
    // account it has not seen from the parent's snapshot, which exists only up to the head. The
    // state pruner in turn never advances to the head, so that snapshot stays readable however
    // far behind the chain the loop has fallen.
    let last_height = storage.get_last_state_height().await?;
    let current_height = storage.block_height().await?;
    tracing::info!(
        node_id = instance.node_id,
        last_height,
        current_height,
        "updating state storage"
    );

    let parent_leaf = leaf_at(&storage, &instance, last_height as u64).await;
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
        // The tree is only written at epoch boundaries, so the parent height
        // usually has no row; fall back to the most recent tree at or below it.
        let reward_merkle_tree_v2 = match storage
            .load_reward_merkle_tree_v2(parent_leaf.height())
            .await
        {
            Ok(tree) => tree,
            Err(_) => storage
                .load_latest_reward_merkle_tree_v2(parent_leaf.height())
                .await
                .context(
                    "Error starting the state storage update loop: failed to load \
                     RewardMerkleTreeV2 for the previous height",
                )?,
        };

        // The fallback returns the latest tree at or below the parent height,
        // which is the parent's tree only if their roots match (they do within an
        // epoch). Verify against the root the parent header commits to.
        let parent_reward_root = parent_leaf
            .block_header()
            .reward_merkle_tree_root()
            .right()
            .context("V5+ parent header must have a v2 reward root")?;
        ensure!(
            reward_merkle_tree_v2.tree.commitment() == parent_reward_root,
            "loaded reward tree root {} does not match parent header {parent_reward_root}",
            reward_merkle_tree_v2.tree.commitment(),
        );

        parent_state.reward_merkle_tree_v2 = reward_merkle_tree_v2.tree;
    }

    if last_height == 0 {
        // If the last height is 0, we need to insert the genesis state, since this state is
        // never the result of a state update and thus is not inserted in the loop below.
        tracing::info!("storing genesis merklized state");
        store_genesis_state(&*storage, instance.chain_config, &instance.genesis_state)
            .await
            .context("storing genesis state")?;
    }

    follow_leaves(&storage, &instance, parent_leaf, parent_state).await
}

/// Apply every leaf above `parent_leaf` as it arrives. Leaves come from the local stream, except
/// those the data pruner has already deleted: the availability layer never fetches a leaf at or
/// below the data pruned height, so the stream would wait for one forever. Those come from peers,
/// one at a time.
async fn follow_leaves<T>(
    storage: &Arc<T>,
    instance: &NodeState,
    mut parent_leaf: Leaf2,
    mut parent_state: ValidatedState,
) -> anyhow::Result<()>
where
    T: SequencerStateDataSource,
    for<'a> T::Transaction<'a>: SequencerStateUpdate,
{
    let mut next_height = parent_leaf.height() + 1;
    loop {
        let mut fetched = 0u64;
        while let Some(pruned) = pruned_past(&**storage, next_height).await {
            if fetched == 0 {
                tracing::warn!(
                    next_height,
                    pruned,
                    "leaves are at or below the data pruned height; fetching them from peers"
                );
            }
            let leaf = fetch_pruned_leaf(instance, next_height, Some(parent_leaf.commit())).await;
            apply_leaf(storage, instance, &mut parent_leaf, &mut parent_state, leaf).await;
            next_height += 1;
            fetched += 1;
            if fetched.is_multiple_of(PRUNED_LEAF_PROGRESS_EVERY) {
                tracing::info!(
                    next_height,
                    pruned,
                    fetched,
                    "still fetching leaves from peers"
                );
            }
        }
        if fetched > 0 {
            tracing::info!(
                next_height,
                fetched,
                "past the data pruned height; back on the local leaf stream"
            );
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
                Ok(None) => return Ok(()),
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
async fn leaf_at<T>(storage: &Arc<T>, instance: &NodeState, height: u64) -> Leaf2
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
            return fetch_pruned_leaf(instance, height, None).await;
        }
        let local = async {
            AvailabilityDataSource::get_leaf(&**storage, height as usize)
                .await
                .await
        };
        // On a timeout the data pruner may have passed this height in the meantime, so check
        // again before waiting further.
        if let Ok(leaf) = timeout(PRUNED_LEAF_CHECK_INTERVAL, local).await {
            return leaf.leaf().clone();
        }
    }
}

/// Fetch the leaf at `height`, which the data pruned height says is gone, retrying until it
/// arrives. The node's catchup reads this database first, which still has the leaf when the pruner
/// has stamped the height but not deleted it yet, and otherwise asks peers. They keep leaves for
/// their own retention, and the node already depends on them for the epoch roots of the same
/// stretch of chain. A fetched leaf is not stored, since the loop needs it once and the pruner
/// would only delete it again, and when `parent` is given its parent link has to match; a leaf
/// that fails that check is retried like a failed fetch rather than applied.
async fn fetch_pruned_leaf(
    instance: &NodeState,
    height: u64,
    parent: Option<Commitment<Leaf2>>,
) -> Leaf2 {
    tracing::debug!(height, "fetching leaf below the data pruned height");
    let catchup = instance.state_catchup.clone();
    let coordinator = instance.coordinator.clone();
    loop {
        let fetched = BackoffParams::default()
            .retry((), |_, retry| {
                let catchup = catchup.clone();
                let coordinator = coordinator.clone();
                async move {
                    let leaf = catchup.try_fetch_leaf(retry, coordinator, height).await?;
                    if let Some(parent) = parent {
                        ensure!(
                            leaf.parent_commitment() == parent,
                            "fetched leaf {height} has parent {} but the loop's parent is {parent}",
                            leaf.parent_commitment()
                        );
                    }
                    Ok(leaf)
                }
                .boxed()
            })
            .await;
        match fetched {
            Ok(leaf) => return leaf,
            // Not reached while retries are enabled; stay alive if it ever is.
            Err(err) => {
                tracing::error!(height, "fetching leaf gave up: {err:#}");
                sleep(Duration::from_secs(1)).await;
            },
        }
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
    loop {
        tracing::debug!(
            height = leaf.height(),
            node_id = instance.node_id,
            ?leaf,
            "updating persistent merklized state"
        );
        // The node's catchup reads the parent snapshot from this database first. Once the data
        // pruner has deleted the parent's header, which that read needs, it asks peers.
        match update_state_storage(
            parent_state,
            storage,
            instance,
            &instance.state_catchup,
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
