use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Duration,
};

use tokio::time::interval;
use tracing::info;

pub const SUMMARY_INTERVAL: Duration = Duration::from_secs(60);

macro_rules! counters {
    ($($name:ident),* $(,)?) => {
        $(pub static $name: AtomicU64 = AtomicU64::new(0);)*

        const COUNTERS: &[(&str, &AtomicU64)] = &[
            $((stringify!($name), &$name),)*
        ];
    };
}

counters! {
    AUTH_FAILURES,
    AUTH_HANDSHAKE_TIMEOUTS,
    DHT_CLOSEST_PEERS_FAILURES,
    DHT_DISAGREEMENTS_GIVEN_UP,
    DHT_KAD_QUERY_ERRORS,
    DHT_LOOKUP_FAILURES,
    DIAL_FAILURES,
    DIRECT_MESSAGE_INBOUND_FAILURES,
    DIRECT_MESSAGE_OUTBOUND_FAILURES,
    GOSSIP_PUBLISH_FAILURES,
    GOSSIPSUB_NOT_SUPPORTED,
    GOSSIPSUB_SLOW_PEER,
    INCOMING_CONN_ERRORS,
    LISTENER_ERRORS,
    NETWORK_SEND_FAILURES,
    VERIFY_FAILURES,
}

fn drain_and_format() -> Option<String> {
    let parts: Vec<String> = COUNTERS
        .iter()
        .filter_map(|(name, counter)| {
            let value = counter.swap(0, Ordering::Relaxed);
            (value != 0).then(|| format!("{}={value}", name.to_ascii_lowercase()))
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join(" "))
}

fn emit_summary() {
    if let Some(body) = drain_and_format() {
        info!("libp2p {}s summary: {body}", SUMMARY_INTERVAL.as_secs());
    }
}

static SPAWNED: AtomicBool = AtomicBool::new(false);

pub fn spawn_summary_task() {
    if SPAWNED
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        return;
    }
    tokio::spawn(async move {
        let mut ticker = interval(SUMMARY_INTERVAL);
        ticker.tick().await; // skip immediate first tick
        loop {
            ticker.tick().await;
            emit_summary();
        }
    });
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard, OnceLock, atomic::Ordering};

    use tracing_test::traced_test;

    use super::{AUTH_FAILURES, COUNTERS, DIAL_FAILURES, drain_and_format, emit_summary};

    fn test_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }

    fn reset() {
        for (_, c) in COUNTERS {
            c.store(0, Ordering::Relaxed);
        }
    }

    #[test]
    #[traced_test]
    fn emits_only_nonzero_and_skips_when_idle() {
        let _g = test_lock();
        reset();
        emit_summary();
        assert!(!logs_contain("libp2p"));

        DIAL_FAILURES.store(3, Ordering::Relaxed);
        AUTH_FAILURES.store(5, Ordering::Relaxed);
        emit_summary();
        assert!(logs_contain("libp2p 60s summary:"));
        assert!(logs_contain("auth_failures=5"));
        assert!(logs_contain("dial_failures=3"));
        assert!(!logs_contain("verify_failures"));
        assert!(drain_and_format().is_none());
    }

    #[test]
    fn concurrent_increments_are_not_lost() {
        let _g = test_lock();
        reset();
        const THREADS: usize = 8;
        const PER_THREAD: u64 = 1_000;
        std::thread::scope(|s| {
            for _ in 0..THREADS {
                s.spawn(|| {
                    for _ in 0..PER_THREAD {
                        DIAL_FAILURES.fetch_add(1, Ordering::Relaxed);
                    }
                });
            }
        });
        let line = drain_and_format().expect("expected a summary");
        assert!(line.contains(&format!("dial_failures={}", THREADS as u64 * PER_THREAD)));
    }
}
