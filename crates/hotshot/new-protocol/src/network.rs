pub mod cliquenet;

use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::node_implementation::NodeType,
    x25519,
};

use crate::message::{Message, Unchecked, Validated};

type Result<T> = std::result::Result<T, NetworkError>;

pub trait Network<T: NodeType> {
    type PeerData;

    /// Send a message to all peers.
    fn broadcast(&mut self, v: ViewNumber, m: &Message<T, Validated>) -> Result<()>;

    /// Variant of [`broadcast`] that sends the given bytes as is.
    fn broadcast_raw(&mut self, v: ViewNumber, m: Vec<u8>) -> Result<()>;

    /// Send a message to the given peer.
    fn unicast(
        &mut self,
        v: ViewNumber,
        to: &T::SignatureKey,
        m: &Message<T, Validated>,
    ) -> Result<()>;

    /// Variant of [`unicast`] that sends the given bytes as is.
    fn unicast_raw(&mut self, v: ViewNumber, to: &T::SignatureKey, m: Vec<u8>) -> Result<()>;

    /// Send a message to the given peers.
    fn multicast(
        &mut self,
        v: ViewNumber,
        to: Vec<&T::SignatureKey>,
        m: &Message<T, Validated>,
    ) -> Result<()>;

    /// Variant of [`multicast`] that sends the given bytes as is.
    fn multicast_raw(&mut self, v: ViewNumber, to: Vec<&T::SignatureKey>, m: Vec<u8>)
    -> Result<()>;

    /// Await the next inbound message.
    fn receive(&mut self) -> impl Future<Output = Result<Message<T, Unchecked>>> + Send;

    /// Shutdown this network.
    fn shutdown(&mut self) -> impl Future<Output = ()> + Send;

    /// Garbage collect all pending outbound messages below the given view.
    fn gc(&mut self, v: ViewNumber) -> Result<()>;

    /// Add new peers to the existing set of peers.
    ///
    /// If a peer already exists but its address information has changed,
    /// it will be updated with the new data.
    fn add_peers(&mut self, r: PeerRole, ps: Vec<(T::SignatureKey, Self::PeerData)>) -> Result<()>;

    /// Remove the given peers from this network.
    fn remove_peers(&mut self, ps: Vec<&T::SignatureKey>) -> Result<()>;

    /// Change the role of the given peers.
    fn assign_role(&mut self, r: PeerRole, ps: Vec<&T::SignatureKey>) -> Result<()>;

    /// Refresh the peer set for the given epoch using the membership coordinator.
    fn apply_epoch(
        &mut self,
        epoch: EpochNumber,
        coord: &EpochMembershipCoordinator<T>,
    ) -> Result<()>;
}

#[derive(Clone, Copy, Debug)]
pub enum PeerRole {
    Active,
    Passive,
}

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("{0}")]
    Io(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("{0}")]
    Critical(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("configured sender {sender} key {configured:?} != actual key {actual}")]
    InvalidSender {
        sender: String,
        configured: Option<x25519::PublicKey>,
        actual: x25519::PublicKey,
    },
}

impl NetworkError {
    pub fn is_critical(&self) -> bool {
        matches!(self, Self::Critical(_))
    }
}
