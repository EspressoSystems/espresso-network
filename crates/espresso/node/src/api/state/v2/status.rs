//! `StatusService`.

use super::*;

#[tonic::async_trait]
impl<D> proto::status_service_server::StatusService for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: hotshot_query_service::status::StatusDataSource + NodeKeysDataSource + Send + Sync,
{
    async fn get_block_height(
        &self,
        _request: tonic::Request<proto::GetBlockHeightRequest>,
    ) -> Result<tonic::Response<proto::BlockHeightResponse>, tonic::Status> {
        let height = <Self as v1::StatusApi>::block_height(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::BlockHeightResponse { height }))
    }

    async fn get_success_rate(
        &self,
        _request: tonic::Request<proto::GetSuccessRateRequest>,
    ) -> Result<tonic::Response<proto::SuccessRateResponse>, tonic::Status> {
        let rate = <Self as v1::StatusApi>::success_rate(self)
            .await
            .map_err(to_status)?;
        // A fresh node computes 0/0 and a restarted one height/0 until its first view tick
        // (the view gauge is in-memory, the height persisted). protoJSON cannot encode a
        // non-finite double and the generated deserializer rejects `null`, so clamp to zero.
        let rate = if rate.is_finite() { rate } else { 0. };
        Ok(tonic::Response::new(proto::SuccessRateResponse { rate }))
    }

    async fn get_time_since_last_decide(
        &self,
        _request: tonic::Request<proto::GetTimeSinceLastDecideRequest>,
    ) -> Result<tonic::Response<proto::TimeSinceLastDecideResponse>, tonic::Status> {
        let seconds = <Self as v1::StatusApi>::time_since_last_decide(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::TimeSinceLastDecideResponse {
            seconds,
        }))
    }

    async fn get_node_keys(
        &self,
        _request: tonic::Request<proto::GetNodeKeysRequest>,
    ) -> Result<tonic::Response<proto::NodeKeysResponse>, tonic::Status> {
        let keys = <Self as v1::StatusApi>::keys(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::NodeKeysResponse {
            eth_account: keys.eth_account.map(|account| format!("{account:#x}")),
            consensus_key: Some(proto::BlsPublicKey {
                key: keys.consensus_key.to_string(),
            }),
            state_ver_key: Some(proto::SchnorrPublicKey {
                key: keys.state_ver_key.to_string(),
            }),
            x25519_key: keys.x25519_key.as_ref().map(ToString::to_string),
            p2p_addr: keys.p2p_addr.as_ref().map(ToString::to_string),
        }))
    }
}
