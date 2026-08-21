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
    body::Bytes,
    extract::{Path, Request, State, ws::WebSocketUpgrade},
    http::{HeaderMap, StatusCode, Uri, header},
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
use vbs::{BinarySerializer, Serializer, version::StaticVersion};

use crate::{
    error::{ApiError, AvailabilityError},
    handlers, v1, v2,
};

/// API error response — wire-compatible with the `Custom` variant of the per-module error enums
/// (`node::Error::Custom`, `merklized_state::Error::Custom`, etc.) that all of tide-disco's
/// `Error::catch_all` calls produce. Most of our migrated endpoints (catchup, submit,
/// state-signature, light-client, node, status, config, token, database) take that path, so this
/// envelope is byte-identical with tide's error response for them. Endpoints that use a specific
/// variant directly (e.g. `availability::Error::FetchLeaf`) emit their own shape on tide; those
/// bytes are not matched here.
#[derive(Debug, Serialize)]
struct ErrorResponse {
    #[serde(rename = "Custom")]
    custom: CustomError,
}

#[derive(Debug, Serialize)]
struct CustomError {
    // Field order matches `node::Error::Custom { message, status }` declaration so serde_json
    // emits the same key order on the wire.
    message: String,
    status: u16,
}

<<<<<<< HEAD
||||||| parent of 355c72eab8f (fix(telemetry): stop logging L1 provider credentials (#4783))
impl ErrorResponse {
    fn new(status: StatusCode, message: String) -> Self {
        Self {
            custom: CustomError {
                message,
                status: status.as_u16(),
            },
        }
    }
}

=======
impl ErrorResponse {
    /// Scrubs credentials out of the message. `ApiError`'s `Display` already does this for handler
    /// errors, which the tonic adapter shares; this also covers messages that never went through
    /// it, such as a serialization failure. Error bodies go to unauthenticated callers and never
    /// pass the OTLP exporter.
    fn new(status: StatusCode, message: String) -> Self {
        Self {
            custom: CustomError {
                message: espresso_utils::redact::scrub(&message),
                status: status.as_u16(),
            },
        }
    }
}

>>>>>>> 355c72eab8f (fix(telemetry): stop logging L1 provider credentials (#4783))
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(ErrorResponse {
            custom: CustomError {
                message: self.to_string(),
                status: status.as_u16(),
            },
        });

        (status, body).into_response()
    }
}

/// Encode a successful response body based on the request's `Accept` header, matching
/// tide-disco's content negotiation.
///
/// surf-disco's default `Accept` is `application/octet-stream`, so production internal clients
/// (peer-catchup, submit-transactions, light-client provider) expect VBS-encoded responses for
/// the endpoints that flow large structured data. Falls back to JSON otherwise.
fn encode_response<T: Serialize>(headers: &HeaderMap, value: T) -> Result<Response, ApiError> {
    let accept = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if accept.contains("application/octet-stream") {
        let bytes = Serializer::<StaticVersion<0, 1>>::serialize(&value)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("vbs serialize: {e}")))?;
        Ok(([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response())
    } else {
        Ok(Json(value).into_response())
    }
}

/// Decode a request body based on its `Content-Type`, matching tide-disco's `body_auto` behavior.
///
/// - `application/octet-stream`: VBS (versioned binary) — what `surf-disco::Request::body_binary`
///   sends, and what production peer-catchup / submit-transactions clients use.
/// - `application/json`: serde_json.
///
/// All v1 endpoints in this codebase use the V0_1 API version for VBS framing.
fn decode_body<T: serde::de::DeserializeOwned>(
    headers: &HeaderMap,
    body: &[u8],
) -> Result<T, ApiError> {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok());
    match content_type {
        Some(ct) if ct.starts_with("application/octet-stream") => {
            Serializer::<StaticVersion<0, 1>>::deserialize(body)
                .map_err(|e| ApiError::BadRequest(anyhow::anyhow!("invalid binary body: {e}")))
        },
        Some(ct) if ct.starts_with("application/json") => serde_json::from_slice(body)
            .map_err(|e| ApiError::BadRequest(anyhow::anyhow!("invalid json body: {e}"))),
        Some(other) => Err(ApiError::BadRequest(anyhow::anyhow!(
            "unsupported Content-Type: {other}"
        ))),
        None => Err(ApiError::BadRequest(anyhow::anyhow!(
            "missing Content-Type header"
        ))),
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

/// Wire format for a WebSocket stream — negotiated from the upgrade request's `Accept` header
/// to match tide-disco. surf-disco clients default to `application/octet-stream`, so production
/// stream consumers expect VBS-encoded `Message::Binary` frames.
#[derive(Clone, Copy)]
enum WsFormat {
    Binary,
    Json,
}

fn ws_format(headers: &HeaderMap) -> WsFormat {
    let accept = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if accept.contains("application/octet-stream") {
        WsFormat::Binary
    } else {
        WsFormat::Json
    }
}

async fn drive_ws_stream<T: Serialize>(
    mut socket: axum::extract::ws::WebSocket,
    stream: BoxStream<'static, T>,
    format: WsFormat,
) {
    use axum::extract::ws::Message;
    use futures::StreamExt as _;
    futures::pin_mut!(stream);
    while let Some(item) = stream.next().await {
        let msg = match format {
            WsFormat::Binary => match Serializer::<StaticVersion<0, 1>>::serialize(&item) {
                Ok(bytes) => Message::Binary(bytes.into()),
                Err(_) => break,
            },
            WsFormat::Json => match serde_json::to_string(&item) {
                Ok(json) => Message::Text(json.into()),
                Err(_) => break,
            },
        };
        if socket.send(msg).await.is_err() {
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
    let stream_leaves = |ws: WebSocketUpgrade,
                         State(state): State<S>,
                         headers: HeaderMap,
                         Path(height): Path<usize>| async move {
        let format = ws_format(&headers);
        ws.on_upgrade(move |socket| async move {
            match state.stream_leaves(height).await {
                Ok(stream) => drive_ws_stream(socket, stream, format).await,
                Err(e) => tracing::warn!("stream_leaves: {e}"),
            }
        })
    };
    let stream_headers = |ws: WebSocketUpgrade,
                          State(state): State<S>,
                          headers: HeaderMap,
                          Path(height): Path<usize>| async move {
        let format = ws_format(&headers);
        ws.on_upgrade(move |socket| async move {
            match state.stream_headers(height).await {
                Ok(stream) => drive_ws_stream(socket, stream, format).await,
                Err(e) => tracing::warn!("stream_headers: {e}"),
            }
        })
    };
    let stream_blocks = |ws: WebSocketUpgrade,
                         State(state): State<S>,
                         headers: HeaderMap,
                         Path(height): Path<usize>| async move {
        let format = ws_format(&headers);
        ws.on_upgrade(move |socket| async move {
            match state.stream_blocks(height).await {
                Ok(stream) => drive_ws_stream(socket, stream, format).await,
                Err(e) => tracing::warn!("stream_blocks: {e}"),
            }
        })
    };
    let stream_payloads = |ws: WebSocketUpgrade,
                           State(state): State<S>,
                           headers: HeaderMap,
                           Path(height): Path<usize>| async move {
        let format = ws_format(&headers);
        ws.on_upgrade(move |socket| async move {
            match state.stream_payloads(height).await {
                Ok(stream) => drive_ws_stream(socket, stream, format).await,
                Err(e) => tracing::warn!("stream_payloads: {e}"),
            }
        })
    };
    let stream_vid_common = |ws: WebSocketUpgrade,
                             State(state): State<S>,
                             headers: HeaderMap,
                             Path(height): Path<usize>| async move {
        let format = ws_format(&headers);
        ws.on_upgrade(move |socket| async move {
            match state.stream_vid_common(height).await {
                Ok(stream) => drive_ws_stream(socket, stream, format).await,
                Err(e) => tracing::warn!("stream_vid_common: {e}"),
            }
        })
    };
    let stream_transactions = |ws: WebSocketUpgrade,
                               State(state): State<S>,
                               headers: HeaderMap,
                               Path(height): Path<usize>| async move {
        let format = ws_format(&headers);
        ws.on_upgrade(move |socket| async move {
            match state.stream_transactions(height, None).await {
                Ok(stream) => drive_ws_stream(socket, stream, format).await,
                Err(e) => tracing::warn!("stream_transactions: {e}"),
            }
        })
    };
    let stream_transactions_ns =
        |ws: WebSocketUpgrade,
         State(state): State<S>,
         headers: HeaderMap,
         Path((height, namespace)): Path<(usize, u32)>| async move {
            let format = ws_format(&headers);
            ws.on_upgrade(move |socket| async move {
                match state.stream_transactions(height, Some(namespace)).await {
                    Ok(stream) => drive_ws_stream(socket, stream, format).await,
                    Err(e) => tracing::warn!("stream_transactions_ns: {e}"),
                }
            })
        };
    let stream_namespace_proofs =
        |ws: WebSocketUpgrade,
         State(state): State<S>,
         headers: HeaderMap,
         Path((height, namespace)): Path<(usize, u32)>| async move {
            let format = ws_format(&headers);
            ws.on_upgrade(move |socket| async move {
                match state.stream_namespace_proofs(height, namespace).await {
                    Ok(stream) => drive_ws_stream(socket, stream, format).await,
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
            .map_err(classify_availability_error)
    };

    let node_count_txs = |State(state): State<S>| async move {
        state
            .count_transactions(None, None, None)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let node_count_txs_to = |State(state): State<S>, Path(to): Path<u64>| async move {
        state
            .count_transactions(None, Some(to), None)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let node_count_txs_from_to = |State(state): State<S>, Path((from, to)): Path<(u64, u64)>| async move {
        state
            .count_transactions(Some(from), Some(to), None)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let node_count_txs_ns = |State(state): State<S>, Path(namespace): Path<u64>| async move {
        state
            .count_transactions(None, None, Some(namespace))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let node_count_txs_ns_to = |State(state): State<S>, Path((namespace, to)): Path<(u64, u64)>| async move {
        state
            .count_transactions(None, Some(to), Some(namespace))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let node_count_txs_ns_from_to =
        |State(state): State<S>, Path((namespace, from, to)): Path<(u64, u64, u64)>| async move {
            state
                .count_transactions(Some(from), Some(to), Some(namespace))
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };

    let node_payload_size = |State(state): State<S>| async move {
        state
            .payload_size(None, None, None)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let node_payload_size_to = |State(state): State<S>, Path(to): Path<u64>| async move {
        state
            .payload_size(None, Some(to), None)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let node_payload_size_from_to = |State(state): State<S>, Path((from, to)): Path<(u64, u64)>| async move {
        state
            .payload_size(Some(from), Some(to), None)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let node_payload_size_ns = |State(state): State<S>, Path(namespace): Path<u64>| async move {
        state
            .payload_size(None, None, Some(namespace))
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let node_payload_size_ns_to =
        |State(state): State<S>, Path((namespace, to)): Path<(u64, u64)>| async move {
            state
                .payload_size(None, Some(to), Some(namespace))
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let node_payload_size_ns_from_to =
        |State(state): State<S>, Path((namespace, from, to)): Path<(u64, u64, u64)>| async move {
            state
                .payload_size(Some(from), Some(to), Some(namespace))
                .await
                .map(Json)
                .map_err(classify_availability_error)
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
            .map_err(classify_availability_error)
    };

    let node_header_window_time = |State(state): State<S>, Path((start, end)): Path<(u64, u64)>| async move {
        state
            .get_header_window(v1::HeaderWindowStart::Time(start), end)
            .await
            .map(Json)
            .map_err(classify_availability_error)
    };
    let node_header_window_height =
        |State(state): State<S>, Path((height, end)): Path<(u64, u64)>| async move {
            state
                .get_header_window(v1::HeaderWindowStart::Height(height), end)
                .await
                .map(Json)
                .map_err(classify_availability_error)
        };
    let node_header_window_hash =
        |State(state): State<S>, Path((hash, end)): Path<(String, u64)>| async move {
            state
                .get_header_window(v1::HeaderWindowStart::Hash(hash), end)
                .await
                .map(Json)
                .map_err(classify_availability_error)
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
                            headers: HeaderMap,
                            body: Bytes| async move {
        let accounts: Vec<<S as v1::CatchupApi>::FeeAccount> = decode_body(&headers, &body)?;
        let tree = state
            .get_accounts(height, view, accounts)
            .await
            .map_err(classify_availability_error)?;
        encode_response(&headers, tree)
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
    let catchup_reward_accounts = |State(state): State<S>,
                                   Path((height, view)): Path<(u64, u64)>,
                                   headers: HeaderMap,
                                   body: Bytes| async move {
        let accounts: Vec<<S as v1::CatchupApi>::RewardAccountV1> = decode_body(&headers, &body)?;
        let tree = state
            .get_reward_accounts_v1(height, view, accounts)
            .await
            .map_err(classify_availability_error)?;
        encode_response(&headers, tree)
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

    // Submit handler — body is decoded as VBS (binary) or JSON based on Content-Type, matching
    // tide-disco's `body_auto`.
    let submit_submit = |State(state): State<S>, headers: HeaderMap, body: Bytes| async move {
        let tx: <S as v1::SubmitApi>::Transaction = decode_body(&headers, &body)?;
        let hash = state.submit(tx).await.map_err(ApiError::Internal)?;
        encode_response(&headers, hash)
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
    let hotshot_events_stream =
        |State(state): State<S>, headers: HeaderMap, ws: WebSocketUpgrade| async move {
            let format = ws_format(&headers);
            match <S as v1::HotShotEventsApi>::events(&state).await {
                Ok(stream) => ws.on_upgrade(move |socket| drive_ws_stream(socket, stream, format)),
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
<<<<<<< HEAD
||||||| parent of 355c72eab8f (fix(telemetry): stop logging L1 provider credentials (#4783))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use futures::stream::BoxStream;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use vbs::{BinarySerializer, Serializer};

    use super::*;

    fn rewritten_uri(uri: &str) -> String {
        let req = Request::builder()
            .uri(uri)
            .body(axum::body::Body::empty())
            .unwrap();
        rewrite_legacy_uri(req).uri().to_string()
    }

    #[test]
    fn rewrite_legacy_uri_prefixes_unversioned_paths() {
        assert_eq!(
            rewritten_uri("/status/block-height"),
            "/v1/status/block-height"
        );
    }

    #[test]
    fn rewrite_legacy_uri_rewrites_v0_to_v1() {
        assert_eq!(
            rewritten_uri("/v0/status/block-height"),
            "/v1/status/block-height"
        );
        assert_eq!(rewritten_uri("/v0"), "/v1");
    }

    #[test]
    fn rewrite_legacy_uri_rewrites_v0_availability_paths() {
        assert_eq!(
            rewritten_uri("/v0/availability/block/1/namespace/2"),
            "/v1/availability/block/1/namespace/2"
        );
        assert_eq!(
            rewritten_uri("/v0/availability/leaf/1"),
            "/v1/availability/leaf/1"
        );
        assert_eq!(
            rewritten_uri("/v0/availability/vid/common/1"),
            "/v1/availability/vid/common/1"
        );
        assert_eq!(
            rewritten_uri("/v0/availability/stream/leaves/0"),
            "/v1/availability/stream/leaves/0"
        );
        assert_eq!(
            rewritten_uri("/availability/block/1/namespace/2"),
            "/v1/availability/block/1/namespace/2"
        );
        assert_eq!(
            rewritten_uri("/availability/leaf/1"),
            "/v1/availability/leaf/1"
        );
    }

    #[test]
    fn rewrite_legacy_uri_leaves_v1_unchanged() {
        assert_eq!(
            rewritten_uri("/v1/node/block-height"),
            "/v1/node/block-height"
        );
    }

    #[test]
    fn rewrite_legacy_uri_leaves_v2_unchanged() {
        assert_eq!(
            rewritten_uri("/v2/rewards/balance/0xabc"),
            "/v2/rewards/balance/0xabc"
        );
    }

    #[test]
    fn rewrite_legacy_uri_respects_version_prefix_boundaries() {
        assert_eq!(rewritten_uri("/v1"), "/v1");
        assert_eq!(rewritten_uri("/v2"), "/v2");
        assert_eq!(rewritten_uri("/v1x"), "/v1/v1x");
        assert_eq!(rewritten_uri("/v2-foo/bar"), "/v1/v2-foo/bar");
        assert_eq!(rewritten_uri("/v0x/leaf"), "/v1/v0x/leaf");
    }

    #[test]
    fn rewrite_legacy_uri_leaves_reserved_paths_unchanged() {
        assert_eq!(rewritten_uri("/"), "/");
        assert_eq!(rewritten_uri("/healthcheck"), "/healthcheck");
        assert_eq!(rewritten_uri("/version"), "/version");
    }

    #[test]
    fn rewrite_legacy_uri_preserves_query_string() {
        assert_eq!(
            rewritten_uri("/availability/leaf/1?foo=bar"),
            "/v1/availability/leaf/1?foo=bar"
        );
    }

    /// Implements every v1 API trait with `unimplemented!()` bodies, purely so `create_router_v1`
    /// can be instantiated in tests that only exercise the static docs routes (root redirect,
    /// swagger UI, OpenAPI spec) and never call into a handler.
    #[derive(Clone)]
    struct MockState;

    #[async_trait::async_trait]
    impl v1::RewardApi for MockState {
        type RewardClaimInput = ();
        type RewardBalance = ();
        type RewardAccountQueryData = ();
        type RewardAmounts = ();
        type RewardMerkleTreeData = ();
        type RewardAccountQueryDataV1 = ();
        type RewardStatePathV1 = ();
        type RewardStatePathV2 = ();

        async fn get_reward_state_height(&self) -> anyhow::Result<u64> {
            unimplemented!()
        }
        async fn get_reward_state_v2_height(&self) -> anyhow::Result<u64> {
            unimplemented!()
        }
        async fn get_reward_account_proof_v1(
            &self,
            _height: u64,
            _address: String,
        ) -> anyhow::Result<Self::RewardAccountQueryDataV1> {
            unimplemented!()
        }
        async fn get_reward_claim_input(
            &self,
            _block_height: u64,
            _address: String,
        ) -> anyhow::Result<Self::RewardClaimInput> {
            unimplemented!()
        }
        async fn get_reward_balance(
            &self,
            _height: u64,
            _address: String,
        ) -> anyhow::Result<Self::RewardBalance> {
            unimplemented!()
        }
        async fn get_latest_reward_balance(
            &self,
            _address: String,
        ) -> anyhow::Result<Self::RewardBalance> {
            unimplemented!()
        }
        async fn get_reward_account_proof(
            &self,
            _height: u64,
            _address: String,
        ) -> anyhow::Result<Self::RewardAccountQueryData> {
            unimplemented!()
        }
        async fn get_latest_reward_account_proof(
            &self,
            _address: String,
        ) -> anyhow::Result<Self::RewardAccountQueryData> {
            unimplemented!()
        }
        async fn get_reward_amounts(
            &self,
            _height: u64,
            _offset: u64,
            _limit: u64,
        ) -> anyhow::Result<Self::RewardAmounts> {
            unimplemented!()
        }
        async fn get_reward_merkle_tree_v2(
            &self,
            _height: u64,
        ) -> anyhow::Result<Self::RewardMerkleTreeData> {
            unimplemented!()
        }
        async fn get_reward_state_path_v1(
            &self,
            _snapshot: v1::merklized_state::Snapshot,
            _key: String,
        ) -> anyhow::Result<Self::RewardStatePathV1> {
            unimplemented!()
        }
        async fn get_reward_state_path_v2(
            &self,
            _snapshot: v1::merklized_state::Snapshot,
            _key: String,
        ) -> anyhow::Result<Self::RewardStatePathV2> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::AvailabilityApi for MockState {
        type NamespaceProofQueryData = ();
        type IncorrectEncodingProof = ();
        type StateCertQueryDataV1 = ();
        type StateCertQueryDataV2 = ();

        async fn get_namespace_proof(
            &self,
            _block_id: v1::BlockId,
            _namespace: u32,
        ) -> anyhow::Result<Self::NamespaceProofQueryData> {
            unimplemented!()
        }
        async fn get_namespace_proof_range(
            &self,
            _from: u64,
            _until: u64,
            _namespace: u32,
        ) -> anyhow::Result<Vec<Self::NamespaceProofQueryData>> {
            unimplemented!()
        }
        async fn stream_namespace_proofs(
            &self,
            _from: usize,
            _namespace: u32,
        ) -> anyhow::Result<BoxStream<'static, Self::NamespaceProofQueryData>> {
            unimplemented!()
        }
        async fn get_incorrect_encoding_proof(
            &self,
            _block_id: v1::BlockId,
            _namespace: u32,
        ) -> anyhow::Result<Self::IncorrectEncodingProof> {
            unimplemented!()
        }
        async fn get_state_cert(&self, _epoch: u64) -> anyhow::Result<Self::StateCertQueryDataV1> {
            unimplemented!()
        }
        async fn get_state_cert_v2(
            &self,
            _epoch: u64,
        ) -> anyhow::Result<Self::StateCertQueryDataV2> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::HotShotAvailabilityApi for MockState {
        type Leaf = ();
        type Block = ();
        type Header = ();
        type Payload = ();
        type VidCommon = ();
        type Transaction = ();
        type TransactionWithProof = ();
        type BlockSummary = ();
        type Limits = ();
        type Cert2 = ();

        async fn get_leaf(&self, _id: v1::LeafId) -> anyhow::Result<Self::Leaf> {
            unimplemented!()
        }
        async fn get_leaf_range(
            &self,
            _from: usize,
            _until: usize,
        ) -> anyhow::Result<Vec<Self::Leaf>> {
            unimplemented!()
        }
        async fn get_header(&self, _id: v1::BlockId) -> anyhow::Result<Self::Header> {
            unimplemented!()
        }
        async fn get_header_range(
            &self,
            _from: usize,
            _until: usize,
        ) -> anyhow::Result<Vec<Self::Header>> {
            unimplemented!()
        }
        async fn get_block(&self, _id: v1::BlockId) -> anyhow::Result<Self::Block> {
            unimplemented!()
        }
        async fn get_block_range(
            &self,
            _from: usize,
            _until: usize,
        ) -> anyhow::Result<Vec<Self::Block>> {
            unimplemented!()
        }
        async fn get_payload(&self, _id: v1::PayloadId) -> anyhow::Result<Self::Payload> {
            unimplemented!()
        }
        async fn get_payload_range(
            &self,
            _from: usize,
            _until: usize,
        ) -> anyhow::Result<Vec<Self::Payload>> {
            unimplemented!()
        }
        async fn get_vid_common(&self, _id: v1::BlockId) -> anyhow::Result<Self::VidCommon> {
            unimplemented!()
        }
        async fn get_vid_common_range(
            &self,
            _from: usize,
            _until: usize,
        ) -> anyhow::Result<Vec<Self::VidCommon>> {
            unimplemented!()
        }
        async fn get_transaction_by_position(
            &self,
            _height: u64,
            _index: u64,
        ) -> anyhow::Result<Self::Transaction> {
            unimplemented!()
        }
        async fn get_transaction_by_hash(
            &self,
            _hash: String,
        ) -> anyhow::Result<Self::Transaction> {
            unimplemented!()
        }
        async fn get_transaction_proof_by_position(
            &self,
            _height: u64,
            _index: u64,
        ) -> anyhow::Result<Self::TransactionWithProof> {
            unimplemented!()
        }
        async fn get_transaction_proof_by_hash(
            &self,
            _hash: String,
        ) -> anyhow::Result<Self::TransactionWithProof> {
            unimplemented!()
        }
        async fn get_block_summary(&self, _height: usize) -> anyhow::Result<Self::BlockSummary> {
            unimplemented!()
        }
        async fn get_block_summary_range(
            &self,
            _from: usize,
            _until: usize,
        ) -> anyhow::Result<Vec<Self::BlockSummary>> {
            unimplemented!()
        }
        async fn get_limits(&self) -> anyhow::Result<Self::Limits> {
            unimplemented!()
        }
        async fn get_cert2(&self, _height: u64) -> anyhow::Result<Option<Self::Cert2>> {
            unimplemented!()
        }
        async fn stream_leaves(
            &self,
            _from: usize,
        ) -> anyhow::Result<BoxStream<'static, Self::Leaf>> {
            unimplemented!()
        }
        async fn stream_headers(
            &self,
            _from: usize,
        ) -> anyhow::Result<BoxStream<'static, Self::Header>> {
            unimplemented!()
        }
        async fn stream_blocks(
            &self,
            _from: usize,
        ) -> anyhow::Result<BoxStream<'static, Self::Block>> {
            unimplemented!()
        }
        async fn stream_payloads(
            &self,
            _from: usize,
        ) -> anyhow::Result<BoxStream<'static, Self::Payload>> {
            unimplemented!()
        }
        async fn stream_vid_common(
            &self,
            _from: usize,
        ) -> anyhow::Result<BoxStream<'static, Self::VidCommon>> {
            unimplemented!()
        }
        async fn stream_transactions(
            &self,
            _from: usize,
            _namespace: Option<u32>,
        ) -> anyhow::Result<BoxStream<'static, Self::Transaction>> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::BlockStateApi for MockState {
        type MerkleProof = ();

        async fn get_block_state_path(
            &self,
            _snapshot: v1::merklized_state::Snapshot,
            _key: String,
        ) -> anyhow::Result<Self::MerkleProof> {
            unimplemented!()
        }
        async fn get_block_state_height(&self) -> anyhow::Result<u64> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::FeeStateApi for MockState {
        type MerkleProof = ();
        type FeeAmount = ();

        async fn get_fee_state_path(
            &self,
            _snapshot: v1::merklized_state::Snapshot,
            _key: String,
        ) -> anyhow::Result<Self::MerkleProof> {
            unimplemented!()
        }
        async fn get_fee_state_height(&self) -> anyhow::Result<u64> {
            unimplemented!()
        }
        async fn get_fee_balance_latest(
            &self,
            _address: String,
        ) -> anyhow::Result<Option<Self::FeeAmount>> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::StatusApi for MockState {
        type Keys = ();

        async fn block_height(&self) -> anyhow::Result<u64> {
            unimplemented!()
        }
        async fn success_rate(&self) -> anyhow::Result<f64> {
            unimplemented!()
        }
        async fn time_since_last_decide(&self) -> anyhow::Result<u64> {
            unimplemented!()
        }
        async fn metrics(&self) -> anyhow::Result<String> {
            unimplemented!()
        }
        async fn keys(&self) -> anyhow::Result<Self::Keys> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::ConfigApi for MockState {
        type HotShotConfig = ();
        type RuntimeConfig = ();

        async fn hotshot_config(&self) -> anyhow::Result<Self::HotShotConfig> {
            unimplemented!()
        }
        async fn env(&self) -> anyhow::Result<Vec<String>> {
            unimplemented!()
        }
        async fn runtime_config(&self) -> anyhow::Result<Self::RuntimeConfig> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::NodeApi for MockState {
        type VidShare = ();
        type SyncStatus = ();
        type HeaderWindow = ();
        type Limits = ();
        type StakeTable = ();
        type StakeTableCurrent = ();
        type Validators = ();
        type AllValidators = ();
        type Participation = ();
        type BlockReward = ();
        type Block = ();
        type Leaf = ();

        async fn block_height(&self) -> anyhow::Result<u64> {
            unimplemented!()
        }
        async fn count_transactions(
            &self,
            _from: Option<u64>,
            _to: Option<u64>,
            _namespace: Option<u64>,
        ) -> anyhow::Result<u64> {
            unimplemented!()
        }
        async fn payload_size(
            &self,
            _from: Option<u64>,
            _to: Option<u64>,
            _namespace: Option<u64>,
        ) -> anyhow::Result<u64> {
            unimplemented!()
        }
        async fn get_vid_share(&self, _id: v1::VidShareId) -> anyhow::Result<Self::VidShare> {
            unimplemented!()
        }
        async fn sync_status(&self) -> anyhow::Result<Self::SyncStatus> {
            unimplemented!()
        }
        async fn get_header_window(
            &self,
            _start: v1::HeaderWindowStart,
            _end: u64,
        ) -> anyhow::Result<Self::HeaderWindow> {
            unimplemented!()
        }
        async fn limits(&self) -> anyhow::Result<Self::Limits> {
            unimplemented!()
        }
        async fn stake_table(&self, _epoch: u64) -> anyhow::Result<Self::StakeTable> {
            unimplemented!()
        }
        async fn stake_table_current(&self) -> anyhow::Result<Self::StakeTableCurrent> {
            unimplemented!()
        }
        async fn da_stake_table(&self, _epoch: u64) -> anyhow::Result<Self::StakeTable> {
            unimplemented!()
        }
        async fn da_stake_table_current(&self) -> anyhow::Result<Self::StakeTableCurrent> {
            unimplemented!()
        }
        async fn get_validators(&self, _epoch: u64) -> anyhow::Result<Self::Validators> {
            unimplemented!()
        }
        async fn get_all_validators(
            &self,
            _epoch: u64,
            _offset: u64,
            _limit: u64,
        ) -> anyhow::Result<Self::AllValidators> {
            unimplemented!()
        }
        async fn current_proposal_participation(&self) -> anyhow::Result<Self::Participation> {
            unimplemented!()
        }
        async fn proposal_participation(&self, _epoch: u64) -> anyhow::Result<Self::Participation> {
            unimplemented!()
        }
        async fn current_vote_participation(&self) -> anyhow::Result<Self::Participation> {
            unimplemented!()
        }
        async fn vote_participation(&self, _epoch: u64) -> anyhow::Result<Self::Participation> {
            unimplemented!()
        }
        async fn get_block_reward(&self, _epoch: Option<u64>) -> anyhow::Result<Self::BlockReward> {
            unimplemented!()
        }
        async fn get_oldest_block(&self) -> anyhow::Result<Option<Self::Block>> {
            unimplemented!()
        }
        async fn get_oldest_leaf(&self) -> anyhow::Result<Option<Self::Leaf>> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::CatchupApi for MockState {
        type FeeAccount = ();
        type RewardAccountV1 = ();
        type RewardAccountV2 = ();
        type AccountQueryData = ();
        type FeeMerkleTree = ();
        type BlocksFrontier = ();
        type ChainConfig = ();
        type LeafChain = ();
        type Cert2 = ();
        type RewardAccountQueryDataV1 = ();
        type RewardMerkleTreeV1 = ();
        type RewardAccountQueryDataV2 = ();
        type RewardMerkleTreeV2Data = ();
        type StateCert = ();

        async fn get_account(
            &self,
            _height: u64,
            _view: u64,
            _address: String,
        ) -> anyhow::Result<Self::AccountQueryData> {
            unimplemented!()
        }
        async fn get_accounts(
            &self,
            _height: u64,
            _view: u64,
            _accounts: Vec<Self::FeeAccount>,
        ) -> anyhow::Result<Self::FeeMerkleTree> {
            unimplemented!()
        }
        async fn get_blocks_frontier(
            &self,
            _height: u64,
            _view: u64,
        ) -> anyhow::Result<Self::BlocksFrontier> {
            unimplemented!()
        }
        async fn get_chain_config(&self, _commitment: String) -> anyhow::Result<Self::ChainConfig> {
            unimplemented!()
        }
        async fn get_leaf_chain(&self, _height: u64) -> anyhow::Result<Self::LeafChain> {
            unimplemented!()
        }
        async fn get_cert2(&self, _height: u64) -> anyhow::Result<Self::Cert2> {
            unimplemented!()
        }
        async fn get_reward_account_v1(
            &self,
            _height: u64,
            _view: u64,
            _address: String,
        ) -> anyhow::Result<Self::RewardAccountQueryDataV1> {
            unimplemented!()
        }
        async fn get_reward_accounts_v1(
            &self,
            _height: u64,
            _view: u64,
            _accounts: Vec<Self::RewardAccountV1>,
        ) -> anyhow::Result<Self::RewardMerkleTreeV1> {
            unimplemented!()
        }
        async fn get_reward_account_v2(
            &self,
            _height: u64,
            _view: u64,
            _address: String,
        ) -> anyhow::Result<Self::RewardAccountQueryDataV2> {
            unimplemented!()
        }
        async fn get_reward_merkle_tree_v2(
            &self,
            _height: u64,
            _view: u64,
        ) -> anyhow::Result<Self::RewardMerkleTreeV2Data> {
            unimplemented!()
        }
        async fn get_state_cert(&self, _epoch: u64) -> anyhow::Result<Self::StateCert> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::SubmitApi for MockState {
        type Transaction = ();
        type TxHash = ();

        async fn submit(&self, _tx: Self::Transaction) -> anyhow::Result<Self::TxHash> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::StateSignatureApi for MockState {
        type Signature = ();

        async fn get_state_signature(&self, _height: u64) -> anyhow::Result<Self::Signature> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::HotShotEventsApi for MockState {
        type Event = ();
        type StartupInfo = ();

        async fn startup_info(&self) -> anyhow::Result<Self::StartupInfo> {
            unimplemented!()
        }
        async fn events(&self) -> anyhow::Result<BoxStream<'static, Self::Event>> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::LightClientApi for MockState {
        type LeafProof = ();
        type HeaderProof = ();
        type StakeTableEvents = ();
        type PayloadProof = ();
        type NamespaceProof = ();

        async fn get_leaf_proof(
            &self,
            _query: v1::LeafQuery,
            _finalized: Option<u64>,
        ) -> anyhow::Result<Self::LeafProof> {
            unimplemented!()
        }
        async fn get_header_proof(
            &self,
            _root: u64,
            _requested: v1::HeaderQuery,
        ) -> anyhow::Result<Self::HeaderProof> {
            unimplemented!()
        }
        async fn get_light_client_stake_table(
            &self,
            _epoch: u64,
        ) -> anyhow::Result<Self::StakeTableEvents> {
            unimplemented!()
        }
        async fn get_payload_proof(&self, _height: u64) -> anyhow::Result<Self::PayloadProof> {
            unimplemented!()
        }
        async fn get_payload_proof_range(
            &self,
            _start: u64,
            _end: u64,
        ) -> anyhow::Result<Vec<Self::PayloadProof>> {
            unimplemented!()
        }
        async fn get_lc_namespace_proof(
            &self,
            _height: u64,
            _namespace: u64,
        ) -> anyhow::Result<Self::NamespaceProof> {
            unimplemented!()
        }
        async fn get_lc_namespace_proof_range(
            &self,
            _start: u64,
            _end: u64,
            _namespace: u64,
        ) -> anyhow::Result<Vec<Self::NamespaceProof>> {
            unimplemented!()
        }
        async fn get_lc_namespaces_proof_range(
            &self,
            _start: u64,
            _end: u64,
            _namespaces: String,
        ) -> anyhow::Result<Vec<std::collections::HashMap<u64, Self::NamespaceProof>>> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::ExplorerApi for MockState {
        type BlockDetail = ();
        type BlockSummaries = ();
        type TransactionDetail = ();
        type TransactionSummaries = ();
        type ExplorerSummary = ();
        type SearchResult = ();

        async fn get_block_detail(
            &self,
            _ident: v1::BlockIdent,
        ) -> anyhow::Result<Self::BlockDetail> {
            unimplemented!()
        }
        async fn get_block_summaries(
            &self,
            _target: v1::BlockIdent,
            _limit: u64,
        ) -> anyhow::Result<Self::BlockSummaries> {
            unimplemented!()
        }
        async fn get_transaction_detail(
            &self,
            _ident: v1::TxIdent,
        ) -> anyhow::Result<Self::TransactionDetail> {
            unimplemented!()
        }
        async fn get_transaction_summaries(
            &self,
            _target: v1::TxIdent,
            _limit: u64,
            _filter: v1::TxSummaryFilter,
        ) -> anyhow::Result<Self::TransactionSummaries> {
            unimplemented!()
        }
        async fn get_explorer_summary(&self) -> anyhow::Result<Self::ExplorerSummary> {
            unimplemented!()
        }
        async fn get_search_result(&self, _query: String) -> anyhow::Result<Self::SearchResult> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::TokenApi for MockState {
        async fn total_minted_supply(&self) -> anyhow::Result<String> {
            unimplemented!()
        }
        async fn circulating_supply(&self) -> anyhow::Result<String> {
            unimplemented!()
        }
        async fn circulating_supply_ethereum(&self) -> anyhow::Result<String> {
            unimplemented!()
        }
        async fn total_issued_supply(&self) -> anyhow::Result<String> {
            unimplemented!()
        }
        async fn total_reward_distributed(&self) -> anyhow::Result<String> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::DatabaseApi for MockState {
        type TableSizes = ();
        type MigrationStatus = ();

        async fn get_table_sizes(&self) -> anyhow::Result<Self::TableSizes> {
            unimplemented!()
        }
        async fn get_migration_status(&self) -> anyhow::Result<Self::MigrationStatus> {
            unimplemented!()
        }
    }

    async fn body_string(resp: Response) -> String {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("read response body");
        String::from_utf8(bytes.to_vec()).expect("response body is utf8")
    }

    /// Checks that every response carries
    /// `Access-Control-Allow-Origin: *`: top-level routes, API routes merged in by the caller,
    /// error responses, and 404s. Also checks that an OPTIONS preflight is answered with the
    /// allow-origin, allow-methods, and allow-headers a browser requires.
    #[tokio::test]
    async fn responses_carry_cors_headers() {
        let router = with_top_level_routes(
            Router::new()
                .route("/v1/status/block-height", get(|| async { "0" }))
                .route(
                    "/v1/failing",
                    get(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
                ),
        )
        .layer(cors_layer());

        let allow_origin = |resp: &Response, uri: &str| {
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap_or_else(|| panic!("no CORS header on {uri}"))
                .clone()
        };

        for (uri, expected_status) in [
            ("/healthcheck", StatusCode::OK),
            ("/v1/status/block-height", StatusCode::OK),
            ("/v1/failing", StatusCode::INTERNAL_SERVER_ERROR),
            ("/no/such/route", StatusCode::NOT_FOUND),
        ] {
            let req = Request::builder()
                .uri(uri)
                .header(header::ORIGIN, "https://example.com")
                .body(axum::body::Body::empty())
                .unwrap();
            let resp = tower::ServiceExt::oneshot(router.clone(), req)
                .await
                .unwrap();
            assert_eq!(resp.status(), expected_status, "{uri}");
            assert_eq!(allow_origin(&resp, uri), "*", "{uri}");
        }

        // Browsers preflight non-simple requests (e.g. a JSON POST to submit) with OPTIONS and
        // require allow-origin, allow-methods, and allow-headers in the answer, even on routes
        // that only register GET handlers.
        let preflight = Request::builder()
            .method(axum::http::Method::OPTIONS)
            .uri("/v1/status/block-height")
            .header(header::ORIGIN, "https://example.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = tower::ServiceExt::oneshot(router, preflight).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(allow_origin(&resp, "preflight"), "*");
        assert_eq!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_METHODS)
                .expect("allow-methods on preflight"),
            "*"
        );
        assert_eq!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_HEADERS)
                .expect("allow-headers on preflight"),
            "*"
        );
    }

    /// Serves with zero connection slots so every request is shed, and checks the 429 still
    /// carries `Access-Control-Allow-Origin: *`.
    #[tokio::test]
    async fn shed_requests_carry_cors_headers() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(crate::serve_router(
            listener,
            "test",
            Router::new().route("/v1/status/block-height", get(|| async { "0" })),
            Some(0),
        ));

        let mut sock = tokio::net::TcpStream::connect(addr).await.unwrap();
        sock.write_all(
            b"GET /v1/status/block-height HTTP/1.1\r\nHost: localhost\r\nOrigin: https://example.com\r\n\r\n",
        )
        .await
        .unwrap();
        let mut head = Vec::new();
        let mut buf = [0u8; 512];
        while !head.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = sock.read(&mut buf).await.unwrap();
            assert!(
                n > 0,
                "connection closed before the response head: {head:?}"
            );
            head.extend_from_slice(&buf[..n]);
        }
        let head = String::from_utf8_lossy(&head).to_ascii_lowercase();
        assert!(head.contains("429"), "request must be shed: {head}");
        assert!(
            head.contains("access-control-allow-origin: *"),
            "no CORS header on 429: {head}"
        );
    }

    #[tokio::test]
    async fn root_redirects_to_v1() {
        let router = with_top_level_routes(Router::new());
        let req = Request::builder()
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(
            resp.headers().get(axum::http::header::LOCATION).unwrap(),
            "/v1"
        );
    }

    /// The app-level `/healthcheck` reports `tide_disco::app::AppHealth`, which is what every
    /// non-singleton tide app served; a module-level one reports the bare `HealthStatus`. Both
    /// shapes are load-bearing for clients built against the tide-disco servers.
    #[tokio::test]
    async fn healthcheck_shapes_match_tide() {
        let router = with_top_level_routes(Router::new());

        async fn get(router: &Router, uri: &str, accept: &str) -> Vec<u8> {
            let req = Request::builder()
                .uri(uri)
                .header(header::ACCEPT, accept)
                .body(axum::body::Body::empty())
                .unwrap();
            let resp = tower::ServiceExt::oneshot(router.clone(), req)
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK, "{uri}");
            axum::body::to_bytes(resp.into_body(), usize::MAX)
                .await
                .unwrap()
                .to_vec()
        }

        assert_eq!(
            get(&router, "/healthcheck", "application/json").await,
            br#"{"status":"available","modules":{}}"#
        );
        assert_eq!(
            get(&router, "/v1/status/healthcheck", "application/json").await,
            br#""available""#
        );

        // vbs field order (status ordinal, then modules map) must not change either: surf-disco
        // clients default to `Accept: application/octet-stream`.
        #[derive(Debug, PartialEq, serde::Deserialize)]
        enum TideHealthStatus {
            Available,
        }
        #[derive(Debug, PartialEq, serde::Deserialize)]
        struct TideAppHealth {
            status: TideHealthStatus,
            modules: BTreeMap<String, BTreeMap<u64, u16>>,
        }
        let binary = get(&router, "/healthcheck", "application/octet-stream").await;
        // `BuilderClient::connect` and the events-service wrapper poll this route and decode the
        // body as a bare `HealthStatus`. That only works because bincode allows trailing bytes and
        // `AppHealth`'s first field is the status ordinal, so the bare enum is a prefix of the
        // object. Reordering `AppHealth`'s fields would break both clients silently.
        assert_eq!(
            Serializer::<WireVersion>::deserialize::<TideHealthStatus>(&binary).unwrap(),
            TideHealthStatus::Available
        );
        assert_eq!(
            Serializer::<WireVersion>::deserialize::<TideAppHealth>(&binary).unwrap(),
            TideAppHealth {
                status: TideHealthStatus::Available,
                modules: BTreeMap::new(),
            }
        );
    }

    #[tokio::test]
    async fn v1_swagger_ui_serves_html() {
        let router = create_router_v1(MockState);
        let req = Request::builder()
            .uri("/v1")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let content_type = resp
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(content_type.contains("text/html"));
        let body = body_string(resp).await;
        assert!(body.contains(routes::v1::OPENAPI_SPEC_ROUTE));
    }

    #[tokio::test]
    async fn v1_openapi_spec_contains_known_route() {
        let router = create_router_v1(MockState);
        let req = Request::builder()
            .uri(routes::v1::OPENAPI_SPEC_ROUTE)
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        let spec: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
        assert!(
            spec["paths"]
                .as_object()
                .expect("spec has paths")
                .contains_key(routes::v1::STATUS_BLOCK_HEIGHT_ROUTE),
            "expected {} in spec paths: {}",
            routes::v1::STATUS_BLOCK_HEIGHT_ROUTE,
            body
        );
    }

    /// `submit` and the bulk `catchup` routes take bodies over axum's 2 MiB `Bytes` default, and
    /// the chain's `max_block_size` is what decides whether a transaction is too big, so the body
    /// has to reach the handler. Drives the real `serve_router`.
    #[tokio::test]
    async fn served_router_admits_bodies_over_the_axum_default() {
        const LEN: usize = 3 * 1024 * 1024;
        let router = Router::new().route(
            "/v1/submit/submit",
            axum::routing::post(|body: Bytes| async move { body.len().to_string() }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(crate::serve_router(listener, "test", router, None));

        let mut sock = tokio::net::TcpStream::connect(addr).await.unwrap();
        sock.write_all(
            format!(
                "POST /v1/submit/submit HTTP/1.1\r\nHost: localhost\r\nContent-Type: \
                 application/octet-stream\r\nContent-Length: {LEN}\r\n\r\n"
            )
            .as_bytes(),
        )
        .await
        .unwrap();
        // The server may reset the connection mid-write if it rejects the body early; the asserts
        // below report that legibly.
        let _ = sock.write_all(&vec![b'x'; LEN]).await;

        let mut resp = String::new();
        let read = async {
            loop {
                let mut buf = [0u8; 1024];
                // A read error is end-of-input too: the server resets the connection after an
                // early rejection, and whatever arrived before the reset belongs in the asserts.
                let Ok(n) = sock.read(&mut buf).await else {
                    break;
                };
                if n == 0 {
                    break;
                }
                resp.push_str(&String::from_utf8_lossy(&buf[..n]));
                if resp.contains(&LEN.to_string()) || resp.len() > 4096 {
                    break;
                }
            }
        };
        tokio::time::timeout(std::time::Duration::from_secs(30), read)
            .await
            .expect("server never answered; a 413 would produce no matching body");
        assert!(resp.starts_with("HTTP/1.1 200 OK"), "{resp}");
        let body = resp.split_once("\r\n\r\n").map_or("", |(_, body)| body);
        assert!(
            body.contains(&LEN.to_string()),
            "handler saw a truncated body: {resp}"
        );
    }

    /// The control: the same handler without the layer, pinning that the test above can fail.
    #[tokio::test]
    async fn axum_default_body_limit_rejects_the_same_request() {
        let router = Router::new().route(
            "/v1/submit/submit",
            axum::routing::post(|body: Bytes| async move { body.len().to_string() }),
        );
        let req = Request::builder()
            .method("POST")
            .uri("/v1/submit/submit")
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .body(axum::body::Body::from(vec![b'x'; 3 * 1024 * 1024]))
            .unwrap();
        let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn max_connections_limits_in_flight_requests() {
        let router = Router::new().route(
            "/slow",
            get(|| async {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                "ok"
            }),
        );
        let router = crate::apply_connection_limit(router, 2);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        async fn get_slow(addr: std::net::SocketAddr) -> (tokio::net::TcpStream, String) {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut sock = tokio::net::TcpStream::connect(addr).await.unwrap();
            sock.write_all(b"GET /slow HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .await
                .unwrap();
            let mut buf = [0u8; 32];
            let n = sock.read(&mut buf).await.unwrap();
            (sock, String::from_utf8_lossy(&buf[..n]).to_string())
        }

        use tokio::io::AsyncWriteExt;
        let mut s1 = tokio::net::TcpStream::connect(addr).await.unwrap();
        s1.write_all(b"GET /slow HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut s2 = tokio::net::TcpStream::connect(addr).await.unwrap();
        s2.write_all(b"GET /slow HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        // Both requests in flight (each sleeps 2s); the third must be shed.
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let (_s3, status) = get_slow(addr).await;
        assert!(
            status.contains("429"),
            "third request must be limited: {status}"
        );
    }

    /// Regression test: the docs routes must exist in the app a serve mode actually builds, not
    /// only in `create_router_v1` (which the serve modes don't call). Assembles a router the way
    /// `serve_axum_status` does, wrapped in the same top-level routes and legacy-URI rewrite
    /// layers as `serve_router`, and checks the docs are reachable and the spec reflects only
    /// the mounted modules.
    #[tokio::test]
    async fn serve_mode_assembly_serves_v1_docs() {
        let api_router = router_status(MockState).merge(router_state_signature(MockState));
        let router = with_top_level_routes(finish_v1_docs(api_router));
        let app = tower::Layer::layer(
            &tower::util::MapRequestLayer::new(rewrite_legacy_uri),
            router,
        );

        let get = |uri: &'static str| {
            let app = app.clone();
            async move {
                let req = Request::builder()
                    .uri(uri)
                    .body(axum::body::Body::empty())
                    .unwrap();
                tower::ServiceExt::oneshot(app, req).await.unwrap()
            }
        };

        let resp = get("/").await;
        assert_eq!(resp.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(
            resp.headers().get(axum::http::header::LOCATION).unwrap(),
            "/v1"
        );

        let resp = get("/v1").await;
        assert_eq!(resp.status(), StatusCode::OK, "/v1 must serve the docs UI");

        let resp = get(routes::v1::OPENAPI_SPEC_ROUTE).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let spec: serde_json::Value =
            serde_json::from_str(&body_string(resp).await).expect("valid JSON");
        let paths = spec["paths"].as_object().expect("spec has paths");
        assert!(paths.contains_key(routes::v1::STATUS_BLOCK_HEIGHT_ROUTE));
        assert!(
            !paths.contains_key(routes::v1::LEAF_BY_HEIGHT_ROUTE),
            "spec must only document the modules this mode mounts"
        );

        // Every `{name}` template segment must be declared as a path parameter, or Swagger's
        // try-it-out cannot fill the URL.
        let params = &paths[routes::v1::STATE_SIGNATURE_BLOCK_ROUTE]["get"]["parameters"];
        assert_eq!(
            params[0]["name"], "height",
            "template parameters must be declared: {params}"
        );
        assert_eq!(params[0]["in"], "path");
        assert_eq!(params[0]["required"], true);
        assert_eq!(params[0]["schema"]["type"], "integer");
    }

    /// Multi-segment templates declare one parameter per `{name}`, in template order.
    #[tokio::test]
    async fn v1_spec_declares_all_template_parameters() {
        let router = create_router_v1(MockState);
        let req = Request::builder()
            .uri(routes::v1::OPENAPI_SPEC_ROUTE)
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
        let spec: serde_json::Value =
            serde_json::from_str(&body_string(resp).await).expect("valid JSON");
        let paths = spec["paths"].as_object().expect("spec has paths");
        for (path, item) in paths {
            let names: Vec<&str> = path
                .split('/')
                .filter_map(|s| s.strip_prefix('{').and_then(|s| s.strip_suffix('}')))
                .collect();
            for op in item.as_object().unwrap().values() {
                let declared: Vec<&str> = op["parameters"]
                    .as_array()
                    .map(|ps| {
                        ps.iter()
                            .filter(|p| p["in"] == "path")
                            .map(|p| p["name"].as_str().unwrap())
                            .collect()
                    })
                    .unwrap_or_default();
                assert_eq!(
                    declared, names,
                    "path {path} must declare its template params"
                );
            }
        }

        // Numeric segments are typed integer, hash/key-like segments string.
        let key_path = &paths[routes::v1::REWARD_STATE_PATH_BY_HEIGHT_ROUTE]["get"]["parameters"];
        assert_eq!(key_path[0]["name"], "height");
        assert_eq!(key_path[0]["schema"]["type"], "integer");
        assert_eq!(key_path[1]["name"], "key");
        assert_eq!(key_path[1]["schema"]["type"], "string");

        // Operations are grouped by module tag.
        assert_eq!(
            paths[routes::v1::LEAF_BY_HEIGHT_ROUTE]["get"]["tags"][0],
            "availability"
        );
        assert!(
            spec["tags"]
                .as_array()
                .expect("spec has tags")
                .iter()
                .any(|t| t["name"] == "status")
        );
    }
=======
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use futures::stream::BoxStream;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use vbs::{BinarySerializer, Serializer};

    use super::*;

    fn rewritten_uri(uri: &str) -> String {
        let req = Request::builder()
            .uri(uri)
            .body(axum::body::Body::empty())
            .unwrap();
        rewrite_legacy_uri(req).uri().to_string()
    }

    #[test]
    fn rewrite_legacy_uri_prefixes_unversioned_paths() {
        assert_eq!(
            rewritten_uri("/status/block-height"),
            "/v1/status/block-height"
        );
    }

    #[test]
    fn rewrite_legacy_uri_rewrites_v0_to_v1() {
        assert_eq!(
            rewritten_uri("/v0/status/block-height"),
            "/v1/status/block-height"
        );
        assert_eq!(rewritten_uri("/v0"), "/v1");
    }

    #[test]
    fn rewrite_legacy_uri_rewrites_v0_availability_paths() {
        assert_eq!(
            rewritten_uri("/v0/availability/block/1/namespace/2"),
            "/v1/availability/block/1/namespace/2"
        );
        assert_eq!(
            rewritten_uri("/v0/availability/leaf/1"),
            "/v1/availability/leaf/1"
        );
        assert_eq!(
            rewritten_uri("/v0/availability/vid/common/1"),
            "/v1/availability/vid/common/1"
        );
        assert_eq!(
            rewritten_uri("/v0/availability/stream/leaves/0"),
            "/v1/availability/stream/leaves/0"
        );
        assert_eq!(
            rewritten_uri("/availability/block/1/namespace/2"),
            "/v1/availability/block/1/namespace/2"
        );
        assert_eq!(
            rewritten_uri("/availability/leaf/1"),
            "/v1/availability/leaf/1"
        );
    }

    #[test]
    fn rewrite_legacy_uri_leaves_v1_unchanged() {
        assert_eq!(
            rewritten_uri("/v1/node/block-height"),
            "/v1/node/block-height"
        );
    }

    #[test]
    fn rewrite_legacy_uri_leaves_v2_unchanged() {
        assert_eq!(
            rewritten_uri("/v2/rewards/balance/0xabc"),
            "/v2/rewards/balance/0xabc"
        );
    }

    #[test]
    fn rewrite_legacy_uri_respects_version_prefix_boundaries() {
        assert_eq!(rewritten_uri("/v1"), "/v1");
        assert_eq!(rewritten_uri("/v2"), "/v2");
        assert_eq!(rewritten_uri("/v1x"), "/v1/v1x");
        assert_eq!(rewritten_uri("/v2-foo/bar"), "/v1/v2-foo/bar");
        assert_eq!(rewritten_uri("/v0x/leaf"), "/v1/v0x/leaf");
    }

    #[test]
    fn rewrite_legacy_uri_leaves_reserved_paths_unchanged() {
        assert_eq!(rewritten_uri("/"), "/");
        assert_eq!(rewritten_uri("/healthcheck"), "/healthcheck");
        assert_eq!(rewritten_uri("/version"), "/version");
    }

    #[test]
    fn rewrite_legacy_uri_preserves_query_string() {
        assert_eq!(
            rewritten_uri("/availability/leaf/1?foo=bar"),
            "/v1/availability/leaf/1?foo=bar"
        );
    }

    /// Error bodies go to unauthenticated callers, so an L1 provider URL in the message must not
    /// reach them.
    #[test]
    fn error_response_redacts_provider_credentials() {
        let msg = r#"failed to get total supply. err=reqwest::Error { url: "https://u:p@rpc.invalid/v1/FAKEKEY" }"#;

        let body = ErrorResponse::new(StatusCode::NOT_FOUND, msg.to_string());

        assert!(!body.custom.message.contains("FAKEKEY"), "{body:?}");
        assert!(!body.custom.message.contains("u:p"), "{body:?}");
        assert!(body.custom.message.contains("rpc.invalid"), "{body:?}");
    }

    /// Implements every v1 API trait with `unimplemented!()` bodies, purely so `create_router_v1`
    /// can be instantiated in tests that only exercise the static docs routes (root redirect,
    /// swagger UI, OpenAPI spec) and never call into a handler.
    #[derive(Clone)]
    struct MockState;

    #[async_trait::async_trait]
    impl v1::RewardApi for MockState {
        type RewardClaimInput = ();
        type RewardBalance = ();
        type RewardAccountQueryData = ();
        type RewardAmounts = ();
        type RewardMerkleTreeData = ();
        type RewardAccountQueryDataV1 = ();
        type RewardStatePathV1 = ();
        type RewardStatePathV2 = ();

        async fn get_reward_state_height(&self) -> anyhow::Result<u64> {
            unimplemented!()
        }
        async fn get_reward_state_v2_height(&self) -> anyhow::Result<u64> {
            unimplemented!()
        }
        async fn get_reward_account_proof_v1(
            &self,
            _height: u64,
            _address: String,
        ) -> anyhow::Result<Self::RewardAccountQueryDataV1> {
            unimplemented!()
        }
        async fn get_reward_claim_input(
            &self,
            _block_height: u64,
            _address: String,
        ) -> anyhow::Result<Self::RewardClaimInput> {
            unimplemented!()
        }
        async fn get_reward_balance(
            &self,
            _height: u64,
            _address: String,
        ) -> anyhow::Result<Self::RewardBalance> {
            unimplemented!()
        }
        async fn get_latest_reward_balance(
            &self,
            _address: String,
        ) -> anyhow::Result<Self::RewardBalance> {
            unimplemented!()
        }
        async fn get_reward_account_proof(
            &self,
            _height: u64,
            _address: String,
        ) -> anyhow::Result<Self::RewardAccountQueryData> {
            unimplemented!()
        }
        async fn get_latest_reward_account_proof(
            &self,
            _address: String,
        ) -> anyhow::Result<Self::RewardAccountQueryData> {
            unimplemented!()
        }
        async fn get_reward_amounts(
            &self,
            _height: u64,
            _offset: u64,
            _limit: u64,
        ) -> anyhow::Result<Self::RewardAmounts> {
            unimplemented!()
        }
        async fn get_reward_merkle_tree_v2(
            &self,
            _height: u64,
        ) -> anyhow::Result<Self::RewardMerkleTreeData> {
            unimplemented!()
        }
        async fn get_reward_state_path_v1(
            &self,
            _snapshot: v1::merklized_state::Snapshot,
            _key: String,
        ) -> anyhow::Result<Self::RewardStatePathV1> {
            unimplemented!()
        }
        async fn get_reward_state_path_v2(
            &self,
            _snapshot: v1::merklized_state::Snapshot,
            _key: String,
        ) -> anyhow::Result<Self::RewardStatePathV2> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::AvailabilityApi for MockState {
        type NamespaceProofQueryData = ();
        type IncorrectEncodingProof = ();
        type StateCertQueryDataV1 = ();
        type StateCertQueryDataV2 = ();

        async fn get_namespace_proof(
            &self,
            _block_id: v1::BlockId,
            _namespace: u32,
        ) -> anyhow::Result<Self::NamespaceProofQueryData> {
            unimplemented!()
        }
        async fn get_namespace_proof_range(
            &self,
            _from: u64,
            _until: u64,
            _namespace: u32,
        ) -> anyhow::Result<Vec<Self::NamespaceProofQueryData>> {
            unimplemented!()
        }
        async fn stream_namespace_proofs(
            &self,
            _from: usize,
            _namespace: u32,
        ) -> anyhow::Result<BoxStream<'static, Self::NamespaceProofQueryData>> {
            unimplemented!()
        }
        async fn get_incorrect_encoding_proof(
            &self,
            _block_id: v1::BlockId,
            _namespace: u32,
        ) -> anyhow::Result<Self::IncorrectEncodingProof> {
            unimplemented!()
        }
        async fn get_state_cert(&self, _epoch: u64) -> anyhow::Result<Self::StateCertQueryDataV1> {
            unimplemented!()
        }
        async fn get_state_cert_v2(
            &self,
            _epoch: u64,
        ) -> anyhow::Result<Self::StateCertQueryDataV2> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::HotShotAvailabilityApi for MockState {
        type Leaf = ();
        type Block = ();
        type Header = ();
        type Payload = ();
        type VidCommon = ();
        type Transaction = ();
        type TransactionWithProof = ();
        type BlockSummary = ();
        type Limits = ();
        type Cert2 = ();

        async fn get_leaf(&self, _id: v1::LeafId) -> anyhow::Result<Self::Leaf> {
            unimplemented!()
        }
        async fn get_leaf_range(
            &self,
            _from: usize,
            _until: usize,
        ) -> anyhow::Result<Vec<Self::Leaf>> {
            unimplemented!()
        }
        async fn get_header(&self, _id: v1::BlockId) -> anyhow::Result<Self::Header> {
            unimplemented!()
        }
        async fn get_header_range(
            &self,
            _from: usize,
            _until: usize,
        ) -> anyhow::Result<Vec<Self::Header>> {
            unimplemented!()
        }
        async fn get_block(&self, _id: v1::BlockId) -> anyhow::Result<Self::Block> {
            unimplemented!()
        }
        async fn get_block_range(
            &self,
            _from: usize,
            _until: usize,
        ) -> anyhow::Result<Vec<Self::Block>> {
            unimplemented!()
        }
        async fn get_payload(&self, _id: v1::PayloadId) -> anyhow::Result<Self::Payload> {
            unimplemented!()
        }
        async fn get_payload_range(
            &self,
            _from: usize,
            _until: usize,
        ) -> anyhow::Result<Vec<Self::Payload>> {
            unimplemented!()
        }
        async fn get_vid_common(&self, _id: v1::BlockId) -> anyhow::Result<Self::VidCommon> {
            unimplemented!()
        }
        async fn get_vid_common_range(
            &self,
            _from: usize,
            _until: usize,
        ) -> anyhow::Result<Vec<Self::VidCommon>> {
            unimplemented!()
        }
        async fn get_transaction_by_position(
            &self,
            _height: u64,
            _index: u64,
        ) -> anyhow::Result<Self::Transaction> {
            unimplemented!()
        }
        async fn get_transaction_by_hash(
            &self,
            _hash: String,
        ) -> anyhow::Result<Self::Transaction> {
            unimplemented!()
        }
        async fn get_transaction_proof_by_position(
            &self,
            _height: u64,
            _index: u64,
        ) -> anyhow::Result<Self::TransactionWithProof> {
            unimplemented!()
        }
        async fn get_transaction_proof_by_hash(
            &self,
            _hash: String,
        ) -> anyhow::Result<Self::TransactionWithProof> {
            unimplemented!()
        }
        async fn get_block_summary(&self, _height: usize) -> anyhow::Result<Self::BlockSummary> {
            unimplemented!()
        }
        async fn get_block_summary_range(
            &self,
            _from: usize,
            _until: usize,
        ) -> anyhow::Result<Vec<Self::BlockSummary>> {
            unimplemented!()
        }
        async fn get_limits(&self) -> anyhow::Result<Self::Limits> {
            unimplemented!()
        }
        async fn get_cert2(&self, _height: u64) -> anyhow::Result<Option<Self::Cert2>> {
            unimplemented!()
        }
        async fn stream_leaves(
            &self,
            _from: usize,
        ) -> anyhow::Result<BoxStream<'static, Self::Leaf>> {
            unimplemented!()
        }
        async fn stream_headers(
            &self,
            _from: usize,
        ) -> anyhow::Result<BoxStream<'static, Self::Header>> {
            unimplemented!()
        }
        async fn stream_blocks(
            &self,
            _from: usize,
        ) -> anyhow::Result<BoxStream<'static, Self::Block>> {
            unimplemented!()
        }
        async fn stream_payloads(
            &self,
            _from: usize,
        ) -> anyhow::Result<BoxStream<'static, Self::Payload>> {
            unimplemented!()
        }
        async fn stream_vid_common(
            &self,
            _from: usize,
        ) -> anyhow::Result<BoxStream<'static, Self::VidCommon>> {
            unimplemented!()
        }
        async fn stream_transactions(
            &self,
            _from: usize,
            _namespace: Option<u32>,
        ) -> anyhow::Result<BoxStream<'static, Self::Transaction>> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::BlockStateApi for MockState {
        type MerkleProof = ();

        async fn get_block_state_path(
            &self,
            _snapshot: v1::merklized_state::Snapshot,
            _key: String,
        ) -> anyhow::Result<Self::MerkleProof> {
            unimplemented!()
        }
        async fn get_block_state_height(&self) -> anyhow::Result<u64> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::FeeStateApi for MockState {
        type MerkleProof = ();
        type FeeAmount = ();

        async fn get_fee_state_path(
            &self,
            _snapshot: v1::merklized_state::Snapshot,
            _key: String,
        ) -> anyhow::Result<Self::MerkleProof> {
            unimplemented!()
        }
        async fn get_fee_state_height(&self) -> anyhow::Result<u64> {
            unimplemented!()
        }
        async fn get_fee_balance_latest(
            &self,
            _address: String,
        ) -> anyhow::Result<Option<Self::FeeAmount>> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::StatusApi for MockState {
        type Keys = ();

        async fn block_height(&self) -> anyhow::Result<u64> {
            unimplemented!()
        }
        async fn success_rate(&self) -> anyhow::Result<f64> {
            unimplemented!()
        }
        async fn time_since_last_decide(&self) -> anyhow::Result<u64> {
            unimplemented!()
        }
        async fn metrics(&self) -> anyhow::Result<String> {
            unimplemented!()
        }
        async fn keys(&self) -> anyhow::Result<Self::Keys> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::ConfigApi for MockState {
        type HotShotConfig = ();
        type RuntimeConfig = ();

        async fn hotshot_config(&self) -> anyhow::Result<Self::HotShotConfig> {
            unimplemented!()
        }
        async fn env(&self) -> anyhow::Result<Vec<String>> {
            unimplemented!()
        }
        async fn runtime_config(&self) -> anyhow::Result<Self::RuntimeConfig> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::NodeApi for MockState {
        type VidShare = ();
        type SyncStatus = ();
        type HeaderWindow = ();
        type Limits = ();
        type StakeTable = ();
        type StakeTableCurrent = ();
        type Validators = ();
        type AllValidators = ();
        type Participation = ();
        type BlockReward = ();
        type Block = ();
        type Leaf = ();

        async fn block_height(&self) -> anyhow::Result<u64> {
            unimplemented!()
        }
        async fn count_transactions(
            &self,
            _from: Option<u64>,
            _to: Option<u64>,
            _namespace: Option<u64>,
        ) -> anyhow::Result<u64> {
            unimplemented!()
        }
        async fn payload_size(
            &self,
            _from: Option<u64>,
            _to: Option<u64>,
            _namespace: Option<u64>,
        ) -> anyhow::Result<u64> {
            unimplemented!()
        }
        async fn get_vid_share(&self, _id: v1::VidShareId) -> anyhow::Result<Self::VidShare> {
            unimplemented!()
        }
        async fn sync_status(&self) -> anyhow::Result<Self::SyncStatus> {
            unimplemented!()
        }
        async fn get_header_window(
            &self,
            _start: v1::HeaderWindowStart,
            _end: u64,
        ) -> anyhow::Result<Self::HeaderWindow> {
            unimplemented!()
        }
        async fn limits(&self) -> anyhow::Result<Self::Limits> {
            unimplemented!()
        }
        async fn stake_table(&self, _epoch: u64) -> anyhow::Result<Self::StakeTable> {
            unimplemented!()
        }
        async fn stake_table_current(&self) -> anyhow::Result<Self::StakeTableCurrent> {
            unimplemented!()
        }
        async fn da_stake_table(&self, _epoch: u64) -> anyhow::Result<Self::StakeTable> {
            unimplemented!()
        }
        async fn da_stake_table_current(&self) -> anyhow::Result<Self::StakeTableCurrent> {
            unimplemented!()
        }
        async fn get_validators(&self, _epoch: u64) -> anyhow::Result<Self::Validators> {
            unimplemented!()
        }
        async fn get_all_validators(
            &self,
            _epoch: u64,
            _offset: u64,
            _limit: u64,
        ) -> anyhow::Result<Self::AllValidators> {
            unimplemented!()
        }
        async fn current_proposal_participation(&self) -> anyhow::Result<Self::Participation> {
            unimplemented!()
        }
        async fn proposal_participation(&self, _epoch: u64) -> anyhow::Result<Self::Participation> {
            unimplemented!()
        }
        async fn current_vote_participation(&self) -> anyhow::Result<Self::Participation> {
            unimplemented!()
        }
        async fn vote_participation(&self, _epoch: u64) -> anyhow::Result<Self::Participation> {
            unimplemented!()
        }
        async fn get_block_reward(&self, _epoch: Option<u64>) -> anyhow::Result<Self::BlockReward> {
            unimplemented!()
        }
        async fn get_oldest_block(&self) -> anyhow::Result<Option<Self::Block>> {
            unimplemented!()
        }
        async fn get_oldest_leaf(&self) -> anyhow::Result<Option<Self::Leaf>> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::CatchupApi for MockState {
        type FeeAccount = ();
        type RewardAccountV1 = ();
        type RewardAccountV2 = ();
        type AccountQueryData = ();
        type FeeMerkleTree = ();
        type BlocksFrontier = ();
        type ChainConfig = ();
        type LeafChain = ();
        type Cert2 = ();
        type RewardAccountQueryDataV1 = ();
        type RewardMerkleTreeV1 = ();
        type RewardAccountQueryDataV2 = ();
        type RewardMerkleTreeV2Data = ();
        type StateCert = ();

        async fn get_account(
            &self,
            _height: u64,
            _view: u64,
            _address: String,
        ) -> anyhow::Result<Self::AccountQueryData> {
            unimplemented!()
        }
        async fn get_accounts(
            &self,
            _height: u64,
            _view: u64,
            _accounts: Vec<Self::FeeAccount>,
        ) -> anyhow::Result<Self::FeeMerkleTree> {
            unimplemented!()
        }
        async fn get_blocks_frontier(
            &self,
            _height: u64,
            _view: u64,
        ) -> anyhow::Result<Self::BlocksFrontier> {
            unimplemented!()
        }
        async fn get_chain_config(&self, _commitment: String) -> anyhow::Result<Self::ChainConfig> {
            unimplemented!()
        }
        async fn get_leaf_chain(&self, _height: u64) -> anyhow::Result<Self::LeafChain> {
            unimplemented!()
        }
        async fn get_cert2(&self, _height: u64) -> anyhow::Result<Self::Cert2> {
            unimplemented!()
        }
        async fn get_reward_account_v1(
            &self,
            _height: u64,
            _view: u64,
            _address: String,
        ) -> anyhow::Result<Self::RewardAccountQueryDataV1> {
            unimplemented!()
        }
        async fn get_reward_accounts_v1(
            &self,
            _height: u64,
            _view: u64,
            _accounts: Vec<Self::RewardAccountV1>,
        ) -> anyhow::Result<Self::RewardMerkleTreeV1> {
            unimplemented!()
        }
        async fn get_reward_account_v2(
            &self,
            _height: u64,
            _view: u64,
            _address: String,
        ) -> anyhow::Result<Self::RewardAccountQueryDataV2> {
            unimplemented!()
        }
        async fn get_reward_merkle_tree_v2(
            &self,
            _height: u64,
            _view: u64,
        ) -> anyhow::Result<Self::RewardMerkleTreeV2Data> {
            unimplemented!()
        }
        async fn get_state_cert(&self, _epoch: u64) -> anyhow::Result<Self::StateCert> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::SubmitApi for MockState {
        type Transaction = ();
        type TxHash = ();

        async fn submit(&self, _tx: Self::Transaction) -> anyhow::Result<Self::TxHash> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::StateSignatureApi for MockState {
        type Signature = ();

        async fn get_state_signature(&self, _height: u64) -> anyhow::Result<Self::Signature> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::HotShotEventsApi for MockState {
        type Event = ();
        type StartupInfo = ();

        async fn startup_info(&self) -> anyhow::Result<Self::StartupInfo> {
            unimplemented!()
        }
        async fn events(&self) -> anyhow::Result<BoxStream<'static, Self::Event>> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::LightClientApi for MockState {
        type LeafProof = ();
        type HeaderProof = ();
        type StakeTableEvents = ();
        type PayloadProof = ();
        type NamespaceProof = ();

        async fn get_leaf_proof(
            &self,
            _query: v1::LeafQuery,
            _finalized: Option<u64>,
        ) -> anyhow::Result<Self::LeafProof> {
            unimplemented!()
        }
        async fn get_header_proof(
            &self,
            _root: u64,
            _requested: v1::HeaderQuery,
        ) -> anyhow::Result<Self::HeaderProof> {
            unimplemented!()
        }
        async fn get_light_client_stake_table(
            &self,
            _epoch: u64,
        ) -> anyhow::Result<Self::StakeTableEvents> {
            unimplemented!()
        }
        async fn get_payload_proof(&self, _height: u64) -> anyhow::Result<Self::PayloadProof> {
            unimplemented!()
        }
        async fn get_payload_proof_range(
            &self,
            _start: u64,
            _end: u64,
        ) -> anyhow::Result<Vec<Self::PayloadProof>> {
            unimplemented!()
        }
        async fn get_lc_namespace_proof(
            &self,
            _height: u64,
            _namespace: u64,
        ) -> anyhow::Result<Self::NamespaceProof> {
            unimplemented!()
        }
        async fn get_lc_namespace_proof_range(
            &self,
            _start: u64,
            _end: u64,
            _namespace: u64,
        ) -> anyhow::Result<Vec<Self::NamespaceProof>> {
            unimplemented!()
        }
        async fn get_lc_namespaces_proof_range(
            &self,
            _start: u64,
            _end: u64,
            _namespaces: String,
        ) -> anyhow::Result<Vec<std::collections::HashMap<u64, Self::NamespaceProof>>> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::ExplorerApi for MockState {
        type BlockDetail = ();
        type BlockSummaries = ();
        type TransactionDetail = ();
        type TransactionSummaries = ();
        type ExplorerSummary = ();
        type SearchResult = ();

        async fn get_block_detail(
            &self,
            _ident: v1::BlockIdent,
        ) -> anyhow::Result<Self::BlockDetail> {
            unimplemented!()
        }
        async fn get_block_summaries(
            &self,
            _target: v1::BlockIdent,
            _limit: u64,
        ) -> anyhow::Result<Self::BlockSummaries> {
            unimplemented!()
        }
        async fn get_transaction_detail(
            &self,
            _ident: v1::TxIdent,
        ) -> anyhow::Result<Self::TransactionDetail> {
            unimplemented!()
        }
        async fn get_transaction_summaries(
            &self,
            _target: v1::TxIdent,
            _limit: u64,
            _filter: v1::TxSummaryFilter,
        ) -> anyhow::Result<Self::TransactionSummaries> {
            unimplemented!()
        }
        async fn get_explorer_summary(&self) -> anyhow::Result<Self::ExplorerSummary> {
            unimplemented!()
        }
        async fn get_search_result(&self, _query: String) -> anyhow::Result<Self::SearchResult> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::TokenApi for MockState {
        async fn total_minted_supply(&self) -> anyhow::Result<String> {
            unimplemented!()
        }
        async fn circulating_supply(&self) -> anyhow::Result<String> {
            unimplemented!()
        }
        async fn circulating_supply_ethereum(&self) -> anyhow::Result<String> {
            unimplemented!()
        }
        async fn total_issued_supply(&self) -> anyhow::Result<String> {
            unimplemented!()
        }
        async fn total_reward_distributed(&self) -> anyhow::Result<String> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::DatabaseApi for MockState {
        type TableSizes = ();
        type MigrationStatus = ();

        async fn get_table_sizes(&self) -> anyhow::Result<Self::TableSizes> {
            unimplemented!()
        }
        async fn get_migration_status(&self) -> anyhow::Result<Self::MigrationStatus> {
            unimplemented!()
        }
    }

    async fn body_string(resp: Response) -> String {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("read response body");
        String::from_utf8(bytes.to_vec()).expect("response body is utf8")
    }

    /// Checks that every response carries
    /// `Access-Control-Allow-Origin: *`: top-level routes, API routes merged in by the caller,
    /// error responses, and 404s. Also checks that an OPTIONS preflight is answered with the
    /// allow-origin, allow-methods, and allow-headers a browser requires.
    #[tokio::test]
    async fn responses_carry_cors_headers() {
        let router = with_top_level_routes(
            Router::new()
                .route("/v1/status/block-height", get(|| async { "0" }))
                .route(
                    "/v1/failing",
                    get(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
                ),
        )
        .layer(cors_layer());

        let allow_origin = |resp: &Response, uri: &str| {
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap_or_else(|| panic!("no CORS header on {uri}"))
                .clone()
        };

        for (uri, expected_status) in [
            ("/healthcheck", StatusCode::OK),
            ("/v1/status/block-height", StatusCode::OK),
            ("/v1/failing", StatusCode::INTERNAL_SERVER_ERROR),
            ("/no/such/route", StatusCode::NOT_FOUND),
        ] {
            let req = Request::builder()
                .uri(uri)
                .header(header::ORIGIN, "https://example.com")
                .body(axum::body::Body::empty())
                .unwrap();
            let resp = tower::ServiceExt::oneshot(router.clone(), req)
                .await
                .unwrap();
            assert_eq!(resp.status(), expected_status, "{uri}");
            assert_eq!(allow_origin(&resp, uri), "*", "{uri}");
        }

        // Browsers preflight non-simple requests (e.g. a JSON POST to submit) with OPTIONS and
        // require allow-origin, allow-methods, and allow-headers in the answer, even on routes
        // that only register GET handlers.
        let preflight = Request::builder()
            .method(axum::http::Method::OPTIONS)
            .uri("/v1/status/block-height")
            .header(header::ORIGIN, "https://example.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = tower::ServiceExt::oneshot(router, preflight).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(allow_origin(&resp, "preflight"), "*");
        assert_eq!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_METHODS)
                .expect("allow-methods on preflight"),
            "*"
        );
        assert_eq!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_HEADERS)
                .expect("allow-headers on preflight"),
            "*"
        );
    }

    /// Serves with zero connection slots so every request is shed, and checks the 429 still
    /// carries `Access-Control-Allow-Origin: *`.
    #[tokio::test]
    async fn shed_requests_carry_cors_headers() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(crate::serve_router(
            listener,
            "test",
            Router::new().route("/v1/status/block-height", get(|| async { "0" })),
            Some(0),
        ));

        let mut sock = tokio::net::TcpStream::connect(addr).await.unwrap();
        sock.write_all(
            b"GET /v1/status/block-height HTTP/1.1\r\nHost: localhost\r\nOrigin: https://example.com\r\n\r\n",
        )
        .await
        .unwrap();
        let mut head = Vec::new();
        let mut buf = [0u8; 512];
        while !head.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = sock.read(&mut buf).await.unwrap();
            assert!(
                n > 0,
                "connection closed before the response head: {head:?}"
            );
            head.extend_from_slice(&buf[..n]);
        }
        let head = String::from_utf8_lossy(&head).to_ascii_lowercase();
        assert!(head.contains("429"), "request must be shed: {head}");
        assert!(
            head.contains("access-control-allow-origin: *"),
            "no CORS header on 429: {head}"
        );
    }

    #[tokio::test]
    async fn root_redirects_to_v1() {
        let router = with_top_level_routes(Router::new());
        let req = Request::builder()
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(
            resp.headers().get(axum::http::header::LOCATION).unwrap(),
            "/v1"
        );
    }

    /// The app-level `/healthcheck` reports `tide_disco::app::AppHealth`, which is what every
    /// non-singleton tide app served; a module-level one reports the bare `HealthStatus`. Both
    /// shapes are load-bearing for clients built against the tide-disco servers.
    #[tokio::test]
    async fn healthcheck_shapes_match_tide() {
        let router = with_top_level_routes(Router::new());

        async fn get(router: &Router, uri: &str, accept: &str) -> Vec<u8> {
            let req = Request::builder()
                .uri(uri)
                .header(header::ACCEPT, accept)
                .body(axum::body::Body::empty())
                .unwrap();
            let resp = tower::ServiceExt::oneshot(router.clone(), req)
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK, "{uri}");
            axum::body::to_bytes(resp.into_body(), usize::MAX)
                .await
                .unwrap()
                .to_vec()
        }

        assert_eq!(
            get(&router, "/healthcheck", "application/json").await,
            br#"{"status":"available","modules":{}}"#
        );
        assert_eq!(
            get(&router, "/v1/status/healthcheck", "application/json").await,
            br#""available""#
        );

        // vbs field order (status ordinal, then modules map) must not change either: surf-disco
        // clients default to `Accept: application/octet-stream`.
        #[derive(Debug, PartialEq, serde::Deserialize)]
        enum TideHealthStatus {
            Available,
        }
        #[derive(Debug, PartialEq, serde::Deserialize)]
        struct TideAppHealth {
            status: TideHealthStatus,
            modules: BTreeMap<String, BTreeMap<u64, u16>>,
        }
        let binary = get(&router, "/healthcheck", "application/octet-stream").await;
        // `BuilderClient::connect` and the events-service wrapper poll this route and decode the
        // body as a bare `HealthStatus`. That only works because bincode allows trailing bytes and
        // `AppHealth`'s first field is the status ordinal, so the bare enum is a prefix of the
        // object. Reordering `AppHealth`'s fields would break both clients silently.
        assert_eq!(
            Serializer::<WireVersion>::deserialize::<TideHealthStatus>(&binary).unwrap(),
            TideHealthStatus::Available
        );
        assert_eq!(
            Serializer::<WireVersion>::deserialize::<TideAppHealth>(&binary).unwrap(),
            TideAppHealth {
                status: TideHealthStatus::Available,
                modules: BTreeMap::new(),
            }
        );
    }

    #[tokio::test]
    async fn v1_swagger_ui_serves_html() {
        let router = create_router_v1(MockState);
        let req = Request::builder()
            .uri("/v1")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let content_type = resp
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(content_type.contains("text/html"));
        let body = body_string(resp).await;
        assert!(body.contains(routes::v1::OPENAPI_SPEC_ROUTE));
    }

    #[tokio::test]
    async fn v1_openapi_spec_contains_known_route() {
        let router = create_router_v1(MockState);
        let req = Request::builder()
            .uri(routes::v1::OPENAPI_SPEC_ROUTE)
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        let spec: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
        assert!(
            spec["paths"]
                .as_object()
                .expect("spec has paths")
                .contains_key(routes::v1::STATUS_BLOCK_HEIGHT_ROUTE),
            "expected {} in spec paths: {}",
            routes::v1::STATUS_BLOCK_HEIGHT_ROUTE,
            body
        );
    }

    /// `submit` and the bulk `catchup` routes take bodies over axum's 2 MiB `Bytes` default, and
    /// the chain's `max_block_size` is what decides whether a transaction is too big, so the body
    /// has to reach the handler. Drives the real `serve_router`.
    #[tokio::test]
    async fn served_router_admits_bodies_over_the_axum_default() {
        const LEN: usize = 3 * 1024 * 1024;
        let router = Router::new().route(
            "/v1/submit/submit",
            axum::routing::post(|body: Bytes| async move { body.len().to_string() }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(crate::serve_router(listener, "test", router, None));

        let mut sock = tokio::net::TcpStream::connect(addr).await.unwrap();
        sock.write_all(
            format!(
                "POST /v1/submit/submit HTTP/1.1\r\nHost: localhost\r\nContent-Type: \
                 application/octet-stream\r\nContent-Length: {LEN}\r\n\r\n"
            )
            .as_bytes(),
        )
        .await
        .unwrap();
        // The server may reset the connection mid-write if it rejects the body early; the asserts
        // below report that legibly.
        let _ = sock.write_all(&vec![b'x'; LEN]).await;

        let mut resp = String::new();
        let read = async {
            loop {
                let mut buf = [0u8; 1024];
                // A read error is end-of-input too: the server resets the connection after an
                // early rejection, and whatever arrived before the reset belongs in the asserts.
                let Ok(n) = sock.read(&mut buf).await else {
                    break;
                };
                if n == 0 {
                    break;
                }
                resp.push_str(&String::from_utf8_lossy(&buf[..n]));
                if resp.contains(&LEN.to_string()) || resp.len() > 4096 {
                    break;
                }
            }
        };
        tokio::time::timeout(std::time::Duration::from_secs(30), read)
            .await
            .expect("server never answered; a 413 would produce no matching body");
        assert!(resp.starts_with("HTTP/1.1 200 OK"), "{resp}");
        let body = resp.split_once("\r\n\r\n").map_or("", |(_, body)| body);
        assert!(
            body.contains(&LEN.to_string()),
            "handler saw a truncated body: {resp}"
        );
    }

    /// The control: the same handler without the layer, pinning that the test above can fail.
    #[tokio::test]
    async fn axum_default_body_limit_rejects_the_same_request() {
        let router = Router::new().route(
            "/v1/submit/submit",
            axum::routing::post(|body: Bytes| async move { body.len().to_string() }),
        );
        let req = Request::builder()
            .method("POST")
            .uri("/v1/submit/submit")
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .body(axum::body::Body::from(vec![b'x'; 3 * 1024 * 1024]))
            .unwrap();
        let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn max_connections_limits_in_flight_requests() {
        let router = Router::new().route(
            "/slow",
            get(|| async {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                "ok"
            }),
        );
        let router = crate::apply_connection_limit(router, 2);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        async fn get_slow(addr: std::net::SocketAddr) -> (tokio::net::TcpStream, String) {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut sock = tokio::net::TcpStream::connect(addr).await.unwrap();
            sock.write_all(b"GET /slow HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .await
                .unwrap();
            let mut buf = [0u8; 32];
            let n = sock.read(&mut buf).await.unwrap();
            (sock, String::from_utf8_lossy(&buf[..n]).to_string())
        }

        use tokio::io::AsyncWriteExt;
        let mut s1 = tokio::net::TcpStream::connect(addr).await.unwrap();
        s1.write_all(b"GET /slow HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut s2 = tokio::net::TcpStream::connect(addr).await.unwrap();
        s2.write_all(b"GET /slow HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        // Both requests in flight (each sleeps 2s); the third must be shed.
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let (_s3, status) = get_slow(addr).await;
        assert!(
            status.contains("429"),
            "third request must be limited: {status}"
        );
    }

    /// Regression test: the docs routes must exist in the app a serve mode actually builds, not
    /// only in `create_router_v1` (which the serve modes don't call). Assembles a router the way
    /// `serve_axum_status` does, wrapped in the same top-level routes and legacy-URI rewrite
    /// layers as `serve_router`, and checks the docs are reachable and the spec reflects only
    /// the mounted modules.
    #[tokio::test]
    async fn serve_mode_assembly_serves_v1_docs() {
        let api_router = router_status(MockState).merge(router_state_signature(MockState));
        let router = with_top_level_routes(finish_v1_docs(api_router));
        let app = tower::Layer::layer(
            &tower::util::MapRequestLayer::new(rewrite_legacy_uri),
            router,
        );

        let get = |uri: &'static str| {
            let app = app.clone();
            async move {
                let req = Request::builder()
                    .uri(uri)
                    .body(axum::body::Body::empty())
                    .unwrap();
                tower::ServiceExt::oneshot(app, req).await.unwrap()
            }
        };

        let resp = get("/").await;
        assert_eq!(resp.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(
            resp.headers().get(axum::http::header::LOCATION).unwrap(),
            "/v1"
        );

        let resp = get("/v1").await;
        assert_eq!(resp.status(), StatusCode::OK, "/v1 must serve the docs UI");

        let resp = get(routes::v1::OPENAPI_SPEC_ROUTE).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let spec: serde_json::Value =
            serde_json::from_str(&body_string(resp).await).expect("valid JSON");
        let paths = spec["paths"].as_object().expect("spec has paths");
        assert!(paths.contains_key(routes::v1::STATUS_BLOCK_HEIGHT_ROUTE));
        assert!(
            !paths.contains_key(routes::v1::LEAF_BY_HEIGHT_ROUTE),
            "spec must only document the modules this mode mounts"
        );

        // Every `{name}` template segment must be declared as a path parameter, or Swagger's
        // try-it-out cannot fill the URL.
        let params = &paths[routes::v1::STATE_SIGNATURE_BLOCK_ROUTE]["get"]["parameters"];
        assert_eq!(
            params[0]["name"], "height",
            "template parameters must be declared: {params}"
        );
        assert_eq!(params[0]["in"], "path");
        assert_eq!(params[0]["required"], true);
        assert_eq!(params[0]["schema"]["type"], "integer");
    }

    /// Multi-segment templates declare one parameter per `{name}`, in template order.
    #[tokio::test]
    async fn v1_spec_declares_all_template_parameters() {
        let router = create_router_v1(MockState);
        let req = Request::builder()
            .uri(routes::v1::OPENAPI_SPEC_ROUTE)
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
        let spec: serde_json::Value =
            serde_json::from_str(&body_string(resp).await).expect("valid JSON");
        let paths = spec["paths"].as_object().expect("spec has paths");
        for (path, item) in paths {
            let names: Vec<&str> = path
                .split('/')
                .filter_map(|s| s.strip_prefix('{').and_then(|s| s.strip_suffix('}')))
                .collect();
            for op in item.as_object().unwrap().values() {
                let declared: Vec<&str> = op["parameters"]
                    .as_array()
                    .map(|ps| {
                        ps.iter()
                            .filter(|p| p["in"] == "path")
                            .map(|p| p["name"].as_str().unwrap())
                            .collect()
                    })
                    .unwrap_or_default();
                assert_eq!(
                    declared, names,
                    "path {path} must declare its template params"
                );
            }
        }

        // Numeric segments are typed integer, hash/key-like segments string.
        let key_path = &paths[routes::v1::REWARD_STATE_PATH_BY_HEIGHT_ROUTE]["get"]["parameters"];
        assert_eq!(key_path[0]["name"], "height");
        assert_eq!(key_path[0]["schema"]["type"], "integer");
        assert_eq!(key_path[1]["name"], "key");
        assert_eq!(key_path[1]["schema"]["type"], "string");

        // Operations are grouped by module tag.
        assert_eq!(
            paths[routes::v1::LEAF_BY_HEIGHT_ROUTE]["get"]["tags"][0],
            "availability"
        );
        assert!(
            spec["tags"]
                .as_array()
                .expect("spec has tags")
                .iter()
                .any(|t| t["name"] == "status")
        );
    }
>>>>>>> 355c72eab8f (fix(telemetry): stop logging L1 provider credentials (#4783))
}
