//! gRPC service implementation
//!
//! Thin wrapper around shared handler functions from the handlers module.
//! All business logic is in handlers, this just adapts to the tonic interface.

use serialization_api::v2::{
    GetIncorrectEncodingProofRequest, GetNamespaceProofRequest, GetNamespaceProofResponse,
    GetRewardAccountProofRequest, GetRewardBalanceRequest, GetRewardBalancesRequest,
    GetRewardClaimInputRequest, GetRewardMerkleTreeRequest, GetStakeTableRequest,
    GetStateCertificateRequest, IncorrectEncodingProofResponse, RewardAccountQueryDataV2,
    RewardBalance, RewardBalances, RewardClaimInput, RewardMerkleTreeV2Data,
    StakeTableResponse, StateCertificateResponse,
};
use tonic::{Request, Response, Status};

use crate::{
    error::ApiError,
    handlers,
    proto::{
        consensus_service_server::{ConsensusService, ConsensusServiceServer},
        data_service_server::{DataService, DataServiceServer},
        reward_service_server::{RewardService, RewardServiceServer},
    },
    v2,
};

/// Convert ApiError to tonic::Status with proper status code mapping
fn map_error(err: ApiError) -> Status {
    match err {
        ApiError::BadRequest(e) => Status::invalid_argument(e.to_string()),
        ApiError::Internal(e) => Status::internal(e.to_string()),
    }
}

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
            .map_err(map_error)
    }

    async fn get_reward_balance(
        &self,
        request: Request<GetRewardBalanceRequest>,
    ) -> Result<Response<RewardBalance>, Status> {
        handlers::get_reward_balance(&self.state, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_error)
    }

    async fn get_reward_account_proof(
        &self,
        request: Request<GetRewardAccountProofRequest>,
    ) -> Result<Response<RewardAccountQueryDataV2>, Status> {
        handlers::get_reward_account_proof(&self.state, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_error)
    }

    async fn get_reward_balances(
        &self,
        request: Request<GetRewardBalancesRequest>,
    ) -> Result<Response<RewardBalances>, Status> {
        handlers::get_reward_balances(&self.state, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_error)
    }

    async fn get_reward_merkle_tree_v2(
        &self,
        request: Request<GetRewardMerkleTreeRequest>,
    ) -> Result<Response<RewardMerkleTreeV2Data>, Status> {
        handlers::get_reward_merkle_tree_v2(&self.state, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_error)
    }
}

/// Create the reward gRPC service
pub fn create_reward_service<S>(state: S) -> RewardServiceServer<RewardServiceImpl<S>>
where
    S: v2::RewardApi + Send + Sync + Clone + 'static,
{
    RewardServiceServer::new(RewardServiceImpl::new(state))
}

/// gRPC data service implementation wrapping a DataApi implementation
pub struct DataServiceImpl<S> {
    state: S,
}

impl<S> DataServiceImpl<S> {
    pub fn new(state: S) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl<S> DataService for DataServiceImpl<S>
where
    S: v2::DataApi + Send + Sync + 'static,
{
    async fn get_namespace_proof(
        &self,
        request: Request<GetNamespaceProofRequest>,
    ) -> Result<Response<GetNamespaceProofResponse>, Status> {
        handlers::get_namespace_proof(&self.state, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_error)
    }

    async fn get_incorrect_encoding_proof(
        &self,
        request: Request<GetIncorrectEncodingProofRequest>,
    ) -> Result<Response<IncorrectEncodingProofResponse>, Status> {
        handlers::get_incorrect_encoding_proof(&self.state, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_error)
    }
}

/// Create the data gRPC service
pub fn create_data_service<S>(state: S) -> DataServiceServer<DataServiceImpl<S>>
where
    S: v2::DataApi + Send + Sync + Clone + 'static,
{
    DataServiceServer::new(DataServiceImpl::new(state))
}

/// gRPC consensus service implementation wrapping a ConsensusApi implementation
pub struct ConsensusServiceImpl<S> {
    state: S,
}

impl<S> ConsensusServiceImpl<S> {
    pub fn new(state: S) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl<S> ConsensusService for ConsensusServiceImpl<S>
where
    S: v2::ConsensusApi + Send + Sync + 'static,
{
    async fn get_state_certificate(
        &self,
        request: Request<GetStateCertificateRequest>,
    ) -> Result<Response<StateCertificateResponse>, Status> {
        handlers::get_state_certificate(&self.state, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_error)
    }

    async fn get_stake_table(
        &self,
        request: Request<GetStakeTableRequest>,
    ) -> Result<Response<StakeTableResponse>, Status> {
        handlers::get_stake_table(&self.state, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_error)
    }
}

/// Create the consensus gRPC service
pub fn create_consensus_service<S>(state: S) -> ConsensusServiceServer<ConsensusServiceImpl<S>>
where
    S: v2::ConsensusApi + Send + Sync + Clone + 'static,
{
    ConsensusServiceServer::new(ConsensusServiceImpl::new(state))
}
