//! The status service: how far the store has come.

use std::time::{SystemTime, UNIX_EPOCH};

use espresso_api::proto::{self, status_service_server::StatusService};
use hotshot_types::traits::block_contents::BlockHeader;
use tonic::{Request, Response, Status};

use crate::{ApiService, reader::Wait};

#[tonic::async_trait]
impl StatusService for ApiService {
    /// The number of heights up to and including the newest stored, as v1 counts them.
    async fn get_block_height(
        &self,
        _: Request<proto::GetBlockHeightRequest>,
    ) -> Result<Response<proto::BlockHeightResponse>, Status> {
        let newest = self.reader.newest().await?;
        Ok(Response::new(proto::BlockHeightResponse {
            height: newest.map_or(0, |h| h + 1),
        }))
    }

    /// Seconds since the newest stored block was made, by its header's timestamp.
    async fn get_time_since_last_decide(
        &self,
        _: Request<proto::GetTimeSinceLastDecideRequest>,
    ) -> Result<Response<proto::TimeSinceLastDecideResponse>, Status> {
        let newest = self
            .reader
            .newest()
            .await?
            .ok_or_else(|| Status::not_found("no block is stored yet"))?;
        let item = self.reader.item(newest, Wait::Briefly).await?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| Status::internal(err.to_string()))?
            .as_secs();
        Ok(Response::new(proto::TimeSinceLastDecideResponse {
            seconds: now.saturating_sub(item.leaf.block_header().timestamp()),
        }))
    }

    async fn get_success_rate(
        &self,
        _: Request<proto::GetSuccessRateRequest>,
    ) -> Result<Response<proto::SuccessRateResponse>, Status> {
        Err(Status::unimplemented(
            "the API service takes no part in consensus; ask the consensus node",
        ))
    }

    async fn get_node_keys(
        &self,
        _: Request<proto::GetNodeKeysRequest>,
    ) -> Result<Response<proto::NodeKeysResponse>, Status> {
        Err(Status::unimplemented(
            "the API service has no keys of its own; ask the consensus node",
        ))
    }
}
