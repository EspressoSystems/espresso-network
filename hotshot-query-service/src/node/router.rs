// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

//! Axum router serving the node API wire protocol.
//!
//! Route paths, response forms, range semantics, status codes and the wire error envelope (the
//! crate-level [`Error`](crate::Error)) match the old tide-disco handlers and the `node.toml` route
//! specs, so existing clients keep working unchanged. None of these routes streams: the node API is
//! all point queries and aggregates.
//!
//! The router is an [`ApiRouter`] so that the OpenAPI documentation travels with the routes:
//! an application mounting this module gets the summaries and descriptions without restating
//! them. Use [`From`] to get a plain [`Router`] where the docs are not wanted.

use std::{ops::Bound, sync::Arc};

use aide::axum::{ApiRouter, routing::get_with};
use axum::{
    Router,
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
    routing::get,
};
use disco_types::request::RequestError;
use hotshot_types::{
    data::{VidCommitment, VidShare},
    traits::node_implementation::NodeType,
};
use http_wire::{self as wire, body_limit_layer, cors_layer, healthcheck_response};
use serde::Serialize;
use tagged_base64::TaggedBase64;

use super::{
    BlockHash, BlockId, Error, Limits, NodeDataSource, Options, SyncStatusQueryData,
    TimeWindowQueryData, WindowStart,
};
use crate::{Error as AppError, Header, availability::QueryableHeader};

/// The node module's routes: this node's own view of the chain, which may lag the abstract chain
/// the [availability](crate::availability) module presents.
///
/// `options` supplies the header-window limit, which is passed to the data source rather than
/// enforced here: the response reports a truncated window by leaving `next` null, so the client
/// pages through the rest with the `header/window/from/` forms.
pub fn node_router<Types, S>(options: &Options, data_source: S) -> ApiRouter
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    ApiRouter::new()
        .api_route(
            "/block-height",
            get_with(get_block_height::<Types, S>, |op| {
                op.summary("Get this node's block height").description(
                    "Get the current height of the chain as observed by this node. A node which \
                     is not fully synced reports a smaller height than the chain has reached; see \
                     `sync-status`.",
                )
            }),
        )
        .api_route(
            "/sync-status",
            get_with(get_sync_status::<Types, S>, |op| {
                op.summary("Get this node's sync status").description(
                    "Get this node's progress in syncing with the chain: for blocks, leaves and \
                     VID common data, the number of missing objects and the ranges which are \
                     present, missing or pruned.",
                )
            }),
        )
        .api_route(
            "/vid/share/{height}",
            get_with(get_vid_share::<Types, S>, |op| {
                op.summary("Get this node's VID share by height")
                    .description(
                        "Get this node's VID share for the block at the given position in the \
                         ledger, which is what it contributes to the VID reconstruction protocol. \
                         For the data every storage node holds, see the availability API's \
                         `vid/common`.",
                    )
            }),
        )
        .api_route(
            "/vid/share/hash/{hash}",
            get_with(get_vid_share_by_hash::<Types, S>, |op| {
                op.summary("Get this node's VID share by block hash")
                    .description("Get this node's VID share for the block with the given hash.")
            }),
        )
        .api_route(
            "/vid/share/payload-hash/{payload_hash}",
            get_with(get_vid_share_by_payload_hash::<Types, S>, |op| {
                op.summary("Get this node's VID share by payload hash")
                    .description(
                        "Get this node's VID share for a block with the given payload commitment. \
                         Payloads are not unique, so any block with this payload may answer.",
                    )
            }),
        )
        .api_route(
            "/limits",
            get_with(get_limits::<S>, |op| {
                op.summary("Get node limits").description(
                    "Get the implementation-defined limits restricting node API requests. \
                     `window_limit` is the maximum number of headers one `header/window` query \
                     loads.",
                )
            }),
        )
        .merge(transaction_count_router::<Types, S>())
        .merge(payload_size_router::<Types, S>())
        .merge(header_window_router::<Types, S>())
        .with_state(RouterState::new(options, data_source))
}

/// Wraps a node router with the app-level `healthcheck`, a request body limit, and permissive CORS
/// headers. Mounting the module prefix is up to the caller.
pub fn app(api: Router) -> Router {
    Router::new()
        .route(
            "/healthcheck",
            get(|headers: HeaderMap| async move { healthcheck_response(&headers) }),
        )
        .merge(api)
        .layer(body_limit_layer())
        .layer(cors_layer())
}

/// The transaction-count routes, split out only to keep [`node_router`] readable.
fn transaction_count_router<Types, S>() -> ApiRouter<Arc<RouterState<S>>>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    ApiRouter::new()
        .api_route(
            "/transactions/count",
            get_with(count_transactions::<Types, S>, |op| {
                op.summary("Count transactions")
                    .description("Get the number of transactions in the whole chain.")
            }),
        )
        .api_route(
            "/transactions/count/{to}",
            get_with(count_transactions_to::<Types, S>, |op| {
                op.summary("Count transactions up to a height").description(
                    "Get the number of transactions in all blocks up to and including block `to`.",
                )
            }),
        )
        .api_route(
            "/transactions/count/{from}/{to}",
            get_with(count_transactions_from_to::<Types, S>, |op| {
                op.summary("Count transactions in a height range")
                    .description(
                        "Get the number of transactions in the blocks between `from` and `to`, \
                         both inclusive.",
                    )
            }),
        )
        .api_route(
            "/transactions/count/namespace/{namespace}",
            get_with(count_namespace_transactions::<Types, S>, |op| {
                op.summary("Count transactions in a namespace").description(
                    "Get the number of transactions in the whole chain belonging to the given \
                     namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/count/namespace/{namespace}/{to}",
            get_with(count_namespace_transactions_to::<Types, S>, |op| {
                op.summary("Count namespace transactions up to a height")
                    .description(
                        "Get the number of transactions belonging to the given namespace in all \
                         blocks up to and including block `to`.",
                    )
            }),
        )
        .api_route(
            "/transactions/count/namespace/{namespace}/{from}/{to}",
            get_with(count_namespace_transactions_from_to::<Types, S>, |op| {
                op.summary("Count namespace transactions in a height range")
                    .description(
                        "Get the number of transactions belonging to the given namespace in the \
                         blocks between `from` and `to`, both inclusive.",
                    )
            }),
        )
}

/// The payload-size routes, split out only to keep [`node_router`] readable.
fn payload_size_router<Types, S>() -> ApiRouter<Arc<RouterState<S>>>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    ApiRouter::new()
        .api_route(
            "/payloads/size",
            get_with(payload_size::<Types, S>, |op| {
                op.summary("Get total payload size").description(
                    "Get the cumulative size, in bytes, of all payload data in the chain.",
                )
            }),
        )
        .api_route(
            "/payloads/total-size",
            get_with(payload_size::<Types, S>, |op| {
                op.summary("Get total payload size")
                    .description("Deprecated alias for `payloads/size`.")
            }),
        )
        .api_route(
            "/payloads/size/{to}",
            get_with(payload_size_to::<Types, S>, |op| {
                op.summary("Get payload size up to a height").description(
                    "Get the cumulative size, in bytes, of the payloads of all blocks up to and \
                     including block `to`.",
                )
            }),
        )
        .api_route(
            "/payloads/size/{from}/{to}",
            get_with(payload_size_from_to::<Types, S>, |op| {
                op.summary("Get payload size in a height range")
                    .description(
                        "Get the cumulative size, in bytes, of the payloads of the blocks between \
                         `from` and `to`, both inclusive.",
                    )
            }),
        )
        .api_route(
            "/payloads/size/namespace/{namespace}",
            get_with(namespace_payload_size::<Types, S>, |op| {
                op.summary("Get payload size for a namespace").description(
                    "Get the cumulative size, in bytes, of the payload data in the chain \
                     belonging to the given namespace.",
                )
            }),
        )
        .api_route(
            "/payloads/size/namespace/{namespace}/{to}",
            get_with(namespace_payload_size_to::<Types, S>, |op| {
                op.summary("Get namespace payload size up to a height")
                    .description(
                        "Get the cumulative size, in bytes, of the payload data belonging to the \
                         given namespace in all blocks up to and including block `to`.",
                    )
            }),
        )
        .api_route(
            "/payloads/size/namespace/{namespace}/{from}/{to}",
            get_with(namespace_payload_size_from_to::<Types, S>, |op| {
                op.summary("Get namespace payload size in a height range")
                    .description(
                        "Get the cumulative size, in bytes, of the payload data belonging to the \
                         given namespace in the blocks between `from` and `to`, both inclusive.",
                    )
            }),
        )
}

/// The header-window routes, split out only to keep [`node_router`] readable.
fn header_window_router<Types, S>() -> ApiRouter<Arc<RouterState<S>>>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    ApiRouter::new()
        .api_route(
            "/header/window/{start}/{end}",
            get_with(get_header_window::<Types, S>, |op| {
                op.summary("Get a header window by timestamp").description(
                    "Get the available headers whose timestamps fall between `start` (inclusive) \
                     and `end` (exclusive), in order, plus one header on either side of the \
                     window as proof that none inside it were omitted. `next` is null if the \
                     window is incomplete, either because later blocks are not available yet or \
                     because the response hit the `window_limit` from `/limits`; page through the \
                     rest with the `from/` forms. If not even `prev` is available, this fails.",
                )
            }),
        )
        .api_route(
            "/header/window/from/{height}/{end}",
            get_with(get_header_window_from_height::<Types, S>, |op| {
                op.summary("Get a header window from a height").description(
                    "Get the available headers from the block at `height` (inclusive) up to \
                     timestamp `end` (exclusive). Used to page through a window an earlier \
                     request returned incomplete.",
                )
            }),
        )
        .api_route(
            "/header/window/from/hash/{hash}/{end}",
            get_with(get_header_window_from_hash::<Types, S>, |op| {
                op.summary("Get a header window from a block hash")
                    .description(
                        "Get the available headers from the block with the given hash (inclusive) \
                         up to timestamp `end` (exclusive).",
                    )
            }),
        )
}

/// Encode a handler result, wrapping the module error in the crate-level
/// [`Error`](crate::Error) envelope the old tide app served.
fn respond<T: Serialize>(headers: &HeaderMap, result: Result<T, Error>) -> Response {
    wire::respond::<AppError, _>(headers, result.map_err(AppError::from))
}

/// Handler context: the data source plus the header-window limit from [`Options`].
struct RouterState<S> {
    data_source: S,
    window_limit: usize,
}

impl<S> RouterState<S> {
    fn new(options: &Options, data_source: S) -> Arc<Self> {
        Arc::new(Self {
            data_source,
            window_limit: options.window_limit,
        })
    }
}

/// Parses a TaggedBase64 path parameter the way tide-disco's `blob_param` did: any failure is a
/// request error, which maps to 400.
fn tb64_param<T>(value: &str, field: &str) -> Result<T, Error>
where
    T: for<'a> TryFrom<&'a TaggedBase64>,
{
    let err = || Error::Request {
        source: RequestError::TaggedBase64 {
            reason: format!("invalid tagged base 64 for {field}"),
        },
    };
    let tb64: TaggedBase64 = value.parse().map_err(|_| err())?;
    T::try_from(&tb64).map_err(|_| err())
}

// Loaders shared by handlers: report a missing object with the same error variants (and hence
// status codes and messages) as the old handlers.

async fn load_transaction_count<Types, S>(
    state: &RouterState<S>,
    range: (Bound<usize>, Bound<usize>),
    namespace: Option<i64>,
) -> Result<usize, Error>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    state
        .data_source
        .count_transactions_in_range(range, namespace.map(Into::into))
        .await
        .map_err(|source| Error::Query { source })
}

async fn load_payload_size<Types, S>(
    state: &RouterState<S>,
    range: (Bound<usize>, Bound<usize>),
    namespace: Option<i64>,
) -> Result<usize, Error>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    state
        .data_source
        .payload_size_in_range(range, namespace.map(Into::into))
        .await
        .map_err(|source| Error::Query { source })
}

async fn load_vid_share<Types, S>(
    state: &RouterState<S>,
    id: BlockId<Types>,
) -> Result<VidShare, Error>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    state
        .data_source
        .vid_share(id)
        .await
        .map_err(|source| Error::QueryVid {
            source,
            block: id.to_string(),
        })
}

async fn load_header_window<Types, S>(
    state: &RouterState<S>,
    start: WindowStart<Types>,
    end: u64,
) -> Result<TimeWindowQueryData<Header<Types>>, Error>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    state
        .data_source
        .get_header_window(start, end, state.window_limit)
        .await
        .map_err(|source| Error::QueryWindow {
            source,
            start: format!("{start:?}"),
            end,
        })
}

async fn get_block_height<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let height = state.data_source.block_height().await;
    respond(&headers, height.map_err(|source| Error::Query { source }))
}

async fn get_sync_status<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let status: Result<SyncStatusQueryData, _> = state.data_source.sync_status().await;
    respond(&headers, status.map_err(|source| Error::Query { source }))
}

// Transaction-count handlers. `to` and `from` are both inclusive, as the tide handlers had them.

async fn count_transactions<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let range = (Bound::Unbounded, Bound::Unbounded);
    respond(&headers, load_transaction_count(&state, range, None).await)
}

async fn count_transactions_to<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(to): Path<usize>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let range = (Bound::Unbounded, Bound::Included(to));
    respond(&headers, load_transaction_count(&state, range, None).await)
}

async fn count_transactions_from_to<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((from, to)): Path<(usize, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let range = (Bound::Included(from), Bound::Included(to));
    respond(&headers, load_transaction_count(&state, range, None).await)
}

async fn count_namespace_transactions<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(namespace): Path<i64>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let range = (Bound::Unbounded, Bound::Unbounded);
    let count = load_transaction_count(&state, range, Some(namespace)).await;
    respond(&headers, count)
}

async fn count_namespace_transactions_to<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((namespace, to)): Path<(i64, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let range = (Bound::Unbounded, Bound::Included(to));
    let count = load_transaction_count(&state, range, Some(namespace)).await;
    respond(&headers, count)
}

async fn count_namespace_transactions_from_to<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((namespace, from, to)): Path<(i64, usize, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let range = (Bound::Included(from), Bound::Included(to));
    let count = load_transaction_count(&state, range, Some(namespace)).await;
    respond(&headers, count)
}

// Payload-size handlers, with the same inclusive bounds as the transaction counts.

async fn payload_size<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let range = (Bound::Unbounded, Bound::Unbounded);
    respond(&headers, load_payload_size(&state, range, None).await)
}

async fn payload_size_to<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(to): Path<usize>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let range = (Bound::Unbounded, Bound::Included(to));
    respond(&headers, load_payload_size(&state, range, None).await)
}

async fn payload_size_from_to<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((from, to)): Path<(usize, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let range = (Bound::Included(from), Bound::Included(to));
    respond(&headers, load_payload_size(&state, range, None).await)
}

async fn namespace_payload_size<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(namespace): Path<i64>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let range = (Bound::Unbounded, Bound::Unbounded);
    let size = load_payload_size(&state, range, Some(namespace)).await;
    respond(&headers, size)
}

async fn namespace_payload_size_to<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((namespace, to)): Path<(i64, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let range = (Bound::Unbounded, Bound::Included(to));
    let size = load_payload_size(&state, range, Some(namespace)).await;
    respond(&headers, size)
}

async fn namespace_payload_size_from_to<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((namespace, from, to)): Path<(i64, usize, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let range = (Bound::Included(from), Bound::Included(to));
    let size = load_payload_size(&state, range, Some(namespace)).await;
    respond(&headers, size)
}

// VID share handlers.

async fn get_vid_share<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    respond(
        &headers,
        load_vid_share(&state, BlockId::Number(height)).await,
    )
}

async fn get_vid_share_by_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let result = async {
        let hash = tb64_param::<BlockHash<Types>>(&hash, "hash")?;
        load_vid_share(&state, BlockId::Hash(hash)).await
    }
    .await;
    respond(&headers, result)
}

async fn get_vid_share_by_payload_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let result = async {
        let hash = tb64_param::<VidCommitment>(&hash, "payload-hash")?;
        load_vid_share(&state, BlockId::PayloadHash(hash)).await
    }
    .await;
    respond(&headers, result)
}

// Header window handlers.

async fn get_header_window<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((start, end)): Path<(u64, u64)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let window = load_header_window(&state, WindowStart::Time(start), end).await;
    respond(&headers, window)
}

async fn get_header_window_from_height<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((height, end)): Path<(u64, u64)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let window = load_header_window(&state, WindowStart::Height(height), end).await;
    respond(&headers, window)
}

async fn get_header_window_from_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((hash, end)): Path<(String, u64)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    S: NodeDataSource<Types> + Send + Sync + 'static,
{
    let result = async {
        let hash = tb64_param::<BlockHash<Types>>(&hash, "hash")?;
        load_header_window(&state, WindowStart::Hash(hash), end).await
    }
    .await;
    respond(&headers, result)
}

async fn get_limits<S>(State(state): State<Arc<RouterState<S>>>, headers: HeaderMap) -> Response
where
    S: Send + Sync + 'static,
{
    respond(
        &headers,
        Ok(Limits {
            window_limit: state.window_limit,
        }),
    )
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use committable::Committable;
    use futures::StreamExt;
    use hotshot_types::{
        data::VidDisperseShare,
        event::{EventType, LeafInfo},
        traits::{
            EncodeBytes,
            block_contents::{BlockHeader, BlockPayload},
        },
    };
    use http_client::{Client, ClientError};
    use test_utils::reserve_tcp_port;

    use super::*;
    use crate::{
        QueryError,
        testing::{
            consensus::{MockDataSource, MockNetwork, MockSqlDataSource},
            mocks::{MockBase, MockTypes, mock_transaction},
            sleep,
        },
    };

    /// Every path the router registers, in the form the OpenAPI spec reports them.
    const ROUTES: [&str; 22] = [
        "/block-height",
        "/sync-status",
        "/limits",
        "/transactions/count",
        "/transactions/count/{to}",
        "/transactions/count/{from}/{to}",
        "/transactions/count/namespace/{namespace}",
        "/transactions/count/namespace/{namespace}/{to}",
        "/transactions/count/namespace/{namespace}/{from}/{to}",
        "/payloads/size",
        "/payloads/total-size",
        "/payloads/size/{to}",
        "/payloads/size/{from}/{to}",
        "/payloads/size/namespace/{namespace}",
        "/payloads/size/namespace/{namespace}/{to}",
        "/payloads/size/namespace/{namespace}/{from}/{to}",
        "/vid/share/{height}",
        "/vid/share/hash/{hash}",
        "/vid/share/payload-hash/{payload_hash}",
        "/header/window/{start}/{end}",
        "/header/window/from/{height}/{end}",
        "/header/window/from/hash/{hash}/{end}",
    ];

    /// Serve `api` under the `/node` prefix on a fresh port and return a connected client rooted at
    /// that prefix.
    async fn start_client(api: ApiRouter) -> Client<AppError, MockBase> {
        let port = reserve_tcp_port().unwrap();
        let url = format!("http://0.0.0.0:{port}").parse().unwrap();
        let _server = wire::spawn_serve(&url, app(Router::new().nest("/node", Router::from(api))));

        let client = Client::new(format!("http://localhost:{port}/node").parse().unwrap());
        assert!(client.connect(Some(Duration::from_secs(60))).await);
        client
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_api() {
        let window_limit = 78;

        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;
        let mut events = network.handle().event_stream();
        network.start().await;

        let client = start_client(node_router::<MockTypes, _>(
            &Options { window_limit },
            network.data_source(),
        ))
        .await;

        // Check limits endpoint.
        assert_eq!(
            client.get::<Limits>("limits").send().await.unwrap(),
            Limits { window_limit }
        );

        // Wait until a few blocks have been sequenced.
        let block_height = loop {
            let block_height = client.get::<usize>("block-height").send().await.unwrap();
            if block_height > network.num_nodes() {
                break block_height;
            }
            sleep(Duration::from_secs(1)).await;
        };

        // We test these counters with non-trivial values in `data_source.rs`, here we just want to
        // make sure the API handlers are working, so a response of 0 is fine.
        assert_eq!(
            client
                .get::<u64>("transactions/count")
                .send()
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            client
                .get::<u64>("payloads/total-size")
                .send()
                .await
                .unwrap(),
            0
        );

        let mut headers = vec![];

        // Get VID share for each block.
        tracing::info!(block_height, "checking VID shares");
        'outer: while let Some(event) = events.next().await {
            let EventType::Decide { leaf_chain, .. } = event.event else {
                continue;
            };
            for LeafInfo {
                leaf, vid_share, ..
            } in leaf_chain.iter().rev()
            {
                headers.push(leaf.block_header().clone());
                if leaf.block_header().block_number >= block_height as u64 {
                    break 'outer;
                }
                tracing::info!(height = leaf.block_header().block_number, "checking share");

                let share = client
                    .get::<VidShare>(&format!("vid/share/{}", leaf.block_header().block_number))
                    .send()
                    .await
                    .unwrap();
                if let Some(vid_share) = vid_share.as_ref() {
                    let VidDisperseShare::V0(new_share) = vid_share else {
                        panic!("VID share is not V0");
                    };
                    assert_eq!(share, VidShare::V0(new_share.share.clone()));
                }

                // Query various other ways.
                assert_eq!(
                    share,
                    client
                        .get(&format!("vid/share/hash/{}", leaf.block_header().commit()))
                        .send()
                        .await
                        .unwrap()
                );
                assert_eq!(
                    share,
                    client
                        .get(&format!(
                            "vid/share/payload-hash/{}",
                            leaf.block_header().payload_commitment
                        ))
                        .send()
                        .await
                        .unwrap()
                );
            }
        }

        // Check time window queries. The various edge cases are thoroughly tested for each
        // individual data source. In this test, we just smoketest API parameter handling. Sleep 2
        // seconds to ensure a new header is produced with a timestamp after the latest one in
        // `headers`
        sleep(Duration::from_secs(2)).await;
        let first_header = &headers[0];
        let last_header = &headers.last().unwrap();
        let window: TimeWindowQueryData<Header<MockTypes>> = client
            .get(&format!(
                "header/window/{}/{}",
                first_header.timestamp,
                last_header.timestamp + 1
            ))
            .send()
            .await
            .unwrap();
        assert!(window.window.contains(first_header));
        assert!(window.window.contains(last_header));
        assert!(window.next.is_some());

        // Query for the same window other ways.
        assert_eq!(
            window,
            client
                .get(&format!(
                    "header/window/from/0/{}",
                    last_header.timestamp + 1
                ))
                .send()
                .await
                .unwrap()
        );
        assert_eq!(
            window,
            client
                .get(&format!(
                    "header/window/from/hash/{}/{}",
                    first_header.commit(),
                    last_header.timestamp + 1
                ))
                .send()
                .await
                .unwrap()
        );

        // In this simple test, the node should be fully synchronized.
        let sync_status = client
            .get::<SyncStatusQueryData>("sync-status")
            .send()
            .await
            .unwrap();
        assert!(sync_status.is_fully_synced(), "{sync_status:#?}");

        network.shut_down().await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_aggregate_ranges() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockSqlDataSource>::init().await;
        let mut events = network.handle().event_stream();
        network.start().await;

        let client = start_client(node_router::<MockTypes, _>(
            &Default::default(),
            network.data_source(),
        ))
        .await;

        // Wait until a few transactions have been sequenced.
        let mut tx_heights = vec![];
        let mut tx_sizes = vec![];
        for i in [1, 2] {
            let txn = mock_transaction(vec![0; i]);
            let hash = txn.commit();

            network.submit_transaction(txn).await;

            let leaf = 'outer: loop {
                let EventType::Decide { leaf_chain, .. } = events.next().await.unwrap().event
                else {
                    continue;
                };
                for info in leaf_chain.iter().rev() {
                    let leaf = &info.leaf;
                    if BlockPayload::<MockTypes>::transaction_commitments(
                        &leaf.block_payload().unwrap(),
                        BlockHeader::<MockTypes>::metadata(leaf.block_header()),
                    )
                    .contains(&hash)
                    {
                        break 'outer leaf.clone();
                    }
                }

                tracing::info!("waiting for tx {i}");
                sleep(Duration::from_secs(1)).await;
            };
            tx_heights.push(leaf.height());
            tx_sizes.push(leaf.block_payload().unwrap().encode().len());
        }
        tracing::info!(?tx_heights, ?tx_sizes, "transactions sequenced");

        // Wait for the aggregator to process the inserted blocks.
        while let Err(err) = client
            .get::<usize>(&format!("transactions/count/{}", tx_heights[1]))
            .send()
            .await
        {
            if ClientError::status(&err) == http_client::StatusCode::NOT_FOUND {
                tracing::info!(?tx_heights, "waiting for aggregator");
                sleep(Duration::from_secs(1)).await;
                continue;
            } else {
                panic!("unexpected error: {err:#}");
            }
        }

        // Range including empty blocks (genesis block) only
        assert_eq!(
            0,
            client
                .get::<usize>("transactions/count/0")
                .send()
                .await
                .unwrap()
        );
        assert_eq!(
            0,
            client.get::<usize>("payloads/size/0").send().await.unwrap()
        );

        // First transaction only
        assert_eq!(
            1,
            client
                .get::<usize>(&format!("transactions/count/{}", tx_heights[0]))
                .send()
                .await
                .unwrap()
        );
        assert_eq!(
            tx_sizes[0],
            client
                .get::<usize>(&format!("payloads/size/{}", tx_heights[0]))
                .send()
                .await
                .unwrap()
        );

        // Last transaction only
        assert_eq!(
            1,
            client
                .get::<usize>(&format!(
                    "transactions/count/{}/{}",
                    tx_heights[0] + 1,
                    tx_heights[1]
                ))
                .send()
                .await
                .unwrap()
        );
        assert_eq!(
            tx_sizes[1],
            client
                .get::<usize>(&format!(
                    "payloads/size/{}/{}",
                    tx_heights[0] + 1,
                    tx_heights[1]
                ))
                .send()
                .await
                .unwrap()
        );

        // All transactions
        assert_eq!(
            2,
            client
                .get::<usize>("transactions/count")
                .send()
                .await
                .unwrap()
        );
        assert_eq!(
            tx_sizes[0] + tx_sizes[1],
            client.get::<usize>("payloads/size").send().await.unwrap()
        );

        network.shut_down().await;
    }

    /// An application mounts this router next to its own extensions of the same module, which is
    /// what tide-disco's `extensions` did. Merging must leave both reachable.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn extensions_mount_alongside_the_base() {
        let mut network = MockNetwork::<MockDataSource>::init().await;
        network.start().await;

        let api = node_router::<MockTypes, _>(&Default::default(), network.data_source())
            .api_route(
                "/ext",
                get_with(async || axum::Json(42u64), |op| op.summary("Extension")),
            );
        let client = start_client(api).await;

        assert_eq!(client.get::<u64>("ext").send().await.unwrap(), 42);
        let sync_status: SyncStatusQueryData = client.get("sync-status").send().await.unwrap();
        assert!(sync_status.is_fully_synced(), "{sync_status:#?}");

        network.shut_down().await;
    }

    /// A malformed TaggedBase64 parameter is the client's fault, so it must be reported as a
    /// request error (a 400), not as a missing object.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn malformed_hash_is_a_request_error() {
        let network = MockNetwork::<MockDataSource>::init().await;
        let client = start_client(node_router::<MockTypes, _>(
            &Default::default(),
            network.data_source(),
        ))
        .await;

        for route in [
            "vid/share/hash/not-a-hash",
            "vid/share/payload-hash/not-a-hash",
            "header/window/from/hash/not-a-hash/100",
        ] {
            let err = client.get::<()>(route).send().await.unwrap_err();
            assert!(
                matches!(
                    err,
                    AppError::Node {
                        source: Error::Request { .. }
                    }
                ),
                "{route}: {err:?}"
            );
        }

        network.shut_down().await;
    }

    /// A VID share this node does not have is reported by the route's own error variant, so
    /// clients can tell it apart from a plain query failure.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn missing_vid_share_reports_the_block() {
        let network = MockNetwork::<MockDataSource>::init().await;
        let client = start_client(node_router::<MockTypes, _>(
            &Default::default(),
            network.data_source(),
        ))
        .await;

        let err = client
            .get::<VidShare>("vid/share/1000000")
            .send()
            .await
            .unwrap_err();
        let AppError::Node {
            source: Error::QueryVid { source, block },
        } = &err
        else {
            panic!("unexpected error {err:?}");
        };
        assert_eq!(block, "1000000");
        assert!(matches!(source, QueryError::Missing | QueryError::NotFound));

        network.shut_down().await;
    }

    /// Applications mount this router and serve its documentation as part of their own OpenAPI
    /// spec, so every route it registers must carry a summary.
    #[tokio::test]
    async fn router_documents_every_route() {
        let dir = tempfile::TempDir::new().unwrap();
        let data_source = MockDataSource::create(dir.path(), Default::default())
            .await
            .unwrap();

        let mut api = aide::openapi::OpenApi::default();
        let _ = node_router::<MockTypes, _>(&Options::default(), data_source).finish_api(&mut api);

        let paths = api.paths.expect("router registered paths");
        assert_eq!(paths.paths.len(), ROUTES.len());
        for route in ROUTES {
            let aide::openapi::ReferenceOr::Item(item) = &paths.paths[route] else {
                panic!("{route} is a reference, not an operation");
            };
            let op = item
                .get
                .as_ref()
                .unwrap_or_else(|| panic!("{route} has no GET"));
            assert!(op.summary.is_some(), "{route} has no summary");
            assert!(op.description.is_some(), "{route} has no description");
        }
    }
}
