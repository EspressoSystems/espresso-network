mod addresses;
mod contracts;
mod output;

use std::{fmt, path::PathBuf};

use addresses::DeploymentAddresses;
use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use contracts::CollectedDeployment;
use output::update_readme_from_deployment_files;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Network {
    Decaf,
    Hoodi,
    Mainnet,
}

impl Network {
    fn default_rpc_url(&self) -> Url {
        match self {
            Network::Decaf => "https://ethereum-sepolia-rpc.publicnode.com",
            Network::Hoodi => "https://ethereum-hoodi-rpc.publicnode.com",
            Network::Mainnet => "https://ethereum-rpc.publicnode.com",
        }
        .parse()
        .expect("hardcoded URL is valid")
    }

    pub(crate) fn etherscan_base_url(&self) -> &'static str {
        match self {
            Network::Decaf => "https://sepolia.etherscan.io",
            Network::Hoodi => "https://hoodi.etherscan.io",
            Network::Mainnet => "https://etherscan.io",
        }
    }

    pub(crate) fn display_order(&self) -> u8 {
        match self {
            Network::Mainnet => 0,
            Network::Decaf => 1,
            Network::Hoodi => 2,
        }
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Network::Decaf => f.write_str("decaf"),
            Network::Hoodi => f.write_str("hoodi"),
            Network::Mainnet => f.write_str("mainnet"),
        }
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
        env = "ESPRESSO_L1_PROVIDER",
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

    #[clap(long, help = "Write files even if deployment info is unchanged.")]
    force: bool,
}

pub(crate) fn get_crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub async fn run(migrated_envs: Vec<(&str, &str)>) -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    espresso_utils::env_compat::log_migrated_env_vars(&migrated_envs);

    let args = Args::parse();
    let crate_dir = get_crate_dir();

    let update_readme = if let Some(network) = args.network {
        let env_file = match args.env_file {
            Some(path) => path,
            None => crate_dir.join(format!("addresses/{}.env", network)),
        };

        let addresses = DeploymentAddresses::from_env_file(&env_file)
            .context("Failed to load addresses from env file")?;

        let rpc_url = match args.rpc_url {
            Some(url) => url,
            None => network.default_rpc_url(),
        };

        tracing::info!("Collecting deployment info for network: {network}");

        let collected = CollectedDeployment::collect(rpc_url, network, addresses)
            .await
            .context("Failed to collect deployment info")?;

        let has_custom_output = args.output.is_some();

        if args.stdout {
            print!("{}", collected.to_toml_string()?);
        } else {
            let output_path = match args.output {
                Some(path) => path,
                None => crate_dir.join(format!("deployments/{}.toml", network)),
            };

            collected
                .write_toml(&output_path, args.force)
                .context("Failed to write deployment info to file")?;
        }

        !args.stdout && !has_custom_output
    } else {
        if args.env_file.is_some() || args.output.is_some() || args.stdout {
            bail!("--env-file, --output, and --stdout are only valid with --network");
        }

        for network in Network::value_variants() {
            let env_file = crate_dir.join(format!("addresses/{}.env", network));

            let addresses = DeploymentAddresses::from_env_file(&env_file)
                .context("Failed to load addresses from env file")?;

            let rpc_url = match &args.rpc_url {
                Some(url) => url.clone(),
                None => network.default_rpc_url(),
            };

            tracing::info!("Collecting deployment info for network: {network}");

            let collected = CollectedDeployment::collect(rpc_url, *network, addresses)
                .await
                .context("Failed to collect deployment info")?;

            let output_path = crate_dir.join(format!("deployments/{}.toml", network));
            collected
                .write_toml(&output_path, args.force)
                .context("Failed to write deployment info to file")?;
        }
        true
    };

    if update_readme {
        update_readme_from_deployment_files()?;
    } else {
        tracing::info!("Skipping README update");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::Duration};

    use alloy::{
        node_bindings::Anvil,
        primitives::{Address, U256},
        providers::{Provider, ProviderBuilder, WalletProvider},
    };
    use espresso_contract_deployer::{
        Contract, Contracts, builder::DeployerArgsBuilder,
        network_config::light_client_genesis_from_stake_table,
    };
    use hotshot_state_prover::v3::mock_ledger::STAKE_TABLE_CAPACITY_FOR_TEST;

    use crate::{
        Network,
        addresses::{DeploymentAddresses, KnownAddresses},
        contracts::{
            AccessControlDeployment, CollectedDeployment, ContractType, DeploymentInfo,
            DeploymentQuerier, OwnableDeployment, TimelockDeployment,
        },
    };

    #[test]
    fn test_resolve_name_unknown_address() {
        let known = KnownAddresses(HashMap::new());
        let addr = Address::repeat_byte(0x42);
        let result = known.resolve(addr);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not a known address")
        );
    }

    #[test]
    fn test_resolve_name_known_address() {
        let addr = Address::repeat_byte(0x42);
        let known = KnownAddresses(HashMap::from([(addr, "my_multisig".to_string())]));
        assert_eq!(known.resolve(addr).unwrap(), "my_multisig");
    }

    #[test]
    fn test_build_known_addresses() {
        let multisig_addr = Address::repeat_byte(0x01);
        let ops_addr = Address::repeat_byte(0x02);
        let safe_addr = Address::repeat_byte(0x03);
        let addresses = DeploymentAddresses {
            multisigs: HashMap::from([("my_multisig".to_string(), multisig_addr)]),
            ops_timelock: Some(ops_addr),
            safe_exit_timelock: Some(safe_addr),
            ..Default::default()
        };
        let known = KnownAddresses::from_deployment(&addresses);
        assert_eq!(known.0.len(), 3);
        assert_eq!(known.0[&multisig_addr], "my_multisig");
        assert_eq!(known.0[&ops_addr], "ops_timelock");
        assert_eq!(known.0[&safe_addr], "safe_exit_timelock");
    }

    #[test]
    fn test_contract_type_display() {
        assert_eq!(ContractType::LightClient.to_string(), "LightClient");
        assert_eq!(ContractType::FeeContract.to_string(), "FeeContract");
        assert_eq!(ContractType::EspToken.to_string(), "EspToken");
        assert_eq!(ContractType::StakeTable.to_string(), "StakeTable");
        assert_eq!(ContractType::RewardClaim.to_string(), "RewardClaim");
    }

    #[test]
    fn test_network_display() {
        assert_eq!(Network::Decaf.to_string(), "decaf");
        assert_eq!(Network::Hoodi.to_string(), "hoodi");
        assert_eq!(Network::Mainnet.to_string(), "mainnet");
    }

    #[test_log::test(tokio::test)]
    async fn test_collect_deployment_info_with_deployed_contracts() -> anyhow::Result<()> {
        let anvil = Anvil::new().spawn();
        let provider = ProviderBuilder::new()
            .wallet(anvil.wallet().unwrap())
            .connect_http(anvil.endpoint_url());
        let deployer_address = provider.default_signer_address();

        let (genesis_state, genesis_stake) = light_client_genesis_from_stake_table(
            &Default::default(),
            STAKE_TABLE_CAPACITY_FOR_TEST,
        )
        .unwrap();

        let ops_delay = Duration::from_secs(100);
        let safe_exit_delay = Duration::from_secs(200);

        let mut contracts = Contracts::new();
        let args = DeployerArgsBuilder::default()
            .deployer(provider.clone())
            .rpc_url(anvil.endpoint_url())
            .mock_light_client(true)
            .genesis_lc_state(genesis_state)
            .genesis_st_state(genesis_stake)
            .blocks_per_epoch(100)
            .epoch_start_block(1)
            .multisig_pauser(deployer_address)
            .exit_escrow_period(U256::from(604800))
            .token_name("Espresso".to_string())
            .token_symbol("ESP".to_string())
            .initial_token_supply(U256::from(3590000000u64))
            .ops_timelock_delay(U256::from(ops_delay.as_secs()))
            .ops_timelock_admin(deployer_address)
            .ops_timelock_proposers(vec![deployer_address])
            .ops_timelock_executors(vec![deployer_address])
            .safe_exit_timelock_delay(U256::from(safe_exit_delay.as_secs()))
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

        let known = KnownAddresses(HashMap::from([
            (deployer_address, "test_multisig".to_string()),
            (ops_timelock_addr, "ops_timelock".to_string()),
            (safe_exit_timelock_addr, "safe_exit_timelock".to_string()),
        ]));
        let block_number = provider.get_block_number().await?;
        let querier = DeploymentQuerier::new(&provider, known, block_number);

        // Test each contract individually
        let stake_table_info = querier
            .query_access_control(stake_table_addr, ContractType::StakeTable)
            .await?;
        assert_eq!(
            stake_table_info,
            AccessControlDeployment::Deployed {
                address: stake_table_addr,
                default_admin_address: deployer_address,
                default_admin_name: "test_multisig".to_string(),
                version: "2.0.0".to_string(),
                pauser_address: deployer_address,
                pauser_name: "test_multisig".to_string(),
            }
        );

        let esp_token_info = querier
            .query_ownable(esp_token_addr, ContractType::EspToken)
            .await?;
        assert_eq!(
            esp_token_info,
            OwnableDeployment::Deployed {
                address: esp_token_addr,
                owner_address: deployer_address,
                owner_name: "test_multisig".to_string(),
                version: "2.0.0".to_string(),
            }
        );

        let light_client_info = querier
            .query_ownable(light_client_addr, ContractType::LightClient)
            .await?;
        assert_eq!(
            light_client_info,
            OwnableDeployment::Deployed {
                address: light_client_addr,
                owner_address: deployer_address,
                owner_name: "test_multisig".to_string(),
                version: "3.0.0".to_string(),
            }
        );

        let fee_contract_info = querier
            .query_ownable(fee_contract_addr, ContractType::FeeContract)
            .await?;
        assert_eq!(
            fee_contract_info,
            OwnableDeployment::Deployed {
                address: fee_contract_addr,
                owner_address: deployer_address,
                owner_name: "test_multisig".to_string(),
                version: "1.0.1".to_string(),
            }
        );

        let reward_claim_info = querier
            .query_access_control(reward_claim_addr, ContractType::RewardClaim)
            .await?;
        assert_eq!(
            reward_claim_info,
            AccessControlDeployment::Deployed {
                address: reward_claim_addr,
                default_admin_address: deployer_address,
                default_admin_name: "test_multisig".to_string(),
                version: "1.0.0".to_string(),
                pauser_address: deployer_address,
                pauser_name: "test_multisig".to_string(),
            }
        );

        // Test timelocks
        let ops_tl = querier.query_timelock(ops_timelock_addr).await?;
        assert_eq!(
            ops_tl,
            TimelockDeployment::Deployed {
                address: ops_timelock_addr,
                min_delay: ops_delay,
            }
        );

        let safe_tl = querier.query_timelock(safe_exit_timelock_addr).await?;
        assert_eq!(
            safe_tl,
            TimelockDeployment::Deployed {
                address: safe_exit_timelock_addr,
                min_delay: safe_exit_delay,
            }
        );

        Ok(())
    }

    #[test]
    fn test_generate_deployment_table_contracts() {
        let info = DeploymentInfo::for_test();
        let table = info.to_markdown_table();

        assert!(table.starts_with("### mainnet\n"));
        assert!(table.contains("| Contract | Address | Version | Owner | Pauser |"));
        assert!(table.contains("| StakeTable |"));
        assert!(table.contains("| 2.0.0 | ops_timelock | espresso_labs |"));
        assert!(table.contains("| EspToken | Not deployed |"));
        assert!(table.contains("| LightClient |"));
        assert!(table.contains("| 1.0.0 | espresso_labs | - |"));
        assert!(table.contains("etherscan.io/address/0x"));
    }

    #[test]
    fn test_generate_deployment_table_multisigs() {
        let info = DeploymentInfo::for_test();
        let table = info.to_markdown_table();

        assert!(table.contains("| Multisig | Address | Version | Threshold |"));
        assert!(table.contains("| espresso_labs |"));
        assert!(table.contains("| 1.4.1 | 3 |"));
    }

    #[test]
    fn test_generate_deployment_table_timelocks() {
        let info = DeploymentInfo::for_test();
        let table = info.to_markdown_table();

        assert!(table.contains("| Timelock | Address | Min Delay |"));
        assert!(table.contains("| ops_timelock |"));
        assert!(table.contains("| safe_exit_timelock | Not deployed |"));
    }

    #[test]
    fn test_generate_deployment_table_full_addresses() {
        let info = DeploymentInfo::for_test();
        let table = info.to_markdown_table();

        assert!(table.contains("0x1111111111111111111111111111111111111111"));
        assert!(!table.contains("..."));
    }

    #[test]
    fn test_write_deployment_info_unchanged() {
        let info = DeploymentInfo::for_test();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-unchanged.toml");

        let collected = CollectedDeployment {
            info: info.clone(),
            block_number: 100,
            block_timestamp: 1000,
        };
        collected.write_toml(&path, false).unwrap();
        let first_content = std::fs::read_to_string(&path).unwrap();
        assert!(first_content.starts_with("# fetched at block 100"));

        let collected2 = CollectedDeployment {
            info,
            block_number: 200,
            block_timestamp: 2000,
        };
        collected2.write_toml(&path, false).unwrap();
        let second_content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(first_content, second_content);
    }

    #[test]
    fn test_write_deployment_info_force() {
        let info = DeploymentInfo::for_test();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-force.toml");

        let collected = CollectedDeployment {
            info: info.clone(),
            block_number: 100,
            block_timestamp: 1000,
        };
        collected.write_toml(&path, false).unwrap();
        let first_content = std::fs::read_to_string(&path).unwrap();

        let collected2 = CollectedDeployment {
            info,
            block_number: 200,
            block_timestamp: 2000,
        };
        collected2.write_toml(&path, true).unwrap();
        let second_content = std::fs::read_to_string(&path).unwrap();
        assert_ne!(first_content, second_content);
        assert!(second_content.starts_with("# fetched at block 200"));
    }

    #[test]
    fn test_write_deployment_info_changed() {
        let mut info = DeploymentInfo::for_test();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-changed.toml");

        let collected = CollectedDeployment {
            info: info.clone(),
            block_number: 100,
            block_timestamp: 1000,
        };
        collected.write_toml(&path, false).unwrap();
        let first_content = std::fs::read_to_string(&path).unwrap();

        info.esp_token = OwnableDeployment::Deployed {
            address: Address::repeat_byte(0x44),
            owner_address: Address::repeat_byte(0x55),
            owner_name: "new_owner".to_string(),
            version: "1.0.0".to_string(),
        };

        let collected2 = CollectedDeployment {
            info,
            block_number: 200,
            block_timestamp: 2000,
        };
        collected2.write_toml(&path, false).unwrap();
        let second_content = std::fs::read_to_string(&path).unwrap();
        assert_ne!(first_content, second_content);
        assert!(second_content.starts_with("# fetched at block 200"));
    }

    #[test]
    fn test_load_addresses_empty_value_errors() {
        let dir = tempfile::tempdir().unwrap();
        let env_path = dir.path().join("test.env");
        std::fs::write(&env_path, "ESPRESSO_STAKE_TABLE_PROXY_ADDRESS=\n").unwrap();
        let result = DeploymentAddresses::from_env_file(&env_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_network_display_order() {
        assert!(Network::Mainnet.display_order() < Network::Decaf.display_order());
        assert!(Network::Decaf.display_order() < Network::Hoodi.display_order());
    }
}
