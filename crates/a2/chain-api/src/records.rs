//! Chain records as the chain service sends them.

use bytes::Bytes;
use chain_store::{Part, Record};

use crate::proto;

/// The most heights one request returns. Each carries a payload, which can be megabytes.
pub const RECORD_RANGE_LIMIT: u64 = 100;

/// `record` as the chain service sends it, without its payload and VID common data if
/// `skip_payloads`.
pub fn to_proto(record: Record, skip_payloads: bool) -> proto::ChainRecord {
    let mut sent = proto::ChainRecord {
        height: record.height,
        item: record.item,
        ..Default::default()
    };
    for (part, data) in record.parts {
        let data = Some(data);
        match part {
            Part::Payload if !skip_payloads => sent.payload = data,
            Part::VidCommon if !skip_payloads => sent.vid_common = data,
            Part::Payload | Part::VidCommon => {},
            Part::Cert2 => sent.cert2 = data,
            Part::StateCert => sent.state_cert = data,
            Part::CertsKnown => sent.certs_known = true,
        }
    }
    sent
}

/// `record` as the chain service received it.
pub fn from_proto(record: proto::ChainRecord) -> Record {
    let parts = [
        (Part::Payload, record.payload),
        (Part::VidCommon, record.vid_common),
        (Part::Cert2, record.cert2),
        (Part::StateCert, record.state_cert),
        (Part::CertsKnown, record.certs_known.then(Bytes::new)),
    ]
    .into_iter()
    .filter_map(|(part, data)| Some((part, data?)))
    .collect();
    Record {
        height: record.height,
        item: record.item,
        parts,
    }
}
