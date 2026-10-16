// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Network access compatibility
//!
//! Contains types and traits used by `HotShot` to abstract over network access

use std::{collections::HashMap, fmt::Display, hash::Hash};

use async_trait::async_trait;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc::error::TrySendError;

use super::{node_implementation::NodeType, signature_key::SignatureKey};
use crate::{
    BoxSyncFuture,
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::SequencingMessage,
};

/// the type of transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransmitType<TYPES: NodeType> {
    /// directly transmit
    Direct(TYPES::SignatureKey),
    /// broadcast the message to all
    Broadcast,
    /// broadcast to DA committee
    DaCommitteeBroadcast,
}

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

/// When a message should be broadcast to the network.
///
/// Network implementations may or may not respect this, at their discretion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BroadcastDelay {
    /// Broadcast the message immediately
    None,
    /// Delay the broadcast to a given view.
    View(u64),
}

/// represents a networking implmentration
/// exposes low level API for interacting with a network
/// intended to be implemented for a message transport,
/// and memory network
#[async_trait]
pub trait ConnectedNetwork<K: SignatureKey + 'static>: Clone + Send + Sync + 'static {
    /// Pauses the underlying network
    fn pause(&self);

    /// Resumes the underlying network
    fn resume(&self);

    /// Blocks until the network is successfully initialized
    async fn wait_for_ready(&self);

    /// Blocks until the network is shut down
    fn shut_down<'a, 'b>(&'a self) -> BoxSyncFuture<'b, ()>
    where
        'a: 'b,
        Self: 'b;

    /// broadcast message to some subset of nodes
    /// blocking
    async fn broadcast_message(
        &self,
        view: ViewNumber,
        message: Vec<u8>,
        topic: Topic,
        broadcast_delay: BroadcastDelay,
    ) -> Result<(), NetworkError>;

    /// broadcast a message only to a DA committee
    /// blocking
    async fn da_broadcast_message(
        &self,
        view: ViewNumber,
        message: Vec<u8>,
        recipients: Vec<K>,
        broadcast_delay: BroadcastDelay,
    ) -> Result<(), NetworkError>;

    /// send messages with vid shares to its recipients
    /// blocking
    async fn vid_broadcast_message(
        &self,
        messages: HashMap<K, (ViewNumber, Vec<u8>)>,
    ) -> Result<(), NetworkError> {
        let future_results = messages
            .into_iter()
            .map(|(recipient_key, (v, m))| self.direct_message(v, m, recipient_key));
        let results = join_all(future_results).await;

        let errors: Vec<_> = results.into_iter().filter_map(|r| r.err()).collect();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(NetworkError::Multiple(errors))
        }
    }

    /// Sends a direct message to a specific node
    /// blocking
    async fn direct_message(
        &self,
        view: ViewNumber,
        message: Vec<u8>,
        recipient: K,
    ) -> Result<(), NetworkError>;

    /// Receive one or many messages from the underlying network.
    ///
    /// # Errors
    /// If there is a network-related failure.
    async fn recv_message(&self) -> Result<Vec<u8>, NetworkError>;

    /// queues lookup of a node
    ///
    /// # Errors
    /// Does not error.
    fn queue_node_lookup(
        &self,
        _: ViewNumber,
        _: K,
    ) -> Result<(), TrySendError<Option<(ViewNumber, K)>>> {
        Ok(())
    }

    /// Update view can be used for any reason, but mostly it's for canceling tasks,
    /// and looking up the address of the leader of a future view.
    async fn update_view<TYPES>(
        &self,
        _: ViewNumber,
        _: Option<EpochNumber>,
        _: EpochMembershipCoordinator<TYPES>,
    ) where
        TYPES: NodeType<SignatureKey = K>,
    {
    }

    /// Is primary network down? Makes sense only for combined network
    fn is_primary_down(&self) -> bool {
        false
    }
}

/// Changes that can occur in the network
#[derive(Debug)]
pub enum NetworkChange<P: SignatureKey> {
    /// A node is connected
    NodeConnected(P),

    /// A node is disconnected
    NodeDisconnected(P),
}

/// Used when broadcasting messages
#[derive(Clone, Debug, PartialEq, Eq, Hash, Copy)]
pub enum Topic {
    /// The `Global` topic goes out to all nodes
    Global,
    /// The `Da` topic goes out to only the DA committee
    Da,
}

/// Topics travel as strings, so we need to convert our enum to a string
impl Display for Topic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Topic::Global => write!(f, "global"),
            Topic::Da => write!(f, "DA"),
        }
    }
}
