use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::{bail, ensure, Context};
use async_trait::async_trait;
use committable::{Commitment, Committable};
use espresso_types::{
    get_l1_deposits,
    v0_1::IterableFeeInfo,
    v0_3::{
        ChainConfig, RewardAccountProofV1, RewardAccountQueryDataV1, RewardAccountV1, RewardAmount,
        RewardMerkleTreeV1, REWARD_MERKLE_TREE_V1_HEIGHT,
    },
    v0_4::{
        RewardAccountProofV2, RewardAccountQueryDataV2, RewardAccountV2, RewardMerkleTreeV2,
        REWARD_MERKLE_TREE_V2_HEIGHT,
    },
    BlockMerkleTree, DrbAndHeaderUpgradeVersion, EpochVersion, FeeAccount, FeeMerkleTree, Leaf2,
    NodeState, ValidatedState,
};
use hotshot::traits::ValidatedState as _;
use hotshot_query_service::{
    availability::{BlockId, LeafId},
    data_source::{
        sql::{Config, SqlDataSource, Transaction},
        storage::{
            sql::{query_as, Db, TransactionMode, Write},
            AvailabilityStorage, MerklizedStateHeightStorage, MerklizedStateStorage, NodeStorage,
            SqlStorage,
        },
        VersionedDataSource,
    },
    merklized_state::{MerklizedState, Snapshot},
    Resolvable,
};
use hotshot_types::{
    data::{EpochNumber, QuorumProposalWrapper, ViewNumber},
    message::Proposal,
    traits::node_implementation::ConsensusTime,
    utils::epoch_from_block_number,
    vote::HasViewNumber,
};
use jf_merkle_tree_compat::{
    prelude::MerkleNode, ForgetableMerkleTreeScheme, ForgetableUniversalMerkleTreeScheme,
    LookupResult, MerkleTreeScheme,
};
use serde_json::Value;
use sqlx::{Encode, Type};
use vbs::version::StaticVersionType;

use super::{
    data_source::{Provider, SequencerDataSource},
    BlocksFrontier,
};
use crate::{
    api::RewardAccountProofDataSource,
    catchup::{CatchupStorage, NullStateCatchup},
    persistence::{sql::Options, ChainConfigPersistence},
    state::compute_state_update,
    util::BoundedJoinSet,
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

impl RewardAccountProofDataSource for SqlStorage {
    async fn load_v1_reward_account_proof(
        &self,
        height: u64,
        account: RewardAccountV1,
    ) -> anyhow::Result<RewardAccountQueryDataV1> {
        let mut tx = self.read().await.context(format!(
            "opening transaction to fetch v1 reward account {account:?}; height {height}"
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
            let (tree, _) = load_v1_reward_accounts(self, height, &[account])
                .await
                .with_context(|| {
                    format!("failed to load v1 reward account {account:?} at height {height}")
                })?;

            let (proof, balance) = RewardAccountProofV1::prove(&tree, account.into())
                .with_context(|| {
                    format!("reward account {account:?} not available at height {height}")
                })?;

            Ok(RewardAccountQueryDataV1 { balance, proof })
        } else {
            bail!(
                "requested height {height} is not yet available (latest block height: \
                 {block_height})"
            );
        }
    }

    async fn load_v2_reward_account_proof(
        &self,
        height: u64,
        account: RewardAccountV2,
    ) -> anyhow::Result<RewardAccountQueryDataV2> {
        let mut tx = self.read().await.context(format!(
            "opening transaction to fetch reward account {account:?}; height {height}"
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
            let (tree, _) = load_v2_reward_accounts(self, height, &[account])
                .await
                .with_context(|| {
                    format!("failed to load v2 reward account {account:?} at height {height}")
                })?;

            let (proof, balance) = RewardAccountProofV2::prove(&tree, account.into())
                .with_context(|| {
                    format!("reward account {account:?} not available at height {height}")
                })?;

            Ok(RewardAccountQueryDataV2 { balance, proof })
        } else {
            bail!(
                "requested height {height} is not yet available (latest block height: \
                 {block_height})"
            );
        }
    }
}
impl CatchupStorage for SqlStorage {
    async fn get_reward_accounts_v1(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[RewardAccountV1],
    ) -> anyhow::Result<(RewardMerkleTreeV1, Leaf2)> {
        let mut tx = self.read().await.context(format!(
            "opening transaction to fetch v1 reward account {accounts:?}; height {height}"
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
            load_v1_reward_accounts(self, height, accounts).await
        } else {
            let accounts: Vec<_> = accounts
                .iter()
                .map(|acct| RewardAccountV2::from(*acct))
                .collect();
            // If we do not have the exact snapshot we need, we can try going back to the last
            // snapshot we _do_ have and replaying subsequent blocks to compute the desired state.
            let (state, leaf) = reconstruct_state(
                instance,
                self,
                &mut tx,
                block_height - 1,
                view,
                &[],
                &accounts,
            )
            .await?;
            Ok((state.reward_merkle_tree_v1, leaf))
        }
    }

    async fn get_reward_accounts_v2(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[RewardAccountV2],
    ) -> anyhow::Result<(RewardMerkleTreeV2, Leaf2)> {
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
            load_v2_reward_accounts(self, height, accounts).await
        } else {
            // If we do not have the exact snapshot we need, we can try going back to the last
            // snapshot we _do_ have and replaying subsequent blocks to compute the desired state.
            let (state, leaf) = reconstruct_state(
                instance,
                self,
                &mut tx,
                block_height - 1,
                view,
                &[],
                accounts,
            )
            .await?;
            Ok((state.reward_merkle_tree_v2, leaf))
        }
    }

    async fn get_all_reward_accounts(
        &self,
        height: u64,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<(RewardAccountV2, RewardAmount)>> {
        let mut tx = self.read().await.context(format!(
            "opening transaction to fetch all reward accounts; height {height}"
        ))?;

        let block_height = NodeStorage::<SeqTypes>::block_height(&mut tx)
            .await
            .context("getting block height")? as u64;
        ensure!(
            block_height > 0,
            "cannot get accounts for height {height}: no blocks available"
        );

        ensure!(
            height < block_height,
            "requested height {height} is not yet available (latest block height: {block_height})"
        );

        let merklized_state_height = tx
            .get_last_state_height()
            .await
            .context("getting merklized state height")? as u64;
        ensure!(
            height <= merklized_state_height,
            "requested height {height} is not yet available. latest merklized state height: \
             {merklized_state_height}"
        );

        let header = tx
            .get_header(BlockId::<SeqTypes>::from(height as usize))
            .await
            .context(format!("header {height} not available"))?;

        if header.version() < DrbAndHeaderUpgradeVersion::version() {
            return Ok(Vec::new());
        }

        // get the latest balance for each account.
        // use DISTINCT ON for Postgres
        // use ROW_NUMBER() as DISTINCT ON is not supported for SQLite
        #[cfg(not(feature = "embedded-db"))]
        let query = format!(
            "SELECT DISTINCT ON (idx) idx, entry
               FROM {}
              WHERE idx IS NOT NULL AND created <= $1
              ORDER BY idx DESC, created DESC
              LIMIT $2 OFFSET $3",
            RewardMerkleTreeV2::state_type()
        );

        #[cfg(feature = "embedded-db")]
        let query = format!(
            "SELECT idx, entry FROM (
                 SELECT idx, entry, ROW_NUMBER() OVER (PARTITION BY idx ORDER BY created DESC) as \
             rn
                   FROM {}
                  WHERE created <= $1 AND idx IS NOT NULL
             ) sub
             WHERE rn = 1
             ORDER BY idx DESC
             LIMIT $2 OFFSET $3",
            RewardMerkleTreeV2::state_type()
        );

        let rows = query_as::<(Value, Value)>(&query)
            .bind(height as i64)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(tx.as_mut())
            .await
            .context("loading reward accounts from storage")?;

        let mut accounts = Vec::new();
        for (idx, entry) in rows {
            let account: RewardAccountV2 =
                serde_json::from_value(idx).context("deserializing reward account")?;
            let balance: RewardAmount = serde_json::from_value(entry).context(format!(
                "deserializing reward balance for account {account}"
            ))?;

            accounts.push((account, balance));
        }

        Ok(accounts)
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
            let (state, leaf) = reconstruct_state(
                instance,
                self,
                &mut tx,
                block_height - 1,
                view,
                accounts,
                &[],
            )
            .await?;
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
                reconstruct_state(instance, self, &mut tx, block_height - 1, view, &[], &[])
                    .await?;
            match state.block_merkle_tree.lookup(height - 1) {
                LookupResult::Ok(_, proof) => Ok(proof),
                _ => {
                    bail!(
                        "state snapshot {view:?},{height} was found but does not contain frontier \
                         at height {}; this should not be possible",
                        height - 1
                    );
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

impl RewardAccountProofDataSource for DataSource {
    async fn load_v1_reward_account_proof(
        &self,
        height: u64,
        account: RewardAccountV1,
    ) -> anyhow::Result<RewardAccountQueryDataV1> {
        self.as_ref()
            .load_v1_reward_account_proof(height, account)
            .await
    }

    async fn load_v2_reward_account_proof(
        &self,
        height: u64,
        account: RewardAccountV2,
    ) -> anyhow::Result<RewardAccountQueryDataV2> {
        self.as_ref()
            .load_v2_reward_account_proof(height, account)
            .await
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

    async fn get_reward_accounts_v2(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[RewardAccountV2],
    ) -> anyhow::Result<(RewardMerkleTreeV2, Leaf2)> {
        self.as_ref()
            .get_reward_accounts_v2(instance, height, view, accounts)
            .await
    }

    async fn get_reward_accounts_v1(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[RewardAccountV1],
    ) -> anyhow::Result<(RewardMerkleTreeV1, Leaf2)> {
        self.as_ref()
            .get_reward_accounts_v1(instance, height, view, accounts)
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

    async fn get_all_reward_accounts(
        &self,
        height: u64,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<(RewardAccountV2, RewardAmount)>> {
        self.as_ref()
            .get_all_reward_accounts(height, offset, limit)
            .await
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

async fn load_v1_reward_accounts(
    db: &SqlStorage,
    height: u64,
    accounts: &[RewardAccountV1],
) -> anyhow::Result<(RewardMerkleTreeV1, Leaf2)> {
    // Open a new read transaction to get the leaf
    let mut tx = db
        .read()
        .await
        .with_context(|| "failed to open read transaction")?;

    // Get the leaf from the database
    let leaf = tx
        .get_leaf(LeafId::<SeqTypes>::from(height as usize))
        .await
        .context(format!("leaf {height} not available"))?;
    let header = leaf.header();

    if header.version() < EpochVersion::version()
        || header.version() >= DrbAndHeaderUpgradeVersion::version()
    {
        return Ok((
            RewardMerkleTreeV1::new(REWARD_MERKLE_TREE_V1_HEIGHT),
            leaf.leaf().clone(),
        ));
    }

    // Get the merkle root from the header and create a snapshot from it
    let merkle_root = header.reward_merkle_tree_root().unwrap_left();
    let mut snapshot = RewardMerkleTreeV1::from_commitment(merkle_root);

    // Create a bounded join set with 10 concurrent tasks
    let mut join_set = BoundedJoinSet::new(10);

    // Create a map from task ID to account
    let mut task_id_to_account = HashMap::new();

    // Loop through each account, spawning a task to get the path for the account
    for account in accounts {
        // Clone things we will need in the closure
        let db_clone = db.clone();
        let account_clone = *account;
        let header_height = header.height();

        // Create the closure that will get the path for the account
        let func = async move {
            // Open a new transaction
            let mut tx = db_clone
                .read()
                .await
                .with_context(|| "failed to open read transaction")?;

            // Get the path for the account
            let proof = tx
                .get_path(
                    Snapshot::<SeqTypes, RewardMerkleTreeV1, { RewardMerkleTreeV1::ARITY }>::Index(
                        header_height,
                    ),
                    account_clone,
                )
                .await
                .with_context(|| {
                    format!(
                        "failed to get path for v1 reward account {account_clone:?}; height \
                         {height}"
                    )
                })?;

            Ok::<_, anyhow::Error>(proof)
        };

        // Spawn the task
        let id = join_set.spawn(func).id();

        // Add the task ID to the account map
        task_id_to_account.insert(id, account);
    }

    // Wait for each task to complete
    while let Some(result) = join_set.join_next_with_id().await {
        // Get the inner result (past the join error)
        let (id, result) = result.with_context(|| "failed to join task")?;

        // Get the proof from the result
        let proof = result?;

        // Get the account from the task ID to account map
        let account = task_id_to_account
            .remove(&id)
            .with_context(|| "task ID for spawned task not found")?;

        match proof.proof.first().with_context(|| {
            format!("empty proof for v1 reward account {account:?}; height {height}")
        })? {
            MerkleNode::Leaf { pos, elem, .. } => {
                snapshot.remember(*pos, *elem, proof)?;
            },
            MerkleNode::Empty => {
                snapshot.non_membership_remember(*account, proof)?;
            },
            _ => {
                bail!("invalid proof for v1 reward account {account:?}; height {height}");
            },
        }
    }

    Ok((snapshot, leaf.leaf().clone()))
}

/// Loads reward accounts for new reward merkle tree (V4).
async fn load_v2_reward_accounts(
    db: &SqlStorage,
    height: u64,
    accounts: &[RewardAccountV2],
) -> anyhow::Result<(RewardMerkleTreeV2, Leaf2)> {
    // Open a new read transaction to get the leaf
    let mut tx = db
        .read()
        .await
        .with_context(|| "failed to open read transaction")?;

    // Get the leaf from the database
    let leaf = tx
        .get_leaf(LeafId::<SeqTypes>::from(height as usize))
        .await
        .with_context(|| format!("leaf {height} not available"))?;
    let header = leaf.header();

    // If the header is before the epoch version, we can return the new reward merkle tree
    if header.version() <= EpochVersion::version() {
        return Ok((
            RewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT),
            leaf.leaf().clone(),
        ));
    }

    // Get the merkle root from the header and create a snapshot from it
    let merkle_root = header.reward_merkle_tree_root().unwrap_right();
    let mut snapshot = RewardMerkleTreeV2::from_commitment(merkle_root);

    // Create a bounded join set with 10 concurrent tasks
    let mut join_set = BoundedJoinSet::new(10);

    // Create a map from task ID to account
    let mut task_id_to_account = HashMap::new();

    // Loop through each account, spawning a task to get the path for the account
    for account in accounts {
        // Clone things we will need in the closure
        let db_clone = db.clone();
        let account_clone = *account;
        let header_height = header.height();

        // Create the closure that will get the path for the account
        let func = async move {
            // Open a new transaction
            let mut tx = db_clone
                .read()
                .await
                .with_context(|| "failed to open read transaction")?;

            // Get the path for the account
            let proof = tx
                .get_path(
                    Snapshot::<SeqTypes, RewardMerkleTreeV2, { RewardMerkleTreeV2::ARITY }>::Index(
                        header_height,
                    ),
                    account_clone,
                )
                .await
                .with_context(|| {
                    format!(
                        "failed to get path for v2 reward account {account_clone:?}; height \
                         {height}"
                    )
                })?;

            Ok::<_, anyhow::Error>(proof)
        };

        // Spawn the task
        let id = join_set.spawn(func).id();

        // Add the task ID to the account map
        task_id_to_account.insert(id, account);
    }

    // Wait for each task to complete
    while let Some(result) = join_set.join_next_with_id().await {
        // Get the inner result (past the join error)
        let (id, result) = result.with_context(|| "failed to join task")?;

        // Get the proof from the result
        let proof = result?;

        // Get the account from the task ID to account map
        let account = task_id_to_account
            .remove(&id)
            .with_context(|| "task ID for spawned task not found")?;

        match proof.proof.first().with_context(|| {
            format!("empty proof for v2 reward account {account:?}; height {height}")
        })? {
            MerkleNode::Leaf { pos, elem, .. } => {
                snapshot.remember(*pos, *elem, proof)?;
            },
            MerkleNode::Empty => {
                snapshot.non_membership_remember(*account, proof)?;
            },
            _ => {
                bail!("invalid proof for v2 reward account {account:?}; height {height}");
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
pub(crate) async fn reconstruct_state<Mode: TransactionMode>(
    instance: &NodeState,
    db: &SqlStorage,
    tx: &mut Transaction<Mode>,
    from_height: u64,
    to_view: ViewNumber,
    fee_accounts: &[FeeAccount],
    reward_accounts: &[RewardAccountV2],
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
        "reconstructing fee accounts state for from height {from_height} to view {to_view}"
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
        "reconstructing reward accounts for from height {from_height} to view {to_view}"
    );

    let mut reward_accounts = reward_accounts.iter().copied().collect::<HashSet<_>>();

    // Collect all reward account dependencies needed for replaying the STF.
    // These accounts must be preloaded into the reward Merkle tree to prevent recursive catchups.
    let dependencies = reward_header_dependencies(instance, &leaves).await?;
    reward_accounts.extend(dependencies);
    let reward_accounts = reward_accounts.into_iter().collect::<Vec<_>>();

    // Load all required reward accounts and update the reward Merkle tree.
    match parent.block_header().reward_merkle_tree_root() {
        either::Either::Left(expected_root) => {
            let accts = reward_accounts
                .into_iter()
                .map(RewardAccountV1::from)
                .collect::<Vec<_>>();
            state.reward_merkle_tree_v1 = load_v1_reward_accounts(db, from_height, &accts)
                .await
                .context(
                    "unable to reconstruct state because v1 reward accounts are not available at \
                     origin",
                )?
                .0;
            ensure!(
                state.reward_merkle_tree_v1.commitment() == expected_root,
                "loaded v1 reward state does not match parent header"
            );
        },
        either::Either::Right(expected_root) => {
            state.reward_merkle_tree_v2 =
                load_v2_reward_accounts(db, from_height, &reward_accounts)
                    .await
                    .context(
                        "unable to reconstruct state because v2 reward accounts are not available \
                         at origin",
                    )?
                    .0;
            ensure!(
                state.reward_merkle_tree_v2.commitment() == expected_root,
                "loaded reward state does not match parent header"
            );
        },
    }

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
) -> anyhow::Result<HashSet<RewardAccountV2>> {
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
                    .context(format!("failed to catchup for epoch={proposal_epoch}"))?
            },
        };

        let leader = epoch_membership.leader(proposal.view_number()).await?;
        let membership_lock = coordinator.membership().read().await;
        let validator = membership_lock.get_validator_config(&proposal_epoch, leader)?;
        drop(membership_lock);

        reward_accounts.insert(RewardAccountV2(validator.account));

        let delegators: Vec<RewardAccountV2> = validator
            .delegators
            .keys()
            .map(|d| RewardAccountV2(*d))
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
pub(crate) mod impl_testable_data_source {

    use hotshot_query_service::data_source::storage::sql::testing::TmpDb;

    use super::*;
    use crate::api::{self, data_source::testing::TestableSequencerDataSource};

    pub fn tmp_options(db: &TmpDb) -> Options {
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
            let opt = crate::persistence::sql::SqliteOptions { path: db.path() };
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
mod tests {
    use alloy::primitives::Address;
    use espresso_types::{
        v0_3::RewardAmount,
        v0_4::{RewardAccountV2, RewardMerkleTreeV2, REWARD_MERKLE_TREE_V2_HEIGHT},
    };
    use hotshot_query_service::{
        data_source::{
            sql::Config,
            storage::{
                sql::{
                    testing::TmpDb, SqlStorage, StorageConnectionType,
                    Transaction as SqlTransaction, Write,
                },
                MerklizedStateStorage,
            },
            Transaction, VersionedDataSource,
        },
        merklized_state::{MerklizedState, Snapshot, UpdateStateData},
    };
    use jf_merkle_tree_compat::{
        LookupResult, MerkleTreeScheme, ToTraversalPath, UniversalMerkleTreeScheme,
    };

    use super::impl_testable_data_source::tmp_options;
    use crate::SeqTypes;

    fn make_reward_account(i: usize) -> RewardAccountV2 {
        let mut addr_bytes = [0u8; 20];
        addr_bytes[16..20].copy_from_slice(&(i as u32).to_be_bytes());
        RewardAccountV2(Address::from(addr_bytes))
    }

    async fn insert_test_header(
        tx: &mut SqlTransaction<Write>,
        block_height: u64,
        reward_tree: &RewardMerkleTreeV2,
    ) {
        let reward_commitment = serde_json::to_value(reward_tree.commitment()).unwrap();
        let test_data = serde_json::json!({
            "block_merkle_tree_root": format!("block_root_{}", block_height),
            "fee_merkle_tree_root": format!("fee_root_{}", block_height),
            "fields": {
                RewardMerkleTreeV2::header_state_commitment_field(): reward_commitment
            }
        });
        tx.upsert(
            "header",
            ["height", "hash", "payload_hash", "timestamp", "data"],
            ["height"],
            [(
                block_height as i64,
                format!("hash_{}", block_height),
                format!("payload_{}", block_height),
                block_height as i64,
                test_data,
            )],
        )
        .await
        .unwrap();
    }

    async fn batch_insert_proofs(
        tx: &mut SqlTransaction<Write>,
        reward_tree: &RewardMerkleTreeV2,
        accounts: &[RewardAccountV2],
        block_height: u64,
    ) {
        let proofs_and_paths: Vec<_> = accounts
            .iter()
            .map(|account| {
                let proof = match reward_tree.universal_lookup(*account) {
                    LookupResult::Ok(_, proof) => proof,
                    LookupResult::NotInMemory => panic!("account not in memory"),
                    LookupResult::NotFound(proof) => proof,
                };
                let traversal_path = <RewardAccountV2 as ToTraversalPath<
                    { RewardMerkleTreeV2::ARITY },
                >>::to_traversal_path(
                    account, reward_tree.height()
                );
                (proof, traversal_path)
            })
            .collect();

        UpdateStateData::<SeqTypes, RewardMerkleTreeV2, { RewardMerkleTreeV2::ARITY }>::insert_merkle_nodes_batch(
            tx,
            proofs_and_paths,
            block_height,
        )
        .await
        .expect("failed to batch insert proofs");
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_reward_accounts_batch_insertion() {
        // Batch insertion of 1000 accounts at height 1
        // Balance updates for some accounts at height 2
        // New accounts added at height 2
        // More balance updates at height 3
        // Querying correct balances at each height snapshot

        let db = TmpDb::init().await;
        let opt = tmp_options(&db);
        let cfg = Config::try_from(&opt).expect("failed to create config from options");
        let storage = SqlStorage::connect(cfg, StorageConnectionType::Query)
            .await
            .expect("failed to connect to storage");

        let num_initial_accounts = 1000usize;

        let initial_accounts: Vec<RewardAccountV2> =
            (0..num_initial_accounts).map(make_reward_account).collect();

        tracing::info!(
            "Height 1: Inserting {} initial accounts",
            num_initial_accounts
        );

        let mut reward_tree_h1 = RewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);

        for (i, account) in initial_accounts.iter().enumerate() {
            let reward_amount = RewardAmount::from(((i + 1) * 1000) as u64);
            reward_tree_h1.update(*account, reward_amount).unwrap();
        }

        let mut tx = storage.write().await.unwrap();
        insert_test_header(&mut tx, 1, &reward_tree_h1).await;
        batch_insert_proofs(&mut tx, &reward_tree_h1, &initial_accounts, 1).await;

        UpdateStateData::<SeqTypes, RewardMerkleTreeV2, { RewardMerkleTreeV2::ARITY }>::set_last_state_height(&mut tx, 1)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        tracing::info!("Height 2: Updating balances and adding new accounts");

        let mut reward_tree_h2 = reward_tree_h1.clone();

        // Update balances for accounts 0-99
        let updated_accounts_h2: Vec<RewardAccountV2> = (0..100).map(make_reward_account).collect();
        for (i, account) in updated_accounts_h2.iter().enumerate() {
            let new_reward = RewardAmount::from(((i + 1) * 2000) as u64);
            reward_tree_h2.update(*account, new_reward).unwrap();
        }

        // Add 100 new accounts (1000..1099)
        let new_accounts_h2: Vec<RewardAccountV2> = (1000..1100).map(make_reward_account).collect();
        for (i, account) in new_accounts_h2.iter().enumerate() {
            let reward_amount = RewardAmount::from(((i + 1001) * 500) as u64);
            reward_tree_h2.update(*account, reward_amount).unwrap();
        }

        let mut changed_accounts_h2 = updated_accounts_h2.clone();
        changed_accounts_h2.extend(new_accounts_h2.clone());

        let mut tx = storage.write().await.unwrap();
        insert_test_header(&mut tx, 2, &reward_tree_h2).await;
        batch_insert_proofs(&mut tx, &reward_tree_h2, &changed_accounts_h2, 2).await;

        UpdateStateData::<SeqTypes, RewardMerkleTreeV2, { RewardMerkleTreeV2::ARITY }>::set_last_state_height(&mut tx, 2)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        tracing::info!("Height 3: More balance updates");

        let mut reward_tree_h3 = reward_tree_h2.clone();

        // Update balances for accounts 500-599
        let updated_accounts_h3: Vec<RewardAccountV2> =
            (500..600).map(make_reward_account).collect();
        for (i, account) in updated_accounts_h3.iter().enumerate() {
            let new_reward = RewardAmount::from(((500 + i + 1) * 3000) as u64);
            reward_tree_h3.update(*account, new_reward).unwrap();
        }

        let mut tx = storage.write().await.unwrap();
        insert_test_header(&mut tx, 3, &reward_tree_h3).await;
        batch_insert_proofs(&mut tx, &reward_tree_h3, &updated_accounts_h3, 3).await;

        UpdateStateData::<SeqTypes, RewardMerkleTreeV2, { RewardMerkleTreeV2::ARITY }>::set_last_state_height(&mut tx, 3)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        tracing::info!("Verifying all account proofs at each height");

        // Verify height=1
        // All 1000 initial accounts
        let snapshot_h1 =
            Snapshot::<SeqTypes, RewardMerkleTreeV2, { RewardMerkleTreeV2::ARITY }>::Index(1);
        for i in 0..num_initial_accounts {
            let account = make_reward_account(i);
            let proof = storage
                .read()
                .await
                .unwrap()
                .get_path(snapshot_h1, account)
                .await
                .unwrap_or_else(|e| panic!("failed to get path for account {i} at h1: {e}"));

            let expected_reward = RewardAmount::from(((i + 1) * 1000) as u64);
            let actual_reward = proof.elem().expect("account should exist");
            assert_eq!(*actual_reward, expected_reward,);

            assert!(
                RewardMerkleTreeV2::verify(reward_tree_h1.commitment(), account, proof)
                    .unwrap()
                    .is_ok(),
            );
        }
        tracing::info!("Verified height=1 {num_initial_accounts} accounts with proofs",);

        // Verify accounts 1000-1099 don't exist at height 1
        for i in 1000..1100 {
            let account = make_reward_account(i);
            let proof = storage
                .read()
                .await
                .unwrap()
                .get_path(snapshot_h1, account)
                .await
                .unwrap();
            assert!(proof.elem().is_none(),);

            // Verify non-membership proof
            assert!(RewardMerkleTreeV2::non_membership_verify(
                reward_tree_h1.commitment(),
                account,
                proof
            )
            .unwrap(),);
        }
        tracing::info!("Height 1: Verified 100 non-membership proofs");

        // Verify height 2
        let snapshot_h2 =
            Snapshot::<SeqTypes, RewardMerkleTreeV2, { RewardMerkleTreeV2::ARITY }>::Index(2);

        // Accounts 0-99
        for i in 0..100 {
            let account = make_reward_account(i);
            let proof = storage
                .read()
                .await
                .unwrap()
                .get_path(snapshot_h2, account)
                .await
                .unwrap_or_else(|e| panic!("failed to get path for account {i} at h2: {e}"));

            let expected_reward = RewardAmount::from(((i + 1) * 2000) as u64);
            let actual_reward = proof.elem().expect("account should exist");
            assert_eq!(*actual_reward, expected_reward,);

            assert!(
                RewardMerkleTreeV2::verify(reward_tree_h2.commitment(), account, proof)
                    .unwrap()
                    .is_ok(),
            );
        }

        // Accounts 100-999: original rewards
        for i in 100..1000 {
            let account = make_reward_account(i);
            let proof = storage
                .read()
                .await
                .unwrap()
                .get_path(snapshot_h2, account)
                .await
                .unwrap_or_else(|e| panic!("failed to get path for account {i} at h2: {e}"));

            let expected_reward = RewardAmount::from(((i + 1) * 1000) as u64);
            let actual_reward = proof.elem().expect("account should exist");
            assert_eq!(*actual_reward, expected_reward,);

            assert!(
                RewardMerkleTreeV2::verify(reward_tree_h2.commitment(), account, proof)
                    .unwrap()
                    .is_ok(),
            );
        }

        // Accounts 1000-1099
        // new accounts
        for i in 1000..1100 {
            let account = make_reward_account(i);
            let proof = storage
                .read()
                .await
                .unwrap()
                .get_path(snapshot_h2, account)
                .await
                .unwrap_or_else(|e| panic!("failed to get path for account {i} at h2: {e}"));

            let expected_reward = RewardAmount::from(((i + 1) * 500) as u64);
            let actual_reward = proof.elem().expect("account should exist");
            assert_eq!(*actual_reward, expected_reward,);

            assert!(
                RewardMerkleTreeV2::verify(reward_tree_h2.commitment(), account, proof)
                    .unwrap()
                    .is_ok(),
            );
        }
        tracing::info!("Height 2: Verified all 1100 accounts with proofs");

        // Verify HEIGHT 3: All accounts
        let snapshot_h3 =
            Snapshot::<SeqTypes, RewardMerkleTreeV2, { RewardMerkleTreeV2::ARITY }>::Index(3);

        // Accounts 0-99
        for i in 0..100 {
            let account = make_reward_account(i);
            let proof = storage
                .read()
                .await
                .unwrap()
                .get_path(snapshot_h3, account)
                .await
                .unwrap_or_else(|e| panic!("failed to get path for account {i} at h3: {e}"));

            let expected_reward = RewardAmount::from(((i + 1) * 2000) as u64);
            let actual_reward = proof.elem().expect("account should exist");
            assert_eq!(*actual_reward, expected_reward,);

            assert!(
                RewardMerkleTreeV2::verify(reward_tree_h3.commitment(), account, proof)
                    .unwrap()
                    .is_ok(),
            );
        }

        for i in 100..500 {
            let account = make_reward_account(i);
            let proof = storage
                .read()
                .await
                .unwrap()
                .get_path(snapshot_h3, account)
                .await
                .unwrap_or_else(|e| panic!("failed to get path for account {i} at h3: {e}"));

            let expected_reward = RewardAmount::from(((i + 1) * 1000) as u64);
            let actual_reward = proof.elem().expect("account should exist");
            assert_eq!(*actual_reward, expected_reward,);

            assert!(
                RewardMerkleTreeV2::verify(reward_tree_h3.commitment(), account, proof)
                    .unwrap()
                    .is_ok(),
            );
        }

        // Accounts 500-599
        for i in 500..600 {
            let account = make_reward_account(i);
            let proof = storage
                .read()
                .await
                .unwrap()
                .get_path(snapshot_h3, account)
                .await
                .unwrap_or_else(|e| panic!("failed to get path for account {i} at h3: {e}"));

            let expected_reward = RewardAmount::from(((i + 1) * 3000) as u64);
            let actual_reward = proof.elem().expect("account should exist");
            assert_eq!(*actual_reward, expected_reward,);

            assert!(
                RewardMerkleTreeV2::verify(reward_tree_h3.commitment(), account, proof)
                    .unwrap()
                    .is_ok(),
            );
        }

        // Accounts 600-999
        for i in 600..1000 {
            let account = make_reward_account(i);
            let proof = storage
                .read()
                .await
                .unwrap()
                .get_path(snapshot_h3, account)
                .await
                .unwrap_or_else(|e| panic!("failed to get path for account {i} at h3: {e}"));

            let expected_reward = RewardAmount::from(((i + 1) * 1000) as u64);
            let actual_reward = proof.elem().expect("account should exist");
            assert_eq!(*actual_reward, expected_reward,);

            assert!(
                RewardMerkleTreeV2::verify(reward_tree_h3.commitment(), account, proof)
                    .unwrap()
                    .is_ok(),
            );
        }

        // Accounts 1000-1099: new accounts (from h2)
        for i in 1000..1100 {
            let account = make_reward_account(i);
            let proof = storage
                .read()
                .await
                .unwrap()
                .get_path(snapshot_h3, account)
                .await
                .unwrap_or_else(|e| panic!("failed to get path for account {i} at h3: {e}"));

            let expected_reward = RewardAmount::from(((i + 1) * 500) as u64);
            let actual_reward = proof.elem().expect("account should exist");
            assert_eq!(*actual_reward, expected_reward,);

            assert!(
                RewardMerkleTreeV2::verify(reward_tree_h3.commitment(), account, proof)
                    .unwrap()
                    .is_ok(),
            );
        }
        tracing::info!("Height 3: Verified all 1100 accounts with proofs");

        // Verify non-membership proofs for accounts that never existed
        for i in 1100..1110 {
            let account = make_reward_account(i);
            let proof = storage
                .read()
                .await
                .unwrap()
                .get_path(snapshot_h3, account)
                .await
                .unwrap();

            assert!(
                proof.elem().is_none(),
                "Account {i} should not exist at height 3"
            );

            assert!(RewardMerkleTreeV2::non_membership_verify(
                reward_tree_h3.commitment(),
                account,
                proof
            )
            .unwrap(),);
        }
        tracing::info!("Height 3: Verified 10 non-membership proofs");
    }
}
