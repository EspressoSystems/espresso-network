//! Serves the chain store over gRPC. It is the only process that writes to the store's database;
//! the ingester and the API service reach the store through it.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{Context, anyhow, ensure};
use chain_store::{Backend, postgres::PostgresBackend, service};
use clap::Parser;
use tokio::{net::TcpListener, task::JoinSet, time::sleep};
use tonic::transport::{Server, server::TcpIncoming};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

/// How often the store prunes.
const PRUNE_INTERVAL: Duration = Duration::from_secs(60);

/// The fewest heights a store may keep. The API services over it resume from snapshots of the
/// chain's state some heights back, and consensus nodes catch up from its recent heights; both
/// need the heights to still be there.
const MIN_KEEP_HEIGHTS: u64 = 1024;

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
            prune(backend, keep).await;
            anyhow!("pruning stopped")
        });
    }
    tasks.spawn(async move {
        let served = Server::builder()
            .add_service(service::server(backend))
            .serve_with_incoming(TcpIncoming::from(listener))
            .await;
        match served {
            Ok(()) => anyhow!("the server stopped"),
            Err(err) => anyhow!(err).context("serving the store"),
        }
    });
    let stopped = tasks.join_next().await.context("no task to run")?;
    Err(stopped.unwrap_or_else(|err| anyhow!("a task failed: {err}")))
}

/// Keeps the newest `keep` heights, pruning the rest every [`PRUNE_INTERVAL`].
async fn prune(backend: Arc<dyn Backend>, keep: u64) {
    loop {
        match backend.newest().await {
            Ok(Some(newest)) => {
                let below = (newest + 1).saturating_sub(keep);
                if let Err(err) = backend.prune_below(below).await {
                    tracing::warn!(below, "pruning failed: {err:#}");
                }
            },
            Ok(None) => {},
            Err(err) => tracing::warn!("reading the newest height failed: {err:#}"),
        }
        sleep(PRUNE_INTERVAL).await;
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
