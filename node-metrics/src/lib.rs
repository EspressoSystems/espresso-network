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

//! # Node Validator Service
//!
//! The Node Validator Service is a general purpose relay service that watches
//! data flow from the Hot Shot protocol via the CDN pub sub service. It
//! maintains a local state of the network map and is able to relay the
//! stored details to any client that requests it. In addition it is also
//! able to provide individual state change updates to any client that
//! subscribes to that particular event stream.  In order to be able to
//! provide identity information to the clients, this identity information
//! must be volunteered by the nodes in the network.  This requires the
//! nodes to be able to receive and respond to these requests, and relay
//! to anyone who desires it, the identity information of the node.
//!
//! ## Storage
//!
//! In order for this service to be effective and efficient it needs to be
//! able to store the state of the network in an efficient manner.  The
//! storage should be fast and efficient.  We are not expecting a lot of
//! data to be stored within this storage, but as things tend to grow and
//! change it may be necessary to have more robust storage mechanisms in
//! place, or even to have the ability to introduce new storage mechanisms.
//! In order to effectively store the data that we need to store, we need
//! to ask a fundamental question:
//!
//! What states do we need to track?
//! 1. Node Information
//!    * Node Identity Information
//!    * Node State Information (specifically voter participation, latest block
//!      information, and staking information)
//! 2. Network Information
//!    * Latest Block
//!    * The most recent N blocks (N assumed to be 50 at the moment)
//!      - Information can be derived from these most recent 50 blocks
//!        that allows us to derive histogram data, producer data, and
//!        the most recent block information.  We might be able to get away
//!        with just storing the header information of these blocks, since we
//!        don't need the full block data.
//!    * The most recent N votes participants
//!    * The top block producers over the latest N blocks
//!    * Histogram data for the latest N blocks
//!      - Block Size
//!      - Block Time
//!      - Block Space Used
//!
//! ## Data Streams
//!
//! In order for clients to be able to receive the information from the node
//! validator service, we need to be able to facilitate requests.  We could
//! simply just start streaming data to the clients as soon as they connect,
//! however, this causes potential compatibility issues with the clients
//! in question.  For example, if we want to add a new data stream that
//! can be retrieved for the client, and the client isn't expecting it, they
//! won't know how to handle the data, and it can potentially cause errors.
//! As such, it makes sense to only provide data streams when the client asks
//! for them.  This allows for new features to be added to the data stream
//! without breaking compatibility with the clients, provided that the existing
//! streams don't change in a way that would break the client.
//!
//! Starting out, there doesn't need to be a lot of data that needs to be
//! streamed to to the client.  In fact, we might be able to be a little
//! naive about this, and broadcast general objects in an event stream, as
//! data may be derivable from the objects that are broadcast.  For example,
//! if we start out by sending the latest N block information, the client
//! may be able to derive histogram data from that information, which would
//! prevent us from having to send and store the histogram data.  However,
//! there may be some pieces of data that are lacking from this approach which
//! would require us to send out additional data streams.
//!
//! Ideally, we should strive for a balance between the data we store locally
//! and the data that we stream to the clients. In order to know what we
//! need to store, we need to know what data we are expecting the client to
//! consume, and which data can be derived for these purposes.
//!
//! What Data Streams do we need to provide to clients?
//!
//! 1. Node Information
//!    * Node Identity Information
//!      - Should be able to be sent in an initial batch
//!      - Should be able to send individual updates as they occur
//!    * Node State Information
//!      - Should be able to be sent in an initial batch
//!      - Should be able to send individual updates as they occur
//!    * Block Information
//!      - Should be able to be sent in an initial batch
//!      - Should be able to send individual updates as they occur

pub mod api;
pub mod service;

use api::node_validator::v0::SurfDiscoAvailabilityAPIStream;
use clap::Parser;
use futures::{
    channel::mpsc::{self, Sender},
    StreamExt,
};
use service::data_state::MAX_VOTERS_HISTORY;
use tide_disco::App;
use tokio::spawn;
use url::Url;

use crate::{
    api::node_validator::v0::{
        create_node_validator_api::{create_node_validator_processing, NodeValidatorConfig},
        BridgeLeafAndBlockStreamToSenderTask, StateClientMessageSender, STATIC_VER_0_1,
    },
    service::{client_message::InternalClientMessage, server_message::ServerMessage},
};

/// Options represents the configuration options that are available for running
/// the node validator service via the [run_standalone_service] function.
/// These options are configurable via command line arguments or environment
/// variables.
#[derive(Parser, Clone, Debug)]
pub struct Options {
    /// stake_table_source_based_url is the base URL for the config API
    /// endpoint that is provided by Espresso Sequencers.
    ///
    /// This endpoint is expected to point to the version root path of the
    /// URL.
    /// Example:
    ///   - https://query.cappuccino.testnet.espresso.network/v0/
    #[clap(long, env = "ESPRESSO_NODE_VALIDATOR_STAKE_TABLE_SOURCE_BASE_URL")]
    stake_table_source_base_url: Url,

    /// leaf_stream_base_url is the base URL for the availability API endpoint
    /// that is capable of providing a stream of leaf data.
    ///
    /// This endpoint is expected to point to the version root path of the
    /// URL.
    /// Example:
    ///   - https://query.cappuccino.testnet.espresso.network/v0/
    ///
    #[clap(long, env = "ESPRESSO_NODE_VALIDATOR_LEAF_STREAM_SOURCE_BASE_URL")]
    leaf_stream_base_url: Url,

    /// initial_node_public_base_urls is a list of URLs that are the initial
    /// public base URLs of the nodes that are in the network.  These can be
    /// supplied as an initial source of URLS to scrape for node identity.
    ///
    /// These urls are expected to point to the root path of the URL for the
    /// node, and are expected to be URLS that support the status endpoint
    /// for the nodes.
    ///
    /// Example URL:
    ///  - https://query-1.cappuccino.testnet.espresso.network/
    #[clap(
        long,
        env = "ESPRESSO_NODE_VALIDATOR_INITIAL_NODE_PUBLIC_BASE_URLS",
        value_delimiter = ','
    )]
    initial_node_public_base_urls: Vec<Url>,

    /// port is the port that the node validator service will listen on.
    /// This port is expected to be a valid port number that is available
    /// for the service to bind to.
    #[clap(
        long,
        value_parser,
        env = "ESPRESSO_NODE_VALIDATOR_PORT",
        default_value = "9000"
    )]
    port: u16,
}

impl Options {
    fn stake_table_source_base_url(&self) -> &Url {
        &self.stake_table_source_base_url
    }

    fn leaf_stream_base_url(&self) -> &Url {
        &self.leaf_stream_base_url
    }

    fn initial_node_public_base_urls(&self) -> &[Url] {
        &self.initial_node_public_base_urls
    }

    fn port(&self) -> u16 {
        self.port
    }
}

/// MainState represents the State of the application this is available to
/// tide_disco.
struct MainState {
    internal_client_message_sender: Sender<InternalClientMessage<Sender<ServerMessage>>>,
}

impl StateClientMessageSender<Sender<ServerMessage>> for MainState {
    fn sender(&self) -> Sender<InternalClientMessage<Sender<ServerMessage>>> {
        self.internal_client_message_sender.clone()
    }
}

/// Run the service by itself.
///
/// This function will run the node validator as its own service.  It has some
/// options that allow it to be configured in order for it to operate
/// effectively.
pub async fn run_standalone_service(options: Options) {
    let (internal_client_message_sender, internal_client_message_receiver) = mpsc::channel(32);
    let state = MainState {
        internal_client_message_sender,
    };

    let mut app: App<_, api::node_validator::v0::Error> = App::with_state(state);
    let node_validator_api =
        api::node_validator::v0::define_api().expect("error defining node validator api");

    match app.register_module("node-validator", node_validator_api) {
        Ok(_) => {},
        Err(err) => {
            panic!("error registering node validator api: {:?}", err);
        },
    }

    let (leaf_and_block_pair_sender, leaf_and_block_pair_receiver) = mpsc::channel(10);

    let client = surf_disco::Client::new(options.leaf_stream_base_url().clone());

    // Let's get the current starting block height.
    let block_height = {
        // Retry up to 4 times to get the block height
        let mut i = 0;
        let block_height: Option<u64> = loop {
            let block_height_result = client.get("status/block-height").send().await;
            match block_height_result {
                Ok(block_height) => break Some(block_height),
                Err(err) => {
                    tracing::warn!("retrieve block height request failed: {}", err);
                },
            };

            // Sleep so we're not spamming too much with back to back requests.
            // The sleep time delay will be 10ms, then 100ms, then 1s, then 10s.
            tokio::time::sleep(std::time::Duration::from_millis(10u64.pow(i + 1))).await;
            i += 1;

            if i >= 4 {
                break None;
            }
        };

        if let Some(block_height) = block_height {
            // We want to make sure that we have at least MAX_VOTERS_HISTORY blocks of
            // history that we are pulling
            block_height.saturating_sub(MAX_VOTERS_HISTORY as u64 + 1)
        } else {
            panic!("unable to retrieve block height");
        }
    };

    tracing::debug!("creating stream starting at block height: {}", block_height);

    let leaf_stream = SurfDiscoAvailabilityAPIStream::new_leaf_stream(client.clone(), block_height);
    let block_stream = SurfDiscoAvailabilityAPIStream::new_block_stream(client, block_height);

    let zipped_stream = leaf_stream.zip(block_stream);

    let _process_consume_leaves =
        BridgeLeafAndBlockStreamToSenderTask::new(zipped_stream, leaf_and_block_pair_sender);

    let _node_validator_task_state = match create_node_validator_processing(
        NodeValidatorConfig {
            stake_table_url_base: options.stake_table_source_base_url().clone(),
            initial_node_public_base_urls: options.initial_node_public_base_urls().to_vec(),
            starting_block_height: block_height,
        },
        internal_client_message_receiver,
        leaf_and_block_pair_receiver,
    )
    .await
    {
        Ok(node_validator_task_state) => node_validator_task_state,

        Err(err) => {
            panic!("error defining node validator api: {:?}", err);
        },
    };

    let port = options.port();
    // We would like to wait until being signaled
    let app_serve_handle = spawn(async move {
        let app_serve_result = app.serve(format!("0.0.0.0:{}", port), STATIC_VER_0_1).await;
        tracing::info!("app serve result: {:?}", app_serve_result);
    });

    let _ = app_serve_handle.await;
}
