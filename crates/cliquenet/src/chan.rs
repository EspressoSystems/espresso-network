//! A channel implementation that keeps only a single copy of an item, as
//! identified by its Id.

use std::collections::VecDeque;
use std::sync::Arc;

use nohash_hasher::IntSet;
use parking_lot::Mutex;
use tokio::sync::Notify;

use crate::Id;

#[derive(Debug)]
pub struct Sender<T>(Arc<Chan<T>>);

#[derive(Debug)]
pub struct Receiver<T>(Arc<Chan<T>>);

#[derive(Debug)]
struct Chan<T> {
    /// Channel capacity.
    cap: usize,
    /// Notifier for receivers that are waiting for items.
    sig: Notify,
    /// The items currently in flight.
    buf: Mutex<Buf<T>>,
}

#[derive(Debug)]
struct Buf<T> {
    /// Ordered queue of items.
    xs: VecDeque<(Option<Id>, T)>,
    /// The set of Ids in the queue.
    ids: IntSet<Id>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub fn channel<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let chan = Arc::new(Chan {
        cap,
        sig: Notify::new(),
        buf: Mutex::new(Buf {
            xs: VecDeque::new(),
            ids: IntSet::default(),
        }),
    });
    (Sender(chan.clone()), Receiver(chan))
}

impl<T> Sender<T> {
    pub fn send(&self, id: Option<Id>, val: T) {
        if let Some(id) = id {
            let mut buf = self.0.buf.lock();
            if buf.ids.contains(&id) {
                return;
            }
            if buf.xs.len() == self.0.cap {
                if let Some((Some(id), _)) = buf.xs.pop_front() {
                    buf.ids.remove(&id);
                }
            }
            buf.xs.push_back((Some(id), val));
            buf.ids.insert(id);
        } else {
            let mut buf = self.0.buf.lock();
            if buf.xs.len() == self.0.cap {
                if let Some((Some(id), _)) = buf.xs.pop_front() {
                    buf.ids.remove(&id);
                }
            }
            buf.xs.push_back((None, val));
        }
        self.0.sig.notify_waiters();
    }

    pub fn capacity(&self) -> usize {
        self.0.cap
    }
}

impl<T> Receiver<T> {
    pub async fn recv(&self) -> Option<T> {
        loop {
            let future = self.0.sig.notified();
            {
                let mut buf = self.0.buf.lock();
                if let Some((id, val)) = buf.xs.pop_front() {
                    if let Some(id) = id {
                        buf.ids.remove(&id);
                    }
                    return Some(val);
                }
            }
            future.await;
        }
    }
}
