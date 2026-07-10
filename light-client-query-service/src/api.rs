//! Axum port of the `availability` and `node` API modules that this service used to serve via
//! `hotshot_query_service::{availability, node}::define_api` on a `tide_disco::App`.
//!
//! Route paths, status codes and the wire error type are all taken directly from
//! `hotshot-query-service` (see `availability.rs`/`node.rs` there and their handler bodies) so
//! that clients built against the old tide-disco server keep working unmodified.

use std::{ops::Bound, time::Duration};

use axum::{
    Json, Router,
    extract::{
        Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use espresso_node::api::sql::DataSource;
use espresso_types::SeqTypes;
use futures::{StreamExt as _, TryStreamExt as _, stream::BoxStream};
use hotshot_query_service::{
    Error as ApiError, Header,
    availability::{
        self, AvailabilityDataSource as _, BlockHash, BlockId, BlockQueryData,
        BlockSummaryQueryData, BlockWithTransaction, LeafHash, LeafId, LeafQueryData,
        Limits as AvailabilityLimits, PayloadQueryData, QueryableHeader as _,
        QueryablePayload as _, TransactionHash, TransactionQueryData,
        TransactionWithProofQueryData, VidCommonQueryData,
    },
    node::{self, Limits as NodeLimits, NodeDataSource as _, WindowStart},
    types::HeightIndexed as _,
};
use hotshot_types::data::VidCommitment;
use serde::Serialize;
use surf_disco::Error as _;
use vbs::{BinarySerializer, Serializer, version::StaticVersion};

/// Binary framing version for VBS-negotiated responses, matching the wire version this service
/// used under tide-disco.
type WireVersion = StaticVersion<0, 1>;

/// Mirrors `hotshot_query_service::availability::Options::default()` and
/// `node::Options::default()`, which is what this service's tide-disco setup used.
const FETCH_TIMEOUT: Duration = Duration::from_millis(500);
const SMALL_OBJECT_RANGE_LIMIT: usize = 500;
const LARGE_OBJECT_RANGE_LIMIT: usize = 100;
const WINDOW_LIMIT: usize = 500;

fn wants_binary(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("application/octet-stream"))
}

/// Maps `hotshot_query_service`'s wrapped `reqwest`-based status code onto axum's.
fn wire_status(status: surf_disco::StatusCode) -> StatusCode {
    StatusCode::from_u16(u16::from(status)).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

/// Encode a successful response, negotiating VBS binary vs JSON from the `Accept` header, just
/// as tide-disco did for `surf-disco` clients (which default to `application/octet-stream`).
fn encode_ok<T: Serialize>(headers: &HeaderMap, value: T) -> Response {
    if wants_binary(headers) {
        match Serializer::<WireVersion>::serialize(&value) {
            Ok(bytes) => {
                ([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response()
            },
            Err(err) => encode_err(headers, ApiError::internal(err)),
        }
    } else {
        Json(value).into_response()
    }
}

/// Encode an error response using the same content negotiation as [`encode_ok`]. `err` is
/// `hotshot_query_service::Error`, the exact type the old tide-disco `App` used, so both its
/// status mapping and its wire shape (externally tagged enum) match byte-for-byte.
fn encode_err(headers: &HeaderMap, err: ApiError) -> Response {
    let status = wire_status(err.status());
    if wants_binary(headers) {
        match Serializer::<WireVersion>::serialize(&err) {
            Ok(bytes) => (
                status,
                [(header::CONTENT_TYPE, "application/octet-stream")],
                bytes,
            )
                .into_response(),
            Err(_) => (status, Json(err)).into_response(),
        }
    } else {
        (status, Json(err)).into_response()
    }
}

fn respond<T: Serialize>(headers: &HeaderMap, result: Result<T, ApiError>) -> Response {
    match result {
        Ok(v) => encode_ok(headers, v),
        Err(e) => encode_err(headers, e),
    }
}

/// Parses a path parameter the way tide-disco's `TaggedBase64`/`Integer` param types did,
/// reporting failures the same way tide-disco's own request-parsing errors are surfaced: as a
/// 400 with a descriptive message.
fn parse_availability_param<T: std::str::FromStr>(
    value: &str,
    field: &str,
) -> Result<T, availability::Error>
where
    T::Err: std::fmt::Display,
{
    value.parse().map_err(|e| availability::Error::Custom {
        message: format!("invalid {field}: {e}"),
        status: surf_disco::StatusCode::BAD_REQUEST,
    })
}

/// Same as [`parse_availability_param`], for handlers in the `node` module.
fn parse_node_param<T: std::str::FromStr>(value: &str, field: &str) -> Result<T, node::Error>
where
    T::Err: std::fmt::Display,
{
    value.parse().map_err(|e| node::Error::Custom {
        message: format!("invalid {field}: {e}"),
        status: surf_disco::StatusCode::BAD_REQUEST,
    })
}

fn enforce_range_limit(from: usize, until: usize, limit: usize) -> Result<(), availability::Error> {
    if until.saturating_sub(from) > limit {
        return Err(availability::Error::RangeLimit { from, until, limit });
    }
    Ok(())
}

async fn healthcheck() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "Available" }))
}

async fn drive_ws_stream<T: Serialize + Send + 'static>(
    mut socket: WebSocket,
    mut stream: BoxStream<'static, T>,
    binary: bool,
) {
    while let Some(item) = stream.next().await {
        let msg = if binary {
            match Serializer::<WireVersion>::serialize(&item) {
                Ok(bytes) => Message::Binary(bytes.into()),
                Err(_) => break,
            }
        } else {
            match serde_json::to_string(&item) {
                Ok(text) => Message::Text(text.into()),
                Err(_) => break,
            }
        };
        if socket.send(msg).await.is_err() {
            break;
        }
    }
}

// --- availability -----------------------------------------------------------------------------

async fn fetch_leaf(
    ds: &DataSource,
    id: LeafId<SeqTypes>,
) -> Result<LeafQueryData<SeqTypes>, availability::Error> {
    ds.get_leaf(id)
        .await
        .with_timeout(FETCH_TIMEOUT)
        .await
        .ok_or_else(|| availability::Error::FetchLeaf {
            resource: id.to_string(),
        })
}

async fn fetch_leaf_range(
    ds: &DataSource,
    from: usize,
    until: usize,
) -> Result<Vec<LeafQueryData<SeqTypes>>, availability::Error> {
    enforce_range_limit(from, until, SMALL_OBJECT_RANGE_LIMIT)?;
    ds.get_leaf_range(from..until)
        .await
        .enumerate()
        .then(|(index, fetch)| async move {
            fetch
                .with_timeout(FETCH_TIMEOUT)
                .await
                .ok_or_else(|| availability::Error::FetchLeaf {
                    resource: (index + from).to_string(),
                })
        })
        .try_collect()
        .await
}

async fn fetch_header(
    ds: &DataSource,
    id: BlockId<SeqTypes>,
) -> Result<Header<SeqTypes>, availability::Error> {
    ds.get_header(id)
        .await
        .with_timeout(FETCH_TIMEOUT)
        .await
        .ok_or_else(|| availability::Error::FetchHeader {
            resource: id.to_string(),
        })
}

async fn fetch_header_range(
    ds: &DataSource,
    from: usize,
    until: usize,
) -> Result<Vec<Header<SeqTypes>>, availability::Error> {
    enforce_range_limit(from, until, LARGE_OBJECT_RANGE_LIMIT)?;
    ds.get_header_range(from..until)
        .await
        .enumerate()
        .then(|(index, fetch)| async move {
            fetch.with_timeout(FETCH_TIMEOUT).await.ok_or_else(|| {
                availability::Error::FetchHeader {
                    resource: (index + from).to_string(),
                }
            })
        })
        .try_collect()
        .await
}

async fn fetch_block(
    ds: &DataSource,
    id: BlockId<SeqTypes>,
) -> Result<BlockQueryData<SeqTypes>, availability::Error> {
    ds.get_block(id)
        .await
        .with_timeout(FETCH_TIMEOUT)
        .await
        .ok_or_else(|| availability::Error::FetchBlock {
            resource: id.to_string(),
        })
}

async fn fetch_block_range(
    ds: &DataSource,
    from: usize,
    until: usize,
) -> Result<Vec<BlockQueryData<SeqTypes>>, availability::Error> {
    enforce_range_limit(from, until, LARGE_OBJECT_RANGE_LIMIT)?;
    ds.get_block_range(from..until)
        .await
        .enumerate()
        .then(|(index, fetch)| async move {
            fetch
                .with_timeout(FETCH_TIMEOUT)
                .await
                .ok_or_else(|| availability::Error::FetchBlock {
                    resource: (index + from).to_string(),
                })
        })
        .try_collect()
        .await
}

async fn fetch_payload(
    ds: &DataSource,
    id: BlockId<SeqTypes>,
) -> Result<PayloadQueryData<SeqTypes>, availability::Error> {
    // Matches tide: payloads are keyed by `BlockId` and report `FetchBlock` on a miss, there is
    // no separate `FetchPayload` variant.
    ds.get_payload(id)
        .await
        .with_timeout(FETCH_TIMEOUT)
        .await
        .ok_or_else(|| availability::Error::FetchBlock {
            resource: id.to_string(),
        })
}

async fn fetch_payload_range(
    ds: &DataSource,
    from: usize,
    until: usize,
) -> Result<Vec<PayloadQueryData<SeqTypes>>, availability::Error> {
    enforce_range_limit(from, until, LARGE_OBJECT_RANGE_LIMIT)?;
    ds.get_payload_range(from..until)
        .await
        .enumerate()
        .then(|(index, fetch)| async move {
            fetch
                .with_timeout(FETCH_TIMEOUT)
                .await
                .ok_or_else(|| availability::Error::FetchBlock {
                    resource: (index + from).to_string(),
                })
        })
        .try_collect()
        .await
}

async fn fetch_vid_common(
    ds: &DataSource,
    id: BlockId<SeqTypes>,
) -> Result<VidCommonQueryData<SeqTypes>, availability::Error> {
    ds.get_vid_common(id)
        .await
        .with_timeout(FETCH_TIMEOUT)
        .await
        .ok_or_else(|| availability::Error::FetchBlock {
            resource: id.to_string(),
        })
}

async fn fetch_vid_common_range(
    ds: &DataSource,
    from: usize,
    until: usize,
) -> Result<Vec<VidCommonQueryData<SeqTypes>>, availability::Error> {
    enforce_range_limit(from, until, SMALL_OBJECT_RANGE_LIMIT)?;
    ds.get_vid_common_range(from..until)
        .await
        .enumerate()
        .then(|(index, fetch)| async move {
            fetch
                .with_timeout(FETCH_TIMEOUT)
                .await
                .ok_or_else(|| availability::Error::FetchBlock {
                    resource: (index + from).to_string(),
                })
        })
        .try_collect()
        .await
}

async fn fetch_transaction_by_position(
    ds: &DataSource,
    height: u64,
    index: u64,
) -> Result<BlockWithTransaction<SeqTypes>, availability::Error> {
    let block = fetch_block(ds, BlockId::Number(height as usize)).await?;
    let ix = block
        .payload()
        .nth(block.metadata(), index as usize)
        .ok_or(availability::Error::InvalidTransactionIndex { height, index })?;
    let transaction = block
        .transaction(&ix)
        .ok_or(availability::Error::InvalidTransactionIndex { height, index })?;
    let transaction = TransactionQueryData::new(transaction, &block, &ix, index)
        .ok_or(availability::Error::InvalidTransactionIndex { height, index })?;
    Ok(BlockWithTransaction {
        block,
        transaction,
        index: ix,
    })
}

async fn fetch_transaction_by_hash(
    ds: &DataSource,
    hash: &str,
) -> Result<BlockWithTransaction<SeqTypes>, availability::Error> {
    let hash = parse_availability_param::<TransactionHash<SeqTypes>>(hash, "hash")?;
    ds.get_block_containing_transaction(hash)
        .await
        .with_timeout(FETCH_TIMEOUT)
        .await
        .ok_or_else(|| availability::Error::FetchTransaction {
            resource: hash.to_string(),
        })
}

async fn fetch_transaction_with_proof(
    ds: &DataSource,
    bwt: BlockWithTransaction<SeqTypes>,
) -> Result<TransactionWithProofQueryData<SeqTypes>, availability::Error> {
    let height = bwt.block.height();
    let vid = fetch_vid_common(ds, BlockId::Number(height as usize)).await?;
    let proof = bwt.block.transaction_proof(&vid, &bwt.index).ok_or(
        availability::Error::InvalidTransactionIndex {
            height,
            index: bwt.transaction.index(),
        },
    )?;
    Ok(TransactionWithProofQueryData::new(bwt.transaction, proof))
}

async fn fetch_block_summary(
    ds: &DataSource,
    height: usize,
) -> Result<BlockSummaryQueryData<SeqTypes>, availability::Error> {
    fetch_block(ds, BlockId::Number(height))
        .await
        .map(BlockSummaryQueryData::from)
}

async fn fetch_block_summary_range(
    ds: &DataSource,
    from: usize,
    until: usize,
) -> Result<Vec<BlockSummaryQueryData<SeqTypes>>, availability::Error> {
    enforce_range_limit(from, until, LARGE_OBJECT_RANGE_LIMIT)?;
    ds.get_block_range(from..until)
        .await
        .enumerate()
        .then(|(index, fetch)| async move {
            fetch
                .with_timeout(FETCH_TIMEOUT)
                .await
                .ok_or_else(|| availability::Error::FetchBlock {
                    resource: (index + from).to_string(),
                })
        })
        .map(|result| result.map(BlockSummaryQueryData::from))
        .try_collect()
        .await
}

/// Dispatches a `BlockId`-keyed fetch once the id has been parsed from the path, matching the
/// `respond`/error-conversion boilerplate every `availability` route needs.
async fn respond_block_resource<T, F>(
    headers: &HeaderMap,
    id: Result<BlockId<SeqTypes>, availability::Error>,
    fetch: F,
) -> Response
where
    T: Serialize,
    F: AsyncFnOnce(BlockId<SeqTypes>) -> Result<T, availability::Error>,
{
    let result = match id {
        Ok(id) => fetch(id).await,
        Err(e) => Err(e),
    };
    respond(headers, result.map_err(ApiError::from))
}

async fn get_leaf_by_height(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(height): Path<u64>,
) -> Response {
    let result = fetch_leaf(&ds, LeafId::Number(height as usize)).await;
    respond(&headers, result.map_err(ApiError::from))
}

async fn get_leaf_by_hash(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response {
    let result = match parse_availability_param::<LeafHash<SeqTypes>>(&hash, "hash") {
        Ok(hash) => fetch_leaf(&ds, LeafId::Hash(hash)).await,
        Err(e) => Err(e),
    };
    respond(&headers, result.map_err(ApiError::from))
}

async fn get_leaf_range(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path((from, until)): Path<(usize, usize)>,
) -> Response {
    let result = fetch_leaf_range(&ds, from, until).await;
    respond(&headers, result.map_err(ApiError::from))
}

async fn stream_leaves(
    ws: WebSocketUpgrade,
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response {
    let binary = wants_binary(&headers);
    ws.on_upgrade(move |socket| async move {
        let stream = ds.subscribe_leaves(height).await;
        drive_ws_stream(socket, stream, binary).await;
    })
}

async fn get_header_by_height(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(height): Path<u64>,
) -> Response {
    respond_block_resource(&headers, Ok(BlockId::Number(height as usize)), async |id| {
        fetch_header(&ds, id).await
    })
    .await
}

async fn get_header_by_hash(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response {
    let id = parse_availability_param::<BlockHash<SeqTypes>>(&hash, "hash").map(BlockId::Hash);
    respond_block_resource(&headers, id, async |id| fetch_header(&ds, id).await).await
}

async fn get_header_by_payload_hash(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(payload_hash): Path<String>,
) -> Response {
    let id = parse_availability_param::<VidCommitment>(&payload_hash, "payload-hash")
        .map(BlockId::PayloadHash);
    respond_block_resource(&headers, id, async |id| fetch_header(&ds, id).await).await
}

async fn get_header_range(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path((from, until)): Path<(usize, usize)>,
) -> Response {
    let result = fetch_header_range(&ds, from, until).await;
    respond(&headers, result.map_err(ApiError::from))
}

async fn stream_headers(
    ws: WebSocketUpgrade,
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response {
    let binary = wants_binary(&headers);
    ws.on_upgrade(move |socket| async move {
        let stream = ds.subscribe_headers(height).await;
        drive_ws_stream(socket, stream, binary).await;
    })
}

async fn get_block_by_height(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(height): Path<u64>,
) -> Response {
    respond_block_resource(&headers, Ok(BlockId::Number(height as usize)), async |id| {
        fetch_block(&ds, id).await
    })
    .await
}

async fn get_block_by_hash(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response {
    let id = parse_availability_param::<BlockHash<SeqTypes>>(&hash, "hash").map(BlockId::Hash);
    respond_block_resource(&headers, id, async |id| fetch_block(&ds, id).await).await
}

async fn get_block_by_payload_hash(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(payload_hash): Path<String>,
) -> Response {
    let id = parse_availability_param::<VidCommitment>(&payload_hash, "payload-hash")
        .map(BlockId::PayloadHash);
    respond_block_resource(&headers, id, async |id| fetch_block(&ds, id).await).await
}

async fn get_block_range(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path((from, until)): Path<(usize, usize)>,
) -> Response {
    let result = fetch_block_range(&ds, from, until).await;
    respond(&headers, result.map_err(ApiError::from))
}

async fn stream_blocks(
    ws: WebSocketUpgrade,
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response {
    let binary = wants_binary(&headers);
    ws.on_upgrade(move |socket| async move {
        let stream = ds.subscribe_blocks(height).await;
        drive_ws_stream(socket, stream, binary).await;
    })
}

async fn get_payload_by_height(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(height): Path<u64>,
) -> Response {
    respond_block_resource(&headers, Ok(BlockId::Number(height as usize)), async |id| {
        fetch_payload(&ds, id).await
    })
    .await
}

async fn get_payload_by_payload_hash(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response {
    let id = parse_availability_param::<VidCommitment>(&hash, "hash").map(BlockId::PayloadHash);
    respond_block_resource(&headers, id, async |id| fetch_payload(&ds, id).await).await
}

async fn get_payload_by_block_hash(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(block_hash): Path<String>,
) -> Response {
    let id = parse_availability_param::<BlockHash<SeqTypes>>(&block_hash, "block-hash")
        .map(BlockId::Hash);
    respond_block_resource(&headers, id, async |id| fetch_payload(&ds, id).await).await
}

async fn get_payload_range(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path((from, until)): Path<(usize, usize)>,
) -> Response {
    let result = fetch_payload_range(&ds, from, until).await;
    respond(&headers, result.map_err(ApiError::from))
}

async fn stream_payloads(
    ws: WebSocketUpgrade,
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response {
    let binary = wants_binary(&headers);
    ws.on_upgrade(move |socket| async move {
        let stream = ds.subscribe_payloads(height).await;
        drive_ws_stream(socket, stream, binary).await;
    })
}

async fn get_vid_common_by_height(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(height): Path<u64>,
) -> Response {
    respond_block_resource(&headers, Ok(BlockId::Number(height as usize)), async |id| {
        fetch_vid_common(&ds, id).await
    })
    .await
}

async fn get_vid_common_by_hash(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response {
    let id = parse_availability_param::<BlockHash<SeqTypes>>(&hash, "hash").map(BlockId::Hash);
    respond_block_resource(&headers, id, async |id| fetch_vid_common(&ds, id).await).await
}

async fn get_vid_common_by_payload_hash(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(payload_hash): Path<String>,
) -> Response {
    let id = parse_availability_param::<VidCommitment>(&payload_hash, "payload-hash")
        .map(BlockId::PayloadHash);
    respond_block_resource(&headers, id, async |id| fetch_vid_common(&ds, id).await).await
}

async fn get_vid_common_range(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path((from, until)): Path<(usize, usize)>,
) -> Response {
    let result = fetch_vid_common_range(&ds, from, until).await;
    respond(&headers, result.map_err(ApiError::from))
}

async fn stream_vid_common(
    ws: WebSocketUpgrade,
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response {
    let binary = wants_binary(&headers);
    ws.on_upgrade(move |socket| async move {
        let stream = ds.subscribe_vid_common(height).await;
        drive_ws_stream(socket, stream, binary).await;
    })
}

async fn get_transaction_by_position(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path((height, index)): Path<(u64, u64)>,
) -> Response {
    let result = fetch_transaction_by_position(&ds, height, index)
        .await
        .map(|bwt| bwt.transaction);
    respond(&headers, result.map_err(ApiError::from))
}

async fn get_transaction_by_hash(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response {
    let result = fetch_transaction_by_hash(&ds, &hash)
        .await
        .map(|bwt| bwt.transaction);
    respond(&headers, result.map_err(ApiError::from))
}

async fn get_transaction_proof_by_position(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path((height, index)): Path<(u64, u64)>,
) -> Response {
    let result = async {
        let bwt = fetch_transaction_by_position(&ds, height, index).await?;
        fetch_transaction_with_proof(&ds, bwt).await
    }
    .await;
    respond(&headers, result.map_err(ApiError::from))
}

async fn get_transaction_proof_by_hash(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response {
    let result = async {
        let bwt = fetch_transaction_by_hash(&ds, &hash).await?;
        fetch_transaction_with_proof(&ds, bwt).await
    }
    .await;
    respond(&headers, result.map_err(ApiError::from))
}

async fn stream_transactions(
    ws: WebSocketUpgrade,
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response {
    let binary = wants_binary(&headers);
    ws.on_upgrade(move |socket| async move {
        let stream = transactions_stream(ds.subscribe_blocks(height).await, None);
        drive_ws_stream(socket, stream, binary).await;
    })
}

async fn stream_transactions_ns(
    ws: WebSocketUpgrade,
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path((height, namespace)): Path<(usize, i64)>,
) -> Response {
    let binary = wants_binary(&headers);
    ws.on_upgrade(move |socket| async move {
        let stream = transactions_stream(ds.subscribe_blocks(height).await, Some(namespace));
        drive_ws_stream(socket, stream, binary).await;
    })
}

/// Mirrors the filtering closure in tide's `stream_transactions` handler: pulls every
/// transaction out of each block, optionally restricted to a single namespace.
fn transactions_stream(
    blocks: BoxStream<'static, BlockQueryData<SeqTypes>>,
    namespace: Option<i64>,
) -> BoxStream<'static, TransactionQueryData<SeqTypes>> {
    blocks
        .flat_map(move |block| {
            let header = block.header().clone();
            let filtered: Vec<_> = block
                .enumerate()
                .enumerate()
                .filter_map(|(i, (index, _tx))| {
                    if let Some(ns) = namespace {
                        let ns_id = header.namespace_id(&index.ns_index)?;
                        if i64::from(ns_id) != ns {
                            return None;
                        }
                    }
                    let tx = block.transaction(&index)?;
                    TransactionQueryData::new(tx, &block, &index, i as u64)
                })
                .collect();
            futures::stream::iter(filtered)
        })
        .boxed()
}

async fn get_block_summary_by_height(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response {
    let result = fetch_block_summary(&ds, height).await;
    respond(&headers, result.map_err(ApiError::from))
}

async fn get_block_summary_range(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path((from, until)): Path<(usize, usize)>,
) -> Response {
    let result = fetch_block_summary_range(&ds, from, until).await;
    respond(&headers, result.map_err(ApiError::from))
}

async fn get_cert2(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path(height): Path<u64>,
) -> Response {
    let result = ds
        .get_cert2(height)
        .await
        .map_err(availability::Error::from);
    respond(&headers, result.map_err(ApiError::from))
}

async fn get_availability_limits(headers: HeaderMap) -> Response {
    encode_ok(
        &headers,
        AvailabilityLimits {
            small_object_range_limit: SMALL_OBJECT_RANGE_LIMIT,
            large_object_range_limit: LARGE_OBJECT_RANGE_LIMIT,
        },
    )
}

fn availability_router(ds: DataSource) -> Router {
    Router::new()
        .route("/leaf/{height}", get(get_leaf_by_height))
        .route("/leaf/hash/{hash}", get(get_leaf_by_hash))
        .route("/leaf/{from}/{until}", get(get_leaf_range))
        .route("/stream/leaves/{height}", get(stream_leaves))
        .route("/header/{height}", get(get_header_by_height))
        .route("/header/hash/{hash}", get(get_header_by_hash))
        .route(
            "/header/payload-hash/{payload_hash}",
            get(get_header_by_payload_hash),
        )
        .route("/header/{from}/{until}", get(get_header_range))
        .route("/stream/headers/{height}", get(stream_headers))
        .route("/block/{height}", get(get_block_by_height))
        .route("/block/hash/{hash}", get(get_block_by_hash))
        .route(
            "/block/payload-hash/{payload_hash}",
            get(get_block_by_payload_hash),
        )
        .route("/block/{from}/{until}", get(get_block_range))
        .route("/stream/blocks/{height}", get(stream_blocks))
        .route("/payload/{height}", get(get_payload_by_height))
        .route("/payload/hash/{hash}", get(get_payload_by_payload_hash))
        .route(
            "/payload/block-hash/{block_hash}",
            get(get_payload_by_block_hash),
        )
        .route("/payload/{from}/{until}", get(get_payload_range))
        .route("/stream/payloads/{height}", get(stream_payloads))
        .route("/vid/common/{height}", get(get_vid_common_by_height))
        .route("/vid/common/hash/{hash}", get(get_vid_common_by_hash))
        .route(
            "/vid/common/payload-hash/{payload_hash}",
            get(get_vid_common_by_payload_hash),
        )
        .route("/vid/common/{from}/{until}", get(get_vid_common_range))
        .route("/stream/vid/common/{height}", get(stream_vid_common))
        .route(
            "/transaction/{height}/{index}/noproof",
            get(get_transaction_by_position),
        )
        .route(
            "/transaction/hash/{hash}/noproof",
            get(get_transaction_by_hash),
        )
        .route(
            "/transaction/{height}/{index}",
            get(get_transaction_proof_by_position),
        )
        .route(
            "/transaction/hash/{hash}",
            get(get_transaction_proof_by_hash),
        )
        .route(
            "/transaction/{height}/{index}/proof",
            get(get_transaction_proof_by_position),
        )
        .route(
            "/transaction/hash/{hash}/proof",
            get(get_transaction_proof_by_hash),
        )
        .route(
            "/stream/transactions/{height}/namespace/{namespace}",
            get(stream_transactions_ns),
        )
        .route("/stream/transactions/{height}", get(stream_transactions))
        .route("/block/summary/{height}", get(get_block_summary_by_height))
        .route(
            "/block/summaries/{from}/{until}",
            get(get_block_summary_range),
        )
        .route("/cert2/{height}", get(get_cert2))
        .route("/limits", get(get_availability_limits))
        .with_state(ds)
}

// --- node ---------------------------------------------------------------------------------------

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
    respond(&headers, result.map_err(ApiError::from))
}

async fn node_count_transactions(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    from: Option<u64>,
    to: Option<u64>,
    namespace: Option<i64>,
) -> Response {
    let result = fetch_count_transactions(&ds, from, to, namespace).await;
    respond(&headers, result.map_err(ApiError::from))
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
    respond(&headers, result.map_err(ApiError::from))
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
    respond(&headers, result.map_err(ApiError::from))
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
    respond(&headers, result.map_err(ApiError::from))
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
    respond(&headers, result.map_err(ApiError::from))
}

async fn node_sync_status(State(ds): State<DataSource>, headers: HeaderMap) -> Response {
    let result = ds.sync_status().await.map_err(node::Error::from);
    respond(&headers, result.map_err(ApiError::from))
}

async fn node_header_window(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path((start, end)): Path<(u64, u64)>,
) -> Response {
    let result = fetch_header_window(&ds, WindowStart::Time(start), end).await;
    respond(&headers, result.map_err(ApiError::from))
}

async fn node_header_window_from_height(
    State(ds): State<DataSource>,
    headers: HeaderMap,
    Path((height, end)): Path<(u64, u64)>,
) -> Response {
    let result = fetch_header_window(&ds, WindowStart::Height(height), end).await;
    respond(&headers, result.map_err(ApiError::from))
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
    respond(&headers, result.map_err(ApiError::from))
}

async fn node_limits(headers: HeaderMap) -> Response {
    encode_ok(
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
    let availability = availability_router(ds.clone());
    let node = node_router(ds);
    Router::new()
        .route("/healthcheck", get(healthcheck))
        .nest("/availability", availability.clone())
        .nest("/v1/availability", availability)
        .nest("/node", node.clone())
        .nest("/v1/node", node)
}
