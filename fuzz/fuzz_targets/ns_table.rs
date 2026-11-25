//! Namespace table and payload bytes from a block header and payload.
//!
//! Two parsers read a namespace table: hotshot's `parse_ns_table`, which picks
//! the ranges VID disperses, and espresso's `NsTable`, which namespace and
//! transaction proofs index into. Any table espresso accepts for a payload
//! must give both the same ranges, or proofs point at bytes VID never
//! committed to as that namespace. Walking the payload through a table that
//! passed deserialization must never panic.

#![no_main]

use arbitrary::Arbitrary;
use espresso_types::{NsTable, Payload};
use hotshot_query_service_types::availability::QueryablePayload;
use hotshot_types::{
    data::ns_table::{ns_table_payload_byte_len, parse_ns_table},
    traits::BlockPayload,
};
use libfuzzer_sys::fuzz_target;

/// Largest payload the target pads out to, so a table claiming a huge final
/// offset cannot turn one input into a multi-megabyte allocation.
const MAX_FITTED_PAYLOAD: usize = 1 << 20;

#[derive(Arbitrary, Debug)]
struct Input {
    ns_table: Table,
    payload: Vec<u8>,
    /// Pad or cut the payload to the table's final offset, the only length
    /// `NsTable::validate` accepts.
    fit_payload: bool,
}

/// Random bytes almost never pass `NsTable` deserialization, which needs a
/// matching header, strictly increasing offsets and unique ids, so an input
/// can instead describe the entries and let the target encode them.
#[derive(Arbitrary, Debug)]
enum Table {
    Raw(Vec<u8>),
    Entries {
        /// Header count, the real entry count when `None`.
        declared_len: Option<u32>,
        /// Namespace id and the step from the previous offset. A zero step
        /// repeats an offset.
        entries: Vec<(u32, u16)>,
    },
}

impl Table {
    fn encode(&self) -> Vec<u8> {
        match self {
            Table::Raw(bytes) => bytes.clone(),
            Table::Entries {
                declared_len,
                entries,
            } => {
                let len = declared_len.unwrap_or(entries.len() as u32);
                let mut bytes = len.to_le_bytes().to_vec();
                let mut offset = 0u32;
                for &(ns_id, step) in entries {
                    offset = offset.saturating_add(u32::from(step));
                    bytes.extend_from_slice(&ns_id.to_le_bytes());
                    bytes.extend_from_slice(&offset.to_le_bytes());
                }
                bytes
            },
        }
    }
}

fuzz_target!(|input: Input| {
    let table_bytes = input.ns_table.encode();
    let mut payload_bytes = input.payload;
    if input.fit_payload {
        let fitted =
            ns_table_payload_byte_len(&table_bytes).filter(|&len| len <= MAX_FITTED_PAYLOAD);
        if let Some(len) = fitted {
            payload_bytes.resize(len, 0);
        }
    }

    let payload_len = payload_bytes.len();
    let ranges = parse_ns_table(payload_len, &table_bytes);

    let Ok(ns_table) = bincode::deserialize::<NsTable>(&length_prefixed(&table_bytes)) else {
        return;
    };
    let payload = Payload::from_bytes(&payload_bytes, &ns_table);
    let byte_len = payload.byte_len();
    for index in ns_table.iter() {
        let range = ns_table.ns_range(&index, &byte_len).as_block_range();
        assert!(range.start <= range.end && range.end <= payload_len);
    }
    for index in payload.iter(&ns_table) {
        assert!(
            payload.transaction(&index).is_some(),
            "iterator yielded {index:?} with no transaction"
        );
    }

    if ns_table.validate(&byte_len).is_err() {
        return;
    }
    let espresso_ranges: Vec<_> = ns_table
        .iter()
        .map(|index| ns_table.ns_range(&index, &byte_len).as_block_range())
        .collect();
    // An empty payload has no namespaces for espresso, while VID disperses it
    // as one empty namespace.
    let one_empty_namespace = matches!(ranges.as_slice(), [range] if range.is_empty());
    if !(espresso_ranges.is_empty() && one_empty_namespace) {
        assert_eq!(espresso_ranges, ranges);
    }
});

/// `bytes` as bincode encodes a byte string, the form a header carries its
/// namespace table in.
fn length_prefixed(bytes: &[u8]) -> Vec<u8> {
    let mut encoded = (bytes.len() as u64).to_le_bytes().to_vec();
    encoded.extend_from_slice(bytes);
    encoded
}
