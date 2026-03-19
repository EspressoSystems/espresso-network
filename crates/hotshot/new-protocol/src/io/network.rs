use std::sync::Arc;

use anyhow::{Context, Result, anyhow}; // TODO: replace with proper error type
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::{EXTERNAL_MESSAGE_VERSION, UpgradeLock},
    traits::{
        network::{BroadcastDelay, ConnectedNetwork, Topic},
        node_implementation::NodeType,
    },
};
use tracing::{error, warn};
use vbs::version::Version;

use crate::message::{Message, MessageType};

pub(crate) struct Network<T: NodeType, N> {
    network: Arc<N>,
    upgrade_lock: UpgradeLock<T>,
}

impl<T: NodeType, N: ConnectedNetwork<T::SignatureKey>> Network<T, N> {
    pub fn new(network: Arc<N>, upgrade_lock: UpgradeLock<T>) -> Self {
        Self {
            network,
            upgrade_lock,
        }
    }

    pub async fn send(&mut self, v: ViewNumber, m: Message<T>) -> Result<()> {
        let bytes = self.serialize(m)?;
        self.network
            .broadcast_message(v, bytes, Topic::Global, BroadcastDelay::None)
            .await?;
        Ok(())
    }

    pub async fn receive(&mut self) -> Result<Message<T>> {
        let m = self.network.recv_message().await?;
        Ok(self.deserialize(m)?)
    }

    pub async fn update_view(
        &mut self,
        v: ViewNumber,
        e: EpochNumber,
        m: EpochMembershipCoordinator<T>,
    ) {
        self.network.update_view(v, Some(e), m).await
    }

    fn deserialize(&mut self, message: Vec<u8>) -> Result<Message<T>> {
        // Deserialize the message and get the version
        let (deserialized_message, version): (Message<T>, Version) =
            match self.upgrade_lock.deserialize(&message) {
                Ok(message) => message,
                Err(e) => {
                    error!("Failed to deserialize message: {:?}", e);
                    return Err(anyhow!("Failed to deserialize message: {:?}", e));
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
            warn!("Received a non-external message with version 0.0");
            return Err(anyhow!("Received a non-external message with version 0.0"));
        }
        Ok(deserialized_message)
    }

    fn serialize(&mut self, message: Message<T>) -> Result<Vec<u8>> {
        self.upgrade_lock
            .serialize(&message)
            .context("Failed to serialize message")
    }
}
