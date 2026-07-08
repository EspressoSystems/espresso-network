//! Helpers for writing a proposal directory under the `contracts/deployments/proposals/` convention.

use std::path::PathBuf;

use alloy::{
    primitives::{Address, B256, Bytes, U256},
    providers::Provider,
    sol,
};
use anyhow::{Context, Result};
use chrono::Local;
use url::Url;

use crate::proposals::{
    deployment_info::deployment_info,
    proposal_toml::{PhaseToml, ProposalToml},
    safe_hash::safe_tx_hashes,
};

sol! {
    #[sol(rpc)]
    interface ISafe {
        function nonce() external view returns (uint256);
    }
}

/// Map well-known chain IDs to their public RPC endpoints.
///
/// Returns `None` for unknown chain IDs; callers must require `--rpc-url` in that case.
pub fn default_rpc_url(chain_id: u64) -> Option<Url> {
    match chain_id {
        1 => Some(
            "https://ethereum-rpc.publicnode.com"
                .parse()
                .expect("static URL"),
        ),
        11155111 => Some(
            "https://ethereum-sepolia-rpc.publicnode.com"
                .parse()
                .expect("static URL"),
        ),
        560048 => Some(
            "https://ethereum-hoodi-rpc.publicnode.com"
                .parse()
                .expect("static URL"),
        ),
        _ => None,
    }
}

/// Reject path components that could escape the proposals root.
///
/// Disallows empty strings, `/`, `\`, and the `..` component.
pub fn validate_path_component(value: &str, label: &str) -> Result<()> {
    if value.is_empty() {
        anyhow::bail!("{label} must not be empty");
    }
    if value.contains('/') || value.contains('\\') {
        anyhow::bail!("{label} must not contain '/' or '\\': {value:?}");
    }
    if value == ".." || value.starts_with("../") || value.ends_with("/..") {
        anyhow::bail!("{label} must not be '..': {value:?}");
    }
    Ok(())
}

/// Map well-known chain IDs to their network names.
pub fn network_name(chain_id: u64) -> Option<String> {
    match chain_id {
        1 => Some("mainnet".to_owned()),
        11155111 => Some("decaf".to_owned()),
        560048 => Some("hoodi".to_owned()),
        _ => None,
    }
}

/// Resolve network name: explicit override, then chain-id map, else error.
pub fn resolve_network(chain_id: u64, override_name: Option<String>) -> Result<String> {
    if let Some(name) = override_name {
        validate_path_component(&name, "--network")?;
        return Ok(name);
    }
    network_name(chain_id)
        .ok_or_else(|| anyhow::anyhow!("unknown chain id {chain_id}; pass --network"))
}

/// Compute the convention directory:
///   `<proposals_root>/<network>/<YYYYMMDD>-<slug>/`
pub fn proposal_dir(proposals_root: PathBuf, network: &str, slug: &str) -> PathBuf {
    let date = Local::now().format("%Y%m%d").to_string();
    proposals_root.join(network).join(format!("{date}-{slug}"))
}

/// Parameters for writing the full StakeTableV3 proposal directory.
pub struct WriteProposalParams {
    /// Root of the `contracts/deployments/proposals/` tree (defaults to CWD-relative).
    pub proposals_root: PathBuf,
    /// Resolved network name (e.g. "decaf", "mainnet").
    pub network: String,
    /// Slug used in the directory name (e.g. "stake-table-v3").
    pub slug: String,
    pub chain_id: u64,
    pub proxy: Address,
    pub new_impl: Address,
    pub timelock: Address,
    pub salt: B256,
    pub delay: U256,
    /// Outer schedule calldata (timelock.schedule(...)) for hash computation.
    pub schedule_calldata: Bytes,
    /// Outer execute calldata (timelock.execute(...)) for hash computation.
    pub execute_calldata: Bytes,
    /// Optional Safe address override (bypasses deployment-info resolution).
    pub safe_override: Option<Address>,
}

/// Query the Safe nonce on-chain.
async fn safe_nonce(provider: &impl Provider, safe: Address) -> Result<u64> {
    let n = ISafe::new(safe, provider)
        .nonce()
        .call()
        .await
        .with_context(|| format!("failed to query nonce() on Safe {safe:#x}"))?;
    n.try_into().context("Safe nonce overflows u64")
}

/// Resolve schedule/execute Safe addresses and nonces, then compute all hashes.
///
/// Fails loudly if the signer set is ambiguous or the nonce query fails; a
/// partial proposal.toml would be unverifiable and must not be written.
async fn resolve_toml_phases(
    provider: &impl Provider,
    params: &WriteProposalParams,
) -> Result<(PhaseToml, PhaseToml)> {
    let (schedule_safe, execute_safe) = if let Some(safe) = params.safe_override {
        (safe, safe)
    } else {
        let info = deployment_info(&params.network).with_context(|| {
            format!(
                "deployment-info unavailable for network {:?}",
                params.network
            )
        })?;
        let signers = &info.ops_timelock;
        if signers.proposers.len() != 1 || signers.executors.len() != 1 {
            anyhow::bail!(
                "ambiguous signer set for network {:?}: {} proposer(s), {} executor(s); pass \
                 --safe to override",
                params.network,
                signers.proposers.len(),
                signers.executors.len(),
            );
        }
        (signers.proposers[0], signers.executors[0])
    };

    let schedule_nonce = safe_nonce(provider, schedule_safe).await?;
    let execute_nonce = if execute_safe == schedule_safe {
        schedule_nonce + 1
    } else {
        safe_nonce(provider, execute_safe).await?
    };

    let schedule_hashes = safe_tx_hashes(
        schedule_safe,
        params.chain_id,
        params.timelock,
        U256::ZERO,
        &params.schedule_calldata,
        0,
        schedule_nonce,
    );
    let execute_hashes = safe_tx_hashes(
        execute_safe,
        params.chain_id,
        params.timelock,
        U256::ZERO,
        &params.execute_calldata,
        0,
        execute_nonce,
    );

    let schedule_phase = PhaseToml {
        safe: schedule_safe,
        nonce: schedule_nonce,
        domain: schedule_hashes.domain,
        message: schedule_hashes.message,
        safe_tx: schedule_hashes.safe_tx,
    };
    let execute_phase = PhaseToml {
        safe: execute_safe,
        nonce: execute_nonce,
        domain: execute_hashes.domain,
        message: execute_hashes.message,
        safe_tx: execute_hashes.safe_tx,
    };
    Ok((schedule_phase, execute_phase))
}

/// Create the proposal directory, write `schedule.json`, `execute.json`, and `proposal.toml`.
///
/// Fails if the Safe set is ambiguous or any nonce query fails; no partial output.
pub async fn write_stake_table_v3_proposal_dir(
    params: WriteProposalParams,
    provider: &impl Provider,
) -> Result<PathBuf> {
    validate_path_component(&params.network, "--network")?;
    validate_path_component(&params.slug, "--proposal-slug")?;

    let dir = proposal_dir(params.proposals_root.clone(), &params.network, &params.slug);

    if dir.join("schedule.json").exists() {
        anyhow::bail!(
            "proposal dir {} already exists; remove it or pass a different --proposal-slug",
            dir.display()
        );
    }

    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create proposal dir {}", dir.display()))?;

    let (schedule_phase, execute_phase) = resolve_toml_phases(provider, &params).await?;

    let delay_u64: u64 = params.delay.try_into().context("delay overflows u64")?;

    let toml = ProposalToml {
        contract: "stake-table-v3".to_owned(),
        network: params.network.clone(),
        chain_id: params.chain_id,
        proxy: params.proxy,
        new_impl: params.new_impl,
        timelock: params.timelock,
        salt: params.salt,
        delay: delay_u64,
        predecessor: B256::ZERO,
        schedule: schedule_phase,
        execute: execute_phase,
    };
    toml.write(&dir)?;

    Ok(dir)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use alloy::{node_bindings::Anvil, providers::ProviderBuilder};

    use super::*;
    use crate::proposals::proposal_toml::ProposalToml;

    #[test]
    fn test_network_name_known() {
        assert_eq!(network_name(1), Some("mainnet".to_owned()));
        assert_eq!(network_name(11155111), Some("decaf".to_owned()));
        assert_eq!(network_name(560048), Some("hoodi".to_owned()));
    }

    #[test]
    fn test_network_name_unknown() {
        assert_eq!(network_name(9999), None);
    }

    #[test]
    fn test_resolve_network_override() {
        let result = resolve_network(9999, Some("my-network".to_owned())).unwrap();
        assert_eq!(result, "my-network");
    }

    #[test]
    fn test_resolve_network_from_chain_id() {
        assert_eq!(resolve_network(1, None).unwrap(), "mainnet");
        assert_eq!(resolve_network(11155111, None).unwrap(), "decaf");
    }

    #[test]
    fn test_resolve_network_unknown_errors() {
        let err = resolve_network(9999, None).unwrap_err();
        assert!(err.to_string().contains("unknown chain id 9999"));
        assert!(err.to_string().contains("--network"));
    }

    #[test]
    fn test_proposal_dir_format() {
        let root = PathBuf::from("/tmp/proposals");
        let dir = proposal_dir(root, "decaf", "stake-table-v3");
        let s = dir.to_string_lossy();
        assert!(s.contains("/tmp/proposals/decaf/"));
        assert!(s.contains("-stake-table-v3"));
    }

    #[test]
    fn test_validate_path_component_ok() {
        assert!(validate_path_component("mainnet", "--network").is_ok());
        assert!(validate_path_component("stake-table-v3", "--proposal-slug").is_ok());
    }

    #[test]
    fn test_validate_path_component_rejects_slash() {
        assert!(validate_path_component("foo/bar", "--network").is_err());
        assert!(validate_path_component("foo\\bar", "--network").is_err());
    }

    #[test]
    fn test_validate_path_component_rejects_dotdot() {
        assert!(validate_path_component("..", "--network").is_err());
    }

    #[test]
    fn test_validate_path_component_rejects_empty() {
        assert!(validate_path_component("", "--network").is_err());
    }

    #[test]
    fn test_resolve_network_rejects_traversal() {
        let err = resolve_network(9999, Some("../evil".to_owned())).unwrap_err();
        assert!(err.to_string().contains("--network"));
    }

    /// proposal.toml round-trip: serialize then deserialize recovers identical struct.
    #[test]
    fn test_proposal_toml_round_trip() {
        let safe: Address = "0xb76834e371b666feee48e5d7d9a97ca08b5a0620"
            .parse()
            .unwrap();
        let domain: alloy::primitives::B256 =
            "0x8f560c9d209e6d9320305560aee98fa1dea01510aa5451a9c0911401893835c6"
                .parse()
                .unwrap();
        let original = ProposalToml {
            contract: "stake-table-v3".to_owned(),
            network: "decaf".to_owned(),
            chain_id: 11155111,
            proxy: "0x40304FbE94D5E7D1492Dd90c53a2D63E8506a037"
                .parse()
                .unwrap(),
            new_impl: "0x5a6250dd35d875c0529573d9d934629a1b2778db"
                .parse()
                .unwrap(),
            timelock: "0x8e3b6563D683b87964104A2c3A4bf542bb70767F"
                .parse()
                .unwrap(),
            salt: B256::repeat_byte(0x99),
            delay: 300,
            predecessor: B256::ZERO,
            schedule: crate::proposals::proposal_toml::PhaseToml {
                safe,
                nonce: 24,
                domain,
                message: "0x9c5a62271d73b6accf3c8957a1e80b6434618d3bd4b8bd23e30817479c60d35b"
                    .parse()
                    .unwrap(),
                safe_tx: "0xa3d4b5bfa93b559f34478b3988f1132c35ba67f953a87326c8a1c8250709c6b8"
                    .parse()
                    .unwrap(),
            },
            execute: crate::proposals::proposal_toml::PhaseToml {
                safe,
                nonce: 25,
                domain,
                message: "0xf7edebe09a94e770ddbccf107a5685d50d902adb08db5e2043c7b1f9c4ef648b"
                    .parse()
                    .unwrap(),
                safe_tx: "0xbb7fd662e5b724a50e33f18ef737d6df9c1d92b8810def16fb190b7c27c16f45"
                    .parse()
                    .unwrap(),
            },
        };

        let tmp = tempfile::tempdir().unwrap();
        original.write(tmp.path()).unwrap();

        let loaded = ProposalToml::load(tmp.path()).unwrap();
        assert_eq!(original, loaded);
    }

    /// The "already exists" guard fires before any on-chain call.
    #[tokio::test]
    async fn test_write_proposal_dir_rejects_existing() {
        let anvil = Anvil::new().spawn();
        let provider = ProviderBuilder::new().connect_http(anvil.endpoint_url());

        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let safe: Address =
            Address::from_str("0xb76834e371b666feee48e5d7d9a97ca08b5a0620").unwrap();

        let make_params = |root: PathBuf| WriteProposalParams {
            proposals_root: root,
            network: "decaf".to_owned(),
            slug: "stake-table-v3".to_owned(),
            chain_id: 11155111,
            proxy: Address::from_str("0x40304FbE94D5E7D1492Dd90c53a2D63E8506a037").unwrap(),
            new_impl: Address::from_str("0x5a6250dd35d875c0529573d9d934629a1b2778db").unwrap(),
            timelock: Address::from_str("0x8e3b6563D683b87964104A2c3A4bf542bb70767F").unwrap(),
            salt: B256::repeat_byte(0x11),
            delay: U256::from(300u64),
            schedule_calldata: Bytes::new(),
            execute_calldata: Bytes::new(),
            safe_override: Some(safe),
        };

        // Pre-create the dir and plant a schedule.json so the guard fires immediately.
        let dir = proposal_dir(root.clone(), "decaf", "stake-table-v3");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("schedule.json"), "{}").unwrap();

        let err = write_stake_table_v3_proposal_dir(make_params(root), &provider)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("already exists"),
            "unexpected error: {err}"
        );
    }
}
