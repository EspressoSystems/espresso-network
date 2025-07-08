use std::process::Output;

use super::*;

#[derive(Clone)]
pub struct TransferOwnershipParams {
    pub new_owner: Address,
    pub rpc_url: String,
    pub safe_addr: Address,
    pub use_hardware_wallet: bool,
    pub dry_run: bool,
}

/// Call the transfer ownership script to transfer ownership of a contract to a new owner
///
/// Parameters:
/// - `proxy_addr`: The address of the proxy contract
/// - `new_owner`: The address of the new owner
/// - `rpc_url`: The RPC URL for the network
pub async fn call_transfer_ownership_script(
    proxy_addr: Address,
    params: TransferOwnershipParams,
) -> Result<Output, anyhow::Error> {
    let script_path = super::find_script_path()?;
    let output = Command::new(script_path)
        .arg("transferOwnership.ts")
        .arg("--from-rust")
        .arg("--proxy")
        .arg(proxy_addr.to_string())
        .arg("--new-owner")
        .arg(params.new_owner.to_string())
        .arg("--rpc-url")
        .arg(params.rpc_url)
        .arg("--safe-address")
        .arg(params.safe_addr.to_string())
        .arg("--dry-run")
        .arg(params.dry_run.to_string())
        .arg("--use-hardware-wallet")
        .arg(params.use_hardware_wallet.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let output = output.unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    // if stderr is not empty, return the stderr
    if !output.status.success() {
        return Err(anyhow!("Transfer ownership script failed: {}", stderr));
    }
    Ok(output)
}

pub async fn transfer_ownership_from_multisig_to_timelock(
    provider: impl Provider,
    contracts: &mut Contracts,
    contract: Contract,
    params: TransferOwnershipParams,
) -> Result<Output> {
    tracing::info!(
        "Proposing ownership transfer for {} from multisig {} to timelock {}",
        contract,
        params.safe_addr,
        params.new_owner
    );

    let (proxy_addr, proxy_instance) = match contract {
        Contract::LightClientProxy
        | Contract::FeeContractProxy
        | Contract::EspTokenProxy
        | Contract::StakeTableProxy => {
            let addr = contracts
                .address(contract)
                .ok_or_else(|| anyhow!("{contract} (multisig owner) not found, can't upgrade"))?;
            (addr, OwnableUpgradeable::new(addr, &provider))
        },
        _ => anyhow::bail!("Not a proxy contract, can't transfer ownership"),
    };
    tracing::info!("{} found at {proxy_addr:#x}", contract);

    let owner_addr = proxy_instance.owner().call().await?._0;

    if !params.dry_run && !super::is_contract(provider, owner_addr).await? {
        tracing::error!("Proxy owner is not a contract. Expected: {owner_addr:#x}");
        anyhow::bail!(
            "Proxy owner is not a contract. Expected: {owner_addr:#x}. Use --dry-run to skip this \
             check."
        );
    }

    // invoke upgrade on proxy via the safeSDK
    let result = call_transfer_ownership_script(proxy_addr, params.clone()).await?;
    if !result.status.success() {
        anyhow::bail!(
            "Transfer ownership script failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    if !params.dry_run {
        tracing::info!("Transfer Ownership proposal sent to {}", contract);
        tracing::info!("Send this link to the signers to sign the proposal: https://app.safe.global/transactions/queue?safe={}", params.safe_addr);
        // IDEA: add a function to wait for the proposal to be executed
    } else {
        tracing::info!("Dry run, skipping proposal");
    }
    Ok(result)
}

pub mod timelock_proposals {
    use clap::ValueEnum;

    use super::*;

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
                        .await?
                        ._0)
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
                        .await?
                        ._0)
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
                        .await?
                        ._0)
                },
                TimelockContract::SafeExitTimelock(timelock_addr) => {
                    Ok(SafeExitTimelock::new(*timelock_addr, &provider)
                        .isOperationPending(operation_id)
                        .call()
                        .await?
                        ._0)
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
                        .await?
                        ._0)
                },
                TimelockContract::SafeExitTimelock(timelock_addr) => {
                    Ok(SafeExitTimelock::new(*timelock_addr, &provider)
                        .isOperationReady(operation_id)
                        .call()
                        .await?
                        ._0)
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
                        .await?
                        ._0)
                },
                TimelockContract::SafeExitTimelock(timelock_addr) => {
                    Ok(SafeExitTimelock::new(*timelock_addr, &provider)
                        .isOperationDone(operation_id)
                        .call()
                        .await?
                        ._0)
                },
            }
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
                    Ok(receipt)
                },
            }
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
        let timelock = match contract_type {
            Contract::FeeContractProxy => {
                let proxy = FeeContract::new(target_addr, &provider);
                let proxy_owner = proxy.owner().call().await?._0;
                TimelockContract::OpsTimelock(proxy_owner)
            },
            Contract::EspTokenProxy => {
                let proxy = EspToken::new(target_addr, &provider);
                let proxy_owner = proxy.owner().call().await?._0;
                TimelockContract::SafeExitTimelock(proxy_owner)
            },
            Contract::LightClientProxy => {
                let proxy = LightClient::new(target_addr, &provider);
                let proxy_owner = proxy.owner().call().await?._0;
                TimelockContract::OpsTimelock(proxy_owner)
            },
            Contract::StakeTableProxy => {
                let proxy = StakeTable::new(target_addr, &provider);
                let proxy_owner = proxy.owner().call().await?._0;
                TimelockContract::OpsTimelock(proxy_owner)
            },
            _ => anyhow::bail!(
                "Invalid contract type for timelock schedule operation: {}",
                contract_type
            ),
        };
        let operation_id = timelock.get_operation_id(&operation, &provider).await?;

        let receipt = timelock.schedule(operation, &provider).await?;
        tracing::info!(%receipt.gas_used, %receipt.transaction_hash, "tx mined");
        if !receipt.inner.is_success() {
            anyhow::bail!("tx failed: {:?}", receipt);
        }

        // check that the tx is scheduled
        if !(timelock
            .is_operation_pending(operation_id, &provider)
            .await?
            || timelock.is_operation_ready(operation_id, &provider).await?)
        {
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
        let timelock = match contract_type {
            Contract::FeeContractProxy => {
                let proxy = FeeContract::new(target_addr, &provider);
                let proxy_owner = proxy.owner().call().await?._0;
                TimelockContract::OpsTimelock(proxy_owner)
            },
            Contract::EspTokenProxy => {
                let proxy = EspToken::new(target_addr, &provider);
                let proxy_owner = proxy.owner().call().await?._0;
                TimelockContract::SafeExitTimelock(proxy_owner)
            },
            Contract::LightClientProxy => {
                let proxy = LightClient::new(target_addr, &provider);
                let proxy_owner = proxy.owner().call().await?._0;
                TimelockContract::OpsTimelock(proxy_owner)
            },
            Contract::StakeTableProxy => {
                let proxy = StakeTable::new(target_addr, &provider);
                let proxy_owner = proxy.owner().call().await?._0;
                TimelockContract::OpsTimelock(proxy_owner)
            },
            _ => anyhow::bail!(
                "Invalid contract type for timelock execute operation: {}",
                contract_type
            ),
        };
        let operation_id = timelock.get_operation_id(&operation, &provider).await?;

        // execute the tx
        let receipt = timelock.execute(operation, &provider).await?;
        tracing::info!(%receipt.gas_used, %receipt.transaction_hash, "tx mined");
        if !receipt.inner.is_success() {
            anyhow::bail!("tx failed: {:?}", receipt);
        }

        // check that the tx is executed
        if !timelock.is_operation_done(operation_id, &provider).await? {
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
        let timelock = match contract_type {
            Contract::FeeContractProxy => {
                let proxy = FeeContract::new(target_addr, &provider);
                let proxy_owner = proxy.owner().call().await?._0;
                TimelockContract::OpsTimelock(proxy_owner)
            },
            Contract::EspTokenProxy => {
                let proxy = EspToken::new(target_addr, &provider);
                let proxy_owner = proxy.owner().call().await?._0;
                TimelockContract::SafeExitTimelock(proxy_owner)
            },
            Contract::LightClientProxy => {
                let proxy = LightClient::new(target_addr, &provider);
                let proxy_owner = proxy.owner().call().await?._0;
                TimelockContract::OpsTimelock(proxy_owner)
            },
            Contract::StakeTableProxy => {
                let proxy = StakeTable::new(target_addr, &provider);
                let proxy_owner = proxy.owner().call().await?._0;
                TimelockContract::OpsTimelock(proxy_owner)
            },
            _ => anyhow::bail!(
                "Invalid contract type for timelock cancel operation: {}",
                contract_type
            ),
        };
        let operation_id = timelock.get_operation_id(&operation, &provider).await?;
        let receipt = timelock.cancel(operation_id, &provider).await?;
        tracing::info!(%receipt.gas_used, %receipt.transaction_hash, "tx mined");
        if !receipt.inner.is_success() {
            anyhow::bail!("tx failed: {:?}", receipt);
        }
        tracing::info!("tx cancelled with id: {}", operation_id);
        Ok(operation_id)
    }
}
