//! Datadog recipes. JSON logs (`RUST_LOG_FORMAT=json`) put event fields under `fields`.
//!
//! 1. Log setup: facets on `@fields.node` and `@fields.kind`, and a message remapper on
//!    `fields.message`.
//! 2. Either a log monitor or a metric monitor for missing data:
//!    - Log monitor, multi alert grouped by `@fields.node,@fields.kind`. Query:
//!      `service:<svc> status:warn "missing data"`. Message:
//!      `{{[@fields.node].name}} {{[@fields.kind].name}} missing for
//!      {{log.attributes.[fields.oldest_age_seconds]}} s: {{log.attributes.[fields.url]}}`.
//!      One WARN per node and kind per minute while the oldest missing range is older than
//!      `--warn-missing-after`; `url` is the GET URL of that range. Syntax:
//!      docs.datadoghq.com/monitors/notify/variables/ (no `@` in `log.attributes`, brackets
//!      around dotted keys).
//!    - Metric monitor, no link:
//!      `max(last_5m):max:query_watchdog_oldest_missing_age_seconds{*} by {node,kind} > 600`.
//!      The metric name carries the namespace of the Datadog openmetrics check config.
//! 3. Metric monitors on lag and liveness:
//!    - `max(last_5m):max:query_watchdog_follow_lag_blocks{*} by {node} > <blocks>`, and the same
//!      for `query_watchdog_merklized_state_lag_blocks`.
//!    - No-data monitor, or a rate of 0 on `query_watchdog_probes_total` (counter, every probe),
//!      per node: the watchdog is down or stuck.

mod chunk;
mod metrics;
mod store;
mod watch;

use std::{collections::BTreeSet, fmt::Write, path::PathBuf, sync::Arc};

use anyhow::{Context, ensure};
use axum::{Router, extract::State, http::StatusCode, response::Html, routing::get};
use chrono::{SubsecRound, Utc};
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
    store::{MissingRanges, NodeProgress, Store, SweepProgress, SweepState},
    watch::{NodeWatch, SweepWindow, WatchOptions, Watchdog, WatchedNode},
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
    let app = AppState {
        registry,
        metrics: metrics.clone(),
        store: store.clone(),
        nodes: Arc::new(nodes.clone()),
        window: options.watch.sweep_window(),
    };
    let server = tokio::spawn(serve_metrics(listener, app));

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
        result = tip_poller.run() => match result {},
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

/// Rows per node on the missing-data page.
const PAGE_ROWS: u32 = 200;

/// What the HTTP handlers read.
#[derive(Clone)]
struct AppState {
    registry: Registry,
    metrics: Arc<WatchdogMetrics>,
    store: Store,
    nodes: Arc<Vec<WatchedNode>>,
    window: SweepWindow,
}

/// `axum::serve` of a `Router` with `/healthcheck` (unit handler, 200), `/status/metrics`
/// (`TextEncoder::encode_to_string(&registry.gather())`, 500 on encode error) and `/`, the page of
/// missing ranges per node (500 with the error text on a store error).
async fn serve_metrics(listener: TcpListener, app: AppState) -> anyhow::Result<()> {
    let router = Router::new()
        .route("/", get(missing_page))
        .route("/healthcheck", get(|| async {}))
        .route("/status/metrics", get(encode_metrics))
        .with_state(app);
    axum::serve(listener, router).await?;
    Ok(())
}

/// Text exposition of the registry.
async fn encode_metrics(State(app): State<AppState>) -> Result<String, StatusCode> {
    TextEncoder::new()
        .encode_to_string(&app.registry.gather())
        .map_err(|error| {
            tracing::error!(%error, "encoding metrics");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// HTML page of the progress and oldest missing ranges of every node, read from the store.
async fn missing_page(State(app): State<AppState>) -> Result<Html<String>, (StatusCode, String)> {
    let now = Utc::now().trunc_subsecs(0);
    let mut pages = Vec::new();
    for node in app.nodes.iter() {
        let store = app.store.node(&node.label);
        let page = async {
            Ok::<_, anyhow::Error>(NodePage {
                node,
                tip_height: app.metrics.tip_height(&node.label),
                progress: store.load_progress().await?,
                listed: store.missing_ranges(now, PAGE_ROWS).await?,
            })
        }
        .await
        .map_err(|error| {
            tracing::error!(%error, node = %node.label, "reading page data");
            (StatusCode::INTERNAL_SERVER_ERROR, format!("{error:#}"))
        })?;
        pages.push(page);
    }
    Ok(Html(render_page(app.window, &pages)))
}

/// What the page shows for one node.
struct NodePage<'a> {
    node: &'a WatchedNode,
    tip_height: i64,
    /// `None` before the node watch started.
    progress: Option<NodeProgress>,
    listed: MissingRanges,
}

/// Page with one heading, progress line and table per node.
fn render_page(window: SweepWindow, nodes: &[NodePage<'_>]) -> String {
    let mut page = String::from(
        "<!DOCTYPE html><html><head><title>query-watchdog</title></head><body><h1>query-watchdog</\
         h1>",
    );
    for node in nodes {
        write_node(&mut page, window, node).expect("writing to a String");
    }
    page.push_str("</body></html>");
    page
}

/// Appends the heading, progress line and table of one node.
fn write_node(
    page: &mut String,
    window: SweepWindow,
    node_page: &NodePage<'_>,
) -> std::fmt::Result {
    let NodePage {
        node,
        tip_height,
        progress,
        listed,
    } = node_page;
    write!(page, "<h2>{}</h2>", escape(&node.label))?;
    match progress {
        Some(progress) => write!(
            page,
            "<p>tip={tip_height} follow_next_height={} {}; last sweep completed {}</p>",
            progress.follow_next_height,
            describe_sweep(window, &progress.sweep),
            progress
                .last_sweep_completed_at
                .map_or_else(|| "never".to_owned(), |t| t.to_rfc3339()),
        )?,
        None => page.write_str("<p>not started</p>")?,
    }
    if listed.shown.is_empty() {
        return page.write_str("<p>nothing missing</p>");
    }
    page.write_str(
        "<table border=\"1\"><tr><th>kind</th><th>range_start</th><th>range_end</th><th>age</\
         th><th>link</th></tr>",
    )?;
    for missing in &listed.shown {
        let probe = &missing.probe;
        let url = escape(node.probe_url(probe).as_str());
        let kind: &str = probe.kind.into();
        let age = humantime::format_duration(missing.age);
        write!(
            page,
            "<tr><td>{kind}</td><td>{}</td><td>{}</td><td>{age}</td><td><a \
             href=\"{url}\">{url}</a></td></tr>",
            probe.range.start, probe.range.end
        )?;
    }
    page.write_str("</table>")?;
    let hidden = listed.total - listed.shown.len() as u64;
    if hidden > 0 {
        write!(page, "<p>{hidden} more not shown</p>")?;
    }
    Ok(())
}

/// `sweep running 45% (next 300 of 1000)` or `sweep waiting until <time>`.
fn describe_sweep(window: SweepWindow, sweep: &SweepState) -> String {
    match sweep {
        SweepState::Running(SweepProgress {
            until_height,
            next_height,
            ..
        }) => {
            // The sweep started at `window.start_height(until_height)`, its tip then.
            let start = window.start_height(*until_height);
            let total = until_height.saturating_sub(start).max(1);
            let done = next_height.saturating_sub(start).min(total);
            format!(
                "sweep running {}% (next {next_height} of {until_height})",
                done * 100 / total
            )
        },
        SweepState::Waiting { start_at } => {
            format!("sweep waiting until {}", start_at.to_rfc3339())
        },
    }
}

/// Escapes text for HTML content and double-quoted attributes.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
