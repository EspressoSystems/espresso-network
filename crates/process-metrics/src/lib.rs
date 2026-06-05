//! Process-level Prometheus metrics.
//!
//! The portable subset (resident/virtual memory, CPU count, uptime) is collected on every
//! platform via `sysinfo`. The Linux-only extension (`/proc`, cgroup, and PSI pressure data,
//! read via `procfs`) is compiled in on Linux and stubbed out everywhere else, so the only
//! platform `cfg` lives here.

mod portable;
pub use portable::ProcessMetrics;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod ext;

/// No-op stand-in for the Linux `/proc`/cgroup extension on other platforms (e.g. macOS).
/// [`ProcessMetrics`] still collects the portable subset; only these metrics are absent.
#[cfg(not(target_os = "linux"))]
mod ext {
    use hotshot_types::traits::metrics::Metrics;

    pub struct LinuxMetrics;

    impl LinuxMetrics {
        pub fn new(_metrics: &(impl Metrics + ?Sized)) -> Self {
            Self
        }

        pub fn init(&mut self) {
            tracing::info!("/proc and cgroup metrics are Linux-only; collecting portable subset");
        }

        pub fn sample(&mut self) {}
    }
}
