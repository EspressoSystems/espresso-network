use chrono::{DateTime, Utc};
use prometheus::{IntCounterVec, IntGaugeVec, Opts, Registry};

use crate::watch::{FetchError, FetchTarget, NodeSummary};

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
}

impl WatchdogMetrics {
    /// Creates (`IntGaugeVec::new(Opts, &labels)`) and registers (`registry.register(Box::new(..))`)
    /// the six families on `registry`.
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
