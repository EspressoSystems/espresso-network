//! Namespaced AvidmGf2 shares and common data as a Byzantine peer sends them.
//!
//! An honest dispersal is serialized, edited, and decoded back. The decoded
//! common and shares then go through verification and recovery the way the
//! reconstructor uses them: recovery runs on shares whose Merkle proofs were
//! never checked. Besides not panicking or over-allocating, shares that do
//! verify against the honest commitment and cover the threshold with distinct
//! shards must recover the original payload.

#![no_main]

use std::{collections::HashSet, ops::Range};

use arbitrary::Arbitrary;
use espresso_fuzz::{Edit, apply_edits};
use libfuzzer_sys::fuzz_target;
use vid::avidm_gf2::namespaced::{NsAvidmGf2Common, NsAvidmGf2Scheme, NsAvidmGf2Share};

const MAX_NODES: usize = 8;
const MAX_NAMESPACES: usize = 4;

#[derive(Arbitrary, Debug)]
struct Input {
    weights: Vec<u8>,
    payload: Vec<u8>,
    ns_ends: Vec<u16>,
    edits: Vec<Edit>,
}

fuzz_target!(|input: Input| {
    let weights: Vec<u32> = input
        .weights
        .iter()
        .take(MAX_NODES)
        .map(|w| 1 + u32::from(*w % 4))
        .collect();
    if weights.is_empty() {
        return;
    }
    let total_weight: usize = weights.iter().map(|&w| w as usize).sum();
    let recovery_threshold = total_weight.div_ceil(3);
    let param = NsAvidmGf2Scheme::setup(recovery_threshold, total_weight).unwrap();
    let ns_table = ns_table(&input.ns_ends, input.payload.len());
    let (commit, common, shares) =
        NsAvidmGf2Scheme::ns_disperse(&param, &weights, &input.payload, ns_table).unwrap();

    let honest = bincode::serialize(&(&common, &shares)).unwrap();
    let Ok((received_common, received_shares)) = bincode::deserialize::<(
        NsAvidmGf2Common,
        Vec<NsAvidmGf2Share>,
    )>(&apply_edits(honest, &input.edits)) else {
        return;
    };

    let _ = received_common.payload_byte_len();
    let verified: Vec<NsAvidmGf2Share> = received_shares
        .iter()
        .filter(|share| {
            matches!(
                NsAvidmGf2Scheme::verify_share(&commit, &received_common, share),
                Ok(Ok(()))
            )
        })
        .cloned()
        .collect();
    let recovered = NsAvidmGf2Scheme::recover(&received_common, &received_shares);
    for ns_index in 0..received_common.ns_lens.len().min(MAX_NAMESPACES + 1) {
        let _ = NsAvidmGf2Scheme::ns_recover(&received_common, ns_index, &received_shares);
    }

    if received_common == common && covers_threshold(&verified, recovery_threshold) {
        assert_eq!(
            NsAvidmGf2Scheme::recover(&common, &verified).ok(),
            Some(input.payload.clone()),
            "verified shares covering the threshold must recover the payload"
        );
    }
    if received_common == common && received_shares == shares {
        assert_eq!(recovered.ok(), Some(input.payload));
    }
});

/// Namespace ranges covering `payload_len`, ending at the given offsets
/// clamped into the payload. Empty namespaces are dropped, as a block never
/// has one.
fn ns_table(ns_ends: &[u16], payload_len: usize) -> Vec<Range<usize>> {
    let mut ends: Vec<usize> = ns_ends
        .iter()
        .take(MAX_NAMESPACES - 1)
        .map(|&end| usize::from(end) % (payload_len + 1))
        .chain([payload_len])
        .collect();
    ends.sort_unstable();
    ends.dedup();
    let mut start = 0;
    ends.into_iter()
        .filter(|&end| end > 0 || payload_len == 0)
        .map(|end| {
            let range = start..end;
            start = end;
            range
        })
        .collect()
}

/// Whether `shares` hold at least `threshold` distinct shard indices with no
/// index claimed twice. Every namespace shares the same range, so the first
/// namespace's range stands for the whole share.
fn covers_threshold(shares: &[NsAvidmGf2Share], threshold: usize) -> bool {
    let mut seen = HashSet::new();
    for share in shares {
        let Some(range) = share.range() else {
            return false;
        };
        if !range.clone().all(|index| seen.insert(index)) {
            return false;
        }
    }
    seen.len() >= threshold
}
