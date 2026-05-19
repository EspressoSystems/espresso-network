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
//! counter here. A background task emits a single `warn!` event every
//! `SUMMARY_INTERVAL`, with each counter as a named tracing field so Datadog
//! and other structured backends can index and filter by counter directly.
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

/// Drained values for all counters at one point in time. Field names match
/// the counter names; structured backends index them as named attributes.
#[derive(Default, Debug)]
struct Snapshot {
    auth_failures: u64,
    auth_handshake_timeouts: u64,
    dht_closest_peers_failures: u64,
    dht_disagreements_given_up: u64,
    dht_kad_query_errors: u64,
    dht_lookup_failures: u64,
    dial_failures: u64,
    direct_message_inbound_failures: u64,
    direct_message_outbound_failures: u64,
    gossip_publish_failures: u64,
    gossipsub_not_supported: u64,
    gossipsub_slow_peer: u64,
    incoming_conn_errors: u64,
    listener_errors: u64,
    network_send_failures: u64,
    verify_failures: u64,
}

impl Snapshot {
    /// Drain every counter to zero and return their values.
    fn drain() -> Self {
        Self {
            auth_failures: AUTH_FAILURES.swap(0, Ordering::Relaxed),
            auth_handshake_timeouts: AUTH_HANDSHAKE_TIMEOUTS.swap(0, Ordering::Relaxed),
            dht_closest_peers_failures: DHT_CLOSEST_PEERS_FAILURES.swap(0, Ordering::Relaxed),
            dht_disagreements_given_up: DHT_DISAGREEMENTS_GIVEN_UP.swap(0, Ordering::Relaxed),
            dht_kad_query_errors: DHT_KAD_QUERY_ERRORS.swap(0, Ordering::Relaxed),
            dht_lookup_failures: DHT_LOOKUP_FAILURES.swap(0, Ordering::Relaxed),
            dial_failures: DIAL_FAILURES.swap(0, Ordering::Relaxed),
            direct_message_inbound_failures: DIRECT_MESSAGE_INBOUND_FAILURES
                .swap(0, Ordering::Relaxed),
            direct_message_outbound_failures: DIRECT_MESSAGE_OUTBOUND_FAILURES
                .swap(0, Ordering::Relaxed),
            gossip_publish_failures: GOSSIP_PUBLISH_FAILURES.swap(0, Ordering::Relaxed),
            gossipsub_not_supported: GOSSIPSUB_NOT_SUPPORTED.swap(0, Ordering::Relaxed),
            gossipsub_slow_peer: GOSSIPSUB_SLOW_PEER.swap(0, Ordering::Relaxed),
            incoming_conn_errors: INCOMING_CONN_ERRORS.swap(0, Ordering::Relaxed),
            listener_errors: LISTENER_ERRORS.swap(0, Ordering::Relaxed),
            network_send_failures: NETWORK_SEND_FAILURES.swap(0, Ordering::Relaxed),
            verify_failures: VERIFY_FAILURES.swap(0, Ordering::Relaxed),
        }
    }

    fn any_nonzero(&self) -> bool {
        let Self {
            auth_failures,
            auth_handshake_timeouts,
            dht_closest_peers_failures,
            dht_disagreements_given_up,
            dht_kad_query_errors,
            dht_lookup_failures,
            dial_failures,
            direct_message_inbound_failures,
            direct_message_outbound_failures,
            gossip_publish_failures,
            gossipsub_not_supported,
            gossipsub_slow_peer,
            incoming_conn_errors,
            listener_errors,
            network_send_failures,
            verify_failures,
        } = self;
        *auth_failures
            | *auth_handshake_timeouts
            | *dht_closest_peers_failures
            | *dht_disagreements_given_up
            | *dht_kad_query_errors
            | *dht_lookup_failures
            | *dial_failures
            | *direct_message_inbound_failures
            | *direct_message_outbound_failures
            | *gossip_publish_failures
            | *gossipsub_not_supported
            | *gossipsub_slow_peer
            | *incoming_conn_errors
            | *listener_errors
            | *network_send_failures
            | *verify_failures
            != 0
    }
}

fn emit_summary() {
    let s = Snapshot::drain();
    if !s.any_nonzero() {
        return;
    }
    warn!(
        auth_failures = s.auth_failures,
        auth_handshake_timeouts = s.auth_handshake_timeouts,
        dht_closest_peers_failures = s.dht_closest_peers_failures,
        dht_disagreements_given_up = s.dht_disagreements_given_up,
        dht_kad_query_errors = s.dht_kad_query_errors,
        dht_lookup_failures = s.dht_lookup_failures,
        dial_failures = s.dial_failures,
        direct_message_inbound_failures = s.direct_message_inbound_failures,
        direct_message_outbound_failures = s.direct_message_outbound_failures,
        gossip_publish_failures = s.gossip_publish_failures,
        gossipsub_not_supported = s.gossipsub_not_supported,
        gossipsub_slow_peer = s.gossipsub_slow_peer,
        incoming_conn_errors = s.incoming_conn_errors,
        listener_errors = s.listener_errors,
        network_send_failures = s.network_send_failures,
        verify_failures = s.verify_failures,
        interval_seconds = SUMMARY_INTERVAL.as_secs(),
        "libp2p summary"
    );
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
        AUTH_FAILURES, DIAL_FAILURES, SPAWNED, Snapshot, emit_summary, spawn_summary_task,
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

    /// Reset every counter to zero by draining and discarding.
    fn reset_all_counters() {
        let _ = Snapshot::drain();
    }

    #[test]
    #[traced_test]
    // TEST:log-summary-emits-when-counters-nonzero-ok (REQ:log-summary-emits-when-counters-nonzero)
    fn emits_named_fields_when_any_counter_nonzero() {
        let _g = test_lock();
        reset_all_counters();
        DIAL_FAILURES.store(3, Ordering::Relaxed);
        AUTH_FAILURES.store(5, Ordering::Relaxed);
        emit_summary();
        assert!(logs_contain("libp2p summary"));
        assert!(logs_contain("dial_failures=3"));
        assert!(logs_contain("auth_failures=5"));
        // Zero-valued counters must still be present as fields so structured
        // backends can filter on them.
        assert!(logs_contain("verify_failures=0"));
        assert!(logs_contain("interval_seconds=60"));
    }

    #[test]
    #[traced_test]
    // TEST:log-summary-silent-when-idle-ok (REQ:log-summary-silent-when-idle)
    fn silent_when_all_counters_zero() {
        let _g = test_lock();
        reset_all_counters();
        emit_summary();
        assert!(!logs_contain("libp2p summary"));
    }

    #[test]
    // TEST:log-summary-counters-reset-each-tick-ok (REQ:log-summary-counters-reset-each-tick)
    fn counters_reset_after_drain() {
        let _g = test_lock();
        reset_all_counters();
        AUTH_FAILURES.store(7, Ordering::Relaxed);
        let snapshot_a = Snapshot::drain();
        assert_eq!(snapshot_a.auth_failures, 7);
        let snapshot_b = Snapshot::drain();
        assert_eq!(snapshot_b.auth_failures, 0);
        assert!(!snapshot_b.any_nonzero());
    }

    #[test]
    // TEST:log-summary-counter-overflow-ok (EDGE:log-summary-counter-overflow)
    fn counter_at_max_wraps_without_panic() {
        let _g = test_lock();
        reset_all_counters();
        DIAL_FAILURES.store(u64::MAX, Ordering::Relaxed);
        // Wrapping fetch_add on u64 is defined (wraps to 0). We just want to
        // confirm no panic and the drain returns whatever the swap saw.
        DIAL_FAILURES.fetch_add(1, Ordering::Relaxed);
        let snapshot = Snapshot::drain();
        assert_eq!(snapshot.dial_failures, 0);
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
        let snapshot = Snapshot::drain();
        assert_eq!(snapshot.dial_failures, THREADS as u64 * PER_THREAD);
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
