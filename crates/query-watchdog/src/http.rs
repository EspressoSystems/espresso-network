//! HTTP endpoints: `/healthcheck`, `/status/metrics` and `/`, the page of missing ranges.

use std::{
    fmt::{self, Write},
    sync::Arc,
};

use axum::{Json, Router, extract::State, http::StatusCode, response::Html, routing::get};
use chrono::{DateTime, Utc};
use prometheus::{Registry, TextEncoder};
use serde::Serialize;
use tokio::net::TcpListener;

use crate::{
    block_time::{BlockTimes, BlockTimesReceiver},
    node::WatchedNode,
    schedule::SweepState,
    system::{Clock, WallClock},
    view::{self, KindSummary, NodeView, NodeViewSlot, TryTimes},
};

/// What the HTTP handlers read.
#[derive(Clone)]
pub struct AppState {
    registry: Registry,
    nodes: Arc<[NodeViewSlot]>,
    block_times: BlockTimesReceiver,
    clock: WallClock,
}

impl AppState {
    /// The page lists `nodes` in order and reads `block_times`.
    pub fn new(
        registry: Registry,
        nodes: Vec<NodeViewSlot>,
        block_times: BlockTimesReceiver,
    ) -> Self {
        Self {
            registry,
            nodes: nodes.into(),
            block_times,
            clock: WallClock,
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

/// HTML page from the latest view of every node and the block times; never reads the store.
async fn missing_page(State(app): State<AppState>) -> Html<String> {
    let views: Vec<_> = app.nodes.iter().map(|slot| slot.view.borrow()).collect();
    let nodes: Vec<_> = app
        .nodes
        .iter()
        .zip(&views)
        .map(|(slot, view)| (&slot.node, view.as_ref()))
        .collect();
    let block_times = app.block_times.borrow();
    let page = Page {
        nodes: &nodes,
        block_times: &block_times,
        generated_at: app.clock.now(),
    };
    Html(page.render())
}

/// One render of `/`: the views, the block times and the render time; every age is at
/// `generated_at`.
pub(crate) struct Page<'a> {
    pub nodes: &'a [(&'a WatchedNode, Option<&'a NodeView>)],
    pub block_times: &'a BlockTimes,
    pub generated_at: DateTime<Utc>,
}

impl Page<'_> {
    /// Status line, `generated at`, endpoint links, the overview table, then one section per node.
    pub(crate) fn render(&self) -> String {
        let total = self.not_found_units();
        let (color, background, text) = if total > 0 {
            ("red", "#f8d7da", "#721c24")
        } else {
            ("green", "#d4edda", "#155724")
        };
        let mut page = String::from(
            "<!DOCTYPE html><html><head><title>query-watchdog</title></\
             head><body><h1>query-watchdog</h1>",
        );
        write!(
            page,
            "<p style=\"background:{background};color:{text};padding:0.5em\">{color}: {total} \
             not-found units</p><p>generated at {} UTC</p><ul><li><a \
             href=\"/status/metrics\">/status/metrics</a></li><li><a \
             href=\"/healthcheck\">/healthcheck</a></li></ul>",
            time_text(self.generated_at)
        )
        .expect("writing to a String");
        self.write_overview(&mut page).expect("writing to a String");
        for (node, view) in self.nodes {
            self.write_node(&mut page, node, *view)
                .expect("writing to a String");
        }
        page.push_str("</body></html>");
        page
    }

    /// Sum of `not_found_units` over the published views.
    fn not_found_units(&self) -> u64 {
        self.nodes
            .iter()
            .filter_map(|(_, view)| *view)
            .map(|view| view.not_found_units)
            .sum()
    }

    /// One table with a row per node; full times are in `title` attributes.
    fn write_overview(&self, out: &mut String) -> fmt::Result {
        out.write_str(
            "<table border=\"1\"><tr><th>node</th><th>not found</th><th>published</th><th>newest \
             block</th><th>follow lag</th><th>sweep</th><th>last sweep</th></tr>",
        )?;
        for (node, view) in self.nodes {
            let link = node_link(node);
            match view {
                None => write!(
                    out,
                    "<tr><td>{link}</td><td>-</td><td>-</td><td>-</td><td>-</td><td>not \
                     started</td><td>-</td></tr>"
                )?,
                Some(view) => {
                    let summary = view.summary(view.frontier.tip(), self.generated_at);
                    let newest = view
                        .newest_block()
                        .map_or_else(|| "-".to_owned(), |height| height.to_string());
                    let last_sweep = match view.progress.sweep.last_completed() {
                        Some(done) => self.age_cell(done),
                        None => "<td>never</td>".to_owned(),
                    };
                    write!(
                        out,
                        "<tr><td>{link}</td><td>{}</td>{}<td>{newest}</td><td>{}</\
                         td>{}{last_sweep}</tr>",
                        summary.not_found_units,
                        self.age_cell(view.published_at),
                        summary.follow_lag_blocks,
                        self.sweep_cell(view)
                    )?;
                },
            }
        }
        out.write_str("</table>")
    }

    /// `<td title="<time>"><age></td>`.
    fn age_cell(&self, at: DateTime<Utc>) -> String {
        format!(
            "<td title=\"{}\">{}</td>",
            time_text(at),
            view::short_duration(view::age_at(at, self.generated_at))
        )
    }

    /// `running 45.0% (eta 12m)`, `next in 6d 23h`, `due now` or `done`.
    fn sweep_cell(&self, view: &NodeView) -> String {
        let (text, title) = match &view.progress.sweep {
            SweepState::Running { sweep, .. } => {
                let eta = view.sweep_eta.as_ref();
                let text = match eta {
                    Some(eta) => format!(
                        "running {:.1}% (eta {})",
                        sweep.percent_done(),
                        view::short_duration(eta.remaining)
                    ),
                    None => format!("running {:.1}%", sweep.percent_done()),
                };
                (text, eta.map(|eta| eta.finish))
            },
            SweepState::NeverSwept | SweepState::Waiting { .. } => {
                match view.next_sweep_start.filter(|at| *at > self.generated_at) {
                    Some(at) => (
                        format!(
                            "next in {}",
                            view::short_duration(view::age_at(self.generated_at, at))
                        ),
                        Some(at),
                    ),
                    None => ("due now".to_owned(), None),
                }
            },
            SweepState::Done { .. } => ("done".to_owned(), None),
        };
        match title {
            Some(at) => format!("<td title=\"{}\">{}</td>", time_text(at), escape(&text)),
            None => format!("<td>{}</td>", escape(&text)),
        }
    }

    /// Heading, missing counts and missing table of one node.
    fn write_node(
        &self,
        out: &mut String,
        node: &WatchedNode,
        view: Option<&NodeView>,
    ) -> fmt::Result {
        write!(out, "<h2>{}</h2>", node_link(node))?;
        let Some(view) = view else {
            return out.write_str("<p>nothing missing</p>");
        };
        let summary = view.summary(view.frontier.tip(), self.generated_at);
        if let Some(line) = missing_line(&summary.kinds) {
            write!(out, "<p>{}</p>", escape(&line))?;
        }
        self.write_missing_table(out, node, view)
    }

    /// The listed rows with first block time, missing for, last tried, next try and link.
    fn write_missing_table(
        &self,
        out: &mut String,
        node: &WatchedNode,
        view: &NodeView,
    ) -> fmt::Result {
        let listed = &view.listed;
        if listed.shown.is_empty() {
            return out.write_str("<p>nothing missing</p>");
        }
        out.write_str(
            "<table border=\"1\"><tr><th>kind</th><th>range_start</th><th>range_end</th><th>first \
             block time</th><th>missing for</th><th>last tried</th><th>next \
             try</th><th>link</th></tr>",
        )?;
        for row in &listed.shown {
            let key = &row.key;
            let url = escape(node.route_url(&key.first_probe().route()).as_str());
            let kind: &str = key.kind().into();
            let first_block = match key.first_block(view.epochs) {
                None => "-".to_owned(),
                Some(height) => self.block_times.get(height).map_or_else(
                    || "time unknown".to_owned(),
                    |time| {
                        let age = view::age_at(time, self.generated_at);
                        format!("{} ({} ago)", time_text(time), view::short_duration(age))
                    },
                ),
            };
            let age = view::short_duration(view::age_at(
                row.state.first_missing_at(),
                self.generated_at,
            ));
            let tries = TryTimes::at(&row.state.retry(), self.generated_at);
            let units = key.units();
            write!(
                out,
                "<tr><td>{kind}</td><td>{}</td><td>{}</td><td>{first_block}</td><td>{age}</\
                 td><td>{}</td><td>{}</td><td><a href=\"{url}\">{url}</a></td></tr>",
                units.start,
                units.end,
                tries.last_tried_text(),
                tries.next_try_text()
            )?;
        }
        out.write_str("</table>")?;
        let hidden = listed.total - listed.shown.len() as u64;
        if hidden > 0 {
            write!(out, "<p>{hidden} more not shown</p>")?;
        }
        Ok(())
    }
}

/// Link to the node's base URL labelled with its label.
fn node_link(node: &WatchedNode) -> String {
    format!(
        "<a href=\"{}\">{}</a>",
        escape(node.base_url.as_str()),
        escape(&node.label)
    )
}

/// `2026-11-06 14:56:22`, UTC.
fn time_text(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// `missing: leaf 300 heights, block 2 heights`, or `None` when nothing is missing.
fn missing_line(kinds: &[KindSummary]) -> Option<String> {
    let parts: Vec<_> = kinds
        .iter()
        .filter(|kind| kind.missing_count > 0)
        .map(|kind| {
            format!(
                "{} {} {}",
                <&str>::from(kind.kind),
                kind.missing_count,
                kind.kind.unit_name()
            )
        })
        .collect();
    (!parts.is_empty()).then(|| format!("missing: {}", parts.join(", ")))
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

    use url::Url;

    use super::*;
    use crate::{
        kind::{DataKind, Epochs, Height, RangeKind},
        missing::{KindTally, ListedRows, MissingRow, RowKey},
        schedule::{CompletedSweep, Frontier, NodeProgress, RunningSweep, SweepWindow},
        state::{Retry, RetryDelay, UnitState},
        view::{SweepEta, WatchCounters},
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
            epochs: Epochs::Off,
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

    /// Times of the first blocks of `views`' rows, one second per height after 1000 s.
    fn times_of(views: &[&NodeView]) -> BlockTimes {
        let mut times = BlockTimes::default();
        for height in views.iter().flat_map(|view| view.missing_row_blocks()) {
            times.insert(height, at(1_000 + height.0 as i64));
        }
        times
    }

    /// Renders at `generated_at` with no block times.
    fn render(nodes: &[(&WatchedNode, Option<&NodeView>)], generated_at: DateTime<Utc>) -> String {
        render_with(nodes, &BlockTimes::default(), generated_at)
    }

    fn render_with(
        nodes: &[(&WatchedNode, Option<&NodeView>)],
        block_times: &BlockTimes,
        generated_at: DateTime<Utc>,
    ) -> String {
        Page {
            nodes,
            block_times,
            generated_at,
        }
        .render()
    }

    fn listed_view() -> NodeView {
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
        view(
            SweepState::Running {
                sweep: running(0, 250, 1000),
                last_completed_at: None,
            },
            listed,
            vec![tally],
        )
    }

    #[test]
    fn the_status_line_is_red_with_the_total_and_green_at_zero() {
        let (a, b, c) = (node("a"), node("b"), node("c"));
        let mut red = view(SweepState::NeverSwept, no_rows(), Vec::new());
        red.not_found_units = 15;
        let green = view(SweepState::NeverSwept, no_rows(), Vec::new());

        let html = render(&[(&a, Some(&red)), (&b, Some(&green)), (&c, None)], at(120));
        assert!(html.contains("red: 15 not-found units"), "{html}");
        assert!(html.contains("background:#f8d7da;color:#721c24"), "{html}");

        let html = render(&[(&b, Some(&green)), (&c, None)], at(120));
        assert!(html.contains("green: 0 not-found units"), "{html}");
        assert!(html.contains("background:#d4edda;color:#155724"), "{html}");
    }

    #[test]
    fn the_page_prints_generated_at_in_utc() {
        let html = render(&[], at(120));
        assert!(
            html.contains("generated at 1970-01-01 00:02:00 UTC"),
            "{html}"
        );
    }

    #[test]
    fn render_page_escapes_labels_and_shows_a_node_without_view() {
        let node = node("<&\">");
        let html = render(&[(&node, None)], at(0));
        assert!(html.contains("&lt;&amp;&quot;&gt;</a></h2>"), "{html}");
        assert!(!html.contains("<&\">"));
        assert!(
            html.contains(
                "<td>-</td><td>-</td><td>-</td><td>-</td><td>not started</td><td>-</td></tr>"
            ),
            "{html}"
        );
        assert!(html.contains("<p>nothing missing</p>"), "{html}");
        assert!(html.contains("green: 0 not-found units"), "{html}");
    }

    #[test]
    fn the_overview_row_has_ages_with_times_in_titles() {
        let mut view = listed_view();
        view.sweep_eta = Some(SweepEta {
            remaining: Duration::from_secs(725),
            finish: at(845),
        });
        let node = node("a:1");
        let html = render(&[(&node, Some(&view))], at(120));
        let row = "<tr><td><a href=\"http://localhost:1/\">a:1</a></td><td>0</td><td \
                   title=\"1970-01-01 00:01:30\">30s</td><td>4999</td><td>10</td>";
        assert!(html.contains(row), "{html}");
        let sweep =
            "<td title=\"1970-01-01 00:14:05\">running 25.0% (eta 12m 5s)</td><td>never</td></tr>";
        assert!(html.contains(sweep), "{html}");
    }

    #[test]
    fn sweep_cells() {
        let node = node("a:1");
        let mut view = view(SweepState::NeverSwept, no_rows(), Vec::new());
        let cell = |view: &NodeView| render(&[(&node, Some(view))], at(120));
        assert!(cell(&view).contains("<td>due now</td>"));
        view.next_sweep_start = Some(at(120 + 6 * 86_400 + 23 * 3_600 + 58 * 60));
        assert!(cell(&view).contains("<td title=\"1970-01-08 00:00:00\">next in 6d 23h</td>"));
        view.next_sweep_start = Some(at(100));
        assert!(cell(&view).contains("<td>due now</td>"));
        view.progress.sweep = SweepState::Done {
            last: CompletedSweep {
                started_at: at(0),
                completed_at: at(60),
            },
        };
        let html = cell(&view);
        assert!(html.contains("<td>done</td><td title=\"1970-01-01 00:01:00\">1m</td>"));
    }

    #[test]
    fn the_node_section_has_missing_counts_and_the_table() {
        let view = listed_view();
        let node = node("a:1");
        let times = times_of(&[&view]);
        let html = render_with(&[(&node, Some(&view))], &times, at(120));
        for expected in [
            "<h2><a href=\"http://localhost:1/\">a:1</a></h2>",
            "<p>missing: leaf 300 heights</p>",
            "<td>leaf</td><td>1000</td><td>1300</td><td>1970-01-01 00:33:20 (0s ago)</td>",
            "</td><td>2m</td><td>2m ago</td><td>due</td>",
            "href=\"http://localhost:1/v1/availability/leaf/1000/1100\"",
            "<p>2 more not shown</p>",
        ] {
            assert!(html.contains(expected), "{expected} missing in {html}");
        }
        assert!(!html.contains("tip:"), "{html}");
        assert!(!html.contains("time unknown"), "{html}");
    }

    #[test]
    fn row_ages_and_try_times_are_at_generated_at() {
        let view = listed_view();
        let node = node("a:1");
        let at_publish = render(&[(&node, Some(&view))], at(90));
        assert!(at_publish.contains("<td>1m 30s</td><td>1m 30s ago</td><td>in 30s</td>"));
        let later = render(&[(&node, Some(&view))], at(120));
        assert!(
            later.contains("<td>2m</td><td>2m ago</td><td>due</td>"),
            "{later}"
        );
    }

    #[test]
    fn a_row_without_block_time_says_so() {
        let view = listed_view();
        let node = node("a:1");
        let html = render(&[(&node, Some(&view))], at(120));
        assert!(
            html.contains("<td>1300</td><td>time unknown</td>"),
            "{html}"
        );
    }

    #[test]
    fn time_text_is_utc_without_offset() {
        assert_eq!(time_text(at(1_793_977_000)), "2026-11-06 14:56:40");
    }
}
