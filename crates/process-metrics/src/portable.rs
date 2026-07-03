use std::time::{Duration, Instant};

use hotshot_types::traits::metrics::{Gauge, Metrics};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::time::interval;

use crate::ext::LinuxMetrics;

const SAMPLE_INTERVAL: Duration = Duration::from_secs(5);

/// Cross-platform process metrics, plus a Linux-only extension for `/proc`/cgroup data.
pub struct ProcessMetrics {
    resident_memory_bytes: Box<dyn Gauge>,
    virtual_memory_bytes: Box<dyn Gauge>,
    uptime_seconds: Box<dyn Gauge>,
    cpu_count: Box<dyn Gauge>,
    linux: LinuxMetrics,
}

impl ProcessMetrics {
    pub fn new(metrics: &(impl Metrics + ?Sized)) -> Self {
        let bytes = || Some("bytes".into());
        Self {
            resident_memory_bytes: metrics
                .create_gauge("process_resident_memory_bytes".into(), bytes()),
            virtual_memory_bytes: metrics
                .create_gauge("process_virtual_memory_bytes".into(), bytes()),
            uptime_seconds: metrics
                .create_gauge("process_uptime_seconds".into(), Some("seconds".into())),
            cpu_count: metrics.create_gauge("node_cpu_count".into(), None),
            linux: LinuxMetrics::new(metrics),
        }
    }

    pub async fn run(mut self) {
        let pid = match sysinfo::get_current_pid() {
            Ok(pid) => pid,
            Err(err) => {
                tracing::warn!(%err, "could not determine current pid; process metrics disabled");
                return;
            },
        };
        let start = Instant::now();

        // CPU count is process-invariant; set once and drop the periodic sample.
        self.cpu_count.set(cpu_count());
        self.linux.init();

        let mut system = System::new();
        let mut ticker = interval(SAMPLE_INTERVAL);
        loop {
            ticker.tick().await;
            self.sample(&mut system, pid, start);
        }
    }

    fn sample(&mut self, system: &mut System, pid: Pid, start: Instant) {
        if let Some((resident, virtual_)) = process_memory(system, pid) {
            self.resident_memory_bytes.set(resident as usize);
            self.virtual_memory_bytes.set(virtual_ as usize);
        }
        self.uptime_seconds
            .set(Instant::now().duration_since(start).as_secs() as usize);

        self.linux.sample();
    }
}

fn cpu_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

fn process_memory(system: &mut System, pid: Pid) -> Option<(u64, u64)> {
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing().with_memory(),
    );
    let p = system.process(pid)?;
    Some((p.memory(), p.virtual_memory()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_count_is_at_least_one() {
        assert_eq!(System::new().cpus().len(), 0);
        assert!(cpu_count() >= 1);
    }

    #[test]
    fn process_memory_reports_current_process() {
        let pid = sysinfo::get_current_pid().unwrap();
        let (resident, virtual_) = process_memory(&mut System::new(), pid).unwrap();
        assert!(resident > 0);
        assert!(virtual_ > 0);
    }
}
