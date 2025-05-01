use std::collections::{HashSet, VecDeque};

use anyhow::{bail, ensure, Context};
use async_trait::async_trait;
use committable::{Commitment, Committable};
use espresso_types::{
    get_l1_deposits,
    v0_1::{RewardAccount, RewardMerkleTree, REWARD_MERKLE_TREE_HEIGHT},
    v0_99::{ChainConfig, IterableFeeInfo},
    BlockMerkleTree, EpochVersion, FeeAccount, FeeMerkleTree, Leaf2, NodeState, ValidatedState,
};
use hotshot::traits::ValidatedState as _;
use hotshot_query_service::{
    availability::LeafId,
    data_source::{
        sql::{Config, SqlDataSource, Transaction},
        storage::{
            sql::{query_as, Db, TransactionMode, Write},
            AvailabilityStorage, MerklizedStateStorage, NodeStorage, SqlStorage,
        },
        VersionedDataSource,
    },
    merklized_state::Snapshot,
    Resolvable,
};
use hotshot_types::{
    data::{EpochNumber, QuorumProposalWrapper, ViewNumber},
    message::Proposal,
    traits::node_implementation::ConsensusTime,
    utils::epoch_from_block_number,
    vote::HasViewNumber,
};
use jf_merkle_tree::{
    prelude::MerkleNode, ForgetableMerkleTreeScheme, ForgetableUniversalMerkleTreeScheme,
    LookupResult, MerkleTreeScheme,
};
use sqlx::{Encode, Type};
use vbs::version::StaticVersionType;

use super::{
    data_source::{Provider, SequencerDataSource},
    BlocksFrontier,
};
use crate::{
    catchup::{CatchupStorage, NullStateCatchup},
    persistence::{sql::Options, ChainConfigPersistence},
    state::compute_state_update,
    SeqTypes,
};

pub type DataSource = SqlDataSource<SeqTypes, Provider>;

#[async_trait]
impl SequencerDataSource for DataSource {
    type Options = Options;

    async fn create(opt: Self::Options, provider: Provider, reset: bool) -> anyhow::Result<Self> {
        let fetch_limit = opt.fetch_rate_limit;
        let active_fetch_delay = opt.active_fetch_delay;
        let chunk_fetch_delay = opt.chunk_fetch_delay;
        let mut cfg = Config::try_from(&opt)?;

        if reset {
            cfg = cfg.reset_schema();
        }

        let mut builder = cfg.builder(provider).await?;

        if let Some(limit) = fetch_limit {
            builder = builder.with_rate_limit(limit);
        }

        if opt.lightweight {
            tracing::warn!("enabling light weight mode..");
            builder = builder.leaf_only();
        }

        if let Some(delay) = active_fetch_delay {
            builder = builder.with_active_fetch_delay(delay);
        }
        if let Some(delay) = chunk_fetch_delay {
            builder = builder.with_chunk_fetch_delay(delay);
        }

        if let Some(batch_size) = opt.types_migration_batch_size {
            builder = builder.with_types_migration_batch_size(batch_size);
        }

        builder.build().await
    }
}

impl CatchupStorage for SqlStorage {
    async fn get_reward_accounts(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[RewardAccount],
    ) -> anyhow::Result<(RewardMerkleTree, Leaf2)> {
        let mut tx = self.read().await.context(format!(
            "opening transaction to fetch reward account {accounts:?}; height {height}"
        ))?;

        let block_height = NodeStorage::<SeqTypes>::block_height(&mut tx)
            .await
            .context("getting block height")? as u64;
        ensure!(
            block_height > 0,
            "cannot get accounts for height {height}: no blocks available"
        );

        // Check if we have the desired state snapshot. If so, we can load the desired accounts
        // directly.
        if height < block_height {
            load_reward_accounts(&mut tx, height, accounts).await
        } else {
            // If we do not have the exact snapshot we need, we can try going back to the last
            // snapshot we _do_ have and replaying subsequent blocks to compute the desired state.
            let (state, leaf) =
                reconstruct_state(instance, &mut tx, block_height - 1, view, &[], accounts).await?;
            Ok((state.reward_merkle_tree, leaf))
        }
    }

    async fn get_accounts(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[FeeAccount],
    ) -> anyhow::Result<(FeeMerkleTree, Leaf2)> {
        let mut tx = self.read().await.context(format!(
            "opening transaction to fetch account {accounts:?}; height {height}"
        ))?;

        let block_height = NodeStorage::<SeqTypes>::block_height(&mut tx)
            .await
            .context("getting block height")? as u64;
        ensure!(
            block_height > 0,
            "cannot get accounts for height {height}: no blocks available"
        );

        // Check if we have the desired state snapshot. If so, we can load the desired accounts
        // directly.
        if height < block_height {
            load_accounts(&mut tx, height, accounts).await
        } else {
            // If we do not have the exact snapshot we need, we can try going back to the last
            // snapshot we _do_ have and replaying subsequent blocks to compute the desired state.
            let (state, leaf) =
                reconstruct_state(instance, &mut tx, block_height - 1, view, accounts, &[]).await?;
            Ok((state.fee_merkle_tree, leaf))
        }
    }

    async fn get_frontier(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
    ) -> anyhow::Result<BlocksFrontier> {
        let mut tx = self.read().await.context(format!(
            "opening transaction to fetch frontier at height {height}"
        ))?;

        let block_height = NodeStorage::<SeqTypes>::block_height(&mut tx)
            .await
            .context("getting block height")? as u64;
        ensure!(
            block_height > 0,
            "cannot get frontier for height {height}: no blocks available"
        );

        // Check if we have the desired state snapshot. If so, we can load the desired frontier
        // directly.
        if height < block_height {
            load_frontier(&mut tx, height).await
        } else {
            // If we do not have the exact snapshot we need, we can try going back to the last
            // snapshot we _do_ have and replaying subsequent blocks to compute the desired state.
            let (state, _) =
                reconstruct_state(instance, &mut tx, block_height - 1, view, &[], &[]).await?;
            match state.block_merkle_tree.lookup(height - 1) {
                LookupResult::Ok(_, proof) => Ok(proof),
                _ => {
                    bail!("state snapshot {view:?},{height} was found but does not contain frontier at height {}; this should not be possible", height - 1);
                },
            }
        }
    }

    async fn get_chain_config(
        &self,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        let mut tx = self.read().await.context(format!(
            "opening transaction to fetch chain config {commitment}"
        ))?;
        load_chain_config(&mut tx, commitment).await
    }

    async fn get_leaf_chain(&self, height: u64) -> anyhow::Result<Vec<Leaf2>> {
        let mut tx = self
            .read()
            .await
            .context(format!("opening transaction to fetch leaf at {height}"))?;
        let leaf = tx
            .get_leaf((height as usize).into())
            .await
            .context(format!("leaf {height} not available"))?;
        let mut last_leaf: Leaf2 = leaf.leaf().clone();
        let mut chain = vec![last_leaf.clone()];
        let mut h = height + 1;

        loop {
            let lqd = tx.get_leaf((h as usize).into()).await?;
            let leaf = lqd.leaf();

            if leaf.justify_qc().view_number() == last_leaf.view_number() {
                chain.push(leaf.clone());
            } else {
                h += 1;
                continue;
            }

            // just one away from deciding
            if leaf.view_number() == last_leaf.view_number() + 1 {
                last_leaf = leaf.clone();
                h += 1;
                break;
            }
            h += 1;
            last_leaf = leaf.clone();
        }

        loop {
            let lqd = tx.get_leaf((h as usize).into()).await?;
            let leaf = lqd.leaf();
            if leaf.justify_qc().view_number() == last_leaf.view_number() {
                chain.push(leaf.clone());
                break;
            }
            h += 1;
        }

        Ok(chain)
    }
}

impl CatchupStorage for DataSource {
    async fn get_accounts(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[FeeAccount],
    ) -> anyhow::Result<(FeeMerkleTree, Leaf2)> {
        self.as_ref()
            .get_accounts(instance, height, view, accounts)
            .await
    }

    async fn get_reward_accounts(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[RewardAccount],
    ) -> anyhow::Result<(RewardMerkleTree, Leaf2)> {
        self.as_ref()
            .get_reward_accounts(instance, height, view, accounts)
            .await
    }

    async fn get_frontier(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
    ) -> anyhow::Result<BlocksFrontier> {
        self.as_ref().get_frontier(instance, height, view).await
    }

    async fn get_chain_config(
        &self,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        self.as_ref().get_chain_config(commitment).await
    }
    async fn get_leaf_chain(&self, height: u64) -> anyhow::Result<Vec<Leaf2>> {
        self.as_ref().get_leaf_chain(height).await
    }
}

#[async_trait]
impl ChainConfigPersistence for Transaction<Write> {
    async fn insert_chain_config(&mut self, chain_config: ChainConfig) -> anyhow::Result<()> {
        let commitment = chain_config.commitment();
        let data = bincode::serialize(&chain_config)?;
        self.upsert(
            "chain_config",
            ["commitment", "data"],
            ["commitment"],
            [(commitment.to_string(), data)],
        )
        .await
    }
}

async fn load_frontier<Mode: TransactionMode>(
    tx: &mut Transaction<Mode>,
    height: u64,
) -> anyhow::Result<BlocksFrontier> {
    tx.get_path(
        Snapshot::<SeqTypes, BlockMerkleTree, { BlockMerkleTree::ARITY }>::Index(height),
        height
            .checked_sub(1)
            .ok_or(anyhow::anyhow!("Subtract with overflow ({height})!"))?,
    )
    .await
    .context(format!("fetching frontier at height {height}"))
}

async fn load_reward_accounts<Mode: TransactionMode>(
    tx: &mut Transaction<Mode>,
    height: u64,
    accounts: &[RewardAccount],
) -> anyhow::Result<(RewardMerkleTree, Leaf2)> {
    let leaf = tx
        .get_leaf(LeafId::<SeqTypes>::from(height as usize))
        .await
        .context(format!("leaf {height} not available"))?;
    let header = leaf.header();

    if header.version() < EpochVersion::version() {
        return Ok((
            RewardMerkleTree::new(REWARD_MERKLE_TREE_HEIGHT),
            leaf.leaf().clone(),
        ));
    }

    let merkle_root = header.reward_merkle_tree_root();
    let mut snapshot = RewardMerkleTree::from_commitment(merkle_root);
    for account in accounts {
        let proof = tx
            .get_path(
                Snapshot::<SeqTypes, RewardMerkleTree, { RewardMerkleTree::ARITY }>::Index(
                    header.height(),
                ),
                *account,
            )
            .await
            .context(format!(
                "fetching reward account {account}; height {}",
                header.height()
            ))?;
        match proof.proof.first().context(format!(
            "empty proof for reward account {account}; height {}",
            header.height()
        ))? {
            MerkleNode::Leaf { pos, elem, .. } => {
                snapshot.remember(*pos, *elem, proof)?;
            },
            MerkleNode::Empty => {
                snapshot.non_membership_remember(*account, proof)?;
            },
            _ => {
                bail!("Invalid proof");
            },
        }
    }

    Ok((snapshot, leaf.leaf().clone()))
}

async fn load_accounts<Mode: TransactionMode>(
    tx: &mut Transaction<Mode>,
    height: u64,
    accounts: &[FeeAccount],
) -> anyhow::Result<(FeeMerkleTree, Leaf2)> {
    let leaf = tx
        .get_leaf(LeafId::<SeqTypes>::from(height as usize))
        .await
        .context(format!("leaf {height} not available"))?;
    let header = leaf.header();

    let mut snapshot = FeeMerkleTree::from_commitment(header.fee_merkle_tree_root());
    for account in accounts {
        let proof = tx
            .get_path(
                Snapshot::<SeqTypes, FeeMerkleTree, { FeeMerkleTree::ARITY }>::Index(
                    header.height(),
                ),
                *account,
            )
            .await
            .context(format!(
                "fetching account {account}; height {}",
                header.height()
            ))?;
        match proof.proof.first().context(format!(
            "empty proof for account {account}; height {}",
            header.height()
        ))? {
            MerkleNode::Leaf { pos, elem, .. } => {
                snapshot.remember(*pos, *elem, proof)?;
            },
            MerkleNode::Empty => {
                snapshot.non_membership_remember(*account, proof)?;
            },
            _ => {
                bail!("Invalid proof");
            },
        }
    }

    Ok((snapshot, leaf.leaf().clone()))
}

async fn load_chain_config<Mode: TransactionMode>(
    tx: &mut Transaction<Mode>,
    commitment: Commitment<ChainConfig>,
) -> anyhow::Result<ChainConfig> {
    let (data,) = query_as::<(Vec<u8>,)>("SELECT data from chain_config where commitment = $1")
        .bind(commitment.to_string())
        .fetch_one(tx.as_mut())
        .await
        .unwrap();

    bincode::deserialize(&data[..]).context("failed to deserialize")
}

/// Reconstructs the `ValidatedState` from a specific block height up to a given view.
///
/// This loads all required fee and reward accounts into the Merkle tree before applying the
/// State Transition Function (STF), preventing recursive catchup during STF replay.
///
/// Note: Even if the primary goal is to catch up the block Merkle tree,
/// fee and reward header dependencies must still be present beforehand
/// This is because reconstructing the `ValidatedState` involves replaying the STF over a
/// range of leaves, and the STF requires all associated data to be present in the `ValidatedState`;
/// otherwise, it will attempt to trigger catchup itself.
#[tracing::instrument(skip(instance, tx))]
async fn reconstruct_state<Mode: TransactionMode>(
    instance: &NodeState,
    tx: &mut Transaction<Mode>,
    from_height: u64,
    to_view: ViewNumber,
    fee_accounts: &[FeeAccount],
    reward_accounts: &[RewardAccount],
) -> anyhow::Result<(ValidatedState, Leaf2)> {
    tracing::info!("attempting to reconstruct fee state");
    let from_leaf = tx
        .get_leaf((from_height as usize).into())
        .await
        .context(format!("leaf {from_height} not available"))?;
    let from_leaf: Leaf2 = from_leaf.leaf().clone();
    ensure!(
        from_leaf.view_number() < to_view,
        "state reconstruction: starting state {:?} must be before ending state {to_view:?}",
        from_leaf.view_number(),
    );

    // Get the sequence of headers we will be applying to compute the latest state.
    let mut leaves = VecDeque::new();
    let to_leaf = get_leaf_from_proposal(tx, "view = $1", &(to_view.u64() as i64))
        .await
        .context(format!(
            "unable to reconstruct state because leaf {to_view:?} is not available"
        ))?;
    let mut parent = to_leaf.parent_commitment();
    tracing::debug!(?to_leaf, ?parent, view = ?to_view, "have required leaf");
    leaves.push_front(to_leaf.clone());
    while parent != Committable::commit(&from_leaf) {
        let leaf = get_leaf_from_proposal(tx, "leaf_hash = $1", &parent.to_string())
            .await
            .context(format!(
                "unable to reconstruct state because leaf {parent} is not available"
            ))?;
        parent = leaf.parent_commitment();
        tracing::debug!(?leaf, ?parent, "have required leaf");
        leaves.push_front(leaf);
    }

    // Get the initial state.
    let mut parent = from_leaf;
    let mut state = ValidatedState::from_header(parent.block_header());

    // Pre-load the state with the accounts we care about to ensure they will be present in the
    // final state.
    let mut catchup = NullStateCatchup::default();

    let mut fee_accounts = fee_accounts.iter().copied().collect::<HashSet<_>>();
    // Add in all the accounts we will need to replay any of the headers, to ensure that we don't
    // need to do catchup recursively.
    tracing::info!(
        "reconstructing fee accounts state for from height {} to view {}",
        from_height,
        to_view
    );

    let dependencies =
        fee_header_dependencies(&mut catchup, tx, instance, &parent, &leaves).await?;
    fee_accounts.extend(dependencies);
    let fee_accounts = fee_accounts.into_iter().collect::<Vec<_>>();
    state.fee_merkle_tree = load_accounts(tx, from_height, &fee_accounts)
        .await
        .context("unable to reconstruct state because accounts are not available at origin")?
        .0;
    ensure!(
        state.fee_merkle_tree.commitment() == parent.block_header().fee_merkle_tree_root(),
        "loaded fee state does not match parent header"
    );

    tracing::info!(
        "reconstructing reward accounts for from height {} to view {}",
        from_height,
        to_view
    );

    let mut reward_accounts = reward_accounts.iter().copied().collect::<HashSet<_>>();

    // Collect all reward account dependencies needed for replaying the STF.
    // These accounts must be preloaded into the reward Merkle tree to prevent recursive catchups.
    let dependencies = reward_header_dependencies(instance, &leaves).await?;
    reward_accounts.extend(dependencies);
    let reward_accounts = reward_accounts.into_iter().collect::<Vec<_>>();

    // Load all required reward accounts and update the reward Merkle tree.
    state.reward_merkle_tree = load_reward_accounts(tx, from_height, &reward_accounts)
        .await
        .context("unable to reconstruct state because reward accounts are not available at origin")?
        .0;
    ensure!(
        state.reward_merkle_tree.commitment() == parent.block_header().reward_merkle_tree_root(),
        "loaded reward state does not match parent header"
    );

    // We need the blocks frontier as well, to apply the STF.
    let frontier = load_frontier(tx, from_height)
        .await
        .context("unable to reconstruct state because frontier is not available at origin")?;
    match frontier
        .proof
        .first()
        .context("empty proof for frontier at origin")?
    {
        MerkleNode::Leaf { pos, elem, .. } => state
            .block_merkle_tree
            .remember(*pos, *elem, frontier)
            .context("failed to remember frontier")?,
        _ => bail!("invalid frontier proof"),
    }

    // Apply subsequent headers to compute the later state.
    for proposal in leaves {
        state = compute_state_update(&state, instance, &catchup, &parent, &proposal)
            .await
            .context(format!(
                "unable to reconstruct state because state update {} failed",
                proposal.height(),
            ))?
            .0;
        parent = proposal;
    }

    tracing::info!(from_height, ?to_view, "successfully reconstructed state");
    Ok((state, to_leaf))
}

/// Get the dependencies needed to apply the STF to the given list of headers.
///
/// Returns
/// * A state catchup implementation seeded with all the chain configs required to apply the headers
///   in `leaves`
/// * The set of accounts that must be preloaded to apply these headers
async fn fee_header_dependencies<Mode: TransactionMode>(
    catchup: &mut NullStateCatchup,
    tx: &mut Transaction<Mode>,
    instance: &NodeState,
    mut parent: &Leaf2,
    leaves: impl IntoIterator<Item = &Leaf2>,
) -> anyhow::Result<HashSet<FeeAccount>> {
    let mut accounts = HashSet::default();

    for proposal in leaves {
        let header = proposal.block_header();
        let height = header.height();
        let view = proposal.view_number();
        tracing::debug!(height, ?view, "fetching dependencies for proposal");

        let header_cf = header.chain_config();
        let chain_config = if header_cf.commit() == instance.chain_config.commit() {
            instance.chain_config
        } else {
            match header_cf.resolve() {
                Some(cf) => cf,
                None => {
                    tracing::info!(
                        height,
                        ?view,
                        commit = %header_cf.commit(),
                        "chain config not available, attempting to load from storage",
                    );
                    let cf = load_chain_config(tx, header_cf.commit())
                        .await
                        .context(format!(
                            "loading chain config {} for header {},{:?}",
                            header_cf.commit(),
                            header.height(),
                            proposal.view_number()
                        ))?;

                    // If we had to fetch a chain config now, store it in the catchup implementation
                    // so the STF will be able to look it up later.
                    catchup.add_chain_config(cf);
                    cf
                },
            }
        };

        accounts.insert(chain_config.fee_recipient);
        accounts.extend(
            get_l1_deposits(instance, header, parent, chain_config.fee_contract)
                .await
                .into_iter()
                .map(|fee| fee.account()),
        );
        accounts.extend(header.fee_info().accounts());
        parent = proposal;
    }
    Ok(accounts)
}

/// Identifies all reward accounts required to replay the State Transition Function
/// for the given leaf proposals. These accounts should be present in the Merkle tree
/// *before* applying the STF to avoid recursive catchup (i.e., STF triggering another catchup).
async fn reward_header_dependencies(
    instance: &NodeState,
    leaves: impl IntoIterator<Item = &Leaf2>,
) -> anyhow::Result<HashSet<RewardAccount>> {
    let mut reward_accounts = HashSet::default();
    let epoch_height = instance.epoch_height;

    let Some(epoch_height) = epoch_height else {
        tracing::info!("epoch height is not set. returning empty reward_header_dependencies");
        return Ok(HashSet::new());
    };

    let coordinator = instance.coordinator.clone();
    let membership_lock = coordinator.membership().read().await;
    let first_epoch = membership_lock.first_epoch();
    drop(membership_lock);
    // add all the chain configs needed to apply STF to headers to the catchup
    for proposal in leaves {
        let header = proposal.block_header();

        let height = header.height();
        let view = proposal.view_number();
        tracing::debug!(height, ?view, "fetching dependencies for proposal");

        let version = header.version();
        // Skip if version is less than epoch version
        if version < EpochVersion::version() {
            continue;
        }

        let first_epoch = first_epoch.context("first epoch not found")?;

        let proposal_epoch = EpochNumber::new(epoch_from_block_number(height, epoch_height));

        // reward distribution starts third epoch onwards
        if proposal_epoch <= first_epoch + 1 {
            continue;
        }

        let epoch_membership = match coordinator.membership_for_epoch(Some(proposal_epoch)).await {
            Ok(e) => e,
            Err(err) => {
                tracing::info!(
                    "failed to get membership for epoch={proposal_epoch:?}. err={err:#}"
                );

                coordinator
                    .wait_for_catchup(proposal_epoch)
                    .await
                    .context(format!("failed to catchup for epoch={proposal_epoch:?}"))?
            },
        };

        let leader = epoch_membership.leader(proposal.view_number()).await?;
        let membership_lock = coordinator.membership().read().await;
        let validator = membership_lock.get_validator_config(&proposal_epoch, leader)?;
        drop(membership_lock);

        reward_accounts.insert(RewardAccount(validator.account));

        let delegators: Vec<RewardAccount> = validator
            .delegators
            .keys()
            .map(|d| RewardAccount(*d))
            .collect();

        reward_accounts.extend(delegators);
    }
    Ok(reward_accounts)
}

async fn get_leaf_from_proposal<Mode, P>(
    tx: &mut Transaction<Mode>,
    where_clause: &str,
    param: P,
) -> anyhow::Result<Leaf2>
where
    P: Type<Db> + for<'q> Encode<'q, Db>,
{
    let (data,) = query_as::<(Vec<u8>,)>(&format!(
        "SELECT data FROM quorum_proposals2 WHERE {where_clause} LIMIT 1",
    ))
    .bind(param)
    .fetch_one(tx.as_mut())
    .await?;
    let proposal: Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>> =
        bincode::deserialize(&data)?;
    Ok(Leaf2::from_quorum_proposal(&proposal.data))
}

#[cfg(any(test, feature = "testing"))]
mod impl_testable_data_source {

    use hotshot_query_service::data_source::storage::sql::testing::TmpDb;

    use super::*;
    use crate::api::{self, data_source::testing::TestableSequencerDataSource};

    fn tmp_options(db: &TmpDb) -> Options {
        #[cfg(not(feature = "embedded-db"))]
        {
            let opt = crate::persistence::sql::PostgresOptions {
                port: Some(db.port()),
                host: Some(db.host()),
                user: Some("postgres".into()),
                password: Some("password".into()),
                ..Default::default()
            };

            opt.into()
        }

        #[cfg(feature = "embedded-db")]
        {
            let opt = crate::persistence::sql::SqliteOptions {
                path: Some(db.path()),
            };
            opt.into()
        }
    }

    #[async_trait]
    impl TestableSequencerDataSource for DataSource {
        type Storage = TmpDb;

        async fn create_storage() -> Self::Storage {
            TmpDb::init().await
        }

        fn persistence_options(storage: &Self::Storage) -> Self::Options {
            tmp_options(storage)
        }

        fn leaf_only_ds_options(
            storage: &Self::Storage,
            opt: api::Options,
        ) -> anyhow::Result<api::Options> {
            let mut ds_opts = tmp_options(storage);
            ds_opts.lightweight = true;
            Ok(opt.query_sql(Default::default(), ds_opts))
        }

        fn options(storage: &Self::Storage, opt: api::Options) -> api::Options {
            opt.query_sql(Default::default(), tmp_options(storage))
        }
    }
}

#[cfg(test)]
mod generic_tests {
    use super::{super::api_tests, DataSource};
    // For some reason this is the only way to import the macro defined in another module of this
    // crate.
    use crate::*;

    instantiate_api_tests!(DataSource);
}
