use std::{collections::HashMap, path::Path};

use alloy::primitives::Address;
use anyhow::{Context, Result, bail};

const STAKE_TABLE_PROXY_ADDRESS: &str = "ESPRESSO_STAKE_TABLE_PROXY_ADDRESS";
const ESP_TOKEN_PROXY_ADDRESS: &str = "ESP_TOKEN_PROXY_ADDRESS";
const LIGHT_CLIENT_PROXY_ADDRESS: &str = "ESPRESSO_LIGHT_CLIENT_PROXY_ADDRESS";
const FEE_CONTRACT_PROXY_ADDRESS: &str = "ESPRESSO_FEE_CONTRACT_PROXY_ADDRESS";
const REWARD_CLAIM_PROXY_ADDRESS: &str = "ESPRESSO_REWARD_CLAIM_PROXY_ADDRESS";
const MULTISIG_PREFIX: &str = "ESPRESSO_MULTISIG_";
const LEGACY_MULTISIG_PREFIX: &str = "ESPRESSO_SEQUENCER_MULTISIG_";
const MULTISIG_SUFFIX: &str = "_ADDRESS";
const OPS_TIMELOCK_ADDRESS: &str = "ESPRESSO_OPS_TIMELOCK_ADDRESS";
const SAFE_EXIT_TIMELOCK_ADDRESS: &str = "ESPRESSO_SAFE_EXIT_TIMELOCK_ADDRESS";

/// Contract and governance addresses read from a per-network .env file.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct DeploymentAddresses {
    pub(crate) stake_table: Option<Address>,
    pub(crate) esp_token: Option<Address>,
    pub(crate) light_client: Option<Address>,
    pub(crate) fee_contract: Option<Address>,
    pub(crate) reward_claim: Option<Address>,
    pub(crate) multisigs: HashMap<String, Address>,
    pub(crate) ops_timelock: Option<Address>,
    pub(crate) safe_exit_timelock: Option<Address>,
}

impl DeploymentAddresses {
    pub(crate) fn from_env_file(path: &Path) -> Result<Self> {
        let env_map: HashMap<String, String> = dotenvy::from_path_iter(path)
            .with_context(|| format!("Failed to read env file: {:?}", path))?
            .filter_map(|item| {
                item.map_err(|e| tracing::warn!("Invalid line in env file {:?}: {}", path, e))
                    .ok()
            })
            .collect();

        let mut multisigs = HashMap::new();
        for key in env_map.keys() {
            // Try new prefix first, fall back to deprecated ESPRESSO_SEQUENCER_MULTISIG_*
            let name = key
                .strip_prefix(MULTISIG_PREFIX)
                .and_then(|s| s.strip_suffix(MULTISIG_SUFFIX))
                .or_else(|| {
                    key.strip_prefix(LEGACY_MULTISIG_PREFIX)
                        .and_then(|s| s.strip_suffix(MULTISIG_SUFFIX))
                        .inspect(|_| {
                            tracing::error!(
                                "{key} is deprecated, use {MULTISIG_PREFIX}...{MULTISIG_SUFFIX} \
                                 instead"
                            );
                        })
                });
            if let Some(name) = name {
                let name = name.to_lowercase();
                if let Some(addr) = parse_address(&env_map, key)? {
                    multisigs.insert(name, addr);
                }
            }
        }

        Ok(DeploymentAddresses {
            stake_table: parse_address(&env_map, STAKE_TABLE_PROXY_ADDRESS)?,
            esp_token: parse_address(&env_map, ESP_TOKEN_PROXY_ADDRESS)?,
            light_client: parse_address(&env_map, LIGHT_CLIENT_PROXY_ADDRESS)?,
            fee_contract: parse_address(&env_map, FEE_CONTRACT_PROXY_ADDRESS)?,
            reward_claim: parse_address(&env_map, REWARD_CLAIM_PROXY_ADDRESS)?,
            multisigs,
            ops_timelock: parse_address(&env_map, OPS_TIMELOCK_ADDRESS)?,
            safe_exit_timelock: parse_address(&env_map, SAFE_EXIT_TIMELOCK_ADDRESS)?,
        })
    }
}

/// Reverse map from address to human-readable name (multisigs + timelocks).
/// Used to validate that all contract role holders are tracked in the .env config.
#[derive(Debug, Clone)]
pub(crate) struct KnownAddresses(pub(crate) HashMap<Address, String>);

impl KnownAddresses {
    pub(crate) fn from_deployment(addresses: &DeploymentAddresses) -> Self {
        let mut known = HashMap::new();
        for (name, addr) in &addresses.multisigs {
            known.insert(*addr, name.clone());
        }
        if let Some(addr) = addresses.ops_timelock {
            known.insert(addr, "ops_timelock".to_string());
        }
        if let Some(addr) = addresses.safe_exit_timelock {
            known.insert(addr, "safe_exit_timelock".to_string());
        }
        Self(known)
    }

    pub(crate) fn resolve(&self, addr: Address) -> Result<String> {
        self.0.get(&addr).cloned().ok_or_else(|| {
            anyhow::anyhow!(
                "Address {addr} is not a known address. The .env config may be missing a multisig \
                 or other contract."
            )
        })
    }

    pub(crate) fn keys(&self) -> impl Iterator<Item = &Address> {
        self.0.keys()
    }
}

fn parse_address(env_map: &HashMap<String, String>, key: &str) -> Result<Option<Address>> {
    match env_map.get(key) {
        None => Ok(None),
        Some(val) if val.is_empty() => {
            bail!("{key} is set but empty")
        },
        Some(val) => {
            let addr = val
                .parse()
                .with_context(|| format!("Failed to parse {key} with value '{val}'"))?;
            Ok(Some(addr))
        },
    }
}
