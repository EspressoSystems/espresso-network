mod frame;
mod trailer;

use std::fmt;

pub use frame::{FrameType, Header, InvalidHeader};
pub use trailer::{Trailer, TrailerType};

/// Max. message size using noise protocol.
pub const MAX_NOISE_MESSAGE_SIZE: usize = 64 * 1024;

/// Max. number of bytes for payload data.
pub const MAX_PAYLOAD_SIZE: usize = MAX_NOISE_MESSAGE_SIZE - 32;

/// Slots contain messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Slot(pub(crate) u64);

impl Slot {
    pub const MIN: Slot = Slot(u64::MIN);
    pub const MAX: Slot = Slot(u64::MAX);

    pub const fn new(n: u64) -> Self {
        Self(n)
    }
}

impl From<Slot> for u64 {
    fn from(s: Slot) -> Self {
        s.0
    }
}

impl fmt::Display for Slot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A message identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MsgId(pub(crate) u64);

impl MsgId {
    pub const fn new(n: u64) -> Self {
        Self(n)
    }
}

impl From<MsgId> for u64 {
    fn from(i: MsgId) -> Self {
        i.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Ack(pub [u8; 16]);

impl From<(Slot, MsgId)> for Ack {
    fn from((s, i): (Slot, MsgId)) -> Self {
        let s = u64::to_be_bytes(s.0);
        let i = u64::to_be_bytes(i.0);
        let mut a = [0; 16];
        a[..8].copy_from_slice(&s);
        a[8..].copy_from_slice(&i);
        Self(a)
    }
}

impl From<Ack> for (Slot, MsgId) {
    fn from(a: Ack) -> Self {
        let s = Slot(u64::from_be_bytes(a.0[..8].try_into().expect("8 bytes")));
        let i = MsgId(u64::from_be_bytes(a.0[8..].try_into().expect("8 bytes")));
        (s, i)
    }
}

impl TryFrom<&[u8]> for Ack {
    type Error = ();

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(Ack(value.try_into().map_err(|_| ())?))
    }
}
