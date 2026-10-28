use std::{collections::BTreeMap, sync::Arc};

use parking_lot::Mutex;
use tokio::sync::Notify;

use crate::msg::{MsgId, Slot};

#[derive(Debug)]
pub struct Queue<T>(Arc<Inner<T>>);

#[derive(Debug)]
struct Inner<T> {
    sig: Notify,
    map: Mutex<BTreeMap<(Slot, MsgId), T>>,
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for Queue<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Self(Arc::new(Inner {
            sig: Notify::new(),
            map: Mutex::new(BTreeMap::new()),
        }))
    }

    pub fn enqueue(&self, s: Slot, i: MsgId, val: T) {
        self.0.map.lock().insert((s, i), val);
        self.0.sig.notify_waiters();
    }

    pub fn gc(&self, s: Slot) {
        let mut map = self.0.map.lock();
        *map = map.split_off(&(s, MsgId(0)))
    }

    pub fn remove(&self, s: Slot, i: MsgId) {
        self.0.map.lock().remove(&(s, i));
    }

    pub fn len(&self) -> usize {
        self.0.map.lock().len()
    }
}

impl<T: Clone> Queue<T> {
    pub fn try_next(&self) -> Option<(Slot, MsgId, T)> {
        let map = self.0.map.lock();
        let (&(s, i), v) = map.first_key_value()?;
        Some((s, i, v.clone()))
    }

    pub async fn next(&self) -> (Slot, MsgId, T) {
        loop {
            let future = self.0.sig.notified();
            if let Some(v) = self.try_next() {
                return v;
            }
            future.await;
        }
    }
}
