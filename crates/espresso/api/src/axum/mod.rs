//! Axum HTTP/JSON API handlers
//!
//! [`mod.rs`](self) holds the shared plumbing (error/response encoding, websocket streaming,
//! top-level routes, OpenAPI docs) and the v1/v2 router assembly. The per-module `router_*`
//! builders live in [`routers`].

pub mod routes;

mod routers;

#[cfg(test)]
mod tests;

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
use schemars::transform::Transform;
use serde::Serialize;
use serialization_api::v2::{
    GetIncorrectEncodingProofRequest, GetNamespaceProofRequest, GetRewardAccountProofRequest,
    GetRewardBalanceRequest, GetRewardBalancesRequest, GetRewardClaimInputRequest,
    GetRewardMerkleTreeRequest, GetStakeTableRequest, GetStateCertificateRequest,
};
use tokio::sync::Semaphore;
use vbs::{BinarySerializer, Serializer, version::StaticVersion};

// The serve modes in `crate::lib` and `create_router_v1` mount the per-module builders by name.
pub(crate) use self::routers::*;
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
    Html(include_str!("../../templates/swagger.html").replace("{{OPENAPI_SPEC_ROUTE}}", spec_route))
}

/// v2 is WIP, so `/` points at the v1 docs; 307 so browsers don't cache the redirect.
async fn redirect_to_docs() -> axum::response::Redirect {
    axum::response::Redirect::temporary(routes::v1::VERSION_PREFIX)
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
    loop {
        // Also poll the client side: a disconnect must end this task even while the stream is
        // quiet, or the socket's connection slot and the stream task leak until the next send.
        let item = tokio::select! {
            item = stream.next() => item,
            msg = socket.recv() => match msg {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return,
                Some(Ok(_)) => continue,
            },
        };
        let Some(item) = item else { break };
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
/// `/`, `/healthcheck`, and `/version`. The per-module `/v1/{module}/healthcheck` lives inside the
/// `/v1` nest (see [`finish_v1_docs`]); adding it here would conflict with the nest's wildcard.
pub(crate) fn with_top_level_routes(router: Router) -> Router {
    router
        .route("/", get(redirect_to_docs))
        .route("/healthcheck", get(healthcheck))
        .route("/version", get(version))
}

/// Health status of an application.
///
/// Wire-compatible with `tide_disco::healthcheck::HealthStatus` 0.9.6: `Available` is its first
/// variant, so JSON emits the same name and vbs/bincode the same ordinal. The server only ever
/// reports `Available`; the remaining tide variants are omitted until a client-side type exists.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Available,
}

/// Wire-compatible with `tide_disco::app::AppHealth`: JSON keys, variant casing, and the
/// vbs/bincode field order (status ordinal, then modules map) must not change.
#[derive(Serialize)]
struct AppHealth {
    status: HealthStatus,
    // Tide populated this with each module's versioned health status; the axum modules don't
    // report individual health, so it stays empty.
    modules: BTreeMap<String, BTreeMap<u64, u16>>,
}

/// Top-level healthcheck, matching tide-disco's app-level `AppHealth` response for multi-module
/// apps, in JSON or vbs binary depending on `Accept`.
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
/// crate version so `surf_disco::Client::connect` and similar polling helpers succeed.
async fn version() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
    }))
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
///
/// The module routers register version-agnostic paths (`/status/...`); mounting them under
/// [`routes::v1::VERSION_PREFIX`] here is what makes them `/v1/...`, and aide records the prefixed
/// paths in the generated spec. This is the single place the `/v1` prefix is applied.
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

    // Register the docs routes on the module router too (version-agnostic, like every route it
    // already holds), so the single `nest` below serves them at `/v1/docs/openapi.json`,
    // `/v1/scalar`, and the swagger UI at the bare `/v1` (the nested `/` root). Keeping them inside
    // the one nest avoids a second route claiming `/v1`, which would conflict with the nest. They
    // use `.route` (not `.api_route`), so aide leaves them out of the spec; the pages fetch the
    // spec by its absolute `openapi_spec_url()`.
    let spec_url = routes::v1::openapi_spec_url();
    let swagger = swagger_html(&spec_url);
    let router = router
        .route(routes::v1::OPENAPI_SPEC_ROUTE, get(serve_openapi_spec_v1))
        .route(
            routes::v1::SWAGGER_ROUTE,
            get(move || std::future::ready(swagger.clone())),
        )
        .route("/{module}/healthcheck", get(module_healthcheck))
        .route(
            routes::v1::SCALAR_ROUTE,
            get(Scalar::new(&spec_url)
                .with_title("Espresso Node API v1")
                .axum_handler()),
        );

    let router = ApiRouter::new()
        .nest_api_service(routes::v1::VERSION_PREFIX, router)
        .finish_api(&mut api);

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

    router.layer(Extension(OpenApiV1(api)))
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
