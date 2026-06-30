use std::{
    cmp::Reverse,
    collections::{BTreeMap, BinaryHeap},
    mem,
    sync::Arc,
    time::Duration,
};

use bytes::Bytes;
use parking_lot::Mutex;
use tokio::{sync::Notify, time::Instant};

use crate::{
    Config, RetryPolicy,
    msg::{MsgId, Slot},
};

/// Message queue with retry support.
#[derive(Clone, Debug)]
pub struct Queue {
    conf: Arc<Config>,
    inner: Arc<Inner>,
}

/// Message queue item.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Item {
    pub data: Bytes,
    pub policy: RetryPolicy,
}

#[derive(Debug, Default)]
struct Inner {
    notify: Notify,
    messages: Mutex<Messages>,
}

#[derive(Debug, Default)]
struct Messages {
    /// Messages enqueued by a producer.
    map: BTreeMap<(Slot, MsgId), (Bytes, RetryPolicy)>,
    /// Dequeued messages awaiting an ACK.
    ack: BTreeMap<(Slot, MsgId), (Bytes, RetryPolicy, usize)>,
    /// Min-heap of messages that need to be resent.
    due: BinaryHeap<Reverse<(Instant, Slot, MsgId)>>,
}

impl Queue {
    pub fn new(conf: Arc<Config>) -> Self {
        Self {
            conf,
            inner: Arc::new(Inner::default()),
        }
    }

    /// Enqueue the given message plus its metadata.
    pub fn enqueue(&self, s: Slot, i: MsgId, msg: Bytes, pol: RetryPolicy) {
        self.inner.messages.lock().map.insert((s, i), (msg, pol));
        self.inner.notify.notify_waiters();
    }

    /// Get the next message (if any).
    ///
    /// If the enqueued item's retry policy demands an ACK we insert a copy
    /// into a separate collection where it is removed from, once the ACK has
    /// arrived. Until then it will be resent after some delay.
    pub fn try_dequeue(&self) -> Option<Item> {
        let now = Instant::now();
        let mut messages = self.inner.messages.lock();
        let ((s, i), (m, p)) = messages.map.pop_first()?;
        if p.is_retry() {
            let t = timeout(&self.conf, now, 0);
            messages.ack.insert((s, i), (m.clone(), p, 0));
            messages.due.push(Reverse((t, s, i)));
        }
        Some(Item { data: m, policy: p })
    }

    /// Await the next enqued item.
    ///
    /// Apart from asynchronously awaiting the next item, this method behaves
    /// exactly like `[Self::try_dequeue]`.
    pub async fn dequeue(&self) -> Item {
        loop {
            let future = self.inner.notify.notified();
            if let Some(v) = self.try_dequeue() {
                return v;
            }
            future.await;
        }
    }

    /// Get the next item (if any) that should be resent.
    ///
    /// This gets the first item due, and schedules it for retrying again after
    /// some more time.
    pub fn next_retry(&self, now: Instant) -> Option<Item> {
        let mut messages = self.inner.messages.lock();
        loop {
            let Reverse((t, ..)) = messages.due.peek()?;
            if *t > now {
                return None;
            }
            let Reverse((_, s, i)) = messages.due.pop()?;
            if let Some((bytes, policy, retry)) = messages.ack.get_mut(&(s, i)) {
                *retry = retry.saturating_add(1);
                let t = timeout(&self.conf, now, *retry);
                let m = bytes.clone();
                let p = *policy;
                messages.due.push(Reverse((t, s, i)));
                return Some(Item { data: m, policy: p });
            }
        }
    }

    /// Remove an item from the retry collection.
    ///
    /// This is done after some confirmation has been received that the item
    /// has been received.
    pub fn stop_retry(&self, s: Slot, i: MsgId) {
        let mut messages = self.inner.messages.lock();
        messages.ack.remove(&(s, i));
    }

    /// Reset the schedules for retrying messages.
    pub fn reset_retry(&self) {
        let now = Instant::now();
        let mut messages = self.inner.messages.lock();
        messages.due.clear();
        let mut due = mem::take(&mut messages.due);
        for ((s, i), (_, _, retry)) in &mut messages.ack {
            *retry = 0;
            due.push(Reverse((now, *s, *i)))
        }
        messages.due = due;
    }

    /// Drop all messages below the given slot.
    pub fn gc(&self, s: Slot) {
        let mut messages = self.inner.messages.lock();
        messages.map = messages.map.split_off(&(s, MsgId(0)));
        messages.ack = messages.ack.split_off(&(s, MsgId(0)))
    }

    /// The number of enqueued messages minus the ones scheduled for retry.
    pub fn len_messages(&self) -> usize {
        self.inner.messages.lock().map.len()
    }

    /// The number of messages scheduled for retry.
    pub fn len_retry(&self) -> usize {
        self.inner.messages.lock().ack.len()
    }

    /// Is any message scheduled for retry due?
    pub fn is_due(&self, now: Instant) -> bool {
        let messages = self.inner.messages.lock();
        let Some(Reverse((t, ..))) = messages.due.peek() else {
            return false;
        };
        *t <= now
    }
}

fn timeout(cfg: &Config, now: Instant, at: usize) -> Instant {
    let d = *cfg
        .send_retry_delays
        .get(at)
        .unwrap_or_else(|| cfg.send_retry_delays.last());
    now + Duration::from_secs(d.into())
}
