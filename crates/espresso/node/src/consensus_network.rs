use std::{collections::HashMap, future::Future, sync::Arc};

use async_broadcast::{Receiver, RecvError, Sender, broadcast};
use async_lock::Mutex;
use async_trait::async_trait;
use espresso_types::PubKey;
use hotshot_types::{
    BoxSyncFuture,
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::EXTERNAL_MESSAGE_VERSION,
    traits::{
        network::{BroadcastDelay, ConnectedNetwork, NetworkError, Topic},
        node_implementation::NodeType,
        signature_key::SignatureKey,
    },
};
use tokio::sync::mpsc::error::TrySendError;
use vbs::version::Version;
use versions::CLIQUENET_VERSION;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConsensusMessageRoute {
    Legacy,
    Coordinator,
}

#[derive(Clone)]
pub struct ConsensusNetwork<N> {
    network: N,
    receiver: Arc<Mutex<Receiver<Vec<u8>>>>,
}

impl<N> ConsensusNetwork<N> {
    fn new(network: N, receiver: Receiver<Vec<u8>>) -> Self {
        Self {
            network,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }
}

pub(crate) fn create_consensus_networks<N>(
    network: N,
) -> (
    ConsensusNetwork<N>,
    ConsensusNetwork<N>,
    impl Future<Output = ()> + Send + 'static,
)
where
    N: ConnectedNetwork<PubKey>,
{
    let (mut legacy_tx, mut legacy_rx) = broadcast(1000);
    let (mut coordinator_tx, mut coordinator_rx) = broadcast(1000);
    legacy_tx.set_await_active(false);
    coordinator_tx.set_await_active(false);
    legacy_rx.set_overflow(true);
    coordinator_rx.set_overflow(true);
    let hotshot = ConsensusNetwork::new(network.clone(), legacy_rx);
    let coordinator = ConsensusNetwork::new(network.clone(), coordinator_rx);
    let driver = async move {
        drive_consensus_network(network, legacy_tx, coordinator_tx).await;
    };
    (hotshot, coordinator, driver)
}

fn classify_message_route(message: &[u8]) -> Result<ConsensusMessageRoute, NetworkError> {
    let (version, _) = Version::deserialize(message)
        .map_err(|err| NetworkError::FailedToDeserialize(err.to_string()))?;
    Ok(match version {
        EXTERNAL_MESSAGE_VERSION => ConsensusMessageRoute::Coordinator,
        v if v > CLIQUENET_VERSION => ConsensusMessageRoute::Coordinator,
        _ => ConsensusMessageRoute::Legacy,
    })
}

async fn route_message(
    route: ConsensusMessageRoute,
    message: Vec<u8>,
    legacy_tx: &Sender<Vec<u8>>,
    coordinator_tx: &Sender<Vec<u8>>,
) {
    let tx = match route {
        ConsensusMessageRoute::Legacy => legacy_tx,
        ConsensusMessageRoute::Coordinator => coordinator_tx,
    };

    match tx.broadcast_direct(message).await {
        Ok(None) => {},
        Ok(Some(_overflowed)) => {
            tracing::debug!(
                ?route,
                "consensus network route overflowed, oldest message dropped"
            );
        },
        Err(err) => {
            tracing::warn!(?route, %err, "failed to route consensus network message");
        },
    }
}

async fn drive_consensus_network<N>(
    network: N,
    legacy_tx: Sender<Vec<u8>>,
    coordinator_tx: Sender<Vec<u8>>,
) where
    N: ConnectedNetwork<PubKey>,
{
    loop {
        match network.recv_message().await {
            Ok(message) => {
                let route = match classify_message_route(&message) {
                    Ok(route) => route,
                    Err(err) => {
                        tracing::error!(%err, "unexpected error classifying consensus network message");
                        continue;
                    },
                };

                route_message(route, message, &legacy_tx, &coordinator_tx).await;
            },
            Err(NetworkError::ShutDown) => {
                tracing::info!("consensus network shutting down");
                return;
            },
            Err(err) => {
                tracing::error!(%err, "network receive error");
            },
        }
    }
}

#[async_trait]
impl<K, N> ConnectedNetwork<K> for ConsensusNetwork<N>
where
    K: SignatureKey + 'static,
    N: ConnectedNetwork<K>,
{
    fn pause(&self) {
        self.network.pause();
    }

    fn resume(&self) {
        self.network.resume();
    }

    async fn wait_for_ready(&self) {
        self.network.wait_for_ready().await;
    }

    fn shut_down<'a, 'b>(&'a self) -> BoxSyncFuture<'b, ()>
    where
        'a: 'b,
        Self: 'b,
    {
        self.network.shut_down()
    }

    async fn broadcast_message(
        &self,
        view: ViewNumber,
        message: Vec<u8>,
        topic: Topic,
        broadcast_delay: BroadcastDelay,
    ) -> Result<(), NetworkError> {
        self.network
            .broadcast_message(view, message, topic, broadcast_delay)
            .await
    }

    async fn da_broadcast_message(
        &self,
        view: ViewNumber,
        message: Vec<u8>,
        recipients: Vec<K>,
        broadcast_delay: BroadcastDelay,
    ) -> Result<(), NetworkError> {
        self.network
            .da_broadcast_message(view, message, recipients, broadcast_delay)
            .await
    }

    async fn vid_broadcast_message(
        &self,
        messages: HashMap<K, (ViewNumber, Vec<u8>)>,
    ) -> Result<(), NetworkError> {
        self.network.vid_broadcast_message(messages).await
    }

    async fn direct_message(
        &self,
        view: ViewNumber,
        message: Vec<u8>,
        recipient: K,
    ) -> Result<(), NetworkError> {
        self.network.direct_message(view, message, recipient).await
    }

    async fn recv_message(&self) -> Result<Vec<u8>, NetworkError> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await.map_err(|err| match err {
            RecvError::Closed => NetworkError::ShutDown,
            RecvError::Overflowed(missed) => NetworkError::ChannelReceiveError(format!(
                "consensus network receiver overflowed after missing {missed} messages"
            )),
        })
    }

    fn queue_node_lookup(
        &self,
        view: ViewNumber,
        key: K,
    ) -> Result<(), TrySendError<Option<(ViewNumber, K)>>> {
        self.network.queue_node_lookup(view, key)
    }

    async fn update_view<TYPES>(
        &self,
        view: ViewNumber,
        epoch: Option<EpochNumber>,
        membership: EpochMembershipCoordinator<TYPES>,
    ) where
        TYPES: NodeType<SignatureKey = K>,
    {
        self.network.update_view(view, epoch, membership).await;
    }

    fn is_primary_down(&self) -> bool {
        self.network.is_primary_down()
    }
}
