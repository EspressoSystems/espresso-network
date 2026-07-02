//! Minimal parser for `contracts/rust/deployment-info/deployments/<network>.toml`.
//!
//! Only parses the ops_timelock proposers and executors needed for Safe hash auto-fill.

use std::path::{Path, PathBuf};

use alloy::primitives::Address;
use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RawDeploymentInfo {
    ops_timelock: Option<RawTimelockSection>,
}

#[derive(Debug, Deserialize)]
struct RawTimelockSection {
    proposers: Option<Vec<RawMember>>,
    executors: Option<Vec<RawMember>>,
}

#[derive(Debug, Deserialize)]
struct RawMember {
    address: Address,
    #[allow(dead_code)]
    name: String,
}

/// Resolved proposer/executor Safe addresses for the ops_timelock.
#[derive(Debug, Clone)]
pub struct OpsTimelockSigners {
    pub proposers: Vec<Address>,
    pub executors: Vec<Address>,
}

/// Load and parse the deployment-info TOML for a network.
///
/// `deployment_info_dir` defaults to `contracts/rust/deployment-info/deployments`.
pub fn load_ops_timelock_signers(
    network: &str,
    deployment_info_dir: &Path,
) -> Result<OpsTimelockSigners> {
    let path = deployment_info_dir.join(format!("{network}.toml"));
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("deployment-info not found: {}", path.display()))?;
    let raw: RawDeploymentInfo =
        toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))?;

    let section = raw
        .ops_timelock
        .ok_or_else(|| anyhow::anyhow!("no [ops_timelock] section in {}", path.display()))?;

    Ok(OpsTimelockSigners {
        proposers: section
            .proposers
            .unwrap_or_default()
            .into_iter()
            .map(|m| m.address)
            .collect(),
        executors: section
            .executors
            .unwrap_or_default()
            .into_iter()
            .map(|m| m.address)
            .collect(),
    })
}

/// Default deployment-info directory (relative to repo root).
pub fn default_deployment_info_dir() -> PathBuf {
    PathBuf::from("contracts/rust/deployment-info/deployments")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_decaf_signers() {
        let dir = PathBuf::from("contracts/rust/deployment-info/deployments");
        if !dir.exists() {
            return;
        }
        let signers = load_ops_timelock_signers("decaf", &dir).unwrap();
        assert_eq!(signers.proposers.len(), 1);
        assert_eq!(signers.executors.len(), 1);
        let espresso_labs: Address = "0xb76834e371b666feee48e5d7d9a97ca08b5a0620"
            .parse()
            .unwrap();
        assert_eq!(signers.proposers[0], espresso_labs);
        assert_eq!(signers.executors[0], espresso_labs);
    }

    #[test]
    fn test_load_mainnet_signers_multiple() {
        let dir = PathBuf::from("contracts/rust/deployment-info/deployments");
        if !dir.exists() {
            return;
        }
        let signers = load_ops_timelock_signers("mainnet", &dir).unwrap();
        // mainnet has 2 proposers, so auto-fill should be skipped
        assert_eq!(signers.proposers.len(), 2);
        assert_eq!(signers.executors.len(), 1);
    }
}
