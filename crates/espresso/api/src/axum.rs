//! Axum HTTP/JSON API handlers

pub mod routes;

use aide::{
    axum::{routing::get_with, ApiRouter},
    openapi::{Info, OpenApi},
    redoc::Redoc,
    scalar::Scalar,
    swagger::Swagger,
};
use axum::{
    extract::{Path, Request, State},
    http::{StatusCode, Uri},
    middleware::{self, Next},
    response::Response,
    routing::get,
    Extension, Json, Router,
};

use crate::{handlers, v1, v2};
use serialization_api::v2::{
    GetLatestRewardAccountProofRequest, GetLatestRewardBalanceRequest,
    GetRewardAccountProofRequest, GetRewardAmountsRequest, GetRewardBalanceRequest,
    GetRewardClaimInputRequest, GetRewardMerkleTreeRequest,
};

/// Serve the OpenAPI spec (extracted from Extension)
async fn serve_openapi_spec(Extension(api): Extension<OpenApi>) -> Json<OpenApi> {
    Json(api)
}

/// Middleware to rewrite root paths to /v2 paths
///
/// Requests to `/rewards/...` get rewritten to `/v2/rewards/...`
/// Paths already prefixed with `/v1` or `/v2` are left unchanged
async fn rewrite_root_to_v2(mut req: Request, next: Next) -> Response {
    let path = req.uri().path();

    // If path doesn't start with /v1 or /v2, prepend /v2
    if !path.starts_with("/v1") && !path.starts_with("/v2") {
        let new_path = format!("/v2{}", path);
        if let Ok(new_uri) = Uri::builder().path_and_query(new_path).build() {
            *req.uri_mut() = new_uri;
        }
        // If URI building fails, continue with original request
    }

    next.run(req).await
}

/// Create a combined router serving both v1 and v2 APIs
///
/// This is the main entry point for espresso-node. Routes are available at:
/// - `/v1/reward-state-v2/*` - V1 API (internal types, no OpenAPI docs)
/// - `/v2/rewards/*` - V2 API (proto types, with OpenAPI docs)
/// - `/rewards/*` - V2 API (rewritten to /v2/rewards/*)
/// - `/`, `/scalar`, `/redoc` - Documentation UIs
pub fn create_combined_router<S>(state: S) -> Router
where
    S: v1::RewardApi + v2::RewardApi + Clone + Send + Sync + 'static,
{
    let router_v1 = create_router_v1(state.clone());
    let router = create_router_v2(state);

    router
        .merge(router_v1)
        .layer(middleware::from_fn(rewrite_root_to_v2))
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
                .map_err(|e| {
                    tracing::error!("get_reward_claim_input error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })
        };

    let get_reward_balance =
        |State(state): State<S>, Path((height, address)): Path<(u64, String)>| async move {
            state
                .get_reward_balance(height, address)
                .await
                .map(Json)
                .map_err(|e| {
                    tracing::error!("get_reward_balance error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })
        };

    let get_latest_reward_balance = |State(state): State<S>, Path(address): Path<String>| async move {
        state
            .get_latest_reward_balance(address)
            .await
            .map(Json)
            .map_err(|e| {
                tracing::error!("get_latest_reward_balance error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })
    };

    let get_reward_account_proof =
        |State(state): State<S>, Path((height, address)): Path<(u64, String)>| async move {
            state
                .get_reward_account_proof(height, address)
                .await
                .map(Json)
                .map_err(|e| {
                    tracing::error!("get_reward_account_proof error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })
        };

    let get_latest_reward_account_proof = |State(state): State<S>, Path(address): Path<String>| async move {
        state
            .get_latest_reward_account_proof(address)
            .await
            .map(Json)
            .map_err(|e| {
                tracing::error!("get_latest_reward_account_proof error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })
    };

    let get_reward_amounts =
        |State(state): State<S>, Path((height, offset, limit)): Path<(u64, u64, u64)>| async move {
            state
                .get_reward_amounts(height, offset, limit)
                .await
                .map(Json)
                .map_err(|e| {
                    tracing::error!("get_reward_amounts error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })
        };

    let get_reward_merkle_tree_v2 = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_reward_merkle_tree_v2(height)
            .await
            .map(Json)
            .map_err(|e| {
                tracing::error!("get_reward_merkle_tree_v2 error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })
    };

    // Build plain Axum router without OpenAPI (for v1 - internal types)
    Router::new()
        .route(routes::v1::REWARD_CLAIM_INPUT_ROUTE, get(get_reward_claim_input))
        .route(routes::v1::REWARD_BALANCE_ROUTE, get(get_reward_balance))
        .route(routes::v1::LATEST_REWARD_BALANCE_ROUTE, get(get_latest_reward_balance))
        .route(routes::v1::REWARD_ACCOUNT_PROOF_ROUTE, get(get_reward_account_proof))
        .route(routes::v1::LATEST_REWARD_ACCOUNT_PROOF_ROUTE, get(get_latest_reward_account_proof))
        .route(routes::v1::REWARD_AMOUNTS_ROUTE, get(get_reward_amounts))
        .route(routes::v1::REWARD_MERKLE_TREE_V2_ROUTE, get(get_reward_merkle_tree_v2))
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

    // Handler closures: build proto requests and call shared handlers

    let get_reward_claim_input =
        |State(state): State<S>, Path((height, address)): Path<(u64, String)>| async move {
            let request = GetRewardClaimInputRequest {
                block_height: height,
                address,
            };
            handlers::get_reward_claim_input(&state, request)
                .await
                .map(Json)
                .map_err(|e| {
                    tracing::error!("get_reward_claim_input error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })
        };

    let get_reward_balance =
        |State(state): State<S>, Path((height, address)): Path<(u64, String)>| async move {
            let request = GetRewardBalanceRequest {
                height,
                address,
            };
            handlers::get_reward_balance(&state, request)
                .await
                .map(Json)
                .map_err(|e| {
                    tracing::error!("get_reward_balance error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })
        };

    let get_latest_reward_balance = |State(state): State<S>, Path(address): Path<String>| async move {
        let request = GetLatestRewardBalanceRequest { address };
        handlers::get_latest_reward_balance(&state, request)
            .await
            .map(Json)
            .map_err(|e| {
                tracing::error!("get_latest_reward_balance error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })
    };

    let get_reward_account_proof =
        |State(state): State<S>, Path((height, address)): Path<(u64, String)>| async move {
            let request = GetRewardAccountProofRequest {
                height,
                address,
            };
            handlers::get_reward_account_proof(&state, request)
                .await
                .map(Json)
                .map_err(|e| {
                    tracing::error!("get_reward_account_proof error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })
        };

    let get_latest_reward_account_proof = |State(state): State<S>, Path(address): Path<String>| async move {
        let request = GetLatestRewardAccountProofRequest { address };
        handlers::get_latest_reward_account_proof(&state, request)
            .await
            .map(Json)
            .map_err(|e| {
                tracing::error!("get_latest_reward_account_proof error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })
    };

    let get_reward_amounts =
        |State(state): State<S>, Path((height, offset, limit)): Path<(u64, u64, u64)>| async move {
            let request = GetRewardAmountsRequest {
                height,
                offset,
                limit,
            };
            handlers::get_reward_amounts(&state, request)
                .await
                .map(Json)
                .map_err(|e| {
                    tracing::error!("get_reward_amounts error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })
        };

    let get_reward_merkle_tree_v2 = |State(state): State<S>, Path(height): Path<u64>| async move {
        let request = GetRewardMerkleTreeRequest { height };
        handlers::get_reward_merkle_tree_v2(&state, request)
            .await
            .map(Json)
            .map_err(|e| {
                tracing::error!("get_reward_merkle_tree_v2 error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })
    };

    ApiRouter::new()
        // Reward claim input (most important - for L1 contract interaction)
        .api_route(
            routes::v2::REWARD_CLAIM_INPUT_ROUTE.http,
            get_with(get_reward_claim_input, |op| {
                op.description("Get reward claim input for L1 contract submission. Returns lifetime rewards and Merkle proof needed to call claimRewards() on the L1 contract.")
                    .tag("Rewards - Contract Interaction")
            }),
        )
        // Reward balances
        .api_route(
            routes::v2::REWARD_BALANCE_ROUTE.http,
            get_with(get_reward_balance, |op| {
                op.description("Get reward balance for an address at a specific block height")
                    .tag("Rewards - Balances")
            }),
        )
        .api_route(
            routes::v2::LATEST_REWARD_BALANCE_ROUTE.http,
            get_with(get_latest_reward_balance, |op| {
                op.description("Get latest reward balance for an address at the most recent finalized height")
                    .tag("Rewards - Balances")
            }),
        )
        // Reward proofs
        .api_route(
            routes::v2::REWARD_ACCOUNT_PROOF_ROUTE.http,
            get_with(get_reward_account_proof, |op| {
                op.description("Get Merkle proof for a reward account at a specific height. Returns version-aware proof (V1 for protocol ≤V3, V2 for V4+)")
                    .tag("Rewards - Proofs")
            }),
        )
        .api_route(
            routes::v2::LATEST_REWARD_ACCOUNT_PROOF_ROUTE.http,
            get_with(get_latest_reward_account_proof, |op| {
                op.description("Get Merkle proof for a reward account at the latest finalized height. Returns V2 proof with Keccak256 hashing")
                    .tag("Rewards - Proofs")
            }),
        )
        // Bulk queries
        .api_route(
            routes::v2::REWARD_AMOUNTS_ROUTE.http,
            get_with(get_reward_amounts, |op| {
                op.description("Get paginated list of all reward amounts at a specific height. Limit must be ≤ 10000")
                    .tag("Rewards - Bulk Queries")
            }),
        )
        // Tree snapshots
        .api_route(
            routes::v2::REWARD_MERKLE_TREE_V2_ROUTE.http,
            get_with(get_reward_merkle_tree_v2, |op| {
                op.description("Get raw RewardMerkleTreeV2 snapshot at a given height. Returns serialized merkle tree data")
                    .tag("Rewards - Tree Snapshots")
            }),
        )
        .finish_api(&mut api)
        .route(routes::v2::OPENAPI_SPEC_ROUTE, get(serve_openapi_spec))
        .route(
            routes::v2::SWAGGER_ROUTE,
            get(Swagger::new(routes::v2::OPENAPI_SPEC_PATH)
                .with_title("Espresso Node API v2")
                .axum_handler()),
        )
        .route(
            routes::v2::SCALAR_ROUTE,
            get(Scalar::new(routes::v2::OPENAPI_SPEC_PATH)
                .with_title("Espresso Node API v2")
                .axum_handler()),
        )
        .route(
            routes::v2::REDOC_ROUTE,
            get(Redoc::new(routes::v2::OPENAPI_SPEC_PATH)
                .with_title("Espresso Node API v2")
                .axum_handler()),
        )
        .layer(Extension(api))
        .with_state(state)
}
