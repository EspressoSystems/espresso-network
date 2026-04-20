pub mod testing {
    use std::{collections::HashSet, future::pending, sync::Arc};

    use async_trait::async_trait;
    use hotshot::{traits::NetworkError, types::SignatureKey};
    use hotshot_example_types::node_types::TestTypes;
    use hotshot_types::{
        BoxSyncFuture,
        data::{EpochNumber, ViewNumber},
        epoch_membership::EpochMembershipCoordinator,
        traits::{
            network::{BroadcastDelay, ConnectedNetwork, Topic},
            node_implementation::NodeType,
        },
    };
    use tokio::sync::Mutex;

    use crate::coordinator::Coordinator;

    pub type MockCoordinator = Coordinator<TestTypes, MockNetwork>;

    #[derive(Clone, Default)]
    pub struct MockNetwork {
        sent_messages: Arc<Mutex<HashSet<Vec<u8>>>>,
    }

    #[async_trait]
    impl<K: SignatureKey + 'static> ConnectedNetwork<K> for MockNetwork {
        fn pause(&self) {
            todo!()
        }

        fn resume(&self) {
            todo!()
        }

        async fn wait_for_ready(&self) {
            todo!()
        }

        fn shut_down<'a, 'b>(&'a self) -> BoxSyncFuture<'b, ()>
        where
            'a: 'b,
            Self: 'b,
        {
            todo!()
        }

        async fn broadcast_message(
            &self,
            _view: ViewNumber,
            message: Vec<u8>,
            _topic: Topic,
            _delay: BroadcastDelay,
        ) -> Result<(), NetworkError> {
            self.sent_messages.lock().await.insert(message.clone());
            Ok(())
        }

        async fn da_broadcast_message(
            &self,
            _view: ViewNumber,
            _message: Vec<u8>,
            _recipients: Vec<K>,
            _delay: BroadcastDelay,
        ) -> Result<(), NetworkError> {
            todo!()
        }

        async fn direct_message(
            &self,
            _view: ViewNumber,
            message: Vec<u8>,
            _recipient: K,
        ) -> Result<(), NetworkError> {
            self.sent_messages.lock().await.insert(message.clone());
            Ok(())
        }

        async fn recv_message(&self) -> Result<Vec<u8>, NetworkError> {
            Ok(pending::<Vec<u8>>().await)
        }

        async fn update_view<T: NodeType>(
            &self,
            _view: ViewNumber,
            _epoch: Option<EpochNumber>,
            _membership_coordinator: EpochMembershipCoordinator<T>,
        ) {
        }
    }
}
