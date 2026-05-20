use std::{
    fs,
    sync::Arc,
    time::{Duration, Instant},
};

use hotshot_types::traits::metrics::{Gauge, Metrics};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::time::interval;

const SAMPLE_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct ProcessMetrics {
    resident_memory_bytes: Arc<dyn Gauge>,
    virtual_memory_bytes: Arc<dyn Gauge>,
    open_fds: Arc<dyn Gauge>,
    threads: Arc<dyn Gauge>,
    uptime_seconds: Arc<dyn Gauge>,
}

impl ProcessMetrics {
    pub fn new(metrics: &(impl Metrics + ?Sized)) -> Self {
        Self {
            resident_memory_bytes: metrics
                .create_gauge("process_resident_memory_bytes".into(), Some("bytes".into()))
                .into(),
            virtual_memory_bytes: metrics
                .create_gauge("process_virtual_memory_bytes".into(), Some("bytes".into()))
                .into(),
            open_fds: metrics.create_gauge("process_open_fds".into(), None).into(),
            threads: metrics.create_gauge("process_threads".into(), None).into(),
            uptime_seconds: metrics
                .create_gauge("process_uptime_seconds".into(), Some("seconds".into()))
                .into(),
        }
    }

    pub async fn run(self) {
        let pid = match sysinfo::get_current_pid() {
            Ok(pid) => pid,
            Err(err) => {
                tracing::warn!(%err, "could not determine current pid; process metrics disabled");
                return;
            },
        };

        let start = Instant::now();
        let mut system = System::new();
        let mut ticker = interval(SAMPLE_INTERVAL);
        loop {
            ticker.tick().await;
            self.sample(&mut system, pid, start);
        }
    }

    fn sample(&self, system: &mut System, pid: Pid, start: Instant) {
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            ProcessRefreshKind::nothing().with_memory(),
        );

        if let Some(process) = system.process(pid) {
            self.resident_memory_bytes.set(process.memory() as usize);
            self.virtual_memory_bytes
                .set(process.virtual_memory() as usize);
        }

        self.open_fds.set(count_dir_entries("/proc/self/fd"));
        self.threads.set(count_dir_entries("/proc/self/task"));
        self.uptime_seconds
            .set(Instant::now().duration_since(start).as_secs() as usize);
    }
}

fn count_dir_entries(path: &str) -> usize {
    match fs::read_dir(path) {
        Ok(d) => d.filter(Result::is_ok).count(),
        Err(err) => {
            tracing::debug!(%path, %err, "could not read directory for process metrics");
            0
        },
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashSet,
        sync::{Arc, Mutex},
    };

    use hotshot_types::traits::metrics::{
        Counter, CounterFamily, Gauge, GaugeFamily, Histogram, HistogramFamily, Metrics,
        MetricsFamily, TextFamily,
    };

    use super::*;

    #[derive(Clone, Debug, Default)]
    struct RecordingMetrics {
        gauges: Arc<Mutex<HashSet<String>>>,
    }

    impl Metrics for RecordingMetrics {
        fn create_counter(&self, _: String, _: Option<String>) -> Box<dyn Counter> {
            Box::new(NoopMetric)
        }
        fn create_gauge(&self, name: String, _: Option<String>) -> Box<dyn Gauge> {
            self.gauges.lock().unwrap().insert(name);
            Box::new(NoopMetric)
        }
        fn create_histogram(&self, _: String, _: Option<String>) -> Box<dyn Histogram> {
            Box::new(NoopMetric)
        }
        fn create_text(&self, _: String) {}
        fn counter_family(&self, _: String, _: Vec<String>) -> Box<dyn CounterFamily> {
            Box::new(NoopMetric)
        }
        fn gauge_family(&self, _: String, _: Vec<String>) -> Box<dyn GaugeFamily> {
            Box::new(NoopMetric)
        }
        fn histogram_family(&self, _: String, _: Vec<String>) -> Box<dyn HistogramFamily> {
            Box::new(NoopMetric)
        }
        fn text_family(&self, _: String, _: Vec<String>) -> Box<dyn TextFamily> {
            Box::new(NoopMetric)
        }
        fn subgroup(&self, _: String) -> Box<dyn Metrics> {
            Box::new(self.clone())
        }
    }

    #[derive(Clone, Debug)]
    struct NoopMetric;

    impl Counter for NoopMetric {
        fn add(&self, _: usize) {}
    }
    impl Gauge for NoopMetric {
        fn set(&self, _: usize) {}
        fn update(&self, _: i64) {}
    }
    impl Histogram for NoopMetric {
        fn add_point(&self, _: f64) {}
    }
    impl MetricsFamily<Box<dyn Counter>> for NoopMetric {
        fn create(&self, _: Vec<String>) -> Box<dyn Counter> {
            Box::new(NoopMetric)
        }
    }
    impl MetricsFamily<Box<dyn Gauge>> for NoopMetric {
        fn create(&self, _: Vec<String>) -> Box<dyn Gauge> {
            Box::new(NoopMetric)
        }
    }
    impl MetricsFamily<Box<dyn Histogram>> for NoopMetric {
        fn create(&self, _: Vec<String>) -> Box<dyn Histogram> {
            Box::new(NoopMetric)
        }
    }
    impl MetricsFamily<()> for NoopMetric {
        fn create(&self, _: Vec<String>) {}
    }

    #[test]
    fn process_metrics_registers_all_five_gauges() {
        let metrics = RecordingMetrics::default();
        let _ = ProcessMetrics::new(&metrics);
        let names = metrics.gauges.lock().unwrap().clone();
        for expected in [
            "process_resident_memory_bytes",
            "process_virtual_memory_bytes",
            "process_open_fds",
            "process_threads",
            "process_uptime_seconds",
        ] {
            assert!(
                names.contains(expected),
                "missing gauge {expected}; got {names:?}"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn process_metrics_sample_nonzero_on_linux() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        #[derive(Clone, Debug)]
        struct CapturingGauge(Arc<AtomicUsize>);
        impl Gauge for CapturingGauge {
            fn set(&self, v: usize) {
                self.0.store(v, Ordering::SeqCst);
            }
            fn update(&self, _: i64) {}
        }

        let rss = Arc::new(AtomicUsize::new(0));
        let vsz = Arc::new(AtomicUsize::new(0));
        let fds = Arc::new(AtomicUsize::new(0));
        let threads = Arc::new(AtomicUsize::new(0));
        let uptime = Arc::new(AtomicUsize::new(0));

        let metrics = ProcessMetrics {
            resident_memory_bytes: Arc::new(CapturingGauge(rss.clone())),
            virtual_memory_bytes: Arc::new(CapturingGauge(vsz.clone())),
            open_fds: Arc::new(CapturingGauge(fds.clone())),
            threads: Arc::new(CapturingGauge(threads.clone())),
            uptime_seconds: Arc::new(CapturingGauge(uptime.clone())),
        };

        let pid = sysinfo::get_current_pid().expect("pid");
        let mut system = System::new();
        let start = Instant::now() - Duration::from_secs(1);
        metrics.sample(&mut system, pid, start);

        assert!(rss.load(Ordering::SeqCst) > 0, "rss should be positive");
        assert!(
            threads.load(Ordering::SeqCst) > 0,
            "thread count should be positive"
        );
        assert!(
            fds.load(Ordering::SeqCst) > 0,
            "fd count should be positive"
        );
        assert!(
            uptime.load(Ordering::SeqCst) >= 1,
            "uptime should be at least 1s"
        );
    }
}
