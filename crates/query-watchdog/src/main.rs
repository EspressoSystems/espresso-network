mod chunk;
mod metrics;
mod store;
mod watch;

use std::{collections::BTreeSet, path::PathBuf, sync::Arc};

use anyhow::{Context, ensure};
use axum::{Router, extract::State, http::StatusCode, routing::get};
use clap::Parser;
use futures::future::try_join_all;
use prometheus::{Registry, TextEncoder};
use tokio::{
    net::TcpListener,
    signal::unix::{SignalKind, signal},
};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};
use url::Url;

use crate::{
    metrics::WatchdogMetrics,
    store::Store,
    watch::{NodeWatch, WatchOptions, Watchdog, WatchedNode},
};

/// Watches archive query nodes for heights they fail to serve.
#[derive(Debug, Parser)]
struct Options {
    /// Metrics and healthcheck port.
    #[arg(long, env = "ESPRESSO_QUERY_WATCHDOG_PORT", default_value_t = 8080)]
    port: u16,
    /// Base URLs without `/v1`, comma-separated.
    #[arg(
        long,
        env = "ESPRESSO_QUERY_WATCHDOG_NODES",
        value_delimiter = ',',
        required = true
    )]
    nodes: Vec<Url>,
    /// SQLite state file.
    #[arg(long, env = "ESPRESSO_QUERY_WATCHDOG_DB_PATH")]
    db_path: PathBuf,
    /// Pacing.
    #[command(flatten)]
    watch: WatchOptions,
}

/// Parses options, opens the store, forgets other nodes, fetches the epoch config, starts each node,
/// and runs every `NodeWatch::run` (`try_join_all`) with `serve_metrics` in one `select!` against
/// `shutdown_signal()`.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = Options::parse();
    init_logging();

    let nodes = options
        .nodes
        .into_iter()
        .map(WatchedNode::new)
        .collect::<anyhow::Result<Vec<_>>>()?;
    let labels: Vec<_> = nodes.iter().map(|node| node.label.clone()).collect();
    ensure!(
        labels.iter().collect::<BTreeSet<_>>().len() == labels.len(),
        "duplicate node labels in {labels:?}"
    );
    let retention_blocks = options.watch.retention_blocks;
    tracing::info!(
        nodes = ?labels,
        db_path = %options.db_path.display(),
        sweep_interval = %humantime::format_duration(options.watch.sweep_interval),
        request_interval = %humantime::format_duration(options.watch.request_interval),
        retention_blocks,
        retention = retention_blocks.is_none().then_some("archive"),
        "config"
    );

    let store = Store::open(&options.db_path).await?;
    let forgotten = store.forget_other_nodes(&labels).await?;
    tracing::info!(count = forgotten, "forgot nodes");

    let registry = Registry::new();
    let metrics = Arc::new(WatchdogMetrics::new(&registry)?);
    let listener = TcpListener::bind(("0.0.0.0", options.port))
        .await
        .with_context(|| format!("binding port {}", options.port))?;
    let mut server = tokio::spawn(serve_metrics(listener, registry));

    let watchdog = Watchdog::new(options.watch, metrics, store)?;
    let epochs = watchdog.fetch_epoch_config(&nodes).await?;
    tracing::info!(
        epoch_height = epochs.map(|e| e.epoch_height),
        epoch_start_block = epochs.map(|e| e.epoch_start_block),
        epochs = epochs.is_none().then_some("off"),
        "epochs"
    );

    let watches = try_join_all(
        nodes
            .iter()
            .map(|node| watchdog.start_node(node.clone(), nodes.clone(), epochs)),
    )
    .await?;

    tokio::select! {
        result = try_join_all(watches.into_iter().map(NodeWatch::run)) => result.map(drop),
        result = &mut server => result?,
        signal = shutdown_signal() => {
            tracing::debug!(signal = signal?, "shutting down");
            Ok(())
        },
    }
}

/// fmt subscriber on stdout, `EnvFilter::from_default_env()`, JSON when `RUST_LOG_FORMAT=json`.
fn init_logging() {
    // `RUST_LOG` unset means INFO, not the `from_default_env` default of ERROR.
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

/// Resolves on SIGINT or SIGTERM and returns its name.
async fn shutdown_signal() -> anyhow::Result<&'static str> {
    let mut sigterm = signal(SignalKind::terminate())?;
    tokio::select! {
        result = tokio::signal::ctrl_c() => result.map(|()| "SIGINT").map_err(Into::into),
        _ = sigterm.recv() => Ok("SIGTERM"),
    }
}

/// `axum::serve` of a `Router` with `/healthcheck` (unit handler, 200) and `/status/metrics`
/// (`State<Registry>`, `TextEncoder::encode_to_string(&registry.gather())`, 500 on encode error).
async fn serve_metrics(listener: TcpListener, registry: Registry) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/healthcheck", get(|| async {}))
        .route("/status/metrics", get(encode_metrics))
        .with_state(registry);
    axum::serve(listener, app).await?;
    Ok(())
}

/// Text exposition of the registry.
async fn encode_metrics(State(registry): State<Registry>) -> Result<String, StatusCode> {
    TextEncoder::new()
        .encode_to_string(&registry.gather())
        .map_err(|error| {
            tracing::error!(%error, "encoding metrics");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}
