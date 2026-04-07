use hotshot::traits::NetworkError;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::{EXTERNAL_MESSAGE_VERSION, UpgradeLock},
    traits::{
        network::{BroadcastDelay, ConnectedNetwork, Topic},
        node_implementation::NodeType,
    },
    vote::HasViewNumber,
};

use crate::message::{Message, Unchecked, Validated};

pub type Result<T> = std::result::Result<T, NetworkError>;

pub struct Network<T: NodeType, N> {
    network: N,
    membership_coordinator: EpochMembershipCoordinator<T>,
    upgrade_lock: UpgradeLock<T>,
}

impl<T, N> Network<T, N>
where
    T: NodeType,
    N: ConnectedNetwork<T::SignatureKey>,
{
    pub fn new(n: N, m: EpochMembershipCoordinator<T>, u: UpgradeLock<T>) -> Self {
        Self {
            network: n,
            membership_coordinator: m,
            upgrade_lock: u,
        }
    }
    pub fn gc(&mut self, _view_number: ViewNumber, _epoch: EpochNumber) {
        // TODO: Implement
    }

    pub async fn receive(&mut self) -> Result<Message<T, Unchecked>> {
        let m = self.network.recv_message().await?;
        self.deserialize(m)
    }

    pub async fn broadcast(&mut self, msg: Message<T, Validated>) -> Result<()> {
        let view = msg.view_number();
        let bytes = self.serialize(&msg)?;
        self.network
            .broadcast_message(view, bytes, Topic::Global, BroadcastDelay::None)
            .await?;
        Ok(())
    }

    pub async fn unicast(&mut self, to: T::SignatureKey, msg: Message<T, Validated>) -> Result<()> {
        let view = msg.view_number();
        let bytes = self.serialize(&msg)?;
        self.network.direct_message(view, bytes, to).await?;
        Ok(())
    }

    pub async fn update_view(&mut self, v: ViewNumber, e: EpochNumber) {
        self.network
            .update_view(v, Some(e), self.membership_coordinator.clone())
            .await;
    }

    fn deserialize(&self, bytes: Vec<u8>) -> Result<Message<T, Unchecked>> {
        match self
            .upgrade_lock
            .deserialize::<Message<T, Unchecked>>(&bytes)
        {
            Ok((m, v)) => {
                if v == EXTERNAL_MESSAGE_VERSION && !m.is_external() {
                    let e = "received a non-external message with version 0.0".to_string();
                    return Err(NetworkError::FailedToDeserialize(e));
                }
                Ok(m)
            },
            Err(err) => Err(NetworkError::FailedToDeserialize(err.to_string())),
        }
    }

    fn serialize(&self, m: &Message<T, Validated>) -> Result<Vec<u8>> {
        self.upgrade_lock
            .serialize(m)
            .map_err(|e| NetworkError::FailedToSerialize(e.to_string()))
    }
}

pub fn is_critical(e: &NetworkError) -> bool {
    match e {
        NetworkError::ChannelReceiveError(_)
        | NetworkError::ChannelSendError(_)
        | NetworkError::ConfigError(_)
        | NetworkError::ListenError(_)
        | NetworkError::ShutDown
        | NetworkError::Unimplemented => true,

        NetworkError::FailedToDeserialize(_)
        | NetworkError::FailedToSerialize(_)
        | NetworkError::LookupError(_)
        | NetworkError::MessageReceiveError(_)
        | NetworkError::MessageSendError(_)
        | NetworkError::NoPeersYet
        | NetworkError::RequestCancelled
        | NetworkError::Timeout(_) => false,

        NetworkError::Multiple(es) => es.iter().any(is_critical),
    }
}
