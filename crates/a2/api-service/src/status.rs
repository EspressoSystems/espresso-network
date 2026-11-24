//! The status service: how far the store has come, and how well the chain does by it.

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

    /// The share of views that decided a block, up to the newest stored block: its height over its
    /// view. main divides by the consensus node's current view instead, which this API service
    /// does not know; the two differ by the views since the newest decide.
    async fn get_success_rate(
        &self,
        _: Request<proto::GetSuccessRateRequest>,
    ) -> Result<Response<proto::SuccessRateResponse>, Status> {
        let rate = match self.reader.newest().await? {
            Some(newest) => {
                let item = self.reader.item(newest, Wait::Briefly).await?;
                (newest + 1) as f64 / item.leaf.view_number().u64() as f64
            },
            None => 0.,
        };
        // protoJSON cannot encode a non-finite double, as for main's.
        let rate = if rate.is_finite() { rate } else { 0. };
        Ok(Response::new(proto::SuccessRateResponse { rate }))
    }

    async fn get_node_keys(
        &self,
        _: Request<proto::GetNodeKeysRequest>,
    ) -> Result<Response<proto::NodeKeysResponse>, Status> {
        Err(Status::unimplemented(
            "the API service has no keys of its own",
        ))
    }
}
