use bytes::Bytes;

use crate::msg::{MsgId, Slot};

/// Meta information appended at the end of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Trailer {
    pub(crate) ty: TrailerType,
    /// The slot the message corresponds to.
    pub(crate) slot: Slot,
    /// The message ID.
    pub(crate) id: MsgId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum TrailerType {
    Std = 0,
    NoAck = 1,
    Unknown,
}

impl From<u8> for TrailerType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Std,
            1 => Self::NoAck,
            _ => Self::Unknown,
        }
    }
}

impl TrailerType {
    pub fn is_ack(self) -> bool {
        matches!(self, Self::Std | Self::Unknown)
    }
}

// Besides the actual meta information, the last byte of the trailer encodes its
// total length (including the length byte itself).
impl Trailer {
    pub const SIZE: usize = 18;
    pub const MAX_SIZE: usize = u8::MAX as usize;

    pub fn new(ty: TrailerType, slot: Slot, id: MsgId) -> Self {
        Self { ty, slot, id }
    }

    pub fn from_bytes(bytes: &mut Bytes) -> Option<Self> {
        let len = bytes.len();
        let trailer_len: usize = (*bytes.last()?).into();
        if trailer_len < Self::SIZE || len < trailer_len {
            return None;
        }
        let ty = TrailerType::from(bytes[len - 2]);
        let id = u64::from_be_bytes(bytes[len - 10..len - 2].try_into().ok()?);
        let slot = u64::from_be_bytes(bytes[len - 18..len - 10].try_into().ok()?);
        bytes.truncate(len - trailer_len);
        Some(Self {
            ty,
            slot: Slot(slot),
            id: MsgId(id),
        })
    }

    pub fn to_bytes(self) -> [u8; Self::SIZE] {
        let mut buf = [0; Self::SIZE];
        buf[..8].copy_from_slice(&self.slot.0.to_be_bytes()[..]);
        buf[8..16].copy_from_slice(&self.id.0.to_be_bytes()[..]);
        buf[16] = self.ty as u8;
        buf[17] = Self::SIZE as u8;
        buf
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use quickcheck::quickcheck;

    use super::Trailer;
    use crate::msg::{MsgId, Slot};

    quickcheck! {
        fn to_from_bytes(t: u8, b: u64, i: u64) -> bool {
            let a = Trailer {
                ty: t.into(),
                slot: Slot(b),
                id: MsgId(i)
            };
            let mut bytes = Bytes::copy_from_slice(&a.to_bytes());
            let b = Trailer::from_bytes(&mut bytes).unwrap();
            a == b
        }
    }
}
