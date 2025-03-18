use std::{sync::Arc, time::Duration};

use async_lock::RwLock;
use espresso_types::SeqTypes;
use futures::{
    channel::mpsc::{self, Receiver, SendError, Sender},
    Sink, SinkExt,
};
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
    pub submit_public_urls_handle: Option<SubmitPublicUrlsToScrapeTask>,
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

/// [SubmitPublicUrlsToScrapeTask] is a task that is capable of submitting
/// public urls to a url sender at a regular interval.  This task will
/// submit the provided urls to the url sender every 5 minutes.
pub struct SubmitPublicUrlsToScrapeTask {
    pub task_handle: Option<JoinHandle<()>>,
}

const PUBLIC_URL_RESUBMIT_INTERVAL: Duration = Duration::from_secs(300);

impl SubmitPublicUrlsToScrapeTask {
    pub fn new<S>(url_sender: S, urls: Vec<Url>) -> Self
    where
        S: Sink<Url, Error = SendError> + Send + Unpin + 'static,
    {
        let task_handle = spawn(Self::submit_urls(url_sender, urls));

        Self {
            task_handle: Some(task_handle),
        }
    }

    pub async fn submit_urls<S>(url_sender: S, urls: Vec<Url>)
    where
        S: Sink<Url, Error = SendError> + Unpin + 'static,
    {
        if urls.is_empty() {
            tracing::warn!("no urls to send to url sender");
            return;
        }

        let mut url_sender = url_sender;
        tracing::debug!("sending initial urls to url sender to process node identity");
        loop {
            for url in urls.iter() {
                let send_result = url_sender.send(url.clone()).await;
                if let Err(err) = send_result {
                    tracing::error!("url sender closed: {}", err);
                    panic!("SubmitPublicUrlsToScrapeTask url sender is closed, unrecoverable, the node state will stagnate.");
                }
            }

            // Sleep for 5 minutes before sending the urls again
            tokio::time::sleep(PUBLIC_URL_RESUBMIT_INTERVAL).await;
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
    let (url_sender, url_receiver) = mpsc::channel(32);

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

    // Send any initial URLS to the url sender for immediate processing.
    // These urls are supplied by the configuration of this function
    let submit_public_urls_handle = SubmitPublicUrlsToScrapeTask::new(
        url_sender.clone(),
        config.initial_node_public_base_urls.clone(),
    );

    Ok(NodeValidatorAPI {
        process_internal_client_message_handle: Some(process_internal_client_message_handle),
        process_distribute_block_detail_handle: Some(process_distribute_block_detail_handle),
        process_distribute_node_identity_handle: Some(process_distribute_node_identity_handle),
        process_distribute_voters_handle: Some(process_distribute_voters_handle),
        process_leaf_stream_handle: Some(process_leaf_stream_handle),
        process_node_identity_stream_handle: Some(process_node_identity_stream_handle),
        process_url_stream_handle: Some(process_url_stream_handle),
        submit_public_urls_handle: Some(submit_public_urls_handle),
        url_sender,
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

        let client = surf_disco::Client::builder(
            "https://query.main.net.espresso.network/v0/"
                .parse()
                .unwrap(),
        )
        .content_type(surf_disco::ContentType::Json)
        .build();

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
