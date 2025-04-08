use std::{num::NonZeroUsize, time::Duration};

use alloy::hex::{self, FromHexError};
use clap::Parser;
use derive_more::From;
use espresso_types::{parse_duration, Ratio, SeqTypes};
use hotshot_orchestrator::run_orchestrator;
use hotshot_types::network::{Libp2pConfig, NetworkConfig};
use sequencer_utils::logging;
use snafu::Snafu;
use url::Url;
use vec1::Vec1;

#[derive(Parser)]
struct Args {
    /// Port to run the server on.
    #[clap(short, long, env = "ESPRESSO_ORCHESTRATOR_PORT")]
    port: u16,

    /// Port to run the server on.
    #[clap(short, long, env = "ESPRESSO_ORCHESTRATOR_MANUAL_START_PASSWORD")]
    manual_start_password: Option<String>,

    /// Number of nodes in the network.
    #[clap(short, long, env = "ESPRESSO_ORCHESTRATOR_NUM_NODES")]
    num_nodes: NonZeroUsize,

    /// Duration to wait after all nodes are connected before starting the run.
    #[clap(long, env = "ESPRESSO_ORCHESTRATOR_START_DELAY", default_value = "10s", value_parser = parse_duration)]
    start_delay: Duration,

    /// Base duration for next-view timeout.
    #[arg(
        long,
        env = "ESPRESSO_ORCHESTRATOR_NEXT_VIEW_TIMEOUT",
        default_value = "60s",
        value_parser = parse_duration
    )]
    next_view_timeout: Duration,

    /// The exponential backoff ratio for the next-view timeout.
    #[arg(
        long,
        env = "ESPRESSO_ORCHESTRATOR_TIMEOUT_RATIO",
        default_value = "11:10"
    )]
    timeout_ratio: Ratio,

    /// The threshold
    #[arg(
        long,
        env = "ESPRESSO_ORCHESTRATOR_START_THRESHOLD",
        default_value = "8:10"
    )]
    start_threshold: Ratio,

    /// The delay a leader inserts before starting pre-commit.
    #[arg(
        long,
        env = "ESPRESSO_ORCHESTRATOR_ROUND_START_DELAY",
        default_value = "1ms",
        value_parser = parse_duration
    )]
    round_start_delay: Duration,

    /// The number of nodes a Libp2p node should try to maintain
    /// a connection with at one time.
    #[arg(
        long,
        env = "ESPRESSO_ORCHESTRATOR_LIBP2P_MESH_N",
        default_value = "20"
    )]
    libp2p_mesh_n: usize,

    /// Seed to use for generating node keys.
    ///
    /// The seed is a 32 byte integer, encoded in hex.
    #[arg(long, env = "ESPRESSO_ORCHESTRATOR_KEYGEN_SEED", default_value = "0x0000000000000000000000000000000000000000000000000000000000000000", value_parser = parse_seed)]
    keygen_seed: [u8; 32],

    /// HotShot builder URL
    #[arg(long, env = "ESPRESSO_ORCHESTRATOR_BUILDER_URLS", num_args = 1.., value_delimiter = ',')]
    builder_urls: Vec<Url>,

    /// The maximum amount of time a leader can wait to get a block from a builder.
    ///
    /// If the consensus leader is unable to get a block from a builder within this time, it will
    /// propose an empty block instead.
    #[arg(
        long,
        env = "ESPRESSO_ORCHESTRATOR_BUILDER_TIMEOUT",
        default_value = "1s",
        value_parser = parse_duration
    )]
    builder_timeout: Duration,

    #[clap(flatten)]
    logging: logging::Config,
}

#[derive(Debug, Snafu, From)]
enum ParseSeedError {
    #[snafu(display("seed must be valid hex: {source}"))]
    Hex { source: FromHexError },

    #[snafu(display("wrong length for seed {length} (expected 32)"))]
    WrongLength { length: usize },
}

fn parse_seed(s: &str) -> Result<[u8; 32], ParseSeedError> {
    <[u8; 32]>::try_from(hex::decode(s)?)
        .map_err(|vec| ParseSeedError::WrongLength { length: vec.len() })
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    args.logging.init();

    let mut config = NetworkConfig::<SeqTypes> {
        manual_start_password: args.manual_start_password,
        indexed_da: false,
        ..Default::default()
    };

    // The Libp2p configuration
    let libp2p_config = Libp2pConfig {
        bootstrap_nodes: Vec::new(),
    };

    config.config.num_nodes_with_stake = args.num_nodes;
    config.config.known_nodes_with_stake = vec![Default::default(); args.num_nodes.get()];
    config.config.known_da_nodes = Vec::new();
    config.config.next_view_timeout = args.next_view_timeout.as_millis() as u64;
    config.libp2p_config = Some(libp2p_config);
    config.config.start_threshold = args.start_threshold.into();
    config.config.da_staked_committee_size = args.num_nodes.get();
    config.config.builder_urls = Vec1::try_from_vec(args.builder_urls).unwrap();
    config.config.builder_timeout = args.builder_timeout;
    run_orchestrator(
        config,
        format!("http://0.0.0.0:{}", args.port).parse().unwrap(),
    )
    .await
    .unwrap();
}
