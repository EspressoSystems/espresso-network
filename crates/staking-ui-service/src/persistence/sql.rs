//! SQL-based persistent storage
use crate::{
    Error, Result,
    input::{
        espresso::{ActiveNode, ActiveNodeSet, EspressoPersistence, RewardDistribution},
        l1::{L1BlockSnapshot, L1Persistence, NodeSet, Snapshot, Update, Wallet, Wallets},
    },
    types::{
        common::{
            Address, Delegation, ESPTokenAmount, EpochAndBlock, L1BlockId, NodeSetEntry,
            PendingWithdrawal, Ratio,
        },
        global::{ActiveNodeSetDiff, ActiveNodeSetUpdate, FullNodeSetDiff},
        wallet::WalletDiff,
    },
};
use alloy::primitives::U256;
use anyhow::Context;
use clap::Parser;
use futures::{TryStreamExt, future::try_join_all};
use serde_json::Value;
use sqlx::{
    ConnectOptions, QueryBuilder,
    sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions},
};
use std::{collections::HashMap, path::PathBuf, str::FromStr};
use tracing::{instrument, log::LevelFilter};

/// Options for persistence.
#[derive(Parser, Clone, Debug)]
pub struct PersistenceOptions {
    /// Path to the SQLite database file.
    ///
    /// If the file does not exist, it will be created.
    /// The parent directory must exist.
    #[clap(long, env = "ESPRESSO_STAKING_SERVICE_DB_PATH")]
    pub path: PathBuf,

    /// Maximum number of connections in the connection pool.
    #[clap(
        long,
        env = "ESPRESSO_STAKING_SERVICE_DB_MAX_CONNECTIONS",
        default_value = "5"
    )]
    pub max_connections: u32,
}

#[derive(Debug, Clone)]
pub struct Persistence {
    pool: SqlitePool,
}

impl Persistence {
    /// Create a new SQLite database with the given options.
    pub async fn new(options: &PersistenceOptions) -> Result<Self> {
        let connect_options = SqliteConnectOptions::from_str(
            options
                .path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("invalid path"))?,
        )?
        .create_if_missing(true)
        .log_statements(LevelFilter::Debug);

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(options.max_connections)
            .connect_with(connect_options)
            .await?;

        Self::run_migrations(&pool).await?;

        tracing::info!("SQLite persistence initialized successfully");
        Ok(Self { pool })
    }

    /// Run database migrations using SQLx's migration system
    async fn run_migrations(pool: &SqlitePool) -> Result<()> {
        tracing::warn!("running database migrations");

        sqlx::migrate!("./migrations/sqlite")
            .run(pool)
            .await
            .context("failed to run migrations")?;

        tracing::warn!("migrations completed");
        Ok(())
    }

    /// Load the finalized snapshot from the database.
    ///
    /// Returns `None` if no snapshot exists (i.e., database is empty).
    /// Otherwise returns the complete state including:
    /// - L1 block info (hash, number, timestamp, exit escrow period)
    /// - Full node set (all registered validators)
    /// - All wallet states (delegations, pending withdrawals, claimed rewards)
    #[instrument(skip(self))]
    async fn load_finalized_snapshot(&self) -> Result<Option<Snapshot>> {
        tracing::info!("loading finalized snapshot from database");

        // The l1_block table has only one row, representing the latest finalized block
        let block_row = sqlx::query_as::<_, (String, i64, String, i64, i64)>(
            "SELECT hash, number, parent_hash, timestamp, exit_escrow_period FROM l1_block LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some((hash, number, parent_hash, timestamp, exit_escrow_period)) = block_row else {
            tracing::info!("no finalized snapshot found in database");
            return Ok(None);
        };

        let block = L1BlockSnapshot {
            id: L1BlockId {
                hash: hash.parse().context("failed to parse block hash")?,
                number: number as u64,
                parent: parent_hash.parse().context("failed to parse parent hash")?,
            },
            timestamp: timestamp as u64,
            exit_escrow_period: exit_escrow_period as u64,
        };

        // Load all registered nodes
        let node_rows = sqlx::query_as::<_, (String, String, String, f64, String, Option<Value>)>(
            "SELECT address, staking_key, state_key, commission, stake, metadata FROM node",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut node_set = NodeSet::default();
        for (address, staking_key, state_key, commission, stake_str, metadata) in node_rows {
            let node = NodeSetEntry {
                address: address.parse().context("failed to parse node address")?,
                staking_key: staking_key.parse().context("failed to parse staking key")?,
                state_key: state_key.parse().context("failed to parse state key")?,
                stake: U256::from_str(&stake_str).context("failed to parse node stake")?,
                commission: Ratio::from(commission as f32),
                metadata: metadata
                    .map(serde_json::from_value)
                    .transpose()
                    .context("failed to parse metadata")?,
            };
            node_set.push(node);
        }

        // Load all wallets and their delegations in parallel
        let wallet_rows =
            sqlx::query_as::<_, (String, String)>("SELECT address, claimed_rewards FROM wallet")
                .fetch_all(&self.pool)
                .await?;

        let wallet_futures =
            wallet_rows
                .into_iter()
                .map(|(wallet_address, claimed_rewards_str)| {
                    let pool = self.pool.clone();
                    async move {
                        let address: Address = wallet_address
                            .parse()
                            .context("failed to parse wallet address")?;
                        let claimed_rewards = U256::from_str(&claimed_rewards_str)
                            .context("failed to parse claimed rewards")?;

                        // Get active delegations
                        let delegation_rows = sqlx::query_as::<_, (String, String)>(
                            "SELECT node, amount
                             FROM delegation
                             WHERE delegator = $1",
                        )
                        .bind(&wallet_address)
                        .fetch_all(&pool)
                        .await?;

                        let mut nodes = im::OrdMap::new();
                        for (node_str, amount_str) in delegation_rows {
                            let node: Address =
                                node_str.parse().context("failed to parse node address")?;
                            let amount = U256::from_str(&amount_str)
                                .context("failed to parse delegation amount")?;
                            if !amount.is_zero() {
                                let delegation = Delegation {
                                    delegator: address,
                                    node,
                                    amount,
                                };
                                nodes.insert(node, delegation);
                            }
                        }

                        // Get pending withdrawals
                        let pending_rows = sqlx::query_as::<_, (String, String, String, i64)>(
                            "SELECT node, withdrawal_type, amount, unlocks_at
                             FROM pending_withdrawals
                             WHERE delegator = $1",
                        )
                        .bind(&wallet_address)
                        .fetch_all(&pool)
                        .await?;

                        let mut pending_undelegations = im::OrdMap::new();
                        let mut pending_exits = im::OrdMap::new();

                        for (node_str, withdrawal_type_str, amount_str, available_time) in
                            pending_rows
                        {
                            let node: Address =
                                node_str.parse().context("failed to parse node address")?;
                            let amount = U256::from_str(&amount_str)
                                .context("failed to parse pending amount")?;
                            let withdrawal_type = WithdrawalType::try_from(withdrawal_type_str)?;

                            let withdrawal = PendingWithdrawal {
                                delegator: address,
                                node,
                                amount,
                                available_time: available_time as u64,
                            };

                            match withdrawal_type {
                                WithdrawalType::Undelegation => {
                                    pending_undelegations.insert(node, withdrawal);
                                }
                                WithdrawalType::Exit => {
                                    pending_exits.insert(node, withdrawal);
                                }
                            }
                        }

                        let wallet = Wallet {
                            nodes,
                            pending_undelegations,
                            pending_exits,
                            claimed_rewards,
                        };
                        Ok::<_, anyhow::Error>((address, wallet))
                    }
                });

        let wallet_results = try_join_all(wallet_futures).await?;
        let mut wallets = Wallets::default();
        for (address, wallet) in wallet_results {
            wallets.insert(address, wallet);
        }

        let snapshot = Snapshot {
            block,
            node_set,
            wallets,
        };

        tracing::info!(block = ?snapshot.block, "loaded finalized snapshot");
        Ok(Some(snapshot))
    }

    /// Apply a full node set diff to the database.
    async fn apply_node_set_diff(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        diff: &FullNodeSetDiff,
    ) -> Result<()> {
        match diff {
            FullNodeSetDiff::NodeUpdate(node) => {
                let metadata = node
                    .metadata
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()
                    .context("serializing metadata")?;
                sqlx::query(
                    "INSERT INTO node (address, staking_key, state_key, commission, stake, metadata)
                     VALUES ($1, $2, $3, $4, $5, $6)
                     ON CONFLICT(address) DO UPDATE SET
                         staking_key = excluded.staking_key,
                         state_key = excluded.state_key,
                         commission = excluded.commission,
                         stake = excluded.stake,
                         metadata = excluded.metadata",
                )
                .bind(node.address.to_string())
                .bind(node.staking_key.to_string())
                .bind(node.state_key.to_string())
                .bind(f32::from(node.commission) as f64)
                .bind(node.stake.to_string())
                .bind(metadata)
                .execute(&mut **tx)
                .await?;
            }
            FullNodeSetDiff::NodeExit(exit) => {
                let result = sqlx::query("DELETE FROM node WHERE address = $1")
                    .bind(exit.address.to_string())
                    .execute(&mut **tx)
                    .await?;

                if result.rows_affected() != 1 {
                    return Err(anyhow::anyhow!(
                        "Expected to delete 1 node row, but {} were affected",
                        result.rows_affected()
                    ))
                    .map_err(Into::into);
                }
            }
        }
        Ok(())
    }

    /// Apply a wallet diff to the database.
    #[instrument(skip(self, tx, diff))]
    async fn apply_wallet_diff(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        address: Address,
        diff: &WalletDiff,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO wallet (address, claimed_rewards)
             VALUES ($1, '0')
             ON CONFLICT(address) DO NOTHING",
        )
        .bind(address.to_string())
        .execute(&mut **tx)
        .await?;

        match diff {
            WalletDiff::ClaimedRewards(amount) => {
                // Read current claimed rewards
                let (claimed_rewards,) = sqlx::query_as::<_, (String,)>(
                    "SELECT claimed_rewards FROM wallet WHERE address = $1",
                )
                .bind(address.to_string())
                .fetch_one(&mut **tx)
                .await?;

                let current_amount = U256::from_str(&claimed_rewards).unwrap_or(U256::ZERO);
                let new_claimed_rewards = current_amount
                    .checked_add(U256::from(*amount))
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Overflow: adding {amount} to claimed rewards {current_amount}"
                        )
                    })?;

                // Update claimed rewards with the computed value
                let result = sqlx::query(
                    "UPDATE wallet
                     SET claimed_rewards = $1
                     WHERE address = $2",
                )
                .bind(new_claimed_rewards.to_string())
                .bind(address.to_string())
                .execute(&mut **tx)
                .await?;

                if result.rows_affected() != 1 {
                    return Err(anyhow::anyhow!(
                        "Expected to update 1 wallet row, but {} were affected",
                        result.rows_affected()
                    ))
                    .map_err(Into::into);
                }
            }
            WalletDiff::DelegatedToNode(delegation) => {
                // Check if delegation exists
                let existing = sqlx::query_as::<_, (String,)>(
                    "SELECT amount FROM delegation WHERE delegator = $1 AND node = $2",
                )
                .bind(delegation.delegator.to_string())
                .bind(delegation.node.to_string())
                .fetch_optional(&mut **tx)
                .await?;

                let new_amount = if let Some((amount,)) = existing {
                    let current_amount = U256::from_str(&amount).unwrap_or(U256::ZERO);
                    current_amount.checked_add(delegation.amount)
                        .ok_or_else(|| anyhow::anyhow!(
                            "Overflow: adding delegation {} to existing amount {current_amount}",
                            delegation.amount
                        ))?
                } else {
                    delegation.amount
                };

                // upsert
                sqlx::query(
                    "INSERT INTO delegation (delegator, node, amount)
                     VALUES ($1, $2, $3)
                     ON CONFLICT(delegator, node) DO UPDATE SET
                        amount = excluded.amount",
                )
                .bind(delegation.delegator.to_string())
                .bind(delegation.node.to_string())
                .bind(new_amount.to_string())
                .execute(&mut **tx)
                .await?;
            }
            WalletDiff::UndelegatedFromNode(withdrawal) => {
                let (amount,) = sqlx::query_as::<_, (String,)>(
                    "SELECT amount FROM delegation WHERE delegator = $1 AND node = $2",
                )
                .bind(withdrawal.delegator.to_string())
                .bind(withdrawal.node.to_string())
                .fetch_one(&mut **tx)
                .await?;

                let current_amount = U256::from_str(&amount).unwrap_or(U256::ZERO);
                let new_amount = current_amount.checked_sub(withdrawal.amount).ok_or_else(
                    || {
                        anyhow::anyhow!(
                            "Underflow: withdrawal {} exceeds delegation amount {current_amount}",
                            withdrawal.amount
                        )
                    },
                )?;

                // If new amount is zero, delete the delegation
                // otherwise update it
                if new_amount.is_zero() {
                    let result =
                        sqlx::query("DELETE FROM delegation WHERE delegator = $1 AND node = $2")
                            .bind(withdrawal.delegator.to_string())
                            .bind(withdrawal.node.to_string())
                            .execute(&mut **tx)
                            .await?;

                    if result.rows_affected() != 1 {
                        return Err(anyhow::anyhow!(
                            "Expected to delete 1 delegation row, but {} were affected",
                            result.rows_affected()
                        ))
                        .map_err(Into::into);
                    }
                } else {
                    // Update delegation amount
                    let result = sqlx::query(
                        "UPDATE delegation
                         SET amount = $1
                         WHERE delegator = $2 AND node = $3",
                    )
                    .bind(new_amount.to_string())
                    .bind(withdrawal.delegator.to_string())
                    .bind(withdrawal.node.to_string())
                    .execute(&mut **tx)
                    .await?;

                    if result.rows_affected() != 1 {
                        return Err(anyhow::anyhow!(
                            "Expected to update 1 delegation row, but {} were affected",
                            result.rows_affected()
                        ))
                        .map_err(Into::into);
                    }
                }

                // Insert pending undelegation
                // we expect this to fail if there's already a pending undelegation
                sqlx::query(
                    "INSERT INTO pending_withdrawals (delegator, node, withdrawal_type, amount, unlocks_at)
                     VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(withdrawal.delegator.to_string())
                .bind(withdrawal.node.to_string())
                .bind(String::from(WithdrawalType::Undelegation))
                .bind(withdrawal.amount.to_string())
                .bind(withdrawal.available_time as i64)
                .execute(&mut **tx)
                .await?;
            }
            WalletDiff::NodeExited(withdrawal) => {
                // Delete the delegation as node has exited
                let result = sqlx::query(
                    "DELETE FROM delegation
                     WHERE delegator = $1 AND node = $2",
                )
                .bind(withdrawal.delegator.to_string())
                .bind(withdrawal.node.to_string())
                .execute(&mut **tx)
                .await?;

                if result.rows_affected() != 1 {
                    return Err(anyhow::anyhow!(
                        "Expected to delete 1 delegation for node exit, but {} were affected",
                        result.rows_affected()
                    ))
                    .map_err(Into::into);
                }

                // Insert pending exit with the amount from the withdrawal
                // we expect this to fail if there is already a pending exit
                sqlx::query(
                    "INSERT INTO pending_withdrawals (delegator, node, withdrawal_type, amount, unlocks_at)
                     VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(withdrawal.delegator.to_string())
                .bind(withdrawal.node.to_string())
                .bind(String::from(WithdrawalType::Exit))
                .bind(withdrawal.amount.to_string())
                .bind(withdrawal.available_time as i64)
                .execute(&mut **tx)
                .await?;
            }
            WalletDiff::UndelegationWithdrawal(withdrawal) => {
                // Delete the pending undelegation withdrawal
                let result = sqlx::query(
                    "DELETE FROM pending_withdrawals
                     WHERE delegator = $1 AND node = $2 AND withdrawal_type = 'undelegation'",
                )
                .bind(withdrawal.delegator.to_string())
                .bind(withdrawal.node.to_string())
                .execute(&mut **tx)
                .await?;

                if result.rows_affected() != 1 {
                    return Err(anyhow::anyhow!(
                        "Expected to delete 1 pending undelegation, but {} were affected",
                        result.rows_affected()
                    ))
                    .map_err(Into::into);
                }
            }
            WalletDiff::NodeExitWithdrawal(withdrawal) => {
                // Delete the pending exit withdrawal
                let result = sqlx::query(
                    "DELETE FROM pending_withdrawals
                     WHERE delegator = $1 AND node = $2 AND withdrawal_type = 'exit'",
                )
                .bind(withdrawal.delegator.to_string())
                .bind(withdrawal.node.to_string())
                .execute(&mut **tx)
                .await?;

                if result.rows_affected() != 1 {
                    return Err(anyhow::anyhow!(
                        "Expected to delete 1 pending exit, but {} were affected",
                        result.rows_affected()
                    ))
                    .map_err(Into::into);
                }
            }
        }
        Ok(())
    }
}

/// Type of pending withdrawal
#[derive(Clone, Copy, Debug, PartialEq)]
enum WithdrawalType {
    /// Withdrawal due to pending undelegation
    Undelegation,
    /// Full withdrawal due to validator exit
    Exit,
}

impl TryFrom<String> for WithdrawalType {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "undelegation" => Ok(WithdrawalType::Undelegation),
            "exit" => Ok(WithdrawalType::Exit),
            _ => Err(anyhow::anyhow!("Unknown withdrawal type: {value}")),
        }
    }
}

impl From<WithdrawalType> for String {
    fn from(val: WithdrawalType) -> Self {
        match val {
            WithdrawalType::Undelegation => "undelegation".to_string(),
            WithdrawalType::Exit => "exit".to_string(),
        }
    }
}

impl L1Persistence for Persistence {
    async fn finalized_snapshot(&self) -> Result<Option<Snapshot>> {
        self.load_finalized_snapshot().await
    }

    async fn save_genesis(&mut self, snapshot: Snapshot) -> Result<()> {
        tracing::info!(block = ?snapshot.block, "saving genesis L1 block");

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO l1_block (hash, number, parent_hash, timestamp, exit_escrow_period)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(snapshot.block.id().hash.to_string())
        .bind(snapshot.block.id().number as i64)
        .bind(snapshot.block.id().parent.to_string())
        .bind(snapshot.block.timestamp() as i64)
        .bind(snapshot.block.exit_escrow_period as i64)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        tracing::info!("genesis L1 block saved");
        Ok(())
    }

    #[instrument(skip(self, updates))]
    async fn apply_updates(&mut self, updates: Vec<Update>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let mut latest_l1_block = None;
        for update in &updates {
            // In rare cases, this update or a later one may have already been processed. This can
            // only happen outside the control of this program, since we do have an exclusive lock
            // on the database here. For example, in some cases AWS may briefly run two instances of
            // the service at the same time, causing each instance to process the same update.
            //
            // These updates are not inherently idempotent, since some updates involve incrementing
            // amounts, and others (like node exits) cause an error if applied twice. Thus, we must
            // detect this case and avoid applying an update that has already been applied.
            //
            // We are at least in a SQLite write transaction, and two simultaneous write
            // transactions are prohibited from both committing. So we do not have to worry about
            // races within this function itself (e.g. we check, the update has not been applied so
            // we proceed, then another process applies the update, then we duplicate). We are only
            // worried about the same update being applied twice, _one after another_, by two
            // processes that aren't aware of each other.
            let (last_block,) = sqlx::query_as("SELECT number FROM l1_block LIMIT 1")
                .fetch_one(tx.as_mut())
                .await
                .context("loading last L1 block number")?;
            if update.block.number() <= last_block {
                // The update or a later one has already been applied. Although this is not
                // technically an error, it is exceptional and not expected to happen often, so we
                // should report it loudly.
                tracing::error!(
                    last_block,
                    ?update.block,
                    "update has already been applied; this is OK, but may indicate two instances \
                    of this service are sharing a database, which is best to be avoided"
                );

                // We can continue here without erroring because the update is already applied,
                // which means the work we were trying to do here is already done, even if it was
                // done by somebody else.
                //
                // Note that this does rely on determinism of updates; i.e. any two processes
                // modifying the same database (which _should_ be, at worst, two exact copies of
                // this same service) will always generate the same updates for the same sequence of
                // L1 blocks. Otherwise, we might be leaving the database in a different state than
                // if we had applied the update we were given, if someone else had applied a
                // different update for the same block.
                //
                // This determinism assumption should be upheld by the rest of the logic in this
                // service, with the exception that `NodeUpdate` diffs may have different `metadata`
                // content when processed by different services, due to inherent nondeterminism in
                // third-party services providing metadata. This is acceptable since the contents of
                // the metadata are informational only: they do not effect state _transitions_, and
                // so at worst, our database will be left with different metadata for a node than it
                // otherwise would have had by continuing here, but this will not cause our state to
                // diverge any further: this metadata will not be touched until it is eventually
                // overwritten in its entirety by a subsequent `MetadataUriUpdated` event.
                continue;
            }

            tracing::debug!(
                block_number = update.block.number(),
                node_set_diffs = update.node_set_diffs.len(),
                wallet_diffs = update.wallet_diffs.len(),
                "applying events to database"
            );

            // Apply node set diffs
            for diff in &update.node_set_diffs {
                self.apply_node_set_diff(&mut tx, diff).await?;
            }

            // Apply wallet diffs
            for (address, diffs) in &update.wallet_diffs {
                tracing::debug!(%address, num = diffs.len(), "applying diffs for wallet");
                for diff in diffs {
                    self.apply_wallet_diff(&mut tx, *address, diff).await?;
                }
            }
            tracing::debug!("events applied successfully");
            latest_l1_block = Some(update.block);
        }

        // Update L1 block info to that of the latest new update.
        if let Some(block) = latest_l1_block {
            sqlx::query(
                "UPDATE l1_block SET
             hash = $1,
             number = $2,
             parent_hash = $3,
             timestamp = $4,
             exit_escrow_period = $5",
            )
            .bind(block.hash().to_string())
            .bind(block.number() as i64)
            .bind(block.parent().to_string())
            .bind(block.timestamp() as i64)
            .bind(block.exit_escrow_period as i64)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

impl EspressoPersistence for Persistence {
    async fn active_node_set(&self) -> Result<Option<ActiveNodeSet>> {
        let mut tx = self.pool.begin().await.context("acquiring connection")?;

        // Load saved Espresso block.
        let espresso_block: Option<(u64, u64, u64, f32)> = sqlx::query_as(
            "SELECT number, epoch, timestamp, apr FROM espresso_block
              WHERE always_one = 1 LIMIT 1",
        )
        .fetch_optional(tx.as_mut())
        .await
        .context("loading last Espresso block")?;
        let Some((block, epoch, timestamp, apr)) = espresso_block else {
            return Ok(None);
        };
        let espresso_block = EpochAndBlock {
            block,
            epoch,
            timestamp,
        };

        // Load active nodes.
        let nodes = sqlx::query_as::<_, (String, u64, u64, u64)>(
            "SELECT address, votes, proposals, slots FROM active_node ORDER BY idx",
        )
        .fetch_all(tx.as_mut())
        .await
        .context("loading active node set")?;
        let nodes = nodes
            .into_iter()
            .map(|(address, votes, proposals, slots)| {
                let address = address.parse().context("invalid address")?;
                let proposals = proposals.try_into().context("proposal count overflow")?;
                let slots = slots.try_into().context("slot count overflow")?;
                let votes = votes.try_into().context("vote count overflow")?;
                anyhow::Ok(ActiveNode {
                    address,
                    proposals,
                    slots,
                    votes,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(Some(ActiveNodeSet {
            espresso_block,
            apr: apr.into(),
            nodes,
        }))
    }

    async fn lifetime_rewards(&self, account: Address) -> Result<ESPTokenAmount> {
        let mut tx = self.pool.begin().await.context("acquiring connection")?;
        let amount_opt = sqlx::query_as::<_, (String,)>(
            "SELECT amount FROM lifetime_rewards WHERE address = $1",
        )
        .bind(account.to_string())
        .fetch_optional(tx.as_mut())
        .await
        .context(format!("fetching lifetime reward amount for {account}"))?;
        match amount_opt {
            Some((amount_str,)) => Ok(amount_str.parse().context("invalid amount string")?),
            None => {
                // All accounts start at 0, if we don't have another value for this account it has
                // never accrued any rewards.
                Ok(ESPTokenAmount::ZERO)
            }
        }
    }

    async fn initialize_lifetime_rewards(
        &mut self,
        rewards: Vec<(Address, ESPTokenAmount)>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.context("acquiring connection")?;

        sqlx::query("DELETE FROM lifetime_rewards")
            .execute(tx.as_mut())
            .await
            .context("clearing existing lifetime rewards")?;

        if !rewards.is_empty() {
            QueryBuilder::new("INSERT INTO lifetime_rewards (address, amount) ")
                .push_values(
                    rewards
                        .into_iter()
                        .map(|(addr, amount)| (addr.to_string(), amount.to_string())),
                    |mut q, (addr, amount)| {
                        q.push_bind(addr);
                        q.push_bind(amount);
                    },
                )
                .build()
                .execute(tx.as_mut())
                .await
                .context("initializing lifetime rewards")?;
        }

        tx.commit().await.context("committing transaction")?;

        Ok(())
    }

    async fn apply_update(
        &mut self,
        update: ActiveNodeSetUpdate,
        rewards: RewardDistribution,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await.context("acquiring connection")?;

        // In rare cases, this update or a later one may have already been processed. This can only
        // happen outside the control of this program, since we do have an exclusive lock on the
        // database here. For example, in some cases AWS may briefly run two instances of the
        // service at the same time, causing each instance to process the same update.
        //
        // These updates are not inherently idempotent, since they involve incrementing the
        // statistics columns. Thus, we must detect this case and avoid applying an update that has
        // already been applied.
        //
        // We are at least in a SQLite write transaction, and two simultaneous write transactions
        // are prohibited from both committing. So we do not have to worry about races within this
        // function itself (e.g. we check, the update has not been applied so we proceed, then
        // another process applies the update, then we duplicate). We are only worried about the
        // same update being applied twice, _one after another_, by two processes that aren't aware
        // of each other.
        let last_block: Option<(u64,)> =
            sqlx::query_as("SELECT number FROM espresso_block WHERE always_one = 1 LIMIT 1")
                .fetch_optional(tx.as_mut())
                .await
                .context("loading last Espresso block number")?;
        if let Some((last_block,)) = last_block {
            if update.espresso_block.block <= last_block {
                // The update or a later one has already been applied. Although this is not
                // technically an error, it is exceptional and not expected to happen often, so we
                // should report it loudly.
                tracing::error!(
                    last_block,
                    ?update.espresso_block,
                    "update has already been applied; this is OK, but may indicate two instances \
                    of this service are sharing a database, which is best to be avoided"
                );

                // We can return `Ok` here because the update is already applied, which means the
                // work of this function is already done, even if it was done by somebody else. We
                // want to tell the caller that they are OK to move forward, rather than retrying
                // this update and hitting the same error again.
                //
                // Note that this does rely on determinism of updates; i.e. any two processes
                // modifying the same database (which _should_ be, at worst, two exact copies of
                // this same service) will always generate the same update for the same Espresso
                // block. Otherwise, we might be returning `Ok` here but leaving the database in a
                // different state than if we had applied the update we were given, if someone else
                // had applied a different update for the same block. This determinism assumption
                // should be upheld by the rest of the logic in this service.
                return Ok(());
            } else if update.espresso_block.block > last_block + 1 {
                // Now THIS is a real error. We have skipped an update somehow. This cannot be
                // caused by two processes running at the same time, and should never be possible
                // within a single process either. Something has gone quite wrong.
                //
                // The exception to this is when the update starts with a `NewEpoch` event, which is
                // just going to wipe out any state we missed anyways. And it is common to skip a
                // block for new epochs, as when we start up the service, we always fast forward to
                // the current epoch regardless of where we left off.
                if update.diff.is_empty()
                    || matches!(&update.diff[0], ActiveNodeSetDiff::NewEpoch { .. })
                {
                    tracing::info!(
                        last_block,
                        ?update,
                        "skipping blocks to fast forward to new epoch"
                    );
                } else {
                    return Err(Error::internal().context(format!(
                    "an update has been skipped; last_update: {last_block}, current update: {:?}",
                    update.espresso_block
                )));
                }
            }
        } else {
            // We are dealing with an empty database; this is the first transaction ever to write to
            // the espresso-related tables. Obviously this update has not already been applied, so
            // we don't have to worry about this case.
        }

        // Update Espresso block information.
        sqlx::query(
            "INSERT INTO espresso_block (always_one, number, epoch, timestamp, apr) 
                VALUES (1, $1, $2, $3, 0)
                ON CONFLICT (always_one) DO UPDATE SET
                    number    = excluded.number,
                    epoch     = excluded.epoch,
                    timestamp = excluded.timestamp",
        )
        .bind(i64::try_from(update.espresso_block.block).context("Espresso block overflow")?)
        .bind(i64::try_from(update.espresso_block.epoch).context("Espresso epoch overflow")?)
        .bind(
            i64::try_from(update.espresso_block.timestamp)
                .context("Espresso timestamp overflow")?,
        )
        .execute(tx.as_mut())
        .await
        .context("updating Espresso block")?;

        // Apply changes to active node set.
        for diff in update.diff {
            match diff {
                ActiveNodeSetDiff::NewEpoch { nodes, apr } => {
                    sqlx::query("UPDATE espresso_block SET apr = $1")
                        .bind(f32::from(apr))
                        .execute(tx.as_mut())
                        .await
                        .context("updating APR")?;

                    let nodes = nodes
                        .into_iter()
                        .enumerate()
                        .map(|(i, addr)| {
                            let i = i64::try_from(i).context("node index overflow")?;
                            Ok((i, addr.to_string()))
                        })
                        .collect::<anyhow::Result<Vec<_>>>()?;

                    // Remove nodes from previous epoch before inserting new nodes.
                    sqlx::query("DELETE FROM active_node")
                        .execute(tx.as_mut())
                        .await
                        .context("deleting old active node set")?;

                    QueryBuilder::new(
                        "INSERT INTO active_node (idx, address, votes, proposals, slots) ",
                    )
                    .push_values(nodes, |mut q, (i, address)| {
                        q.push_bind(i)
                            .push_bind(address)
                            .push("0")
                            .push("0")
                            .push("0");
                    })
                    .build()
                    .execute(tx.as_mut())
                    .await
                    .context("inserting new active node set")?;
                }
                ActiveNodeSetDiff::NewBlock {
                    leader,
                    failed_leaders,
                    voters,
                } => {
                    let leader = i32::try_from(leader).context("leader index overflow")?;
                    let voters = voters
                        .iter_ones()
                        .map(|i| i32::try_from(i).context("voter index overflow"))
                        .collect::<anyhow::Result<Vec<_>>>()?;

                    // Increment leader's slot and proposal counts.
                    sqlx::query(
                        "UPDATE active_node SET proposals = proposals + 1, slots = slots + 1
                            WHERE idx = $1",
                    )
                    .bind(leader)
                    .execute(tx.as_mut())
                    .await
                    .context("updating leader stats")?;

                    // Update slot counts for leaders who missed a proposal.
                    if !failed_leaders.is_empty() {
                        // Count the number of times each node index appears in `failed_leaders`.
                        let mut increments: HashMap<i32, i32> = HashMap::new();
                        for index in failed_leaders {
                            let index =
                                i32::try_from(index).context("failed leader index overflow")?;
                            *increments.entry(index).or_default() += 1i32;
                        }
                        // Increment the slots count each node that appears in `failed_leaders` by
                        // the number of times it appears.
                        QueryBuilder::new(
                            "UPDATE active_node AS n
                                SET slots = n.slots + increments.count
                                FROM (SELECT column1 AS idx, column2 AS count FROM (",
                        )
                        .push_values(increments, |mut q, (index, count)| {
                            q.push_bind(index).push_bind(count);
                        })
                        .push(")) AS increments WHERE n.idx = increments.idx")
                        .build()
                        .execute(tx.as_mut())
                        .await
                        .context("updating failed leader stats")?;
                    }

                    // Update vote counts for everyone who signed the QC.
                    if !voters.is_empty() {
                        QueryBuilder::new("UPDATE active_node SET votes = votes + 1 WHERE idx IN ")
                            .push_tuples(voters, |mut q, i| {
                                q.push_bind(i);
                            })
                            .build()
                            .execute(tx.as_mut())
                            .await
                            .context("updating voter stats")?;
                    }
                }
            }
        }

        // Dole out rewards.
        if !rewards.is_empty() {
            // handle duplicates
            // if node is a validator and also a delegator
            let mut aggregated_rewards: HashMap<Address, ESPTokenAmount> = HashMap::new();
            for (addr, amount) in rewards {
                *aggregated_rewards.entry(addr).or_default() += amount;
            }

            let current_amounts =
                QueryBuilder::new("SELECT address, amount FROM lifetime_rewards WHERE address IN ")
                    .push_tuples(aggregated_rewards.keys(), |mut q, addr| {
                        q.push_bind(addr.to_string());
                    })
                    .build_query_as::<(String, String)>()
                    .fetch(tx.as_mut())
                    .map_err(anyhow::Error::new)
                    .and_then(|(addr, amount)| async move {
                        let addr: Address = addr.parse().context("invalid address")?;
                        let amount: ESPTokenAmount = amount.parse().context("invalid amount")?;
                        Ok((addr, amount))
                    })
                    .try_collect::<HashMap<Address, ESPTokenAmount>>()
                    .await
                    .context("fetching current reward amounts")?;
            let new_amounts = aggregated_rewards.into_iter().map(|(addr, amount)| {
                let new_amount = amount + current_amounts.get(&addr).copied().unwrap_or_default();
                (addr.to_string(), new_amount.to_string())
            });
            QueryBuilder::new("INSERT INTO lifetime_rewards (address, amount) ")
                .push_values(new_amounts, |mut q, (addr, amount)| {
                    q.push_bind(addr);
                    q.push_bind(amount);
                })
                .push("ON CONFLICT (address) DO UPDATE SET amount = excluded.amount")
                .build()
                .execute(tx.as_mut())
                .await
                .context("storing new reward amounts")?;
        }

        Ok(tx.commit().await?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::input::l1::testing::{block_snapshot, make_node};
    use crate::types::common::{
        Address, ESPTokenAmount, ImageSet, NodeExit, NodeMetadata, NodeMetadataContent,
        PendingWithdrawal, RatioSet, Withdrawal,
    };
    use crate::types::global::FullNodeSetDiff;
    use espresso_types::PubKey;
    use hotshot_types::traits::signature_key::BuilderSignatureKey;
    use im::ordmap;

    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    /// Tests the complete persistence lifecycle
    #[test_log::test(tokio::test)]
    async fn test_snapshot_save_apply_load() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let options = PersistenceOptions {
            path: db_path,
            max_connections: 5,
        };

        let mut persistence = Persistence::new(&options).await.unwrap();

        let node1 = make_node(1);
        let node2 = make_node(2);
        let node3 = make_node(3);
        let node4 = make_node(4);

        let delegator1 = Address::random();
        let delegator2 = Address::random();
        let delegator3 = Address::random();

        let genesis_snapshot = Snapshot::empty(block_snapshot(0));
        persistence
            .save_genesis(genesis_snapshot.clone())
            .await
            .unwrap();

        let snapshot = persistence
            .load_finalized_snapshot()
            .await
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.block.number(), 0);
        assert_eq!(snapshot.node_set.len(), 0);
        assert_eq!(snapshot.wallets.len(), 0);

        // Block 100: Register 4 nodes, set up initial delegations, delegator1 withdraws and claims rewards
        let initial_node_set_diffs = vec![
            FullNodeSetDiff::NodeUpdate(Arc::new(node1.clone())),
            FullNodeSetDiff::NodeUpdate(Arc::new(node2.clone())),
            FullNodeSetDiff::NodeUpdate(Arc::new(node3.clone())),
            FullNodeSetDiff::NodeUpdate(Arc::new(node4.clone())),
        ];

        let initial_wallet_diffs = [
            (
                delegator1,
                vec![
                    // Delegator1 delegates to node1
                    WalletDiff::DelegatedToNode(Delegation {
                        delegator: delegator1,
                        node: node1.address,
                        amount: U256::from(6000000u64), // Initial amount before undelegation
                    }),
                    // Delegator1 undelegates from node1
                    WalletDiff::UndelegatedFromNode(PendingWithdrawal {
                        delegator: delegator1,
                        node: node1.address,
                        amount: U256::from(1000000u64),
                        available_time: 800,
                    }),
                    // Delegator1 withdraws the undelegation and then claims rewards
                    WalletDiff::UndelegationWithdrawal(Withdrawal {
                        delegator: delegator1,
                        node: node1.address,
                        amount: U256::from(1000000u64),
                    }),
                    // Delegator1 claims rewards
                    WalletDiff::ClaimedRewards(U256::from(500)),
                    // Delegator1 delegates to node2
                    WalletDiff::DelegatedToNode(Delegation {
                        delegator: delegator1,
                        node: node2.address,
                        amount: U256::from(3000000u64),
                    }),
                ],
            ),
            (
                delegator2,
                vec![
                    // Delegator2 delegates to node1
                    WalletDiff::DelegatedToNode(Delegation {
                        delegator: delegator2,
                        node: node1.address,
                        amount: U256::from(1000000u64),
                    }),
                    // Delegator2 undelegates from node1
                    WalletDiff::UndelegatedFromNode(PendingWithdrawal {
                        delegator: delegator2,
                        node: node1.address,
                        amount: U256::from(500000u64),
                        available_time: 900,
                    }),
                    // Delegator2 delegates to node2
                    WalletDiff::DelegatedToNode(Delegation {
                        delegator: delegator2,
                        node: node2.address,
                        amount: U256::from(10000000u64),
                    }),
                    // Delegator2 delegates to node3
                    WalletDiff::DelegatedToNode(Delegation {
                        delegator: delegator2,
                        node: node3.address,
                        amount: U256::from(2000000u64),
                    }),
                ],
            ),
            // Delegator3 delegates to node4
            (
                delegator3,
                vec![WalletDiff::DelegatedToNode(Delegation {
                    delegator: delegator3,
                    node: node4.address,
                    amount: U256::from(8000000u64),
                })],
            ),
        ]
        .into_iter()
        .collect();

        persistence
            .apply_updates(vec![Update {
                block: block_snapshot(100),
                node_set_diffs: initial_node_set_diffs,
                wallet_diffs: initial_wallet_diffs,
            }])
            .await
            .unwrap();

        // Verify state after block 100
        let snapshot_after_block_100 = persistence
            .load_finalized_snapshot()
            .await
            .unwrap()
            .unwrap();

        assert_eq!(snapshot_after_block_100.block.number(), 100);
        assert_eq!(snapshot_after_block_100.node_set.len(), 4);
        assert_eq!(snapshot_after_block_100.wallets.len(), 3);

        // Verify delegator1 after block 100
        let wallet1_block100 = snapshot_after_block_100.wallets.get(&delegator1).unwrap();
        assert_eq!(wallet1_block100.nodes.len(), 2);
        let node1_del = wallet1_block100.nodes.get(&node1.address).unwrap();
        assert_eq!(node1_del.amount, U256::from(5000000u64));
        assert_eq!(wallet1_block100.pending_undelegations.len(), 0);
        assert_eq!(wallet1_block100.claimed_rewards, U256::from(500u64));

        // Verify delegator2 after block 100
        let wallet2_block100 = snapshot_after_block_100.wallets.get(&delegator2).unwrap();
        assert_eq!(wallet2_block100.nodes.len(), 3);
        assert_eq!(wallet2_block100.pending_undelegations.len(), 1);
        assert_eq!(wallet2_block100.claimed_rewards, U256::ZERO);

        // Verify delegator3 after block 100
        let wallet3_block100 = snapshot_after_block_100.wallets.get(&delegator3).unwrap();
        assert_eq!(wallet3_block100.nodes.len(), 1);
        assert_eq!(wallet3_block100.pending_exits.len(), 0);

        // Block 101: Node4 exits, triggering NodeExited for delegator3
        let node_exit_diffs = vec![FullNodeSetDiff::NodeExit(NodeExit {
            address: node4.address,
            exit_time: 1500,
        })];

        let node_exit_wallet_diffs = [(
            delegator3,
            vec![WalletDiff::NodeExited(PendingWithdrawal {
                delegator: delegator3,
                node: node4.address,
                amount: U256::from(8000000u64),
                available_time: 1500,
            })],
        )]
        .into_iter()
        .collect();

        // Apply node4 exit in block 101
        persistence
            .apply_updates(vec![Update {
                block: block_snapshot(101),
                node_set_diffs: node_exit_diffs,
                wallet_diffs: node_exit_wallet_diffs,
            }])
            .await
            .unwrap();

        // Verify state after block 101
        let snapshot = persistence
            .load_finalized_snapshot()
            .await
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.block.number(), 101);
        assert_eq!(snapshot.node_set.len(), 3); // node4 exited
        assert!(
            snapshot
                .node_set
                .iter()
                .any(|(_, n)| n.address == node1.address)
        );
        assert!(
            snapshot
                .node_set
                .iter()
                .any(|(_, n)| n.address == node2.address)
        );
        assert!(
            snapshot
                .node_set
                .iter()
                .any(|(_, n)| n.address == node3.address)
        );
        // node4 should NOT be in the node set anymore since it exited
        assert!(
            !snapshot
                .node_set
                .iter()
                .any(|(_, n)| n.address == node4.address)
        );
        assert_eq!(snapshot.wallets.len(), 3);

        // Verify delegator1
        let loaded_wallet1 = snapshot.wallets.get(&delegator1).unwrap();
        assert_eq!(loaded_wallet1.nodes.len(), 2);

        // Delegator1 had 6M, undelegated 1M
        let node1_delegation = loaded_wallet1
            .nodes
            .get(&node1.address)
            .expect("should have delegation to node1");
        assert_eq!(node1_delegation.amount, U256::from(5000000u64));

        assert_eq!(loaded_wallet1.pending_exits.len(), 0);
        assert_eq!(loaded_wallet1.pending_undelegations.len(), 0);
        assert_eq!(loaded_wallet1.claimed_rewards, U256::from(500u64));

        // Verify delegator2
        let loaded_wallet2 = snapshot.wallets.get(&delegator2).unwrap();
        assert_eq!(loaded_wallet2.nodes.len(), 3); // node1, node2, node3

        // Check node1 still has remaining delegation after undelegation
        let node1_delegation = loaded_wallet2
            .nodes
            .get(&node1.address)
            .expect("should have remaining delegation to node1");
        assert_eq!(node1_delegation.amount, U256::from(500000u64));

        assert!(loaded_wallet2.nodes.contains_key(&node2.address));
        assert!(loaded_wallet2.nodes.contains_key(&node3.address));
        assert_eq!(loaded_wallet2.pending_undelegations.len(), 1);
        assert_eq!(
            loaded_wallet2
                .pending_undelegations
                .get(&node1.address)
                .unwrap()
                .amount,
            U256::from(500000u64)
        );
        assert_eq!(loaded_wallet2.pending_exits.len(), 0);
        assert_eq!(loaded_wallet2.claimed_rewards, U256::ZERO);

        // Verify delegator3
        let loaded_wallet3 = snapshot.wallets.get(&delegator3).unwrap();
        assert_eq!(loaded_wallet3.nodes.len(), 0);
        assert_eq!(loaded_wallet3.pending_undelegations.len(), 0);
        assert_eq!(loaded_wallet3.pending_exits.len(), 1);
        let node4_exit = loaded_wallet3.pending_exits.get(&node4.address).unwrap();
        assert_eq!(node4_exit.node, node4.address);
        assert_eq!(node4_exit.amount, U256::from(8000000u64));
        assert_eq!(node4_exit.available_time, 1500);
        assert_eq!(loaded_wallet3.claimed_rewards, U256::ZERO);

        // Block 102: Register node5, node2 exits, withdrawals and claim rewards
        let node5 = make_node(5);

        let node_set_diffs = vec![
            FullNodeSetDiff::NodeUpdate(Arc::new(node5.clone())),
            FullNodeSetDiff::NodeExit(NodeExit {
                address: node2.address,
                exit_time: 1200,
            }),
        ];

        let wallet_diffs = [
            (
                delegator1,
                vec![
                    // Delegator1 undelegates 1M from node1
                    WalletDiff::UndelegatedFromNode(PendingWithdrawal {
                        delegator: delegator1,
                        node: node1.address,
                        amount: U256::from(1000000u64),
                        available_time: 1100,
                    }),
                    // Node2 exits
                    // delegator1's delegation moves to pending_exits
                    WalletDiff::NodeExited(PendingWithdrawal {
                        delegator: delegator1,
                        node: node2.address,
                        amount: U256::from(3000000u64),
                        available_time: 1200,
                    }),
                    // Delegator1 claims  rewards
                    WalletDiff::ClaimedRewards(U256::from(1500)),
                ],
            ),
            (
                delegator2,
                vec![
                    // Node2 exits
                    //  delegator2 delegation moves to pending_exits
                    WalletDiff::NodeExited(PendingWithdrawal {
                        delegator: delegator2,
                        node: node2.address,
                        amount: U256::from(10000000u64),
                        available_time: 1200,
                    }),
                    // Delegator2 completes withdrawal of undelegation from block 100
                    WalletDiff::UndelegationWithdrawal(Withdrawal {
                        delegator: delegator2,
                        node: node1.address,
                        amount: U256::from(500000u64),
                    }),
                    // Delegator2 claims rewards
                    WalletDiff::ClaimedRewards(U256::from(750)),
                ],
            ),
            (
                delegator3,
                vec![
                    // Delegator3 withdrawal from exited node4
                    WalletDiff::NodeExitWithdrawal(Withdrawal {
                        delegator: delegator3,
                        node: node4.address,
                        amount: U256::from(8000000u64),
                    }),
                    // Delegator3 delegates to new node5
                    WalletDiff::DelegatedToNode(Delegation {
                        delegator: delegator3,
                        node: node5.address,
                        amount: U256::from(15000000u64),
                    }),
                ],
            ),
        ]
        .into_iter()
        .collect();

        let updated_block = block_snapshot(102);

        persistence
            .apply_updates(vec![Update {
                block: updated_block,
                node_set_diffs,
                wallet_diffs,
            }])
            .await
            .unwrap();

        // Verify final state after block 102
        let loaded_snapshot = persistence
            .load_finalized_snapshot()
            .await
            .unwrap()
            .unwrap();

        assert_eq!(loaded_snapshot.block, updated_block);
        assert_eq!(loaded_snapshot.node_set.len(), 3); // node1, node3, node5 (node2 and node4 exited)

        assert!(
            loaded_snapshot
                .node_set
                .iter()
                .any(|(_, n)| n.address == node1.address)
        );
        // node2 exited in block 102
        assert!(
            !loaded_snapshot
                .node_set
                .iter()
                .any(|(_, n)| n.address == node2.address)
        );
        assert!(
            loaded_snapshot
                .node_set
                .iter()
                .any(|(_, n)| n.address == node3.address)
        );
        // node4 exited in block 101
        assert!(
            !loaded_snapshot
                .node_set
                .iter()
                .any(|(_, n)| n.address == node4.address)
        );
        assert!(
            loaded_snapshot
                .node_set
                .iter()
                .any(|(_, n)| n.address == node5.address)
        );

        let wallet1 = loaded_snapshot
            .wallets
            .get(&delegator1)
            .expect("wallet1 should exist");
        assert_eq!(wallet1.claimed_rewards, U256::from(2000u64));
        assert_eq!(wallet1.nodes.len(), 1);
        let node1_delegation = wallet1
            .nodes
            .get(&node1.address)
            .expect("should have delegation to node1");
        assert_eq!(node1_delegation.node, node1.address);
        assert_eq!(node1_delegation.amount, U256::from(4000000u64));
        assert_eq!(wallet1.pending_undelegations.len(), 1);
        assert_eq!(
            wallet1
                .pending_undelegations
                .get(&node1.address)
                .unwrap()
                .amount,
            U256::from(1000000u64)
        );
        assert_eq!(wallet1.pending_exits.len(), 1);
        assert_eq!(
            wallet1.pending_exits.get(&node2.address).unwrap().node,
            node2.address
        );
        assert_eq!(
            wallet1.pending_exits.get(&node2.address).unwrap().amount,
            U256::from(3000000u64)
        );

        let wallet2 = loaded_snapshot
            .wallets
            .get(&delegator2)
            .expect("wallet2 should exist");
        assert_eq!(wallet2.claimed_rewards, U256::from(750u64)); // Claimed in block 102
        assert_eq!(wallet2.nodes.len(), 2); // node1  and node3

        let node1_delegation = wallet2
            .nodes
            .get(&node1.address)
            .expect("should still have delegation to node1");
        assert_eq!(node1_delegation.amount, U256::from(500000u64));

        let node3_delegation = wallet2
            .nodes
            .get(&node3.address)
            .expect("should have delegation to node3");
        assert_eq!(node3_delegation.amount, U256::from(2000000u64));

        assert_eq!(wallet2.pending_undelegations.len(), 0);

        assert_eq!(wallet2.pending_exits.len(), 1);
        assert_eq!(
            wallet2.pending_exits.get(&node2.address).unwrap().node,
            node2.address
        );
        assert_eq!(
            wallet2.pending_exits.get(&node2.address).unwrap().amount,
            U256::from(10000000u64)
        );

        let wallet3 = loaded_snapshot
            .wallets
            .get(&delegator3)
            .expect("wallet3 should exist");
        assert_eq!(wallet3.claimed_rewards, U256::ZERO);
        assert_eq!(wallet3.nodes.len(), 1); // Only node5 delegation remains
        let node5_delegation = wallet3
            .nodes
            .get(&node5.address)
            .expect("should have delegation to node5");
        assert_eq!(node5_delegation.amount, U256::from(15000000u64));
        assert_eq!(wallet3.pending_undelegations.len(), 0);
        // NodeExitWithdrawal completed, so no more pending_exits
        assert_eq!(wallet3.pending_exits.len(), 0);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_active_node_set_new_epoch_from_genesis() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let options = PersistenceOptions {
            path: db_path,
            max_connections: 5,
        };
        let mut persistence = Persistence::new(&options).await.unwrap();

        // Prior to the first update being processed, we have no snapshot.
        assert_eq!(persistence.active_node_set().await.unwrap(), None);

        // Store an update with an Espresso block and a list of nodes.
        let espresso_block = EpochAndBlock {
            block: 100,
            epoch: 200,
            timestamp: 300,
        };
        let nodes = vec![Address::random(), Address::random()];
        let apr = Ratio::new(1, 100);
        persistence
            .apply_update(
                ActiveNodeSetUpdate {
                    espresso_block,
                    diff: vec![ActiveNodeSetDiff::NewEpoch {
                        nodes: nodes.clone(),
                        apr,
                    }],
                },
                Default::default(),
            )
            .await
            .unwrap();
        let snapshot = persistence.active_node_set().await.unwrap().unwrap();
        assert_eq!(snapshot.espresso_block, espresso_block);
        assert_eq!(
            snapshot.nodes,
            [ActiveNode::new(nodes[0]), ActiveNode::new(nodes[1])]
        );
        assert_eq!(snapshot.apr, apr);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_active_node_set_new_epoch_nodes_added() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let options = PersistenceOptions {
            path: db_path,
            max_connections: 5,
        };
        let mut persistence = Persistence::new(&options).await.unwrap();

        // Prior to the first update being processed, we have no snapshot.
        assert_eq!(persistence.active_node_set().await.unwrap(), None);

        // Store an update with an Espresso block and a list of nodes.
        let mut espresso_block = EpochAndBlock {
            block: 100,
            epoch: 200,
            timestamp: 300,
        };
        let nodes = vec![Address::random(), Address::random()];
        persistence
            .apply_update(
                ActiveNodeSetUpdate {
                    espresso_block,
                    diff: vec![
                        ActiveNodeSetDiff::NewEpoch {
                            nodes: nodes.clone(),
                            apr: Ratio::new(1, 100),
                        },
                        // Populate some statistics. These will become stale in the next epoch and
                        // we can check that they get cleared.
                        ActiveNodeSetDiff::NewBlock {
                            leader: 0,
                            failed_leaders: vec![],
                            voters: [true, true].into_iter().collect(),
                        },
                    ],
                },
                Default::default(),
            )
            .await
            .unwrap();
        let snapshot = persistence.active_node_set().await.unwrap().unwrap();
        assert_eq!(snapshot.espresso_block, espresso_block);
        assert_eq!(snapshot.nodes.len(), nodes.len());
        for (node, addr) in snapshot.nodes.into_iter().zip(&nodes) {
            assert_eq!(node.address, *addr);
            assert_ne!(node.votes, 0);
        }

        // Start a new epoch, expanding and reordering the list of nodes.
        let new_nodes = vec![Address::random(), Address::random(), nodes[0], nodes[1]];
        // On a new epoch event, the block number is allowed to increase by more than one.
        espresso_block.block += 2;
        persistence
            .apply_update(
                ActiveNodeSetUpdate {
                    espresso_block,
                    diff: vec![ActiveNodeSetDiff::NewEpoch {
                        nodes: new_nodes.clone(),
                        apr: Ratio::new(2, 100),
                    }],
                },
                Default::default(),
            )
            .await
            .unwrap();
        let snapshot = persistence.active_node_set().await.unwrap().unwrap();
        assert_eq!(
            snapshot.nodes,
            new_nodes
                .iter()
                .copied()
                .map(ActiveNode::new)
                .collect::<Vec<_>>()
        );
        assert_eq!(snapshot.apr, Ratio::new(2, 100));
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_active_node_set_new_epoch_nodes_removed() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let options = PersistenceOptions {
            path: db_path,
            max_connections: 5,
        };
        let mut persistence = Persistence::new(&options).await.unwrap();

        // Prior to the first update being processed, we have no snapshot.
        assert_eq!(persistence.active_node_set().await.unwrap(), None);

        // Store an update with an Espresso block and a list of nodes.
        let mut espresso_block = EpochAndBlock {
            block: 100,
            epoch: 200,
            timestamp: 300,
        };
        let nodes = vec![Address::random(), Address::random()];
        persistence
            .apply_update(
                ActiveNodeSetUpdate {
                    espresso_block,
                    diff: vec![
                        ActiveNodeSetDiff::NewEpoch {
                            nodes: nodes.clone(),
                            apr: Ratio::new(1, 100),
                        },
                        // Populate some statistics. These will become stale in the next epoch and
                        // we can check that they get cleared.
                        ActiveNodeSetDiff::NewBlock {
                            leader: 0,
                            failed_leaders: vec![],
                            voters: [true, true].into_iter().collect(),
                        },
                    ],
                },
                Default::default(),
            )
            .await
            .unwrap();
        let snapshot = persistence.active_node_set().await.unwrap().unwrap();
        assert_eq!(snapshot.espresso_block, espresso_block);
        assert_eq!(snapshot.nodes.len(), nodes.len());
        for (node, addr) in snapshot.nodes.into_iter().zip(&nodes) {
            assert_eq!(node.address, *addr);
            assert_ne!(node.votes, 0);
        }

        // Start a new epoch, removing the first node.
        let new_nodes = vec![nodes[1]];
        // On a new epoch event, the block number is allowed to increase by more than one.
        espresso_block.block += 2;
        persistence
            .apply_update(
                ActiveNodeSetUpdate {
                    espresso_block,
                    diff: vec![ActiveNodeSetDiff::NewEpoch {
                        nodes: new_nodes.clone(),
                        apr: Ratio::new(1, 100),
                    }],
                },
                Default::default(),
            )
            .await
            .unwrap();
        let snapshot = persistence.active_node_set().await.unwrap().unwrap();
        assert_eq!(
            snapshot.nodes,
            new_nodes
                .iter()
                .copied()
                .map(ActiveNode::new)
                .collect::<Vec<_>>()
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_active_node_set_new_block_stats() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let options = PersistenceOptions {
            path: db_path,
            max_connections: 5,
        };
        let mut persistence = Persistence::new(&options).await.unwrap();

        let nodes = vec![
            Address::random(),
            Address::random(),
            Address::random(),
            Address::random(),
        ];
        let leader = 0;
        let failed_leaders = vec![1, 2];
        let voters = [false, false, true, true].into_iter().collect();

        let update = ActiveNodeSetUpdate {
            espresso_block: EpochAndBlock {
                epoch: 0,
                block: 0,
                timestamp: 0,
            },
            diff: vec![
                ActiveNodeSetDiff::NewEpoch {
                    nodes: nodes.clone(),
                    apr: Ratio::new(1, 100),
                },
                ActiveNodeSetDiff::NewBlock {
                    leader,
                    failed_leaders,
                    voters,
                },
            ],
        };
        persistence
            .apply_update(update, Default::default())
            .await
            .unwrap();
        let node_set = persistence.active_node_set().await.unwrap().unwrap();
        assert_eq!(
            node_set.nodes,
            [
                ActiveNode {
                    address: nodes[0],
                    votes: 0,
                    proposals: 1,
                    slots: 1,
                },
                ActiveNode {
                    address: nodes[1],
                    votes: 0,
                    proposals: 0,
                    slots: 1,
                },
                ActiveNode {
                    address: nodes[2],
                    votes: 1,
                    proposals: 0,
                    slots: 1,
                },
                ActiveNode {
                    address: nodes[3],
                    votes: 1,
                    proposals: 0,
                    slots: 0,
                }
            ]
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_active_node_set_new_block_no_failed_leaders() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let options = PersistenceOptions {
            path: db_path,
            max_connections: 5,
        };
        let mut persistence = Persistence::new(&options).await.unwrap();

        let nodes = vec![Address::random(), Address::random()];
        let leader = 0;
        let voters = [false, true].into_iter().collect();

        let update = ActiveNodeSetUpdate {
            espresso_block: EpochAndBlock {
                epoch: 0,
                block: 0,
                timestamp: 0,
            },
            diff: vec![
                ActiveNodeSetDiff::NewEpoch {
                    nodes: nodes.clone(),
                    apr: Ratio::new(1, 100),
                },
                ActiveNodeSetDiff::NewBlock {
                    leader,
                    failed_leaders: vec![],
                    voters,
                },
            ],
        };
        persistence
            .apply_update(update, Default::default())
            .await
            .unwrap();
        let node_set = persistence.active_node_set().await.unwrap().unwrap();
        assert_eq!(
            node_set.nodes,
            [
                ActiveNode {
                    address: nodes[0],
                    votes: 0,
                    proposals: 1,
                    slots: 1,
                },
                ActiveNode {
                    address: nodes[1],
                    votes: 1,
                    proposals: 0,
                    slots: 0,
                },
            ]
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_active_node_set_new_block_duplicate_failed_leaders() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let options = PersistenceOptions {
            path: db_path,
            max_connections: 5,
        };
        let mut persistence = Persistence::new(&options).await.unwrap();

        let nodes = vec![Address::random(), Address::random()];
        let leader = 0;
        let failed_leaders = vec![0, 1, 1];
        let voters = [true, true].into_iter().collect();

        let update = ActiveNodeSetUpdate {
            espresso_block: EpochAndBlock {
                epoch: 0,
                block: 0,
                timestamp: 0,
            },
            diff: vec![
                ActiveNodeSetDiff::NewEpoch {
                    nodes: nodes.clone(),
                    apr: Ratio::new(1, 100),
                },
                ActiveNodeSetDiff::NewBlock {
                    leader,
                    failed_leaders,
                    voters,
                },
            ],
        };
        persistence
            .apply_update(update, Default::default())
            .await
            .unwrap();
        let node_set = persistence.active_node_set().await.unwrap().unwrap();
        assert_eq!(
            node_set.nodes,
            [
                ActiveNode {
                    address: nodes[0],
                    votes: 1,
                    proposals: 1,
                    slots: 2,
                },
                ActiveNode {
                    address: nodes[1],
                    votes: 1,
                    proposals: 0,
                    slots: 2,
                },
            ]
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_active_node_set_rewards() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let options = PersistenceOptions {
            path: db_path,
            max_connections: 5,
        };
        let mut persistence = Persistence::new(&options).await.unwrap();

        // Query for unknown account.
        assert_eq!(
            persistence
                .lifetime_rewards(Address::random())
                .await
                .unwrap(),
            ESPTokenAmount::from(0)
        );

        // Apply two updates: one with a fresh account, and then one with both a fresh account and
        // an existing account.
        let accounts = [Address::random(), Address::random()];
        let reward_updates = [
            vec![(accounts[0], ESPTokenAmount::from(1))],
            vec![
                (accounts[0], ESPTokenAmount::from(2)),
                (accounts[1], ESPTokenAmount::from(5)),
            ],
        ];
        let mut update = ActiveNodeSetUpdate {
            espresso_block: EpochAndBlock {
                epoch: 0,
                block: 0,
                timestamp: 0,
            },
            diff: vec![],
        };
        persistence
            .apply_update(update.clone(), reward_updates[0].clone())
            .await
            .unwrap();
        update.espresso_block.block += 1;
        persistence
            .apply_update(update, reward_updates[1].clone())
            .await
            .unwrap();
        assert_eq!(persistence.lifetime_rewards(accounts[0]).await.unwrap(), 3);
        assert_eq!(persistence.lifetime_rewards(accounts[1]).await.unwrap(), 5);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_active_node_set_idempotency() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let options = PersistenceOptions {
            path: db_path,
            max_connections: 5,
        };
        let mut persistence = Persistence::new(&options).await.unwrap();

        // Initialize with a set of nodes.
        let mut espresso_block = EpochAndBlock {
            block: 100,
            epoch: 200,
            timestamp: 300,
        };
        let nodes = vec![Address::random()];
        persistence
            .apply_update(
                ActiveNodeSetUpdate {
                    espresso_block,
                    diff: vec![ActiveNodeSetDiff::NewEpoch {
                        nodes: nodes.clone(),
                        apr: Ratio::new(1, 100),
                    }],
                },
                Default::default(),
            )
            .await
            .unwrap();

        // Apply the same update twice, check that the node stats are only incremented once.
        espresso_block.block += 1;
        let update = ActiveNodeSetUpdate {
            espresso_block,
            diff: vec![ActiveNodeSetDiff::NewBlock {
                leader: 0,
                failed_leaders: vec![0],
                voters: [true].into_iter().collect(),
            }],
        };
        persistence
            .apply_update(update.clone(), Default::default())
            .await
            .unwrap();
        persistence
            .apply_update(update, Default::default())
            .await
            .unwrap();

        let snapshot = persistence.active_node_set().await.unwrap().unwrap();
        assert_eq!(snapshot.espresso_block, espresso_block);
        assert_eq!(
            snapshot.nodes,
            [ActiveNode {
                address: nodes[0],
                votes: 1,
                proposals: 1,
                slots: 2
            }]
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_active_node_set_skipped_update() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let options = PersistenceOptions {
            path: db_path,
            max_connections: 5,
        };
        let mut persistence = Persistence::new(&options).await.unwrap();

        // Initialize with a set of nodes.
        let espresso_block = EpochAndBlock {
            block: 100,
            epoch: 200,
            timestamp: 300,
        };
        let nodes = vec![Address::random()];
        persistence
            .apply_update(
                ActiveNodeSetUpdate {
                    espresso_block,
                    diff: vec![ActiveNodeSetDiff::NewEpoch {
                        nodes: nodes.clone(),
                        apr: Ratio::new(1, 100),
                    }],
                },
                Default::default(),
            )
            .await
            .unwrap();

        // Apply an update that skips a block; check that node statistics are not affected.
        let update = ActiveNodeSetUpdate {
            espresso_block: EpochAndBlock {
                block: espresso_block.block + 2,
                ..espresso_block
            },
            diff: vec![ActiveNodeSetDiff::NewBlock {
                leader: 0,
                failed_leaders: vec![0],
                voters: [true].into_iter().collect(),
            }],
        };
        persistence
            .apply_update(update.clone(), Default::default())
            .await
            .unwrap_err();

        let snapshot = persistence.active_node_set().await.unwrap().unwrap();
        assert_eq!(snapshot.espresso_block, espresso_block);
        assert_eq!(snapshot.nodes, [ActiveNode::new(nodes[0])]);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_l1_idempotency() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let options = PersistenceOptions {
            path: db_path,
            max_connections: 5,
        };
        let mut persistence = Persistence::new(&options).await.unwrap();

        // Initialize with a couple nodes.
        persistence
            .save_genesis(Snapshot::empty(block_snapshot(0)))
            .await
            .unwrap();
        let nodes = [make_node(0), make_node(1)];
        persistence
            .apply_updates(vec![Update {
                block: block_snapshot(1),
                node_set_diffs: nodes
                    .iter()
                    .cloned()
                    .map(Arc::new)
                    .map(FullNodeSetDiff::NodeUpdate)
                    .collect(),
                wallet_diffs: Default::default(),
            }])
            .await
            .unwrap();

        // Create an update which is not inherently idempotent.
        //
        // The same node cannot exit twice in a row.
        let node_set_diff = FullNodeSetDiff::NodeExit(NodeExit {
            address: nodes[1].address,
            exit_time: 1_000_000,
        });
        // Delegating increases the stake given to a node each time, so is not idempotent.
        let wallet = Address::random();
        let delegation = Delegation {
            delegator: wallet,
            node: nodes[0].address,
            amount: ESPTokenAmount::ONE,
        };
        let wallet_diff = WalletDiff::DelegatedToNode(delegation);
        let update = Update {
            block: block_snapshot(2),
            node_set_diffs: vec![node_set_diff],
            wallet_diffs: [(wallet, vec![wallet_diff])].into_iter().collect(),
        };
        persistence
            .apply_updates(vec![update.clone()])
            .await
            .unwrap();

        // Apply the same event again, _plus a new one_. The first event, which was already applied,
        // should have no effect, but the new event should.
        let new_node = make_node(2);
        let new_delegation = Delegation {
            delegator: wallet,
            node: new_node.address,
            amount: ESPTokenAmount::ONE,
        };
        persistence
            .apply_updates(vec![
                update,
                Update {
                    block: block_snapshot(3),
                    node_set_diffs: vec![FullNodeSetDiff::NodeUpdate(Arc::new(new_node.clone()))],
                    wallet_diffs: [(wallet, vec![WalletDiff::DelegatedToNode(new_delegation)])]
                        .into_iter()
                        .collect(),
                },
            ])
            .await
            .unwrap();

        let snapshot = persistence
            .load_finalized_snapshot()
            .await
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.block, block_snapshot(3));
        assert_eq!(
            *snapshot.node_set,
            ordmap! {
                nodes[0].address => nodes[0].clone(),
                new_node.address => new_node.clone()
            }
        );
        assert_eq!(
            snapshot.wallets[&wallet].nodes,
            ordmap! {
                nodes[0].address => delegation,
                new_node.address => new_delegation
            }
        );
    }

    #[test_log::test(tokio::test)]
    async fn test_node_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let options = PersistenceOptions {
            path: db_path,
            max_connections: 5,
        };
        let mut persistence = Persistence::new(&options).await.unwrap();

        let content = NodeMetadataContent {
            pub_key: PubKey::generated_from_seed_indexed(Default::default(), 42).0,
            name: Some("test".into()),
            description: Some("longer description".into()),
            company_name: Some("Espresso Systems".into()),
            company_website: Some("https://www.espressosys.com/".parse().unwrap()),
            client_version: Some("release-main".into()),
            icon: Some(ImageSet {
                small: RatioSet {
                    ratio1: Some(
                        "https://www.espressosys.com/icon-14x14@1x.svg"
                            .parse()
                            .unwrap(),
                    ),
                    ratio2: Some(
                        "https://www.espressosys.com/icon-14x14@2x.svg"
                            .parse()
                            .unwrap(),
                    ),
                    ratio3: Some(
                        "https://www.espressosys.com/icon-14x14@3x.svg"
                            .parse()
                            .unwrap(),
                    ),
                },
                large: RatioSet {
                    ratio1: Some(
                        "https://www.espressosys.com/icon-24x24@1x.svg"
                            .parse()
                            .unwrap(),
                    ),
                    ratio2: Some(
                        "https://www.espressosys.com/icon-24x24@2x.svg"
                            .parse()
                            .unwrap(),
                    ),
                    ratio3: Some(
                        "https://www.espressosys.com/icon-24x24@3x.svg"
                            .parse()
                            .unwrap(),
                    ),
                },
            }),
        };
        let metadata = NodeMetadata {
            uri: "https://www.espressosys.com/metadata/".parse().unwrap(),
            content: Some(content),
        };

        // Initialize a node.
        persistence
            .save_genesis(Snapshot::empty(block_snapshot(0)))
            .await
            .unwrap();
        let node = NodeSetEntry {
            metadata: Some(metadata.clone()),
            ..make_node(0)
        };
        persistence
            .apply_updates(vec![Update {
                block: block_snapshot(1),
                node_set_diffs: vec![FullNodeSetDiff::NodeUpdate(Arc::new(node.clone()))],
                wallet_diffs: Default::default(),
            }])
            .await
            .unwrap();

        // Check that we get the same metadata when we read it back.
        let snapshot = persistence
            .load_finalized_snapshot()
            .await
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.node_set[&node.address].metadata, Some(metadata));
    }
}
