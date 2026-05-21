use std::{
    fs,
    time::{Duration, Instant},
};

use hotshot_types::traits::metrics::{Gauge, Metrics};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::time::interval;

const SAMPLE_INTERVAL: Duration = Duration::from_secs(5);

pub struct ProcessMetrics {
    resident_memory_bytes: Box<dyn Gauge>,
    virtual_memory_bytes: Box<dyn Gauge>,
    open_fds: Box<dyn Gauge>,
    threads: Box<dyn Gauge>,
    uptime_seconds: Box<dyn Gauge>,
}

impl ProcessMetrics {
    pub fn new(metrics: &(impl Metrics + ?Sized)) -> Self {
        Self {
            resident_memory_bytes: metrics
                .create_gauge("process_resident_memory_bytes".into(), Some("bytes".into())),
            virtual_memory_bytes: metrics
                .create_gauge("process_virtual_memory_bytes".into(), Some("bytes".into())),
            open_fds: metrics.create_gauge("process_open_fds".into(), None),
            threads: metrics.create_gauge("process_threads".into(), None),
            uptime_seconds: metrics
                .create_gauge("process_uptime_seconds".into(), Some("seconds".into())),
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
