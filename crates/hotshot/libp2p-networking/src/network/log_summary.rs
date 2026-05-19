// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Periodic aggregated summary of suppressed libp2p log noise.
//!
//! Many libp2p event-flow logs are not individually actionable on a public p2p
//! network (dial timeouts, auth handshake failures from random scanners, DHT
//! disagreements, etc.). They have been demoted to `debug!`. To preserve
//! aggregate visibility, sites that demote a log increment a process-global
//! counter here. A background task emits a single compact `info!` line every
//! `SUMMARY_INTERVAL`, listing only the counters that were non-zero. The line
//! is a human-readable heartbeat; chartable/filterable metrics belong in the
//! existing Prometheus `Libp2pMetricsValue` path.
//!
//! Counters are process-global because the summary itself is process-global;
//! threading an `Arc` through every libp2p call site would be far more
//! invasive than the alternative.

use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Duration,
};

use tokio::time::interval;
use tracing::info;

/// How often the summary task wakes up and emits an aggregated event.
pub const SUMMARY_INTERVAL: Duration = Duration::from_secs(60);

pub static DIAL_FAILURES: AtomicU64 = AtomicU64::new(0);
pub static INCOMING_CONN_ERRORS: AtomicU64 = AtomicU64::new(0);
pub static LISTENER_ERRORS: AtomicU64 = AtomicU64::new(0);
pub static GOSSIPSUB_NOT_SUPPORTED: AtomicU64 = AtomicU64::new(0);
pub static GOSSIPSUB_SLOW_PEER: AtomicU64 = AtomicU64::new(0);
pub static AUTH_FAILURES: AtomicU64 = AtomicU64::new(0);
pub static VERIFY_FAILURES: AtomicU64 = AtomicU64::new(0);
pub static AUTH_HANDSHAKE_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
pub static DHT_KAD_QUERY_ERRORS: AtomicU64 = AtomicU64::new(0);
pub static DHT_DISAGREEMENTS_GIVEN_UP: AtomicU64 = AtomicU64::new(0);
pub static DHT_CLOSEST_PEERS_FAILURES: AtomicU64 = AtomicU64::new(0);
pub static DHT_LOOKUP_FAILURES: AtomicU64 = AtomicU64::new(0);
pub static DIRECT_MESSAGE_INBOUND_FAILURES: AtomicU64 = AtomicU64::new(0);
pub static DIRECT_MESSAGE_OUTBOUND_FAILURES: AtomicU64 = AtomicU64::new(0);
pub static GOSSIP_PUBLISH_FAILURES: AtomicU64 = AtomicU64::new(0);
pub static NETWORK_SEND_FAILURES: AtomicU64 = AtomicU64::new(0);

/// All counters, in display order. Used by the summary task to drain and
/// format, and by tests to reset.
const COUNTERS: &[(&str, &AtomicU64)] = &[
    ("auth_failures", &AUTH_FAILURES),
    ("auth_handshake_timeouts", &AUTH_HANDSHAKE_TIMEOUTS),
    ("dht_closest_peers_failures", &DHT_CLOSEST_PEERS_FAILURES),
    ("dht_disagreements_given_up", &DHT_DISAGREEMENTS_GIVEN_UP),
    ("dht_kad_query_errors", &DHT_KAD_QUERY_ERRORS),
    ("dht_lookup_failures", &DHT_LOOKUP_FAILURES),
    ("dial_failures", &DIAL_FAILURES),
    (
        "direct_message_inbound_failures",
        &DIRECT_MESSAGE_INBOUND_FAILURES,
    ),
    (
        "direct_message_outbound_failures",
        &DIRECT_MESSAGE_OUTBOUND_FAILURES,
    ),
    ("gossip_publish_failures", &GOSSIP_PUBLISH_FAILURES),
    ("gossipsub_not_supported", &GOSSIPSUB_NOT_SUPPORTED),
    ("gossipsub_slow_peer", &GOSSIPSUB_SLOW_PEER),
    ("incoming_conn_errors", &INCOMING_CONN_ERRORS),
    ("listener_errors", &LISTENER_ERRORS),
    ("network_send_failures", &NETWORK_SEND_FAILURES),
    ("verify_failures", &VERIFY_FAILURES),
];

/// Drain every counter and format the non-zero ones into `name=value` tokens.
/// Returns `None` if every counter was zero.
fn drain_and_format() -> Option<String> {
    let parts: Vec<String> = COUNTERS
        .iter()
        .filter_map(|(name, counter)| {
            let value = counter.swap(0, Ordering::Relaxed);
            (value != 0).then(|| format!("{name}={value}"))
        })
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn emit_summary() {
    if let Some(body) = drain_and_format() {
        info!("libp2p {}s summary: {body}", SUMMARY_INTERVAL.as_secs());
    }
}

/// Tracks whether the summary task has been spawned. Used to make
/// `spawn_summary_task` idempotent.
static SPAWNED: AtomicBool = AtomicBool::new(false);

/// Spawn the periodic summary task. Idempotent: subsequent calls are no-ops.
///
/// Returns `true` if this call actually spawned the task, `false` if a
/// previous call already did.
pub fn spawn_summary_task() -> bool {
    if SPAWNED
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        return false;
    }
    tokio::spawn(async move {
        let mut ticker = interval(SUMMARY_INTERVAL);
        // First tick fires immediately; skip it so we always wait a full
        // interval before emitting anything.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            emit_summary();
        }
    });
    true
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Mutex, MutexGuard, OnceLock, atomic::Ordering},
        thread,
    };

    use tracing_test::traced_test;

    use super::{
        AUTH_FAILURES, COUNTERS, DIAL_FAILURES, SPAWNED, drain_and_format, emit_summary,
        spawn_summary_task,
    };

    /// Process-wide lock to serialize tests in this module. All test functions
    /// touch process-global counters and the SPAWNED flag, so they cannot run
    /// concurrently without interfering with each other.
    fn test_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        // Recover from a poisoned lock so a previously-panicked test does not
        // poison every subsequent test in this module.
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn reset_all_counters() {
        for (_, counter) in COUNTERS {
            counter.store(0, Ordering::Relaxed);
        }
    }

    #[test]
    #[traced_test]
    // TEST:log-summary-emits-when-counters-nonzero-ok (REQ:log-summary-emits-when-counters-nonzero)
    fn emits_only_nonzero_counters() {
        let _g = test_lock();
        reset_all_counters();
        DIAL_FAILURES.store(3, Ordering::Relaxed);
        AUTH_FAILURES.store(5, Ordering::Relaxed);
        emit_summary();
        assert!(logs_contain("libp2p 60s summary:"));
        assert!(logs_contain("auth_failures=5"));
        assert!(logs_contain("dial_failures=3"));
        // Zero-valued counters must NOT appear in the line.
        assert!(!logs_contain("verify_failures"));
        assert!(!logs_contain("listener_errors"));
    }

    #[test]
    #[traced_test]
    // TEST:log-summary-silent-when-idle-ok (REQ:log-summary-silent-when-idle)
    fn silent_when_all_counters_zero() {
        let _g = test_lock();
        reset_all_counters();
        emit_summary();
        assert!(!logs_contain("libp2p"));
    }

    #[test]
    // TEST:log-summary-counters-reset-each-tick-ok (REQ:log-summary-counters-reset-each-tick)
    fn counters_reset_after_drain() {
        let _g = test_lock();
        reset_all_counters();
        AUTH_FAILURES.store(7, Ordering::Relaxed);
        let first = drain_and_format().expect("expected non-empty summary");
        assert!(first.contains("auth_failures=7"), "got: {first}");
        // Second drain finds everything at zero.
        assert!(drain_and_format().is_none());
    }

    #[test]
    // TEST:log-summary-counter-overflow-ok (EDGE:log-summary-counter-overflow)
    fn counter_at_max_wraps_without_panic() {
        let _g = test_lock();
        reset_all_counters();
        DIAL_FAILURES.store(u64::MAX, Ordering::Relaxed);
        // Wrapping fetch_add on u64 is defined (wraps to 0). We just want to
        // confirm no panic; after the wrap, the counter reads 0 and the
        // summary skips it.
        DIAL_FAILURES.fetch_add(1, Ordering::Relaxed);
        assert!(drain_and_format().is_none());
    }

    #[test]
    // TEST:log-summary-concurrent-increment-ok (EDGE:log-summary-concurrent-increment)
    fn concurrent_increments_are_not_lost() {
        let _g = test_lock();
        reset_all_counters();
        const THREADS: usize = 8;
        const PER_THREAD: u64 = 1_000;
        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                thread::spawn(|| {
                    for _ in 0..PER_THREAD {
                        DIAL_FAILURES.fetch_add(1, Ordering::Relaxed);
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        let line = drain_and_format().expect("expected a summary");
        let expected = format!("dial_failures={}", THREADS as u64 * PER_THREAD);
        assert!(line.contains(&expected), "got: {line}");
    }

    #[test]
    // TEST:log-summary-spawn-idempotent-ok (EDGE:log-summary-spawn-once)
    fn spawn_summary_task_is_idempotent() {
        let _g = test_lock();
        // The SPAWNED flag is process-global. Reset it here so this test is
        // independent of test ordering. We do not actually need the task to
        // run, only to verify the spawn gate.
        SPAWNED.store(false, Ordering::Relaxed);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let first = spawn_summary_task();
            let second = spawn_summary_task();
            let third = spawn_summary_task();
            assert!(first, "first call should spawn");
            assert!(!second, "second call should be a no-op");
            assert!(!third, "third call should be a no-op");
        });
        SPAWNED.store(false, Ordering::Relaxed);
    }
}
