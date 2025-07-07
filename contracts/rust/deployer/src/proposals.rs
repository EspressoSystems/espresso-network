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
