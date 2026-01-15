use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fmt::{self, Display};
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;

use bytes::{Bytes, BytesMut};
use minicbor::encode::write::Cursor;
use minicbor::{Decode, Encode};
use nohash_hasher::IntMap;
use parking_lot::Mutex;
use thiserror::Error;
use tokio::spawn;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use tokio::time::{self, Duration, Instant};
use tracing::warn;

use crate::net::Command;
use crate::{Address, Id, Network, Role};

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
#[derive(Debug)]
pub struct Retry<K> {
    this: K,
    net: Network<K>,
    sender: Sender<Command<K>>,
    id: Id,
    buffer: Buffer<K>,
    encoded: Cursor<[u8; Trailer::MAX_LEN]>,
    retry: JoinHandle<Infallible>,
    pending: Option<Pending<K>>,
}

impl<K> Drop for Retry<K> {
    fn drop(&mut self) {
        self.retry.abort()
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode)]
#[cbor(transparent)]
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
#[derive(Debug, Encode, Decode)]
struct Trailer {
    /// The bucket number the message corresponds to.
    #[n(0)]
    bucket: Bucket,
    /// The message ID.
    #[n(1)]
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
    K: Encode<()>
        + for<'a> Decode<'a, ()>
        + Eq
        + Ord
        + Send
        + Clone
        + Copy
        + Display
        + Hash
        + 'static,
{
    pub fn new(net: Network<K>) -> Self {
        let buffer = Buffer::default();
        let retry = spawn(retry(buffer.clone(), net.sender()));
        Self {
            this: *net.public_key(),
            sender: net.sender(),
            net,
            buffer,
            encoded: Cursor::new([0; Trailer::MAX_LEN]),
            id: Id::from(0),
            retry,
            pending: None,
        }
    }

    pub fn parties(&self) -> impl Iterator<Item = (&K, &Role)> {
        self.net.parties()
    }

    pub async fn broadcast<B>(&mut self, b: B, data: Data) -> Result<Id>
    where
        B: Into<Bucket>,
    {
        self.send(b.into(), Target::All, data).await
    }

    pub async fn multicast<B>(&mut self, to: Vec<K>, b: B, data: Data) -> Result<Id>
    where
        B: Into<Bucket>,
    {
        self.send(b.into(), Target::Multi(to), data).await
    }

    pub async fn unicast<B>(&mut self, to: K, b: B, data: Data) -> Result<Id>
    where
        B: Into<Bucket>,
    {
        self.send(b.into(), Target::Single(to), data).await
    }

    pub async fn add(&mut self, peers: Vec<(K, Address)>) -> Result<()> {
        self.net.add(peers).await.map_err(|_| NetworkDown(()))
    }

    pub async fn remove(&mut self, peers: Vec<K>) -> Result<()> {
        self.net.remove(peers).await.map_err(|_| NetworkDown(()))
    }

    pub async fn assign(&mut self, r: Role, peers: Vec<K>) -> Result<()> {
        self.net.assign(r, peers).await.map_err(|_| NetworkDown(()))
    }

    pub async fn receive(&mut self) -> Result<(K, Bytes)> {
        if let Some(Pending { src, data, trailer }) = &self.pending {
            self.sender
                .send(Command::Unicast(*src, None, trailer.clone()))
                .await
                .map_err(|_| NetworkDown(()))?;
            let src = *src;
            let dat = data.clone();
            self.pending = None;
            return Ok((src, dat));
        }
        loop {
            debug_assert!(self.pending.is_none());

            let (src, mut bytes) = self.net.receive().await.map_err(|_| NetworkDown(()))?;

            let Some(trailer_bytes) = Trailer::split_off(&mut bytes) else {
                warn!(node = %self.this, "invalid trailer bytes");
                continue;
            };

            let trailer: Trailer = match minicbor::decode(&trailer_bytes) {
                Ok(t) => t,
                Err(e) => {
                    warn!(node = %self.this, err = %e, "invalid trailer");
                    continue;
                },
            };

            if !bytes.is_empty() {
                // Send the trailer back as acknowledgement:
                match self
                    .sender
                    .try_send(Command::Unicast(src, None, trailer_bytes))
                {
                    Ok(()) => return Ok((src, bytes)),
                    Err(TrySendError::Closed(_)) => return Err(NetworkDown(())),
                    Err(TrySendError::Full(Command::Unicast(src, _, trailer_bytes))) => {
                        // Save received data for cancellation safety:
                        self.pending = Some(Pending {
                            src,
                            data: bytes.clone(),
                            trailer: trailer_bytes.clone(),
                        });
                        self.sender
                            .send(Command::Unicast(src, None, trailer_bytes))
                            .await
                            .map_err(|_| NetworkDown(()))?;
                        self.pending = None;
                        return Ok((src, bytes));
                    },
                    Err(TrySendError::Full(_)) => {
                        unreachable!(
                            "We tried sending a Command::Unicast so this is what we get back."
                        )
                    },
                }
            }

            let mut messages = self.buffer.0.lock();

            if let Some(buckets) = messages.get_mut(&trailer.bucket) {
                if let Some(m) = buckets.get_mut(&trailer.id) {
                    m.remaining.retain(|k| *k != src);
                    if m.remaining.is_empty() {
                        buckets.remove(&trailer.id);
                    }
                }
            }
        }
    }

    pub fn gc<B: Into<Bucket>>(&mut self, bucket: B) {
        let bucket = bucket.into();
        self.buffer.0.lock().retain(|b, _| *b >= bucket);
    }

    pub fn rm<B: Into<Bucket>>(&mut self, bucket: B, id: Id) {
        let bucket = bucket.into();
        if let Some(messages) = self.buffer.0.lock().get_mut(&bucket) {
            messages.remove(&id);
        }
    }

    async fn send(&mut self, b: Bucket, to: Target<K>, data: Data) -> Result<Id> {
        let id = self.next_id();

        let trailer = Trailer { bucket: b, id };

        self.encoded.set_position(0);
        minicbor::encode(&trailer, &mut self.encoded).expect("trailer encoding never fails");

        let mut msg = data.bytes;

        msg.extend_from_slice(self.encoded.get_ref());
        msg.extend_from_slice(&[self
            .encoded
            .get_ref()
            .len()
            .try_into()
            .expect("|trailer| <= 32")]);
        let msg = msg.freeze();

        let now = Instant::now();

        let rem = match to {
            Target::Single(to) => {
                self.sender
                    .send(Command::Unicast(to, Some(id), msg.clone()))
                    .await
                    .map_err(|_| NetworkDown(()))?;
                vec![to]
            },
            Target::Multi(peers) => {
                self.sender
                    .send(Command::Multicast(peers.clone(), Some(id), msg.clone()))
                    .await
                    .map_err(|_| NetworkDown(()))?;
                peers
            },
            Target::All => {
                self.sender
                    .send(Command::Broadcast(Some(id), msg.clone()))
                    .await
                    .map_err(|_| NetworkDown(()))?;
                self.net
                    .parties()
                    .filter(|(_, r)| r.is_active())
                    .map(|(p, _)| *p)
                    .collect()
            },
        };

        self.buffer.0.lock().entry(b).or_default().insert(
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

    fn next_id(&mut self) -> Id {
        let id = self.id;
        self.id = (u64::from(self.id) + 1).into();
        id
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
