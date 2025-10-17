use std::{
    path::PathBuf,
    process::{Command, Output, Stdio},
};

use alloy::{
    hex::{FromHex, ToHexExt},
    network::TransactionBuilder,
    primitives::{Address, Bytes},
    providers::Provider,
};
use anyhow::{anyhow, Context, Result};
use hotshot_contract_adapter::sol_types::{
    EspToken, EspTokenV2, LightClient, LightClientV2, LightClientV2Mock, LightClientV3,
    LightClientV3Mock, OwnableUpgradeable, PlonkVerifierV2, PlonkVerifierV3, StakeTable,
    StakeTableV2,
};

use crate::{Contract, Contracts, LIBRARY_PLACEHOLDER_ADDRESS};

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
    let script_path = find_script_path()?;
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
        | Contract::StakeTableProxy
        | Contract::RewardClaimProxy => {
            let addr = contracts
                .address(contract)
                .ok_or_else(|| anyhow!("{contract} (multisig owner) not found, can't upgrade"))?;
            (addr, OwnableUpgradeable::new(addr, &provider))
        },
        _ => anyhow::bail!("Not a proxy contract, can't transfer ownership"),
    };
    tracing::info!("{} found at {proxy_addr:#x}", contract);

    let owner_addr = proxy_instance.owner().call().await?;

    if !params.dry_run && !crate::is_contract(provider, owner_addr).await? {
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

pub fn find_script_path() -> Result<PathBuf> {
    let mut path_options = Vec::new();
    if let Ok(cargo_manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        path_options.push(
            PathBuf::from(cargo_manifest_dir.clone())
                .join("../../../scripts/multisig-upgrade-entrypoint"),
        );
        path_options
            .push(PathBuf::from(cargo_manifest_dir).join("../scripts/multisig-upgrade-entrypoint"));
    }
    path_options.push(PathBuf::from("/bin/multisig-upgrade-entrypoint"));
    for path in path_options {
        if path.exists() {
            return Ok(path);
        }
    }
    anyhow::bail!(
        "Upgrade entrypoint script, multisig-upgrade-entrypoint, not found in any of the possible \
         locations"
    );
}

/// Call the upgrade proxy script to upgrade a proxy contract
///
/// Parameters:
/// - `proxy_addr`: The address of the proxy contract
/// - `new_impl_addr`: The address of the new implementation
/// - `init_data`: The initialization data for the new implementation
/// - `rpc_url`: The RPC URL for the network
/// - `safe_addr`: The address of the Safe multisig wallet
/// - `dry_run`: Whether to do a dry run
///
/// Returns:
/// - stdout from the script execution
pub async fn call_upgrade_proxy_script(
    proxy_addr: Address,
    new_impl_addr: Address,
    init_data: String,
    rpc_url: String,
    safe_addr: Address,
    dry_run: Option<bool>,
) -> anyhow::Result<String> {
    let dry_run = dry_run.unwrap_or(false);
    tracing::info!("Dry run: {}", dry_run);
    tracing::info!(
        "Attempting to send the upgrade proposal to multisig: {}",
        safe_addr
    );

    let script_path = find_script_path()?;

    let output = Command::new(script_path)
        .arg("upgradeProxy.ts")
        .arg("--from-rust")
        .arg("--proxy")
        .arg(proxy_addr.to_string())
        .arg("--impl")
        .arg(new_impl_addr.to_string())
        .arg("--init-data")
        .arg(init_data)
        .arg("--rpc-url")
        .arg(rpc_url)
        .arg("--safe-address")
        .arg(safe_addr.to_string())
        .arg("--dry-run")
        .arg(dry_run.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let output = output.unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // if stderr is not empty, return the stderr
    if !output.status.success() {
        anyhow::bail!("Upgrade script failed: {}", stderr);
    }
    Ok(stdout.to_string())
}

/// Verify the node js files are present and can be executed.
///
/// It calls the upgrade proxy script with a dummy address and a dummy rpc url in dry run mode
pub async fn verify_node_js_files() -> Result<()> {
    call_upgrade_proxy_script(
        Address::random(),
        Address::random(),
        String::from("0x"),
        String::from("https://sepolia.infura.io/v3/"),
        Address::random(),
        Some(true),
    )
    .await?;
    tracing::info!("Node.js files verified successfully");
    Ok(())
}

/// Parameters for upgrading LightClient to V2
pub struct LightClientV2UpgradeParams {
    pub blocks_per_epoch: u64,
    pub epoch_start_block: u64,
}

/// Upgrade the light client proxy to use LightClientV2.
/// Internally, first detect existence of proxy, then deploy LCV2, then upgrade and initializeV2.
/// Internal to "deploy LCV2", we deploy PlonkVerifierV2 whose address will be used at LCV2 init time.
/// Assumes:
/// - the proxy is already deployed.
/// - the proxy is owned by a multisig.
/// - the proxy is not yet initialized for V2
///
/// Returns the url link to the upgrade proposal
/// This function can only be called on a real network supported by the safeSDK
pub async fn upgrade_light_client_v2_multisig_owner(
    provider: impl Provider,
    contracts: &mut Contracts,
    params: LightClientV2UpgradeParams,
    is_mock: bool,
    rpc_url: String,
    dry_run: Option<bool>,
) -> Result<String> {
    let expected_major_version: u8 = 2;
    let dry_run = dry_run.unwrap_or_else(|| {
        tracing::warn!("Dry run not specified, defaulting to false");
        false
    });

    let proxy_addr = contracts
        .address(Contract::LightClientProxy)
        .ok_or_else(|| anyhow!("LightClientProxy (multisig owner) not found, can't upgrade"))?;
    tracing::info!("LightClientProxy found at {proxy_addr:#x}");
    let proxy = LightClient::new(proxy_addr, &provider);
    let owner_addr = proxy.owner().call().await?;

    if !dry_run && !crate::is_contract(&provider, owner_addr).await? {
        tracing::error!("Proxy owner is not a contract. Expected: {owner_addr:#x}");
        anyhow::bail!("Proxy owner is not a contract. Expected: {owner_addr:#x}");
    }

    // Prepare addresses
    let (_pv2_addr, lcv2_addr) = if !dry_run {
        // Deploy PlonkVerifierV2.sol (if not already deployed)
        let pv2_addr = contracts
            .deploy(
                Contract::PlonkVerifierV2,
                PlonkVerifierV2::deploy_builder(&provider),
            )
            .await?;

        // then deploy LightClientV2.sol
        let target_lcv2_bytecode = if is_mock {
            LightClientV2Mock::BYTECODE.encode_hex()
        } else {
            LightClientV2::BYTECODE.encode_hex()
        };
        let lcv2_linked_bytecode = {
            match target_lcv2_bytecode
                .matches(LIBRARY_PLACEHOLDER_ADDRESS)
                .count()
            {
                0 => return Err(anyhow!("lib placeholder not found")),
                1 => Bytes::from_hex(target_lcv2_bytecode.replacen(
                    LIBRARY_PLACEHOLDER_ADDRESS,
                    &pv2_addr.encode_hex(),
                    1,
                ))?,
                _ => {
                    return Err(anyhow!(
                        "more than one lib placeholder found, consider using a different value"
                    ))
                },
            }
        };
        let lcv2_addr = if is_mock {
            let addr = LightClientV2Mock::deploy_builder(&provider)
                .map(|req| req.with_deploy_code(lcv2_linked_bytecode))
                .deploy()
                .await?;
            tracing::info!("deployed LightClientV2Mock at {addr:#x}");
            addr
        } else {
            contracts
                .deploy(
                    Contract::LightClientV2,
                    LightClientV2::deploy_builder(&provider)
                        .map(|req| req.with_deploy_code(lcv2_linked_bytecode)),
                )
                .await?
        };
        (pv2_addr, lcv2_addr)
    } else {
        // Use dummy addresses for dry run
        (Address::random(), Address::random())
    };

    // Prepare init data (checks if already initialized)
    // you cannot initialize a proxy that is already initialized
    // so if one wanted to use this function to upgrade a proxy to v2, that's already v2
    // then we shouldn't call the initialize function
    let init_data =
        if crate::already_initialized(&provider, proxy_addr, expected_major_version).await? {
            tracing::info!(
                "Proxy was already initialized for version {}",
                expected_major_version
            );
            vec![].into()
        } else {
            tracing::info!(
                "Init Data to be signed.\n Function: initializeV2\n Arguments:\n \
                 blocks_per_epoch: {:?}\n epoch_start_block: {:?}",
                params.blocks_per_epoch,
                params.epoch_start_block
            );
            LightClientV2::new(lcv2_addr, &provider)
                .initializeV2(params.blocks_per_epoch, params.epoch_start_block)
                .calldata()
                .to_owned()
        };

    // invoke upgrade on proxy via the safeSDK
    let result = call_upgrade_proxy_script(
        proxy_addr,
        lcv2_addr,
        init_data.to_string(),
        rpc_url,
        owner_addr,
        Some(dry_run),
    )
    .await?;

    tracing::info!("Init data: {:?}", init_data);
    if init_data.to_string() != "0x" {
        tracing::info!(
            "Data to be signed:\n Function: initializeV2\n Arguments:\n blocks_per_epoch: {:?}\n \
             epoch_start_block: {:?}",
            params.blocks_per_epoch,
            params.epoch_start_block
        );
    }
    if !dry_run {
        tracing::info!(
                "LightClientProxy upgrade proposal sent. Send this link to the signers to sign the proposal: https://app.safe.global/transactions/queue?safe={}",
                owner_addr
            );
    }
    // IDEA: add a function to wait for the proposal to be executed

    Ok(result)
}

/// Upgrade the light client proxy to use LightClientV3.
/// Internally, first detect existence of proxy, then deploy LCV3, then upgrade and initializeV3.
/// Internal to "deploy LCV3", we deploy PlonkVerifierV3 whose address will be used at LCV3 init time.
/// Assumes:
/// - the proxy is already deployed.
/// - the proxy is owned by a multisig.
/// - the proxy is not yet initialized for V3
///
/// Returns the url link to the upgrade proposal
/// This function can only be called on a real network supported by the safeSDK
pub async fn upgrade_light_client_v3_multisig_owner(
    provider: impl Provider,
    contracts: &mut Contracts,
    is_mock: bool,
    rpc_url: String,
    dry_run: Option<bool>,
) -> Result<String> {
    let expected_major_version: u8 = 3;
    let dry_run = dry_run.unwrap_or_else(|| {
        tracing::warn!("Dry run not specified, defaulting to false");
        false
    });

    let proxy_addr = contracts
        .address(Contract::LightClientProxy)
        .ok_or_else(|| anyhow!("LightClientProxy (multisig owner) not found, can't upgrade"))?;
    tracing::info!("LightClientProxy found at {proxy_addr:#x}");
    let proxy = LightClient::new(proxy_addr, &provider);
    let owner_addr = proxy.owner().call().await?;

    if !dry_run && !crate::is_contract(&provider, owner_addr).await? {
        tracing::error!("Proxy owner is not a contract. Expected: {owner_addr:#x}");
        anyhow::bail!("Proxy owner is not a contract. Expected: {owner_addr:#x}");
    }

    // Prepare addresses
    let (_pv3_addr, lcv3_addr) = if !dry_run {
        // Deploy PlonkVerifierV3.sol (if not already deployed)
        let pv3_addr = contracts
            .deploy(
                Contract::PlonkVerifierV3,
                PlonkVerifierV3::deploy_builder(&provider),
            )
            .await?;

        // then deploy LightClientV3.sol
        let target_lcv3_bytecode = if is_mock {
            LightClientV3Mock::BYTECODE.encode_hex()
        } else {
            LightClientV3::BYTECODE.encode_hex()
        };
        let lcv3_linked_bytecode = {
            match target_lcv3_bytecode
                .matches(LIBRARY_PLACEHOLDER_ADDRESS)
                .count()
            {
                0 => return Err(anyhow!("lib placeholder not found")),
                1 => Bytes::from_hex(target_lcv3_bytecode.replacen(
                    LIBRARY_PLACEHOLDER_ADDRESS,
                    &pv3_addr.encode_hex(),
                    1,
                ))?,
                _ => {
                    return Err(anyhow!(
                        "more than one lib placeholder found, consider using a different value"
                    ))
                },
            }
        };
        let lcv3_addr = if is_mock {
            let addr = LightClientV3Mock::deploy_builder(&provider)
                .map(|req| req.with_deploy_code(lcv3_linked_bytecode))
                .deploy()
                .await?;
            tracing::info!("deployed LightClientV3Mock at {addr:#x}");
            addr
        } else {
            contracts
                .deploy(
                    Contract::LightClientV3,
                    LightClientV3::deploy_builder(&provider)
                        .map(|req| req.with_deploy_code(lcv3_linked_bytecode)),
                )
                .await?
        };
        (pv3_addr, lcv3_addr)
    } else {
        // Use dummy addresses for dry run
        (Address::random(), Address::random())
    };

    // Prepare init data (checks if already initialized)
    // you cannot initialize a proxy that is already initialized
    // so if one wanted to use this function to upgrade a proxy to v3, that's already v3
    // then we shouldn't call the initialize function
    let init_data =
        if crate::already_initialized(&provider, proxy_addr, expected_major_version).await? {
            tracing::info!(
                "Proxy was already initialized for version {}",
                expected_major_version
            );
            vec![].into()
        } else {
            tracing::info!(
                "Init Data to be signed.\n Function: initializeV3\n Arguments: none (V3 inherits \
                 from V2)"
            );
            LightClientV3::new(lcv3_addr, &provider)
                .initializeV3()
                .calldata()
                .to_owned()
        };

    // invoke upgrade on proxy via the safeSDK
    let result = call_upgrade_proxy_script(
        proxy_addr,
        lcv3_addr,
        init_data.to_string(),
        rpc_url,
        owner_addr,
        Some(dry_run),
    )
    .await?;

    tracing::info!("Init data: {:?}", init_data);
    if init_data.to_string() != "0x" {
        tracing::info!(
            "Data to be signed:\n Function: initializeV3\n Arguments: none (V3 inherits from V2)"
        );
    }
    if !dry_run {
        tracing::info!(
                "LightClientProxy upgrade proposal sent. Send this link to the signers to sign the proposal: https://app.safe.global/transactions/queue?safe={}",
                owner_addr
            );
    }
    // IDEA: add a function to wait for the proposal to be executed

    Ok(result)
}

/// Upgrade the EspToken proxy to use EspTokenV2.
/// Internally, first detect existence of proxy, then deploy EspTokenV2, then upgrade and initializeV2.
/// Assumes:
/// - the proxy is already deployed.
/// - the proxy is owned by a multisig.
///
/// Returns the url link to the upgrade proposal
/// This function can only be called on a real network supported by the safeSDK
pub async fn upgrade_esp_token_v2_multisig_owner(
    provider: impl Provider,
    contracts: &mut Contracts,
    rpc_url: String,
    dry_run: Option<bool>,
) -> Result<String> {
    let dry_run = dry_run.unwrap_or_else(|| {
        tracing::warn!("Dry run not specified, defaulting to false");
        false
    });

    let proxy_addr = contracts
        .address(Contract::EspTokenProxy)
        .ok_or_else(|| anyhow!("EspTokenProxy (multisig owner) not found, can't upgrade"))?;
    tracing::info!("EspTokenProxy found at {proxy_addr:#x}");
    let proxy = EspToken::new(proxy_addr, &provider);
    let owner_addr = proxy.owner().call().await?;

    if !dry_run {
        tracing::info!("Checking if owner is a contract");
        assert!(
            crate::is_contract(&provider, owner_addr).await?,
            "Owner is not a contract so not a multisig wallet"
        );
    }

    // Prepare addresses
    let esp_token_v2_addr = if !dry_run {
        contracts
            .deploy(Contract::EspTokenV2, EspTokenV2::deploy_builder(&provider))
            .await?
    } else {
        // Use dummy addresses for dry run
        Address::random()
    };

    let reward_claim_addr = contracts
        .address(Contract::RewardClaimProxy)
        .ok_or_else(|| anyhow!("RewardClaimProxy not found"))?;
    let proxy_as_v2 = EspTokenV2::new(proxy_addr, &provider);
    let init_data = proxy_as_v2
        .initializeV2(reward_claim_addr)
        .calldata()
        .to_owned();

    // invoke upgrade on proxy via the safeSDK
    let result = call_upgrade_proxy_script(
        proxy_addr,
        esp_token_v2_addr,
        init_data.to_string(),
        rpc_url,
        owner_addr,
        Some(dry_run),
    )
    .await?;

    tracing::info!(
        %reward_claim_addr,
        "Data to be signed: Function: initializeV2 Arguments:"
    );

    if !dry_run {
        tracing::info!(
                "EspTokenProxy upgrade proposal sent. Send this link to the signers to \
                 sign the proposal: https://app.safe.global/transactions/queue?safe={}",
                owner_addr
            );
    }

    Ok(result)
}

/// Upgrade the stake table proxy to use StakeTableV2.
/// Internally, first detect existence of proxy, then deploy StakeTableV2
/// Assumes:
/// - the proxy is already deployed.
/// - the proxy is owned by a multisig.
///
/// Returns the url link to the upgrade proposal
/// This function can only be called on a real network supported by the safeSDK
pub async fn upgrade_stake_table_v2_multisig_owner(
    provider: impl Provider,
    contracts: &mut Contracts,
    rpc_url: String,
    multisig_address: Address,
    pauser: Address,
    dry_run: Option<bool>,
) -> Result<()> {
    tracing::info!("Upgrading StakeTableProxy to StakeTableV2 using multisig owner");
    let dry_run = dry_run.unwrap_or(false);
    let Some(proxy_addr) = contracts.address(Contract::StakeTableProxy) else {
        anyhow::bail!("StakeTableProxy not found, can't upgrade")
    };

    let proxy = StakeTable::new(proxy_addr, &provider);
    let owner = proxy.owner().call().await?;
    let owner_addr = owner;

    if owner_addr != multisig_address {
        anyhow::bail!(
            "Proxy not owned by multisig. expected: {multisig_address:#x}, got: {owner_addr:#x}"
        );
    }
    if !dry_run && !crate::is_contract(&provider, owner_addr).await? {
        tracing::error!("Proxy owner is not a contract. Expected: {owner_addr:#x}");
        anyhow::bail!("Proxy owner is not a contract");
    }
    // TODO: check if owner is a SAFE multisig

    let (_init_commissions, init_data) =
        crate::prepare_stake_table_v2_upgrade(&provider, proxy_addr, pauser, owner_addr).await?;

    let stake_table_v2_addr = contracts
        .deploy(
            Contract::StakeTableV2,
            StakeTableV2::deploy_builder(&provider),
        )
        .await?;

    // invoke upgrade on proxy via the safeSDK
    call_upgrade_proxy_script(
        proxy_addr,
        stake_table_v2_addr,
        init_data.unwrap_or_default().to_string(),
        rpc_url,
        owner_addr,
        Some(dry_run),
    )
    .await
    .context("Calling upgrade proxy script failed")?;

    tracing::info!("StakeTableProxy upgrade proposal sent");

    Ok(())
}
