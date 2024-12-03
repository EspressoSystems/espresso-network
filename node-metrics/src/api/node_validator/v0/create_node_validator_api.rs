use std::sync::Arc;

use async_lock::RwLock;
use espresso_types::{PubKey, SeqTypes};
use futures::{
    channel::mpsc::{self, Receiver, SendError, Sender},
    Sink, SinkExt, Stream, StreamExt,
};
use hotshot_types::{
    data::Leaf2,
    event::{Event, EventType},
};
use serde::{Deserialize, Serialize};
use tokio::{spawn, task::JoinHandle};
use url::Url;

use super::{get_stake_table_from_sequencer, LeafAndBlock, ProcessNodeIdentityUrlStreamTask};
use crate::service::{
    client_id::ClientId,
    client_message::InternalClientMessage,
    client_state::{
        ClientThreadState, InternalClientMessageProcessingTask,
        ProcessDistributeBlockDetailHandlingTask, ProcessDistributeNodeIdentityHandlingTask,
        ProcessDistributeVotersHandlingTask,
    },
    data_state::{DataState, ProcessLeafAndBlockPairStreamTask, ProcessNodeIdentityStreamTask},
    server_message::ServerMessage,
};

pub struct NodeValidatorAPI<K> {
    pub process_internal_client_message_handle: Option<InternalClientMessageProcessingTask>,
    pub process_distribute_block_detail_handle: Option<ProcessDistributeBlockDetailHandlingTask>,
    pub process_distribute_node_identity_handle: Option<ProcessDistributeNodeIdentityHandlingTask>,
    pub process_distribute_voters_handle: Option<ProcessDistributeVotersHandlingTask>,
    pub process_leaf_stream_handle: Option<ProcessLeafAndBlockPairStreamTask>,
    pub process_node_identity_stream_handle: Option<ProcessNodeIdentityStreamTask>,
    pub process_url_stream_handle: Option<ProcessNodeIdentityUrlStreamTask>,
    pub url_sender: K,
}

pub struct NodeValidatorConfig {
    pub stake_table_url_base: Url,
    pub initial_node_public_base_urls: Vec<Url>,
}

#[derive(Debug)]
pub enum CreateNodeValidatorProcessingError {
    FailedToGetStakeTable(hotshot_query_service::Error),
}

/// An external message that can be sent to or received from a node
#[derive(Serialize, Deserialize, Clone)]
pub enum ExternalMessage {
    /// A request for a node to respond with its identifier
    /// Contains the public key of the node that is requesting the roll call
    RollCallRequest(PubKey),

    /// A response to a roll call request
    /// Contains the identifier of the node
    RollCallResponse(RollCallInfo),
}

/// Information about a node that is used in a roll call response
#[derive(Serialize, Deserialize, Clone)]
pub struct RollCallInfo {
    // The public API URL of the node
    pub public_api_url: Url,
}

/// [HotShotEventProcessingTask] is a task that is capable of processing events
/// that are coming in from a HotShot event stream.  This task will keep an
/// eye out for ExternalMessageReceived events that can be decoded as a
/// RollCallResponse.  When a RollCallResponse is received, the public API URL
/// of the node that sent the message will be sent to the provided url_sender.
///
/// Additionally, this can receive Decide events and send the discovered leaves
/// to the provided leaf_sender.  This can can be used as a means of receiving
/// leaves that doesn't involve hitting an external service like the task
/// [ProcessProduceLeafStreamTask] does.
pub struct HotShotEventProcessingTask {
    pub task_handle: Option<JoinHandle<()>>,
}

impl HotShotEventProcessingTask {
    /// [new] creates a new [HotShotEventProcessingTask] that will process
    /// events from the provided event_stream.
    ///
    /// Calls to [new] will spawn a new task that will start processing
    /// immediately. The task handle will be stored in the returned structure.
    pub fn new<S, K1, K2>(event_stream: S, url_sender: K1, leaf_sender: K2) -> Self
    where
        S: Stream<Item = Event<SeqTypes>> + Send + Unpin + 'static,
        K1: Sink<Url, Error = SendError> + Send + Unpin + 'static,
        K2: Sink<Leaf2<SeqTypes>, Error = SendError> + Send + Unpin + 'static,
    {
        let task_handle = spawn(Self::process_messages(
            event_stream,
            url_sender,
            leaf_sender,
        ));

        Self {
            task_handle: Some(task_handle),
        }
    }

    /// [process_messages] is a function that will process messages from the
    /// provided event stream.
    async fn process_messages<S, K1, K2>(event_receiver: S, url_sender: K1, leaf_sender: K2)
    where
        S: Stream<Item = Event<SeqTypes>> + Send + Unpin + 'static,
        K1: Sink<Url, Error = SendError> + Unpin,
        K2: Sink<Leaf2<SeqTypes>, Error = SendError> + Unpin,
    {
        let mut event_stream = event_receiver;
        let mut url_sender = url_sender;
        let mut leaf_sender = leaf_sender;
        loop {
            let event_result = event_stream.next().await;
            let event = match event_result {
                Some(event) => event,
                None => {
                    tracing::info!("event stream closed");
                    break;
                },
            };

            let Event { event, .. } = event;

            match event {
                EventType::Decide { leaf_chain, .. } => {
                    for leaf_info in leaf_chain.iter().rev() {
                        let leaf = leaf_info.leaf.clone();

                        let send_result = leaf_sender.send(leaf).await;
                        if let Err(err) = send_result {
                            tracing::error!("leaf sender closed: {}", err);
                            panic!("HotShotEventProcessingTask leaf sender is closed, unrecoverable, the block state will stagnate.");
                        }
                    }
                },

                EventType::ExternalMessageReceived { data, .. } => {
                    let roll_call_info = match bincode::deserialize(&data) {
                        Ok(ExternalMessage::RollCallResponse(roll_call_info)) => roll_call_info,

                        Err(err) => {
                            tracing::info!(
                                "failed to deserialize external message, unrecognized: {}",
                                err
                            );
                            continue;
                        },

                        _ => {
                            // Ignore any other potentially recognized messages
                            continue;
                        },
                    };

                    let public_api_url = roll_call_info.public_api_url;

                    // Send the discovered public url to the sink
                    let send_result = url_sender.send(public_api_url).await;
                    if let Err(err) = send_result {
                        tracing::error!("url sender closed: {}", err);
                        panic!("HotShotEventProcessingTask url sender is closed, unrecoverable, the node state will stagnate.");
                    }
                },
                _ => {
                    // Ignore all other events
                    continue;
                },
            }
        }
    }
}

/// [Drop] implementation for [HotShotEventProcessingTask] that will cancel the
/// task when the structure is dropped.
impl Drop for HotShotEventProcessingTask {
    fn drop(&mut self) {
        if let Some(task_handle) = self.task_handle.take() {
            task_handle.abort();
        }
    }
}

/// [ProcessExternalMessageHandlingTask] is a task that is capable of processing
/// external messages that are coming in from an external message stream.  This
/// task will keep an eye out for ExternalMessageReceived events that can be
/// decoded as a RollCallResponse.  When a RollCallResponse is received, the
/// public API URL of the node that sent the message will be sent to the
/// provided url_sender.
///
/// This task can be used as a means of processing [ExternalMessage]s that are
/// not being provided by a HotShot event stream.  It can be used as an
/// alternative to the [HotShotEventProcessingTask] for processing external
/// messages.
pub struct ProcessExternalMessageHandlingTask {
    pub task_handle: Option<JoinHandle<()>>,
}

impl ProcessExternalMessageHandlingTask {
    /// [new] creates a new [ProcessExternalMessageHandlingTask] that will
    /// process external messages from the provided external_message_receiver.
    ///
    /// Calls to [new] will spawn a new task that will start processing
    /// immediately. The task handle will be stored in the returned structure.
    pub fn new<S, K>(external_message_receiver: S, url_sender: K) -> Self
    where
        S: Stream<Item = ExternalMessage> + Send + Unpin + 'static,
        K: Sink<Url, Error = SendError> + Send + Unpin + 'static,
    {
        let task_handle = spawn(Self::process_external_messages(
            external_message_receiver,
            url_sender,
        ));

        Self {
            task_handle: Some(task_handle),
        }
    }

    /// [process_external_messages] is a function that will process messages from
    /// the provided external message stream.
    async fn process_external_messages<S, K>(external_message_receiver: S, url_sender: K)
    where
        S: Stream<Item = ExternalMessage> + Send + Unpin + 'static,
        K: Sink<Url, Error = SendError> + Send + Unpin + 'static,
    {
        let mut external_message_receiver = external_message_receiver;
        let mut url_sender = url_sender;

        loop {
            let external_message_result = external_message_receiver.next().await;
            let external_message = match external_message_result {
                Some(external_message) => external_message,
                None => {
                    tracing::error!("external message receiver closed");
                    break;
                },
            };

            match external_message {
                ExternalMessage::RollCallResponse(roll_call_info) => {
                    let public_api_url = roll_call_info.public_api_url;

                    let send_result = url_sender.send(public_api_url).await;
                    if let Err(err) = send_result {
                        tracing::error!("url sender closed: {}", err);
                        break;
                    }
                },

                _ => {
                    // Ignore all other messages
                    continue;
                },
            }
        }
    }
}

/// [Drop] implementation for [ProcessExternalMessageHandlingTask] that will
/// cancel the task when the structure is dropped.
impl Drop for ProcessExternalMessageHandlingTask {
    fn drop(&mut self) {
        if let Some(task_handle) = self.task_handle.take() {
            task_handle.abort();
        }
    }
}

/**
 * create_node_validator_processing is a function that creates a node validator
 * processing environment.  This function will create a number of tasks that
 * will be responsible for processing the data streams that are coming in from
 * the various sources.  This function will also create the data state that
 * will be used to store the state of the network.
 */
pub async fn create_node_validator_processing(
    config: NodeValidatorConfig,
    internal_client_message_receiver: Receiver<InternalClientMessage<Sender<ServerMessage>>>,
    leaf_and_block_pair_receiver: Receiver<LeafAndBlock<SeqTypes>>,
) -> Result<NodeValidatorAPI<Sender<Url>>, CreateNodeValidatorProcessingError> {
    let client_thread_state = ClientThreadState::<Sender<ServerMessage>>::new(
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        ClientId::from_count(1),
    );

    let client_stake_table = surf_disco::Client::new(config.stake_table_url_base.clone());

    let stake_table = get_stake_table_from_sequencer(client_stake_table)
        .await
        .map_err(CreateNodeValidatorProcessingError::FailedToGetStakeTable)?;

    let data_state = DataState::new(Default::default(), Default::default(), stake_table);

    let data_state = Arc::new(RwLock::new(data_state));
    let client_thread_state = Arc::new(RwLock::new(client_thread_state));
    let (block_detail_sender, block_detail_receiver) = mpsc::channel(32);
    let (node_identity_sender_1, node_identity_receiver_1) = mpsc::channel(32);
    let (node_identity_sender_2, node_identity_receiver_2) = mpsc::channel(32);
    let (voters_sender, voters_receiver) = mpsc::channel(32);
    let (mut url_sender, url_receiver) = mpsc::channel(32);

    let process_internal_client_message_handle = InternalClientMessageProcessingTask::new(
        internal_client_message_receiver,
        data_state.clone(),
        client_thread_state.clone(),
    );

    let process_distribute_block_detail_handle = ProcessDistributeBlockDetailHandlingTask::new(
        client_thread_state.clone(),
        block_detail_receiver,
    );

    let process_distribute_node_identity_handle = ProcessDistributeNodeIdentityHandlingTask::new(
        client_thread_state.clone(),
        node_identity_receiver_2,
    );

    let process_distribute_voters_handle =
        ProcessDistributeVotersHandlingTask::new(client_thread_state.clone(), voters_receiver);

    let process_leaf_stream_handle = ProcessLeafAndBlockPairStreamTask::new(
        leaf_and_block_pair_receiver,
        data_state.clone(),
        block_detail_sender,
        voters_sender,
    );

    let process_node_identity_stream_handle = ProcessNodeIdentityStreamTask::new(
        node_identity_receiver_1,
        data_state.clone(),
        node_identity_sender_2,
    );

    let process_url_stream_handle =
        ProcessNodeIdentityUrlStreamTask::new(url_receiver, node_identity_sender_1);

    tracing::debug!("sending initial urls to url sender to process node identity");
    // Send any initial URLS to the url sender for immediate processing.
    // These urls are supplied by the configuration of this function
    {
        let urls = config.initial_node_public_base_urls;

        for url in urls {
            let send_result = url_sender.send(url).await;
            if let Err(err) = send_result {
                tracing::info!("url sender closed: {}", err);
                break;
            }
        }
    }

    Ok(NodeValidatorAPI {
        process_internal_client_message_handle: Some(process_internal_client_message_handle),
        process_distribute_block_detail_handle: Some(process_distribute_block_detail_handle),
        process_distribute_node_identity_handle: Some(process_distribute_node_identity_handle),
        process_distribute_voters_handle: Some(process_distribute_voters_handle),
        process_leaf_stream_handle: Some(process_leaf_stream_handle),
        process_node_identity_stream_handle: Some(process_node_identity_stream_handle),
        process_url_stream_handle: Some(process_url_stream_handle),
        url_sender: url_sender.clone(),
    })
}

#[cfg(test)]
mod test {
    use futures::{
        channel::mpsc::{self, Sender},
        StreamExt,
    };
    use tide_disco::App;
    use tokio::spawn;

    use crate::{
        api::node_validator::v0::{
            BridgeLeafAndBlockStreamToSenderTask, StateClientMessageSender,
            SurfDiscoAvailabilityAPIStream, STATIC_VER_0_1,
        },
        service::{
            client_message::InternalClientMessage, data_state::MAX_VOTERS_HISTORY,
            server_message::ServerMessage,
        },
    };

    struct TestState(Sender<InternalClientMessage<Sender<ServerMessage>>>);

    impl StateClientMessageSender<Sender<ServerMessage>> for TestState {
        fn sender(&self) -> Sender<InternalClientMessage<Sender<ServerMessage>>> {
            self.0.clone()
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn test_full_setup_example() {
        use hotshot::helpers::initialize_logging;
        initialize_logging();

        let (internal_client_message_sender, internal_client_message_receiver) = mpsc::channel(32);
        let state = TestState(internal_client_message_sender);

        let mut app: App<_, crate::api::node_validator::v0::Error> = App::with_state(state);
        let node_validator_api_result = super::super::define_api::<TestState>();
        let node_validator_api = match node_validator_api_result {
            Ok(node_validator_api) => node_validator_api,
            Err(err) => {
                panic!("error defining node validator api: {:?}", err);
            },
        };

        match app.register_module("node-validator", node_validator_api) {
            Ok(_) => {},
            Err(err) => {
                panic!("error registering node validator api: {:?}", err);
            },
        }

        let client = surf_disco::Client::new(
            "https://query.main.net.espresso.network/v0/"
                .parse()
                .unwrap(),
        );

        // Let's get the current starting block height.
        let block_height = {
            let block_height_result = client.get("status/block-height").send().await;
            let block_height: u64 = match block_height_result {
                Ok(block_height) => block_height,
                Err(err) => {
                    tracing::warn!("retrieve block height request failed: {}", err);
                    panic!("error retrieving block height request failed: {}", err);
                },
            };

            // We want to make sure that we have at least MAX_VOTERS_HISTORY blocks of
            // history that we are pulling
            block_height.saturating_sub(MAX_VOTERS_HISTORY as u64 + 1)
        };

        tracing::info!("retrieved initial block height, starting at {block_height}");

        let leaf_stream =
            SurfDiscoAvailabilityAPIStream::new_leaf_stream(client.clone(), block_height);
        let block_stream = SurfDiscoAvailabilityAPIStream::new_block_stream(client, block_height);

        let zipped_stream = leaf_stream.zip(block_stream);

        let (leaf_and_block_pair_sender, leaf_and_block_pair_receiver) = mpsc::channel(10);

        let process_consume_leaves =
            BridgeLeafAndBlockStreamToSenderTask::new(zipped_stream, leaf_and_block_pair_sender);

        let node_validator_task_state = match super::create_node_validator_processing(
            super::NodeValidatorConfig {
                stake_table_url_base: "https://query.main.net.espresso.network/v0"
                    .parse()
                    .unwrap(),
                initial_node_public_base_urls: vec![
                    "https://query-1.main.net.espresso.network/"
                        .parse()
                        .unwrap(),
                    "https://query-2.main.net.espresso.network/"
                        .parse()
                        .unwrap(),
                    "https://query-3.main.net.espresso.network/"
                        .parse()
                        .unwrap(),
                    "https://query-4.main.net.espresso.network/"
                        .parse()
                        .unwrap(),
                ],
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

        // We would like to wait until being signaled
        let app_serve_handle = spawn(async move {
            let app_serve_result = app.serve("0.0.0.0:9000", STATIC_VER_0_1).await;
            tracing::info!("app serve result: {:?}", app_serve_result);
        });
        tracing::info!("now listening on port 9000");

        let _ = app_serve_handle.await;

        drop(node_validator_task_state);
        drop(process_consume_leaves);
    }
}
