use std::{fs::File, io::stdout, path::PathBuf, thread::sleep, time::Duration};

use alloy::{
    primitives::{
        utils::{format_ether, parse_ether},
        Address, U256,
    },
    providers::{Provider, WalletProvider},
};
use anyhow::Context as _;
use clap::{Parser, Subcommand};
use espresso_contract_deployer::{
    build_provider, build_provider_ledger,
    builder::DeployerArgsBuilder,
    network_config::{light_client_genesis, light_client_genesis_from_stake_table},
    proposals::{multisig::verify_node_js_files, timelock::TimelockOperationType},
    provider::connect_ledger,
    Contract, Contracts, DeployedContracts,
};
use espresso_types::{config::PublicNetworkConfig, parse_duration};
use hotshot_types::light_client::DEFAULT_STAKE_TABLE_CAPACITY;
use sequencer_utils::logging;
use tide_disco::error::ServerError;
use url::Url;
use vbs::version::StaticVersion;

/// Deploy contracts needed to run the sequencer.
///
/// This script deploys contracts needed to run the sequencer to an L1. It outputs a .env file
/// containing the addresses of the deployed contracts.
///
/// This script can also be used to do incremental deployments. The only contract addresses
/// needed to configure the sequencer network are ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS,
/// ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS and ESPRESSO_SEQUENCER_STAKE_TABLE_ADDRESS.
/// These contracts, however, have dependencies, and a full deployment involves several
/// contracts. Some of these contracts, especially libraries may already have been deployed, or
/// perhaps one of the top-level contracts has been deployed and we only need to deploy the other
/// one.
///
/// It is possible to pass in the addresses of already deployed contracts, in which case those
/// addresses will be used in place of deploying a new contract wherever that contract is required
/// in the deployment process. The generated .env file will include all the addresses passed in as
/// well as those newly deployed.
#[derive(Clone, Debug, Parser)]

struct Options {
    /// A JSON-RPC endpoint for the L1 to deploy to.
    #[clap(
        short,
        long,
        env = "ESPRESSO_SEQUENCER_L1_PROVIDER",
        default_value = "http://localhost:8545"
    )]
    rpc_url: Url,

    /// Request rate when polling L1.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_L1_POLLING_INTERVAL",
        default_value = "7s",
        value_parser = parse_duration,
    )]
    pub l1_polling_interval: Duration,

    /// URL of a sequencer node that is currently providing the HotShot config.
    /// This is used to initialize the stake table.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_URL",
        default_value = "http://localhost:24000"
    )]
    pub sequencer_url: Url,

    /// Mnemonic for an L1 wallet.
    ///
    /// This wallet is used to deploy the contracts, so the account indicated by ACCOUNT_INDEX must
    /// be funded with with ETH.
    #[clap(
        long,
        name = "MNEMONIC",
        env = "ESPRESSO_SEQUENCER_ETH_MNEMONIC",
        default_value = "test test test test test test test test test test test junk",
        conflicts_with = "LEDGER"
    )]
    mnemonic: Option<String>,

    /// Address for the multisig wallet that will be the admin
    ///
    /// If provided, this the multisig wallet that will be able to upgrade contracts and execute
    /// admin only functions on contracts. If not provided, admin power for all contracts will be
    /// held by the account used to deploy the contracts (determined from MNEMONIC, ACCOUNT_INDEX).
    #[clap(
        long,
        name = "MULTISIG_ADDRESS",
        env = "ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS"
    )]
    multisig_address: Option<Address>,

    /// Address for the multisig wallet that will be the pauser
    ///
    /// The multisig pauser can pause functions in contracts that have the `whenNotPaused` modifier
    #[clap(
        long,
        name = "MULTISIG_PAUSER_ADDRESS",
        env = "ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS"
    )]
    multisig_pauser_address: Option<Address>,

    /// Account index in the L1 wallet generated by MNEMONIC to use when deploying the contracts.
    #[clap(
        long,
        name = "ACCOUNT_INDEX",
        env = "ESPRESSO_DEPLOYER_ACCOUNT_INDEX",
        default_value = "0"
    )]
    account_index: u32,

    /// Use a ledger device to sign transactions.
    ///
    /// NOTE: ledger must be unlocked, Ethereum app open and blind signing must be enabled in the
    /// Ethereum app settings.
    #[clap(
        long,
        name = "LEDGER",
        env = "ESPRESSO_DEPLOYER_USE_LEDGER",
        conflicts_with = "MNEMONIC"
    )]
    ledger: bool,

    /// Option to deploy fee contracts
    #[clap(long, default_value = "false")]
    deploy_fee: bool,
    /// Option to deploy LightClient V1 and proxy
    #[clap(long, default_value = "false")]
    deploy_light_client_v1: bool,
    /// Option to upgrade to LightClient V2
    #[clap(long, default_value = "false")]
    upgrade_light_client_v2: bool,
    /// Option to deploy esp token
    #[clap(long, default_value = "false")]
    deploy_esp_token: bool,
    /// Option to upgrade esp token v2
    #[clap(long, default_value = "false")]
    upgrade_esp_token_v2: bool,
    /// Option to deploy StakeTable V1 and proxy
    #[clap(long, default_value = "false")]
    deploy_stake_table: bool,
    /// Option to upgrade to StakeTable V2
    #[clap(long, default_value = "false")]
    upgrade_stake_table_v2: bool,
    /// Option to deploy ops timelock
    #[clap(long, default_value = "false")]
    deploy_ops_timelock: bool,
    /// Option to deploy safe exit timelock
    #[clap(long, default_value = "false")]
    deploy_safe_exit_timelock: bool,
    /// Option to use timelock as the owner of the proxy
    #[clap(long, default_value = "false")]
    use_timelock_owner: bool,
    /// Option to transfer ownership from multisig
    #[clap(long, default_value = "false")]
    propose_transfer_ownership_to_timelock: bool,

    /// Option to transfer ownership directly from EOA to a new owner
    #[clap(long, default_value = "false")]
    transfer_ownership_from_eoa: bool,

    /// The new owner address (when using --transfer-ownership-from-eoa)
    #[clap(long, env = "ESPRESSO_TRANSFER_OWNERSHIP_NEW_OWNER")]
    transfer_ownership_new_owner: Option<Address>,

    /// Write deployment results to OUT as a .env file.
    ///
    /// If not provided, the results will be written to stdout.
    #[clap(short, long, name = "OUT", env = "ESPRESSO_DEPLOYER_OUT_PATH")]
    out: Option<PathBuf>,

    #[clap(flatten)]
    contracts: DeployedContracts,

    /// If toggled, launch a mock LightClient contract with a smaller verification key for testing.
    /// Applies to both V1 and V2 of LightClient.
    #[clap(short, long)]
    pub use_mock: bool,

    /// Option to deploy contracts owned by multisig
    #[clap(long, default_value = "false")]
    pub use_multisig: bool,

    /// Option to test upgrade stake table v2 multisig owner dry run
    #[clap(long, default_value = "false")]
    pub dry_run: bool,

    /// Option to test locally but with a real eth network
    #[clap(long, default_value = "false")]
    pub mock_espresso_live_network: bool,

    /// Option to verify node js files access to upgrade stake table v2 multisig owner dry run
    #[clap(long, default_value = "false")]
    pub verify_node_js_files: bool,

    /// Stake table capacity for the prover circuit
    #[clap(short, long, env = "ESPRESSO_SEQUENCER_STAKE_TABLE_CAPACITY", default_value_t = DEFAULT_STAKE_TABLE_CAPACITY)]
    pub stake_table_capacity: usize,
    ///
    /// If the light client contract is being deployed and this is set, the prover will be
    /// permissioned so that only this address can update the light client state. Otherwise, proving
    /// will be permissionless.
    ///
    /// If the light client contract is not being deployed, this option is ignored.
    #[clap(long, env = "ESPRESSO_SEQUENCER_PERMISSIONED_PROVER")]
    permissioned_prover: Option<Address>,

    /// Exit escrow period for the stake table contract.
    ///
    /// This is the period for which stake table contract will retain funds after withdrawals have
    /// been requested. It should be set to a value that is at least 3 hotshot epochs plus ample
    /// time to allow for submission of slashing evidence. Initially it will probably be around one
    /// week.
    #[clap(long, env = "ESPRESSO_SEQUENCER_STAKE_TABLE_EXIT_ESCROW_PERIOD", value_parser = parse_duration)]
    exit_escrow_period: Option<Duration>,

    /// The address that the tokens will be minted to.
    ///
    /// If unset the tokens will be minted to the deployer account.
    #[clap(long, env = "ESP_TOKEN_INITIAL_GRANT_RECIPIENT_ADDRESS")]
    initial_token_grant_recipient: Option<Address>,

    /// The blocks per epoch    
    #[clap(long, env = "ESPRESSO_SEQUENCER_BLOCKS_PER_EPOCH")]
    blocks_per_epoch: Option<u64>,

    /// The epoch start block
    #[clap(long, env = "ESPRESSO_SEQUENCER_EPOCH_START_BLOCK")]
    epoch_start_block: Option<u64>,
    /// The initial supply of the tokens.
    #[clap(long, env = "ESP_TOKEN_INITIAL_SUPPLY")]
    initial_token_supply: Option<U256>,

    /// The name of the tokens.
    #[clap(long, env = "ESP_TOKEN_NAME")]
    token_name: Option<String>,

    /// The symbol of the tokens.
    #[clap(long, env = "ESP_TOKEN_SYMBOL")]
    token_symbol: Option<String>,

    /// The admin of the ops timelock
    #[clap(long, env = "ESPRESSO_OPS_TIMELOCK_ADMIN")]
    ops_timelock_admin: Option<Address>,

    /// The delay of the ops timelock
    #[clap(long, env = "ESPRESSO_OPS_TIMELOCK_DELAY")]
    ops_timelock_delay: Option<u64>,

    /// The executor(s) of the ops timelock
    #[clap(long, env = "ESPRESSO_OPS_TIMELOCK_EXECUTORS")]
    ops_timelock_executors: Option<Vec<Address>>,

    /// The proposer(s) of the ops timelock
    #[clap(long, env = "ESPRESSO_OPS_TIMELOCK_PROPOSERS")]
    ops_timelock_proposers: Option<Vec<Address>>,

    /// The admin of the safe exit timelock
    #[clap(long, env = "ESPRESSO_SAFE_EXIT_TIMELOCK_ADMIN")]
    safe_exit_timelock_admin: Option<Address>,

    /// The delay of the safe exit timelock
    #[clap(long, env = "ESPRESSO_SAFE_EXIT_TIMELOCK_DELAY")]
    safe_exit_timelock_delay: Option<u64>,

    /// The executor(s) of the safe exit timelock
    #[clap(long, env = "ESPRESSO_SAFE_EXIT_TIMELOCK_EXECUTORS")]
    safe_exit_timelock_executors: Option<Vec<Address>>,

    /// The proposer(s) of the safe exit timelock
    #[clap(long, env = "ESPRESSO_SAFE_EXIT_TIMELOCK_PROPOSERS")]
    safe_exit_timelock_proposers: Option<Vec<Address>>,

    /// Option to perform a timelock operation on a target contract
    /// Operations include: schedule, execute, cancel
    #[clap(long, default_value = "false")]
    perform_timelock_operation: bool,

    /// The type of the timelock operation
    #[clap(
        long,
        env = "ESPRESSO_TIMELOCK_OPERATION_TYPE",
        requires = "perform_timelock_operation"
    )]
    timelock_operation_type: Option<TimelockOperationType>,

    /// The target contract of the timelock operation
    /// The timelock is the owner of this contract and can perform the timelock operation on it
    #[clap(long, env = "ESPRESSO_TARGET_CONTRACT")]
    target_contract: Option<String>,

    /// The value to send with the timelock operation
    #[clap(
        long,
        env = "ESPRESSO_TIMELOCK_OPERATION_VALUE",
        requires = "perform_timelock_operation",
        default_value = "0",
        value_parser = parse_ether,
    )]
    timelock_operation_value: Option<U256>,

    /// The function signature for the target contract of the timelock operation
    #[clap(
        long,
        env = "ESPRESSO_TIMELOCK_OPERATION_FUNCTION_SIGNATURE",
        requires = "perform_timelock_operation"
    )]
    function_signature: Option<String>,

    /// The function data of the function selector for the target contract of the timelock operation
    #[clap(
        long,
        env = "ESPRESSO_TIMELOCK_OPERATION_FUNCTION_VALUES",
        requires = "perform_timelock_operation"
    )]
    function_values: Option<Vec<String>>,

    /// The salt for the timelock operation
    #[clap(
        long,
        env = "ESPRESSO_TIMELOCK_OPERATION_SALT",
        requires = "perform_timelock_operation"
    )]
    timelock_operation_salt: Option<String>,

    /// The delay for the timelock operation
    #[clap(
        long,
        env = "ESPRESSO_TIMELOCK_OPERATION_DELAY",
        requires = "perform_timelock_operation"
    )]
    timelock_operation_delay: Option<u64>,
    /// The address of the timelock controller
    #[clap(long, env = "ESPRESSO_SEQUENCER_TIMELOCK_ADDRESS")]
    timelock_address: Option<Address>,

    #[clap(flatten)]
    logging: logging::Config,

    /// Command to run
    ///
    /// For backwards compatibility, the default is to deploy contracts, if no
    /// subcommand is specified.
    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
enum Command {
    Account,
    Balance,
    VerifyNodeJsFiles,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Options::parse();

    opt.logging.init();

    if matches!(opt.command, Some(Command::VerifyNodeJsFiles)) {
        verify_node_js_files().await?;
        return Ok(());
    };

    let mut contracts = Contracts::from(opt.contracts);
    let provider = if opt.ledger {
        let signer = connect_ledger(opt.account_index as usize).await?;
        tracing::info!("Using ledger for signing, watch ledger device for prompts.");
        build_provider_ledger(signer, opt.rpc_url.clone(), Some(opt.l1_polling_interval))
    } else {
        build_provider(
            opt.mnemonic
                .expect("Mnemonic provided when not using ledger"),
            opt.account_index,
            opt.rpc_url.clone(),
            Some(opt.l1_polling_interval),
        )
    };

    // Fail early if we can't connect to the Ethereum RPC.
    let chain_id = provider.get_chain_id().await.with_context(|| {
        // The URL may contain a key in the query string.
        let mut url = opt.rpc_url.clone();
        url.set_query(None);
        format!("Unable query Ethereum provider {url}..")
    })?;
    tracing::info!("Connected to chain with chain ID: {chain_id}");

    let account = provider.default_signer_address();
    if let Some(command) = &opt.command {
        match command {
            Command::Account => {
                println!("{account}");
                return Ok(());
            },
            Command::Balance => {
                let balance = provider.get_balance(account).await?;
                println!("{account}: {} Eth", format_ether(balance));
                return Ok(());
            },
            _ => unreachable!(),
        };
    };

    // No subcommand specified. Deploy contracts.

    let balance = provider.get_balance(account).await?;
    tracing::info!(
        "Using deployer account {account} with balance: {}",
        format_ether(balance),
    );
    if balance.is_zero() {
        anyhow::bail!(
            "account_index {}, address={account} has no balance. A funded account is required.",
            opt.account_index
        );
    }

    // First use builder to build constructor input arguments
    let mut args_builder = DeployerArgsBuilder::default();
    args_builder
        .deployer(provider.clone())
        .mock_light_client(opt.use_mock)
        .use_multisig(opt.use_multisig)
        .dry_run(opt.dry_run)
        .rpc_url(opt.rpc_url.to_string());
    if let Some(multisig) = opt.multisig_address {
        args_builder.multisig(multisig);
    }
    if let Some(multisig_pauser) = opt.multisig_pauser_address {
        args_builder.multisig_pauser(multisig_pauser);
    }

    if let Some(blocks_per_epoch) = opt.blocks_per_epoch {
        args_builder.blocks_per_epoch(blocks_per_epoch);
    }
    if let Some(epoch_start_block) = opt.epoch_start_block {
        args_builder.epoch_start_block(epoch_start_block);
    }

    if opt.deploy_light_client_v1 {
        let (genesis_state, genesis_stake) = if opt.mock_espresso_live_network {
            light_client_genesis_from_stake_table(&Default::default(), DEFAULT_STAKE_TABLE_CAPACITY)
                .unwrap()
        } else {
            light_client_genesis(&opt.sequencer_url, opt.stake_table_capacity).await?
        };
        args_builder
            .genesis_lc_state(genesis_state)
            .genesis_st_state(genesis_stake);
        if let Some(prover) = opt.permissioned_prover {
            args_builder.permissioned_prover(prover);
        }
    }
    if opt.upgrade_light_client_v2 {
        let (blocks_per_epoch, epoch_start_block) =
            if (opt.dry_run && opt.use_multisig) || opt.mock_espresso_live_network {
                (10, 22)
            } else {
                // fetch epoch length from HotShot config
                // Request the configuration until it is successful
                loop {
                    match surf_disco::Client::<ServerError, StaticVersion<0, 1>>::new(
                        opt.sequencer_url.clone(),
                    )
                    .get::<PublicNetworkConfig>("config/hotshot")
                    .send()
                    .await
                    {
                        Ok(resp) => {
                            let config = resp.hotshot_config();
                            break (config.blocks_per_epoch(), config.epoch_start_block());
                        },
                        Err(e) => {
                            tracing::error!("Failed to fetch the network config: {e}");
                            sleep(Duration::from_secs(5));
                        },
                    }
                }
            };
        args_builder.blocks_per_epoch(blocks_per_epoch);
        args_builder.epoch_start_block(epoch_start_block);
    }

    if opt.deploy_stake_table {
        if let Some(escrow_period) = opt.exit_escrow_period {
            args_builder.exit_escrow_period(U256::from(escrow_period.as_secs()));
        }
    }
    if opt.deploy_ops_timelock {
        let ops_timelock_admin = opt.ops_timelock_admin.ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide --ops-timelock-admin or ESPRESSO_OPS_TIMELOCK_ADMIN env var when \
                 deploying ops timelock"
            )
        })?;
        args_builder.ops_timelock_admin(ops_timelock_admin);
        let ops_timelock_delay = opt.ops_timelock_delay.ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide --ops-timelock-delay or ESPRESSO_OPS_TIMELOCK_DELAY env var when \
                 deploying ops timelock"
            )
        })?;
        args_builder.ops_timelock_delay(U256::from(ops_timelock_delay));
        let ops_timelock_executors = opt.ops_timelock_executors.ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide --ops-timelock-executors or ESPRESSO_OPS_TIMELOCK_EXECUTORS env var \
                 when deploying ops timelock"
            )
        })?;
        args_builder.ops_timelock_executors(ops_timelock_executors.into_iter().collect());
        let ops_timelock_proposers = opt.ops_timelock_proposers.ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide --ops-timelock-proposers or ESPRESSO_OPS_TIMELOCK_PROPOSERS env var \
                 when deploying ops timelock"
            )
        })?;
        args_builder.ops_timelock_proposers(ops_timelock_proposers.into_iter().collect());
    }

    if opt.deploy_safe_exit_timelock {
        let safe_exit_timelock_admin = opt.safe_exit_timelock_admin.ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide --safe-exit-timelock-admin or ESPRESSO_SAFE_EXIT_TIMELOCK_ADMIN env \
                 var when deploying safe exit timelock"
            )
        })?;
        args_builder.safe_exit_timelock_admin(safe_exit_timelock_admin);
        let safe_exit_timelock_delay = opt.safe_exit_timelock_delay.ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide --safe-exit-timelock-delay or ESPRESSO_SAFE_EXIT_TIMELOCK_DELAY env \
                 var when deploying safe exit timelock"
            )
        })?;
        args_builder.safe_exit_timelock_delay(U256::from(safe_exit_timelock_delay));
        let safe_exit_timelock_executors = opt.safe_exit_timelock_executors.ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide --safe-exit-timelock-executors or \
                 ESPRESSO_SAFE_EXIT_TIMELOCK_EXECUTORS env var when deploying safe exit timelock"
            )
        })?;
        args_builder
            .safe_exit_timelock_executors(safe_exit_timelock_executors.into_iter().collect());
        let safe_exit_timelock_proposers = opt.safe_exit_timelock_proposers.ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide --safe-exit-timelock-proposers or \
                 ESPRESSO_SAFE_EXIT_TIMELOCK_PROPOSERS env var when deploying safe exit timelock"
            )
        })?;
        args_builder
            .safe_exit_timelock_proposers(safe_exit_timelock_proposers.into_iter().collect());
    }
    if opt.use_timelock_owner {
        args_builder.use_timelock_owner(true);
    }

    if opt.perform_timelock_operation {
        let timelock_operation_type = opt.timelock_operation_type.ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide --timelock-operation-type or ESPRESSO_TIMELOCK_OPERATION_TYPE env \
                 var when scheduling timelock operation"
            )
        })?;

        args_builder.timelock_operation_type(timelock_operation_type);
        let target_contract = opt.target_contract.clone().ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide --target-contract or ESPRESSO_TARGET_CONTRACT env var when \
                 scheduling timelock operation"
            )
        })?;
        args_builder.target_contract(target_contract);
        let function_signature = opt.function_signature.ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide --function-signature or \
                 ESPRESSO_TIMELOCK_OPERATION_FUNCTION_SIGNATURE env var when performing timelock \
                 operation"
            )
        })?;
        args_builder.timelock_operation_function_signature(function_signature);
        let function_values = opt.function_values.ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide --function-values or ESPRESSO_TIMELOCK_OPERATION_FUNCTION_VALUES \
                 env var when performing timelock operation"
            )
        })?;
        args_builder.timelock_operation_function_values(function_values);
        let timelock_operation_salt = opt.timelock_operation_salt.ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide --timelock-operation-salt or ESPRESSO_TIMELOCK_OPERATION_SALT env \
                 var when scheduling timelock operation"
            )
        })?;
        args_builder.timelock_operation_salt(timelock_operation_salt);
        let timelock_operation_delay = opt.timelock_operation_delay.ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide --timelock-operation-delay or ESPRESSO_TIMELOCK_OPERATION_DELAY env \
                 var when scheduling timelock operation"
            )
        })?;
        args_builder.timelock_operation_delay(U256::from(timelock_operation_delay));
        let timelock_operation_value = opt.timelock_operation_value.unwrap_or_default();
        args_builder.timelock_operation_value(timelock_operation_value);
    }

    if opt.deploy_esp_token {
        let token_recipient = opt
            .initial_token_grant_recipient
            .expect("Must provide --initial-token-grant-recipient when deploying esp token");
        let token_name = opt
            .token_name
            .expect("Must provide --token-name when deploying esp token");
        let token_symbol = opt
            .token_symbol
            .expect("Must provide --token-symbol when deploying esp token");
        let initial_token_supply = opt
            .initial_token_supply
            .expect("Must provide --initial-token-supply when deploying esp token");
        args_builder.token_name(token_name);
        args_builder.token_symbol(token_symbol);
        args_builder.initial_token_supply(initial_token_supply);
        args_builder.token_recipient(token_recipient);
    }

    // Add EOA ownership transfer parameters to builder
    if opt.transfer_ownership_from_eoa {
        let target_contract = opt.target_contract.clone().ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide --target-contract when using --transfer-ownership-from-eoa"
            )
        })?;
        let new_owner = opt.transfer_ownership_new_owner.ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide --transfer-ownership-new-owner when using \
                 --transfer-ownership-from-eoa"
            )
        })?;
        args_builder.transfer_ownership_from_eoa(true);
        args_builder.target_contract(target_contract);
        args_builder.transfer_ownership_new_owner(new_owner);
    }

    // Add multisig to timelock transfer parameters to builder
    if opt.propose_transfer_ownership_to_timelock {
        let target_contract = opt.target_contract.clone().ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide --target-contract when using \
                 --propose-transfer-ownership-to-timelock"
            )
        })?;
        let timelock_address = opt.timelock_address.ok_or_else(|| {
            anyhow::anyhow!(
                "Must provide --timelock-address when using \
                 --propose-transfer-ownership-to-timelock"
            )
        })?;
        args_builder.target_contract(target_contract);
        args_builder.timelock_address(timelock_address);
    }

    // then deploy specified contracts
    let args = args_builder.build()?;

    // Deploy timelocks first so they can be used as owners for other contracts
    if opt.deploy_ops_timelock {
        args.deploy(&mut contracts, Contract::OpsTimelock).await?;
    }
    if opt.deploy_safe_exit_timelock {
        args.deploy(&mut contracts, Contract::SafeExitTimelock)
            .await?;
    }

    // Then deploy other contracts
    if opt.deploy_fee {
        args.deploy(&mut contracts, Contract::FeeContractProxy)
            .await?;
    }
    if opt.deploy_esp_token {
        args.deploy(&mut contracts, Contract::EspTokenProxy).await?;
    }
    if opt.upgrade_esp_token_v2 {
        args.deploy(&mut contracts, Contract::EspTokenV2).await?;
    }
    if opt.deploy_light_client_v1 {
        args.deploy(&mut contracts, Contract::LightClientProxy)
            .await?;
    }
    if opt.upgrade_light_client_v2 {
        args.deploy(&mut contracts, Contract::LightClientV2).await?;
    }
    if opt.deploy_stake_table {
        args.deploy(&mut contracts, Contract::StakeTableProxy)
            .await?;
    }
    if opt.upgrade_stake_table_v2 {
        args.deploy(&mut contracts, Contract::StakeTableV2).await?;
    }

    // then perform the timelock operation if any
    if opt.perform_timelock_operation {
        args.perform_timelock_operation_on_contract(&mut contracts)
            .await?;
    }

    // Execute ownership transfer proposal if requested
    if opt.propose_transfer_ownership_to_timelock {
        args.propose_transfer_ownership_to_timelock(&mut contracts)
            .await?;
    }

    // Execute ownership transfer when the admin is an EOA
    if opt.transfer_ownership_from_eoa {
        args.transfer_ownership_from_eoa(&mut contracts).await?;
    }

    // finally print out or persist deployed addresses
    if let Some(out) = &opt.out {
        let file = File::options()
            .create(true)
            .truncate(true)
            .write(true)
            .open(out)?;
        contracts.write(file)?;
    } else {
        contracts.write(stdout())?;
    }

    Ok(())
}
