use std::{collections::BTreeMap, fmt, time::Duration};

use alloy::{
    eips::BlockId,
    primitives::{Address, FixedBytes},
    providers::Provider,
    sol,
};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    Network,
    addresses::{DeploymentAddresses, KnownAddresses},
};

sol! {
    #[sol(rpc)]
    interface IOwnable {
        function owner() external view returns (address);
    }

    #[sol(rpc)]
    interface IAccessControl {
        function hasRole(bytes32 role, address account) external view returns (bool);
    }

    #[sol(rpc)]
    interface ITimelock {
        function getMinDelay() external view returns (uint256);
    }

    #[sol(rpc)]
    interface ISafe {
        function VERSION() external view returns (string memory);
        function getOwners() external view returns (address[] memory);
        function getThreshold() external view returns (uint256);
    }

    #[sol(rpc)]
    interface IVersioned {
        function getVersion() external view returns (uint8, uint8, uint8);
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ContractType {
    LightClient,
    FeeContract,
    EspToken,
    StakeTable,
    RewardClaim,
}

impl fmt::Display for ContractType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractType::LightClient => f.write_str("LightClient"),
            ContractType::FeeContract => f.write_str("FeeContract"),
            ContractType::EspToken => f.write_str("EspToken"),
            ContractType::StakeTable => f.write_str("StakeTable"),
            ContractType::RewardClaim => f.write_str("RewardClaim"),
        }
    }
}

pub(crate) enum AccessControlRole {
    DefaultAdmin,
    Pauser,
}

impl AccessControlRole {
    fn hash(&self) -> FixedBytes<32> {
        match self {
            AccessControlRole::DefaultAdmin => FixedBytes::ZERO,
            AccessControlRole::Pauser => alloy::primitives::keccak256("PAUSER_ROLE"),
        }
    }
}

impl fmt::Display for AccessControlRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessControlRole::DefaultAdmin => f.write_str("DEFAULT_ADMIN_ROLE"),
            AccessControlRole::Pauser => f.write_str("PAUSER_ROLE"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RoleHolder {
    pub(crate) address: Address,
    pub(crate) name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub(crate) enum OwnableDeployment {
    Deployed {
        address: Address,
        owner_address: Address,
        owner_name: String,
        version: String,
    },
    NotYetDeployed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub(crate) enum AccessControlDeployment {
    Deployed {
        address: Address,
        default_admin_address: Address,
        default_admin_name: String,
        version: String,
        pauser_address: Address,
        pauser_name: String,
    },
    NotYetDeployed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct MultisigDeployment {
    pub(crate) address: Address,
    pub(crate) version: String,
    pub(crate) owners: Vec<Address>,
    pub(crate) threshold: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub(crate) enum TimelockDeployment {
    Deployed {
        address: Address,
        #[serde(with = "humantime_serde")]
        min_delay: Duration,
    },
    NotYetDeployed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct DeploymentInfo {
    pub(crate) network: Network,
    pub(crate) multisigs: BTreeMap<String, MultisigDeployment>,
    pub(crate) ops_timelock: TimelockDeployment,
    pub(crate) safe_exit_timelock: TimelockDeployment,
    pub(crate) esp_token: OwnableDeployment,
    pub(crate) fee_contract: OwnableDeployment,
    pub(crate) light_client: OwnableDeployment,
    pub(crate) reward_claim: AccessControlDeployment,
    pub(crate) stake_table: AccessControlDeployment,
}

pub(crate) struct DeploymentQuerier<'a, P: Provider> {
    provider: &'a P,
    known: KnownAddresses,
    block_id: BlockId,
}

impl<'a, P: Provider> DeploymentQuerier<'a, P> {
    pub(crate) fn new(provider: &'a P, known: KnownAddresses, block_number: u64) -> Self {
        Self {
            provider,
            known,
            block_id: BlockId::number(block_number),
        }
    }

    /// Finds which known address holds the given role. Errors if the holder is not
    /// in `self.known` -- this validates that all role holders are tracked in the .env config.
    async fn find_role_holder(
        &self,
        contract_addr: Address,
        role: AccessControlRole,
    ) -> Result<Address> {
        let contract = IAccessControl::new(contract_addr, self.provider);
        let role_hash = role.hash();
        let mut holders = Vec::new();
        for addr in self.known.keys() {
            let has_role = contract
                .hasRole(role_hash, *addr)
                .block(self.block_id)
                .call()
                .await?;
            if has_role {
                holders.push(*addr);
            }
        }
        match holders.len() {
            0 => bail!(
                "No known address holds {role} at {contract_addr}. The .env config may be missing \
                 a multisig or other contract."
            ),
            1 => Ok(holders[0]),
            _ => bail!(
                "Multiple known addresses hold {role} at {contract_addr}: {holders:?}. This is \
                 unexpected."
            ),
        }
    }

    async fn get_version(&self, addr: Address) -> Result<String> {
        let v = IVersioned::new(addr, self.provider)
            .getVersion()
            .block(self.block_id)
            .call()
            .await?;
        Ok(format!("{}.{}.{}", v._0, v._1, v._2))
    }

    fn resolve_role_holder(&self, addr: Address) -> Result<RoleHolder> {
        let name = self.known.resolve(addr)?;
        Ok(RoleHolder {
            address: addr,
            name,
        })
    }

    pub(crate) async fn query_ownable(
        &self,
        addr: Address,
        contract_type: ContractType,
    ) -> Result<OwnableDeployment> {
        tracing::info!("querying {contract_type} at {addr}");

        let owner_addr = IOwnable::new(addr, self.provider)
            .owner()
            .block(self.block_id)
            .call()
            .await?;
        let version = self.get_version(addr).await?;

        let owner = self
            .resolve_role_holder(owner_addr)
            .context(format!("owner of {contract_type}"))?;

        tracing::info!("  owner={} version={version}", owner.name);

        Ok(OwnableDeployment::Deployed {
            address: addr,
            owner_address: owner.address,
            owner_name: owner.name,
            version,
        })
    }

    pub(crate) async fn query_access_control(
        &self,
        addr: Address,
        contract_type: ContractType,
    ) -> Result<AccessControlDeployment> {
        tracing::info!("querying {contract_type} at {addr}");

        let admin_addr = self
            .find_role_holder(addr, AccessControlRole::DefaultAdmin)
            .await
            .context(format!("default admin of {contract_type}"))?;
        let version = self.get_version(addr).await?;

        let admin = self
            .resolve_role_holder(admin_addr)
            .context(format!("default admin of {contract_type}"))?;

        let pauser_addr = match contract_type {
            ContractType::StakeTable | ContractType::RewardClaim => self
                .find_role_holder(addr, AccessControlRole::Pauser)
                .await
                .context(format!("pauser of {contract_type}"))?,
            other => bail!("{other} is not an AccessControl contract"),
        };
        let pauser = self
            .resolve_role_holder(pauser_addr)
            .context(format!("pauser of {contract_type}"))?;

        tracing::info!(
            "  default_admin={} version={version} pauser={}",
            admin.name,
            pauser.name
        );

        Ok(AccessControlDeployment::Deployed {
            address: addr,
            default_admin_address: admin.address,
            default_admin_name: admin.name,
            version,
            pauser_address: pauser.address,
            pauser_name: pauser.name,
        })
    }

    async fn query_multisig(&self, name: &str, addr: Address) -> Result<MultisigDeployment> {
        let contract = ISafe::new(addr, self.provider);
        let version = contract
            .VERSION()
            .block(self.block_id)
            .call()
            .await
            .with_context(|| format!("Failed to get VERSION for multisig '{name}' at {addr}"))?;
        let owners = contract
            .getOwners()
            .block(self.block_id)
            .call()
            .await
            .with_context(|| format!("Failed to get owners for multisig '{name}' at {addr}"))?;
        let threshold: u64 = contract
            .getThreshold()
            .block(self.block_id)
            .call()
            .await
            .with_context(|| format!("Failed to get threshold for multisig '{name}' at {addr}"))?
            .try_into()
            .context("threshold exceeds u64")?;
        Ok(MultisigDeployment {
            address: addr,
            version,
            owners,
            threshold,
        })
    }

    pub(crate) async fn query_timelock(&self, addr: Address) -> Result<TimelockDeployment> {
        let min_delay_secs: u64 = ITimelock::new(addr, self.provider)
            .getMinDelay()
            .block(self.block_id)
            .call()
            .await?
            .try_into()
            .context("min_delay exceeds u64")?;
        let min_delay = Duration::from_secs(min_delay_secs);

        Ok(TimelockDeployment::Deployed {
            address: addr,
            min_delay,
        })
    }
}

pub(crate) struct CollectedDeployment {
    pub(crate) info: DeploymentInfo,
    pub(crate) block_number: u64,
    pub(crate) block_timestamp: u64,
}

#[cfg(test)]
impl DeploymentInfo {
    pub(crate) fn for_test() -> Self {
        let addr1: Address = "0x1111111111111111111111111111111111111111"
            .parse()
            .unwrap();
        let addr2: Address = "0x2222222222222222222222222222222222222222"
            .parse()
            .unwrap();
        let addr3: Address = "0x3333333333333333333333333333333333333333"
            .parse()
            .unwrap();

        DeploymentInfo {
            network: crate::Network::Mainnet,
            multisigs: BTreeMap::from([(
                "espresso_labs".to_string(),
                MultisigDeployment {
                    address: addr2,
                    version: "1.4.1".to_string(),
                    owners: vec![addr1],
                    threshold: 3,
                },
            )]),
            ops_timelock: TimelockDeployment::Deployed {
                address: addr3,
                min_delay: Duration::from_secs(172800),
            },
            safe_exit_timelock: TimelockDeployment::NotYetDeployed,
            esp_token: OwnableDeployment::NotYetDeployed,
            fee_contract: OwnableDeployment::NotYetDeployed,
            light_client: OwnableDeployment::Deployed {
                address: addr2,
                owner_address: addr1,
                owner_name: "espresso_labs".to_string(),
                version: "1.0.0".to_string(),
            },
            reward_claim: AccessControlDeployment::NotYetDeployed,
            stake_table: AccessControlDeployment::Deployed {
                address: addr1,
                default_admin_address: addr3,
                default_admin_name: "ops_timelock".to_string(),
                version: "2.0.0".to_string(),
                pauser_address: addr2,
                pauser_name: "espresso_labs".to_string(),
            },
        }
    }
}

impl CollectedDeployment {
    pub(crate) async fn collect(
        rpc_url: Url,
        network: Network,
        addresses: DeploymentAddresses,
    ) -> Result<Self> {
        let provider = alloy::providers::ProviderBuilder::new().connect_http(rpc_url);

        let block_number = provider
            .get_block_number()
            .await
            .context("Failed to get block number")?;
        let block = provider
            .get_block_by_number(block_number.into())
            .await
            .context("Failed to get block")?
            .context("Block not found")?;
        let block_timestamp = block.header.timestamp;

        let known = KnownAddresses::from_deployment(&addresses);
        let querier = DeploymentQuerier::new(&provider, known, block_number);

        let mut multisigs = BTreeMap::new();
        for (name, addr) in &addresses.multisigs {
            let deployment = querier.query_multisig(name, *addr).await?;
            multisigs.insert(name.clone(), deployment);
        }

        let ops_timelock = match addresses.ops_timelock {
            Some(addr) => querier
                .query_timelock(addr)
                .await
                .with_context(|| format!("Failed to query OpsTimelock at {addr}"))?,
            None => TimelockDeployment::NotYetDeployed,
        };

        let safe_exit_timelock = match addresses.safe_exit_timelock {
            Some(addr) => querier
                .query_timelock(addr)
                .await
                .with_context(|| format!("Failed to query SafeExitTimelock at {addr}"))?,
            None => TimelockDeployment::NotYetDeployed,
        };

        let esp_token = match addresses.esp_token {
            Some(addr) => querier.query_ownable(addr, ContractType::EspToken).await?,
            None => OwnableDeployment::NotYetDeployed,
        };
        let fee_contract = match addresses.fee_contract {
            Some(addr) => {
                querier
                    .query_ownable(addr, ContractType::FeeContract)
                    .await?
            },
            None => OwnableDeployment::NotYetDeployed,
        };
        let light_client = match addresses.light_client {
            Some(addr) => {
                querier
                    .query_ownable(addr, ContractType::LightClient)
                    .await?
            },
            None => OwnableDeployment::NotYetDeployed,
        };
        let reward_claim = match addresses.reward_claim {
            Some(addr) => {
                querier
                    .query_access_control(addr, ContractType::RewardClaim)
                    .await?
            },
            None => AccessControlDeployment::NotYetDeployed,
        };
        let stake_table = match addresses.stake_table {
            Some(addr) => {
                querier
                    .query_access_control(addr, ContractType::StakeTable)
                    .await?
            },
            None => AccessControlDeployment::NotYetDeployed,
        };

        Ok(CollectedDeployment {
            info: DeploymentInfo {
                network,
                multisigs,
                ops_timelock,
                safe_exit_timelock,
                stake_table,
                esp_token,
                light_client,
                fee_contract,
                reward_claim,
            },
            block_number,
            block_timestamp,
        })
    }
}
