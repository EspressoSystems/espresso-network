use std::sync::atomic::{AtomicU64, Ordering};

use hotshot_types::{
    data::EpochNumber,
    traits::metrics::{Counter, Gauge, Histogram, Metrics, NoMetrics},
};

/// Metrics reported by a [`LightClient`](crate::LightClient).
///
/// Create one per registry, since registering a metric name twice panics.
#[derive(Debug)]
pub struct LightClientMetrics {
    pub(crate) latest_stake_table_epoch: Box<dyn Gauge>,
    pub(crate) stake_table_backlog: Box<dyn Gauge>,
    pub(crate) stake_table_catchup_duration: Box<dyn Histogram>,
    pub(crate) block_height: Box<dyn Gauge>,
    pub(crate) verification_failures: Box<dyn Counter>,
    // Older epochs are verified too, for instance when backfilling headers, and must not move
    // the gauge back.
    max_stake_table_epoch: AtomicU64,
}

impl LightClientMetrics {
    pub fn new(metrics: &(impl Metrics + ?Sized)) -> Self {
        let metrics = metrics.subgroup("light_client".into());
        Self {
            latest_stake_table_epoch: metrics.create_gauge("latest_stake_table_epoch".into(), None),
            stake_table_backlog: metrics
                .create_gauge("stake_table_backlog".into(), Some("epochs".into())),
            stake_table_catchup_duration: metrics.create_histogram(
                "stake_table_catchup_duration".into(),
                Some("seconds".into()),
            ),
            block_height: metrics.create_gauge("block_height".into(), None),
            verification_failures: metrics.create_counter("verification_failures".into(), None),
            max_stake_table_epoch: AtomicU64::new(0),
        }
    }

    pub(crate) fn stake_table_verified(&self, epoch: EpochNumber) {
        let latest = self
            .max_stake_table_epoch
            .fetch_max(*epoch, Ordering::Relaxed)
            .max(*epoch);
        self.latest_stake_table_epoch.set(latest as usize);
    }
}

impl Default for LightClientMetrics {
    fn default() -> Self {
        Self::new(&NoMetrics)
    }
}
