use std::{collections::BTreeMap, sync::Arc, time::Duration};

use bytes::Bytes;
use tokio::time::Instant;

use crate::{
    Config,
    msg::{MsgId, Slot},
};

/// Number of due items to have available at once.
const READY_BATCH: usize = 32;

pub struct DelayQueue {
    conf: Arc<Config>,
    /// All items that need to be resend at some point.
    items: BTreeMap<(Slot, MsgId), RetryItem>,
    /// Items that are due and should be sent asap.
    ready: BTreeMap<(Slot, MsgId), Bytes>,
}

struct RetryItem {
    data: Bytes,
    /// Next due time.
    timeout: Instant,
    /// The number of times an item has been resent already.
    retries: usize,
}

impl DelayQueue {
    pub fn new(c: Arc<Config>) -> Self {
        Self {
            conf: c,
            items: BTreeMap::new(),
            ready: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn is_ready(&self) -> bool {
        !self.ready.is_empty()
    }

    pub fn add(&mut self, s: Slot, i: MsgId, b: Bytes) {
        let t = timeout(&self.conf, 0);
        let r = RetryItem {
            data: b,
            timeout: t,
            retries: 0,
        };
        self.items.insert((s, i), r);
    }

    pub fn del(&mut self, s: Slot, i: MsgId) {
        self.items.remove(&(s, i));
        self.ready.remove(&(s, i));
    }

    pub fn reset(&mut self) {
        let now = Instant::now();
        for x in self.items.values_mut() {
            x.retries = 0;
            x.timeout = now;
        }
    }

    /// Get any due item to resend.
    pub fn next(&mut self) -> Option<Bytes> {
        self.ready.pop_first().map(|(_, b)| b)
    }

    /// Scan all items to find the next ones that are due.
    ///
    /// Scanning stops when a `READY_BATCH` is full. Since scanning happens
    /// less frequently (about once per second) and is O(n) in the worst case,
    /// we make several items that are due available for quick access.
    pub fn check(&mut self, now: Instant) {
        let mut n = self.ready.len();
        for (k, x) in &mut self.items {
            if n >= READY_BATCH {
                break;
            }
            if x.timeout > now {
                continue;
            }
            x.retries = x.retries.saturating_add(1);
            x.timeout = timeout(&self.conf, x.retries);
            self.ready.entry(*k).or_insert(x.data.clone());
            n += 1
        }
    }

    pub fn gc(&mut self, s: Slot) {
        let key = (s, MsgId(0));
        self.items = self.items.split_off(&key);
        self.ready = self.ready.split_off(&key)
    }
}

fn timeout(conf: &Config, at: usize) -> Instant {
    let d = conf
        .retry_delays
        .get(at)
        .copied()
        .map(|d| Duration::from_secs(d.into()))
        .unwrap_or(conf.max_retry_delay);
    Instant::now() + d
}
