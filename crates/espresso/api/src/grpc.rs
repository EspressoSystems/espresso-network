//! gRPC service implementation

use serialization_api::v1::{
    GetNamespaceProofRequest, GetViewNumberRequest, NamespaceProofQueryData, ViewNumber,
};
use tonic::{Request, Response, Status};

use crate::{
    proto::status_service_server::{StatusService, StatusServiceServer},
    r#trait::{NodeApi, NodeApiState},
};

/// gRPC service implementation wrapping NodeApiState
pub struct StatusServiceImpl {
    state: NodeApiState,
}

impl StatusServiceImpl {
    pub fn new(state: NodeApiState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl StatusService for StatusServiceImpl {
    async fn get_view_number(
        &self,
        _request: Request<GetViewNumberRequest>,
    ) -> Result<Response<ViewNumber>, Status> {
        self.state
            .get_view_number()
            .await
            .map(Response::new)
            .map_err(|e| Status::internal(e.to_string()))
    }

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

/// Create the gRPC service
pub fn create_grpc_service(state: NodeApiState) -> StatusServiceServer<StatusServiceImpl> {
    StatusServiceServer::new(StatusServiceImpl::new(state))
}
