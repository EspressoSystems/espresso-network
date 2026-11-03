//! Metrics of the tokio runtime the node runs on.

use hotshot_types::traits::metrics::{Counter, Gauge, Metrics};
use tokio::runtime::{Handle, RuntimeMetrics};

use crate::accumulate::SecondsAccumulator;

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Runtime-level metrics, sampled from the runtime the sampling task itself
/// runs on.
pub struct TokioMetrics {
    workers: Box<dyn Gauge>,
    alive_tasks: Box<dyn Gauge>,
    global_queue_depth: Box<dyn Gauge>,
    worker_busy_seconds_total: Box<dyn Counter>,
    #[cfg(tokio_unstable)]
    blocking: BlockingPool,
    runtime: Option<RuntimeMetrics>,
    busy_nanos: SecondsAccumulator,
}

impl TokioMetrics {
    pub fn new(metrics: &(impl Metrics + ?Sized)) -> Self {
        Self {
            workers: metrics.create_gauge("tokio_workers".into(), None),
            alive_tasks: metrics.create_gauge("tokio_alive_tasks".into(), None),
            global_queue_depth: metrics.create_gauge("tokio_global_queue_depth".into(), None),
            worker_busy_seconds_total: metrics.create_counter(
                "tokio_worker_busy_seconds_total".into(),
                Some("seconds".into()),
            ),
            #[cfg(tokio_unstable)]
            blocking: BlockingPool::new(metrics),
            runtime: None,
            busy_nanos: SecondsAccumulator::default(),
        }
    }

    /// Bind to the current runtime before the sampling loop starts.
    ///
    /// The worker count is fixed for the runtime's lifetime, so it is set here
    /// rather than sampled.
    pub fn init(&mut self) {
        let Ok(handle) = Handle::try_current() else {
            tracing::warn!("no current tokio runtime; runtime metrics disabled");
            return;
        };
        let runtime = handle.metrics();
        self.workers.set(runtime.num_workers());
        tracing::info!(
            workers = runtime.num_workers(),
            blocking_pool = cfg!(tokio_unstable),
            "tokio runtime metrics bound"
        );
        if !cfg!(tokio_unstable) {
            tracing::warn!(
                "built without --cfg tokio_unstable; blocking-pool metrics are not collected"
            );
        }
        self.runtime = Some(runtime);
    }

    pub fn sample(&mut self) {
        let Some(runtime) = &self.runtime else {
            return;
        };

        self.alive_tasks.set(runtime.num_alive_tasks());
        self.global_queue_depth.set(runtime.global_queue_depth());
        self.worker_busy_seconds_total.add(
            self.busy_nanos
                .observe(busy_nanos(runtime), NANOS_PER_SECOND),
        );

        #[cfg(tokio_unstable)]
        self.blocking.sample(runtime);
    }
}

/// The pool `spawn_blocking` runs on, which is separate from the workers and
/// where a synchronous call that should have been `spawn_blocking`'d shows up
/// as a queue that never drains.
///
/// Only `--cfg tokio_unstable` exposes these, so a build without it keeps the
/// worker metrics and drops these; see `.cargo/config.toml`.
#[cfg(tokio_unstable)]
struct BlockingPool {
    threads: Box<dyn Gauge>,
    idle_threads: Box<dyn Gauge>,
    queue_depth: Box<dyn Gauge>,
}

#[cfg(tokio_unstable)]
impl BlockingPool {
    fn new(metrics: &(impl Metrics + ?Sized)) -> Self {
        Self {
            threads: metrics.create_gauge("tokio_blocking_threads".into(), None),
            idle_threads: metrics.create_gauge("tokio_idle_blocking_threads".into(), None),
            queue_depth: metrics.create_gauge("tokio_blocking_queue_depth".into(), None),
        }
    }

    fn sample(&self, runtime: &RuntimeMetrics) {
        self.threads.set(runtime.num_blocking_threads());
        self.idle_threads.set(runtime.num_idle_blocking_threads());
        self.queue_depth.set(runtime.blocking_queue_depth());
    }
}

/// Time all workers have spent polling since the runtime was created.
///
/// Per-worker busy durations are only interesting when they diverge, which the
/// multi-threaded runtime's work stealing keeps them from doing, so they are
/// summed into one counter instead of a series per worker.
fn busy_nanos(runtime: &RuntimeMetrics) -> u64 {
    let nanos: u128 = (0..runtime.num_workers())
        .map(|worker| runtime.worker_total_busy_duration(worker).as_nanos())
        .sum();
    u64::try_from(nanos).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use hotshot_types::traits::metrics::NoMetrics;

    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn samples_every_worker_of_the_current_runtime() {
        let mut metrics = TokioMetrics::new(&NoMetrics);
        metrics.init();
        let runtime = metrics.runtime.as_ref().expect("bound to this runtime");
        assert_eq!(runtime.num_workers(), 2);
        // `worker_total_busy_duration` panics on an out-of-range worker index.
        metrics.sample();
    }

    /// Tripwire for a build path that lost `--cfg tokio_unstable` and with it
    /// the blocking-pool metrics; see `.cargo/config.toml`.
    #[test]
    fn built_with_tokio_unstable() {
        if !cfg!(tokio_unstable) {
            panic!(
                "--cfg tokio_unstable missing: a RUSTFLAGS in the environment replaces \
                 build.rustflags from .cargo/config.toml rather than adding to it"
            );
        }
    }

    #[test]
    fn sampling_without_a_runtime_is_a_no_op() {
        let mut metrics = TokioMetrics::new(&NoMetrics);
        metrics.init();
        assert!(metrics.runtime.is_none());
        metrics.sample();
    }
}
