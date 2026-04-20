use alloy::{
    primitives::{Address, B256, Bytes, U256},
    providers::Provider,
    rpc::types::TransactionReceipt,
};
use anyhow::{Result, anyhow};
use clap::ValueEnum;
use hotshot_contract_adapter::sol_types::{
    EspToken, FeeContract, LightClient, OpsTimelock, RewardClaim, SafeExitTimelock, StakeTable,
    StakeTableV3,
};

use crate::{
    Contract, Contracts, OwnableContract,
    output::CalldataInfo,
    proposals::multisig::{encode_generic_calldata, encode_upgrade_calldata},
    retry_until_true,
};

/// Data structure for timelock operations payload
#[derive(Debug, Default, Clone)]
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
    /// Optional operation ID (for cancel operations when you already have the ID)
    pub operation_id: Option<B256>,
    /// Whether to perform a dry run (for testing, no proposal is created)
    pub dry_run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
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
        // the timelock contract only requires the operation_id to cancel an operation
        let placeholder_operation = TimelockOperationPayload::default();
        self.call_timelock_method(
            TimelockOperationType::Cancel,
            placeholder_operation,
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
    // for cancel operations: if operation_id is provided, use it directly;
    // otherwise, compute it from the operation payload
    let operation_id =
        if let (TimelockOperationType::Cancel, Some(id)) = (operation_type, params.operation_id) {
            id
        } else {
            timelock.get_operation_id(&operation, &provider).await?
        };

    if let Some(multisig_proposer) = params.multisig_proposer {
        perform_timelock_operation_via_multisig(
            timelock,
            operation,
            operation_type,
            operation_id,
            multisig_proposer,
        )
        .await
    } else {
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

    // Verify operation state based on type (with retry for RPC timing)
    match operation_type {
        TimelockOperationType::Schedule => {
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
        },
        TimelockOperationType::Execute => {
            let check_name = format!("Execute operation {}", operation_id);
            let is_done = retry_until_true(&check_name, || async {
                timelock.is_operation_done(operation_id, &provider).await
            })
            .await?;
            if !is_done {
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
    multisig_proposer: Address,
) -> Result<B256> {
    let timelock_addr = match timelock {
        TimelockContract::OpsTimelock(addr) => addr,
        TimelockContract::SafeExitTimelock(addr) => addr,
    };

    // Determine function signature and arguments based on operation type
    let (function_signature, function_args) = match operation_type {
        TimelockOperationType::Schedule => (
            "schedule(address,uint256,bytes,bytes32,bytes32,uint256)",
            vec![
                operation.target.to_string(),
                operation.value.to_string(),
                operation.data.to_string(),
                operation.predecessor.to_string(),
                operation.salt.to_string(),
                operation.delay.to_string(),
            ],
        ),
        TimelockOperationType::Execute => (
            "execute(address,uint256,bytes,bytes32,bytes32)",
            vec![
                operation.target.to_string(),
                operation.value.to_string(),
                operation.data.to_string(),
                operation.predecessor.to_string(),
                operation.salt.to_string(),
            ],
        ),
        TimelockOperationType::Cancel => ("cancel(bytes32)", vec![operation_id.to_string()]),
    };

    tracing::info!(
        "Encoding {:?} operation calldata for timelock {}",
        operation_type,
        timelock_addr
    );

    let calldata =
        encode_generic_calldata(timelock_addr, function_signature, function_args, U256::ZERO)?;

    tracing::info!(
        "Timelock {:?} operation calldata encoded. Operation ID: {}",
        operation_type,
        operation_id
    );
    tracing::info!(
        "Multisig proposer: {}. To: {}, Data: {}",
        multisig_proposer,
        calldata.to,
        calldata.data
    );

    Ok(operation_id)
}

/// Parameters for proposing a StakeTable V3 upgrade through a timelock owner.
#[derive(Clone, Debug)]
pub struct StakeTableV3TimelockProposalParams {
    /// Salt for the timelock `schedule`/`execute` operation.
    pub salt: B256,
    /// Delay for the timelock `schedule` operation (must be >= the timelock min delay).
    pub delay: U256,
}

/// Encoded timelock transactions for a StakeTable V3 upgrade.
///
/// `schedule` is submitted first (by a timelock proposer), then after the delay
/// elapses `execute` is submitted (by a timelock executor).
pub struct StakeTableV3TimelockProposal {
    pub schedule: CalldataInfo,
    pub execute: CalldataInfo,
    /// Address of the freshly deployed StakeTableV3 implementation.
    pub v3_impl_addr: Address,
    /// Timelock address that must submit both txs.
    pub timelock_addr: Address,
}

/// Encode timelock `schedule` + `execute` calldata wrapping a StakeTable V3 upgrade.
///
/// The inner payload is `proxy.upgradeToAndCall(v3_impl, init_data)` where
/// `init_data` is `initializeV3()` calldata (or empty if the proxy is already at V3).
///
/// This is a pure encoding helper: it does not deploy contracts or make RPC calls,
/// which keeps it unit-testable without an Anvil instance.
pub fn encode_stake_table_v3_timelock_proposal(
    proxy_addr: Address,
    v3_impl_addr: Address,
    timelock_addr: Address,
    init_data: Bytes,
    params: &StakeTableV3TimelockProposalParams,
) -> Result<StakeTableV3TimelockProposal> {
    // Inner call: proxy.upgradeToAndCall(v3_impl, init_data).
    let upgrade_calldata = encode_upgrade_calldata(proxy_addr, v3_impl_addr, init_data)?;

    let schedule = encode_generic_calldata(
        timelock_addr,
        "schedule(address,uint256,bytes,bytes32,bytes32,uint256)",
        vec![
            proxy_addr.to_string(),
            U256::ZERO.to_string(),
            upgrade_calldata.data.to_string(),
            B256::ZERO.to_string(),
            params.salt.to_string(),
            params.delay.to_string(),
        ],
        U256::ZERO,
    )?
    .with_description(format!(
        "Schedule StakeTable -> V3 upgrade via timelock {timelock_addr:#x} (proxy \
         {proxy_addr:#x}, impl {v3_impl_addr:#x})"
    ));

    let execute = encode_generic_calldata(
        timelock_addr,
        "execute(address,uint256,bytes,bytes32,bytes32)",
        vec![
            proxy_addr.to_string(),
            U256::ZERO.to_string(),
            upgrade_calldata.data.to_string(),
            B256::ZERO.to_string(),
            params.salt.to_string(),
        ],
        U256::ZERO,
    )?
    .with_description(format!(
        "Execute StakeTable -> V3 upgrade via timelock {timelock_addr:#x} (proxy {proxy_addr:#x}, \
         impl {v3_impl_addr:#x})"
    ));

    Ok(StakeTableV3TimelockProposal {
        schedule,
        execute,
        v3_impl_addr,
        timelock_addr,
    })
}

/// Upgrade the stake table proxy to StakeTableV3 through a timelock owner.
///
/// Deploys the V3 implementation, then encodes `schedule(...)` and `execute(...)`
/// timelock calldata. The inner payload is `upgradeToAndCall(v3_impl, initializeV3())`
/// targeting the stake table proxy. Mirrors `upgrade_stake_table_v3_multisig_owner`
/// but routes through the timelock instead of a multisig.
pub async fn upgrade_stake_table_v3_timelock_proposal(
    provider: impl Provider,
    contracts: &mut Contracts,
    params: StakeTableV3TimelockProposalParams,
) -> Result<StakeTableV3TimelockProposal> {
    let expected_major_version: u8 = 3;

    tracing::info!("Encoding StakeTableProxy -> StakeTableV3 upgrade via timelock owner");
    let proxy_addr = contracts
        .address(Contract::StakeTableProxy)
        .ok_or_else(|| anyhow!("StakeTableProxy not found, can't upgrade"))?;

    let proxy = StakeTableV3::new(proxy_addr, &provider);

    // The proxy owner must be the OpsTimelock for this flow.
    let owner_addr = proxy.owner().call().await?;
    let timelock_addr =
        derive_timelock_address_from_contract_type(OwnableContract::StakeTableProxy, contracts)?;
    if owner_addr != timelock_addr {
        anyhow::bail!(
            "StakeTableProxy owner {owner_addr:#x} is not the OpsTimelock {timelock_addr:#x}"
        );
    }

    // V3 requires V2 as a prerequisite.
    let version = proxy.getVersion().call().await?;
    if version.majorVersion < 2 {
        anyhow::bail!(
            "StakeTableProxy must be at major version >= 2 to upgrade to V3, found {}",
            version.majorVersion
        );
    }

    let v3_impl_addr = contracts
        .deploy(
            Contract::StakeTableV3,
            StakeTableV3::deploy_builder(&provider),
        )
        .await?;

    // If already at V3, skip initializeV3() to avoid the "already initialized" revert.
    let init_data =
        if crate::already_initialized(&provider, proxy_addr, expected_major_version).await? {
            tracing::info!(
                "StakeTableProxy already initialized at V{expected_major_version}, skipping \
                 initializeV3()"
            );
            Bytes::new()
        } else {
            StakeTableV3::new(Address::ZERO, &provider)
                .initializeV3()
                .calldata()
                .to_owned()
        };

    encode_stake_table_v3_timelock_proposal(
        proxy_addr,
        v3_impl_addr,
        timelock_addr,
        init_data,
        &params,
    )
}

#[cfg(test)]
mod tests {
    use alloy::{
        primitives::{Address, U256},
        sol_types::SolCall,
    };
    use hotshot_contract_adapter::sol_types::OpsTimelock;

    use super::*;

    /// Verify that `encode_stake_table_v3_timelock_proposal` produces non-empty
    /// `schedule` + `execute` calldata targeting the timelock, with the inner
    /// payload matching `proxy.upgradeToAndCall(v3_impl, initializeV3())`.
    #[test]
    fn test_encode_stake_table_v3_timelock_proposal() -> Result<()> {
        let proxy_addr = Address::random();
        let v3_impl_addr = Address::random();
        let timelock_addr = Address::random();
        let salt = B256::repeat_byte(0x42);
        let delay = U256::from(3600);
        let init_data: Bytes = StakeTableV3::initializeV3Call {}.abi_encode().into();

        let proposal = encode_stake_table_v3_timelock_proposal(
            proxy_addr,
            v3_impl_addr,
            timelock_addr,
            init_data.clone(),
            &StakeTableV3TimelockProposalParams { salt, delay },
        )?;

        assert_eq!(proposal.timelock_addr, timelock_addr);
        assert_eq!(proposal.v3_impl_addr, v3_impl_addr);
        assert_eq!(proposal.schedule.to, timelock_addr);
        assert_eq!(proposal.execute.to, timelock_addr);
        assert!(proposal.schedule.data.len() > 4);
        assert!(proposal.execute.data.len() > 4);

        // The inner payload the timelock will run is
        // `proxy.upgradeToAndCall(v3_impl, initializeV3())`.
        let expected_inner: Bytes = StakeTableV3::upgradeToAndCallCall {
            newImplementation: v3_impl_addr,
            data: init_data,
        }
        .abi_encode()
        .into();

        let expected_schedule = OpsTimelock::scheduleCall {
            target: proxy_addr,
            value: U256::ZERO,
            data: expected_inner.clone(),
            predecessor: B256::ZERO,
            salt,
            delay,
        }
        .abi_encode();
        assert_eq!(proposal.schedule.data.to_vec(), expected_schedule);

        let expected_execute = OpsTimelock::executeCall {
            target: proxy_addr,
            value: U256::ZERO,
            payload: expected_inner,
            predecessor: B256::ZERO,
            salt,
        }
        .abi_encode();
        assert_eq!(proposal.execute.data.to_vec(), expected_execute);

        Ok(())
    }

    /// When the proxy is already at V3, the inner `upgradeToAndCall` carries
    /// empty init data so we don't re-run `initializeV3()`.
    #[test]
    fn test_encode_stake_table_v3_timelock_proposal_already_initialized() -> Result<()> {
        let proxy_addr = Address::random();
        let v3_impl_addr = Address::random();
        let timelock_addr = Address::random();
        let salt = B256::ZERO;
        let delay = U256::ZERO;

        let proposal = encode_stake_table_v3_timelock_proposal(
            proxy_addr,
            v3_impl_addr,
            timelock_addr,
            Bytes::new(),
            &StakeTableV3TimelockProposalParams { salt, delay },
        )?;

        let expected_inner: Bytes = StakeTableV3::upgradeToAndCallCall {
            newImplementation: v3_impl_addr,
            data: Bytes::new(),
        }
        .abi_encode()
        .into();

        let expected_schedule = OpsTimelock::scheduleCall {
            target: proxy_addr,
            value: U256::ZERO,
            data: expected_inner,
            predecessor: B256::ZERO,
            salt,
            delay,
        }
        .abi_encode();
        assert_eq!(proposal.schedule.data.to_vec(), expected_schedule);

        Ok(())
    }
}
