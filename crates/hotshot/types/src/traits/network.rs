// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Network access compatibility
//!
//! Contains types and traits used by `HotShot` to abstract over network access

use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    hash::Hash,
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use dyn_clone::DynClone;
use futures::{future::join_all, Future};
use rand::{
    distributions::{Bernoulli, Uniform},
    prelude::Distribution,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{sync::mpsc::error::TrySendError, time::sleep};

use super::{node_implementation::NodeType, signature_key::SignatureKey};
use crate::{
    data::ViewNumber, epoch_membership::EpochMembershipCoordinator, message::SequencingMessage,
    BoxSyncFuture,
};

/// Centralized server specific errors
#[derive(Debug, Error, Serialize, Deserialize)]
pub enum PushCdnNetworkError {}

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

    /// The network was not ready yet
    #[error("The network was not ready yet")]
    NotReadyYet,

    /// Failed to look up a node on the network
    #[error("Node lookup failed: {0}")]
    LookupError(String),
}

/// Trait that bundles what we need from a request ID
pub trait Id: Eq + PartialEq + Hash {}

/// a message
pub trait ViewMessage<TYPES: NodeType> {
    /// get the view out of the message
    fn view_number(&self) -> TYPES::View;
}

/// A request for some data that the consensus layer is asking for.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(bound(deserialize = ""))]
pub struct DataRequest<TYPES: NodeType> {
    /// Request
    pub request: RequestKind<TYPES>,
    /// View this message is for
    pub view: TYPES::View,
    /// signature of the Sha256 hash of the data so outsiders can't use know
    /// public keys with stake.
    pub signature: <TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
}

/// Underlying data request
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RequestKind<TYPES: NodeType> {
    /// Request VID data by our key and the VID commitment
    Vid(TYPES::View, TYPES::SignatureKey),
    /// Request a DA proposal for a certain view
    DaProposal(TYPES::View),
    /// Request for quorum proposal for a view
    Proposal(TYPES::View),
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

#[derive(Debug, Clone, PartialEq, Eq)]
/// When a message should be broadcast to the network.
///
/// Network implementations may or may not respect this, at their discretion.
pub enum BroadcastDelay {
    /// Broadcast the message immediately
    None,
    /// Delay the broadcast to a given view.
    View(u64),
}

#[async_trait]
/// represents a networking implmentration
/// exposes low level API for interacting with a network
/// intended to be implemented for libp2p, the centralized server,
/// and memory network
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
        message: Vec<u8>,
        topic: Topic,
        broadcast_delay: BroadcastDelay,
    ) -> Result<(), NetworkError>;

    /// broadcast a message only to a DA committee
    /// blocking
    async fn da_broadcast_message(
        &self,
        message: Vec<u8>,
        recipients: Vec<K>,
        broadcast_delay: BroadcastDelay,
    ) -> Result<(), NetworkError>;

    /// send messages with vid shares to its recipients
    /// blocking
    async fn vid_broadcast_message(
        &self,
        messages: HashMap<K, Vec<u8>>,
    ) -> Result<(), NetworkError> {
        let future_results = messages
            .into_iter()
            .map(|(recipient_key, message)| self.direct_message(message, recipient_key));
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
    async fn direct_message(&self, message: Vec<u8>, recipient: K) -> Result<(), NetworkError>;

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
        _view_number: ViewNumber,
        _pk: K,
    ) -> Result<(), TrySendError<Option<(ViewNumber, K)>>> {
        Ok(())
    }

    /// Update view can be used for any reason, but mostly it's for canceling tasks,
    /// and looking up the address of the leader of a future view.
    async fn update_view<'a, TYPES>(
        &'a self,
        _view: u64,
        _epoch: Option<u64>,
        _membership_coordinator: EpochMembershipCoordinator<TYPES>,
    ) where
        TYPES: NodeType<SignatureKey = K> + 'a,
    {
    }

    /// Is primary network down? Makes sense only for combined network
    fn is_primary_down(&self) -> bool {
        false
    }
}

/// A channel generator for types that need asynchronous execution
pub type AsyncGenerator<T> =
    Pin<Box<dyn Fn(u64) -> Pin<Box<dyn Future<Output = T> + Send>> + Send + Sync>>;

/// Describes additional functionality needed by the test network implementation
pub trait TestableNetworkingImplementation<TYPES: NodeType>
where
    Self: Sized,
{
    /// generates a network given an expected node count
    #[allow(clippy::type_complexity)]
    fn generator(
        expected_node_count: usize,
        num_bootstrap: usize,
        network_id: usize,
        da_committee_size: usize,
        reliability_config: Option<Box<dyn NetworkReliability>>,
        secondary_network_delay: Duration,
    ) -> AsyncGenerator<Arc<Self>>;

    /// Get the number of messages in-flight.
    ///
    /// Some implementations will not be able to tell how many messages there are in-flight. These implementations should return `None`.
    fn in_flight_message_count(&self) -> Option<usize>;
}

/// Changes that can occur in the network
#[derive(Debug)]
pub enum NetworkChange<P: SignatureKey> {
    /// A node is connected
    NodeConnected(P),

    /// A node is disconnected
    NodeDisconnected(P),
}

/// interface describing how reliable the network is
#[async_trait]
pub trait NetworkReliability: Debug + Sync + std::marker::Send + DynClone + 'static {
    /// Sample from bernoulli distribution to decide whether
    /// or not to keep a packet
    /// # Panics
    ///
    /// Panics if `self.keep_numerator > self.keep_denominator`
    ///
    fn sample_keep(&self) -> bool {
        true
    }

    /// sample from uniform distribution to decide whether
    /// or not to keep a packet
    fn sample_delay(&self) -> Duration {
        std::time::Duration::ZERO
    }

    /// scramble the packet
    fn scramble(&self, msg: Vec<u8>) -> Vec<u8> {
        msg
    }

    /// number of times to repeat the packet
    fn sample_repeat(&self) -> usize {
        1
    }

    /// given a message and a way to send the message,
    /// decide whether or not to send the message
    /// how long to delay the message
    /// whether or not to send duplicates
    /// and whether or not to include noise with the message
    /// then send the message
    /// note: usually self is stored in a rwlock
    /// so instead of doing the sending part, we just fiddle with the message
    /// then return a future that does the sending and delaying
    fn chaos_send_msg(
        &self,
        msg: Vec<u8>,
        send_fn: Arc<dyn Send + Sync + 'static + Fn(Vec<u8>) -> BoxSyncFuture<'static, ()>>,
    ) -> BoxSyncFuture<'static, ()> {
        let sample_keep = self.sample_keep();
        let delay = self.sample_delay();
        let repeats = self.sample_repeat();
        let mut msgs = Vec::new();
        for _idx in 0..repeats {
            let scrambled = self.scramble(msg.clone());
            msgs.push(scrambled);
        }
        let closure = async move {
            if sample_keep {
                sleep(delay).await;
                for msg in msgs {
                    send_fn(msg).await;
                }
            }
        };
        Box::pin(closure)
    }
}

// hack to get clone
dyn_clone::clone_trait_object!(NetworkReliability);

/// ideal network
#[derive(Clone, Copy, Debug, Default)]
pub struct PerfectNetwork {}

impl NetworkReliability for PerfectNetwork {}

/// A synchronous network. Packets may be delayed, but are guaranteed
/// to arrive within `timeout` ns
#[derive(Clone, Copy, Debug, Default)]
pub struct SynchronousNetwork {
    /// Max value in milliseconds that a packet may be delayed
    pub delay_high_ms: u64,
    /// Lowest value in milliseconds that a packet may be delayed
    pub delay_low_ms: u64,
}

impl NetworkReliability for SynchronousNetwork {
    /// never drop a packet
    fn sample_keep(&self) -> bool {
        true
    }
    fn sample_delay(&self) -> Duration {
        Duration::from_millis(
            Uniform::new_inclusive(self.delay_low_ms, self.delay_high_ms)
                .sample(&mut rand::thread_rng()),
        )
    }
}

/// An asynchronous network. Packets may be dropped entirely
/// or delayed for arbitrarily long periods
/// probability that packet is kept = `keep_numerator` / `keep_denominator`
/// packet delay is obtained by sampling from a uniform distribution
/// between `delay_low_ms` and `delay_high_ms`, inclusive
#[derive(Debug, Clone, Copy)]
pub struct AsynchronousNetwork {
    /// numerator for probability of keeping packets
    pub keep_numerator: u32,
    /// denominator for probability of keeping packets
    pub keep_denominator: u32,
    /// lowest value in milliseconds that a packet may be delayed
    pub delay_low_ms: u64,
    /// highest value in milliseconds that a packet may be delayed
    pub delay_high_ms: u64,
}

impl NetworkReliability for AsynchronousNetwork {
    fn sample_keep(&self) -> bool {
        Bernoulli::from_ratio(self.keep_numerator, self.keep_denominator)
            .unwrap()
            .sample(&mut rand::thread_rng())
    }
    fn sample_delay(&self) -> Duration {
        Duration::from_millis(
            Uniform::new_inclusive(self.delay_low_ms, self.delay_high_ms)
                .sample(&mut rand::thread_rng()),
        )
    }
}

/// An partially synchronous network. Behaves asynchronously
/// until some arbitrary time bound, GST,
/// then synchronously after GST
#[allow(clippy::similar_names)]
#[derive(Debug, Clone, Copy)]
pub struct PartiallySynchronousNetwork {
    /// asynchronous portion of network
    pub asynchronous: AsynchronousNetwork,
    /// synchronous portion of network
    pub synchronous: SynchronousNetwork,
    /// time when GST occurs
    pub gst: std::time::Duration,
    /// when the network was started
    pub start: std::time::Instant,
}

impl NetworkReliability for PartiallySynchronousNetwork {
    /// never drop a packet
    fn sample_keep(&self) -> bool {
        true
    }
    fn sample_delay(&self) -> Duration {
        // act asynchronous before gst
        if self.start.elapsed() < self.gst {
            if self.asynchronous.sample_keep() {
                self.asynchronous.sample_delay()
            } else {
                // assume packet was "dropped" and will arrive after gst
                self.synchronous.sample_delay() + self.gst
            }
        } else {
            // act synchronous after gst
            self.synchronous.sample_delay()
        }
    }
}

impl Default for AsynchronousNetwork {
    // disable all chance of failure
    fn default() -> Self {
        AsynchronousNetwork {
            keep_numerator: 1,
            keep_denominator: 1,
            delay_low_ms: 0,
            delay_high_ms: 0,
        }
    }
}

impl Default for PartiallySynchronousNetwork {
    fn default() -> Self {
        PartiallySynchronousNetwork {
            synchronous: SynchronousNetwork::default(),
            asynchronous: AsynchronousNetwork::default(),
            gst: std::time::Duration::new(0, 0),
            start: std::time::Instant::now(),
        }
    }
}

impl SynchronousNetwork {
    /// create new `SynchronousNetwork`
    #[must_use]
    pub fn new(timeout: u64, delay_low_ms: u64) -> Self {
        SynchronousNetwork {
            delay_high_ms: timeout,
            delay_low_ms,
        }
    }
}

impl AsynchronousNetwork {
    /// create new `AsynchronousNetwork`
    #[must_use]
    pub fn new(
        keep_numerator: u32,
        keep_denominator: u32,
        delay_low_ms: u64,
        delay_high_ms: u64,
    ) -> Self {
        AsynchronousNetwork {
            keep_numerator,
            keep_denominator,
            delay_low_ms,
            delay_high_ms,
        }
    }
}

impl PartiallySynchronousNetwork {
    /// create new `PartiallySynchronousNetwork`
    #[allow(clippy::similar_names)]
    #[must_use]
    pub fn new(
        asynchronous: AsynchronousNetwork,
        synchronous: SynchronousNetwork,
        gst: std::time::Duration,
    ) -> Self {
        PartiallySynchronousNetwork {
            asynchronous,
            synchronous,
            gst,
            start: std::time::Instant::now(),
        }
    }
}

/// A chaotic network using all the networking calls
#[derive(Debug, Clone)]
pub struct ChaosNetwork {
    /// numerator for probability of keeping packets
    pub keep_numerator: u32,
    /// denominator for probability of keeping packets
    pub keep_denominator: u32,
    /// lowest value in milliseconds that a packet may be delayed
    pub delay_low_ms: u64,
    /// highest value in milliseconds that a packet may be delayed
    pub delay_high_ms: u64,
    /// lowest value of repeats for a message
    pub repeat_low: usize,
    /// highest value of repeats for a message
    pub repeat_high: usize,
}

impl NetworkReliability for ChaosNetwork {
    fn sample_keep(&self) -> bool {
        Bernoulli::from_ratio(self.keep_numerator, self.keep_denominator)
            .unwrap()
            .sample(&mut rand::thread_rng())
    }

    fn sample_delay(&self) -> Duration {
        Duration::from_millis(
            Uniform::new_inclusive(self.delay_low_ms, self.delay_high_ms)
                .sample(&mut rand::thread_rng()),
        )
    }

    fn sample_repeat(&self) -> usize {
        Uniform::new_inclusive(self.repeat_low, self.repeat_high).sample(&mut rand::thread_rng())
    }
}

/// Used when broadcasting messages
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Topic {
    /// The `Global` topic goes out to all nodes
    Global,
    /// The `Da` topic goes out to only the DA committee
    Da,
}

/// Libp2p topics require a string, so we need to convert our enum to a string
impl Display for Topic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Topic::Global => write!(f, "global"),
            Topic::Da => write!(f, "DA"),
        }
    }
}
