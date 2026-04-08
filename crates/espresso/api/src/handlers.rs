use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use utoipa::{OpenApi, ToSchema};
use utoipa_scalar::{Scalar, Servable};

use crate::{data_source::DataSourceError, DataSource};

#[derive(Serialize, ToSchema)]
pub struct RewardClaimResponse {
    pub address: String,
    pub lifetime_rewards: String,
    /// ABI-encoded auth data as hex string (0x-prefixed)
    pub auth_data: String,
}

#[derive(Serialize, ToSchema)]
pub struct NamespaceProofResponse {}

#[derive(Debug)]
pub enum ApiError {
    Internal(String),
    NotFound(String),
    BadRequest(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            ApiError::Internal(msg) => {
                tracing::error!("internal error: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            },
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
        };
        (status, Json(serde_json::json!({"error": message}))).into_response()
    }
}

impl From<DataSourceError> for ApiError {
    fn from(err: DataSourceError) -> Self {
        match err {
            DataSourceError::BadRequest(msg) => ApiError::BadRequest(msg),
            DataSourceError::NotFound(msg) => ApiError::NotFound(msg),
            DataSourceError::Internal(msg) => ApiError::Internal(msg),
        }
    }
}

// REST handlers

#[utoipa::path(
    get,
    path = "/v1/namespace-proof/{height}/{namespace}",
    params(("height" = u64, Path, description = "Block height"), ("namespace" = u64, Path, description = "Namespace ID")),
    responses((status = 200, body = NamespaceProofResponse)),
    tag = "Data Availability"
)]
pub async fn get_namespace_proof<D: DataSource>(
    State(_ds): State<D>,
    Path((_height, _namespace)): Path<(u64, u64)>,
) -> Result<Json<NamespaceProofResponse>, ApiError> {
    // TODO: implement namespace proof retrieval
    Ok(Json(NamespaceProofResponse {}))
}

#[utoipa::path(
    get,
    path = "/v1/reward-claim-input/{block_height}/{address}",
    params(("block_height" = u64, Path, description = "Block height"), ("address" = String, Path, description = "Ethereum address")),
    responses((status = 200, body = RewardClaimResponse)),
    tag = "Rewards"
)]
pub async fn get_reward_claim_input<D: DataSource>(
    State(ds): State<D>,
    Path((block_height, address)): Path<(u64, String)>,
) -> Result<Json<RewardClaimResponse>, ApiError> {
    let data = ds
        .get_reward_claim_input(block_height, &address)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(RewardClaimResponse {
        address,
        lifetime_rewards: data.lifetime_rewards,
        auth_data: data.auth_data_hex,
    }))
}

// OpenAPI

#[derive(OpenApi)]
#[openapi(
    paths(get_namespace_proof, get_reward_claim_input),
    components(schemas(RewardClaimResponse, NamespaceProofResponse))
)]
pub struct ApiDoc;

pub async fn serve_openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

pub fn rest_router<D: DataSource>(state: D) -> Router {
    Router::new()
        .route(
            "/v1/namespace-proof/{height}/{namespace}",
            get(get_namespace_proof::<D>),
        )
        .route(
            "/v1/reward-claim-input/{block_height}/{address}",
            get(get_reward_claim_input::<D>),
        )
        .route("/docs/openapi.json", get(serve_openapi))
        .merge(Scalar::with_url("/docs", ApiDoc::openapi()))
        .with_state(state)
}
