use std::{collections::HashMap, path::Path};

use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder},
};
use anyhow::{Context, Result};
use hotshot_contract_adapter::sol_types::{
    EspTokenV2, FeeContract, ISafe, IVersioned, LightClient, OpsTimelock, SafeExitTimelock,
    StakeTable,
};
use serde::{Deserialize, Serialize};
use url::Url;

const STAKE_TABLE_PROXY_ADDRESS: &str = "ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS";
const ESP_TOKEN_PROXY_ADDRESS: &str = "ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS";
const LIGHT_CLIENT_PROXY_ADDRESS: &str = "ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS";
const FEE_CONTRACT_PROXY_ADDRESS: &str = "ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS";
const REWARD_CLAIM_PROXY_ADDRESS: &str = "ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS";
const MULTISIG_ADDRESS: &str = "ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS";
const OPS_TIMELOCK_PROXY_ADDRESS: &str = "ESPRESSO_SEQUENCER_OPS_TIMELOCK_PROXY_ADDRESS";
const SAFE_EXIT_TIMELOCK_PROXY_ADDRESS: &str =
    "ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_PROXY_ADDRESS";

#[derive(Debug, Default, Clone, PartialEq)]
pub struct DeploymentAddresses {
    pub stake_table_proxy: Option<Address>,
    pub esp_token_proxy: Option<Address>,
    pub light_client_proxy: Option<Address>,
    pub fee_contract_proxy: Option<Address>,
    pub reward_claim_proxy: Option<Address>,
    pub multisig: Option<Address>,
    pub ops_timelock: Option<Address>,
    pub safe_exit_timelock: Option<Address>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum ContractDeployment {
    Deployed {
        proxy_address: Address,
        owner: Address,
        version: String,
    },
    NotYetDeployed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum MultisigDeployment {
    Deployed {
        address: Address,
        version: String,
        owners: Vec<Address>,
        threshold: u64,
    },
    NotYetDeployed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum TimelockDeployment {
    Deployed { address: Address, min_delay: u64 },
    NotYetDeployed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeploymentInfo {
    pub network: String,
    pub multisig: MultisigDeployment,
    pub ops_timelock: TimelockDeployment,
    pub safe_exit_timelock: TimelockDeployment,
    pub stake_table_proxy: ContractDeployment,
    pub esp_token_proxy: ContractDeployment,
    pub light_client_proxy: ContractDeployment,
    pub fee_contract_proxy: ContractDeployment,
    pub reward_claim_proxy: ContractDeployment,
}

pub fn load_addresses_from_env_file(path: Option<&Path>) -> Result<DeploymentAddresses> {
    let env_map: HashMap<String, String> = if let Some(p) = path {
        dotenvy::from_path_iter(p)
            .with_context(|| format!("Failed to read env file: {:?}", p))?
            .filter_map(|item| item.ok())
            .collect()
    } else {
        dotenvy::dotenv_iter()
            .ok()
            .map(|iter| iter.filter_map(|item| item.ok()).collect())
            .unwrap_or_default()
    };

    fn parse_address(env_map: &HashMap<String, String>, key: &str) -> Option<Address> {
        env_map.get(key).and_then(|val| {
            if val.is_empty() {
                tracing::warn!("{} is set but empty", key);
                None
            } else {
                match val.parse() {
                    Ok(addr) => Some(addr),
                    Err(e) => {
                        tracing::warn!("Failed to parse {} with value '{}': {}", key, val, e);
                        None
                    },
                }
            }
        })
    }

    Ok(DeploymentAddresses {
        stake_table_proxy: parse_address(&env_map, STAKE_TABLE_PROXY_ADDRESS),
        esp_token_proxy: parse_address(&env_map, ESP_TOKEN_PROXY_ADDRESS),
        light_client_proxy: parse_address(&env_map, LIGHT_CLIENT_PROXY_ADDRESS),
        fee_contract_proxy: parse_address(&env_map, FEE_CONTRACT_PROXY_ADDRESS),
        reward_claim_proxy: parse_address(&env_map, REWARD_CLAIM_PROXY_ADDRESS),
        multisig: parse_address(&env_map, MULTISIG_ADDRESS),
        ops_timelock: parse_address(&env_map, OPS_TIMELOCK_PROXY_ADDRESS),
        safe_exit_timelock: parse_address(&env_map, SAFE_EXIT_TIMELOCK_PROXY_ADDRESS),
    })
}

#[derive(Debug, Clone, Copy)]
enum ContractType {
    LightClient,
    FeeContract,
    EspToken,
    StakeTable,
    RewardClaim,
}

async fn get_owner<P: Provider>(
    provider: &P,
    addr: Address,
    contract_type: ContractType,
) -> Result<Address> {
    match contract_type {
        ContractType::LightClient => {
            let contract = LightClient::new(addr, provider);
            Ok(contract.owner().call().await?)
        },
        ContractType::FeeContract => {
            let contract = FeeContract::new(addr, provider);
            Ok(contract.owner().call().await?)
        },
        ContractType::EspToken => {
            let contract = EspTokenV2::new(addr, provider);
            Ok(contract.owner().call().await?)
        },
        ContractType::StakeTable => {
            let contract = StakeTable::new(addr, provider);
            Ok(contract.owner().call().await?)
        },
        ContractType::RewardClaim => Ok(Address::ZERO),
    }
}

async fn get_version<P: Provider>(
    provider: &P,
    addr: Address,
    _contract_type: ContractType,
) -> Result<String> {
    let contract = IVersioned::new(addr, provider);
    let v = contract.getVersion().call().await?;
    Ok(format!("{}.{}.{}", v._0, v._1, v._2))
}

async fn get_contract_info<P: Provider>(
    provider: &P,
    proxy_addr: Address,
    contract_type: ContractType,
) -> Result<ContractDeployment> {
    let owner = get_owner(provider, proxy_addr, contract_type).await?;
    let version = get_version(provider, proxy_addr, contract_type).await?;

    Ok(ContractDeployment::Deployed {
        proxy_address: proxy_addr,
        owner,
        version,
    })
}

async fn collect_contract_info<P: Provider>(
    provider: &P,
    addr: Option<Address>,
    contract_type: ContractType,
    contract_name: &str,
) -> Result<ContractDeployment> {
    let Some(addr) = addr else {
        return Ok(ContractDeployment::NotYetDeployed);
    };

    get_contract_info(provider, addr, contract_type)
        .await
        .with_context(|| format!("Failed to query {} at {}", contract_name, addr))
}

async fn get_multisig_info<P: Provider>(provider: &P, addr: Address) -> Result<MultisigDeployment> {
    let contract = ISafe::new(addr, provider);

    let version = contract
        .VERSION()
        .call()
        .await
        .context("Failed to get VERSION")?;

    let owners = contract
        .getOwners()
        .call()
        .await
        .context("Failed to get owners")?;

    let threshold = contract
        .getThreshold()
        .call()
        .await
        .context("Failed to get threshold")?
        .to::<u64>();

    Ok(MultisigDeployment::Deployed {
        address: addr,
        version,
        owners,
        threshold,
    })
}

async fn collect_multisig_info<P: Provider>(
    provider: &P,
    addr: Option<Address>,
) -> Result<MultisigDeployment> {
    let Some(addr) = addr else {
        return Ok(MultisigDeployment::NotYetDeployed);
    };

    get_multisig_info(provider, addr)
        .await
        .with_context(|| format!("Failed to query multisig at {}", addr))
}

async fn get_timelock_info<P: Provider>(
    provider: &P,
    addr: Address,
    is_ops: bool,
) -> Result<TimelockDeployment> {
    let min_delay = if is_ops {
        OpsTimelock::new(addr, provider)
            .getMinDelay()
            .call()
            .await
            .context("Failed to get min delay from OpsTimelock")?
            .to::<u64>()
    } else {
        SafeExitTimelock::new(addr, provider)
            .getMinDelay()
            .call()
            .await
            .context("Failed to get min delay from SafeExitTimelock")?
            .to::<u64>()
    };

    Ok(TimelockDeployment::Deployed {
        address: addr,
        min_delay,
    })
}

async fn collect_timelock_info<P: Provider>(
    provider: &P,
    addr: Option<Address>,
    name: &str,
    is_ops: bool,
) -> Result<TimelockDeployment> {
    let Some(addr) = addr else {
        return Ok(TimelockDeployment::NotYetDeployed);
    };

    get_timelock_info(provider, addr, is_ops)
        .await
        .with_context(|| format!("Failed to query {} at {}", name, addr))
}

pub async fn collect_deployment_info(
    rpc_url: Url,
    network: String,
    addresses: DeploymentAddresses,
) -> Result<DeploymentInfo> {
    let provider = ProviderBuilder::new().connect_http(rpc_url);

    Ok(DeploymentInfo {
        network,
        multisig: collect_multisig_info(&provider, addresses.multisig).await?,
        ops_timelock: collect_timelock_info(&provider, addresses.ops_timelock, "OpsTimelock", true)
            .await?,
        safe_exit_timelock: collect_timelock_info(
            &provider,
            addresses.safe_exit_timelock,
            "SafeExitTimelock",
            false,
        )
        .await?,
        stake_table_proxy: collect_contract_info(
            &provider,
            addresses.stake_table_proxy,
            ContractType::StakeTable,
            "StakeTable",
        )
        .await?,
        esp_token_proxy: collect_contract_info(
            &provider,
            addresses.esp_token_proxy,
            ContractType::EspToken,
            "EspToken",
        )
        .await?,
        light_client_proxy: collect_contract_info(
            &provider,
            addresses.light_client_proxy,
            ContractType::LightClient,
            "LightClient",
        )
        .await?,
        fee_contract_proxy: collect_contract_info(
            &provider,
            addresses.fee_contract_proxy,
            ContractType::FeeContract,
            "FeeContract",
        )
        .await?,
        reward_claim_proxy: collect_contract_info(
            &provider,
            addresses.reward_claim_proxy,
            ContractType::RewardClaim,
            "RewardClaim",
        )
        .await?,
    })
}

pub fn write_deployment_info(info: &DeploymentInfo, output_path: &std::path::Path) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create output directory")?;
    }

    let json = serde_json::to_string_pretty(info).context("Failed to serialize deployment info")?;

    std::fs::write(output_path, json).context("Failed to write deployment info")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use alloy::{
        node_bindings::Anvil,
        providers::{ProviderBuilder, WalletProvider},
    };
    use espresso_contract_deployer::{
        builder::DeployerArgsBuilder, network_config::light_client_genesis_from_stake_table,
        Contract, Contracts,
    };
    use hotshot_state_prover::v3::mock_ledger::STAKE_TABLE_CAPACITY_FOR_TEST;

    use super::*;

    #[test_log::test(tokio::test)]
    async fn test_collect_deployment_info_with_deployed_contracts() -> Result<()> {
        let anvil = Anvil::new().spawn();
        let provider = ProviderBuilder::new()
            .wallet(anvil.wallet().unwrap())
            .connect_http(anvil.endpoint_url());
        let rpc_url = anvil.endpoint_url();
        let deployer_address = provider.default_signer_address();

        let (genesis_state, genesis_stake) = light_client_genesis_from_stake_table(
            &Default::default(),
            STAKE_TABLE_CAPACITY_FOR_TEST,
        )
        .unwrap();

        let mut contracts = Contracts::new();
        let args = DeployerArgsBuilder::default()
            .deployer(provider.clone())
            .rpc_url(rpc_url.clone())
            .mock_light_client(true)
            .genesis_lc_state(genesis_state)
            .genesis_st_state(genesis_stake)
            .blocks_per_epoch(100)
            .epoch_start_block(1)
            .multisig_pauser(deployer_address)
            .exit_escrow_period(alloy::primitives::U256::from(250))
            .token_name("Espresso".to_string())
            .token_symbol("ESP".to_string())
            .initial_token_supply(alloy::primitives::U256::from(3590000000u64))
            .ops_timelock_delay(alloy::primitives::U256::from(100))
            .ops_timelock_admin(deployer_address)
            .ops_timelock_proposers(vec![deployer_address])
            .ops_timelock_executors(vec![deployer_address])
            .safe_exit_timelock_delay(alloy::primitives::U256::from(200))
            .safe_exit_timelock_admin(deployer_address)
            .safe_exit_timelock_proposers(vec![deployer_address])
            .safe_exit_timelock_executors(vec![deployer_address])
            .use_timelock_owner(false)
            .build()
            .unwrap();

        args.deploy_all(&mut contracts).await?;

        let stake_table_addr = contracts
            .address(Contract::StakeTableProxy)
            .expect("StakeTableProxy deployed");
        let esp_token_addr = contracts
            .address(Contract::EspTokenProxy)
            .expect("EspTokenProxy deployed");
        let light_client_addr = contracts
            .address(Contract::LightClientProxy)
            .expect("LightClientProxy deployed");
        let fee_contract_addr = contracts
            .address(Contract::FeeContractProxy)
            .expect("FeeContractProxy deployed");
        let reward_claim_addr = contracts
            .address(Contract::RewardClaimProxy)
            .expect("RewardClaimProxy deployed");
        let ops_timelock_addr = contracts
            .address(Contract::OpsTimelock)
            .expect("OpsTimelock deployed");
        let safe_exit_timelock_addr = contracts
            .address(Contract::SafeExitTimelock)
            .expect("SafeExitTimelock deployed");

        let addresses = DeploymentAddresses {
            stake_table_proxy: Some(stake_table_addr),
            esp_token_proxy: Some(esp_token_addr),
            light_client_proxy: Some(light_client_addr),
            fee_contract_proxy: Some(fee_contract_addr),
            reward_claim_proxy: Some(reward_claim_addr),
            multisig: None,
            ops_timelock: Some(ops_timelock_addr),
            safe_exit_timelock: Some(safe_exit_timelock_addr),
        };

        let info = collect_deployment_info(rpc_url, "test-network".to_string(), addresses).await?;

        assert_eq!(info.network, "test-network");
        assert_eq!(
            info.stake_table_proxy,
            ContractDeployment::Deployed {
                proxy_address: stake_table_addr,
                owner: deployer_address,
                version: "2.0.0".to_string(),
            }
        );
        assert_eq!(
            info.esp_token_proxy,
            ContractDeployment::Deployed {
                proxy_address: esp_token_addr,
                owner: deployer_address,
                version: "2.0.0".to_string(),
            }
        );
        assert_eq!(
            info.light_client_proxy,
            ContractDeployment::Deployed {
                proxy_address: light_client_addr,
                owner: deployer_address,
                version: "3.0.0".to_string(),
            }
        );
        assert_eq!(
            info.fee_contract_proxy,
            ContractDeployment::Deployed {
                proxy_address: fee_contract_addr,
                owner: deployer_address,
                version: "1.0.0".to_string(),
            }
        );
        assert_eq!(
            info.reward_claim_proxy,
            ContractDeployment::Deployed {
                proxy_address: reward_claim_addr,
                owner: Address::ZERO,
                version: "1.0.0".to_string(),
            }
        );
        assert_eq!(
            info.ops_timelock,
            TimelockDeployment::Deployed {
                address: ops_timelock_addr,
                min_delay: 100
            }
        );
        assert_eq!(
            info.safe_exit_timelock,
            TimelockDeployment::Deployed {
                address: safe_exit_timelock_addr,
                min_delay: 200
            }
        );
        assert_eq!(info.multisig, MultisigDeployment::NotYetDeployed);

        Ok(())
    }
}
