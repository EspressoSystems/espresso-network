use std::{
    collections::BTreeMap,
    convert::Infallible,
    fmt::{self, Display},
    hash::Hash,
    io::Cursor,
    ops::Deref,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use bytes::{Bytes, BytesMut};
use nohash_hasher::IntMap;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;
use tokio::{
    spawn,
    sync::mpsc::{Sender, error::TrySendError},
    task::JoinHandle,
    time::{self, Duration, Instant},
};
use tracing::warn;

use crate::{Address, Id, Network, PublicKey, Role, net::Command};

type Result<T> = std::result::Result<T, NetworkDown>;

/// Max. bucket number.
pub const MAX_BUCKET: Bucket = Bucket(u64::MAX);

/// `Retry` wraps a [`Network`] and returns acknowledgements to senders.
///
/// It also retries messages until either an acknowledgement has been received
/// or client code has indicated that the messages are no longer of interest
/// by invoking `Retry::gc`.
///
/// Each message that is sent has a trailer appended that contains the bucket
/// number and ID of the message. Receivers will send this trailer back. The
/// sender then stops retrying the corresponding message.
///
/// Note that if malicious parties modify the trailer and have it point to a
/// different message, they can only remove themselves from the set of parties
/// the sender is expecting an acknowledgement from.
#[derive(Debug, Clone)]
pub struct Retry<K> {
    inner: Arc<Inner<K>>,
}

#[derive(Debug)]
struct Inner<K> {
    this: K,
    net: Network<K>,
    sender: Sender<Command<K>>,
    id: AtomicU64,
    buffer: Buffer<K>,
    retry: JoinHandle<Infallible>,
    pending: Mutex<BTreeMap<Trailer, Pending<K>>>,
}

impl<K> Drop for Retry<K> {
    fn drop(&mut self) {
        self.inner.retry.abort()
    }
}

/// Data wraps some length-checked, tagged bytes.
///
/// This exists to allow clients to construct a message item that will
/// not be rejected by the network due to size violations.
#[derive(Debug, Clone)]
pub struct Data {
    bytes: BytesMut,
}

/// Buckets conceptionally contain messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Bucket(u64);

/// Messages are associated with IDs and put into buckets.
///
/// Bucket numbers are given to us by clients which also garbage collect
/// explicitly by specifying the bucket up to which to remove messages.
/// Buckets often correspond to rounds elsewhere.
#[derive(Debug, Clone)]
#[allow(clippy::type_complexity)]
struct Buffer<K>(Arc<Mutex<BTreeMap<Bucket, IntMap<Id, Message<K>>>>>);

impl<K> Default for Buffer<K> {
    fn default() -> Self {
        Self(Default::default())
    }
}

#[derive(Debug)]
struct Message<K> {
    /// The message bytes to (re-)send.
    data: Bytes,
    /// The time we started sending this message.
    time: Instant,
    /// The number of times we have sent this message.
    retries: usize,
    /// The remaining number of parties that have to acknowledge the message.
    remaining: Vec<K>,
}

/// Meta information appended at the end of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct Trailer {
    /// The bucket number the message corresponds to.
    bucket: Bucket,
    /// The message ID.
    id: Id,
}

/// Data we have received but could not acknowledge yet.
#[derive(Debug)]
struct Pending<K> {
    src: K,
    data: Bytes,
    trailer: Bytes,
}

enum Target<K> {
    Single(K),
    Multi(Vec<K>),
    All,
}

impl<K> Retry<K>
where
    K: Serialize + DeserializeOwned + Eq + Ord + Send + Clone + Display + Hash + 'static,
{
    pub fn new(net: Network<K>) -> Self {
        let buffer = Buffer::default();
        let retry = spawn(retry(buffer.clone(), net.sender()));
        Self {
            inner: Arc::new(Inner {
                this: net.public_key().clone(),
                sender: net.sender(),
                net,
                buffer,
                id: AtomicU64::new(0),
                retry,
                pending: Mutex::new(BTreeMap::new()),
            }),
        }
    }

    pub async fn broadcast<B>(&self, b: B, data: Data) -> Result<Id>
    where
        B: Into<Bucket>,
    {
        self.send(b.into(), Target::All, data).await
    }

    pub async fn multicast<B>(&self, to: Vec<K>, b: B, data: Data) -> Result<Id>
    where
        B: Into<Bucket>,
    {
        self.send(b.into(), Target::Multi(to), data).await
    }

    pub async fn unicast<B>(&self, to: K, b: B, data: Data) -> Result<Id>
    where
        B: Into<Bucket>,
    {
        self.send(b.into(), Target::Single(to), data).await
    }

    pub async fn add(&self, peers: Vec<(K, PublicKey, Address)>) -> Result<()> {
        self.inner.net.add(peers).await.map_err(|_| NetworkDown(()))
    }

    pub async fn remove(&self, peers: Vec<K>) -> Result<()> {
        self.inner
            .net
            .remove(peers)
            .await
            .map_err(|_| NetworkDown(()))
    }

    pub async fn assign(&mut self, r: Role, peers: Vec<K>) -> Result<()> {
        self.inner
            .net
            .assign(r, peers)
            .await
            .map_err(|_| NetworkDown(()))
    }

    pub async fn receive(&self) -> Result<(K, Bytes)> {
        let pending = self.inner.pending.lock().pop_first();
        if let Some((_, Pending { src, data, trailer })) = pending {
            self.inner
                .sender
                .send(Command::Unicast(src.clone(), None, trailer.clone()))
                .await
                .map_err(|_| NetworkDown(()))?;
            return Ok((src, data));
        }
        loop {
            let (src, mut bytes) = self
                .inner
                .net
                .receive()
                .await
                .map_err(|_| NetworkDown(()))?;

            let Some(trailer_bytes) = Trailer::split_off(&mut bytes) else {
                warn!(node = %self.inner.this, "invalid trailer bytes");
                continue;
            };

            let trailer: Trailer = match bincode::deserialize(&trailer_bytes) {
                Ok(t) => t,
                Err(e) => {
                    warn!(node = %self.inner.this, err = %e, "invalid trailer");
                    continue;
                },
            };

            if !bytes.is_empty() {
                // Send the trailer back as acknowledgement:
                match self
                    .inner
                    .sender
                    .try_send(Command::Unicast(src.clone(), None, trailer_bytes))
                {
                    Ok(()) => return Ok((src, bytes)),
                    Err(TrySendError::Closed(_)) => return Err(NetworkDown(())),
                    Err(TrySendError::Full(Command::Unicast(src, _, trailer_bytes))) => {
                        // Save received data for cancellation safety:
                        self.inner.pending.lock().insert(
                            trailer,
                            Pending {
                                src: src.clone(),
                                data: bytes.clone(),
                                trailer: trailer_bytes.clone(),
                            },
                        );
                        self.inner
                            .sender
                            .send(Command::Unicast(src.clone(), None, trailer_bytes))
                            .await
                            .map_err(|_| NetworkDown(()))?;
                        self.inner.pending.lock().remove(&trailer);
                        return Ok((src, bytes));
                    },
                    Err(TrySendError::Full(_)) => {
                        unreachable!(
                            "We tried sending a Command::Unicast so this is what we get back."
                        )
                    },
                }
            }

            let mut messages = self.inner.buffer.0.lock();

            if let Some(buckets) = messages.get_mut(&trailer.bucket)
                && let Some(m) = buckets.get_mut(&trailer.id)
            {
                m.remaining.retain(|k| *k != src);
                if m.remaining.is_empty() {
                    buckets.remove(&trailer.id);
                }
            }
        }
    }

    pub fn gc<B: Into<Bucket>>(&mut self, bucket: B) {
        let bucket = bucket.into();
        self.inner.buffer.0.lock().retain(|b, _| *b >= bucket);
    }

    pub fn rm<B: Into<Bucket>>(&mut self, bucket: B, id: Id) {
        let bucket = bucket.into();
        if let Some(messages) = self.inner.buffer.0.lock().get_mut(&bucket) {
            messages.remove(&id);
        }
    }

    async fn send(&self, b: Bucket, to: Target<K>, data: Data) -> Result<Id> {
        let id = self.next_id();

        let trailer = Trailer { bucket: b, id };

        let mut encoded = Cursor::new([0u8; Trailer::MAX_LEN]);
        bincode::serialize_into(&mut encoded, &trailer).expect("trailer encoding never fails");

        let mut msg = data.bytes;

        msg.extend_from_slice(&encoded.get_ref()[..encoded.position() as usize]);
        msg.extend_from_slice(&[encoded.position().try_into().expect("|trailer| <= 32")]);
        let msg = msg.freeze();

        let now = Instant::now();

        let rem = match to {
            Target::Single(to) => {
                self.inner
                    .sender
                    .send(Command::Unicast(to.clone(), Some(id), msg.clone()))
                    .await
                    .map_err(|_| NetworkDown(()))?;
                vec![to]
            },
            Target::Multi(peers) => {
                self.inner
                    .sender
                    .send(Command::Multicast(peers.clone(), Some(id), msg.clone()))
                    .await
                    .map_err(|_| NetworkDown(()))?;
                peers
            },
            Target::All => {
                self.inner
                    .sender
                    .send(Command::Broadcast(Some(id), msg.clone()))
                    .await
                    .map_err(|_| NetworkDown(()))?;
                self.inner.net.parties(Role::Active)
            },
        };

        self.inner.buffer.0.lock().entry(b).or_default().insert(
            id,
            Message {
                data: msg,
                time: now,
                retries: 0,
                remaining: rem,
            },
        );

        Ok(id)
    }

    fn next_id(&self) -> Id {
        Id::from(self.inner.id.fetch_add(1, Ordering::Relaxed))
    }
}

async fn retry<K: Clone>(buf: Buffer<K>, net: Sender<Command<K>>) -> Infallible {
    const DELAYS: [u64; 5] = [1, 1, 3, 5, 15];

    let mut i = time::interval(Duration::from_secs(1));
    i.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    let mut buckets = Vec::new();
    let mut ids = Vec::new();

    loop {
        let now = i.tick().await;

        debug_assert!(buckets.is_empty());
        buckets.extend(buf.0.lock().keys().copied());

        for b in buckets.drain(..) {
            debug_assert!(ids.is_empty());
            ids.extend(
                buf.0
                    .lock()
                    .get(&b)
                    .into_iter()
                    .flat_map(|m| m.keys().copied()),
            );

            for id in ids.drain(..) {
                let message;
                let remaining;

                {
                    let mut buf = buf.0.lock();
                    let Some(m) = buf.get_mut(&b).and_then(|m| m.get_mut(&id)) else {
                        continue;
                    };

                    let delay = DELAYS.get(m.retries).copied().unwrap_or(30);

                    if now.saturating_duration_since(m.time) < Duration::from_secs(delay) {
                        continue;
                    }

                    m.time = now;
                    m.retries = m.retries.saturating_add(1);

                    message = m.data.clone();
                    remaining = m.remaining.clone();
                }

                let _ = net
                    .send(Command::Multicast(remaining, Some(id), message.clone()))
                    .await;
            }
        }
    }
}

#[derive(Debug, Error)]
#[error("network down")]
pub struct NetworkDown(());

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DataError {
    #[error("data size exceeds allowed maximum")]
    MaxSize,
}

impl TryFrom<BytesMut> for Data {
    type Error = DataError;

    fn try_from(val: BytesMut) -> std::result::Result<Self, Self::Error> {
        if val.len() > crate::MAX_MESSAGE_SIZE {
            return Err(DataError::MaxSize);
        }
        Ok(Self { bytes: val })
    }
}

impl Deref for Data {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.bytes.as_ref()
    }
}

impl From<u64> for Bucket {
    fn from(val: u64) -> Self {
        Self(val)
    }
}

impl From<Bucket> for u64 {
    fn from(val: Bucket) -> Self {
        val.0
    }
}

impl fmt::Display for Bucket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Trailer {
    /// Max. byte length of a trailer.
    pub const MAX_LEN: usize = 32;

    fn split_off(bytes: &mut Bytes) -> Option<Bytes> {
        let len = usize::from(*bytes.last()?);

        if bytes.len() < len + 1 {
            return None;
        }

        Some(bytes.split_off(bytes.len() - (len + 1)))
    }
}
