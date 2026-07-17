//! Axum HTTP/JSON API handlers

pub mod routes;

use std::{collections::BTreeMap, sync::Arc};

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
use futures::{StreamExt, stream::BoxStream};
use http_client::healthcheck::{AppHealth, HealthStatus};
use schemars::transform::Transform;
use serde::Serialize;
use serialization_api::v2::{
    GetIncorrectEncodingProofRequest, GetNamespaceProofRequest, GetRewardAccountProofRequest,
    GetRewardBalanceRequest, GetRewardBalancesRequest, GetRewardClaimInputRequest,
    GetRewardMerkleTreeRequest, GetStakeTableRequest, GetStateCertificateRequest,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
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

/// Lifetime slots for streaming sockets; tower's permit is released at the 101 upgrade.
#[derive(Clone)]
pub(crate) struct StreamLimit(pub(crate) Arc<Semaphore>);

/// Websocket upgrades skip the request slot (the socket takes a lifetime slot in its handler);
/// everything else holds a slot for the request duration.
pub(crate) async fn limit_plain_requests(
    Extension(StreamLimit(semaphore)): Extension<StreamLimit>,
    req: Request,
    next: axum::middleware::Next,
) -> Response {
    let is_upgrade = req
        .headers()
        .get(header::UPGRADE)
        .is_some_and(|v| v.as_bytes().eq_ignore_ascii_case(b"websocket"));
    if is_upgrade {
        return next.run(req).await;
    }
    match semaphore.try_acquire_owned() {
        Ok(_permit) => next.run(req).await,
        Err(_) => StatusCode::TOO_MANY_REQUESTS.into_response(),
    }
}

fn acquire_stream_permit(
    limit: Option<Extension<StreamLimit>>,
) -> Result<Option<OwnedSemaphorePermit>, StatusCode> {
    match limit {
        None => Ok(None),
        Some(Extension(StreamLimit(semaphore))) => match semaphore.try_acquire_owned() {
            Ok(permit) => Ok(Some(permit)),
            Err(_) => Err(StatusCode::TOO_MANY_REQUESTS),
        },
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
            return;
        }
    }
    // Close handshake, like tide-disco's socket handler. Without it, dropping the socket resets
    // the connection and clients see an error instead of end-of-stream — the finite v0 streams
    // rely on a clean close to signal completion.
    let _ = socket.send(Message::Close(None)).await;
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
    let router_v2 = create_router_v2(state);

    with_top_level_routes(router_v2.merge(router_v1))
}

/// Add the routes that every mode serves regardless of which API modules are enabled:
/// `/`, `/healthcheck`, and `/version`.
pub(crate) fn with_top_level_routes(router: Router) -> Router {
    router
        .route("/", get(redirect_to_docs))
        .route("/healthcheck", get(healthcheck))
        .route("/v1/{module}/healthcheck", get(module_healthcheck))
        .route("/version", get(version))
}

/// Top-level healthcheck, matching tide-disco's app-level `AppHealth` response for multi-module
/// apps, in JSON or vbs binary depending on `Accept`.
///
/// Tide populated `modules` with each module's versioned health status; the axum modules don't
/// report individual health, so it stays empty.
async fn healthcheck(headers: HeaderMap) -> Result<Response, ApiError> {
    encode_response(
        &headers,
        AppHealth {
            status: HealthStatus::Available,
            modules: BTreeMap::new(),
        },
    )
}

/// Module-level healthcheck response, matching tide-disco's per-module `/healthcheck`: a bare
/// [`HealthStatus`], in JSON or vbs binary depending on `Accept`. Exported for the standalone
/// axum servers (submit-transactions, nasty-client, dev-node) that tide served as singleton apps.
pub fn healthcheck_response(headers: &HeaderMap) -> Response {
    match encode_response(headers, HealthStatus::Available) {
        Ok(resp) => resp,
        Err(err) => err.into_response(),
    }
}

/// `/v1/{module}/healthcheck`, reached by legacy clients via the `/{module}/healthcheck` rewrite.
///
/// Divergence from tide-disco: matches any `{module}` string, so unregistered module names report
/// healthy instead of 404. Constraining it to the registered set would have to track which
/// modules each serve mode mounts; not worth it for a liveness probe.
async fn module_healthcheck(headers: HeaderMap) -> Response {
    healthcheck_response(&headers)
}

/// Tide-disco-compatible version response. Tide emits the binary's clap version; we emit the
/// crate version so `http_client::Client::connect` and similar polling helpers succeed.
async fn version() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

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

    let get_reward_state_height = |State(state): State<S>| async move {
        state
            .get_reward_state_height()
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_reward_state_v2_height = |State(state): State<S>| async move {
        state
            .get_reward_state_v2_height()
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

    // Merklized-state `get_path` handlers, inherited by both reward mounts from
    // `hotshot-query-service`'s base `state.toml` routes (mirrors router_block_state /
    // router_fee_state below).
    let get_reward_state_path_v1_by_height =
        |State(state): State<S>, Path((height, key)): Path<(u64, String)>| async move {
            <S as v1::RewardApi>::get_reward_state_path_v1(
                &state,
                v1::Snapshot::Height(height),
                key,
            )
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
        };

    let get_reward_state_path_v1_by_commit =
        |State(state): State<S>, Path((commit, key)): Path<(String, String)>| async move {
            <S as v1::RewardApi>::get_reward_state_path_v1(
                &state,
                v1::Snapshot::Commit(commit),
                key,
            )
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
        };

    let get_reward_state_path_v2_by_height =
        |State(state): State<S>, Path((height, key)): Path<(u64, String)>| async move {
            <S as v1::RewardApi>::get_reward_state_path_v2(
                &state,
                v1::Snapshot::Height(height),
                key,
            )
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
        };

    let get_reward_state_path_v2_by_commit =
        |State(state): State<S>, Path((commit, key)): Path<(String, String)>| async move {
            <S as v1::RewardApi>::get_reward_state_path_v2(
                &state,
                v1::Snapshot::Commit(commit),
                key,
            )
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
            routes::v1::REWARD_STATE_HEIGHT_ROUTE,
            get_with(get_reward_state_height, |op| {
                op.summary("Get reward-state block height").description("Latest block height for which the merklized reward state (V1) is available.")
            }),
        )
        .api_route(
            routes::v1::REWARD_STATE_V2_HEIGHT_ROUTE,
            get_with(get_reward_state_v2_height, |op| {
                op.summary("Get reward-state-v2 block height").description("Latest block height for which the merklized reward state (V2) is available.")
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
        .api_route(
            routes::v1::REWARD_STATE_PATH_BY_HEIGHT_ROUTE,
            get_with(get_reward_state_path_v1_by_height, |op| {
                op.summary("Get reward-state Merkle path by height").description("Retrieve the Merkle path for the membership proof of a leaf in the reward-state (V1) tree, by block height and key.")
            }),
        )
        .api_route(
            routes::v1::REWARD_STATE_PATH_BY_COMMIT_ROUTE,
            get_with(get_reward_state_path_v1_by_commit, |op| {
                op.summary("Get reward-state Merkle path by commitment").description("Retrieve the Merkle path for the membership proof of a leaf in the reward-state (V1) tree, by tree commitment and key.")
            }),
        )
        .api_route(
            routes::v1::REWARD_STATE_V2_PATH_BY_HEIGHT_ROUTE,
            get_with(get_reward_state_path_v2_by_height, |op| {
                op.summary("Get reward-state-v2 Merkle path by height").description("Retrieve the Merkle path for the membership proof of a leaf in the reward-state-v2 tree, by block height and key.")
            }),
        )
        .api_route(
            routes::v1::REWARD_STATE_V2_PATH_BY_COMMIT_ROUTE,
            get_with(get_reward_state_path_v2_by_commit, |op| {
                op.summary("Get reward-state-v2 Merkle path by commitment").description("Retrieve the Merkle path for the membership proof of a leaf in the reward-state-v2 tree, by tree commitment and key.")
            }),
        )
        .with_state(state)
}

pub(crate) fn router_availability<S>(state: S) -> ApiRouter
where
    S: v1::AvailabilityApi + v1::HotShotAvailabilityApi + Clone + Send + Sync + 'static,
{
    // Availability API handlers
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
        <S as v1::AvailabilityApi>::get_state_cert(&state, epoch)
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

    // HotShot availability API handlers
    let get_leaf_by_height = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_leaf(v1::LeafId::Height(height))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_leaf_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_leaf(v1::LeafId::Hash(hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_leaf_range = |State(state): State<S>, Path((from, until)): Path<(usize, usize)>| async move {
        state
            .get_leaf_range(from, until)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_header_by_height = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_header(v1::BlockId::Height(height))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_header_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_header(v1::BlockId::Hash(hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_header_by_payload_hash = |State(state): State<S>, Path(payload_hash): Path<String>| async move {
        state
            .get_header(v1::BlockId::PayloadHash(payload_hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_header_range = |State(state): State<S>, Path((from, until)): Path<(usize, usize)>| async move {
        state
            .get_header_range(from, until)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_block_by_height = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_block(v1::BlockId::Height(height))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_block_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_block(v1::BlockId::Hash(hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_block_by_payload_hash = |State(state): State<S>, Path(payload_hash): Path<String>| async move {
        state
            .get_block(v1::BlockId::PayloadHash(payload_hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_block_range = |State(state): State<S>, Path((from, until)): Path<(usize, usize)>| async move {
        state
            .get_block_range(from, until)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_payload_by_height = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_payload(v1::PayloadId::Height(height))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_payload_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_payload(v1::PayloadId::Hash(hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_payload_by_block_hash = |State(state): State<S>, Path(block_hash): Path<String>| async move {
        state
            .get_payload(v1::PayloadId::BlockHash(block_hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_payload_range = |State(state): State<S>, Path((from, until)): Path<(usize, usize)>| async move {
        state
            .get_payload_range(from, until)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_vid_common_by_height = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_vid_common(v1::BlockId::Height(height))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_vid_common_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_vid_common(v1::BlockId::Hash(hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_vid_common_by_payload_hash =
        |State(state): State<S>, Path(payload_hash): Path<String>| async move {
            state
                .get_vid_common(v1::BlockId::PayloadHash(payload_hash))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let get_vid_common_range =
        |State(state): State<S>, Path((from, until)): Path<(usize, usize)>| async move {
            state
                .get_vid_common_range(from, until)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let get_transaction_by_position =
        |State(state): State<S>, Path((height, index)): Path<(u64, u64)>| async move {
            state
                .get_transaction_by_position(height, index)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let get_transaction_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_transaction_by_hash(hash)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_transaction_proof_by_position =
        |State(state): State<S>, Path((height, index)): Path<(u64, u64)>| async move {
            state
                .get_transaction_proof_by_position(height, index)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let get_transaction_proof_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_transaction_proof_by_hash(hash)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_block_summary_by_height = |State(state): State<S>, Path(height): Path<usize>| async move {
        state
            .get_block_summary(height)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_block_summary_range =
        |State(state): State<S>, Path((from, until)): Path<(usize, usize)>| async move {
            state
                .get_block_summary_range(from, until)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let get_limits = |State(state): State<S>| async move {
        state
            .get_limits()
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let get_cert2 = |State(state): State<S>, Path(height): Path<u64>| async move {
        <S as v1::HotShotAvailabilityApi>::get_cert2(&state, height)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    // WebSocket streaming handlers
    let stream_leaves = |ws: WebSocketUpgrade,
                         State(state): State<S>,
                         headers: HeaderMap,
                         Path(height): Path<usize>,
                         limit: Option<Extension<StreamLimit>>| async move {
        let format = ws_format(&headers);
        let permit = match acquire_stream_permit(limit) {
            Ok(permit) => permit,
            Err(status) => return status.into_response(),
        };
        ws.on_upgrade(move |socket| async move {
            let _permit = permit;
            match state.stream_leaves(height).await {
                Ok(stream) => drive_ws_stream(socket, stream, format).await,
                Err(e) => tracing::warn!("stream_leaves: {e}"),
            }
        })
    };

    let stream_headers = |ws: WebSocketUpgrade,
                          State(state): State<S>,
                          headers: HeaderMap,
                          Path(height): Path<usize>,
                          limit: Option<Extension<StreamLimit>>| async move {
        let format = ws_format(&headers);
        let permit = match acquire_stream_permit(limit) {
            Ok(permit) => permit,
            Err(status) => return status.into_response(),
        };
        ws.on_upgrade(move |socket| async move {
            let _permit = permit;
            match state.stream_headers(height).await {
                Ok(stream) => drive_ws_stream(socket, stream, format).await,
                Err(e) => tracing::warn!("stream_headers: {e}"),
            }
        })
    };

    let stream_blocks = |ws: WebSocketUpgrade,
                         State(state): State<S>,
                         headers: HeaderMap,
                         Path(height): Path<usize>,
                         limit: Option<Extension<StreamLimit>>| async move {
        let format = ws_format(&headers);
        let permit = match acquire_stream_permit(limit) {
            Ok(permit) => permit,
            Err(status) => return status.into_response(),
        };
        ws.on_upgrade(move |socket| async move {
            let _permit = permit;
            match state.stream_blocks(height).await {
                Ok(stream) => drive_ws_stream(socket, stream, format).await,
                Err(e) => tracing::warn!("stream_blocks: {e}"),
            }
        })
    };

    let stream_payloads = |ws: WebSocketUpgrade,
                           State(state): State<S>,
                           headers: HeaderMap,
                           Path(height): Path<usize>,
                           limit: Option<Extension<StreamLimit>>| async move {
        let format = ws_format(&headers);
        let permit = match acquire_stream_permit(limit) {
            Ok(permit) => permit,
            Err(status) => return status.into_response(),
        };
        ws.on_upgrade(move |socket| async move {
            let _permit = permit;
            match state.stream_payloads(height).await {
                Ok(stream) => drive_ws_stream(socket, stream, format).await,
                Err(e) => tracing::warn!("stream_payloads: {e}"),
            }
        })
    };

    let stream_vid_common = |ws: WebSocketUpgrade,
                             State(state): State<S>,
                             headers: HeaderMap,
                             Path(height): Path<usize>,
                             limit: Option<Extension<StreamLimit>>| async move {
        let format = ws_format(&headers);
        let permit = match acquire_stream_permit(limit) {
            Ok(permit) => permit,
            Err(status) => return status.into_response(),
        };
        ws.on_upgrade(move |socket| async move {
            let _permit = permit;
            match state.stream_vid_common(height).await {
                Ok(stream) => drive_ws_stream(socket, stream, format).await,
                Err(e) => tracing::warn!("stream_vid_common: {e}"),
            }
        })
    };

    let stream_transactions =
        |ws: WebSocketUpgrade,
         State(state): State<S>,
         headers: HeaderMap,
         Path(height): Path<usize>,
         limit: Option<Extension<StreamLimit>>| async move {
            let format = ws_format(&headers);
            let permit = match acquire_stream_permit(limit) {
                Ok(permit) => permit,
                Err(status) => return status.into_response(),
            };
            ws.on_upgrade(move |socket| async move {
                let _permit = permit;
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
         Path((height, namespace)): Path<(usize, u32)>,
         limit: Option<Extension<StreamLimit>>| async move {
            let format = ws_format(&headers);
            let permit = match acquire_stream_permit(limit) {
                Ok(permit) => permit,
                Err(status) => return status.into_response(),
            };
            ws.on_upgrade(move |socket| async move {
                let _permit = permit;
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
         Path((height, namespace)): Path<(usize, u32)>,
         limit: Option<Extension<StreamLimit>>| async move {
            let format = ws_format(&headers);
            let permit = match acquire_stream_permit(limit) {
                Ok(permit) => permit,
                Err(status) => return status.into_response(),
            };
            ws.on_upgrade(move |socket| async move {
                let _permit = permit;
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
            routes::v1::LEAF_BY_HEIGHT_ROUTE,
            get_with(get_leaf_by_height, |op| {
                op.summary("Get leaf").description(
                    "Get a leaf by its position in the ledger (0 is genesis) or its hash.",
                )
            }),
        )
        .api_route(
            routes::v1::LEAF_BY_HASH_ROUTE,
            get_with(get_leaf_by_hash, |op| {
                op.summary("Get leaf").description(
                    "Get a leaf by its position in the ledger (0 is genesis) or its hash.",
                )
            }),
        )
        .api_route(
            routes::v1::LEAF_RANGE_ROUTE,
            get_with(get_leaf_range, |op| {
                op.summary("Get leaves in range").description(
                    "Get leaves by position in the ledger, from the given `from` up to `until`.",
                )
            }),
        )
        .api_route(
            routes::v1::HEADER_BY_HEIGHT_ROUTE,
            get_with(get_header_by_height, |op| {
                op.summary("Get header").description(
                    "Get a header by its position in the ledger (0 is genesis) or its hash.",
                )
            }),
        )
        .api_route(
            routes::v1::HEADER_BY_HASH_ROUTE,
            get_with(get_header_by_hash, |op| {
                op.summary("Get header").description(
                    "Get a header by its position in the ledger (0 is genesis) or its hash.",
                )
            }),
        )
        .api_route(
            routes::v1::HEADER_BY_PAYLOAD_HASH_ROUTE,
            get_with(get_header_by_payload_hash, |op| {
                op.summary("Get header").description(
                    "Get a header by its position in the ledger (0 is genesis) or its hash.",
                )
            }),
        )
        .api_route(
            routes::v1::HEADER_RANGE_ROUTE,
            get_with(get_header_range, |op| {
                op.summary("Get headers in range").description(
                    "Get headers by position in the ledger, from the given `from` up to `until`.",
                )
            }),
        )
        .api_route(
            routes::v1::BLOCK_BY_HEIGHT_ROUTE,
            get_with(get_block_by_height, |op| {
                op.summary("Get block").description(
                    "Get a block (header, payload, hash, size) by its position in the ledger or \
                     its hash.",
                )
            }),
        )
        .api_route(
            routes::v1::BLOCK_BY_HASH_ROUTE,
            get_with(get_block_by_hash, |op| {
                op.summary("Get block").description(
                    "Get a block (header, payload, hash, size) by its position in the ledger or \
                     its hash.",
                )
            }),
        )
        .api_route(
            routes::v1::BLOCK_BY_PAYLOAD_HASH_ROUTE,
            get_with(get_block_by_payload_hash, |op| {
                op.summary("Get block").description(
                    "Get a block (header, payload, hash, size) by its position in the ledger or \
                     its hash.",
                )
            }),
        )
        .api_route(
            routes::v1::BLOCK_RANGE_ROUTE,
            get_with(get_block_range, |op| {
                op.summary("Get blocks in range").description(
                    "Get blocks by position in the ledger, from the given `from` up to `until`.",
                )
            }),
        )
        .api_route(
            routes::v1::PAYLOAD_BY_HEIGHT_ROUTE,
            get_with(get_payload_by_height, |op| {
                op.summary("Get payload").description(
                    "Get the payload of a block by its position in the ledger or its hash.",
                )
            }),
        )
        .api_route(
            routes::v1::PAYLOAD_BY_HASH_ROUTE,
            get_with(get_payload_by_hash, |op| {
                op.summary("Get payload").description(
                    "Get the payload of a block by its position in the ledger or its hash.",
                )
            }),
        )
        .api_route(
            routes::v1::PAYLOAD_BY_BLOCK_HASH_ROUTE,
            get_with(get_payload_by_block_hash, |op| {
                op.summary("Get payload").description(
                    "Get the payload of a block by its position in the ledger or its hash.",
                )
            }),
        )
        .api_route(
            routes::v1::PAYLOAD_RANGE_ROUTE,
            get_with(get_payload_range, |op| {
                op.summary("Get payloads in range").description(
                    "Get payloads by block position, from the given `from` up to `until`.",
                )
            }),
        )
        .api_route(
            routes::v1::VID_COMMON_BY_HEIGHT_ROUTE,
            get_with(get_vid_common_by_height, |op| {
                op.summary("Get VID common data").description(
                    "Get common VID data for a block; data shared by all storage nodes, not a VID \
                     share.",
                )
            }),
        )
        .api_route(
            routes::v1::VID_COMMON_BY_HASH_ROUTE,
            get_with(get_vid_common_by_hash, |op| {
                op.summary("Get VID common data").description(
                    "Get common VID data for a block; data shared by all storage nodes, not a VID \
                     share.",
                )
            }),
        )
        .api_route(
            routes::v1::VID_COMMON_BY_PAYLOAD_HASH_ROUTE,
            get_with(get_vid_common_by_payload_hash, |op| {
                op.summary("Get VID common data").description(
                    "Get common VID data for a block; data shared by all storage nodes, not a VID \
                     share.",
                )
            }),
        )
        .api_route(
            routes::v1::VID_COMMON_RANGE_ROUTE,
            get_with(get_vid_common_range, |op| {
                op.summary("Get VID common data in range").description(
                    "Get VID common objects by block position, from the given `from` up to \
                     `until`.",
                )
            }),
        )
        .api_route(
            routes::v1::TRANSACTION_BY_POSITION_NOPROOF_ROUTE,
            get_with(get_transaction_by_position, |op| {
                op.summary("Get transaction (no proof)").description(
                    "Get a transaction by its index in a block or by its hash, without an \
                     inclusion proof.",
                )
            }),
        )
        .api_route(
            routes::v1::TRANSACTION_BY_HASH_NOPROOF_ROUTE,
            get_with(get_transaction_by_hash, |op| {
                op.summary("Get transaction (no proof)").description(
                    "Get a transaction by its index in a block or by its hash, without an \
                     inclusion proof.",
                )
            }),
        )
        .api_route(
            routes::v1::TRANSACTION_PROOF_BY_POSITION_ROUTE,
            get_with(get_transaction_proof_by_position, |op| {
                op.summary("Get transaction with inclusion proof")
                    .description(
                        "Get a transaction by its index in a block or by its hash, along with an \
                         application-defined inclusion proof.",
                    )
            }),
        )
        .api_route(
            routes::v1::TRANSACTION_PROOF_BY_HASH_ROUTE,
            get_with(get_transaction_proof_by_hash, |op| {
                op.summary("Get transaction with inclusion proof")
                    .description(
                        "Get a transaction by its index in a block or by its hash, along with an \
                         application-defined inclusion proof.",
                    )
            }),
        )
        .api_route(
            routes::v1::TRANSACTION_BY_POSITION_ROUTE,
            get_with(get_transaction_proof_by_position, |op| {
                op.summary("Get transaction with inclusion proof")
                    .description(
                        "Get a transaction by its index in a block or by its hash, along with an \
                         application-defined inclusion proof.",
                    )
            }),
        )
        .api_route(
            routes::v1::TRANSACTION_BY_HASH_ROUTE,
            get_with(get_transaction_proof_by_hash, |op| {
                op.summary("Get transaction with inclusion proof")
                    .description(
                        "Get a transaction by its index in a block or by its hash, along with an \
                         application-defined inclusion proof.",
                    )
            }),
        )
        .api_route(
            routes::v1::BLOCK_SUMMARY_BY_HEIGHT_ROUTE,
            get_with(get_block_summary_by_height, |op| {
                op.summary("Get block summary").description(
                    "Get the block summary for a block based on its position in the ledger.",
                )
            }),
        )
        .api_route(
            routes::v1::BLOCK_SUMMARY_RANGE_ROUTE,
            get_with(get_block_summary_range, |op| {
                op.summary("Get block summaries in range").description(
                    "Get block summaries by position, from the given `from` up to `until`.",
                )
            }),
        )
        .api_route(
            routes::v1::LIMITS_ROUTE,
            get_with(get_limits, |op| {
                op.summary("Get availability limits").description(
                    "Get implementation-defined limits restricting availability range queries \
                     (small/large object range limits).",
                )
            }),
        )
        .api_route(
            routes::v1::CERT2_BY_HEIGHT_ROUTE,
            get_with(get_cert2, |op| {
                op.summary("Get finality certificate").description(
                    "Get the finality certificate (Certificate2) at the given block height.",
                )
            }),
        )
        .api_route(
            routes::v1::STREAM_LEAVES_ROUTE,
            get_with(stream_leaves, |op| {
                op.summary("Stream leaves (websocket)").description(
                    "Websocket endpoint: subscribe to a stream of leaves in sequence order, \
                     starting at the given height.",
                )
            }),
        )
        .api_route(
            routes::v1::STREAM_HEADERS_ROUTE,
            get_with(stream_headers, |op| {
                op.summary("Stream headers (websocket)").description(
                    "Websocket endpoint: subscribe to a stream of headers in sequence order, \
                     starting at the given height.",
                )
            }),
        )
        .api_route(
            routes::v1::STREAM_BLOCKS_ROUTE,
            get_with(stream_blocks, |op| {
                op.summary("Stream blocks (websocket)").description(
                    "Websocket endpoint: subscribe to a stream of blocks in sequence order, \
                     starting at the given height.",
                )
            }),
        )
        .api_route(
            routes::v1::STREAM_PAYLOADS_ROUTE,
            get_with(stream_payloads, |op| {
                op.summary("Stream payloads (websocket)").description(
                    "Websocket endpoint: subscribe to a stream of block payloads in sequence \
                     order, starting at the given height.",
                )
            }),
        )
        .api_route(
            routes::v1::STREAM_VID_COMMON_ROUTE,
            get_with(stream_vid_common, |op| {
                op.summary("Stream VID common data (websocket)")
                    .description(
                        "Websocket endpoint: subscribe to a stream of VID common data in sequence \
                         order, starting at the given height.",
                    )
            }),
        )
        .api_route(
            routes::v1::STREAM_TRANSACTIONS_ROUTE,
            get_with(stream_transactions, |op| {
                op.summary("Stream transactions (websocket)").description(
                    "Websocket endpoint: subscribe to a stream of all transactions starting at \
                     the given height.",
                )
            }),
        )
        .api_route(
            routes::v1::STREAM_TRANSACTIONS_NS_ROUTE,
            get_with(stream_transactions_ns, |op| {
                op.summary("Stream namespace transactions (websocket)")
                    .description(
                        "Websocket endpoint: subscribe to a stream of transactions in one \
                         namespace, starting at the given height.",
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

pub(crate) fn router_block_state<S>(state: S) -> ApiRouter
where
    S: v1::BlockStateApi + Clone + Send + Sync + 'static,
{
    let get_block_state_height = |State(state): State<S>| async move {
        <S as v1::BlockStateApi>::get_block_state_height(&state)
            .await
            .map(ApiJson)
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
            .map(ApiJson)
            .map_err(classify_availability_error)
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
            .map(ApiJson)
            .map_err(classify_availability_error)
        };

    ApiRouter::new()
        .api_route(
            routes::v1::BLOCK_STATE_HEIGHT_ROUTE,
            get_with(get_block_state_height, |op| {
                op.summary("Get block-state height").description(
                    "Latest block height for which the merklized blocks-Merkle-tree state is \
                     available.",
                )
            }),
        )
        .api_route(
            routes::v1::BLOCK_STATE_PATH_BY_COMMIT_ROUTE,
            get_with(get_block_state_path_by_commit, |op| {
                op.summary("Get block-state Merkle path by commitment")
                    .description(
                        "Retrieve the Merkle path for a leaf in the blocks Merkle tree, by tree \
                         commitment and key.",
                    )
            }),
        )
        .api_route(
            routes::v1::BLOCK_STATE_PATH_BY_HEIGHT_ROUTE,
            get_with(get_block_state_path_by_height, |op| {
                op.summary("Get block-state Merkle path by height")
                    .description(
                        "Retrieve the Merkle path for a leaf in the blocks Merkle tree, by block \
                         height and key.",
                    )
            }),
        )
        .with_state(state)
}

pub(crate) fn router_fee_state<S>(state: S) -> ApiRouter
where
    S: v1::FeeStateApi + Clone + Send + Sync + 'static,
{
    let get_fee_state_height = |State(state): State<S>| async move {
        <S as v1::FeeStateApi>::get_fee_state_height(&state)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_fee_balance_latest = |State(state): State<S>, Path(address): Path<String>| async move {
        state
            .get_fee_balance_latest(address)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_fee_state_path_by_commit =
        |State(state): State<S>, Path((commit, key)): Path<(String, String)>| async move {
            <S as v1::FeeStateApi>::get_fee_state_path(&state, v1::Snapshot::Commit(commit), key)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    // Merklized state handlers: fee-state
    let get_fee_state_path_by_height =
        |State(state): State<S>, Path((height, key)): Path<(u64, String)>| async move {
            <S as v1::FeeStateApi>::get_fee_state_path(&state, v1::Snapshot::Height(height), key)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    ApiRouter::new()
        .api_route(
            routes::v1::FEE_STATE_HEIGHT_ROUTE,
            get_with(get_fee_state_height, |op| {
                op.summary("Get fee-state height").description(
                    "Latest block height for which the merklized fee state is available.",
                )
            }),
        )
        .api_route(
            routes::v1::FEE_STATE_BALANCE_LATEST_ROUTE,
            get_with(get_fee_balance_latest, |op| {
                op.summary("Get latest fee balance").description(
                    "Get the latest fee account balance for an address from the fee Merkle tree.",
                )
            }),
        )
        .api_route(
            routes::v1::FEE_STATE_PATH_BY_COMMIT_ROUTE,
            get_with(get_fee_state_path_by_commit, |op| {
                op.summary("Get fee-state Merkle path by commitment")
                    .description(
                        "Retrieve the Merkle path for a leaf in the fee state tree, by tree \
                         commitment and key.",
                    )
            }),
        )
        .api_route(
            routes::v1::FEE_STATE_PATH_BY_HEIGHT_ROUTE,
            get_with(get_fee_state_path_by_height, |op| {
                op.summary("Get fee-state Merkle path by height")
                    .description(
                        "Retrieve the Merkle path for a leaf in the fee state tree, by block \
                         height and key.",
                    )
            }),
        )
        .with_state(state)
}

pub(crate) fn router_status<S>(state: S) -> ApiRouter
where
    S: v1::StatusApi + Clone + Send + Sync + 'static,
{
    let status_block_height = |State(state): State<S>| async move {
        <S as v1::StatusApi>::block_height(&state)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let status_success_rate = |State(state): State<S>| async move {
        <S as v1::StatusApi>::success_rate(&state)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let status_time_since_last_decide = |State(state): State<S>| async move {
        <S as v1::StatusApi>::time_since_last_decide(&state)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

    let status_metrics = |State(state): State<S>| async move {
        match <S as v1::StatusApi>::metrics(&state).await {
            Ok(text) => (
                [(
                    axum::http::header::CONTENT_TYPE,
                    "text/plain; charset=utf-8",
                )],
                text,
            )
                .into_response(),
            Err(e) => ApiError::Internal(e).into_response(),
        }
    };

    ApiRouter::new()
        .api_route(
            routes::v1::STATUS_BLOCK_HEIGHT_ROUTE,
            get_with(status_block_height, |op| {
                op.summary("Get latest committed block height")
                    .description("Get the height of the latest committed block.")
            }),
        )
        .api_route(
            routes::v1::STATUS_SUCCESS_RATE_ROUTE,
            get_with(status_success_rate, |op| {
                op.summary("Get view success rate")
                    .description("Get the fraction of views which resulted in a committed block.")
            }),
        )
        .api_route(
            routes::v1::STATUS_TIME_SINCE_LAST_DECIDE_ROUTE,
            get_with(status_time_since_last_decide, |op| {
                op.summary("Get time since last decide")
                    .description("Get the time elapsed in seconds since the last decided view.")
            }),
        )
        .api_route(
            routes::v1::STATUS_METRICS_ROUTE,
            get_with(status_metrics, |op| {
                op.summary("Get Prometheus metrics")
                    .description("Prometheus endpoint exposing consensus-related metrics.")
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

pub(crate) fn router_node<S>(state: S) -> ApiRouter
where
    S: v1::NodeApi + Clone + Send + Sync + 'static,
{
    let node_block_height = |State(state): State<S>| async move {
        <S as v1::NodeApi>::block_height(&state)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let node_count_txs = |State(state): State<S>| async move {
        state
            .count_transactions(None, None, None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let node_count_txs_ns = |State(state): State<S>, Path(namespace): Path<u64>| async move {
        state
            .count_transactions(None, None, Some(namespace))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let node_count_txs_ns_to = |State(state): State<S>, Path((namespace, to)): Path<(u64, u64)>| async move {
        state
            .count_transactions(None, Some(to), Some(namespace))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let node_count_txs_ns_from_to =
        |State(state): State<S>, Path((namespace, from, to)): Path<(u64, u64, u64)>| async move {
            state
                .count_transactions(Some(from), Some(to), Some(namespace))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let node_count_txs_to = |State(state): State<S>, Path(to): Path<u64>| async move {
        state
            .count_transactions(None, Some(to), None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let node_count_txs_from_to = |State(state): State<S>, Path((from, to)): Path<(u64, u64)>| async move {
        state
            .count_transactions(Some(from), Some(to), None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let node_payload_size = |State(state): State<S>| async move {
        state
            .payload_size(None, None, None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let node_payload_size_ns = |State(state): State<S>, Path(namespace): Path<u64>| async move {
        state
            .payload_size(None, None, Some(namespace))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let node_payload_size_ns_to =
        |State(state): State<S>, Path((namespace, to)): Path<(u64, u64)>| async move {
            state
                .payload_size(None, Some(to), Some(namespace))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let node_payload_size_ns_from_to =
        |State(state): State<S>, Path((namespace, from, to)): Path<(u64, u64, u64)>| async move {
            state
                .payload_size(Some(from), Some(to), Some(namespace))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let node_payload_size_to = |State(state): State<S>, Path(to): Path<u64>| async move {
        state
            .payload_size(None, Some(to), None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let node_payload_size_from_to = |State(state): State<S>, Path((from, to)): Path<(u64, u64)>| async move {
        state
            .payload_size(Some(from), Some(to), None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let node_vid_share_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_vid_share(v1::VidShareId::Hash(hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let node_vid_share_by_payload_hash =
        |State(state): State<S>, Path(payload_hash): Path<String>| async move {
            state
                .get_vid_share(v1::VidShareId::PayloadHash(payload_hash))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let node_vid_share_by_height = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_vid_share(v1::VidShareId::Height(height))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let node_sync_status = |State(state): State<S>| async move {
        state
            .sync_status()
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let node_header_window_hash =
        |State(state): State<S>, Path((hash, end)): Path<(String, u64)>| async move {
            state
                .get_header_window(v1::HeaderWindowStart::Hash(hash), end)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let node_header_window_height =
        |State(state): State<S>, Path((height, end)): Path<(u64, u64)>| async move {
            state
                .get_header_window(v1::HeaderWindowStart::Height(height), end)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let node_header_window_time = |State(state): State<S>, Path((start, end)): Path<(u64, u64)>| async move {
        state
            .get_header_window(v1::HeaderWindowStart::Time(start), end)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let node_limits = |State(state): State<S>| async move {
        <S as v1::NodeApi>::limits(&state)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    };

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
            routes::v1::NODE_BLOCK_HEIGHT_ROUTE,
            get_with(node_block_height, |op| {
                op.summary("Get node's block height")
                    .description("The current height of the chain, as observed by this node.")
            }),
        )
        .api_route(
            routes::v1::NODE_TRANSACTIONS_COUNT_ROUTE,
            get_with(node_count_txs, |op| {
                op.summary("Count transactions").description(
                    "Get the number of transactions in the chain, optionally restricted by block \
                     range and/or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_TRANSACTIONS_COUNT_NS_ROUTE,
            get_with(node_count_txs_ns, |op| {
                op.summary("Count transactions").description(
                    "Get the number of transactions in the chain, optionally restricted by block \
                     range and/or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_TRANSACTIONS_COUNT_NS_TO_ROUTE,
            get_with(node_count_txs_ns_to, |op| {
                op.summary("Count transactions").description(
                    "Get the number of transactions in the chain, optionally restricted by block \
                     range and/or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_TRANSACTIONS_COUNT_NS_FROM_TO_ROUTE,
            get_with(node_count_txs_ns_from_to, |op| {
                op.summary("Count transactions").description(
                    "Get the number of transactions in the chain, optionally restricted by block \
                     range and/or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_TRANSACTIONS_COUNT_TO_ROUTE,
            get_with(node_count_txs_to, |op| {
                op.summary("Count transactions").description(
                    "Get the number of transactions in the chain, optionally restricted by block \
                     range and/or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_TRANSACTIONS_COUNT_FROM_TO_ROUTE,
            get_with(node_count_txs_from_to, |op| {
                op.summary("Count transactions").description(
                    "Get the number of transactions in the chain, optionally restricted by block \
                     range and/or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_PAYLOADS_SIZE_ROUTE,
            get_with(node_payload_size, |op| {
                op.summary("Get payload size").description(
                    "Get the cumulative size (bytes) of payload data in the chain, optionally \
                     restricted by block range and/or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_PAYLOADS_TOTAL_SIZE_ROUTE,
            get_with(node_payload_size, |op| {
                op.summary("Get payload size")
                    .description("Deprecated alias for payloads/size.")
            }),
        )
        .api_route(
            routes::v1::NODE_PAYLOADS_SIZE_NS_ROUTE,
            get_with(node_payload_size_ns, |op| {
                op.summary("Get payload size").description(
                    "Get the cumulative size (bytes) of payload data in the chain, optionally \
                     restricted by block range and/or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_PAYLOADS_SIZE_NS_TO_ROUTE,
            get_with(node_payload_size_ns_to, |op| {
                op.summary("Get payload size").description(
                    "Get the cumulative size (bytes) of payload data in the chain, optionally \
                     restricted by block range and/or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_PAYLOADS_SIZE_NS_FROM_TO_ROUTE,
            get_with(node_payload_size_ns_from_to, |op| {
                op.summary("Get payload size").description(
                    "Get the cumulative size (bytes) of payload data in the chain, optionally \
                     restricted by block range and/or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_PAYLOADS_SIZE_TO_ROUTE,
            get_with(node_payload_size_to, |op| {
                op.summary("Get payload size").description(
                    "Get the cumulative size (bytes) of payload data in the chain, optionally \
                     restricted by block range and/or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_PAYLOADS_SIZE_FROM_TO_ROUTE,
            get_with(node_payload_size_from_to, |op| {
                op.summary("Get payload size").description(
                    "Get the cumulative size (bytes) of payload data in the chain, optionally \
                     restricted by block range and/or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_VID_SHARE_BY_HASH_ROUTE,
            get_with(node_vid_share_by_hash, |op| {
                op.summary("Get this node's VID share").description(
                    "Get information needed to run the VID reconstruction protocol for a block: \
                     this node's VID share, if available.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_VID_SHARE_BY_PAYLOAD_HASH_ROUTE,
            get_with(node_vid_share_by_payload_hash, |op| {
                op.summary("Get this node's VID share").description(
                    "Get information needed to run the VID reconstruction protocol for a block: \
                     this node's VID share, if available.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_VID_SHARE_BY_HEIGHT_ROUTE,
            get_with(node_vid_share_by_height, |op| {
                op.summary("Get this node's VID share").description(
                    "Get information needed to run the VID reconstruction protocol for a block: \
                     this node's VID share, if available.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_SYNC_STATUS_ROUTE,
            get_with(node_sync_status, |op| {
                op.summary("Get node sync status").description(
                    "Get the node's progress syncing with the latest chain state \
                     (missing/present/pruned ranges for blocks, leaves, and VID common).",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_HEADER_WINDOW_HASH_ROUTE,
            get_with(node_header_window_hash, |op| {
                op.summary("Get header window").description(
                    "Get block headers whose timestamps fall in a time window, plus one header \
                     before and after to prove completeness.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_HEADER_WINDOW_HEIGHT_ROUTE,
            get_with(node_header_window_height, |op| {
                op.summary("Get header window").description(
                    "Get block headers whose timestamps fall in a time window, plus one header \
                     before and after to prove completeness.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_HEADER_WINDOW_TIME_ROUTE,
            get_with(node_header_window_time, |op| {
                op.summary("Get header window").description(
                    "Get block headers whose timestamps fall in a time window, plus one header \
                     before and after to prove completeness.",
                )
            }),
        )
        .api_route(
            routes::v1::NODE_LIMITS_ROUTE,
            get_with(node_limits, |op| {
                op.summary("Get node limits").description(
                    "Get implementation-defined limits restricting node API requests (e.g. \
                     header/window query size).",
                )
            }),
        )
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
        encode_response(&headers, tree)
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
        encode_response(&headers, tree)
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
        encode_response(&headers, hash)
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
        |State(state): State<S>,
         headers: HeaderMap,
         ws: WebSocketUpgrade,
         limit: Option<Extension<StreamLimit>>| async move {
            let format = ws_format(&headers);
            let permit = match acquire_stream_permit(limit) {
                Ok(permit) => permit,
                Err(status) => return status.into_response(),
            };
            match <S as v1::HotShotEventsApi>::events(&state).await {
                Ok(stream) => ws.on_upgrade(move |socket| async move {
                    let _permit = permit;
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

pub(crate) fn router_explorer<S>(state: S) -> ApiRouter
where
    S: v1::ExplorerApi + Clone + Send + Sync + 'static,
{
    // Explorer handlers
    let explorer_block_detail_by_height = |State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_block_detail(v1::BlockIdent::Height(height))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let explorer_block_detail_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_block_detail(v1::BlockIdent::Hash(hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let explorer_block_summaries_latest = |State(state): State<S>, Path(limit): Path<u64>| async move {
        state
            .get_block_summaries(v1::BlockIdent::Latest, limit)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let explorer_block_summaries_from =
        |State(state): State<S>, Path((from, limit)): Path<(u64, u64)>| async move {
            state
                .get_block_summaries(v1::BlockIdent::Height(from), limit)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let explorer_tx_detail_by_position =
        |State(state): State<S>, Path((height, offset)): Path<(u64, u64)>| async move {
            state
                .get_transaction_detail(v1::TxIdent::HeightAndOffset(height, offset))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let explorer_tx_detail_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_transaction_detail(v1::TxIdent::Hash(hash))
            .await
            .map(ApiJson)
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
                .map(ApiJson)
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
                .map(ApiJson)
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
                .map(ApiJson)
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
                .map(ApiJson)
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
                .map(ApiJson)
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
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let explorer_tx_summaries_latest = |State(state): State<S>, Path(limit): Path<u64>| async move {
        state
            .get_transaction_summaries(v1::TxIdent::Latest, limit, v1::TxSummaryFilter::None)
            .await
            .map(ApiJson)
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
                .map(ApiJson)
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
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let explorer_summary = |State(state): State<S>| async move {
        state
            .get_explorer_summary()
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let explorer_search = |State(state): State<S>, Path(query): Path<String>| async move {
        state
            .get_search_result(query)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    ApiRouter::new()
        .api_route(
            routes::v1::EXPLORER_BLOCK_DETAIL_BY_HEIGHT_ROUTE,
            get_with(explorer_block_detail_by_height, |op| {
                op.summary("Get block detail")
                    .description("Get details for a block identified by height or hash.")
            }),
        )
        .api_route(
            routes::v1::EXPLORER_BLOCK_DETAIL_BY_HASH_ROUTE,
            get_with(explorer_block_detail_by_hash, |op| {
                op.summary("Get block detail")
                    .description("Get details for a block identified by height or hash.")
            }),
        )
        .api_route(
            routes::v1::EXPLORER_BLOCK_SUMMARIES_LATEST_ROUTE,
            get_with(explorer_block_summaries_latest, |op| {
                op.summary("List block summaries").description(
                    "Retrieve up to `limit` block summaries, targeting the latest block or a \
                     block identified by height.",
                )
            }),
        )
        .api_route(
            routes::v1::EXPLORER_BLOCK_SUMMARIES_FROM_ROUTE,
            get_with(explorer_block_summaries_from, |op| {
                op.summary("List block summaries").description(
                    "Retrieve up to `limit` block summaries, targeting the latest block or a \
                     block identified by height.",
                )
            }),
        )
        .api_route(
            routes::v1::EXPLORER_TX_DETAIL_BY_POSITION_ROUTE,
            get_with(explorer_tx_detail_by_position, |op| {
                op.summary("Get transaction detail").description(
                    "Get details for a transaction identified by height and offset, or by hash.",
                )
            }),
        )
        .api_route(
            routes::v1::EXPLORER_TX_DETAIL_BY_HASH_ROUTE,
            get_with(explorer_tx_detail_by_hash, |op| {
                op.summary("Get transaction detail").description(
                    "Get details for a transaction identified by height and offset, or by hash.",
                )
            }),
        )
        .api_route(
            routes::v1::EXPLORER_TX_SUMMARIES_LATEST_BLOCK_ROUTE,
            get_with(explorer_tx_summaries_latest_block, |op| {
                op.summary("List transaction summaries").description(
                    "Retrieve up to `limit` transaction summaries, targeting the latest \
                     transaction, one identified by height/offset, or by hash; optionally \
                     filtered by block or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::EXPLORER_TX_SUMMARIES_FROM_BLOCK_ROUTE,
            get_with(explorer_tx_summaries_from_block, |op| {
                op.summary("List transaction summaries").description(
                    "Retrieve up to `limit` transaction summaries, targeting the latest \
                     transaction, one identified by height/offset, or by hash; optionally \
                     filtered by block or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::EXPLORER_TX_SUMMARIES_BY_HASH_BLOCK_ROUTE,
            get_with(explorer_tx_summaries_by_hash_block, |op| {
                op.summary("List transaction summaries").description(
                    "Retrieve up to `limit` transaction summaries, targeting the latest \
                     transaction, one identified by height/offset, or by hash; optionally \
                     filtered by block or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::EXPLORER_TX_SUMMARIES_LATEST_NS_ROUTE,
            get_with(explorer_tx_summaries_latest_ns, |op| {
                op.summary("List transaction summaries").description(
                    "Retrieve up to `limit` transaction summaries, targeting the latest \
                     transaction, one identified by height/offset, or by hash; optionally \
                     filtered by block or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::EXPLORER_TX_SUMMARIES_FROM_NS_ROUTE,
            get_with(explorer_tx_summaries_from_ns, |op| {
                op.summary("List transaction summaries").description(
                    "Retrieve up to `limit` transaction summaries, targeting the latest \
                     transaction, one identified by height/offset, or by hash; optionally \
                     filtered by block or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::EXPLORER_TX_SUMMARIES_BY_HASH_NS_ROUTE,
            get_with(explorer_tx_summaries_by_hash_ns, |op| {
                op.summary("List transaction summaries").description(
                    "Retrieve up to `limit` transaction summaries, targeting the latest \
                     transaction, one identified by height/offset, or by hash; optionally \
                     filtered by block or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::EXPLORER_TX_SUMMARIES_LATEST_ROUTE,
            get_with(explorer_tx_summaries_latest, |op| {
                op.summary("List transaction summaries").description(
                    "Retrieve up to `limit` transaction summaries, targeting the latest \
                     transaction, one identified by height/offset, or by hash; optionally \
                     filtered by block or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::EXPLORER_TX_SUMMARIES_FROM_ROUTE,
            get_with(explorer_tx_summaries_from, |op| {
                op.summary("List transaction summaries").description(
                    "Retrieve up to `limit` transaction summaries, targeting the latest \
                     transaction, one identified by height/offset, or by hash; optionally \
                     filtered by block or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::EXPLORER_TX_SUMMARIES_BY_HASH_ROUTE,
            get_with(explorer_tx_summaries_by_hash, |op| {
                op.summary("List transaction summaries").description(
                    "Retrieve up to `limit` transaction summaries, targeting the latest \
                     transaction, one identified by height/offset, or by hash; optionally \
                     filtered by block or namespace.",
                )
            }),
        )
        .api_route(
            routes::v1::EXPLORER_SUMMARY_ROUTE,
            get_with(explorer_summary, |op| {
                op.summary("Get explorer summary")
                    .description("Get the current chain explorer summary.")
            }),
        )
        .api_route(
            routes::v1::EXPLORER_SEARCH_ROUTE,
            get_with(explorer_search, |op| {
                op.summary("Search blocks and transactions").description(
                    "Search for blocks or transactions matching the given query string; currently \
                     matched against hash.",
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
    // Each `router_*` function already calls `with_state`, so the merged router is already
    // stateless (`ApiRouter<()>`) by the time it reaches `finish_api`.
    let router = router_reward(state.clone())
        .merge(router_availability(state.clone()))
        .merge(router_block_state(state.clone()))
        .merge(router_fee_state(state.clone()))
        .merge(router_status(state.clone()))
        .merge(router_config(state.clone()))
        .merge(router_node(state.clone()))
        .merge(router_catchup(state.clone()))
        .merge(router_submit(state.clone()))
        .merge(router_state_signature(state.clone()))
        .merge(router_hotshot_events(state.clone()))
        .merge(router_light_client(state.clone()))
        .merge(router_explorer(state.clone()))
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

    #[tokio::test]
    async fn max_connections_bounds_streaming_sockets() {
        let ws_route = |ws: WebSocketUpgrade, limit: Option<Extension<StreamLimit>>| async move {
            let permit = match acquire_stream_permit(limit) {
                Ok(permit) => permit,
                Err(status) => return status.into_response(),
            };
            ws.on_upgrade(move |socket| async move {
                let _permit = permit;
                let stream: BoxStream<'static, u64> =
                    Box::pin(futures::stream::unfold((), |()| async {
                        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                        Some((0u64, ()))
                    }));
                drive_ws_stream(socket, stream, WsFormat::Json).await
            })
        };
        let router = Router::new().route("/ws", get(ws_route));
        let router = crate::apply_connection_limit(router, 2);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        async fn upgrade(addr: std::net::SocketAddr) -> (tokio::net::TcpStream, String) {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut sock = tokio::net::TcpStream::connect(addr).await.unwrap();
            sock.write_all(
                b"GET /ws HTTP/1.1\r\nHost: localhost\r\nConnection: Upgrade\r\n\
                  Upgrade: websocket\r\nSec-WebSocket-Version: 13\r\n\
                  Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n",
            )
            .await
            .unwrap();
            let mut buf = [0u8; 64];
            let n = sock.read(&mut buf).await.unwrap();
            let status = String::from_utf8_lossy(&buf[..n])
                .lines()
                .next()
                .unwrap_or_default()
                .to_string();
            (sock, status)
        }

        let (_s1, status) = upgrade(addr).await;
        assert!(status.contains("101"), "first socket: {status}");
        let (_s2, status) = upgrade(addr).await;
        assert!(status.contains("101"), "second socket: {status}");
        let (_s3, status) = upgrade(addr).await;
        assert!(
            status.contains("429"),
            "third socket must be limited: {status}"
        );

        // Closing a socket frees its slot once the server notices on the next send.
        drop(_s1);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let (_s4, status) = upgrade(addr).await;
            if status.contains("101") {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "slot was not released after socket close: {status}"
            );
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
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
}
