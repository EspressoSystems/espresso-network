//! A whole deployment in one process: the consensus node, a storage instance over the store, and an
//! API instance that repairs the store, for a small validator on one host.
//!
//! The parts talk as they do across processes, except where a payload passes: the storage instance
//! takes each payload from consensus's payload queue, sharing the buffer consensus holds, writes it
//! to its staging directory once, and keeps it in memory, where the API instance reads it from. The
//! decide log too is called directly rather than over the consensus API, and the API instance
//! reaches the store through a `LocalStore` rather than the store service. Submissions still reach
//! consensus through its API, which also serves storage instances in other processes.

use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Context, anyhow, ensure};
use api_service::{ApiService, Store, catchup::Catchup, compat::Compat, repair::Repair};
use chain_store::{Backend, ChainStore, payloads::PayloadStore, postgres::PostgresBackend};
use clap::Parser;
use consensus::storage::DEFAULT_DECIDE_LOG_LIMIT;
use consensus_node::node::{Config, Local, run_with};
use espresso_genesis::{Genesis, GenesisSource};
use espresso_keyset::{KeySet, KeySetOptions};
use espresso_types::SeqTypes;
use hotshot_types::{addr::NetAddr, traits::metrics::NoMetrics};
use store_api::LocalStore;
use store_service::{MIN_KEEP_HEIGHTS, ingest};
use tokio::{net::TcpListener, runtime::Handle, task::JoinSet};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};
use url::Url;

#[derive(Debug, Parser)]
struct Options {
    #[clap(flatten)]
    keys: KeySetOptions,

    /// The genesis file of the chain, a path or an HTTP URL.
    #[arg(long, env = "ESPRESSO_DEPLOYMENT_GENESIS_FILE")]
    genesis_file: GenesisSource,

    /// L1 RPC URLs, comma-separated.
    #[arg(long, env = "ESPRESSO_DEPLOYMENT_L1_PROVIDER", value_delimiter = ',')]
    l1_provider: Vec<Url>,

    /// The directory of the consensus store.
    #[arg(long, env = "ESPRESSO_DEPLOYMENT_CONSENSUS_STORE_PATH")]
    consensus_store_path: PathBuf,

    /// Where to listen for the other consensus nodes, as `host:port`.
    #[arg(
        long,
        env = "ESPRESSO_DEPLOYMENT_BIND_ADDRESS",
        default_value = "0.0.0.0:9977"
    )]
    bind_address: NetAddr,

    /// Where to serve the consensus API, for submissions and for storage instances in other
    /// processes.
    #[arg(
        long,
        env = "ESPRESSO_DEPLOYMENT_CONSENSUS_API_ADDRESS",
        default_value = "127.0.0.1:9978"
    )]
    consensus_api_address: SocketAddr,

    /// The URL other services reach this deployment's API at, which the validator's signed
    /// record tells them.
    #[arg(long, env = "ESPRESSO_DEPLOYMENT_PUBLIC_API_URL")]
    public_api_url: Url,

    /// API services to catch up and repair from besides this one, comma-separated.
    #[arg(long, env = "ESPRESSO_DEPLOYMENT_BOOTSTRAP", value_delimiter = ',')]
    bootstrap: Vec<Url>,

    /// The state relay server to send light client state signatures to.
    #[arg(long, env = "ESPRESSO_DEPLOYMENT_STATE_RELAY_SERVER_URL")]
    state_relay_server_url: Option<Url>,

    /// How many bytes the decide log may take up before the node drops its oldest entries.
    #[arg(long, env = "ESPRESSO_DEPLOYMENT_DECIDE_LOG_LIMIT", default_value_t = DEFAULT_DECIDE_LOG_LIMIT)]
    decide_log_limit: u64,

    /// How long a leader waits for transactions before proposing an empty block.
    #[arg(
        long,
        env = "ESPRESSO_DEPLOYMENT_EMPTY_BLOCK_DELAY",
        default_value = "1s",
        value_parser = humantime::parse_duration
    )]
    empty_block_delay: Duration,

    /// The database the store keeps its data in, as a Postgres URL.
    #[arg(long, env = "ESPRESSO_DEPLOYMENT_DATABASE")]
    database: String,

    /// How many of the newest heights to keep; older ones are pruned. Keeps everything if unset.
    /// At least [`MIN_KEEP_HEIGHTS`].
    #[arg(long, env = "ESPRESSO_DEPLOYMENT_KEEP_HEIGHTS")]
    keep_heights: Option<u64>,

    /// The object store payloads live in, as a URL such as `file:///var/payloads` or
    /// `s3://bucket/prefix`.
    #[arg(long, env = "ESPRESSO_DEPLOYMENT_PAYLOAD_STORE")]
    payload_store: String,

    /// Where payloads wait, durable, until they are uploaded to the object store.
    #[arg(long, env = "ESPRESSO_DEPLOYMENT_PAYLOAD_DIR")]
    payload_dir: PathBuf,

    /// The URL clients fetch the payload store's objects from, if they can reach it.
    #[arg(long, env = "ESPRESSO_DEPLOYMENT_PAYLOAD_URL")]
    payload_url: Option<String>,

    /// How many bytes of the newest payloads to keep in memory, for the API to serve without
    /// reading them back.
    #[arg(
        long,
        env = "ESPRESSO_DEPLOYMENT_RECENT_PAYLOADS",
        default_value_t = 1 << 30
    )]
    recent_payloads: usize,

    /// The port to serve the API on, HTTP and gRPC.
    #[arg(long, env = "ESPRESSO_DEPLOYMENT_API_PORT", default_value_t = 8080)]
    api_port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = Options::parse();
    init_logging();
    if let Some(keep) = options.keep_heights {
        ensure!(
            keep >= MIN_KEEP_HEIGHTS,
            "--keep-heights must be at least {MIN_KEEP_HEIGHTS}, not {keep}"
        );
    }
    let KeySet {
        staking,
        state,
        x25519,
    } = options.keys.try_into()?;
    let genesis = Genesis::load(&options.genesis_file)
        .await
        .context("loading the genesis file")?;

    // The storage instance and the API instance run apart from consensus, which they would
    // otherwise hold up whenever they keep every worker busy. Their connections and sockets are
    // opened on their own runtime, whose driver then serves them.
    let parts = tokio::runtime::Builder::new_multi_thread()
        .thread_name("deployment-store")
        .enable_all()
        .build()
        .context("starting the runtime of the storage and API instances")?;
    let handle = parts.handle().clone();
    let (database, payload_store, payload_url, payload_dir, recent, api_port) = (
        options.database,
        options.payload_store,
        options.payload_url,
        options.payload_dir,
        options.recent_payloads,
        options.api_port,
    );
    let opened = handle.spawn(async move {
        let backend: Arc<dyn Backend> = Arc::new(
            PostgresBackend::connect(&database)
                .await
                .context("connecting to the database")?,
        );
        let payloads = PayloadStore::from_url(&payload_store, payload_url)
            .context("opening the payload store")?;
        let (store, uploader) = ChainStore::shared(backend.clone())
            .with_recent_payloads(recent)
            .with_staged_payloads(payloads, payload_dir)
            .await
            .context("staging payloads")?;
        let listener = TcpListener::bind(("0.0.0.0", api_port))
            .await
            .with_context(|| format!("binding port {api_port}"))?;
        anyhow::Ok((backend, store, uploader, listener))
    });
    let (backend, store, uploader, listener) = opened.await??;

    let repair = Repair {
        l1: options.l1_provider.clone(),
        bootstrap: options.bootstrap.clone(),
        public_url: Some(options.public_api_url.clone()),
    };
    let config = Config {
        genesis: genesis.clone(),
        staking_key: staking,
        state_key: state,
        x25519_key: x25519,
        l1: options.l1_provider,
        store: options.consensus_store_path,
        bind: options.bind_address,
        consensus_api: options.consensus_api_address,
        api_url: options.public_api_url,
        api_key: None,
        bootstrap: options.bootstrap,
        relay: options.state_relay_server_url,
        decide_log_limit: options.decide_log_limit,
        empty_block_delay: options.empty_block_delay,
    };
    let keep_heights = options.keep_heights;
    let ran = run_with(config, Box::new(NoMetrics), move |local, tasks| {
        tasks.spawn(on(handle.clone(), async move {
            uploader.run().await;
            anyhow!("the payload uploader stopped")
        }));
        if let Some(keep) = keep_heights {
            tasks.spawn(on(handle.clone(), async move {
                store_service::prune(backend, keep).await;
                anyhow!("pruning stopped")
            }));
        }
        run_store(local, store, listener, genesis, repair, &handle, tasks);
    })
    .await;
    parts.shutdown_background();
    ran
}

/// Runs `task` on the runtime of `handle`, and ends when it does.
async fn on(
    handle: Handle,
    task: impl Future<Output = anyhow::Error> + Send + 'static,
) -> anyhow::Error {
    match handle.spawn(task).await {
        Ok(stopped) => stopped,
        Err(err) => anyhow!("a task failed: {err}"),
    }
}

/// Adds to `tasks` the storage instance, which takes what `local` offers into `store`, and the
/// API instance over `store`, served on `listener` and repairing the store with `repair`, both on
/// the runtime of `handle`.
fn run_store(
    local: Local,
    store: ChainStore,
    listener: TcpListener,
    genesis: Genesis,
    repair: Repair,
    handle: &Handle,
    tasks: &mut JoinSet<anyhow::Error>,
) {
    let Local {
        payloads,
        decide_log,
        consensus_api,
    } = local;
    let taker = store.clone();
    tasks.spawn(on(handle.clone(), async move {
        ingest::payloads::run_local(payloads, &taker).await
    }));
    let ingester = store.clone();
    tasks.spawn(on(handle.clone(), async move {
        ingest::run_local::<SeqTypes, _>(decide_log, &ingester)
            .await
            .context("ingesting the decide log")
    }));
    tasks.spawn(on(handle.clone(), async move {
        serve_api(store, consensus_api, listener, genesis, repair).await
    }));
}

/// Serves the API over `store` on `listener`, submitting transactions to the consensus API at
/// `consensus_api`, and repairs the store, until either stops.
async fn serve_api(
    store: ChainStore,
    consensus_api: SocketAddr,
    listener: TcpListener,
    genesis: Genesis,
    repair: Repair,
) -> anyhow::Error {
    let config = match genesis.unstaked_network_config() {
        Ok(config) => config,
        Err(err) => return err.context("reading the genesis file's network config"),
    };
    let epoch_height = config.config.epoch_height;
    // Repair's payloads are uploaded at once; see `LocalStore::writing_through`.
    let writer = store.unstaged();
    let (local, follower) = LocalStore::new(store);
    let local = local.writing_through(writer);
    let store: Arc<dyn Store> = Arc::new(local);
    let api = ApiService::new(store.clone(), consensus_api, epoch_height)
        .with_compat(Compat::new(store.clone(), config))
        .with_catchup(Catchup::new(store.clone(), epoch_height));
    tokio::select! {
        served = api.serve(listener) => match served {
            Ok(()) => anyhow!("the server stopped"),
            Err(err) => err.context("serving the API"),
        },
        repaired = repair.run(store, &genesis) => repaired,
        () = follower.run() => anyhow!("following the store's tip stopped"),
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
