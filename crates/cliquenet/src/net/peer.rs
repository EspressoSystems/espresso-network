mod delay;

#[cfg(test)]
mod tests;

use std::{
    cmp::min,
    future::ready,
    mem,
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use bon::bon;
use bytes::{Bytes, BytesMut};
use delay::DelayQueue;
use snow::StatelessTransportState;
use tokio::{
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    select, spawn,
    sync::{
        Semaphore,
        mpsc::{self, UnboundedReceiver, UnboundedSender, error::TrySendError},
        watch,
    },
    time::{MissedTickBehavior, interval},
};
use tokio_util::{sync::CancellationToken, task::AbortOnDropHandle};
use tracing::{error, trace, warn};

use crate::{
    Config, Metrics, PublicKey,
    connection::{Connection, recv_frame, send_frame},
    error::{Empty, NetworkError},
    msg::{Ack, FrameType, Header, MAX_NOISE_MESSAGE_SIZE, MAX_PAYLOAD_SIZE, Slot, Trailer},
    net::{PeerMessage, RetryPolicy},
    queue::Queue,
    time::Countdown,
};

type NoiseBuf = Box<[u8; MAX_NOISE_MESSAGE_SIZE]>;
type Result<T> = std::result::Result<T, NetworkError>;

/// A peer sends and receives messages over a connection with a remote.
///
/// Messages sent are expected to be acknowledged, i.e. the remote needs to
/// send back an ACK frame. Otherwise the message is resent after some time.
pub struct Peer {
    /// Network configuration.
    conf: Arc<Config>,

    /// A budget limits how many message a peer can deliver to the application.
    budget: Budget,

    /// Messages the application wants to be sent to the remote.
    msgs: Queue<(RetryPolicy, Bytes)>,

    /// Messages waiting to be retried if no ACK has been received.
    retry: DelayQueue,

    /// Receive notifications about changes to the GC threshold
    next_slot: watch::Receiver<Slot>,

    /// Our current GC threshold (slot number).
    lower_bound: Arc<AtomicU64>,

    /// The channel over which to deliver inbound messages to the application.
    tx: UnboundedSender<PeerMessage>,

    /// The true max. message size.
    ///
    /// It accounts for the additional `Trailer` bytes.
    max_message_size: usize,

    metrics: Arc<dyn Metrics>,
}

/// A budget limits how many messages a peer can delivery to the application.
#[derive(Clone)]
pub struct Budget(Arc<Semaphore>);

impl Budget {
    pub fn new(amount: NonZeroUsize) -> Self {
        Self(Arc::new(Semaphore::new(amount.get())))
    }

    fn remaining(&self) -> usize {
        self.0.available_permits()
    }
}

#[bon]
impl Peer {
    #[builder]
    pub fn new(
        config: Arc<Config>,
        budget: NonZeroUsize,
        messages: Queue<(RetryPolicy, Bytes)>,
        inbound: UnboundedSender<PeerMessage>,
        next_slot: watch::Receiver<Slot>,
        metrics: Arc<dyn Metrics>,
    ) -> Self {
        Self {
            max_message_size: config.max_message_size.get() + Trailer::MAX_SIZE,
            conf: config.clone(),
            budget: Budget::new(budget),
            next_slot,
            lower_bound: Arc::new(AtomicU64::new(Slot::MIN.into())),
            tx: inbound,
            retry: DelayQueue::new(config),
            msgs: messages,
            metrics,
        }
    }

    /// Start I/O with a peer over the given connection.
    ///
    /// This method continues until an error occurs, after which callers may
    /// want to reconnect and resume peer operation with a new `Connection`.
    pub async fn start(&mut self, conn: Connection, cancel: CancellationToken) -> Result<Empty> {
        // Early check if we already got cancelled which can happen with many
        // simultaneous connects where we need to drop connections.
        if cancel.is_cancelled() {
            return Err(NetworkError::PeerInterrupt);
        }

        // ACKs received from remote.
        //
        // When receiving an ACK we remove the message from the retry buffer.
        // The channel is unbounded because we can always remove an entry
        // quickly.
        let (ibound_acks_tx, mut ibound_acks_rx) = mpsc::unbounded_channel();

        // ACKs to send to the remote.
        //
        // When we have received a message we need to send an ACK back.
        // This channel is bounded. In case we accumulate too many ACKs we
        // drop the connection as the sender task can not make progress.
        let (obound_acks_tx, obound_acks_rx) = mpsc::channel(self.conf.peer_budget.get());

        // Messages to send to the remote.
        //
        // These are the plain bytes (plus retry policy) that the sending task
        // should deliver to the remote.
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        self.retry.reset();

        let Connection {
            key: peer,
            addr,
            stream,
            state,
            recv_nonce,
            send_nonce,
        } = conn;

        let state = Arc::new(state);

        let (read_half, write_half) = stream.into_split();

        let countdown = Countdown::new();

        let sender = Sender {
            acks: obound_acks_rx,
            conf: self.conf.clone(),
            countdown: countdown.clone(),
            messages: message_rx,
            nonce: send_nonce,
            state: state.clone(),
            stream: write_half,
        };

        let receiver = Receiver {
            budget: self.budget.clone(),
            countdown,
            ibound_acks: ibound_acks_tx,
            lower_bound: self.lower_bound.clone(),
            max_message_size: self.max_message_size,
            messages: self.tx.clone(),
            nonce: recv_nonce,
            obound_acks: obound_acks_tx,
            peer,
            state,
            stream: read_half,
        };

        let mut send_task = AbortOnDropHandle::new(spawn(sender.start()));
        let mut recv_task = AbortOnDropHandle::new(spawn(receiver.start()));

        let mut clock = interval(Duration::from_secs(1));
        clock.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            select! {
                // Wait for the next message.
                //
                // We limit writing if we expect too many ACKs that the remote
                // has not sent yet. This is to prevent an attack were a
                // malicious peer never sends ACKs, which would cause our
                // retry queue to grow unbounded.
                m = self.msgs.dequeue(), if self.retry.len() < self.conf.peer_budget.get() => {
                    trace!(name = %self.conf.name, %peer, %addr, "next outbound message");
                    let (slot, id, (policy, bytes)) = m;
                    if policy.is_retry() {
                        self.retry.add(slot, id, bytes.clone());
                    }
                    message_tx.send((policy, bytes)).map_err(|_| NetworkError::ChannelClosed)?
                }

                // When an ACK has been received we can remove the message from the retry queue.
                a = ibound_acks_rx.recv() => {
                    let Some(ack) = a else {
                        return Err(NetworkError::ChannelClosed)
                    };
                    trace!(name = %self.conf.name, %peer, %addr, ?ack, "ack received");
                    let (s, i) = ack.into();
                    self.retry.del(s, i);
                }

                // If requested, interrupt all I/O processing and return.
                _ = cancel.cancelled() => {
                    trace!(name = %self.conf.name, %peer, %addr, "interrupt");
                    recv_task.abort();
                    send_task.abort();
                    return Err(NetworkError::PeerInterrupt)
                }

                // Wait for a slot change, signaling GC.
                r = self.next_slot.changed() => {
                    trace!(name = %self.conf.name, %peer, %addr, "gc");
                    if r.is_err() {
                        return Err(NetworkError::ChannelClosed)
                    }
                    let s = *self.next_slot.borrow_and_update();
                    self.lower_bound.store(s.into(), Ordering::Relaxed);
                    self.retry.gc(s);
                }

                // Periodic maintenance:
                //
                // - Retry messages
                // - Update metrics
                t = clock.tick() => {
                    if !self.retry.is_empty() {
                        trace!(name = %self.conf.name, %peer, %addr, "retry check");
                        self.retry.check(t);
                    }
                    self.update_metrics(&peer)
                }

                // If messages should be re-sent and we can do so, send it:
                //
                // This is separate from the retry check above because the check
                // scans multiple items and here we just take the next one that
                // is ready and go with it.
                () = ready(()), if self.retry.is_ready() => {
                    let Some(bytes) = self.retry.next() else {
                        continue
                    };
                    trace!(name = %self.conf.name, %peer, %addr, "resending message");
                    message_tx
                        .send((RetryPolicy::Default, bytes))
                        .map_err(|_| NetworkError::ChannelClosed)?
                }

                // Check that our send task has not terminated.
                r = &mut send_task => match r {
                    Ok(Err(err)) => {
                        warn!(name = %self.conf.name, %peer, %addr, %err, "send task error");
                        recv_task.abort();
                        return Err(err)
                    }
                    Err(err) => {
                        recv_task.abort();
                        return if err.is_cancelled() {
                            Err(NetworkError::Task("cancelled"))
                        } else {
                            error!(name = %self.conf.name, %peer, %addr, %err, "send task panic");
                            Err(NetworkError::Task("send panic"))
                        }
                    }
                },

                // Check that our receive task has not terminated.
                r = &mut recv_task => match r {
                    Ok(Err(err)) => {
                        warn!(name = %self.conf.name, %peer, %addr, %err, "receive task error");
                        send_task.abort();
                        return Err(err)
                    }
                    Err(err) => {
                        send_task.abort();
                        return if err.is_cancelled() {
                            Err(NetworkError::Task("cancelled"))
                        } else {
                            error!(name = %self.conf.name, %peer, %addr, %err, "receive task panic");
                            Err(NetworkError::Task("receive panic"))
                        }
                    }
                }
            }
        }
    }

    fn update_metrics(&self, key: &PublicKey) {
        self.metrics.set(key, "outbound_messages", self.msgs.len());
        self.metrics.set(key, "retrying_messages", self.retry.len());
        self.metrics
            .set(key, "remaining_budget", self.budget.remaining());
    }
}

struct Sender {
    conf: Arc<Config>,
    stream: OwnedWriteHalf,
    messages: UnboundedReceiver<(RetryPolicy, Bytes)>,
    acks: mpsc::Receiver<Ack>,
    state: Arc<StatelessTransportState>,
    nonce: u64,
    countdown: Countdown,
}

impl Sender {
    async fn start(mut self) -> Result<Empty> {
        let mut buf = NoiseBuf::new([0; _]);
        let mut msg = Bytes::new();
        let mut pol = RetryPolicy::default();
        loop {
            select! {
                Some((p, m)) = self.messages.recv(), if msg.is_empty() => {
                    msg = m;
                    pol = p
                }

                Some(ack) = self.acks.recv() => {
                    let x = self.nonce();
                    let n = self.state.write_message(x, &ack.0, &mut buf[Header::SIZE..])?;
                    let h = Header::ack(n as u16);
                    send_frame(&mut self.stream, h, &mut buf[..Header::SIZE + n]).await?
                }

                // We split messages into frames in this separate select branch
                // to interleave the sending of data frames with ACK frames from
                // the branch above.
                () = ready(()), if !msg.is_empty() => {
                    let b = msg.split_to(min(msg.len(), MAX_PAYLOAD_SIZE));
                    let x = self.nonce();
                    let n = self.state.write_message(x, &b, &mut buf[Header::SIZE..])?;
                    let h = if msg.is_empty() {
                        Header::data(n as u16)
                    } else {
                        Header::data(n as u16).partial()
                    };
                    send_frame(&mut self.stream, h, &mut buf[..Header::SIZE + n]).await?;
                    if msg.is_empty() && pol.is_retry() {
                        self.countdown.start(self.conf.receive_timeout)
                    }
                }
                else => return Err(NetworkError::ChannelClosed)
            }
        }
    }

    fn nonce(&mut self) -> u64 {
        let n = self.nonce;
        self.nonce += 1;
        n
    }
}

struct Receiver {
    peer: PublicKey,
    budget: Budget,
    max_message_size: usize,
    stream: OwnedReadHalf,
    messages: UnboundedSender<PeerMessage>,
    ibound_acks: UnboundedSender<Ack>,
    obound_acks: mpsc::Sender<Ack>,
    nonce: u64,
    state: Arc<StatelessTransportState>,
    lower_bound: Arc<AtomicU64>,
    countdown: Countdown,
}

impl Receiver {
    async fn start(mut self) -> Result<Empty> {
        let mut fbuf = NoiseBuf::new([0; _]); // frame buffer (raw data)
        let mut rbuf = NoiseBuf::new([0; _]); // read buffer (decrypted)
        let mut msg = BytesMut::new();
        loop {
            loop {
                select! {
                    h = recv_frame(&mut self.stream, &mut fbuf) => match h {
                        Ok(h) => {
                            self.countdown.stop();
                            match h.frame_type() {
                                Ok(FrameType::Data) => {
                                    let x = self.nonce();
                                    let n = self.state.read_message(x, &fbuf[.. h.len().into()], &mut *rbuf)?;
                                    msg.extend_from_slice(&rbuf[..n]);
                                    if msg.len() > self.max_message_size {
                                        return Err(NetworkError::MessageTooLarge)
                                    }
                                    if !h.is_partial() {
                                        break
                                    }
                                }
                                Ok(FrameType::Ack) => {
                                    let x = self.nonce();
                                    let n = self.state.read_message(x, &fbuf[.. h.len().into()], &mut *rbuf)?;
                                    let Ok(a) = Ack::try_from(&rbuf[..n]) else {
                                        return Err(NetworkError::InvalidAck)
                                    };
                                    self.ibound_acks.send(a).map_err(|_| NetworkError::ChannelClosed)?
                                }
                                Err(t) => return Err(NetworkError::UnknownFrameType(t))
                            }
                        }
                        Err(err) => return Err(err.into())
                    },
                    () = &mut self.countdown => {
                        return Err(NetworkError::Timeout("receiving from peer"))
                    }
                }
            }

            let mut msg = mem::take(&mut msg).freeze();

            let Some(t) = Trailer::from_bytes(&mut msg) else {
                return Err(NetworkError::InvalidTrailer);
            };

            let slot = match t {
                Trailer::Std { slot, id } => {
                    match self.obound_acks.try_send(Ack::from((slot, id))) {
                        Ok(()) => {},
                        Err(TrySendError::Full(_)) => {
                            return Err(NetworkError::TooManyPendingAcks(self.peer));
                        },
                        Err(TrySendError::Closed(_)) => return Err(NetworkError::ChannelClosed),
                    }
                    Some(slot)
                },
                Trailer::NoAck { slot } => Some(slot),
                Trailer::Unknown => None,
            };

            if let Some(s) = slot
                && u64::from(s) < self.lower_bound.load(Ordering::Relaxed)
            {
                continue;
            }

            let permit = self
                .budget
                .clone()
                .0
                .acquire_owned()
                .await
                .map_err(|_| NetworkError::BudgetClosed)?;

            if self.messages.send((self.peer, msg, Some(permit))).is_err() {
                return Err(NetworkError::ChannelClosed);
            }
        }
    }

    fn nonce(&mut self) -> u64 {
        let n = self.nonce;
        self.nonce += 1;
        n
    }
}
