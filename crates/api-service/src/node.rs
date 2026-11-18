//! The node service: the chain height, and the API services of the validators.

use consensus_api::{DecideLogClient, proto::ServiceRecordsRequest};
use espresso_api::{
    error::to_status,
    proto::{self, node_service_server::NodeService},
};
use espresso_types::SeqTypes;
use hotshot_new_protocol::discovery::SignedServiceRecord;
use tagged_base64::TaggedBase64;
use tonic::{Code, Request, Response, Status};

use crate::ApiService;

fn unimplemented<T>(what: &str) -> Result<T, Status> {
    Err(Status::unimplemented(format!(
        "this API service does not serve {what} yet"
    )))
}

/// `signed` as clients receive it; they check the signature against the stake table of the
/// record's epoch themselves.
fn listed(signed: SignedServiceRecord<SeqTypes>) -> proto::ServiceRecord {
    proto::ServiceRecord {
        signer: Some(proto::BlsPublicKey {
            key: signed.signer.to_string(),
        }),
        epoch: *signed.record.epoch,
        signed_message: signed.record.signed_bytes(),
        api_url: signed.record.api_url,
        api_key: signed.record.api_key.map(|key| key.to_vec()),
        sequence: signed.record.sequence,
        signature: TaggedBase64::from(signed.signature).to_string(),
    }
}

#[tonic::async_trait]
impl NodeService for ApiService {
    async fn get_node_block_height(
        &self,
        _: Request<proto::GetNodeBlockHeightRequest>,
    ) -> Result<Response<proto::NodeBlockHeightResponse>, Status> {
        let newest = self.reader.store().newest().await.map_err(to_status)?;
        Ok(Response::new(proto::NodeBlockHeightResponse {
            height: newest.map_or(0, |h| h + 1),
        }))
    }

    /// The records consensus holds for the epoch, as it received them.
    async fn get_peers(
        &self,
        request: Request<proto::GetPeersRequest>,
    ) -> Result<Response<proto::PeersResponse>, Status> {
        let mut client = DecideLogClient::connect(format!("http://{}", self.consensus_api))
            .await
            .map_err(|err| Status::unavailable(format!("cannot reach consensus: {err}")))?;
        let response = client
            .get_service_records(ServiceRecordsRequest {
                epoch: request.into_inner().epoch,
            })
            .await
            .map_err(|status| match status.code() {
                Code::NotFound => Status::not_found("no service records for the epoch"),
                _ => Status::unavailable(format!("asking consensus for records: {status}")),
            })?
            .into_inner();
        let peers = response
            .records
            .iter()
            .filter_map(|bytes| {
                bincode::deserialize::<SignedServiceRecord<SeqTypes>>(bytes)
                    .inspect_err(|err| tracing::warn!(%err, "undecodable service record"))
                    .ok()
            })
            .filter(SignedServiceRecord::is_valid)
            .map(listed)
            .collect();
        Ok(Response::new(proto::PeersResponse {
            peers,
            epoch: response.epoch,
        }))
    }

    async fn get_transaction_count(
        &self,
        _: Request<proto::GetTransactionCountRequest>,
    ) -> Result<Response<proto::TransactionCountResponse>, Status> {
        unimplemented("transaction counts")
    }

    async fn get_payload_size(
        &self,
        _: Request<proto::GetPayloadSizeRequest>,
    ) -> Result<Response<proto::PayloadSizeResponse>, Status> {
        unimplemented("payload sizes")
    }

    async fn get_sync_status(
        &self,
        _: Request<proto::GetSyncStatusRequest>,
    ) -> Result<Response<proto::SyncStatusResponse>, Status> {
        unimplemented("sync status")
    }

    async fn get_block_reward(
        &self,
        _: Request<proto::GetBlockRewardRequest>,
    ) -> Result<Response<proto::BlockRewardResponse>, Status> {
        unimplemented("block rewards")
    }

    async fn get_header_window(
        &self,
        _: Request<proto::GetHeaderWindowRequest>,
    ) -> Result<Response<proto::HeaderWindowResponse>, Status> {
        unimplemented("header windows")
    }

    async fn get_vid_share(
        &self,
        _: Request<proto::GetVidShareRequest>,
    ) -> Result<Response<proto::VidShareResponse>, Status> {
        unimplemented("VID shares")
    }

    async fn get_node_limits(
        &self,
        _: Request<proto::GetNodeLimitsRequest>,
    ) -> Result<Response<proto::NodeLimitsResponse>, Status> {
        unimplemented("node limits")
    }

    async fn get_stake_table(
        &self,
        _: Request<proto::GetStakeTableRequest>,
    ) -> Result<Response<proto::StakeTableResponse>, Status> {
        unimplemented("stake tables")
    }

    async fn get_validators(
        &self,
        _: Request<proto::GetValidatorsRequest>,
    ) -> Result<Response<proto::ValidatorsResponse>, Status> {
        unimplemented("validators")
    }

    async fn get_all_validators(
        &self,
        _: Request<proto::GetAllValidatorsRequest>,
    ) -> Result<Response<proto::ValidatorsResponse>, Status> {
        unimplemented("validators")
    }

    async fn get_proposal_participation(
        &self,
        _: Request<proto::GetProposalParticipationRequest>,
    ) -> Result<Response<proto::ParticipationResponse>, Status> {
        unimplemented("participation")
    }

    async fn get_vote_participation(
        &self,
        _: Request<proto::GetVoteParticipationRequest>,
    ) -> Result<Response<proto::ParticipationResponse>, Status> {
        unimplemented("participation")
    }
}
