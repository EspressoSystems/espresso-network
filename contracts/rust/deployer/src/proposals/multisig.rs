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
    EspToken, EspTokenV2, LightClient, LightClientV2, LightClientV2Mock, OwnableUpgradeable,
    PlonkVerifierV2, StakeTable, StakeTableV2,
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
/// - A tuple of (stdout, success) from the script execution
pub async fn call_upgrade_proxy_script(
    proxy_addr: Address,
    new_impl_addr: Address,
    init_data: String,
    rpc_url: String,
    safe_addr: Address,
    dry_run: Option<bool>,
) -> Result<(String, bool), anyhow::Error> {
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
        return Err(anyhow!("Upgrade script failed: {}", stderr));
    }
    Ok((stdout.to_string(), true))
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
) -> Result<(String, bool)> {
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
    let owner_addr = proxy.owner().call().await?._0;

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

    // Prepare init data
    let init_data = if crate::already_initialized(
        &provider,
        proxy_addr,
        Contract::LightClientV2,
        expected_major_version,
    )
    .await?
    {
        tracing::info!(
            "Proxy was already initialized for version {}",
            expected_major_version
        );
        vec![].into()
    } else {
        tracing::info!(
            "Init Data to be signed.\n Function: initializeV2\n Arguments:\n blocks_per_epoch: \
             {:?}\n epoch_start_block: {:?}",
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
) -> Result<(String, bool)> {
    let dry_run = dry_run.unwrap_or_else(|| {
        tracing::warn!("Dry run not specified, defaulting to false");
        false
    });

    let proxy_addr = contracts
        .address(Contract::EspTokenProxy)
        .ok_or_else(|| anyhow!("EspTokenProxy (multisig owner) not found, can't upgrade"))?;
    tracing::info!("EspTokenProxy found at {proxy_addr:#x}");
    let proxy = EspToken::new(proxy_addr, &provider);
    let owner_addr = proxy.owner().call().await?._0;

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

    // no reinitializer so empty init data
    let init_data = "0x".to_string();

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

    tracing::info!("No init data to be signed");
    if !dry_run {
        tracing::info!(
                "EspTokenProxy upgrade proposal sent. Send this link to the signers to sign the proposal: https://app.safe.global/transactions/queue?safe={}",
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
) -> Result<(String, bool)> {
    tracing::info!("Upgrading StakeTableProxy to StakeTableV2 using multisig owner");
    let dry_run = dry_run.unwrap_or(false);
    match contracts.address(Contract::StakeTableProxy) {
        // check if proxy already exists
        None => Err(anyhow!("StakeTableProxy not found, can't upgrade")),
        Some(proxy_addr) => {
            let proxy = StakeTable::new(proxy_addr, &provider);
            let owner = proxy.owner().call().await?;
            let owner_addr = owner._0;
            if owner_addr != multisig_address {
                tracing::error!(
                    "Proxy is not owned by the multisig. Expected: {multisig_address:#x}, Got: \
                     {owner_addr:#x}"
                );
                anyhow::bail!("Proxy is not owned by the multisig");
            }
            if !dry_run && !crate::is_contract(&provider, owner_addr).await? {
                tracing::error!("Proxy owner is not a contract. Expected: {owner_addr:#x}");
                anyhow::bail!("Proxy owner is not a contract");
            }
            // TODO: check if owner is a SAFE multisig

            // first deploy StakeTableV2.sol implementation
            let stake_table_v2_addr = contracts
                .deploy(
                    Contract::StakeTableV2,
                    StakeTableV2::deploy_builder(&provider),
                )
                .await?;

            // prepare init data
            let expected_major_version = 2;
            let init_data = if crate::already_initialized(
                &provider,
                proxy_addr,
                Contract::StakeTableV2,
                expected_major_version,
            )
            .await?
            {
                tracing::info!(
                    "Proxy was already initialized for version {}",
                    expected_major_version
                );
                vec![].into()
            } else {
                tracing::info!(
                    "Init Data to be signed.\n Function: initializeV2\n Arguments:\n pauser: \
                     {:?}\n admin: {:?}",
                    pauser,
                    owner_addr
                );
                StakeTableV2::new(stake_table_v2_addr, &provider)
                    .initializeV2(pauser, owner_addr)
                    .calldata()
                    .to_owned()
            };

            // invoke upgrade on proxy via the safeSDK
            let result = call_upgrade_proxy_script(
                proxy_addr,
                stake_table_v2_addr,
                init_data.to_string(),
                rpc_url,
                owner_addr,
                Some(dry_run),
            )
            .await;

            // Check the result directly
            if let Err(ref err) = result {
                tracing::error!("StakeTableProxy upgrade failed: {:?}", err);
            } else {
                tracing::info!("StakeTableProxy upgrade proposal sent");
                // IDEA: add a function to wait for the proposal to be executed
            }
            Ok(result.context("Upgrade proposal failed")?)
        },
    }
}
