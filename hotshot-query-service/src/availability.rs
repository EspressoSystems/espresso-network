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

//! Queries for HotShot chain state.
//!
//! The availability API provides an objective view of the HotShot blockchain. It provides access
//! only to normative data: that is, data which is agreed upon by all honest consensus nodes and
//! which is immutable. This means access to core consensus data structures including leaves,
//! blocks, and headers, where each query is pure and idempotent. This also means that it is
//! possible for a client to verify all of the information provided by this API, by running a
//! HotShot light client and downloading the appropriate evidence with each query.
//!
//! This API does not provide any queries which represent only the _current_ state of the chain or
//! may change over time, and it does not provide information for which there is not (yet) agreement
//! of a supermajority of consensus nodes. For information about the current dynamic state of
//! consensus and uncommitted state, try the [status](crate::status) API. For information about the
//! chain which is tabulated by this specific node and not subject to full consensus agreement, try
//! the [node](crate::node) API.

use std::{fmt::Display, path::PathBuf, time::Duration};

use derive_more::From;
use futures::{FutureExt, StreamExt, TryFutureExt, TryStreamExt};
use hotshot_types::{
    data::{Leaf, Leaf2, QuorumProposal, VidCommitment},
    simple_certificate::QuorumCertificate,
    traits::node_implementation::NodeType,
};
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, Snafu};
use tide_disco::{api::ApiError, method::ReadState, Api, RequestError, StatusCode};
use vbs::version::StaticVersionType;

use crate::{api::load_api, Payload, QueryError, VidCommon};

pub(crate) mod data_source;
mod fetch;
pub(crate) mod query_data;
pub use data_source::*;
pub use fetch::Fetch;
pub use query_data::*;

#[derive(Debug)]
pub struct Options {
    pub api_path: Option<PathBuf>,

    /// Timeout for failing requests due to missing data.
    ///
    /// If data needed to respond to a request is missing, it can (in some cases) be fetched from an
    /// external provider. This parameter controls how long the request handler will wait for
    /// missing data to be fetched before giving up and failing the request.
    pub fetch_timeout: Duration,

    /// Additional API specification files to merge with `availability-api-path`.
    ///
    /// These optional files may contain route definitions for application-specific routes that have
    /// been added as extensions to the basic availability API.
    pub extensions: Vec<toml::Value>,

    /// The maximum number of small objects which can be loaded in a single range query.
    ///
    /// Currently small objects include leaves only. In the future this limit will also apply to
    /// headers, block summaries, and VID common, however
    /// * loading of headers and block summaries is currently implemented by loading the entire
    ///   block
    /// * imperfect VID parameter tuning means that VID common can be much larger than it should
    pub small_object_range_limit: usize,

    /// The maximum number of large objects which can be loaded in a single range query.
    ///
    /// Large objects include anything that _might_ contain a full payload or an object proportional
    /// in size to a payload. Note that this limit applies to the entire class of objects: we do not
    /// check the size of objects while loading to determine which limit to apply. If an object
    /// belongs to a class which might contain a large payload, the large object limit always
    /// applies.
    pub large_object_range_limit: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            api_path: None,
            fetch_timeout: Duration::from_millis(500),
            extensions: vec![],
            large_object_range_limit: 100,
            small_object_range_limit: 500,
        }
    }
}

#[derive(Clone, Debug, From, Snafu, Deserialize, Serialize)]
#[snafu(visibility(pub))]
pub enum Error {
    Request {
        source: RequestError,
    },
    #[snafu(display("leaf {resource} missing or not available"))]
    #[from(ignore)]
    FetchLeaf {
        resource: String,
    },
    #[snafu(display("block {resource} missing or not available"))]
    #[from(ignore)]
    FetchBlock {
        resource: String,
    },
    #[snafu(display("header {resource} missing or not available"))]
    #[from(ignore)]
    FetchHeader {
        resource: String,
    },
    #[snafu(display("transaction {resource} missing or not available"))]
    #[from(ignore)]
    FetchTransaction {
        resource: String,
    },
    #[snafu(display("transaction index {index} out of range for block {height}"))]
    #[from(ignore)]
    InvalidTransactionIndex {
        height: u64,
        index: u64,
    },
    #[snafu(display("request for range {from}..{until} exceeds limit {limit}"))]
    #[from(ignore)]
    RangeLimit {
        from: usize,
        until: usize,
        limit: usize,
    },
    #[snafu(display("{source}"))]
    Query {
        source: QueryError,
    },
    #[snafu(display("State cert for epoch {epoch} not found"))]
    #[from(ignore)]
    FetchStateCert {
        epoch: u64,
    },
    Custom {
        message: String,
        status: StatusCode,
    },
}

impl Error {
    pub fn internal<M: Display>(message: M) -> Self {
        Self::Custom {
            message: message.to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn status(&self) -> StatusCode {
        match self {
            Self::Request { .. } | Self::RangeLimit { .. } => StatusCode::BAD_REQUEST,
            Self::FetchLeaf { .. }
            | Self::FetchBlock { .. }
            | Self::FetchTransaction { .. }
            | Self::FetchHeader { .. }
            | Self::FetchStateCert { .. } => StatusCode::NOT_FOUND,
            Self::InvalidTransactionIndex { .. } | Self::Query { .. } => StatusCode::NOT_FOUND,
            Self::Custom { status, .. } => *status,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct Leaf1QueryData<Types: NodeType> {
    pub(crate) leaf: Leaf<Types>,
    pub(crate) qc: QuorumCertificate<Types>,
}

impl<Types: NodeType> Leaf1QueryData<Types> {
    pub fn new(leaf: Leaf<Types>, qc: QuorumCertificate<Types>) -> Self {
        Self { leaf, qc }
    }

    pub fn leaf(&self) -> &Leaf<Types> {
        &self.leaf
    }

    pub fn qc(&self) -> &QuorumCertificate<Types> {
        &self.qc
    }
}

fn downgrade_leaf<Types: NodeType>(leaf2: Leaf2<Types>) -> Leaf<Types> {
    // TODO do we still need some check here?
    // `drb_seed` no longer exists on `Leaf2`
    // if leaf2.drb_seed != [0; 32] && leaf2.drb_result != [0; 32] {
    //     panic!("Downgrade of Leaf2 to Leaf will lose DRB information!");
    // }
    let quorum_proposal = QuorumProposal {
        block_header: leaf2.block_header().clone(),
        view_number: leaf2.view_number(),
        justify_qc: leaf2.justify_qc().to_qc(),
        upgrade_certificate: leaf2.upgrade_certificate(),
        proposal_certificate: None,
    };
    let mut leaf = Leaf::from_quorum_proposal(&quorum_proposal);
    if let Some(payload) = leaf2.block_payload() {
        leaf.fill_block_payload_unchecked(payload);
    }
    leaf
}

fn downgrade_leaf_query_data<Types: NodeType>(leaf: LeafQueryData<Types>) -> Leaf1QueryData<Types> {
    Leaf1QueryData {
        leaf: downgrade_leaf(leaf.leaf),
        qc: leaf.qc.to_qc(),
    }
}

async fn get_leaf_handler<Types, State>(
    req: tide_disco::RequestParams,
    state: &State,
    timeout: Duration,
) -> Result<LeafQueryData<Types>, Error>
where
    State: 'static + Send + Sync + ReadState,
    <State as ReadState>::State: Send + Sync + AvailabilityDataSource<Types>,
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    let id = match req.opt_integer_param("height")? {
        Some(height) => LeafId::Number(height),
        None => LeafId::Hash(req.blob_param("hash")?),
    };
    let fetch = state.read(|state| state.get_leaf(id).boxed()).await;
    fetch.with_timeout(timeout).await.context(FetchLeafSnafu {
        resource: id.to_string(),
    })
}

async fn get_leaf_range_handler<Types, State>(
    req: tide_disco::RequestParams,
    state: &State,
    timeout: Duration,
    small_object_range_limit: usize,
) -> Result<Vec<LeafQueryData<Types>>, Error>
where
    State: 'static + Send + Sync + ReadState,
    <State as ReadState>::State: Send + Sync + AvailabilityDataSource<Types>,
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    let from = req.integer_param::<_, usize>("from")?;
    let until = req.integer_param("until")?;
    enforce_range_limit(from, until, small_object_range_limit)?;

    let leaves = state
        .read(|state| state.get_leaf_range(from..until).boxed())
        .await;
    leaves
        .enumerate()
        .then(|(index, fetch)| async move {
            fetch.with_timeout(timeout).await.context(FetchLeafSnafu {
                resource: (index + from).to_string(),
            })
        })
        .try_collect::<Vec<_>>()
        .await
}

fn downgrade_vid_common_query_data<Types: NodeType>(
    data: VidCommonQueryData<Types>,
) -> Option<ADVZCommonQueryData<Types>> {
    let VidCommonQueryData {
        height,
        block_hash,
        payload_hash: VidCommitment::V0(payload_hash),
        common: VidCommon::V0(common),
    } = data
    else {
        return None;
    };
    Some(ADVZCommonQueryData {
        height,
        block_hash,
        payload_hash,
        common,
    })
}

async fn get_vid_common_handler<Types, State>(
    req: tide_disco::RequestParams,
    state: &State,
    timeout: Duration,
) -> Result<VidCommonQueryData<Types>, Error>
where
    State: 'static + Send + Sync + ReadState,
    <State as ReadState>::State: Send + Sync + AvailabilityDataSource<Types>,
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    let id = if let Some(height) = req.opt_integer_param("height")? {
        BlockId::Number(height)
    } else if let Some(hash) = req.opt_blob_param("hash")? {
        BlockId::Hash(hash)
    } else {
        BlockId::PayloadHash(req.blob_param("payload-hash")?)
    };
    let fetch = state.read(|state| state.get_vid_common(id).boxed()).await;
    fetch.with_timeout(timeout).await.context(FetchBlockSnafu {
        resource: id.to_string(),
    })
}

pub fn define_api<State, Types: NodeType, Ver: StaticVersionType + 'static>(
    options: &Options,
    _: Ver,
    api_ver: semver::Version,
) -> Result<Api<State, Error, Ver>, ApiError>
where
    State: 'static + Send + Sync + ReadState,
    <State as ReadState>::State: Send + Sync + AvailabilityDataSource<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    let mut api = load_api::<State, Error, Ver>(
        options.api_path.as_ref(),
        include_str!("../api/availability.toml"),
        options.extensions.clone(),
    )?;
    let timeout = options.fetch_timeout;
    let small_object_range_limit = options.small_object_range_limit;
    let large_object_range_limit = options.large_object_range_limit;

    api.with_version(api_ver.clone());

    // `LeafQueryData` now contains `Leaf2` and `QC2``, which is a breaking change.
    // On node startup, all leaves are migrated to `Leaf2`.
    //
    // To maintain compatibility with nodes running an older version
    // (which expect `LeafQueryData` with `Leaf1` and `QC1`),
    // we downgrade `Leaf2` to `Leaf1` and `QC2` to `QC1` if the API version is V0.
    // Otherwise, we return the new types.
    if api_ver.major == 0 {
        api.at("get_leaf", move |req, state| {
            get_leaf_handler(req, state, timeout)
                .map(|res| res.map(downgrade_leaf_query_data))
                .boxed()
        })?;

        api.at("get_leaf_range", move |req, state| {
            get_leaf_range_handler(req, state, timeout, small_object_range_limit)
                .map(|res| {
                    res.map(|r| {
                        r.into_iter()
                            .map(downgrade_leaf_query_data)
                            .collect::<Vec<Leaf1QueryData<_>>>()
                    })
                })
                .boxed()
        })?;

        api.stream("stream_leaves", move |req, state| {
            async move {
                let height = req.integer_param("height")?;
                state
                    .read(|state| {
                        async move {
                            Ok(state
                                .subscribe_leaves(height)
                                .await
                                .map(|leaf| Ok(downgrade_leaf_query_data(leaf))))
                        }
                        .boxed()
                    })
                    .await
            }
            .try_flatten_stream()
            .boxed()
        })?;
    } else {
        api.at("get_leaf", move |req, state| {
            get_leaf_handler(req, state, timeout).boxed()
        })?;

        api.at("get_leaf_range", move |req, state| {
            get_leaf_range_handler(req, state, timeout, small_object_range_limit).boxed()
        })?;

        api.stream("stream_leaves", move |req, state| {
            async move {
                let height = req.integer_param("height")?;
                state
                    .read(|state| {
                        async move { Ok(state.subscribe_leaves(height).await.map(Ok)) }.boxed()
                    })
                    .await
            }
            .try_flatten_stream()
            .boxed()
        })?;
    }

    // VIDCommon data is version gated after the VID upgrade.
    // We keep the old struct and data in the API version V0. Starting from V1 we are returning version gated structs.
    if api_ver.major == 0 {
        api.at("get_vid_common", move |req, state| {
            get_vid_common_handler(req, state, timeout)
                .map(|r| match r {
                    Ok(data) => downgrade_vid_common_query_data(data).ok_or(Error::Custom {
                        message: "Incompatible VID version.".to_string(),
                        status: StatusCode::BAD_REQUEST,
                    }),
                    Err(e) => Err(e),
                })
                .boxed()
        })?
        .stream("stream_vid_common", move |req, state| {
            async move {
                let height = req.integer_param("height")?;
                state
                    .read(|state| {
                        async move {
                            Ok(state.subscribe_vid_common(height).await.map(|data| {
                                downgrade_vid_common_query_data(data).ok_or(Error::Custom {
                                    message: "Incompatible VID version.".to_string(),
                                    status: StatusCode::BAD_REQUEST,
                                })
                            }))
                        }
                        .boxed()
                    })
                    .await
            }
            .try_flatten_stream()
            .boxed()
        })?;
    } else {
        api.at("get_vid_common", move |req, state| {
            get_vid_common_handler(req, state, timeout).boxed().boxed()
        })?
        .stream("stream_vid_common", move |req, state| {
            async move {
                let height = req.integer_param("height")?;
                state
                    .read(|state| {
                        async move { Ok(state.subscribe_vid_common(height).await.map(Ok)) }.boxed()
                    })
                    .await
            }
            .try_flatten_stream()
            .boxed()
        })?;
    }

    api.at("get_header", move |req, state| {
        async move {
            let id = if let Some(height) = req.opt_integer_param("height")? {
                BlockId::Number(height)
            } else if let Some(hash) = req.opt_blob_param("hash")? {
                BlockId::Hash(hash)
            } else {
                BlockId::PayloadHash(req.blob_param("payload-hash")?)
            };
            let fetch = state.read(|state| state.get_header(id).boxed()).await;
            fetch.with_timeout(timeout).await.context(FetchHeaderSnafu {
                resource: id.to_string(),
            })
        }
        .boxed()
    })?
    .at("get_header_range", move |req, state| {
        async move {
            let from = req.integer_param::<_, usize>("from")?;
            let until = req.integer_param::<_, usize>("until")?;
            enforce_range_limit(from, until, large_object_range_limit)?;

            let headers = state
                .read(|state| state.get_header_range(from..until).boxed())
                .await;
            headers
                .enumerate()
                .then(|(index, fetch)| async move {
                    fetch.with_timeout(timeout).await.context(FetchHeaderSnafu {
                        resource: (index + from).to_string(),
                    })
                })
                .try_collect::<Vec<_>>()
                .await
        }
        .boxed()
    })?
    .stream("stream_headers", move |req, state| {
        async move {
            let height = req.integer_param("height")?;
            state
                .read(|state| {
                    async move { Ok(state.subscribe_headers(height).await.map(Ok)) }.boxed()
                })
                .await
        }
        .try_flatten_stream()
        .boxed()
    })?
    .at("get_block", move |req, state| {
        async move {
            let id = if let Some(height) = req.opt_integer_param("height")? {
                BlockId::Number(height)
            } else if let Some(hash) = req.opt_blob_param("hash")? {
                BlockId::Hash(hash)
            } else {
                BlockId::PayloadHash(req.blob_param("payload-hash")?)
            };
            let fetch = state.read(|state| state.get_block(id).boxed()).await;
            fetch.with_timeout(timeout).await.context(FetchBlockSnafu {
                resource: id.to_string(),
            })
        }
        .boxed()
    })?
    .at("get_block_range", move |req, state| {
        async move {
            let from = req.integer_param::<_, usize>("from")?;
            let until = req.integer_param("until")?;
            enforce_range_limit(from, until, large_object_range_limit)?;

            let blocks = state
                .read(|state| state.get_block_range(from..until).boxed())
                .await;
            blocks
                .enumerate()
                .then(|(index, fetch)| async move {
                    fetch.with_timeout(timeout).await.context(FetchBlockSnafu {
                        resource: (index + from).to_string(),
                    })
                })
                .try_collect::<Vec<_>>()
                .await
        }
        .boxed()
    })?
    .stream("stream_blocks", move |req, state| {
        async move {
            let height = req.integer_param("height")?;
            state
                .read(|state| {
                    async move { Ok(state.subscribe_blocks(height).await.map(Ok)) }.boxed()
                })
                .await
        }
        .try_flatten_stream()
        .boxed()
    })?
    .at("get_payload", move |req, state| {
        async move {
            let id = if let Some(height) = req.opt_integer_param("height")? {
                BlockId::Number(height)
            } else if let Some(hash) = req.opt_blob_param("hash")? {
                BlockId::PayloadHash(hash)
            } else {
                BlockId::Hash(req.blob_param("block-hash")?)
            };
            let fetch = state.read(|state| state.get_payload(id).boxed()).await;
            fetch.with_timeout(timeout).await.context(FetchBlockSnafu {
                resource: id.to_string(),
            })
        }
        .boxed()
    })?
    .at("get_payload_range", move |req, state| {
        async move {
            let from = req.integer_param::<_, usize>("from")?;
            let until = req.integer_param("until")?;
            enforce_range_limit(from, until, large_object_range_limit)?;

            let payloads = state
                .read(|state| state.get_payload_range(from..until).boxed())
                .await;
            payloads
                .enumerate()
                .then(|(index, fetch)| async move {
                    fetch.with_timeout(timeout).await.context(FetchBlockSnafu {
                        resource: (index + from).to_string(),
                    })
                })
                .try_collect::<Vec<_>>()
                .await
        }
        .boxed()
    })?
    .stream("stream_payloads", move |req, state| {
        async move {
            let height = req.integer_param("height")?;
            state
                .read(|state| {
                    async move { Ok(state.subscribe_payloads(height).await.map(Ok)) }.boxed()
                })
                .await
        }
        .try_flatten_stream()
        .boxed()
    })?
    .at("get_transaction", move |req, state| {
        async move {
            match req.opt_blob_param("hash")? {
                Some(hash) => {
                    let fetch = state
                        .read(|state| state.get_transaction(hash).boxed())
                        .await;
                    fetch
                        .with_timeout(timeout)
                        .await
                        .context(FetchTransactionSnafu {
                            resource: hash.to_string(),
                        })
                },
                None => {
                    let height: u64 = req.integer_param("height")?;
                    let fetch = state
                        .read(|state| state.get_block(height as usize).boxed())
                        .await;
                    let block = fetch.with_timeout(timeout).await.context(FetchBlockSnafu {
                        resource: height.to_string(),
                    })?;
                    let i: u64 = req.integer_param("index")?;
                    let index = block
                        .payload()
                        .nth(block.metadata(), i as usize)
                        .context(InvalidTransactionIndexSnafu { height, index: i })?;
                    TransactionQueryData::new(&block, index, i)
                        .context(InvalidTransactionIndexSnafu { height, index: i })
                },
            }
        }
        .boxed()
    })?
    .at("get_block_summary", move |req, state| {
        async move {
            let id: usize = req.integer_param("height")?;

            let fetch = state.read(|state| state.get_block(id).boxed()).await;
            fetch
                .with_timeout(timeout)
                .await
                .context(FetchBlockSnafu {
                    resource: id.to_string(),
                })
                .map(BlockSummaryQueryData::from)
        }
        .boxed()
    })?
    .at("get_block_summary_range", move |req, state| {
        async move {
            let from: usize = req.integer_param("from")?;
            let until: usize = req.integer_param("until")?;
            enforce_range_limit(from, until, large_object_range_limit)?;

            let blocks = state
                .read(|state| state.get_block_range(from..until).boxed())
                .await;
            let result: Vec<BlockSummaryQueryData<Types>> = blocks
                .enumerate()
                .then(|(index, fetch)| async move {
                    fetch.with_timeout(timeout).await.context(FetchBlockSnafu {
                        resource: (index + from).to_string(),
                    })
                })
                .map(|result| result.map(BlockSummaryQueryData::from))
                .try_collect()
                .await?;

            Ok(result)
        }
        .boxed()
    })?
    .at("get_limits", move |_req, _state| {
        async move {
            Ok(Limits {
                small_object_range_limit,
                large_object_range_limit,
            })
        }
        .boxed()
    })?
    .at("get_state_cert", move |req, state| {
        async move {
            let epoch = req.integer_param("epoch")?;
            let fetch = state
                .read(|state| state.get_state_cert(epoch).boxed())
                .await;
            fetch
                .with_timeout(timeout)
                .await
                .context(FetchStateCertSnafu { epoch })
        }
        .boxed()
    })?;
    Ok(api)
}

fn enforce_range_limit(from: usize, until: usize, limit: usize) -> Result<(), Error> {
    if until.saturating_sub(from) > limit {
        return Err(Error::RangeLimit { from, until, limit });
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use std::{fmt::Debug, time::Duration};

    use async_lock::RwLock;
    use committable::Committable;
    use futures::future::FutureExt;
    use hotshot_example_types::node_types::EpochsTestVersions;
    use hotshot_types::{
        data::Leaf2, simple_certificate::QuorumCertificate2,
        traits::node_implementation::ConsensusTime,
    };
    use portpicker::pick_unused_port;
    use serde::de::DeserializeOwned;
    use surf_disco::{Client, Error as _};
    use tempfile::TempDir;
    use tide_disco::App;
    use toml::toml;

    use super::*;
    use crate::{
        data_source::{storage::AvailabilityStorage, ExtensibleDataSource, VersionedDataSource},
        status::StatusDataSource,
        task::BackgroundTask,
        testing::{
            consensus::{MockDataSource, MockNetwork, MockSqlDataSource},
            mocks::{mock_transaction, MockBase, MockHeader, MockPayload, MockTypes, MockVersions},
            setup_test,
        },
        types::HeightIndexed,
        ApiState, Error, Header,
    };

    /// Get the current ledger height and a list of non-empty leaf/block pairs.
    async fn get_non_empty_blocks(
        client: &Client<Error, MockBase>,
    ) -> (
        u64,
        Vec<(LeafQueryData<MockTypes>, BlockQueryData<MockTypes>)>,
    ) {
        let mut blocks = vec![];
        // Ignore the genesis block (start from height 1).
        for i in 1.. {
            match client
                .get::<BlockQueryData<MockTypes>>(&format!("block/{}", i))
                .send()
                .await
            {
                Ok(block) => {
                    if !block.is_empty() {
                        let leaf = client.get(&format!("leaf/{}", i)).send().await.unwrap();
                        blocks.push((leaf, block));
                    }
                },
                Err(Error::Availability {
                    source: super::Error::FetchBlock { .. },
                }) => {
                    tracing::info!(
                        "found end of ledger at height {i}, non-empty blocks are {blocks:?}",
                    );
                    return (i, blocks);
                },
                Err(err) => panic!("unexpected error {}", err),
            }
        }
        unreachable!()
    }

    async fn validate(client: &Client<Error, MockBase>, height: u64) {
        // Check the consistency of every block/leaf pair.
        for i in 0..height {
            // Limit the number of blocks we validate in order to
            // speeed up the tests.
            if ![0, 1, height / 2, height - 1].contains(&i) {
                continue;
            }
            tracing::info!("validate block {i}/{height}");

            // Check that looking up the leaf various ways returns the correct leaf.
            let leaf: LeafQueryData<MockTypes> =
                client.get(&format!("leaf/{}", i)).send().await.unwrap();
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
                client.get(&format!("block/{}", i)).send().await.unwrap();
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
                .get(&format!("block/summary/{}", i))
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
                    .get(&format!("transaction/{}/{}", i, j))
                    .send()
                    .await
                    .unwrap();
                assert_eq!(txn.block_height(), i);
                assert_eq!(txn.block_hash(), block.hash());
                assert_eq!(txn.index(), j as u64);
                assert_eq!(txn.hash(), txn_from_block.commit());
                assert_eq!(txn.transaction(), &txn_from_block);
                // We should be able to look up the transaction by hash. Note that for duplicate
                // transactions, this endpoint may return a different transaction with the same
                // hash, which is acceptable. Therefore, we don't check equivalence of the entire
                // `TransactionQueryData` response, only its commitment.
                assert_eq!(
                    txn.hash(),
                    client
                        .get::<TransactionQueryData<MockTypes>>(&format!(
                            "transaction/hash/{}",
                            txn.hash()
                        ))
                        .send()
                        .await
                        .unwrap()
                        .hash()
                );
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

            let header_range: Vec<Header<MockTypes>> = client
                .get(&format!("header/{}/{}", 0, i))
                .send()
                .await
                .unwrap();

            assert_eq!(header_range.len() as u64, i);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_api() {
        setup_test();

        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;
        network.start().await;

        // Start the web server.
        let port = pick_unused_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        let options = Options {
            small_object_range_limit: 500,
            large_object_range_limit: 500,
            ..Default::default()
        };

        app.register_module(
            "availability",
            define_api(&options, MockBase::instance(), "1.0.0".parse().unwrap()).unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{}", port), MockBase::instance()),
        );

        // Start a client.
        let client = Client::<Error, MockBase>::new(
            format!("http://localhost:{}/availability", port)
                .parse()
                .unwrap(),
        );
        assert!(client.connect(Some(Duration::from_secs(60))).await);
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
            assert_eq!(
                leaf,
                client.get(&format!("leaf/{}", i)).send().await.unwrap()
            );
            assert_eq!(
                block,
                client.get(&format!("block/{}", i)).send().await.unwrap()
            );
            assert_eq!(
                common,
                client.get(&format!("vid/common/{i}")).send().await.unwrap()
            );

            validate(&client, (i + 1) as u64).await;
        }

        network.shut_down().await;
    }

    async fn validate_old(client: &Client<Error, MockBase>, height: u64) {
        // Check the consistency of every block/leaf pair.
        for i in 0..height {
            // Limit the number of blocks we validate in order to
            // speeed up the tests.
            if ![0, 1, height / 2, height - 1].contains(&i) {
                continue;
            }
            tracing::info!("validate block {i}/{height}");

            // Check that looking up the leaf various ways returns the correct leaf.
            let leaf: Leaf1QueryData<MockTypes> =
                client.get(&format!("leaf/{}", i)).send().await.unwrap();
            assert_eq!(leaf.leaf.height(), i);
            assert_eq!(
                leaf,
                client
                    .get(&format!(
                        "leaf/hash/{}",
                        <Leaf<MockTypes> as Committable>::commit(&leaf.leaf)
                    ))
                    .send()
                    .await
                    .unwrap()
            );

            // Check that looking up the block various ways returns the correct block.
            let block: BlockQueryData<MockTypes> =
                client.get(&format!("block/{}", i)).send().await.unwrap();
            let expected_payload = PayloadQueryData::from(block.clone());
            assert_eq!(leaf.leaf.block_header().commit(), block.hash());
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
            let common: ADVZCommonQueryData<MockTypes> = client
                .get(&format!("vid/common/{}", block.height()))
                .send()
                .await
                .unwrap();
            assert_eq!(common.height(), block.height());
            assert_eq!(common.block_hash(), block.hash());
            assert_eq!(
                VidCommitment::V0(common.payload_hash()),
                block.payload_hash(),
            );
            assert_eq!(
                common,
                client
                    .get(&format!("vid/common/hash/{}", block.hash()))
                    .send()
                    .await
                    .unwrap()
            );

            let block_summary = client
                .get(&format!("block/summary/{}", i))
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
                    .get::<ADVZCommonQueryData<MockTypes>>(&format!(
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
                    .get(&format!("transaction/{}/{}", i, j))
                    .send()
                    .await
                    .unwrap();
                assert_eq!(txn.block_height(), i);
                assert_eq!(txn.block_hash(), block.hash());
                assert_eq!(txn.index(), j as u64);
                assert_eq!(txn.hash(), txn_from_block.commit());
                assert_eq!(txn.transaction(), &txn_from_block);
                // We should be able to look up the transaction by hash. Note that for duplicate
                // transactions, this endpoint may return a different transaction with the same
                // hash, which is acceptable. Therefore, we don't check equivalence of the entire
                // `TransactionQueryData` response, only its commitment.
                assert_eq!(
                    txn.hash(),
                    client
                        .get::<TransactionQueryData<MockTypes>>(&format!(
                            "transaction/hash/{}",
                            txn.hash()
                        ))
                        .send()
                        .await
                        .unwrap()
                        .hash()
                );
            }

            let block_range: Vec<BlockQueryData<MockTypes>> = client
                .get(&format!("block/{}/{}", 0, i))
                .send()
                .await
                .unwrap();

            assert_eq!(block_range.len() as u64, i);

            let leaf_range: Vec<Leaf1QueryData<MockTypes>> = client
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

            let header_range: Vec<Header<MockTypes>> = client
                .get(&format!("header/{}/{}", 0, i))
                .send()
                .await
                .unwrap();

            assert_eq!(header_range.len() as u64, i);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_api_epochs() {
        setup_test();

        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource, EpochsTestVersions>::init().await;
        let epoch_height = network.epoch_height();
        network.start().await;

        // Start the web server.
        let port = pick_unused_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{}", port), MockBase::instance()),
        );

        // Start a client.
        let client = Client::<Error, MockBase>::new(
            format!("http://localhost:{}/availability", port)
                .parse()
                .unwrap(),
        );
        assert!(client.connect(Some(Duration::from_secs(60))).await);

        // Submit a few blocks and make sure each one gets reflected in the query service and
        // preserves the consistency of the data and indices.
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

        for epoch in 1..4 {
            let state_cert: StateCertQueryData<MockTypes> = client
                .get(&format!("state-cert/{}", epoch))
                .send()
                .await
                .unwrap();
            tracing::info!("state-cert: {state_cert:?}");
            assert_eq!(state_cert.0.epoch.u64(), epoch);
        }

        network.shut_down().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_old_api() {
        setup_test();

        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;
        network.start().await;

        // Start the web server.
        let port = pick_unused_port().unwrap();

        let options = Options {
            small_object_range_limit: 500,
            large_object_range_limit: 500,
            ..Default::default()
        };

        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(&options, MockBase::instance(), "0.1.0".parse().unwrap()).unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{}", port), MockBase::instance()),
        );

        // Start a client.
        let client = Client::<Error, MockBase>::new(
            format!("http://localhost:{}/availability", port)
                .parse()
                .unwrap(),
        );
        assert!(client.connect(Some(Duration::from_secs(60))).await);
        assert_eq!(get_non_empty_blocks(&client).await.1, vec![]);

        // Submit a few blocks and make sure each one gets reflected in the query service and
        // preserves the consistency of the data and indices.
        let leaves = client
            .socket("stream/leaves/0")
            .subscribe::<Leaf1QueryData<MockTypes>>()
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
            .subscribe::<ADVZCommonQueryData<MockTypes>>()
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
                assert_eq!(leaf.leaf.height() as usize, i);
                assert_eq!(leaf.leaf.block_header().commit(), block.hash());
                assert_eq!(block.header(), &header);
                assert_eq!(common.height() as usize, i);
                if !block.is_empty() {
                    break (i, leaf, block, common);
                }
            };
            assert_eq!(
                leaf,
                client.get(&format!("leaf/{}", i)).send().await.unwrap()
            );
            assert_eq!(
                block,
                client.get(&format!("block/{}", i)).send().await.unwrap()
            );
            assert_eq!(
                common,
                client.get(&format!("vid/common/{i}")).send().await.unwrap()
            );

            validate_old(&client, (i + 1) as u64).await;
        }

        network.shut_down().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_extensions() {
        use hotshot_example_types::node_types::TestVersions;

        setup_test();

        let dir = TempDir::with_prefix("test_availability_extensions").unwrap();
        let data_source = ExtensibleDataSource::new(
            MockDataSource::create(dir.path(), Default::default())
                .await
                .unwrap(),
            0,
        );

        // mock up some consensus data.
        let leaf =
            Leaf2::<MockTypes>::genesis::<MockVersions>(&Default::default(), &Default::default())
                .await;
        let qc =
            QuorumCertificate2::genesis::<TestVersions>(&Default::default(), &Default::default())
                .await;
        let leaf = LeafQueryData::new(leaf, qc).unwrap();
        let block = BlockQueryData::new(leaf.header().clone(), MockPayload::genesis());
        data_source
            .append(BlockInfo::new(leaf, Some(block.clone()), None, None, None))
            .await
            .unwrap();

        // assert that the store has data before we move on to API requests
        assert_eq!(
            ExtensibleDataSource::<MockDataSource, u64>::block_height(&data_source)
                .await
                .unwrap(),
            1
        );
        assert_eq!(block, data_source.get_block(0).await.await);

        // Create the API extensions specification.
        let extensions = toml! {
            [route.post_ext]
            PATH = ["/ext/:val"]
            METHOD = "POST"
            ":val" = "Integer"

            [route.get_ext]
            PATH = ["/ext"]
            METHOD = "GET"
        };

        let mut api =
            define_api::<RwLock<ExtensibleDataSource<MockDataSource, u64>>, MockTypes, MockBase>(
                &Options {
                    extensions: vec![extensions.into()],
                    ..Default::default()
                },
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap();
        api.get("get_ext", |_, state| {
            async move { Ok(*state.as_ref()) }.boxed()
        })
        .unwrap()
        .post("post_ext", |req, state| {
            async move {
                *state.as_mut() = req.integer_param("val")?;
                Ok(())
            }
            .boxed()
        })
        .unwrap();

        let mut app = App::<_, Error>::with_state(RwLock::new(data_source));
        app.register_module("availability", api).unwrap();

        let port = pick_unused_port().unwrap();
        let _server = BackgroundTask::spawn(
            "server",
            app.serve(format!("0.0.0.0:{}", port), MockBase::instance()),
        );

        let client = Client::<Error, MockBase>::new(
            format!("http://localhost:{}/availability", port)
                .parse()
                .unwrap(),
        );
        assert!(client.connect(Some(Duration::from_secs(60))).await);

        assert_eq!(client.get::<u64>("ext").send().await.unwrap(), 0);
        client.post::<()>("ext/42").send().await.unwrap();
        assert_eq!(client.get::<u64>("ext").send().await.unwrap(), 42);

        // Ensure we can still access the built-in functionality.
        assert_eq!(
            client
                .get::<MockHeader>("header/0")
                .send()
                .await
                .unwrap()
                .block_number,
            0
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_range_limit() {
        setup_test();

        let large_object_range_limit = 2;
        let small_object_range_limit = 3;

        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;
        network.start().await;

        // Start the web server.
        let port = pick_unused_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Options {
                    large_object_range_limit,
                    small_object_range_limit,
                    ..Default::default()
                },
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{}", port), MockBase::instance()),
        );

        // Start a client.
        let client = Client::<Error, MockBase>::new(
            format!("http://localhost:{}/availability", port)
                .parse()
                .unwrap(),
        );
        assert!(client.connect(Some(Duration::from_secs(60))).await);

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
            client: &Client<Error, MockBase>,
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
            assert_eq!(err.status(), StatusCode::BAD_REQUEST);
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

    #[tokio::test(flavor = "multi_thread")]
    async fn test_header_endpoint() {
        setup_test();

        // Create the consensus network.
        let mut network = MockNetwork::<MockSqlDataSource, MockVersions>::init().await;
        network.start().await;

        // Start the web server.
        let port = pick_unused_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{}", port), MockBase::instance()),
        );

        let ds = network.data_source();

        // Get the current block height and fetch header for some later block height
        // This fetch will only resolve when we receive a leaf or block for that block height
        let block_height = ds.block_height().await.unwrap();
        let fetch = ds
            .get_header(BlockId::<MockTypes>::Number(block_height + 25))
            .await;

        assert!(fetch.is_pending());
        let header = fetch.await;
        assert_eq!(header.height() as usize, block_height + 25);

        network.shut_down().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_leaf_only_ds() {
        setup_test();

        // Create the consensus network.
        let mut network = MockNetwork::<MockSqlDataSource, MockVersions>::init_with_leaf_ds().await;
        network.start().await;

        // Start the web server.
        let port = pick_unused_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{}", port), MockBase::instance()),
        );

        // Start a client.
        let client = Client::<Error, MockBase>::new(
            format!("http://localhost:{}/availability", port)
                .parse()
                .unwrap(),
        );
        assert!(client.connect(Some(Duration::from_secs(60))).await);

        // Wait for some headers to be produced.
        client
            .socket("stream/headers/0")
            .subscribe::<Header<MockTypes>>()
            .await
            .unwrap()
            .take(5)
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        // Wait for some leaves to be produced.
        client
            .socket("stream/leaves/5")
            .subscribe::<LeafQueryData<MockTypes>>()
            .await
            .unwrap()
            .take(5)
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        let ds = network.data_source();

        // Get the current block height and fetch header for some later block height
        // This fetch will only resolve if we get a block notification
        // However, this block will never be stored
        let block_height = ds.block_height().await.unwrap();
        let target_block_height = block_height + 20;
        let fetch = ds
            .get_block(BlockId::<MockTypes>::Number(target_block_height))
            .await;

        assert!(fetch.is_pending());
        let block = fetch.await;
        assert_eq!(block.height() as usize, target_block_height);

        let mut tx = ds.read().await.unwrap();
        tx.get_block(BlockId::<MockTypes>::Number(target_block_height))
            .await
            .unwrap_err();
        drop(tx);

        network.shut_down().await;
    }
}
