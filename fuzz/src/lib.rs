//! Helpers shared by the fuzz targets in `fuzz_targets/`.

use arbitrary::Arbitrary;

/// One change to an honestly serialized message, standing in for the bytes a
/// malicious peer or server sends in its place.
///
/// Mutating a real encoding keeps most inputs decodable, so the fuzzer spends
/// its time in verification and recovery rather than in the decoder's first
/// length check.
#[derive(Arbitrary, Debug)]
pub enum Edit {
    Overwrite { at: u16, bytes: Vec<u8> },
    Insert { at: u16, len: u16, fill: u8 },
    Truncate { at: u16 },
}

/// Apply `edits` in order. Offsets wrap around the current length, so every
/// edit lands somewhere in the message. Inserts stop growing the message at
/// four times its original length or 64 KiB, whichever is larger.
pub fn apply_edits(mut bytes: Vec<u8>, edits: &[Edit]) -> Vec<u8> {
    let max_len = (4 * bytes.len()).max(1 << 16);
    for edit in edits {
        let at = |at: u16| usize::from(at) % (bytes.len() + 1);
        match edit {
            Edit::Overwrite {
                at: pos,
                bytes: patch,
            } => {
                let start = at(*pos);
                let end = (start + patch.len()).min(bytes.len());
                bytes[start..end].copy_from_slice(&patch[..end - start]);
            },
            Edit::Insert { at: pos, len, fill } => {
                let start = at(*pos);
                let len = usize::from(*len).min(max_len.saturating_sub(bytes.len()));
                bytes.splice(start..start, std::iter::repeat_n(*fill, len));
            },
            Edit::Truncate { at: pos } => bytes.truncate(at(*pos)),
        }
    }
    bytes
}
