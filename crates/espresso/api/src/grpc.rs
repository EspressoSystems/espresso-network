//! gRPC service implementation

use serialization_api::v1::{
    GetNamespaceProofRequest, GetRewardClaimInputRequest, NamespaceProofQueryData, RewardClaimInput,
};
use tonic::{Request, Response, Status};

use crate::{
    proto::{
        reward_service_server::{RewardService, RewardServiceServer},
        status_service_server::{StatusService, StatusServiceServer},
    },
    r#trait::NodeApi,
};

/// gRPC service implementation wrapping a NodeApi implementation
pub struct StatusServiceImpl<S> {
    state: S,
}

impl<S> StatusServiceImpl<S> {
    pub fn new(state: S) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl<S> StatusService for StatusServiceImpl<S>
where
    S: NodeApi + Send + Sync + 'static,
{
    async fn get_namespace_proof(
        &self,
        request: Request<GetNamespaceProofRequest>,
    ) -> Result<Response<NamespaceProofQueryData>, Status> {
        let req = request.into_inner();
        self.state
            .get_namespace_proof(req.height, req.namespace)
            .await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }
}

/// gRPC reward service implementation wrapping a NodeApi implementation
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
    S: NodeApi + Send + Sync + 'static,
{
    async fn get_reward_claim_input(
        &self,
        request: Request<GetRewardClaimInputRequest>,
    ) -> Result<Response<RewardClaimInput>, Status> {
        let req = request.into_inner();
        self.state
            .get_reward_claim_input(req.block_height, req.address)
            .await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }
}

/// Create the status gRPC service
pub(crate) fn create_status_service<S>(state: S) -> StatusServiceServer<StatusServiceImpl<S>>
where
    S: NodeApi + Send + Sync + Clone + 'static,
{
    StatusServiceServer::new(StatusServiceImpl::new(state))
}

/// Create the reward gRPC service
pub(crate) fn create_reward_service<S>(state: S) -> RewardServiceServer<RewardServiceImpl<S>>
where
    S: NodeApi + Send + Sync + Clone + 'static,
{
    RewardServiceServer::new(RewardServiceImpl::new(state))
}
