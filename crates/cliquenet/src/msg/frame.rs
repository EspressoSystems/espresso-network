//! # Frame header
//!
//! The unit of data exchanged over the network is called a `Frame` and consists of
//! a 4-byte header and a body of variable size. The header has the following
//! structure:
//!
//! ```text
//!  0                   1                   2                   3
//!  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |       |       |P|             |                               |
//! |Version|  Type |a|  Reserved   |        Payload length         |
//! |       |       |r|             |                               |
//! |       |       |t|             |                               |
//! +-------+-------+-+-------------+-------------------------------+
//! ```
//!
//! where
//!
//! - Version (4 bits)
//! - Type (4 bits)
//!    - Data (0)
//!    - Ack  (1)
//! - Partial (1 bit)
//! - Reserved (7 bits)
//! - Payload length (16 bits)
//!
//! If the partial bit is set, the frame is only a part of the message and the read task
//! will assemble all frames to produce the final message.

use std::fmt;

/// The header of a frame.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Header(u32);

impl Header {
    pub const SIZE: usize = 4;

    pub fn new(ty: Type, len: u16) -> Self {
        match ty {
            Type::Data => Self::data(len),
            Type::Ack => Self::ack(len),
        }
    }

    /// Create a new, unvalidated header from the given bytes.
    pub fn unvalidated(bytes: [u8; Self::SIZE]) -> Self {
        Self(u32::from_be_bytes(bytes))
    }

    /// Create a data header with the given payload length.
    pub fn data(len: u16) -> Self {
        Self(len as u32)
    }

    /// Create an ack header with the given payload length.
    pub fn ack(len: u16) -> Self {
        Self(0x1000000 | len as u32)
    }

    /// The type of the frame following this header.
    pub fn frame_type(self) -> Result<Type, u8> {
        match (self.0 & 0xF000000) >> 24 {
            0 => Ok(Type::Data),
            1 => Ok(Type::Ack),
            t => Err(t as u8),
        }
    }

    /// Set the partial flag to indicate that more frames follow.
    pub fn partial(self) -> Self {
        Self(self.0 | 0x800000)
    }

    /// Is this a data frame header?
    pub fn is_data(self) -> bool {
        self.0 & 0xF000000 == 0
    }

    /// Is this an ack frame header?
    pub fn is_ack(self) -> bool {
        self.0 & 0xF000000 == 0x1000000
    }

    /// Is this a partial frame?
    pub fn is_partial(self) -> bool {
        self.0 & 0x800000 == 0x800000
    }

    /// Get the payload length.
    pub fn len(self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }

    /// Is the payload length 0?
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Convert this header into a byte array.
    pub fn to_bytes(self) -> [u8; Self::SIZE] {
        self.0.to_be_bytes()
    }
}

/// The type of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Data,
    Ack,
}

impl From<Header> for [u8; Header::SIZE] {
    fn from(val: Header) -> Self {
        val.to_bytes()
    }
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Header")
            .field("type", &self.frame_type())
            .field("len", &self.len())
            .field("partial", &self.is_partial())
            .finish()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid header: {0}")]
pub struct InvalidHeader(&'static str);

#[cfg(test)]
mod tests {
    use quickcheck::quickcheck;

    use super::{Header, Type};

    quickcheck! {
        fn data(len: u16) -> bool {
            let hdr = Header::data(len);
            hdr.is_data() && !hdr.is_partial() && hdr.frame_type() == Ok(Type::Data)
        }

        fn ack(len: u16) -> bool {
            let hdr = Header::ack(len);
            hdr.is_ack() && !hdr.is_partial() && hdr.frame_type() == Ok(Type::Ack)
        }

        fn partial_data(len: u16) -> bool {
            Header::data(len).partial().is_partial()
        }

        fn partial_ack(len: u16) -> bool {
            Header::ack(len).partial().is_partial()
        }

        fn data_len(len: u16) -> bool {
            Header::data(len).len() == len
        }

        fn ack_len(len: u16) -> bool {
            Header::ack(len).len() == len
        }
    }
}
