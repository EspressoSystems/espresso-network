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

//! Axum router serving the explorer API wire protocol.
//!
//! Route paths, response forms, limit validation, status codes and the wire error envelope (the
//! crate-level [`Error`](crate::Error)) match the old tide-disco handlers and the `explorer.toml`
//! route specs, so existing clients keep working unchanged. Every error body carries the `code`
//! that spec requires: a client can identify the failure from that field alone.
//!
//! The router is an [`ApiRouter`] so that the OpenAPI documentation travels with the routes:
//! an application mounting this module gets the summaries and descriptions without restating
//! them. Use [`From`] to get a plain [`Router`] where the docs are not wanted.

use std::{num::NonZeroUsize, sync::Arc};

use aide::axum::{ApiRouter, routing::get_with};
use axum::{
    Router,
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
    routing::get,
};
use hotshot_types::traits::node_implementation::NodeType;
use http_wire::{self as wire, body_limit_layer, cors_layer, healthcheck_response};
use serde::Serialize;
use tagged_base64::TaggedBase64;

use super::{
    BlockDetailResponse, BlockIdentifier, BlockRange, BlockSummaryResponse, Error,
    ExplorerDataSource, ExplorerHeader, ExplorerSummaryResponse, ExplorerTransaction,
    GetBlockDetailError, GetBlockSummariesError, GetBlockSummariesRequest, GetSearchResultsError,
    GetTransactionDetailError, GetTransactionSummariesError, GetTransactionSummariesRequest,
    Options, SearchResultResponse, TransactionDetailResponse, TransactionIdentifier,
    TransactionRange, TransactionSummariesResponse, TransactionSummaryFilter,
    errors::{BadQuery, InvalidLimit, NotFound},
};
use crate::{
    Error as AppError, Header, Payload, Transaction,
    availability::{BlockHash, QueryableHeader, QueryablePayload, TransactionHash},
};

/// The explorer module's routes: block and transaction detail views, descending summary listings,
/// the chain overview, and hash search.
///
/// Unlike [availability](crate::availability), these routes serve whatever storage already holds:
/// nothing here fetches missing data, so a response is a snapshot of the current state.
pub fn explorer_router<Types, S>(options: &Options, data_source: S) -> ApiRouter
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    ApiRouter::new()
        .api_route(
            "/block/{height}",
            get_with(get_block_detail::<Types, S>, |op| {
                op.summary("Get block detail by height").description(
                    "Get the explorer's detail view of the block at the given position in the \
                     ledger.",
                )
            }),
        )
        .api_route(
            "/block/hash/{hash}",
            get_with(get_block_detail_by_hash::<Types, S>, |op| {
                op.summary("Get block detail by hash").description(
                    "Get the explorer's detail view of the block with the given commitment.",
                )
            }),
        )
        .api_route(
            "/blocks/latest/{limit}",
            get_with(get_latest_block_summaries::<Types, S>, |op| {
                op.summary("Get the latest block summaries").description(
                    "Get up to `limit` block summaries ending at the latest block, in descending \
                     height order. `limit` must be between 1 and 100.",
                )
            }),
        )
        .api_route(
            "/blocks/{from}/{limit}",
            get_with(get_block_summaries::<Types, S>, |op| {
                op.summary("Get block summaries from a height").description(
                    "Get up to `limit` block summaries ending at the block at `from`, in \
                     descending height order. `limit` must be between 1 and 100.",
                )
            }),
        )
        .api_route(
            "/transaction/{height}/{offset}",
            get_with(get_transaction_detail::<Types, S>, |op| {
                op.summary("Get transaction detail by position")
                    .description(
                        "Get the explorer's detail view of the transaction at `offset` within the \
                         block at `height`.",
                    )
            }),
        )
        .api_route(
            "/transaction/hash/{hash}",
            get_with(get_transaction_detail_by_hash::<Types, S>, |op| {
                op.summary("Get transaction detail by hash").description(
                    "Get the explorer's detail view of the transaction with the given commitment.",
                )
            }),
        )
        .api_route(
            "/explorer-summary",
            get_with(get_explorer_summary::<Types, S>, |op| {
                op.summary("Get the explorer summary").description(
                    "Get the chain overview an explorer landing page shows: totals since genesis, \
                     the latest block, the most recent blocks and transactions, and histograms of \
                     recent block size, time and transaction count.",
                )
            }),
        )
        .api_route(
            "/search/{query}",
            get_with(get_search_results::<Types, S>, |op| {
                op.summary("Search blocks and transactions").description(
                    "Get the blocks and transactions matching the query, which is matched against \
                     hashes only.",
                )
            }),
        )
        .merge(transaction_summaries_router::<Types, S>())
        .with_state(RouterState::new(options, data_source))
}

/// Wraps an explorer router with the app-level `healthcheck`, a request body limit, and permissive
/// CORS headers. Mounting the module prefix is up to the caller.
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

/// Encode a handler result, wrapping the module error in the crate-level
/// [`Error`](crate::Error) envelope the old tide app served.
fn respond<T: Serialize>(headers: &HeaderMap, result: Result<T, Error>) -> Response {
    wire::respond::<AppError, _>(headers, result.map_err(AppError::from))
}

/// Handler context: the data source. [`Options`] carries no settings yet; the router takes it for
/// symmetry with the other modules, and a setting added later lands here rather than in every
/// handler.
struct RouterState<S> {
    data_source: S,
}

impl<S> RouterState<S> {
    fn new(_options: &Options, data_source: S) -> Arc<Self> {
        Arc::new(Self { data_source })
    }
}

/// The transaction-summary listings: one route per target (latest, position, hash) and filter
/// (unfiltered, one block, one namespace), split out only to keep [`explorer_router`] readable.
fn transaction_summaries_router<Types, S>() -> ApiRouter<Arc<RouterState<S>>>
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    ApiRouter::new()
        .api_route(
            "/transactions/latest/{limit}",
            get_with(get_latest_transaction_summaries::<Types, S>, |op| {
                op.summary("Get the latest transaction summaries")
                    .description(
                        "Get up to `limit` transaction summaries ending at the last transaction \
                         of the highest block, in descending order. `limit` must be between 1 and \
                         100.",
                    )
            }),
        )
        .api_route(
            "/transactions/from/{height}/{offset}/{limit}",
            get_with(get_transaction_summaries::<Types, S>, |op| {
                op.summary("Get transaction summaries from a position")
                    .description(
                        "Get up to `limit` transaction summaries ending at the transaction at \
                         `offset` within the block at `height`, in descending order. `limit` must \
                         be between 1 and 100.",
                    )
            }),
        )
        .api_route(
            "/transactions/hash/{hash}/{limit}",
            get_with(get_transaction_summaries_by_hash::<Types, S>, |op| {
                op.summary("Get transaction summaries from a hash")
                    .description(
                        "Get up to `limit` transaction summaries ending at the transaction with \
                         the given commitment, in descending order. `limit` must be between 1 and \
                         100.",
                    )
            }),
        )
        .api_route(
            "/transactions/latest/{limit}/block/{block}",
            get_with(
                get_latest_transaction_summaries_in_block::<Types, S>,
                |op| {
                    op.summary("Get the latest transaction summaries in a block")
                        .description(
                            "Get up to `limit` transaction summaries from the block at `block`, \
                             ending at the last transaction of the highest block.",
                        )
                },
            ),
        )
        .api_route(
            "/transactions/from/{height}/{offset}/{limit}/block/{block}",
            get_with(get_transaction_summaries_in_block::<Types, S>, |op| {
                op.summary("Get transaction summaries in a block from a position")
                    .description(
                        "Get up to `limit` transaction summaries from the block at `block`, \
                         ending at the transaction at `offset` within the block at `height`.",
                    )
            }),
        )
        .api_route(
            "/transactions/hash/{hash}/{limit}/block/{block}",
            get_with(
                get_transaction_summaries_in_block_by_hash::<Types, S>,
                |op| {
                    op.summary("Get transaction summaries in a block from a hash")
                        .description(
                            "Get up to `limit` transaction summaries from the block at `block`, \
                             ending at the transaction with the given commitment.",
                        )
                },
            ),
        )
        .api_route(
            "/transactions/latest/{limit}/namespace/{namespace}",
            get_with(
                get_latest_transaction_summaries_in_namespace::<Types, S>,
                |op| {
                    op.summary("Get the latest transaction summaries in a namespace")
                        .description(
                            "Get up to `limit` transaction summaries belonging to `namespace`, \
                             ending at the last transaction of the highest block.",
                        )
                },
            ),
        )
        .api_route(
            "/transactions/from/{height}/{offset}/{limit}/namespace/{namespace}",
            get_with(get_transaction_summaries_in_namespace::<Types, S>, |op| {
                op.summary("Get transaction summaries in a namespace from a position")
                    .description(
                        "Get up to `limit` transaction summaries belonging to `namespace`, ending \
                         at the transaction at `offset` within the block at `height`.",
                    )
            }),
        )
        .api_route(
            "/transactions/hash/{hash}/{limit}/namespace/{namespace}",
            get_with(
                get_transaction_summaries_in_namespace_by_hash::<Types, S>,
                |op| {
                    op.summary("Get transaction summaries in a namespace from a hash")
                        .description(
                            "Get up to `limit` transaction summaries belonging to `namespace`, \
                             ending at the transaction with the given commitment.",
                        )
                },
            ),
        )
}

/// The listing routes serve at most 100 objects and have no default: a missing, zero or oversized
/// `limit` is `INVALID_LIMIT`.
fn validate_limit(limit: usize) -> Result<NonZeroUsize, InvalidLimit> {
    let limit = NonZeroUsize::new(limit).ok_or(InvalidLimit {})?;
    if limit.get() > 100 {
        return Err(InvalidLimit {});
    }
    Ok(limit)
}

/// Parses a TaggedBase64 path parameter the way tide-disco's `blob_param` did. The module error
/// has no request-error variant, so each caller reports a failure as its own resource being
/// unidentifiable.
fn tb64_param<T>(value: &str) -> Option<T>
where
    T: for<'a> TryFrom<&'a TaggedBase64>,
{
    let tb64: TaggedBase64 = value.parse().ok()?;
    T::try_from(&tb64).ok()
}

// Loaders shared by handlers: run one data source query and wrap its dedicated error type in the
// matching module error variant, so every path reports the same status and code as the old
// handlers did.

async fn load_block_detail<Types, S>(
    state: &RouterState<S>,
    target: BlockIdentifier<Types>,
) -> Result<BlockDetailResponse<Types>, Error>
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    state
        .data_source
        .get_block_detail(target)
        .await
        .map(BlockDetailResponse::from)
        .map_err(Error::GetBlockDetail)
}

async fn load_block_summaries<Types, S>(
    state: &RouterState<S>,
    target: BlockIdentifier<Types>,
    limit: usize,
) -> Result<BlockSummaryResponse<Types>, Error>
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let num_blocks = validate_limit(limit)
        .map_err(GetBlockSummariesError::InvalidLimit)
        .map_err(Error::GetBlockSummaries)?;
    state
        .data_source
        .get_block_summaries(GetBlockSummariesRequest(BlockRange { target, num_blocks }))
        .await
        .map(BlockSummaryResponse::from)
        .map_err(Error::GetBlockSummaries)
}

async fn load_transaction_detail<Types, S>(
    state: &RouterState<S>,
    target: TransactionIdentifier<Types>,
) -> Result<TransactionDetailResponse<Types>, Error>
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    state
        .data_source
        .get_transaction_detail(target)
        .await
        .map(TransactionDetailResponse::from)
        .map_err(Error::GetTransactionDetail)
}

/// `target` is a `Result` so that the limit is reported before it, the order the tide handler
/// checked them in: the hash routes are the only ones whose target can fail.
async fn load_transaction_summaries<Types, S>(
    state: &RouterState<S>,
    target: Result<TransactionIdentifier<Types>, Error>,
    limit: usize,
    filter: TransactionSummaryFilter<Types>,
) -> Result<TransactionSummariesResponse<Types>, Error>
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let num_transactions = validate_limit(limit)
        .map_err(GetTransactionSummariesError::InvalidLimit)
        .map_err(Error::GetTransactionSummaries)?;
    let target = target?;
    state
        .data_source
        .get_transaction_summaries(GetTransactionSummariesRequest {
            range: TransactionRange {
                target,
                num_transactions,
            },
            filter,
        })
        .await
        .map(TransactionSummariesResponse::from)
        .map_err(Error::GetTransactionSummaries)
}

/// Resolve the `transactions/hash/{hash}` target, which every by-hash listing route shares.
fn transaction_hash_target<Types>(hash: &str) -> Result<TransactionIdentifier<Types>, Error>
where
    Types: NodeType,
{
    tb64_param::<TransactionHash<Types>>(hash)
        .map(TransactionIdentifier::Hash)
        .ok_or_else(|| {
            Error::GetTransactionSummaries(GetTransactionSummariesError::TargetNotFound(NotFound {
                key: format!("hash {hash}"),
            }))
        })
}

// Block handlers.

async fn get_block_detail<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let result = load_block_detail(&state, BlockIdentifier::Height(height)).await;
    respond(&headers, result)
}

async fn get_block_detail_by_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let result = async {
        let hash = tb64_param::<BlockHash<Types>>(&hash).ok_or_else(|| {
            Error::GetBlockDetail(GetBlockDetailError::BlockNotFound(NotFound {
                key: format!("hash {hash}"),
            }))
        })?;
        load_block_detail(&state, BlockIdentifier::Hash(hash)).await
    }
    .await;
    respond(&headers, result)
}

async fn get_latest_block_summaries<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(limit): Path<usize>,
) -> Response
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let result = load_block_summaries(&state, BlockIdentifier::Latest, limit).await;
    respond(&headers, result)
}

async fn get_block_summaries<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((from, limit)): Path<(usize, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let result = load_block_summaries(&state, BlockIdentifier::Height(from), limit).await;
    respond(&headers, result)
}

// Transaction detail handlers.

async fn get_transaction_detail<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((height, offset)): Path<(usize, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let result = load_transaction_detail(
        &state,
        TransactionIdentifier::HeightAndOffset(height, offset),
    )
    .await;
    respond(&headers, result)
}

async fn get_transaction_detail_by_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let result = async {
        let hash = tb64_param::<TransactionHash<Types>>(&hash).ok_or_else(|| {
            Error::GetTransactionDetail(GetTransactionDetailError::TransactionNotFound(NotFound {
                key: format!("hash {hash}"),
            }))
        })?;
        load_transaction_detail(&state, TransactionIdentifier::Hash(hash)).await
    }
    .await;
    respond(&headers, result)
}

// Transaction summary handlers, one per target and filter pair.

async fn get_latest_transaction_summaries<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(limit): Path<usize>,
) -> Response
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let result = load_transaction_summaries(
        &state,
        Ok(TransactionIdentifier::Latest),
        limit,
        TransactionSummaryFilter::None,
    )
    .await;
    respond(&headers, result)
}

async fn get_transaction_summaries<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((height, offset, limit)): Path<(usize, usize, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let result = load_transaction_summaries(
        &state,
        Ok(TransactionIdentifier::HeightAndOffset(height, offset)),
        limit,
        TransactionSummaryFilter::None,
    )
    .await;
    respond(&headers, result)
}

async fn get_transaction_summaries_by_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((hash, limit)): Path<(String, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let result = load_transaction_summaries(
        &state,
        transaction_hash_target(&hash),
        limit,
        TransactionSummaryFilter::None,
    )
    .await;
    respond(&headers, result)
}

async fn get_latest_transaction_summaries_in_block<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((limit, block)): Path<(usize, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let result = load_transaction_summaries(
        &state,
        Ok(TransactionIdentifier::Latest),
        limit,
        TransactionSummaryFilter::Block(block),
    )
    .await;
    respond(&headers, result)
}

async fn get_transaction_summaries_in_block<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((height, offset, limit, block)): Path<(usize, usize, usize, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let result = load_transaction_summaries(
        &state,
        Ok(TransactionIdentifier::HeightAndOffset(height, offset)),
        limit,
        TransactionSummaryFilter::Block(block),
    )
    .await;
    respond(&headers, result)
}

async fn get_transaction_summaries_in_block_by_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((hash, limit, block)): Path<(String, usize, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let result = load_transaction_summaries(
        &state,
        transaction_hash_target(&hash),
        limit,
        TransactionSummaryFilter::Block(block),
    )
    .await;
    respond(&headers, result)
}

async fn get_latest_transaction_summaries_in_namespace<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((limit, namespace)): Path<(usize, i64)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let result = load_transaction_summaries(
        &state,
        Ok(TransactionIdentifier::Latest),
        limit,
        TransactionSummaryFilter::RollUp(namespace.into()),
    )
    .await;
    respond(&headers, result)
}

async fn get_transaction_summaries_in_namespace<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((height, offset, limit, namespace)): Path<(usize, usize, usize, i64)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let result = load_transaction_summaries(
        &state,
        Ok(TransactionIdentifier::HeightAndOffset(height, offset)),
        limit,
        TransactionSummaryFilter::RollUp(namespace.into()),
    )
    .await;
    respond(&headers, result)
}

async fn get_transaction_summaries_in_namespace_by_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((hash, limit, namespace)): Path<(String, usize, i64)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let result = load_transaction_summaries(
        &state,
        transaction_hash_target(&hash),
        limit,
        TransactionSummaryFilter::RollUp(namespace.into()),
    )
    .await;
    respond(&headers, result)
}

// Chain-wide handlers.

async fn get_explorer_summary<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
) -> Response
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let result = state
        .data_source
        .get_explorer_summary()
        .await
        .map(ExplorerSummaryResponse::from)
        .map_err(Error::GetExplorerSummary);
    respond(&headers, result)
}

/// The query reaches the data source as a raw TaggedBase64: the explorer decides for itself which
/// kinds of hash it matches.
async fn get_search_results<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(query): Path<String>,
) -> Response
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: ExplorerDataSource<Types> + Send + Sync + 'static,
{
    let result = async {
        let query: TaggedBase64 = query.parse().map_err(|_| {
            Error::GetSearchResults(GetSearchResultsError::InvalidQuery(BadQuery {}))
        })?;
        state
            .data_source
            .get_search_results(query)
            .await
            .map(SearchResultResponse::from)
            .map_err(Error::GetSearchResults)
    }
    .await;
    respond(&headers, result)
}

#[cfg(test)]
mod test {
    use std::{cmp::min, time::Duration};

    use disco_types::status::StatusCode;
    use futures::StreamExt;
    use http_client::Client;
    use test_utils::reserve_tcp_port;

    use super::{
        super::{
            BlockDetail, BlockSummary, ExplorerSummary, GenesisOverview, GetExplorerSummaryError,
            SearchResult, TransactionSummary,
        },
        *,
    };
    use crate::{
        availability::{self, BlockQueryData, router::availability_router},
        testing::{
            consensus::{MockNetwork, MockSqlDataSource},
            mocks::{MockBase, MockTypes, mock_transaction},
        },
    };

    const NUM_BLOCKS: usize = 10;
    const NUM_TXNS_PER_BLOCK: usize = 5;

    /// Serve the explorer and availability routers on a fresh port, each under its own prefix, and
    /// return a client rooted at each. The availability socket is how the test observes blocks
    /// being sequenced; the explorer routes serve only what storage already holds.
    async fn start_clients(
        explorer: ApiRouter,
        availability: ApiRouter,
    ) -> (Client<AppError, MockBase>, Client<AppError, MockBase>) {
        let port = reserve_tcp_port().unwrap();
        let url = format!("http://0.0.0.0:{port}").parse().unwrap();
        let _server = wire::spawn_serve(
            &url,
            app(Router::new()
                .nest("/explorer", Router::from(explorer))
                .nest("/availability", Router::from(availability))),
        );

        let explorer_client =
            Client::new(format!("http://localhost:{port}/explorer").parse().unwrap());
        let availability_client = Client::new(
            format!("http://localhost:{port}/availability")
                .parse()
                .unwrap(),
        );
        assert!(
            availability_client
                .connect(Some(Duration::from_secs(60)))
                .await
        );
        (explorer_client, availability_client)
    }

    async fn validate(client: &Client<AppError, MockBase>) {
        let explorer_summary_response: ExplorerSummaryResponse<MockTypes> =
            client.get("explorer-summary").send().await.unwrap();

        let ExplorerSummary {
            histograms,
            latest_block,
            latest_blocks,
            latest_transactions,
            genesis_overview,
            ..
        } = explorer_summary_response.explorer_summary;

        let GenesisOverview {
            blocks: num_blocks,
            transactions: num_transactions,
            ..
        } = genesis_overview;

        assert!(num_blocks > 0);
        assert_eq!(histograms.block_heights.len(), min(num_blocks as usize, 50));
        assert_eq!(histograms.block_size.len(), histograms.block_heights.len());
        assert_eq!(histograms.block_time.len(), histograms.block_heights.len());
        assert_eq!(
            histograms.block_transactions.len(),
            histograms.block_heights.len()
        );

        assert_eq!(latest_block.height, num_blocks - 1);
        assert_eq!(latest_blocks.len(), min(num_blocks as usize, 10));
        assert_eq!(
            latest_transactions.len(),
            min(num_transactions as usize, 10)
        );

        {
            // Retrieve Block Detail using the block height
            let block_detail_response: BlockDetailResponse<MockTypes> = client
                .get(format!("block/{}", latest_block.height).as_str())
                .send()
                .await
                .unwrap();
            assert_eq!(block_detail_response.block_detail, latest_block);
        }

        {
            // Retrieve Block Detail using the block hash
            let block_detail_response: BlockDetailResponse<MockTypes> = client
                .get(format!("block/hash/{}", latest_block.hash).as_str())
                .send()
                .await
                .unwrap();
            assert_eq!(block_detail_response.block_detail, latest_block);
        }

        {
            // Retrieve 20 Block Summaries using the block height
            let block_summaries_response: BlockSummaryResponse<MockTypes> = client
                .get(format!("blocks/{}/{}", num_blocks - 1, 20).as_str())
                .send()
                .await
                .unwrap();
            for (a, b) in block_summaries_response
                .block_summaries
                .iter()
                .zip(latest_blocks.iter())
            {
                assert_eq!(a, b);
            }
        }

        {
            let target_num = min(num_blocks as usize, 10);
            // Retrieve the 20 latest block summaries
            let block_summaries_response: BlockSummaryResponse<MockTypes> = client
                .get(format!("blocks/latest/{target_num}").as_str())
                .send()
                .await
                .unwrap();

            // These blocks aren't guaranteed to have any overlap with what has
            // been previously generated, so we don't know if we can check
            // equality of the set.  However, we **can** check to see if the
            // number of blocks we were asking for get returned.
            assert_eq!(block_summaries_response.block_summaries.len(), target_num);

            // We can also perform a check on the first block to ensure that it
            // is larger than or equal to our `num_blocks` variable.
            assert!(
                block_summaries_response
                    .block_summaries
                    .first()
                    .unwrap()
                    .height
                    >= num_blocks - 1
            );
        }
        let get_search_response: SearchResultResponse<MockTypes> = client
            .get(format!("search/{}", latest_block.hash).as_str())
            .send()
            .await
            .unwrap();

        assert!(!get_search_response.search_results.blocks.is_empty());

        if num_transactions > 0 {
            let last_transaction = latest_transactions.first().unwrap();
            let transaction_detail_response: TransactionDetailResponse<MockTypes> = client
                .get(format!("transaction/hash/{}", last_transaction.hash).as_str())
                .send()
                .await
                .unwrap();

            assert!(
                transaction_detail_response
                    .transaction_detail
                    .details
                    .block_confirmed
            );

            assert_eq!(
                transaction_detail_response.transaction_detail.details.hash,
                last_transaction.hash
            );

            assert_eq!(
                transaction_detail_response
                    .transaction_detail
                    .details
                    .height,
                last_transaction.height
            );

            assert_eq!(
                transaction_detail_response
                    .transaction_detail
                    .details
                    .num_transactions,
                last_transaction.num_transactions
            );

            assert_eq!(
                transaction_detail_response
                    .transaction_detail
                    .details
                    .offset,
                last_transaction.offset
            );

            assert_eq!(
                transaction_detail_response.transaction_detail.details.time,
                last_transaction.time
            );

            // Transactions Summaries - No Filter
            let n_txns = NUM_TXNS_PER_BLOCK;

            {
                // Retrieve transactions summaries via hash
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(format!("transactions/hash/{}/{}", last_transaction.hash, 20).as_str())
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
                    .iter()
                    .zip(latest_transactions.iter().take(10).collect::<Vec<_>>())
                {
                    assert_eq!(a, b);
                }
            }

            {
                // Retrieve transactions summaries via height and offset
                // No offset, which should indicate the most recent transaction
                // within the targeted block.
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(
                            format!("transactions/from/{}/{}/{}", last_transaction.height, 0, 20)
                                .as_str(),
                        )
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
                    .iter()
                    .zip(latest_transactions.iter().take(10).collect::<Vec<_>>())
                {
                    assert_eq!(a, b);
                }
            }

            {
                // Retrieve transactions summaries via height and offset (different offset)
                // In this case since we're creating n_txns transactions per
                // block, an offset of n_txns - 1 will ensure that we're still
                // within the same starting target block.
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(
                            format!(
                                "transactions/from/{}/{}/{}",
                                last_transaction.height,
                                n_txns - 1,
                                20
                            )
                            .as_str(),
                        )
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
                    .iter()
                    .zip(
                        latest_transactions
                            .iter()
                            .skip(n_txns - 1)
                            .take(10)
                            .collect::<Vec<_>>(),
                    )
                {
                    assert_eq!(a, b);
                }
            }

            {
                // Retrieve transactions summaries via height and offset (different offset)
                // In this case since we're creating n_txns transactions per
                // block, an offset of n_txns + 1 will ensure that we're
                // outside of the starting block
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(
                            format!(
                                "transactions/from/{}/{}/{}",
                                last_transaction.height,
                                n_txns + 1,
                                20
                            )
                            .as_str(),
                        )
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
                    .iter()
                    .zip(
                        latest_transactions
                            .iter()
                            .skip(6)
                            .take(10)
                            .collect::<Vec<_>>(),
                    )
                {
                    assert_eq!(a, b);
                }
            }

            {
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(format!("transactions/latest/{}", 20).as_str())
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
                    .iter()
                    .zip(latest_transactions.iter().take(10).collect::<Vec<_>>())
                {
                    assert_eq!(a, b);
                }
            }

            // Transactions Summaries - Block Filter

            {
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(
                            format!(
                                "transactions/hash/{}/{}/block/{}",
                                last_transaction.hash, 20, last_transaction.height
                            )
                            .as_str(),
                        )
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
                    .iter()
                    .take_while(|t: &&TransactionSummary<MockTypes>| {
                        t.height == last_transaction.height
                    })
                    .zip(latest_transactions.iter().take(10).collect::<Vec<_>>())
                {
                    assert_eq!(a, b);
                }
            }

            {
                // With an offset of 0, we should start at the most recent
                // transaction within the specified block.
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(
                            format!(
                                "transactions/from/{}/{}/{}/block/{}",
                                last_transaction.height, 0, 20, last_transaction.height
                            )
                            .as_str(),
                        )
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
                    .iter()
                    .take_while(|t: &&TransactionSummary<MockTypes>| {
                        t.height == last_transaction.height
                    })
                    .zip(latest_transactions.iter().take(10).collect::<Vec<_>>())
                {
                    assert_eq!(a, b);
                }
            }

            {
                // In this case, since we're creating n_txns transactions per
                // block, an offset of n_txns - 1 will ensure that we're still
                // within the same starting target block.
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(
                            format!(
                                "transactions/from/{}/{}/{}/block/{}",
                                last_transaction.height,
                                n_txns - 1,
                                20,
                                last_transaction.height
                            )
                            .as_str(),
                        )
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
                    .iter()
                    .skip(n_txns - 1)
                    .take_while(|t: &&TransactionSummary<MockTypes>| {
                        t.height == last_transaction.height
                    })
                    .zip(latest_transactions.iter().take(10).collect::<Vec<_>>())
                {
                    assert_eq!(a, b);
                }
            }

            {
                // In this case, since we're creating n_txns transactions per
                // block, an offset of n_txns + 1 will ensure that we're
                // outside of the starting target block
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(
                            format!(
                                "transactions/from/{}/{}/{}/block/{}",
                                last_transaction.height,
                                n_txns + 1,
                                20,
                                last_transaction.height
                            )
                            .as_str(),
                        )
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
                    .iter()
                    .skip(n_txns + 1)
                    .take_while(|t: &&TransactionSummary<MockTypes>| {
                        t.height == last_transaction.height
                    })
                    .zip(latest_transactions.iter().take(10).collect::<Vec<_>>())
                {
                    assert_eq!(a, b);
                }
            }

            {
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(
                            format!(
                                "transactions/latest/{}/block/{}",
                                20, last_transaction.height
                            )
                            .as_str(),
                        )
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
                    .iter()
                    .take_while(|t: &&TransactionSummary<MockTypes>| {
                        t.height == last_transaction.height
                    })
                    .zip(latest_transactions.iter().take(10).collect::<Vec<_>>())
                {
                    assert_eq!(a, b);
                }
            }
        }
    }

    /// The `explorer.toml` spec requires every error to be identifiable from its `code` alone, so
    /// check the wire body of the one failure a request can provoke without touching storage.
    async fn validate_limit_errors(client: &Client<AppError, MockBase>) {
        for route in [
            "blocks/latest/0",
            "blocks/latest/101",
            "blocks/1/0",
            "transactions/latest/0",
            "transactions/latest/101",
            "transactions/latest/0/block/1",
            "transactions/latest/0/namespace/1",
            "transactions/from/1/0/0",
            // The limit is reported ahead of the target, so an unparseable hash does not mask it.
            "transactions/hash/TX~0/0",
        ] {
            // `bytes` reports the raw body of an error response, which `send` would have decoded
            // into an ambiguous variant: the module error is an untagged enum.
            let err = client.get::<()>(route).bytes().await.unwrap_err();
            let AppError::Custom { message, status } = err else {
                panic!("{route} must fail with the error body, got {err:?}");
            };
            assert_eq!(status, StatusCode::BAD_REQUEST, "{route}: {message}");
            assert!(message.contains("INVALID_LIMIT"), "{route}: {message}");
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_api() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockSqlDataSource>::init().await;
        network.start().await;

        let (explorer_client, availability_client) = start_clients(
            explorer_router::<MockTypes, _>(&Options::default(), network.data_source()),
            availability_router::<MockTypes, _>(
                &availability::Options {
                    fetch_timeout: Duration::from_secs(5),
                    ..Default::default()
                },
                network.data_source(),
            ),
        )
        .await;

        let mut blocks = availability_client
            .socket("stream/blocks/0")
            .subscribe::<BlockQueryData<MockTypes>>()
            .await
            .unwrap();

        for b in 0..NUM_BLOCKS {
            for t in 0..NUM_TXNS_PER_BLOCK {
                let nonce = b * NUM_TXNS_PER_BLOCK + t;
                network
                    .submit_transaction(mock_transaction(vec![nonce as u8]))
                    .await;
            }

            // Wait for the transaction to be finalized.
            for _ in 0..10 {
                let block = blocks.next().await.unwrap().unwrap();
                if !block.is_empty() {
                    break;
                }
            }
        }

        assert!(explorer_client.connect(Some(Duration::from_secs(60))).await);

        validate(&explorer_client).await;
        validate_limit_errors(&explorer_client).await;
        network.shut_down().await;
    }

    /// Registers every route with `unimplemented!()` bodies, so the documentation test needs no
    /// database: it inspects the routes the router declares and calls no handler.
    struct UnimplementedDataSource;

    #[async_trait::async_trait]
    impl ExplorerDataSource<MockTypes> for UnimplementedDataSource {
        async fn get_block_detail(
            &self,
            _request: BlockIdentifier<MockTypes>,
        ) -> Result<BlockDetail<MockTypes>, GetBlockDetailError> {
            unimplemented!()
        }
        async fn get_block_summaries(
            &self,
            _request: GetBlockSummariesRequest<MockTypes>,
        ) -> Result<Vec<BlockSummary<MockTypes>>, GetBlockSummariesError> {
            unimplemented!()
        }
        async fn get_transaction_detail(
            &self,
            _request: TransactionIdentifier<MockTypes>,
        ) -> Result<
            super::super::query_data::TransactionDetailResponse<MockTypes>,
            GetTransactionDetailError,
        > {
            unimplemented!()
        }
        async fn get_transaction_summaries(
            &self,
            _request: GetTransactionSummariesRequest<MockTypes>,
        ) -> Result<Vec<TransactionSummary<MockTypes>>, GetTransactionSummariesError> {
            unimplemented!()
        }
        async fn get_explorer_summary(
            &self,
        ) -> Result<ExplorerSummary<MockTypes>, GetExplorerSummaryError> {
            unimplemented!()
        }
        async fn get_search_results(
            &self,
            _query: TaggedBase64,
        ) -> Result<SearchResult<MockTypes>, GetSearchResultsError> {
            unimplemented!()
        }
    }

    /// Applications mount this router and serve its documentation as part of their own OpenAPI
    /// spec, so every route it registers must carry a summary.
    #[tokio::test]
    async fn router_documents_every_route() {
        let mut api = aide::openapi::OpenApi::default();
        let _ = explorer_router::<MockTypes, _>(&Options::default(), UnimplementedDataSource)
            .finish_api(&mut api);

        let paths = api.paths.expect("router registered paths");
        for route in [
            "/block/{height}",
            "/block/hash/{hash}",
            "/blocks/latest/{limit}",
            "/blocks/{from}/{limit}",
            "/transaction/{height}/{offset}",
            "/transaction/hash/{hash}",
            "/transactions/latest/{limit}",
            "/transactions/from/{height}/{offset}/{limit}",
            "/transactions/hash/{hash}/{limit}",
            "/transactions/latest/{limit}/block/{block}",
            "/transactions/from/{height}/{offset}/{limit}/block/{block}",
            "/transactions/hash/{hash}/{limit}/block/{block}",
            "/transactions/latest/{limit}/namespace/{namespace}",
            "/transactions/from/{height}/{offset}/{limit}/namespace/{namespace}",
            "/transactions/hash/{hash}/{limit}/namespace/{namespace}",
            "/explorer-summary",
            "/search/{query}",
        ] {
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
