//! The node service: the chain height.

use espresso_api::proto::{self, node_service_server::NodeService};
use tonic::{Request, Response, Status};

use crate::ApiService;

fn unimplemented<T>(what: &str) -> Result<T, Status> {
    Err(Status::unimplemented(format!(
        "this API service does not serve {what} yet"
    )))
}

#[tonic::async_trait]
impl NodeService for ApiService {
    async fn get_node_block_height(
        &self,
        _: Request<proto::GetNodeBlockHeightRequest>,
    ) -> Result<Response<proto::NodeBlockHeightResponse>, Status> {
        let newest = self.reader.newest().await?;
        Ok(Response::new(proto::NodeBlockHeightResponse {
            height: newest.map_or(0, |h| h + 1),
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
