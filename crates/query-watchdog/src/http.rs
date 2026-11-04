//! HTTP endpoints: `/healthcheck`, `/status/metrics` and `/`, the page of missing ranges.

use std::{
    fmt::Write,
    sync::{Arc, OnceLock},
    time::Duration,
};

use axum::{Json, Router, extract::State, http::StatusCode, response::Html, routing::get};
use chrono::{DateTime, Utc};
use prometheus::{Registry, TextEncoder};
use serde::Serialize;
use tokio::{net::TcpListener, sync::watch::Receiver};

use crate::{
    store::{KindSummary, MissingRanges, NodeProgress, Store, SweepProgress, SweepState},
    watch::{SweepEtas, SweepWindow, WatchedNode, utc_now},
};

/// Rows per node on the missing-data page.
const PAGE_ROWS: u32 = 200;

/// What the HTTP handlers read.
#[derive(Clone)]
pub struct AppState {
    registry: Registry,
    store: Store,
    nodes: Arc<[WatchedNode]>,
    window: SweepWindow,
    tip: Arc<OnceLock<Receiver<u64>>>,
    sweep_etas: SweepEtas,
}

impl AppState {
    /// `store` should be a read-only pool; the tip is unset until `set_tip`.
    pub fn new(
        registry: Registry,
        store: Store,
        nodes: Vec<WatchedNode>,
        window: SweepWindow,
        sweep_etas: SweepEtas,
    ) -> Self {
        Self {
            registry,
            store,
            nodes: nodes.into(),
            window,
            tip: Arc::default(),
            sweep_etas,
        }
    }

    /// Publishes the tip receiver once the tip poller has started.
    pub fn set_tip(&self, tip: Receiver<u64>) {
        self.tip.set(tip).expect("the tip is set once");
    }
}

/// `axum::serve` of a `Router` with `/healthcheck` (`{"status":"available"}`, 200),
/// `/status/metrics`
/// (`TextEncoder::encode_to_string(&registry.gather())`, 500 on encode error) and `/`, the page of
/// missing ranges per node (500 with the error text on a store error).
pub async fn serve(listener: TcpListener, app: AppState) -> anyhow::Result<()> {
    let router = Router::new()
        .route("/", get(missing_page))
        .route("/healthcheck", get(healthcheck))
        .route("/status/metrics", get(encode_metrics))
        .with_state(app);
    axum::serve(listener, router).await?;
    Ok(())
}

/// Body of `/healthcheck`.
#[derive(Serialize)]
struct Health {
    status: &'static str,
}

/// Always available while the process serves.
async fn healthcheck() -> Json<Health> {
    Json(Health {
        status: "available",
    })
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
    let now = utc_now();
    let tip = app.tip.get().map(|tip| *tip.borrow());
    let mut pages = Vec::new();
    for node in app.nodes.iter() {
        let page = load_node_page(&app, node, tip, now)
            .await
            .map_err(|error| {
                tracing::error!(%error, node = %node.label, "reading page data");
                (StatusCode::INTERNAL_SERVER_ERROR, format!("{error:#}"))
            })?;
        pages.push(page);
    }
    Ok(Html(render_page(app.window, &pages)))
}

/// Time left of the running sweep and when it ends.
struct SweepEta {
    remaining: Duration,
    finish: DateTime<Utc>,
}

/// What the page shows for one node.
struct NodePage<'a> {
    node: &'a WatchedNode,
    /// `None` until the tip poller has started.
    tip_height: Option<u64>,
    /// `None` until this process has swept a height of the running sweep.
    sweep_eta: Option<SweepEta>,
    /// `None` before the node watch started.
    progress: Option<NodeProgress>,
    /// One entry per kind.
    kinds: Vec<KindSummary>,
    listed: MissingRanges,
}

/// Reads the page data of `node` from the store.
async fn load_node_page<'a>(
    app: &AppState,
    node: &'a WatchedNode,
    tip_height: Option<u64>,
    now: DateTime<Utc>,
) -> anyhow::Result<NodePage<'a>> {
    let store = app.store.node(&node.label);
    let finish = app
        .sweep_etas
        .lock()
        .expect("sweep_etas lock poisoned")
        .get(&node.label)
        .copied();
    // A finish time already passed reads as no time left.
    let sweep_eta = finish.map(|finish| SweepEta {
        remaining: (finish - now).to_std().unwrap_or(Duration::ZERO),
        finish,
    });
    Ok(NodePage {
        node,
        tip_height,
        sweep_eta,
        progress: store.load_progress().await?,
        kinds: store.missing_summary(now).await?,
        listed: store.missing_ranges(now, PAGE_ROWS).await?,
    })
}

/// Page with the endpoint links, then one heading, progress line and table per node.
fn render_page(window: SweepWindow, nodes: &[NodePage<'_>]) -> String {
    let mut page = String::from(
        "<!DOCTYPE html><html><head><title>query-watchdog</title></head><body><h1>query-watchdog</\
         h1><ul><li><a href=\"/status/metrics\">/status/metrics</a></li><li><a \
         href=\"/healthcheck\">/healthcheck</a></li></ul>",
    );
    for node in nodes {
        write_node(&mut page, window, node).expect("writing to a String");
    }
    page.push_str("</body></html>");
    page
}

/// Appends the heading, one line per progress fact and the missing table of one node.
fn write_node(
    page: &mut String,
    window: SweepWindow,
    node_page: &NodePage<'_>,
) -> std::fmt::Result {
    let NodePage {
        node,
        tip_height,
        sweep_eta,
        progress,
        kinds,
        listed,
    } = node_page;
    write!(
        page,
        "<h2><a href=\"{}\">{}</a></h2><ul>",
        escape(node.base_url.as_str()),
        escape(&node.label)
    )?;
    let tip = tip_height.map_or_else(|| "no tip yet".to_owned(), |tip| tip.to_string());
    write_line(page, &format!("tip: {tip}"))?;
    match progress {
        Some(progress) => {
            for line in progress_lines(window, *tip_height, progress, sweep_eta.as_ref()) {
                write_line(page, &line)?;
            }
        },
        None => write_line(page, "not started")?,
    }
    for line in missing_lines(kinds) {
        write_line(page, &line)?;
    }
    page.write_str("</ul>")?;
    write_missing_table(page, node, listed)
}

/// Appends `text` as a list item.
fn write_line(page: &mut String, text: &str) -> std::fmt::Result {
    write!(page, "<li>{}</li>", escape(text))
}

/// Follow, sweep and last-sweep lines of a started node.
fn progress_lines(
    window: SweepWindow,
    tip: Option<u64>,
    progress: &NodeProgress,
    eta: Option<&SweepEta>,
) -> Vec<String> {
    let follow = progress.follow_next_height;
    let lag = tip.map_or_else(String::new, |tip| {
        format!(" (lag {})", tip.saturating_sub(follow))
    });
    let mut lines = vec![format!("follow next height: {follow}{lag}")];
    lines.extend(sweep_lines(window, &progress.sweep, eta));
    let completed = progress
        .last_sweep_completed_at
        .map_or_else(|| "never".to_owned(), |t| t.to_rfc3339());
    lines.push(format!("last sweep completed: {completed}"));
    lines
}

/// The sweep lines: running (see `running_lines`) or `sweep: waiting until <time>`.
fn sweep_lines(window: SweepWindow, sweep: &SweepState, eta: Option<&SweepEta>) -> Vec<String> {
    match sweep {
        SweepState::Running(sweep) => running_lines(window, sweep, eta),
        SweepState::Waiting { start_at } => {
            vec![format!("sweep: waiting until {}", start_at.to_rfc3339())]
        },
    }
}

/// `sweep: running 45.0000%` with its ETA, next and until heights; an empty window reads 100%.
fn running_lines(
    window: SweepWindow,
    sweep: &SweepProgress,
    eta: Option<&SweepEta>,
) -> Vec<String> {
    // The sweep started at `window.start_height` of its tip, which is `until_height`.
    let start = window.start_height(sweep.until_height);
    let total = sweep.until_height.saturating_sub(start);
    let done = sweep.next_height.saturating_sub(start).min(total);
    let percent = if total == 0 {
        100.0
    } else {
        done as f64 * 100.0 / total as f64
    };
    let eta = eta.map_or_else(|| "unknown".to_owned(), format_eta);
    vec![
        format!("sweep: running {percent:.4}%"),
        format!("sweep eta: {eta}"),
        format!("sweep next height: {}", sweep.next_height),
        format!("sweep until height: {}", sweep.until_height),
    ]
}

/// `12m 3s (<finish time>)`.
fn format_eta(eta: &SweepEta) -> String {
    format!(
        "{} ({})",
        humantime::format_duration(eta.remaining),
        eta.finish.to_rfc3339()
    )
}

/// One line per kind with missing data, or `missing: none`.
fn missing_lines(kinds: &[KindSummary]) -> Vec<String> {
    let lines: Vec<_> = kinds
        .iter()
        .filter(|kind| kind.missing_count > 0)
        .map(|kind| {
            let unit = if kind.kind.is_per_epoch() {
                "epochs"
            } else {
                "heights"
            };
            format!(
                "missing {}: {} {unit}",
                <&str>::from(kind.kind),
                kind.missing_count
            )
        })
        .collect();
    if lines.is_empty() {
        vec!["missing: none".to_owned()]
    } else {
        lines
    }
}

/// Appends the missing ranges of `node`, or "nothing missing".
fn write_missing_table(
    page: &mut String,
    node: &WatchedNode,
    listed: &MissingRanges,
) -> std::fmt::Result {
    if listed.shown.is_empty() {
        return page.write_str("<p>nothing missing</p>");
    }
    page.write_str(
        "<table border=\"1\"><tr><th>kind</th><th>range_start</th><th>range_end</th><th>age</\
         th><th>link</th></tr>",
    )?;
    for missing in &listed.shown {
        let probe = &missing.probe;
        let url = escape(node.route_url(&probe.route()).as_str());
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

/// Escapes text for HTML content and double-quoted attributes.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use url::Url;

    use super::*;

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).unwrap()
    }

    fn running(next_height: u64, until_height: u64) -> SweepState {
        SweepState::Running(SweepProgress {
            until_height,
            next_height,
            started_at: at(0),
        })
    }

    #[test]
    fn sweep_lines_percent() {
        let retention = SweepWindow::Retention { blocks: 500 };
        let cases = [
            (SweepWindow::Archive, running(250, 1000), "25.0000"),
            (retention, running(750, 1000), "50.0000"),
            (retention, running(900, 1000), "80.0000"),
            (SweepWindow::Archive, running(1, 3), "33.3333"),
            // Window start rounds up past the tip: empty window.
            (
                SweepWindow::Retention { blocks: 10 },
                running(1050, 1050),
                "100.0000",
            ),
            // `next` beyond `until` caps at 100%.
            (SweepWindow::Archive, running(1200, 1000), "100.0000"),
        ];
        for (window, sweep, percent) in cases {
            let lines = sweep_lines(window, &sweep, None);
            assert_eq!(lines[0], format!("sweep: running {percent}%"));
            assert_eq!(lines.len(), 4);
            assert_eq!(lines[1], "sweep eta: unknown");
        }
        let eta = SweepEta {
            remaining: Duration::from_secs(300),
            finish: at(300),
        };
        let lines = sweep_lines(SweepWindow::Archive, &running(250, 1000), Some(&eta));
        assert_eq!(lines[1], "sweep eta: 5m (1970-01-01T00:05:00+00:00)");
    }

    #[test]
    fn render_page_escapes_labels() {
        let node = WatchedNode {
            label: "<&\">".to_owned(),
            base_url: Url::parse("http://localhost:1/").unwrap(),
        };
        let page = NodePage {
            node: &node,
            tip_height: None,
            sweep_eta: None,
            progress: None,
            kinds: Vec::new(),
            listed: MissingRanges {
                shown: Vec::new(),
                total: 0,
            },
        };
        let html = render_page(SweepWindow::Archive, &[page]);
        assert!(html.contains("&lt;&amp;&quot;&gt;</a></h2>"), "{html}");
        assert!(!html.contains("<&\">"));
        assert!(
            html.contains("<li>tip: no tip yet</li><li>not started</li><li>missing: none</li>")
        );
    }
}
