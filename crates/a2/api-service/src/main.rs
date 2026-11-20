//! An API instance: serves the v2 API from the chain store.

use std::net::SocketAddr;

use anyhow::{Context, anyhow};
use api_service::{ApiService, catchup::Catchup, compat::Compat};
use chain_store::{ChainStore, remote::RemoteBackend};
use clap::Parser;
use espresso_genesis::{Genesis, GenesisSource};
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

#[derive(Debug, Parser)]
struct Options {
    /// The port to serve HTTP and gRPC on.
    #[arg(long, env = "ESPRESSO_API_SERVICE_PORT", default_value_t = 8080)]
    port: u16,

    /// The store service to read through, as a URL.
    #[arg(long, env = "ESPRESSO_API_SERVICE_STORE")]
    store: String,

    /// The address of the consensus node's API, for submissions.
    #[arg(long, env = "ESPRESSO_API_SERVICE_CONSENSUS_API")]
    consensus_api: SocketAddr,

    /// The genesis file of the chain, a path or an HTTP URL, for its epochs and genesis stake
    /// table.
    #[arg(long, env = "ESPRESSO_API_SERVICE_GENESIS_FILE")]
    genesis_file: GenesisSource,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = Options::parse();
    init_logging();
    let backend = RemoteBackend::connect(&options.store)
        .await
        .context("connecting to the store")?;
    let listener = TcpListener::bind(("0.0.0.0", options.port))
        .await
        .with_context(|| format!("binding port {}", options.port))?;
    let store = ChainStore::new(backend);
    let genesis = Genesis::load(&options.genesis_file)
        .await
        .context("loading the genesis file")?;
    let config = genesis.unstaked_network_config()?;
    let epoch_height = config.config.epoch_height;
    let api = ApiService::new(store.clone(), options.consensus_api)
        .with_compat(Compat::new(store.clone(), config))
        .with_catchup(Catchup::new(store, epoch_height));
    match api.serve(listener).await {
        Ok(()) => Err(anyhow!("the server stopped")),
        Err(err) => Err(err.context("serving the API")),
    }
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
