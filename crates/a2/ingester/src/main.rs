//! The ingester of a deployment: copies the decide log of its consensus node into the store, and
//! does the deployment's derived work.

use std::{net::SocketAddr, time::Duration};

use anyhow::{Context, anyhow};
use chain_api::Tables;
use chain_store::{ChainStore, remote::RemoteBackend};
use clap::Parser;
use espresso_genesis::{Genesis, GenesisSource};
use espresso_types::SeqTypes;
use ingester::{ChainFollower, GapFiller, Membership, Peers, Publisher, node_state};
use tokio::{task::JoinSet, time::sleep};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};
use url::Url;

/// How long to wait before following the chain again after the follower failed.
const FOLLOWER_RETRY: Duration = Duration::from_secs(60);

#[derive(Debug, Parser)]
struct Options {
    /// The address of the consensus node's API.
    #[arg(long, env = "ESPRESSO_INGESTER_CONSENSUS_API")]
    consensus_api: SocketAddr,

    /// The store service to write to, as a URL.
    #[arg(long, env = "ESPRESSO_INGESTER_STORE")]
    store: String,

    /// The genesis file of the chain, a path or an HTTP URL.
    #[arg(long, env = "ESPRESSO_INGESTER_GENESIS_FILE")]
    genesis_file: GenesisSource,

    /// L1 RPC URLs to read the stake table contract from, comma-separated.
    #[arg(long, env = "ESPRESSO_INGESTER_L1_PROVIDER", value_delimiter = ',')]
    l1_provider: Vec<Url>,

    /// API services to learn the validators' API services from, and to fill gaps from until then,
    /// comma-separated.
    #[arg(long, env = "ESPRESSO_INGESTER_BOOTSTRAP", value_delimiter = ',')]
    bootstrap: Vec<Url>,

    /// The URL other services reach this deployment's API at, as its validator's record names
    /// it. It is left out of the peers.
    #[arg(long, env = "ESPRESSO_INGESTER_PUBLIC_URL")]
    public_url: Option<Url>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = Options::parse();
    init_logging();
    let backend = RemoteBackend::connect(&options.store)
        .await
        .context("connecting to the store")?;
    let store = ChainStore::new(backend);
    let genesis = Genesis::load(&options.genesis_file)
        .await
        .context("loading the genesis file")?;
    let state = node_state(&genesis, options.l1_provider, store.clone())
        .await
        .context("building the node state")?;
    let membership = Membership::new(&state)?;
    let peers = Peers::new(
        Tables::new(store.clone()),
        options.bootstrap.clone(),
        options.public_url.clone(),
    );
    let filler = GapFiller::new(store.clone(), peers, Some(membership.clone()));
    let follower_store = store.clone();
    let publisher = Publisher::new(store.clone(), membership, options.consensus_api);
    tracing::info!(consensus_api = %options.consensus_api, "ingesting");

    // The ingester runs as long as all of its parts do.
    let mut tasks = JoinSet::new();
    let consensus_api = options.consensus_api;
    tasks.spawn(async move {
        // Returns only when the decide log does not continue the chain in the store.
        ingester::run::<SeqTypes>(consensus_api, &store)
            .await
            .context("ingesting the decide log")
    });
    tasks.spawn(async move {
        filler.run().await;
        anyhow!("the gap filler stopped")
    });
    tasks.spawn(async move {
        // A failing follower holds up only the state catchup serves, never the decide log, so
        // it is started again rather than stopping the ingester.
        loop {
            let peers = Peers::new(
                Tables::new(follower_store.clone()),
                options.bootstrap.clone(),
                options.public_url.clone(),
            );
            match ChainFollower::new(follower_store.clone(), state.clone(), peers)
                .run()
                .await
            {
                Ok(()) => return anyhow!("the chain follower stopped"),
                Err(err) => tracing::error!("following the chain failed: {err:#}"),
            }
            sleep(FOLLOWER_RETRY).await;
        }
    });
    tasks.spawn(async move {
        publisher.run().await;
        anyhow!("the publisher stopped")
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
