//! HTTP endpoints: `/healthcheck`, `/status/metrics` and `/`, the page of missing ranges.

use std::{fmt::Write, sync::Arc};

use axum::{Json, Router, extract::State, http::StatusCode, response::Html, routing::get};
use prometheus::{Registry, TextEncoder};
use serde::Serialize;
use tokio::net::TcpListener;

use crate::{
    node::WatchedNode,
    schedule::{RunningSweep, SweepState},
    view::{KindSummary, NodeView, NodeViewSlot, SweepEta, TryTimes, age_at},
};

/// What the HTTP handlers read.
#[derive(Clone)]
pub struct AppState {
    registry: Registry,
    nodes: Arc<[NodeViewSlot]>,
}

impl AppState {
    /// The page lists `nodes` in order.
    pub fn new(registry: Registry, nodes: Vec<NodeViewSlot>) -> Self {
        Self {
            registry,
            nodes: nodes.into(),
        }
    }
}

/// `axum::serve` of a `Router` with `/healthcheck` (`{"status":"available"}`, 200),
/// `/status/metrics` (`TextEncoder::encode_to_string(&registry.gather())`, 500 on encode error)
/// and `/`, the page of missing ranges per node.
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
            tracing::warn!(%error, "encoding metrics");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// HTML page from the latest view of every node; never reads the store.
async fn missing_page(State(app): State<AppState>) -> Html<String> {
    let views: Vec<_> = app.nodes.iter().map(|slot| slot.view.borrow()).collect();
    let nodes: Vec<_> = app
        .nodes
        .iter()
        .zip(&views)
        .map(|(slot, view)| (&slot.node, view.as_ref()))
        .collect();
    Html(render_page(&nodes))
}

/// Page with the endpoint links, the status line, then one section per node.
pub(crate) fn render_page(nodes: &[(&WatchedNode, Option<&NodeView>)]) -> String {
    let mut page = String::from(
        "<!DOCTYPE html><html><head><title>query-watchdog</title></head><body><h1>query-watchdog</\
         h1><ul><li><a href=\"/status/metrics\">/status/metrics</a></li><li><a \
         href=\"/healthcheck\">/healthcheck</a></li></ul>",
    );
    let total: u64 = nodes
        .iter()
        .filter_map(|(_, view)| *view)
        .map(|view| view.not_found_units)
        .sum();
    let color = if total > 0 { "red" } else { "green" };
    write!(
        page,
        "<p style=\"background:{color}\">{color}: {total} not-found units</p>"
    )
    .expect("writing to a String");
    for (node, view) in nodes {
        write_node(&mut page, node, *view).expect("writing to a String");
    }
    page.push_str("</body></html>");
    page
}

/// Heading, progress lines and missing table of one node; `None` view: `tip: no tip yet`,
/// `not started`, `missing: none`.
fn write_node(page: &mut String, node: &WatchedNode, view: Option<&NodeView>) -> std::fmt::Result {
    write!(
        page,
        "<h2><a href=\"{}\">{}</a></h2><ul>",
        escape(node.base_url.as_str()),
        escape(&node.label)
    )?;
    let Some(view) = view else {
        write_line(page, "tip: no tip yet")?;
        write_line(page, "not started")?;
        write_line(page, "missing: none")?;
        return page.write_str("</ul>");
    };
    let summary = view.summary(view.frontier.tip(), view.published_at);
    write_line(page, &format!("tip: {}", summary.tip_height))?;
    for line in progress_lines(view, summary.follow_lag_blocks) {
        write_line(page, &line)?;
    }
    for line in missing_lines(&summary.kinds) {
        write_line(page, &line)?;
    }
    page.write_str("</ul>")?;
    write_missing_table(page, node, view)
}

/// Appends `text` as a list item.
fn write_line(page: &mut String, text: &str) -> std::fmt::Result {
    write!(page, "<li>{}</li>", escape(text))
}

/// Follow, sweep and last-sweep lines.
fn progress_lines(view: &NodeView, lag: u64) -> Vec<String> {
    let follow = view.progress.follow_next_height;
    let mut lines = vec![format!("follow next height: {follow} (lag {lag})")];
    lines.extend(sweep_lines(view));
    let completed = view
        .progress
        .sweep
        .last_completed()
        .map_or_else(|| "never".to_owned(), |t| t.to_rfc3339());
    lines.push(format!("last sweep completed: {completed}"));
    lines
}

/// Running lines, `sweep: waiting until <time>` (`due now` when `NeverSwept`), or `sweep: done`;
/// reads `progress.sweep`, `next_sweep_start` and `sweep_eta`.
fn sweep_lines(view: &NodeView) -> Vec<String> {
    match &view.progress.sweep {
        SweepState::Running { sweep, .. } => running_lines(sweep, view.sweep_eta.as_ref()),
        SweepState::NeverSwept | SweepState::Waiting { .. } => {
            let when = view.next_sweep_start.map_or_else(
                || "due now".to_owned(),
                |t| format!("until {}", t.to_rfc3339()),
            );
            vec![format!("sweep: waiting {when}")]
        },
        SweepState::Done { .. } => vec!["sweep: done".to_owned()],
    }
}

/// `sweep: running 45.0000%` from `RunningSweep::percent_done`, ETA, next and until heights.
fn running_lines(sweep: &RunningSweep, eta: Option<&SweepEta>) -> Vec<String> {
    let eta = eta.map_or_else(|| "unknown".to_owned(), format_eta);
    vec![
        format!("sweep: running {:.4}%", sweep.percent_done()),
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
            format!(
                "missing {}: {} {}",
                <&str>::from(kind.kind),
                kind.missing_count,
                kind.kind.unit_name()
            )
        })
        .collect();
    if lines.is_empty() {
        vec!["missing: none".to_owned()]
    } else {
        lines
    }
}

/// The listed rows with links; ages and try times at the view's `published_at`.
fn write_missing_table(page: &mut String, node: &WatchedNode, view: &NodeView) -> std::fmt::Result {
    let listed = &view.listed;
    if listed.shown.is_empty() {
        return page.write_str("<p>nothing missing</p>");
    }
    page.write_str(
        "<table border=\"1\"><tr><th>kind</th><th>range_start</th><th>range_end</th><th>age</\
         th><th>last tried</th><th>next try</th><th>link</th></tr>",
    )?;
    for row in &listed.shown {
        let key = &row.key;
        let url = escape(node.route_url(&key.first_probe().route()).as_str());
        let kind: &str = key.kind().into();
        let age =
            humantime::format_duration(age_at(row.state.first_missing_at(), view.published_at));
        let tries = TryTimes::at(&row.state.retry(), view.published_at);
        let units = key.units();
        write!(
            page,
            "<tr><td>{kind}</td><td>{}</td><td>{}</td><td>{age}</td><td>{}</td><td>{}</td><td><a \
             href=\"{url}\">{url}</a></td></tr>",
            units.start,
            units.end,
            tries.last_tried_text(),
            tries.next_try_text()
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
    use std::{num::NonZeroU32, time::Duration};

    use chrono::{DateTime, Utc};
    use url::Url;

    use super::*;
    use crate::{
        kind::{DataKind, Height, RangeKind},
        missing::{KindTally, ListedRows, MissingRow, RowKey},
        schedule::{Frontier, NodeProgress, RunningSweep, SweepWindow},
        state::{Retry, RetryDelay, UnitState},
        view::WatchCounters,
    };

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).unwrap()
    }

    fn running(start: u64, next: u64, until: u64) -> RunningSweep {
        RunningSweep {
            started_at: at(0),
            start_height: Height(start),
            next_height: Height(next),
            until_height: Height(until),
        }
    }

    fn node(label: &str) -> WatchedNode {
        WatchedNode {
            label: label.to_owned(),
            base_url: Url::parse("http://localhost:1/").unwrap(),
        }
    }

    fn view(sweep: SweepState, listed: ListedRows, tallies: Vec<KindTally>) -> NodeView {
        NodeView {
            published_at: at(90),
            frontier: Frontier::new(Height(5_000), SweepWindow::Archive),
            progress: NodeProgress {
                follow_next_height: Height(4_990),
                sweep,
            },
            merklized_state_height: None,
            not_found_units: 0,
            tallies,
            listed,
            counters: WatchCounters::default(),
            next_sweep_start: None,
            sweep_eta: None,
        }
    }

    fn no_rows() -> ListedRows {
        ListedRows {
            shown: Vec::new(),
            total: 0,
        }
    }

    #[test]
    fn the_status_line_is_red_with_the_total_and_green_at_zero() {
        let (a, b, c) = (node("a"), node("b"), node("c"));
        let mut red = view(SweepState::NeverSwept, no_rows(), Vec::new());
        red.not_found_units = 15;
        let green = view(SweepState::NeverSwept, no_rows(), Vec::new());

        let html = render_page(&[(&a, Some(&red)), (&b, Some(&green)), (&c, None)]);
        assert!(html.contains("red: 15 not-found units"), "{html}");
        assert!(html.contains("background:red"), "{html}");

        let html = render_page(&[(&b, Some(&green)), (&c, None)]);
        assert!(html.contains("green: 0 not-found units"), "{html}");
        assert!(html.contains("background:green"), "{html}");
    }

    #[test]
    fn render_page_escapes_labels() {
        let node = node("<&\">");
        let html = render_page(&[(&node, None)]);
        assert!(html.contains("&lt;&amp;&quot;&gt;</a></h2>"), "{html}");
        assert!(!html.contains("<&\">"));
        assert!(
            html.contains("<li>tip: no tip yet</li><li>not started</li><li>missing: none</li>")
        );
    }

    #[test]
    fn render_page_shows_the_published_view() {
        let state = UnitState::NotFound {
            first_missing_at: at(0),
            not_found_probes: NonZeroU32::new(3).unwrap(),
            retry: Retry::after(RetryDelay::from_secs(120).unwrap(), at(0)),
        };
        let key = RowKey::Range {
            kind: RangeKind::Leaf,
            heights: Height(1_000)..Height(1_300),
        };
        let row = MissingRow { key, state };
        let listed = ListedRows {
            shown: vec![row.clone()],
            total: 3,
        };
        let tally = KindTally {
            kind: DataKind::Leaf,
            missing_count: 300,
            oldest: Some(row),
        };
        let view = view(
            SweepState::Running {
                sweep: running(0, 250, 1000),
                last_completed_at: None,
            },
            listed,
            vec![tally],
        );
        let node = node("a:1");
        let html = render_page(&[(&node, Some(&view))]);
        for expected in [
            "<li>tip: 5000</li>",
            "<li>follow next height: 4990 (lag 10)</li>",
            "<li>sweep: running 25.0000%</li>",
            "<li>last sweep completed: never</li>",
            "<li>missing leaf: 300 heights</li>",
            "<td>leaf</td><td>1000</td><td>1300</td><td>1m 30s</td><td>1m 30s ago</td><td>in \
             30s</td>",
            "href=\"http://localhost:1/v1/availability/leaf/1000/1100\"",
            "<p>2 more not shown</p>",
        ] {
            assert!(html.contains(expected), "{expected} missing in {html}");
        }
    }

    #[test]
    fn sweep_lines_percent() {
        let cases = [
            (running(0, 250, 1000), "25.0000"),
            (running(500, 745, 990), "50.0000"),
            (running(500, 892, 990), "80.0000"),
            (running(0, 1, 3), "33.3333"),
            // Window start rounds up past the tip: empty window.
            (running(1050, 1050, 1050), "100.0000"),
            // `next` beyond `until` caps at 100%.
            (running(0, 1200, 1000), "100.0000"),
        ];
        for (sweep, percent) in cases {
            let lines = running_lines(&sweep, None);
            assert_eq!(lines[0], format!("sweep: running {percent}%"));
            assert_eq!(lines.len(), 4);
            assert_eq!(lines[1], "sweep eta: unknown");
        }
        let eta = SweepEta {
            remaining: Duration::from_secs(300),
            finish: at(300),
        };
        let lines = running_lines(&running(0, 250, 1000), Some(&eta));
        assert_eq!(lines[1], "sweep eta: 5m (1970-01-01T00:05:00+00:00)");
    }

    #[test]
    fn waiting_sweeps_show_their_start() {
        let mut view = view(SweepState::NeverSwept, no_rows(), Vec::new());
        assert_eq!(sweep_lines(&view), ["sweep: waiting due now"]);
        view.next_sweep_start = Some(at(300));
        assert_eq!(
            sweep_lines(&view),
            ["sweep: waiting until 1970-01-01T00:05:00+00:00"]
        );
    }
}
