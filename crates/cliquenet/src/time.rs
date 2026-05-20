use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};

use parking_lot::Mutex;
use tokio::time::{Duration, Instant, Sleep, sleep};

/// A countdown timer that can be reset.
#[derive(Debug, Clone)]
pub struct Countdown {
    inner: Arc<Mutex<Inner>>,
}
#[derive(Debug)]
struct Inner {
    // The actual future to await.
    sleep: Pin<Box<Sleep>>,

    // Is this countdown running?
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
                sleep: Box::pin(sleep(Duration::from_secs(1))),
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
        // Take the waker and drop the lock before calling `wake`
        // to avoid holding it across scheduling.
        let waker = {
            let mut inner = self.inner.lock();
            if !inner.stopped {
                // The countdown is already running.
                return;
            }
            inner.stopped = false;
            inner.sleep.as_mut().reset(Instant::now() + timeout);
            inner.waker.take()
        };
        if let Some(w) = waker {
            w.wake();
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
                if !w.will_wake(cx.waker()) {
                    w.clone_from(cx.waker())
                }
            } else {
                inner.waker = Some(cx.waker().clone())
            }
            return Poll::Pending;
        }
        debug_assert!(inner.waker.is_none());
        inner.sleep.as_mut().poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use tokio::time::{Duration, Instant, timeout};

    use super::Countdown;

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
