use std::{
    pin::Pin,
    task::{Context, Poll, ready},
    time::Duration,
};

use hotshot_types::data::{EpochNumber, ViewNumber};
use tokio::time::{Instant, Sleep, sleep};

/// A view timeout that fires once per reset.
///
/// Awaiting it resolves when the current view's timeout elapses. After that it
/// stays pending until one of the `reset` methods arms it again, so it is meant
/// to be driven by a loop that resets it on every firing and on every view
/// change, which is what [`Coordinator::next_consensus_input`] does. Awaiting a
/// fired `Timer` on its own, with nothing else to wake the task, therefore waits
/// forever: the reset that would arm it needs `&mut`, which a task parked on the
/// timer cannot take.
///
/// [`Coordinator::next_consensus_input`]: crate::coordinator::Coordinator::next_consensus_input
pub struct Timer {
    sleep: Pin<Box<Sleep>>,
    view: ViewNumber,
    epoch: EpochNumber,
    duration: Duration,
    /// Whether the current arming has already fired. Without it the elapsed
    /// `Sleep` would report ready on every poll and spin the loop that selects
    /// over this timer.
    done: bool,
}

impl Timer {
    pub fn new(d: Duration, v: ViewNumber, e: EpochNumber) -> Self {
        Self {
            sleep: Box::pin(sleep(d)),
            view: v,
            epoch: e,
            duration: d,
            done: false,
        }
    }

    pub fn view(&self) -> ViewNumber {
        self.view
    }

    pub fn epoch(&self) -> EpochNumber {
        self.epoch
    }

    pub fn reset(&mut self) {
        self.done = false;
        self.sleep.as_mut().reset(Instant::now() + self.duration);
    }

    pub fn reset_with_epoch(&mut self, v: ViewNumber, e: EpochNumber) {
        self.view = v;
        self.epoch = e;
        self.done = false;
        self.sleep.as_mut().reset(Instant::now() + self.duration);
    }
}

impl Future for Timer {
    type Output = ();

    /// Pending once this arming has fired, and deliberately without registering
    /// the waker: nothing can wake the task, because arming again takes `&mut`.
    /// A caller that polls a fired timer must be driven by something else - a
    /// `select!` over other branches, as in the coordinator - and must reset it.
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.done {
            return Poll::Pending;
        }
        ready!(self.sleep.as_mut().poll(cx));
        self.done = true;
        Poll::Ready(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::timeout;

    use super::*;

    const PERIOD: Duration = Duration::from_millis(20);
    /// Long enough that a timer which should fire has fired, short enough that
    /// a test observing "still pending" does not stall the suite.
    const OBSERVE: Duration = Duration::from_millis(200);

    fn timer() -> Timer {
        Timer::new(PERIOD, ViewNumber::new(1), EpochNumber::new(1))
    }

    /// One firing per arming: it fires, then stays pending, then fires again
    /// once reset. The middle assertion is what the `done` flag buys - without
    /// it the elapsed `Sleep` would report ready forever and spin the caller.
    #[tokio::test]
    async fn fires_once_per_arming() {
        let mut t = timer();
        timeout(OBSERVE, &mut t).await.expect("timer should fire");

        assert!(
            timeout(OBSERVE, &mut t).await.is_err(),
            "a fired timer must stay pending until it is reset"
        );

        t.reset();
        timeout(OBSERVE, &mut t)
            .await
            .expect("reset timer should fire");
    }

    /// Resetting carries the view and epoch the next firing reports.
    #[tokio::test]
    async fn reset_carries_view_and_epoch() {
        let mut t = timer();
        t.reset_with_epoch(ViewNumber::new(7), EpochNumber::new(3));
        assert_eq!(t.view(), ViewNumber::new(7));
        assert_eq!(t.epoch(), EpochNumber::new(3));
        timeout(OBSERVE, &mut t).await.expect("timer should fire");
        assert_eq!(t.view(), ViewNumber::new(7));
        assert_eq!(t.epoch(), EpochNumber::new(3));
    }
}
