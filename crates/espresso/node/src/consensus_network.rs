use std::{collections::HashMap, future::Future, sync::Arc};

use async_broadcast::{Receiver, RecvError, Sender, broadcast};
use async_lock::Mutex;
use async_trait::async_trait;
use espresso_types::PubKey;
use hotshot_types::{
    BoxSyncFuture,
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        network::{BroadcastDelay, ConnectedNetwork, NetworkError, Topic},
        node_implementation::NodeType,
        signature_key::SignatureKey,
    },
};
use tokio::sync::mpsc::error::TrySendError;

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
    let (tx, rx1) = broadcast(1000);
    let rx2 = tx.new_receiver();
    let hotshot = ConsensusNetwork::new(network.clone(), rx1);
    let coordinator = ConsensusNetwork::new(network.clone(), rx2);
    let driver = async move {
        drive_consensus_network(network, tx).await;
    };
    (hotshot, coordinator, driver)
}

async fn drive_consensus_network<N>(network: N, tx: Sender<Vec<u8>>)
where
    N: ConnectedNetwork<PubKey>,
{
    loop {
        match network.recv_message().await {
            Ok(message) => {
                if let Err(err) = tx.broadcast_direct(message).await {
                    tracing::info!(%err, "consensus network closed");
                    return;
                }
            },
            Err(NetworkError::ShutDown) => {
                tracing::info!("consensus network shutting down");
                return;
            },
            Err(err) => {
                panic!("consensus network driver stopped after network receive error: {err}");
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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use async_lock::Mutex;
    use async_trait::async_trait;
    use espresso_types::PubKey;
    use hotshot_types::{
        BoxSyncFuture, boxed_sync,
        data::ViewNumber,
        traits::network::{BroadcastDelay, ConnectedNetwork, NetworkError, Topic},
    };
    use tokio::sync::mpsc::error::TrySendError;

    use super::create_consensus_networks;

    #[derive(Clone)]
    struct TestNetwork {
        rx: Arc<Mutex<async_channel::Receiver<Vec<u8>>>>,
        tx: async_channel::Sender<Vec<u8>>,
    }

    impl TestNetwork {
        fn new() -> Self {
            let (tx, rx) = async_channel::unbounded();
            Self {
                rx: Arc::new(Mutex::new(rx)),
                tx,
            }
        }

        async fn inject(&self, message: Vec<u8>) {
            self.tx.send(message).await.unwrap();
        }
    }

    #[async_trait]
    impl ConnectedNetwork<PubKey> for TestNetwork {
        fn pause(&self) {}

        fn resume(&self) {}

        async fn wait_for_ready(&self) {}

        fn shut_down<'a, 'b>(&'a self) -> BoxSyncFuture<'b, ()>
        where
            'a: 'b,
            Self: 'b,
        {
            boxed_sync(async {})
        }

        async fn broadcast_message(
            &self,
            _: ViewNumber,
            _: Vec<u8>,
            _: Topic,
            _: BroadcastDelay,
        ) -> Result<(), NetworkError> {
            Ok(())
        }

        async fn da_broadcast_message(
            &self,
            _: ViewNumber,
            _: Vec<u8>,
            _: Vec<PubKey>,
            _: BroadcastDelay,
        ) -> Result<(), NetworkError> {
            Ok(())
        }

        async fn vid_broadcast_message(
            &self,
            _: HashMap<PubKey, (ViewNumber, Vec<u8>)>,
        ) -> Result<(), NetworkError> {
            Ok(())
        }

        async fn direct_message(
            &self,
            _: ViewNumber,
            _: Vec<u8>,
            _: PubKey,
        ) -> Result<(), NetworkError> {
            Ok(())
        }

        async fn recv_message(&self) -> Result<Vec<u8>, NetworkError> {
            let receiver = self.rx.lock().await;
            receiver
                .recv()
                .await
                .map_err(|err| NetworkError::ChannelReceiveError(err.to_string()))
        }

        fn queue_node_lookup(
            &self,
            _: ViewNumber,
            _: PubKey,
        ) -> Result<(), TrySendError<Option<(ViewNumber, PubKey)>>> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn delivers_messages_to_legacy_and_coordinator_networks() {
        let network = TestNetwork::new();
        let (legacy_network, coordinator_network, driver) =
            create_consensus_networks(network.clone());
        let driver = tokio::spawn(driver);

        network.inject(vec![1, 2, 3]).await;

        let legacy_message = legacy_network.recv_message().await.unwrap();
        let coordinator_message = coordinator_network.recv_message().await.unwrap();

        assert_eq!(legacy_message, vec![1, 2, 3]);
        assert_eq!(coordinator_message, vec![1, 2, 3]);

        driver.abort();
    }
}
