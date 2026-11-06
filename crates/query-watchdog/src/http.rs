//! HTTP endpoints: `/healthcheck`, `/status/metrics` and `/`, the page of missing ranges.

use std::{
    fmt::{self, Write},
    sync::Arc,
    time::Duration,
};

use axum::{Json, Router, extract::State, http::StatusCode, response::Html, routing::get};
use chrono::{DateTime, Utc};
use prometheus::{Registry, TextEncoder};
use serde::Serialize;
use tokio::net::TcpListener;

use crate::{
    block_time::{BlockTimes, BlockTimesReceiver},
    kind::Height,
    node::WatchedNode,
    schedule::{RunningSweep, SweepState},
    system::{Clock, WallClock},
    view::{KindSummary, NodeView, NodeViewSlot, SweepEta, TryTimes, age_at},
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
    /// Links, status line (`red: N not-found units` or `green: 0 not-found units`),
    /// `generated at`, then one section per node.
    pub(crate) fn render(&self) -> String {
        let mut page = String::from(
            "<!DOCTYPE html><html><head><title>query-watchdog</title></\
             head><body><h1>query-watchdog</h1><ul><li><a \
             href=\"/status/metrics\">/status/metrics</a></li><li><a \
             href=\"/healthcheck\">/healthcheck</a></li></ul>",
        );
        let total = self.not_found_units();
        let color = if total > 0 { "red" } else { "green" };
        write!(
            page,
            "<p style=\"background:{color}\">{color}: {total} not-found units</p>"
        )
        .expect("writing to a String");
        write!(
            page,
            "<p>generated at {}</p>",
            self.generated_at.to_rfc3339()
        )
        .expect("writing to a String");
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

    /// Heading, `published at`, `not found`, progress lines, missing lines and table of one node;
    /// `None` view: `tip: no tip yet`, `not started`, `missing: none`.
    fn write_node(
        &self,
        out: &mut String,
        node: &WatchedNode,
        view: Option<&NodeView>,
    ) -> fmt::Result {
        write!(
            out,
            "<h2><a href=\"{}\">{}</a></h2><ul>",
            escape(node.base_url.as_str()),
            escape(&node.label)
        )?;
        let Some(view) = view else {
            write_line(out, "tip: no tip yet")?;
            write_line(out, "not started")?;
            write_line(out, "missing: none")?;
            return out.write_str("</ul>");
        };
        let summary = view.summary(view.frontier.tip(), self.generated_at);
        let published = time_text(view.published_at, self.generated_at);
        write_line(out, &format!("published at {published}"))?;
        write_line(
            out,
            &format!("not found: {} units", summary.not_found_units),
        )?;
        for line in self.progress_lines(view, summary.follow_lag_blocks) {
            write_line(out, &line)?;
        }
        for line in missing_lines(&summary.kinds) {
            write_line(out, &line)?;
        }
        out.write_str("</ul>")?;
        self.write_missing_table(out, node, view)
    }

    /// Tip, follow, sweep and last-sweep lines with block times.
    fn progress_lines(&self, view: &NodeView, lag: u64) -> Vec<String> {
        let tip = view.frontier.tip();
        let tip_line = match view.newest_block() {
            Some(newest) => format!("tip: {tip}, newest block {}", self.height_text(newest)),
            None => format!("tip: {tip}"),
        };
        let follow = self.height_text(view.progress.follow_next_height);
        let mut lines = vec![
            tip_line,
            format!("follow next height: {follow} (lag {lag})"),
        ];
        lines.extend(self.sweep_lines(view));
        let completed = view
            .progress
            .sweep
            .last_completed()
            .map_or_else(|| "never".to_owned(), |t| time_text(t, self.generated_at));
        lines.push(format!("last sweep completed: {completed}"));
        lines
    }

    /// Running, waiting or done lines.
    fn sweep_lines(&self, view: &NodeView) -> Vec<String> {
        match &view.progress.sweep {
            SweepState::Running { sweep, .. } => self.running_lines(sweep, view.sweep_eta.as_ref()),
            SweepState::NeverSwept | SweepState::Waiting { .. } => {
                let when = view.next_sweep_start.map_or_else(
                    || "due now".to_owned(),
                    |t| format!("until {}", time_text(t, self.generated_at)),
                );
                vec![format!("sweep: waiting {when}")]
            },
            SweepState::Done { .. } => vec!["sweep: done".to_owned()],
        }
    }

    /// `sweep: running 45.0000%` from `RunningSweep::percent_done`, ETA, next and until heights
    /// with block times.
    fn running_lines(&self, sweep: &RunningSweep, eta: Option<&SweepEta>) -> Vec<String> {
        let eta = eta.map_or_else(|| "unknown".to_owned(), format_eta);
        vec![
            format!("sweep: running {:.4}%", sweep.percent_done()),
            format!("sweep eta: {eta}"),
            format!("sweep next height: {}", self.height_text(sweep.next_height)),
            format!(
                "sweep until height: {}",
                self.height_text(sweep.until_height)
            ),
        ]
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
            let first_block = key
                .first_block(view.epochs)
                .map_or_else(|| "-".to_owned(), |height| self.height_text(height));
            let age =
                humantime::format_duration(age_at(row.state.first_missing_at(), self.generated_at));
            let tries = TryTimes::at(&row.state.retry(), self.generated_at);
            let units = key.units();
            write!(
                out,
                "<tr><td>{kind}</td><td>{}</td><td>{}</td><td>{}</td><td>{age}</td><td>{}</\
                 td><td>{}</td><td><a href=\"{url}\">{url}</a></td></tr>",
                units.start,
                units.end,
                escape(&first_block),
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

    /// `<h> at <time_text>` or `<h> (time unknown)`.
    fn height_text(&self, height: Height) -> String {
        match self.block_times.get(height) {
            Some(time) => format!("{height} at {}", time_text(time, self.generated_at)),
            None => format!("{height} (time unknown)"),
        }
    }
}

/// Appends `text` as a list item.
fn write_line(out: &mut String, text: &str) -> fmt::Result {
    write!(out, "<li>{}</li>", escape(text))
}

/// `<rfc3339> (<age> ago)`, or `<rfc3339> (in <wait>)` for a time after `now`.
fn time_text(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    match (now - at).to_std() {
        Ok(age) => format!(
            "{} ({} ago)",
            at.to_rfc3339(),
            humantime::format_duration(age)
        ),
        Err(_) => {
            let wait = (at - now).to_std().unwrap_or(Duration::ZERO);
            format!(
                "{} (in {})",
                at.to_rfc3339(),
                humantime::format_duration(wait)
            )
        },
    }
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
        kind::{DataKind, Epochs, RangeKind},
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

    /// Times of every height `views` show, one second per height after 1000 s.
    fn times_of(views: &[&NodeView]) -> BlockTimes {
        let mut times = BlockTimes::default();
        for height in views.iter().flat_map(|view| view.shown_block_heights()) {
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
        assert!(html.contains("background:red"), "{html}");

        let html = render(&[(&b, Some(&green)), (&c, None)], at(120));
        assert!(html.contains("green: 0 not-found units"), "{html}");
        assert!(html.contains("background:green"), "{html}");
    }

    #[test]
    fn render_page_escapes_labels() {
        let node = node("<&\">");
        let html = render(&[(&node, None)], at(0));
        assert!(html.contains("&lt;&amp;&quot;&gt;</a></h2>"), "{html}");
        assert!(!html.contains("<&\">"));
        assert!(
            html.contains("<li>tip: no tip yet</li><li>not started</li><li>missing: none</li>")
        );
        assert!(html.contains("green: 0 not-found units"), "{html}");
    }

    #[test]
    fn render_page_shows_the_published_view() {
        let view = listed_view();
        let node = node("a:1");
        let times = times_of(&[&view]);
        let html = render_with(&[(&node, Some(&view))], &times, at(120));
        for expected in [
            "generated at",
            "published at 1970-01-01T00:01:30+00:00 (30s ago)",
            "not found: 0 units",
            "tip: 5000, newest block 4999 at 1970-01-01T01:39:59+00:00 (in 1h 37m 59s)",
            "follow next height: 4990 at ",
            "(lag 10)",
            "sweep: running 25.0000%",
            "sweep next height: 250 at ",
            "sweep until height: 1000 at ",
            "last sweep completed: never",
            "missing leaf: 300 heights",
            "<td>leaf</td><td>1000</td><td>1300</td><td>1000 at ",
            "</td><td>2m</td><td>2m ago</td><td>due</td>",
            "href=\"http://localhost:1/v1/availability/leaf/1000/1100\"",
            "<p>2 more not shown</p>",
        ] {
            assert!(html.contains(expected), "{expected} missing in {html}");
        }
        assert!(!html.contains("time unknown"), "{html}");
    }

    #[test]
    fn the_page_prints_generated_at() {
        let html = render(&[], at(120));
        assert!(
            html.contains("generated at 1970-01-01T00:02:00+00:00"),
            "{html}"
        );
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
    fn every_shown_height_has_its_time() {
        let view = listed_view();
        let node = node("a:1");
        let timed = render_with(&[(&node, Some(&view))], &times_of(&[&view]), at(120));
        assert!(!timed.contains("time unknown"), "{timed}");

        let untimed = render(&[(&node, Some(&view))], at(120));
        let shown = view.shown_block_heights().len();
        assert_eq!(untimed.matches("(time unknown)").count(), shown + 1);
    }

    #[test]
    fn time_text_reads_ago_and_in() {
        let now = at(100);
        assert_eq!(
            time_text(at(70), now),
            "1970-01-01T00:01:10+00:00 (30s ago)"
        );
        assert_eq!(
            time_text(at(100), now),
            "1970-01-01T00:01:40+00:00 (0s ago)"
        );
        assert_eq!(time_text(at(160), now), "1970-01-01T00:02:40+00:00 (in 1m)");
    }

    #[test]
    fn a_view_published_after_the_page_reads_zero_ago() {
        let view = listed_view();
        let node = node("a:1");
        let html = render(&[(&node, Some(&view))], at(60));
        assert!(
            html.contains("published at 1970-01-01T00:01:30+00:00 (in 30s)"),
            "{html}"
        );
    }

    #[test]
    fn sweep_lines_percent() {
        let page_times = BlockTimes::default();
        let page = Page {
            nodes: &[],
            block_times: &page_times,
            generated_at: at(0),
        };
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
            let lines = page.running_lines(&sweep, None);
            assert_eq!(lines[0], format!("sweep: running {percent}%"));
            assert_eq!(lines.len(), 4);
            assert_eq!(lines[1], "sweep eta: unknown");
        }
        let eta = SweepEta {
            remaining: Duration::from_secs(300),
            finish: at(300),
        };
        let lines = page.running_lines(&running(0, 250, 1000), Some(&eta));
        assert_eq!(lines[1], "sweep eta: 5m (1970-01-01T00:05:00+00:00)");
    }

    #[test]
    fn waiting_sweeps_show_their_start() {
        let page_times = BlockTimes::default();
        let page = Page {
            nodes: &[],
            block_times: &page_times,
            generated_at: at(120),
        };
        let mut view = view(SweepState::NeverSwept, no_rows(), Vec::new());
        assert_eq!(page.sweep_lines(&view), ["sweep: waiting due now"]);
        view.next_sweep_start = Some(at(300));
        assert_eq!(
            page.sweep_lines(&view),
            ["sweep: waiting until 1970-01-01T00:05:00+00:00 (in 3m)"]
        );
    }
}
