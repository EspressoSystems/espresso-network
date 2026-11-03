//! `arc_bytes` must not change what `DaProposal::encoded_transactions` puts on the wire.
//!
//! `vbs::Serializer::serialize` and `versions::encode` are both a four-byte version prefix over
//! `bincode::serialize_into`, so bincode is the only encoder in play.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// The field as declared before `arc_bytes`.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct AsSeq {
    view: u64,
    payload: Arc<[u8]>,
    epoch: Option<u64>,
}

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

        let back: AsSeq = bincode::deserialize(&bincode::serialize(&new).unwrap()).unwrap();
        assert_eq!(back, old);
    }
}

/// `serde_json` writes `serialize_bytes` as a sequence too, so JSON is unchanged as well.
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

        let parsed: AsBytes = serde_json::from_str(&from_seq).unwrap();
        assert_eq!(parsed.payload.as_ref(), payload.as_slice());
    }
}

/// Same bytes through `bincode_opts`, which adds `reject_trailing_bytes` on decode.
///
/// bincode 1.x's free `serialize` is fixint; `bincode_opts` is `DefaultOptions` put back to
/// fixint, since `DefaultOptions` alone is varint.
#[test]
fn bincode_opts_encoding_is_unchanged() {
    use bincode::Options;

    for payload in payloads() {
        let as_seq = AsSeq {
            view: 7,
            payload: Arc::from(payload.clone()),
            epoch: Some(3),
        };
        let as_bytes = AsBytes {
            view: 7,
            payload: Arc::from(payload),
            epoch: Some(3),
        };

        let seq = hotshot_types::utils::bincode_opts()
            .serialize(&as_seq)
            .unwrap();
        let bytes = hotshot_types::utils::bincode_opts()
            .serialize(&as_bytes)
            .unwrap();
        assert_eq!(seq, bytes, "bincode_opts encoding differs");

        let parsed: AsBytes = hotshot_types::utils::bincode_opts()
            .deserialize(&seq)
            .unwrap();
        assert_eq!(
            parsed, as_bytes,
            "bincode_opts could not read the old encoding"
        );
    }
}
