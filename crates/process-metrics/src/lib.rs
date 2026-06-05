use std::{
    fs,
    io::BufReader,
    path::Path,
    time::{Duration, Instant},
};

use hotshot_types::traits::metrics::{Counter, Gauge, Metrics};
use procfs::{
    Current, LoadAverage, PressureRecord, get_pressure,
    process::{Io, Process},
};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::time::interval;

const SAMPLE_INTERVAL: Duration = Duration::from_secs(5);

const CGROUP_ROOT: &str = "/sys/fs/cgroup";
const HOST_PRESSURE_DIR: &str = "/proc/pressure";

/// Which directory to read PSI files from. Detected once at startup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PressureSource {
    /// cgroup v2: `/sys/fs/cgroup/{cpu,memory,io}.pressure`
    CgroupV2,
    /// host-wide: `/proc/pressure/{cpu,memory,io}`
    Host,
    /// PSI unavailable on this kernel/container.
    None,
}

impl PressureSource {
    fn detect() -> Self {
        let cgroup = Path::new(CGROUP_ROOT).join("cpu.pressure");
        if Path::new(CGROUP_ROOT).join("cgroup.controllers").exists() && cgroup.exists() {
            return Self::CgroupV2;
        }
        if Path::new(HOST_PRESSURE_DIR).join("cpu").exists() {
            return Self::Host;
        }
        Self::None
    }

    fn path(self, resource: &str) -> Option<String> {
        match self {
            Self::CgroupV2 => Some(format!("{CGROUP_ROOT}/{resource}.pressure")),
            Self::Host => Some(format!("{HOST_PRESSURE_DIR}/{resource}")),
            Self::None => None,
        }
    }
}

/// Whether cgroup v2 cpu/memory accounting files are readable.
fn detect_cgroup_v2() -> bool {
    Path::new(CGROUP_ROOT).join("cpu.stat").exists()
        && Path::new(CGROUP_ROOT).join("memory.current").exists()
}

/// Accumulates fractional units (µs or ticks) into a counter measured in whole seconds,
/// preserving sub-second precision across many ticks.
#[derive(Default)]
struct SecondsAccumulator {
    /// Last absolute reading from the kernel, in the source unit (µs or ticks).
    last: Option<u64>,
    /// Sub-second remainder carried between calls, in the source unit.
    remainder: u64,
}

impl SecondsAccumulator {
    /// Feed an absolute monotonic reading. Returns whole-seconds delta to add to the counter.
    fn observe(&mut self, current: u64, units_per_second: u64) -> usize {
        let Some(prev) = self.last.replace(current) else {
            return 0;
        };
        let delta = current.saturating_sub(prev);
        let total = self.remainder + delta;
        let whole = total / units_per_second;
        self.remainder = total % units_per_second;
        whole as usize
    }
}

/// Tracks the previous absolute value of a `u64` counter for delta-add against a `Counter`.
#[derive(Default)]
struct U64Delta {
    last: Option<u64>,
}

impl U64Delta {
    fn observe(&mut self, current: u64) -> usize {
        let Some(prev) = self.last.replace(current) else {
            return 0;
        };
        current.saturating_sub(prev) as usize
    }
}

pub struct ProcessMetrics {
    resident_memory_bytes: Box<dyn Gauge>,
    virtual_memory_bytes: Box<dyn Gauge>,
    open_fds: Box<dyn Gauge>,
    threads: Box<dyn Gauge>,
    uptime_seconds: Box<dyn Gauge>,

    cpu_count: Box<dyn Gauge>,
    load1_milli: Box<dyn Gauge>,
    load5_milli: Box<dyn Gauge>,
    load15_milli: Box<dyn Gauge>,

    process_cpu_seconds_total: Box<dyn Counter>,

    pressure_cpu_some_total: Box<dyn Counter>,
    pressure_memory_some_total: Box<dyn Counter>,
    pressure_memory_full_total: Box<dyn Counter>,
    pressure_io_some_total: Box<dyn Counter>,
    pressure_io_full_total: Box<dyn Counter>,

    cgroup_cpu_periods_total: Box<dyn Counter>,
    cgroup_cpu_throttled_periods_total: Box<dyn Counter>,
    cgroup_cpu_throttled_seconds_total: Box<dyn Counter>,

    cgroup_memory_current_bytes: Box<dyn Gauge>,

    process_read_bytes_total: Box<dyn Counter>,
    process_write_bytes_total: Box<dyn Counter>,
}

/// Immutable per-tick context detected once at startup.
#[derive(Clone, Copy)]
struct Env {
    pid: Pid,
    start: Instant,
    pressure: PressureSource,
    cgroup_v2: bool,
    ticks_per_second: u64,
}

/// Cross-tick state: previous absolute readings + sub-second remainders.
#[derive(Default)]
struct Previous {
    cpu_ticks: SecondsAccumulator,
    pressure_cpu_some: SecondsAccumulator,
    pressure_memory_some: SecondsAccumulator,
    pressure_memory_full: SecondsAccumulator,
    pressure_io_some: SecondsAccumulator,
    pressure_io_full: SecondsAccumulator,
    cgroup_cpu_throttled_us: SecondsAccumulator,
    cgroup_cpu_periods: U64Delta,
    cgroup_cpu_throttled_periods: U64Delta,
    read_bytes: U64Delta,
    write_bytes: U64Delta,
}

impl ProcessMetrics {
    pub fn new(metrics: &(impl Metrics + ?Sized)) -> Self {
        let bytes = || Some("bytes".into());
        let seconds = || Some("seconds".into());

        // `memory.max` is either a u64 or the literal "max" (unlimited). Only register the
        // gauge when finite so operators don't see a perpetual 0 that looks like a 0-byte
        // limit. Container memory limits don't change at runtime, so set it once here; the
        // registry retains its own handle, so we don't keep the `Box` around.
        if let Some(max_bytes) = read_cgroup_memory_max() {
            metrics
                .create_gauge("cgroup_memory_max_bytes".into(), bytes())
                .set(max_bytes as usize);
        }

        Self {
            resident_memory_bytes: metrics
                .create_gauge("process_resident_memory_bytes".into(), bytes()),
            virtual_memory_bytes: metrics
                .create_gauge("process_virtual_memory_bytes".into(), bytes()),
            open_fds: metrics.create_gauge("process_open_fds".into(), None),
            threads: metrics.create_gauge("process_threads".into(), None),
            uptime_seconds: metrics.create_gauge("process_uptime_seconds".into(), seconds()),

            cpu_count: metrics.create_gauge("node_cpu_count".into(), None),
            load1_milli: metrics.create_gauge("node_load1_milli".into(), None),
            load5_milli: metrics.create_gauge("node_load5_milli".into(), None),
            load15_milli: metrics.create_gauge("node_load15_milli".into(), None),

            process_cpu_seconds_total: metrics
                .create_counter("process_cpu_seconds_total".into(), seconds()),

            pressure_cpu_some_total: metrics
                .create_counter("node_pressure_cpu_waiting_seconds_total".into(), seconds()),
            pressure_memory_some_total: metrics.create_counter(
                "node_pressure_memory_waiting_seconds_total".into(),
                seconds(),
            ),
            pressure_memory_full_total: metrics.create_counter(
                "node_pressure_memory_stalled_seconds_total".into(),
                seconds(),
            ),
            pressure_io_some_total: metrics
                .create_counter("node_pressure_io_waiting_seconds_total".into(), seconds()),
            pressure_io_full_total: metrics
                .create_counter("node_pressure_io_stalled_seconds_total".into(), seconds()),

            cgroup_cpu_periods_total: metrics
                .create_counter("cgroup_cpu_periods_total".into(), None),
            cgroup_cpu_throttled_periods_total: metrics
                .create_counter("cgroup_cpu_throttled_periods_total".into(), None),
            cgroup_cpu_throttled_seconds_total: metrics
                .create_counter("cgroup_cpu_throttled_seconds_total".into(), seconds()),

            cgroup_memory_current_bytes: metrics
                .create_gauge("cgroup_memory_current_bytes".into(), bytes()),

            process_read_bytes_total: metrics
                .create_counter("process_read_bytes_total".into(), bytes()),
            process_write_bytes_total: metrics
                .create_counter("process_write_bytes_total".into(), bytes()),
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

        let env = Env {
            pid,
            start: Instant::now(),
            pressure: PressureSource::detect(),
            cgroup_v2: detect_cgroup_v2(),
            ticks_per_second: procfs::ticks_per_second(),
        };
        tracing::info!(
            pressure = ?env.pressure,
            cgroup_v2 = env.cgroup_v2,
            ticks_per_second = env.ticks_per_second,
            "process metrics source detection complete"
        );

        // CPU count is process-invariant; set once and drop the periodic sample.
        self.cpu_count.set(sysinfo::System::new().cpus().len());

        let mut system = System::new();
        let mut previous = Previous::default();
        let mut ticker = interval(SAMPLE_INTERVAL);
        loop {
            ticker.tick().await;
            self.sample(&mut system, env, &mut previous);
        }
    }

    fn sample(&mut self, system: &mut System, env: Env, prev: &mut Previous) {
        let Env {
            pid,
            start,
            pressure,
            cgroup_v2,
            ticks_per_second,
        } = env;
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

        if let Some(load) = read_or_debug("loadavg", LoadAverage::current) {
            self.load1_milli.set(milli(load.one));
            self.load5_milli.set(milli(load.five));
            self.load15_milli.set(milli(load.fifteen));
        }

        if let Some(p) = read_or_debug("process self", Process::myself) {
            if let Some(stat) = read_or_debug("/proc/self/stat", || p.stat()) {
                let total_ticks = stat.utime + stat.stime;
                self.process_cpu_seconds_total
                    .add(prev.cpu_ticks.observe(total_ticks, ticks_per_second));
            }
            if let Some(Io {
                read_bytes,
                write_bytes,
                ..
            }) = read_or_debug("/proc/self/io", || p.io())
            {
                self.process_read_bytes_total
                    .add(prev.read_bytes.observe(read_bytes));
                self.process_write_bytes_total
                    .add(prev.write_bytes.observe(write_bytes));
            }
        }

        self.sample_pressure(pressure, prev);

        if cgroup_v2 {
            self.sample_cgroup_cpu(prev);
            self.sample_cgroup_memory();
        }
    }

    fn sample_pressure(&self, pressure: PressureSource, prev: &mut Previous) {
        if let Some(cpu_path) = pressure.path("cpu")
            && let Some((some, _full)) = read_pressure(&cpu_path)
        {
            self.pressure_cpu_some_total
                .add(prev.pressure_cpu_some.observe(some.total, 1_000_000));
        }

        if let Some(mem_path) = pressure.path("memory")
            && let Some((some, full)) = read_pressure(&mem_path)
        {
            self.pressure_memory_some_total
                .add(prev.pressure_memory_some.observe(some.total, 1_000_000));
            self.pressure_memory_full_total
                .add(prev.pressure_memory_full.observe(full.total, 1_000_000));
        }

        if let Some(io_path) = pressure.path("io")
            && let Some((some, full)) = read_pressure(&io_path)
        {
            self.pressure_io_some_total
                .add(prev.pressure_io_some.observe(some.total, 1_000_000));
            self.pressure_io_full_total
                .add(prev.pressure_io_full.observe(full.total, 1_000_000));
        }
    }

    fn sample_cgroup_cpu(&self, prev: &mut Previous) {
        let Some(stat) = read_cgroup_cpu_stat() else {
            return;
        };
        self.cgroup_cpu_periods_total
            .add(prev.cgroup_cpu_periods.observe(stat.nr_periods));
        self.cgroup_cpu_throttled_periods_total
            .add(prev.cgroup_cpu_throttled_periods.observe(stat.nr_throttled));
        self.cgroup_cpu_throttled_seconds_total.add(
            prev.cgroup_cpu_throttled_us
                .observe(stat.throttled_usec, 1_000_000),
        );
    }

    fn sample_cgroup_memory(&self) {
        if let Some(bytes) = read_u64_file(&format!("{CGROUP_ROOT}/memory.current")) {
            self.cgroup_memory_current_bytes.set(bytes as usize);
        }
        // `cgroup_memory_max_bytes` is set once at startup in `new()`.
    }
}

fn milli(v: f32) -> usize {
    (v * 1000.0).max(0.0) as usize
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

fn read_or_debug<T, E, F>(what: &str, f: F) -> Option<T>
where
    F: FnOnce() -> Result<T, E>,
    E: std::fmt::Display,
{
    match f() {
        Ok(v) => Some(v),
        Err(err) => {
            tracing::debug!(%what, %err, "process metrics read failed");
            None
        },
    }
}

fn read_pressure(path: &str) -> Option<(PressureRecord, PressureRecord)> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(err) => {
            tracing::debug!(%path, %err, "could not open pressure file");
            return None;
        },
    };
    match get_pressure(BufReader::new(file)) {
        Ok(v) => Some(v),
        Err(err) => {
            tracing::debug!(%path, %err, "could not parse pressure file");
            None
        },
    }
}

#[derive(Default)]
struct CpuStat {
    nr_periods: u64,
    nr_throttled: u64,
    throttled_usec: u64,
}

fn read_cgroup_cpu_stat() -> Option<CpuStat> {
    let path = format!("{CGROUP_ROOT}/cpu.stat");
    let contents = read_string_file(&path)?;
    let mut out = CpuStat::default();
    let mut saw_any = false;
    for line in contents.lines() {
        let mut parts = line.split_whitespace();
        let (Some(key), Some(value)) = (parts.next(), parts.next()) else {
            continue;
        };
        let Ok(value) = value.parse::<u64>() else {
            continue;
        };
        match key {
            "nr_periods" => {
                out.nr_periods = value;
                saw_any = true;
            },
            "nr_throttled" => {
                out.nr_throttled = value;
                saw_any = true;
            },
            "throttled_usec" => {
                out.throttled_usec = value;
                saw_any = true;
            },
            _ => {},
        }
    }
    saw_any.then_some(out)
}

fn read_string_file(path: &str) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(s) => Some(s),
        Err(err) => {
            tracing::debug!(%path, %err, "could not read file for process metrics");
            None
        },
    }
}

/// Read `cgroup_root/memory.max`. Returns `None` when the file is missing/unreadable or
/// holds the literal `max` (unlimited).
fn read_cgroup_memory_max() -> Option<u64> {
    let raw = read_string_file(&format!("{CGROUP_ROOT}/memory.max"))?;
    raw.trim().parse::<u64>().ok()
}

fn read_u64_file(path: &str) -> Option<u64> {
    let s = read_string_file(path)?;
    match s.trim().parse::<u64>() {
        Ok(v) => Some(v),
        Err(err) => {
            tracing::debug!(%path, %err, "could not parse u64 from file");
            None
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds_accumulator_first_sample_is_zero() {
        let mut acc = SecondsAccumulator::default();
        assert_eq!(acc.observe(123_456, 1_000_000), 0);
    }

    #[test]
    fn seconds_accumulator_preserves_remainder() {
        let mut acc = SecondsAccumulator::default();
        // First call seeds the baseline.
        assert_eq!(acc.observe(0, 1_000_000), 0);
        // 0.5s delta — no whole second yet.
        assert_eq!(acc.observe(500_000, 1_000_000), 0);
        // Another 0.6s delta — one whole second, 0.1s remainder.
        assert_eq!(acc.observe(1_100_000, 1_000_000), 1);
        // Another 0.95s — total now 1.05s of remainder + delta → 1 sec.
        assert_eq!(acc.observe(2_050_000, 1_000_000), 1);
    }

    #[test]
    fn seconds_accumulator_handles_counter_reset() {
        let mut acc = SecondsAccumulator::default();
        acc.observe(10_000_000, 1_000_000);
        // Apparent regression (e.g. proc remount or wraparound), saturate to 0.
        assert_eq!(acc.observe(5_000_000, 1_000_000), 0);
        // After saturating, `last` should equal the most recent reading; the next
        // legitimate delta from there should still register.
        assert_eq!(acc.observe(6_000_000, 1_000_000), 1);
    }

    #[test]
    fn u64_delta_first_sample_is_zero() {
        let mut d = U64Delta::default();
        assert_eq!(d.observe(42), 0);
        assert_eq!(d.observe(45), 3);
        // Reset: don't emit a negative spike.
        assert_eq!(d.observe(10), 0);
    }

    #[test]
    fn milli_clamps_negative_to_zero() {
        assert_eq!(milli(0.0), 0);
        assert_eq!(milli(1.25), 1250);
        assert_eq!(milli(-0.1), 0);
    }
}
