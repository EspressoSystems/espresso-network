use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    path::{Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};

use alloy::{
    eips::BlockId,
    primitives::{Address, FixedBytes},
    providers::{Provider, ProviderBuilder},
    sol,
};
use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
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
    fn default_rpc_url(&self) -> Url {
        match self {
            Network::Decaf => "https://ethereum-sepolia-rpc.publicnode.com",
            Network::Hoodi => "https://ethereum-hoodi-rpc.publicnode.com",
            Network::Mainnet => "https://ethereum-rpc.publicnode.com",
        }
        .parse()
        .expect("hardcoded URL is valid")
    }

    fn etherscan_base_url(&self) -> &'static str {
        match self {
            Network::Decaf => "https://sepolia.etherscan.io",
            Network::Hoodi => "https://hoodi.etherscan.io",
            Network::Mainnet => "https://etherscan.io",
        }
    }

    fn display_order(&self) -> u8 {
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

    #[clap(long, help = "Write files even if deployment info is unchanged.")]
    force: bool,
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
enum OwnableDeployment {
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
enum AccessControlDeployment {
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
struct MultisigDeployment {
    address: Address,
    version: String,
    owners: Vec<Address>,
    threshold: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
enum TimelockDeployment {
    Deployed {
        address: Address,
        #[serde(with = "humantime_serde")]
        min_delay: Duration,
    },
    NotYetDeployed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct DeploymentInfo {
    network: Network,
    multisigs: BTreeMap<String, MultisigDeployment>,
    ops_timelock: TimelockDeployment,
    safe_exit_timelock: TimelockDeployment,
    esp_token: OwnableDeployment,
    fee_contract: OwnableDeployment,
    light_client: OwnableDeployment,
    reward_claim: AccessControlDeployment,
    stake_table: AccessControlDeployment,
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

    let mut multisigs = HashMap::new();
    for key in env_map.keys() {
        if let Some(name) = key
            .strip_prefix(MULTISIG_PREFIX)
            .and_then(|s| s.strip_suffix(MULTISIG_SUFFIX))
        {
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

struct DeploymentQuerier<'a, P: Provider> {
    provider: &'a P,
    known: KnownAddresses,
    block_id: BlockId,
}

impl<'a, P: Provider> DeploymentQuerier<'a, P> {
    fn new(provider: &'a P, known: KnownAddresses, block_number: u64) -> Self {
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

    async fn query_ownable(
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

    async fn query_access_control(
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

    async fn query_timelock(&self, addr: Address) -> Result<TimelockDeployment> {
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

struct CollectedDeployment {
    info: DeploymentInfo,
    block_number: u64,
    block_timestamp: u64,
}

async fn collect_deployment_info(
    rpc_url: Url,
    network: Network,
    addresses: DeploymentAddresses,
) -> Result<CollectedDeployment> {
    let provider = ProviderBuilder::new().connect_http(rpc_url);

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

fn format_header_comment(block_number: u64, block_timestamp: u64) -> String {
    let system_time = UNIX_EPOCH + Duration::from_secs(block_timestamp);
    let formatted = humantime::format_rfc3339_seconds(system_time);
    format!("# fetched at block {block_number} ({formatted})\n")
}

fn write_deployment_info(
    info: &DeploymentInfo,
    block_number: u64,
    block_timestamp: u64,
    output_path: &Path,
    force: bool,
) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create output directory")?;
    }

    if !force && output_path.exists() {
        let existing =
            std::fs::read_to_string(output_path).context("Failed to read existing file")?;
        let existing_info: DeploymentInfo =
            toml::from_str(&existing).context("Failed to parse existing deployment file")?;
        if existing_info == *info {
            tracing::info!(
                "{:?}: deployment info unchanged, skipping write",
                output_path.file_name().unwrap_or_default()
            );
            return Ok(());
        }
    }

    let header = format_header_comment(block_number, block_timestamp);
    let toml_data = toml::to_string_pretty(info)?;
    let output = format!("{header}{toml_data}");
    std::fs::write(output_path, output).context("Failed to write deployment info")?;
    tracing::info!("Wrote: {:?}", output_path);

    Ok(())
}

async fn process_network(
    network: Network,
    rpc_url: Option<&Url>,
    env_file: Option<&Path>,
    output: Option<&Path>,
    stdout: bool,
    force: bool,
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

    let collected = collect_deployment_info(rpc_url, network, addresses)
        .await
        .context("Failed to collect deployment info")?;

    if stdout {
        let header = format_header_comment(collected.block_number, collected.block_timestamp);
        let toml_output = toml::to_string_pretty(&collected.info)?;
        print!("{header}{toml_output}");
    } else {
        let output_path = match output {
            Some(path) => path.to_path_buf(),
            None => crate_dir.join(format!("deployments/{}.toml", network)),
        };

        write_deployment_info(
            &collected.info,
            collected.block_number,
            collected.block_timestamp,
            &output_path,
            force,
        )
        .context("Failed to write deployment info to file")?;
    }

    Ok(())
}

fn address_link(addr: Address, etherscan_url: &str) -> String {
    format!("[`{addr}`]({etherscan_url}/address/{addr})")
}

fn contract_row(
    name: &str,
    address: Address,
    version: &str,
    owner: &str,
    pauser: &str,
    etherscan: &str,
) -> String {
    format!(
        "| {name} | {} | {version} | {owner} | {pauser} |\n",
        address_link(address, etherscan),
    )
}

fn generate_deployment_table(info: &DeploymentInfo) -> String {
    let etherscan = info.network.etherscan_base_url();
    let mut out = format!("### {}\n\n", info.network);

    out.push_str("| Contract | Address | Version | Owner | Pauser |\n");
    out.push_str("|----------|---------|---------|-------|--------|\n");

    // Alphabetical order
    for (name, deployment) in [
        ("EspToken", &info.esp_token),
        ("FeeContract", &info.fee_contract),
        ("LightClient", &info.light_client),
    ] {
        match deployment {
            OwnableDeployment::Deployed {
                address,
                owner_name,
                version,
                ..
            } => out.push_str(&contract_row(
                name, *address, version, owner_name, "-", etherscan,
            )),
            OwnableDeployment::NotYetDeployed => {
                out.push_str(&format!("| {name} | Not deployed | | | |\n"))
            },
        }
    }
    for (name, deployment) in [
        ("RewardClaim", &info.reward_claim),
        ("StakeTable", &info.stake_table),
    ] {
        match deployment {
            AccessControlDeployment::Deployed {
                address,
                default_admin_name,
                version,
                pauser_name,
                ..
            } => out.push_str(&contract_row(
                name,
                *address,
                version,
                default_admin_name,
                pauser_name,
                etherscan,
            )),
            AccessControlDeployment::NotYetDeployed => {
                out.push_str(&format!("| {name} | Not deployed | | | |\n"))
            },
        }
    }

    if !info.multisigs.is_empty() {
        out.push('\n');
        out.push_str("| Multisig | Address | Version | Threshold |\n");
        out.push_str("|----------|---------|---------|----------|\n");
        for (name, ms) in &info.multisigs {
            out.push_str(&format!(
                "| {name} | {} | {} | {} |\n",
                address_link(ms.address, etherscan),
                ms.version,
                ms.threshold,
            ));
        }
    }

    let timelocks: &[(&str, &TimelockDeployment)] = &[
        ("ops_timelock", &info.ops_timelock),
        ("safe_exit_timelock", &info.safe_exit_timelock),
    ];

    let has_timelocks = timelocks
        .iter()
        .any(|(_, tl)| matches!(tl, TimelockDeployment::Deployed { .. }));

    if has_timelocks {
        out.push('\n');
        out.push_str("| Timelock | Address | Min Delay |\n");
        out.push_str("|---------|---------|----------|\n");
        for (name, tl) in timelocks {
            match tl {
                TimelockDeployment::Deployed {
                    address, min_delay, ..
                } => {
                    out.push_str(&format!(
                        "| {name} | {} | {} |\n",
                        address_link(*address, etherscan),
                        humantime::format_duration(*min_delay),
                    ));
                },
                TimelockDeployment::NotYetDeployed => {
                    out.push_str(&format!("| {name} | Not deployed | |\n"));
                },
            }
        }
    }

    out
}

fn replace_between_markers(
    content: &str,
    start_marker: &str,
    end_marker: &str,
    replacement: &str,
) -> Result<String> {
    let start = content.find(start_marker).context("Missing start marker")?;
    let end = content.find(end_marker).context("Missing end marker")?;
    if end < start + start_marker.len() {
        bail!("End marker appears before start marker");
    }
    Ok(format!(
        "{}{start_marker}\n<!-- prettier-ignore-start -->\n{replacement}<!-- prettier-ignore-end \
         -->\n{end_marker}{}",
        &content[..start],
        &content[end + end_marker.len()..],
    ))
}

fn update_readme_from_deployment_files() -> Result<()> {
    let crate_dir = get_crate_dir();
    let deployments_dir = crate_dir.join("deployments");
    let readme_path = crate_dir.join("README.md");

    let mut deployments = Vec::new();
    for entry in std::fs::read_dir(&deployments_dir)
        .context("Failed to read deployments directory")?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "toml"))
    {
        let path = entry.path();
        let content =
            std::fs::read_to_string(&path).with_context(|| format!("Failed to read {:?}", path))?;
        let info: DeploymentInfo =
            toml::from_str(&content).with_context(|| format!("Failed to parse {:?}", path))?;
        deployments.push(info);
    }
    deployments.sort_by_key(|info| info.network.display_order());

    let sections: Vec<_> = deployments.iter().map(generate_deployment_table).collect();
    let combined = sections.join("\n");

    let readme = std::fs::read_to_string(&readme_path).context("Failed to read README.md")?;
    let new_readme = replace_between_markers(
        &readme,
        "<!-- DEPLOYMENT_TABLE_START -->",
        "<!-- DEPLOYMENT_TABLE_END -->",
        &combined,
    )
    .context("README.md marker error")?;

    if readme == new_readme {
        tracing::info!("README.md unchanged, skipping write");
        return Ok(());
    }

    std::fs::write(&readme_path, new_readme).context("Failed to write README.md")?;
    tracing::info!("Updated README.md with deployment tables");

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
    let update_readme = if let Some(network) = args.network {
        process_network(
            network,
            args.rpc_url.as_ref(),
            args.env_file.as_deref(),
            args.output.as_deref(),
            args.stdout,
            args.force,
        )
        .await?;
        !args.stdout && args.output.is_none()
    } else {
        if args.env_file.is_some() || args.output.is_some() || args.stdout {
            bail!("--env-file, --output, and --stdout are only valid with --network");
        }

        for network in Network::value_variants() {
            process_network(
                *network,
                args.rpc_url.as_ref(),
                None,
                None,
                false,
                args.force,
            )
            .await?;
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
    fn test_format_header_comment() {
        let comment = format_header_comment(12345678, 1705312235);
        assert!(comment.starts_with("# fetched at block 12345678 ("));
        assert!(comment.ends_with(")\n"));
        assert!(comment.contains("2024-01-15"));
    }

    #[test]
    fn test_address_link() {
        let addr: Address = "0x1111111111111111111111111111111111111111"
            .parse()
            .unwrap();
        let link = address_link(addr, "https://etherscan.io");
        assert_eq!(
            link,
            "[`0x1111111111111111111111111111111111111111`](https://etherscan.io/address/0x1111111111111111111111111111111111111111)"
        );
    }

    #[test]
    fn test_replace_between_markers() {
        let content = "before\n<!-- START -->\nold content\n<!-- END -->\nafter\n";
        let result =
            replace_between_markers(content, "<!-- START -->", "<!-- END -->", "new\n").unwrap();
        assert_eq!(
            result,
            "before\n<!-- START -->\n<!-- prettier-ignore-start -->\nnew\n<!-- \
             prettier-ignore-end -->\n<!-- END -->\nafter\n"
        );
    }

    #[test]
    fn test_replace_between_markers_missing_start() {
        let content = "no markers here";
        let result = replace_between_markers(content, "<!-- START -->", "<!-- END -->", "x");
        assert!(result.is_err());
    }

    #[test]
    fn test_replace_between_markers_reversed() {
        let content = "<!-- END -->\n<!-- START -->";
        let result = replace_between_markers(content, "<!-- START -->", "<!-- END -->", "x");
        assert!(result.is_err());
    }

    impl DeploymentInfo {
        fn for_test() -> Self {
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
                network: Network::Mainnet,
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

    #[test]
    fn test_generate_deployment_table_contracts() {
        let info = DeploymentInfo::for_test();
        let table = generate_deployment_table(&info);

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
        let table = generate_deployment_table(&info);

        assert!(table.contains("| Multisig | Address | Version | Threshold |"));
        assert!(table.contains("| espresso_labs |"));
        assert!(table.contains("| 1.4.1 | 3 |"));
    }

    #[test]
    fn test_generate_deployment_table_timelocks() {
        let info = DeploymentInfo::for_test();
        let table = generate_deployment_table(&info);

        assert!(table.contains("| Timelock | Address | Min Delay |"));
        assert!(table.contains("| ops_timelock |"));
        assert!(table.contains("| safe_exit_timelock | Not deployed |"));
    }

    #[test]
    fn test_generate_deployment_table_full_addresses() {
        let info = DeploymentInfo::for_test();
        let table = generate_deployment_table(&info);

        assert!(table.contains("0x1111111111111111111111111111111111111111"));
        assert!(!table.contains("..."));
    }

    #[test]
    fn test_write_deployment_info_unchanged() {
        let info = DeploymentInfo::for_test();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-unchanged.toml");

        write_deployment_info(&info, 100, 1000, &path, false).unwrap();
        let first_content = std::fs::read_to_string(&path).unwrap();
        assert!(first_content.starts_with("# fetched at block 100"));

        write_deployment_info(&info, 200, 2000, &path, false).unwrap();
        let second_content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(first_content, second_content);
    }

    #[test]
    fn test_write_deployment_info_force() {
        let info = DeploymentInfo::for_test();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-force.toml");

        write_deployment_info(&info, 100, 1000, &path, false).unwrap();
        let first_content = std::fs::read_to_string(&path).unwrap();

        write_deployment_info(&info, 200, 2000, &path, true).unwrap();
        let second_content = std::fs::read_to_string(&path).unwrap();
        assert_ne!(first_content, second_content);
        assert!(second_content.starts_with("# fetched at block 200"));
    }

    #[test]
    fn test_write_deployment_info_changed() {
        let mut info = DeploymentInfo::for_test();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-changed.toml");

        write_deployment_info(&info, 100, 1000, &path, false).unwrap();
        let first_content = std::fs::read_to_string(&path).unwrap();

        info.esp_token = OwnableDeployment::Deployed {
            address: Address::repeat_byte(0x44),
            owner_address: Address::repeat_byte(0x55),
            owner_name: "new_owner".to_string(),
            version: "1.0.0".to_string(),
        };

        write_deployment_info(&info, 200, 2000, &path, false).unwrap();
        let second_content = std::fs::read_to_string(&path).unwrap();
        assert_ne!(first_content, second_content);
        assert!(second_content.starts_with("# fetched at block 200"));
    }

    #[test]
    fn test_load_addresses_empty_value_errors() {
        let dir = tempfile::tempdir().unwrap();
        let env_path = dir.path().join("test.env");
        std::fs::write(&env_path, "ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS=\n").unwrap();
        let result = load_addresses_from_env_file(&env_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_network_display_order() {
        assert!(Network::Mainnet.display_order() < Network::Decaf.display_order());
        assert!(Network::Decaf.display_order() < Network::Hoodi.display_order());
    }
}
