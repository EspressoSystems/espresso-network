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

//! A node's view of a HotShot chain
//!
//! The node API provides a subjective view of the HotShot blockchain, from the perspective of
//! one particular node. It provides access to information that the
//! [availability](crate::availability) API does not, because this information depends on the
//! perspective of the node observing it, and may be subject to eventual consistency. For example,
//! `/node/block-height` may return smaller counts than expected, if the node being queried is not
//! fully synced with the entire history of the chain. However, the node will _eventually_ sync and
//! return the expected counts.

use std::{fmt::Display, ops::Bound, path::PathBuf};

use derive_more::From;
use futures::FutureExt;
use hotshot_types::traits::node_implementation::NodeType;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use tide_disco::{api::ApiError, method::ReadState, Api, RequestError, StatusCode};
use vbs::version::StaticVersionType;

use crate::{api::load_api, QueryError};

pub(crate) mod data_source;
pub(crate) mod query_data;
pub use data_source::*;
pub use query_data::*;

#[derive(Debug)]
pub struct Options {
    pub api_path: Option<PathBuf>,

    /// Additional API specification files to merge with `node-api-path`.
    ///
    /// These optional files may contain route definitions for application-specific routes that have
    /// been added as extensions to the basic node API.
    pub extensions: Vec<toml::Value>,

    /// The maximum number of headers which can be loaded in a single `header/window` query.
    pub window_limit: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            api_path: None,
            extensions: vec![],
            window_limit: 500,
        }
    }
}

#[derive(Clone, Debug, From, Snafu, Deserialize, Serialize)]
#[snafu(visibility(pub))]
pub enum Error {
    Request {
        source: RequestError,
    },
    #[snafu(display("{source}"))]
    Query {
        source: QueryError,
    },
    #[snafu(display("error fetching VID share for block {block}: {source}"))]
    #[from(ignore)]
    QueryVid {
        source: QueryError,
        block: String,
    },
    #[snafu(display(
        "error fetching window starting from {start} and ending at time {end}: {source}"
    ))]
    #[from(ignore)]
    QueryWindow {
        source: QueryError,
        start: String,
        end: u64,
    },
    #[snafu(display("error {status}: {message}"))]
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
            Self::Request { .. } => StatusCode::BAD_REQUEST,
            Self::Query { source, .. }
            | Self::QueryVid { source, .. }
            | Self::QueryWindow { source, .. } => source.status(),
            Self::Custom { status, .. } => *status,
        }
    }
}

pub fn define_api<State, Types: NodeType, Ver: StaticVersionType + 'static>(
    options: &Options,
    _: Ver,
    api_ver: semver::Version,
) -> Result<Api<State, Error, Ver>, ApiError>
where
    State: 'static + Send + Sync + ReadState,
    <State as ReadState>::State: NodeDataSource<Types> + Send + Sync,
{
    let mut api = load_api::<State, Error, Ver>(
        options.api_path.as_ref(),
        include_str!("../api/node.toml"),
        options.extensions.clone(),
    )?;
    let window_limit = options.window_limit;
    api.with_version(api_ver)
        .get("block_height", |_req, state| {
            async move { state.block_height().await.context(QuerySnafu) }.boxed()
        })?
        .get("count_transactions", |req, state| {
            async move {
                let from: Bound<usize> = match req.opt_integer_param("from")? {
                    Some(from) => Bound::Included(from),
                    None => Bound::Unbounded,
                };
                let to = match req.opt_integer_param("to")? {
                    Some(to) => Bound::Included(to),
                    None => Bound::Unbounded,
                };
                Ok(state.count_transactions_in_range((from, to)).await?)
            }
            .boxed()
        })?
        .get("payload_size", |req, state| {
            async move {
                let from: Bound<usize> = match req.opt_integer_param("from")? {
                    Some(from) => Bound::Included(from),
                    None => Bound::Unbounded,
                };
                let to = match req.opt_integer_param("to")? {
                    Some(to) => Bound::Included(to),
                    None => Bound::Unbounded,
                };
                Ok(state.payload_size_in_range((from, to)).await?)
            }
            .boxed()
        })?
        .get("get_vid_share", |req, state| {
            async move {
                let id = if let Some(height) = req.opt_integer_param("height")? {
                    BlockId::Number(height)
                } else if let Some(hash) = req.opt_blob_param("hash")? {
                    BlockId::Hash(hash)
                } else {
                    BlockId::PayloadHash(req.blob_param("payload-hash")?)
                };
                state.vid_share(id).await.context(QueryVidSnafu {
                    block: id.to_string(),
                })
            }
            .boxed()
        })?
        .get("sync_status", |_req, state| {
            async move { state.sync_status().await.context(QuerySnafu) }.boxed()
        })?
        .get("get_header_window", move |req, state| {
            async move {
                let start = if let Some(height) = req.opt_integer_param("height")? {
                    WindowStart::Height(height)
                } else if let Some(hash) = req.opt_blob_param("hash")? {
                    WindowStart::Hash(hash)
                } else {
                    WindowStart::Time(req.integer_param("start")?)
                };
                let end = req.integer_param("end")?;
                state
                    .get_header_window(start, end, window_limit)
                    .await
                    .context(QueryWindowSnafu {
                        start: format!("{start:?}"),
                        end,
                    })
            }
            .boxed()
        })?
        .get("get_limits", move |_req, _state| {
            async move { Ok(Limits { window_limit }) }.boxed()
        })?;
    Ok(api)
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use async_lock::RwLock;
    use committable::Committable;
    use futures::{FutureExt, StreamExt};
    use hotshot_types::{
        data::{VidDisperseShare, VidShare},
        event::{EventType, LeafInfo},
        traits::{
            block_contents::{BlockHeader, BlockPayload},
            EncodeBytes,
        },
    };
    use portpicker::pick_unused_port;
    use surf_disco::Client;
    use tempfile::TempDir;
    use tide_disco::{App, Error as _};
    use tokio::time::sleep;
    use toml::toml;

    use super::*;
    use crate::{
        data_source::ExtensibleDataSource,
        task::BackgroundTask,
        testing::{
            consensus::{MockDataSource, MockNetwork, MockSqlDataSource},
            mocks::{mock_transaction, MockBase, MockTypes, MockVersions},
            setup_test,
        },
        ApiState, Error, Header,
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn test_api() {
        setup_test();

        let window_limit = 78;

        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;
        let mut events = network.handle().event_stream();
        network.start().await;

        // Start the web server.
        let port = pick_unused_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "node",
            define_api(
                &Options {
                    window_limit,
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
            format!("http://localhost:{}/node", port).parse().unwrap(),
        );
        assert!(client.connect(Some(Duration::from_secs(60))).await);

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
            .get::<SyncStatus>("sync-status")
            .send()
            .await
            .unwrap();
        assert_eq!(sync_status.missing_blocks, 0);
        assert_eq!(sync_status.missing_leaves, 0);

        network.shut_down().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_aggregate_ranges() {
        setup_test();

        // Create the consensus network.
        let mut network = MockNetwork::<MockSqlDataSource, MockVersions>::init().await;
        let mut events = network.handle().event_stream();
        network.start().await;

        // Start the web server.
        let port = pick_unused_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "node",
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
        let client =
            Client::<Error, MockBase>::new(format!("http://localhost:{}", port).parse().unwrap());
        assert!(client.connect(Some(Duration::from_secs(60))).await);

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
            .get::<usize>(&format!("node/transactions/count/{}", tx_heights[1]))
            .send()
            .await
        {
            if err.status() == StatusCode::NOT_FOUND {
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
                .get::<usize>("node/transactions/count/0")
                .send()
                .await
                .unwrap()
        );
        assert_eq!(
            0,
            client
                .get::<usize>("node/payloads/size/0")
                .send()
                .await
                .unwrap()
        );

        // First transaction only
        assert_eq!(
            1,
            client
                .get::<usize>(&format!("node/transactions/count/{}", tx_heights[0]))
                .send()
                .await
                .unwrap()
        );
        assert_eq!(
            tx_sizes[0],
            client
                .get::<usize>(&format!("node/payloads/size/{}", tx_heights[0]))
                .send()
                .await
                .unwrap()
        );

        // Last transaction only
        assert_eq!(
            1,
            client
                .get::<usize>(&format!(
                    "node/transactions/count/{}/{}",
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
                    "node/payloads/size/{}/{}",
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
                .get::<usize>("node/transactions/count",)
                .send()
                .await
                .unwrap()
        );
        assert_eq!(
            tx_sizes[0] + tx_sizes[1],
            client
                .get::<usize>("node/payloads/size",)
                .send()
                .await
                .unwrap()
        );

        network.shut_down().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_extensions() {
        setup_test();

        let dir = TempDir::with_prefix("test_node_extensions").unwrap();
        let data_source = ExtensibleDataSource::new(
            MockDataSource::create(dir.path(), Default::default())
                .await
                .unwrap(),
            0,
        );

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
        app.register_module("node", api).unwrap();

        let port = pick_unused_port().unwrap();
        let _server = BackgroundTask::spawn(
            "server",
            app.serve(format!("0.0.0.0:{}", port), MockBase::instance()),
        );

        let client = Client::<Error, MockBase>::new(
            format!("http://localhost:{}/node", port).parse().unwrap(),
        );
        assert!(client.connect(Some(Duration::from_secs(60))).await);

        assert_eq!(client.get::<u64>("ext").send().await.unwrap(), 0);
        client.post::<()>("ext/42").send().await.unwrap();
        assert_eq!(client.get::<u64>("ext").send().await.unwrap(), 42);

        // Ensure we can still access the built-in functionality.
        let sync_status: SyncStatus = client.get("sync-status").send().await.unwrap();
        assert!(sync_status.is_fully_synced(), "{sync_status:?}");
    }
}
