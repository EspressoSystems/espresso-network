//! IO boundary of the watch loops: the clock and HTTP to the nodes.

#[cfg(test)]
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(test)]
use chrono::TimeDelta;
use chrono::{DateTime, SubsecRound, Utc};
use tokio::time::Instant;

use crate::node::NodeHttp;

/// Wall time, monotonic time and sleeping; the only reader of the system clock and the only
/// sleeper.
pub trait Clock: Clone + Send + Sync + 'static {
    /// Wall time at whole seconds, the precision of persisted times.
    fn now(&self) -> DateTime<Utc>;
    /// Monotonic time for pacing; `tokio::time::Instant` as at HEAD.
    fn instant(&self) -> Instant;
    /// Waits `duration`.
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send;
}

/// The process clock: `Utc::now()` truncated to seconds, tokio time.
#[derive(Clone, Copy, Debug)]
pub struct WallClock;

impl Clock for WallClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now().trunc_subsecs(0)
    }

    fn instant(&self) -> Instant {
        Instant::now()
    }

    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}

/// Test clock: time moves only by `advance` and `sleep`, which advances instead of waiting.
/// Concurrent sleeps add up.
#[cfg(test)]
#[derive(Clone, Debug)]
pub struct ManualClock {
    wall_start: DateTime<Utc>,
    instant_start: Instant,
    elapsed: Arc<Mutex<Duration>>,
}

#[cfg(test)]
impl ManualClock {
    /// Starts at `wall_start` and the current tokio instant.
    pub fn new(wall_start: DateTime<Utc>) -> Self {
        Self {
            wall_start,
            instant_start: Instant::now(),
            elapsed: Arc::default(),
        }
    }

    /// Moves wall and monotonic time forward by `by`.
    pub fn advance(&self, by: Duration) {
        *self.elapsed.lock().unwrap() += by;
    }
}

#[cfg(test)]
impl Clock for ManualClock {
    fn now(&self) -> DateTime<Utc> {
        let elapsed = *self.elapsed.lock().unwrap();
        (self.wall_start + TimeDelta::from_std(elapsed).expect("test durations are small"))
            .trunc_subsecs(0)
    }

    fn instant(&self) -> Instant {
        self.instant_start + *self.elapsed.lock().unwrap()
    }

    async fn sleep(&self, duration: Duration) {
        self.advance(duration);
        tokio::task::yield_now().await;
    }
}

/// IO boundary of the watch loops: time and HTTP to the nodes. One value per process; tests use
/// `ManualClock` and a fake axum node.
#[derive(Clone)]
pub struct System<C> {
    pub clock: C,
    pub http: NodeHttp,
}
