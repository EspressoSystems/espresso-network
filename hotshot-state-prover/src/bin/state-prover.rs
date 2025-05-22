use std::time::Duration;

use alloy::{
    primitives::{utils::parse_units, Address},
    providers::{Provider, ProviderBuilder},
    signers::{
        local::{coins_bip39::English, MnemonicBuilder},
        Signer,
    },
};
use clap::Parser;
use espresso_contract_deployer::network_config::fetch_epoch_config_from_sequencer;
use espresso_types::parse_duration;
use hotshot_state_prover::service::{run_prover_once, run_prover_service, StateProverConfig};
use hotshot_types::light_client::DEFAULT_STAKE_TABLE_CAPACITY;
use sequencer_utils::logging;
use url::Url;
use vbs::version::StaticVersion;

#[derive(Parser)]
struct Args {
    /// Start the prover service daemon
    #[arg(short, long)]
    daemon: bool,

    /// Url of the state relay server
    #[arg(
        long,
        default_value = "http://localhost:8083",
        env = "ESPRESSO_STATE_RELAY_SERVER_URL"
    )]
    relay_server: Url,

    /// The frequency of updating the light client state, expressed in update interval
    #[arg(short, long = "freq", value_parser = parse_duration, default_value = "10m", env = "ESPRESSO_STATE_PROVER_UPDATE_INTERVAL")]
    update_interval: Duration,

    /// Interval between retries if a state update fails
    #[arg(long = "retry-freq", value_parser = parse_duration, default_value = "2s", env = "ESPRESSO_STATE_PROVER_RETRY_INTERVAL")]
    retry_interval: Duration,

    /// Interval between retries if a state update fails
    #[arg(
        long = "retries",
        default_value = "10",
        env = "ESPRESSO_STATE_PROVER_ONESHOT_RETRIES"
    )]
    max_retries: u64,

    /// URL of layer 1 Ethereum JSON-RPC provider.
    #[arg(
        long,
        env = "ESPRESSO_SEQUENCER_L1_PROVIDER",
        default_value = "http://localhost:8545"
    )]
    l1_provider: Url,

    /// Address of LightClient contract on layer 1.
    #[arg(long, env = "ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS")]
    light_client_address: Address,

    /// Mnemonic phrase for a funded Ethereum wallet.
    #[arg(long, env = "ESPRESSO_SEQUENCER_ETH_MNEMONIC", default_value = None)]
    eth_mnemonic: String,

    /// Index of a funded account derived from eth-mnemonic.
    #[arg(
        long,
        env = "ESPRESSO_SEQUENCER_STATE_PROVER_ACCOUNT_INDEX",
        default_value = "0"
    )]
    eth_account_index: u32,

    /// URL of a sequencer node that is currently providing the HotShot config.
    /// This is used to initialize the stake table.
    #[arg(
        long,
        env = "ESPRESSO_SEQUENCER_URL",
        default_value = "http://localhost:24000"
    )]
    pub sequencer_url: Url,

    /// If daemon and provided, the service will run a basic HTTP server on the given port.
    ///
    /// The server provides healthcheck and version endpoints.
    #[arg(short, long, env = "ESPRESSO_PROVER_SERVICE_PORT")]
    pub port: Option<u16>,

    /// Stake table capacity for the prover circuit
    #[arg(short, long, env = "ESPRESSO_SEQUENCER_STAKE_TABLE_CAPACITY", default_value_t = DEFAULT_STAKE_TABLE_CAPACITY)]
    pub stake_table_capacity: usize,

    /// max acceptable gas price **in Gwei** for prover to send light client update transaction
    #[arg(short, long, env = "ESPRESSO_STATE_PROVER_MAX_GAS_PRICE_IN_GWEI")]
    pub max_gas_price: Option<String>,

     #[command(flatten)]
  
    logging: logging::Config,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    args.logging.init();

    // prepare config for state prover from user options
    let l1_provider = ProviderBuilder::new().on_http(args.l1_provider.clone());
    let chain_id = l1_provider.get_chain_id().await.unwrap();
    let signer = MnemonicBuilder::<English>::default()
        .phrase(args.eth_mnemonic)
        .index(args.eth_account_index)
        .expect("wrong mnemonic or index")
        .build()
        .expect("fail to build signer")
        .with_chain_id(Some(chain_id));

    let (blocks_per_epoch, epoch_start_block) =
        fetch_epoch_config_from_sequencer(&args.sequencer_url)
            .await
            .unwrap_or((u64::MAX, u64::MAX));

    // If the sequencer returns 0 for pre-epoch configuration.
    let blocks_per_epoch = if blocks_per_epoch == 0 {
        u64::MAX
    } else {
        blocks_per_epoch
    };
    let epoch_start_block = if epoch_start_block == 0 {
        u64::MAX
    } else {
        epoch_start_block
    };

    tracing::info!(
        "Epoch config fetched from sequencer: blocks_per_epoch = {}, epoch_start_block = {}",
        blocks_per_epoch,
        epoch_start_block
    );
    let max_gas_price = args.max_gas_price.map(|v| {
        parse_units(&v, "gwei")
            .expect("parse_unit on max_gas_price in GWEI failed")
            .try_into()
            .expect("fail to convert gas price to u128")
    });

    let config = StateProverConfig {
        relay_server: args.relay_server,
        update_interval: args.update_interval,
        retry_interval: args.retry_interval,
        provider_endpoint: args.l1_provider,
        light_client_address: args.light_client_address,
        signer,
        sequencer_url: args.sequencer_url,
        port: args.port,
        stake_table_capacity: args.stake_table_capacity,
        blocks_per_epoch,
        epoch_start_block,
        max_retries: args.max_retries,
        max_gas_price,
    };

    // validate that the light client contract is a proxy, panics otherwise
    config.validate_light_client_contract().await.unwrap();

    if args.daemon {
        // Launching the prover service daemon
        if let Err(err) = run_prover_service(config, StaticVersion::<0, 1> {}).await {
            tracing::error!("Error running prover service: {:?}", err);
        };
    } else {
        // Run light client state update once
        if let Err(err) = run_prover_once(config, StaticVersion::<0, 1> {}).await {
            tracing::error!("Error running prover once: {:?}", err);
        };
    }
}
