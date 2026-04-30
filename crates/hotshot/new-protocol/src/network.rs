pub mod cliquenet;

use hotshot_types::{data::ViewNumber, traits::node_implementation::NodeType};

use crate::message::{Message, Unchecked, Validated};

type Result<T> = std::result::Result<T, NetworkError>;

pub trait Network<T: NodeType> {
    type PeerData;

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

    fn gc(&mut self, v: ViewNumber) -> Result<()>;

    fn add_peers(&mut self, r: PeerRole, ps: Vec<(T::SignatureKey, Self::PeerData)>) -> Result<()>;
    fn remove_peers(&mut self, ps: Vec<&T::SignatureKey>) -> Result<()>;
    fn assign_role(&mut self, r: PeerRole, ps: Vec<&T::SignatureKey>) -> Result<()>;
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
