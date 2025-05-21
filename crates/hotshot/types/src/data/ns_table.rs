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
    let mut result = vec![];
    if bytes.len() < NUM_NSS_BYTE_LEN
        || (bytes.len() - NUM_NSS_BYTE_LEN) % (NS_OFFSET_BYTE_LEN + NS_ID_BYTE_LEN) != 0
    {
        tracing::warn!("Failed to parse the metadata as namespace table. Use a single namespace table instead.");
        return vec![(0..payload_byte_len)];
    }
    let num_entries = u32::from_le_bytes(bytes[..NUM_NSS_BYTE_LEN].try_into().unwrap()) as usize;
    if num_entries
        != bytes.len().saturating_sub(NUM_NSS_BYTE_LEN)
            / NS_ID_BYTE_LEN.saturating_add(NS_OFFSET_BYTE_LEN)
        || (num_entries == 0 && payload_byte_len != 0)
    {
        tracing::warn!("Failed to parse the metadata as namespace table. Use a single namespace table instead.");
        return vec![(0..payload_byte_len)];
    }
    // Early breaks for empty payload and namespace table
    if num_entries == 0 {
        return vec![(0..payload_byte_len)];
    }
    let mut l = 0;
    for i in 0..num_entries {
        let offset = NUM_NSS_BYTE_LEN + i * (NS_ID_BYTE_LEN + NS_OFFSET_BYTE_LEN) + NS_ID_BYTE_LEN;
        let r = u32::from_le_bytes(
            bytes[offset..offset + NS_OFFSET_BYTE_LEN]
                .try_into()
                .unwrap(),
        ) as usize;
        if r < l || r > payload_byte_len {
            tracing::warn!("Failed to parse the metadata as namespace table. Use a single namespace table instead.");
            return vec![(0..payload_byte_len)];
        }
        result.push(l..r);
        l = r;
    }
    if l != payload_byte_len {
        tracing::warn!("Failed to parse the metadata as namespace table. Use a single namespace table instead.");
        return vec![(0..payload_byte_len)];
    }
    result
}
