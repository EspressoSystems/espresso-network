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
//!    a. Node Identity Information
//!    b. Node State Information (specifically voter participation, latest block
//!       information, and staking information)
//! 2. Network Information
//!    a. Latest Block
//!    b. The most recent N blocks (N assumed to be 50 at the moment)
//!        - Information can be derived from these most recent 50 blocks
//!          that allows us to derive histogram data, producer data, and
//!          the most recent block information.  We might be able to get away
//!          with just storing the header information of these blocks, since we
//!          don't need the full block data.
//!    c. The most recent N votes participants
//!    d. The top block producers over the latest N blocks
//!    e. Histogram data for the latest N blocks
//!        - Block Size
//!        - Block Time
//!        - Block Space Used
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
//!     a. Node Identity Information
//!         - Should be able to be sent in an initial batch
//!         - Should be able to send individual updates as they occur
//!     b. Node State Information
//!         - Should be able to be sent in an initial batch
//!         - Should be able to send individual updates as they occur
//!     c. Block Information
//!         - Should be able to be sent in an initial batch
//!         - Should be able to send individual updates as they occur

pub mod api;
pub mod service;

use clap::Parser;
use espresso_types::{PubKey, SeqTypes};
use futures::channel::mpsc::{self, Sender};
use hotshot::traits::implementations::{
    CdnMetricsValue, CdnTopic, PushCdnNetwork, WrappedSignatureKey,
};
use hotshot_query_service::metrics::PrometheusMetrics;
use hotshot_types::traits::{node_implementation::NodeType, signature_key::BuilderSignatureKey};
use tide_disco::App;
use tokio::spawn;
use url::Url;

use crate::{
    api::node_validator::v0::{
        cdn::{BroadcastRollCallTask, CdnReceiveMessagesTask},
        create_node_validator_api::{create_node_validator_processing, NodeValidatorConfig},
        HotshotQueryServiceLeafStreamRetriever, ProcessProduceLeafStreamTask,
        StateClientMessageSender, STATIC_VER_0_1,
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
    #[arg(long, env = "ESPRESSO_NODE_VALIDATOR_STAKE_TABLE_SOURCE_BASE_URL")]
    stake_table_source_base_url: Url,

    /// leaf_stream_base_url is the base URL for the availability API endpoint
    /// that is capable of providing a stream of leaf data.
    ///
    /// This endpoint is expected to point to the version root path of the
    /// URL.
    /// Example:
    ///   - https://query.cappuccino.testnet.espresso.network/v0/
    #[arg(long, env = "ESPRESSO_NODE_VALIDATOR_LEAF_STREAM_SOURCE_BASE_URL")]
    leaf_stream_base_url: Url,

    /// initial_node_public_base_urls is a list of URLs that are the initial
    /// public base URLs of the nodes that are in the network. These can be
    /// supplied as an initial source of URLS to scrape for node identity.
    ///
    /// These urls are expected to point to the root path of the URL for the
    /// node, and are expected to be URLS that support the status endpoint
    /// for the nodes.
    ///
    /// Example URL:
    ///  - https://query-1.cappuccino.testnet.espresso.network/
    #[arg(
        long,
        env = "ESPRESSO_NODE_VALIDATOR_INITIAL_NODE_PUBLIC_BASE_URLS",
        value_delimiter = ','
    )]
    initial_node_public_base_urls: Vec<Url>,

    /// port is the port that the node validator service will listen on.
    /// This port is expected to be a valid port number that is available
    /// for the service to bind to.
    #[arg(
        long,
        env = "ESPRESSO_NODE_VALIDATOR_PORT",
        default_value = "9000"
    )]
    port: u16,

    /// cdn_marshal_endpoint is the endpoint for the CDN marshal service.
    ///
    /// This endpoint is optional, and if it is not provided, then the CDN
    /// service will not be utilized.
    #[arg(long, env = "ESPRESSO_NODE_VALIDATOR_CDN_MARSHAL_ENDPOINT")]
    cdn_marshal_endpoint: Option<String>,
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

    fn cdn_marshal_endpoint(&self) -> &Option<String> {
        &self.cdn_marshal_endpoint
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

    let (leaf_sender, leaf_receiver) = mpsc::channel(10);

    let _process_consume_leaves = ProcessProduceLeafStreamTask::new(
        HotshotQueryServiceLeafStreamRetriever::new(options.leaf_stream_base_url().clone()),
        leaf_sender,
    );

    let node_validator_task_state = match create_node_validator_processing(
        NodeValidatorConfig {
            stake_table_url_base: options.stake_table_source_base_url().clone(),
            initial_node_public_base_urls: options.initial_node_public_base_urls().to_vec(),
        },
        internal_client_message_receiver,
        leaf_receiver,
    )
    .await
    {
        Ok(node_validator_task_state) => node_validator_task_state,

        Err(err) => {
            panic!("error defining node validator api: {:?}", err);
        },
    };

    let _cdn_tasks = if let Some(cdn_broker_url_string) = options.cdn_marshal_endpoint() {
        let (public_key, private_key) = PubKey::generated_from_seed_indexed([1; 32], 0);
        let cdn_network_result = PushCdnNetwork::<<SeqTypes as NodeType>::SignatureKey>::new(
            cdn_broker_url_string.to_string(),
            vec![CdnTopic::Global],
            hotshot::traits::implementations::KeyPair {
                public_key: WrappedSignatureKey(public_key),
                private_key: private_key.clone(),
            },
            CdnMetricsValue::new(&PrometheusMetrics::default()),
        );
        let cdn_network = match cdn_network_result {
            Ok(cdn_network) => cdn_network,
            Err(err) => {
                panic!("error creating cdn network: {:?}", err);
            },
        };

        let url_sender = node_validator_task_state.url_sender.clone();

        let broadcast_cdn_network = cdn_network.clone();
        let cdn_receive_message_task = CdnReceiveMessagesTask::new(cdn_network, url_sender);
        let broadcast_roll_call_task =
            BroadcastRollCallTask::new(broadcast_cdn_network, public_key);

        Some((broadcast_roll_call_task, cdn_receive_message_task))
    } else {
        None
    };

    let port = options.port();
    // We would like to wait until being signaled
    let app_serve_handle = spawn(async move {
        let app_serve_result = app.serve(format!("0.0.0.0:{}", port), STATIC_VER_0_1).await;
        tracing::info!("app serve result: {:?}", app_serve_result);
    });

    let _ = app_serve_handle.await;
}
