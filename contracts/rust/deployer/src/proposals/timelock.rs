use alloy::{
    primitives::{Address, Bytes, B256, U256},
    providers::Provider,
    rpc::types::TransactionReceipt,
};
use anyhow::Result;
use clap::ValueEnum;
use hotshot_contract_adapter::sol_types::{
    EspToken, FeeContract, LightClient, OpsTimelock, RewardClaim, SafeExitTimelock, StakeTable,
};

use crate::{retry_until_true, Contract, Contracts, OwnableContract};

/// Data structure for timelock operations
#[derive(Debug, Clone)]
pub struct TimelockOperationData {
    /// The address of the contract to call
    pub target: Address,
    /// The value to send with the call
    pub value: U256,
    /// The data to send with the call e.g. the calldata of a function call
    pub data: Bytes,
    /// The predecessor operation id if you need to chain operations
    pub predecessor: B256,
    /// The salt for the operation
    pub salt: B256,
    /// The delay for the operation, must be >= the timelock's min delay
    pub delay: U256,
}

/// Types of timelock operations
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum TimelockOperationType {
    Schedule,
    Execute,
    Cancel,
}

/// Enum representing different types of timelock contracts
#[derive(Debug)]
pub enum TimelockContract {
    OpsTimelock(Address),
    SafeExitTimelock(Address),
}

impl TimelockContract {
    pub async fn get_operation_id(
        &self,
        operation: &TimelockOperationData,
        provider: &impl Provider,
    ) -> Result<B256> {
        match self {
            TimelockContract::OpsTimelock(timelock_addr) => {
                Ok(OpsTimelock::new(*timelock_addr, &provider)
                    .hashOperation(
                        operation.target,
                        operation.value,
                        operation.data.clone(),
                        operation.predecessor,
                        operation.salt,
                    )
                    .call()
                    .await?)
            },
            TimelockContract::SafeExitTimelock(timelock_addr) => {
                Ok(SafeExitTimelock::new(*timelock_addr, &provider)
                    .hashOperation(
                        operation.target,
                        operation.value,
                        operation.data.clone(),
                        operation.predecessor,
                        operation.salt,
                    )
                    .call()
                    .await?)
            },
        }
    }

    pub async fn schedule(
        &self,
        operation: TimelockOperationData,
        provider: &impl Provider,
    ) -> Result<TransactionReceipt> {
        match self {
            TimelockContract::OpsTimelock(timelock_addr) => {
                let pending_tx = OpsTimelock::new(*timelock_addr, &provider)
                    .schedule(
                        operation.target,
                        operation.value,
                        operation.data,
                        operation.predecessor,
                        operation.salt,
                        operation.delay,
                    )
                    .send()
                    .await?;
                let tx_hash = *pending_tx.tx_hash();
                tracing::info!(%tx_hash, "waiting for tx to be mined");
                let receipt = pending_tx.get_receipt().await?;
                if !receipt.inner.is_success() {
                    anyhow::bail!("tx failed: {:?}", receipt);
                }
                Ok(receipt)
            },
            TimelockContract::SafeExitTimelock(timelock_addr) => {
                let pending_tx = SafeExitTimelock::new(*timelock_addr, &provider)
                    .schedule(
                        operation.target,
                        operation.value,
                        operation.data,
                        operation.predecessor,
                        operation.salt,
                        operation.delay,
                    )
                    .send()
                    .await?;
                let tx_hash = *pending_tx.tx_hash();
                tracing::info!(%tx_hash, "waiting for tx to be mined");
                let receipt = pending_tx.get_receipt().await?;
                if !receipt.inner.is_success() {
                    anyhow::bail!("tx failed: {:?}", receipt);
                }
                Ok(receipt)
            },
        }
    }

    pub async fn is_operation_pending(
        &self,
        operation_id: B256,
        provider: &impl Provider,
    ) -> Result<bool> {
        match self {
            TimelockContract::OpsTimelock(timelock_addr) => {
                Ok(OpsTimelock::new(*timelock_addr, &provider)
                    .isOperationPending(operation_id)
                    .call()
                    .await?)
            },
            TimelockContract::SafeExitTimelock(timelock_addr) => {
                Ok(SafeExitTimelock::new(*timelock_addr, &provider)
                    .isOperationPending(operation_id)
                    .call()
                    .await?)
            },
        }
    }

    pub async fn is_operation_ready(
        &self,
        operation_id: B256,
        provider: &impl Provider,
    ) -> Result<bool> {
        match self {
            TimelockContract::OpsTimelock(timelock_addr) => {
                Ok(OpsTimelock::new(*timelock_addr, &provider)
                    .isOperationReady(operation_id)
                    .call()
                    .await?)
            },
            TimelockContract::SafeExitTimelock(timelock_addr) => {
                Ok(SafeExitTimelock::new(*timelock_addr, &provider)
                    .isOperationReady(operation_id)
                    .call()
                    .await?)
            },
        }
    }

    pub async fn is_operation_done(
        &self,
        operation_id: B256,
        provider: &impl Provider,
    ) -> Result<bool> {
        match self {
            TimelockContract::OpsTimelock(timelock_addr) => {
                Ok(OpsTimelock::new(*timelock_addr, &provider)
                    .isOperationDone(operation_id)
                    .call()
                    .await?)
            },
            TimelockContract::SafeExitTimelock(timelock_addr) => {
                Ok(SafeExitTimelock::new(*timelock_addr, &provider)
                    .isOperationDone(operation_id)
                    .call()
                    .await?)
            },
        }
    }

    pub async fn is_operation_canceled(
        &self,
        operation_id: B256,
        provider: &impl Provider,
    ) -> Result<bool> {
        let pending = self.is_operation_pending(operation_id, provider).await?;
        let done = self.is_operation_done(operation_id, provider).await?;
        // it's canceled if it's not pending and not done
        Ok(!pending && !done)
    }

    pub async fn execute(
        &self,
        operation: TimelockOperationData,
        provider: &impl Provider,
    ) -> Result<TransactionReceipt> {
        match self {
            TimelockContract::OpsTimelock(timelock_addr) => {
                let pending_tx = OpsTimelock::new(*timelock_addr, &provider)
                    .execute(
                        operation.target,
                        operation.value,
                        operation.data,
                        operation.predecessor,
                        operation.salt,
                    )
                    .send()
                    .await?;
                let tx_hash = *pending_tx.tx_hash();
                tracing::info!(%tx_hash, "waiting for tx to be mined");
                let receipt = pending_tx.get_receipt().await?;
                if !receipt.inner.is_success() {
                    anyhow::bail!("tx failed: {:?}", receipt);
                }
                Ok(receipt)
            },
            TimelockContract::SafeExitTimelock(timelock_addr) => {
                let pending_tx = SafeExitTimelock::new(*timelock_addr, &provider)
                    .execute(
                        operation.target,
                        operation.value,
                        operation.data,
                        operation.predecessor,
                        operation.salt,
                    )
                    .send()
                    .await?;
                let tx_hash = *pending_tx.tx_hash();
                tracing::info!(%tx_hash, "waiting for tx to be mined");
                let receipt = pending_tx.get_receipt().await?;
                if !receipt.inner.is_success() {
                    anyhow::bail!("tx failed: {:?}", receipt);
                }
                Ok(receipt)
            },
        }
    }

    pub async fn cancel(
        &self,
        operation_id: B256,
        provider: &impl Provider,
    ) -> Result<TransactionReceipt> {
        match self {
            TimelockContract::OpsTimelock(timelock_addr) => {
                let pending_tx = OpsTimelock::new(*timelock_addr, &provider)
                    .cancel(operation_id)
                    .send()
                    .await?;
                let tx_hash = *pending_tx.tx_hash();
                tracing::info!(%tx_hash, "waiting for tx to be mined");
                let receipt = pending_tx.get_receipt().await?;
                if !receipt.inner.is_success() {
                    anyhow::bail!("tx failed: {:?}", receipt);
                }
                Ok(receipt)
            },
            TimelockContract::SafeExitTimelock(timelock_addr) => {
                let pending_tx = SafeExitTimelock::new(*timelock_addr, &provider)
                    .cancel(operation_id)
                    .send()
                    .await?;
                let tx_hash = *pending_tx.tx_hash();
                tracing::info!(%tx_hash, "waiting for tx to be mined");
                let receipt = pending_tx.get_receipt().await?;
                if !receipt.inner.is_success() {
                    anyhow::bail!("tx failed: {:?}", receipt);
                }
                Ok(receipt)
            },
        }
    }
}

// Derive timelock address from contract type
// FeeContract, LightClient, StakeTable => OpsTimelock
// EspToken, RewardClaim => SafeExitTimelock
pub fn derive_timelock_address_from_contract_type(
    contract_type: OwnableContract,
    contracts: &Contracts,
) -> Result<Address> {
    let timelock_type = match contract_type {
        OwnableContract::FeeContractProxy
        | OwnableContract::LightClientProxy
        | OwnableContract::StakeTableProxy => Contract::OpsTimelock,
        OwnableContract::EspTokenProxy | OwnableContract::RewardClaimProxy => {
            Contract::SafeExitTimelock
        },
    };

    contracts.address(timelock_type).ok_or_else(|| {
        anyhow::anyhow!(
            "{:?} not found in deployed contracts. Deploy it first or provide it via flag.",
            timelock_type
        )
    })
}

// Get the timelock for a contract by querying the contract owner or current admin
pub async fn get_timelock_for_contract(
    provider: &impl Provider,
    contract_type: Contract,
    target_addr: Address,
) -> Result<TimelockContract> {
    match contract_type {
        Contract::FeeContractProxy => Ok(TimelockContract::OpsTimelock(
            FeeContract::new(target_addr, &provider)
                .owner()
                .call()
                .await?,
        )),
        Contract::EspTokenProxy => Ok(TimelockContract::SafeExitTimelock(
            EspToken::new(target_addr, &provider).owner().call().await?,
        )),
        Contract::LightClientProxy => Ok(TimelockContract::OpsTimelock(
            LightClient::new(target_addr, &provider)
                .owner()
                .call()
                .await?,
        )),
        Contract::StakeTableProxy => Ok(TimelockContract::OpsTimelock(
            StakeTable::new(target_addr, &provider)
                .owner()
                .call()
                .await?,
        )),
        Contract::RewardClaimProxy => Ok(TimelockContract::SafeExitTimelock(
            RewardClaim::new(target_addr, &provider)
                .currentAdmin()
                .call()
                .await?,
        )),
        _ => anyhow::bail!(
            "Invalid contract type for timelock get operation: {}",
            contract_type
        ),
    }
}

/// Schedule a timelock operation
///
/// Parameters:
/// - `provider`: the provider to use
/// - `contract_type`: the type of contract to schedule the operation on
/// - `operation`: the operation to schedule (see TimelockOperationData struct for more details)
///
/// Returns:
/// - The operation id
pub async fn schedule_timelock_operation(
    provider: &impl Provider,
    contract_type: Contract,
    operation: TimelockOperationData,
) -> Result<B256> {
    let target_addr = operation.target;
    let timelock = get_timelock_for_contract(provider, contract_type, target_addr).await?;
    let operation_id = timelock.get_operation_id(&operation, &provider).await?;

    let receipt = timelock.schedule(operation, &provider).await?;
    tracing::info!(%receipt.gas_used, %receipt.transaction_hash, "tx mined");
    if !receipt.inner.is_success() {
        anyhow::bail!("tx failed: {:?}", receipt);
    }

    // check that the tx is scheduled (with retry for RPC timing)
    let check_name = format!("Schedule operation {}", operation_id);
    let is_scheduled = retry_until_true(&check_name, || async {
        Ok(timelock
            .is_operation_pending(operation_id, &provider)
            .await?
            || timelock.is_operation_ready(operation_id, &provider).await?)
    })
    .await?;

    if !is_scheduled {
        anyhow::bail!("tx not correctly scheduled: {}", operation_id);
    }
    tracing::info!("tx scheduled with id: {}", operation_id);
    Ok(operation_id)
}

/// Execute a timelock operation
///
/// Parameters:
/// - `provider`: the provider to use
/// - `contract_type`: the type of contract to execute the operation on
/// - `operation`: the operation to execute (see TimelockOperationData struct for more details)
///
/// Returns:
/// - The operation id
pub async fn execute_timelock_operation(
    provider: &impl Provider,
    contract_type: Contract,
    operation: TimelockOperationData,
) -> Result<B256> {
    let target_addr = operation.target;
    let timelock = get_timelock_for_contract(provider, contract_type, target_addr).await?;
    let operation_id = timelock.get_operation_id(&operation, &provider).await?;

    // execute the tx
    let receipt = timelock.execute(operation, &provider).await?;
    tracing::info!(%receipt.gas_used, %receipt.transaction_hash, "tx mined");
    if !receipt.inner.is_success() {
        anyhow::bail!("tx failed: {:?}", receipt);
    }

    // check that the tx is executed (with retry for RPC timing)
    let check_name = format!("Execute operation {}", operation_id);
    let is_done = retry_until_true(&check_name, || async {
        Ok(timelock.is_operation_done(operation_id, &provider).await?)
    })
    .await?;

    if !is_done {
        anyhow::bail!("tx not correctly executed: {}", operation_id);
    }
    tracing::info!("tx executed with id: {}", operation_id);
    Ok(operation_id)
}

/// Cancel a timelock operation
///
/// Parameters:
/// - `provider`: the provider to use
/// - `contract_type`: the type of contract to cancel the operation on
/// - `operation`: the operation to cancel (see TimelockOperationData struct for more details)
///
/// Returns:
/// - The operation id
pub async fn cancel_timelock_operation(
    provider: &impl Provider,
    contract_type: Contract,
    operation: TimelockOperationData,
) -> Result<B256> {
    let target_addr = operation.target;
    let timelock = get_timelock_for_contract(provider, contract_type, target_addr).await?;
    let operation_id = timelock.get_operation_id(&operation, &provider).await?;
    let receipt = timelock.cancel(operation_id, &provider).await?;
    tracing::info!(%receipt.gas_used, %receipt.transaction_hash, "tx mined");
    if !receipt.inner.is_success() {
        anyhow::bail!("tx failed: {:?}", receipt);
    }

    // check that the tx is cancelled (with retry for RPC timing)
    let check_name = format!("Cancel operation {}", operation_id);
    let is_cancelled = retry_until_true(&check_name, || async {
        Ok(timelock
            .is_operation_canceled(operation_id, &provider)
            .await?)
    })
    .await?;

    if !is_cancelled {
        anyhow::bail!("tx not correctly cancelled: {}", operation_id);
    }

    tracing::info!("tx cancelled with id: {}", operation_id);
    Ok(operation_id)
}
