mod delay;

#[cfg(test)]
mod tests;

use std::{
    cmp::min, collections::VecDeque, future::ready, io, mem, net::SocketAddr, num::NonZeroUsize,
    sync::Arc, time::Duration,
};

use bon::bon;
use bytes::{Bytes, BytesMut};
use delay::DelayQueue;
use snow::TransportState;
use tokio::{
    select,
    sync::{OwnedSemaphorePermit, Semaphore, mpsc::UnboundedSender, watch},
    time::{MissedTickBehavior, interval},
};
use tokio_util::sync::CancellationToken;
use tracing::{trace, warn};

use crate::{
    Config, PublicKey,
    connection::Connection,
    error::{Empty, NetworkError},
    msg::{Ack, Header, MAX_NOISE_MESSAGE_SIZE, MAX_PAYLOAD_SIZE, Slot, Trailer, Type},
    net::PeerMessage,
    queue::Queue,
    time::Countdown,
};

type NoiseBuf = Box<[u8; MAX_NOISE_MESSAGE_SIZE]>;
type Result<T> = std::result::Result<T, NetworkError>;

/// A peer sends and receives messages over a connection with a remote.
///
/// Peers are initialised with a [`Connection`], which can be replaced
/// later, should it be necessary.
///
/// Messages sent are expected to be acknowledged, i.e. the remote needs to
/// send back an ACK frame. Otherwise the message is resent after some time.
pub struct Peer {
    /// Network configuration.
    conf: Arc<Config>,

    /// A budget limits how many message a peer can deliver to the application.
    budget: Budget,

    /// The current TCP+Noise connection.
    conn: Connection,

    /// Messages the application wants to be sent to the remote.
    msgs: Queue<Bytes>,

    /// Messages waiting to be retried if no ACK has been received.
    retry: DelayQueue,

    /// Receive notifications about changes to the GC threshold
    next_slot: watch::Receiver<Slot>,

    /// Our current GC threshold.
    lower_bound: Slot,

    /// The channel over which to deliver inbound messages to the application.
    tx: UnboundedSender<PeerMessage>,

    /// A healthcheck countdown.
    ///
    /// When dropped to 0 the connection should be replaced.
    countdown: Countdown,

    /// Token to interrupt processing.
    cancel: CancellationToken,

    /// The true max. message size.
    ///
    /// It accounts for the additional `Trailer` bytes.
    max_message_size: usize,
}

/// A budget limits how many messages a peer can delivery to the application.
pub struct Budget(Arc<Semaphore>);

impl Budget {
    pub fn new(amount: NonZeroUsize) -> Self {
        Self(Arc::new(Semaphore::new(amount.get())))
    }
}

#[bon]
impl Peer {
    #[builder]
    pub fn new(
        config: Arc<Config>,
        budget: NonZeroUsize,
        messages: Queue<Bytes>,
        inbound: UnboundedSender<PeerMessage>,
        next_slot: watch::Receiver<Slot>,
        connection: Connection,
    ) -> Self {
        Self {
            max_message_size: config.max_message_size.get() + Trailer::MAX_SIZE,
            conf: config.clone(),
            budget: Budget::new(budget),
            next_slot,
            lower_bound: Slot::MIN,
            tx: inbound,
            conn: connection,
            retry: DelayQueue::new(config),
            msgs: messages,
            countdown: Countdown::new(),
            cancel: CancellationToken::new(),
        }
    }

    /// Replace the existing connection.
    pub fn set_connection(&mut self, c: Connection) {
        self.conn = c;
        self.retry.reset();
        self.cancel = CancellationToken::new()
    }

    /// Get a token to interrupt processing.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    /// Get the peer's public key.
    pub fn public_key(&self) -> &PublicKey {
        &self.conn.key
    }

    /// Get the peer's socket address.
    pub fn socket_addr(&self) -> &SocketAddr {
        &self.conn.addr
    }

    /// Start I/O with a connected peer.
    ///
    /// This method continues until an error occurs, after which callers may
    /// want to reconnect and resume peer operation with a new `Connection`.
    pub async fn start(&mut self) -> Result<Empty> {
        /// Messages are broken into frames and each frame has a header.
        ///
        /// The `ReadState` tracks what we have received from the remote.
        enum ReadState<'a> {
            Header {
                off: usize,
                buf: [u8; Header::SIZE],
            },
            Frame {
                hdr: Header,
                off: usize,
                buf: &'a mut Vec<u8>,
            },
        }

        /// This state tracks what we have sent to the remote. We currently
        /// support either data or ACK frames. The offset and length values
        /// are relative to the write buffer (see below).
        enum WriteState {
            /// No write operation is in progress.
            Idle,
            /// An ACK frame is sent.
            Ack { off: usize, len: usize },
            /// A data frame is sent.
            Data { off: usize, len: usize },
        }

        impl WriteState {
            fn is_idle(&self) -> bool {
                matches!(self, Self::Idle)
            }

            /// Encrypt a single data frame with Noise.
            fn data_frame(
                data: &[u8],
                is_partial: bool,
                state: &mut TransportState,
                buf: &mut NoiseBuf,
            ) -> Result<Self> {
                let n = state.write_message(data, &mut buf[Header::SIZE..])?;
                let h = if is_partial {
                    Header::data(n as u16).partial()
                } else {
                    Header::data(n as u16)
                };
                buf[..Header::SIZE].copy_from_slice(&h.to_bytes());
                Ok(Self::Data {
                    off: 0,
                    len: n + Header::SIZE,
                })
            }

            /// Encrypt a single ACK frame with Noise.
            fn ack_frame(a: Ack, state: &mut TransportState, buf: &mut NoiseBuf) -> Result<Self> {
                let n = state.write_message(&a.0, &mut buf[Header::SIZE..])?;
                let h = Header::ack(n as u16);
                buf[..Header::SIZE].copy_from_slice(&h.to_bytes());
                Ok(Self::Ack {
                    off: 0,
                    len: n + Header::SIZE,
                })
            }
        }

        // Early check if we already got cancelled which can happen with many
        // simultaneous connects where we need to drop connections.
        if self.cancel.is_cancelled() {
            return Err(NetworkError::PeerInterrupt);
        }

        // Noise packages are limited to 64KiB.
        //
        // We retain one such buffer for reading and one for writing.
        let mut rbuf = NoiseBuf::new([0; _]);
        let mut wbuf = NoiseBuf::new([0; _]);

        // A frame buffer for reading before it is decrypted.
        let mut fbuf = Vec::new();

        // An incoming message that is assembled from its frames.
        let mut ibound_msg = BytesMut::new();

        // An outgoing message. The integer tracks the chunk size as we need
        // to break the message into frames that fit into a noise package.
        let mut obound_msg: Option<(Bytes, usize)> = None;

        // Pending outbound ACK messages. This is appended when we received a
        // message and picked up and interleaved when sending frames or else
        // at the start of the loop (see below).
        let mut obound_acks: VecDeque<Ack> = VecDeque::new();

        // Track write and read states:
        let mut wstate = WriteState::Idle;
        let mut rstate = ReadState::Header {
            off: 0,
            buf: [0; _],
        };

        // Each read needs a permit taken from our budget in order to deliver
        // to the application.
        let mut read_permit: Option<OwnedSemaphorePermit> = None;

        // Measure time.
        //
        // Used to trigger resends of messages that have not been ACKed yet.
        let mut clock = interval(Duration::from_secs(1));
        clock.set_missed_tick_behavior(MissedTickBehavior::Skip);

        // Ensure any previously fired countdown is reset.
        self.countdown.stop();

        loop {
            trace!(
                name     = %self.conf.name,
                peer     = %self.conn.key,
                addr     = %self.conn.addr,
                writing  = %!wstate.is_idle(),
                acks     = %obound_acks.len(),
                retries  = %self.retry.len(),
                can_read = %read_permit.is_some(),
                "entering event loop"
            );

            select! {
                // Wait for the next message if no write is in progress.
                //
                // We limit writing if we expect too many ACKs that the remote
                // has not sent yet. This is to prevent an attack were a
                // malicious peer never sends ACKs, which would cause our
                // delay queue to grow unbounded.
                m = self.msgs.dequeue(), if wstate.is_idle() && self.retry.len() < self.conf.peer_budget.get() => {
                    trace!(name = %self.conf.name, peer = %self.conn.key, "next outbound message");
                    let (slot, id, bytes) = m;
                    self.retry.add(slot, id, bytes.clone());
                    let chunk = min(bytes.len(), MAX_PAYLOAD_SIZE);
                    wstate = WriteState::data_frame(
                        &bytes[..chunk],
                        chunk < bytes.len(),
                        &mut self.conn.state,
                        &mut wbuf
                    )?;
                    obound_msg = Some((bytes, chunk))
                }

                // Pick up an ACK and send it if possible.
                () = ready(()), if wstate.is_idle() && !obound_acks.is_empty() => {
                    trace!(name = %self.conf.name, peer = %self.conn.key, "next outbound ack");
                    let ack = obound_acks.pop_front().expect("obound_acks is not empty");
                    wstate = WriteState::ack_frame(ack, &mut self.conn.state, &mut wbuf)?
                }

                // If requested, interrupt all I/O processing.
                //
                // Once a peer has been interrupted, its connection needs to be
                // replaced before calling start again.
                _ = self.cancel.cancelled() => {
                    trace!(name = %self.conf.name, peer = %self.conn.key, "interrupt");
                    return Err(NetworkError::PeerInterrupt)
                }

                // Wait for a slot change, signaling GC.
                r = self.next_slot.changed() => {
                    trace!(name = %self.conf.name, peer = %self.conn.key, "gc");
                    if r.is_err() {
                        return Err(NetworkError::ChannelClosed)
                    }
                    let s = *self.next_slot.borrow_and_update();
                    debug_assert!(s > self.lower_bound);
                    self.lower_bound = s;
                    self.retry.gc(s);
                }

                // Check if there are messages that should be re-sent:
                t = clock.tick(), if !self.retry.is_empty() => {
                    trace!(name = %self.conf.name, peer = %self.conn.key, "retry check");
                    self.retry.check(t)
                }

                // If messages should be re-sent and we can do so, send it:
                //
                // This is separate from the retry check above because the check
                // scans multiple items and here we just take the next one that
                // is ready and go with it.
                () = ready(()), if self.retry.is_ready() && wstate.is_idle() => {
                    trace!(name = %self.conf.name, peer = %self.conn.key, "resending message");
                    let Some(bytes) = self.retry.next() else {
                        continue
                    };
                    let chunk = min(bytes.len(), MAX_PAYLOAD_SIZE);
                    wstate = WriteState::data_frame(
                        &bytes[..chunk],
                        chunk < bytes.len(),
                        &mut self.conn.state,
                        &mut wbuf
                    )?;
                    obound_msg = Some((bytes, chunk))
                }

                // Continue an ongoing write operation.
                r = self.conn.stream.writable(), if !wstate.is_idle() => {
                    trace!(name = %self.conf.name, peer = %self.conn.key, "continue writing");
                    if let Err(e) = r {
                        return Err(e.into())
                    }
                    match &mut wstate {
                        WriteState::Ack { off, len } => {
                            match self.conn.stream.try_write(&wbuf[*off..*len]) {
                                Ok(n) => {
                                    *off += n;
                                    if *off < *len {
                                        continue
                                    }
                                    if let Some((bytes, chunk)) = &mut obound_msg && *chunk < bytes.len() {
                                        let end = min(*chunk + MAX_PAYLOAD_SIZE, bytes.len());
                                        wstate = WriteState::data_frame(
                                            &bytes[*chunk..end],
                                            end < bytes.len(),
                                            &mut self.conn.state,
                                            &mut wbuf
                                        )?;
                                        *chunk = end;
                                    } else {
                                        obound_msg = None;
                                        wstate = WriteState::Idle;
                                        self.countdown.start(self.conf.receive_timeout)
                                    }
                                }
                                Err(e) => if e.kind() != io::ErrorKind::WouldBlock {
                                    return Err(e.into())
                                }
                            }
                        }
                        WriteState::Data { off, len } => {
                            match self.conn.stream.try_write(&wbuf[*off..*len]) {
                                Ok(n) => {
                                    *off += n;
                                    if *off < *len {
                                        continue
                                    }
                                    if let Some(ack) = obound_acks.pop_front() {
                                        wstate = WriteState::ack_frame(ack, &mut self.conn.state, &mut wbuf)?
                                    } else if let Some((bytes, chunk)) = &mut obound_msg && *chunk < bytes.len() {
                                        let end = min(*chunk + MAX_PAYLOAD_SIZE, bytes.len());
                                        wstate = WriteState::data_frame(
                                            &bytes[*chunk..end],
                                            end < bytes.len(),
                                            &mut self.conn.state,
                                            &mut wbuf
                                        )?;
                                        *chunk = end;
                                    } else {
                                        obound_msg = None;
                                        wstate = WriteState::Idle;
                                        self.countdown.start(self.conf.receive_timeout)
                                    }
                                }
                                Err(e) => if e.kind() != io::ErrorKind::WouldBlock {
                                    return Err(e.into())
                                }
                            }
                        },
                        WriteState::Idle => { /* unreachable!() */ }
                    }
                }

                // Wait for the healthcheck countdown to finish.
                //
                // The countdown is started after writing a message and reset
                // when we received a frame.
                () = &mut self.countdown => {
                    trace!(name = %self.conf.name, peer = %self.conn.key, "read timeout");
                    return Err(NetworkError::Timeout)
                }

                // Await the next read permit.
                //
                // If our budget is used up, we need to wait for capacity to become
                // available before we can continue to read from the socket (and
                // eventually deliver the message to the application).
                p = self.budget.0.clone().acquire_owned(), if read_permit.is_none() => {
                    trace!(name = %self.conf.name, peer = %self.conn.key, "next read permit");
                    read_permit = Some(p.map_err(|_| NetworkError::BudgetClosed)?);
                }

                // Continue reading from the socket if possible.
                //
                // NB that we require the ACKs that we have appended before to
                // picked up. This should be very fast as writing interleaves
                // ACKs in between frames. We do this to exercise backpressure
                // because if the remote does not or can not read what we write
                // but keeps sending us data we would accumulate ACKs without
                // bound.
                r = self.conn.stream.readable(), if read_permit.is_some() => {
                    trace!(name = %self.conf.name, peer = %self.conn.key, "continue reading");
                    if let Err(e) = r {
                        return Err(e.into())
                    }
                    if obound_acks.len() > self.conf.peer_budget.get() {
                        return Err(NetworkError::TooManyPendingAcks(self.conn.key))
                    }
                    match &mut rstate {
                        ReadState::Header { off, buf } => {
                            match self.conn.stream.try_read(&mut buf[*off..]) {
                                Ok(0) => {
                                    let e = io::ErrorKind::UnexpectedEof.into();
                                    return Err(NetworkError::Io(e))
                                }
                                Ok(n) => {
                                    self.countdown.stop();
                                    *off += n;
                                    if *off < buf.len() {
                                        continue
                                    }
                                    let hdr = Header::unvalidated(*buf);
                                    fbuf.resize(hdr.len().into(), 0);
                                    rstate = ReadState::Frame { hdr, off: 0, buf: &mut fbuf }
                                }
                                Err(e) => if e.kind() != io::ErrorKind::WouldBlock {
                                    return Err(e.into())
                                }
                            }
                        }
                        ReadState::Frame { hdr, off, buf } => {
                            match self.conn.stream.try_read(&mut buf[*off..]) {
                                Ok(0) => {
                                    let e = io::ErrorKind::UnexpectedEof.into();
                                    return Err(NetworkError::Io(e))
                                }
                                Ok(n) => {
                                    self.countdown.stop();
                                    *off += n;
                                    if *off < buf.len() {
                                        continue
                                    }
                                    match hdr.frame_type() {
                                        Ok(Type::Data) => {
                                            let n = self.conn.state.read_message(buf, &mut *rbuf)?;
                                            ibound_msg.extend_from_slice(&rbuf[..n]);
                                            if ibound_msg.len() > self.max_message_size {
                                                return Err(NetworkError::MessageTooLarge)
                                            }
                                            if !hdr.is_partial() { // message complete
                                                let mut msg = mem::take(&mut ibound_msg).freeze();
                                                let Some(t) = Trailer::from_bytes(&mut msg) else {
                                                    warn!(
                                                        name = %self.conf.name,
                                                        node = %self.conf.keypair.public_key(),
                                                        peer = %self.conn.key,
                                                        addr = %self.conn.addr,
                                                        "invalid trailer"
                                                    );
                                                    return Err(NetworkError::InvalidTrailer);
                                                };
                                                obound_acks.push_back(Ack::from((t.slot, t.id)));
                                                if t.slot >= self.lower_bound {
                                                    let p = read_permit.take();
                                                    debug_assert!(p.is_some());
                                                    if self.tx.send((self.conn.key, msg, p)).is_err() {
                                                        return Err(NetworkError::ChannelClosed)
                                                    }
                                                    trace!(
                                                        name = %self.conf.name,
                                                        node = %self.conf.keypair.public_key(),
                                                        peer = %self.conn.key,
                                                        addr = %self.conn.addr,
                                                        "message delivered"
                                                    );
                                                }
                                            }
                                            rstate = ReadState::Header { off: 0, buf: [0; _] };
                                        }
                                        Ok(Type::Ack) if hdr.is_partial() => {
                                            return Err(NetworkError::InvalidAck)
                                        }
                                        Ok(Type::Ack) => {
                                            let n = self.conn.state.read_message(buf, &mut *rbuf)?;
                                            let Ok(a) = Ack::try_from(&rbuf[..n]) else {
                                                return Err(NetworkError::InvalidAck)
                                            };
                                            let (s, i) = a.into();
                                            self.retry.del(s, i);
                                            rstate = ReadState::Header { off: 0, buf: [0; _] };
                                        }
                                        Err(t) => {
                                            return Err(NetworkError::UnknownFrameType(t))
                                        }
                                    }
                                }
                                Err(e) => if e.kind() != io::ErrorKind::WouldBlock {
                                    return Err(e.into())
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
