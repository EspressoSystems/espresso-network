//! Runs an a2 consensus node.

use std::{net::SocketAddr, path::PathBuf, time::Duration};

use anyhow::Context;
use clap::Parser;
use consensus::storage::DEFAULT_DECIDE_LOG_LIMIT;
use consensus_node::node::{Config, run};
use espresso_genesis::{Genesis, GenesisSource};
use espresso_keyset::{KeySet, KeySetOptions};
use hotshot_types::{addr::NetAddr, traits::metrics::NoMetrics};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};
use url::Url;

#[derive(Debug, Parser)]
struct Options {
    #[clap(flatten)]
    keys: KeySetOptions,

    /// The genesis file of the chain, a path or an HTTP URL.
    #[arg(long, env = "ESPRESSO_CONSENSUS_GENESIS_FILE")]
    genesis_file: GenesisSource,

    /// L1 RPC URLs, comma-separated.
    #[arg(long, env = "ESPRESSO_CONSENSUS_L1_PROVIDER", value_delimiter = ',')]
    l1_provider: Vec<Url>,

    /// The directory of the consensus store.
    #[arg(long, env = "ESPRESSO_CONSENSUS_STORE_PATH")]
    store_path: PathBuf,

    /// Where to listen for the other consensus nodes, as `host:port`.
    #[arg(
        long,
        env = "ESPRESSO_CONSENSUS_BIND_ADDRESS",
        default_value = "0.0.0.0:9977"
    )]
    bind_address: NetAddr,

    /// Where to serve the consensus API, for the storage instances and the API service.
    #[arg(
        long,
        env = "ESPRESSO_CONSENSUS_API_ADDRESS",
        default_value = "127.0.0.1:9978"
    )]
    api_address: SocketAddr,

    /// The URL of this validator's API service, which its signed record tells the others.
    #[arg(long, env = "ESPRESSO_CONSENSUS_PUBLIC_API_URL")]
    public_api_url: Url,

    /// API services to catch up from besides the node's own, comma-separated.
    #[arg(long, env = "ESPRESSO_CONSENSUS_BOOTSTRAP", value_delimiter = ',')]
    bootstrap: Vec<Url>,

    /// The state relay server to send light client state signatures to.
    #[arg(long, env = "ESPRESSO_CONSENSUS_STATE_RELAY_SERVER_URL")]
    state_relay_server_url: Option<Url>,

    /// How many bytes the decide log may take up before the node drops its oldest entries.
    #[arg(long, env = "ESPRESSO_CONSENSUS_DECIDE_LOG_LIMIT", default_value_t = DEFAULT_DECIDE_LOG_LIMIT)]
    decide_log_limit: u64,

    /// How long a leader waits for transactions before proposing an empty block.
    #[arg(
        long,
        env = "ESPRESSO_CONSENSUS_EMPTY_BLOCK_DELAY",
        default_value = "1s",
        value_parser = humantime::parse_duration
    )]
    empty_block_delay: Duration,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = Options::parse();
    init_logging();
    let KeySet {
        staking,
        state,
        x25519,
    } = options.keys.try_into()?;
    let genesis = Genesis::load(&options.genesis_file)
        .await
        .context("loading the genesis file")?;
    let config = Config {
        genesis,
        staking_key: staking,
        state_key: state,
        x25519_key: x25519,
        l1: options.l1_provider,
        store: options.store_path,
        bind: options.bind_address,
        consensus_api: options.api_address,
        api_url: options.public_api_url,
        api_key: None,
        bootstrap: options.bootstrap,
        relay: options.state_relay_server_url,
        decide_log_limit: options.decide_log_limit,
        empty_block_delay: options.empty_block_delay,
    };
    run(config, Box::new(NoMetrics)).await
}

/// Logs to stdout, at the level `RUST_LOG` sets or INFO, as JSON when `RUST_LOG_FORMAT=json`.
fn init_logging() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stdout);
    if std::env::var("RUST_LOG_FORMAT").is_ok_and(|format| format == "json") {
        builder.json().init();
    } else {
        builder.init();
    }
}
