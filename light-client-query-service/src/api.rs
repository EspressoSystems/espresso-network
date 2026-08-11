//! Axum port of the `node` API module that this service used to serve via
//! `hotshot_query_service::node::define_api` on a tide-disco `App`, plus the `availability`
//! module served by `hotshot_query_service`'s own router.
//!
//! Route paths, status codes and the wire error type are all taken directly from
//! `hotshot-query-service` (see `node.rs` there and its handler bodies) so that clients built
//! against the old tide-disco server keep working unmodified.

use std::ops::Bound;

use axum::{
    Router,
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
    routing::get,
};
use espresso_node::api::sql::DataSource;
use espresso_types::SeqTypes;
use hotshot_query_service::{
    Error as ApiError, Header,
    availability::{self, BlockHash, BlockId, router::availability_router},
    node::{self, Limits as NodeLimits, NodeDataSource as _, WindowStart},
};
use hotshot_types::data::VidCommitment;
use http_wire::{self as wire, cors_layer, healthcheck_response};

/// Mirrors `hotshot_query_service::node::Options::default()`, which is what this service's
/// tide-disco setup used.
const WINDOW_LIMIT: usize = 500;

/// Parses a path parameter the way tide-disco's `TaggedBase64`/`Integer` param types did,
/// reporting failures the same way tide-disco's own request-parsing errors are surfaced: as a
/// 400 with a descriptive message.
fn parse_node_param<T: std::str::FromStr>(value: &str, field: &str) -> Result<T, node::Error>
where
    T::Err: std::fmt::Display,
{
    value.parse().map_err(|e| node::Error::Custom {
        message: format!("invalid {field}: {e}"),
        status: disco_types::status::StatusCode::BAD_REQUEST,
    })
}

fn range_bounds(from: Option<u64>, to: Option<u64>) -> (Bound<usize>, Bound<usize>) {
    (
        from.map_or(Bound::Unbounded, |f| Bound::Included(f as usize)),
        to.map_or(Bound::Unbounded, |t| Bound::Included(t as usize)),
    )
}

async fn fetch_count_transactions(
    ds: &DataSource,
    from: Option<u64>,
    to: Option<u64>,
    namespace: Option<i64>,
) -> Result<usize, node::Error> {
    ds.count_transactions_in_range(range_bounds(from, to), namespace.map(Into::into))
        .await
        .map_err(Into::into)
}

async fn fetch_payload_size(
    ds: &DataSource,
    from: Option<u64>,
    to: Option<u64>,
    namespace: Option<i64>,
) -> Result<usize, node::Error> {
    ds.payload_size_in_range(range_bounds(from, to), namespace.map(Into::into))
        .await
        .map_err(Into::into)
}

async fn fetch_vid_share(
    ds: &DataSource,
    id: BlockId<SeqTypes>,
) -> Result<hotshot_types::data::VidShare, node::Error> {
    ds.vid_share(id)
        .await
        .map_err(|source| node::Error::QueryVid {
            source,
            block: id.to_string(),
        })
}

async fn fetch_header_window(
    ds: &DataSource,
    start: WindowStart<SeqTypes>,
    end: u64,
) -> Result<hotshot_query_service::node::TimeWindowQueryData<Header<SeqTypes>>, node::Error> {
    ds.get_header_window(start, end, WINDOW_LIMIT)
        .await
        .map_err(|source| node::Error::QueryWindow {
            source,
            start: format!("{start:?}"),
            end,
        })
}

async fn node_block_height(State(ds): State<DataSource>, headers: HeaderMap) -> Response {
    let result = ds.block_height().await.map_err(node::Error::from);
    wire::respond::<ApiError, _>(&headers, result.map_err(ApiError::from))
}

async fn node_count_transactions(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    from: Option<u64>,
    to: Option<u64>,
    namespace: Option<i64>,
) -> Response {
    let result = fetch_count_transactions(&ds, from, to, namespace).await;
    wire::respond::<ApiError, _>(&headers, result.map_err(ApiError::from))
}

async fn node_count_transactions_all(state: State<DataSource>, headers: HeaderMap) -> Response {
    node_count_transactions(state, headers, None, None, None).await
}

async fn node_count_transactions_to(
    state: State<DataSource>,
    headers: HeaderMap,
    Path(to): Path<u64>,
) -> Response {
    node_count_transactions(state, headers, None, Some(to), None).await
}

async fn node_count_transactions_from_to(
    state: State<DataSource>,
    headers: HeaderMap,
    Path((from, to)): Path<(u64, u64)>,
) -> Response {
    node_count_transactions(state, headers, Some(from), Some(to), None).await
}

async fn node_count_transactions_ns(
    state: State<DataSource>,
    headers: HeaderMap,
    Path(namespace): Path<i64>,
) -> Response {
    node_count_transactions(state, headers, None, None, Some(namespace)).await
}

async fn node_count_transactions_ns_to(
    state: State<DataSource>,
    headers: HeaderMap,
    Path((namespace, to)): Path<(i64, u64)>,
) -> Response {
    node_count_transactions(state, headers, None, Some(to), Some(namespace)).await
}

async fn node_count_transactions_ns_from_to(
    state: State<DataSource>,
    headers: HeaderMap,
    Path((namespace, from, to)): Path<(i64, u64, u64)>,
) -> Response {
    node_count_transactions(state, headers, Some(from), Some(to), Some(namespace)).await
}

async fn node_payload_size(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    from: Option<u64>,
    to: Option<u64>,
    namespace: Option<i64>,
) -> Response {
    let result = fetch_payload_size(&ds, from, to, namespace).await;
    wire::respond::<ApiError, _>(&headers, result.map_err(ApiError::from))
}

async fn node_payload_size_all(state: State<DataSource>, headers: HeaderMap) -> Response {
    node_payload_size(state, headers, None, None, None).await
}

async fn node_payload_size_to(
    state: State<DataSource>,
    headers: HeaderMap,
    Path(to): Path<u64>,
) -> Response {
    node_payload_size(state, headers, None, Some(to), None).await
}

async fn node_payload_size_from_to(
    state: State<DataSource>,
    headers: HeaderMap,
    Path((from, to)): Path<(u64, u64)>,
) -> Response {
    node_payload_size(state, headers, Some(from), Some(to), None).await
}

async fn node_payload_size_ns(
    state: State<DataSource>,
    headers: HeaderMap,
    Path(namespace): Path<i64>,
) -> Response {
    node_payload_size(state, headers, None, None, Some(namespace)).await
}

async fn node_payload_size_ns_to(
    state: State<DataSource>,
    headers: HeaderMap,
    Path((namespace, to)): Path<(i64, u64)>,
) -> Response {
    node_payload_size(state, headers, None, Some(to), Some(namespace)).await
}

async fn node_payload_size_ns_from_to(
    state: State<DataSource>,
    headers: HeaderMap,
    Path((namespace, from, to)): Path<(i64, u64, u64)>,
) -> Response {
    node_payload_size(state, headers, Some(from), Some(to), Some(namespace)).await
}

async fn node_vid_share_by_height(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(height): Path<u64>,
) -> Response {
    let result = fetch_vid_share(&ds, BlockId::Number(height as usize)).await;
    wire::respond::<ApiError, _>(&headers, result.map_err(ApiError::from))
}

async fn node_vid_share_by_hash(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response {
    let result = match parse_node_param::<BlockHash<SeqTypes>>(&hash, "hash") {
        Ok(hash) => fetch_vid_share(&ds, BlockId::Hash(hash)).await,
        Err(e) => Err(e),
    };
    wire::respond::<ApiError, _>(&headers, result.map_err(ApiError::from))
}

async fn node_vid_share_by_payload_hash(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(payload_hash): Path<String>,
) -> Response {
    let result = match parse_node_param::<VidCommitment>(&payload_hash, "payload-hash") {
        Ok(hash) => fetch_vid_share(&ds, BlockId::PayloadHash(hash)).await,
        Err(e) => Err(e),
    };
    wire::respond::<ApiError, _>(&headers, result.map_err(ApiError::from))
}

async fn node_sync_status(State(ds): State<DataSource>, headers: HeaderMap) -> Response {
    let result = ds.sync_status().await.map_err(node::Error::from);
    wire::respond::<ApiError, _>(&headers, result.map_err(ApiError::from))
}

async fn node_header_window(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path((start, end)): Path<(u64, u64)>,
) -> Response {
    let result = fetch_header_window(&ds, WindowStart::Time(start), end).await;
    wire::respond::<ApiError, _>(&headers, result.map_err(ApiError::from))
}

async fn node_header_window_from_height(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path((height, end)): Path<(u64, u64)>,
) -> Response {
    let result = fetch_header_window(&ds, WindowStart::Height(height), end).await;
    wire::respond::<ApiError, _>(&headers, result.map_err(ApiError::from))
}

async fn node_header_window_from_hash(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path((hash, end)): Path<(String, u64)>,
) -> Response {
    let result = match parse_node_param::<BlockHash<SeqTypes>>(&hash, "hash") {
        Ok(hash) => fetch_header_window(&ds, WindowStart::Hash(hash), end).await,
        Err(e) => Err(e),
    };
    wire::respond::<ApiError, _>(&headers, result.map_err(ApiError::from))
}

async fn node_limits(headers: HeaderMap) -> Response {
    wire::encode_ok::<ApiError, _>(
        &headers,
        NodeLimits {
            window_limit: WINDOW_LIMIT,
        },
    )
}

fn node_router(ds: DataSource) -> Router {
    Router::new()
        .route("/block-height", get(node_block_height))
        .route("/transactions/count", get(node_count_transactions_all))
        .route("/transactions/count/{to}", get(node_count_transactions_to))
        .route(
            "/transactions/count/{from}/{to}",
            get(node_count_transactions_from_to),
        )
        .route(
            "/transactions/count/namespace/{namespace}",
            get(node_count_transactions_ns),
        )
        .route(
            "/transactions/count/namespace/{namespace}/{to}",
            get(node_count_transactions_ns_to),
        )
        .route(
            "/transactions/count/namespace/{namespace}/{from}/{to}",
            get(node_count_transactions_ns_from_to),
        )
        .route("/payloads/size", get(node_payload_size_all))
        .route("/payloads/total-size", get(node_payload_size_all))
        .route("/payloads/size/{to}", get(node_payload_size_to))
        .route("/payloads/size/{from}/{to}", get(node_payload_size_from_to))
        .route(
            "/payloads/size/namespace/{namespace}",
            get(node_payload_size_ns),
        )
        .route(
            "/payloads/size/namespace/{namespace}/{to}",
            get(node_payload_size_ns_to),
        )
        .route(
            "/payloads/size/namespace/{namespace}/{from}/{to}",
            get(node_payload_size_ns_from_to),
        )
        .route("/vid/share/{height}", get(node_vid_share_by_height))
        .route("/vid/share/hash/{hash}", get(node_vid_share_by_hash))
        .route(
            "/vid/share/payload-hash/{payload_hash}",
            get(node_vid_share_by_payload_hash),
        )
        .route("/sync-status", get(node_sync_status))
        .route("/header/window/{start}/{end}", get(node_header_window))
        .route(
            "/header/window/from/{height}/{end}",
            get(node_header_window_from_height),
        )
        .route(
            "/header/window/from/hash/{hash}/{end}",
            get(node_header_window_from_hash),
        )
        .route("/limits", get(node_limits))
        .with_state(ds)
}

/// Builds the full router: `healthcheck`, plus the `availability` and `node` modules served both
/// unversioned and under `/v1`, matching the paths tide-disco exposed for this service (which
/// only ever registered API version `1.0.0`).
pub fn router(ds: DataSource) -> Router {
    let api = Router::new()
        .nest(
            "/availability",
            // This service serves no OpenAPI spec, so drop the router's documentation.
            Router::from(availability_router::<SeqTypes, DataSource>(
                &availability::Options::default(),
                ds.clone(),
            )),
        )
        .nest("/node", node_router(ds));
    Router::new()
        .route(
            "/healthcheck",
            get(|headers: HeaderMap| async move { healthcheck_response(&headers) }),
        )
        .merge(api.clone())
        .nest("/v1", api)
        .layer(cors_layer())
}
