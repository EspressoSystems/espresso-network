use std::time::Duration;

use chrono::{DateTime, Utc};
use prometheus::{IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry};

use crate::{
    chunk::DataKind,
    store::{NodeProgress, SweepState},
    watch::{FetchError, FetchTarget, NodeSummary},
};

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
    /// Max block height of all watched nodes.
    tip_height: IntGauge,
    /// `{node}`.
    follow_next_height: IntGaugeVec,
    /// `{node}`, 1 while a sweep runs, 0 while it waits.
    sweep_running: IntGaugeVec,
    /// `{node}`, next height of the running sweep; 0 while waiting.
    sweep_next_height: IntGaugeVec,
    /// `{node}`, end height of the running sweep; 0 while waiting.
    sweep_until_height: IntGaugeVec,
    /// `{node}`, estimated time to finish the running sweep; absent while waiting or unknown.
    sweep_eta_seconds: IntGaugeVec,
    /// `{node,kind}`, every probe sent.
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
            fetch_errors_total: counter(
                registry,
                "fetch_errors_total",
                "Requests without a 200",
                &["node", "kind", "status"],
            )?,
            last_sweep_completed_seconds: gauge(
                registry,
                "last_sweep_completed_seconds",
                "Unix time of the last completed sweep",
                &["node"],
            )?,
            tip_height: {
                let gauge = IntGauge::new("query_watchdog_tip_height", "Tip height")?;
                registry.register(Box::new(gauge.clone()))?;
                gauge
            },
            follow_next_height: gauge(
                registry,
                "follow_next_height",
                "Next height follow checks",
                &["node"],
            )?,
            sweep_running: gauge(
                registry,
                "sweep_running",
                "1 while a sweep runs, 0 while it waits",
                &["node"],
            )?,
            sweep_next_height: gauge(
                registry,
                "sweep_next_height",
                "Next height of the running sweep, 0 while waiting",
                &["node"],
            )?,
            sweep_until_height: gauge(
                registry,
                "sweep_until_height",
                "End height of the running sweep, 0 while waiting",
                &["node"],
            )?,
            sweep_eta_seconds: gauge(
                registry,
                "sweep_eta_seconds",
                "Estimated seconds to finish the running sweep, absent while waiting or unknown",
                &["node"],
            )?,
            probes_total: counter(registry, "probes_total", "Probes sent", &["node", "kind"])?,
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

    /// Sets the follow and sweep progress gauges of `node_label`; the sweep height gauges read 0
    /// while the sweep waits, and the ETA gauge is removed while waiting or unknown.
    pub fn export_progress(
        &self,
        node_label: &str,
        progress: &NodeProgress,
        sweep_eta: Option<Duration>,
    ) -> anyhow::Result<()> {
        let (running, next, until) = match &progress.sweep {
            SweepState::Running(sweep) => (1, sweep.next_height, sweep.until_height),
            SweepState::Waiting { .. } | SweepState::Done => (0, 0, 0),
        };
        let labels = [node_label];
        self.follow_next_height
            .with_label_values(&labels)
            .set(i64::try_from(progress.follow_next_height)?);
        self.sweep_running.with_label_values(&labels).set(running);
        match sweep_eta {
            Some(eta) => self
                .sweep_eta_seconds
                .with_label_values(&labels)
                .set(i64::try_from(eta.as_secs())?),
            None => {
                // Fails only when the series does not exist yet.
                self.sweep_eta_seconds.remove_label_values(&labels).ok();
            },
        }
        self.sweep_next_height
            .with_label_values(&labels)
            .set(i64::try_from(next)?);
        self.sweep_until_height
            .with_label_values(&labels)
            .set(i64::try_from(until)?);
        Ok(())
    }

    /// Sets the tip height.
    pub fn set_tip_height(&self, tip_height: u64) -> anyhow::Result<()> {
        self.tip_height.set(i64::try_from(tip_height)?);
        Ok(())
    }

    /// Counts one probe of `kind` under `{node_label, kind}`.
    pub fn count_probe(&self, node_label: &str, kind: DataKind) {
        self.probes_total
            .with_label_values(&[node_label, kind.into()])
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

/// Creates a `query_watchdog_{name}` counter family and registers it.
fn counter(
    registry: &Registry,
    name: &str,
    help: &str,
    labels: &[&str],
) -> prometheus::Result<IntCounterVec> {
    let counter = IntCounterVec::new(Opts::new(format!("query_watchdog_{name}"), help), labels)?;
    registry.register(Box::new(counter.clone()))?;
    Ok(counter)
}
