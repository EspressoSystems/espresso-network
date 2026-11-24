//! The discovery service: the API services of the validators, as their signed records name them.

use chain_api::{
    discovery::listed,
    proto::{self, discovery_service_server::DiscoveryService},
};
use espresso_api::error::to_status;
use hotshot_types::data::EpochNumber;
use tonic::{Request, Response, Status};

use crate::ApiService;

#[tonic::async_trait]
impl DiscoveryService for ApiService {
    /// The records of the epoch, or of the latest epoch with records, as the store got them
    /// from its consensus node.
    async fn get_peers(
        &self,
        request: Request<proto::GetPeersRequest>,
    ) -> Result<Response<proto::PeersResponse>, Status> {
        let epoch = request.into_inner().epoch.map(EpochNumber::new);
        let (epoch, records) = self
            .tables
            .service_records(epoch)
            .await
            .map_err(to_status)?
            .ok_or_else(|| Status::not_found("no service records for the epoch"))?;
        Ok(Response::new(proto::PeersResponse {
            peers: records.into_iter().map(listed).collect(),
            epoch: *epoch,
        }))
    }
}

#[cfg(test)]
mod tests {
    use chain_api::Tables;
    use chain_store::{ChainStore, mem::MemBackend};
    use consensus_api::discovery::{ServiceRecord, SignedServiceRecord};
    use espresso_types::{PubKey, SeqTypes};
    use hotshot_types::traits::signature_key::SignatureKey;

    use super::*;

    /// The records come from the store, for the epoch asked or the latest one.
    #[tokio::test]
    async fn lists_the_stored_records() {
        let store = ChainStore::new(MemBackend::default());
        let (public, private) = PubKey::generated_from_seed_indexed([0; 32], 0);
        let record: SignedServiceRecord<SeqTypes> = ServiceRecord {
            epoch: EpochNumber::new(3),
            api_url: "http://127.0.0.1:1".into(),
            api_key: None,
            sequence: 1,
        }
        .sign(public, &private)
        .unwrap();
        Tables::new(store.clone())
            .put_service_records(EpochNumber::new(3), &[record], EpochNumber::new(1))
            .await
            .unwrap();
        let service = ApiService::new(store, "127.0.0.1:1".parse().unwrap(), 10);

        for epoch in [Some(3), None] {
            let peers = service
                .get_peers(Request::new(proto::GetPeersRequest { epoch }))
                .await
                .unwrap()
                .into_inner();
            assert_eq!((peers.epoch, peers.peers.len()), (3, 1));
        }
        let status = service
            .get_peers(Request::new(proto::GetPeersRequest { epoch: Some(4) }))
            .await
            .expect_err("no records of epoch 4");
        assert_eq!(status.code(), tonic::Code::NotFound);
    }
}
