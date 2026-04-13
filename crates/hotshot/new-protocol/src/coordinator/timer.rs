use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use hotshot_types::data::{EpochNumber, ViewNumber};
use tokio::time::{Instant, Sleep, sleep};

pub struct Timer {
    sleep: Pin<Box<Sleep>>,
    view: ViewNumber,
    epoch: EpochNumber,
    duration: Duration,
}

impl Timer {
    pub fn new(d: Duration, v: ViewNumber, e: EpochNumber) -> Self {
        Self {
            sleep: Box::pin(sleep(d)),
            view: v,
            epoch: e,
            duration: d,
        }
    }

    pub fn view(&self) -> ViewNumber {
        self.view
    }

    pub fn epoch(&self) -> EpochNumber {
        self.epoch
    }

    pub fn reset(&mut self) {
        self.sleep.as_mut().reset(Instant::now() + self.duration);
    }

    pub fn reset_with(&mut self, v: ViewNumber) {
        self.view = v;
        self.sleep.as_mut().reset(Instant::now() + self.duration);
    }
    pub fn reset_with_epoch(&mut self, v: ViewNumber, e: EpochNumber) {
        self.view = v;
        self.epoch = e;
        self.sleep.as_mut().reset(Instant::now() + self.duration);
    }
}

impl Future for Timer {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        self.sleep.as_mut().poll(cx)
    }
}
