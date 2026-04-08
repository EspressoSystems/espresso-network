//! Axum HTTP/JSON API handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::r#trait::NodeApi;

#[derive(OpenApi)]
#[openapi(
    paths(get_namespace_proof, get_reward_claim_input),
    info(
        title = "Espresso Sequencer API",
        description = "HTTP/JSON API for Espresso Network",
        version = "0.1.0"
    )
)]
struct ApiDoc;

#[utoipa::path(
    get,
    path = "/namespace-proof/{height}/{namespace}",
    tag = "Data Availability",
    description = "Query namespace proof and transactions by height and namespace ID",
    params(
        ("height" = u64, Path, description = "Block height"),
        ("namespace" = u64, Path, description = "Namespace ID")
    ),
    responses(
        (status = 200, description = "Namespace proof data"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_namespace_proof<S: NodeApi>(
    State(state): State<S>,
    Path((height, namespace)): Path<(u64, u64)>,
) -> Result<Json<serialization_api::v1::NamespaceProofQueryData>, StatusCode> {
    state
        .get_namespace_proof(height, namespace)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[utoipa::path(
    get,
    path = "/reward-claim-input/{block_height}/{address}",
    tag = "Rewards",
    description = "Get reward claim input for L1 contract submission",
    params(
        ("block_height" = u64, Path, description = "Block height matching Light Client finalized height"),
        ("address" = String, Path, description = "Ethereum address (hex format)")
    ),
    responses(
        (status = 200, description = "Reward claim input data"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_reward_claim_input<S: NodeApi>(
    State(state): State<S>,
    Path((block_height, address)): Path<(u64, String)>,
) -> Result<Json<serialization_api::v1::RewardClaimInput>, StatusCode> {
    state
        .get_reward_claim_input(block_height, address)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) fn create_axum_router<S>(state: S) -> Router
where
    S: NodeApi + Clone + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/namespace-proof/{height}/{namespace}",
            get(get_namespace_proof::<S>),
        )
        .route(
            "/reward-claim-input/{block_height}/{address}",
            get(get_reward_claim_input::<S>),
        )
        .merge(Scalar::with_url("/docs", ApiDoc::openapi()))
        .with_state(state)
}
