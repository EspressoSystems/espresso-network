//! Serves the chain store over gRPC. It is the only process that writes to the store's database;
//! the ingester and the API service reach the store through it.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Context;
use chain_store::{Backend, postgres::PostgresBackend, service};
use clap::Parser;
use tokio::{net::TcpListener, time::sleep};
use tonic::transport::{Server, server::TcpIncoming};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

/// How often the store prunes.
const PRUNE_INTERVAL: Duration = Duration::from_secs(60);

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
    #[arg(long, env = "ESPRESSO_STORE_SERVICE_KEEP_HEIGHTS")]
    keep_heights: Option<u64>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = Options::parse();
    init_logging();
    let backend: Arc<dyn Backend> = Arc::new(
        PostgresBackend::connect(&options.database)
            .await
            .context("connecting to the database")?,
    );
    if let Some(keep) = options.keep_heights {
        tokio::spawn(prune(backend.clone(), keep));
    }
    let listener = TcpListener::bind(options.address)
        .await
        .with_context(|| format!("binding {}", options.address))?;
    tracing::info!(address = %options.address, "serving the store");
    Server::builder()
        .add_service(service::server(backend))
        .serve_with_incoming(TcpIncoming::from(listener))
        .await?;
    Ok(())
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
