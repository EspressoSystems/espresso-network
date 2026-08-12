use std::{collections::HashSet, fs, io::BufReader, path::Path};

use hotshot_types::traits::metrics::{Counter, Gauge, Metrics};
use procfs::{
    Current, CurrentSI, KernelStats, LoadAverage, PressureRecord, get_pressure,
    net::TcpState,
    process::{FDInfo, FDTarget, Io, Process},
};

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

/// Per-mode host CPU time counters, matching the fields of `/proc/stat`'s aggregate `cpu` line.
struct CpuModeCounters {
    user: Box<dyn Counter>,
    nice: Box<dyn Counter>,
    system: Box<dyn Counter>,
    idle: Box<dyn Counter>,
    iowait: Box<dyn Counter>,
    irq: Box<dyn Counter>,
    softirq: Box<dyn Counter>,
    steal: Box<dyn Counter>,
    guest: Box<dyn Counter>,
    guest_nice: Box<dyn Counter>,
}

/// Cross-tick accumulator state mirroring [`CpuModeCounters`].
#[derive(Default)]
struct CpuModeAccumulators {
    user: SecondsAccumulator,
    nice: SecondsAccumulator,
    system: SecondsAccumulator,
    idle: SecondsAccumulator,
    iowait: SecondsAccumulator,
    irq: SecondsAccumulator,
    softirq: SecondsAccumulator,
    steal: SecondsAccumulator,
    guest: SecondsAccumulator,
    guest_nice: SecondsAccumulator,
}

/// Immutable per-tick context detected once at startup.
#[derive(Clone, Copy)]
struct Env {
    pressure: PressureSource,
    cgroup_v2: bool,
    ticks_per_second: u64,
}

/// Cross-tick state: previous absolute readings + sub-second remainders.
#[derive(Default)]
struct Previous {
    cpu_ticks: SecondsAccumulator,
    cpu_modes: CpuModeAccumulators,
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

/// Linux-only metrics sourced from `/proc`, cgroup v2, and PSI pressure files.
pub struct LinuxMetrics {
    open_fds: Box<dyn Gauge>,
    threads: Box<dyn Gauge>,

    load1_milli: Box<dyn Gauge>,
    load5_milli: Box<dyn Gauge>,
    load15_milli: Box<dyn Gauge>,

    process_cpu_seconds_total: Box<dyn Counter>,
    cpu_mode_seconds_total: CpuModeCounters,

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

    env: Option<Env>,
    prev: Previous,

    tcp_recv_queue_max_bytes: Box<dyn Gauge>,
    tcp_recv_queue_total_bytes: Box<dyn Gauge>,
    tcp_send_queue_max_bytes: Box<dyn Gauge>,
    tcp_send_queue_total_bytes: Box<dyn Gauge>,
    tcp_established_sockets: Box<dyn Gauge>,
}

impl LinuxMetrics {
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
            open_fds: metrics.create_gauge("process_open_fds".into(), None),
            threads: metrics.create_gauge("process_threads".into(), None),

            load1_milli: metrics.create_gauge("node_load1_milli".into(), None),
            load5_milli: metrics.create_gauge("node_load5_milli".into(), None),
            load15_milli: metrics.create_gauge("node_load15_milli".into(), None),

            process_cpu_seconds_total: metrics
                .create_counter("process_cpu_seconds_total".into(), seconds()),
            cpu_mode_seconds_total: {
                let family = metrics
                    .counter_family("node_cpu_mode_seconds_total".into(), vec!["mode".into()]);
                CpuModeCounters {
                    user: family.create(vec!["user".into()]),
                    nice: family.create(vec!["nice".into()]),
                    system: family.create(vec!["system".into()]),
                    idle: family.create(vec!["idle".into()]),
                    iowait: family.create(vec!["iowait".into()]),
                    irq: family.create(vec!["irq".into()]),
                    softirq: family.create(vec!["softirq".into()]),
                    steal: family.create(vec!["steal".into()]),
                    guest: family.create(vec!["guest".into()]),
                    guest_nice: family.create(vec!["guest_nice".into()]),
                }
            },

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

            env: None,
            prev: Previous::default(),

            tcp_recv_queue_max_bytes: metrics
                .create_gauge("process_tcp_recv_queue_max_bytes".into(), bytes()),
            tcp_recv_queue_total_bytes: metrics
                .create_gauge("process_tcp_recv_queue_total_bytes".into(), bytes()),
            tcp_send_queue_max_bytes: metrics
                .create_gauge("process_tcp_send_queue_max_bytes".into(), bytes()),
            tcp_send_queue_total_bytes: metrics
                .create_gauge("process_tcp_send_queue_total_bytes".into(), bytes()),
            tcp_established_sockets: metrics
                .create_gauge("process_tcp_established_sockets".into(), None),
        }
    }

    /// Detect the PSI/cgroup sources once before the sampling loop starts.
    pub fn init(&mut self) {
        let env = Env {
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
        self.env = Some(env);
    }

    pub fn sample(&mut self) {
        let Some(env) = self.env else {
            return;
        };

        self.threads.set(count_dir_entries("/proc/self/task"));

        if let Some(load) = read_or_debug("loadavg", LoadAverage::current) {
            self.load1_milli.set(milli(load.one));
            self.load5_milli.set(milli(load.five));
            self.load15_milli.set(milli(load.fifteen));
        }

        if let Some(p) = read_or_debug("process self", Process::myself) {
            if let Some(stat) = read_or_debug("/proc/self/stat", || p.stat()) {
                let total_ticks = stat.utime + stat.stime;
                self.process_cpu_seconds_total.add(
                    self.prev
                        .cpu_ticks
                        .observe(total_ticks, env.ticks_per_second),
                );
            }
            if let Some(Io {
                read_bytes,
                write_bytes,
                ..
            }) = read_or_debug("/proc/self/io", || p.io())
            {
                self.process_read_bytes_total
                    .add(self.prev.read_bytes.observe(read_bytes));
                self.process_write_bytes_total
                    .add(self.prev.write_bytes.observe(write_bytes));
            }

            if let Some((fd_count, sockets)) = own_socket_inodes(&p) {
                self.open_fds.set(fd_count);
                self.sample_tcp(&p, &sockets);
            }
        }

        self.sample_cpu_stat(env.ticks_per_second);

        self.sample_pressure(env.pressure);

        if env.cgroup_v2 {
            self.sample_cgroup_cpu();
            self.sample_cgroup_memory();
        }
    }

    /// Host-wide CPU time by mode, aggregated across all CPUs. Unlike `process_cpu_seconds_total`,
    /// this exposes time (e.g. `steal`) the process itself never sees but that still explains why
    /// the host is slow. `guest`/`guest_nice` ticks are already included in `user`/`nice`
    /// respectively (the kernel's `account_guest_time()` double-books them), so summing all modes
    /// over-counts the denominator on hypervisors and understates utilization.
    fn sample_cpu_stat(&mut self, ticks_per_second: u64) {
        let Some(cpu) = read_or_debug("/proc/stat", KernelStats::current).map(|s| s.total) else {
            return;
        };
        let counters = &self.cpu_mode_seconds_total;
        let prev = &mut self.prev.cpu_modes;
        let modes: [(&dyn Counter, &mut SecondsAccumulator, Option<u64>); 10] = [
            (&*counters.user, &mut prev.user, Some(cpu.user)),
            (&*counters.nice, &mut prev.nice, Some(cpu.nice)),
            (&*counters.system, &mut prev.system, Some(cpu.system)),
            (&*counters.idle, &mut prev.idle, Some(cpu.idle)),
            (&*counters.iowait, &mut prev.iowait, cpu.iowait),
            (&*counters.irq, &mut prev.irq, cpu.irq),
            (&*counters.softirq, &mut prev.softirq, cpu.softirq),
            (&*counters.steal, &mut prev.steal, cpu.steal),
            (&*counters.guest, &mut prev.guest, cpu.guest),
            (&*counters.guest_nice, &mut prev.guest_nice, cpu.guest_nice),
        ];
        for (counter, acc, ticks) in modes {
            if let Some(ticks) = ticks {
                counter.add(acc.observe(ticks, ticks_per_second));
            }
        }
    }

    fn sample_pressure(&mut self, pressure: PressureSource) {
        if let Some(cpu_path) = pressure.path("cpu")
            && let Some((some, _full)) = read_pressure(&cpu_path)
        {
            self.pressure_cpu_some_total
                .add(self.prev.pressure_cpu_some.observe(some.total, 1_000_000));
        }

        if let Some(mem_path) = pressure.path("memory")
            && let Some((some, full)) = read_pressure(&mem_path)
        {
            self.pressure_memory_some_total.add(
                self.prev
                    .pressure_memory_some
                    .observe(some.total, 1_000_000),
            );
            self.pressure_memory_full_total.add(
                self.prev
                    .pressure_memory_full
                    .observe(full.total, 1_000_000),
            );
        }

        if let Some(io_path) = pressure.path("io")
            && let Some((some, full)) = read_pressure(&io_path)
        {
            self.pressure_io_some_total
                .add(self.prev.pressure_io_some.observe(some.total, 1_000_000));
            self.pressure_io_full_total
                .add(self.prev.pressure_io_full.observe(full.total, 1_000_000));
        }
    }

    fn sample_cgroup_cpu(&mut self) {
        let Some(stat) = read_cgroup_cpu_stat() else {
            return;
        };
        self.cgroup_cpu_periods_total
            .add(self.prev.cgroup_cpu_periods.observe(stat.nr_periods));
        self.cgroup_cpu_throttled_periods_total.add(
            self.prev
                .cgroup_cpu_throttled_periods
                .observe(stat.nr_throttled),
        );
        self.cgroup_cpu_throttled_seconds_total.add(
            self.prev
                .cgroup_cpu_throttled_us
                .observe(stat.throttled_usec, 1_000_000),
        );
    }

    fn sample_cgroup_memory(&self) {
        if let Some(bytes) = read_u64_file(&format!("{CGROUP_ROOT}/memory.current")) {
            self.cgroup_memory_current_bytes.set(bytes as usize);
        }
        // `cgroup_memory_max_bytes` is set once at startup in `new()`.
    }

    /// Whether this process is draining its sockets, so a networking problem can be attributed
    /// to the application or ruled out before an operator is asked to check their infrastructure.
    /// `owned` scopes the netns-wide tables to our sockets.
    fn sample_tcp(&self, p: &Process, owned: &HashSet<u64>) {
        let tcp4 = read_or_debug("/proc/self/net/tcp", || p.tcp()).unwrap_or_default();
        let tcp6 = read_or_debug("/proc/self/net/tcp6", || p.tcp6()).unwrap_or_default();
        let queues = aggregate_tcp(
            tcp4.into_iter()
                .chain(tcp6)
                .map(|e| (e.state, e.rx_queue, e.tx_queue, e.inode)),
            owned,
        );
        self.tcp_recv_queue_max_bytes.set(queues.recv_max as usize);
        self.tcp_recv_queue_total_bytes
            .set(queues.recv_total as usize);
        self.tcp_send_queue_max_bytes.set(queues.send_max as usize);
        self.tcp_send_queue_total_bytes
            .set(queues.send_total as usize);
        self.tcp_established_sockets.set(queues.sockets);
    }
}

fn milli(v: f32) -> usize {
    (v * 1000.0).max(0.0) as usize
}

/// One walk of `/proc/self/fd`: total count for `open_fds`, socket inodes for `sample_tcp`.
fn own_socket_inodes(p: &Process) -> Option<(usize, HashSet<u64>)> {
    let fds = read_or_debug("/proc/self/fd", || p.fd())?;
    let mut total = 0;
    let mut sockets = HashSet::new();
    for fd in fds {
        total += 1;
        if let Ok(FDInfo {
            target: FDTarget::Socket(inode),
            ..
        }) = fd
        {
            sockets.insert(inode);
        }
    }
    Some((total, sockets))
}

#[derive(Debug, Default)]
struct TcpQueues {
    recv_max: u32,
    recv_total: u64,
    send_max: u32,
    send_total: u64,
    sockets: usize,
}

/// Tuples rather than `TcpNetEntry`: that type is `#[non_exhaustive]`, so tests can't build one.
fn aggregate_tcp(
    entries: impl Iterator<Item = (TcpState, u32, u32, u64)>,
    owned: &HashSet<u64>,
) -> TcpQueues {
    let mut out = TcpQueues::default();
    for (state, rx_queue, tx_queue, inode) in entries {
        // On a listener these columns hold the accept backlog, not buffer occupancy.
        if state != TcpState::Established || !owned.contains(&inode) {
            continue;
        }
        out.recv_max = out.recv_max.max(rx_queue);
        out.recv_total += u64::from(rx_queue);
        out.send_max = out.send_max.max(tx_queue);
        out.send_total += u64::from(tx_queue);
        out.sockets += 1;
    }
    out
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

    #[test]
    fn aggregate_tcp_excludes_listening_backlog_columns() {
        let owned = HashSet::from([1]);
        let entries = [(TcpState::Listen, 5, 128, 1)];
        let out = aggregate_tcp(entries.into_iter(), &owned);
        assert_eq!(out.sockets, 0);
        assert_eq!(out.recv_max, 0);
        assert_eq!(out.send_max, 0);
    }

    #[test]
    fn aggregate_tcp_excludes_unowned_inode() {
        let owned = HashSet::from([1]);
        let entries = [(TcpState::Established, 100, 200, 2)];
        let out = aggregate_tcp(entries.into_iter(), &owned);
        assert_eq!(out.sockets, 0);
    }

    #[test]
    fn aggregate_tcp_computes_max_and_total_across_owned_established() {
        let owned = HashSet::from([1, 2, 3]);
        let entries = [
            (TcpState::Established, 100, 10, 1),
            (TcpState::Established, 400, 5, 2),
            (TcpState::Established, 50, 900, 3),
        ];
        let out = aggregate_tcp(entries.into_iter(), &owned);
        assert_eq!(out.recv_max, 400);
        assert_eq!(out.recv_total, 550);
        assert_eq!(out.send_max, 900);
        assert_eq!(out.send_total, 915);
        assert_eq!(out.sockets, 3);
    }

    #[test]
    fn aggregate_tcp_empty_input_yields_zeros() {
        let out = aggregate_tcp(std::iter::empty(), &HashSet::new());
        assert_eq!(out.recv_max, 0);
        assert_eq!(out.recv_total, 0);
        assert_eq!(out.send_max, 0);
        assert_eq!(out.send_total, 0);
        assert_eq!(out.sockets, 0);
    }
}
