//! `TokenService`.

use super::*;

#[tonic::async_trait]
impl<D> proto::token_service_server::TokenService for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: TokenDataSource<SeqTypes> + NodeStateDataSource + Send + Sync,
{
    async fn get_total_minted_supply(
        &self,
        _request: tonic::Request<proto::GetTotalMintedSupplyRequest>,
    ) -> Result<tonic::Response<proto::TotalMintedSupplyResponse>, tonic::Status> {
        let amount = <Self as v1::TokenApi>::total_minted_supply(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::TotalMintedSupplyResponse {
            amount,
        }))
    }

    async fn get_circulating_supply(
        &self,
        _request: tonic::Request<proto::GetCirculatingSupplyRequest>,
    ) -> Result<tonic::Response<proto::CirculatingSupplyResponse>, tonic::Status> {
        let amount = <Self as v1::TokenApi>::circulating_supply(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::CirculatingSupplyResponse {
            amount,
        }))
    }

    async fn get_circulating_supply_ethereum(
        &self,
        _request: tonic::Request<proto::GetCirculatingSupplyEthereumRequest>,
    ) -> Result<tonic::Response<proto::CirculatingSupplyEthereumResponse>, tonic::Status> {
        let amount = <Self as v1::TokenApi>::circulating_supply_ethereum(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(
            proto::CirculatingSupplyEthereumResponse { amount },
        ))
    }

    async fn get_total_issued_supply(
        &self,
        _request: tonic::Request<proto::GetTotalIssuedSupplyRequest>,
    ) -> Result<tonic::Response<proto::TotalIssuedSupplyResponse>, tonic::Status> {
        let amount = <Self as v1::TokenApi>::total_issued_supply(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::TotalIssuedSupplyResponse {
            amount,
        }))
    }

    async fn get_total_reward_distributed(
        &self,
        _request: tonic::Request<proto::GetTotalRewardDistributedRequest>,
    ) -> Result<tonic::Response<proto::TotalRewardDistributedResponse>, tonic::Status> {
        let amount = <Self as v1::TokenApi>::total_reward_distributed(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(
            proto::TotalRewardDistributedResponse { amount },
        ))
    }
}
