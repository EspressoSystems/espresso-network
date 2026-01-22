use alloy::{
    primitives::{Address, Bytes, B256, U256},
    providers::Provider,
    rpc::types::TransactionReceipt,
};
use anyhow::Result;
use clap::ValueEnum;
use derive_more::Display;
use hotshot_contract_adapter::sol_types::{
    EspToken, FeeContract, LightClient, OpsTimelock, RewardClaim, SafeExitTimelock, StakeTable,
};

use crate::{
    proposals::multisig::call_propose_transaction_generic_script, Contract, Contracts,
    OwnableContract,
};

/// Data structure for timelock operations payload
#[derive(Debug, Clone)]
pub struct TimelockOperationPayload {
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

/// Parameters for executing timelock operations (how to route/execute)
#[derive(Debug, Clone, Default)]
pub struct TimelockOperationParams {
    /// Optional multisig proposer address. If provided, operation will be routed through Safe proposal.
    pub multisig_proposer: Option<Address>,
    /// RPC URL (required if using multisig)
    pub rpc_url: Option<String>,
    /// Whether to use hardware wallet for signing
    pub use_hardware_wallet: bool,
    /// Optional operation ID (for cancel operations when you already have the ID)
    pub operation_id: Option<B256>,
    /// Whether to perform a dry run (for testing, no proposal is created)
    pub dry_run: bool,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, ValueEnum)]
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
        operation: &TimelockOperationPayload,
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
        operation: TimelockOperationPayload,
        provider: &impl Provider,
    ) -> Result<TransactionReceipt> {
        self.call_timelock_method(TimelockOperationType::Schedule, operation, None, provider)
            .await
    }

    pub async fn execute(
        &self,
        operation: TimelockOperationPayload,
        provider: &impl Provider,
    ) -> Result<TransactionReceipt> {
        self.call_timelock_method(TimelockOperationType::Execute, operation, None, provider)
            .await
    }

    pub async fn cancel(
        &self,
        operation_id: B256,
        provider: &impl Provider,
    ) -> Result<TransactionReceipt> {
        self.call_timelock_method(
            TimelockOperationType::Cancel,
            TimelockOperationPayload {
                target: Address::ZERO,
                value: U256::ZERO,
                data: Bytes::new(),
                predecessor: B256::ZERO,
                salt: B256::ZERO,
                delay: U256::ZERO,
            },
            Some(operation_id),
            provider,
        )
        .await
    }

    /// Internal helper to reduce duplication in schedule/execute/cancel
    async fn call_timelock_method(
        &self,
        method: TimelockOperationType,
        operation: TimelockOperationPayload,
        operation_id: Option<B256>,
        provider: &impl Provider,
    ) -> Result<TransactionReceipt> {
        let pending_tx = match (self, method) {
            (TimelockContract::OpsTimelock(addr), TimelockOperationType::Schedule) => {
                OpsTimelock::new(*addr, &provider)
                    .schedule(
                        operation.target,
                        operation.value,
                        operation.data,
                        operation.predecessor,
                        operation.salt,
                        operation.delay,
                    )
                    .send()
                    .await?
            },
            (TimelockContract::SafeExitTimelock(addr), TimelockOperationType::Schedule) => {
                SafeExitTimelock::new(*addr, &provider)
                    .schedule(
                        operation.target,
                        operation.value,
                        operation.data,
                        operation.predecessor,
                        operation.salt,
                        operation.delay,
                    )
                    .send()
                    .await?
            },
            (TimelockContract::OpsTimelock(addr), TimelockOperationType::Execute) => {
                OpsTimelock::new(*addr, &provider)
                    .execute(
                        operation.target,
                        operation.value,
                        operation.data,
                        operation.predecessor,
                        operation.salt,
                    )
                    .send()
                    .await?
            },
            (TimelockContract::SafeExitTimelock(addr), TimelockOperationType::Execute) => {
                SafeExitTimelock::new(*addr, &provider)
                    .execute(
                        operation.target,
                        operation.value,
                        operation.data,
                        operation.predecessor,
                        operation.salt,
                    )
                    .send()
                    .await?
            },
            (TimelockContract::OpsTimelock(addr), TimelockOperationType::Cancel) => {
                OpsTimelock::new(*addr, &provider)
                    .cancel(
                        operation_id
                            .ok_or_else(|| anyhow::anyhow!("operation_id required for cancel"))?,
                    )
                    .send()
                    .await?
            },
            (TimelockContract::SafeExitTimelock(addr), TimelockOperationType::Cancel) => {
                SafeExitTimelock::new(*addr, &provider)
                    .cancel(
                        operation_id
                            .ok_or_else(|| anyhow::anyhow!("operation_id required for cancel"))?,
                    )
                    .send()
                    .await?
            },
        };

        let tx_hash = *pending_tx.tx_hash();
        tracing::info!(%tx_hash, "waiting for tx to be mined");
        let receipt = pending_tx.get_receipt().await?;
        Ok(receipt)
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

/// Unified function to perform timelock operations (schedule, execute, cancel)
/// Routes to EOA or multisig based on params
pub async fn perform_timelock_operation(
    provider: &impl Provider,
    contract_type: Contract,
    operation: TimelockOperationPayload,
    operation_type: TimelockOperationType,
    params: TimelockOperationParams,
) -> Result<B256> {
    let timelock = get_timelock_for_contract(provider, contract_type, operation.target).await?;
    let operation_id =
        if let (TimelockOperationType::Cancel, Some(id)) = (operation_type, params.operation_id) {
            id
        } else {
            timelock.get_operation_id(&operation, &provider).await?
        };

    if let Some(multisig_proposer) = params.multisig_proposer {
        // Multisig path
        let rpc_url = params
            .rpc_url
            .ok_or_else(|| anyhow::anyhow!("RPC URL is required when using multisig proposer"))?;

        perform_timelock_operation_via_multisig(
            timelock,
            operation,
            operation_type,
            operation_id,
            rpc_url,
            multisig_proposer,
            params.use_hardware_wallet,
            params.dry_run,
        )
        .await
    } else {
        // EOA path
        perform_timelock_operation_via_eoa(
            timelock,
            operation,
            operation_type,
            operation_id,
            provider,
        )
        .await
    }
}

/// Perform timelock operation via EOA (direct transaction)
async fn perform_timelock_operation_via_eoa(
    timelock: TimelockContract,
    operation: TimelockOperationPayload,
    operation_type: TimelockOperationType,
    operation_id: B256,
    provider: &impl Provider,
) -> Result<B256> {
    let receipt = match operation_type {
        TimelockOperationType::Schedule => timelock.schedule(operation, &provider).await?,
        TimelockOperationType::Execute => timelock.execute(operation, &provider).await?,
        TimelockOperationType::Cancel => timelock.cancel(operation_id, &provider).await?,
    };

    tracing::info!(%receipt.gas_used, %receipt.transaction_hash, "tx mined");
    if !receipt.inner.is_success() {
        anyhow::bail!("tx failed: {:?}", receipt);
    }
    // Verify operation state based on type
    match operation_type {
        TimelockOperationType::Schedule => {
            // Check that the tx is scheduled
            if !(timelock
                .is_operation_pending(operation_id, &provider)
                .await?
                || timelock.is_operation_ready(operation_id, &provider).await?)
            {
                anyhow::bail!("tx not correctly scheduled: {}", operation_id);
            }
            tracing::info!("tx scheduled with id: {}", operation_id);
        },
        TimelockOperationType::Execute => {
            // Check that the tx is executed
            if !timelock.is_operation_done(operation_id, &provider).await? {
                anyhow::bail!("tx not correctly executed: {}", operation_id);
            }
            tracing::info!("tx executed with id: {}", operation_id);
        },
        TimelockOperationType::Cancel => {
            tracing::info!("tx cancelled with id: {}", operation_id);
        },
    }

    Ok(operation_id)
}

/// Perform timelock operation via Safe multisig proposal
async fn perform_timelock_operation_via_multisig(
    timelock: TimelockContract,
    operation: TimelockOperationPayload,
    operation_type: TimelockOperationType,
    operation_id: B256,
    rpc_url: String,
    multisig_proposer: Address,
    use_hardware_wallet: bool,
    dry_run: bool,
) -> Result<B256> {
    // Get timelock address
    let timelock_addr = match timelock {
        TimelockContract::OpsTimelock(addr) => addr,
        TimelockContract::SafeExitTimelock(addr) => addr,
    };

    // Determine function signature and arguments based on operation type
    let (function_signature, function_args) = match operation_type {
        TimelockOperationType::Schedule => {
            (
                "schedule(address,uint256,bytes,bytes32,bytes32,uint256)".to_string(),
                vec![
                    operation.target.to_string(),
                    operation.value.to_string(),
                    operation.data.to_string(), // Bytes implements Display with 0x prefix
                    operation.predecessor.to_string(), // B256 implements Display with 0x prefix
                    operation.salt.to_string(), // B256 implements Display with 0x prefix
                    operation.delay.to_string(),
                ],
            )
        },
        TimelockOperationType::Execute => (
            "execute(address,uint256,bytes,bytes32,bytes32)".to_string(),
            vec![
                operation.target.to_string(),
                operation.value.to_string(),
                operation.data.to_string(),
                operation.predecessor.to_string(),
                operation.salt.to_string(),
            ],
        ),
        TimelockOperationType::Cancel => {
            (
                "cancel(bytes32)".to_string(),
                vec![operation_id.to_string()], // B256 implements Display with 0x prefix
            )
        },
    };

    tracing::info!(
        "Calling proposeTransactionGeneric.ts for {:?} operation on timelock {}",
        operation_type,
        timelock_addr
    );

    call_propose_transaction_generic_script(
        timelock_addr,
        function_signature,
        function_args,
        rpc_url,
        multisig_proposer,
        use_hardware_wallet,
        Some(operation.value.to_string()),
        dry_run,
    )
    .await?;

    tracing::info!(
        "Timelock {:?} operation multisig proposal created successfully. Operation ID: {}",
        operation_type,
        operation_id
    );
    tracing::info!(
        "Send this link to the signers to sign the proposal: https://app.safe.global/transactions/queue?safe={}",
        multisig_proposer
    );

    Ok(operation_id)
}
