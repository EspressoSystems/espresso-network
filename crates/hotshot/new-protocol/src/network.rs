pub mod cliquenet;

use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::node_implementation::NodeType,
};

use crate::message::{Message, Unchecked, Validated};

type Result<T> = std::result::Result<T, NetworkError>;

/// Clone-able send-only handle. Allows holding a network sender from a
/// `spawn_blocking` worker (e.g. share fan-out) without keeping `&mut self`
/// borrow on the network.
pub trait NetworkSender<T: NodeType>: Send + Sync + Clone + 'static {
    fn unicast(&self, v: ViewNumber, to: &T::SignatureKey, m: &Message<T, Validated>)
    -> Result<()>;
}

pub trait Network<T: NodeType> {
    type PeerData;
    type Sender: NetworkSender<T>;

    /// Return a Clone-able send-only handle.
    fn sender(&self) -> Self::Sender;

    fn broadcast(&mut self, v: ViewNumber, m: &Message<T, Validated>) -> Result<()>;

    fn unicast(
        &mut self,
        v: ViewNumber,
        to: &T::SignatureKey,
        m: &Message<T, Validated>,
    ) -> Result<()>;

    fn multicast(
        &mut self,
        v: ViewNumber,
        to: Vec<&T::SignatureKey>,
        m: &Message<T, Validated>,
    ) -> Result<()>;

    fn receive(&mut self) -> impl Future<Output = Result<Message<T, Unchecked>>> + Send;

    fn shutdown(&mut self) -> impl Future<Output = ()> + Send;

    fn gc(&mut self, v: ViewNumber) -> Result<()>;

    fn add_peers(&mut self, r: PeerRole, ps: Vec<(T::SignatureKey, Self::PeerData)>) -> Result<()>;
    fn remove_peers(&mut self, ps: Vec<&T::SignatureKey>) -> Result<()>;
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
}

impl NetworkError {
    pub fn is_critical(&self) -> bool {
        matches!(self, Self::Critical(_))
    }
}
