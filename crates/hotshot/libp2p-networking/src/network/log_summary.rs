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
//! counter here. A background task emits a single `warn!` summary line every
//! `SUMMARY_INTERVAL`, listing only non-zero counters.
//!
//! Counters are process-global because the summary itself is process-global;
//! threading an `Arc` through every libp2p call site would be far more
//! invasive than the alternative.

use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Duration,
};

use tokio::time::interval;
use tracing::warn;

/// How often the summary task wakes up and emits an aggregated line.
pub const SUMMARY_INTERVAL: Duration = Duration::from_secs(60);

// --- Counters -------------------------------------------------------------

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

/// All counters, sorted alphabetically by name for stable summary output.
const COUNTERS: &[(&str, &AtomicU64)] = &[
    ("auth_failures", &AUTH_FAILURES),
    ("auth_handshake_timeouts", &AUTH_HANDSHAKE_TIMEOUTS),
    ("dht_closest_peers_failures", &DHT_CLOSEST_PEERS_FAILURES),
    ("dht_disagreements_given_up", &DHT_DISAGREEMENTS_GIVEN_UP),
    ("dht_kad_query_errors", &DHT_KAD_QUERY_ERRORS),
    ("dht_lookup_failures", &DHT_LOOKUP_FAILURES),
    (
        "direct_message_inbound_failures",
        &DIRECT_MESSAGE_INBOUND_FAILURES,
    ),
    (
        "direct_message_outbound_failures",
        &DIRECT_MESSAGE_OUTBOUND_FAILURES,
    ),
    ("dial_failures", &DIAL_FAILURES),
    ("gossip_publish_failures", &GOSSIP_PUBLISH_FAILURES),
    ("gossipsub_not_supported", &GOSSIPSUB_NOT_SUPPORTED),
    ("gossipsub_slow_peer", &GOSSIPSUB_SLOW_PEER),
    ("incoming_conn_errors", &INCOMING_CONN_ERRORS),
    ("listener_errors", &LISTENER_ERRORS),
    ("network_send_failures", &NETWORK_SEND_FAILURES),
    ("verify_failures", &VERIFY_FAILURES),
];

/// Atomically read each counter and reset it to zero, returning `(name, value)`
/// pairs in `COUNTERS` order (alphabetical).
fn snapshot_and_reset() -> Vec<(&'static str, u64)> {
    COUNTERS
        .iter()
        .map(|(name, counter)| (*name, counter.swap(0, Ordering::Relaxed)))
        .collect()
}

/// Format a snapshot into a single summary line. Returns `None` if every
/// counter is zero. Output is `libp2p last 60s: name1=v1 name2=v2 ...` with
/// only non-zero counters, in input order (alphabetical by counter name).
fn format_summary(snapshot: &[(&str, u64)]) -> Option<String> {
    let parts: Vec<String> = snapshot
        .iter()
        .filter(|(_, value)| *value != 0)
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(format!(
            "libp2p last {}s: {}",
            SUMMARY_INTERVAL.as_secs(),
            parts.join(" ")
        ))
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
            let snapshot = snapshot_and_reset();
            if let Some(line) = format_summary(&snapshot) {
                warn!("{line}");
            }
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

    use super::{
        AUTH_FAILURES, COUNTERS, DHT_LOOKUP_FAILURES, DIAL_FAILURES, SPAWNED, format_summary,
        snapshot_and_reset, spawn_summary_task,
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

    /// Reset every counter to zero. Tests share process-global state, so we
    /// always reset before doing anything observable.
    fn reset_all_counters() {
        for (_, counter) in COUNTERS {
            counter.store(0, Ordering::Relaxed);
        }
    }

    #[test]
    // TEST:log-summary-emits-when-counters-nonzero-ok (REQ:log-summary-emits-when-counters-nonzero)
    fn emits_summary_when_any_counter_nonzero() {
        let _guard = test_lock();
        reset_all_counters();
        DIAL_FAILURES.store(3, Ordering::Relaxed);
        let snapshot = snapshot_and_reset();
        let line = format_summary(&snapshot).expect("expected a summary line");
        assert!(line.contains("dial_failures=3"), "got: {line}");
        assert!(line.starts_with("libp2p last 60s:"), "got: {line}");
    }

    #[test]
    // TEST:log-summary-silent-when-idle-ok (REQ:log-summary-silent-when-idle)
    fn silent_when_all_counters_zero() {
        let _guard = test_lock();
        reset_all_counters();
        let snapshot = snapshot_and_reset();
        assert!(format_summary(&snapshot).is_none());
    }

    #[test]
    // TEST:log-summary-counters-reset-each-tick-ok (REQ:log-summary-counters-reset-each-tick)
    fn counters_reset_after_snapshot() {
        let _guard = test_lock();
        reset_all_counters();
        AUTH_FAILURES.store(7, Ordering::Relaxed);
        let snapshot_a = snapshot_and_reset();
        assert!(
            snapshot_a
                .iter()
                .any(|(name, value)| *name == "auth_failures" && *value == 7)
        );
        let snapshot_b = snapshot_and_reset();
        assert!(snapshot_b.iter().all(|(_, value)| *value == 0));
    }

    #[test]
    // TEST:log-summary-format-stable-ok (REQ:log-summary-format-stable)
    fn format_lists_only_nonzero_counters_in_alphabetical_order() {
        let _guard = test_lock();
        reset_all_counters();
        // Pick a non-alphabetically-first counter and a later one to verify
        // ordering follows counter name, not insertion order.
        DHT_LOOKUP_FAILURES.store(2, Ordering::Relaxed);
        AUTH_FAILURES.store(5, Ordering::Relaxed);
        let snapshot = snapshot_and_reset();
        let line = format_summary(&snapshot).expect("expected a summary line");
        let body = line
            .strip_prefix("libp2p last 60s: ")
            .expect("unexpected prefix");
        assert_eq!(body, "auth_failures=5 dht_lookup_failures=2", "got: {line}");
    }

    #[test]
    // TEST:log-summary-counter-overflow-ok (EDGE:log-summary-counter-overflow)
    fn counter_at_max_wraps_without_panic() {
        let _guard = test_lock();
        reset_all_counters();
        DIAL_FAILURES.store(u64::MAX, Ordering::Relaxed);
        // Wrapping fetch_add on u64 is defined (wraps to 0). We just want to
        // confirm no panic and the snapshot returns whatever the swap saw.
        DIAL_FAILURES.fetch_add(1, Ordering::Relaxed);
        let snapshot = snapshot_and_reset();
        let value = snapshot
            .iter()
            .find_map(|(name, value)| (*name == "dial_failures").then_some(*value))
            .expect("dial_failures missing");
        // After overflow the value is 0; assert behavior matches the swap.
        assert_eq!(value, 0);
    }

    #[test]
    // TEST:log-summary-concurrent-increment-ok (EDGE:log-summary-concurrent-increment)
    fn concurrent_increments_are_not_lost() {
        let _guard = test_lock();
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
        let snapshot = snapshot_and_reset();
        let value = snapshot
            .iter()
            .find_map(|(name, value)| (*name == "dial_failures").then_some(*value))
            .expect("dial_failures missing");
        assert_eq!(value, THREADS as u64 * PER_THREAD);
    }

    #[test]
    // TEST:log-summary-spawn-idempotent-ok (EDGE:log-summary-spawn-once)
    fn spawn_summary_task_is_idempotent() {
        let _guard = test_lock();
        // The SPAWNED flag is process-global. Reset it here so this test is
        // independent of test ordering. We do not actually need the task to
        // run, only to verify the spawn gate.
        SPAWNED.store(false, Ordering::Release);
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
        // Reset for any later tests that touch the flag.
        SPAWNED.store(false, Ordering::Release);
    }
}
