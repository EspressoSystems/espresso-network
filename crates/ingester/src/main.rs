//! Copies the decide log of a consensus node into the API service's store.

use std::net::SocketAddr;

use anyhow::Context;
use chain_store::{ChainStore, postgres::PostgresBackend};
use clap::Parser;
use espresso_types::SeqTypes;
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

#[derive(Debug, Parser)]
struct Options {
    /// The address of the consensus node's API.
    #[arg(long, env = "ESPRESSO_INGESTER_CONSENSUS_API")]
    consensus_api: SocketAddr,

    /// The store to write to, as a Postgres URL.
    #[arg(long, env = "ESPRESSO_INGESTER_STORE")]
    store: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = Options::parse();
    init_logging();
    let backend = PostgresBackend::connect(&options.store)
        .await
        .context("connecting to the store")?;
    let store = ChainStore::new(backend);
    tracing::info!(consensus_api = %options.consensus_api, "ingesting");
    // Returns only when the decide log does not continue the chain in the store.
    Err(ingester::run::<SeqTypes>(options.consensus_api, &store).await)
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
