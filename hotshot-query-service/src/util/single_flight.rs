// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

//! Coalesce concurrent identical requests so only one underlying call runs at a time.
//!
//! Multiple callers asking for the same `key` share one `Shared` future and one result.
//! Once the in-flight future resolves and the owning caller returns, the entry is
//! cleared — subsequent calls re-run the work (this is not a result cache).

use std::{
    collections::HashMap,
    future::Future,
    hash::Hash,
    sync::{Arc, Mutex},
};

use futures::future::{BoxFuture, FutureExt, Shared};

type SharedFut<V> = Shared<BoxFuture<'static, V>>;

pub struct SingleFlight<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    in_flight: Arc<Mutex<HashMap<K, SharedFut<V>>>>,
}

impl<K, V> Default for SingleFlight<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    fn default() -> Self {
        Self {
            in_flight: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl<K, V> Clone for SingleFlight<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    fn clone(&self) -> Self {
        Self {
            in_flight: self.in_flight.clone(),
        }
    }
}

impl<K, V> std::fmt::Debug for SingleFlight<K, V>
where
    K: Eq + Hash + std::fmt::Debug,
    V: Clone,
{
    /// Lists the keys currently in flight. Uses `try_lock` and falls back to `<locked>` rather
    /// than blocking: `Debug` may be called from a logging or panic path, and the lock is not
    /// reentrant, so we must never risk a deadlock here.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("SingleFlight");
        match self.in_flight.try_lock() {
            Ok(map) => dbg.field("in_flight", &map.keys().collect::<Vec<_>>()),
            Err(_) => dbg.field("in_flight", &"<locked>"),
        };
        dbg.finish()
    }
}

impl<K, V> SingleFlight<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    pub fn new() -> Self {
        Self::default()
    }
}

impl<K, V> SingleFlight<K, V>
where
    K: Eq + Hash + Clone + Send + Sync + std::fmt::Debug + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Run `f` to produce a value for `key`, deduping concurrent calls.
    ///
    /// If another caller is already running for the same key, this awaits the same
    /// shared future instead of starting a new one. The owning caller (the one that
    /// inserted the future) removes the entry from the in-flight table on completion
    /// or drop, so a serial repeat after the in-flight finishes will rerun `f`.
    ///
    /// Locking: the map lock is held only for the lookup/insert in the block below and is
    /// **released before the `.await`** — it is never held across the wait, so concurrent
    /// callers for *other* keys are not blocked while one key's work is in flight. `f()` is
    /// invoked under the lock, but only to *construct* the future (it is not polled there), so
    /// `f` itself must be cheap and non-blocking; the actual work runs after the lock is dropped.
    pub async fn run<F, Fut>(&self, key: K, f: F) -> V
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = V> + Send + 'static,
    {
        let (shared, guard) = {
            // `lock()` only returns `Err` if the mutex was poisoned (a thread panicked while
            // holding it). The critical section runs no user code (the future is constructed but
            // not polled here), so poisoning is effectively unreachable; even if it happened the
            // map is still structurally valid, so we recover the guard rather than propagate.
            let mut map = self.in_flight.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(existing) = map.get(&key) {
                tracing::debug!(?key, "single_flight: coalescing concurrent request");
                (existing.clone(), None)
            } else {
                let fut: BoxFuture<'static, V> = f().boxed();
                let shared: SharedFut<V> = fut.shared();
                map.insert(key.clone(), shared.clone());
                let guard = RemoveOnDrop {
                    map: self.in_flight.clone(),
                    key: Some(key),
                };
                (shared, Some(guard))
            }
        };

        let value = shared.await;
        drop(guard);
        value
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.in_flight
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }
}

/// Removes the key from the in-flight map when the owning future is dropped or returns.
///
/// Only the caller that inserted the entry holds a `RemoveOnDrop`; late joiners get `None`.
/// This means the entry is cleared as soon as the owner finishes (or is canceled), even
/// if late joiners are still awaiting — those joiners keep driving the `Shared` future
/// to completion via their own clones.
struct RemoveOnDrop<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    map: Arc<Mutex<HashMap<K, SharedFut<V>>>>,
    key: Option<K>,
}

impl<K, V> Drop for RemoveOnDrop<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    fn drop(&mut self) {
        if let Some(key) = self.key.take() {
            // Recover the guard on poison (see `run`): skipping the removal would leak the entry,
            // leaving the key permanently marked in-flight and wedging all future coalescing for it.
            let mut map = self.map.lock().unwrap_or_else(|e| e.into_inner());
            map.remove(&key);
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };

    use tokio::time::sleep;

    use super::*;

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn coalesces_concurrent_same_key() {
        let flight: SingleFlight<u32, u32> = SingleFlight::new();
        let calls = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for _ in 0..10 {
            let flight = flight.clone();
            let calls = calls.clone();
            handles.push(tokio::spawn(async move {
                flight
                    .run(42, move || {
                        let calls = calls.clone();
                        async move {
                            calls.fetch_add(1, Ordering::SeqCst);
                            sleep(Duration::from_millis(50)).await;
                            7u32
                        }
                    })
                    .await
            }));
        }

        for h in handles {
            assert_eq!(h.await.unwrap(), 7);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1, "underlying f ran more than once");
        assert_eq!(flight.len(), 0, "entry not cleared after completion");
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn different_keys_run_independently() {
        let flight: SingleFlight<u32, u32> = SingleFlight::new();
        let calls = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for k in 0..5u32 {
            let flight = flight.clone();
            let calls = calls.clone();
            handles.push(tokio::spawn(async move {
                flight
                    .run(k, move || {
                        let calls = calls.clone();
                        async move {
                            calls.fetch_add(1, Ordering::SeqCst);
                            sleep(Duration::from_millis(20)).await;
                            k * 2
                        }
                    })
                    .await
            }));
        }

        for (k, h) in handles.into_iter().enumerate() {
            assert_eq!(h.await.unwrap(), (k as u32) * 2);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 5);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn serial_repeat_reruns_after_completion() {
        let flight: SingleFlight<u32, u32> = SingleFlight::new();
        let calls = Arc::new(AtomicUsize::new(0));

        for _ in 0..3 {
            let calls = calls.clone();
            let v = flight
                .run(1, move || {
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        100u32
                    }
                })
                .await;
            assert_eq!(v, 100);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 3, "serial calls should rerun f");
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn errors_propagate_to_all_waiters_without_poisoning() {
        let flight: SingleFlight<u32, Result<u32, String>> = SingleFlight::new();
        let calls = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for _ in 0..5 {
            let flight = flight.clone();
            let calls = calls.clone();
            handles.push(tokio::spawn(async move {
                flight
                    .run(7, move || {
                        let calls = calls.clone();
                        async move {
                            calls.fetch_add(1, Ordering::SeqCst);
                            sleep(Duration::from_millis(30)).await;
                            Err::<u32, _>("boom".to_string())
                        }
                    })
                    .await
            }));
        }

        for h in handles {
            assert_eq!(h.await.unwrap(), Err("boom".to_string()));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // After the error in-flight completes, the next call should re-run f.
        let v = flight
            .run(7, || async { Ok::<u32, String>(99) })
            .await;
        assert_eq!(v, Ok(99));
        assert_eq!(calls.load(Ordering::SeqCst), 1, "second key has its own f");
    }
}
