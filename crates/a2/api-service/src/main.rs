//! Serves the v2 API from the chain store.

use std::net::SocketAddr;

use anyhow::{Context, anyhow};
use api_service::{
    ApiService, GapFiller, Membership, Peers,
    catchup::Catchup,
    chain_state,
    compat::Compat,
    membership::{network_config, node_state},
};
use chain_store::{ChainStore, remote::RemoteBackend};
use clap::Parser;
use espresso_genesis::{Genesis, GenesisSource};
use tokio::{net::TcpListener, task::JoinSet};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};
use url::Url;

#[derive(Debug, Parser)]
struct Options {
    /// The port to serve HTTP and gRPC on.
    #[arg(long, env = "ESPRESSO_API_SERVICE_PORT", default_value_t = 8080)]
    port: u16,

    /// The store service to read and fill gaps through, as a URL.
    #[arg(long, env = "ESPRESSO_API_SERVICE_STORE")]
    store: String,

    /// The address of the consensus node's API, for submissions and discovery records.
    #[arg(long, env = "ESPRESSO_API_SERVICE_CONSENSUS_API")]
    consensus_api: SocketAddr,

    /// API services to learn the validators' API services from, and to fill gaps from until then,
    /// comma-separated.
    #[arg(long, env = "ESPRESSO_API_SERVICE_BOOTSTRAP", value_delimiter = ',')]
    bootstrap: Vec<Url>,

    /// The URL other services reach this API service at, as its validator's record names it. It
    /// is left out of the peers.
    #[arg(long, env = "ESPRESSO_API_SERVICE_PUBLIC_URL")]
    public_url: Option<Url>,

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
    let state = node_state(&genesis, options.l1_provider, store.clone())
        .await
        .context("building the node state")?;
    let membership = Membership::new(&state)?;
    let peers = Peers::new(options.consensus_api, options.bootstrap, options.public_url);
    let filler = GapFiller::new(store.clone(), peers, Some(membership.clone()));
    let (follower, states) = chain_state::follower(store.clone(), state.clone());
    let config = network_config(&genesis)?;
    let api = ApiService::new(store.clone(), options.consensus_api)
        .with_compat(Compat::new(store.clone(), membership, config))
        .with_catchup(Catchup::new(store, states, state));

    // The service runs as long as all of its parts do.
    let mut tasks = JoinSet::new();
    tasks.spawn(async move {
        filler.run().await;
        anyhow!("the gap filler stopped")
    });
    tasks.spawn(async move {
        match follower.run().await {
            Ok(()) => anyhow!("the chain follower stopped"),
            Err(err) => err.context("following the chain"),
        }
    });
    tasks.spawn(async move {
        match api.serve(listener).await {
            Ok(()) => anyhow!("the server stopped"),
            Err(err) => err.context("serving the API"),
        }
    });
    let stopped = tasks.join_next().await.context("no task to run")?;
    Err(stopped.unwrap_or_else(|err| anyhow!("a task failed: {err}")))
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
