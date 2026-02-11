use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    path::{Path, PathBuf},
    time::Duration,
};

use alloy::{
    primitives::{Address, FixedBytes},
    providers::{Provider, ProviderBuilder},
    sol,
};
use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use humantime::format_duration;
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

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Network {
    Decaf,
    Hoodi,
    Mainnet,
}

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

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
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

/// Contract and governance addresses read from a per-network .env file.
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

/// Reverse map from address to human-readable name (multisigs + timelocks).
/// Used to validate that all contract role holders are tracked in the .env config.
#[derive(Debug, Clone)]
struct KnownAddresses(HashMap<Address, String>);

impl KnownAddresses {
    fn from_deployment(addresses: &DeploymentAddresses) -> Self {
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

    fn resolve(&self, addr: Address) -> Result<String> {
        self.0.get(&addr).cloned().ok_or_else(|| {
            anyhow::anyhow!(
                "Address {addr} is not a known address. The .env config may be missing a multisig \
                 or other contract."
            )
        })
    }

    fn keys(&self) -> impl Iterator<Item = &Address> {
        self.0.keys()
    }
}

#[derive(Debug, Clone, Copy)]
enum ContractType {
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

enum AccessControlRole {
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
struct RoleHolder {
    address: Address,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
enum ContractDeployment {
    Deployed {
        address: Address,
        owner_address: Address,
        owner_name: String,
        version: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pauser_address: Option<Address>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pauser_name: Option<String>,
    },
    NotYetDeployed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MultisigDeployment {
    address: Address,
    version: String,
    owners: Vec<Address>,
    threshold: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
enum TimelockDeployment {
    Deployed { address: Address, min_delay: String },
    NotYetDeployed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct DeploymentInfo {
    network: Network,
    multisigs: BTreeMap<String, MultisigDeployment>,
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
        .filter_map(|item| {
            item.map_err(|e| tracing::warn!("Invalid line in env file {:?}: {}", path, e))
                .ok()
        })
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
    for key in env_map.keys() {
        if let Some(name) = key
            .strip_prefix(MULTISIG_PREFIX)
            .and_then(|s| s.strip_suffix(MULTISIG_SUFFIX))
        {
            let name = name.to_lowercase();
            if let Some(addr) = parse_address(&env_map, key) {
                multisigs.insert(name, addr);
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

struct DeploymentQuerier<P: Provider> {
    provider: P,
    known: KnownAddresses,
}

impl<P: Provider> DeploymentQuerier<P> {
    fn new(provider: P, known: KnownAddresses) -> Self {
        Self { provider, known }
    }

    async fn get_owner(&self, addr: Address, contract_type: ContractType) -> Result<Address> {
        match contract_type {
            ContractType::StakeTable | ContractType::RewardClaim => self
                .find_role_holder(addr, AccessControlRole::DefaultAdmin)
                .await
                .context(format!("owner of {contract_type}")),
            _ => {
                let contract = IOwnable::new(addr, &self.provider);
                Ok(contract.owner().call().await?)
            },
        }
    }

    /// Finds which known address holds the given role. Errors if the holder is not
    /// in `self.known` -- this validates that all role holders are tracked in the .env config.
    async fn find_role_holder(
        &self,
        contract_addr: Address,
        role: AccessControlRole,
    ) -> Result<Address> {
        let contract = IAccessControl::new(contract_addr, &self.provider);
        let role_hash = role.hash();
        let mut holders = Vec::new();
        for addr in self.known.keys() {
            let has_role = contract.hasRole(role_hash, *addr).call().await?;
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

    async fn get_pauser(
        &self,
        contract_addr: Address,
        contract_type: ContractType,
    ) -> Result<Option<Address>> {
        match contract_type {
            ContractType::StakeTable => self
                .find_role_holder(contract_addr, AccessControlRole::Pauser)
                .await
                .map(Some)
                .context(format!("pauser of {contract_type}")),
            ContractType::RewardClaim => {
                match self
                    .find_role_holder(contract_addr, AccessControlRole::Pauser)
                    .await
                {
                    Ok(addr) => Ok(Some(addr)),
                    Err(e) => {
                        tracing::warn!("PAUSER_ROLE lookup failed, older RewardClaim version: {e}");
                        Ok(None)
                    },
                }
            },
            _ => Ok(None),
        }
    }

    fn resolve_role_holder(&self, addr: Address, context: &str) -> Result<RoleHolder> {
        let name = if addr == Address::ZERO {
            tracing::debug!("  {context} is zero address, using \"none\"");
            "none".to_string()
        } else {
            self.known.resolve(addr).context(context.to_string())?
        };
        Ok(RoleHolder {
            address: addr,
            name,
        })
    }

    async fn query_contract(
        &self,
        addr: Address,
        contract_type: ContractType,
    ) -> Result<ContractDeployment> {
        tracing::info!("querying {contract_type} at {addr}");

        tracing::debug!("  fetching owner");
        let owner_addr = self.get_owner(addr, contract_type).await?;

        tracing::debug!("  fetching version");
        let v = IVersioned::new(addr, &self.provider)
            .getVersion()
            .call()
            .await?;
        let version = format!("{}.{}.{}", v._0, v._1, v._2);

        let owner = self.resolve_role_holder(owner_addr, &format!("owner of {contract_type}"))?;

        tracing::debug!("  fetching pauser");
        let pauser_addr = self.get_pauser(addr, contract_type).await?;
        let pauser = pauser_addr
            .map(|pa| self.resolve_role_holder(pa, &format!("pauser of {contract_type}")))
            .transpose()?;

        let pauser_display = pauser.as_ref().map(|p| p.name.as_str()).unwrap_or("none");
        tracing::info!(
            "  owner={} version={version} pauser={pauser_display}",
            owner.name
        );

        Ok(ContractDeployment::Deployed {
            address: addr,
            owner_address: owner.address,
            owner_name: owner.name,
            version,
            pauser_address: pauser.as_ref().map(|p| p.address),
            pauser_name: pauser.map(|p| p.name),
        })
    }

    async fn query_optional(
        &self,
        addr: Option<Address>,
        contract_type: ContractType,
    ) -> Result<ContractDeployment> {
        match addr {
            Some(addr) => self
                .query_contract(addr, contract_type)
                .await
                .with_context(|| format!("Failed to query {contract_type} at {addr}")),
            None => Ok(ContractDeployment::NotYetDeployed),
        }
    }
}

async fn get_timelock_info<P: Provider>(provider: &P, addr: Address) -> Result<TimelockDeployment> {
    let min_delay_secs: u64 = ITimelock::new(addr, provider)
        .getMinDelay()
        .call()
        .await?
        .try_into()
        .context("min_delay exceeds u64")?;
    let min_delay = format_duration(Duration::from_secs(min_delay_secs)).to_string();

    Ok(TimelockDeployment::Deployed {
        address: addr,
        min_delay,
    })
}

async fn collect_deployment_info(
    rpc_url: Url,
    network: Network,
    addresses: DeploymentAddresses,
) -> Result<DeploymentInfo> {
    let provider = ProviderBuilder::new().connect_http(rpc_url);
    let known = KnownAddresses::from_deployment(&addresses);
    let querier = DeploymentQuerier::new(provider.clone(), known);

    let mut multisigs = BTreeMap::new();
    for (name, addr) in &addresses.multisigs {
        let contract = ISafe::new(*addr, &provider);
        let version =
            contract.VERSION().call().await.with_context(|| {
                format!("Failed to get VERSION for multisig '{name}' at {addr}")
            })?;
        let owners = contract
            .getOwners()
            .call()
            .await
            .with_context(|| format!("Failed to get owners for multisig '{name}' at {addr}"))?;
        let threshold: u64 = contract
            .getThreshold()
            .call()
            .await
            .with_context(|| format!("Failed to get threshold for multisig '{name}' at {addr}"))?
            .try_into()
            .context("threshold exceeds u64")?;
        multisigs.insert(
            name.clone(),
            MultisigDeployment {
                address: *addr,
                version,
                owners,
                threshold,
            },
        );
    }

    let ops_timelock = match addresses.ops_timelock {
        Some(addr) => get_timelock_info(&provider, addr)
            .await
            .with_context(|| format!("Failed to query OpsTimelock at {addr}"))?,
        None => TimelockDeployment::NotYetDeployed,
    };

    let safe_exit_timelock = match addresses.safe_exit_timelock {
        Some(addr) => get_timelock_info(&provider, addr)
            .await
            .with_context(|| format!("Failed to query SafeExitTimelock at {addr}"))?,
        None => TimelockDeployment::NotYetDeployed,
    };

    let stake_table = querier
        .query_optional(addresses.stake_table, ContractType::StakeTable)
        .await?;
    let esp_token = querier
        .query_optional(addresses.esp_token, ContractType::EspToken)
        .await?;
    let light_client = querier
        .query_optional(addresses.light_client, ContractType::LightClient)
        .await?;
    let fee_contract = querier
        .query_optional(addresses.fee_contract, ContractType::FeeContract)
        .await?;
    let reward_claim = querier
        .query_optional(addresses.reward_claim, ContractType::RewardClaim)
        .await?;

    Ok(DeploymentInfo {
        network,
        multisigs,
        ops_timelock,
        safe_exit_timelock,
        stake_table,
        esp_token,
        light_client,
        fee_contract,
        reward_claim,
    })
}

fn write_deployment_info(info: &DeploymentInfo, output_path: &Path) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create output directory")?;
    }

    let toml_output = toml::to_string_pretty(info)?;
    std::fs::write(output_path, toml_output).context("Failed to write deployment info")?;

    Ok(())
}

async fn process_network(
    network: Network,
    rpc_url: Option<&Url>,
    env_file: Option<&Path>,
    output: Option<&Path>,
    stdout: bool,
) -> Result<()> {
    let crate_dir = get_crate_dir();

    let env_file = match env_file {
        Some(path) => path.to_path_buf(),
        None => crate_dir.join(format!("addresses/{}.env", network)),
    };

    let addresses = load_addresses_from_env_file(&env_file)
        .context("Failed to load addresses from env file")?;

    let rpc_url = match rpc_url {
        Some(url) => url.clone(),
        None => network.default_rpc_url(),
    };

    tracing::info!("Collecting deployment info for network: {network}");

    let info = collect_deployment_info(rpc_url, network, addresses)
        .await
        .context("Failed to collect deployment info")?;

    if stdout {
        let toml_output = toml::to_string_pretty(&info)?;
        println!("{}", toml_output);
    } else {
        let output_path = match output {
            Some(path) => path.to_path_buf(),
            None => crate_dir.join(format!("deployments/{}.toml", network)),
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
            args.env_file.as_deref(),
            args.output.as_deref(),
            args.stdout,
        )
        .await?;
    } else {
        if args.env_file.is_some() || args.output.is_some() || args.stdout {
            bail!("--env-file, --output, and --stdout are only valid with --network");
        }

        for network in Network::value_variants() {
            process_network(*network, args.rpc_url.as_ref(), None, None, false).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use alloy::{
        node_bindings::Anvil,
        primitives::U256,
        providers::{ProviderBuilder, WalletProvider},
    };
    use espresso_contract_deployer::{
        builder::DeployerArgsBuilder, network_config::light_client_genesis_from_stake_table,
        Contract, Contracts,
    };
    use hotshot_state_prover::v3::mock_ledger::STAKE_TABLE_CAPACITY_FOR_TEST;

    use super::*;

    #[test]
    fn test_resolve_name_unknown_address() {
        let known = KnownAddresses(HashMap::new());
        let addr = Address::repeat_byte(0x42);
        let result = known.resolve(addr);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not a known address"));
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
    async fn test_collect_deployment_info_with_deployed_contracts() -> Result<()> {
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
            .ops_timelock_delay(U256::from(100))
            .ops_timelock_admin(deployer_address)
            .ops_timelock_proposers(vec![deployer_address])
            .ops_timelock_executors(vec![deployer_address])
            .safe_exit_timelock_delay(U256::from(200))
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
        let querier = DeploymentQuerier::new(provider.clone(), known.clone());

        // Test each contract individually
        let stake_table_info = querier
            .query_contract(stake_table_addr, ContractType::StakeTable)
            .await?;
        assert_eq!(
            stake_table_info,
            ContractDeployment::Deployed {
                address: stake_table_addr,
                owner_address: deployer_address,
                owner_name: "test_multisig".to_string(),
                version: "2.0.0".to_string(),
                pauser_address: Some(deployer_address),
                pauser_name: Some("test_multisig".to_string()),
            }
        );

        let esp_token_info = querier
            .query_contract(esp_token_addr, ContractType::EspToken)
            .await?;
        assert_eq!(
            esp_token_info,
            ContractDeployment::Deployed {
                address: esp_token_addr,
                owner_address: deployer_address,
                owner_name: "test_multisig".to_string(),
                version: "2.0.0".to_string(),
                pauser_address: None,
                pauser_name: None,
            }
        );

        let light_client_info = querier
            .query_contract(light_client_addr, ContractType::LightClient)
            .await?;
        assert_eq!(
            light_client_info,
            ContractDeployment::Deployed {
                address: light_client_addr,
                owner_address: deployer_address,
                owner_name: "test_multisig".to_string(),
                version: "3.0.0".to_string(),
                pauser_address: None,
                pauser_name: None,
            }
        );

        let fee_contract_info = querier
            .query_contract(fee_contract_addr, ContractType::FeeContract)
            .await?;
        assert_eq!(
            fee_contract_info,
            ContractDeployment::Deployed {
                address: fee_contract_addr,
                owner_address: deployer_address,
                owner_name: "test_multisig".to_string(),
                version: "1.0.1".to_string(),
                pauser_address: None,
                pauser_name: None,
            }
        );

        let reward_claim_info = querier
            .query_contract(reward_claim_addr, ContractType::RewardClaim)
            .await?;
        assert_eq!(
            reward_claim_info,
            ContractDeployment::Deployed {
                address: reward_claim_addr,
                owner_address: deployer_address,
                owner_name: "test_multisig".to_string(),
                version: "1.0.0".to_string(),
                pauser_address: Some(deployer_address),
                pauser_name: Some("test_multisig".to_string()),
            }
        );

        // Test timelocks
        let ops_tl = get_timelock_info(&provider, ops_timelock_addr).await?;
        assert_eq!(
            ops_tl,
            TimelockDeployment::Deployed {
                address: ops_timelock_addr,
                min_delay: "1m 40s".to_string()
            }
        );

        let safe_tl = get_timelock_info(&provider, safe_exit_timelock_addr).await?;
        assert_eq!(
            safe_tl,
            TimelockDeployment::Deployed {
                address: safe_exit_timelock_addr,
                min_delay: "3m 20s".to_string()
            }
        );

        Ok(())
    }
}
