use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Duration,
};

use tokio::time::interval;
use tracing::info;

pub const SUMMARY_INTERVAL: Duration = Duration::from_secs(60);

macro_rules! events {
    ($($variant:ident => $name:literal),* $(,)?) => {
        #[derive(Clone, Copy)]
        #[repr(usize)]
        pub enum LogEvent {
            $($variant),*
        }

        const NAMES: &[&str] = &[$($name),*];
        static COUNTERS: [AtomicU64; NAMES.len()] =
            [const { AtomicU64::new(0) }; NAMES.len()];
    };
}

events! {
    AuthFailure => "auth_failures",
    AuthHandshakeTimeout => "auth_handshake_timeouts",
    DhtClosestPeersFailure => "dht_closest_peers_failures",
    DhtDisagreementGivenUp => "dht_disagreements_given_up",
    DhtKadQueryError => "dht_kad_query_errors",
    DhtLookupFailure => "dht_lookup_failures",
    DialFailure => "dial_failures",
    DirectMessageInboundFailure => "direct_message_inbound_failures",
    DirectMessageOutboundFailure => "direct_message_outbound_failures",
    GossipPublishFailure => "gossip_publish_failures",
    GossipsubNotSupported => "gossipsub_not_supported",
    GossipsubSlowPeer => "gossipsub_slow_peer",
    IncomingConnError => "incoming_conn_errors",
    ListenerError => "listener_errors",
    NetworkSendFailure => "network_send_failures",
    VerifyFailure => "verify_failures",
}

impl LogEvent {
    pub fn record(self) {
        COUNTERS[self as usize].fetch_add(1, Ordering::Relaxed);
    }
}

fn drain_and_format() -> Option<String> {
    let parts: Vec<String> = COUNTERS
        .iter()
        .zip(NAMES.iter())
        .filter_map(|(counter, name)| {
            let value = counter.swap(0, Ordering::Relaxed);
            (value != 0).then(|| format!("{name}={value}"))
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
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
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

    use super::{COUNTERS, LogEvent, drain_and_format, emit_summary};

    fn test_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }

    fn reset() {
        for c in COUNTERS.iter() {
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

        for _ in 0..3 {
            LogEvent::DialFailure.record();
        }
        for _ in 0..5 {
            LogEvent::AuthFailure.record();
        }
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
                        LogEvent::DialFailure.record();
                    }
                });
            }
        });
        let line = drain_and_format().expect("expected a summary");
        assert!(line.contains(&format!("dial_failures={}", THREADS as u64 * PER_THREAD)));
    }
}
