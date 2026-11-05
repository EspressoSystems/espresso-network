use std::{
    future::Future,
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(feature = "client")]
use hotshot_types::traits::metrics::Histogram;
use hotshot_types::{
    data::EpochNumber,
    traits::metrics::{Counter, Gauge, Metrics, NoMetrics},
};

/// Metrics reported by a [`LightClient`](crate::LightClient).
///
/// Create one per registry, since registering a metric name twice panics.
#[derive(Debug)]
pub struct LightClientMetrics {
    latest_stake_table_epoch: Box<dyn Gauge>,
    stake_table_backlog: Box<dyn Gauge>,
    #[cfg(feature = "client")]
    pub(crate) stake_table_catchup_duration: Box<dyn Histogram>,
    pub(crate) follower_block_height: Box<dyn Gauge>,
    pub(crate) verification_failures: Box<dyn Counter>,
    #[cfg(feature = "client")]
    pub(crate) fetch_failures: Box<dyn Counter>,
    #[cfg(feature = "client")]
    db_read_duration: Box<dyn Histogram>,
    #[cfg(feature = "client")]
    db_write_duration: Box<dyn Histogram>,
    // Older epochs are verified too, for instance when backfilling headers, and must not move
    // the gauge back.
    max_stake_table_epoch: AtomicU64,
}

impl LightClientMetrics {
    pub fn new(metrics: &dyn Metrics) -> Self {
        let metrics = metrics.subgroup("light_client".into());
        Self {
            latest_stake_table_epoch: metrics.create_gauge("latest_stake_table_epoch".into(), None),
            stake_table_backlog: metrics
                .create_gauge("stake_table_backlog".into(), Some("epochs".into())),
            #[cfg(feature = "client")]
            stake_table_catchup_duration: metrics.create_histogram(
                "stake_table_catchup_duration".into(),
                Some("seconds".into()),
            ),
            follower_block_height: metrics.create_gauge("follower_block_height".into(), None),
            verification_failures: metrics.create_counter("verification_failures".into(), None),
            #[cfg(feature = "client")]
            fetch_failures: metrics.create_counter("fetch_failures".into(), None),
            #[cfg(feature = "client")]
            db_read_duration: metrics
                .create_histogram("db_read_duration".into(), Some("seconds".into())),
            #[cfg(feature = "client")]
            db_write_duration: metrics
                .create_histogram("db_write_duration".into(), Some("seconds".into())),
            max_stake_table_epoch: AtomicU64::new(0),
        }
    }

    pub(crate) fn stake_table_verified(&self, epoch: EpochNumber) {
        // Adding only the increase keeps a concurrent caller from writing an older maximum last.
        let prev = self
            .max_stake_table_epoch
            .fetch_max(*epoch, Ordering::Relaxed);
        if *epoch > prev {
            self.latest_stake_table_epoch.update((*epoch - prev) as i64);
        }
    }

    pub(crate) async fn db_read<T>(&self, op: impl Future<Output = T>) -> T {
        #[cfg(feature = "client")]
        let op = timed(&*self.db_read_duration, op);
        op.await
    }

    pub(crate) async fn db_write<T>(&self, op: impl Future<Output = T>) -> T {
        #[cfg(feature = "client")]
        let op = timed(&*self.db_write_duration, op);
        op.await
    }

    pub(crate) fn start_catchup(&self, epochs: u64) -> CatchupBacklog<'_> {
        self.stake_table_backlog.update(epochs as i64);
        CatchupBacklog {
            gauge: &*self.stake_table_backlog,
            remaining: epochs,
        }
    }
}

// Only with `client`, since std's clock panics on zkVM targets.
#[cfg(feature = "client")]
async fn timed<T>(histogram: &dyn Histogram, op: impl Future<Output = T>) -> T {
    let started = std::time::Instant::now();
    let out = op.await;
    histogram.add_point(started.elapsed().as_secs_f64());
    out
}

impl Default for LightClientMetrics {
    fn default() -> Self {
        Self::new(&NoMetrics)
    }
}

/// One catchup's share of the stake table backlog, removed when the catchup ends however it ends,
/// so the gauge sums the epochs left across the catchups still running.
pub(crate) struct CatchupBacklog<'a> {
    gauge: &'a dyn Gauge,
    remaining: u64,
}

impl CatchupBacklog<'_> {
    pub(crate) fn epoch_done(&mut self) {
        self.remaining -= 1;
        self.gauge.update(-1);
    }
}

impl Drop for CatchupBacklog<'_> {
    fn drop(&mut self) {
        self.gauge.update(-(self.remaining as i64));
    }
}
