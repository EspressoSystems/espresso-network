//! Watches archive query nodes for data they fail to serve; see `README.md`.

mod chunk;
mod gaps;
mod http;
mod metrics;
mod store;
mod watch;

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, ensure};
use clap::Parser;
use directories::ProjectDirs;
use futures::future::try_join_all;
use prometheus::Registry;
use tokio::{
    net::TcpListener,
    signal::unix::{SignalKind, signal},
};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};
use url::Url;

use crate::{
    http::AppState,
    metrics::WatchdogMetrics,
    store::Store,
    watch::{NodeWatch, SweepEtas, WatchOptions, Watchdog, WatchedNode},
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
    /// SQLite state file; default `state.sqlite` in the platform data directory of `query-watchdog`.
    #[arg(long, env = "ESPRESSO_QUERY_WATCHDOG_DB_PATH")]
    db_path: Option<PathBuf>,
    /// Pacing.
    #[command(flatten)]
    watch: WatchOptions,
}

impl Options {
    /// `--db-path`, or `state.sqlite` in the platform data directory; errors when there is none.
    fn resolve_db_path(&self) -> anyhow::Result<PathBuf> {
        if let Some(path) = &self.db_path {
            return Ok(path.clone());
        }
        let dirs = ProjectDirs::from("", "espresso", "query-watchdog")
            .context("no home directory found for the default state file, pass --db-path")?;
        Ok(dirs.data_dir().join("state.sqlite"))
    }
}

/// Parses options, opens the store, forgets other nodes, opens the read-only pool of the page,
/// spawns `http::serve`, fetches the epoch config and the first tip, starts each node, and runs
/// `TipPoller::run` and every `NodeWatch::run` in one `select!` with the server and
/// `shutdown_signal()`.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = Options::parse();
    init_logging();

    let nodes = options
        .nodes
        .iter()
        .cloned()
        .map(WatchedNode::new)
        .collect::<anyhow::Result<Vec<_>>>()?;
    let labels: Vec<_> = nodes.iter().map(|node| node.label.clone()).collect();
    ensure!(
        labels.iter().collect::<BTreeSet<_>>().len() == labels.len(),
        "duplicate node labels in {labels:?}"
    );
    let db_path = options.resolve_db_path()?;
    log_config(&options, &db_path, &labels);

    let store = Store::open(&db_path).await?;
    let forgotten = store.forget_other_nodes(&labels).await?;
    tracing::info!(count = forgotten, "forgot nodes");

    let registry = Registry::new();
    let metrics = Arc::new(WatchdogMetrics::new(&registry)?);
    let listener = TcpListener::bind(("0.0.0.0", options.port))
        .await
        .with_context(|| format!("binding port {}", options.port))?;
    let reader = Store::open_read_only(&db_path).await?;
    let sweep_etas = SweepEtas::default();
    let window = options.watch.sweep_window();
    let app = AppState::new(registry, reader, nodes.clone(), window, sweep_etas.clone());
    let server = tokio::spawn(http::serve(listener, app.clone()));
    log_serving(options.port);

    let watchdog = Watchdog::new(options.watch, metrics, store, sweep_etas)?;
    let epochs = watchdog.fetch_epoch_config(&nodes).await?;
    tracing::info!(
        epoch_height = epochs.map(|e| e.epoch_height),
        epoch_start_block = epochs.map(|e| e.epoch_start_block),
        epochs = epochs.is_none().then_some("off"),
        "epochs"
    );

    let (tip_poller, tip) = watchdog.start_tip_poller(nodes.clone()).await?;
    app.set_tip(tip.clone());
    let watches = try_join_all(
        nodes
            .iter()
            .map(|node| watchdog.start_node(node.clone(), tip.clone(), epochs)),
    )
    .await?;

    tokio::select! {
        result = try_join_all(watches.into_iter().map(NodeWatch::run)) => result.map(drop),
        result = tip_poller.run() => result.map(drop),
        result = server => result?,
        signal = shutdown_signal() => {
            tracing::debug!(signal = signal?, "shutting down");
            Ok(())
        },
    }
}

/// The `config` INFO line.
fn log_config(options: &Options, db_path: &Path, labels: &[String]) {
    let watch = &options.watch;
    tracing::info!(
        nodes = ?labels,
        db_path = %db_path.display(),
        sweep_interval = %humantime::format_duration(watch.sweep_interval),
        request_interval = %humantime::format_duration(watch.request_interval),
        alert_missing_after = %humantime::format_duration(watch.alert_missing_after),
        retention_blocks = watch.retention_blocks,
        retention = watch.retention_blocks.is_none().then_some("archive"),
        "config"
    );
}

/// The `serving` INFO line with the endpoint URLs.
fn log_serving(port: u16) {
    tracing::info!(
        page = %format!("http://localhost:{port}/"),
        metrics = %format!("http://localhost:{port}/status/metrics"),
        healthcheck = %format!("http://localhost:{port}/healthcheck"),
        "serving"
    );
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
