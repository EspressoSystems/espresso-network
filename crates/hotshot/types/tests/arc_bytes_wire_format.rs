//! `crate::arc_bytes` must not change what `DaProposal::encoded_transactions` puts on the wire.
//!
//! DA proposals travel as bincode: `versions::encode` writes a four-byte version and then calls
//! `bincode::serialize_into`. serde's slice impl emits a length prefix and then one element per
//! byte, and `serialize_bytes` emits a length prefix and then the bytes; bincode writes those
//! identically, which is what makes the swap safe. These tests hold that claim to the bytes
//! rather than to the argument.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// How the field was declared before `arc_bytes`: serde's slice impl, via `Arc`'s deref.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct AsSeq {
    view: u64,
    payload: Arc<[u8]>,
    epoch: Option<u64>,
}

/// How it is declared now.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct AsBytes {
    view: u64,
    #[serde(with = "hotshot_types::arc_bytes")]
    payload: Arc<[u8]>,
    epoch: Option<u64>,
}

fn payloads() -> Vec<Vec<u8>> {
    vec![
        vec![],
        vec![0],
        (0..=255u8).collect(),
        (0..4096).map(|i| (i % 251) as u8).collect(),
    ]
}

#[test]
fn bincode_encoding_is_unchanged() {
    for payload in payloads() {
        let as_seq = AsSeq {
            view: 7,
            payload: Arc::from(payload.clone()),
            epoch: Some(3),
        };
        let as_bytes = AsBytes {
            view: 7,
            payload: Arc::from(payload.clone()),
            epoch: Some(3),
        };

        assert_eq!(
            bincode::serialize(&as_seq).unwrap(),
            bincode::serialize(&as_bytes).unwrap(),
            "arc_bytes changed the bincode encoding for a {}-byte payload",
            payload.len()
        );
    }
}

#[test]
fn bincode_reads_what_the_old_encoding_wrote() {
    for payload in payloads() {
        let old = AsSeq {
            view: 7,
            payload: Arc::from(payload.clone()),
            epoch: Some(3),
        };
        let wire = bincode::serialize(&old).unwrap();

        let new: AsBytes = bincode::deserialize(&wire).unwrap();
        assert_eq!(new.payload.as_ref(), payload.as_slice());

        // and the other direction: an old build reading what this one writes
        let back: AsSeq = bincode::deserialize(&bincode::serialize(&new).unwrap()).unwrap();
        assert_eq!(back, old);
    }
}

/// JSON is not the DA wire format, but nothing stops a tool from using it, and
/// `serde_json::serialize_bytes` collects a sequence too, so that encoding is also unchanged.
#[test]
fn json_encoding_is_unchanged() {
    for payload in payloads() {
        let as_seq = AsSeq {
            view: 7,
            payload: Arc::from(payload.clone()),
            epoch: None,
        };
        let as_bytes = AsBytes {
            view: 7,
            payload: Arc::from(payload.clone()),
            epoch: None,
        };

        let from_seq = serde_json::to_string(&as_seq).unwrap();
        assert_eq!(
            from_seq,
            serde_json::to_string(&as_bytes).unwrap(),
            "arc_bytes changed the JSON encoding for a {}-byte payload",
            payload.len()
        );

        // and the visitor reads back what either wrote
        let parsed: AsBytes = serde_json::from_str(&from_seq).unwrap();
        assert_eq!(parsed.payload.as_ref(), payload.as_slice());
    }
}
