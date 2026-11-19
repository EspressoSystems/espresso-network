//! The chain service: the store's records, for other API services to fill their gaps from.

use chain_store::{Part, Record};
use espresso_api::error::to_status;
use futures::stream::{self, BoxStream, StreamExt};
use tonic::{Request, Response, Status};

use crate::{
    ApiService,
    proto::{self, chain_service_server::ChainService},
    request::range,
};

/// The most heights one request returns. Each carries a payload, which can be megabytes.
pub(crate) const RECORD_RANGE_LIMIT: u64 = 100;

#[tonic::async_trait]
impl ChainService for ApiService {
    type GetRecordsStream = BoxStream<'static, Result<proto::ChainRecord, Status>>;

    /// Reads one height at a time as the client takes them, so a long range holds one payload in
    /// memory at a time.
    async fn get_records(
        &self,
        request: Request<proto::GetRecordsRequest>,
    ) -> Result<Response<Self::GetRecordsStream>, Status> {
        let r = request.into_inner();
        let heights = range(r.from, r.until, RECORD_RANGE_LIMIT)?;
        let skip_payloads = r.skip_payloads;
        let store = self.reader.store().clone();
        let records = stream::iter(heights)
            .then(move |height| {
                let store = store.clone();
                async move { store.record(height).await.map_err(to_status) }
            })
            .filter_map(move |record| async move {
                match record {
                    Ok(Some(record)) => Some(Ok(to_proto(record, skip_payloads))),
                    Ok(None) => None,
                    Err(err) => Some(Err(err)),
                }
            })
            .boxed();
        Ok(Response::new(records))
    }
}

/// `record` as the chain service sends it, without its payload and VID common data if
/// `skip_payloads`.
fn to_proto(record: Record, skip_payloads: bool) -> proto::ChainRecord {
    let mut sent = proto::ChainRecord {
        height: record.height,
        item: record.item.to_vec(),
        ..Default::default()
    };
    for (part, data) in record.parts {
        let data = Some(data.to_vec());
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
pub(crate) fn from_proto(record: proto::ChainRecord) -> Record {
    let parts = [
        (Part::Payload, record.payload),
        (Part::VidCommon, record.vid_common),
        (Part::Cert2, record.cert2),
        (Part::StateCert, record.state_cert),
        (Part::CertsKnown, record.certs_known.then(Vec::new)),
    ]
    .into_iter()
    .filter_map(|(part, data)| Some((part, data?.into())))
    .collect();
    Record {
        height: record.height,
        item: record.item.into(),
        parts,
    }
}
