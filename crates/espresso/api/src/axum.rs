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
        + v1::BlockStateApi
        + v1::FeeStateApi
        + v1::StatusApi
        + v1::ConfigApi
        + v1::NodeApi
        + v1::CatchupApi
        + v1::SubmitApi
        + v1::StateSignatureApi
        + v1::HotShotEventsApi
        + v1::LightClientApi
        + v1::ExplorerApi
        + v1::TokenApi
        + v1::DatabaseApi
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
        + v1::BlockStateApi
        + v1::FeeStateApi
        + v1::StatusApi
        + v1::ConfigApi
        + v1::NodeApi
        + v1::CatchupApi
        + v1::SubmitApi
        + v1::StateSignatureApi
        + v1::HotShotEventsApi
        + v1::LightClientApi
        + v1::ExplorerApi
        + v1::TokenApi
        + v1::DatabaseApi
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
        <S as v1::RewardApi>::get_reward_merkle_tree_v2(&state, height)
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
        <S as v1::AvailabilityApi>::get_state_cert(&state, epoch)
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
        <S as v1::HotShotAvailabilityApi>::get_cert2(&state, height)
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

    // Merklized state handlers: block-state
    let get_block_state_path_by_height =
        |State(state): State<S>, Path((height, key)): Path<(u64, String)>| async move {
            <S as v1::BlockStateApi>::get_block_state_path(
                &state,
                v1::Snapshot::Height(height),
                key,
            )
            .await
            .map(Json)
            .map_err(classify_availability_error)
        };
    let get_block_state_path_by_commit =
        |State(state): State<S>, Path((commit, key)): Path<(String, String)>| async move {
            <S as v1::BlockStateApi>::get_block_state_path(
                &state,
                v1::Snapshot::Commit(commit),
                key,
            )
            .await
            .map(Json)
            .map_err(classify_availability_error)
        };
    let get_block_state_height = |State(state): State<S>| async move {
        <S as v1::BlockStateApi>::get_block_state_height(&state)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };

    // Merklized state handlers: fee-state
    let get_fee_state_path_by_height =
        |State(state): State<S>, Path((height, key)): Path<(u64, String)>| async move {
            <S as v1::FeeStateApi>::get_fee_state_path(&state, v1::Snapshot::Height(height), key)
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let get_fee_state_path_by_commit =
        |State(state): State<S>, Path((commit, key)): Path<(String, String)>| async move {
            <S as v1::FeeStateApi>::get_fee_state_path(&state, v1::Snapshot::Commit(commit), key)
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let get_fee_state_height = |State(state): State<S>| async move {
        <S as v1::FeeStateApi>::get_fee_state_height(&state)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let get_fee_balance_latest = |State(state): State<S>, Path(address): Path<String>| async move {
        state
            .get_fee_balance_latest(address)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };

    let status_block_height = |State(state): State<S>| async move {
        <S as v1::StatusApi>::block_height(&state)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let status_success_rate = |State(state): State<S>| async move {
        <S as v1::StatusApi>::success_rate(&state)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let status_time_since_last_decide = |State(state): State<S>| async move {
        <S as v1::StatusApi>::time_since_last_decide(&state)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let status_metrics = |State(state): State<S>| async move {
        match <S as v1::StatusApi>::metrics(&state).await {
            Ok(text) => Ok((
                [(
                    axum::http::header::CONTENT_TYPE,
                    "text/plain; charset=utf-8",
                )],
                text,
            )),
            Err(e) => Err(ApiError::Internal(e)),
        }
    };

    let config_hotshot = |State(state): State<S>| async move {
        <S as v1::ConfigApi>::hotshot_config(&state)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let config_env = |State(state): State<S>| async move {
        <S as v1::ConfigApi>::env(&state)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let config_runtime = |State(state): State<S>| async move {
        <S as v1::ConfigApi>::runtime_config(&state)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };

    let node_block_height = |State(state): State<S>| async move {
        <S as v1::NodeApi>::block_height(&state)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };

    let node_count_txs = |State(state): State<S>| async move {
        state
            .count_transactions(None, None, None)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_count_txs_to = |State(state): State<S>, Path(to): Path<u64>| async move {
        state
            .count_transactions(None, Some(to), None)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_count_txs_from_to = |State(state): State<S>, Path((from, to)): Path<(u64, u64)>| async move {
        state
            .count_transactions(Some(from), Some(to), None)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_count_txs_ns = |State(state): State<S>, Path(namespace): Path<u32>| async move {
        state
            .count_transactions(None, None, Some(namespace))
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_count_txs_ns_to = |State(state): State<S>, Path((namespace, to)): Path<(u32, u64)>| async move {
        state
            .count_transactions(None, Some(to), Some(namespace))
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_count_txs_ns_from_to =
        |State(state): State<S>, Path((namespace, from, to)): Path<(u32, u64, u64)>| async move {
            state
                .count_transactions(Some(from), Some(to), Some(namespace))
                .await
                .map(Json)
                .map_err(ApiError::Internal)
        };

    let node_payload_size = |State(state): State<S>| async move {
        state
            .payload_size(None, None, None)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_payload_size_to = |State(state): State<S>, Path(to): Path<u64>| async move {
        state
            .payload_size(None, Some(to), None)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_payload_size_from_to = |State(state): State<S>, Path((from, to)): Path<(u64, u64)>| async move {
        state
            .payload_size(Some(from), Some(to), None)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_payload_size_ns = |State(state): State<S>, Path(namespace): Path<u32>| async move {
        state
            .payload_size(None, None, Some(namespace))
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_payload_size_ns_to =
        |State(state): State<S>, Path((namespace, to)): Path<(u32, u64)>| async move {
            state
                .payload_size(None, Some(to), Some(namespace))
                .await
                .map(Json)
                .map_err(ApiError::Internal)
        };
    let node_payload_size_ns_from_to =
        |State(state): State<S>, Path((namespace, from, to)): Path<(u32, u64, u64)>| async move {
            state
                .payload_size(Some(from), Some(to), Some(namespace))
                .await
                .map(Json)
                .map_err(ApiError::Internal)
        };

    let node_vid_share_by_height = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_vid_share(v1::VidShareId::Height(height))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let node_vid_share_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_vid_share(v1::VidShareId::Hash(hash))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let node_vid_share_by_payload_hash =
        |State(state): State<S>, Path(payload_hash): Path<String>| async move {
            state
                .get_vid_share(v1::VidShareId::PayloadHash(payload_hash))
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };

    let node_sync_status = |State(state): State<S>| async move {
        state
            .sync_status()
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };

    let node_header_window_time = |State(state): State<S>, Path((start, end)): Path<(u64, u64)>| async move {
        state
            .get_header_window(v1::HeaderWindowStart::Time(start), end)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_header_window_height =
        |State(state): State<S>, Path((height, end)): Path<(u64, u64)>| async move {
            state
                .get_header_window(v1::HeaderWindowStart::Height(height), end)
                .await
                .map(Json)
                .map_err(ApiError::Internal)
        };
    let node_header_window_hash =
        |State(state): State<S>, Path((hash, end)): Path<(String, u64)>| async move {
            state
                .get_header_window(v1::HeaderWindowStart::Hash(hash), end)
                .await
                .map(Json)
                .map_err(ApiError::Internal)
        };

    let node_limits = |State(state): State<S>| async move {
        <S as v1::NodeApi>::limits(&state)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };

    let node_stake_table_current = |State(state): State<S>| async move {
        state
            .stake_table_current()
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_stake_table = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .stake_table(epoch)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_da_stake_table_current = |State(state): State<S>| async move {
        state
            .da_stake_table_current()
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_da_stake_table = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .da_stake_table(epoch)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };

    let node_validators = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .get_validators(epoch)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_all_validators =
        |State(state): State<S>, Path((epoch, offset, limit)): Path<(u64, u64, u64)>| async move {
            state
                .get_all_validators(epoch, offset, limit)
                .await
                .map(Json)
                .map_err(ApiError::BadRequest)
        };

    let node_proposal_participation_current = |State(state): State<S>| async move {
        state
            .current_proposal_participation()
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_proposal_participation = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .proposal_participation(epoch)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_vote_participation_current = |State(state): State<S>| async move {
        state
            .current_vote_participation()
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_vote_participation = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .vote_participation(epoch)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };

    let node_block_reward = |State(state): State<S>| async move {
        state
            .get_block_reward(None)
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_block_reward_epoch = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .get_block_reward(Some(epoch))
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };

    let node_oldest_block = |State(state): State<S>| async move {
        state
            .get_oldest_block()
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let node_oldest_leaf = |State(state): State<S>| async move {
        state
            .get_oldest_leaf()
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };

    // Catchup handlers
    let catchup_account =
        |State(state): State<S>, Path((height, view, address)): Path<(u64, u64, String)>| async move {
            state
                .get_account(height, view, address)
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let catchup_accounts = |State(state): State<S>,
                            Path((height, view)): Path<(u64, u64)>,
                            Json(body): Json<Vec<String>>| async move {
        state
            .get_accounts(height, view, body)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let catchup_blocks = |State(state): State<S>, Path((height, view)): Path<(u64, u64)>| async move {
        state
            .get_blocks_frontier(height, view)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let catchup_chainconfig = |State(state): State<S>, Path(commitment): Path<String>| async move {
        <S as v1::CatchupApi>::get_chain_config(&state, commitment)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let catchup_leafchain = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_leaf_chain(height)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let catchup_cert2 = |State(state): State<S>, Path(height): Path<u64>| async move {
        <S as v1::CatchupApi>::get_cert2(&state, height)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let catchup_reward_account =
        |State(state): State<S>, Path((height, view, address)): Path<(u64, u64, String)>| async move {
            state
                .get_reward_account_v1(height, view, address)
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let catchup_reward_accounts =
        |State(state): State<S>,
         Path((height, view)): Path<(u64, u64)>,
         Json(body): Json<Vec<String>>| async move {
            state
                .get_reward_accounts_v1(height, view, body)
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let catchup_reward_account_v2 =
        |State(state): State<S>, Path((height, view, address)): Path<(u64, u64, String)>| async move {
            state
                .get_reward_account_v2(height, view, address)
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let catchup_reward_accounts_v2 =
        |State(_): State<S>, Path((_height, _view)): Path<(u64, u64)>| async move {
            Err::<Json<()>, ApiError>(ApiError::NotFound(anyhow::anyhow!(
                "catchup/reward-accounts-v2 is deprecated"
            )))
        };
    let catchup_reward_amounts =
        |State(_): State<S>, Path((_height, _limit, _offset)): Path<(u64, u64, u64)>| async move {
            Err::<Json<()>, ApiError>(ApiError::NotFound(anyhow::anyhow!(
                "catchup/reward-amounts is deprecated"
            )))
        };
    let catchup_reward_merkle_tree_v2 =
        |State(state): State<S>, Path((height, view)): Path<(u64, u64)>| async move {
            <S as v1::CatchupApi>::get_reward_merkle_tree_v2(&state, height, view)
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let catchup_state_cert = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        <S as v1::CatchupApi>::get_state_cert(&state, epoch)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };

    // Submit handler
    let submit_submit = |State(state): State<S>, body: ::axum::body::Bytes| async move {
        let tx = match serde_json::from_slice::<<S as v1::SubmitApi>::Transaction>(&body) {
            Ok(tx) => tx,
            Err(err) => {
                return Err(ApiError::BadRequest(anyhow::anyhow!(
                    "invalid transaction body: {err}"
                )));
            },
        };
        state.submit(tx).await.map(Json).map_err(ApiError::Internal)
    };

    // State signature handler
    let state_signature_block = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_state_signature(height)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };

    // HotShot events handlers
    let hotshot_events_startup = |State(state): State<S>| async move {
        state
            .startup_info()
            .await
            .map(Json)
            .map_err(ApiError::Internal)
    };
    let hotshot_events_stream = |State(state): State<S>, ws: WebSocketUpgrade| async move {
        match <S as v1::HotShotEventsApi>::events(&state).await {
            Ok(stream) => ws.on_upgrade(move |socket| drive_ws_stream(socket, stream)),
            Err(err) => ApiError::Internal(err).into_response(),
        }
    };

    // Light-client handlers
    let lc_leaf_by_height = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_leaf_proof(v1::LeafQuery::Height(height), None)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let lc_leaf_by_height_finalized =
        |State(state): State<S>, Path((height, finalized)): Path<(u64, u64)>| async move {
            state
                .get_leaf_proof(v1::LeafQuery::Height(height), Some(finalized))
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let lc_leaf_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_leaf_proof(v1::LeafQuery::Hash(hash), None)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let lc_leaf_by_hash_finalized =
        |State(state): State<S>, Path((hash, finalized)): Path<(String, u64)>| async move {
            state
                .get_leaf_proof(v1::LeafQuery::Hash(hash), Some(finalized))
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let lc_leaf_by_block_hash = |State(state): State<S>, Path(block_hash): Path<String>| async move {
        state
            .get_leaf_proof(v1::LeafQuery::BlockHash(block_hash), None)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let lc_leaf_by_block_hash_finalized =
        |State(state): State<S>, Path((block_hash, finalized)): Path<(String, u64)>| async move {
            state
                .get_leaf_proof(v1::LeafQuery::BlockHash(block_hash), Some(finalized))
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let lc_leaf_by_payload_hash = |State(state): State<S>, Path(payload_hash): Path<String>| async move {
        state
            .get_leaf_proof(v1::LeafQuery::PayloadHash(payload_hash), None)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let lc_leaf_by_payload_hash_finalized =
        |State(state): State<S>, Path((payload_hash, finalized)): Path<(String, u64)>| async move {
            state
                .get_leaf_proof(v1::LeafQuery::PayloadHash(payload_hash), Some(finalized))
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };

    let lc_header_by_height = |State(state): State<S>, Path((root, height)): Path<(u64, u64)>| async move {
        state
            .get_header_proof(root, v1::HeaderQuery::Height(height))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let lc_header_by_hash = |State(state): State<S>, Path((root, hash)): Path<(u64, String)>| async move {
        state
            .get_header_proof(root, v1::HeaderQuery::Hash(hash))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let lc_header_by_payload_hash =
        |State(state): State<S>, Path((root, payload_hash)): Path<(u64, String)>| async move {
            state
                .get_header_proof(root, v1::HeaderQuery::PayloadHash(payload_hash))
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let lc_stake_table = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .get_light_client_stake_table(epoch)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let lc_payload = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_payload_proof(height)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let lc_payload_range = |State(state): State<S>, Path((start, end)): Path<(u64, u64)>| async move {
        state
            .get_payload_proof_range(start, end)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let lc_namespace = |State(state): State<S>, Path((height, namespace)): Path<(u64, u64)>| async move {
        state
            .get_lc_namespace_proof(height, namespace)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let lc_namespace_range =
        |State(state): State<S>, Path((start, end, namespace)): Path<(u64, u64, u64)>| async move {
            state
                .get_lc_namespace_proof_range(start, end, namespace)
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };

    // Explorer handlers
    let explorer_block_detail_by_height = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_block_detail(v1::BlockIdent::Height(height))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let explorer_block_detail_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_block_detail(v1::BlockIdent::Hash(hash))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let explorer_block_summaries_latest = |State(state): State<S>, Path(limit): Path<u64>| async move {
        state
            .get_block_summaries(v1::BlockIdent::Latest, limit)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let explorer_block_summaries_from =
        |State(state): State<S>, Path((from, limit)): Path<(u64, u64)>| async move {
            state
                .get_block_summaries(v1::BlockIdent::Height(from), limit)
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let explorer_tx_detail_by_position =
        |State(state): State<S>, Path((height, offset)): Path<(u64, u64)>| async move {
            state
                .get_transaction_detail(v1::TxIdent::HeightAndOffset(height, offset))
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let explorer_tx_detail_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_transaction_detail(v1::TxIdent::Hash(hash))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let explorer_tx_summaries_latest = |State(state): State<S>, Path(limit): Path<u64>| async move {
        state
            .get_transaction_summaries(v1::TxIdent::Latest, limit, v1::TxSummaryFilter::None)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let explorer_tx_summaries_from =
        |State(state): State<S>, Path((height, offset, limit)): Path<(u64, u64, u64)>| async move {
            state
                .get_transaction_summaries(
                    v1::TxIdent::HeightAndOffset(height, offset),
                    limit,
                    v1::TxSummaryFilter::None,
                )
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let explorer_tx_summaries_by_hash =
        |State(state): State<S>, Path((hash, limit)): Path<(String, u64)>| async move {
            state
                .get_transaction_summaries(
                    v1::TxIdent::Hash(hash),
                    limit,
                    v1::TxSummaryFilter::None,
                )
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let explorer_tx_summaries_latest_block =
        |State(state): State<S>, Path((limit, block)): Path<(u64, u64)>| async move {
            state
                .get_transaction_summaries(
                    v1::TxIdent::Latest,
                    limit,
                    v1::TxSummaryFilter::Block(block),
                )
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let explorer_tx_summaries_from_block =
        |State(state): State<S>,
         Path((height, offset, limit, block)): Path<(u64, u64, u64, u64)>| async move {
            state
                .get_transaction_summaries(
                    v1::TxIdent::HeightAndOffset(height, offset),
                    limit,
                    v1::TxSummaryFilter::Block(block),
                )
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let explorer_tx_summaries_by_hash_block =
        |State(state): State<S>, Path((hash, limit, block)): Path<(String, u64, u64)>| async move {
            state
                .get_transaction_summaries(
                    v1::TxIdent::Hash(hash),
                    limit,
                    v1::TxSummaryFilter::Block(block),
                )
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let explorer_tx_summaries_latest_ns =
        |State(state): State<S>, Path((limit, namespace)): Path<(u64, i64)>| async move {
            state
                .get_transaction_summaries(
                    v1::TxIdent::Latest,
                    limit,
                    v1::TxSummaryFilter::Namespace(namespace),
                )
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let explorer_tx_summaries_from_ns =
        |State(state): State<S>,
         Path((height, offset, limit, namespace)): Path<(u64, u64, u64, i64)>| async move {
            state
                .get_transaction_summaries(
                    v1::TxIdent::HeightAndOffset(height, offset),
                    limit,
                    v1::TxSummaryFilter::Namespace(namespace),
                )
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let explorer_tx_summaries_by_hash_ns = |State(state): State<S>,
                                            Path((hash, limit, namespace)): Path<(
        String,
        u64,
        i64,
    )>| async move {
        state
            .get_transaction_summaries(
                v1::TxIdent::Hash(hash),
                limit,
                v1::TxSummaryFilter::Namespace(namespace),
            )
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let explorer_summary = |State(state): State<S>| async move {
        state
            .get_explorer_summary()
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let explorer_search = |State(state): State<S>, Path(query): Path<String>| async move {
        state
            .get_search_result(query)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };

    // Token handlers
    let token_total_minted = |State(state): State<S>| async move {
        state
            .total_minted_supply()
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let token_circulating = |State(state): State<S>| async move {
        state
            .circulating_supply()
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let token_circulating_eth = |State(state): State<S>| async move {
        state
            .circulating_supply_ethereum()
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let token_total_issued = |State(state): State<S>| async move {
        state
            .total_issued_supply()
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let token_total_reward_distributed = |State(state): State<S>| async move {
        state
            .total_reward_distributed()
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };

    // Database handlers
    let database_table_sizes = |State(state): State<S>| async move {
        <S as v1::DatabaseApi>::get_table_sizes(&state)
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
        // Merklized state: block-state.
        .route(
            routes::v1::BLOCK_STATE_HEIGHT_ROUTE,
            get(get_block_state_height),
        )
        .route(
            routes::v1::BLOCK_STATE_PATH_BY_COMMIT_ROUTE,
            get(get_block_state_path_by_commit),
        )
        .route(
            routes::v1::BLOCK_STATE_PATH_BY_HEIGHT_ROUTE,
            get(get_block_state_path_by_height),
        )
        // Merklized state: fee-state
        .route(
            routes::v1::FEE_STATE_HEIGHT_ROUTE,
            get(get_fee_state_height),
        )
        .route(
            routes::v1::FEE_STATE_BALANCE_LATEST_ROUTE,
            get(get_fee_balance_latest),
        )
        .route(
            routes::v1::FEE_STATE_PATH_BY_COMMIT_ROUTE,
            get(get_fee_state_path_by_commit),
        )
        .route(
            routes::v1::FEE_STATE_PATH_BY_HEIGHT_ROUTE,
            get(get_fee_state_path_by_height),
        )
        // Status routes
        .route(routes::v1::STATUS_BLOCK_HEIGHT_ROUTE, get(status_block_height))
        .route(routes::v1::STATUS_SUCCESS_RATE_ROUTE, get(status_success_rate))
        .route(
            routes::v1::STATUS_TIME_SINCE_LAST_DECIDE_ROUTE,
            get(status_time_since_last_decide),
        )
        .route(routes::v1::STATUS_METRICS_ROUTE, get(status_metrics))
        // Config routes
        .route(routes::v1::CONFIG_HOTSHOT_ROUTE, get(config_hotshot))
        .route(routes::v1::CONFIG_ENV_ROUTE, get(config_env))
        .route(routes::v1::CONFIG_RUNTIME_ROUTE, get(config_runtime))
        // Node routes
        .route(routes::v1::NODE_BLOCK_HEIGHT_ROUTE, get(node_block_height))
        .route(routes::v1::NODE_TRANSACTIONS_COUNT_ROUTE, get(node_count_txs))
        .route(
            routes::v1::NODE_TRANSACTIONS_COUNT_NS_ROUTE,
            get(node_count_txs_ns),
        )
        .route(
            routes::v1::NODE_TRANSACTIONS_COUNT_NS_TO_ROUTE,
            get(node_count_txs_ns_to),
        )
        .route(
            routes::v1::NODE_TRANSACTIONS_COUNT_NS_FROM_TO_ROUTE,
            get(node_count_txs_ns_from_to),
        )
        .route(
            routes::v1::NODE_TRANSACTIONS_COUNT_TO_ROUTE,
            get(node_count_txs_to),
        )
        .route(
            routes::v1::NODE_TRANSACTIONS_COUNT_FROM_TO_ROUTE,
            get(node_count_txs_from_to),
        )
        .route(routes::v1::NODE_PAYLOADS_SIZE_ROUTE, get(node_payload_size))
        .route(
            routes::v1::NODE_PAYLOADS_TOTAL_SIZE_ROUTE,
            get(node_payload_size),
        )
        .route(
            routes::v1::NODE_PAYLOADS_SIZE_NS_ROUTE,
            get(node_payload_size_ns),
        )
        .route(
            routes::v1::NODE_PAYLOADS_SIZE_NS_TO_ROUTE,
            get(node_payload_size_ns_to),
        )
        .route(
            routes::v1::NODE_PAYLOADS_SIZE_NS_FROM_TO_ROUTE,
            get(node_payload_size_ns_from_to),
        )
        .route(
            routes::v1::NODE_PAYLOADS_SIZE_TO_ROUTE,
            get(node_payload_size_to),
        )
        .route(
            routes::v1::NODE_PAYLOADS_SIZE_FROM_TO_ROUTE,
            get(node_payload_size_from_to),
        )
        .route(
            routes::v1::NODE_VID_SHARE_BY_HASH_ROUTE,
            get(node_vid_share_by_hash),
        )
        .route(
            routes::v1::NODE_VID_SHARE_BY_PAYLOAD_HASH_ROUTE,
            get(node_vid_share_by_payload_hash),
        )
        .route(
            routes::v1::NODE_VID_SHARE_BY_HEIGHT_ROUTE,
            get(node_vid_share_by_height),
        )
        .route(routes::v1::NODE_SYNC_STATUS_ROUTE, get(node_sync_status))
        .route(
            routes::v1::NODE_HEADER_WINDOW_HASH_ROUTE,
            get(node_header_window_hash),
        )
        .route(
            routes::v1::NODE_HEADER_WINDOW_HEIGHT_ROUTE,
            get(node_header_window_height),
        )
        .route(
            routes::v1::NODE_HEADER_WINDOW_TIME_ROUTE,
            get(node_header_window_time),
        )
        .route(routes::v1::NODE_LIMITS_ROUTE, get(node_limits))
        .route(
            routes::v1::NODE_STAKE_TABLE_CURRENT_ROUTE,
            get(node_stake_table_current),
        )
        .route(routes::v1::NODE_STAKE_TABLE_ROUTE, get(node_stake_table))
        .route(
            routes::v1::NODE_DA_STAKE_TABLE_CURRENT_ROUTE,
            get(node_da_stake_table_current),
        )
        .route(routes::v1::NODE_DA_STAKE_TABLE_ROUTE, get(node_da_stake_table))
        .route(routes::v1::NODE_VALIDATORS_ROUTE, get(node_validators))
        .route(routes::v1::NODE_ALL_VALIDATORS_ROUTE, get(node_all_validators))
        .route(
            routes::v1::NODE_PROPOSAL_PARTICIPATION_CURRENT_ROUTE,
            get(node_proposal_participation_current),
        )
        .route(
            routes::v1::NODE_PROPOSAL_PARTICIPATION_ROUTE,
            get(node_proposal_participation),
        )
        .route(
            routes::v1::NODE_VOTE_PARTICIPATION_CURRENT_ROUTE,
            get(node_vote_participation_current),
        )
        .route(
            routes::v1::NODE_VOTE_PARTICIPATION_ROUTE,
            get(node_vote_participation),
        )
        .route(routes::v1::NODE_BLOCK_REWARD_ROUTE, get(node_block_reward))
        .route(
            routes::v1::NODE_BLOCK_REWARD_EPOCH_ROUTE,
            get(node_block_reward_epoch),
        )
        .route(routes::v1::NODE_OLDEST_BLOCK_ROUTE, get(node_oldest_block))
        .route(routes::v1::NODE_OLDEST_LEAF_ROUTE, get(node_oldest_leaf))
        // Catchup routes
        .route(routes::v1::CATCHUP_ACCOUNT_ROUTE, get(catchup_account))
        .route(routes::v1::CATCHUP_ACCOUNTS_ROUTE, ::axum::routing::post(catchup_accounts))
        .route(routes::v1::CATCHUP_BLOCKS_ROUTE, get(catchup_blocks))
        .route(routes::v1::CATCHUP_CHAINCONFIG_ROUTE, get(catchup_chainconfig))
        .route(routes::v1::CATCHUP_LEAFCHAIN_ROUTE, get(catchup_leafchain))
        .route(routes::v1::CATCHUP_CERT2_ROUTE, get(catchup_cert2))
        .route(
            routes::v1::CATCHUP_REWARD_ACCOUNT_ROUTE,
            get(catchup_reward_account),
        )
        .route(
            routes::v1::CATCHUP_REWARD_ACCOUNTS_ROUTE,
            ::axum::routing::post(catchup_reward_accounts),
        )
        .route(
            routes::v1::CATCHUP_REWARD_ACCOUNT_V2_ROUTE,
            get(catchup_reward_account_v2),
        )
        .route(
            routes::v1::CATCHUP_REWARD_ACCOUNTS_V2_ROUTE,
            ::axum::routing::post(catchup_reward_accounts_v2),
        )
        .route(
            routes::v1::CATCHUP_REWARD_AMOUNTS_ROUTE,
            get(catchup_reward_amounts),
        )
        .route(
            routes::v1::CATCHUP_REWARD_MERKLE_TREE_V2_ROUTE,
            get(catchup_reward_merkle_tree_v2),
        )
        .route(routes::v1::CATCHUP_STATE_CERT_ROUTE, get(catchup_state_cert))
        // Submit
        .route(routes::v1::SUBMIT_ROUTE, ::axum::routing::post(submit_submit))
        // State signature
        .route(
            routes::v1::STATE_SIGNATURE_BLOCK_ROUTE,
            get(state_signature_block),
        )
        // HotShot events
        .route(
            routes::v1::HOTSHOT_EVENTS_STARTUP_ROUTE,
            get(hotshot_events_startup),
        )
        .route(
            routes::v1::HOTSHOT_EVENTS_STREAM_ROUTE,
            get(hotshot_events_stream),
        )
        // Light client
        .route(routes::v1::LC_LEAF_BY_HEIGHT_ROUTE, get(lc_leaf_by_height))
        .route(
            routes::v1::LC_LEAF_BY_HEIGHT_FINALIZED_ROUTE,
            get(lc_leaf_by_height_finalized),
        )
        .route(routes::v1::LC_LEAF_BY_HASH_ROUTE, get(lc_leaf_by_hash))
        .route(
            routes::v1::LC_LEAF_BY_HASH_FINALIZED_ROUTE,
            get(lc_leaf_by_hash_finalized),
        )
        .route(
            routes::v1::LC_LEAF_BY_BLOCK_HASH_ROUTE,
            get(lc_leaf_by_block_hash),
        )
        .route(
            routes::v1::LC_LEAF_BY_BLOCK_HASH_FINALIZED_ROUTE,
            get(lc_leaf_by_block_hash_finalized),
        )
        .route(
            routes::v1::LC_LEAF_BY_PAYLOAD_HASH_ROUTE,
            get(lc_leaf_by_payload_hash),
        )
        .route(
            routes::v1::LC_LEAF_BY_PAYLOAD_HASH_FINALIZED_ROUTE,
            get(lc_leaf_by_payload_hash_finalized),
        )
        .route(routes::v1::LC_HEADER_BY_HEIGHT_ROUTE, get(lc_header_by_height))
        .route(routes::v1::LC_HEADER_BY_HASH_ROUTE, get(lc_header_by_hash))
        .route(
            routes::v1::LC_HEADER_BY_PAYLOAD_HASH_ROUTE,
            get(lc_header_by_payload_hash),
        )
        .route(routes::v1::LC_STAKE_TABLE_ROUTE, get(lc_stake_table))
        .route(routes::v1::LC_PAYLOAD_ROUTE, get(lc_payload))
        .route(routes::v1::LC_PAYLOAD_RANGE_ROUTE, get(lc_payload_range))
        .route(routes::v1::LC_NAMESPACE_ROUTE, get(lc_namespace))
        .route(
            routes::v1::LC_NAMESPACE_RANGE_ROUTE,
            get(lc_namespace_range),
        )
        // Explorer
        .route(
            routes::v1::EXPLORER_BLOCK_DETAIL_BY_HEIGHT_ROUTE,
            get(explorer_block_detail_by_height),
        )
        .route(
            routes::v1::EXPLORER_BLOCK_DETAIL_BY_HASH_ROUTE,
            get(explorer_block_detail_by_hash),
        )
        .route(
            routes::v1::EXPLORER_BLOCK_SUMMARIES_LATEST_ROUTE,
            get(explorer_block_summaries_latest),
        )
        .route(
            routes::v1::EXPLORER_BLOCK_SUMMARIES_FROM_ROUTE,
            get(explorer_block_summaries_from),
        )
        .route(
            routes::v1::EXPLORER_TX_DETAIL_BY_POSITION_ROUTE,
            get(explorer_tx_detail_by_position),
        )
        .route(
            routes::v1::EXPLORER_TX_DETAIL_BY_HASH_ROUTE,
            get(explorer_tx_detail_by_hash),
        )
        .route(
            routes::v1::EXPLORER_TX_SUMMARIES_LATEST_BLOCK_ROUTE,
            get(explorer_tx_summaries_latest_block),
        )
        .route(
            routes::v1::EXPLORER_TX_SUMMARIES_FROM_BLOCK_ROUTE,
            get(explorer_tx_summaries_from_block),
        )
        .route(
            routes::v1::EXPLORER_TX_SUMMARIES_BY_HASH_BLOCK_ROUTE,
            get(explorer_tx_summaries_by_hash_block),
        )
        .route(
            routes::v1::EXPLORER_TX_SUMMARIES_LATEST_NS_ROUTE,
            get(explorer_tx_summaries_latest_ns),
        )
        .route(
            routes::v1::EXPLORER_TX_SUMMARIES_FROM_NS_ROUTE,
            get(explorer_tx_summaries_from_ns),
        )
        .route(
            routes::v1::EXPLORER_TX_SUMMARIES_BY_HASH_NS_ROUTE,
            get(explorer_tx_summaries_by_hash_ns),
        )
        .route(
            routes::v1::EXPLORER_TX_SUMMARIES_LATEST_ROUTE,
            get(explorer_tx_summaries_latest),
        )
        .route(
            routes::v1::EXPLORER_TX_SUMMARIES_FROM_ROUTE,
            get(explorer_tx_summaries_from),
        )
        .route(
            routes::v1::EXPLORER_TX_SUMMARIES_BY_HASH_ROUTE,
            get(explorer_tx_summaries_by_hash),
        )
        .route(routes::v1::EXPLORER_SUMMARY_ROUTE, get(explorer_summary))
        .route(routes::v1::EXPLORER_SEARCH_ROUTE, get(explorer_search))
        // Token
        .route(
            routes::v1::TOKEN_TOTAL_MINTED_SUPPLY_ROUTE,
            get(token_total_minted),
        )
        .route(
            routes::v1::TOKEN_CIRCULATING_SUPPLY_ROUTE,
            get(token_circulating),
        )
        .route(
            routes::v1::TOKEN_CIRCULATING_SUPPLY_ETHEREUM_ROUTE,
            get(token_circulating_eth),
        )
        .route(
            routes::v1::TOKEN_TOTAL_ISSUED_SUPPLY_ROUTE,
            get(token_total_issued),
        )
        .route(
            routes::v1::TOKEN_TOTAL_REWARD_DISTRIBUTED_ROUTE,
            get(token_total_reward_distributed),
        )
        // Database (diagnostic)
        .route(
            routes::v1::DATABASE_TABLE_SIZES_ROUTE,
            get(database_table_sizes),
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
