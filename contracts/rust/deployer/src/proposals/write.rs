//! Helpers for writing a proposal directory under the `contracts/deployments/proposals/` convention.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use alloy::{
    primitives::{Address, B256, Bytes, U256},
    providers::Provider,
    sol,
};
use anyhow::{Context, Result};
use chrono::Local;

use crate::proposals::{
    deployment_info::load_ops_timelock_signers,
    safe_hash::{SafeTxHashes, safe_tx_hashes},
};

const README_TEMPLATE: &str = include_str!("templates/proposal_readme.md");

sol! {
    #[sol(rpc)]
    interface ISafe {
        function nonce() external view returns (uint256);
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

/// Run `git rev-parse --short HEAD` in `repo_dir`; fall back to `"unknown"`.
pub fn git_short_hash(repo_dir: &Path) -> String {
    let hash = Command::new("git")
        .args([
            "-C",
            &repo_dir.to_string_lossy(),
            "rev-parse",
            "--short",
            "HEAD",
        ])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|| "unknown".to_owned());
    if hash == "unknown" {
        tracing::warn!(
            "git rev-parse failed; source commit in README will be recorded as \"unknown\""
        );
    }
    hash
}

fn network_label(network: &str) -> String {
    match network {
        "mainnet" => "Espresso Mainnet".to_owned(),
        "decaf" => "Decaf Testnet".to_owned(),
        other => format!("{other} network"),
    }
}

/// Resolved Safe hashes for both timelock phases.
#[derive(Debug, Clone)]
pub struct ResolvedSafeHashes {
    pub schedule_safe: Address,
    pub schedule_nonce: u64,
    pub schedule: SafeTxHashes,
    pub execute_safe: Address,
    pub execute_nonce: u64,
    pub execute: SafeTxHashes,
}

/// Fill the README template with proposal-specific values.
///
/// `params` supplies all chain/contract data; `dir` is the final output directory
/// (used to build the `--input` paths in the verify command).
/// `source_commit` is the short git hash recorded at generation time.
/// `safe_hashes` is `Some` when auto-fill succeeded, `None` when skipped.
fn render_stake_table_v3_readme(
    params: &WriteProposalParams,
    dir: &Path,
    source_commit: &str,
    safe_hashes: Option<&ResolvedSafeHashes>,
) -> String {
    let schedule_path = dir.join("schedule.json").display().to_string();
    let execute_path = dir.join("execute.json").display().to_string();

    let schedule_safe = safe_hashes
        .map(|h| format!("{:#x}", h.schedule_safe))
        .unwrap_or_else(|| "<SAFE_ADDRESS>".to_owned());
    let schedule_nonce = safe_hashes
        .map(|h| h.schedule_nonce.to_string())
        .unwrap_or_else(|| "<NONCE>".to_owned());

    README_TEMPLATE
        .replace("{{NETWORK_LABEL}}", &network_label(&params.network))
        .replace("{{NETWORK}}", &params.network)
        .replace("{{CHAIN_ID}}", &params.chain_id.to_string())
        .replace("{{SOURCE_COMMIT}}", source_commit)
        .replace("{{PROXY}}", &format!("{:#x}", params.proxy))
        .replace("{{IMPL}}", &format!("{:#x}", params.new_impl))
        .replace("{{TIMELOCK}}", &format!("{:#x}", params.timelock))
        .replace("{{SALT}}", &params.salt.to_string())
        .replace("{{DELAY}}", &params.delay.to_string())
        .replace("{{SCHEDULE_PATH}}", &schedule_path)
        .replace("{{EXECUTE_PATH}}", &execute_path)
        .replace("{{SCHEDULE_SAFE}}", &schedule_safe)
        .replace("{{SCHEDULE_NONCE}}", &schedule_nonce)
        .replace(
            "{{SCHEDULE_DOMAIN}}",
            &safe_hashes
                .map(|h| h.schedule.domain.to_string())
                .unwrap_or_else(|| "<run verify-proposal to compute>".to_owned()),
        )
        .replace(
            "{{SCHEDULE_MESSAGE}}",
            &safe_hashes
                .map(|h| h.schedule.message.to_string())
                .unwrap_or_else(|| "<run verify-proposal to compute>".to_owned()),
        )
        .replace(
            "{{SCHEDULE_SAFE_TX}}",
            &safe_hashes
                .map(|h| h.schedule.safe_tx.to_string())
                .unwrap_or_else(|| "<run verify-proposal to compute>".to_owned()),
        )
        .replace(
            "{{EXECUTE_SAFE}}",
            &safe_hashes
                .map(|h| format!("{:#x}", h.execute_safe))
                .unwrap_or_else(|| "<SAFE_ADDRESS>".to_owned()),
        )
        .replace(
            "{{EXECUTE_NONCE}}",
            &safe_hashes
                .map(|h| h.execute_nonce.to_string())
                .unwrap_or_else(|| "<NONCE+1>".to_owned()),
        )
        .replace(
            "{{EXECUTE_DOMAIN}}",
            &safe_hashes
                .map(|h| h.execute.domain.to_string())
                .unwrap_or_else(|| "<run verify-proposal to compute>".to_owned()),
        )
        .replace(
            "{{EXECUTE_MESSAGE}}",
            &safe_hashes
                .map(|h| h.execute.message.to_string())
                .unwrap_or_else(|| "<run verify-proposal to compute>".to_owned()),
        )
        .replace(
            "{{EXECUTE_SAFE_TX}}",
            &safe_hashes
                .map(|h| h.execute.safe_tx.to_string())
                .unwrap_or_else(|| "<run verify-proposal to compute>".to_owned()),
        )
        .replace("{{VERIFY_SAFE}}", &schedule_safe)
        .replace("{{VERIFY_NONCE}}", &schedule_nonce)
}

pub fn write_proposal_contract_file(dir: &Path, kind_kebab: &str) -> Result<()> {
    std::fs::write(dir.join("contract"), format!("{kind_kebab}\n"))
        .with_context(|| format!("failed to write contract file in {}", dir.display()))
}

pub fn write_proposal_readme(dir: &Path, readme: &str) -> Result<()> {
    std::fs::write(dir.join("README.md"), readme)
        .with_context(|| format!("failed to write README.md in {}", dir.display()))
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
    /// Repo root for `git rev-parse`.
    pub repo_dir: PathBuf,
    /// Directory containing `<network>.toml` deployment-info files.
    /// Defaults to `contracts/rust/deployment-info/deployments`.
    pub deployment_info_dir: PathBuf,
    /// Outer schedule calldata (timelock.schedule(...)) for hash computation.
    pub schedule_calldata: Bytes,
    /// Outer execute calldata (timelock.execute(...)) for hash computation.
    pub execute_calldata: Bytes,
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

/// Resolve the signing Safes from deployment-info and query their nonces.
///
/// Returns `None` (with a warning) when the signer set is ambiguous (0 or >1).
async fn resolve_safe_hashes(
    provider: &impl Provider,
    params: &WriteProposalParams,
) -> Option<ResolvedSafeHashes> {
    let signers = match load_ops_timelock_signers(&params.network, &params.deployment_info_dir) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                "skipping Safe hash auto-fill: deployment-info unavailable ({}); run \
                 verify-proposal with --safe/--nonce",
                e
            );
            return None;
        },
    };

    if signers.proposers.len() != 1 || signers.executors.len() != 1 {
        tracing::warn!(
            proposers = signers.proposers.len(),
            executors = signers.executors.len(),
            "skipping Safe hash auto-fill: ambiguous signer set (proposers={}, executors={}); run \
             verify-proposal with --safe/--nonce",
            signers.proposers.len(),
            signers.executors.len(),
        );
        return None;
    }

    let schedule_safe = signers.proposers[0];
    let execute_safe = signers.executors[0];

    let schedule_nonce = match safe_nonce(provider, schedule_safe).await {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(
                "skipping Safe hash auto-fill: could not query nonce for schedule Safe \
                 {schedule_safe:#x}: {e}"
            );
            return None;
        },
    };

    let execute_nonce = if execute_safe == schedule_safe {
        schedule_nonce + 1
    } else {
        match safe_nonce(provider, execute_safe).await {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!(
                    "skipping Safe hash auto-fill: could not query nonce for execute Safe \
                     {execute_safe:#x}: {e}"
                );
                return None;
            },
        }
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

    Some(ResolvedSafeHashes {
        schedule_safe,
        schedule_nonce,
        schedule: schedule_hashes,
        execute_safe,
        execute_nonce,
        execute: execute_hashes,
    })
}

/// Create the proposal directory, write all convention files, and return the directory path.
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

    let commit = git_short_hash(&params.repo_dir);
    let safe_hashes = resolve_safe_hashes(provider, &params).await;
    let readme = render_stake_table_v3_readme(&params, &dir, &commit, safe_hashes.as_ref());

    write_proposal_contract_file(&dir, "stake-table-v3")?;
    write_proposal_readme(&dir, &readme)?;

    Ok(dir)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use alloy::{
        node_bindings::Anvil,
        providers::{ProviderBuilder, ext::AnvilApi},
    };

    use super::*;

    #[test]
    fn test_network_name_known() {
        assert_eq!(network_name(1), Some("mainnet".to_owned()));
        assert_eq!(network_name(11155111), Some("decaf".to_owned()));
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

    fn make_params(network: &str, chain_id: u64) -> WriteProposalParams {
        WriteProposalParams {
            proposals_root: PathBuf::from("/tmp/proposals"),
            network: network.to_owned(),
            slug: "stake-table-v3".to_owned(),
            chain_id,
            proxy: Address::from_str("0x40304FbE94D5E7D1492Dd90c53a2D63E8506a037").unwrap(),
            new_impl: Address::from_str("0x5a6250dd35d875c0529573d9d934629a1b2778db").unwrap(),
            timelock: Address::from_str("0x8e3b6563D683b87964104A2c3A4bf542bb70767F").unwrap(),
            salt: B256::ZERO,
            delay: U256::from(300u64),
            repo_dir: PathBuf::from("."),
            deployment_info_dir: PathBuf::from("contracts/rust/deployment-info/deployments"),
            schedule_calldata: Bytes::new(),
            execute_calldata: Bytes::new(),
        }
    }

    #[test]
    fn test_render_readme_contains_fields() {
        let dir = PathBuf::from("contracts/deployments/proposals/decaf/20260702-stake-table-v3");
        let params = make_params("decaf", 11155111);
        let readme = render_stake_table_v3_readme(&params, &dir, "abc1234", None);
        assert!(readme.contains("11155111"));
        assert!(readme.contains("abc1234"));
        assert!(readme.contains("stake-table-v3"));
        assert!(readme.contains("300 seconds"));
        assert!(readme.contains("schedule.json"));
        assert!(readme.contains("execute.json"));
        assert!(readme.contains("--nonce"));
    }

    #[test]
    fn test_render_readme_no_unresolved_placeholders() {
        let dir = PathBuf::from("/tmp/test-proposal");
        let mut params = make_params("mainnet", 1);
        params.delay = U256::from(86400u64);
        let readme = render_stake_table_v3_readme(&params, &dir, "deadbeef", None);
        assert!(!readme.contains("{{"), "unresolved placeholder in README");
    }

    #[test]
    fn test_render_readme_with_safe_hashes() {
        let dir = PathBuf::from("/tmp/test-proposal");
        let params = make_params("decaf", 11155111);
        let safe: Address = "0xb76834e371b666feee48e5d7d9a97ca08b5a0620"
            .parse()
            .unwrap();
        let hashes = safe_tx_hashes(
            safe,
            11155111,
            params.timelock,
            U256::ZERO,
            &Bytes::new(),
            0,
            5,
        );
        let resolved = ResolvedSafeHashes {
            schedule_safe: safe,
            schedule_nonce: 5,
            schedule: hashes.clone(),
            execute_safe: safe,
            execute_nonce: 6,
            execute: hashes,
        };
        let readme = render_stake_table_v3_readme(&params, &dir, "abc", Some(&resolved));
        assert!(!readme.contains("{{"), "unresolved placeholder in README");
        assert!(readme.contains("0xb76834"));
        assert!(readme.contains("nonce=5"));
        assert!(readme.contains("nonce=6"));
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

    #[tokio::test]
    async fn test_write_proposal_dir_rejects_existing() {
        let anvil = Anvil::new().spawn();
        let provider = ProviderBuilder::new().connect_http(anvil.endpoint_url());

        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let mut params = make_params("decaf", 11155111);
        params.proposals_root = root.clone();
        // Point deployment_info_dir at something that doesn't exist so auto-fill is skipped.
        params.deployment_info_dir = PathBuf::from("/nonexistent");

        // First write succeeds.
        let dir = write_stake_table_v3_proposal_dir(params, &provider)
            .await
            .unwrap();

        // Plant a schedule.json to simulate an existing proposal.
        std::fs::write(dir.join("schedule.json"), "{}").unwrap();

        // Second write with the same params must fail.
        let mut params2 = make_params("decaf", 11155111);
        params2.proposals_root = root;
        params2.deployment_info_dir = PathBuf::from("/nonexistent");
        let err = write_stake_table_v3_proposal_dir(params2, &provider)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("already exists"),
            "unexpected error: {err}"
        );
    }
}
