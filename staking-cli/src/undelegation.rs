use std::collections::BTreeSet;

use alloy::{
    eips::BlockId,
    primitives::{Address, B256, U256},
    providers::Provider,
    rpc::types::Filter,
    sol_types::SolEvent,
};
use anyhow::{Context, Result};
use chrono::DateTime;
use espresso_safe_tx_builder::{CalldataInfo, output_safe_tx_builder_batch};
use hotshot_contract_adapter::sol_types::StakeTableV2::{self, Delegated};

use crate::{
    output::{format_esp, output_info, output_success},
    receipt::ReceiptExt as _,
    transaction::Transaction,
};

pub const DEFAULT_BLOCK_RANGE: u64 = 10_000;

const VALIDATOR_STATUS_EXITED: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimKind {
    Undelegation,
    ValidatorExit,
}

impl std::fmt::Display for ClaimKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClaimKind::Undelegation => write!(f, "undelegation"),
            ClaimKind::ValidatorExit => write!(f, "validator exit"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingClaim {
    pub kind: ClaimKind,
    pub validator: Address,
    pub amount: U256,
    pub unlocks_at: U256,
}

/// Scan on-chain events to discover all validators a delegator has interacted with,
/// then check for pending undelegations and validator exit claims.
pub async fn fetch_pending_claims(
    provider: &impl Provider,
    stake_table_address: Address,
    delegator: Address,
    block_range: u64,
) -> Result<Vec<PendingClaim>> {
    let stake_table = StakeTableV2::new(stake_table_address, provider);

    let initialized_at: u64 = stake_table
        .initializedAtBlock()
        .call()
        .await
        .context("Failed to get initializedAtBlock")?
        .to();

    let latest_block = provider
        .get_block_number()
        .await
        .context("Failed to get latest block number")?;

    output_info(format!(
        "Scanning blocks {initialized_at}..{latest_block} (chunk size {block_range})"
    ));

    // Scan Delegated events in chunks to find all validators this delegator interacted with.
    let mut validators = BTreeSet::new();
    let mut from = initialized_at;
    while from <= latest_block {
        let to = (from + block_range - 1).min(latest_block);
        let filter = Filter::new()
            .event_signature(Delegated::SIGNATURE_HASH)
            .address(stake_table_address)
            .topic1(B256::left_padding_from(delegator.as_slice()))
            .from_block(from)
            .to_block(to);

        let logs = provider
            .get_logs(&filter)
            .await
            .context("Failed to fetch Delegated logs")?;

        for log in &logs {
            if let Some(topic2) = log.topics().get(2) {
                validators.insert(Address::from_word(*topic2));
            }
        }

        from = to + 1;
    }

    // For each validator, check for pending undelegations and exit claims.
    let mut claims = Vec::new();

    for validator in &validators {
        let undelegation = stake_table
            .undelegations(*validator, delegator)
            .call()
            .await
            .context("Failed to fetch undelegation")?;

        if undelegation.amount > U256::ZERO {
            claims.push(PendingClaim {
                kind: ClaimKind::Undelegation,
                validator: *validator,
                amount: undelegation.amount,
                unlocks_at: undelegation.unlocksAt,
            });
        }

        let validator_info = stake_table
            .validators(*validator)
            .call()
            .await
            .context("Failed to fetch validator info")?;

        if validator_info.status == VALIDATOR_STATUS_EXITED {
            let delegation_amount = stake_table
                .delegations(*validator, delegator)
                .call()
                .await
                .context("Failed to fetch delegation amount")?;

            if delegation_amount > U256::ZERO {
                let unlocks_at = stake_table
                    .validatorExits(*validator)
                    .call()
                    .await
                    .context("Failed to fetch validator exit info")?;

                claims.push(PendingClaim {
                    kind: ClaimKind::ValidatorExit,
                    validator: *validator,
                    amount: delegation_amount,
                    unlocks_at,
                });
            }
        }
    }

    claims.sort_by_key(|c| c.unlocks_at);
    Ok(claims)
}

fn format_timestamp(ts: U256) -> String {
    let secs: i64 = ts.try_into().unwrap_or(i64::MAX);
    DateTime::from_timestamp(secs, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| ts.to_string())
}

/// Display pending claims to the user.
pub fn display_pending_claims(claims: &[PendingClaim], current_timestamp: u64) {
    if claims.is_empty() {
        output_success("No pending claims found");
        return;
    }

    let current_ts = U256::from(current_timestamp);
    for claim in claims {
        let status = if claim.unlocks_at <= current_ts {
            "Unlocked"
        } else {
            "Locked"
        };
        let unlock_time = format_timestamp(claim.unlocks_at);
        output_success(format!(
            "{} from {}: {}, unlocks at {unlock_time} ({status})",
            claim.kind,
            claim.validator,
            format_esp(claim.amount),
        ));
    }
}

fn build_claim_tx(stake_table_address: Address, claim: &PendingClaim) -> Transaction {
    match claim.kind {
        ClaimKind::Undelegation => Transaction::ClaimWithdrawal {
            stake_table: stake_table_address,
            validator: claim.validator,
        },
        ClaimKind::ValidatorExit => Transaction::ClaimValidatorExit {
            stake_table: stake_table_address,
            validator: claim.validator,
        },
    }
}

/// Fetch all pending claims and export calldata for unlocked ones
/// in Safe Transaction Builder batch JSON format.
pub async fn export_unlocked_claims(
    provider: &impl Provider,
    stake_table_address: Address,
    delegator: Address,
    output_path: Option<&std::path::Path>,
    block_range: u64,
) -> Result<()> {
    let unlocked = fetch_unlocked(provider, stake_table_address, delegator, block_range).await?;

    if unlocked.is_empty() {
        output_success("No unlocked claims to export");
        return Ok(());
    }

    let txs: Vec<CalldataInfo> = unlocked
        .iter()
        .map(|claim| {
            let (to, data, function_info) = build_claim_tx(stake_table_address, claim).calldata();
            let mut info = CalldataInfo::new(to, data);
            info.function_info = function_info;
            info
        })
        .collect();

    let chain_id = provider.get_chain_id().await?;
    let description = format!("Claim {} withdrawal(s) for {delegator}", unlocked.len());
    output_safe_tx_builder_batch(&txs, output_path, chain_id, &description)
}

/// Fetch all pending claims, display them, and claim any that are unlocked.
pub async fn claim_all_unlocked(
    provider: &impl Provider,
    stake_table_address: Address,
    delegator: Address,
    block_range: u64,
) -> Result<()> {
    let unlocked = fetch_unlocked(provider, stake_table_address, delegator, block_range).await?;

    if unlocked.is_empty() {
        output_success("No unlocked claims to process");
        return Ok(());
    }

    output_info(format!("Claiming {} withdrawal(s)", unlocked.len()));
    for claim in &unlocked {
        let tx = build_claim_tx(stake_table_address, claim);
        let receipt = tx.send(provider).await?.assert_success().await?;
        output_success(format!(
            "Claimed {} from {} for {} (tx {})",
            claim.kind,
            claim.validator,
            format_esp(claim.amount),
            receipt.transaction_hash,
        ));
    }

    Ok(())
}

async fn fetch_unlocked(
    provider: &impl Provider,
    stake_table_address: Address,
    delegator: Address,
    block_range: u64,
) -> Result<Vec<PendingClaim>> {
    let claims =
        fetch_pending_claims(provider, stake_table_address, delegator, block_range).await?;

    let block = provider
        .get_block(BlockId::latest())
        .await?
        .context("Failed to fetch latest block")?;

    let current_ts = U256::from(block.header.timestamp);
    Ok(claims
        .into_iter()
        .filter(|c| c.unlocks_at <= current_ts)
        .collect())
}

#[cfg(test)]
mod test {
    use alloy::{primitives::utils::parse_ether, providers::ext::AnvilApi};

    use super::*;
    use crate::deploy::TestSystem;

    #[tokio::test]
    async fn test_fetch_pending_empty() -> Result<()> {
        let system = TestSystem::deploy().await?;

        let claims = fetch_pending_claims(
            &system.provider,
            system.stake_table,
            system.deployer_address,
            DEFAULT_BLOCK_RANGE,
        )
        .await?;

        assert!(claims.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_pending_undelegation() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let amount = parse_ether("1")?;

        system.register_validator().await?;
        system.delegate(amount).await?;
        system.undelegate(amount).await?;

        let claims = fetch_pending_claims(
            &system.provider,
            system.stake_table,
            system.deployer_address,
            DEFAULT_BLOCK_RANGE,
        )
        .await?;

        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].kind, ClaimKind::Undelegation);
        assert_eq!(claims[0].validator, system.deployer_address);
        assert_eq!(claims[0].amount, amount);

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_pending_after_claim() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let amount = parse_ether("1")?;

        system.register_validator().await?;
        system.delegate(amount).await?;
        system.undelegate(amount).await?;
        system.warp_to_unlock_time().await?;

        Transaction::ClaimWithdrawal {
            stake_table: system.stake_table,
            validator: system.deployer_address,
        }
        .send(&system.provider)
        .await?
        .assert_success()
        .await?;

        let claims = fetch_pending_claims(
            &system.provider,
            system.stake_table,
            system.deployer_address,
            DEFAULT_BLOCK_RANGE,
        )
        .await?;

        assert!(claims.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_pending_validator_exit() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let amount = parse_ether("1")?;

        system.register_validator().await?;
        system.delegate(amount).await?;
        system.deregister_validator().await?;

        let claims = fetch_pending_claims(
            &system.provider,
            system.stake_table,
            system.deployer_address,
            DEFAULT_BLOCK_RANGE,
        )
        .await?;

        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].kind, ClaimKind::ValidatorExit);
        assert_eq!(claims[0].validator, system.deployer_address);
        assert_eq!(claims[0].amount, amount);

        Ok(())
    }

    #[tokio::test]
    async fn test_claim_all_unlocked() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let undelegation_amount = parse_ether("1")?;
        let exit_amount = parse_ether("2")?;
        let total = undelegation_amount + exit_amount;

        // Delegate 3 ETH, undelegate 1 ETH, then deregister (exit claim for remaining 2 ETH).
        system.register_validator().await?;
        system.delegate(total).await?;
        system.undelegate(undelegation_amount).await?;
        system.deregister_validator().await?;
        system.warp_to_unlock_time().await?;
        system.provider.anvil_mine(Some(1), None).await?;

        // Verify both claim types are detected.
        let claims = fetch_pending_claims(
            &system.provider,
            system.stake_table,
            system.deployer_address,
            DEFAULT_BLOCK_RANGE,
        )
        .await?;
        assert_eq!(claims.len(), 2);
        assert!(claims.iter().any(|c| c.kind == ClaimKind::Undelegation));
        assert!(claims.iter().any(|c| c.kind == ClaimKind::ValidatorExit));

        let balance_before = system.balance(system.deployer_address).await?;

        claim_all_unlocked(
            &system.provider,
            system.stake_table,
            system.deployer_address,
            DEFAULT_BLOCK_RANGE,
        )
        .await?;

        let balance_after = system.balance(system.deployer_address).await?;
        assert_eq!(balance_after, balance_before + total);

        Ok(())
    }
}
