//! The newest payloads written through a store, kept in memory for readers in the same process.
//!
//! A deployment that runs consensus, a storage instance and an API instance in one process writes
//! each payload once and reads it back for every client soon after. Keeping the newest payloads
//! here, as the buffers they were written from, spares those reads, and the wait until a staged
//! payload is uploaded. A payload larger than the whole cache is not kept.

use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
};

use bytes::Bytes;

/// The newest payloads by commitment, up to a total size.
#[derive(Debug)]
pub(crate) struct RecentPayloads {
    limit: usize,
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    bytes: usize,
    by_commitment: HashMap<String, Bytes>,

    /// The commitments in the order their payloads were kept, oldest first.
    order: VecDeque<String>,
}

impl RecentPayloads {
    /// A cache of at most `limit` bytes of payloads.
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            limit,
            inner: Mutex::default(),
        }
    }

    /// Keeps `payload` under `commitment`, forgetting the oldest payloads as needed.
    pub(crate) fn insert(&self, commitment: &str, payload: Bytes) {
        if payload.len() > self.limit {
            return;
        }
        let mut inner = self.inner.lock().expect("the cache is not poisoned");
        if inner.by_commitment.contains_key(commitment) {
            return;
        }
        inner.bytes += payload.len();
        inner.by_commitment.insert(commitment.to_string(), payload);
        inner.order.push_back(commitment.to_string());
        while inner.bytes > self.limit {
            let oldest = inner
                .order
                .pop_front()
                .expect("kept payloads have commitments");
            let forgotten = inner
                .by_commitment
                .remove(&oldest)
                .expect("kept commitments have payloads");
            inner.bytes -= forgotten.len();
        }
    }

    /// The payload kept under `commitment`, sharing its buffer.
    pub(crate) fn get(&self, commitment: &str) -> Option<Bytes> {
        let inner = self.inner.lock().expect("the cache is not poisoned");
        inner.by_commitment.get(commitment).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The oldest payloads go once the total passes the limit; one larger than the limit is
    /// never kept.
    #[test]
    fn keeps_the_newest_payloads_up_to_the_limit() {
        let cache = RecentPayloads::new(10);
        cache.insert("a", Bytes::from_static(b"aaaa"));
        cache.insert("b", Bytes::from_static(b"bbbb"));
        cache.insert("c", Bytes::from_static(b"cccc"));
        assert_eq!(cache.get("a"), None);
        assert_eq!(cache.get("b").as_deref(), Some(&b"bbbb"[..]));
        assert_eq!(cache.get("c").as_deref(), Some(&b"cccc"[..]));

        cache.insert("d", Bytes::from_static(b"ddddddddddd"));
        assert_eq!(cache.get("d"), None);
        assert!(cache.get("c").is_some());
    }
}
