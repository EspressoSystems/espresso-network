//! Durations are measured only with the `client` feature, since std's clock panics on zkVM
//! targets.

#[cfg(feature = "client")]
use std::time::Instant;
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
    stake_table_catchup_duration: Box<dyn Histogram>,
    follower_block_height: Box<dyn Gauge>,
    verification_failures: Box<dyn Counter>,
    #[cfg(feature = "client")]
    fetch_requests: Box<dyn Counter>,
    #[cfg(feature = "client")]
    fetch_failures: Box<dyn Counter>,
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
            fetch_requests: metrics.create_counter("fetch_requests".into(), None),
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

    pub(crate) fn set_follower_block_height(&self, height: u64) {
        self.follower_block_height.set(height as usize);
    }

    pub(crate) fn verification_failed(&self) {
        self.verification_failures.add(1);
    }

    #[cfg(feature = "client")]
    pub(crate) fn fetch_requested(&self) {
        self.fetch_requests.add(1);
    }

    #[cfg(feature = "client")]
    pub(crate) fn fetch_failed(&self) {
        self.fetch_failures.add(1);
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

    pub(crate) fn start_catchup(&self, epochs: u64) -> CatchupProgress<'_> {
        self.stake_table_backlog.update(epochs as i64);
        CatchupProgress {
            metrics: self,
            remaining: epochs,
            #[cfg(feature = "client")]
            epoch_started: Instant::now(),
        }
    }
}

impl Default for LightClientMetrics {
    fn default() -> Self {
        Self::new(&NoMetrics)
    }
}

#[cfg(feature = "client")]
async fn timed<T>(histogram: &dyn Histogram, op: impl Future<Output = T>) -> T {
    let started = Instant::now();
    let out = op.await;
    histogram.add_point(started.elapsed().as_secs_f64());
    out
}

/// One stake table catchup. Its share of the backlog is removed when it ends however it ends, so
/// the gauge sums the epochs left across the catchups still running.
#[derive(Debug)]
pub(crate) struct CatchupProgress<'a> {
    metrics: &'a LightClientMetrics,
    remaining: u64,
    #[cfg(feature = "client")]
    epoch_started: Instant,
}

impl CatchupProgress<'_> {
    pub(crate) fn epoch_done(&mut self) {
        self.remaining -= 1;
        self.metrics.stake_table_backlog.update(-1);
        #[cfg(feature = "client")]
        {
            self.metrics
                .stake_table_catchup_duration
                .add_point(self.epoch_started.elapsed().as_secs_f64());
            self.epoch_started = Instant::now();
        }
    }
}

impl Drop for CatchupProgress<'_> {
    fn drop(&mut self) {
        self.metrics
            .stake_table_backlog
            .update(-(self.remaining as i64));
    }
}
