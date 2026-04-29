use std::io;

use thiserror::Error;

pub use crate::{addr::InvalidNetAddr, msg::InvalidHeader};
use crate::{addr::NetAddr, x25519::PublicKey};

/// The empty type has no values.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Empty {}

/// The various errors that can occur during networking.
#[derive(Debug, Error)]
pub enum NetworkError {
    /// Generic I/O error.
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),

    /// Bind error.
    #[error("error binding to address {0}: {1}")]
    Bind(NetAddr, #[source] io::Error),

    /// The received frame header is not valid.
    #[error("invalid frame header: {0}")]
    InvalidFrameHeader(#[from] InvalidHeader),

    /// The received message trailer is not valid.
    #[error("invalid message trailer")]
    InvalidTrailer,

    /// The received ack frame is not valid.
    #[error("invalid ack frame")]
    InvalidAck,

    /// The received frame has an unknown type.
    #[error("unknown frame type: {0}")]
    UnknownFrameType(u8),

    /// Generic Noise error.
    #[error("noise error: {0}")]
    Noise(#[from] snow::Error),

    /// The Noise handshake message is not valid.
    #[error("invalid handshake message")]
    InvalidHandshakeMessage,

    /// The total message size exceeds the allowed maximum.
    #[error("message too large")]
    MessageTooLarge,

    /// An MPSC channel is unexpectedly closed.
    #[error("channel closed")]
    ChannelClosed,

    /// A receive budget has unexpectedly closed.
    #[error("receive budget closed")]
    BudgetClosed,

    /// An operation timed out.
    #[error("timeout")]
    Timeout,

    /// A peer's I/O processing has been interrupted.
    #[error("peer process interrupted")]
    PeerInterrupt,

    /// We have accumulated too many ACKs for a peer that we can not send
    /// to the remote, meaning we can read fine, but not write properly.
    #[error("too many pending acks for peer {0}")]
    TooManyPendingAcks(PublicKey),
}
