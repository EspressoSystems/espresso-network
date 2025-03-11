pub mod cdn;
pub mod create_node_validator_api;

use std::{fmt, future::Future, io::BufRead, pin::Pin, str::FromStr, time::Duration};

use espresso_types::{BackoffParams, SeqTypes};
use futures::{
    channel::mpsc::{self, SendError, Sender},
    future::Either,
    FutureExt, Sink, SinkExt, Stream, StreamExt,
};
use hotshot_query_service::Leaf2;
use hotshot_stake_table::vec_based::StakeTable;
use hotshot_types::{
    light_client::{CircuitField, StateVerKey},
    signature_key::BLSPubKey,
    traits::{signature_key::StakeTableEntryType, stake_table::StakeTableScheme},
    PeerConfig,
};
use prometheus_parse::{Sample, Scrape};
use serde::{Deserialize, Serialize};
use tide_disco::{api::ApiError, socket::Connection, Api};
use tokio::{spawn, task::JoinHandle, time::sleep};
use url::Url;
use vbs::version::{StaticVersion, StaticVersionType, Version};

use crate::service::{
    client_message::{ClientMessage, InternalClientMessage},
    data_state::{LocationDetails, NodeIdentity},
    server_message::ServerMessage,
};

/// CONSTANT for protocol major version
pub const VERSION_MAJ: u16 = 0;

/// CONSTANT for protocol minor version
pub const VERSION_MIN: u16 = 1;

pub const VERSION_0_1: Version = Version {
    major: VERSION_MAJ,
    minor: VERSION_MIN,
};

/// Constant for the version of this API.
pub const BASE_VERSION: Version = VERSION_0_1;

/// Specific type for version 0.1
pub type Version01 = StaticVersion<VERSION_MAJ, VERSION_MIN>;

// Static instance of the Version01 type
pub const STATIC_VER_0_1: Version01 = StaticVersion {};

#[derive(Debug, Serialize, Deserialize)]
pub enum Error {
    UnhandledTideDisco(tide_disco::StatusCode, String),
    UnhandledSurfDisco(surf_disco::StatusCode, String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::UnhandledSurfDisco(status, msg) => {
                write!(f, "Unhandled Surf Disco Error: {} - {}", status, msg)
            },

            Self::UnhandledTideDisco(status, msg) => {
                write!(f, "Unhandled Tide Disco Error: {} - {}", status, msg)
            },
        }
    }
}

impl std::error::Error for Error {}

impl tide_disco::Error for Error {
    fn catch_all(status: tide_disco::StatusCode, msg: String) -> Self {
        Self::UnhandledTideDisco(status, msg)
    }

    fn status(&self) -> tide_disco::StatusCode {
        tide_disco::StatusCode::INTERNAL_SERVER_ERROR
    }
}

#[derive(Debug)]
pub enum LoadApiError {
    Toml(toml::de::Error),
    Api(ApiError),
}

impl From<toml::de::Error> for LoadApiError {
    fn from(err: toml::de::Error) -> Self {
        LoadApiError::Toml(err)
    }
}

impl From<ApiError> for LoadApiError {
    fn from(err: ApiError) -> Self {
        LoadApiError::Api(err)
    }
}

pub(crate) fn load_api<State: 'static, ApiVer: StaticVersionType + 'static>(
    default: &str,
) -> Result<Api<State, Error, ApiVer>, LoadApiError> {
    let toml: toml::Value = toml::from_str(default)?;
    Ok(Api::new(toml)?)
}

#[derive(Debug)]
pub enum LoadTomlError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    Utf8(std::str::Utf8Error),
}

impl From<std::io::Error> for LoadTomlError {
    fn from(err: std::io::Error) -> Self {
        LoadTomlError::Io(err)
    }
}

impl From<toml::de::Error> for LoadTomlError {
    fn from(err: toml::de::Error) -> Self {
        LoadTomlError::Toml(err)
    }
}

impl From<std::str::Utf8Error> for LoadTomlError {
    fn from(err: std::str::Utf8Error) -> Self {
        LoadTomlError::Utf8(err)
    }
}

#[derive(Debug)]
pub enum DefineApiError {
    LoadApiError(LoadApiError),
    LoadTomlError(LoadTomlError),
    ApiError(ApiError),
}

impl From<LoadApiError> for DefineApiError {
    fn from(err: LoadApiError) -> Self {
        DefineApiError::LoadApiError(err)
    }
}

impl From<LoadTomlError> for DefineApiError {
    fn from(err: LoadTomlError) -> Self {
        DefineApiError::LoadTomlError(err)
    }
}

impl From<ApiError> for DefineApiError {
    fn from(err: ApiError) -> Self {
        DefineApiError::ApiError(err)
    }
}

/// [StateClientMessageSender] allows for the retrieval of a [Sender] for sending
/// messages received from the client to the Server for request processing.
pub trait StateClientMessageSender<K> {
    fn sender(&self) -> Sender<InternalClientMessage<K>>;
}

#[derive(Debug)]
pub enum EndpointError {}

pub fn define_api<State>() -> Result<Api<State, Error, Version01>, DefineApiError>
where
    State: StateClientMessageSender<Sender<ServerMessage>> + Send + Sync + 'static,
{
    let mut api = load_api::<State, Version01>(include_str!("./node_validator.toml"))?;

    api.with_version("0.0.1".parse().unwrap()).socket(
        "details",
        move |_req, socket: Connection<ServerMessage, ClientMessage, Error, Version01>, state| {
            async move {
                let mut socket_stream = socket.clone();
                let mut socket_sink = socket;

                let mut internal_client_message_sender = state.sender();
                let (server_message_sender, mut server_message_receiver) = mpsc::channel(32);

                // Let's register ourselves with the Server
                if let Err(err) = internal_client_message_sender
                    .send(InternalClientMessage::Connected(server_message_sender))
                    .await
                {
                    // This means that the client_message_sender is closed
                    // we need to exit the stream.
                    tracing::info!(
                        "client message sender is closed before first message: {}",
                        err
                    );
                    return Ok(());
                }

                // We should receive a response from the server that identifies us
                // uniquely.
                let client_id = if let Some(ServerMessage::YouAre(client_id)) =
                    server_message_receiver.next().await
                {
                    client_id
                } else {
                    // The channel is closed, and this client should be removed
                    // we need to exit the stream
                    tracing::info!("server message receiver closed before first message",);
                    return Ok(());
                };

                // We want to start these futures outside of the loop.  If we
                // don't do this then every iteration will not be guaranteed
                // to not skip a message.
                let mut next_client_message = socket_stream.next();
                let mut next_server_message = server_message_receiver.next();

                loop {
                    match futures::future::select(next_client_message, next_server_message).await {
                        Either::Left((client_request, remaining_server_message)) => {
                            let client_request = if let Some(client_request) = client_request {
                                client_request
                            } else {
                                // The client has disconnected, we need to exit the stream
                                tracing::info!("client message has disconnected");
                                break;
                            };

                            let client_request = if let Ok(client_request) = client_request {
                                client_request
                            } else {
                                // This indicates that there was a more
                                // specific error with the socket message.
                                // This error can be various, and may be
                                // recoverable depending on the actual nature
                                // of the error.  We will treat it as
                                // unrecoverable for now.
                                break;
                            };

                            let internal_client_message =
                                client_request.to_internal_with_client_id(client_id);
                            if let Err(err) = internal_client_message_sender
                                .send(internal_client_message)
                                .await
                            {
                                // This means that the client_message_sender is closed
                                tracing::info!("client message sender is closed: {}", err);
                                break;
                            }

                            // let's queue up the next client message to receive
                            next_client_message = socket_stream.next();
                            next_server_message = remaining_server_message;
                        },
                        Either::Right((server_message, remaining_client_message)) => {
                            // Alright, we have a server message, we want to forward it
                            // to the down-stream client.

                            let server_message = if let Some(server_message) = server_message {
                                server_message
                            } else {
                                // The server has disconnected, we need to exit the stream
                                break;
                            };

                            // We want to forward the message to the client
                            if let Err(err) = socket_sink.send(&server_message).await {
                                // This means that the socket is closed
                                tracing::info!("socket is closed: {}", err);
                                break;
                            }

                            // let's queue up the next server message to receive
                            next_server_message = server_message_receiver.next();
                            next_client_message = remaining_client_message;
                        },
                    }
                }

                // We don't actually care if this fails or not, as we're exiting
                // this function anyway, and these Senders and Receivers will
                // automatically be dropped.
                _ = internal_client_message_sender
                    .send(InternalClientMessage::Disconnected(client_id))
                    .await;

                Ok(())
            }
            .boxed()
        },
    )?;
    Ok(api)
}

#[derive(Debug, Deserialize)]
pub struct PublishHotShotConfig {
    pub known_nodes_with_stake: Vec<PeerConfig<SeqTypes>>,
}

#[derive(Debug, Deserialize)]
pub struct SequencerConfig {
    pub config: PublishHotShotConfig,
}

/// [get_stake_table_from_sequencer] retrieves the stake table from the
/// Sequencer.  It expects a [surf_disco::Client] to be provided so that it can
/// make the request to the Hotshot Query Service.  It will return a
/// [StakeTable] that is populated with the data retrieved from the Hotshot
/// Query Service.
pub async fn get_stake_table_from_sequencer(
    client: surf_disco::Client<hotshot_query_service::Error, Version01>,
) -> Result<StakeTable<BLSPubKey, StateVerKey, CircuitField>, hotshot_query_service::Error> {
    let request = client
        .get("config/hotshot")
        // We need to set the Accept header, otherwise the Content-Type
        // will be application/octet-stream, and we won't be able to
        // deserialize the response.
        .header("Accept", "application/json");
    let stake_table_result = request.send().await;

    let sequencer_config: SequencerConfig = match stake_table_result {
        Ok(public_hot_shot_config) => public_hot_shot_config,
        Err(err) => {
            tracing::info!("retrieve stake table request failed: {}", err);
            return Err(err);
        },
    };

    let public_hot_shot_config = sequencer_config.config;

    let mut stake_table = StakeTable::<BLSPubKey, StateVerKey, CircuitField>::new(
        public_hot_shot_config.known_nodes_with_stake.len(),
    );

    for node in public_hot_shot_config.known_nodes_with_stake.into_iter() {
        stake_table
            .register(
                *node.stake_table_entry.key(),
                node.stake_table_entry.stake(),
                node.state_ver_key,
            )
            .expect("registering stake table entry");
    }

    stake_table.advance();
    stake_table.advance();

    Ok(stake_table)
}

pub enum GetNodeIdentityFromUrlError {
    Url(url::ParseError),
    Reqwest(reqwest::Error),
    Io(std::io::Error),
    NoNodeIdentity,
}

impl std::fmt::Display for GetNodeIdentityFromUrlError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GetNodeIdentityFromUrlError::Url(err) => write!(f, "url: {}", err),
            GetNodeIdentityFromUrlError::Reqwest(err) => write!(f, "reqwest error: {}", err),
            GetNodeIdentityFromUrlError::Io(err) => write!(f, "io error: {}", err),
            GetNodeIdentityFromUrlError::NoNodeIdentity => write!(f, "no node identity"),
        }
    }
}

impl From<url::ParseError> for GetNodeIdentityFromUrlError {
    fn from(err: url::ParseError) -> Self {
        GetNodeIdentityFromUrlError::Url(err)
    }
}

impl From<reqwest::Error> for GetNodeIdentityFromUrlError {
    fn from(err: reqwest::Error) -> Self {
        GetNodeIdentityFromUrlError::Reqwest(err)
    }
}

impl From<std::io::Error> for GetNodeIdentityFromUrlError {
    fn from(err: std::io::Error) -> Self {
        GetNodeIdentityFromUrlError::Io(err)
    }
}

/// [get_node_identity_from_url] retrieves a [NodeIdentity] from a URL.  It
/// expects a [url::Url] to be provided so that it can make the request to the
/// Sequencer status metrics API. It will return a [NodeIdentity] that is
/// populated with the data retrieved from the Sequencer status metrics API.
/// If no [NodeIdentity] is found, it will return a
/// [GetNodeIdentityFromUrlError::NoNodeIdentity] error.
pub async fn get_node_identity_from_url(
    url: url::Url,
) -> Result<NodeIdentity, GetNodeIdentityFromUrlError> {
    let client = reqwest::Client::new();

    let completed_url = url.join("v0/status/metrics")?;
    let request = client.get(completed_url).build()?;
    let response = client.execute(request).await?;
    let response_bytes = response.bytes().await?;

    let buffered_response = std::io::BufReader::new(&*response_bytes);
    let scrape = prometheus_parse::Scrape::parse(buffered_response.lines())?;

    if let Some(node_identity) = node_identity_from_scrape(scrape) {
        let mut node_identity = node_identity;
        node_identity.public_url = Some(url);
        Ok(node_identity)
    } else {
        Err(GetNodeIdentityFromUrlError::NoNodeIdentity)
    }
}

/// LeafStreamRetriever is a general trait that allows for the retrieval of a
/// list of Leaves from a source. The specific implementation doesn't care about
/// the source, only that it is able to retrieve a stream of Leaves.
///
/// This allows us to swap the implementation of the [LeafStreamRetriever] for
/// testing purposes, or for newer sources in the future.
pub trait LeafStreamRetriever: Send {
    type Item;
    type ItemError: std::error::Error + Send;
    type Error: std::error::Error + Send;
    type Stream: Stream<Item = Result<Self::Item, Self::ItemError>> + Send + Unpin;
    type Future: Future<Output = Result<Self::Stream, Self::Error>> + Send;

    /// [retrieve_stream] retrieves a stream of [Leaf]s from the source.  It
    /// expects the current block height to be provided so that it can determine
    /// the starting block height to retrieve the stream of [Leaf]s from.
    ///
    /// It should check the current height of the chain so that it only needs
    /// to retrieve the number of older blocks that are needed, instead of
    /// starting from the beginning of time.
    fn retrieve_stream(&self, current_block_height: Option<u64>) -> Self::Future;
}

/// [HotshotQueryServiceLeafStreamRetriever] is a [LeafStreamRetriever] that
/// retrieves a stream of [Leaf]s from the Hotshot Query Service.  It expects
/// the base URL of the Hotshot Query Service to be provided so that it can
/// make the request to the Hotshot Query Service.
pub struct HotshotQueryServiceLeafStreamRetriever {
    base_url: Url,
}

impl HotshotQueryServiceLeafStreamRetriever {
    /// [new] creates a new [HotshotQueryServiceLeafStreamRetriever] that
    /// will use the given base [Url] to be able to retrieve the stream of
    /// [Leaf]s from the Hotshot Query Service.
    ///
    /// The [Url] is expected to point to the API version root of the
    /// Hotshot Query Service.  Example:
    ///   https://example.com/v0
    pub fn new(base_url: Url) -> Self {
        Self { base_url }
    }
}

impl LeafStreamRetriever for HotshotQueryServiceLeafStreamRetriever {
    type Item = Leaf2<SeqTypes>;
    type ItemError = hotshot_query_service::Error;
    type Error = hotshot_query_service::Error;
    type Stream = surf_disco::socket::Connection<
        Leaf2<SeqTypes>,
        surf_disco::socket::Unsupported,
        Self::ItemError,
        Version01,
    >;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Stream, Self::Error>> + Send>>;

    fn retrieve_stream(&self, current_block_height: Option<u64>) -> Self::Future {
        let client = surf_disco::Client::new(self.base_url.clone());
        async move {
            let block_height_result = client.get("status/block-height").send().await;
            let block_height: u64 = match block_height_result {
                Ok(block_height) => block_height,
                Err(err) => {
                    tracing::info!("retrieve block height request failed: {}", err);
                    return Err(err);
                },
            };

            let latest_block_start = block_height.saturating_sub(50);
            let start_block_height = if let Some(known_height) = current_block_height {
                std::cmp::min(known_height, latest_block_start)
            } else {
                latest_block_start
            };

            let leaves_stream_result = client
                .socket(&format!(
                    "availability/stream/leaves/{}",
                    start_block_height
                ))
                .subscribe::<espresso_types::Leaf2>()
                .await;

            let leaves_stream = match leaves_stream_result {
                Ok(leaves_stream) => leaves_stream,
                Err(err) => {
                    tracing::info!("retrieve leaves stream failed: {}", err);
                    return Err(err);
                },
            };

            Ok(leaves_stream)
        }
        .boxed()
    }
}

/// [RetrieveLeafStreamError] indicates the various failure conditions that can
/// occur when attempting to retrieve a stream of [Leaf]s using the
/// [ProcessProduceLeafStreamTask::retrieve_leaf_stream] function.
enum RetrieveLeafStreamError {
    /// [MaxAttemptsExceeded] indicates that the maximum number of attempts to
    /// attempt to retrieve the [Stream] of [Leaf]s has been exceeded.
    /// In this case, it doesn't make sense to continue to re-attempt to
    /// reconnect to the service, as it does not seem to be available.
    MaxAttemptsExceeded,
}

/// [ProcessProduceLeafStreamTask] is a task that produce a stream of [Leaf]s
/// from the Hotshot Query Service.  It will attempt to retrieve the [Leaf]s
/// from the Hotshot Query Service and then send them to the [Sink] provided.
pub struct ProcessProduceLeafStreamTask {
    pub task_handle: Option<JoinHandle<()>>,
}

impl ProcessProduceLeafStreamTask {
    /// [new] creates a new [ProcessConsumeLeafStreamTask] that produces a
    /// stream of [Leaf]s from the Hotshot Query Service.
    ///
    /// Calling this function will create an async task that will start
    /// processing immediately.  The task's handle will be stored in the
    /// returned state.
    pub fn new<R, K>(leaf_stream_retriever: R, leaf_sender: K) -> Self
    where
        R: LeafStreamRetriever<Item = Leaf2<SeqTypes>> + Send + Sync + 'static,
        K: Sink<Leaf2<SeqTypes>, Error = SendError> + Clone + Send + Sync + Unpin + 'static,
    {
        // let future = Self::process_consume_leaf_stream(leaf_stream_retriever, leaf_sender);
        let task_handle = spawn(Self::connect_and_process_leaves(
            leaf_stream_retriever,
            leaf_sender,
        ));

        Self {
            task_handle: Some(task_handle),
        }
    }

    async fn connect_and_process_leaves<R, K>(leaf_stream_retriever: R, leaf_sender: K)
    where
        R: LeafStreamRetriever<Item = Leaf2<SeqTypes>>,
        K: Sink<Leaf2<SeqTypes>, Error = SendError> + Clone + Send + Sync + Unpin + 'static,
    {
        // We want to try and ensure that we are connected to the HotShot Query
        // Service, and are consuming leaves.
        // - If we are able to connect, then we can start relaying Leaves into
        //   the leaf sender.
        // - If we are not able to connect, then we should sleep, and retry
        //   until we are able to reconnect.  Each failure **should** make some
        //   noise so that if we are never able to reconnect, we are at least
        //   telling someone about it.
        // - If the task for consuming the leaves completes, then we should
        //   also attempt to reestablish the connection to start consuming
        //   the leave again.

        loop {
            // Retrieve a stream
            let Ok(stream) = Self::retrieve_leaf_stream(&leaf_stream_retriever).await else {
                panic!("failed to retrieve leaf stream");
            };

            // Consume the leaves of a stream
            Self::process_consume_leaf_stream::<R, K>(stream, leaf_sender.clone()).await;
            tracing::warn!("leaf stream ended, will attempt to re-acquire leaf stream");
        }
    }

    /// [retrieve_leaf_stream] attempts to retrieve the Stream of Leaves from
    /// the given [LeafStreamRetriever].
    ///
    /// This function will loop on failure until it is able to retrieve the
    /// [Stream].  This does mean that it could potentially get in a state
    /// where it can loop indefinitely.
    ///
    /// This function also implements exponential backoff with a maximum
    /// delay of 5 seconds.
    async fn retrieve_leaf_stream<R>(
        leaf_stream_receiver: &R,
    ) -> Result<R::Stream, RetrieveLeafStreamError>
    where
        R: LeafStreamRetriever<Item = Leaf2<SeqTypes>>,
    {
        let backoff_params = BackoffParams::default();
        let mut delay = Duration::ZERO;

        for attempt in 1..=100 {
            let leaves_stream_result = leaf_stream_receiver.retrieve_stream(None).await;

            let leaves_stream = match leaves_stream_result {
                Err(error) => {
                    // We failed to retrieve the stream. We will try again, but we
                    // should sleep for a bit before so as not to overwhelm the
                    // service.
                    tracing::warn!(
                        "attempt {attempt} to connect to leaf stream failed with error {error}"
                    );

                    // Our retry penalty will be a minimum of 100ms, and a maximum
                    // of 5 seconds.
                    // For every failed iteration, we will double our delay, up
                    // to the maximum of 5 seconds.

                    delay = backoff_params.backoff(delay);
                    sleep(delay).await;
                    continue;
                },

                Ok(leaves_stream) => leaves_stream,
            };

            return Ok(leaves_stream);
        }

        Err(RetrieveLeafStreamError::MaxAttemptsExceeded)
    }

    /// [process_consume_leaf_stream] produces a stream of [Leaf]s from the
    /// Hotshot Query Service.  It will attempt to retrieve the [Leaf]s from the
    /// Hotshot Query Service and then send them to the [Sink] provided.  If the
    /// [Sink] is closed, or if the Stream ends prematurely, then the function
    /// will return.
    async fn process_consume_leaf_stream<R, K>(leaves_stream: R::Stream, leaf_sender: K)
    where
        R: LeafStreamRetriever<Item = Leaf2<SeqTypes>>,
        K: Sink<Leaf2<SeqTypes>, Error = SendError> + Clone + Send + Sync + Unpin + 'static,
    {
        let mut leaf_sender = leaf_sender;
        let mut leaves_stream = leaves_stream;

        loop {
            let leaf_result = leaves_stream.next().await;
            let leaf = if let Some(Ok(leaf)) = leaf_result {
                leaf
            } else {
                tracing::info!("leaf stream closed");
                break;
            };

            let leaf_send_result = leaf_sender.send(leaf).await;
            if let Err(err) = leaf_send_result {
                tracing::info!("leaf sender closed: {}", err);
                break;
            }
        }
    }
}

/// [Drop] implementation for [ProcessConsumeLeafStreamTask] that will cancel
/// the task if it hasn't already been completed.
impl Drop for ProcessProduceLeafStreamTask {
    fn drop(&mut self) {
        if let Some(task_handle) = self.task_handle.take() {
            task_handle.abort();
        }
    }
}

/// [populate_node_identity_general_from_scrape] populates the general
/// information of a [NodeIdentity] from a [Sample] that is expected to be
/// the "consensus_node_identity_general" sample.
fn populate_node_identity_general_from_scrape(
    node_identity: &mut NodeIdentity,
    node_identity_general_sample: &Sample,
) {
    node_identity.name = node_identity_general_sample
        .labels
        .get("name")
        .map(|s| s.into());
    node_identity.company = node_identity_general_sample
        .labels
        .get("company_name")
        .map(|s| s.into());
    let company_website = match node_identity_general_sample
        .labels
        .get("company_website")
        .map(Url::parse)
    {
        Some(Ok(url)) => Some(url),
        _ => None,
    };
    node_identity.company_website = company_website;
    node_identity.network_type = node_identity_general_sample
        .labels
        .get("network_type")
        .map(|s| s.into());
    node_identity.node_type = node_identity_general_sample
        .labels
        .get("node_type")
        .map(|s| s.into());
    node_identity.operating_system = node_identity_general_sample
        .labels
        .get("operating_system")
        .map(|s| s.into());
}

/// [populate_node_location_from_scrape] populates the location information of a
/// [NodeIdentity] from a [Sample] that is expected to be the
/// "consensus_node_identity_location" sample.
fn populate_node_location_from_scrape(
    node_identity: &mut NodeIdentity,
    node_identity_location_sample: &Sample,
) {
    let mut location = node_identity
        .location
        .take()
        .unwrap_or(LocationDetails::new(None, None));
    location.country = node_identity_location_sample
        .labels
        .get("country")
        .map(|s| s.into());

    let latitude = node_identity_location_sample
        .labels
        .get("latitude")
        .map(|s| s.parse::<f64>());
    let longitude = node_identity_location_sample
        .labels
        .get("longitude")
        .map(|s| s.parse::<f64>());

    if let (Some(Ok(latitude)), Some(Ok(longitude))) = (latitude, longitude) {
        location.coords = Some((latitude, longitude));
    }

    // Are there any details populated?
    if location.country.is_some() || location.coords.is_some() {
        node_identity.location = Some(location);
    } else {
        node_identity.location = None;
    }
}

/// [populate_node_identity_from_scrape] populates a [NodeIdentity] from a
/// [Scrape] that is expected to contain the necessary information to populate
/// the [NodeIdentity].
pub fn populate_node_identity_from_scrape(node_identity: &mut NodeIdentity, scrape: Scrape) {
    // Handle General Information Population

    // Let's verify that the scrape information contains and matches our node
    // identity's public key.
    {
        let node_key = scrape
            .docs
            .iter()
            .find(|(_, key)| key == &"node")
            .map(|(key, _)| key);

        let node_key = if let Some(node_key) = node_key {
            node_key
        } else {
            // We were unable to find the key for the public key on the metrics
            // scrape result.
            tracing::warn!("scrape result doesn't seem to contain 'node' key, preventing us from verifying the public key");
            return;
        };

        let node_sample = scrape
            .samples
            .iter()
            .find(|sample| &sample.metric == node_key);

        let node_sample = if let Some(node_sample) = node_sample {
            node_sample
        } else {
            // We were unable to find the sample for the public key on the metrics
            // scrape result.
            tracing::warn!("scrape result doesn't seem to contain 'node' sample, preventing us from verifying the public key. This is especially odd considering that we found the 'node' key already.");
            return;
        };

        let public_key_string = node_sample.labels.get("key");

        let public_key_from_scrape: BLSPubKey = if let Some(public_key_string) = public_key_string {
            match BLSPubKey::from_str(public_key_string) {
                Ok(public_key) => public_key,
                Err(err) => {
                    // We couldn't parse the public key, so we can't create a NodeIdentity.
                    tracing::info!("parsing public key failed: {}", err);
                    return;
                },
            }
        } else {
            // We were unable to find the public key in the scrape result.
            tracing::warn!("scrape result doesn't seem to contain 'key' label in the 'node' sample, preventing us from verifying the public key. This is especially odd considering that we found the 'node' key and sample already.");
            return;
        };

        let public_key_from_scrape_string = public_key_from_scrape.to_string();
        let node_identity_public_key_string = node_identity.public_key().to_string();

        if public_key_from_scrape_string != node_identity_public_key_string {
            tracing::warn!("node identity public key doesn't match public key in scrape, are we hitting the wrong URL, or is it behind a load balancer between multiple nodes?");
            return;
        }

        debug_assert_eq!(&public_key_from_scrape, node_identity.public_key());
    }

    // Determine the key for the "consensus_node_identity_general" sample
    // so we can populate the general information concerning node identity.
    let node_identity_general_key = scrape
        .docs
        .iter()
        .find(|(_, key)| key == &"node_identity_general")
        .map(|(key, _)| key);

    if let Some(node_identity_general_key) = node_identity_general_key {
        let node_identity_general_sample = scrape
            .samples
            .iter()
            .find(|sample| &sample.metric == node_identity_general_key);

        if let Some(node_identity_general_sample) = node_identity_general_sample {
            populate_node_identity_general_from_scrape(node_identity, node_identity_general_sample);
        }
    }

    // Lookup node identity location information, so we can populate it.
    let node_identity_location_key = scrape
        .docs
        .iter()
        .find(|(_, key)| key == &"node_identity_location")
        .map(|(key, _)| key);
    if let Some(node_identity_location_key) = node_identity_location_key {
        let node_identity_location_sample = scrape
            .samples
            .iter()
            .find(|sample| &sample.metric == node_identity_location_key);

        if let Some(node_identity_location_sample) = node_identity_location_sample {
            populate_node_location_from_scrape(node_identity, node_identity_location_sample);
        }
    }
}

/// [node_identity_from_scrape] creates a [NodeIdentity] from a [Scrape].  It
/// expects the [Scrape] to contain the necessary information to populate the
/// [NodeIdentity].  If the [Scrape] doesn't contain the necessary information
/// to populate the [NodeIdentity], then it will return [None].
pub fn node_identity_from_scrape(scrape: Scrape) -> Option<NodeIdentity> {
    let node_key = scrape
        .docs
        .iter()
        .find(|(_, key)| key == &"node")
        .map(|(key, _)| key);

    let node_key = node_key?;

    let node_sample = scrape
        .samples
        .iter()
        .find(|sample| &sample.metric == node_key);

    let node_sample = node_sample?;

    let public_key_string = node_sample.labels.get("key")?;

    let public_key = match BLSPubKey::from_str(public_key_string) {
        Ok(public_key) => public_key,
        Err(err) => {
            tracing::info!("parsing public key failed: {}", err);
            return None;
        },
    };

    let mut node_identity = NodeIdentity::from_public_key(public_key);
    populate_node_identity_from_scrape(&mut node_identity, scrape);

    Some(node_identity)
}

/// [ProcessNodeIdentityUrlStreamTask] is a task that processes a stream of
/// [Url]s that are expected to contain a Node Identity.  It will attempt to
/// retrieve the Node Identity from the [Url] and then send it to the [Sink]
/// provided.
pub struct ProcessNodeIdentityUrlStreamTask {
    pub task_handle: Option<JoinHandle<()>>,
}

impl ProcessNodeIdentityUrlStreamTask {
    /// [new] creates a new [ProcessNodeIdentityUrlStreamTask] that processes a
    /// stream of [Url]s that are expected to contain a Node Identity.
    ///
    /// Calling this function will spawn a new task that will start processing
    /// immediately.  The tasks' handle will be stored in the returned
    /// state.
    pub fn new<S, K>(url_receiver: S, node_identity_sender: K) -> Self
    where
        S: Stream<Item = Url> + Send + Sync + Unpin + 'static,
        K: Sink<NodeIdentity, Error = SendError> + Clone + Send + Sync + Unpin + 'static,
    {
        let task_handle = spawn(Self::process_node_identity_url_stream(
            url_receiver,
            node_identity_sender,
        ));

        Self {
            task_handle: Some(task_handle),
        }
    }

    /// [process_node_identity_url_stream] processes a stream of [Url]s that are
    /// expected to contain a Node Identity.  It will attempt to retrieve the Node
    /// Identity from the [Url] and then send it to the [Sink] provided.  If the
    /// [Sink] is closed, then the function will return.
    async fn process_node_identity_url_stream<T, K>(
        node_identity_url_stream: T,
        node_identity_sink: K,
    ) where
        T: futures::Stream<Item = Url> + Unpin,
        K: Sink<NodeIdentity, Error = futures::channel::mpsc::SendError> + Unpin,
    {
        let mut node_identity_url_stream = node_identity_url_stream;
        let mut node_identity_sender = node_identity_sink;
        loop {
            let node_identity_url_result = node_identity_url_stream.next().await;
            let node_identity_url = match node_identity_url_result {
                Some(node_identity_url) => node_identity_url,
                None => {
                    tracing::info!("node identity url stream closed");
                    return;
                },
            };

            // Alright we have a new Url to try and scrape for a Node Identity.
            // Let's attempt to do that.
            let node_identity_result = get_node_identity_from_url(node_identity_url).await;

            let node_identity = match node_identity_result {
                Ok(node_identity) => node_identity,
                Err(err) => {
                    tracing::warn!("get node identity from url failed.  bad base url?: {}", err);
                    continue;
                },
            };

            let send_result = node_identity_sender.send(node_identity).await;
            if let Err(err) = send_result {
                tracing::error!("node identity sender closed: {}", err);

                // We will be unable to provide any additional node identity
                // updates. This is considered a critical error.
                panic!("ProcessNodeIdentityUrlStreamTask node_identity_sender closed, future node identity information will stagnate: {}", err);
            }
        }
    }
}

/// [ProcessNodeIdentityUrlStreamTask] will cancel the task when it is dropped.
impl Drop for ProcessNodeIdentityUrlStreamTask {
    fn drop(&mut self) {
        let task_handle = self.task_handle.take();
        if let Some(task_handle) = task_handle {
            task_handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader};

    fn example_prometheus_output() -> &'static str {
        include_str!("example_prometheus_metrics_output.txt")
    }

    #[test]
    fn test_prometheus_scraping_example() {
        let example_input = example_prometheus_output();

        let buffered_reader = BufReader::new(example_input.as_bytes());
        let lines = buffered_reader.lines();

        let scrape_result = prometheus_parse::Scrape::parse(lines);

        assert!(scrape_result.is_ok());
        let scrape = scrape_result.unwrap();

        let node_identity_general_key = scrape
            .docs
            .iter()
            .find(|(_, key)| key == &"node_identity_general")
            .map(|(key, _)| key);
        let node_identity_location_key = scrape
            .docs
            .iter()
            .find(|(_, key)| key == &"node_identity_location")
            .map(|(key, _)| key);

        assert!(node_identity_general_key.is_some());
        assert!(node_identity_location_key.is_some());

        let node_identity_general_key = node_identity_general_key.unwrap();
        let node_identity_location_key = node_identity_location_key.unwrap();

        // Let's look for the general_info
        let node_identity_general = scrape
            .samples
            .iter()
            .find(|sample| &sample.metric == node_identity_general_key);

        let node_identity_location = scrape
            .samples
            .iter()
            .find(|sample| &sample.metric == node_identity_location_key);

        assert!(node_identity_general.is_some());
        assert!(node_identity_location.is_some());

        let node_identity_general = node_identity_general.unwrap();
        let node_identity_location = node_identity_location.unwrap();

        assert_eq!(
            node_identity_general.labels.get("company_name"),
            Some("Espresso Systems")
        );
        assert_eq!(node_identity_general.labels.get("name"), Some("sequencer0"));
        assert_eq!(
            node_identity_general.labels.get("network_type"),
            Some("local")
        );
        assert_eq!(
            node_identity_general.labels.get("node_type"),
            Some("espresso-sequencer 0.1")
        );
        assert_eq!(
            node_identity_general.labels.get("node_type"),
            Some("espresso-sequencer 0.1")
        );
        assert_eq!(
            node_identity_general.labels.get("operating_system"),
            Some("Linux 5.15.153.1")
        );
        assert_eq!(
            node_identity_general.labels.get("wallet"),
            Some("0x0000000000000000000000000000000000000000")
        );

        assert_eq!(node_identity_location.labels.get("country"), Some("US"));
        assert_eq!(
            node_identity_location.labels.get("latitude"),
            Some("-40.7128")
        );
        assert_eq!(
            node_identity_location.labels.get("longitude"),
            Some("-74.0060")
        );

        print!("{:?}", scrape);
    }

    #[test]
    fn test_node_identity_from_scrape() {
        let example_input = example_prometheus_output();

        let buffered_reader = BufReader::new(example_input.as_bytes());
        let lines = buffered_reader.lines();

        let scrape_result = prometheus_parse::Scrape::parse(lines);

        assert!(scrape_result.is_ok());
        let scrape = scrape_result.unwrap();

        let node_identity = super::node_identity_from_scrape(scrape);

        assert!(node_identity.is_some());
        let node_identity = node_identity.unwrap();

        assert_eq!(
            node_identity.company(),
            &Some("Espresso Systems".to_string())
        );
        assert_eq!(node_identity.name(), &Some("sequencer0".to_string()));
        assert_eq!(node_identity.network_type(), &Some("local".to_string()));
        assert_eq!(
            node_identity.node_type(),
            &Some("espresso-sequencer 0.1".to_string())
        );
        assert_eq!(
            node_identity.operating_system(),
            &Some("Linux 5.15.153.1".to_string())
        );

        assert!(node_identity.location().is_some());
        let node_identity_location = node_identity.location().unwrap();

        assert_eq!(node_identity_location.country(), &Some("US".to_string()));
        assert_eq!(node_identity_location.coords, Some((-40.7128, -74.0060)));
    }
}
