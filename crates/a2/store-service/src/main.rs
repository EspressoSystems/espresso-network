//! A storage instance: serves the chain store over gRPC, and takes its share of the decide log of
//! the deployment's consensus node into the store. Storage instances are the only processes that
//! write to the store's database; the API service reaches the store through them. Any number of
//! them can share one database.

use std::{net::SocketAddr, sync::Arc};

use anyhow::{Context, anyhow, ensure};
use chain_store::{
    Backend, ChainStore, payloads::PayloadStore, postgres::PostgresBackend, service,
};
use clap::Parser;
use espresso_types::SeqTypes;
use store_service::MIN_KEEP_HEIGHTS;
use tokio::{net::TcpListener, task::JoinSet};
use tonic::transport::{Server, server::TcpIncoming};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

#[derive(Debug, Parser)]
struct Options {
    /// The address to serve on.
    #[arg(
        long,
        env = "ESPRESSO_STORE_SERVICE_ADDRESS",
        default_value = "0.0.0.0:8070"
    )]
    address: SocketAddr,

    /// The database the store keeps its data in, as a Postgres URL.
    #[arg(long, env = "ESPRESSO_STORE_SERVICE_DATABASE")]
    database: String,

    /// How many of the newest heights to keep; older ones are pruned. Keeps everything if unset.
    /// At least [`MIN_KEEP_HEIGHTS`].
    #[arg(long, env = "ESPRESSO_STORE_SERVICE_KEEP_HEIGHTS")]
    keep_heights: Option<u64>,

    /// The address of the consensus node's API, to take a share of its decide log and of its
    /// payloads from. Without it the instance only serves the store. Needs `--payload-store`:
    /// the node votes on a block only once a storage instance has its payload.
    #[arg(
        long,
        env = "ESPRESSO_STORE_SERVICE_CONSENSUS_API",
        requires = "payload_store"
    )]
    consensus_api: Option<SocketAddr>,

    /// The object store payloads live in, as a URL such as `file:///var/payloads` or
    /// `s3://bucket/prefix`, one object per payload under its VID commitment. Without it,
    /// payloads stay in the database.
    #[arg(
        long,
        env = "ESPRESSO_STORE_SERVICE_PAYLOAD_STORE",
        requires = "payload_dir"
    )]
    payload_store: Option<String>,

    /// Where payloads wait, durable, until they are uploaded to the object store. Private to this
    /// instance.
    #[arg(long, env = "ESPRESSO_STORE_SERVICE_PAYLOAD_DIR")]
    payload_dir: Option<std::path::PathBuf>,
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
    let backend: Arc<dyn Backend> = Arc::new(
        PostgresBackend::connect(&options.database)
            .await
            .context("connecting to the database")?,
    );
    let listener = TcpListener::bind(options.address)
        .await
        .with_context(|| format!("binding {}", options.address))?;
    tracing::info!(address = %options.address, "serving the store");

    // The service runs as long as all of its parts do.
    let mut tasks = JoinSet::new();
    if let Some(keep) = options.keep_heights {
        let backend = backend.clone();
        tasks.spawn(async move {
            store_service::prune(backend, keep).await;
            anyhow!("pruning stopped")
        });
    }
    if let Some(consensus_api) = options.consensus_api {
        let mut store = ChainStore::shared(backend.clone());
        if let (Some(url), Some(dir)) = (&options.payload_store, &options.payload_dir) {
            let payloads =
                PayloadStore::from_url(url, None).context("opening the payload store")?;
            let (staged, uploader) = store
                .with_staged_payloads(payloads, dir.clone())
                .await
                .context("staging payloads")?;
            store = staged;
            tasks.spawn(async move {
                uploader.run().await;
                anyhow!("the payload uploader stopped")
            });
            let payload_store = store.clone();
            tasks.spawn(async move {
                store_service::ingest::payloads::run(consensus_api, &payload_store).await
            });
        }
        tracing::info!(%consensus_api, "taking a share of the decide log");
        tasks.spawn(async move {
            // Returns only when the decide log does not continue the chain in the store.
            store_service::ingest::run::<SeqTypes>(consensus_api, &store)
                .await
                .context("ingesting the decide log")
        });
    }
    let (store_service, follower) = service::server(backend);
    tasks.spawn(async move {
        // Ends only with the store's changes.
        follower.await;
        anyhow!("following the store's tip stopped")
    });
    tasks.spawn(async move {
        // Without TCP_NODELAY a small answer waits for the client's delayed acknowledgement.
        let served = Server::builder()
            .add_service(store_service)
            .serve_with_incoming(TcpIncoming::from(listener).with_nodelay(Some(true)))
            .await;
        match served {
            Ok(()) => anyhow!("the server stopped"),
            Err(err) => anyhow!(err).context("serving the store"),
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
