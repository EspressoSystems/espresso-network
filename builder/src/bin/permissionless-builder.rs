use std::{num::NonZeroUsize, path::PathBuf, time::Duration};

use builder::non_permissioned::{build_instance_state, BuilderConfig};
use clap::Parser;
use espresso_types::{eth_signature_key::EthKeyPair, parse_duration, SequencerVersions};
use futures::future::pending;
use hotshot::traits::ValidatedState;
use hotshot_types::{
    data::ViewNumber,
    traits::node_implementation::{ConsensusTime, Versions},
};
use sequencer::{Genesis, L1Params};
use sequencer_utils::logging;
use url::Url;
use vbs::version::StaticVersionType;

#[derive(Parser, Clone, Debug)]
struct NonPermissionedBuilderOptions {
    /// URL of hotshot events API running on Espresso Sequencer DA committee node
    /// The builder will subscribe to this server to receive hotshot events
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_HOTSHOT_EVENT_STREAMING_API_URL",
        default_value = "http://localhost:22001"
    )]
    hotshot_event_streaming_url: Url,

    /// Mnemonic phrase for builder account.
    ///
    /// This is the address fees will be charged to.
    /// It must be funded with ETH in the Espresso fee ledger
    #[clap(long, env = "ESPRESSO_BUILDER_ETH_MNEMONIC")]
    eth_mnemonic: String,

    /// Index of a funded account derived from eth-mnemonic.
    #[clap(long, env = "ESPRESSO_BUILDER_ETH_ACCOUNT_INDEX", default_value = "8")]
    eth_account_index: u32,

    /// Url we will use for RPC communication with L1.
    #[clap(
        long,
        env = "ESPRESSO_BUILDER_L1_PROVIDER",
        value_delimiter = ',',
        num_args = 1..,
    )]
    l1_provider_url: Vec<Url>,

    /// Peer nodes use to fetch missing state
    #[clap(long, env = "ESPRESSO_SEQUENCER_STATE_PEERS", value_delimiter = ',')]
    state_peers: Vec<Url>,

    /// Port to run the builder server on.
    #[clap(short, long, env = "ESPRESSO_BUILDER_SERVER_PORT")]
    port: u16,

    /// Bootstrapping View number
    #[clap(short, long, env = "ESPRESSO_BUILDER_BOOTSTRAPPED_VIEW")]
    view_number: u64,

    /// BUILDER TRANSACTIONS CHANNEL CAPACITY
    #[clap(long, env = "ESPRESSO_BUILDER_TX_CHANNEL_CAPACITY")]
    pub tx_channel_capacity: NonZeroUsize,

    /// BUILDER HS EVENTS CHANNEL CAPACITY
    #[clap(long, env = "ESPRESSO_BUILDER_EVENT_CHANNEL_CAPACITY")]
    pub event_channel_capacity: NonZeroUsize,

    /// NETWORK INITIAL NODE COUNT
    #[clap(short, long, env = "ESPRESSO_BUILDER_INIT_NODE_COUNT")]
    node_count: NonZeroUsize,

    /// The amount of time a builder can wait before timing out a request to the API.
    #[clap(
        short,
        long,
        env = "ESPRESSO_BUILDER_WEBSERVER_RESPONSE_TIMEOUT_DURATION",
        default_value = "1s",
        value_parser = parse_duration
    )]
    max_api_timeout_duration: Duration,

    /// The amount of time a builder can wait before incrementing the max block size.
    #[clap(
        short = 'M',
        long,
        env = "ESPRESSO_BUILDER_MAX_BLOCK_SIZE_INCREMENT_PERIOD",
        default_value = "3600s",
        value_parser = parse_duration
    )]
    max_block_size_increment_period: Duration,

    /// The amount of time a builder can wait before incrementing the max block size.
    #[clap(
        long,
        env = "ESPRESSO_BUILDER_TX_STATUS_CACHE_SIZE",
        default_value = "819200"
    )]
    tx_status_cache_size: usize,

    /// Path to TOML file containing genesis state.
    #[clap(long, name = "GENESIS_FILE", env = "ESPRESSO_BUILDER_GENESIS_FILE")]
    genesis_file: PathBuf,

    #[clap(flatten)]
    logging: logging::Config,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = NonPermissionedBuilderOptions::parse();
    opt.logging.init();

    let genesis = Genesis::from_file(&opt.genesis_file)?;
    tracing::info!(?genesis, "genesis");

    let base = genesis.base_version;
    let upgrade = genesis.upgrade_version;

    match (base, upgrade) {
        #[cfg(all(feature = "fee", feature = "pos"))]
        (espresso_types::FeeVersion::VERSION, espresso_types::EpochVersion::VERSION) => {
            run::<SequencerVersions<espresso_types::FeeVersion, espresso_types::EpochVersion>>(
                genesis, opt
            )
            .await
        }
        #[cfg(feature = "pos")]
        (espresso_types::EpochVersion::VERSION, _) => {
            run::<SequencerVersions<espresso_types::EpochVersion, espresso_types::V0_0>>(
                genesis, opt,
            )
            .await
        }
        #[cfg(all(feature = "fee", feature = "marketplace"))]
        (espresso_types::FeeVersion::VERSION, espresso_types::MarketplaceVersion::VERSION) => {
            run::<SequencerVersions<espresso_types::FeeVersion, espresso_types::MarketplaceVersion>>(
                genesis, opt
            )
            .await
        },
        #[cfg(feature = "fee")]
        (espresso_types::FeeVersion::VERSION, _) => {
            run::<SequencerVersions<espresso_types::FeeVersion, espresso_types::V0_0>>(
                genesis, opt
            )
            .await
        },
        #[cfg(feature = "marketplace")]
        (espresso_types::MarketplaceVersion::VERSION, _) => {
            run::<SequencerVersions<espresso_types::MarketplaceVersion, espresso_types::V0_0>>(
                genesis, opt
            )
            .await

        },
        _ => panic!(
            "Invalid base ({base}) and upgrade ({upgrade}) versions specified in the toml file."
        ),
    }
}

async fn run<V: Versions>(
    genesis: Genesis,
    opt: NonPermissionedBuilderOptions,
) -> anyhow::Result<()> {
    let l1_params = L1Params {
        urls: opt.l1_provider_url,
        options: Default::default(),
    };

    let builder_key_pair = EthKeyPair::from_mnemonic(&opt.eth_mnemonic, opt.eth_account_index)?;
    let bootstrapped_view = ViewNumber::new(opt.view_number);

    let builder_server_url: Url = format!("http://0.0.0.0:{}", opt.port).parse().unwrap();

    let instance_state =
        build_instance_state::<V>(genesis.chain_config, l1_params, opt.state_peers);

    let base_fee = genesis.max_base_fee();
    tracing::info!(?base_fee, "base_fee");

    let validated_state = ValidatedState::genesis(&instance_state).0;

    let api_response_timeout_duration = opt.max_api_timeout_duration;

    // make the txn timeout as 1/4 of the api_response_timeout_duration
    let txn_timeout_duration = api_response_timeout_duration / 4;

    let _builder_config = BuilderConfig::init::<V>(
        builder_key_pair,
        bootstrapped_view,
        opt.tx_channel_capacity,
        opt.event_channel_capacity,
        opt.node_count,
        instance_state.clone(),
        validated_state,
        opt.hotshot_event_streaming_url,
        builder_server_url,
        api_response_timeout_duration,
        opt.max_block_size_increment_period,
        txn_timeout_duration,
        base_fee,
        opt.tx_status_cache_size,
    )
    .await?;

    // Sleep forever
    pending::<()>().await;

    Ok(())
}
