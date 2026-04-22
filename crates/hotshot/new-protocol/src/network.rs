pub mod cliquenet;

use hotshot::traits::NetworkError;
use hotshot_types::{data::ViewNumber, traits::node_implementation::NodeType};

use crate::message::{Message, Unchecked, Validated};

pub trait Network<T: NodeType> {
    fn unicast(
        &mut self,
        v: ViewNumber,
        to: &T::SignatureKey,
        m: &Message<T, Validated>,
    ) -> Result<(), NetworkError>;

    fn multicast(
        &mut self,
        v: ViewNumber,
        to: Vec<&T::SignatureKey>,
        m: &Message<T, Validated>,
    ) -> Result<(), NetworkError>;

    fn broadcast(&mut self, v: ViewNumber, m: &Message<T, Validated>) -> Result<(), NetworkError>;

    fn receive(
        &mut self,
    ) -> impl Future<Output = Result<Message<T, Unchecked>, NetworkError>> + Send;

    fn gc(&mut self, v: ViewNumber) -> Result<(), NetworkError>;
}

pub trait PeerManagement<T: NodeType> {
    type Data;

    fn add_peers(
        &mut self,
        r: PeerRole,
        ps: Vec<(T::SignatureKey, Self::Data)>,
    ) -> Result<(), NetworkError>;

    fn remove_peers(&mut self, ps: Vec<&T::SignatureKey>) -> Result<(), NetworkError>;

    fn assign_role(&mut self, r: PeerRole, ps: Vec<&T::SignatureKey>) -> Result<(), NetworkError>;
}

#[derive(Clone, Copy, Debug)]
pub enum PeerRole {
    Active,
    Passive,
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
