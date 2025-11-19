//! builder pattern for

use alloy::{
    hex::FromHex,
    primitives::{Address, B256, U256},
    providers::{Provider, WalletProvider},
};
use anyhow::{Context, Result};
use derive_builder::Builder;
use espresso_types::v0_1::L1Client;
use hotshot_contract_adapter::sol_types::{LightClientStateSol, StakeTableStateSol};
use url::Url;

use crate::{
    encode_function_call,
    proposals::{
        multisig::{
            transfer_ownership_from_multisig_to_timelock, upgrade_esp_token_v2_multisig_owner,
            upgrade_light_client_v2_multisig_owner, upgrade_light_client_v3_multisig_owner,
            upgrade_stake_table_v2_multisig_owner, LightClientV2UpgradeParams,
            TransferOwnershipParams,
        },
        timelock::{
            cancel_timelock_operation, execute_timelock_operation, schedule_timelock_operation,
            TimelockOperationData, TimelockOperationType,
        },
    },
    Contract, Contracts,
};

/// Convenient handler that builds all the input arguments ready to be deployed.
/// - `deployer`: deployer's wallet provider
/// - `rpc_url`: RPC URL for the L1 network
/// - `token_recipient`: initial token holder, same as deployer if None.
/// - `mock_light_client`: flag to indicate whether deploying mocked contract
/// - `genesis_lc_state`: Genesis light client state
/// - `genesis_st_state`: Genesis stake table state
/// - `permissioned_prover`: permissioned light client prover address
/// - `blocks_per_epoch`: epoch length in block height
/// - `epoch_start_block`: block height for the first *activated* epoch
/// - `exit_escrow_period`: exit escrow period for stake table (in seconds)
/// - `multisig`: new owner/multisig that owns all the proxy contracts
/// - `multisig_pauser`: new multisig that owns the pauser role
/// - `initial_token_supply`: initial token supply for the token contract
/// - `token_name`: name of the token
/// - `token_symbol`: symbol of the token
/// - `ops_timelock_admin`: admin address for the ops timelock
/// - `ops_timelock_delay`: delay for the ops timelock
/// - `ops_timelock_executors`: executors for the ops timelock
/// - `ops_timelock_proposers`: proposers for the ops timelock
/// - `safe_exit_timelock_admin`: admin address for the safe exit timelock
/// - `safe_exit_timelock_delay`: delay for the safe exit timelock
/// - `safe_exit_timelock_executors`: executors for the safe exit timelock
/// - `safe_exit_timelock_proposers`: proposers for the safe exit timelock
/// - `timelock_operation_type`: type of the timelock operation
/// - `target_contract`: target contract for the contract operations
/// - `timelock_operation_value`: value for the timelock operation
/// - `timelock_operation_delay`: delay for the timelock operation
/// - `timelock_operation_function_signature`: function signature for the timelock operation
/// - `timelock_operation_function_values`: function values for the timelock operation
/// - `timelock_operation_salt`: salt for the timelock operation
/// - `use_timelock_owner`: flag to indicate whether to transfer ownership to the timelock owner
/// - `timelock_address`: address of the timelock contract
#[derive(Builder, Clone)]
#[builder(setter(strip_option))]
pub struct DeployerArgs<P: Provider + WalletProvider> {
    deployer: P,
    rpc_url: Url,
    #[builder(default)]
    token_recipient: Option<Address>,
    #[builder(default)]
    mock_light_client: bool,
    #[builder(default)]
    use_multisig: bool,
    #[builder(default)]
    dry_run: bool,
    #[builder(default)]
    genesis_lc_state: Option<LightClientStateSol>,
    #[builder(default)]
    genesis_st_state: Option<StakeTableStateSol>,
    #[builder(default)]
    permissioned_prover: Option<Address>,
    #[builder(default)]
    blocks_per_epoch: Option<u64>,
    #[builder(default)]
    epoch_start_block: Option<u64>,
    #[builder(default)]
    exit_escrow_period: Option<U256>,
    #[builder(default)]
    multisig: Option<Address>,
    #[builder(default)]
    multisig_pauser: Option<Address>,
    #[builder(default)]
    initial_token_supply: Option<U256>,
    #[builder(default)]
    token_name: Option<String>,
    #[builder(default)]
    token_symbol: Option<String>,
    #[builder(default)]
    ops_timelock_admin: Option<Address>,
    #[builder(default)]
    ops_timelock_delay: Option<U256>,
    #[builder(default)]
    ops_timelock_executors: Option<Vec<Address>>,
    #[builder(default)]
    ops_timelock_proposers: Option<Vec<Address>>,
    #[builder(default)]
    safe_exit_timelock_admin: Option<Address>,
    #[builder(default)]
    safe_exit_timelock_delay: Option<U256>,
    #[builder(default)]
    safe_exit_timelock_executors: Option<Vec<Address>>,
    #[builder(default)]
    safe_exit_timelock_proposers: Option<Vec<Address>>,
    #[builder(default)]
    timelock_operation_type: Option<TimelockOperationType>,
    #[builder(default)]
    target_contract: Option<String>,
    #[builder(default)]
    timelock_operation_value: Option<U256>,
    #[builder(default)]
    timelock_operation_delay: Option<U256>,
    #[builder(default)]
    timelock_operation_function_signature: Option<String>,
    #[builder(default)]
    timelock_operation_function_values: Option<Vec<String>>,
    #[builder(default)]
    timelock_operation_salt: Option<String>,
    #[builder(default)]
    use_timelock_owner: Option<bool>,
    #[builder(default)]
    transfer_ownership_from_eoa: Option<bool>,
    #[builder(default)]
    transfer_ownership_new_owner: Option<Address>,
    #[builder(default)]
    timelock_address: Option<Address>,
}

impl<P: Provider + WalletProvider> DeployerArgs<P> {
    /// deploy target contracts
    pub async fn deploy(&self, contracts: &mut Contracts, target: Contract) -> Result<()> {
        let provider = &self.deployer;
        let admin = provider.default_signer_address();
        match target {
            Contract::FeeContractProxy => {
                let addr = crate::deploy_fee_contract_proxy(provider, contracts, admin).await?;

                if let Some(use_timelock_owner) = self.use_timelock_owner {
                    // FeeContract uses OpsTimelock because:
                    // - It handles critical fee collection and distribution logic
                    // - May require emergency updates for security or functionality
                    // - OpsTimelock provides a shorter delay for critical operations
                    tracing::info!(
                        "Transferring ownership to OpsTimelock: {:?}",
                        use_timelock_owner
                    );
                    // deployer is the timelock owner
                    if use_timelock_owner {
                        let timelock_addr = contracts
                            .address(Contract::OpsTimelock)
                            .expect("fail to get OpsTimelock address");
                        crate::transfer_ownership(provider, target, addr, timelock_addr).await?;
                    }
                } else if let Some(multisig) = self.multisig {
                    tracing::info!("Transferring ownership to multisig: {:?}", multisig);
                    crate::transfer_ownership(provider, target, addr, multisig).await?;
                }
            },
            Contract::EspTokenProxy => {
                let token_recipient = self.token_recipient.unwrap_or(admin);
                let token_name = self
                    .token_name
                    .clone()
                    .context("Token name must be set when deploying esp token")?;
                let token_symbol = self
                    .token_symbol
                    .clone()
                    .context("Token symbol must be set when deploying esp token")?;
                let initial_supply = self
                    .initial_token_supply
                    .context("Initial token supply must be set when deploying esp token")?;
                crate::deploy_token_proxy(
                    provider,
                    contracts,
                    admin,
                    token_recipient,
                    initial_supply,
                    &token_name,
                    &token_symbol,
                )
                .await?;

                // NOTE: we don't transfer ownership to multisig, we only do so after V2 upgrade
            },
            Contract::EspTokenV2 => {
                let use_multisig = self.use_multisig;

                if use_multisig {
                    upgrade_esp_token_v2_multisig_owner(
                        provider,
                        contracts,
                        self.rpc_url.to_string(),
                        Some(self.dry_run),
                    )
                    .await?;
                } else {
                    crate::upgrade_esp_token_v2(provider, contracts).await?;
                    let addr = contracts
                        .address(Contract::EspTokenProxy)
                        .expect("fail to get EspTokenProxy address");

                    if let Some(use_timelock_owner) = self.use_timelock_owner {
                        // deployer is the timelock owner
                        if use_timelock_owner {
                            // EspToken uses SafeExitTimelock (not OpsTimelock) because:
                            // - It's a simple ERC20 token with minimal upgrade complexity
                            // - No emergency updates are expected for token functionality
                            // - SafeExitTimelock provides sufficient security for token operations
                            tracing::info!("Transferring ownership to SafeExitTimelock");
                            let timelock_addr = contracts
                                .address(Contract::SafeExitTimelock)
                                .expect("fail to get SafeExitTimelock address");
                            crate::transfer_ownership(provider, target, addr, timelock_addr)
                                .await?;
                        }
                    } else if let Some(multisig) = self.multisig {
                        let token_proxy = contracts
                            .address(Contract::EspTokenProxy)
                            .expect("fail to get EspTokenProxy address");
                        crate::transfer_ownership(
                            provider,
                            Contract::EspTokenProxy,
                            token_proxy,
                            multisig,
                        )
                        .await?;
                    }
                }
            },
            Contract::LightClientProxy => {
                assert!(
                    self.genesis_lc_state.is_some(),
                    "forget to specify genesis_lc_state()"
                );
                assert!(
                    self.genesis_st_state.is_some(),
                    "forget to specify genesis_st_state()"
                );
                crate::deploy_light_client_proxy(
                    provider,
                    contracts,
                    self.mock_light_client,
                    self.genesis_lc_state.clone().unwrap(),
                    self.genesis_st_state.clone().unwrap(),
                    admin,
                    self.permissioned_prover,
                )
                .await?;
                // NOTE: we don't transfer ownership to multisig, we only do so after V2 upgrade
            },
            Contract::LightClientV2 => {
                assert!(
                    self.blocks_per_epoch.is_some(),
                    "forgot to specify blocks_per_epoch()"
                );
                assert!(
                    self.epoch_start_block.is_some(),
                    "forgot to specify epoch_start_block()"
                );

                let use_mock = self.mock_light_client;
                let dry_run = self.dry_run;
                let use_multisig = self.use_multisig;
                let mut blocks_per_epoch = self.blocks_per_epoch.unwrap();
                let epoch_start_block = self.epoch_start_block.unwrap();
                let rpc_url = self.rpc_url.clone();

                // TEST-ONLY: if this config is not yet set, we use u64::MAX
                // to avoid contract complaining about invalid zero-valued blocks_per_epoch.
                // This value will allow tests to proceed with realistic epoch behavior.
                // TODO: remove this once we have a proper way to set blocks_per_epoch
                if use_mock && blocks_per_epoch == 0 {
                    blocks_per_epoch = u64::MAX;
                }
                tracing::info!(%blocks_per_epoch, ?dry_run, ?use_multisig, "Upgrading LightClientV2 with ");
                if use_multisig {
                    upgrade_light_client_v2_multisig_owner(
                        provider,
                        contracts,
                        LightClientV2UpgradeParams {
                            blocks_per_epoch,
                            epoch_start_block,
                        },
                        use_mock,
                        rpc_url.to_string(),
                        Some(dry_run),
                    )
                    .await?;
                } else {
                    crate::upgrade_light_client_v2(
                        provider,
                        contracts,
                        use_mock,
                        blocks_per_epoch,
                        epoch_start_block,
                    )
                    .await?;
                    // NOTE: we don't transfer ownership to multisig, we only do so after V3 upgrade
                }
            },
            Contract::LightClientV3 => {
                let use_mock = self.mock_light_client;
                let dry_run = self.dry_run;
                let use_multisig = self.use_multisig;
                let rpc_url = self.rpc_url.clone();

                tracing::info!(?dry_run, ?use_multisig, "Upgrading LightClientV3 with ");
                if use_multisig {
                    upgrade_light_client_v3_multisig_owner(
                        provider,
                        contracts,
                        use_mock,
                        rpc_url.to_string(),
                        Some(dry_run),
                    )
                    .await?;
                } else {
                    crate::upgrade_light_client_v3(provider, contracts, use_mock).await?;

                    // Transfer ownership to Timelook or MultiSig
                    let addr = contracts
                        .address(Contract::LightClientProxy)
                        .expect("fail to get LightClientProxy address");

                    if let Some(use_timelock_owner) = self.use_timelock_owner {
                        // LightClient uses OpsTimelock because:
                        // - It's a critical security component for the network
                        // - May require emergency updates for security vulnerabilities
                        // - OpsTimelock provides a shorter delay for critical operations
                        tracing::info!("Transferring ownership to OpsTimelock");
                        // deployer is the timelock owner
                        if use_timelock_owner {
                            let timelock_addr = contracts
                                .address(Contract::OpsTimelock)
                                .expect("fail to get OpsTimelock address");
                            crate::transfer_ownership(provider, target, addr, timelock_addr)
                                .await?;
                        }
                    } else if let Some(multisig) = self.multisig {
                        let lc_proxy = contracts
                            .address(Contract::LightClientProxy)
                            .expect("fail to get LightClientProxy address");
                        crate::transfer_ownership(
                            provider,
                            Contract::LightClientProxy,
                            lc_proxy,
                            multisig,
                        )
                        .await?;
                    }
                }
            },
            Contract::StakeTableProxy => {
                let token_addr = contracts
                    .address(Contract::EspTokenProxy)
                    .context("no ESP token proxy address")?;
                let lc_addr = contracts
                    .address(Contract::LightClientProxy)
                    .context("no LightClient proxy address")?;
                let escrow_period = self.exit_escrow_period.unwrap_or(U256::from(250));
                crate::deploy_stake_table_proxy(
                    provider,
                    contracts,
                    token_addr,
                    lc_addr,
                    escrow_period,
                    admin,
                )
                .await?;

                // NOTE: we don't transfer ownership to multisig, we only do so after V2 upgrade
            },
            Contract::StakeTableV2 => {
                let use_multisig = self.use_multisig;
                let dry_run = self.dry_run;
                let multisig_pauser = self.multisig_pauser.context(
                    "Multisig pauser address must be set for the upgrade to StakeTableV2",
                )?;
                let l1_client = L1Client::new(vec![self.rpc_url.clone()])?;
                tracing::info!(?dry_run, ?use_multisig, "Upgrading to StakeTableV2 with ");
                if use_multisig {
                    upgrade_stake_table_v2_multisig_owner(
                        provider,
                        l1_client,
                        contracts,
                        self.rpc_url.to_string(),
                        self.multisig.context(
                            "Multisig address must be set when upgrading to --use-multisig flag \
                             is present",
                        )?,
                        multisig_pauser,
                        Some(dry_run),
                    )
                    .await?;
                } else {
                    crate::upgrade_stake_table_v2(
                        provider,
                        l1_client,
                        contracts,
                        multisig_pauser,
                        admin,
                    )
                    .await?;

                    let addr = contracts
                        .address(Contract::StakeTableProxy)
                        .expect("fail to get StakeTableProxy address");

                    if let Some(use_timelock_owner) = self.use_timelock_owner {
                        // StakeTable uses OpsTimelock because:
                        // - It manages critical staking and validator operations
                        // - May require emergency updates for security or functionality
                        // - OpsTimelock provides a shorter delay for critical operations
                        tracing::info!("Transferring ownership to OpsTimelock");
                        // deployer is the timelock owner
                        if use_timelock_owner {
                            let timelock_addr = contracts
                                .address(Contract::OpsTimelock)
                                .expect("fail to get OpsTimelock address");
                            crate::transfer_ownership(provider, target, addr, timelock_addr)
                                .await?;
                        }
                    } else if let Some(multisig) = self.multisig {
                        let stake_table_proxy = contracts
                            .address(Contract::StakeTableProxy)
                            .expect("fail to get StakeTableProxy address");
                        crate::transfer_ownership(
                            provider,
                            Contract::StakeTableProxy,
                            stake_table_proxy,
                            multisig,
                        )
                        .await?;
                    }
                }
            },
            Contract::OpsTimelock => {
                let ops_timelock_delay = self
                    .ops_timelock_delay
                    .context("Ops Timelock delay must be set when deploying Ops Timelock")?;
                let ops_timelock_proposers = self
                    .ops_timelock_proposers
                    .clone()
                    .context("Ops Timelock proposers must be set when deploying Ops Timelock")?;
                let ops_timelock_executors = self
                    .ops_timelock_executors
                    .clone()
                    .context("Ops Timelock executors must be set when deploying Ops Timelock")?;
                let ops_timelock_admin = self
                    .ops_timelock_admin
                    .context("Ops Timelock admin must be set when deploying Ops Timelock")?;
                crate::deploy_ops_timelock(
                    provider,
                    contracts,
                    ops_timelock_delay,
                    ops_timelock_proposers,
                    ops_timelock_executors,
                    ops_timelock_admin,
                )
                .await?;
            },
            Contract::SafeExitTimelock => {
                let safe_exit_timelock_delay = self.safe_exit_timelock_delay.context(
                    "SafeExitTimelock delay must be set when deploying SafeExitTimelock",
                )?;
                let safe_exit_timelock_proposers =
                    self.safe_exit_timelock_proposers.clone().context(
                        "SafeExitTimelock proposers must be set when deploying SafeExitTimelock",
                    )?;
                let safe_exit_timelock_executors =
                    self.safe_exit_timelock_executors.clone().context(
                        "SafeExitTimelock executors must be set when deploying SafeExitTimelock",
                    )?;
                let safe_exit_timelock_admin = self.safe_exit_timelock_admin.context(
                    "SafeExitTimelock admin must be set when deploying SafeExitTimelock",
                )?;
                crate::deploy_safe_exit_timelock(
                    provider,
                    contracts,
                    safe_exit_timelock_delay,
                    safe_exit_timelock_proposers,
                    safe_exit_timelock_executors,
                    safe_exit_timelock_admin,
                )
                .await?;
            },
            Contract::RewardClaimProxy => {
                let token_addr = contracts
                    .address(Contract::EspTokenProxy)
                    .context("no ESP token proxy address")?;
                let lc_addr = contracts
                    .address(Contract::LightClientProxy)
                    .context("no LightClient proxy address")?;
                let pauser = self.multisig_pauser.context(
                    "Multisig pauser address must be set for RewardClaimProxy deployment",
                )?;
                let addr = crate::deploy_reward_claim_proxy(
                    provider, contracts, token_addr, lc_addr, admin, pauser,
                )
                .await?;

                if let Some(use_timelock_owner) = self.use_timelock_owner {
                    // RewardClaim uses SafeExitTimelock because:
                    // - It is not expected to require urgent upgrades.
                    // - In emergency situations it can be paused.
                    // - It can mint ESP tokens, users should have enough time
                    //   to react if they do not agree with an upgrade.
                    tracing::info!("Transferring ownership to SafeExitTimelock");
                    if use_timelock_owner {
                        let timelock_addr = contracts
                            .address(Contract::SafeExitTimelock)
                            .expect("fail to get SafeExitTimelock address");
                        crate::transfer_ownership(provider, target, addr, timelock_addr).await?;
                    }
                } else if let Some(multisig) = self.multisig {
                    tracing::info!("Transferring ownership to multisig: {:?}", multisig);
                    crate::transfer_ownership(provider, target, addr, multisig).await?;
                }
            },
            _ => {
                panic!("Deploying {target} not supported.");
            },
        }
        Ok(())
    }

    /// Deploy all contracts up to and including stake table v1
    pub async fn deploy_to_stake_table_v1(&self, contracts: &mut Contracts) -> Result<()> {
        // Deploy timelocks first so they can be used as owners for other contracts
        self.deploy(contracts, Contract::OpsTimelock).await?;
        self.deploy(contracts, Contract::SafeExitTimelock).await?;

        // Then deploy other contracts
        self.deploy(contracts, Contract::FeeContractProxy).await?;
        self.deploy(contracts, Contract::EspTokenProxy).await?;
        self.deploy(contracts, Contract::LightClientProxy).await?;
        self.deploy(contracts, Contract::LightClientV2).await?;
        self.deploy(contracts, Contract::StakeTableProxy).await?;
        Ok(())
    }

    /// Deploy all contracts
    pub async fn deploy_all(&self, contracts: &mut Contracts) -> Result<()> {
        self.deploy_to_stake_table_v1(contracts).await?;
        self.deploy(contracts, Contract::StakeTableV2).await?;
        self.deploy(contracts, Contract::LightClientV3).await?;
        self.deploy(contracts, Contract::RewardClaimProxy).await?;
        self.deploy(contracts, Contract::EspTokenV2).await?;
        Ok(())
    }

    // Perform a timelock operation
    ///
    /// Parameters:
    /// - `contracts`: ref to deployed contracts
    ///
    pub async fn perform_timelock_operation_on_contract(
        &self,
        contracts: &mut Contracts,
    ) -> Result<()> {
        let timelock_operation_type = self
            .timelock_operation_type
            .context("Timelock operation type not found")?;
        let target_contract = self
            .target_contract
            .clone()
            .context("Timelock target not found")?;
        let value = self
            .timelock_operation_value
            .context("Timelock operation value not found")?;
        let function_signature = self
            .timelock_operation_function_signature
            .as_ref()
            .context("Timelock operation function signature not found")?;
        let function_values = self
            .timelock_operation_function_values
            .clone()
            .context("Timelock operation function values not found")?;
        let salt = self
            .timelock_operation_salt
            .clone()
            .context("Timelock operation salt not found")?;
        let delay = self
            .timelock_operation_delay
            .context("Timelock operation delay not found")?;

        let (target_addr, contract_type) = match target_contract.as_str() {
            "FeeContract" => (
                contracts
                    .address(Contract::FeeContractProxy)
                    .context("FeeContractProxy address not found")?,
                Contract::FeeContractProxy,
            ),
            "EspToken" => (
                contracts
                    .address(Contract::EspTokenProxy)
                    .context("EspTokenProxy address not found")?,
                Contract::EspTokenProxy,
            ),
            "LightClient" => (
                contracts
                    .address(Contract::LightClientProxy)
                    .context("LightClientProxy address not found")?,
                Contract::LightClientProxy,
            ),
            "StakeTable" => (
                contracts
                    .address(Contract::StakeTableProxy)
                    .context("StakeTableProxy address not found")?,
                Contract::StakeTableProxy,
            ),
            _ => anyhow::bail!("Invalid target contract: {}", target_contract),
        };

        let function_calldata = encode_function_call(function_signature, function_values.clone())
            .context("Failed to encode function data")?;

        // Parse salt from string to B256
        let salt_bytes = if salt == "0x" || salt.is_empty() {
            B256::ZERO // Use zero salt if empty
        } else if let Some(stripped) = salt.strip_prefix("0x") {
            B256::from_hex(stripped).context("Invalid salt hex format")?
        } else {
            B256::from_hex(&salt).context("Invalid salt hex format")?
        };

        let timelock_operation_data = TimelockOperationData {
            target: target_addr,
            value,
            data: function_calldata,
            predecessor: B256::ZERO, // Default to no predecessor
            salt: salt_bytes,
            delay,
        };

        match timelock_operation_type {
            TimelockOperationType::Schedule => {
                let operation_id = schedule_timelock_operation(
                    &self.deployer,
                    contract_type,
                    timelock_operation_data,
                )
                .await?;
                tracing::info!("Timelock operation scheduled with ID: {}", operation_id);
            },
            TimelockOperationType::Execute => {
                let tx_id = execute_timelock_operation(
                    &self.deployer,
                    contract_type,
                    timelock_operation_data,
                )
                .await?;
                tracing::info!("Timelock operation executed with ID: {}", tx_id);
            },
            TimelockOperationType::Cancel => {
                let tx_id = cancel_timelock_operation(
                    &self.deployer,
                    contract_type,
                    timelock_operation_data,
                )
                .await?;
                tracing::info!("Timelock operation cancelled with ID: {}", tx_id);
            },
        }
        Ok(())
    }

    /// Propose ownership transfer from multisig to timelock
    pub async fn propose_transfer_ownership_to_timelock(
        &self,
        contracts: &mut Contracts,
    ) -> Result<()> {
        let multisig = self.multisig.expect(
            "Multisig address must be set when proposing ownership transfer. Use \
             --multisig-address or ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS",
        );
        let target_contract = self.target_contract.clone().ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide target_contract when using \
                 --propose-transfer-ownership-to-timelock. Use --target-contract or \
                 ESPRESSO_TARGET_CONTRACT"
            )
        })?;

        let timelock_address = self.timelock_address.ok_or_else(|| {
            anyhow::anyhow!(
                "Timelock address must be set when proposing ownership transfer. Use \
                 --timelock-address or ESPRESSO_SEQUENCER_TIMELOCK_ADDRESS"
            )
        })?;

        // Parse the contract type from string
        let contract_type = match target_contract.to_lowercase().as_str() {
            "lightclient" | "lightclientproxy" => Contract::LightClientProxy,
            "feecontract" | "feecontractproxy" => Contract::FeeContractProxy,
            "esptoken" | "esptokenproxy" => Contract::EspTokenProxy,
            "staketable" | "staketableproxy" => Contract::StakeTableProxy,
            _ => anyhow::bail!(
                "Unknown contract type: {}. Supported types: lightclient, feecontract, esptoken, \
                 staketable",
                target_contract
            ),
        };

        tracing::info!(
            "Proposing transfer of ownership from multisig to timelock for {}",
            target_contract
        );

        let contract = contract_type;
        let rpc_url = self.rpc_url.clone();
        let dry_run = self.dry_run;
        let use_hardware_wallet = false;
        let result = transfer_ownership_from_multisig_to_timelock(
            &self.deployer,
            contracts,
            contract,
            TransferOwnershipParams {
                new_owner: timelock_address,
                rpc_url,
                safe_addr: multisig,
                use_hardware_wallet,
                dry_run,
            },
        )
        .await?;
        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            let stdout = String::from_utf8_lossy(&result.stdout);
            anyhow::bail!(
                "Failed to propose ownership transfer for {}: {}\nStdout: {}\nStderr: {}",
                contract,
                result.status,
                stdout,
                stderr
            );
        }
        tracing::info!("Successfully proposed ownership transfer for {}", contract);
        Ok(())
    }

    /// Transfer ownership from EOA to new owner
    pub async fn transfer_ownership_from_eoa(&self, contracts: &mut Contracts) -> Result<()> {
        let transfer_ownership_from_eoa = self
            .transfer_ownership_from_eoa
            .ok_or_else(|| anyhow::anyhow!("transfer_ownership_from_eoa flag not set"))?;

        if !transfer_ownership_from_eoa {
            return Ok(());
        }

        let target_contract = self.target_contract.clone().ok_or_else(|| {
            anyhow::anyhow!("Must provide target_contract when using transfer_ownership_from_eoa")
        })?;
        let new_owner = self.transfer_ownership_new_owner.ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide transfer_ownership_new_owner when using transfer_ownership_from_eoa"
            )
        })?;

        // Parse the contract type from string
        let contract_type = match target_contract.to_lowercase().as_str() {
            "lightclient" | "lightclientproxy" => Contract::LightClientProxy,
            "feecontract" | "feecontractproxy" => Contract::FeeContractProxy,
            "esptoken" | "esptokenproxy" => Contract::EspTokenProxy,
            "staketable" | "staketableproxy" => Contract::StakeTableProxy,
            _ => anyhow::bail!(
                "Unknown contract type: {}. Supported types: lightclient, feecontract, esptoken, \
                 staketable",
                target_contract
            ),
        };

        // Get the contract address from the contracts map
        let contract_address = contracts.address(contract_type).ok_or_else(|| {
            anyhow::anyhow!(
                "Contract {} not found in deployed contracts",
                target_contract
            )
        })?;

        tracing::info!(
            "Transferring ownership of {} from EOA to {}",
            target_contract,
            new_owner
        );

        // Use the existing transfer_ownership function from lib.rs
        let receipt =
            crate::transfer_ownership(&self.deployer, contract_type, contract_address, new_owner)
                .await?;

        tracing::info!(
            "Successfully transferred ownership of {} to {}. Transaction: {}",
            target_contract,
            new_owner,
            receipt.transaction_hash
        );

        Ok(())
    }
}
