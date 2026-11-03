//! Watches archive query nodes for data they fail to serve.
//!
//! Datadog recipes. JSON logs (`RUST_LOG_FORMAT=json`) put event fields under `fields`, so the
//! attributes are `@fields.node`, `@fields.kind` and so on.
//!
//! 1. Log monitor, multi alert grouped by `@fields.node,@fields.kind`:
//!    - Query: `service:<svc> status:warn @fields.message:"missing data"`.
//!    - Message: `{{[@fields.node].name}} {{[@fields.kind].name}} missing for
//!      {{log.attributes.[fields.oldest_age_seconds]}} s: {{log.attributes.[fields.url]}}`.
//!    - One WARN per node and kind per minute while the oldest missing range is older than
//!      `--warn-missing-after`. The `url` field is the GET URL of that range.
//!    - Syntax from docs.datadoghq.com/monitors/notify/variables/: no `@` in `log.attributes`,
//!      brackets around dotted keys. The docs do not cover nested JSON, so check one rendered alert.
//! 2. Metric monitor on age:
//!    `max(last_5m):max:query_watchdog_oldest_missing_age_seconds{*} by {node,kind} > 600`.
//!    The metric name carries the namespace of the Datadog openmetrics check config.
//! 3. Metric monitors on lag and liveness:
//!    - `max(last_5m):max:query_watchdog_follow_lag_blocks{*} by {node} > <blocks>`, and the same
//!      for `query_watchdog_merklized_state_lag_blocks`.
//!    - No-data monitor on any series, for example `query_watchdog_follow_lag_blocks`, notifying
//!      on no data: the watchdog is down.

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

/// Parses options, opens the store, forgets other nodes, spawns `serve_metrics`, fetches the epoch
/// config and the first tip, starts each node, and runs `TipPoller::run` and every `NodeWatch::run`
/// in one `select!` with the server and `shutdown_signal()`.
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
        warn_missing_after = %humantime::format_duration(options.watch.warn_missing_after),
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
    let server = tokio::spawn(serve_metrics(listener, registry));

    let watchdog = Watchdog::new(options.watch, metrics, store)?;
    let epochs = watchdog.fetch_epoch_config(&nodes).await?;
    tracing::info!(
        epoch_height = epochs.map(|e| e.epoch_height),
        epoch_start_block = epochs.map(|e| e.epoch_start_block),
        epochs = epochs.is_none().then_some("off"),
        "epochs"
    );

    let (tip_poller, tip) = watchdog.start_tip_poller(nodes.clone()).await?;
    let watches = try_join_all(
        nodes
            .iter()
            .map(|node| watchdog.start_node(node.clone(), tip.clone(), epochs)),
    )
    .await?;

    tokio::select! {
        result = try_join_all(watches.into_iter().map(NodeWatch::run)) => result.map(drop),
        result = tip_poller.run() => result,
        result = server => result?,
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
