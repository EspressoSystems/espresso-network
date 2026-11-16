//! How a value is stored in a redb row: inline, or as a reference to extern data.

use std::{fs::File, os::unix::fs::FileExt};

use bytes::Bytes;
use consensus_store::{EXTERN_THRESHOLD, Error, Table, Usage};

const INLINE: u8 = 0;
const EXTERN: u8 = 1;
const EXTERN_LEN: usize = 1 + 8 + 8 + 8 + 4;

/// A stored row value.
pub enum Row<'a> {
    Inline(&'a [u8]),
    Extern(ExternRef),
}

/// Where an extern value lies in the segment files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExternRef {
    pub segment: u64,
    pub offset: u64,
    pub len: u64,
    pub crc: u32,
}

impl<'a> Row<'a> {
    pub fn decode(table: &Table, key: &[u8], raw: &'a [u8]) -> Result<Self, Error> {
        let corrupt = |reason| Error::Corrupt {
            table: table.clone(),
            key: Bytes::copy_from_slice(key),
            reason,
        };
        match raw.split_first() {
            Some((&INLINE, value)) if value.len() <= EXTERN_THRESHOLD => Ok(Self::Inline(value)),
            Some((&EXTERN, rest)) if rest.len() == EXTERN_LEN - 1 => {
                let u64_at = |i: usize| u64::from_be_bytes(rest[i..i + 8].try_into().unwrap());
                let r = ExternRef {
                    segment: u64_at(0),
                    offset: u64_at(8),
                    len: u64_at(16),
                    crc: u32::from_be_bytes(rest[24..28].try_into().unwrap()),
                };
                if r.len as usize > EXTERN_THRESHOLD {
                    Ok(Self::Extern(r))
                } else {
                    Err(corrupt("extern value at or below the threshold"))
                }
            },
            _ => Err(corrupt("malformed row")),
        }
    }

    pub fn usage(&self, key_len: usize) -> Usage {
        match self {
            Self::Inline(v) => Usage::of_row(key_len, v.len()),
            Self::Extern(r) => Usage::of_row(key_len, r.len as usize),
        }
    }

    pub fn extern_ref(&self) -> Option<ExternRef> {
        match self {
            Self::Inline(_) => None,
            Self::Extern(r) => Some(*r),
        }
    }

    /// The value, read from `file` if it is extern.
    ///
    /// `file` returns the open file of a segment.
    pub fn read(
        &self,
        table: &Table,
        key: &[u8],
        file: impl FnOnce(u64) -> Option<std::sync::Arc<File>>,
    ) -> Result<Bytes, Error> {
        let r = match self {
            Self::Inline(v) => return Ok(Bytes::copy_from_slice(v)),
            Self::Extern(r) => r,
        };
        let corrupt = |reason| Error::Corrupt {
            table: table.clone(),
            key: Bytes::copy_from_slice(key),
            reason,
        };
        let file = file(r.segment).ok_or_else(|| corrupt("missing segment"))?;
        match r.offset.checked_add(r.len) {
            Some(end) if end <= file.metadata()?.len() => {},
            _ => return Err(corrupt("extern value beyond the end of its segment")),
        }
        let mut buf = vec![0; r.len as usize];
        file.read_exact_at(&mut buf, r.offset)?;
        if crc32fast::hash(&buf) != r.crc {
            return Err(corrupt("extern value fails its checksum"));
        }
        Ok(buf.into())
    }
}

pub fn encode_inline(value: &[u8]) -> Vec<u8> {
    debug_assert!(value.len() <= EXTERN_THRESHOLD);
    let mut raw = Vec::with_capacity(1 + value.len());
    raw.push(INLINE);
    raw.extend_from_slice(value);
    raw
}

pub fn encode_extern(r: ExternRef) -> [u8; EXTERN_LEN] {
    let mut raw = [0; EXTERN_LEN];
    raw[0] = EXTERN;
    raw[1..9].copy_from_slice(&r.segment.to_be_bytes());
    raw[9..17].copy_from_slice(&r.offset.to_be_bytes());
    raw[17..25].copy_from_slice(&r.len.to_be_bytes());
    raw[25..29].copy_from_slice(&r.crc.to_be_bytes());
    raw
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let t = Table::new("t").unwrap();
        let r = ExternRef {
            segment: 3,
            offset: 1 << 40,
            len: EXTERN_THRESHOLD as u64 + 1,
            crc: 0xdead_beef,
        };
        let raw = encode_extern(r);
        assert!(matches!(Row::decode(&t, b"k", &raw), Ok(Row::Extern(d)) if d == r));

        let raw = encode_inline(b"value");
        assert!(matches!(
            Row::decode(&t, b"k", &raw),
            Ok(Row::Inline(b"value"))
        ));
    }

    #[test]
    fn malformed() {
        let t = Table::new("t").unwrap();
        let small = ExternRef {
            segment: 0,
            offset: 0,
            len: EXTERN_THRESHOLD as u64,
            crc: 0,
        };
        let too_long = [&[INLINE][..], &vec![0; EXTERN_THRESHOLD + 1]].concat();
        for raw in [
            &[][..],
            &[2, 0],
            &encode_extern(small)[..],
            &encode_extern(small)[..EXTERN_LEN - 1],
            &too_long,
        ] {
            assert!(matches!(
                Row::decode(&t, b"k", raw),
                Err(Error::Corrupt { .. })
            ));
        }
    }
}
