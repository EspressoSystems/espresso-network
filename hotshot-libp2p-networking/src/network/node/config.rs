// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{collections::HashSet, num::NonZeroUsize, sync::Arc, time::Duration};

use async_lock::RwLock;
use hotshot_types::traits::node_implementation::NodeType;
use libp2p::{identity::Keypair, Multiaddr};
use libp2p_identity::PeerId;

use super::MAX_GOSSIP_MSG_SIZE;

/// The default Kademlia replication factor
pub const DEFAULT_REPLICATION_FACTOR: Option<NonZeroUsize> = NonZeroUsize::new(10);

/// describe the configuration of the network
#[derive(Default, derive_builder::Builder, derive_more::Debug)]
pub struct NetworkNodeConfig<T: NodeType> {
    /// The keypair for the node
    #[builder(setter(into, strip_option), default)]
    #[debug(skip)]
    pub keypair: Option<Keypair>,

    /// The address to bind to
    #[builder(default)]
    pub bind_address: Option<Multiaddr>,

    /// Replication factor for entries in the DHT
    #[builder(setter(into, strip_option), default = "DEFAULT_REPLICATION_FACTOR")]
    pub replication_factor: Option<NonZeroUsize>,

    #[builder(default)]
    /// Configuration for `GossipSub`
    pub gossip_config: GossipConfig,

    #[builder(default)]
    /// Configuration for `RequestResponse`
    pub request_response_config: RequestResponseConfig,

    /// list of addresses to connect to at initialization
    pub to_connect_addrs: HashSet<(PeerId, Multiaddr)>,

    /// republication interval in DHT, must be much less than `ttl`
    #[builder(default)]
    pub republication_interval: Option<Duration>,

    /// expiratiry for records in DHT
    #[builder(default)]
    pub ttl: Option<Duration>,

    /// The stake table. Used for authenticating other nodes. If not supplied
    /// we will not check other nodes against the stake table
    #[builder(default)]
    pub membership: Option<Arc<RwLock<T::Membership>>>,

    /// The path to the file to save the DHT to
    #[builder(default)]
    pub dht_file_path: Option<String>,

    /// The signed authentication message sent to the remote peer
    /// If not supplied we will not send an authentication message during the handshake
    #[builder(default)]
    pub auth_message: Option<Vec<u8>>,

    #[builder(default)]
    /// The timeout for DHT lookups.
    pub dht_timeout: Option<Duration>,
}

impl<T: NodeType> Clone for NetworkNodeConfig<T> {
    fn clone(&self) -> Self {
        Self {
            keypair: self.keypair.clone(),
            bind_address: self.bind_address.clone(),
            replication_factor: self.replication_factor,
            gossip_config: self.gossip_config.clone(),
            request_response_config: self.request_response_config.clone(),
            to_connect_addrs: self.to_connect_addrs.clone(),
            republication_interval: self.republication_interval,
            ttl: self.ttl,
            membership: self.membership.as_ref().map(Arc::clone),
            dht_file_path: self.dht_file_path.clone(),
            auth_message: self.auth_message.clone(),
            dht_timeout: self.dht_timeout,
        }
    }
}

/// Configuration for Libp2p's Gossipsub
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct GossipConfig {
    /// The heartbeat interval
    pub heartbeat_interval: Duration,

    /// The number of past heartbeats to gossip about
    pub history_gossip: usize,
    /// The number of past heartbeats to remember the full messages for
    pub history_length: usize,

    /// The target number of peers in the mesh
    pub mesh_n: usize,
    /// The maximum number of peers in the mesh
    pub mesh_n_high: usize,
    /// The minimum number of peers in the mesh
    pub mesh_n_low: usize,
    /// The minimum number of mesh peers that must be outbound
    pub mesh_outbound_min: usize,

    /// The maximum gossip message size
    pub max_transmit_size: usize,

    /// The maximum number of messages in an IHAVE message
    pub max_ihave_length: usize,

    /// Maximum number of IHAVE messages to accept from a peer within a heartbeat
    pub max_ihave_messages: usize,

    /// Cache duration for published message IDs
    pub published_message_ids_cache_time: Duration,

    /// Time to wait for a message requested through IWANT following an IHAVE advertisement
    pub iwant_followup_time: Duration,

    /// The maximum number of messages we will process in a given RPC
    pub max_messages_per_rpc: Option<usize>,

    /// Controls how many times we will allow a peer to request the same message id through IWANT gossip before we start ignoring them.
    pub gossip_retransmission: u32,

    /// If enabled newly created messages will always be sent to all peers that are subscribed to the topic and have a good enough score.
    pub flood_publish: bool,

    /// The time period that messages are stored in the cache
    pub duplicate_cache_time: Duration,

    /// Time to live for fanout peers
    pub fanout_ttl: Duration,

    /// Initial delay in each heartbeat
    pub heartbeat_initial_delay: Duration,

    /// Affects how many peers we will emit gossip to at each heartbeat
    pub gossip_factor: f64,

    /// Minimum number of peers to emit gossip to during a heartbeat
    pub gossip_lazy: usize,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(1), // Default of Libp2p

            // The following are slightly modified defaults of Libp2p
            history_gossip: 6, // The number of past heartbeats to gossip about
            history_length: 8, // The number of past heartbeats to remember the full messages for

            // The mesh parameters are borrowed from Ethereum:
            // https://github.com/ethereum/consensus-specs/blob/dev/specs/phase0/p2p-interface.md#the-gossip-domain-gossipsub
            mesh_n: 8,            // The target number of peers in the mesh
            mesh_n_high: 12,      // The maximum number of peers in the mesh
            mesh_n_low: 6,        // The minimum number of peers in the mesh
            mesh_outbound_min: 2, // The minimum number of mesh peers that must be outbound

            max_ihave_length: 5000,
            max_ihave_messages: 10,
            published_message_ids_cache_time: Duration::from_secs(60 * 20), // 20 minutes
            iwant_followup_time: Duration::from_secs(3),
            max_messages_per_rpc: None,
            gossip_retransmission: 3,
            flood_publish: true,
            duplicate_cache_time: Duration::from_secs(60),
            fanout_ttl: Duration::from_secs(60),
            heartbeat_initial_delay: Duration::from_secs(5),
            gossip_factor: 0.25,
            gossip_lazy: 6,

            max_transmit_size: MAX_GOSSIP_MSG_SIZE, // The maximum gossip message size
        }
    }
}

/// Configuration for Libp2p's request-response
#[derive(Clone, Debug)]
pub struct RequestResponseConfig {
    /// The maximum request size in bytes
    pub request_size_maximum: u64,
    /// The maximum response size in bytes
    pub response_size_maximum: u64,
}

impl Default for RequestResponseConfig {
    fn default() -> Self {
        Self {
            request_size_maximum: 20 * 1024 * 1024,
            response_size_maximum: 20 * 1024 * 1024,
        }
    }
}
