use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use async_lock::{RwLock, RwLockWriteGuard};
use bitvec::vec::BitVec;
use espresso_types::SeqTypes;
use futures::{channel::mpsc::SendError, Sink, SinkExt, Stream, StreamExt};
use hotshot_query_service::explorer::{BlockDetail, ExplorerHistograms};
use tokio::{spawn, task::JoinHandle};

use super::{
    client_id::ClientId,
    client_message::{ClientMessage, InternalClientMessage},
    data_state::{DataState, NodeIdentity},
    server_message::ServerMessage,
};

/// ClientState represents the service state of the connected clients.
/// It maintains and represents the connected clients, and their subscriptions.
// This state is meant to be managed in a separate thread that assists with
// processing and updating of individual client states.
pub struct ClientState<K> {
    client_id: ClientId,
    sender: K,
}

impl<K> ClientState<K> {
    /// Create a new ClientState with the given client_id and receiver.
    pub fn new(client_id: ClientId, sender: K) -> Self {
        Self { client_id, sender }
    }

    pub fn client_id(&self) -> ClientId {
        self.client_id
    }

    pub fn sender(&self) -> &K {
        &self.sender
    }
}

/// [ClientThreadState] represents the state of all of the active client
/// connections connected to the service. This state governs which clients
/// are connected, and what subscriptions they have setup.
pub struct ClientThreadState<K> {
    clients: HashMap<ClientId, ClientState<K>>,
    subscribed_latest_block: HashSet<ClientId>,
    subscribed_node_identity: HashSet<ClientId>,
    subscribed_voters: HashSet<ClientId>,
    connection_id_counter: ClientId,
}

impl<K> ClientThreadState<K> {
    pub fn new(
        clients: HashMap<ClientId, ClientState<K>>,
        subscribed_latest_block: HashSet<ClientId>,
        subscribed_node_identity: HashSet<ClientId>,
        subscribed_voters: HashSet<ClientId>,
        connection_id_counter: ClientId,
    ) -> Self {
        Self {
            clients,
            subscribed_latest_block,
            subscribed_node_identity,
            subscribed_voters,
            connection_id_counter,
        }
    }
}

/// [drop_client_client_thread_state_write_guard] is a utility function for
/// cleaning up the [ClientThreadState]
fn drop_client_client_thread_state_write_guard<K>(
    client_id: &ClientId,
    client_thread_state_write_guard: &mut RwLockWriteGuard<ClientThreadState<K>>,
) -> Option<ClientState<K>> {
    let client = client_thread_state_write_guard.clients.remove(client_id);
    client_thread_state_write_guard
        .subscribed_latest_block
        .remove(client_id);
    client_thread_state_write_guard
        .subscribed_node_identity
        .remove(client_id);

    client
}

/// [drop_client_no_lock_guard] is a utility function for cleaning up the [ClientThreadState]
/// when a client is detected as disconnected.
async fn drop_client_no_lock_guard<K>(
    client_id: &ClientId,
    client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
) -> Option<ClientState<K>> {
    let mut client_thread_state_write_lock_guard = client_thread_state.write().await;

    drop_client_client_thread_state_write_guard(
        client_id,
        &mut client_thread_state_write_lock_guard,
    )
}

/// [HandleConnectedError] represents the scope of errors that can be
/// returned from the [handle_client_message_connected] function.
#[derive(Debug)]
pub enum HandleConnectedError {
    ClientSendError(SendError),
}

impl std::fmt::Display for HandleConnectedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandleConnectedError::ClientSendError(err) => {
                write!(f, "handle connected error: client send error: {}", err)
            },
        }
    }
}

impl std::error::Error for HandleConnectedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HandleConnectedError::ClientSendError(err) => Some(err),
        }
    }
}

/// [handle_client_message_connected] is a function that processes the client
/// message to connect a client to the service.
pub async fn handle_client_message_connected<K>(
    mut sender: K,
    client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
) -> Result<ClientId, HandleConnectedError>
where
    K: Sink<ServerMessage, Error = SendError> + Clone + Unpin,
{
    let mut client_thread_state_write_lock_guard = client_thread_state.write().await;

    client_thread_state_write_lock_guard.connection_id_counter += 1;
    let client_id = client_thread_state_write_lock_guard.connection_id_counter;

    client_thread_state_write_lock_guard.clients.insert(
        client_id,
        ClientState {
            client_id,
            sender: sender.clone(),
        },
    );

    // Explicitly unlock
    drop(client_thread_state_write_lock_guard);

    // Send the client their new id.
    if let Err(err) = sender.send(ServerMessage::YouAre(client_id)).await {
        // We need to remove drop the client now.
        drop_client_no_lock_guard(&client_id, client_thread_state.clone()).await;
        return Err(HandleConnectedError::ClientSendError(err));
    }

    Ok(client_id)
}

/// [handle_client_message_disconnected] is a function that processes the client
/// message to disconnect a client from the service.
pub async fn handle_client_message_disconnected<K>(
    client_id: ClientId,
    client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
) {
    // We might receive an implicit disconnect when attempting to
    // send a message, as the receiving channel might be closed.
    drop_client_no_lock_guard(&client_id, client_thread_state.clone()).await;
}

/// [handle_client_message_subscribe_latest_block] is a function that processes
/// the client message to subscribe to the latest block stream.
pub async fn handle_client_message_subscribe_latest_block<K>(
    client_id: ClientId,
    client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
) {
    let mut client_thread_state_write_lock_guard = client_thread_state.write().await;

    client_thread_state_write_lock_guard
        .subscribed_latest_block
        .insert(client_id);

    // Explicitly unlock
    drop(client_thread_state_write_lock_guard);
}

/// [handle_client_message_subscribe_node_identity] is a function that processes
/// the client message to subscribe to the node identity stream.
pub async fn handle_client_message_subscribe_node_identity<K>(
    client_id: ClientId,
    client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
) {
    let mut client_thread_state_write_lock_guard = client_thread_state.write().await;

    client_thread_state_write_lock_guard
        .subscribed_node_identity
        .insert(client_id);

    // Explicitly unlock
    drop(client_thread_state_write_lock_guard);
}

/// [handle_client_message_subscribe_voters] is a function that processes
/// the client message to subscribe to the voters bitvecs.
pub async fn handle_client_message_subscribe_voters<K>(
    client_id: ClientId,
    client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
) {
    let mut client_thread_state_write_lock_guard = client_thread_state.write().await;

    client_thread_state_write_lock_guard
        .subscribed_voters
        .insert(client_id);

    // Explicitly unlock
    drop(client_thread_state_write_lock_guard);
}

/// [HandleRequestBlocksSnapshotsError] represents the scope of errors that can
/// be returned from the [handle_client_message_request_blocks_snapshot] function.
#[derive(Debug)]
pub enum HandleRequestBlocksSnapshotsError {
    ClientSendError(SendError),
}

impl std::fmt::Display for HandleRequestBlocksSnapshotsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandleRequestBlocksSnapshotsError::ClientSendError(err) => {
                write!(
                    f,
                    "handle request blocks snapshot error: client send error:: {}",
                    err
                )
            },
        }
    }
}

impl std::error::Error for HandleRequestBlocksSnapshotsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HandleRequestBlocksSnapshotsError::ClientSendError(err) => Some(err),
        }
    }
}

/// [handle_client_message_request_blocks_snapshot] is a function that processes
/// the client message request for a blocks snapshot.
pub async fn handle_client_message_request_blocks_snapshot<K>(
    client_id: ClientId,
    data_state: Arc<RwLock<DataState>>,
    client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
) -> Result<(), HandleRequestBlocksSnapshotsError>
where
    K: Sink<ServerMessage, Error = SendError> + Clone + Unpin,
{
    let (client_thread_state_read_lock_guard, data_state_read_lock_guard) =
        futures::join!(client_thread_state.read(), data_state.read());

    let latest_blocks = data_state_read_lock_guard
        .latest_blocks()
        .map(|block| BlockDetail {
            hash: block.hash,
            proposer_id: block.proposer_id.clone(),
            height: block.height,
            size: block.size,
            time: block.time,
            num_transactions: block.num_transactions,
            fee_recipient: block.fee_recipient.clone(),
            block_reward: block.block_reward.clone(),
        })
        .collect::<Vec<BlockDetail<SeqTypes>>>();

    if let Some(client) = client_thread_state_read_lock_guard.clients.get(&client_id) {
        let mut sender = client.sender.clone();
        if let Err(err) = sender
            .send(ServerMessage::BlocksSnapshot(Arc::new(latest_blocks)))
            .await
        {
            drop_client_no_lock_guard(&client_id, client_thread_state.clone()).await;
            return Err(HandleRequestBlocksSnapshotsError::ClientSendError(err));
        }
    }

    Ok(())
}

/// [HandleRequestNodeIdentitySnapshotError] represents the scope of errors that
/// can be returned from the [handle_client_message_request_node_identity_snapshot]
/// function.
#[derive(Debug)]
pub enum HandleRequestNodeIdentitySnapshotError {
    ClientSendError(SendError),
}

impl std::fmt::Display for HandleRequestNodeIdentitySnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandleRequestNodeIdentitySnapshotError::ClientSendError(err) => {
                write!(
                    f,
                    "handle request node identity snapshot error: client send error: {}",
                    err
                )
            },
        }
    }
}

impl std::error::Error for HandleRequestNodeIdentitySnapshotError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HandleRequestNodeIdentitySnapshotError::ClientSendError(err) => Some(err),
        }
    }
}

/// [handle_client_message_request_node_identity_snapshot] is a function that
/// processes the client message request for a node identity snapshot.
pub async fn handle_client_message_request_node_identity_snapshot<K>(
    client_id: ClientId,
    data_state: Arc<RwLock<DataState>>,
    client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
) -> Result<(), HandleRequestNodeIdentitySnapshotError>
where
    K: Sink<ServerMessage, Error = SendError> + Clone + Unpin,
{
    // Let's send the current Blocks Snapshot to the client
    let (client_thread_state_read_lock_guard, data_state_read_lock_guard) =
        futures::join!(client_thread_state.read(), data_state.read());
    let client_result = client_thread_state_read_lock_guard.clients.get(&client_id);
    if let Some(client) = client_result {
        let mut sender = client.sender.clone();

        // Let's copy the current node identity snapshot and send them
        let nodes = data_state_read_lock_guard
            .node_identity()
            .cloned()
            .collect::<Vec<_>>();

        if let Err(err) = sender
            .send(ServerMessage::NodeIdentitySnapshot(Arc::new(nodes)))
            .await
        {
            drop(client_thread_state_read_lock_guard);
            drop_client_no_lock_guard(&client_id, client_thread_state.clone()).await;
            return Err(HandleRequestNodeIdentitySnapshotError::ClientSendError(err));
        }

        return Ok(());
    }

    Ok(())
}

/// [HandleRequestHistogramSnapshotError] represents the scope of errors that
/// can be returned from the [handle_client_message_request_histogram_snapshot]
/// function.
#[derive(Debug)]
pub enum HandleRequestHistogramSnapshotError {
    ClientSendError(SendError),
}

impl std::fmt::Display for HandleRequestHistogramSnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandleRequestHistogramSnapshotError::ClientSendError(err) => {
                write!(
                    f,
                    "handle request histogram snapshot error: client send error: {}",
                    err
                )
            },
        }
    }
}

impl std::error::Error for HandleRequestHistogramSnapshotError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HandleRequestHistogramSnapshotError::ClientSendError(err) => Some(err),
        }
    }
}

/// [handle_client_message_request_histogram_snapshot] is a function that
/// processes the client message request for a histogram snapshot.
pub async fn handle_client_message_request_histogram_snapshot<K>(
    client_id: ClientId,
    data_state: Arc<RwLock<DataState>>,
    client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
) -> Result<(), HandleRequestHistogramSnapshotError>
where
    K: Sink<ServerMessage, Error = SendError> + Clone + Unpin,
{
    // Let's send the current histogram data snapshot to the client
    let (client_thread_state_read_lock_guard, data_state_read_lock_guard) =
        futures::join!(client_thread_state.read(), data_state.read());

    let histogram_data = ExplorerHistograms {
        block_size: data_state_read_lock_guard
            .latest_blocks()
            .skip(1)
            .map(|block| Some(block.size))
            .collect(),
        block_time: data_state_read_lock_guard
            .latest_blocks()
            .skip(1)
            .zip(data_state_read_lock_guard.latest_blocks())
            .map(|(block_i, block_i_sub_1)| {
                Some((block_i.time.0 - block_i_sub_1.time.0).whole_seconds() as u64)
            })
            .collect(),
        block_transactions: data_state_read_lock_guard
            .latest_blocks()
            .skip(1)
            .map(|block| block.num_transactions)
            .collect(),
        block_heights: data_state_read_lock_guard
            .latest_blocks()
            .skip(1)
            .map(|block| block.height)
            .collect(),
    };
    let arc_histogram_data = Arc::new(histogram_data);
    drop(data_state_read_lock_guard);

    if let Some(client) = client_thread_state_read_lock_guard.clients.get(&client_id) {
        let mut sender = client.sender.clone();
        drop(client_thread_state_read_lock_guard);

        if let Err(err) = sender
            .send(ServerMessage::HistogramSnapshot(arc_histogram_data))
            .await
        {
            drop_client_no_lock_guard(&client_id, client_thread_state.clone()).await;
            return Err(HandleRequestHistogramSnapshotError::ClientSendError(err));
        }

        return Ok(());
    }

    Ok(())
}

#[derive(Debug)]
pub enum HandleRequestVotersSnapshotError {
    ClientSendError(SendError),
}

impl std::fmt::Display for HandleRequestVotersSnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandleRequestVotersSnapshotError::ClientSendError(err) => {
                write!(
                    f,
                    "handle request voters snapshot error: client send error: {}",
                    err
                )
            },
        }
    }
}

impl std::error::Error for HandleRequestVotersSnapshotError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HandleRequestVotersSnapshotError::ClientSendError(err) => Some(err),
        }
    }
}

/// [handle_client_message_request_voters_snapshot] is a function that processes
/// the client message request for a voters snapshot.
pub async fn handle_client_message_request_voters_snapshot<K>(
    client_id: ClientId,
    data_state: Arc<RwLock<DataState>>,
    client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
) -> Result<(), HandleRequestVotersSnapshotError>
where
    K: Sink<ServerMessage, Error = SendError> + Clone + Unpin,
{
    let (client_thread_state_read_lock_guard, data_state_read_lock_guard) =
        futures::join!(client_thread_state.read(), data_state.read());

    let voters_data = data_state_read_lock_guard
        .latest_voters()
        .cloned()
        .collect::<Vec<_>>();

    let voters_data = Arc::new(voters_data);

    if let Some(client) = client_thread_state_read_lock_guard.clients.get(&client_id) {
        let mut sender = client.sender.clone();
        drop(client_thread_state_read_lock_guard);

        if let Err(err) = sender
            .send(ServerMessage::VotersSnapshot(voters_data.clone()))
            .await
        {
            drop_client_no_lock_guard(&client_id, client_thread_state.clone()).await;
            return Err(HandleRequestVotersSnapshotError::ClientSendError(err));
        }

        return Ok(());
    }
    Ok(())
}

/// [ProcessClientMessageError] represents the scope of errors that can be
/// returned from the [process_client_message] function.
#[derive(Debug)]
pub enum ProcessClientMessageError {
    Connected(HandleConnectedError),
    BlocksSnapshot(HandleRequestBlocksSnapshotsError),
    NodeIdentitySnapshot(HandleRequestNodeIdentitySnapshotError),
    HistogramSnapshot(HandleRequestHistogramSnapshotError),
    VotersSnapshot(HandleRequestVotersSnapshotError),
}

impl From<HandleConnectedError> for ProcessClientMessageError {
    fn from(err: HandleConnectedError) -> Self {
        ProcessClientMessageError::Connected(err)
    }
}

impl From<HandleRequestBlocksSnapshotsError> for ProcessClientMessageError {
    fn from(err: HandleRequestBlocksSnapshotsError) -> Self {
        ProcessClientMessageError::BlocksSnapshot(err)
    }
}

impl From<HandleRequestNodeIdentitySnapshotError> for ProcessClientMessageError {
    fn from(err: HandleRequestNodeIdentitySnapshotError) -> Self {
        ProcessClientMessageError::NodeIdentitySnapshot(err)
    }
}

impl From<HandleRequestHistogramSnapshotError> for ProcessClientMessageError {
    fn from(err: HandleRequestHistogramSnapshotError) -> Self {
        ProcessClientMessageError::HistogramSnapshot(err)
    }
}

impl From<HandleRequestVotersSnapshotError> for ProcessClientMessageError {
    fn from(err: HandleRequestVotersSnapshotError) -> Self {
        ProcessClientMessageError::VotersSnapshot(err)
    }
}

impl std::fmt::Display for ProcessClientMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessClientMessageError::Connected(err) => {
                write!(f, "process client message error: connected: {}", err)
            },
            ProcessClientMessageError::BlocksSnapshot(err) => {
                write!(f, "process client message error: blocks snapshot: {}", err)
            },
            ProcessClientMessageError::NodeIdentitySnapshot(err) => {
                write!(
                    f,
                    "process client message error: node identity snapshot: {}",
                    err
                )
            },
            ProcessClientMessageError::HistogramSnapshot(err) => {
                write!(
                    f,
                    "process client message error: histogram snapshot: {}",
                    err
                )
            },
            ProcessClientMessageError::VotersSnapshot(err) => {
                write!(f, "process client message error: voters snapshot: {}", err)
            },
        }
    }
}

impl std::error::Error for ProcessClientMessageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProcessClientMessageError::Connected(err) => Some(err),
            ProcessClientMessageError::BlocksSnapshot(err) => Some(err),
            ProcessClientMessageError::NodeIdentitySnapshot(err) => Some(err),
            ProcessClientMessageError::HistogramSnapshot(err) => Some(err),
            ProcessClientMessageError::VotersSnapshot(err) => Some(err),
        }
    }
}

/// [process_client_message] is a function that processes the client message
/// and processes the message accordingly.
///
/// The [DataState] is provided and is used only as a Read lock to distribute
/// the current state of the system to the clients upon request.
///
/// The [ClientThreadState] is provided as it needs to be updated with new
/// subscriptions / new connections depending on the incoming
/// [InternalClientMessage]
pub async fn process_client_message<K>(
    message: InternalClientMessage<K>,
    data_state: Arc<RwLock<DataState>>,
    client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
) -> Result<(), ProcessClientMessageError>
where
    K: Sink<ServerMessage, Error = SendError> + Clone + Unpin,
{
    match message {
        InternalClientMessage::Connected(sender) => {
            handle_client_message_connected(sender, client_thread_state).await?;
            Ok(())
        },

        InternalClientMessage::Disconnected(client_id) => {
            handle_client_message_disconnected(client_id, client_thread_state).await;
            Ok(())
        },

        InternalClientMessage::Request(client_id, ClientMessage::SubscribeLatestBlock) => {
            handle_client_message_subscribe_latest_block(client_id, client_thread_state).await;
            Ok(())
        },

        InternalClientMessage::Request(client_id, ClientMessage::SubscribeNodeIdentity) => {
            handle_client_message_subscribe_node_identity(client_id, client_thread_state).await;
            Ok(())
        },

        InternalClientMessage::Request(client_id, ClientMessage::SubscribeVoters) => {
            handle_client_message_subscribe_voters(client_id, client_thread_state).await;
            Ok(())
        },

        InternalClientMessage::Request(client_id, ClientMessage::RequestBlocksSnapshot) => {
            handle_client_message_request_blocks_snapshot(
                client_id,
                data_state,
                client_thread_state,
            )
            .await?;
            Ok(())
        },

        InternalClientMessage::Request(client_id, ClientMessage::RequestNodeIdentitySnapshot) => {
            handle_client_message_request_node_identity_snapshot(
                client_id,
                data_state,
                client_thread_state,
            )
            .await?;
            Ok(())
        },

        InternalClientMessage::Request(client_id, ClientMessage::RequestHistogramSnapshot) => {
            handle_client_message_request_histogram_snapshot(
                client_id,
                data_state,
                client_thread_state,
            )
            .await?;
            Ok(())
        },

        InternalClientMessage::Request(client_id, ClientMessage::RequestVotersSnapshot) => {
            handle_client_message_request_voters_snapshot(
                client_id,
                data_state,
                client_thread_state,
            )
            .await?;
            Ok(())
        },
    }
}

/// [drop_failed_client_sends] is a function that will drop all of the failed
/// client sends from the client thread state.
async fn drop_failed_client_sends<K>(
    client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
    failed_client_sends: Vec<ClientId>,
) {
    // Let's acquire our write lock
    let mut client_thread_state_write_lock_guard = client_thread_state.write().await;

    // We want to drop all of the failed clients.
    // There's an optimization to be had here
    for client_id in failed_client_sends {
        drop_client_client_thread_state_write_guard(
            &client_id,
            &mut client_thread_state_write_lock_guard,
        );
    }
}

/// [handle_received_block_detail] is a function that processes received Block
/// details and will attempt to distribute the message to all of the clients
/// that are subscribed to the latest block stream.
async fn handle_received_block_detail<K>(
    client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
    block_detail: BlockDetail<SeqTypes>,
) where
    K: Sink<ServerMessage, Error = SendError> + Clone + Unpin,
{
    let client_thread_state_read_lock_guard = client_thread_state.read().await;

    // These are the clients who are subscribed to the latest blocks, that
    // have an active ClientState within the system.
    let latest_block_subscribers = client_thread_state_read_lock_guard
        .subscribed_latest_block
        .iter()
        .map(|client_id| {
            (
                client_id,
                client_thread_state_read_lock_guard.clients.get(client_id),
            )
        })
        .filter(|(_, client)| client.is_some());

    let arc_block_detail = Arc::new(block_detail);
    // We collect the results of sending the latest block to the clients.
    let client_send_result_future = latest_block_subscribers.map(|(client_id, client)| {
        let arc_block_detail = arc_block_detail.clone();
        async move {
            // This is guaranteed to be a some now
            let client = client.unwrap();
            let mut sender = client.sender.clone();
            let send_result = sender
                .send(ServerMessage::LatestBlock(arc_block_detail))
                .await;

            (client_id, send_result)
        }
    });

    let client_send_results = futures::future::join_all(client_send_result_future).await;

    // These are the clients we failed to send the message to.  We copy these
    // here so we can drop our read lock.
    let failed_client_sends = client_send_results
        .into_iter()
        .filter(|(_, send_result)| send_result.is_err())
        .map(|(client_id, _)| *client_id)
        .collect::<Vec<_>>();

    // Explicitly Drop the read lock.
    drop(client_thread_state_read_lock_guard);

    if failed_client_sends.is_empty() {
        return;
    }

    drop_failed_client_sends(client_thread_state, failed_client_sends).await;
}

/// [handle_received_node_identity] is a function that processes received
/// NodeIdentity and will attempt to distribute the message to all of the
/// clients that are subscribed to the node identity stream.
async fn handle_received_node_identity<K>(
    client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
    node_identity: NodeIdentity,
) where
    K: Sink<ServerMessage, Error = SendError> + Clone + Unpin,
{
    let client_thread_state_read_lock_guard = client_thread_state.read().await;

    // These are the clients who are subscribed to the node identities, that
    // have an active ClientState within the system.
    let node_identity_subscribers = client_thread_state_read_lock_guard
        .subscribed_node_identity
        .iter()
        .map(|client_id| {
            (
                client_id,
                client_thread_state_read_lock_guard.clients.get(client_id),
            )
        })
        .filter(|(_, client)| client.is_some());

    let arc_node_identity = Arc::new(node_identity);
    // We collect the results of sending the latest block to the clients.
    let client_send_result_future = node_identity_subscribers.map(|(client_id, client)| {
        let arc_node_identity = arc_node_identity.clone();
        async move {
            // This is guaranteed to be a some now
            let client = client.unwrap();
            let mut sender = client.sender.clone();
            let send_result = sender
                .send(ServerMessage::LatestNodeIdentity(arc_node_identity.clone()))
                .await;

            (client_id, send_result)
        }
    });

    let client_send_results = futures::future::join_all(client_send_result_future).await;

    // These are the clients we failed to send the message to.  We copy these
    // here so we can drop our read lock.
    let failed_client_sends = client_send_results
        .into_iter()
        .filter(|(_, send_result)| send_result.is_err())
        .map(|(client_id, _)| *client_id)
        .collect::<Vec<_>>();

    // Explicitly Drop the read lock.
    drop(client_thread_state_read_lock_guard);

    if failed_client_sends.is_empty() {
        return;
    }

    drop_failed_client_sends(client_thread_state, failed_client_sends).await;
}

/// [handle_received_voters] is a function that processes received voters and
/// will attempt to distribute the message to all of the clients that are
/// subscribed to the voters stream.
async fn handle_received_voters<K>(
    client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
    voters: BitVec<u16>,
) where
    K: Sink<ServerMessage, Error = SendError> + Clone + Unpin,
{
    let client_thread_state_read_lock_guard = client_thread_state.read().await;

    // These are the clients who are subscribed to the node identities, that
    // have an active ClientState within the system.
    let node_identity_subscribers = client_thread_state_read_lock_guard
        .subscribed_voters
        .iter()
        .map(|client_id| {
            (
                client_id,
                client_thread_state_read_lock_guard.clients.get(client_id),
            )
        })
        .filter(|(_, client)| client.is_some());

    // We collect the results of sending the latest block to the clients.
    let client_send_result_future = node_identity_subscribers.map(|(client_id, client)| {
        let voters = voters.clone();
        async move {
            // This is guaranteed to be a some now
            let client = client.unwrap();
            let mut sender = client.sender.clone();
            let send_result = sender.send(ServerMessage::LatestVoters(voters)).await;

            (client_id, send_result)
        }
    });

    let client_send_results = futures::future::join_all(client_send_result_future).await;

    // These are the clients we failed to send the message to.  We copy these
    // here so we can drop our read lock.
    let failed_client_sends = client_send_results
        .into_iter()
        .filter(|(_, send_result)| send_result.is_err())
        .map(|(client_id, _)| *client_id)
        .collect::<Vec<_>>();

    // Explicitly Drop the read lock.
    drop(client_thread_state_read_lock_guard);

    if failed_client_sends.is_empty() {
        return;
    }

    drop_failed_client_sends(client_thread_state, failed_client_sends).await;
}

/// InternalClientMessageProcessingTask represents an async task for
/// InternalClientMessages, and making the appropriate updates to the
/// [ClientThreadState] and [DataState].
pub struct InternalClientMessageProcessingTask {
    pub task_handle: Option<JoinHandle<()>>,
}

impl InternalClientMessageProcessingTask {
    /// new creates a new [InternalClientMessageProcessingTask] with the
    /// given internal_client_message_receiver, data_state, and
    /// client_thread_state.
    ///
    /// Calling this function will start an async task that will start
    /// processing.  The handle for the async task is stored within the
    /// returned state.
    pub fn new<S, K>(
        internal_client_message_receiver: S,
        data_state: Arc<RwLock<DataState>>,
        client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
    ) -> Self
    where
        S: Stream<Item = InternalClientMessage<K>> + Send + Sync + Unpin + 'static,
        K: Sink<ServerMessage, Error = SendError> + Clone + Send + Sync + Unpin + 'static,
    {
        let task_handle = spawn(Self::process_internal_client_message_stream(
            internal_client_message_receiver,
            data_state.clone(),
            client_thread_state.clone(),
        ));

        Self {
            task_handle: Some(task_handle),
        }
    }

    /// [process_internal_client_message_stream] is a function that processes the
    /// client handling stream. This stream is responsible for managing the state
    /// of the connected clients, and their subscriptions.
    async fn process_internal_client_message_stream<S, K>(
        mut stream: S,
        data_state: Arc<RwLock<DataState>>,
        client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
    ) where
        S: Stream<Item = InternalClientMessage<K>> + Unpin,
        K: Sink<ServerMessage, Error = SendError> + Clone + Unpin,
    {
        loop {
            let message_result = stream.next().await;
            let message = if let Some(message) = message_result {
                message
            } else {
                tracing::error!("internal client message handler closed.");
                panic!("InternalClientMessageProcessingTask stream closed, unable to process new requests from clients.");
            };

            if let Err(err) =
                process_client_message(message, data_state.clone(), client_thread_state.clone())
                    .await
            {
                // We log this error, but we ignore it so that other connections
                // are not affected by a single client.
                tracing::info!(
                    "internal client message processing encountered an error: {}",
                    err,
                );
            }
        }
    }
}

/// [drop] implementation for [InternalClientMessageProcessingTask] that will
/// cancel the task if it is still running.
impl Drop for InternalClientMessageProcessingTask {
    fn drop(&mut self) {
        let task_handle = self.task_handle.take();
        if let Some(task_handle) = task_handle {
            task_handle.abort();
        }
    }
}

/// [ProcessDistributeBlockDetailHandlingTask] represents an async task for
/// processing the incoming [BlockDetail] and distributing them to all
/// subscribed clients.
pub struct ProcessDistributeBlockDetailHandlingTask {
    pub task_handle: Option<JoinHandle<()>>,
}

impl ProcessDistributeBlockDetailHandlingTask {
    /// [new] creates a new [ProcessDistributeBlockDetailHandlingTask] with the
    /// given client_thread_state and block_detail_receiver.
    ///
    /// Calling this function will start an async task that will start
    /// processing.  The handle for the async task is stored within the
    /// returned state.
    pub fn new<S, K>(
        client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
        block_detail_receiver: S,
    ) -> Self
    where
        S: Stream<Item = BlockDetail<SeqTypes>> + Send + Sync + Unpin + 'static,
        K: Sink<ServerMessage, Error = SendError> + Clone + Send + Sync + Unpin + 'static,
    {
        let task_handle = spawn(Self::process_distribute_block_detail_handling_stream(
            client_thread_state.clone(),
            block_detail_receiver,
        ));

        Self {
            task_handle: Some(task_handle),
        }
    }

    /// [process_distribute_block_detail_handling_stream] is a function that
    /// processes the [Stream] of incoming [BlockDetail] and distributes them
    /// to all subscribed clients.
    async fn process_distribute_block_detail_handling_stream<S, K>(
        client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
        mut stream: S,
    ) where
        S: Stream<Item = BlockDetail<SeqTypes>> + Unpin,
        K: Sink<ServerMessage, Error = SendError> + Clone + Unpin,
    {
        loop {
            let block_detail_result = stream.next().await;

            let block_detail = if let Some(block_detail) = block_detail_result {
                block_detail
            } else {
                tracing::error!(
                    "block detail stream closed.  shutting down client handling stream.",
                );
                return;
            };

            handle_received_block_detail(client_thread_state.clone(), block_detail).await
        }
    }
}

/// [drop] implementation for [ProcessDistributeBlockDetailHandlingTask] that will
/// cancel the task if it is still running.
impl Drop for ProcessDistributeBlockDetailHandlingTask {
    fn drop(&mut self) {
        let task_handle = self.task_handle.take();
        if let Some(task_handle) = task_handle {
            task_handle.abort();
        }
    }
}

/// [ProcessDistributeNodeIdentityHandlingTask] represents an async task for
/// processing the incoming [NodeIdentity] and distributing them to all
/// subscribed clients.
pub struct ProcessDistributeNodeIdentityHandlingTask {
    pub task_handle: Option<JoinHandle<()>>,
}

impl ProcessDistributeNodeIdentityHandlingTask {
    /// [new] creates a new [ProcessDistributeNodeIdentityHandlingTask] with the
    /// given client_thread_state and node_identity_receiver.
    ///
    /// Calling this function will start an async task that will start
    /// processing.  The handle for the async task is stored within the
    /// returned state.
    pub fn new<S, K>(
        client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
        node_identity_receiver: S,
    ) -> Self
    where
        S: Stream<Item = NodeIdentity> + Send + Sync + Unpin + 'static,
        K: Sink<ServerMessage, Error = SendError> + Clone + Send + Sync + Unpin + 'static,
    {
        let task_handle = spawn(Self::process_distribute_node_identity_handling_stream(
            client_thread_state.clone(),
            node_identity_receiver,
        ));

        Self {
            task_handle: Some(task_handle),
        }
    }

    /// [process_distribute_node_identity_handling_stream] is a function that
    /// processes the [Stream] of incoming [NodeIdentity] and distributes them
    /// to all subscribed clients.
    async fn process_distribute_node_identity_handling_stream<S, K>(
        client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
        mut stream: S,
    ) where
        S: Stream<Item = NodeIdentity> + Unpin,
        K: Sink<ServerMessage, Error = SendError> + Clone + Unpin,
    {
        loop {
            let node_identity_result = stream.next().await;

            let node_identity = if let Some(node_identity) = node_identity_result {
                node_identity
            } else {
                tracing::error!(
                    "node identity stream closed.  shutting down client handling stream.",
                );
                return;
            };

            handle_received_node_identity(client_thread_state.clone(), node_identity).await
        }
    }
}

/// [drop] implementation for [ProcessDistributeNodeIdentityHandlingTask] that
/// will cancel the task if it is still running.
impl Drop for ProcessDistributeNodeIdentityHandlingTask {
    fn drop(&mut self) {
        let task_handle = self.task_handle.take();
        if let Some(task_handle) = task_handle {
            task_handle.abort();
        }
    }
}

/// [ProcessDistributeVotersHandlingTask] represents an async task for
/// processing the incoming [BitVec] and distributing them to all
/// subscribed clients.
pub struct ProcessDistributeVotersHandlingTask {
    pub task_handle: Option<JoinHandle<()>>,
}

impl ProcessDistributeVotersHandlingTask {
    /// [new] creates a new [ProcessDistributeVotersHandlingTask] with the
    /// given client_thread_state and voters_receiver.
    ///
    /// Calling this function will start an async task that will start
    /// processing.  The handle for the async task is stored within the
    /// returned state.
    pub fn new<S, K>(
        client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
        voters_receiver: S,
    ) -> Self
    where
        S: Stream<Item = BitVec<u16>> + Send + Sync + Unpin + 'static,
        K: Sink<ServerMessage, Error = SendError> + Clone + Send + Sync + Unpin + 'static,
    {
        let task_handle = spawn(Self::process_distribute_voters_handling_stream(
            client_thread_state.clone(),
            voters_receiver,
        ));

        Self {
            task_handle: Some(task_handle),
        }
    }

    /// [process_distribute_voters_handling_stream] is a function that processes
    /// the [Stream] of incoming [BitVec] and distributes them to all
    /// subscribed clients.
    async fn process_distribute_voters_handling_stream<S, K>(
        client_thread_state: Arc<RwLock<ClientThreadState<K>>>,
        mut stream: S,
    ) where
        S: Stream<Item = BitVec<u16>> + Unpin,
        K: Sink<ServerMessage, Error = SendError> + Clone + Unpin,
    {
        loop {
            let voters_result = stream.next().await;

            let voters = if let Some(voters) = voters_result {
                voters
            } else {
                tracing::error!("voters stream closed.  shutting down client handling stream.",);
                return;
            };

            handle_received_voters(client_thread_state.clone(), voters).await
        }
    }
}

/// [drop] implementation for [ProcessDistributeVotersHandlingTask] that will
/// cancel the task if it is still running.
impl Drop for ProcessDistributeVotersHandlingTask {
    fn drop(&mut self) {
        let task_handle = self.task_handle.take();
        if let Some(task_handle) = task_handle {
            task_handle.abort();
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::{sync::Arc, time::Duration};

    use async_lock::RwLock;
    use bitvec::vec::BitVec;
    use espresso_types::{NodeState, ValidatedState};
    use futures::{
        channel::mpsc::{self, Sender},
        SinkExt, StreamExt,
    };
    use hotshot_example_types::node_types::TestVersions;
    use hotshot_query_service::{
        availability::{BlockQueryData, Leaf1QueryData},
        testing::mocks::MockVersions,
    };
    use hotshot_types::{
        data::Leaf2, signature_key::BLSPubKey, traits::signature_key::SignatureKey,
    };
    use tokio::{
        spawn,
        time::{sleep, timeout},
    };

    use super::{ClientThreadState, InternalClientMessageProcessingTask};
    use crate::{
        api::node_validator::v0::PublicHotShotConfig,
        service::{
            client_id::ClientId,
            client_message::{ClientMessage, InternalClientMessage},
            client_state::{
                ProcessDistributeBlockDetailHandlingTask,
                ProcessDistributeNodeIdentityHandlingTask, ProcessDistributeVotersHandlingTask,
            },
            data_state::{
                create_block_detail_from_block, DataState, LocationDetails, NodeIdentity,
                ProcessLeafAndBlockPairStreamTask,
            },
            server_message::ServerMessage,
        },
    };

    pub fn create_test_client_thread_state() -> ClientThreadState<Sender<ServerMessage>> {
        ClientThreadState {
            clients: Default::default(),
            subscribed_latest_block: Default::default(),
            subscribed_node_identity: Default::default(),
            subscribed_voters: Default::default(),
            connection_id_counter: ClientId::from_count(1),
        }
    }

    pub fn create_test_data_state() -> (NodeIdentity, NodeIdentity, NodeIdentity, DataState) {
        let node_1 = {
            let (pub_key, _) = BLSPubKey::generated_from_seed_indexed([0; 32], 0);
            NodeIdentity::new(
                pub_key,
                Some("a".to_string()),
                Some("http://localhost/".parse().unwrap()),
                Some("company".to_string()),
                Some("https://example.com/".parse().unwrap()),
                Some(LocationDetails::new(
                    Some((0.0, 0.0)),
                    Some("US".to_string()),
                )),
                Some("Windows 11".to_string()),
                Some("espresso".to_string()),
                Some("residential".to_string()),
            )
        };

        let node_2 = {
            let (pub_key, _) = BLSPubKey::generated_from_seed_indexed([0; 32], 1);
            NodeIdentity::new(
                pub_key,
                Some("b".to_string()),
                Some("http://localhost/".parse().unwrap()),
                Some("company".to_string()),
                Some("https://example.com/".parse().unwrap()),
                Some(LocationDetails::new(
                    Some((0.0, 0.0)),
                    Some("US".to_string()),
                )),
                Some("Windows 11".to_string()),
                Some("espresso".to_string()),
                Some("residential".to_string()),
            )
        };

        let node_3 = {
            let (pub_key, _) = BLSPubKey::generated_from_seed_indexed([0; 32], 2);
            NodeIdentity::new(
                pub_key,
                Some("b".to_string()),
                Some("http://localhost/".parse().unwrap()),
                Some("company".to_string()),
                Some("https://example.com/".parse().unwrap()),
                Some(LocationDetails::new(
                    Some((0.0, 0.0)),
                    Some("US".to_string()),
                )),
                Some("Windows 11".to_string()),
                Some("espresso".to_string()),
                Some("residential".to_string()),
            )
        };

        let mut data_state: DataState = Default::default();
        data_state.add_node_identity(node_1.clone());
        data_state.add_node_identity(node_2.clone());
        data_state.add_node_identity(node_3.clone());

        (node_1, node_2, node_3, data_state)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_client_handling_stream_task_shutdown() {
        let (_, _, _, data_state) = create_test_data_state();
        let client_thread_state = Arc::new(RwLock::new(create_test_client_thread_state()));
        let data_state = Arc::new(RwLock::new(data_state));

        let (_internal_client_message_sender, internal_client_message_receiver) = mpsc::channel(1);
        let _process_internal_client_message_handle = InternalClientMessageProcessingTask::new(
            internal_client_message_receiver,
            data_state,
            client_thread_state,
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_process_client_handling_stream_request_latest_voters_snapshot() {
        let (_, _, _, mut data_state) = create_test_data_state();
        let client_thread_state = Arc::new(RwLock::new(create_test_client_thread_state()));
        let voters_1 = BitVec::from_vec(vec![0x55]);
        let voters_2 = BitVec::from_vec(vec![0xAA]);
        data_state.add_latest_voters(voters_1.clone());
        data_state.add_latest_voters(voters_2.clone());

        let data_state = Arc::new(RwLock::new(data_state));

        let (internal_client_message_sender, internal_client_message_receiver) = mpsc::channel(1);
        let (server_message_sender_1, mut server_message_receiver_1) = mpsc::channel(1);
        let (server_message_sender_2, mut server_message_receiver_2) = mpsc::channel(1);
        let mut process_internal_client_message_handle = InternalClientMessageProcessingTask::new(
            internal_client_message_receiver,
            data_state,
            client_thread_state,
        );

        // Send a Connected Message to the server
        let mut internal_client_message_sender_1 = internal_client_message_sender.clone();
        assert_eq!(
            internal_client_message_sender_1
                .send(InternalClientMessage::Connected(server_message_sender_1))
                .await,
            Ok(())
        );

        assert_eq!(
            server_message_receiver_1.next().await,
            Some(ServerMessage::YouAre(ClientId::from_count(2))),
        );

        let client_1_id = ClientId::from_count(2);

        let mut internal_client_message_sender_2 = internal_client_message_sender;
        assert_eq!(
            internal_client_message_sender_2
                .send(InternalClientMessage::Connected(server_message_sender_2))
                .await,
            Ok(())
        );

        assert_eq!(
            server_message_receiver_2.next().await,
            Some(ServerMessage::YouAre(ClientId::from_count(3))),
        );

        assert_eq!(
            internal_client_message_sender_1
                .send(InternalClientMessage::Request(
                    client_1_id,
                    ClientMessage::RequestVotersSnapshot
                ))
                .await,
            Ok(()),
        );

        assert_eq!(
            server_message_receiver_1.next().await,
            Some(ServerMessage::VotersSnapshot(Arc::new(vec![
                voters_1, voters_2
            ]))),
        );

        if let Some(task_handle) = process_internal_client_message_handle.task_handle.take() {
            task_handle.abort();
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    #[cfg(feature = "testing")]
    async fn test_process_client_handling_stream_request_latest_blocks_snapshot() {
        use crate::service::data_state::create_block_detail_from_block;

        let (_, _, _, mut data_state) = create_test_data_state();
        let client_thread_state = Arc::new(RwLock::new(create_test_client_thread_state()));
        let validated_state = ValidatedState::default();
        let instance_state = NodeState::mock();
        let block_1 =
            BlockQueryData::genesis::<MockVersions>(&validated_state, &instance_state).await;
        data_state.add_latest_block(create_block_detail_from_block(&block_1));

        let data_state = Arc::new(RwLock::new(data_state));

        let (internal_client_message_sender, internal_client_message_receiver) = mpsc::channel(1);
        let (server_message_sender_1, mut server_message_receiver_1) = mpsc::channel(1);
        let (server_message_sender_2, mut server_message_receiver_2) = mpsc::channel(1);
        let mut process_internal_client_message_handle = InternalClientMessageProcessingTask::new(
            internal_client_message_receiver,
            data_state,
            client_thread_state,
        );

        // Send a Connected Message to the server
        let mut internal_client_message_sender_1 = internal_client_message_sender.clone();
        assert_eq!(
            internal_client_message_sender_1
                .send(InternalClientMessage::Connected(server_message_sender_1))
                .await,
            Ok(())
        );

        assert_eq!(
            server_message_receiver_1.next().await,
            Some(ServerMessage::YouAre(ClientId::from_count(2))),
        );

        let client_1_id = ClientId::from_count(2);

        let mut internal_client_message_sender_2 = internal_client_message_sender;
        assert_eq!(
            internal_client_message_sender_2
                .send(InternalClientMessage::Connected(server_message_sender_2))
                .await,
            Ok(()),
        );

        assert_eq!(
            server_message_receiver_2.next().await,
            Some(ServerMessage::YouAre(ClientId::from_count(3))),
        );

        assert_eq!(
            internal_client_message_sender_1
                .send(InternalClientMessage::Request(
                    client_1_id,
                    ClientMessage::RequestBlocksSnapshot
                ))
                .await,
            Ok(()),
        );

        assert_eq!(
            server_message_receiver_1.next().await,
            Some(ServerMessage::BlocksSnapshot(Arc::new(vec![
                create_block_detail_from_block(&block_1)
            ]))),
        );

        if let Some(process_internal_client_message_handle) =
            process_internal_client_message_handle.task_handle.take()
        {
            process_internal_client_message_handle.abort();
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_process_client_handling_stream_request_node_identity_snapshot() {
        let (node_1, node_2, node_3, data_state) = create_test_data_state();
        let client_thread_state = Arc::new(RwLock::new(create_test_client_thread_state()));
        let data_state = Arc::new(RwLock::new(data_state));

        let (internal_client_message_sender, internal_client_message_receiver) = mpsc::channel(1);
        let (server_message_sender_1, mut server_message_receiver_1) = mpsc::channel(1);
        let (server_message_sender_2, mut server_message_receiver_2) = mpsc::channel(1);
        let mut process_internal_client_message_handle = InternalClientMessageProcessingTask::new(
            internal_client_message_receiver,
            data_state,
            client_thread_state,
        );

        // Send a Connected Message to the server
        let mut internal_client_message_sender_1 = internal_client_message_sender.clone();
        assert_eq!(
            internal_client_message_sender_1
                .send(InternalClientMessage::Connected(server_message_sender_1))
                .await,
            Ok(())
        );

        assert_eq!(
            server_message_receiver_1.next().await,
            Some(ServerMessage::YouAre(ClientId::from_count(2))),
        );

        let client_1_id = ClientId::from_count(2);

        // Send another Connected Message to the server
        let mut internal_client_message_sender_2 = internal_client_message_sender;
        assert_eq!(
            internal_client_message_sender_2
                .send(InternalClientMessage::Connected(server_message_sender_2))
                .await,
            Ok(())
        );

        assert_eq!(
            server_message_receiver_2.next().await,
            Some(ServerMessage::YouAre(ClientId::from_count(3))),
        );

        assert_eq!(
            internal_client_message_sender_1
                .send(InternalClientMessage::Request(
                    client_1_id,
                    ClientMessage::RequestNodeIdentitySnapshot
                ))
                .await,
            Ok(()),
        );

        assert_eq!(
            server_message_receiver_1.next().await,
            Some(ServerMessage::NodeIdentitySnapshot(Arc::new(vec![
                node_1.clone(),
                node_2.clone(),
                node_3.clone()
            ]))),
        );

        if let Some(process_internal_client_message_handle) =
            process_internal_client_message_handle.task_handle.take()
        {
            process_internal_client_message_handle.abort();
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_process_client_handling_stream_subscribe_latest_block() {
        let (_, _, _, data_state) = create_test_data_state();
        let client_thread_state = Arc::new(RwLock::new(create_test_client_thread_state()));
        let data_state = Arc::new(RwLock::new(data_state));

        let (mut leaf_sender, leaf_receiver) = mpsc::channel(1);
        let (block_detail_sender, block_detail_receiver) = mpsc::channel(1);
        let (voters_sender, voters_receiver) = mpsc::channel(1);
        let (internal_client_message_sender, internal_client_message_receiver) = mpsc::channel(1);
        let (server_message_sender_1, mut server_message_receiver_1) = mpsc::channel(1);
        let (server_message_sender_2, mut server_message_receiver_2) = mpsc::channel(1);
        let (server_message_sender_3, mut server_message_receiver_3) = mpsc::channel(1);
        let mut process_internal_client_message_handle = InternalClientMessageProcessingTask::new(
            internal_client_message_receiver,
            data_state.clone(),
            client_thread_state.clone(),
        );

        let mut process_distribute_block_detail_handle =
            ProcessDistributeBlockDetailHandlingTask::new(
                client_thread_state.clone(),
                block_detail_receiver,
            );

        let mut process_distribute_voters_handle =
            ProcessDistributeVotersHandlingTask::new(client_thread_state, voters_receiver);

        let mut process_leaf_stream_handle = ProcessLeafAndBlockPairStreamTask::new(
            leaf_receiver,
            data_state,
            surf_disco::client::Client::new("http://localhost/".parse().unwrap()),
            PublicHotShotConfig {
                epoch_height: None,
                epoch_start_block: None,
                known_nodes_with_stake: vec![],
            },
            block_detail_sender,
            voters_sender,
        );

        // Send a Connected Message to the server
        let mut internal_client_message_sender_1 = internal_client_message_sender.clone();
        assert_eq!(
            internal_client_message_sender_1
                .send(InternalClientMessage::Connected(server_message_sender_1))
                .await,
            Ok(())
        );

        assert_eq!(
            server_message_receiver_1.next().await,
            Some(ServerMessage::YouAre(ClientId::from_count(2))),
        );

        let client_1_id = ClientId::from_count(2);
        let client_2_id = ClientId::from_count(3);

        // Send another Connected Message to the server
        let mut internal_client_message_sender_2 = internal_client_message_sender.clone();
        assert_eq!(
            internal_client_message_sender_2
                .send(InternalClientMessage::Connected(server_message_sender_2))
                .await,
            Ok(())
        );

        assert_eq!(
            server_message_receiver_2.next().await,
            Some(ServerMessage::YouAre(ClientId::from_count(3))),
        );

        // Send another Connected Message to the server
        let mut internal_client_message_sender_3 = internal_client_message_sender;
        assert_eq!(
            internal_client_message_sender_3
                .send(InternalClientMessage::Connected(server_message_sender_3))
                .await,
            Ok(())
        );

        assert_eq!(
            server_message_receiver_3.next().await,
            Some(ServerMessage::YouAre(ClientId::from_count(4))),
        );

        assert_eq!(
            internal_client_message_sender_1
                .send(InternalClientMessage::Request(
                    client_1_id,
                    ClientMessage::SubscribeLatestBlock
                ))
                .await,
            Ok(()),
        );

        assert_eq!(
            internal_client_message_sender_1
                .send(InternalClientMessage::Request(
                    client_2_id,
                    ClientMessage::SubscribeLatestBlock
                ))
                .await,
            Ok(()),
        );

        // No response expected from the client messages at the moment.

        // send a new leaf
        let validated_state = ValidatedState::default();
        let instance_state = NodeState::mock();

        let leaf =
            Leaf2::genesis::<TestVersions>(&ValidatedState::default(), &NodeState::mock()).await;
        let block_query_data =
            BlockQueryData::genesis::<MockVersions>(&validated_state, &instance_state).await;
        let expected_block = create_block_detail_from_block(&block_query_data);
        let arc_expected_block = Arc::new(expected_block);

        assert_eq!(
            leaf_sender
                .send((
                    Leaf1QueryData::new(leaf.clone().to_leaf_unsafe(), leaf.justify_qc().to_qc()),
                    block_query_data.clone()
                ))
                .await,
            Ok(())
        );

        // We should receive the Block Detail on each subscribed client
        assert_eq!(
            server_message_receiver_1.next().await,
            Some(ServerMessage::LatestBlock(arc_expected_block.clone()))
        );
        assert_eq!(
            server_message_receiver_2.next().await,
            Some(ServerMessage::LatestBlock(arc_expected_block.clone()))
        );

        if timeout(Duration::from_millis(10), server_message_receiver_3.next())
            .await
            .is_ok()
        {
            panic!("receiver 3 should not have received the latest block.");
        }

        if let Some(process_internal_client_message_handle) =
            process_internal_client_message_handle.task_handle.take()
        {
            process_internal_client_message_handle.abort();
        }
        if let Some(process_distribute_block_detail_handle) =
            process_distribute_block_detail_handle.task_handle.take()
        {
            process_distribute_block_detail_handle.abort();
        }
        if let Some(process_distribute_voters_handle) =
            process_distribute_voters_handle.task_handle.take()
        {
            process_distribute_voters_handle.abort();
        }
        if let Some(process_leaf_stream_handle) = process_leaf_stream_handle.task_handle.take() {
            process_leaf_stream_handle.abort();
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_process_client_handling_stream_subscribe_node_identity() {
        let (node_1, _, _, data_state) = create_test_data_state();
        let client_thread_state = Arc::new(RwLock::new(create_test_client_thread_state()));
        let data_state = Arc::new(RwLock::new(data_state));

        let (mut node_identity_sender, node_identity_receiver) = mpsc::channel(1);
        let (internal_client_message_sender, internal_client_message_receiver) = mpsc::channel(1);
        let (server_message_sender_1, mut server_message_receiver_1) = mpsc::channel(1);
        let (server_message_sender_2, mut server_message_receiver_2) = mpsc::channel(1);
        let (server_message_sender_3, mut server_message_receiver_3) = mpsc::channel(1);
        let mut process_internal_client_message_handle = InternalClientMessageProcessingTask::new(
            internal_client_message_receiver,
            data_state.clone(),
            client_thread_state.clone(),
        );

        let mut process_distribute_node_identity_handle =
            ProcessDistributeNodeIdentityHandlingTask::new(
                client_thread_state,
                node_identity_receiver,
            );

        // Send a Connected Message to the server
        let mut internal_client_message_sender_1 = internal_client_message_sender.clone();
        assert_eq!(
            internal_client_message_sender_1
                .send(InternalClientMessage::Connected(server_message_sender_1))
                .await,
            Ok(())
        );

        assert_eq!(
            server_message_receiver_1.next().await,
            Some(ServerMessage::YouAre(ClientId::from_count(2))),
        );

        let client_1_id = ClientId::from_count(2);
        let client_2_id = ClientId::from_count(3);

        // Send another Connected Message to the server
        let mut internal_client_message_sender_2 = internal_client_message_sender.clone();
        assert_eq!(
            internal_client_message_sender_2
                .send(InternalClientMessage::Connected(server_message_sender_2))
                .await,
            Ok(())
        );

        assert_eq!(
            server_message_receiver_2.next().await,
            Some(ServerMessage::YouAre(ClientId::from_count(3))),
        );

        // Send another Connected Message to the server
        let mut internal_client_message_sender_3 = internal_client_message_sender.clone();
        assert_eq!(
            internal_client_message_sender_3
                .send(InternalClientMessage::Connected(server_message_sender_3))
                .await,
            Ok(())
        );

        assert_eq!(
            server_message_receiver_3.next().await,
            Some(ServerMessage::YouAre(ClientId::from_count(4))),
        );

        assert_eq!(
            internal_client_message_sender_1
                .send(InternalClientMessage::Request(
                    client_1_id,
                    ClientMessage::SubscribeNodeIdentity
                ))
                .await,
            Ok(()),
        );

        assert_eq!(
            internal_client_message_sender_1
                .send(InternalClientMessage::Request(
                    client_2_id,
                    ClientMessage::SubscribeNodeIdentity
                ))
                .await,
            Ok(()),
        );

        // No response expected from the client messages at the moment.

        // send a new Node Identity
        let node_identity = node_1;
        assert_eq!(
            node_identity_sender.send(node_identity.clone()).await,
            Ok(())
        );

        let arc_node_identity = Arc::new(node_identity.clone());

        // We should receive the Block Detail on each subscribed client
        assert_eq!(
            server_message_receiver_1.next().await,
            Some(ServerMessage::LatestNodeIdentity(arc_node_identity.clone()))
        );
        assert_eq!(
            server_message_receiver_2.next().await,
            Some(ServerMessage::LatestNodeIdentity(arc_node_identity.clone()))
        );

        if let Some(process_internal_client_message_handle) =
            process_internal_client_message_handle.task_handle.take()
        {
            process_internal_client_message_handle.abort();
        }

        if let Some(process_distribute_node_identity_handle) =
            process_distribute_node_identity_handle.task_handle.take()
        {
            process_distribute_node_identity_handle.abort();
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_process_client_handling_stream_subscribe_voters() {
        let (_, _, _, data_state) = create_test_data_state();
        let client_thread_state = Arc::new(RwLock::new(create_test_client_thread_state()));
        let data_state = Arc::new(RwLock::new(data_state));

        let (mut voters_sender, voters_receiver) = mpsc::channel(1);
        let (internal_client_message_sender, internal_client_message_receiver) = mpsc::channel(1);
        let (server_message_sender_1, mut server_message_receiver_1) = mpsc::channel(1);
        let (server_message_sender_2, mut server_message_receiver_2) = mpsc::channel(1);
        let (server_message_sender_3, mut server_message_receiver_3) = mpsc::channel(1);
        let mut process_internal_client_message_handle = InternalClientMessageProcessingTask::new(
            internal_client_message_receiver,
            data_state.clone(),
            client_thread_state.clone(),
        );

        let mut process_distribute_voters_handle =
            ProcessDistributeVotersHandlingTask::new(client_thread_state, voters_receiver);

        // Send a Connected Message to the server
        let mut internal_client_message_sender_1 = internal_client_message_sender.clone();
        assert_eq!(
            internal_client_message_sender_1
                .send(InternalClientMessage::Connected(server_message_sender_1))
                .await,
            Ok(())
        );

        assert_eq!(
            server_message_receiver_1.next().await,
            Some(ServerMessage::YouAre(ClientId::from_count(2))),
        );

        let client_1_id = ClientId::from_count(2);
        let client_2_id = ClientId::from_count(3);

        // Send another Connected Message to the server
        let mut internal_client_message_sender_2 = internal_client_message_sender.clone();
        assert_eq!(
            internal_client_message_sender_2
                .send(InternalClientMessage::Connected(server_message_sender_2))
                .await,
            Ok(())
        );

        assert_eq!(
            server_message_receiver_2.next().await,
            Some(ServerMessage::YouAre(ClientId::from_count(3))),
        );

        // Send another Connected Message to the server
        let mut internal_client_message_sender_3 = internal_client_message_sender;
        assert_eq!(
            internal_client_message_sender_3
                .send(InternalClientMessage::Connected(server_message_sender_3))
                .await,
            Ok(())
        );

        assert_eq!(
            server_message_receiver_3.next().await,
            Some(ServerMessage::YouAre(ClientId::from_count(4))),
        );

        assert_eq!(
            internal_client_message_sender_1
                .send(InternalClientMessage::Request(
                    client_1_id,
                    ClientMessage::SubscribeVoters
                ))
                .await,
            Ok(()),
        );

        assert_eq!(
            internal_client_message_sender_1
                .send(InternalClientMessage::Request(
                    client_2_id,
                    ClientMessage::SubscribeVoters
                ))
                .await,
            Ok(()),
        );

        // No response expected from the client messages at the moment.

        // send a new Node Identity
        let voters = BitVec::from_vec(vec![0x55]);
        assert_eq!(voters_sender.send(voters.clone()).await, Ok(()));

        // We should receive the Block Detail on each subscribed client
        assert_eq!(
            server_message_receiver_1.next().await,
            Some(ServerMessage::LatestVoters(voters.clone()))
        );
        assert_eq!(
            server_message_receiver_2.next().await,
            Some(ServerMessage::LatestVoters(voters.clone()))
        );

        if let Some(process_internal_client_message_handle) =
            process_internal_client_message_handle.task_handle.take()
        {
            process_internal_client_message_handle.abort();
        }
        if let Some(process_distribute_voters_handle) =
            process_distribute_voters_handle.task_handle.take()
        {
            process_distribute_voters_handle.abort();
        }
    }

    // The following tests codify assumptions being bad on behalf of the Sink
    // and Receivers provided by the async_std library.  The purpose of these
    // tests are to document these assumptions, and add a test to ensure that
    // they behave as expected.  If they ever do not behave as expected, then
    // the rest of this library will need to be modified to account for that
    // change in behavior.

    /// Tests the behavior of the sender and receiver when the sender is
    /// dropped before the receiver is polled.
    ///
    /// This is a separate library test to ensure that the behavior that this
    /// library is built on top of does not introduce a change that would
    /// make this library no longer operate correctly.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_sender_receiver_behavior_drop_sender_before_receiver_polled_closes_receiver() {
        let (sender, mut receiver) = mpsc::channel::<u64>(1);

        drop(sender);

        assert_eq!(receiver.next().await, None);
    }

    /// Tests the behavior of the  sender and receiver when the sender is
    /// dropped after the receiver is polled.
    ///
    /// This is a separate library test to ensure that the behavior that this
    /// library is built on top of does not introduce a change that would
    /// make this library no longer operate correctly.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_sender_receiver_behavior_drop_sender_after_receiver_polled_closes_receiver() {
        let (sender, mut receiver) = mpsc::channel::<u64>(1);

        let join_handle = spawn(async move { receiver.next().await });
        sleep(Duration::from_millis(100)).await;
        drop(sender);

        assert!(join_handle.await.unwrap().is_none());
    }

    /// Tests the behavior of the sender and receiver when the receiver is
    /// dropped before anything is sent across the Sink.
    ///
    /// This is a separate library test to ensure that the behavior that this
    /// library is built on top of does not introduce a change that would
    /// make this library no longer operate correctly.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_sender_receiver_behavior_drop_receiver_before_sender_sends() {
        let (mut sender, receiver) = mpsc::channel(1);

        drop(receiver);

        assert_ne!(sender.send(1).await, Ok(()));
    }

    /// Tests the behavior of the sender and receiver when the receiver is
    /// dropped after the sender has sent a value.
    ///
    /// This is a separate library test to ensure that the behavior that this
    /// library is built on top of does not introduce a change that would
    /// make this library no longer operate correctly.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_sender_receiver_behavior_drop_receiver_after_sender_sends() {
        let (mut sender, mut receiver) = mpsc::channel(1);

        let join_handle = spawn(async move {
            _ = sender.send(1).await;
            sleep(Duration::from_millis(100)).await;
            sender.send(2).await
        });
        sleep(Duration::from_millis(50)).await;
        receiver.close();

        assert_eq!(receiver.next().await, Some(1));
        assert_eq!(receiver.next().await, None);
        assert!(join_handle.await.unwrap().is_err());
    }

    /// Tests to ensure that time timeout on an already ready future does not
    /// cause the future to be dropped.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_timeout_on_already_ready_future() {
        assert_eq!(
            timeout(Duration::ZERO, futures::future::ready(1u64)).await,
            Ok(1u64)
        );
    }

    /// Tests to ensure that time timeout on a pending future does not cause the
    /// future to be dropped.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_timeout_on_async_block_resolves_when_polled() {
        assert_eq!(timeout(Duration::ZERO, async move { 1u64 }).await, Ok(1u64),);

        assert_eq!(
            timeout(Duration::from_millis(100), async move { 1u64 }).await,
            Ok(1u64),
        );
    }

    /// Tests to ensure that time timeout on a pending future does not cause the
    /// future to be dropped.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_timeout_on_pending_future_times_out() {
        assert_ne!(
            timeout(Duration::ZERO, futures::future::pending::<u64>()).await,
            Ok(1u64)
        );
    }

    /// Tests to ensure that bitvec is directly comparable without needing to
    /// worry about their instances points to the same memory.
    #[test]
    fn test_bitvec_is_comparable() {
        let bitvec_1: BitVec<usize> = BitVec::from_vec(vec![0x55]);
        let bitvec_2: BitVec<usize> = BitVec::from_vec(vec![0x55]);
        let bitvec_3: BitVec<usize> = BitVec::from_vec(vec![0xAA]);

        assert_eq!(bitvec_1, bitvec_2);
        assert_ne!(bitvec_1, bitvec_3);
    }
}
