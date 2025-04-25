pub mod create_node_validator_api;

use std::{fmt, future::Future, io::BufRead, pin::Pin, str::FromStr, time::Duration};

use espresso_types::{BackoffParams, SeqTypes};
use futures::{
    channel::mpsc::{self, SendError, Sender},
    future::{BoxFuture, Either},
    pin_mut, FutureExt, Sink, SinkExt, Stream, StreamExt,
};
use hotshot_query_service::{
    availability::{BlockQueryData, Leaf1QueryData},
    types::HeightIndexed,
};
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
use tokio::{spawn, task::JoinHandle};
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
pub struct PublicHotShotConfig {
    pub known_nodes_with_stake: Vec<PeerConfig<SeqTypes>>,
    pub epoch_height: Option<u64>,
    pub epoch_start_block: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct SequencerConfig {
    pub config: PublicHotShotConfig,
}

/// [get_config_stake_table_from_sequencer] retrieves the stake table from the
/// Sequencer.  It expects a [surf_disco::Client] to be provided so that it can
/// make the request to the Hotshot Query Service.  It will return a
/// [StakeTable] that is populated with the data retrieved from the Hotshot
/// Query Service.
pub async fn get_config_stake_table_from_sequencer(
    client: surf_disco::Client<hotshot_query_service::Error, Version01>,
) -> Result<
    (
        PublicHotShotConfig,
        StakeTable<BLSPubKey, StateVerKey, CircuitField>,
    ),
    hotshot_query_service::Error,
> {
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

    for node in public_hot_shot_config.known_nodes_with_stake.iter() {
        stake_table
            .register(
                *node.stake_table_entry.key(),
                node.stake_table_entry.stake(),
                node.state_ver_key.clone(),
            )
            .expect("registering stake table entry");
    }

    stake_table.advance();
    stake_table.advance();

    Ok((public_hot_shot_config, stake_table))
}

// [get_node_stake_table_from_sequencer] is a function that is similar to
// [get_config_stake_table_from_sequencer], but it retrieves the stake table,
// and only the stake table from a sequencer for a given epoch.
pub async fn get_node_stake_table_from_sequencer(
    client: surf_disco::Client<hotshot_query_service::Error, Version01>,
    epoch: u64,
) -> Result<StakeTable<BLSPubKey, StateVerKey, CircuitField>, hotshot_query_service::Error> {
    let path = format!("node/stake-table/{epoch}");
    // Let's figure out our epoch height
    let request = client
        .get(&path)
        // We need to set the Accept header, otherwise the Content-Type
        // will be application/octet-stream, and we won't be able to
        // deserialize the response.
        .header("Accept", "application/json");

    let stake_table_result = request.send().await;

    let peer_config: Vec<PeerConfig<SeqTypes>> = match stake_table_result {
        Ok(public_hot_shot_config) => public_hot_shot_config,
        Err(err) => {
            tracing::info!("retrieve stake table request failed: {}", err);
            return Err(err);
        },
    };

    let mut stake_table =
        StakeTable::<BLSPubKey, StateVerKey, CircuitField>::new(peer_config.len());

    for node in peer_config.into_iter() {
        stake_table
            .register(
                *node.stake_table_entry.key(),
                node.stake_table_entry.stake(),
                node.state_ver_key,
            )
            .expect("registering stake table entry");
    }

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

/// [AvailabilityConnection] is a simple short-hand type alias for a
/// surf-disco [Connection] that is used to retrieve data from the
/// Availability API.
type AvailabilityConnection<T> = surf_disco::socket::Connection<
    T,
    surf_disco::socket::Unsupported,
    hotshot_query_service::Error,
    Version01,
>;

/// BoxFutureConnection is a simple short-hand type alias for a
/// [BoxFuture] that is used to retrieve a [AvailabilityConnection].
type BoxFutureConnection<'a, T> =
    BoxFuture<'a, Result<AvailabilityConnection<T>, hotshot_query_service::Error>>;

pub struct SurfDiscoAvailabilityAPIStream<'a, T> {
    // path_url: Url,
    client: surf_disco::Client<hotshot_query_service::Error, Version01>,

    connection: Option<AvailabilityConnection<T>>,

    connection_future: Option<BoxFutureConnection<'a, T>>,

    last_received_block: u64,

    backoff_params: BackoffParams,
}

const MAX_STREAM_RECONNECT_ATTEMPTS: usize = 100;

/// [SurfDiscoAvailabilityAPIPathResolver] is a trait that allows for the
/// specification of a sub path to the base URL that will resolve in a
/// URL to point to the correct endpoint for the desired Stream type.
///
/// Many streams in the Availability API have a path that is based on the
/// specific type of data you are wanting to stream, and the block height
/// to start retrieving that data for.  This trait allows for us to
/// abstract out these endpoints.
pub trait SurfDiscoAvailabilityAPIPathResolver {
    /// [resolve_path_for_height] resolves the path for the given height.
    /// It is expected that the path will be appended to the base URL
    /// to create a full URL that can be used to connect to the stream.
    fn resolve_path_for_height(&self, height: u64) -> String;
}

impl SurfDiscoAvailabilityAPIPathResolver
    for SurfDiscoAvailabilityAPIStream<'_, Leaf1QueryData<SeqTypes>>
{
    fn resolve_path_for_height(&self, height: u64) -> String {
        format!("availability/stream/leaves/{}", height)
    }
}

impl SurfDiscoAvailabilityAPIPathResolver
    for SurfDiscoAvailabilityAPIStream<'_, BlockQueryData<SeqTypes>>
{
    fn resolve_path_for_height(&self, height: u64) -> String {
        format!("availability/stream/blocks/{}", height)
    }
}

/// [UpdateBlockHeightForEntry] is a trait that allows for the updating of
/// the block height for a given entry.  This is useful for updating the
/// last received block height for a stream.
pub trait UpdateBlockHeightForEntry<T> {
    /// [block_height_for_entry] returns the block height for the given entry.
    /// This is useful for updating the last received block height for a stream.
    fn block_height_for_entry(&self, entry: &T) -> u64;

    /// [update_block_height_for_entry] updates the block height for the
    /// given entry.  This is useful for updating the last received block
    /// height for a stream.
    fn update_block_height_for_entry(&mut self, entry: &T);
}

impl UpdateBlockHeightForEntry<Leaf1QueryData<SeqTypes>>
    for SurfDiscoAvailabilityAPIStream<'_, Leaf1QueryData<SeqTypes>>
{
    fn block_height_for_entry(&self, entry: &Leaf1QueryData<SeqTypes>) -> u64 {
        entry.leaf().height()
    }

    fn update_block_height_for_entry(&mut self, entry: &Leaf1QueryData<SeqTypes>) {
        self.last_received_block = self.block_height_for_entry(entry);
    }
}

impl UpdateBlockHeightForEntry<BlockQueryData<SeqTypes>>
    for SurfDiscoAvailabilityAPIStream<'_, BlockQueryData<SeqTypes>>
{
    fn block_height_for_entry(&self, entry: &BlockQueryData<SeqTypes>) -> u64 {
        entry.height()
    }

    fn update_block_height_for_entry(&mut self, entry: &BlockQueryData<SeqTypes>) {
        self.last_received_block = self.block_height_for_entry(entry);
    }
}

impl SurfDiscoAvailabilityAPIStream<'_, Leaf1QueryData<SeqTypes>> {
    pub fn new_leaf_stream(
        client: surf_disco::Client<hotshot_query_service::Error, Version01>,
        starting_block: u64,
    ) -> Self {
        Self {
            client,
            connection: None,
            last_received_block: starting_block,
            backoff_params: BackoffParams::default(),
            connection_future: None,
        }
    }
}

impl SurfDiscoAvailabilityAPIStream<'_, BlockQueryData<SeqTypes>> {
    pub fn new_block_stream(
        client: surf_disco::Client<hotshot_query_service::Error, Version01>,
        starting_block: u64,
    ) -> Self {
        Self {
            client,
            connection: None,
            last_received_block: starting_block,
            backoff_params: BackoffParams::default(),
            connection_future: None,
        }
    }
}

impl<T> SurfDiscoAvailabilityAPIStream<'_, T>
where
    T: serde::de::DeserializeOwned,
    Self: SurfDiscoAvailabilityAPIPathResolver + UpdateBlockHeightForEntry<T>,
{
}

impl<T> Stream for SurfDiscoAvailabilityAPIStream<'_, T>
where
    T: serde::de::DeserializeOwned,
    Self: SurfDiscoAvailabilityAPIPathResolver + UpdateBlockHeightForEntry<T>,
{
    type Item = T;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<Option<Self::Item>> {
        // We want pausable polling, with mutation. We want to ensure that
        // each step along the way is just fine.
        let self_mut = self.get_mut();

        // Next, do we already have a connection?
        if self_mut.connection.is_some() {
            // Alright, then we'll want to retrieve the next entry
            let connection_mut = self_mut.connection.as_mut().expect("unreachable");
            pin_mut!(connection_mut);

            return match connection_mut.poll_next(cx) {
                // We're waiting for the next entry, still
                std::task::Poll::Pending => std::task::Poll::Pending,

                // We've received a result from the connection
                std::task::Poll::Ready(Some(Ok(entry))) => {
                    let block_height = self_mut.block_height_for_entry(&entry);
                    if block_height <= self_mut.last_received_block {
                        tracing::debug!("we received an entry for a height prior to the last we've seen: {block_height} <= {}", self_mut.last_received_block);
                        // We've received a block that we've already received
                        // before.  We should skip this block and try again.
                        // We need to reschedule ourselves in order to make progress
                        cx.waker().wake_by_ref();
                        return std::task::Poll::Pending;
                    }

                    tracing::debug!("received entry for block height: {block_height}");

                    self_mut.update_block_height_for_entry(&entry);
                    std::task::Poll::Ready(Some(entry))
                },

                // The Stream Closed
                std::task::Poll::Ready(None) => {
                    tracing::debug!("stream ended unexpectedly. Will reacquire");
                    self_mut.connection = None;
                    // We need to reschedule ourselves in order to make progress
                    cx.waker().wake_by_ref();
                    std::task::Poll::Pending
                },

                // The Stream encountered an error.
                std::task::Poll::Ready(Some(Err(err))) => {
                    tracing::debug!("encountered error retrieving entry from Stream: {}", err);
                    self_mut.connection = None;
                    // We need to reschedule ourselves in order to make progress
                    cx.waker().wake_by_ref();
                    std::task::Poll::Pending
                },
            };
        }

        // Do we have an attempt to retrieve a connection in progress?
        if self_mut.connection_future.is_some() {
            tracing::debug!("waiting for connection to be established");
            let connection_future = self_mut.connection_future.as_mut().expect("unreachable");
            pin_mut!(connection_future);
            match connection_future.poll(cx) {
                std::task::Poll::Pending => {
                    return std::task::Poll::Pending;
                },

                std::task::Poll::Ready(Ok(connection)) => {
                    self_mut.connection_future = None;
                    self_mut.connection = Some(connection);
                    // We need to reschedule ourselves in order to make progress
                    cx.waker().wake_by_ref();
                    return std::task::Poll::Pending;
                },

                std::task::Poll::Ready(Err(err)) => {
                    tracing::debug!("encountered error retrieving connection: {}", err);
                    self_mut.connection_future = None;
                    // We need to reschedule ourselves in order to make progress
                    cx.waker().wake_by_ref();
                    return std::task::Poll::Pending;
                },
            }
        }

        tracing::debug!("attempting to open connection for availability stream");
        // We're not connected yet. So let's try to connect.
        let path = self_mut.resolve_path_for_height(self_mut.last_received_block);
        let client = self_mut.client.clone();
        let backoff_params = self_mut.backoff_params;
        self_mut.connection_future.replace(
            async move {
                let path = path;
                let client = client;
                let backoff_params = backoff_params;
                let mut delay = Duration::from_millis(100);

                for attempt in 0..MAX_STREAM_RECONNECT_ATTEMPTS {
                    match client.socket(&path).subscribe().await {
                        Ok(connection) => {
                            tracing::debug!(
                                "attempt {}: successfully acquired connection",
                                attempt
                            );
                            return Ok(connection);
                        },

                        Err(err) => {
                            tracing::debug!(
                                "attempt {}: encountered error retrieving connection: {}",
                                attempt,
                                err
                            );
                            delay = backoff_params.backoff(delay);
                            tokio::time::sleep(delay).await;
                            continue;
                        },
                    }
                }

                tracing::warn!(
                    "unable to retrieve connection after {} attempts",
                    MAX_STREAM_RECONNECT_ATTEMPTS
                );
                panic!(
                    "unable to retrieve connection after {} attempts",
                    MAX_STREAM_RECONNECT_ATTEMPTS
                );
            }
            .boxed(),
        );

        // Let's wake ourselves up
        cx.waker().wake_by_ref();
        std::task::Poll::Pending
    }
}

/// AvailabilityAPILeafStream is a trait that represents a stream of [Leaf]s
/// in their simplest form.  This acts as a simple type declaration for
/// quick reference.
pub trait AvailabilityAPILeafStream: Stream<Item = Leaf1QueryData<SeqTypes>> {}

/// We implicitly implement the [AvailabilityAPILeafStream] trait for any
/// Stream that matches the produced item [Leaf]s.
impl<S> AvailabilityAPILeafStream for S where S: Stream<Item = Leaf1QueryData<SeqTypes>> {}

/// [ProcessProduceLeafStreamTask] is a task that produce a stream of
/// [BlockQueryData]s. This acts as a simple type declaration for quick
/// reference.
pub trait AvailabilityAPIBlockStream: Stream<Item = BlockQueryData<SeqTypes>> {}

/// We implicitly implement the [AvailabilityAPIBlockStream] trait for any
/// Stream that matches the produced item [BlockQueryData]s.
impl<S> AvailabilityAPIBlockStream for S where S: Stream<Item = BlockQueryData<SeqTypes>> {}

/// [LeafAndBlock] is a tuple that contains a [Leaf] and a [BlockQueryData].
/// This acts as a simple type declaration for quick reference.
pub type LeafAndBlock<T> = (Leaf1QueryData<T>, BlockQueryData<T>);

/// [LeafAndBlockPairStream] is a trait that represents a stream of [Leaf1QueryData]s
/// and [BlockQueryData]s.  This acts as a simple type declaration for quick
/// reference.
pub trait LeafAndBlockPairStream: Stream<Item = LeafAndBlock<SeqTypes>> {}

/// We implicitly implement the [LeafAndBlockPairStream] trait for any
/// Stream that matches the produced a pair of [Leaf1QueryData], and [BlockQueryData].
impl<S> LeafAndBlockPairStream for S where S: Stream<Item = LeafAndBlock<SeqTypes>> {}

/// [BridgeLeafAndBlockStreamToSenderTask] is a task that produce a stream of
/// pairs of [Leaf1QueryData]s and [BlockQueryData]s from the Hotshot Query Service. It
/// will attempt to retrieve the [Leaf1QueryData]s and [BlockQueryData]s from the Hotshot
/// Query Service and then send them to the [Sink] provided.
pub struct BridgeLeafAndBlockStreamToSenderTask {
    pub task_handle: Option<JoinHandle<()>>,
}

impl BridgeLeafAndBlockStreamToSenderTask {
    /// [new] creates a new [ProcessConsumeLeafStreamTask] that produces a
    /// stream of [Leaf1QueryData]s from the Hotshot Query Service.
    ///
    /// Calling this function will create an async task that will start
    /// processing immediately.  The task's handle will be stored in the
    /// returned state.
    pub fn new<R, K>(item_stream: R, item_sender: K) -> Self
    where
        R: LeafAndBlockPairStream + Send + Unpin + 'static,
        K: Sink<LeafAndBlock<SeqTypes>, Error = SendError> + Clone + Send + Unpin + 'static,
    {
        // let future = Self::process_consume_leaf_stream(leaf_stream_retriever, leaf_sender);
        let task_handle = spawn(Self::bridge_stream(item_stream, item_sender));

        Self {
            task_handle: Some(task_handle),
        }
    }

    async fn bridge_stream<R, K>(item_stream: R, item_sender: K)
    where
        R: LeafAndBlockPairStream + Unpin,
        K: Sink<LeafAndBlock<SeqTypes>, Error = SendError> + Clone + Unpin + 'static,
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

        // Consume the leaves of a stream
        Self::process_consume_leaf_stream::<R, K>(item_stream, item_sender.clone()).await;
        tracing::warn!("leaf stream ended, will attempt to re-acquire leaf stream");
    }

    /// [process_consume_leaf_stream] produces a stream of [Leaf]s from the
    /// Hotshot Query Service.  It will attempt to retrieve the [Leaf]s from the
    /// Hotshot Query Service and then send them to the [Sink] provided.  If the
    /// [Sink] is closed, or if the Stream ends prematurely, then the function
    /// will return.
    async fn process_consume_leaf_stream<R, K>(item_stream: R, item_sender: K)
    where
        R: LeafAndBlockPairStream + Unpin,
        K: Sink<LeafAndBlock<SeqTypes>, Error = SendError> + Clone + Unpin + 'static,
    {
        let mut leaf_sender = item_sender;
        let mut leaves_stream = item_stream;

        loop {
            let leaf_result = leaves_stream.next().await;
            let leaf = if let Some(leaf) = leaf_result {
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
impl Drop for BridgeLeafAndBlockStreamToSenderTask {
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

            tracing::debug!("received url to scrape: {}", node_identity_url);

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

            tracing::debug!(
                "successfully retrieved node identity from url: {}",
                node_identity.public_key(),
            );

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
