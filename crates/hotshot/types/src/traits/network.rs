// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Network access compatibility
//!
//! Contains types and traits used by `HotShot` to abstract over network access

use std::hash::Hash;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{node_implementation::NodeType, signature_key::SignatureKey};
use crate::{data::ViewNumber, message::SequencingMessage};

/// Errors that can occur in the network
#[derive(Debug, Error)]
pub enum NetworkError {
    /// Multiple errors. Allows us to roll up multiple errors into one.
    #[error("Multiple errors: {0:?}")]
    Multiple(Vec<NetworkError>),

    /// A configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// An error occurred while sending a message
    #[error("Failed to send message: {0}")]
    MessageSendError(String),

    /// An error occurred while receiving a message
    #[error("Failed to receive message: {0}")]
    MessageReceiveError(String),

    /// The feature is unimplemented
    #[error("Unimplemented")]
    Unimplemented,

    /// An error occurred while attempting to listen
    #[error("Listen error: {0}")]
    ListenError(String),

    /// Failed to send over a channel
    #[error("Channel send error: {0}")]
    ChannelSendError(String),

    /// Failed to receive over a channel
    #[error("Channel receive error: {0}")]
    ChannelReceiveError(String),

    /// The network has been shut down and can no longer be used
    #[error("Network has been shut down")]
    ShutDown,

    /// Failed to serialize
    #[error("Failed to serialize: {0}")]
    FailedToSerialize(String),

    /// Failed to deserialize
    #[error("Failed to deserialize: {0}")]
    FailedToDeserialize(String),

    /// Timed out performing an operation
    #[error("Timeout: {0}")]
    Timeout(String),

    /// The network request had been cancelled before it could be fulfilled
    #[error("The request was cancelled before it could be fulfilled")]
    RequestCancelled,

    /// This node does not have any peers yet
    #[error("This node does not have any peers yet")]
    NoPeersYet,

    /// Failed to look up a node on the network
    #[error("Node lookup failed: {0}")]
    LookupError(String),
}

/// Trait that bundles what we need from a request ID
pub trait Id: Eq + PartialEq + Hash {}

/// a message
pub trait ViewMessage<TYPES: NodeType> {
    /// get the view out of the message
    fn view_number(&self) -> ViewNumber;
}

/// A request for some data that the consensus layer is asking for.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(bound(deserialize = ""))]
pub struct DataRequest<TYPES: NodeType> {
    /// Request
    pub request: RequestKind<TYPES>,
    /// View this message is for
    pub view: ViewNumber,
    /// signature of the Sha256 hash of the data so outsiders can't use know
    /// public keys with stake.
    pub signature: <TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
}

/// Underlying data request
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum RequestKind<TYPES: NodeType> {
    /// Request VID data by our key and the VID commitment
    Vid(ViewNumber, TYPES::SignatureKey),
    /// Request a DA proposal for a certain view
    DaProposal(ViewNumber),
    /// Request for quorum proposal for a view
    Proposal(ViewNumber),
}

impl<TYPES: NodeType> std::fmt::Debug for RequestKind<TYPES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestKind::Vid(view, key) => write!(f, "Vid({view:?}, {key})"),
            RequestKind::DaProposal(view) => write!(f, "DaProposal({view:?})"),
            RequestKind::Proposal(view) => write!(f, "Proposal({view:?})"),
        }
    }
}

/// A response for a request.  `SequencingMessage` is the same as other network messages
/// The kind of message `M` is determined by what we requested
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(bound(deserialize = ""))]
#[allow(clippy::large_enum_variant)]
/// TODO: Put `Found` content in a `Box` to make enum smaller
pub enum ResponseMessage<TYPES: NodeType> {
    /// Peer returned us some data
    Found(SequencingMessage<TYPES>),
    /// Peer failed to get us data
    NotFound,
    /// The Request was denied
    Denied,
}

/// Changes that can occur in the network
#[derive(Debug)]
pub enum NetworkChange<P: SignatureKey> {
    /// A node is connected
    NodeConnected(P),

    /// A node is disconnected
    NodeDisconnected(P),
}
