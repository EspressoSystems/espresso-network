use chrono::{DateTime, Utc};
use prometheus::{IntCounterVec, IntGaugeVec, Opts, Registry};
use strum::IntoStaticStr;

use crate::{
    chunk::DataKind,
    store::{NodeProgress, SweepState},
    watch::{FetchError, FetchTarget, NodeSummary},
};

/// Outcome of one probe, the `result` label of `probes_total`.
#[derive(Clone, Copy, Debug, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ProbeResult {
    Found,
    Missing,
}

/// `query_watchdog_*` families on the process `Registry`.
pub struct WatchdogMetrics {
    /// `{node,kind}`, heights or epochs per kind.
    missing: IntGaugeVec,
    /// `{node,kind}`.
    oldest_missing_age_seconds: IntGaugeVec,
    /// `{node}`.
    follow_lag_blocks: IntGaugeVec,
    /// `{node}`.
    merklized_state_lag_blocks: IntGaugeVec,
    /// `{node,kind,status}`.
    fetch_errors_total: IntCounterVec,
    /// `{node}`, unix seconds.
    last_sweep_completed_seconds: IntGaugeVec,
    /// `{node}`.
    tip_height: IntGaugeVec,
    /// `{node}`.
    follow_next_height: IntGaugeVec,
    /// `{node}`, 0 while the sweep is waiting.
    sweep_next_height: IntGaugeVec,
    /// `{node}`, 0 while the sweep is waiting.
    sweep_until_height: IntGaugeVec,
    /// `{node,kind,result}`, every probe.
    probes_total: IntCounterVec,
}

impl WatchdogMetrics {
    /// Creates (`IntGaugeVec::new(Opts, &labels)`) and registers (`registry.register(Box::new(..))`)
    /// the families on `registry`.
    pub fn new(registry: &Registry) -> prometheus::Result<Self> {
        let node_kind = ["node", "kind"];
        Ok(Self {
            missing: gauge(registry, "missing", "Missing heights or epochs", &node_kind)?,
            oldest_missing_age_seconds: gauge(
                registry,
                "oldest_missing_age_seconds",
                "Age of the oldest missing chunk",
                &node_kind,
            )?,
            follow_lag_blocks: gauge(
                registry,
                "follow_lag_blocks",
                "Tip height minus the next height follow checks",
                &["node"],
            )?,
            merklized_state_lag_blocks: gauge(
                registry,
                "merklized_state_lag_blocks",
                "Tip height minus the node's merklized state height",
                &["node"],
            )?,
            fetch_errors_total: {
                let opts = Opts::new(
                    "query_watchdog_fetch_errors_total",
                    "Requests without a 200",
                );
                let counter = IntCounterVec::new(opts, &["node", "kind", "status"])?;
                registry.register(Box::new(counter.clone()))?;
                counter
            },
            last_sweep_completed_seconds: gauge(
                registry,
                "last_sweep_completed_seconds",
                "Unix time of the last completed sweep",
                &["node"],
            )?,
            tip_height: gauge(registry, "tip_height", "Tip height", &["node"])?,
            follow_next_height: gauge(
                registry,
                "follow_next_height",
                "Next height follow checks",
                &["node"],
            )?,
            sweep_next_height: gauge(
                registry,
                "sweep_next_height",
                "Next height the running sweep checks, 0 while waiting",
                &["node"],
            )?,
            sweep_until_height: gauge(
                registry,
                "sweep_until_height",
                "End of the running sweep, 0 while waiting",
                &["node"],
            )?,
            probes_total: {
                let opts = Opts::new("query_watchdog_probes_total", "Probes sent");
                let counter = IntCounterVec::new(opts, &["node", "kind", "result"])?;
                registry.register(Box::new(counter.clone()))?;
                counter
            },
        })
    }

    /// Sets the gauges of `node_label` from `summary`; `None` age exports 0.
    pub fn export_summary(&self, node_label: &str, summary: &NodeSummary) -> anyhow::Result<()> {
        self.follow_lag_blocks
            .with_label_values(&[node_label])
            .set(i64::try_from(summary.follow_lag_blocks)?);
        if let Some(lag) = summary.merklized_state_lag_blocks {
            self.merklized_state_lag_blocks
                .with_label_values(&[node_label])
                .set(i64::try_from(lag)?);
        }
        for kind in &summary.kinds {
            let labels = [node_label, kind.kind.into()];
            self.missing
                .with_label_values(&labels)
                .set(i64::try_from(kind.missing_count)?);
            let age = kind
                .oldest
                .as_ref()
                .map_or(Ok(0), |oldest| i64::try_from(oldest.age.as_secs()))?;
            self.oldest_missing_age_seconds
                .with_label_values(&labels)
                .set(age);
        }
        Ok(())
    }

    /// Sets the tip, follow and sweep progress gauges of `node_label`.
    pub fn export_progress(
        &self,
        node_label: &str,
        tip_height: u64,
        progress: &NodeProgress,
    ) -> anyhow::Result<()> {
        let (next, until) = match &progress.sweep {
            SweepState::Running(sweep) => (sweep.next_height, sweep.until_height),
            SweepState::Waiting { .. } => (0, 0),
        };
        let labels = [node_label];
        self.tip_height
            .with_label_values(&labels)
            .set(i64::try_from(tip_height)?);
        self.follow_next_height
            .with_label_values(&labels)
            .set(i64::try_from(progress.follow_next_height)?);
        self.sweep_next_height
            .with_label_values(&labels)
            .set(i64::try_from(next)?);
        self.sweep_until_height
            .with_label_values(&labels)
            .set(i64::try_from(until)?);
        Ok(())
    }

    /// The last exported tip height of `node_label`.
    pub fn tip_height(&self, node_label: &str) -> i64 {
        self.tip_height.with_label_values(&[node_label]).get()
    }

    /// Counts one probe of `kind` under `{node_label, kind, result}`.
    pub fn count_probe(&self, node_label: &str, kind: DataKind, result: ProbeResult) {
        self.probes_total
            .with_label_values(&[node_label, kind.into(), result.into()])
            .inc();
    }

    /// Counts one failure under `{node_label, target.label(), error.status_label()}`.
    pub fn count_fetch_error(&self, node_label: &str, target: FetchTarget, error: &FetchError) {
        self.fetch_errors_total
            .with_label_values(&[node_label, target.label(), error.status_label()])
            .inc();
    }

    /// Sets the unix time of the node's last completed sweep.
    pub fn mark_sweep_completed(&self, node_label: &str, completed_at: DateTime<Utc>) {
        self.last_sweep_completed_seconds
            .with_label_values(&[node_label])
            .set(completed_at.timestamp());
    }
}

/// Creates a `query_watchdog_{name}` gauge family and registers it.
fn gauge(
    registry: &Registry,
    name: &str,
    help: &str,
    labels: &[&str],
) -> prometheus::Result<IntGaugeVec> {
    let gauge = IntGaugeVec::new(Opts::new(format!("query_watchdog_{name}"), help), labels)?;
    registry.register(Box::new(gauge.clone()))?;
    Ok(gauge)
}
