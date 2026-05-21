//! Axum HTTP/JSON API handlers

pub mod routes;

use aide::{
    axum::{ApiRouter, routing::get_with},
    openapi::{Info, OpenApi},
    operation::OperationOutput,
    redoc::Redoc,
    scalar::Scalar,
};
use axum::{
    Extension, Json, Router,
    extract::{Path, Request, State, ws::WebSocketUpgrade},
    http::{StatusCode, Uri},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use futures::stream::BoxStream;
use schemars::transform::Transform;
use serde::Serialize;
use serialization_api::v2::{
    GetIncorrectEncodingProofRequest, GetNamespaceProofRequest, GetRewardAccountProofRequest,
    GetRewardBalanceRequest, GetRewardBalancesRequest, GetRewardClaimInputRequest,
    GetRewardMerkleTreeRequest, GetStakeTableRequest, GetStateCertificateRequest,
};

use crate::{
    error::{ApiError, AvailabilityError},
    handlers, v1, v2,
};

/// API error response
#[derive(Debug, Serialize, schemars::JsonSchema)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(ErrorResponse {
            error: self.to_string(),
        });

        (status, body).into_response()
    }
}

/// Classify an `anyhow::Error` from an availability handler into the appropriate `ApiError`
/// variant. Errors produced via [`AvailabilityError`] in the state implementation carry semantic
/// meaning; everything else falls back to a 500 Internal Server Error.
fn classify_availability_error(err: anyhow::Error) -> ApiError {
    let is_not_found = err
        .downcast_ref::<AvailabilityError>()
        .map(|e| matches!(e, AvailabilityError::NotFound(_)));
    match is_not_found {
        Some(true) => ApiError::NotFound(err),
        Some(false) => ApiError::BadRequest(err),
        None => ApiError::Internal(err),
    }
}

impl OperationOutput for ApiError {
    type Inner = Self;
}

/// Serve the OpenAPI spec (extracted from Extension)
async fn serve_openapi_spec(Extension(api): Extension<OpenApi>) -> Json<OpenApi> {
    Json(api)
}

/// Serve custom Swagger UI with collapsed defaults
async fn serve_swagger_ui() -> Html<&'static str> {
    Html(include_str!("../templates/swagger.html"))
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

struct SendQuery<T>(T);

impl<T, S> axum::extract::FromRequestParts<S> for SendQuery<T>
where
    T: serde::de::DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = axum::extract::rejection::QueryRejection;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        axum::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map(|axum::extract::Query(inner)| SendQuery(inner))
    }
}

impl<T: schemars::JsonSchema> aide::operation::OperationInput for SendQuery<T> {
    fn operation_input(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
        let schema = ctx.schema.subschema_for::<T>();
        let params = aide::operation::parameters_from_schema(
            ctx,
            schema,
            aide::operation::ParamLocation::Query,
        );
        aide::operation::add_parameters(ctx, operation, params);
    }
}

async fn drive_ws_stream<T: Serialize>(
    mut socket: axum::extract::ws::WebSocket,
    stream: BoxStream<'static, T>,
) {
    use futures::StreamExt as _;
    futures::pin_mut!(stream);
    while let Some(item) = stream.next().await {
        let Ok(json) = serde_json::to_string(&item) else {
            break;
        };
        if socket
            .send(axum::extract::ws::Message::Text(json.into()))
            .await
            .is_err()
        {
            break;
        }
    }
}

/// Create a combined router serving both v1 and v2 APIs
pub fn create_combined_router<S>(state: S) -> Router
where
    S: v1::RewardApi
        + v1::AvailabilityApi
        + v1::HotShotAvailabilityApi
        + v2::RewardApi
        + v2::DataApi
        + v2::ConsensusApi
        + Clone
        + Send
        + Sync
        + 'static,
{
    let router_v1 = create_router_v1(state.clone());
    let router_v2 = create_router_v2(state).layer(middleware::from_fn(rewrite_root_to_v2));

    router_v2.merge(router_v1).route("/", get(redirect_to_docs))
}

/// Create v1 router without OpenAPI documentation (internal types)
pub fn create_router_v1<S>(state: S) -> Router
where
    S: v1::RewardApi
        + v1::AvailabilityApi
        + v1::HotShotAvailabilityApi
        + Clone
        + Send
        + Sync
        + 'static,
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

    // Availability API handlers
    // Route: /v1/availability/block/{height}/namespace/{namespace}
    let get_namespace_proof_by_height =
        |State(state): State<S>, Path((height, namespace)): Path<(u64, u32)>| async move {
            state
                .get_namespace_proof(v1::availability::BlockId::Height(height), namespace)
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };

    // Route: /v1/availability/block/hash/{hash}/namespace/{namespace}
    let get_namespace_proof_by_hash =
        |State(state): State<S>, Path((hash, namespace)): Path<(String, u32)>| async move {
            state
                .get_namespace_proof(v1::availability::BlockId::Hash(hash), namespace)
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };

    // Route: /v1/availability/block/payload-hash/{payload-hash}/namespace/{namespace}
    let get_namespace_proof_by_payload_hash =
        |State(state): State<S>, Path((payload_hash, namespace)): Path<(String, u32)>| async move {
            state
                .get_namespace_proof(
                    v1::availability::BlockId::PayloadHash(payload_hash),
                    namespace,
                )
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };

    // Route: /v1/availability/block/{from}/{until}/namespace/{namespace}
    let get_namespace_proof_range =
        |State(state): State<S>, Path((from, until, namespace)): Path<(u64, u64, u32)>| async move {
            state
                .get_namespace_proof_range(from, until, namespace)
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };

    let get_incorrect_encoding_proof =
        |State(state): State<S>, Path((block_number, namespace)): Path<(u64, u32)>| async move {
            state
                .get_incorrect_encoding_proof(
                    v1::availability::BlockId::Height(block_number),
                    namespace,
                )
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };

    let get_state_cert_v1 = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .get_state_cert(epoch)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };

    let get_state_cert_v2 = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .get_state_cert_v2(epoch)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };

    // HotShot availability API handlers

    let get_leaf_by_height = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_leaf(v1::LeafId::Height(height))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let get_leaf_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_leaf(v1::LeafId::Hash(hash))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let get_leaf_range = |State(state): State<S>, Path((from, until)): Path<(usize, usize)>| async move {
        state
            .get_leaf_range(from, until)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };

    let get_header_by_height = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_header(v1::BlockId::Height(height))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let get_header_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_header(v1::BlockId::Hash(hash))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let get_header_by_payload_hash = |State(state): State<S>, Path(payload_hash): Path<String>| async move {
        state
            .get_header(v1::BlockId::PayloadHash(payload_hash))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let get_header_range = |State(state): State<S>, Path((from, until)): Path<(usize, usize)>| async move {
        state
            .get_header_range(from, until)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };

    let get_block_by_height = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_block(v1::BlockId::Height(height))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let get_block_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_block(v1::BlockId::Hash(hash))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let get_block_by_payload_hash = |State(state): State<S>, Path(payload_hash): Path<String>| async move {
        state
            .get_block(v1::BlockId::PayloadHash(payload_hash))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let get_block_range = |State(state): State<S>, Path((from, until)): Path<(usize, usize)>| async move {
        state
            .get_block_range(from, until)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };

    let get_payload_by_height = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_payload(v1::PayloadId::Height(height))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let get_payload_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_payload(v1::PayloadId::Hash(hash))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let get_payload_by_block_hash = |State(state): State<S>, Path(block_hash): Path<String>| async move {
        state
            .get_payload(v1::PayloadId::BlockHash(block_hash))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let get_payload_range = |State(state): State<S>, Path((from, until)): Path<(usize, usize)>| async move {
        state
            .get_payload_range(from, until)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };

    let get_vid_common_by_height = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_vid_common(v1::BlockId::Height(height))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let get_vid_common_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_vid_common(v1::BlockId::Hash(hash))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let get_vid_common_by_payload_hash =
        |State(state): State<S>, Path(payload_hash): Path<String>| async move {
            state
                .get_vid_common(v1::BlockId::PayloadHash(payload_hash))
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let get_vid_common_range =
        |State(state): State<S>, Path((from, until)): Path<(usize, usize)>| async move {
            state
                .get_vid_common_range(from, until)
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };

    let get_transaction_by_position =
        |State(state): State<S>, Path((height, index)): Path<(u64, u64)>| async move {
            state
                .get_transaction_by_position(height, index)
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let get_transaction_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_transaction_by_hash(hash)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let get_transaction_proof_by_position =
        |State(state): State<S>, Path((height, index)): Path<(u64, u64)>| async move {
            state
                .get_transaction_proof_by_position(height, index)
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let get_transaction_proof_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_transaction_proof_by_hash(hash)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };

    let get_block_summary_by_height = |State(state): State<S>, Path(height): Path<usize>| async move {
        state
            .get_block_summary(height)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let get_block_summary_range =
        |State(state): State<S>, Path((from, until)): Path<(usize, usize)>| async move {
            state
                .get_block_summary_range(from, until)
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };

    let get_limits = |State(state): State<S>| async move {
        state
            .get_limits()
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };

    let get_cert2 = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_cert2(height)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };

    // WebSocket streaming handlers
    let stream_leaves =
        |ws: WebSocketUpgrade, State(state): State<S>, Path(height): Path<usize>| async move {
            ws.on_upgrade(move |socket| async move {
                match state.stream_leaves(height).await {
                    Ok(stream) => drive_ws_stream(socket, stream).await,
                    Err(e) => tracing::warn!("stream_leaves: {e}"),
                }
            })
        };
    let stream_headers =
        |ws: WebSocketUpgrade, State(state): State<S>, Path(height): Path<usize>| async move {
            ws.on_upgrade(move |socket| async move {
                match state.stream_headers(height).await {
                    Ok(stream) => drive_ws_stream(socket, stream).await,
                    Err(e) => tracing::warn!("stream_headers: {e}"),
                }
            })
        };
    let stream_blocks =
        |ws: WebSocketUpgrade, State(state): State<S>, Path(height): Path<usize>| async move {
            ws.on_upgrade(move |socket| async move {
                match state.stream_blocks(height).await {
                    Ok(stream) => drive_ws_stream(socket, stream).await,
                    Err(e) => tracing::warn!("stream_blocks: {e}"),
                }
            })
        };
    let stream_payloads =
        |ws: WebSocketUpgrade, State(state): State<S>, Path(height): Path<usize>| async move {
            ws.on_upgrade(move |socket| async move {
                match state.stream_payloads(height).await {
                    Ok(stream) => drive_ws_stream(socket, stream).await,
                    Err(e) => tracing::warn!("stream_payloads: {e}"),
                }
            })
        };
    let stream_vid_common =
        |ws: WebSocketUpgrade, State(state): State<S>, Path(height): Path<usize>| async move {
            ws.on_upgrade(move |socket| async move {
                match state.stream_vid_common(height).await {
                    Ok(stream) => drive_ws_stream(socket, stream).await,
                    Err(e) => tracing::warn!("stream_vid_common: {e}"),
                }
            })
        };
    let stream_transactions =
        |ws: WebSocketUpgrade, State(state): State<S>, Path(height): Path<usize>| async move {
            ws.on_upgrade(move |socket| async move {
                match state.stream_transactions(height, None).await {
                    Ok(stream) => drive_ws_stream(socket, stream).await,
                    Err(e) => tracing::warn!("stream_transactions: {e}"),
                }
            })
        };
    let stream_transactions_ns =
        |ws: WebSocketUpgrade,
         State(state): State<S>,
         Path((height, namespace)): Path<(usize, u32)>| async move {
            ws.on_upgrade(move |socket| async move {
                match state.stream_transactions(height, Some(namespace)).await {
                    Ok(stream) => drive_ws_stream(socket, stream).await,
                    Err(e) => tracing::warn!("stream_transactions_ns: {e}"),
                }
            })
        };
    let stream_namespace_proofs =
        |ws: WebSocketUpgrade,
         State(state): State<S>,
         Path((height, namespace)): Path<(usize, u32)>| async move {
            ws.on_upgrade(move |socket| async move {
                match state.stream_namespace_proofs(height, namespace).await {
                    Ok(stream) => drive_ws_stream(socket, stream).await,
                    Err(e) => tracing::warn!("stream_namespace_proofs: {e}"),
                }
            })
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
        // Availability API routes
        .route(
            routes::v1::NAMESPACE_PROOF_BY_HEIGHT_ROUTE,
            get(get_namespace_proof_by_height),
        )
        .route(
            routes::v1::NAMESPACE_PROOF_BY_HASH_ROUTE,
            get(get_namespace_proof_by_hash),
        )
        .route(
            routes::v1::NAMESPACE_PROOF_BY_PAYLOAD_HASH_ROUTE,
            get(get_namespace_proof_by_payload_hash),
        )
        .route(
            routes::v1::NAMESPACE_PROOF_RANGE_ROUTE,
            get(get_namespace_proof_range),
        )
        .route(
            routes::v1::INCORRECT_ENCODING_PROOF_ROUTE,
            get(get_incorrect_encoding_proof),
        )
        .route(routes::v1::STATE_CERT_V1_ROUTE, get(get_state_cert_v1))
        .route(routes::v1::STATE_CERT_V2_ROUTE, get(get_state_cert_v2))
        // HotShot availability API routes
        .route(routes::v1::LEAF_BY_HEIGHT_ROUTE, get(get_leaf_by_height))
        .route(routes::v1::LEAF_BY_HASH_ROUTE, get(get_leaf_by_hash))
        .route(routes::v1::LEAF_RANGE_ROUTE, get(get_leaf_range))
        .route(
            routes::v1::HEADER_BY_HEIGHT_ROUTE,
            get(get_header_by_height),
        )
        .route(routes::v1::HEADER_BY_HASH_ROUTE, get(get_header_by_hash))
        .route(
            routes::v1::HEADER_BY_PAYLOAD_HASH_ROUTE,
            get(get_header_by_payload_hash),
        )
        .route(routes::v1::HEADER_RANGE_ROUTE, get(get_header_range))
        .route(routes::v1::BLOCK_BY_HEIGHT_ROUTE, get(get_block_by_height))
        .route(routes::v1::BLOCK_BY_HASH_ROUTE, get(get_block_by_hash))
        .route(
            routes::v1::BLOCK_BY_PAYLOAD_HASH_ROUTE,
            get(get_block_by_payload_hash),
        )
        .route(routes::v1::BLOCK_RANGE_ROUTE, get(get_block_range))
        .route(
            routes::v1::PAYLOAD_BY_HEIGHT_ROUTE,
            get(get_payload_by_height),
        )
        .route(
            routes::v1::PAYLOAD_BY_HASH_ROUTE,
            get(get_payload_by_hash),
        )
        .route(
            routes::v1::PAYLOAD_BY_BLOCK_HASH_ROUTE,
            get(get_payload_by_block_hash),
        )
        .route(routes::v1::PAYLOAD_RANGE_ROUTE, get(get_payload_range))
        .route(
            routes::v1::VID_COMMON_BY_HEIGHT_ROUTE,
            get(get_vid_common_by_height),
        )
        .route(
            routes::v1::VID_COMMON_BY_HASH_ROUTE,
            get(get_vid_common_by_hash),
        )
        .route(
            routes::v1::VID_COMMON_BY_PAYLOAD_HASH_ROUTE,
            get(get_vid_common_by_payload_hash),
        )
        .route(
            routes::v1::VID_COMMON_RANGE_ROUTE,
            get(get_vid_common_range),
        )
        .route(
            routes::v1::TRANSACTION_BY_POSITION_NOPROOF_ROUTE,
            get(get_transaction_by_position),
        )
        .route(
            routes::v1::TRANSACTION_BY_HASH_NOPROOF_ROUTE,
            get(get_transaction_by_hash),
        )
        .route(
            routes::v1::TRANSACTION_PROOF_BY_POSITION_ROUTE,
            get(get_transaction_proof_by_position),
        )
        .route(
            routes::v1::TRANSACTION_PROOF_BY_HASH_ROUTE,
            get(get_transaction_proof_by_hash),
        )
        .route(
            routes::v1::TRANSACTION_BY_POSITION_ROUTE,
            get(get_transaction_proof_by_position),
        )
        .route(
            routes::v1::TRANSACTION_BY_HASH_ROUTE,
            get(get_transaction_proof_by_hash),
        )
        .route(
            routes::v1::BLOCK_SUMMARY_BY_HEIGHT_ROUTE,
            get(get_block_summary_by_height),
        )
        .route(
            routes::v1::BLOCK_SUMMARY_RANGE_ROUTE,
            get(get_block_summary_range),
        )
        .route(routes::v1::LIMITS_ROUTE, get(get_limits))
        .route(routes::v1::CERT2_BY_HEIGHT_ROUTE, get(get_cert2))
        // WebSocket streaming routes
        .route(routes::v1::STREAM_LEAVES_ROUTE, get(stream_leaves))
        .route(routes::v1::STREAM_HEADERS_ROUTE, get(stream_headers))
        .route(routes::v1::STREAM_BLOCKS_ROUTE, get(stream_blocks))
        .route(routes::v1::STREAM_PAYLOADS_ROUTE, get(stream_payloads))
        .route(routes::v1::STREAM_VID_COMMON_ROUTE, get(stream_vid_common))
        .route(
            routes::v1::STREAM_TRANSACTIONS_ROUTE,
            get(stream_transactions),
        )
        .route(
            routes::v1::STREAM_TRANSACTIONS_NS_ROUTE,
            get(stream_transactions_ns),
        )
        .route(
            routes::v1::STREAM_NAMESPACE_PROOFS_ROUTE,
            get(stream_namespace_proofs),
        )
        .with_state(state)
}

/// Create v2 router with OpenAPI documentation (proto types)
pub fn create_router_v2<S>(state: S) -> Router
where
    S: v2::RewardApi + v2::DataApi + v2::ConsensusApi + Clone + Send + Sync + 'static,
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

    let get_reward_claim_input =
        |State(state): State<S>, SendQuery(request): SendQuery<GetRewardClaimInputRequest>| async move {
            handlers::get_reward_claim_input(&state, request)
                .await
                .map(Json)
        };

    let get_reward_balance =
        |State(state): State<S>, SendQuery(request): SendQuery<GetRewardBalanceRequest>| async move {
            handlers::get_reward_balance(&state, request)
                .await
                .map(Json)
        };

    let get_reward_account_proof =
        |State(state): State<S>, SendQuery(request): SendQuery<GetRewardAccountProofRequest>| async move {
            handlers::get_reward_account_proof(&state, request)
                .await
                .map(Json)
        };

    let get_reward_balances =
        |State(state): State<S>, SendQuery(request): SendQuery<GetRewardBalancesRequest>| async move {
            handlers::get_reward_balances(&state, request)
                .await
                .map(Json)
        };

    let get_reward_merkle_tree_v2 =
        |State(state): State<S>, SendQuery(request): SendQuery<GetRewardMerkleTreeRequest>| async move {
            handlers::get_reward_merkle_tree_v2(&state, request)
                .await
                .map(Json)
        };

    let get_state_certificate =
        |State(state): State<S>, SendQuery(request): SendQuery<GetStateCertificateRequest>| async move {
            handlers::get_state_certificate(&state, request)
                .await
                .map(Json)
        };

    let get_stake_table =
        |State(state): State<S>, SendQuery(request): SendQuery<GetStakeTableRequest>| async move {
            handlers::get_stake_table(&state, request).await.map(Json)
        };

    let get_namespace_proof =
        |State(state): State<S>, SendQuery(query): SendQuery<GetNamespaceProofRequest>| async move {
            handlers::get_namespace_proof(&state, query).await.map(Json)
        };

    let get_incorrect_encoding_proof = |State(state): State<S>,
                                        SendQuery(query): SendQuery<
        GetIncorrectEncodingProofRequest,
    >| async move {
        handlers::get_incorrect_encoding_proof(&state, query)
            .await
            .map(Json)
    };

    let router = ApiRouter::new()
        .api_route(
            routes::v2::REWARD_CLAIM_INPUT_ROUTE.http,
            get_with(get_reward_claim_input, |op| {
                op.description(routes::v2::REWARD_CLAIM_INPUT_ROUTE.description)
                    .tag(routes::v2::REWARD_CLAIM_INPUT_ROUTE.tag)
            }),
        )
        .api_route(
            routes::v2::REWARD_BALANCE_ROUTE.http,
            get_with(get_reward_balance, |op| {
                op.description(routes::v2::REWARD_BALANCE_ROUTE.description)
                    .tag(routes::v2::REWARD_BALANCE_ROUTE.tag)
            }),
        )
        .api_route(
            routes::v2::REWARD_ACCOUNT_PROOF_ROUTE.http,
            get_with(get_reward_account_proof, |op| {
                op.description(routes::v2::REWARD_ACCOUNT_PROOF_ROUTE.description)
                    .tag(routes::v2::REWARD_ACCOUNT_PROOF_ROUTE.tag)
            }),
        )
        .api_route(
            routes::v2::REWARD_BALANCES_ROUTE.http,
            get_with(get_reward_balances, |op| {
                op.description(routes::v2::REWARD_BALANCES_ROUTE.description)
                    .tag(routes::v2::REWARD_BALANCES_ROUTE.tag)
            }),
        )
        .api_route(
            routes::v2::REWARD_MERKLE_TREE_V2_ROUTE.http,
            get_with(get_reward_merkle_tree_v2, |op| {
                op.description(routes::v2::REWARD_MERKLE_TREE_V2_ROUTE.description)
                    .tag(routes::v2::REWARD_MERKLE_TREE_V2_ROUTE.tag)
            }),
        )
        .api_route(
            routes::v2::NAMESPACE_PROOF_ROUTE.http,
            get_with(get_namespace_proof, |op| {
                op.description(routes::v2::NAMESPACE_PROOF_ROUTE.description)
                    .tag(routes::v2::NAMESPACE_PROOF_ROUTE.tag)
            }),
        )
        .api_route(
            routes::v2::INCORRECT_ENCODING_PROOF_ROUTE.http,
            get_with(get_incorrect_encoding_proof, |op| {
                op.description(routes::v2::INCORRECT_ENCODING_PROOF_ROUTE.description)
                    .tag(routes::v2::INCORRECT_ENCODING_PROOF_ROUTE.tag)
            }),
        )
        .api_route(
            routes::v2::STATE_CERTIFICATE_ROUTE.http,
            get_with(get_state_certificate, |op| {
                op.description(routes::v2::STATE_CERTIFICATE_ROUTE.description)
                    .tag(routes::v2::STATE_CERTIFICATE_ROUTE.tag)
            }),
        )
        .api_route(
            routes::v2::STAKE_TABLE_ROUTE.http,
            get_with(get_stake_table, |op| {
                op.description(routes::v2::STAKE_TABLE_ROUTE.description)
                    .tag(routes::v2::STAKE_TABLE_ROUTE.tag)
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
        .route(routes::v2::SWAGGER_ROUTE, get(serve_swagger_ui))
        .route("/v2/", get(serve_swagger_ui))
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
