use std::{
    collections::{BTreeMap, BTreeSet},
    mem,
    sync::Arc,
    time::Duration,
};

use bytes::Bytes;
use parking_lot::Mutex;
use tokio::time::Instant;

use crate::{
    Config, RetryPolicy,
    msg::{MsgId, Slot},
};

#[derive(Clone, Debug)]
pub struct DelayQueue {
    conf: Arc<Config>,
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug)]
struct Inner {
    map: BTreeMap<(Slot, MsgId), Entry>,
    due: BTreeSet<(Instant, Slot, MsgId)>,
}

#[derive(Debug)]
struct Entry {
    msg: Bytes,
    pol: RetryPolicy,
    num: usize,
    due: Instant,
}

impl DelayQueue {
    pub fn new(conf: Arc<Config>) -> Self {
        Self {
            conf,
            inner: Arc::new(Mutex::new(Inner {
                map: BTreeMap::new(),
                due: BTreeSet::new(),
            })),
        }
    }

    pub fn add(&self, s: Slot, i: MsgId, msg: Bytes, pol: RetryPolicy, now: Instant) {
        let num = 0;
        let due = timeout(&self.conf, now, num);
        let mut inner = self.inner.lock();
        inner.map.insert((s, i), Entry { msg, pol, num, due });
        inner.due.insert((due, s, i));
    }

    pub fn due(&self, now: Instant) -> Option<(Bytes, RetryPolicy)> {
        let mut inner = self.inner.lock();
        while let Some(&(due, slot, id)) = inner.due.first() {
            if due > now {
                return None;
            }
            let _ = inner.due.pop_first();
            let Some(entry) = inner.map.get_mut(&(slot, id)) else {
                continue;
            };
            entry.num = entry.num.saturating_add(1);
            let t = timeout(&self.conf, now, entry.num);
            entry.due = t;
            let item = (entry.msg.clone(), entry.pol);
            inner.due.insert((t, slot, id));
            return Some(item);
        }
        None
    }

    pub fn remove(&self, s: Slot, i: MsgId) {
        let mut inner = self.inner.lock();
        if let Some(entry) = inner.map.remove(&(s, i)) {
            inner.due.remove(&(entry.due, s, i));
        }
    }

    pub fn reset(&self, now: Instant) {
        let mut inner = self.inner.lock();
        inner.due.clear();
        let mut due = mem::take(&mut inner.due);
        for (&(slot, id), entry) in &mut inner.map {
            entry.num = 0;
            entry.due = now;
            due.insert((now, slot, id));
        }
        inner.due = due
    }

    pub fn gc(&self, s: Slot) {
        let mut inner = self.inner.lock();
        inner.map = inner.map.split_off(&(s, MsgId(0)));
    }

    pub fn len(&self) -> usize {
        self.inner.lock().map.len()
    }

    pub fn is_due(&self, now: Instant) -> bool {
        let inner = self.inner.lock();
        let Some(&(due, ..)) = inner.due.first() else {
            return false;
        };
        due <= now
    }
}

fn timeout(cfg: &Config, now: Instant, at: usize) -> Instant {
    let d = *cfg
        .send_retry_delays
        .get(at)
        .unwrap_or_else(|| cfg.send_retry_delays.last());
    now + Duration::from_secs(d.into())
}
