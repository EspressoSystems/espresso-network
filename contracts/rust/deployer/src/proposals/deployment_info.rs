//! Compile-time embedded deployment-info for all supported networks.
//!
//! TOMLs are embedded at compile time so the binary is self-contained and
//! verification never depends on the local filesystem.

use alloy::primitives::Address;
use anyhow::{Result, anyhow};
use serde::Deserialize;

// ── Embedded TOML sources ─────────────────────────────────────────────────────

const DECAF_TOML: &str = include_str!("../../../deployment-info/deployments/decaf.toml");
const HOODI_TOML: &str = include_str!("../../../deployment-info/deployments/hoodi.toml");
const MAINNET_TOML: &str = include_str!("../../../deployment-info/deployments/mainnet.toml");

// ── Raw TOML types ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RawDeploymentInfo {
    ops_timelock: Option<RawTimelockSection>,
    safe_exit_timelock: Option<RawTimelockSection>,
    esp_token: Option<RawContractSection>,
    fee_contract: Option<RawContractSection>,
    reward_claim: Option<RawContractSection>,
    stake_table: Option<RawContractSection>,
}

#[derive(Debug, Deserialize)]
struct RawTimelockSection {
    address: Address,
    proposers: Option<Vec<RawMember>>,
    executors: Option<Vec<RawMember>>,
}

#[derive(Debug, Deserialize)]
struct RawContractSection {
    address: Address,
}

#[derive(Debug, Deserialize)]
struct RawMember {
    address: Address,
    #[allow(dead_code)]
    name: String,
}

// ── Public types ──────────────────────────────────────────────────────────────

/// Resolved timelock wiring for a single timelock (ops or safe_exit).
#[derive(Debug, Clone)]
pub struct TimelockInfo {
    pub address: Address,
    pub proposers: Vec<Address>,
    pub executors: Vec<Address>,
}

/// Full deployment-info for a network.
#[derive(Debug, Clone)]
pub struct DeploymentInfo {
    pub ops_timelock: TimelockInfo,
    pub safe_exit_timelock: TimelockInfo,
    /// ESP token proxy address.
    pub esp_token: Address,
    /// Fee contract proxy address.
    pub fee_contract: Address,
    /// RewardClaim proxy address.
    pub reward_claim: Address,
    /// StakeTable proxy address (used for StakeTableV2 and StakeTableV3).
    pub stake_table: Address,
}

/// Return embedded deployment-info for `network`.
///
/// Known networks: "mainnet", "decaf", "hoodi".
/// Returns `Err` for any other name.
pub fn deployment_info(network: &str) -> Result<DeploymentInfo> {
    let src = match network {
        "mainnet" => MAINNET_TOML,
        "decaf" => DECAF_TOML,
        "hoodi" => HOODI_TOML,
        other => {
            return Err(anyhow!(
                "unknown network {:?}; known: mainnet, decaf, hoodi",
                other
            ));
        },
    };
    parse_deployment_info(src, network)
}

fn parse_deployment_info(src: &str, network: &str) -> Result<DeploymentInfo> {
    let raw: RawDeploymentInfo = toml::from_str(src)
        .map_err(|e| anyhow!("failed to parse deployment-info for {network}: {e}"))?;

    let ops = raw
        .ops_timelock
        .ok_or_else(|| anyhow!("no [ops_timelock] in deployment-info for {network}"))?;
    let safe_exit = raw
        .safe_exit_timelock
        .ok_or_else(|| anyhow!("no [safe_exit_timelock] in deployment-info for {network}"))?;

    let esp_token = raw
        .esp_token
        .ok_or_else(|| anyhow!("no [esp_token] in deployment-info for {network}"))?
        .address;
    let fee_contract = raw
        .fee_contract
        .ok_or_else(|| anyhow!("no [fee_contract] in deployment-info for {network}"))?
        .address;
    let reward_claim = raw
        .reward_claim
        .ok_or_else(|| anyhow!("no [reward_claim] in deployment-info for {network}"))?
        .address;
    let stake_table = raw
        .stake_table
        .ok_or_else(|| anyhow!("no [stake_table] in deployment-info for {network}"))?
        .address;

    Ok(DeploymentInfo {
        ops_timelock: TimelockInfo {
            address: ops.address,
            proposers: ops
                .proposers
                .unwrap_or_default()
                .into_iter()
                .map(|m| m.address)
                .collect(),
            executors: ops
                .executors
                .unwrap_or_default()
                .into_iter()
                .map(|m| m.address)
                .collect(),
        },
        safe_exit_timelock: TimelockInfo {
            address: safe_exit.address,
            proposers: safe_exit
                .proposers
                .unwrap_or_default()
                .into_iter()
                .map(|m| m.address)
                .collect(),
            executors: safe_exit
                .executors
                .unwrap_or_default()
                .into_iter()
                .map(|m| m.address)
                .collect(),
        },
        esp_token,
        fee_contract,
        reward_claim,
        stake_table,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_info_unknown_network_errors() {
        let err = deployment_info("bogusnet").unwrap_err();
        assert!(err.to_string().contains("unknown network"), "{err}");
    }

    #[test]
    fn test_deployment_info_decaf_timelock_addresses() {
        let info = deployment_info("decaf").unwrap();
        let ops_addr: Address = "0x8e3b6563d683b87964104a2c3a4bf542bb70767f"
            .parse()
            .unwrap();
        let safe_exit_addr: Address = "0x0eb0ef3b5a46a444c38da452055bddb273550d5c"
            .parse()
            .unwrap();
        assert_eq!(info.ops_timelock.address, ops_addr);
        assert_eq!(info.safe_exit_timelock.address, safe_exit_addr);
        assert_eq!(info.ops_timelock.proposers.len(), 1);
        assert_eq!(info.ops_timelock.executors.len(), 1);
        assert_eq!(info.safe_exit_timelock.proposers.len(), 1);
        assert_eq!(info.safe_exit_timelock.executors.len(), 1);
    }

    #[test]
    fn test_deployment_info_mainnet_timelock_addresses() {
        let info = deployment_info("mainnet").unwrap();
        let ops_addr: Address = "0x67861f1ef4db9bcaddd8c5e86db92386dd4ec700"
            .parse()
            .unwrap();
        let safe_exit_addr: Address = "0x6e7941fe8f9c751363b5c156419a0c8912dea6b2"
            .parse()
            .unwrap();
        assert_eq!(info.ops_timelock.address, ops_addr);
        assert_eq!(info.safe_exit_timelock.address, safe_exit_addr);
        assert_eq!(info.ops_timelock.proposers.len(), 2);
        assert_eq!(info.ops_timelock.executors.len(), 1);
    }

    #[test]
    fn test_deployment_info_decaf_proposer_address() {
        let info = deployment_info("decaf").unwrap();
        let espresso_labs: Address = "0xb76834e371b666feee48e5d7d9a97ca08b5a0620"
            .parse()
            .unwrap();
        assert_eq!(info.ops_timelock.proposers[0], espresso_labs);
        assert_eq!(info.ops_timelock.executors[0], espresso_labs);
    }

    #[test]
    fn test_deployment_info_mainnet_multiple_proposers() {
        let info = deployment_info("mainnet").unwrap();
        assert_eq!(info.ops_timelock.proposers.len(), 2);
        assert_eq!(info.ops_timelock.executors.len(), 1);
    }
}
