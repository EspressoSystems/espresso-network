//! gRPC service implementation
//!
//! Thin wrapper around shared handler functions from the handlers module.
//! All business logic is in handlers, this just adapts to the tonic interface.

use serialization_api::v2::{
    GetLatestRewardAccountProofRequest, GetLatestRewardBalanceRequest,
    GetRewardAccountProofRequest, GetRewardAmountsRequest, GetRewardBalanceRequest,
    GetRewardClaimInputRequest, GetRewardMerkleTreeRequest, RewardAccountQueryDataV2,
    RewardAmounts, RewardBalance, RewardClaimInput, RewardMerkleTreeV2Data,
};
use tonic::{Request, Response, Status};

use crate::{handlers, proto::reward_service_server::{RewardService, RewardServiceServer}, v2};

/// gRPC reward service implementation wrapping a RewardApi implementation
pub struct RewardServiceImpl<S> {
    state: S,
}

impl<S> RewardServiceImpl<S> {
    pub fn new(state: S) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl<S> RewardService for RewardServiceImpl<S>
where
    S: v2::RewardApi + Send + Sync + 'static,
{
    async fn get_reward_claim_input(
        &self,
        request: Request<GetRewardClaimInputRequest>,
    ) -> Result<Response<RewardClaimInput>, Status> {
        handlers::get_reward_claim_input(&self.state, request.into_inner())
            .await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_reward_balance(
        &self,
        request: Request<GetRewardBalanceRequest>,
    ) -> Result<Response<RewardBalance>, Status> {
        handlers::get_reward_balance(&self.state, request.into_inner())
            .await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_latest_reward_balance(
        &self,
        request: Request<GetLatestRewardBalanceRequest>,
    ) -> Result<Response<RewardBalance>, Status> {
        handlers::get_latest_reward_balance(&self.state, request.into_inner())
            .await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_reward_account_proof(
        &self,
        request: Request<GetRewardAccountProofRequest>,
    ) -> Result<Response<RewardAccountQueryDataV2>, Status> {
        handlers::get_reward_account_proof(&self.state, request.into_inner())
            .await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_latest_reward_account_proof(
        &self,
        request: Request<GetLatestRewardAccountProofRequest>,
    ) -> Result<Response<RewardAccountQueryDataV2>, Status> {
        handlers::get_latest_reward_account_proof(&self.state, request.into_inner())
            .await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_reward_amounts(
        &self,
        request: Request<GetRewardAmountsRequest>,
    ) -> Result<Response<RewardAmounts>, Status> {
        handlers::get_reward_amounts(&self.state, request.into_inner())
            .await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

    async fn get_reward_merkle_tree_v2(
        &self,
        request: Request<GetRewardMerkleTreeRequest>,
    ) -> Result<Response<RewardMerkleTreeV2Data>, Status> {
        handlers::get_reward_merkle_tree_v2(&self.state, request.into_inner())
            .await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }
}

/// Create the reward gRPC service
pub fn create_reward_service<S>(state: S) -> RewardServiceServer<RewardServiceImpl<S>>
where
    S: v2::RewardApi + Send + Sync + Clone + 'static,
{
    RewardServiceServer::new(RewardServiceImpl::new(state))
}
