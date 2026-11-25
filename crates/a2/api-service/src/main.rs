//! An API instance: serves the v2 API from the chain store, and repairs the store unless told not
//! to.

use std::{net::SocketAddr, sync::Arc};

use anyhow::{Context, anyhow};
use api_service::{ApiService, Store, catchup::Catchup, compat::Compat, repair::Repair};
use clap::Parser;
use espresso_genesis::{Genesis, GenesisSource};
use store_api::RemoteStore;
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, filter::LevelFilter};
use url::Url;

#[derive(Debug, Parser)]
struct Options {
    /// The port to serve HTTP and gRPC on.
    #[arg(long, env = "ESPRESSO_API_SERVICE_PORT", default_value_t = 8080)]
    port: u16,

    /// The store service of a storage instance of the deployment, as a URL. The instance reaches
    /// the store only through it.
    #[arg(long, env = "ESPRESSO_API_SERVICE_STORE")]
    store: String,

    /// The address of the consensus node's API, for submissions.
    #[arg(long, env = "ESPRESSO_API_SERVICE_CONSENSUS_API")]
    consensus_api: SocketAddr,

    /// The genesis file of the chain, a path or an HTTP URL, for its epochs and genesis stake
    /// table.
    #[arg(long, env = "ESPRESSO_API_SERVICE_GENESIS_FILE")]
    genesis_file: GenesisSource,

    /// Do not repair the store. By default an instance fills what the store lacks from other API
    /// services, sharing the work with the other instances that repair it, and writes the state
    /// that state catchup is answered from; a deployment needs at least one instance that does.
    #[arg(long, env = "ESPRESSO_API_SERVICE_NO_REPAIR")]
    no_repair: bool,

    /// L1 RPC URLs to read the stake table contract from, comma-separated; for repair, and
    /// required unless `--no-repair` is given.
    #[arg(
        long,
        env = "ESPRESSO_API_SERVICE_L1_PROVIDER",
        value_delimiter = ',',
        required_unless_present = "no_repair"
    )]
    l1_provider: Vec<Url>,

    /// API services to learn the validators' API services from, and to repair from until then,
    /// comma-separated.
    #[arg(long, env = "ESPRESSO_API_SERVICE_BOOTSTRAP", value_delimiter = ',')]
    bootstrap: Vec<Url>,

    /// The URL other services reach this deployment's API at, as its validator's record names
    /// it. It is never asked for repairs.
    #[arg(long, env = "ESPRESSO_API_SERVICE_PUBLIC_URL")]
    public_url: Option<Url>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = Options::parse();
    init_logging();
    let store: Arc<dyn Store> = Arc::new(
        RemoteStore::connect(&options.store)
            .await
            .context("connecting to the store")?,
    );
    let listener = TcpListener::bind(("0.0.0.0", options.port))
        .await
        .with_context(|| format!("binding port {}", options.port))?;
    let genesis = Genesis::load(&options.genesis_file)
        .await
        .context("loading the genesis file")?;
    let config = genesis.unstaked_network_config()?;
    let epoch_height = config.config.epoch_height;
    let api = ApiService::new(store.clone(), options.consensus_api, epoch_height)
        .with_compat(Compat::new(store.clone(), config))
        .with_catchup(Catchup::new(store.clone(), epoch_height));
    let repair = async {
        if options.no_repair {
            return std::future::pending().await;
        }
        let repair = Repair {
            l1: options.l1_provider,
            bootstrap: options.bootstrap,
            public_url: options.public_url,
        };
        Err(repair.run(store, &genesis).await)
    };
    tokio::select! {
        served = api.serve(listener) => match served {
            Ok(()) => Err(anyhow!("the server stopped")),
            Err(err) => Err(err.context("serving the API")),
        },
        repaired = repair => repaired,
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
