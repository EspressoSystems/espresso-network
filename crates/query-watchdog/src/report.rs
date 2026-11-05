//! The reporter's view of a node: gauges every tick, the `summary` line every few ticks.

use std::time::Duration;

use anyhow::Context;
use chrono::{DateTime, Utc};

use crate::{
    kind::Height,
    node::WatchedNode,
    schedule::NodeProgress,
    view::{NodeSummary, NodeViewSlot, SweepEta, WatchCounters},
};

/// Time between gauge writes; the Datadog OpenMetrics default scrape interval.
pub const GAUGE_INTERVAL: Duration = Duration::from_secs(15);
/// Gauge ticks per `summary` line: one line per 60 s.
pub const TICKS_PER_SUMMARY: u64 = 4;

/// One node as the reporter sees it: its view and the counters of its last summary.
pub struct ReportedNode {
    slot: NodeViewSlot,
    last_logged: WatchCounters,
}

impl ReportedNode {
    /// Nothing logged yet.
    pub fn new(slot: NodeViewSlot) -> Self {
        Self {
            slot,
            last_logged: WatchCounters::default(),
        }
    }

    /// The latest view at `now` against the live `tip`. Errors when the node has published no
    /// view. The view's `borrow()` ends before it returns, so logging never holds the channel lock
    /// a publishing node watch waits on.
    pub fn report(&self, tip: Height, now: DateTime<Utc>) -> anyhow::Result<NodeReport> {
        let guard = self.slot.view.borrow();
        let view = guard
            .as_ref()
            .with_context(|| format!("{} has published no view", self.slot.node.label))?;
        Ok(NodeReport {
            node: self.slot.node.clone(),
            summary: view.summary(tip, now),
            counters: view.counters.clone(),
            progress: view.progress,
            sweep_eta: view.sweep_eta,
        })
    }

    /// The `summary` INFO line (HEAD's fields plus `confirmed_missing`) and `node unhealthy` when
    /// `errors.failures()` since the previous call is not empty; `errors` and `forgotten` since the
    /// previous call.
    pub fn log(&mut self, report: &NodeReport) {
        let node = &report.node.label;
        let since = report.counters.since(&self.last_logged);
        tracing::info!(
            %node,
            errors = ?since.errors,
            forgotten = since.forgotten_rows,
            confirmed_missing = report.summary.confirmed_missing(),
            sweep_eta = report
                .sweep_eta
                .map(|eta| humantime::format_duration(eta.remaining).to_string()),
            summary = %report.summary,
            "summary"
        );
        let failures = since.errors.failures();
        if !failures.is_empty() {
            tracing::warn!(%node, errors = ?failures, "node unhealthy");
        }
        self.last_logged = report.counters.clone();
    }
}

/// One node's state at one tick.
#[derive(Clone, Debug)]
pub struct NodeReport {
    pub node: WatchedNode,
    pub summary: NodeSummary,
    /// Cumulative, from the view.
    pub counters: WatchCounters,
    pub progress: NodeProgress,
    pub sweep_eta: Option<SweepEta>,
}
