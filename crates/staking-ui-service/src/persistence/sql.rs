//! SQL-based persistent storage
use crate::{
    Result,
    input::l1::{L1BlockSnapshot, L1Persistence, NodeSet, Snapshot, Wallet, Wallets},
    types::{
        common::{Address, Delegation, L1BlockId, NodeSetEntry, PendingWithdrawal, Ratio},
        global::FullNodeSetDiff,
        wallet::WalletDiff,
    },
};
use alloy::primitives::U256;
use anyhow::Context;
use clap::Parser;
use futures::future::try_join_all;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::{path::PathBuf, str::FromStr};
use tracing::instrument;

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
        .create_if_missing(true);

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
        let node_rows = sqlx::query_as::<_, (String, String, String, f64, String)>(
            "SELECT address, staking_key, state_key, commission, stake FROM node",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut node_set = NodeSet::default();
        for (address, staking_key, state_key, commission, stake_str) in node_rows {
            let node = NodeSetEntry {
                address: address.parse().context("failed to parse node address")?,
                staking_key: staking_key.parse().context("failed to parse staking key")?,
                state_key: state_key.parse().context("failed to parse state key")?,
                stake: U256::from_str(&stake_str).context("failed to parse node stake")?,
                commission: Ratio::from(commission as f32),
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

                        let delegation_rows = sqlx::query_as::<_, (String, String, i64, String)>(
                            "SELECT node, amount, unlocks_at, withdrawal_amount
                     FROM delegation
                     WHERE delegator = $1
                     ORDER BY unlocks_at ASC, withdrawal_amount ASC",
                        )
                        .bind(&wallet_address)
                        .fetch_all(&pool)
                        .await?;

                        let mut nodes = im::Vector::new();
                        let mut pending_undelegations = im::Vector::new();
                        let mut pending_exits = im::Vector::new();

                        // We use unlocks_at and withdrawal_amount to determine the delegation status
                        for (node_str, amount_str, unlocks_at, withdrawal_amount_str) in
                            delegation_rows
                        {
                            let node: Address =
                                node_str.parse().context("failed to parse node address")?;
                            let amount = U256::from_str(&amount_str)
                                .context("failed to parse delegation amount")?;
                            let withdrawal_amount = U256::from_str(&withdrawal_amount_str)
                                .context("failed to parse withdrawal amount")?;
                            if withdrawal_amount.is_zero() && unlocks_at != 0 {
                                //  Node exit
                                pending_exits.push_back(PendingWithdrawal {
                                    delegator: address,
                                    node,
                                    amount,
                                    available_time: unlocks_at as u64,
                                });
                            } else if unlocks_at != 0 && !withdrawal_amount.is_zero() {
                                // Partial undelegation
                                if !amount.is_zero() {
                                    nodes.push_back(Delegation {
                                        delegator: address,
                                        node,
                                        amount,
                                    });
                                }
                                pending_undelegations.push_back(PendingWithdrawal {
                                    delegator: address,
                                    node,
                                    amount: withdrawal_amount,
                                    available_time: unlocks_at as u64,
                                });
                            } else if !amount.is_zero() {
                                nodes.push_back(Delegation {
                                    delegator: address,
                                    node,
                                    amount,
                                });
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
                sqlx::query(
                    "INSERT INTO node (address, staking_key, state_key, commission, stake)
                     VALUES ($1, $2, $3, $4, $5)
                     ON CONFLICT(address) DO UPDATE SET
                        staking_key = excluded.staking_key,
                        state_key = excluded.state_key,
                        commission = excluded.commission,
                        stake = excluded.stake",
                )
                .bind(node.address.to_string())
                .bind(node.staking_key.to_string())
                .bind(node.state_key.to_string())
                .bind(f32::from(node.commission) as f64)
                .bind(node.stake.to_string())
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
                    "INSERT INTO delegation (delegator, node, amount, unlocks_at, withdrawal_amount)
                     VALUES ($1, $2, $3, 0, '0')
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
                let (amount, unlocks_at, withdrawal_amount) = sqlx::query_as::<_, (String, i64, String)>(
                    "SELECT amount, unlocks_at, withdrawal_amount FROM delegation WHERE delegator = $1 AND node = $2",
                )
                .bind(withdrawal.delegator.to_string())
                .bind(withdrawal.node.to_string())
                .fetch_one(&mut **tx)
                .await?;

                // Sanity check: ensure there's no pending withdrawal already
                if unlocks_at != 0 || withdrawal_amount != "0" {
                    return Err(anyhow::anyhow!(
                        "pending withdrawal already exists for delegator {} to node {}",
                        withdrawal.delegator,
                        withdrawal.node
                    ))
                    .map_err(Into::into);
                }

                let current_amount = U256::from_str(&amount).unwrap_or(U256::ZERO);
                let new_amount = current_amount.checked_sub(withdrawal.amount).ok_or_else(
                    || {
                        anyhow::anyhow!(
                            "Underflow: withdrawal {} exceeds delegation amount {current_amount}",
                            withdrawal.amount
                        )
                    },
                )?;

                // Update delegation: decrement amount and set unlocks_at and withdrawal_amount
                let result = sqlx::query(
                    "UPDATE delegation
                     SET amount = $1,
                         unlocks_at = $2,
                         withdrawal_amount = $3
                     WHERE delegator = $4 AND node = $5",
                )
                .bind(new_amount.to_string())
                .bind(withdrawal.available_time as i64)
                .bind(withdrawal.amount.to_string())
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
            WalletDiff::NodeExited(withdrawal) => {
                // Mark all delegations to this node as exits
                let result = sqlx::query(
                    "UPDATE delegation
                     SET unlocks_at = $1, withdrawal_amount = '0'
                     WHERE delegator = $2 AND node = $3",
                )
                .bind(withdrawal.available_time as i64)
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
            WalletDiff::UndelegationWithdrawal(withdrawal) => {
                let (amount,) = sqlx::query_as::<_, (String,)>(
                    "UPDATE delegation
                     SET unlocks_at = 0,
                         withdrawal_amount = '0'
                     WHERE delegator = $1 AND node = $2
                     RETURNING amount",
                )
                .bind(withdrawal.delegator.to_string())
                .bind(withdrawal.node.to_string())
                .fetch_one(&mut **tx)
                .await?;

                let remaining_amount = U256::from_str(&amount)
                    .context("failed to parse remaining delegation amount")?;

                if remaining_amount.is_zero() {
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
                }
            }
            WalletDiff::NodeExitWithdrawal(withdrawal) => {
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
                        "Expected to delete 1 delegation row, but {} were affected",
                        result.rows_affected()
                    ))
                    .map_err(Into::into);
                }
            }
        }
        Ok(())
    }
}

impl L1Persistence for Persistence {
    async fn finalized_snapshot(&self) -> Result<Option<Snapshot>> {
        self.load_finalized_snapshot().await
    }

    async fn save_genesis(&self, snapshot: Snapshot) -> Result<()> {
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

    #[instrument(skip(self, node_set_diff, wallets_diff))]
    async fn apply_events(
        &self,
        block: L1BlockSnapshot,
        node_set_diff: impl IntoIterator<Item = FullNodeSetDiff> + Send,
        wallets_diff: impl IntoIterator<Item = (Address, WalletDiff)> + Send,
    ) -> Result<()> {
        tracing::debug!(block_number = block.number(), "applying events to database");

        // Collect iterators to ensure Send safety
        let node_set_diff: Vec<_> = node_set_diff.into_iter().collect();
        let wallets_diff: Vec<_> = wallets_diff.into_iter().collect();

        let mut tx = self.pool.begin().await?;

        // Update L1 block info
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

        // Apply node set diffs
        for diff in node_set_diff {
            self.apply_node_set_diff(&mut tx, &diff).await?;
        }

        // Apply wallet diffs
        for (address, diff) in wallets_diff {
            self.apply_wallet_diff(&mut tx, address, &diff).await?;
        }

        tx.commit().await?;
        tracing::debug!("events applied successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::l1::testing::{block_snapshot, make_node};
    use crate::types::common::{NodeExit, PendingWithdrawal, Withdrawal};
    use crate::types::global::FullNodeSetDiff;
    use tempfile::TempDir;

    /// Tests the complete persistence lifecycle
    #[tokio::test]
    async fn test_snapshot_save_apply_load() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let options = PersistenceOptions {
            path: db_path,
            max_connections: 5,
        };

        let persistence = Persistence::new(&options).await.unwrap();

        let node1 = make_node(1);
        let node2 = make_node(2);
        let node3 = make_node(3);
        let node4 = make_node(4);

        let delegator1 = Address::random();
        let delegator2 = Address::random();
        let delegator3 = Address::random();

        let genesis_snapshot = Snapshot::empty(block_snapshot(100));
        persistence
            .save_genesis(genesis_snapshot.clone())
            .await
            .unwrap();

        let snapshot = persistence
            .load_finalized_snapshot()
            .await
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.block.number(), 100);
        assert_eq!(snapshot.node_set.len(), 0);
        assert_eq!(snapshot.wallets.len(), 0);

        // Block 100: Register 4 nodes, set up initial delegations, delegator1 withdraws and claims rewards
        let initial_node_set_diffs = vec![
            FullNodeSetDiff::NodeUpdate(node1.clone()),
            FullNodeSetDiff::NodeUpdate(node2.clone()),
            FullNodeSetDiff::NodeUpdate(node3.clone()),
            FullNodeSetDiff::NodeUpdate(node4.clone()),
        ];

        let initial_wallet_diffs = vec![
            // Delegator1 delegates to node1
            (
                delegator1,
                WalletDiff::DelegatedToNode(Delegation {
                    delegator: delegator1,
                    node: node1.address,
                    amount: U256::from(6000000u64), // Initial amount before undelegation
                }),
            ),
            // Delegator1 undelegates from node1
            (
                delegator1,
                WalletDiff::UndelegatedFromNode(PendingWithdrawal {
                    delegator: delegator1,
                    node: node1.address,
                    amount: U256::from(1000000u64),
                    available_time: 800,
                }),
            ),
            // Delegator1 withdraws the undelegation and then claims rewards
            (
                delegator1,
                WalletDiff::UndelegationWithdrawal(Withdrawal {
                    delegator: delegator1,
                    node: node1.address,
                    amount: U256::from(1000000u64),
                }),
            ),
            // Delegator1 claims rewards after withdrawal
            (delegator1, WalletDiff::ClaimedRewards(500u64)),
            // Delegator1 delegates to node2
            (
                delegator1,
                WalletDiff::DelegatedToNode(Delegation {
                    delegator: delegator1,
                    node: node2.address,
                    amount: U256::from(3000000u64),
                }),
            ),
            // Delegator2 delegates to node1
            (
                delegator2,
                WalletDiff::DelegatedToNode(Delegation {
                    delegator: delegator2,
                    node: node1.address,
                    amount: U256::from(1000000u64),
                }),
            ),
            // Delegator2 undelegates from node1
            (
                delegator2,
                WalletDiff::UndelegatedFromNode(PendingWithdrawal {
                    delegator: delegator2,
                    node: node1.address,
                    amount: U256::from(500000u64),
                    available_time: 900,
                }),
            ),
            // Delegator2 delegates to node2
            (
                delegator2,
                WalletDiff::DelegatedToNode(Delegation {
                    delegator: delegator2,
                    node: node2.address,
                    amount: U256::from(10000000u64),
                }),
            ),
            // Delegator2 delegates to node3
            (
                delegator2,
                WalletDiff::DelegatedToNode(Delegation {
                    delegator: delegator2,
                    node: node3.address,
                    amount: U256::from(2000000u64),
                }),
            ),
            // Delegator3 delegates to node4
            (
                delegator3,
                WalletDiff::DelegatedToNode(Delegation {
                    delegator: delegator3,
                    node: node4.address,
                    amount: U256::from(8000000u64),
                }),
            ),
        ];

        persistence
            .apply_events(
                block_snapshot(100),
                initial_node_set_diffs,
                initial_wallet_diffs,
            )
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
        let node1_del = wallet1_block100
            .nodes
            .iter()
            .find(|d| d.node == node1.address)
            .unwrap();
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

        let node_exit_wallet_diffs = vec![(
            delegator3,
            WalletDiff::NodeExited(PendingWithdrawal {
                delegator: delegator3,
                node: node4.address,
                amount: U256::from(8000000u64),
                available_time: 1500,
            }),
        )];

        // Apply node4 exit in block 101
        persistence
            .apply_events(block_snapshot(101), node_exit_diffs, node_exit_wallet_diffs)
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
            .iter()
            .find(|d| d.node == node1.address)
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
            .iter()
            .find(|d| d.node == node1.address)
            .expect("should have remaining delegation to node1");
        assert_eq!(node1_delegation.amount, U256::from(500000u64));

        assert!(loaded_wallet2.nodes.iter().any(|d| d.node == node2.address));
        assert!(loaded_wallet2.nodes.iter().any(|d| d.node == node3.address));
        assert_eq!(loaded_wallet2.pending_undelegations.len(), 1);
        assert_eq!(
            loaded_wallet2.pending_undelegations[0].amount,
            U256::from(500000u64)
        );
        assert_eq!(loaded_wallet2.pending_exits.len(), 0);
        assert_eq!(loaded_wallet2.claimed_rewards, U256::ZERO);

        // Verify delegator3
        let loaded_wallet3 = snapshot.wallets.get(&delegator3).unwrap();
        assert_eq!(loaded_wallet3.nodes.len(), 0);
        assert_eq!(loaded_wallet3.pending_undelegations.len(), 0);
        assert_eq!(loaded_wallet3.pending_exits.len(), 1);
        assert_eq!(loaded_wallet3.pending_exits[0].node, node4.address);
        assert_eq!(
            loaded_wallet3.pending_exits[0].amount,
            U256::from(8000000u64)
        );
        assert_eq!(loaded_wallet3.pending_exits[0].available_time, 1500);
        assert_eq!(loaded_wallet3.claimed_rewards, U256::ZERO);

        // Block 102: Register node5, node2 exits, withdrawals and claim rewards
        let node5 = make_node(5);

        let node_set_diffs = vec![
            FullNodeSetDiff::NodeUpdate(node5.clone()),
            FullNodeSetDiff::NodeExit(NodeExit {
                address: node2.address,
                exit_time: 1200,
            }),
        ];

        let wallet_diffs = vec![
            // Delegator1 undelegates 1M from node1
            (
                delegator1,
                WalletDiff::UndelegatedFromNode(PendingWithdrawal {
                    delegator: delegator1,
                    node: node1.address,
                    amount: U256::from(1000000u64),
                    available_time: 1100,
                }),
            ),
            // Node2 exits
            // delegator1's delegation moves to pending_exits
            (
                delegator1,
                WalletDiff::NodeExited(PendingWithdrawal {
                    delegator: delegator1,
                    node: node2.address,
                    amount: U256::from(3000000u64),
                    available_time: 1200,
                }),
            ),
            // Node2 exits
            //  delegator2 delegation moves to pending_exits
            (
                delegator2,
                WalletDiff::NodeExited(PendingWithdrawal {
                    delegator: delegator2,
                    node: node2.address,
                    amount: U256::from(10000000u64),
                    available_time: 1200,
                }),
            ),
            // Delegator2 completes withdrawal of undelegation from block 100
            (
                delegator2,
                WalletDiff::UndelegationWithdrawal(Withdrawal {
                    delegator: delegator2,
                    node: node1.address,
                    amount: U256::from(500000u64),
                }),
            ),
            // Delegator3 withdrawal from exited node4
            (
                delegator3,
                WalletDiff::NodeExitWithdrawal(Withdrawal {
                    delegator: delegator3,
                    node: node4.address,
                    amount: U256::from(8000000u64),
                }),
            ),
            // Delegator1 claims  rewards
            (delegator1, WalletDiff::ClaimedRewards(1500u64)),
            // Delegator2 claims rewards after withdrawal
            (delegator2, WalletDiff::ClaimedRewards(750u64)),
            // Delegator3 delegates to new node5
            (
                delegator3,
                WalletDiff::DelegatedToNode(Delegation {
                    delegator: delegator3,
                    node: node5.address,
                    amount: U256::from(15000000u64),
                }),
            ),
        ];

        let updated_block = block_snapshot(102);

        persistence
            .apply_events(updated_block, node_set_diffs, wallet_diffs)
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
        assert_eq!(wallet1.nodes[0].node, node1.address);
        assert_eq!(wallet1.nodes[0].amount, U256::from(4000000u64));
        assert_eq!(wallet1.pending_undelegations.len(), 1);
        assert_eq!(
            wallet1.pending_undelegations[0].amount,
            U256::from(1000000u64)
        );
        assert_eq!(wallet1.pending_exits.len(), 1);
        assert_eq!(wallet1.pending_exits[0].node, node2.address);
        assert_eq!(wallet1.pending_exits[0].amount, U256::from(3000000u64));

        let wallet2 = loaded_snapshot
            .wallets
            .get(&delegator2)
            .expect("wallet2 should exist");
        assert_eq!(wallet2.claimed_rewards, U256::from(750u64)); // Claimed in block 102
        assert_eq!(wallet2.nodes.len(), 2); // node1  and node3 

        let node1_delegation = wallet2
            .nodes
            .iter()
            .find(|d| d.node == node1.address)
            .expect("should still have delegation to node1");
        assert_eq!(node1_delegation.amount, U256::from(500000u64));

        let node3_delegation = wallet2
            .nodes
            .iter()
            .find(|d| d.node == node3.address)
            .expect("should have delegation to node3");
        assert_eq!(node3_delegation.amount, U256::from(2000000u64));

        assert_eq!(wallet2.pending_undelegations.len(), 0);

        assert_eq!(wallet2.pending_exits.len(), 1);
        assert_eq!(wallet2.pending_exits[0].node, node2.address);
        assert_eq!(wallet2.pending_exits[0].amount, U256::from(10000000u64));

        let wallet3 = loaded_snapshot
            .wallets
            .get(&delegator3)
            .expect("wallet3 should exist");
        assert_eq!(wallet3.claimed_rewards, U256::ZERO);
        assert_eq!(wallet3.nodes.len(), 1); // Only node5 delegation remains
        let node5_delegation = wallet3
            .nodes
            .iter()
            .find(|d| d.node == node5.address)
            .expect("should have delegation to node5");
        assert_eq!(node5_delegation.amount, U256::from(15000000u64));
        assert_eq!(wallet3.pending_undelegations.len(), 0);
        // NodeExitWithdrawal completed, so no more pending_exits
        assert_eq!(wallet3.pending_exits.len(), 0);
    }
}
