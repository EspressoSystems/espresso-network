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

//! Axum router serving the availability API wire protocol.
//!
//! Route paths, response forms, fetch-timeout and range-limit semantics, status codes and the
//! wire error envelope (the crate-level [`Error`](crate::Error)) match the old tide-disco
//! handlers and the `availability.toml` route specs, so existing clients keep working unchanged.
//!
//! The router is an [`ApiRouter`] so that the OpenAPI documentation travels with the routes:
//! an application mounting this module gets the summaries and descriptions without restating
//! them. Use [`From`] to get a plain [`Router`] where the docs are not wanted.

use std::{sync::Arc, time::Duration};

use aide::axum::{ApiRouter, routing::get_with};
use axum::{
    Router,
    extract::{Path, State, WebSocketUpgrade},
    http::HeaderMap,
    response::Response,
    routing::get,
};
use disco_types::{request::RequestError, status::StatusCode};
use futures::{StreamExt, TryStreamExt};
use hotshot_types::{data::VidCommitment, traits::node_implementation::NodeType};
use http_wire::{
    self as wire, ContentType, body_limit_layer, cors_layer, drive_ws_stream, healthcheck_response,
};
use serde::Serialize;
use snafu::OptionExt;
use tagged_base64::TaggedBase64;

use super::{
    AvailabilityDataSource, BlockHash, BlockId, BlockQueryData, BlockSummaryQueryData,
    BlockWithTransaction, Error, FetchBlockSnafu, FetchHeaderSnafu, FetchLeafSnafu,
    FetchTransactionSnafu, InvalidTransactionIndexSnafu, LeafHash, LeafId, LeafQueryData, Limits,
    Options, PayloadQueryData, QueryableHeader, QueryablePayload, RangeLimitSnafu, TransactionHash,
    TransactionQueryData, TransactionWithProofQueryData, VidCommonQueryData,
};
use crate::{Error as AppError, Header, Payload, types::HeightIndexed};

/// The availability module's routes: leaves as [`LeafQueryData`] and VID common data as the
/// version-gated [`VidCommonQueryData`].
///
/// `options` supplies the fetch timeout and the range limits enforced by the range routes.
pub fn availability_router<Types, S>(options: &Options, data_source: S) -> ApiRouter
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    ApiRouter::new()
        .api_route(
            "/leaf/{height}",
            get_with(get_leaf::<Types, S>, |op| {
                op.summary("Get leaf by height").description(
                    "Get the leaf at the given position in the ledger; 0 is the genesis leaf. The \
                     response carries the leaf and the quorum certificate signing it.",
                )
            }),
        )
        .api_route(
            "/leaf/hash/{hash}",
            get_with(get_leaf_by_hash::<Types, S>, |op| {
                op.summary("Get leaf by hash").description(
                    "Get the leaf with the given commitment. The response carries the leaf and \
                     the quorum certificate signing it.",
                )
            }),
        )
        .api_route(
            "/leaf/{from}/{until}",
            get_with(get_leaf_range::<Types, S>, |op| {
                op.summary("Get a range of leaves").description(
                    "Get the leaves at positions `from` up to but not including `until`. Ranges \
                     longer than the small-object limit reported by `/limits` fail with a 400.",
                )
            }),
        )
        .api_route(
            "/stream/leaves/{height}",
            get_with(stream_leaves::<Types, S>, |op| {
                op.summary("Stream leaves (websocket)").description(
                    "Subscribe to leaves in the order they are sequenced, starting at the given \
                     height. Sends the same objects as `leaf/{height}` and does not terminate.",
                )
            }),
        )
        .api_route(
            "/vid/common/{height}",
            get_with(get_vid_common::<Types, S>, |op| {
                op.summary("Get VID common data by height").description(
                    "Get the VID common data for the block at the given position in the ledger. \
                     This is the data shared by all storage nodes, not a VID share: it does not \
                     help reconstruct the block, only interpret other VID data. For this node's \
                     share, see the node API's `vid/share`.",
                )
            }),
        )
        .api_route(
            "/vid/common/hash/{hash}",
            get_with(get_vid_common_by_hash::<Types, S>, |op| {
                op.summary("Get VID common data by block hash").description(
                    "Get the VID common data for the block with the given hash. This is the data \
                     shared by all storage nodes, not a VID share.",
                )
            }),
        )
        .api_route(
            "/vid/common/payload-hash/{payload_hash}",
            get_with(get_vid_common_by_payload_hash::<Types, S>, |op| {
                op.summary("Get VID common data by payload hash")
                    .description(
                        "Get the VID common data for a block with the given payload commitment. \
                         Payloads are not unique, so any block with this payload may answer.",
                    )
            }),
        )
        .api_route(
            "/vid/common/{from}/{until}",
            get_with(get_vid_common_range::<Types, S>, |op| {
                op.summary("Get a range of VID common data").description(
                    "Get the VID common data for blocks at positions `from` up to but not \
                     including `until`. Ranges longer than the small-object limit reported by \
                     `/limits` fail with a 400.",
                )
            }),
        )
        .api_route(
            "/stream/vid/common/{height}",
            get_with(stream_vid_common::<Types, S>, |op| {
                op.summary("Stream VID common data (websocket)")
                    .description(
                        "Subscribe to VID common data in the order blocks are sequenced, starting \
                         at the given height. Sends the same objects as `vid/common/{height}` and \
                         does not terminate.",
                    )
            }),
        )
        .merge(common_router::<Types, S>())
        .with_state(RouterState::new(options, data_source))
}

/// Wraps an availability router with the app-level `healthcheck`, a request body limit, and
/// permissive CORS headers. Mounting the module prefix is up to the caller.
pub fn app(api: Router) -> Router {
    Router::new()
        .route("/healthcheck", get(healthcheck))
        .merge(api)
        .layer(body_limit_layer())
        .layer(cors_layer())
}

/// Encode a handler result, wrapping the module error in the crate-level
/// [`Error`](crate::Error) envelope the old tide app served.
fn respond<T: Serialize>(headers: &HeaderMap, result: Result<T, Error>) -> Response {
    wire::respond::<AppError, _>(headers, result.map_err(AppError::from))
}

/// Handler context: the data source plus the fetch timeout and range limits from [`Options`].
struct RouterState<S> {
    data_source: S,
    fetch_timeout: Duration,
    small_object_range_limit: usize,
    large_object_range_limit: usize,
}

impl<S> RouterState<S> {
    fn new(options: &Options, data_source: S) -> Arc<Self> {
        Arc::new(Self {
            data_source,
            fetch_timeout: options.fetch_timeout,
            small_object_range_limit: options.small_object_range_limit,
            large_object_range_limit: options.large_object_range_limit,
        })
    }
}

/// The routes that do not touch leaf or VID objects, split out only to keep
/// [`availability_router`] readable.
fn common_router<Types, S>() -> ApiRouter<Arc<RouterState<S>>>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    ApiRouter::new()
        .api_route(
            "/header/{height}",
            get_with(get_header::<Types, S>, |op| {
                op.summary("Get header by height").description(
                    "Get the header of the block at the given position in the ledger; 0 is the \
                     genesis block.",
                )
            }),
        )
        .api_route(
            "/header/hash/{hash}",
            get_with(get_header_by_hash::<Types, S>, |op| {
                op.summary("Get header by block hash")
                    .description("Get the header of the block with the given hash.")
            }),
        )
        .api_route(
            "/header/payload-hash/{payload_hash}",
            get_with(get_header_by_payload_hash::<Types, S>, |op| {
                op.summary("Get header by payload hash").description(
                    "Get the header of a block with the given payload commitment. Payloads are \
                     not unique, so any block with this payload may answer.",
                )
            }),
        )
        .api_route(
            "/header/{from}/{until}",
            get_with(get_header_range::<Types, S>, |op| {
                op.summary("Get a range of headers").description(
                    "Get the headers of the blocks at positions `from` up to but not including \
                     `until`. Ranges longer than the large-object limit reported by `/limits` \
                     fail with a 400.",
                )
            }),
        )
        .api_route(
            "/stream/headers/{height}",
            get_with(stream_headers::<Types, S>, |op| {
                op.summary("Stream headers (websocket)").description(
                    "Subscribe to headers in the order blocks are sequenced, starting at the \
                     given height. Useful for applications, such as rollups, that do not need the \
                     whole block. Does not terminate.",
                )
            }),
        )
        .api_route(
            "/block/{height}",
            get_with(get_block::<Types, S>, |op| {
                op.summary("Get block by height").description(
                    "Get the block at the given position in the ledger, with its header, payload, \
                     hash and size. Block payloads disseminate asynchronously, so \
                     `block/{height}` can fail for a height at which `leaf/{height}` already \
                     succeeds; once the leaf is available the block eventually becomes available \
                     too.",
                )
            }),
        )
        .api_route(
            "/block/hash/{hash}",
            get_with(get_block_by_hash::<Types, S>, |op| {
                op.summary("Get block by hash").description(
                    "Get the block with the given hash, with its header, payload, hash and size.",
                )
            }),
        )
        .api_route(
            "/block/payload-hash/{payload_hash}",
            get_with(get_block_by_payload_hash::<Types, S>, |op| {
                op.summary("Get block by payload hash").description(
                    "Get a block with the given payload commitment. Payloads are not unique, so \
                     any block with this payload may answer.",
                )
            }),
        )
        .api_route(
            "/block/{from}/{until}",
            get_with(get_block_range::<Types, S>, |op| {
                op.summary("Get a range of blocks").description(
                    "Get the blocks at positions `from` up to but not including `until`. Ranges \
                     longer than the large-object limit reported by `/limits` fail with a 400.",
                )
            }),
        )
        .api_route(
            "/stream/blocks/{height}",
            get_with(stream_blocks::<Types, S>, |op| {
                op.summary("Stream blocks (websocket)").description(
                    "Subscribe to blocks in the order they are sequenced, starting at the given \
                     height. Sends the same objects as `block/{height}` and does not terminate.",
                )
            }),
        )
        .api_route(
            "/block/summary/{height}",
            get_with(get_block_summary::<Types, S>, |op| {
                op.summary("Get block summary by height").description(
                    "Get the header, hash, size and transaction count of the block at the given \
                     position in the ledger, without its payload.",
                )
            }),
        )
        .api_route(
            "/block/summaries/{from}/{until}",
            get_with(get_block_summary_range::<Types, S>, |op| {
                op.summary("Get a range of block summaries").description(
                    "Get the block summaries for positions `from` up to but not including \
                     `until`. Summaries are derived from whole blocks, so the large-object limit \
                     reported by `/limits` applies; longer ranges fail with a 400.",
                )
            }),
        )
        .api_route(
            "/payload/{height}",
            get_with(get_payload::<Types, S>, |op| {
                op.summary("Get payload by height").description(
                    "Get the payload of the block at the given position in the ledger.",
                )
            }),
        )
        .api_route(
            "/payload/hash/{hash}",
            get_with(get_payload_by_hash::<Types, S>, |op| {
                op.summary("Get payload by payload hash")
                    .description("Get the payload with the given commitment.")
            }),
        )
        .api_route(
            "/payload/block-hash/{block_hash}",
            get_with(get_payload_by_block_hash::<Types, S>, |op| {
                op.summary("Get payload by block hash")
                    .description("Get the payload of the block with the given hash.")
            }),
        )
        .api_route(
            "/payload/{from}/{until}",
            get_with(get_payload_range::<Types, S>, |op| {
                op.summary("Get a range of payloads").description(
                    "Get the payloads of the blocks at positions `from` up to but not including \
                     `until`. Ranges longer than the large-object limit reported by `/limits` \
                     fail with a 400.",
                )
            }),
        )
        .api_route(
            "/stream/payloads/{height}",
            get_with(stream_payloads::<Types, S>, |op| {
                op.summary("Stream payloads (websocket)").description(
                    "Subscribe to block payloads in the order blocks are sequenced, starting at \
                     the given height. Sends the same objects as `payload/{height}` and does not \
                     terminate.",
                )
            }),
        )
        .api_route(
            "/transaction/{height}/{index}/noproof",
            get_with(get_transaction::<Types, S>, |op| {
                op.summary("Get transaction by position, without proof")
                    .description(
                        "Get the transaction at `index` within the block at `height`, without an \
                         inclusion proof. Cheaper than the proof form, which has to load VID data.",
                    )
            }),
        )
        .api_route(
            "/transaction/hash/{hash}/noproof",
            get_with(get_transaction_by_hash::<Types, S>, |op| {
                op.summary("Get transaction by hash, without proof")
                    .description(
                        "Get the transaction with the given hash, without an inclusion proof. \
                         Consensus does not reject duplicate transactions, so several positions \
                         in the log may share a hash; the earliest one answers.",
                    )
            }),
        )
        .api_route(
            "/transaction/{height}/{index}",
            get_with(get_transaction_proof::<Types, S>, |op| {
                op.summary("Get transaction by position, with proof")
                    .description(
                        "Get the transaction at `index` within the block at `height`, along with \
                         an application-defined proof of its inclusion in that block. The proof \
                         system varies by application: some prove more than block membership, \
                         some return no proof at all.",
                    )
            }),
        )
        .api_route(
            "/transaction/{height}/{index}/proof",
            get_with(get_transaction_proof::<Types, S>, |op| {
                op.summary("Get transaction by position, with proof")
                    .description(
                        "Get the transaction at `index` within the block at `height`, along with \
                         an application-defined proof of its inclusion in that block. The proof \
                         system varies by application: some prove more than block membership, \
                         some return no proof at all.",
                    )
            }),
        )
        .api_route(
            "/transaction/hash/{hash}",
            get_with(get_transaction_proof_by_hash::<Types, S>, |op| {
                op.summary("Get transaction by hash, with proof")
                    .description(
                        "Get the transaction with the given hash, along with an \
                         application-defined proof of its inclusion in its block. Consensus does \
                         not reject duplicate transactions, so several positions in the log may \
                         share a hash; the earliest one answers.",
                    )
            }),
        )
        .api_route(
            "/transaction/hash/{hash}/proof",
            get_with(get_transaction_proof_by_hash::<Types, S>, |op| {
                op.summary("Get transaction by hash, with proof")
                    .description(
                        "Get the transaction with the given hash, along with an \
                         application-defined proof of its inclusion in its block. Consensus does \
                         not reject duplicate transactions, so several positions in the log may \
                         share a hash; the earliest one answers.",
                    )
            }),
        )
        .api_route(
            "/stream/transactions/{height}",
            get_with(stream_transactions::<Types, S>, |op| {
                op.summary("Stream transactions (websocket)").description(
                    "Subscribe to every transaction in every block, in sequence order, starting \
                     at the given height. Does not terminate.",
                )
            }),
        )
        .api_route(
            "/stream/transactions/{height}/namespace/{namespace}",
            get_with(stream_transactions_in_namespace::<Types, S>, |op| {
                op.summary("Stream namespace transactions (websocket)")
                    .description(
                        "Subscribe to the transactions belonging to one namespace, in sequence \
                         order, starting at the given height. Does not terminate.",
                    )
            }),
        )
        .api_route(
            "/limits",
            get_with(get_limits::<S>, |op| {
                op.summary("Get availability limits").description(
                    "Get the implementation-defined range limits. `small_object_range_limit` caps \
                     leaf and VID common ranges; `large_object_range_limit` caps everything that \
                     may carry a payload, or an object proportional to one.",
                )
            }),
        )
        .api_route(
            "/cert2/{height}",
            get_with(get_cert2::<Types, S>, |op| {
                op.summary("Get finality certificate").description(
                    "Get the Certificate2 finalizing the block at the given height. Under the new \
                     consensus protocol this certificate is the proof that the block is final.",
                )
            }),
        )
}

async fn healthcheck(headers: HeaderMap) -> Response {
    healthcheck_response(&headers)
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

fn enforce_range_limit(from: usize, until: usize, limit: usize) -> Result<(), Error> {
    if until.saturating_sub(from) > limit {
        return RangeLimitSnafu { from, until, limit }.fail();
    }
    Ok(())
}

// Loaders shared by handlers: resolve a fetch against the configured timeout and report missing
// data with the same error variants (and hence status codes) as the old handlers.

async fn load_leaf<Types, S>(
    state: &RouterState<S>,
    id: LeafId<Types>,
) -> Result<LeafQueryData<Types>, Error>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let fetch = state.data_source.get_leaf(id).await;
    fetch
        .with_timeout(state.fetch_timeout)
        .await
        .context(FetchLeafSnafu {
            resource: id.to_string(),
        })
}

async fn load_leaf_range<Types, S>(
    state: &RouterState<S>,
    from: usize,
    until: usize,
) -> Result<Vec<LeafQueryData<Types>>, Error>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    enforce_range_limit(from, until, state.small_object_range_limit)?;
    let timeout = state.fetch_timeout;
    let leaves = state.data_source.get_leaf_range(from..until).await;
    leaves
        .enumerate()
        .then(|(index, fetch)| async move {
            fetch.with_timeout(timeout).await.context(FetchLeafSnafu {
                resource: (index + from).to_string(),
            })
        })
        .try_collect()
        .await
}

async fn load_header<Types, S>(
    state: &RouterState<S>,
    id: BlockId<Types>,
) -> Result<Header<Types>, Error>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let fetch = state.data_source.get_header(id).await;
    fetch
        .with_timeout(state.fetch_timeout)
        .await
        .context(FetchHeaderSnafu {
            resource: id.to_string(),
        })
}

async fn load_header_range<Types, S>(
    state: &RouterState<S>,
    from: usize,
    until: usize,
) -> Result<Vec<Header<Types>>, Error>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    enforce_range_limit(from, until, state.large_object_range_limit)?;
    let timeout = state.fetch_timeout;
    let headers = state.data_source.get_header_range(from..until).await;
    headers
        .enumerate()
        .then(|(index, fetch)| async move {
            fetch.with_timeout(timeout).await.context(FetchHeaderSnafu {
                resource: (index + from).to_string(),
            })
        })
        .try_collect()
        .await
}

async fn load_block<Types, S>(
    state: &RouterState<S>,
    id: BlockId<Types>,
) -> Result<BlockQueryData<Types>, Error>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let fetch = state.data_source.get_block(id).await;
    fetch
        .with_timeout(state.fetch_timeout)
        .await
        .context(FetchBlockSnafu {
            resource: id.to_string(),
        })
}

async fn load_block_range<Types, S>(
    state: &RouterState<S>,
    from: usize,
    until: usize,
) -> Result<Vec<BlockQueryData<Types>>, Error>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    enforce_range_limit(from, until, state.large_object_range_limit)?;
    let timeout = state.fetch_timeout;
    let blocks = state.data_source.get_block_range(from..until).await;
    blocks
        .enumerate()
        .then(|(index, fetch)| async move {
            fetch.with_timeout(timeout).await.context(FetchBlockSnafu {
                resource: (index + from).to_string(),
            })
        })
        .try_collect()
        .await
}

async fn load_payload<Types, S>(
    state: &RouterState<S>,
    id: BlockId<Types>,
) -> Result<PayloadQueryData<Types>, Error>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let fetch = state.data_source.get_payload(id).await;
    fetch
        .with_timeout(state.fetch_timeout)
        .await
        .context(FetchBlockSnafu {
            resource: id.to_string(),
        })
}

async fn load_payload_range<Types, S>(
    state: &RouterState<S>,
    from: usize,
    until: usize,
) -> Result<Vec<PayloadQueryData<Types>>, Error>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    enforce_range_limit(from, until, state.large_object_range_limit)?;
    let timeout = state.fetch_timeout;
    let payloads = state.data_source.get_payload_range(from..until).await;
    payloads
        .enumerate()
        .then(|(index, fetch)| async move {
            fetch.with_timeout(timeout).await.context(FetchBlockSnafu {
                resource: (index + from).to_string(),
            })
        })
        .try_collect()
        .await
}

async fn load_vid_common<Types, S>(
    state: &RouterState<S>,
    id: BlockId<Types>,
) -> Result<VidCommonQueryData<Types>, Error>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let fetch = state.data_source.get_vid_common(id).await;
    fetch
        .with_timeout(state.fetch_timeout)
        .await
        .context(FetchBlockSnafu {
            resource: id.to_string(),
        })
}

async fn load_vid_common_range<Types, S>(
    state: &RouterState<S>,
    from: usize,
    until: usize,
) -> Result<Vec<VidCommonQueryData<Types>>, Error>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    enforce_range_limit(from, until, state.small_object_range_limit)?;
    let timeout = state.fetch_timeout;
    let vid = state.data_source.get_vid_common_range(from..until).await;
    vid.enumerate()
        .then(|(index, fetch)| async move {
            fetch.with_timeout(timeout).await.context(FetchBlockSnafu {
                resource: (index + from).to_string(),
            })
        })
        .try_collect()
        .await
}

async fn load_transaction_by_hash<Types, S>(
    state: &RouterState<S>,
    hash: TransactionHash<Types>,
) -> Result<BlockWithTransaction<Types>, Error>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    state
        .data_source
        .get_block_containing_transaction(hash)
        .await
        .with_timeout(state.fetch_timeout)
        .await
        .context(FetchTransactionSnafu {
            resource: hash.to_string(),
        })
}

async fn load_transaction_by_position<Types, S>(
    state: &RouterState<S>,
    height: u64,
    index: u64,
) -> Result<BlockWithTransaction<Types>, Error>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let block = load_block(state, BlockId::Number(height as usize)).await?;
    let i = index;
    let index = block
        .payload()
        .nth(block.metadata(), i as usize)
        .context(InvalidTransactionIndexSnafu { height, index: i })?;
    let transaction = block
        .transaction(&index)
        .context(InvalidTransactionIndexSnafu { height, index: i })?;
    let transaction = TransactionQueryData::new(transaction, &block, &index, i)
        .context(InvalidTransactionIndexSnafu { height, index: i })?;
    Ok(BlockWithTransaction {
        transaction,
        block,
        index,
    })
}

async fn prove_transaction<Types, S>(
    state: &RouterState<S>,
    tx: BlockWithTransaction<Types>,
) -> Result<TransactionWithProofQueryData<Types>, Error>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let height = tx.block.height();
    let vid = load_vid_common(state, BlockId::Number(height as usize)).await?;
    let proof =
        tx.block
            .transaction_proof(&vid, &tx.index)
            .context(InvalidTransactionIndexSnafu {
                height,
                index: tx.transaction.index(),
            })?;
    Ok(TransactionWithProofQueryData::new(tx.transaction, proof))
}

// Leaf handlers (v1 semantics).

async fn get_leaf<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    respond(&headers, load_leaf(&state, LeafId::Number(height)).await)
}

async fn get_leaf_by_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let result = async {
        let hash = tb64_param::<LeafHash<Types>>(&hash, "hash")?;
        load_leaf(&state, LeafId::Hash(hash)).await
    }
    .await;
    respond(&headers, result)
}

async fn get_leaf_range<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((from, until)): Path<(usize, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    respond(&headers, load_leaf_range(&state, from, until).await)
}

async fn stream_leaves<Types, S>(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let format = ContentType::negotiate(&headers);
    ws.on_upgrade(move |socket| async move {
        let stream = state.data_source.subscribe_leaves(height).await;
        drive_ws_stream(socket, stream, format).await;
    })
}

// Header handlers.

async fn get_header<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    respond(&headers, load_header(&state, BlockId::Number(height)).await)
}

async fn get_header_by_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let result = async {
        let hash = tb64_param::<BlockHash<Types>>(&hash, "hash")?;
        load_header(&state, BlockId::Hash(hash)).await
    }
    .await;
    respond(&headers, result)
}

async fn get_header_by_payload_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let result = async {
        let hash = tb64_param::<VidCommitment>(&hash, "payload-hash")?;
        load_header(&state, BlockId::PayloadHash(hash)).await
    }
    .await;
    respond(&headers, result)
}

async fn get_header_range<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((from, until)): Path<(usize, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    respond(&headers, load_header_range(&state, from, until).await)
}

async fn stream_headers<Types, S>(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let format = ContentType::negotiate(&headers);
    ws.on_upgrade(move |socket| async move {
        let stream = state.data_source.subscribe_headers(height).await;
        drive_ws_stream(socket, stream, format).await;
    })
}

// Block handlers.

async fn get_block<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    respond(&headers, load_block(&state, BlockId::Number(height)).await)
}

async fn get_block_by_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let result = async {
        let hash = tb64_param::<BlockHash<Types>>(&hash, "hash")?;
        load_block(&state, BlockId::Hash(hash)).await
    }
    .await;
    respond(&headers, result)
}

async fn get_block_by_payload_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let result = async {
        let hash = tb64_param::<VidCommitment>(&hash, "payload-hash")?;
        load_block(&state, BlockId::PayloadHash(hash)).await
    }
    .await;
    respond(&headers, result)
}

async fn get_block_range<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((from, until)): Path<(usize, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    respond(&headers, load_block_range(&state, from, until).await)
}

async fn stream_blocks<Types, S>(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let format = ContentType::negotiate(&headers);
    ws.on_upgrade(move |socket| async move {
        let stream = state.data_source.subscribe_blocks(height).await;
        drive_ws_stream(socket, stream, format).await;
    })
}

async fn get_block_summary<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let result = load_block(&state, BlockId::Number(height)).await;
    respond(&headers, result.map(BlockSummaryQueryData::from))
}

async fn get_block_summary_range<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((from, until)): Path<(usize, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let result = load_block_range(&state, from, until).await.map(|blocks| {
        blocks
            .into_iter()
            .map(BlockSummaryQueryData::from)
            .collect::<Vec<_>>()
    });
    respond(&headers, result)
}

// Payload handlers.

async fn get_payload<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    respond(
        &headers,
        load_payload(&state, BlockId::Number(height)).await,
    )
}

async fn get_payload_by_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let result = async {
        let hash = tb64_param::<VidCommitment>(&hash, "hash")?;
        load_payload(&state, BlockId::PayloadHash(hash)).await
    }
    .await;
    respond(&headers, result)
}

async fn get_payload_by_block_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let result = async {
        let hash = tb64_param::<BlockHash<Types>>(&hash, "block-hash")?;
        load_payload(&state, BlockId::Hash(hash)).await
    }
    .await;
    respond(&headers, result)
}

async fn get_payload_range<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((from, until)): Path<(usize, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    respond(&headers, load_payload_range(&state, from, until).await)
}

async fn stream_payloads<Types, S>(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let format = ContentType::negotiate(&headers);
    ws.on_upgrade(move |socket| async move {
        let stream = state.data_source.subscribe_payloads(height).await;
        drive_ws_stream(socket, stream, format).await;
    })
}

// VID common handlers (v1 semantics).

async fn get_vid_common<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    respond(
        &headers,
        load_vid_common(&state, BlockId::Number(height)).await,
    )
}

async fn get_vid_common_by_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let result = async {
        let hash = tb64_param::<BlockHash<Types>>(&hash, "hash")?;
        load_vid_common(&state, BlockId::Hash(hash)).await
    }
    .await;
    respond(&headers, result)
}

async fn get_vid_common_by_payload_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let result = async {
        let hash = tb64_param::<VidCommitment>(&hash, "payload-hash")?;
        load_vid_common(&state, BlockId::PayloadHash(hash)).await
    }
    .await;
    respond(&headers, result)
}

async fn get_vid_common_range<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((from, until)): Path<(usize, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    respond(&headers, load_vid_common_range(&state, from, until).await)
}

async fn stream_vid_common<Types, S>(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let format = ContentType::negotiate(&headers);
    ws.on_upgrade(move |socket| async move {
        let stream = state.data_source.subscribe_vid_common(height).await;
        drive_ws_stream(socket, stream, format).await;
    })
}

// Transaction handlers.

async fn get_transaction<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((height, index)): Path<(u64, u64)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let result = load_transaction_by_position(&state, height, index).await;
    respond(&headers, result.map(|tx| tx.transaction))
}

async fn get_transaction_by_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let result = async {
        let hash = tb64_param::<TransactionHash<Types>>(&hash, "hash")?;
        Ok(load_transaction_by_hash(&state, hash).await?.transaction)
    }
    .await;
    respond(&headers, result)
}

async fn get_transaction_proof<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((height, index)): Path<(u64, u64)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let result = async {
        let tx = load_transaction_by_position(&state, height, index).await?;
        prove_transaction(&state, tx).await
    }
    .await;
    respond(&headers, result)
}

async fn get_transaction_proof_by_hash<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let result = async {
        let hash = tb64_param::<TransactionHash<Types>>(&hash, "hash")?;
        let tx = load_transaction_by_hash(&state, hash).await?;
        prove_transaction(&state, tx).await
    }
    .await;
    respond(&headers, result)
}

async fn stream_transactions<Types, S>(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    stream_transactions_response::<Types, S>(ws, state, &headers, height, None)
}

async fn stream_transactions_in_namespace<Types, S>(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((height, namespace)): Path<(usize, usize)>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let namespace: i64 = match namespace.try_into() {
        Ok(ns) => ns,
        Err(err) => {
            return respond::<Vec<TransactionQueryData<Types>>>(
                &headers,
                Err(Error::Custom {
                    message: format!("Invalid 'namespace': could not convert usize to i64: {err}"),
                    status: StatusCode::BAD_REQUEST,
                }),
            );
        },
    };
    stream_transactions_response::<Types, S>(ws, state, &headers, height, Some(namespace))
}

fn stream_transactions_response<Types, S>(
    ws: WebSocketUpgrade,
    state: Arc<RouterState<S>>,
    headers: &HeaderMap,
    height: usize,
    namespace: Option<i64>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let format = ContentType::negotiate(headers);
    ws.on_upgrade(move |socket| async move {
        let stream = state
            .data_source
            .subscribe_blocks(height)
            .await
            .map(move |block| {
                let transactions = block.enumerate().enumerate();
                let header = block.header();
                let filtered_txs = transactions
                    .filter_map(|(i, (index, _tx))| {
                        if let Some(requested_ns) = namespace {
                            let ns_id =
                                QueryableHeader::<Types>::namespace_id(header, &index.ns_index)?;
                            if ns_id.into() != requested_ns {
                                return None;
                            }
                        }
                        let tx = block.transaction(&index)?;
                        TransactionQueryData::new(tx, &block, &index, i as u64)
                    })
                    .collect::<Vec<_>>();
                futures::stream::iter(filtered_txs)
            })
            .flatten()
            .boxed();
        drive_ws_stream(socket, stream, format).await;
    })
}

// Miscellaneous handlers.

async fn get_limits<S>(State(state): State<Arc<RouterState<S>>>, headers: HeaderMap) -> Response
where
    S: Send + Sync + 'static,
{
    respond(
        &headers,
        Ok(Limits {
            small_object_range_limit: state.small_object_range_limit,
            large_object_range_limit: state.large_object_range_limit,
        }),
    )
}

async fn get_cert2<Types, S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path(height): Path<u64>,
) -> Response
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: AvailabilityDataSource<Types> + Send + Sync + 'static,
{
    let result = state
        .data_source
        .get_cert2(height)
        .await
        .with_timeout(state.fetch_timeout)
        .await
        .ok_or_else(|| Error::Custom {
            message: format!("no cert2 available for height {height}"),
            status: StatusCode::NOT_FOUND,
        });
    respond(&headers, result)
}

#[cfg(test)]
mod test {
    use std::fmt::Debug;

    use committable::Committable;
    use http_client::{Client, ClientError};
    use serde::de::DeserializeOwned;
    use test_utils::reserve_tcp_port;

    use super::*;
    use crate::{
        availability::VerifiableInclusion,
        testing::{
            consensus::{MockDataSource, MockNetwork},
            mocks::{MockBase, MockTypes, mock_transaction},
        },
    };

    /// Serve `api` under the `/availability` prefix on a fresh port and return a connected
    /// client rooted at that prefix.
    async fn start_client(api: ApiRouter) -> Client<AppError, MockBase> {
        let port = reserve_tcp_port().unwrap();
        let url = format!("http://0.0.0.0:{port}").parse().unwrap();
        let _server = wire::spawn_serve(
            &url,
            app(Router::new().nest("/availability", Router::from(api))),
        );

        let client = Client::new(
            format!("http://localhost:{port}/availability")
                .parse()
                .unwrap(),
        );
        assert!(client.connect(Some(Duration::from_secs(60))).await);
        client
    }

    /// Get the current ledger height and a list of non-empty leaf/block pairs.
    async fn get_non_empty_blocks(
        client: &Client<AppError, MockBase>,
    ) -> (
        u64,
        Vec<(LeafQueryData<MockTypes>, BlockQueryData<MockTypes>)>,
    ) {
        let mut blocks = vec![];
        // Ignore the genesis block (start from height 1).
        for i in 1.. {
            match client
                .get::<BlockQueryData<MockTypes>>(&format!("block/{i}"))
                .send()
                .await
            {
                Ok(block) => {
                    if !block.is_empty() {
                        let leaf = client.get(&format!("leaf/{i}")).send().await.unwrap();
                        blocks.push((leaf, block));
                    }
                },
                Err(AppError::Availability {
                    source: Error::FetchBlock { .. },
                }) => {
                    tracing::info!(
                        "found end of ledger at height {i}, non-empty blocks are {blocks:?}",
                    );
                    return (i, blocks);
                },
                Err(err) => panic!("unexpected error {err}"),
            }
        }
        unreachable!()
    }

    async fn validate(client: &Client<AppError, MockBase>, height: u64) {
        // Check the consistency of every block/leaf pair.
        for i in 0..height {
            // Limit the number of blocks we validate in order to
            // speed up the tests.
            if ![0, 1, height / 2, height - 1].contains(&i) {
                continue;
            }
            tracing::info!("validate block {i}/{height}");

            // Check that looking up the leaf various ways returns the correct leaf.
            let leaf: LeafQueryData<MockTypes> =
                client.get(&format!("leaf/{i}")).send().await.unwrap();
            assert_eq!(leaf.height(), i);
            assert_eq!(
                leaf,
                client
                    .get(&format!("leaf/hash/{}", leaf.hash()))
                    .send()
                    .await
                    .unwrap()
            );

            // Check that looking up the block various ways returns the correct block.
            let block: BlockQueryData<MockTypes> =
                client.get(&format!("block/{i}")).send().await.unwrap();
            let expected_payload = PayloadQueryData::from(block.clone());
            assert_eq!(leaf.block_hash(), block.hash());
            assert_eq!(block.height(), i);
            assert_eq!(
                block,
                client
                    .get(&format!("block/hash/{}", block.hash()))
                    .send()
                    .await
                    .unwrap()
            );
            assert_eq!(
                *block.header(),
                client.get(&format!("header/{i}")).send().await.unwrap()
            );
            assert_eq!(
                *block.header(),
                client
                    .get(&format!("header/hash/{}", block.hash()))
                    .send()
                    .await
                    .unwrap()
            );
            assert_eq!(
                expected_payload,
                client.get(&format!("payload/{i}")).send().await.unwrap(),
            );
            assert_eq!(
                expected_payload,
                client
                    .get(&format!("payload/block-hash/{}", block.hash()))
                    .send()
                    .await
                    .unwrap(),
            );
            // Look up the common VID data.
            let common: VidCommonQueryData<MockTypes> = client
                .get(&format!("vid/common/{}", block.height()))
                .send()
                .await
                .unwrap();
            assert_eq!(common.height(), block.height());
            assert_eq!(common.block_hash(), block.hash());
            assert_eq!(common.payload_hash(), block.payload_hash());
            assert_eq!(
                common,
                client
                    .get(&format!("vid/common/hash/{}", block.hash()))
                    .send()
                    .await
                    .unwrap()
            );

            let block_summary = client
                .get(&format!("block/summary/{i}"))
                .send()
                .await
                .unwrap();
            assert_eq!(
                BlockSummaryQueryData::<MockTypes>::from(block.clone()),
                block_summary,
            );
            assert_eq!(block_summary.header(), block.header());
            assert_eq!(block_summary.hash(), block.hash());
            assert_eq!(block_summary.size(), block.size());
            assert_eq!(block_summary.num_transactions(), block.num_transactions());

            let block_summaries: Vec<BlockSummaryQueryData<MockTypes>> = client
                .get(&format!("block/summaries/{}/{}", 0, i))
                .send()
                .await
                .unwrap();
            assert_eq!(block_summaries.len() as u64, i);

            // We should be able to look up the block by payload hash. Note that for duplicate
            // payloads, these endpoints may return a different block with the same payload, which
            // is acceptable. Therefore, we don't check equivalence of the entire `BlockQueryData`
            // response, only its payload.
            assert_eq!(
                block.payload(),
                client
                    .get::<BlockQueryData<MockTypes>>(&format!(
                        "block/payload-hash/{}",
                        block.payload_hash()
                    ))
                    .send()
                    .await
                    .unwrap()
                    .payload()
            );
            assert_eq!(
                block.payload_hash(),
                client
                    .get::<Header<MockTypes>>(&format!(
                        "header/payload-hash/{}",
                        block.payload_hash()
                    ))
                    .send()
                    .await
                    .unwrap()
                    .payload_commitment
            );
            assert_eq!(
                block.payload(),
                client
                    .get::<PayloadQueryData<MockTypes>>(&format!(
                        "payload/hash/{}",
                        block.payload_hash()
                    ))
                    .send()
                    .await
                    .unwrap()
                    .data(),
            );
            assert_eq!(
                common.common(),
                client
                    .get::<VidCommonQueryData<MockTypes>>(&format!(
                        "vid/common/payload-hash/{}",
                        block.payload_hash()
                    ))
                    .send()
                    .await
                    .unwrap()
                    .common()
            );

            // Check that looking up each transaction in the block various ways returns the correct
            // transaction.
            for (j, txn_from_block) in block.enumerate() {
                let txn: TransactionQueryData<MockTypes> = client
                    .get(&format!("transaction/{}/{}/noproof", i, j.position))
                    .send()
                    .await
                    .unwrap();
                assert_eq!(txn.block_height(), i);
                assert_eq!(txn.block_hash(), block.hash());
                assert_eq!(txn.index(), j.position as u64);
                assert_eq!(txn.hash(), txn_from_block.commit());
                assert_eq!(txn.transaction(), &txn_from_block);
                // We should be able to look up the transaction by hash. Note that for duplicate
                // transactions, this endpoint may return a different transaction with the same
                // hash, which is acceptable. Therefore, we don't check equivalence of the entire
                // `TransactionWithProofQueryData` response, only its commitment.
                assert_eq!(
                    txn.hash(),
                    client
                        .get::<TransactionQueryData<MockTypes>>(&format!(
                            "transaction/hash/{}/noproof",
                            txn.hash()
                        ))
                        .send()
                        .await
                        .unwrap()
                        .hash()
                );

                let tx_with_proof = client
                    .get::<TransactionWithProofQueryData<MockTypes>>(&format!(
                        "transaction/{}/{}/proof",
                        i, j.position
                    ))
                    .send()
                    .await
                    .unwrap();
                assert_eq!(txn.hash(), tx_with_proof.hash());
                assert!(tx_with_proof.proof().verify(
                    block.metadata(),
                    txn.transaction(),
                    &block.payload_hash(),
                    common.common()
                ));

                // Similar to above, but by hash
                let tx_with_proof = client
                    .get::<TransactionWithProofQueryData<MockTypes>>(&format!(
                        "transaction/hash/{}/proof",
                        txn.hash()
                    ))
                    .send()
                    .await
                    .unwrap();
                assert_eq!(txn.hash(), tx_with_proof.hash());
                assert!(tx_with_proof.proof().verify(
                    block.metadata(),
                    txn.transaction(),
                    &block.payload_hash(),
                    common.common()
                ));
            }

            let block_range: Vec<BlockQueryData<MockTypes>> = client
                .get(&format!("block/{}/{}", 0, i))
                .send()
                .await
                .unwrap();

            assert_eq!(block_range.len() as u64, i);

            let leaf_range: Vec<LeafQueryData<MockTypes>> = client
                .get(&format!("leaf/{}/{}", 0, i))
                .send()
                .await
                .unwrap();

            assert_eq!(leaf_range.len() as u64, i);

            let payload_range: Vec<PayloadQueryData<MockTypes>> = client
                .get(&format!("payload/{}/{}", 0, i))
                .send()
                .await
                .unwrap();

            assert_eq!(payload_range.len() as u64, i);

            let vid_common_range: Vec<VidCommonQueryData<MockTypes>> = client
                .get(&format!("vid/common/{}/{}", 0, i))
                .send()
                .await
                .unwrap();

            assert_eq!(vid_common_range.len() as u64, i);

            let header_range: Vec<Header<MockTypes>> = client
                .get(&format!("header/{}/{}", 0, i))
                .send()
                .await
                .unwrap();

            assert_eq!(header_range.len() as u64, i);
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_api() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;
        network.start().await;

        // Start the web server.
        let options = Options {
            small_object_range_limit: 500,
            large_object_range_limit: 500,
            ..Default::default()
        };
        let client = start_client(availability_router::<MockTypes, _>(
            &options,
            network.data_source(),
        ))
        .await;
        assert_eq!(get_non_empty_blocks(&client).await.1, vec![]);

        // Submit a few blocks and make sure each one gets reflected in the query service and
        // preserves the consistency of the data and indices.
        let leaves = client
            .socket("stream/leaves/0")
            .subscribe::<LeafQueryData<MockTypes>>()
            .await
            .unwrap();
        let headers = client
            .socket("stream/headers/0")
            .subscribe::<Header<MockTypes>>()
            .await
            .unwrap();
        let blocks = client
            .socket("stream/blocks/0")
            .subscribe::<BlockQueryData<MockTypes>>()
            .await
            .unwrap();
        let vid_common = client
            .socket("stream/vid/common/0")
            .subscribe::<VidCommonQueryData<MockTypes>>()
            .await
            .unwrap();
        let mut chain = leaves.zip(headers.zip(blocks.zip(vid_common))).enumerate();
        for nonce in 0..3 {
            let txn = mock_transaction(vec![nonce]);
            network.submit_transaction(txn).await;

            // Wait for the transaction to be finalized.
            let (i, leaf, block, common) = loop {
                tracing::info!("waiting for block with transaction {}", nonce);
                let (i, (leaf, (header, (block, common)))) = chain.next().await.unwrap();
                tracing::info!(i, ?leaf, ?header, ?block, ?common);
                let leaf = leaf.unwrap();
                let header = header.unwrap();
                let block = block.unwrap();
                let common = common.unwrap();
                assert_eq!(leaf.height() as usize, i);
                assert_eq!(leaf.block_hash(), block.hash());
                assert_eq!(block.header(), &header);
                assert_eq!(common.height() as usize, i);
                if !block.is_empty() {
                    break (i, leaf, block, common);
                }
            };
            assert_eq!(leaf, client.get(&format!("leaf/{i}")).send().await.unwrap());
            assert_eq!(
                block,
                client.get(&format!("block/{i}")).send().await.unwrap()
            );
            assert_eq!(
                common,
                client.get(&format!("vid/common/{i}")).send().await.unwrap()
            );

            validate(&client, (i + 1) as u64).await;
        }

        network.shut_down().await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_api_epochs() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;
        let epoch_height = network.epoch_height();
        network.start().await;

        // Start the web server.
        let client = start_client(availability_router::<MockTypes, _>(
            &Default::default(),
            network.data_source(),
        ))
        .await;

        // Watch consensus progress through several epoch boundaries via the header stream.
        let headers = client
            .socket("stream/headers/0")
            .subscribe::<Header<MockTypes>>()
            .await
            .unwrap();
        let mut chain = headers.enumerate();

        loop {
            let (i, header) = chain.next().await.unwrap();
            let header = header.unwrap();
            assert_eq!(header.height(), i as u64);
            if header.height() >= 3 * epoch_height {
                break;
            }
        }

        network.shut_down().await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_range_limit() {
        let large_object_range_limit = 2;
        let small_object_range_limit = 3;

        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;
        network.start().await;

        // Start the web server.
        let client = start_client(availability_router::<MockTypes, _>(
            &Options {
                large_object_range_limit,
                small_object_range_limit,
                ..Default::default()
            },
            network.data_source(),
        ))
        .await;

        // Check reported limits.
        assert_eq!(
            client.get::<Limits>("limits").send().await.unwrap(),
            Limits {
                small_object_range_limit,
                large_object_range_limit
            }
        );

        // Wait for enough blocks to be produced.
        client
            .socket("stream/blocks/0")
            .subscribe::<BlockQueryData<MockTypes>>()
            .await
            .unwrap()
            .take(small_object_range_limit + 1)
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        async fn check_limit<T: DeserializeOwned + Debug>(
            client: &Client<AppError, MockBase>,
            req: &str,
            limit: usize,
        ) {
            let range: Vec<T> = client
                .get(&format!("{req}/0/{limit}"))
                .send()
                .await
                .unwrap();
            assert_eq!(range.len(), limit);
            let err = client
                .get::<Vec<T>>(&format!("{req}/0/{}", limit + 1))
                .send()
                .await
                .unwrap_err();
            assert_eq!(err.status(), http_client::StatusCode::BAD_REQUEST);
        }

        check_limit::<LeafQueryData<MockTypes>>(&client, "leaf", small_object_range_limit).await;
        check_limit::<Header<MockTypes>>(&client, "header", large_object_range_limit).await;
        check_limit::<BlockQueryData<MockTypes>>(&client, "block", large_object_range_limit).await;
        check_limit::<PayloadQueryData<MockTypes>>(&client, "payload", large_object_range_limit)
            .await;
        check_limit::<BlockSummaryQueryData<MockTypes>>(
            &client,
            "block/summaries",
            large_object_range_limit,
        )
        .await;

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
        let _ = availability_router::<MockTypes, _>(&Options::default(), data_source)
            .finish_api(&mut api);

        let paths = api.paths.expect("router registered paths");
        for route in [
            "/leaf/{height}",
            "/leaf/hash/{hash}",
            "/leaf/{from}/{until}",
            "/header/{height}",
            "/block/{height}",
            "/payload/{height}",
            "/vid/common/{height}",
            "/transaction/{height}/{index}",
            "/block/summary/{height}",
            "/cert2/{height}",
            "/limits",
            "/stream/leaves/{height}",
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
