use hotshot::traits::BlockPayload;
use serde::Serialize;
use serde_json::Value;

use crate::{NamespaceId, NsTable, Payload, SeqTypes, Transaction, v0_3::ChainConfig};

#[derive(Serialize)]
struct Legacy {
    #[serde(with = "base64_bytes")]
    raw_payload: Vec<u8>,
    ns_table: NsTable,
}

fn payload(txs: Vec<Transaction>) -> Payload {
    Payload::from_transactions_sync(txs, ChainConfig::default())
        .unwrap()
        .0
}

fn sample_payload() -> Payload {
    payload(vec![
        Transaction::new(NamespaceId::from(1u32), vec![1, 2, 3]),
        Transaction::new(NamespaceId::from(2u32), vec![4, 5]),
        Transaction::new(NamespaceId::from(1u32), vec![6; 100]),
    ])
}

fn empty_payload() -> Payload {
    <Payload as BlockPayload<SeqTypes>>::empty().0
}

#[test]
fn bincode_round_trip() {
    let p = sample_payload();
    let decoded: Payload = bincode::deserialize(&bincode::serialize(&p).unwrap()).unwrap();
    assert_eq!(decoded, p);
}

#[test]
fn bincode_decodes_legacy_encoding() {
    let p = sample_payload();
    let legacy = Legacy {
        raw_payload: p.raw_payload.to_vec(),
        ns_table: p.ns_table.clone(),
    };
    let decoded: Payload = bincode::deserialize(&bincode::serialize(&legacy).unwrap()).unwrap();
    assert_eq!(decoded, p);
}

#[test]
fn json_round_trip() {
    let p = sample_payload();
    let json = serde_json::to_value(&p).unwrap();
    assert!(json["raw_payload"].is_string());
    assert_eq!(serde_json::from_value::<Payload>(json).unwrap(), p);
}

#[test]
fn empty_payload_round_trips() {
    for p in [payload(vec![]), empty_payload()] {
        let decoded: Payload = bincode::deserialize(&bincode::serialize(&p).unwrap()).unwrap();
        assert_eq!(decoded, p);

        let json: Value = serde_json::to_value(&p).unwrap();
        assert_eq!(serde_json::from_value::<Payload>(json).unwrap(), p);
    }
}
