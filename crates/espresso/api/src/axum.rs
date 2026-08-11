//! Axum HTTP/JSON API handlers

pub mod routes;

use std::sync::Arc;

use aide::{
    axum::{
        ApiRouter,
        routing::{get_with, post_with},
    },
    openapi::{
        Info, OpenApi, Parameter, ParameterData, ParameterSchemaOrContent, PathStyle, ReferenceOr,
        SchemaObject,
    },
    operation::OperationOutput,
    redoc::Redoc,
    scalar::Scalar,
};
use axum::{
    Extension, Json, Router,
    body::Bytes,
    extract::{Path, Request, State, ws::WebSocketUpgrade},
    http::{HeaderMap, StatusCode, Uri, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use http_wire::{
    ContentType, DecodeFailure, WireError, body_limit_layer, cors_layer, drive_ws_stream,
    encode_ok, healthcheck_response, module_healthcheck_response,
};
use schemars::transform::Transform;
use serde::{Deserialize, Serialize};
use serialization_api::v2::{
    GetIncorrectEncodingProofRequest, GetNamespaceProofRequest, GetRewardAccountProofRequest,
    GetRewardBalanceRequest, GetRewardBalancesRequest, GetRewardClaimInputRequest,
    GetRewardMerkleTreeRequest, GetStakeTableRequest, GetStateCertificateRequest,
};
use tokio::sync::Semaphore;

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
#[derive(Debug, Serialize, Deserialize, thiserror::Error)]
#[error("{custom}")]
struct ErrorResponse {
    #[serde(rename = "Custom")]
    custom: CustomError,
}

#[derive(Debug, Serialize, Deserialize, thiserror::Error)]
#[error("error {status}: {message}")]
struct CustomError {
    // Field order matches `node::Error::Custom { message, status }` declaration so serde_json
    // emits the same key order on the wire.
    message: String,
    status: u16,
}

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

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(ErrorResponse::new(status, self.to_string()))).into_response()
    }
}

impl WireError for ErrorResponse {
    fn status(&self) -> StatusCode {
        StatusCode::from_u16(self.custom.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    fn catch_all(status: StatusCode, message: String) -> Self {
        Self::new(status, message)
    }
}

/// Decode a request body based on its `Content-Type`, matched by media-type essence.
///
/// - `application/octet-stream`: VBS (versioned binary) — what `Request::body_binary`
///   sends, and what production peer-catchup / submit-transactions clients use.
/// - `application/json`: serde_json.
fn decode_body<T: serde::de::DeserializeOwned>(
    headers: &HeaderMap,
    body: &[u8],
) -> Result<T, ApiError> {
    http_wire::decode_body(headers, body).map_err(|err| {
        ApiError::BadRequest(match err {
            DecodeFailure::Binary(err) => anyhow::anyhow!("invalid binary body: {err}"),
            DecodeFailure::Json(err) => anyhow::anyhow!("invalid json body: {err}"),
            DecodeFailure::UnsupportedContentType => {
                match headers
                    .get(header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                {
                    Some(other) => anyhow::anyhow!("unsupported Content-Type: {other}"),
                    None => anyhow::anyhow!("missing Content-Type header"),
                }
            },
        })
    })
}

/// Classify an `anyhow::Error` from an availability handler into the appropriate `ApiError`
/// variant. Errors produced via [`AvailabilityError`] in the state implementation carry semantic
/// meaning; everything else falls back to a 500 Internal Server Error.
pub(crate) fn classify_availability_error(err: anyhow::Error) -> ApiError {
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

/// Successful JSON response for v1 handlers, most of which return domain types (from
/// `espresso-types`, `hotshot-query-service`, etc.) that don't implement `schemars::JsonSchema` —
/// this crate doesn't add OpenAPI derives to domain types. Wire format is identical to
/// `axum::Json<T>`; only the OpenAPI operation gets an untyped 200 response instead of a generated
/// schema.
struct ApiJson<T>(T);

impl<T: Serialize> IntoResponse for ApiJson<T> {
    fn into_response(self) -> Response {
        Json(self.0).into_response()
    }
}

impl<T> OperationOutput for ApiJson<T> {
    type Inner = T;

    fn inferred_responses(
        _ctx: &mut aide::generate::GenContext,
        _operation: &mut aide::openapi::Operation,
    ) -> Vec<(Option<u16>, aide::openapi::Response)> {
        vec![(Some(200), aide::openapi::Response::default())]
    }
}

/// Serve the OpenAPI spec (extracted from Extension)
async fn serve_openapi_spec(Extension(api): Extension<OpenApi>) -> Json<OpenApi> {
    Json(api)
}

/// In-flight request slots for `max_connections`.
#[derive(Clone)]
pub(crate) struct RequestLimit(pub(crate) Arc<Semaphore>);

/// Each request holds a slot while in flight; excess gets 429. A websocket's slot is released
/// at the 101 upgrade: long-lived streams are deliberately unbounded here, since demo workloads
/// (nasty-client holds hundreds of streams by design) dwarf the request budget of 25.
pub(crate) async fn limit_requests(
    Extension(RequestLimit(semaphore)): Extension<RequestLimit>,
    req: Request,
    next: axum::middleware::Next,
) -> Response {
    match semaphore.try_acquire_owned() {
        Ok(_permit) => next.run(req).await,
        Err(_) => StatusCode::TOO_MANY_REQUESTS.into_response(),
    }
}

/// The v2 router's `Extension<OpenApi>` layer only covers routes registered on the v2
/// `ApiRouter`; this newtype lets v1 layer its own `OpenApi` extension without the two `Extension`
/// lookups being ambiguous if the routers are ever merged and inspected by type.
#[derive(Clone)]
struct OpenApiV1(OpenApi);

/// Serve the v1 OpenAPI spec (extracted from Extension)
async fn serve_openapi_spec_v1(Extension(OpenApiV1(api)): Extension<OpenApiV1>) -> Json<OpenApi> {
    Json(api)
}

/// Serve custom Swagger UI with collapsed defaults, pointed at the given OpenAPI spec route.
fn swagger_html(spec_route: &str) -> Html<String> {
    Html(include_str!("../templates/swagger.html").replace("{{OPENAPI_SPEC_ROUTE}}", spec_route))
}

/// v2 is WIP, so `/` points at the v1 docs; 307 so browsers don't cache the redirect.
async fn redirect_to_docs() -> axum::response::Redirect {
    axum::response::Redirect::temporary("/v1")
}

/// Tide-disco served every v1 module at both `/<module>/...` and `/v1/<module>/...`, and legacy
/// clients (surf-disco, the light-client, tests) still address the unversioned and `/v0` forms.
/// Axum only declares the `/v1/...` and `/v2/...` route shapes, and `Router::layer` middleware
/// runs after routing, so it can never redirect a request onto a route it doesn't already match.
/// This function is instead wrapped around the whole router with `tower::util::MapRequestLayer`
/// (see `serve_axum`), which runs before routing, to rewrite the URI so the declared routes match.
///
/// Excludes paths that are intentionally unversioned: `/`, `/healthcheck`, `/version`, and
/// anything already prefixed with `/v1` or `/v2`.
pub(crate) fn rewrite_legacy_uri(mut req: Request) -> Request {
    let uri = req.uri().clone();
    let path = uri.path();

    let is_reserved =
        path == "/" || path == "/healthcheck" || path == "/version" || path.is_empty();
    let is_versioned =
        path == "/v1" || path.starts_with("/v1/") || path == "/v2" || path.starts_with("/v2/");
    let new_path = if is_versioned || is_reserved {
        None
    } else if path == "/v0" {
        Some("/v1".to_string())
    } else if let Some(rest) = path.strip_prefix("/v0/") {
        Some(format!("/v1/{rest}"))
    } else {
        Some(format!("/v1{path}"))
    };

    if let Some(new_path) = new_path {
        let pq = if let Some(q) = uri.query() {
            format!("{new_path}?{q}")
        } else {
            new_path
        };
        if let Ok(new_uri) = Uri::builder().path_and_query(pq).build() {
            *req.uri_mut() = new_uri;
        }
    }

    req
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

/// Create a combined router serving both v1 and v2 APIs.
///
/// `hqs_base` is the `hotshot-query-service` router; see [`create_router_v1`].
pub fn create_combined_router<S>(state: S, hqs_base: ApiRouter) -> Router
where
    S: v1::RewardApi
        + v1::AvailabilityApiExtension
        + v1::FeeStateApiExtension
        + v1::StatusApiExtension
        + v1::ConfigApi
        + v1::NodeApiExtension
        + v1::CatchupApi
        + v1::SubmitApi
        + v1::StateSignatureApi
        + v1::HotShotEventsApi
        + v1::LightClientApi
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
    let router_v1 = create_router_v1(state.clone(), hqs_base);
    let router_v2 = create_router_v2(state);

    with_top_level_routes(router_v2.merge(router_v1))
        .layer(body_limit_layer())
        .layer(cors_layer())
}

/// Add the routes that every mode serves regardless of which API modules are enabled:
/// `/`, `/healthcheck`, `/v1/{module}/healthcheck`, and `/version`. Callers apply CORS.
///
/// `/v1/{module}/healthcheck` is reached by legacy clients via the `/{module}/healthcheck`
/// rewrite. Divergence from tide-disco: it matches any `{module}` string, so unregistered module
/// names report healthy instead of 404. Constraining it to the registered set would have to track
/// which modules each serve mode mounts; not worth it for a liveness probe.
pub(crate) fn with_top_level_routes(router: Router) -> Router {
    router
        .route("/", get(redirect_to_docs))
        .route(
            "/healthcheck",
            get(|headers: HeaderMap| async move { healthcheck_response(&headers) }),
        )
        .route(
            "/v1/{module}/healthcheck",
            get(|headers: HeaderMap| async move { module_healthcheck_response(&headers) }),
        )
        .route("/version", get(version))
}

/// Tide-disco-compatible version response. Tide emits the binary's clap version; we emit the
/// crate version so `http_client::Client::connect` and similar polling helpers succeed.
async fn version() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Espresso's reward-state extensions, on both reward mounts. The merklized-state base routes
/// each mount inherits come from `hotshot_query_service::merklized_state`, which the caller builds
/// from its concrete data source; see [`create_router_v1`].
pub(crate) fn router_reward<S>(state: S) -> ApiRouter
where
    S: v1::RewardApi + Clone + Send + Sync + 'static,
{
    // Create handler closures that capture the generic state type
    let get_reward_claim_input =
        |State(state): State<S>, Path((height, address)): Path<(u64, String)>| async move {
            state
                .get_reward_claim_input(height, address)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let get_reward_balance =
        |State(state): State<S>, Path((height, address)): Path<(u64, String)>| async move {
            state
                .get_reward_balance(height, address)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let get_latest_reward_balance = |State(state): State<S>, Path(address): Path<String>| async move {
        state
            .get_latest_reward_balance(address)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_reward_account_proof =
        |State(state): State<S>, Path((height, address)): Path<(u64, String)>| async move {
            state
                .get_reward_account_proof(height, address)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let get_latest_reward_account_proof = |State(state): State<S>, Path(address): Path<String>| async move {
        state
            .get_latest_reward_account_proof(address)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_reward_amounts =
        |State(state): State<S>, Path((height, offset, limit)): Path<(u64, u64, u64)>| async move {
            state
                .get_reward_amounts(height, offset, limit)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let get_reward_merkle_tree_v2 = |State(state): State<S>, Path(height): Path<u64>| async move {
        <S as v1::RewardApi>::get_reward_merkle_tree_v2(&state, height)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    // Same underlying V2-tree lookup as `reward-state-v2/reward-balance`; tide registers this
    // route unconditionally for both merklized-state modules regardless of tree version.
    let get_reward_balance_v1 =
        |State(state): State<S>, Path((height, address)): Path<(u64, String)>| async move {
            state
                .get_reward_balance(height, address)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let get_reward_account_proof_v1 =
        |State(state): State<S>, Path((height, address)): Path<(u64, String)>| async move {
            state
                .get_reward_account_proof_v1(height, address)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    ApiRouter::new()
        .api_route(
            routes::v1::REWARD_CLAIM_INPUT_ROUTE,
            get_with(get_reward_claim_input, |op| {
                op.summary("Get reward claim input").description("Returns the RewardClaimInput needed to call claimRewards() on L1: lifetime rewards, Merkle proof, and auth root inputs, for the account at the given block height finalized by the light client contract.")
            }),
        )
        .api_route(
            routes::v1::REWARD_BALANCE_ROUTE,
            get_with(get_reward_balance, |op| {
                op.summary("Get reward balance at height").description("Get balance in reward state at a specific height for an Ethereum address.")
            }),
        )
        .api_route(
            routes::v1::LATEST_REWARD_BALANCE_ROUTE,
            get_with(get_latest_reward_balance, |op| {
                op.summary("Get latest reward balance").description("Get current balance in reward state for an Ethereum address.")
            }),
        )
        .api_route(
            routes::v1::REWARD_ACCOUNT_PROOF_ROUTE,
            get_with(get_reward_account_proof, |op| {
                op.summary("Get reward account proof").description("Get the Merkle proof for a reward account at a given block height (RewardAccountProofV1 pre-V4, RewardAccountProofV2 from V4 onward).")
            }),
        )
        .api_route(
            routes::v1::LATEST_REWARD_ACCOUNT_PROOF_ROUTE,
            get_with(get_latest_reward_account_proof, |op| {
                op.summary("Get latest reward account proof").description("Get the Merkle proof (RewardAccountProofV2) for a reward account at the latest block height finalized by the light client contract.")
            }),
        )
        .api_route(
            routes::v1::REWARD_AMOUNTS_ROUTE,
            get_with(get_reward_amounts, |op| {
                op.summary("List reward amounts").description("Return all RewardMerkleTreeV2 accounts stored for the requested height, paginated by offset and limit (limit must be <= 10000).")
            }),
        )
        .api_route(
            routes::v1::REWARD_MERKLE_TREE_V2_ROUTE,
            get_with(get_reward_merkle_tree_v2, |op| {
                op.summary("Get RewardMerkleTreeV2 snapshot").description("Get the snapshot of this node's RewardMerkleTreeV2 at the given block height, serialized as RewardMerkleTreeV2Data.")
            }),
        )
        .api_route(
            routes::v1::REWARD_V1_BALANCE_ROUTE,
            get_with(get_reward_balance_v1, |op| {
                op.summary("Get reward balance at height (v1 mount)").description("Same handler as reward-state-v2/reward-balance, registered on the reward-state mount; tide-disco shared this handler across both merklized-state mounts.")
            }),
        )
        .api_route(
            routes::v1::REWARD_V1_ACCOUNT_PROOF_ROUTE,
            get_with(get_reward_account_proof_v1, |op| {
                op.summary("Get reward account proof (v1 mount)").description("Same handler as reward-state-v2/proof, registered on the reward-state mount; tide-disco shared this handler across both merklized-state mounts.")
            }),
        )
        // Tide-disco twins of the reward-state-v2 routes above, registered on the same
        // handlers (tide shared them across both merklized-state modules).
        .api_route(
            routes::v1::REWARD_V1_LATEST_BALANCE_ROUTE,
            get_with(get_latest_reward_balance, |op| {
                op.summary("Get latest reward balance (v1 mount)").description("Same handler as reward-state-v2/reward-balance/latest, registered on the reward-state mount; tide-disco shared this handler across both merklized-state mounts.")
            }),
        )
        .api_route(
            routes::v1::REWARD_V1_LATEST_ACCOUNT_PROOF_ROUTE,
            get_with(get_latest_reward_account_proof, |op| {
                op.summary("Get latest reward account proof (v1 mount)").description("Same handler as reward-state-v2/proof/latest, registered on the reward-state mount; tide-disco shared this handler across both merklized-state mounts.")
            }),
        )
        .api_route(
            routes::v1::REWARD_V1_AMOUNTS_ROUTE,
            get_with(get_reward_amounts, |op| {
                op.summary("List reward amounts (v1 mount)").description("Same handler as reward-state-v2/reward-amounts, registered on the reward-state mount; tide-disco shared this handler across both merklized-state mounts.")
            }),
        )
        .api_route(
            routes::v1::REWARD_V1_MERKLE_TREE_V2_ROUTE,
            get_with(get_reward_merkle_tree_v2, |op| {
                op.summary("Get RewardMerkleTreeV2 snapshot (v1 mount)").description("Same handler as reward-state-v2/reward-merkle-tree-v2, registered on the reward-state mount; tide-disco shared this handler across both merklized-state mounts.")
            }),
        )
        .with_state(state)
}

/// Espresso's availability extensions. The module's base routes come from
/// `hotshot_query_service::availability`, which the caller builds from its concrete data source
/// and passes to its serve entry point as part of the query-service router; see
/// [`create_router_v1`].
pub(crate) fn router_availability<S>(state: S) -> ApiRouter
where
    S: v1::AvailabilityApiExtension + Clone + Send + Sync + 'static,
{
    // Route: /v1/availability/block/{height}/namespace/{namespace}
    let get_namespace_proof_by_height =
        |State(state): State<S>, Path((height, namespace)): Path<(u64, u32)>| async move {
            state
                .get_namespace_proof(v1::availability::BlockId::Height(height), namespace)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    // Route: /v1/availability/block/hash/{hash}/namespace/{namespace}
    let get_namespace_proof_by_hash =
        |State(state): State<S>, Path((hash, namespace)): Path<(String, u32)>| async move {
            state
                .get_namespace_proof(v1::availability::BlockId::Hash(hash), namespace)
                .await
                .map(ApiJson)
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
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    // Route: /v1/availability/block/{from}/{until}/namespace/{namespace}
    let get_namespace_proof_range =
        |State(state): State<S>, Path((from, until, namespace)): Path<(u64, u64, u32)>| async move {
            state
                .get_namespace_proof_range(from, until, namespace)
                .await
                .map(ApiJson)
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
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let get_state_cert_v1 = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        <S as v1::AvailabilityApiExtension>::get_state_cert(&state, epoch)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_state_cert_v2 = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .get_state_cert_v2(epoch)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let stream_namespace_proofs =
        |ws: WebSocketUpgrade,
         State(state): State<S>,
         headers: HeaderMap,
         Path((height, namespace)): Path<(usize, u32)>| async move {
            let format = ContentType::negotiate(&headers);
            ws.on_upgrade(move |socket| async move {
                match state.stream_namespace_proofs(height, namespace).await {
                    Ok(stream) => drive_ws_stream(socket, stream, format).await,
                    Err(e) => tracing::warn!("stream_namespace_proofs: {e}"),
                }
            })
        };

    ApiRouter::new()
        .api_route(
            routes::v1::NAMESPACE_PROOF_BY_HEIGHT_ROUTE,
            get_with(get_namespace_proof_by_height, |op| {
                op.summary("Get namespace proof").description(
                    "Get the transactions in a namespace of the given block, along with a proof \
                     of completeness.",
                )
            }),
        )
        .api_route(
            routes::v1::NAMESPACE_PROOF_BY_HASH_ROUTE,
            get_with(get_namespace_proof_by_hash, |op| {
                op.summary("Get namespace proof").description(
                    "Get the transactions in a namespace of the given block, along with a proof \
                     of completeness.",
                )
            }),
        )
        .api_route(
            routes::v1::NAMESPACE_PROOF_BY_PAYLOAD_HASH_ROUTE,
            get_with(get_namespace_proof_by_payload_hash, |op| {
                op.summary("Get namespace proof").description(
                    "Get the transactions in a namespace of the given block, along with a proof \
                     of completeness.",
                )
            }),
        )
        .api_route(
            routes::v1::NAMESPACE_PROOF_RANGE_ROUTE,
            get_with(get_namespace_proof_range, |op| {
                op.summary("Get namespace proofs for a range").description(
                    "Get the transactions in the specified namespace from each block in a range, \
                     with proofs.",
                )
            }),
        )
        .api_route(
            routes::v1::INCORRECT_ENCODING_PROOF_ROUTE,
            get_with(get_incorrect_encoding_proof, |op| {
                op.summary("Get incorrect-encoding proof").description(
                    "Generate a proof of incorrect namespace encoding for the given block number.",
                )
            }),
        )
        .api_route(
            routes::v1::STATE_CERT_V1_ROUTE,
            get_with(get_state_cert_v1, |op| {
                op.summary("Get state certificate (V1)").description(
                    "Get the light client state update certificate (V1) for the given epoch, used \
                     to update the light client contract's stake table.",
                )
            }),
        )
        .api_route(
            routes::v1::STATE_CERT_V2_ROUTE,
            get_with(get_state_cert_v2, |op| {
                op.summary("Get state certificate (V2)").description(
                    "Get the light client state update certificate (V2) for the given epoch; \
                     includes the auth_root Keccak-256 hash of the reward Merkle tree roots.",
                )
            }),
        )
        .api_route(
            routes::v1::STREAM_NAMESPACE_PROOFS_ROUTE,
            get_with(stream_namespace_proofs, |op| {
                op.summary("Stream namespace proofs (websocket)")
                    .description(
                        "Websocket endpoint: subscribe to namespace data and proofs for each \
                         block, starting at the given height.",
                    )
            }),
        )
        .with_state(state)
}

/// Espresso's fee-state extension. The module's base routes (Merkle path by height and by
/// commitment, snapshot height) come from `hotshot_query_service::merklized_state`, which the
/// caller builds from its concrete data source; see [`create_router_v1`].
pub(crate) fn router_fee_state<S>(state: S) -> ApiRouter
where
    S: v1::FeeStateApiExtension + Clone + Send + Sync + 'static,
{
    let get_fee_balance_latest = |State(state): State<S>, Path(address): Path<String>| async move {
        state
            .get_fee_balance_latest(address)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    ApiRouter::new()
        .api_route(
            routes::v1::FEE_STATE_BALANCE_LATEST_ROUTE,
            get_with(get_fee_balance_latest, |op| {
                op.summary("Get latest fee balance").description(
                    "Get the latest fee account balance for an address from the fee Merkle tree.",
                )
            }),
        )
        .with_state(state)
}

/// Espresso's status extension. The module's base routes (block height, success rate, time since
/// last decide, Prometheus metrics) come from `hotshot_query_service::status`, which the caller
/// builds from its concrete data source; see [`create_router_v1`].
pub(crate) fn router_status<S>(state: S) -> ApiRouter
where
    S: v1::StatusApiExtension + Clone + Send + Sync + 'static,
{
    let status_keys = |State(state): State<S>| async move {
        state.keys().await.map(ApiJson).map_err(ApiError::Internal)
    };

    ApiRouter::new()
        .api_route(
            routes::v1::STATUS_KEYS_ROUTE,
            get_with(status_keys, |op| {
                op.summary("Get node public keys").description(
                    "Get this node's public keys (Ethereum account, BLS, Schnorr, x25519). The \
                     BLS and Schnorr keys are formatted as in stake-table responses; the x25519 \
                     key is tagged base64. The Ethereum account is taken from the node's \
                     stake-table registration and is null if the node is not registered.",
                )
            }),
        )
        .with_state(state)
}

pub(crate) fn router_config<S>(state: S) -> ApiRouter
where
    S: v1::ConfigApi + Clone + Send + Sync + 'static,
{
    let config_hotshot = |State(state): State<S>| async move {
        <S as v1::ConfigApi>::hotshot_config(&state)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let config_env = |State(state): State<S>| async move {
        <S as v1::ConfigApi>::env(&state)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let config_runtime = |State(state): State<S>| async move {
        <S as v1::ConfigApi>::runtime_config(&state)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    ApiRouter::new()
        .api_route(
            routes::v1::CONFIG_HOTSHOT_ROUTE,
            get_with(config_hotshot, |op| {
                op.summary("Get HotShot config")
                    .description("Get the HotShot configuration for the current node.")
            }),
        )
        .api_route(
            routes::v1::CONFIG_ENV_ROUTE,
            get_with(config_env, |op| {
                op.summary("Get environment variables").description(
                    "Get all ESPRESSO_ environment variables set for the current node.",
                )
            }),
        )
        .api_route(
            routes::v1::CONFIG_RUNTIME_ROUTE,
            get_with(config_runtime, |op| {
                op.summary("Get runtime config").description(
                    "Get the merged runtime configuration (CLI flags + env vars + defaults); \
                     secrets and L1 RPC URLs are redacted.",
                )
            }),
        )
        .with_state(state)
}

/// Espresso's node extensions: the stake table, validators, participation rates, the block reward,
/// and the oldest objects storage still holds. The module's base routes (block height, transaction
/// counts, payload sizes, VID shares, sync status, header windows and limits) come from
/// `hotshot_query_service::node`, which the caller builds from its concrete data source; see
/// [`create_router_v1`].
pub(crate) fn router_node<S>(state: S) -> ApiRouter
where
    S: v1::NodeApiExtension + Clone + Send + Sync + 'static,
{
    let node_stake_table_current = |State(state): State<S>| async move {
        state
            .stake_table_current()
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let node_stake_table = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .stake_table(epoch)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let node_da_stake_table_current = |State(state): State<S>| async move {
        state
            .da_stake_table_current()
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let node_da_stake_table = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .da_stake_table(epoch)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let node_validators = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .get_validators(epoch)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let node_all_validators =
        |State(state): State<S>, Path((epoch, offset, limit)): Path<(u64, u64, u64)>| async move {
            state
                .get_all_validators(epoch, offset, limit)
                .await
                .map(ApiJson)
                .map_err(ApiError::BadRequest)
        };

    let node_proposal_participation_current = |State(state): State<S>| async move {
        state
            .current_proposal_participation()
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let node_proposal_participation = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .proposal_participation(epoch)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let node_vote_participation_current = |State(state): State<S>| async move {
        state
            .current_vote_participation()
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let node_vote_participation = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .vote_participation(epoch)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let node_block_reward = |State(state): State<S>| async move {
        state
            .get_block_reward(None)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let node_block_reward_epoch = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .get_block_reward(Some(epoch))
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let node_oldest_block = |State(state): State<S>| async move {
        state
            .get_oldest_block()
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let node_oldest_leaf = |State(state): State<S>| async move {
        state
            .get_oldest_leaf()
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    ApiRouter::new()
        .api_route(
            routes::v1::NODE_STAKE_TABLE_CURRENT_ROUTE,
            get_with(node_stake_table_current, |op| {
                op.summary("Get current stake table")
                    .description("Get the stake table for the current epoch.")
            }),
        )
        .api_route(
            routes::v1::NODE_STAKE_TABLE_ROUTE,
            get_with(node_stake_table, |op| {
                op.summary("Get stake table for epoch")
                    .description("Get the stake table for the given epoch.")
            }),
        )
        .api_route(
            routes::v1::NODE_DA_STAKE_TABLE_CURRENT_ROUTE,
            get_with(node_da_stake_table_current, |op| {
                op.summary("Get current DA stake table")
                    .description("Get the DA stake table for the current epoch.")
            }),
        )
        .api_route(
            routes::v1::NODE_DA_STAKE_TABLE_ROUTE,
            get_with(node_da_stake_table, |op| {
                op.summary("Get DA stake table for epoch")
                    .description("Get the DA stake table for the given epoch.")
            }),
        )
        .api_route(
            routes::v1::NODE_VALIDATORS_ROUTE,
            get_with(node_validators, |op| {
                op.summary("Get validators for epoch")
                    .description("Get the validators map for the given epoch.")
            }),
        )
        .api_route(
            routes::v1::NODE_ALL_VALIDATORS_ROUTE,
            get_with(node_all_validators, |op| {
                op.summary("Get all validators for epoch").description(
                    "Get all validators, including inactive ones, for the given epoch, paginated \
                     by offset and limit.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_PROPOSAL_PARTICIPATION_CURRENT_ROUTE,
            get_with(node_proposal_participation_current, |op| {
                op.summary("Get current proposal participation")
                    .description(
                        "Get the mapping from leader key to the fraction of views proposed \
                         properly as leader.",
                    )
            }),
        )
        .api_route(
            routes::v1::NODE_PROPOSAL_PARTICIPATION_ROUTE,
            get_with(node_proposal_participation, |op| {
                op.summary("Get proposal participation for epoch")
                    .description(
                        "Get the mapping from leader key to proposal participation rate for the \
                         given epoch.",
                    )
            }),
        )
        .api_route(
            routes::v1::NODE_VOTE_PARTICIPATION_CURRENT_ROUTE,
            get_with(node_vote_participation_current, |op| {
                op.summary("Get current vote participation").description(
                    "Get the mapping from node key to the fraction of views properly voted.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_VOTE_PARTICIPATION_ROUTE,
            get_with(node_vote_participation, |op| {
                op.summary("Get vote participation for epoch").description(
                    "Get the mapping from node key to vote participation rate for the given epoch.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_BLOCK_REWARD_ROUTE,
            get_with(node_block_reward, |op| {
                op.summary("Get block reward")
                    .description("Get the block reward.")
            }),
        )
        .api_route(
            routes::v1::NODE_BLOCK_REWARD_EPOCH_ROUTE,
            get_with(node_block_reward_epoch, |op| {
                op.summary("Get block reward for epoch")
                    .description("Get the block reward for the given epoch.")
            }),
        )
        .api_route(
            routes::v1::NODE_OLDEST_BLOCK_ROUTE,
            get_with(node_oldest_block, |op| {
                op.summary("Get oldest block").description(
                    "Get the oldest (smallest height) block present in storage, or null if none \
                     is stored.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_OLDEST_LEAF_ROUTE,
            get_with(node_oldest_leaf, |op| {
                op.summary("Get oldest leaf").description(
                    "Get the oldest (smallest height) leaf present in storage, or null if none is \
                     stored.",
                )
            }),
        )
        .with_state(state)
}

pub(crate) fn router_catchup<S>(state: S) -> ApiRouter
where
    S: v1::CatchupApi + Clone + Send + Sync + 'static,
{
    // Catchup handlers
    let catchup_account =
        |State(state): State<S>, Path((height, view, address)): Path<(u64, u64, String)>| async move {
            state
                .get_account(height, view, address)
                .await
                .map(ApiJson)
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
        Ok::<_, ApiError>(encode_ok::<ErrorResponse, _>(&headers, tree))
    };

    let catchup_blocks = |State(state): State<S>, Path((height, view)): Path<(u64, u64)>| async move {
        state
            .get_blocks_frontier(height, view)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let catchup_chainconfig = |State(state): State<S>, Path(commitment): Path<String>| async move {
        <S as v1::CatchupApi>::get_chain_config(&state, commitment)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let catchup_leafchain = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_leaf_chain(height)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let catchup_cert2 = |State(state): State<S>, Path(height): Path<u64>| async move {
        <S as v1::CatchupApi>::get_cert2(&state, height)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let catchup_reward_account =
        |State(state): State<S>, Path((height, view, address)): Path<(u64, u64, String)>| async move {
            state
                .get_reward_account_v1(height, view, address)
                .await
                .map(ApiJson)
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
        Ok::<_, ApiError>(encode_ok::<ErrorResponse, _>(&headers, tree))
    };

    let catchup_reward_account_v2 =
        |State(state): State<S>, Path((height, view, address)): Path<(u64, u64, String)>| async move {
            state
                .get_reward_account_v2(height, view, address)
                .await
                .map(ApiJson)
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
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let catchup_state_cert = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        <S as v1::CatchupApi>::get_state_cert(&state, epoch)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    ApiRouter::new()
        .api_route(
            routes::v1::CATCHUP_ACCOUNT_ROUTE,
            get_with(catchup_account, |op| {
                op.summary("Catch up fee account balance").description(
                    "Get the fee account balance and Merkle proof for an address at the given \
                     block height and view, for catchup.",
                )
            }),
        )
        .api_route(
            routes::v1::CATCHUP_ACCOUNTS_ROUTE,
            post_with(catchup_accounts, |op| {
                op.summary("Catch up fee accounts (bulk)").description(
                    "Bulk version of the fee account endpoint; request body is a JSON array of \
                     TaggedBase64 fee accounts, response is a FeeMerkleTree.",
                )
            }),
        )
        .api_route(
            routes::v1::CATCHUP_BLOCKS_ROUTE,
            get_with(catchup_blocks, |op| {
                op.summary("Catch up blocks Merkle frontier").description(
                    "Get the blocks Merkle tree frontier at the given block height and view, for \
                     catchup.",
                )
            }),
        )
        .api_route(
            routes::v1::CATCHUP_CHAINCONFIG_ROUTE,
            get_with(catchup_chainconfig, |op| {
                op.summary("Catch up chain config").description(
                    "Retrieve the chain config matching the given commitment from a peer; used \
                     when a node missed a protocol upgrade.",
                )
            }),
        )
        .api_route(
            routes::v1::CATCHUP_LEAFCHAIN_ROUTE,
            get_with(catchup_leafchain, |op| {
                op.summary("Catch up leaf chain").description(
                    "Fetch a leaf chain that decides the block at the given height, for catching \
                     up the stake table.",
                )
            }),
        )
        .api_route(
            routes::v1::CATCHUP_CERT2_ROUTE,
            get_with(catchup_cert2, |op| {
                op.summary("Catch up cert2").description(
                    "Fetch the cert2 stored at exactly the given height, if one exists; 404 \
                     otherwise.",
                )
            }),
        )
        .api_route(
            routes::v1::CATCHUP_REWARD_ACCOUNT_ROUTE,
            get_with(catchup_reward_account, |op| {
                op.summary("Catch up reward account (V1)").description(
                    "Get the reward account balance for an address at the given height and view.",
                )
            }),
        )
        .api_route(
            routes::v1::CATCHUP_REWARD_ACCOUNTS_ROUTE,
            post_with(catchup_reward_accounts, |op| {
                op.summary("Catch up reward accounts (bulk, V1)")
                    .description(
                        "Bulk version of the reward account endpoint; request body is a JSON \
                         array of TaggedBase64 reward accounts, response is a RewardMerkleTreeV1.",
                    )
            }),
        )
        .api_route(
            routes::v1::CATCHUP_REWARD_ACCOUNT_V2_ROUTE,
            get_with(catchup_reward_account_v2, |op| {
                op.summary("Catch up reward account (V2)").description(
                    "Get the reward account balance for an address at the given height and view, \
                     from RewardMerkleTreeV2.",
                )
            }),
        )
        .api_route(
            routes::v1::CATCHUP_REWARD_ACCOUNTS_V2_ROUTE,
            post_with(catchup_reward_accounts_v2, |op| {
                op.summary("Catch up reward accounts (bulk, V2) — deprecated")
                    .description("Deprecated: this endpoint always returns 404 Not Found.")
            }),
        )
        .api_route(
            routes::v1::CATCHUP_REWARD_AMOUNTS_ROUTE,
            get_with(catchup_reward_amounts, |op| {
                op.summary("List reward amounts — deprecated")
                    .description("Deprecated: this endpoint always returns 404 Not Found.")
            }),
        )
        .api_route(
            routes::v1::CATCHUP_REWARD_MERKLE_TREE_V2_ROUTE,
            get_with(catchup_reward_merkle_tree_v2, |op| {
                op.summary("Catch up RewardMerkleTreeV2").description(
                    "Get the RewardMerkleTreeV2 from consensus state at the given height and \
                     view, serialized as RewardMerkleTreeV2Data.",
                )
            }),
        )
        .api_route(
            routes::v1::CATCHUP_STATE_CERT_ROUTE,
            get_with(catchup_state_cert, |op| {
                op.summary("Catch up state certificate")
                    .description("Get the light client state certificate for the given epoch.")
            }),
        )
        .with_state(state)
}

pub(crate) fn router_submit<S>(state: S) -> ApiRouter
where
    S: v1::SubmitApi + Clone + Send + Sync + 'static,
{
    // Submit handler — body is decoded as VBS (binary) or JSON based on Content-Type, matching
    // tide-disco's `body_auto`.
    let submit_submit = |State(state): State<S>, headers: HeaderMap, body: Bytes| async move {
        let tx: <S as v1::SubmitApi>::Transaction = decode_body(&headers, &body)?;
        let hash = state.submit(tx).await.map_err(ApiError::Internal)?;
        Ok::<_, ApiError>(encode_ok::<ErrorResponse, _>(&headers, hash))
    };

    ApiRouter::new()
        .api_route(
            routes::v1::SUBMIT_ROUTE,
            post_with(submit_submit, |op| {
                op.summary("Submit transaction")
                    .description("Submit a transaction to the HotShot handle for sequencing.")
            }),
        )
        .with_state(state)
}

pub(crate) fn router_state_signature<S>(state: S) -> ApiRouter
where
    S: v1::StateSignatureApi + Clone + Send + Sync + 'static,
{
    // State signature handler
    let state_signature_block = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_state_signature(height)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    ApiRouter::new()
        .api_route(
            routes::v1::STATE_SIGNATURE_BLOCK_ROUTE,
            get_with(state_signature_block, |op| {
                op.summary("Get light client state signature").description(
                    "Get this node's signature for the light client state at the given block \
                     height.",
                )
            }),
        )
        .with_state(state)
}

pub(crate) fn router_hotshot_events<S>(state: S) -> ApiRouter
where
    S: v1::HotShotEventsApi + Clone + Send + Sync + 'static,
{
    // HotShot events handlers
    let hotshot_events_startup = |State(state): State<S>| async move {
        state
            .startup_info()
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let hotshot_events_stream =
        |State(state): State<S>, headers: HeaderMap, ws: WebSocketUpgrade| async move {
            let format = ContentType::negotiate(&headers);
            match <S as v1::HotShotEventsApi>::events(&state).await {
                Ok(stream) => ws.on_upgrade(move |socket| async move {
                    drive_ws_stream(socket, stream, format).await
                }),
                Err(err) => ApiError::Internal(err).into_response(),
            }
        };

    ApiRouter::new()
        .api_route(
            routes::v1::HOTSHOT_EVENTS_STARTUP_ROUTE,
            get_with(hotshot_events_startup, |op| {
                op.summary("Get startup info").description(
                    "Get startup info: known nodes with stake and their public keys, and the \
                     count of non-staked nodes.",
                )
            }),
        )
        .api_route(
            routes::v1::HOTSHOT_EVENTS_STREAM_ROUTE,
            get_with(hotshot_events_stream, |op| {
                op.summary("Stream HotShot events (websocket)")
                    .description("Websocket endpoint: get legacy HotShot events starting now.")
            }),
        )
        .with_state(state)
}

pub(crate) fn router_light_client<S>(state: S) -> ApiRouter
where
    S: v1::LightClientApi + Clone + Send + Sync + 'static,
{
    // Light-client handlers
    let lc_leaf_by_height = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_leaf_proof(v1::LeafQuery::Height(height), None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let lc_leaf_by_height_finalized =
        |State(state): State<S>, Path((height, finalized)): Path<(u64, u64)>| async move {
            state
                .get_leaf_proof(v1::LeafQuery::Height(height), Some(finalized))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let lc_leaf_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_leaf_proof(v1::LeafQuery::Hash(hash), None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let lc_leaf_by_hash_finalized =
        |State(state): State<S>, Path((hash, finalized)): Path<(String, u64)>| async move {
            state
                .get_leaf_proof(v1::LeafQuery::Hash(hash), Some(finalized))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let lc_leaf_by_block_hash = |State(state): State<S>, Path(block_hash): Path<String>| async move {
        state
            .get_leaf_proof(v1::LeafQuery::BlockHash(block_hash), None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let lc_leaf_by_block_hash_finalized =
        |State(state): State<S>, Path((block_hash, finalized)): Path<(String, u64)>| async move {
            state
                .get_leaf_proof(v1::LeafQuery::BlockHash(block_hash), Some(finalized))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let lc_leaf_by_payload_hash = |State(state): State<S>, Path(payload_hash): Path<String>| async move {
        state
            .get_leaf_proof(v1::LeafQuery::PayloadHash(payload_hash), None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let lc_leaf_by_payload_hash_finalized =
        |State(state): State<S>, Path((payload_hash, finalized)): Path<(String, u64)>| async move {
            state
                .get_leaf_proof(v1::LeafQuery::PayloadHash(payload_hash), Some(finalized))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let lc_header_by_height = |State(state): State<S>, Path((root, height)): Path<(u64, u64)>| async move {
        state
            .get_header_proof(root, v1::HeaderQuery::Height(height))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let lc_header_by_hash = |State(state): State<S>, Path((root, hash)): Path<(u64, String)>| async move {
        state
            .get_header_proof(root, v1::HeaderQuery::Hash(hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let lc_header_by_payload_hash =
        |State(state): State<S>, Path((root, payload_hash)): Path<(u64, String)>| async move {
            state
                .get_header_proof(root, v1::HeaderQuery::PayloadHash(payload_hash))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let lc_stake_table = |State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .get_light_client_stake_table(epoch)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let lc_payload = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_payload_proof(height)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let lc_payload_range = |State(state): State<S>, Path((start, end)): Path<(u64, u64)>| async move {
        state
            .get_payload_proof_range(start, end)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let lc_namespace = |State(state): State<S>, Path((height, namespace)): Path<(u64, u64)>| async move {
        state
            .get_lc_namespace_proof(height, namespace)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let lc_namespace_range =
        |State(state): State<S>, Path((start, end, namespace)): Path<(u64, u64, u64)>| async move {
            state
                .get_lc_namespace_proof_range(start, end, namespace)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let lc_namespaces_range =
        |State(state): State<S>, Path((start, end, namespaces)): Path<(u64, u64, String)>| async move {
            state
                .get_lc_namespaces_proof_range(start, end, namespaces)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    ApiRouter::new()
        .api_route(
            routes::v1::LC_LEAF_BY_HEIGHT_ROUTE,
            get_with(lc_leaf_by_height, |op| {
                op.summary("Get leaf with finality proof").description(
                    "Fetch a leaf by height plus a proof of its finality, optionally relative to \
                     an already-known-finalized height.",
                )
            }),
        )
        .api_route(
            routes::v1::LC_LEAF_BY_HEIGHT_FINALIZED_ROUTE,
            get_with(lc_leaf_by_height_finalized, |op| {
                op.summary("Get leaf with finality proof").description(
                    "Fetch a leaf by height plus a proof of its finality, optionally relative to \
                     an already-known-finalized height.",
                )
            }),
        )
        .api_route(
            routes::v1::LC_LEAF_BY_HASH_ROUTE,
            get_with(lc_leaf_by_hash, |op| {
                op.summary("Get leaf with finality proof").description(
                    "Fetch a leaf by hash plus a proof of its finality, optionally relative to an \
                     already-known-finalized height.",
                )
            }),
        )
        .api_route(
            routes::v1::LC_LEAF_BY_HASH_FINALIZED_ROUTE,
            get_with(lc_leaf_by_hash_finalized, |op| {
                op.summary("Get leaf with finality proof").description(
                    "Fetch a leaf by hash plus a proof of its finality, optionally relative to an \
                     already-known-finalized height.",
                )
            }),
        )
        .api_route(
            routes::v1::LC_LEAF_BY_BLOCK_HASH_ROUTE,
            get_with(lc_leaf_by_block_hash, |op| {
                op.summary("Get leaf with finality proof").description(
                    "Fetch a leaf by block hash plus a proof of its finality, optionally relative \
                     to an already-known-finalized height.",
                )
            }),
        )
        .api_route(
            routes::v1::LC_LEAF_BY_BLOCK_HASH_FINALIZED_ROUTE,
            get_with(lc_leaf_by_block_hash_finalized, |op| {
                op.summary("Get leaf with finality proof").description(
                    "Fetch a leaf by block hash plus a proof of its finality, optionally relative \
                     to an already-known-finalized height.",
                )
            }),
        )
        .api_route(
            routes::v1::LC_LEAF_BY_PAYLOAD_HASH_ROUTE,
            get_with(lc_leaf_by_payload_hash, |op| {
                op.summary("Get leaf with finality proof").description(
                    "Fetch a leaf by payload hash plus a proof of its finality, optionally \
                     relative to an already-known-finalized height.",
                )
            }),
        )
        .api_route(
            routes::v1::LC_LEAF_BY_PAYLOAD_HASH_FINALIZED_ROUTE,
            get_with(lc_leaf_by_payload_hash_finalized, |op| {
                op.summary("Get leaf with finality proof").description(
                    "Fetch a leaf by payload hash plus a proof of its finality, optionally \
                     relative to an already-known-finalized height.",
                )
            }),
        )
        .api_route(
            routes::v1::LC_HEADER_BY_HEIGHT_ROUTE,
            get_with(lc_header_by_height, |op| {
                op.summary("Get header with inclusion proof").description(
                    "Fetch a header plus a Merkle proof that it belongs to the blocks Merkle tree \
                     rooted at the given root height.",
                )
            }),
        )
        .api_route(
            routes::v1::LC_HEADER_BY_HASH_ROUTE,
            get_with(lc_header_by_hash, |op| {
                op.summary("Get header with inclusion proof").description(
                    "Fetch a header plus a Merkle proof that it belongs to the blocks Merkle tree \
                     rooted at the given root height.",
                )
            }),
        )
        .api_route(
            routes::v1::LC_HEADER_BY_PAYLOAD_HASH_ROUTE,
            get_with(lc_header_by_payload_hash, |op| {
                op.summary("Get header with inclusion proof").description(
                    "Fetch a header plus a Merkle proof that it belongs to the blocks Merkle tree \
                     rooted at the given root height.",
                )
            }),
        )
        .api_route(
            routes::v1::LC_STAKE_TABLE_ROUTE,
            get_with(lc_stake_table, |op| {
                op.summary("Get stake table events for epoch").description(
                    "Get the events needed to transform the stake table from the previous epoch \
                     into the given epoch.",
                )
            }),
        )
        .api_route(
            routes::v1::LC_PAYLOAD_ROUTE,
            get_with(lc_payload, |op| {
                op.summary("Get payload with VID common data").description(
                    "Fetch a payload plus the VID common data needed to recompute and verify its \
                     hash.",
                )
            }),
        )
        .api_route(
            routes::v1::LC_PAYLOAD_RANGE_ROUTE,
            get_with(lc_payload_range, |op| {
                op.summary("Get payload proofs in range").description(
                    "Fetch a list of payload proofs for each block in the given range.",
                )
            }),
        )
        .api_route(
            routes::v1::LC_NAMESPACE_ROUTE,
            get_with(lc_namespace, |op| {
                op.summary("Get namespace proof with VID common data")
                    .description(
                        "Fetch a namespace proof plus the VID common data needed to verify it.",
                    )
            }),
        )
        .api_route(
            routes::v1::LC_NAMESPACE_RANGE_ROUTE,
            get_with(lc_namespace_range, |op| {
                op.summary("Get namespace proofs in range").description(
                    "Fetch a list of namespace proofs for each block in the given range.",
                )
            }),
        )
        .api_route(
            routes::v1::LC_NAMESPACES_RANGE_ROUTE,
            get_with(lc_namespaces_range, |op| {
                op.summary("Get proofs for multiple namespaces in range")
                    .description(
                        "Fetch namespace proofs for each block in the given range, restricted to \
                         a caller-specified set of namespaces.",
                    )
            }),
        )
        .with_state(state)
}

pub(crate) fn router_token<S>(state: S) -> ApiRouter
where
    S: v1::TokenApi + Clone + Send + Sync + 'static,
{
    // Token handlers
    let token_total_minted = |State(state): State<S>| async move {
        state
            .total_minted_supply()
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let token_circulating = |State(state): State<S>| async move {
        state
            .circulating_supply()
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let token_circulating_eth = |State(state): State<S>| async move {
        state
            .circulating_supply_ethereum()
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let token_total_issued = |State(state): State<S>| async move {
        state
            .total_issued_supply()
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let token_total_reward_distributed = |State(state): State<S>| async move {
        state
            .total_reward_distributed()
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    ApiRouter::new()
        .api_route(
            routes::v1::TOKEN_TOTAL_MINTED_SUPPLY_ROUTE,
            get_with(token_total_minted, |op| {
                op.summary("Get total minted supply").description(
                    "Total supply of the ESP token minted on Ethereum; excludes unclaimed \
                     rewards. Cached for an hour.",
                )
            }),
        )
        .api_route(
            routes::v1::TOKEN_CIRCULATING_SUPPLY_ROUTE,
            get_with(token_circulating, |op| {
                op.summary("Get circulating supply").description(
                    "Circulating supply: initial_supply + reward_distributed - locked, following \
                     the mainnet unlock schedule.",
                )
            }),
        )
        .api_route(
            routes::v1::TOKEN_CIRCULATING_SUPPLY_ETHEREUM_ROUTE,
            get_with(token_circulating_eth, |op| {
                op.summary("Get circulating supply (Ethereum L1)")
                    .description(
                        "Circulating supply of ESP tokens on Ethereum L1: total_supply_l1 - \
                         locked.",
                    )
            }),
        )
        .api_route(
            routes::v1::TOKEN_TOTAL_ISSUED_SUPPLY_ROUTE,
            get_with(token_total_issued, |op| {
                op.summary("Get total issued supply").description(
                    "Total issued supply: initial_supply + total_reward_distributed, including \
                     rewards not yet claimed on Ethereum.",
                )
            }),
        )
        .api_route(
            routes::v1::TOKEN_TOTAL_REWARD_DISTRIBUTED_ROUTE,
            get_with(token_total_reward_distributed, |op| {
                op.summary("Get total reward distributed").description(
                    "Total rewards distributed by consensus, including rewards not yet claimed on \
                     Ethereum.",
                )
            }),
        )
        .with_state(state)
}

pub(crate) fn router_database<S>(state: S) -> ApiRouter
where
    S: v1::DatabaseApi + Clone + Send + Sync + 'static,
{
    // Database handlers
    let database_table_sizes = |State(state): State<S>| async move {
        <S as v1::DatabaseApi>::get_table_sizes(&state)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };
    let database_migration_status = |State(state): State<S>| async move {
        <S as v1::DatabaseApi>::get_migration_status(&state)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    ApiRouter::new()
        .api_route(
            routes::v1::DATABASE_TABLE_SIZES_ROUTE,
            get_with(database_table_sizes, |op| {
                op.summary("Get database table sizes")
                    .description("Get the sizes of all database tables: row counts and disk usage.")
            }),
        )
        .api_route(
            routes::v1::DATABASE_MIGRATION_STATUS_ROUTE,
            get_with(database_migration_status, |op| {
                op.summary("Get migration status").description(
                    "Get the status of all deferred background migrations: start/completion time \
                     and last processed offset.",
                )
            }),
        )
        .with_state(state)
}

/// Create v1 router with OpenAPI documentation.
///
/// Unlike v2 (which documents proto request/response types with real JSON schemas), most v1
/// handlers return internal domain types that don't implement `schemars::JsonSchema` by design —
/// see [`ApiJson`]. The generated spec therefore documents routes, parameters, and summaries, but
/// response bodies are mostly untyped.
///
/// `hqs_base` carries the `hotshot-query-service` modules' own routes. This crate is deliberately
/// agnostic of the concrete node types, so it cannot build them; the caller does, from its data
/// source, and nests each at its `routes::v1::*_PREFIX` before passing them here as one router.
/// Merging it puts those routes next to Espresso's extensions for the same modules, and brings
/// their documentation into this spec.
pub fn create_router_v1<S>(state: S, hqs_base: ApiRouter) -> Router
where
    S: v1::RewardApi
        + v1::AvailabilityApiExtension
        + v1::FeeStateApiExtension
        + v1::StatusApiExtension
        + v1::ConfigApi
        + v1::NodeApiExtension
        + v1::CatchupApi
        + v1::SubmitApi
        + v1::StateSignatureApi
        + v1::HotShotEventsApi
        + v1::LightClientApi
        + v1::TokenApi
        + v1::DatabaseApi
        + Clone
        + Send
        + Sync
        + 'static,
{
    // Each `router_*` function already calls `with_state`, so the merged router is already
    // stateless (`ApiRouter<()>`) by the time it reaches `finish_api`.
    let router = hqs_base
        .merge(router_reward(state.clone()))
        .merge(router_availability(state.clone()))
        .merge(router_fee_state(state.clone()))
        .merge(router_status(state.clone()))
        .merge(router_config(state.clone()))
        .merge(router_node(state.clone()))
        .merge(router_catchup(state.clone()))
        .merge(router_submit(state.clone()))
        .merge(router_state_signature(state.clone()))
        .merge(router_hotshot_events(state.clone()))
        .merge(router_light_client(state.clone()))
        .merge(router_token(state.clone()))
        .merge(router_database(state));

    finish_v1_docs(router)
}

/// Build the OpenAPI spec for the mounted routes and attach the docs routes; every serve mode
/// must route through this.
pub fn finish_v1_docs(router: ApiRouter) -> Router {
    let mut api = OpenApi {
        info: Info {
            title: "Espresso Node API v1".to_string(),
            description: None,
            version: "1.0.0".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    let router = router.finish_api(&mut api);

    declare_path_template_parameters(&mut api);
    tag_operations_by_module(&mut api);

    // Transform examples (array) to example (singular) for OpenAPI 3.0/Swagger compatibility,
    // matching create_router_v2 (a no-op unless a future v1 route adds a JsonSchema body/query).
    if let Some(ref mut components) = api.components {
        let mut transform = schemars::transform::SetSingleExample::default();
        for schema in components.schemas.values_mut() {
            transform.transform(&mut schema.json_schema);
        }
    }

    router
        .route(routes::v1::OPENAPI_SPEC_ROUTE, get(serve_openapi_spec_v1))
        .route(
            routes::v1::SWAGGER_ROUTE,
            get(|| async { swagger_html(routes::v1::OPENAPI_SPEC_ROUTE) }),
        )
        .route(
            "/v1/",
            get(|| async { swagger_html(routes::v1::OPENAPI_SPEC_ROUTE) }),
        )
        .route(
            routes::v1::SCALAR_ROUTE,
            get(Scalar::new(routes::v1::OPENAPI_SPEC_ROUTE)
                .with_title("Espresso Node API v1")
                .axum_handler()),
        )
        .layer(Extension(OpenApiV1(api)))
}

/// Declare a path parameter for every `{name}` template segment of every operation.
///
/// aide only derives path parameters from `Path<T>` extractors whose `T` is a named-field
/// struct; the v1 handlers all use primitives and tuples (`Path<u64>`, `Path<(u64, String)>`),
/// so nothing is derived and Swagger's try-it-out cannot fill the URL templates. The template
/// itself names every parameter, so declare them from it.
///
/// Parameter types come from [`path_parameter_schema`]; the handlers parse the raw segment
/// either way, so a wrong entry there affects only documentation, not behavior.
/// Tag each operation with its module (first path segment after `/v1/`) so Swagger groups them.
fn tag_operations_by_module(api: &mut OpenApi) {
    let Some(ref mut paths) = api.paths else {
        return;
    };
    let mut modules = std::collections::BTreeSet::new();
    for (path, path_item_ref) in paths.paths.iter_mut() {
        let ReferenceOr::Item(path_item) = path_item_ref else {
            continue;
        };
        let Some(module) = path
            .strip_prefix("/v1/")
            .and_then(|rest| rest.split('/').next())
        else {
            continue;
        };
        modules.insert(module.to_string());
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
            operation.tags = vec![module.to_string()];
        }
    }
    api.tags = modules
        .into_iter()
        .map(|name| aide::openapi::Tag {
            name,
            ..Default::default()
        })
        .collect();
}

/// Types read off the handlers' `Path<T>` extractors; unknown names are strings.
fn path_parameter_schema(name: &str) -> schemars::Schema {
    match name {
        "height" | "block_number" | "from" | "until" | "to" | "start" | "end" | "epoch"
        | "epoch_number" | "view" | "index" | "limit" | "offset" | "namespace" | "finalized" => {
            schemars::json_schema!({"type": "integer", "minimum": 0})
        },
        _ => schemars::json_schema!({"type": "string"}),
    }
}

fn declare_path_template_parameters(api: &mut OpenApi) {
    let Some(ref mut paths) = api.paths else {
        return;
    };
    for (path, path_item_ref) in paths.paths.iter_mut() {
        let ReferenceOr::Item(path_item) = path_item_ref else {
            continue;
        };
        let names: Vec<&str> = path
            .split('/')
            .filter_map(|seg| seg.strip_prefix('{').and_then(|s| s.strip_suffix('}')))
            .collect();
        if names.is_empty() {
            continue;
        }
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
            for name in &names {
                let already_declared = operation.parameters.iter().any(|p| {
                    matches!(
                        p,
                        ReferenceOr::Item(Parameter::Path {
                            parameter_data,
                            ..
                        }) if parameter_data.name == *name
                    )
                });
                if already_declared {
                    continue;
                }
                operation
                    .parameters
                    .push(ReferenceOr::Item(Parameter::Path {
                        parameter_data: ParameterData {
                            name: (*name).to_string(),
                            description: None,
                            required: true,
                            deprecated: None,
                            format: ParameterSchemaOrContent::Schema(SchemaObject {
                                json_schema: path_parameter_schema(name),
                                external_docs: None,
                                example: None,
                            }),
                            example: None,
                            examples: Default::default(),
                            explode: None,
                            extensions: Default::default(),
                        },
                        style: PathStyle::Simple,
                    }));
            }
        }
    }
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
        .route(
            routes::v2::SWAGGER_ROUTE,
            get(|| async { swagger_html(routes::v2::OPENAPI_SPEC_ROUTE) }),
        )
        .route(
            "/v2/",
            get(|| async { swagger_html(routes::v2::OPENAPI_SPEC_ROUTE) }),
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use futures::stream::BoxStream;
    use http_wire::WireVersion;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use vbs::{BinarySerializer, Serializer};

    use super::*;

    /// Stand-in for the `hotshot_query_service::availability` router, registering two of the same
    /// relative paths so the tests can check where mounting puts them. That crate is not a
    /// dependency here (this one is deliberately agnostic of the node types), so the real router
    /// cannot be built in these tests; its own tests cover the paths it declares.
    fn mock_availability_base() -> ApiRouter {
        ApiRouter::new()
            .api_route(
                "/leaf/{height}",
                get_with(async || ApiJson(()), |op| op.summary("Get leaf")),
            )
            .api_route(
                "/limits",
                get_with(
                    async || ApiJson(()),
                    |op| op.summary("Get availability limits"),
                ),
            )
    }

    /// Stand-in for the `hotshot_query_service::status` router; see [`mock_availability_base`].
    fn mock_status_base() -> ApiRouter {
        ApiRouter::new()
            .api_route(
                "/block-height",
                get_with(async || ApiJson(()), |op| op.summary("Get block height")),
            )
            .api_route(
                "/metrics",
                get_with(
                    async || ApiJson(()),
                    |op| op.summary("Get Prometheus metrics"),
                ),
            )
    }

    /// Stand-in for the `hotshot_query_service::node` router; see [`mock_availability_base`].
    fn mock_node_base() -> ApiRouter {
        ApiRouter::new()
            .api_route(
                "/block-height",
                get_with(
                    async || ApiJson(()),
                    |op| op.summary("Get this node's block height"),
                ),
            )
            .api_route(
                "/limits",
                get_with(async || ApiJson(()), |op| op.summary("Get node limits")),
            )
    }

    /// Stand-in for the `hotshot_query_service::explorer` router; see [`mock_availability_base`].
    fn mock_explorer_base() -> ApiRouter {
        ApiRouter::new()
            .api_route(
                "/explorer-summary",
                get_with(
                    async || ApiJson(()),
                    |op| op.summary("Get the explorer summary"),
                ),
            )
            .api_route(
                "/block/{height}",
                get_with(
                    async || ApiJson(()),
                    |op| op.summary("Get block detail by height"),
                ),
            )
    }

    /// Stand-in for the `hotshot_query_service::merklized_state` router, which the query modes
    /// mount once per merklized tree; see [`mock_availability_base`].
    fn mock_merklized_state_base(tree: &str) -> ApiRouter {
        let summary = format!("Get a {tree} Merkle path by height");
        ApiRouter::new()
            .api_route(
                "/{height}/{key}",
                get_with(async || ApiJson(()), move |op| op.summary(&summary)),
            )
            .api_route(
                "/block-height",
                get_with(
                    async || ApiJson(()),
                    |op| op.summary("Get the latest snapshot height"),
                ),
            )
    }

    /// The query-service router a query mode passes in: every base nested at its module prefix,
    /// the way the binary assembles it. The merklized-state router appears twice, once per tree,
    /// as the SQL mode mounts it four times.
    fn mock_hqs_base() -> ApiRouter {
        ApiRouter::new()
            .nest(routes::v1::AVAILABILITY_PREFIX, mock_availability_base())
            .nest(routes::v1::STATUS_PREFIX, mock_status_base())
            .nest(routes::v1::NODE_PREFIX, mock_node_base())
            .nest(routes::v1::EXPLORER_PREFIX, mock_explorer_base())
            .nest(
                routes::v1::BLOCK_STATE_PREFIX,
                mock_merklized_state_base("block_merkle_tree_bigint"),
            )
            .nest(
                routes::v1::REWARD_STATE_V2_PREFIX,
                mock_merklized_state_base("reward_merkle_tree_v2"),
            )
    }

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
    }

    #[async_trait::async_trait]
    impl v1::AvailabilityApiExtension for MockState {
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
    impl v1::FeeStateApiExtension for MockState {
        type FeeAmount = ();

        async fn get_fee_balance_latest(
            &self,
            _address: String,
        ) -> anyhow::Result<Option<Self::FeeAmount>> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl v1::StatusApiExtension for MockState {
        type Keys = ();

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
    impl v1::NodeApiExtension for MockState {
        type StakeTable = ();
        type StakeTableCurrent = ();
        type Validators = ();
        type AllValidators = ();
        type Participation = ();
        type BlockReward = ();
        type Block = ();
        type Leaf = ();

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
        let router = create_router_v1(MockState, mock_hqs_base());
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
        let router = create_router_v1(MockState, mock_hqs_base());
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

    /// The base availability routes come from `hotshot-query-service` now, so the mount has to
    /// put them (and their documentation) back on the paths this crate used to declare itself.
    #[tokio::test]
    async fn v1_openapi_spec_documents_mounted_availability_base() {
        let router = create_router_v1(MockState, mock_hqs_base());
        let req = Request::builder()
            .uri(routes::v1::OPENAPI_SPEC_ROUTE)
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
        let spec: serde_json::Value =
            serde_json::from_str(&body_string(resp).await).expect("valid JSON");
        let paths = spec["paths"].as_object().expect("spec has paths");

        for route in [routes::v1::LEAF_BY_HEIGHT_ROUTE, routes::v1::LIMITS_ROUTE] {
            let item = paths
                .get(route)
                .unwrap_or_else(|| panic!("{route} missing from spec: {:?}", paths.keys()));
            assert_eq!(item["get"]["tags"][0], "availability");
        }
        assert_eq!(
            paths[routes::v1::LEAF_BY_HEIGHT_ROUTE]["get"]["summary"],
            "Get leaf"
        );
        // The extensions are still there alongside the base.
        assert!(paths.contains_key(routes::v1::STATE_CERT_V1_ROUTE));
    }

    /// The base status routes come from `hotshot-query-service` now, so the mount has to put them
    /// (and their documentation) back on the paths this crate used to declare itself.
    #[tokio::test]
    async fn v1_openapi_spec_documents_mounted_status_base() {
        let router = create_router_v1(MockState, mock_hqs_base());
        let req = Request::builder()
            .uri(routes::v1::OPENAPI_SPEC_ROUTE)
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
        let spec: serde_json::Value =
            serde_json::from_str(&body_string(resp).await).expect("valid JSON");
        let paths = spec["paths"].as_object().expect("spec has paths");

        for route in [
            routes::v1::STATUS_BLOCK_HEIGHT_ROUTE,
            routes::v1::STATUS_METRICS_ROUTE,
        ] {
            let item = paths
                .get(route)
                .unwrap_or_else(|| panic!("{route} missing from spec: {:?}", paths.keys()));
            assert_eq!(item["get"]["tags"][0], "status");
        }
        assert_eq!(
            paths[routes::v1::STATUS_BLOCK_HEIGHT_ROUTE]["get"]["summary"],
            "Get block height"
        );
        // The extension is still there alongside the base.
        assert!(paths.contains_key(routes::v1::STATUS_KEYS_ROUTE));
    }

    /// The base node routes come from `hotshot-query-service` now, so the mount has to put them
    /// (and their documentation) back on the paths this crate used to declare itself.
    #[tokio::test]
    async fn v1_openapi_spec_documents_mounted_node_base() {
        let router = create_router_v1(MockState, mock_hqs_base());
        let req = Request::builder()
            .uri(routes::v1::OPENAPI_SPEC_ROUTE)
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
        let spec: serde_json::Value =
            serde_json::from_str(&body_string(resp).await).expect("valid JSON");
        let paths = spec["paths"].as_object().expect("spec has paths");

        for route in ["/v1/node/block-height", "/v1/node/limits"] {
            let item = paths
                .get(route)
                .unwrap_or_else(|| panic!("{route} missing from spec: {:?}", paths.keys()));
            assert_eq!(item["get"]["tags"][0], "node");
        }
        assert_eq!(
            paths["/v1/node/block-height"]["get"]["summary"],
            "Get this node's block height"
        );
        // The extensions are still there alongside the base.
        assert!(paths.contains_key(routes::v1::NODE_STAKE_TABLE_CURRENT_ROUTE));
    }

    /// The explorer routes come from `hotshot-query-service` now, and this crate no longer serves
    /// any of its own, so the mount is all that puts them (and their documentation) in the spec.
    #[tokio::test]
    async fn v1_openapi_spec_documents_mounted_explorer_base() {
        let router = create_router_v1(MockState, mock_hqs_base());
        let req = Request::builder()
            .uri(routes::v1::OPENAPI_SPEC_ROUTE)
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
        let spec: serde_json::Value =
            serde_json::from_str(&body_string(resp).await).expect("valid JSON");
        let paths = spec["paths"].as_object().expect("spec has paths");

        for route in [
            "/v1/explorer/explorer-summary",
            "/v1/explorer/block/{height}",
        ] {
            let item = paths
                .get(route)
                .unwrap_or_else(|| panic!("{route} missing from spec: {:?}", paths.keys()));
            assert_eq!(item["get"]["tags"][0], "explorer");
        }
        assert_eq!(
            paths["/v1/explorer/explorer-summary"]["get"]["summary"],
            "Get the explorer summary"
        );
    }

    /// The merklized-state routes come from `hotshot-query-service`, mounted once per tree, and
    /// this crate serves only `fee-balance/latest` of its own. Two mounts of the same router must
    /// document independently: one operation per prefix, tagged by its module, each summary naming
    /// its own tree.
    #[tokio::test]
    async fn v1_openapi_spec_documents_mounted_merklized_state_bases() {
        let router = create_router_v1(MockState, mock_hqs_base());
        let req = Request::builder()
            .uri(routes::v1::OPENAPI_SPEC_ROUTE)
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = tower::ServiceExt::oneshot(router, req).await.unwrap();
        let spec: serde_json::Value =
            serde_json::from_str(&body_string(resp).await).expect("valid JSON");
        let paths = spec["paths"].as_object().expect("spec has paths");

        for (prefix, tree) in [
            (routes::v1::BLOCK_STATE_PREFIX, "block_merkle_tree_bigint"),
            (routes::v1::REWARD_STATE_V2_PREFIX, "reward_merkle_tree_v2"),
        ] {
            let module = prefix.strip_prefix("/v1/").unwrap();
            for route in ["/{height}/{key}", "/block-height"] {
                let route = format!("{prefix}{route}");
                let item = paths
                    .get(&route)
                    .unwrap_or_else(|| panic!("{route} missing from spec: {:?}", paths.keys()));
                assert_eq!(item["get"]["tags"][0], module, "{route}");
                assert!(item["get"]["summary"].is_string(), "{route} has no summary");
            }
            let summary = paths[&format!("{prefix}/{{height}}/{{key}}")]["get"]["summary"]
                .as_str()
                .unwrap();
            assert!(summary.contains(tree), "{prefix}: {summary}");
        }
        assert!(
            paths.contains_key(routes::v1::FEE_STATE_BALANCE_LATEST_ROUTE),
            "the fee-state extension must still be served alongside the mounted base"
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
        // The status-only mode's query-service router carries the status base and nothing else: it
        // has no availability data source.
        let api_router = ApiRouter::new()
            .nest(routes::v1::STATUS_PREFIX, mock_status_base())
            .merge(router_status(MockState))
            .merge(router_state_signature(MockState));
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
        let router = create_router_v1(MockState, mock_hqs_base());
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
        let key_path = &paths[&format!("{}/{{height}}/{{key}}", routes::v1::BLOCK_STATE_PREFIX)]
            ["get"]["parameters"];
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
}
