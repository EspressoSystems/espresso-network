use bytes::Bytes;

use crate::msg::{MsgId, Slot};

const STD: u8 = 0;
const NO_ACK: u8 = 1;

/// Meta information appended at the end of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trailer {
    Std {
        /// The slot the message corresponds to.
        slot: Slot,
        /// The message ID.
        id: MsgId,
    },
    NoAck {
        /// The slot the message corresponds to.
        slot: Slot,
    },
    Unknown,
}

impl Trailer {
    pub const MAX_SIZE: usize = u8::MAX as usize;

    pub(crate) fn from_bytes(bytes: &mut Bytes) -> Option<Self> {
        if bytes.len() < 2 {
            return None;
        }
        let len = bytes.len();
        let trailer_len = bytes[len - 1];
        let trailer_typ = bytes[len - 2];
        match trailer_typ {
            STD => {
                if trailer_len != 16 {
                    return None;
                }
                let id = u64::from_be_bytes(bytes[len - 10..len - 2].try_into().ok()?);
                let slot = u64::from_be_bytes(bytes[len - 18..len - 10].try_into().ok()?);
                bytes.truncate(len - 18);
                Some(Self::Std {
                    slot: Slot(slot),
                    id: MsgId(id),
                })
            },
            NO_ACK => {
                if trailer_len != 8 {
                    return None;
                }
                let slot = u64::from_be_bytes(bytes[len - 10..len - 2].try_into().ok()?);
                bytes.truncate(len - 10);
                Some(Self::NoAck { slot: Slot(slot) })
            },
            _ => {
                let trailer_len = 2 + usize::from(trailer_len);
                if trailer_len > len {
                    return None;
                }
                bytes.truncate(len - trailer_len);
                Some(Self::Unknown)
            },
        }
    }

    pub(crate) fn to_bytes(self) -> TrailerBytes {
        match self {
            Self::Std { slot, id } => {
                let mut buf = [0; 18];
                buf[..8].copy_from_slice(&slot.0.to_be_bytes()[..]);
                buf[8..16].copy_from_slice(&id.0.to_be_bytes()[..]);
                buf[16] = STD;
                buf[17] = 16;
                TrailerBytes::Std(buf)
            },
            Self::NoAck { slot } => {
                let mut buf = [0; 10];
                buf[..8].copy_from_slice(&slot.0.to_be_bytes()[..]);
                buf[8] = NO_ACK;
                buf[9] = 8;
                TrailerBytes::NoAck(buf)
            },
            Self::Unknown => {
                unreachable!("nothing constructs an unknown trailer")
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum TrailerBytes {
    Std([u8; 18]),
    NoAck([u8; 10]),
}

impl AsRef<[u8]> for TrailerBytes {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Std(a) => &a[..],
            Self::NoAck(a) => &a[..],
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use quickcheck::{Arbitrary, Gen, quickcheck};

    use super::Trailer;
    use crate::msg::{MsgId, Slot};

    impl Arbitrary for Trailer {
        fn arbitrary(g: &mut Gen) -> Self {
            match bool::arbitrary(g) {
                true => Self::Std {
                    slot: Slot(u64::arbitrary(g)),
                    id: MsgId(u64::arbitrary(g)),
                },
                false => Self::NoAck {
                    slot: Slot(u64::arbitrary(g)),
                },
            }
        }
    }

    quickcheck! {
        fn prop_to_bytes_from_bytes_id(t1: Trailer) -> bool {
            let mut b = Bytes::copy_from_slice(t1.to_bytes().as_ref());
            let t2 = Trailer::from_bytes(&mut b);
            Some(t1) == t2
        }
    }
}
