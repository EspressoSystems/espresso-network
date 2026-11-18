//! Serves the v2 API from the chain store.

use std::net::SocketAddr;

use anyhow::Context;
use api_service::{ApiService, GapFiller, Membership};
use chain_store::{ChainStore, postgres::PostgresBackend};
use clap::Parser;
use espresso_genesis::{Genesis, GenesisSource};
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, filter::LevelFilter};
use url::Url;

#[derive(Debug, Parser)]
struct Options {
    /// The port to serve HTTP and gRPC on.
    #[arg(long, env = "ESPRESSO_API_SERVICE_PORT", default_value_t = 8080)]
    port: u16,

    /// The store to read, as a Postgres URL.
    #[arg(long, env = "ESPRESSO_API_SERVICE_STORE")]
    store: String,

    /// The address of the consensus node's API, for submissions and discovery records.
    #[arg(long, env = "ESPRESSO_API_SERVICE_CONSENSUS_API")]
    consensus_api: SocketAddr,

    /// API services to fill gaps from, besides those of the validators, comma-separated.
    #[arg(long, env = "ESPRESSO_API_SERVICE_BOOTSTRAP", value_delimiter = ',')]
    bootstrap: Vec<Url>,

    /// The genesis file of the chain, a path or an HTTP URL.
    #[arg(long, env = "ESPRESSO_API_SERVICE_GENESIS_FILE")]
    genesis_file: GenesisSource,

    /// L1 RPC URLs to read the stake table contract from, comma-separated.
    #[arg(long, env = "ESPRESSO_API_SERVICE_L1_PROVIDER", value_delimiter = ',')]
    l1_provider: Vec<Url>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = Options::parse();
    init_logging();
    let backend = PostgresBackend::connect(&options.store)
        .await
        .context("connecting to the store")?;
    let listener = TcpListener::bind(("0.0.0.0", options.port))
        .await
        .with_context(|| format!("binding port {}", options.port))?;
    let store = ChainStore::new(backend);
    let genesis = Genesis::load(&options.genesis_file)
        .await
        .context("loading the genesis file")?;
    let membership = Membership::new(&genesis, options.l1_provider)
        .await
        .context("building the membership")?;
    let filler = GapFiller::new(
        store.clone(),
        options.consensus_api,
        options.bootstrap,
        Some(membership),
    );
    tokio::spawn(filler.run());
    ApiService::new(store, options.consensus_api)
        .serve(listener)
        .await
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
