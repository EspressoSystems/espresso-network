//! Periodic CPU usage sampler.
//!
//! Spawns one tokio task that reads `/proc` every `tick`, computes deltas
//! against the previous tick, and writes two CSVs alongside the leader trace:
//!
//!   * `cpu_node{N}.csv`  — per-process + per-thread user/sys microseconds
//!   * `core_node{N}.csv` — per-core user/sys/iowait/idle utilisation (0..1)
//!
//! On non-Linux targets this is a no-op stub so local development on macOS
//! still compiles and runs.

#[cfg(target_os = "linux")]
use std::path::Path;
use std::{path::PathBuf, sync::Arc, time::Duration};

use parking_lot::Mutex;
use serde::Serialize;
#[cfg(target_os = "linux")]
use time::OffsetDateTime;
use tokio::task::JoinHandle;
use tracing::warn;

/// Handle to a running sampler. Holding it does nothing; drop it to leave
/// the sampler running, or call [`CpuSampler::stop`] to flush + join.
pub struct CpuSampler {
    inner: Arc<Inner>,
    join: Option<JoinHandle<()>>,
}

struct Inner {
    node_id: u64,
    out_dir: PathBuf,
    cpu_rows: Mutex<Vec<CpuRow>>,
    core_rows: Mutex<Vec<CoreRow>>,
    net_rows: Mutex<Vec<NetRow>>,
}

#[derive(Serialize)]
struct CpuRow {
    t_ns: i128,
    kind: &'static str, // "proc" | "thread"
    id: i64,            // -1 for proc; tid for thread
    user_us: u64,
    sys_us: u64,
}

#[derive(Serialize)]
struct CoreRow {
    t_ns: i128,
    cpu_id: u32,
    user_pct: f64,
    sys_pct: f64,
    iowait_pct: f64,
    idle_pct: f64,
}

/// Per-interface byte deltas since the previous tick. Loopback (`lo`) is
/// dropped. Rate is derived downstream as
/// `rx_bytes / (t_ns_curr - t_ns_prev_for_same_iface) * 1e9`, so the tick
/// period can be changed without breaking the schema.
#[derive(Serialize)]
struct NetRow {
    t_ns: i128,
    iface: String,
    rx_bytes: u64,
    tx_bytes: u64,
}

impl CpuSampler {
    /// Start sampling. `out_dir` is the directory next to `leader_trace_node*.csv`.
    pub fn start(node_id: u64, out_dir: PathBuf, tick: Duration) -> Self {
        let inner = Arc::new(Inner {
            node_id,
            out_dir,
            cpu_rows: Mutex::new(Vec::with_capacity(4096)),
            core_rows: Mutex::new(Vec::with_capacity(4096)),
            net_rows: Mutex::new(Vec::with_capacity(4096)),
        });

        let join = {
            let inner = inner.clone();
            tokio::spawn(async move {
                run_sampler(inner, tick).await;
            })
        };

        Self {
            inner,
            join: Some(join),
        }
    }

    /// Abort the sampler task and flush whatever is buffered to disk.
    pub async fn stop(mut self) {
        if let Some(h) = self.join.take() {
            h.abort();
            let _ = h.await;
        }
        if let Err(err) = flush(&self.inner) {
            warn!(%err, "failed to flush cpu sampler");
        }
    }
}

#[cfg(target_os = "linux")]
async fn run_sampler(inner: Arc<Inner>, tick: Duration) {
    use std::collections::HashMap;

    let mut prev_proc: Option<(u64, u64)> = None;
    let mut prev_threads: HashMap<i64, (u64, u64)> = HashMap::new();
    let mut prev_cores: HashMap<u32, CoreTicks> = HashMap::new();
    let mut prev_net: HashMap<String, (u64, u64)> = HashMap::new();
    let clk_tck = clk_tck();
    let mut ticker = tokio::time::interval(tick);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;
        let t_ns = OffsetDateTime::now_utc().unix_timestamp_nanos();

        // ---- process-wide /proc/self/stat ----
        if let Some((utime, stime)) = read_self_stat() {
            if let Some((p_u, p_s)) = prev_proc {
                let du = ticks_to_us(utime.saturating_sub(p_u), clk_tck);
                let ds = ticks_to_us(stime.saturating_sub(p_s), clk_tck);
                inner.cpu_rows.lock().push(CpuRow {
                    t_ns,
                    kind: "proc",
                    id: -1,
                    user_us: du,
                    sys_us: ds,
                });
            }
            prev_proc = Some((utime, stime));
        }

        // ---- per-thread /proc/self/task/<tid>/stat ----
        if let Ok(rd) = std::fs::read_dir("/proc/self/task") {
            for ent in rd.flatten() {
                let tid: i64 = match ent.file_name().to_str().and_then(|s| s.parse().ok()) {
                    Some(v) => v,
                    None => continue,
                };
                let path = ent.path().join("stat");
                if let Some((utime, stime)) = read_stat_file(&path) {
                    if let Some(&(p_u, p_s)) = prev_threads.get(&tid) {
                        let du = ticks_to_us(utime.saturating_sub(p_u), clk_tck);
                        let ds = ticks_to_us(stime.saturating_sub(p_s), clk_tck);
                        if du + ds > 0 {
                            inner.cpu_rows.lock().push(CpuRow {
                                t_ns,
                                kind: "thread",
                                id: tid,
                                user_us: du,
                                sys_us: ds,
                            });
                        }
                    }
                    prev_threads.insert(tid, (utime, stime));
                }
            }
        }

        // ---- per-core /proc/stat ----
        if let Some(cores) = read_proc_stat_cores() {
            for (cpu_id, ticks) in cores {
                let prev = prev_cores.get(&cpu_id).copied();
                if let Some(p) = prev {
                    let dt = ticks.total().saturating_sub(p.total());
                    if dt > 0 {
                        let user = (ticks.user.saturating_sub(p.user)) as f64 / dt as f64;
                        let sys_ = (ticks.system.saturating_sub(p.system)) as f64 / dt as f64;
                        let iow = (ticks.iowait.saturating_sub(p.iowait)) as f64 / dt as f64;
                        let idl = (ticks.idle.saturating_sub(p.idle)) as f64 / dt as f64;
                        inner.core_rows.lock().push(CoreRow {
                            t_ns,
                            cpu_id,
                            user_pct: user,
                            sys_pct: sys_,
                            iowait_pct: iow,
                            idle_pct: idl,
                        });
                    }
                }
                prev_cores.insert(cpu_id, ticks);
            }
        }

        // ---- per-interface /proc/net/dev ----
        if let Some(ifaces) = read_proc_net_dev() {
            for (iface, rx, tx) in ifaces {
                if let Some(&(prev_rx, prev_tx)) = prev_net.get(&iface) {
                    let drx = rx.saturating_sub(prev_rx);
                    let dtx = tx.saturating_sub(prev_tx);
                    if drx > 0 || dtx > 0 {
                        inner.net_rows.lock().push(NetRow {
                            t_ns,
                            iface: iface.clone(),
                            rx_bytes: drx,
                            tx_bytes: dtx,
                        });
                    }
                }
                prev_net.insert(iface, (rx, tx));
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
async fn run_sampler(_inner: Arc<Inner>, _tick: Duration) {
    // On non-Linux we don't sample. The task will be aborted on shutdown.
    std::future::pending::<()>().await;
}

fn flush(inner: &Inner) -> std::io::Result<()> {
    std::fs::create_dir_all(&inner.out_dir).ok();

    let cpu_rows = std::mem::take(&mut *inner.cpu_rows.lock());
    if !cpu_rows.is_empty() {
        let path = inner.out_dir.join(format!("cpu_node{}.csv", inner.node_id));
        let mut wtr = csv::Writer::from_path(&path)?;
        for r in cpu_rows {
            wtr.serialize(r)
                .map_err(|e| std::io::Error::other(e.to_string()))?;
        }
        wtr.flush()?;
    }

    let core_rows = std::mem::take(&mut *inner.core_rows.lock());
    if !core_rows.is_empty() {
        let path = inner
            .out_dir
            .join(format!("core_node{}.csv", inner.node_id));
        let mut wtr = csv::Writer::from_path(&path)?;
        for r in core_rows {
            wtr.serialize(r)
                .map_err(|e| std::io::Error::other(e.to_string()))?;
        }
        wtr.flush()?;
    }

    let net_rows = std::mem::take(&mut *inner.net_rows.lock());
    if !net_rows.is_empty() {
        let path = inner.out_dir.join(format!("net_node{}.csv", inner.node_id));
        let mut wtr = csv::Writer::from_path(&path)?;
        for r in net_rows {
            wtr.serialize(r)
                .map_err(|e| std::io::Error::other(e.to_string()))?;
        }
        wtr.flush()?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Default)]
struct CoreTicks {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

#[cfg(target_os = "linux")]
impl CoreTicks {
    fn total(&self) -> u64 {
        self.user
            + self.nice
            + self.system
            + self.idle
            + self.iowait
            + self.irq
            + self.softirq
            + self.steal
    }
}

#[cfg(target_os = "linux")]
fn read_self_stat() -> Option<(u64, u64)> {
    read_stat_file(Path::new("/proc/self/stat"))
}

#[cfg(target_os = "linux")]
fn read_stat_file(path: &Path) -> Option<(u64, u64)> {
    let s = std::fs::read_to_string(path).ok()?;
    // The comm field (field 2) can contain spaces and parens, so split around
    // the last ')'. Fields after that are space-separated.
    let close = s.rfind(')')?;
    let tail = &s[close + 1..];
    let fields: Vec<&str> = tail.split_whitespace().collect();
    // After comm, fields shift: index 0 here = original field 3 (state).
    // utime is original field 14 = index 11 here; stime is field 15 = index 12.
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some((utime, stime))
}

#[cfg(target_os = "linux")]
fn read_proc_stat_cores() -> Option<Vec<(u32, CoreTicks)>> {
    let s = std::fs::read_to_string("/proc/stat").ok()?;
    let mut out = Vec::new();
    for line in s.lines() {
        if !line.starts_with("cpu") || line.starts_with("cpu ") {
            continue;
        }
        let mut it = line.split_whitespace();
        let label = it.next()?;
        let cpu_id: u32 = label.strip_prefix("cpu")?.parse().ok()?;
        let nums: Vec<u64> = it.filter_map(|f| f.parse().ok()).collect();
        if nums.len() < 8 {
            continue;
        }
        out.push((
            cpu_id,
            CoreTicks {
                user: nums[0],
                nice: nums[1],
                system: nums[2],
                idle: nums[3],
                iowait: nums[4],
                irq: nums[5],
                softirq: nums[6],
                steal: nums[7],
            },
        ));
    }
    Some(out)
}

/// Read `/proc/net/dev` and return cumulative `(iface, rx_bytes, tx_bytes)`
/// for every non-loopback interface. Each line looks like
///
/// ```text
///   ens5: 1234567890 9876543 0 0 ...  87654321098 1234567 ...
/// ```
///
/// where the first field after the colon is `rx_bytes` and the 9th is
/// `tx_bytes` (Linux man-page order).
#[cfg(target_os = "linux")]
fn read_proc_net_dev() -> Option<Vec<(String, u64, u64)>> {
    let s = std::fs::read_to_string("/proc/net/dev").ok()?;
    let mut out = Vec::new();
    for line in s.lines().skip(2) {
        let (name, rest) = line.split_once(':')?;
        let iface = name.trim();
        if iface == "lo" || iface.is_empty() {
            continue;
        }
        let nums: Vec<u64> = rest
            .split_whitespace()
            .filter_map(|f| f.parse().ok())
            .collect();
        if nums.len() < 9 {
            continue;
        }
        out.push((iface.to_string(), nums[0], nums[8]));
    }
    Some(out)
}

#[cfg(target_os = "linux")]
fn clk_tck() -> u64 {
    // SAFETY: sysconf with a valid name is always safe.
    let v = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if v > 0 { v as u64 } else { 100 }
}

#[cfg(target_os = "linux")]
fn ticks_to_us(ticks: u64, clk_tck: u64) -> u64 {
    ticks.saturating_mul(1_000_000) / clk_tck.max(1)
}

#[cfg(target_os = "linux")]
fn parse_stat(s: &str) -> Option<(u64, u64)> {
    // Same logic as read_stat_file, factored for testing.
    let close = s.rfind(')')?;
    let tail = &s[close + 1..];
    let fields: Vec<&str> = tail.split_whitespace().collect();
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some((utime, stime))
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::*;

    #[test]
    fn parses_stat_with_spaces_in_comm() {
        // Real-world example: comm can contain spaces, parens, etc.
        // Fields after comm: state ppid pgrp session tty_nr tpgid flags
        //                    minflt cminflt majflt cmajflt UTIME STIME ...
        let line = "1234 ((my proc (foo))) S 1 1 1 0 -1 4194304 100 200 0 0 1500 750 0 0 20 0 8 0 \
                    1234 0 0";
        let (u, s) = parse_stat(line).expect("parse");
        assert_eq!(u, 1500);
        assert_eq!(s, 750);
    }

    #[test]
    fn parses_real_self_stat() {
        let s = std::fs::read_to_string("/proc/self/stat").expect("read");
        let _ = parse_stat(&s).expect("parse self");
    }

    #[test]
    fn reads_real_proc_net_dev() {
        let ifs = read_proc_net_dev().expect("proc/net/dev present");
        // At least one non-loopback interface should be present on any
        // realistic Linux build host. We don't assert *which* one.
        assert!(!ifs.is_empty(), "expected at least one non-lo interface");
        for (iface, _rx, _tx) in &ifs {
            assert_ne!(iface, "lo");
        }
    }
}
