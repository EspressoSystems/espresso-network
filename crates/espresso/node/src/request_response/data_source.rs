//! This file contains the [`DataSource`] trait. This trait allows the [`RequestResponseProtocol`]
//! to calculate/derive a response for a specific request. In the confirmation layer the implementer
//! would be something like a [`FeeMerkleTree`] for fee catchup

use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

use anyhow::{Context, Result, bail, ensure};
use async_trait::async_trait;
use committable::Committable as _;
use espresso_types::{
    Header, NodeState, PubKey, SeqTypes, StakeTableState, retain_accounts,
    stake_table_snapshot_root_height, stake_table_state_from_l1_events,
    traits::{EventsPersistenceRead, MembershipPersistence, SequencerPersistence},
    v0_3::{RewardAccountV1, RewardMerkleTreeV1},
    v0_4::{RewardAccountV2, RewardMerkleTreeV2},
};
use hotshot::traits::NodeImplementation;
use hotshot_new_protocol::storage::NewProtocolStorage;
use hotshot_query_service::{
    data_source::{
        VersionedDataSource,
        storage::{FileSystemStorage, NodeStorage, SqlStorage},
    },
    node::BlockId,
};
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    traits::network::ConnectedNetwork,
    utils::epoch_from_block_number,
    vote::HasViewNumber,
};
use itertools::Itertools;
use jf_merkle_tree_compat::{
    ForgetableMerkleTreeScheme, ForgetableUniversalMerkleTreeScheme, LookupResult,
    MerkleTreeScheme, UniversalMerkleTreeScheme,
};
use parking_lot::Mutex;
use request_response::data_source::DataSource as DataSourceTrait;

use super::request::{Request, Response};
use crate::{
    api::{BlocksFrontier, RewardMerkleTreeDataSource, RewardMerkleTreeV2Data},
    catchup::{
        CatchupStorage, add_fee_accounts_to_state, add_v1_reward_accounts_to_state,
        add_v2_reward_accounts_to_state,
    },
    consensus_handle::ConsensusHandle,
};

/// Number of the most recent epochs whose replayed [`StakeTableState`] is kept in memory, so
/// that repeated (sequential) requests for the same epoch don't each pay for a full event
/// replay from SQL. The cache is populated only after a replay completes, so concurrent misses
/// for the same epoch each replay in full; it only saves later, non-concurrent requests.
const STAKE_TABLE_STATE_CACHE_EPOCHS: usize = 8;

pub(crate) type StakeTableStateCache = Arc<Mutex<BTreeMap<u64, Arc<StakeTableState>>>>;

/// Query Service Storage types that can be used for request-response data source
#[derive(Clone)]
pub enum Storage {
    Sql(Arc<SqlStorage>),
    Fs(Arc<FileSystemStorage<SeqTypes>>),
}

#[derive(Clone)]
pub struct DataSource<
    I: NodeImplementation<SeqTypes>,
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
> {
    /// The consensus adapter handle
    pub consensus_handle: Arc<ConsensusHandle<SeqTypes, I>>,
    /// The node's state
    pub node_state: NodeState,
    /// The storage
    pub storage: Option<Storage>,
    /// sequencer persistence
    pub persistence: Arc<P>,
    /// Cache of replayed [`StakeTableState`]s, keyed by epoch
    pub stake_table_state_cache: StakeTableStateCache,
    /// Phantom data
    pub phantom: PhantomData<N>,
}

/// Implement the trait that allows the [`RequestResponseProtocol`] to calculate/derive a response for a specific request
#[async_trait]
impl<I: NodeImplementation<SeqTypes>, N: ConnectedNetwork<PubKey>, P: SequencerPersistence>
    DataSourceTrait<Request> for DataSource<I, N, P>
where
    I::Storage: NewProtocolStorage<SeqTypes>,
{
    async fn derive_response_for(&self, request: &Request) -> Result<Response> {
        match request {
            Request::Accounts(height, view, accounts) => {
                // Try to get accounts from memory first, then fall back to storage
                if let Some(state) = self.consensus_handle.state(ViewNumber::new(*view)).await
                    && let Ok(accounts) =
                        retain_accounts(&state.fee_merkle_tree, accounts.iter().copied())
                {
                    return Ok(Response::Accounts(accounts));
                }

                // Fall back to storage
                let (merkle_tree, leaf) = match &self.storage {
                    Some(Storage::Sql(storage)) => storage
                        .get_accounts(&self.node_state, *height, ViewNumber::new(*view), accounts)
                        .await
                        .with_context(|| "failed to get accounts from sql storage")?,
                    Some(Storage::Fs(_)) => bail!("fs storage not supported for accounts"),
                    _ => bail!("storage was not initialized"),
                };

                // If we successfully fetched accounts from storage, try to add them back into the in-memory
                // state.
                if let Err(err) = add_fee_accounts_to_state(
                    &*self.consensus_handle,
                    &ViewNumber::new(*view),
                    accounts,
                    &merkle_tree,
                    leaf,
                )
                .await
                {
                    tracing::warn!(?view, "Cannot update fetched account state: {err:#}");
                }

                Ok(Response::Accounts(merkle_tree))
            },

            Request::Leaf(height) => {
                // Legacy heights can be served from in-memory undecided leaves; new-protocol
                // heights always fall through to storage.
                if let Ok(leaf_chain) =
                    legacy_leaf_chain_from_memory(&*self.consensus_handle, *height).await
                {
                    return Ok(Response::Leaf(leaf_chain));
                }

                let leaf_chain = match &self.storage {
                    Some(Storage::Sql(storage)) => storage
                        .get_leaf_chain(*height)
                        .await
                        .with_context(|| "failed to get leaf from sql storage")?,
                    Some(Storage::Fs(_)) => bail!("fs storage not supported for leaf"),
                    _ => bail!("storage was not initialized"),
                };

                Ok(Response::Leaf(leaf_chain))
            },
            Request::ChainConfig(commitment) => {
                // Try to get the chain config from memory first, then fall back to storage
                if let Some(state) = self.consensus_handle.decided_state().await {
                    let chain_config_from_memory = state.chain_config;
                    if chain_config_from_memory.commit() == *commitment
                        && let Some(chain_config) = chain_config_from_memory.resolve()
                    {
                        return Ok(Response::ChainConfig(chain_config));
                    }
                }

                // Fall back to storage
                Ok(Response::ChainConfig(match &self.storage {
                    Some(Storage::Sql(storage)) => storage
                        .get_chain_config(*commitment)
                        .await
                        .with_context(|| "failed to get chain config from sql storage")?,
                    Some(Storage::Fs(_)) => {
                        bail!("fs storage not supported for chain config")
                    },
                    _ => bail!("storage was not initialized"),
                }))
            },
            Request::BlocksFrontier(height, view) => {
                // First try to respond from memory
                let blocks_frontier_from_memory: Option<Result<BlocksFrontier>> = self
                    .consensus_handle
                    .state(ViewNumber::new(*view))
                    .await
                    .map(|state| {
                        let tree = &state.block_merkle_tree;
                        let frontier = tree.lookup(tree.num_leaves() - 1).expect_ok()?.1;
                        Ok(frontier)
                    });

                if let Some(Ok(blocks_frontier_from_memory)) = blocks_frontier_from_memory {
                    return Ok(Response::BlocksFrontier(blocks_frontier_from_memory));
                } else {
                    // If we can't get the blocks frontier from memory, fall through to storage
                    let blocks_frontier_from_storage = match &self.storage {
                        Some(Storage::Sql(storage)) => storage
                            .get_frontier(&self.node_state, *height, ViewNumber::new(*view))
                            .await
                            .with_context(|| "failed to get blocks frontier from sql storage")?,
                        Some(Storage::Fs(_)) => {
                            bail!("fs storage not supported for blocks frontier")
                        },
                        _ => bail!("storage was not initialized"),
                    };

                    Ok(Response::BlocksFrontier(blocks_frontier_from_storage))
                }
            },
            Request::RewardAccountsV2(height, view, accounts) => {
                // Try to get the reward accounts from memory first, then fall back to storage
                if let Some(state) = self.consensus_handle.state(ViewNumber::new(*view)).await
                    && let Ok(reward_accounts) = retain_v2_reward_accounts(
                        &state.reward_merkle_tree_v2,
                        accounts.iter().copied(),
                    )
                {
                    return Ok(Response::RewardAccountsV2(reward_accounts));
                }

                // Fall back to storage
                let (merkle_tree, leaf) = match &self.storage {
                    Some(Storage::Sql(storage)) => storage
                        .get_reward_accounts_v2(
                            &self.node_state,
                            *height,
                            ViewNumber::new(*view),
                            accounts,
                        )
                        .await
                        .with_context(|| "failed to get accounts from sql storage")?,
                    Some(Storage::Fs(_)) => {
                        bail!("fs storage not supported for reward accounts")
                    },
                    _ => bail!("storage was not initialized"),
                };

                // If we successfully fetched accounts from storage, try to add them back into the in-memory
                // state.
                if let Err(err) = add_v2_reward_accounts_to_state(
                    &*self.consensus_handle,
                    &ViewNumber::new(*view),
                    accounts,
                    &merkle_tree,
                    leaf,
                )
                .await
                {
                    tracing::warn!(?view, "Cannot update fetched account state: {err:#}");
                }

                Ok(Response::RewardAccountsV2(merkle_tree))
            },

            Request::RewardAccountsV1(height, view, accounts) => {
                // Try to get the reward accounts from memory first, then fall back to storage
                if let Some(state) = self.consensus_handle.state(ViewNumber::new(*view)).await
                    && let Ok(reward_accounts) = retain_v1_reward_accounts(
                        &state.reward_merkle_tree_v1,
                        accounts.iter().copied(),
                    )
                {
                    return Ok(Response::RewardAccountsV1(reward_accounts));
                }

                // Fall back to storage
                let (merkle_tree, leaf) = match &self.storage {
                    Some(Storage::Sql(storage)) => storage
                        .get_reward_accounts_v1(
                            &self.node_state,
                            *height,
                            ViewNumber::new(*view),
                            accounts,
                        )
                        .await
                        .with_context(|| "failed to get v1 reward accounts from sql storage")?,
                    Some(Storage::Fs(_)) => {
                        bail!("fs storage not supported for v1 reward accounts")
                    },
                    _ => bail!("storage was not initialized"),
                };

                // If we successfully fetched accounts from storage, try to add them back into the in-memory
                // state.
                if let Err(err) = add_v1_reward_accounts_to_state(
                    &*self.consensus_handle,
                    &ViewNumber::new(*view),
                    accounts,
                    &merkle_tree,
                    leaf,
                )
                .await
                {
                    tracing::warn!(
                        ?view,
                        "Cannot update fetched v1 reward account state: {err:#}"
                    );
                }

                Ok(Response::RewardAccountsV1(merkle_tree))
            },
            Request::VidShare(block_number, _request_id) => {
                // Load the VID share from storage
                let vid_share = match &self.storage {
                    Some(Storage::Sql(storage)) => storage
                        .get_vid_share::<SeqTypes>(BlockId::Number(*block_number as usize))
                        .await
                        .with_context(|| "failed to get vid share from sql storage")?,
                    Some(Storage::Fs(storage)) => {
                        // Open a read transaction
                        let mut transaction = storage
                            .read()
                            .await
                            .with_context(|| "failed to open fs storage transaction")?;

                        // Get the VID share
                        transaction
                            .vid_share(BlockId::Number(*block_number as usize))
                            .await
                            .with_context(|| "failed to get vid share from fs storage")?
                    },
                    _ => bail!("storage was not initialized"),
                };

                Ok(Response::VidShare(vid_share))
            },
            Request::StateCert(epoch) => {
                let state_cert = self
                    .persistence
                    .get_state_cert_by_epoch(*epoch)
                    .await
                    .with_context(|| {
                        format!("failed to get state cert for epoch {epoch} from persistence")
                    })?;

                match state_cert {
                    Some(cert) => Ok(Response::StateCert(cert)),
                    None => bail!("State certificate for epoch {epoch} not found"),
                }
            },
            Request::Cert2(height) => {
                let cert2 = match &self.storage {
                    Some(Storage::Sql(storage)) => storage
                        .load_cert2(*height)
                        .await
                        .with_context(|| "failed to load cert2 from sql storage")?,
                    Some(Storage::Fs(_)) => bail!("fs storage not supported for cert2"),
                    _ => bail!("storage was not initialized"),
                };

                match cert2 {
                    Some(cert2) => Ok(Response::Cert2(cert2)),
                    None => bail!("no cert2 available at height {height}"),
                }
            },
            Request::RewardMerkleTreeV2(height, view) => {
                // Try to get the reward merkle tree from memory first, then fall back to storage
                if let Some(state) = self.consensus_handle.state(ViewNumber::new(*view)).await {
                    let tree_data =
                        TryInto::<RewardMerkleTreeV2Data>::try_into(&state.reward_merkle_tree_v2)
                            .inspect_err(|err| {
                            tracing::debug!(
                                %err, height, view,
                                "cannot serve reward merkle tree from memory"
                            )
                        })?;
                    let merkle_tree_bytes = bincode::serialize(&tree_data)
                        .context("Merkle tree serialization failed; this should never happen.")?;

                    return Ok(Response::RewardMerkleTreeV2(merkle_tree_bytes));
                }

                // Fall back to storage
                let merkle_tree_bytes = match &self.storage {
                    Some(Storage::Sql(storage)) => storage
                        .load_tree(*height)
                        .await
                        .with_context(|| "failed to get reward merkle tree from sql storage")?,
                    Some(Storage::Fs(_)) => {
                        bail!("fs storage not supported for reward merkle tree catchup")
                    },
                    _ => bail!("storage was not initialized"),
                };

                Ok(Response::RewardMerkleTreeV2(merkle_tree_bytes))
            },
            Request::StakeTableState { epoch } => Ok(Response::StakeTableState(
                stake_table_state_for_epoch(
                    *epoch,
                    &self.node_state,
                    &*self.persistence,
                    self.storage.as_ref(),
                    &self.stake_table_state_cache,
                )
                .await?,
            )),
        }
    }
}

/// Build a legacy-protocol 3-chain leaf chain decided at `height` from in-memory undecided leaves.
///
/// Returns an error if the chain cannot be assembled from memory (e.g. the height is below the
/// latest decided leaf).
async fn legacy_leaf_chain_from_memory<I: NodeImplementation<SeqTypes>>(
    consensus_handle: &ConsensusHandle<SeqTypes, I>,
    height: u64,
) -> anyhow::Result<Vec<espresso_types::Leaf2>>
where
    I::Storage: NewProtocolStorage<SeqTypes>,
{
    let mut leaves = consensus_handle.undecided_leaves().await;
    leaves.sort_by_key(|l| l.view_number());

    let (position, mut last_leaf) = leaves
        .iter()
        .find_position(|l| l.height() == height)
        .ok_or_else(|| anyhow::anyhow!("leaf at height {height} not in memory"))?;

    let mut leaf_chain = vec![last_leaf.clone()];
    for leaf in leaves.iter().skip(position + 1) {
        if leaf.justify_qc().view_number() == last_leaf.view_number() {
            leaf_chain.push(leaf.clone());
        } else {
            continue;
        }
        if leaf.view_number() == last_leaf.view_number() + 1 {
            last_leaf = leaf;
            break;
        }
        last_leaf = leaf;
    }

    for leaf in leaves
        .iter()
        .skip_while(|l| l.view_number() <= last_leaf.view_number())
    {
        if leaf.justify_qc().view_number() == last_leaf.view_number() {
            leaf_chain.push(leaf.clone());
            return Ok(leaf_chain);
        }
    }

    anyhow::bail!("incomplete leaf chain in memory for height {height}")
}

/// Replay the L1 stake table events committed to at `epoch` into a [`StakeTableState`],
/// consulting `cache` first and populating it on success.
///
/// Does not require a [`ConsensusHandle`]: the epoch root header and event history are read
/// entirely from `persistence` (falling back to `storage` for the epoch root), so this is
/// callable directly from tests.
pub(crate) async fn stake_table_state_for_epoch<P: SequencerPersistence>(
    epoch: u64,
    node_state: &NodeState,
    persistence: &P,
    storage: Option<&Storage>,
    cache: &StakeTableStateCache,
) -> Result<StakeTableState> {
    if let Some(state) = cache.lock().get(&epoch) {
        return Ok((**state).clone());
    }

    let epoch_height = node_state.epoch_height.context("epoch state not set")?;
    let first_epoch = epoch_from_block_number(node_state.epoch_start_block, epoch_height);
    ensure!(
        epoch >= first_epoch + 2,
        "stake table state requires epoch >= {}",
        first_epoch + 2
    );

    let snapshot_height = stake_table_snapshot_root_height(EpochNumber::new(epoch), epoch_height)?;
    let snapshot_root = load_epoch_root_header(
        persistence,
        storage,
        EpochNumber::new(epoch),
        snapshot_height,
    )
    .await?;
    ensure!(
        snapshot_root.height() == snapshot_height,
        "epoch root for epoch {epoch} has height {}, expected snapshot root height \
         {snapshot_height}",
        snapshot_root.height()
    );

    let to_l1_block = snapshot_root
        .l1_finalized()
        .context("epoch root header is missing L1 finalized block")?
        .number();

    let (read, events) = persistence
        .load_events(0, to_l1_block)
        .await
        .context("failed to load stake table events from persistence")?;
    ensure!(
        matches!(read, Some(EventsPersistenceRead::Complete)),
        "stake table events not fully available up to L1 block {to_l1_block}; refusing to serve a \
         partial replay"
    );

    let state = stake_table_state_from_l1_events(events.into_iter().map(|(_, event)| event))
        .context("failed to replay stake table events")?;

    // `load_epoch_root(epoch + 1)` returns the anchor root `H_{e-1}` whose
    // `next_stake_table_hash` commits to this epoch's stake table (see the offset note on
    // `load_epoch_root_header` below).
    if let Some(anchor) = persistence
        .load_epoch_root(EpochNumber::new(epoch + 1))
        .await?
        && let Some(expected) = anchor.next_stake_table_hash()
        && expected != state.commit()
    {
        tracing::error!(
            epoch,
            "replayed stake table does not match anchor commitment; refusing to serve"
        );
        bail!("replayed stake table for epoch {epoch} does not match anchor commitment");
    }

    let state = Arc::new(state);
    {
        let mut cache = cache.lock();
        cache.insert(epoch, state.clone());
        while cache.len() > STAKE_TABLE_STATE_CACHE_EPOCHS {
            let oldest = *cache.keys().next().expect("cache is non-empty");
            cache.remove(&oldest);
        }
    }

    Ok((*state).clone())
}

/// Load the epoch root header for `epoch`, falling back to SQL storage if persistence has not
/// recorded it.
///
/// `store_epoch_root` is called with `epoch_from_block_number(decided_block_number) + 2`
/// (`crates/hotshot/task-impls/src/helpers.rs`), so `load_epoch_root(epoch)` returns the header
/// at `stake_table_snapshot_root_height(epoch, _)`, i.e. two epochs behind `epoch` itself.
async fn load_epoch_root_header<P: MembershipPersistence>(
    persistence: &P,
    storage: Option<&Storage>,
    epoch: EpochNumber,
    snapshot_height: u64,
) -> Result<Header> {
    if let Some(header) = persistence.load_epoch_root(epoch).await? {
        return Ok(header);
    }

    match storage {
        Some(Storage::Sql(storage)) => Ok(storage
            .get_leaf(snapshot_height)
            .await?
            .block_header()
            .clone()),
        _ => bail!("missing epoch root header at height {snapshot_height}"),
    }
}

/// Get a partial snapshot of the given reward state, which contains only the specified accounts.
///
/// Fails if one of the requested accounts is not represented in the original `state`.
pub fn retain_v2_reward_accounts(
    state: &RewardMerkleTreeV2,
    accounts: impl IntoIterator<Item = RewardAccountV2>,
) -> anyhow::Result<RewardMerkleTreeV2> {
    let mut snapshot = RewardMerkleTreeV2::from_commitment(state.commitment());
    for account in accounts {
        match state.universal_lookup(account) {
            LookupResult::Ok(elem, proof) => {
                // This remember cannot fail, since we just constructed a valid proof, and are
                // remembering into a tree with the same commitment.
                snapshot.remember(account, elem, proof).unwrap();
            },
            LookupResult::NotFound(proof) => {
                // Likewise this cannot fail.
                snapshot.non_membership_remember(account, proof).unwrap()
            },
            LookupResult::NotInMemory => {
                bail!("missing account {account}");
            },
        }
    }

    Ok(snapshot)
}

/// Get a partial snapshot of the given reward state, which contains only the specified accounts.
///
/// Fails if one of the requested accounts is not represented in the original `state`.
pub fn retain_v1_reward_accounts(
    state: &RewardMerkleTreeV1,
    accounts: impl IntoIterator<Item = RewardAccountV1>,
) -> anyhow::Result<RewardMerkleTreeV1> {
    let mut snapshot = RewardMerkleTreeV1::from_commitment(state.commitment());
    for account in accounts {
        match state.universal_lookup(account) {
            LookupResult::Ok(elem, proof) => {
                // This remember cannot fail, since we just constructed a valid proof, and are
                // remembering into a tree with the same commitment.
                snapshot.remember(account, elem, proof).unwrap();
            },
            LookupResult::NotFound(proof) => {
                // Likewise this cannot fail.
                snapshot.non_membership_remember(account, proof).unwrap()
            },
            LookupResult::NotInMemory => {
                bail!("missing account {account}");
            },
        }
    }

    Ok(snapshot)
}
