//! The chain service: the store's records, for other API services to fill their gaps from.

use chain_api::{
    proto::{self, chain_service_server::ChainService},
    records::{RECORD_RANGE_LIMIT, to_proto},
};
use espresso_api::error::to_status;
use futures::stream::{self, BoxStream, StreamExt};
use tonic::{Request, Response, Status};

use crate::{ApiService, request::range};

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
