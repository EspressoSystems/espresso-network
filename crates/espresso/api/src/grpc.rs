use serialization_api::v1::{
    GetNamespaceProofRequest, GetRewardClaimInputRequest, NamespaceProofQueryData,
    RewardClaimInput as ProtoRewardClaimInput,
};
use tonic::{Request, Response};

use crate::{data_source::DataSourceError, DataSource, RewardService, StatusService};

pub struct GrpcService<D> {
    ds: D,
}

impl<D> GrpcService<D> {
    pub fn new(ds: D) -> Self {
        Self { ds }
    }
}

#[tonic::async_trait]
impl<D> StatusService for GrpcService<D>
where
    D: DataSource,
{
    async fn get_namespace_proof(
        &self,
        _request: Request<GetNamespaceProofRequest>,
    ) -> Result<Response<NamespaceProofQueryData>, tonic::Status> {
        // TODO: implement
        Ok(Response::new(NamespaceProofQueryData {
            proof: None,
            transactions: vec![],
        }))
    }
}

#[tonic::async_trait]
impl<D> RewardService for GrpcService<D>
where
    D: DataSource,
{
    async fn get_reward_claim_input(
        &self,
        request: Request<GetRewardClaimInputRequest>,
    ) -> Result<Response<ProtoRewardClaimInput>, tonic::Status> {
        let req = request.into_inner();

        let data = self
            .ds
            .get_reward_claim_input(req.block_height, &req.address)
            .await
            .map_err(|e| match e {
                DataSourceError::BadRequest(msg) => tonic::Status::invalid_argument(msg),
                DataSourceError::NotFound(msg) => tonic::Status::not_found(msg),
                DataSourceError::Internal(msg) => tonic::Status::internal(msg),
            })?;

        // auth_data_hex is "0x..." hex string, decode to bytes for protobuf
        let auth_bytes = hex::decode(
            data.auth_data_hex
                .strip_prefix("0x")
                .unwrap_or(&data.auth_data_hex),
        )
        .map_err(|e| tonic::Status::internal(format!("failed to decode auth_data hex: {e}")))?;

        Ok(Response::new(ProtoRewardClaimInput {
            address: req.address,
            lifetime_rewards: data.lifetime_rewards,
            auth_data: auth_bytes,
        }))
    }
}
