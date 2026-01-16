use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, LazyLock},
    task::{Context, Poll, Waker},
};

use parking_lot::Mutex;
use tokio::time::{sleep, Duration, Instant, Sleep};

/// An unspecified epoch for use with `Timestamp`.
static EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);

/// Time measured as duration in µs since an unspecified epoch.
#[derive(Debug, Copy, Clone)]
pub struct Timestamp(u64);

impl Timestamp {
    pub fn now() -> Self {
        Self(Instant::now().saturating_duration_since(*EPOCH).as_micros() as u64)
    }

    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        Self(u64::from_be_bytes(bytes))
    }

    pub fn to_bytes(self) -> [u8; 8] {
        self.0.to_be_bytes()
    }

    pub fn try_from_slice(b: &[u8]) -> Option<Self> {
        let bytes = b.try_into().ok()?;
        Some(Self::from_bytes(bytes))
    }

    #[allow(unused)]
    pub fn diff(self, other: Self) -> Option<Duration> {
        self.0.checked_sub(other.0).map(Duration::from_micros)
    }
}

/// A countdown timer that can be reset.
#[derive(Debug, Clone)]
pub struct Countdown {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug)]
struct Inner {
    // The actual future to await.
    sleep: Option<Pin<Box<Sleep>>>,

    // Is this countdown running?
    //
    // We could utilise the `sleep` `Option` for same purpose, and arguably
    // it would be cleaner to use `Some` as the running state, and `None` as
    // the opposite. However we would like to avoid the allocation every time
    // the countdown is (re-)started, hence this flag.
    stopped: bool,

    /// Waker to call when a stopped `Countdown` should be polled again.
    waker: Option<Waker>,
}

impl Default for Countdown {
    fn default() -> Self {
        Self::new()
    }
}

impl Countdown {
    /// Create a new countdown.
    ///
    /// When ready, use `Countdown::start` to begin.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                sleep: None,
                stopped: true,
                waker: None,
            })),
        }
    }

    /// Start the countdown.
    ///
    /// Once started, a countdown can not be started again, unless
    /// `Countdown::stop` is invoked first.
    pub fn start(&self, timeout: Duration) {
        let mut inner = self.inner.lock();
        if !inner.stopped {
            // The countdown is already running.
            return;
        }
        inner.stopped = false;
        if let Some(sleep) = &mut inner.sleep {
            sleep.as_mut().reset(Instant::now() + timeout)
        } else {
            inner.sleep = Some(Box::pin(sleep(timeout)))
        }
        if let Some(w) = inner.waker.take() {
            w.wake()
        }
    }

    /// Stop this countdown.
    pub fn stop(&self) {
        self.inner.lock().stopped = true
    }
}

impl Future for Countdown {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut inner = self.inner.lock();
        if inner.stopped {
            if let Some(w) = inner.waker.as_mut() {
                // Update existing waker:
                w.clone_from(cx.waker())
            } else {
                inner.waker = Some(cx.waker().clone())
            }
            return Poll::Pending;
        }
        debug_assert!(inner.waker.is_none());
        let sleep = inner.sleep.as_mut().expect("!stopped => sleep future");
        sleep.as_mut().poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use tokio::time::{sleep, timeout, Duration, Instant};

    use super::{Countdown, Timestamp};

    #[tokio::test]
    async fn duration() {
        let d = Duration::from_millis(50);
        let a = Timestamp::now();
        sleep(d).await;
        let b = Timestamp::now();
        let x = b.diff(a).unwrap();
        assert!(x - d < Duration::from_millis(5))
    }

    #[tokio::test]
    async fn countdown() {
        let mut c = Countdown::new();

        let now = Instant::now();
        c.start(Duration::from_secs(1));
        (&mut c).await;
        assert!(now.elapsed() >= Duration::from_secs(1));

        // Once finished, the countdown stays finished:
        let now = Instant::now();
        (&mut c).await;
        assert!(now.elapsed() < Duration::from_millis(1));

        // If stopped it does not end:
        c.start(Duration::from_secs(1));
        c.stop();
        assert!(timeout(Duration::from_secs(2), &mut c).await.is_err());

        // until started again:
        c.start(Duration::from_secs(1));
        let now = Instant::now();
        (&mut c).await;
        assert!(now.elapsed() >= Duration::from_secs(1));
    }
}
