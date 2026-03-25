use anyhow::{Context, Result};
use hotshot_types::{
    epoch_membership::EpochMembershipCoordinator,
    message::{EXTERNAL_MESSAGE_VERSION, UpgradeLock},
    traits::{network::ConnectedNetwork, node_implementation::NodeType},
};
use vbs::version::Version;

use crate::{
    events::NetworkEvent,
    message::{ConsensusMessage, Message, MessageType, ViewSyncMessage},
};

pub struct Network<T: NodeType, N: ConnectedNetwork<T::SignatureKey>> {
    network: N,
    membership_coordinator: EpochMembershipCoordinator<T>,
    upgrade_lock: UpgradeLock<T>,
}

impl<T: NodeType, N: ConnectedNetwork<T::SignatureKey>> Network<T, N> {
    pub fn new(
        network: N,
        membership_coordinator: EpochMembershipCoordinator<T>,
        upgrade_lock: UpgradeLock<T>,
    ) -> Self {
        Self {
            network,
            membership_coordinator,
            upgrade_lock,
        }
    }

    pub async fn recv_message(&self) -> Result<ConsensusMessage<T>> {
        let message = self.network.recv_message().await?;
        let message: Message<T> = self.deserialize(message)?;
        match message.message_type {
            MessageType::Consensus(consensus_message) => Ok(consensus_message),
            _ => Err(anyhow::anyhow!("Received a non-consensus message")),
        }
    }

    async fn handle_event(&self, event: NetworkEvent<T>) {
        match event {
            NetworkEvent::SendMessage(message) => {
                self.send_message(message).await;
            },
            NetworkEvent::ViewChanged(view, epoch) => {
                self.network
                    .update_view(view, Some(epoch), self.membership_coordinator.clone())
                    .await;
            },
        }
    }
    pub async fn send_message(&self, message: ConsensusMessage<T>) {
        todo!()
    }
    async fn handle_message(&self, message: Vec<u8>) -> Result<()> {
        let message = self.deserialize(message)?;
        match message.message_type {
            MessageType::Consensus(consensus_message) => {
                self.handle_consensus_message(consensus_message).await;
            },
            MessageType::ViewSync(view_sync_message) => {
                self.handle_view_sync_message(view_sync_message).await;
            },
            MessageType::External(external_message) => {
                self.handle_external_message(external_message).await;
            },
        }
        Ok(())
    }
    async fn handle_consensus_message(&self, consensus_message: ConsensusMessage<T>) {
        todo!()
    }
    async fn handle_view_sync_message(&self, view_sync_message: ViewSyncMessage<T>) {
        todo!()
    }
    async fn handle_external_message(&self, external_message: Vec<u8>) {
        todo!()
    }
    fn deserialize(&self, message: Vec<u8>) -> Result<Message<T>> {
        // Deserialize the message and get the version
        let (deserialized_message, version): (Message<T>, Version) =
            match self.upgrade_lock.deserialize(&message) {
                Ok(message) => message,
                Err(e) => {
                    tracing::error!("Failed to deserialize message: {:?}", e);
                    return Err(anyhow::anyhow!("Failed to deserialize message: {:?}", e));
                },
            };

        // Special case: external messages (version 0.0). We want to make sure it is an external message
        // and warn and continue otherwise.
        if version == EXTERNAL_MESSAGE_VERSION
            && !matches!(
                deserialized_message.message_type,
                MessageType::<T>::External(_)
            )
        {
            tracing::warn!("Received a non-external message with version 0.0");
            return Err(anyhow::anyhow!(
                "Received a non-external message with version 0.0"
            ));
        }
        Ok(deserialized_message)
    }
    fn serialize(&self, message: Message<T>) -> Result<Vec<u8>> {
        self.upgrade_lock
            .serialize(&message)
            .context("Failed to serialize message")
    }
}
