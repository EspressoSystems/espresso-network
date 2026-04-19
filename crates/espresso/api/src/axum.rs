//! Axum HTTP/JSON API handlers

pub mod routes;

use aide::{
    axum::{ApiRouter, routing::get_with},
    openapi::{Info, OpenApi},
    operation::OperationOutput,
    redoc::Redoc,
    scalar::Scalar,
    swagger::Swagger,
};
use axum::{
    Extension, Json, Router,
    extract::{Path, Request, State},
    http::{StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use schemars::transform::Transform;
use serde::Serialize;
use serialization_api::v2::{
    GetRewardAccountProofRequest, GetRewardBalanceRequest, GetRewardBalancesRequest,
    GetRewardClaimInputRequest, GetRewardMerkleTreeRequest,
};

use crate::{error::ApiError, handlers, v1, v2};

/// API error response
#[derive(Debug, Serialize, schemars::JsonSchema)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(ErrorResponse {
            error: self.to_string(),
        });

        (status, body).into_response()
    }
}

impl OperationOutput for ApiError {
    type Inner = Self;
}

/// Serve the OpenAPI spec (extracted from Extension)
async fn serve_openapi_spec(Extension(api): Extension<OpenApi>) -> Json<OpenApi> {
    Json(api)
}

/// Middleware to rewrite root paths to /v2 paths
///
/// Requests to `/rewards/...` get rewritten to `/v2/rewards/...`
/// Paths already prefixed with `/v2` are left unchanged
///
/// Note: This middleware is only applied to the v2 router, so v1 routes never pass through it
async fn rewrite_root_to_v2(mut req: Request, next: Next) -> Response {
    let uri = req.uri().clone();
    let path = uri.path();

    // Only rewrite unversioned paths (not starting with /v2)
    if !path.starts_with("/v2") && path != "/" {
        let new_path = format!("/v2{}", path);
        let pq = if let Some(q) = uri.query() {
            format!("{}?{}", new_path, q)
        } else {
            new_path
        };
        if let Ok(new_uri) = Uri::builder().path_and_query(pq).build() {
            *req.uri_mut() = new_uri;
        }
    }

    next.run(req).await
}

/// Redirect handler for root path
async fn redirect_to_docs() -> axum::response::Redirect {
    axum::response::Redirect::permanent("/v2")
}

/// Create a combined router serving both v1 and v2 APIs
pub fn create_combined_router<S>(state: S) -> Router
where
    S: v1::RewardApi + v2::RewardApi + Clone + Send + Sync + 'static,
{
    let router_v1 = create_router_v1(state.clone());
    let router_v2 = create_router_v2(state).layer(middleware::from_fn(rewrite_root_to_v2));

    router_v2.merge(router_v1).route("/", get(redirect_to_docs))
}

/// Create v1 router without OpenAPI documentation (internal types)
pub fn create_router_v1<S>(state: S) -> Router
where
    S: v1::RewardApi + Clone + Send + Sync + 'static,
{
    // Create handler closures that capture the generic state type
    let get_reward_claim_input =
        |State(state): State<S>, Path((height, address)): Path<(u64, String)>| async move {
            state
                .get_reward_claim_input(height, address)
                .await
                .map(Json)
                .map_err(ApiError::Internal)
        };

    let get_reward_balance =
        |State(state): State<S>, Path((height, address)): Path<(u64, String)>| async move {
            state
                .get_reward_balance(height, address)
                .await
                .map(Json)
                .map_err(ApiError::Internal)
        };

    let get_latest_reward_balance = |State(state): State<S>, Path(address): Path<String>| async move {
        state
            .get_latest_reward_balance(address)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };

    let get_reward_account_proof =
        |State(state): State<S>, Path((height, address)): Path<(u64, String)>| async move {
            state
                .get_reward_account_proof(height, address)
                .await
                .map(Json)
                .map_err(ApiError::Internal)
        };

    let get_latest_reward_account_proof = |State(state): State<S>, Path(address): Path<String>| async move {
        state
            .get_latest_reward_account_proof(address)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };

    let get_reward_amounts =
        |State(state): State<S>, Path((height, offset, limit)): Path<(u64, u64, u64)>| async move {
            state
                .get_reward_amounts(height, offset, limit)
                .await
                .map(Json)
                .map_err(ApiError::Internal)
        };

    let get_reward_merkle_tree_v2 = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_reward_merkle_tree_v2(height)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };

    // Build plain Axum router without OpenAPI (for v1 - internal types)
    Router::new()
        .route(
            routes::v1::REWARD_CLAIM_INPUT_ROUTE,
            get(get_reward_claim_input),
        )
        .route(routes::v1::REWARD_BALANCE_ROUTE, get(get_reward_balance))
        .route(
            routes::v1::LATEST_REWARD_BALANCE_ROUTE,
            get(get_latest_reward_balance),
        )
        .route(
            routes::v1::REWARD_ACCOUNT_PROOF_ROUTE,
            get(get_reward_account_proof),
        )
        .route(
            routes::v1::LATEST_REWARD_ACCOUNT_PROOF_ROUTE,
            get(get_latest_reward_account_proof),
        )
        .route(routes::v1::REWARD_AMOUNTS_ROUTE, get(get_reward_amounts))
        .route(
            routes::v1::REWARD_MERKLE_TREE_V2_ROUTE,
            get(get_reward_merkle_tree_v2),
        )
        .with_state(state)
}

/// Create v2 router with OpenAPI documentation (proto types)
pub fn create_router_v2<S>(state: S) -> Router
where
    S: v2::RewardApi + Clone + Send + Sync + 'static,
{
    let mut api = OpenApi {
        info: Info {
            title: "Espresso Node API v2".to_string(),
            description: None,
            version: "1.0.0".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    // Handler closures: use proto request types directly with Path<T>

    let get_reward_claim_input =
        |State(state): State<S>, Path(request): Path<GetRewardClaimInputRequest>| async move {
            handlers::get_reward_claim_input(&state, request)
                .await
                .map(Json)
        };

    let get_reward_balance =
        |State(state): State<S>, Path(request): Path<GetRewardBalanceRequest>| async move {
            handlers::get_reward_balance(&state, request)
                .await
                .map(Json)
        };

    let get_reward_account_proof =
        |State(state): State<S>, Path(request): Path<GetRewardAccountProofRequest>| async move {
            handlers::get_reward_account_proof(&state, request)
                .await
                .map(Json)
        };

    let get_reward_balances =
        |State(state): State<S>, Path(request): Path<GetRewardBalancesRequest>| async move {
            handlers::get_reward_balances(&state, request)
                .await
                .map(Json)
        };

    let get_reward_merkle_tree_v2 =
        |State(state): State<S>, Path(request): Path<GetRewardMerkleTreeRequest>| async move {
            handlers::get_reward_merkle_tree_v2(&state, request)
                .await
                .map(Json)
        };

    let router = ApiRouter::new()
        .api_route(
            routes::v2::REWARD_CLAIM_INPUT_ROUTE.http,
            get_with(get_reward_claim_input, |op| {
                op.description(
                    "Get reward claim input for L1 contract submission. Returns lifetime rewards \
                     and Merkle proof needed to call claimRewards() on the L1 contract.",
                )
                .tag("Rewards")
            }),
        )
        .api_route(
            routes::v2::REWARD_BALANCE_ROUTE.http,
            get_with(get_reward_balance, |op| {
                op.description("Get reward balance for an address at the latest finalized height")
                    .tag("Rewards")
            }),
        )
        .api_route(
            routes::v2::REWARD_ACCOUNT_PROOF_ROUTE.http,
            get_with(get_reward_account_proof, |op| {
                op.description(
                    "Get Merkle proof for a reward account at the latest finalized height. \
                     Returns V2 proof with Keccak256 hashing",
                )
                .tag("Rewards")
            }),
        )
        .api_route(
            routes::v2::REWARD_BALANCES_ROUTE.http,
            get_with(get_reward_balances, |op| {
                op.description(
                    "Get paginated list of all reward balances at a specific height. Limit must \
                     be ≤ 10000",
                )
                .tag("Rewards")
            }),
        )
        .api_route(
            routes::v2::REWARD_MERKLE_TREE_V2_ROUTE.http,
            get_with(get_reward_merkle_tree_v2, |op| {
                op.description(
                    "Get raw RewardMerkleTreeV2 snapshot at a given height. Returns serialized \
                     merkle tree data",
                )
                .tag("Rewards")
            }),
        )
        .finish_api(&mut api);

    // Transform examples (array) to example (singular) for OpenAPI 3.0/Swagger compatibility
    if let Some(ref mut components) = api.components {
        let mut transform = schemars::transform::SetSingleExample::default();
        for schema in components.schemas.values_mut() {
            transform.transform(&mut schema.json_schema);
        }
    }

    // Also transform path parameter schemas
    if let Some(ref mut paths) = api.paths {
        let mut transform = schemars::transform::SetSingleExample::default();
        for path_item_ref in paths.paths.values_mut() {
            if let aide::openapi::ReferenceOr::Item(path_item) = path_item_ref {
                for operation in [
                    &mut path_item.get,
                    &mut path_item.post,
                    &mut path_item.put,
                    &mut path_item.delete,
                    &mut path_item.patch,
                ]
                .into_iter()
                .flatten()
                {
                    for param in &mut operation.parameters {
                        if let aide::openapi::ReferenceOr::Item(param_item) = param {
                            let parameter_data = match param_item {
                                aide::openapi::Parameter::Query { parameter_data, .. } => {
                                    parameter_data
                                },
                                aide::openapi::Parameter::Header { parameter_data, .. } => {
                                    parameter_data
                                },
                                aide::openapi::Parameter::Path { parameter_data, .. } => {
                                    parameter_data
                                },
                                aide::openapi::Parameter::Cookie { parameter_data, .. } => {
                                    parameter_data
                                },
                            };
                            if let aide::openapi::ParameterSchemaOrContent::Schema(ref mut schema) =
                                parameter_data.format
                            {
                                transform.transform(&mut schema.json_schema);
                            }
                        }
                    }
                }
            }
        }
    }

    router
        .route(routes::v2::OPENAPI_SPEC_ROUTE, get(serve_openapi_spec))
        .route(
            routes::v2::SWAGGER_ROUTE,
            get(Swagger::new(routes::v2::OPENAPI_SPEC_ROUTE)
                .with_title("Espresso Node API v2")
                .axum_handler()),
        )
        .route(
            "/v2/",
            get(Swagger::new(routes::v2::OPENAPI_SPEC_ROUTE)
                .with_title("Espresso Node API v2")
                .axum_handler()),
        )
        .route(
            routes::v2::SCALAR_ROUTE,
            get(Scalar::new(routes::v2::OPENAPI_SPEC_ROUTE)
                .with_title("Espresso Node API v2")
                .axum_handler()),
        )
        .route(
            routes::v2::REDOC_ROUTE,
            get(Redoc::new(routes::v2::OPENAPI_SPEC_ROUTE)
                .with_title("Espresso Node API v2")
                .axum_handler()),
        )
        .layer(Extension(api))
        .with_state(state)
}
