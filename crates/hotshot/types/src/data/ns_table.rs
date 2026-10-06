// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! This module provides helpers for namespace table.

use std::ops::Range;

/// Byte lengths for the different items that could appear in a namespace table.
const NUM_NSS_BYTE_LEN: usize = 4;
const NS_OFFSET_BYTE_LEN: usize = 4;
const NS_ID_BYTE_LEN: usize = 4;

/// Helper function for AvidM scheme to parse a namespace table.
/// If the namespace table is invalid, it returns a default single entry namespace table.
/// For details, please refer to `block/full_payload/ns_table.rs` in the `sequencer` crate.
#[allow(clippy::single_range_in_vec_init)]
pub fn parse_ns_table(payload_byte_len: usize, bytes: &[u8]) -> Vec<Range<usize>> {
    let Some(offsets) = ns_table_offsets(bytes) else {
        return single_ns_table(payload_byte_len);
    };
    // Early breaks for empty payload and namespace table
    if offsets.is_empty() && payload_byte_len == 0 {
        return vec![(0..0)];
    }
    if offsets.last() != Some(&payload_byte_len) {
        return single_ns_table(payload_byte_len);
    }
    let mut start = 0;
    offsets
        .into_iter()
        .map(|end| {
            let range = start..end;
            start = end;
            range
        })
        .collect()
}

/// The payload length `bytes` claims to be the namespace table of: its final
/// offset, or zero for a table with no entries. `None` if `bytes` is not a
/// namespace table at all.
///
/// [`parse_ns_table`] substitutes one namespace spanning the payload both for
/// bytes that are no table and for a table sized for another payload. A
/// verifier that learned the payload's length from the payload itself must
/// tell the two apart: for a one-namespace payload the substitute is the
/// honest table, so a table claiming a different length would otherwise pass.
pub fn ns_table_payload_byte_len(bytes: &[u8]) -> Option<usize> {
    ns_table_offsets(bytes).map(|offsets| offsets.last().copied().unwrap_or(0))
}

/// The namespace end offsets `bytes` encodes, in order. `None` if `bytes` is
/// not a namespace table: too short, an entry count that disagrees with its
/// length, or offsets that decrease.
fn ns_table_offsets(bytes: &[u8]) -> Option<Vec<usize>> {
    const ENTRY_BYTE_LEN: usize = NS_ID_BYTE_LEN + NS_OFFSET_BYTE_LEN;
    if bytes.len() < NUM_NSS_BYTE_LEN
        || !(bytes.len() - NUM_NSS_BYTE_LEN).is_multiple_of(ENTRY_BYTE_LEN)
    {
        return None;
    }
    let num_entries = u32::from_le_bytes(bytes[..NUM_NSS_BYTE_LEN].try_into().unwrap()) as usize;
    if num_entries != (bytes.len() - NUM_NSS_BYTE_LEN) / ENTRY_BYTE_LEN {
        return None;
    }
    let mut offsets = Vec::with_capacity(num_entries);
    let mut last = 0;
    for i in 0..num_entries {
        let offset = NUM_NSS_BYTE_LEN + i * ENTRY_BYTE_LEN + NS_ID_BYTE_LEN;
        let end = u32::from_le_bytes(
            bytes[offset..offset + NS_OFFSET_BYTE_LEN]
                .try_into()
                .unwrap(),
        ) as usize;
        if end < last {
            return None;
        }
        offsets.push(end);
        last = end;
    }
    Some(offsets)
}

#[allow(clippy::single_range_in_vec_init)]
fn single_ns_table(payload_byte_len: usize) -> Vec<Range<usize>> {
    tracing::debug!(
        "Failed to parse the metadata as namespace table. Use a single namespace table instead."
    );
    vec![(0..payload_byte_len)]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode a namespace table whose namespaces end at `offsets`.
    fn ns_table(offsets: &[u32]) -> Vec<u8> {
        let mut bytes = (offsets.len() as u32).to_le_bytes().to_vec();
        for (id, offset) in offsets.iter().enumerate() {
            bytes.extend_from_slice(&(id as u32).to_le_bytes());
            bytes.extend_from_slice(&offset.to_le_bytes());
        }
        bytes
    }

    /// A table whose entry count disagrees with its length.
    fn miscounted() -> Vec<u8> {
        let mut bytes = ns_table(&[30, 100]);
        bytes[..NUM_NSS_BYTE_LEN].copy_from_slice(&1u32.to_le_bytes());
        bytes
    }

    #[test]
    fn parse_ns_table_slices_a_table_sized_for_the_payload() {
        assert_eq!(
            parse_ns_table(100, &ns_table(&[30, 100])),
            vec![0..30, 30..100]
        );
        assert_eq!(
            parse_ns_table(100, &ns_table(&[0, 100])),
            vec![0..0, 0..100]
        );
        assert_eq!(parse_ns_table(100, &ns_table(&[100])), vec![0..100]);
        assert_eq!(parse_ns_table(0, &ns_table(&[])), vec![0..0]);
    }

    #[test]
    fn parse_ns_table_falls_back_to_one_namespace() {
        // Not a table.
        assert_eq!(parse_ns_table(100, &[]), vec![0..100]);
        assert_eq!(
            parse_ns_table(100, &ns_table(&[30, 100])[..9]),
            vec![0..100]
        );
        assert_eq!(parse_ns_table(100, &miscounted()), vec![0..100]);
        assert_eq!(parse_ns_table(100, &ns_table(&[50, 30])), vec![0..100]);
        // A table, but sized for another payload.
        assert_eq!(parse_ns_table(100, &ns_table(&[30, 90])), vec![0..100]);
        assert_eq!(parse_ns_table(100, &ns_table(&[30, 110])), vec![0..100]);
        assert_eq!(parse_ns_table(100, &ns_table(&[])), vec![0..100]);
    }

    #[test]
    fn ns_table_payload_byte_len_is_the_final_offset_of_a_table() {
        assert_eq!(ns_table_payload_byte_len(&ns_table(&[30, 90])), Some(90));
        assert_eq!(ns_table_payload_byte_len(&ns_table(&[100])), Some(100));
        assert_eq!(ns_table_payload_byte_len(&ns_table(&[])), Some(0));
        assert_eq!(ns_table_payload_byte_len(&[]), None);
        assert_eq!(ns_table_payload_byte_len(&ns_table(&[30, 100])[..9]), None);
        assert_eq!(ns_table_payload_byte_len(&miscounted()), None);
        assert_eq!(ns_table_payload_byte_len(&ns_table(&[50, 30])), None);
    }
}
