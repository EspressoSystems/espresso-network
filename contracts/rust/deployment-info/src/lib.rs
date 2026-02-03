use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder},
};
use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
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
const MULTISIG_PREFIX: &str = "ESPRESSO_SEQUENCER_MULTISIG_";
const MULTISIG_SUFFIX: &str = "_ADDRESS";
const OPS_TIMELOCK_ADDRESS: &str = "ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS";
const SAFE_EXIT_TIMELOCK_ADDRESS: &str = "ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS";

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Network {
    Decaf,
    Hoodi,
    Mainnet,
}

const ALL_NETWORKS: [Network; 3] = [Network::Decaf, Network::Hoodi, Network::Mainnet];

impl Network {
    fn as_str(&self) -> &'static str {
        match self {
            Network::Decaf => "decaf",
            Network::Hoodi => "hoodi",
            Network::Mainnet => "mainnet",
        }
    }

    fn default_rpc_url(&self) -> Url {
        match self {
            Network::Decaf => "https://ethereum-sepolia-rpc.publicnode.com",
            Network::Hoodi => "https://ethereum-hoodi-rpc.publicnode.com",
            Network::Mainnet => "https://ethereum-rpc.publicnode.com",
        }
        .parse()
        .expect("hardcoded URL is valid")
    }
}

#[derive(Debug, Parser)]
#[clap(
    name = "deployment-info",
    about = "Collect and output deployment information for Espresso Network contracts"
)]
struct Args {
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_L1_PROVIDER",
        help = "RPC URL for L1 provider. Defaults to publicnode when --network is specified."
    )]
    rpc_url: Option<Url>,

    #[clap(
        long,
        value_enum,
        help = "Known network. If not specified, all networks are processed."
    )]
    network: Option<Network>,

    #[clap(long, help = "Path to input .env file. Only valid with --network.")]
    env_file: Option<PathBuf>,

    #[clap(long, help = "Output file path. Only valid with --network.")]
    output: Option<PathBuf>,

    #[clap(
        long,
        help = "Print to stdout instead of writing to a file. Only valid with --network."
    )]
    stdout: bool,
}

#[derive(Debug, Default, Clone, PartialEq)]
struct DeploymentAddresses {
    stake_table: Option<Address>,
    esp_token: Option<Address>,
    light_client: Option<Address>,
    fee_contract: Option<Address>,
    reward_claim: Option<Address>,
    multisigs: HashMap<String, Address>,
    ops_timelock: Option<Address>,
    safe_exit_timelock: Option<Address>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
enum ContractDeployment {
    Deployed {
        address: Address,
        owner: Address,
        version: String,
    },
    NotYetDeployed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
enum MultisigDeployment {
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
enum TimelockDeployment {
    Deployed { address: Address, min_delay: u64 },
    NotYetDeployed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct DeploymentInfo {
    network: String,
    multisigs: HashMap<String, MultisigDeployment>,
    ops_timelock: TimelockDeployment,
    safe_exit_timelock: TimelockDeployment,
    stake_table: ContractDeployment,
    esp_token: ContractDeployment,
    light_client: ContractDeployment,
    fee_contract: ContractDeployment,
    reward_claim: ContractDeployment,
}

fn get_crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_addresses_from_env_file(path: &Path) -> Result<DeploymentAddresses> {
    let env_map: HashMap<String, String> = dotenvy::from_path_iter(path)
        .with_context(|| format!("Failed to read env file: {:?}", path))?
        .filter_map(|item| item.ok())
        .collect();

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

    let mut multisigs = HashMap::new();
    for (key, value) in &env_map {
        if key.starts_with(MULTISIG_PREFIX) && key.ends_with(MULTISIG_SUFFIX) {
            let name = &key[MULTISIG_PREFIX.len()..key.len() - MULTISIG_SUFFIX.len()];
            let name = name.to_lowercase();
            if let Some(addr) = parse_address(&env_map, key) {
                multisigs.insert(name, addr);
            } else {
                tracing::warn!(
                    "Multisig key {} found but value '{}' is not a valid address",
                    key,
                    value
                );
            }
        }
    }

    Ok(DeploymentAddresses {
        stake_table: parse_address(&env_map, STAKE_TABLE_PROXY_ADDRESS),
        esp_token: parse_address(&env_map, ESP_TOKEN_PROXY_ADDRESS),
        light_client: parse_address(&env_map, LIGHT_CLIENT_PROXY_ADDRESS),
        fee_contract: parse_address(&env_map, FEE_CONTRACT_PROXY_ADDRESS),
        reward_claim: parse_address(&env_map, REWARD_CLAIM_PROXY_ADDRESS),
        multisigs,
        ops_timelock: parse_address(&env_map, OPS_TIMELOCK_ADDRESS),
        safe_exit_timelock: parse_address(&env_map, SAFE_EXIT_TIMELOCK_ADDRESS),
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
    addr: Address,
    contract_type: ContractType,
) -> Result<ContractDeployment> {
    let owner = get_owner(provider, addr, contract_type).await?;
    let version = get_version(provider, addr, contract_type).await?;

    Ok(ContractDeployment::Deployed {
        address: addr,
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
    name: &str,
    addr: Address,
) -> Result<MultisigDeployment> {
    get_multisig_info(provider, addr)
        .await
        .with_context(|| format!("Failed to query multisig '{}' at {}", name, addr))
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

async fn collect_deployment_info(
    rpc_url: Url,
    network: String,
    addresses: DeploymentAddresses,
) -> Result<DeploymentInfo> {
    let provider = ProviderBuilder::new().connect_http(rpc_url);

    let mut multisigs = HashMap::new();
    for (name, addr) in &addresses.multisigs {
        let info = collect_multisig_info(&provider, name, *addr).await?;
        multisigs.insert(name.clone(), info);
    }

    Ok(DeploymentInfo {
        network,
        multisigs,
        ops_timelock: collect_timelock_info(&provider, addresses.ops_timelock, "OpsTimelock", true)
            .await?,
        safe_exit_timelock: collect_timelock_info(
            &provider,
            addresses.safe_exit_timelock,
            "SafeExitTimelock",
            false,
        )
        .await?,
        stake_table: collect_contract_info(
            &provider,
            addresses.stake_table,
            ContractType::StakeTable,
            "StakeTable",
        )
        .await?,
        esp_token: collect_contract_info(
            &provider,
            addresses.esp_token,
            ContractType::EspToken,
            "EspToken",
        )
        .await?,
        light_client: collect_contract_info(
            &provider,
            addresses.light_client,
            ContractType::LightClient,
            "LightClient",
        )
        .await?,
        fee_contract: collect_contract_info(
            &provider,
            addresses.fee_contract,
            ContractType::FeeContract,
            "FeeContract",
        )
        .await?,
        reward_claim: collect_contract_info(
            &provider,
            addresses.reward_claim,
            ContractType::RewardClaim,
            "RewardClaim",
        )
        .await?,
    })
}

fn write_deployment_info(info: &DeploymentInfo, output_path: &Path) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create output directory")?;
    }

    let yaml = serde_yaml::to_string(info).context("Failed to serialize deployment info")?;

    std::fs::write(output_path, yaml).context("Failed to write deployment info")?;

    Ok(())
}

async fn process_network(
    network: Network,
    rpc_url: Option<&Url>,
    env_file: Option<&PathBuf>,
    output: Option<&PathBuf>,
    stdout: bool,
) -> Result<()> {
    let crate_dir = get_crate_dir();

    let env_file = match env_file {
        Some(path) => path.clone(),
        None => crate_dir.join(format!("addresses/{}.env", network.as_str())),
    };

    let addresses = load_addresses_from_env_file(&env_file)
        .context("Failed to load addresses from env file")?;

    let rpc_url = match rpc_url {
        Some(url) => url.clone(),
        None => network.default_rpc_url(),
    };

    let network_name = network.as_str().to_string();

    tracing::info!("Collecting deployment info for network: {}", network_name);

    let info = collect_deployment_info(rpc_url, network_name, addresses)
        .await
        .context("Failed to collect deployment info")?;

    if stdout {
        let yaml =
            serde_yaml::to_string(&info).context("Failed to serialize deployment info to YAML")?;
        println!("{}", yaml);
    } else {
        let output_path = match output {
            Some(path) => path.clone(),
            None => crate_dir.join(format!("deployments/{}.yaml", network.as_str())),
        };

        write_deployment_info(&info, &output_path)
            .context("Failed to write deployment info to file")?;
        tracing::info!("Wrote: {:?}", output_path);
    }

    Ok(())
}

pub async fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    if let Some(network) = args.network {
        process_network(
            network,
            args.rpc_url.as_ref(),
            args.env_file.as_ref(),
            args.output.as_ref(),
            args.stdout,
        )
        .await?;
    } else {
        if args.env_file.is_some() || args.output.is_some() || args.stdout {
            bail!("--env-file, --output, and --stdout are only valid with --network");
        }

        for network in ALL_NETWORKS {
            process_network(network, args.rpc_url.as_ref(), None, None, false).await?;
        }
    }

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
            stake_table: Some(stake_table_addr),
            esp_token: Some(esp_token_addr),
            light_client: Some(light_client_addr),
            fee_contract: Some(fee_contract_addr),
            reward_claim: Some(reward_claim_addr),
            multisigs: HashMap::new(),
            ops_timelock: Some(ops_timelock_addr),
            safe_exit_timelock: Some(safe_exit_timelock_addr),
        };

        let info = collect_deployment_info(rpc_url, "test-network".to_string(), addresses).await?;

        assert_eq!(info.network, "test-network");
        assert_eq!(
            info.stake_table,
            ContractDeployment::Deployed {
                address: stake_table_addr,
                owner: deployer_address,
                version: "3.0.0".to_string(),
            }
        );
        assert_eq!(
            info.esp_token,
            ContractDeployment::Deployed {
                address: esp_token_addr,
                owner: deployer_address,
                version: "3.0.0".to_string(),
            }
        );
        assert_eq!(
            info.light_client,
            ContractDeployment::Deployed {
                address: light_client_addr,
                owner: deployer_address,
                version: "3.0.0".to_string(),
            }
        );
        assert_eq!(
            info.fee_contract,
            ContractDeployment::Deployed {
                address: fee_contract_addr,
                owner: deployer_address,
                version: "1.0.0".to_string(),
            }
        );
        assert_eq!(
            info.reward_claim,
            ContractDeployment::Deployed {
                address: reward_claim_addr,
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
        assert!(info.multisigs.is_empty());

        Ok(())
    }
}
