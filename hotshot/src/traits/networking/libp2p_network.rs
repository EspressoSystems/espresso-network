// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Libp2p based/production networking implementation
//! This module provides a libp2p based networking implementation where each node in the
//! network forms a tcp or udp connection to a subset of other nodes in the network
#[cfg(feature = "hotshot-testing")]
use std::str::FromStr;
use std::{
    cmp::min,
    collections::{BTreeSet, HashSet},
    fmt::Debug,
    net::{IpAddr, ToSocketAddrs},
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::{anyhow, Context};
use async_lock::RwLock;
use async_trait::async_trait;
use bimap::BiHashMap;
use futures::future::join_all;
#[cfg(feature = "hotshot-testing")]
use hotshot_libp2p_networking::network::behaviours::dht::store::persistent::DhtNoPersistence;
pub use hotshot_libp2p_networking::network::{GossipConfig, RequestResponseConfig};
use hotshot_libp2p_networking::{
    network::{
        behaviours::dht::{
            record::{Namespace, RecordKey, RecordValue},
            store::persistent::DhtPersistentStorage,
        },
        spawn_network_node,
        transport::construct_auth_message,
        NetworkEvent::{self, DirectRequest, DirectResponse, GossipMsg},
        NetworkNodeConfig, NetworkNodeConfigBuilder, NetworkNodeHandle, NetworkNodeReceiver,
        DEFAULT_REPLICATION_FACTOR,
    },
    reexport::Multiaddr,
};
#[cfg(feature = "hotshot-testing")]
use hotshot_types::traits::network::{
    AsyncGenerator, NetworkReliability, TestableNetworkingImplementation,
};
use hotshot_types::{
    boxed_sync,
    constants::LOOK_AHEAD,
    data::ViewNumber,
    network::NetworkConfig,
    traits::{
        metrics::{Counter, Gauge, Metrics, NoMetrics},
        network::{ConnectedNetwork, NetworkError, Topic},
        node_implementation::{ConsensusTime, NodeType},
        signature_key::{PrivateSignatureKey, SignatureKey},
    },
    BoxSyncFuture,
};
use libp2p_identity::{
    ed25519::{self, SecretKey},
    Keypair, PeerId,
};
use rand::{rngs::StdRng, seq::IteratorRandom, SeedableRng};
use serde::Serialize;
use tokio::{
    select, spawn,
    sync::{
        mpsc::{channel, error::TrySendError, Receiver, Sender},
        Mutex,
    },
    time::sleep,
};
use tracing::{error, info, instrument, trace, warn};

use crate::{BroadcastDelay, EpochMembershipCoordinator};

/// Libp2p-specific metrics
#[derive(Clone, Debug)]
pub struct Libp2pMetricsValue {
    /// The number of currently connected peers
    pub num_connected_peers: Box<dyn Gauge>,
    /// The number of failed messages
    pub num_failed_messages: Box<dyn Counter>,
    /// Whether or not the network is considered ready
    pub is_ready: Box<dyn Gauge>,
}

impl Libp2pMetricsValue {
    /// Populate the metrics with Libp2p-specific metrics
    pub fn new(metrics: &dyn Metrics) -> Self {
        // Create a `libp2p subgroup
        let subgroup = metrics.subgroup("libp2p".into());

        // Create the metrics
        Self {
            num_connected_peers: subgroup.create_gauge("num_connected_peers".into(), None),
            num_failed_messages: subgroup.create_counter("num_failed_messages".into(), None),
            is_ready: subgroup.create_gauge("is_ready".into(), None),
        }
    }
}

impl Default for Libp2pMetricsValue {
    /// Initialize with empty metrics
    fn default() -> Self {
        Self::new(&*NoMetrics::boxed())
    }
}

/// convenience alias for the type for bootstrap addresses
/// concurrency primitives are needed for having tests
pub type BootstrapAddrs = Arc<RwLock<Vec<(PeerId, Multiaddr)>>>;

/// hardcoded topic of QC used
pub const QC_TOPIC: &str = "global";

/// Stubbed out Ack
///
/// Note: as part of versioning for upgradability,
/// all network messages must begin with a 4-byte version number.
///
/// Hence:
///   * `Empty` *must* be a struct (enums are serialized with a leading byte for the variant), and
///   * we must have an explicit version field.
#[derive(Serialize)]
pub struct Empty {
    /// This should not be required, but it is. Version automatically gets prepended.
    /// Perhaps this could be replaced with something zero-sized and serializable.
    byte: u8,
}

impl<T: NodeType> Debug for Libp2pNetwork<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Libp2p").field("inner", &"inner").finish()
    }
}

/// Type alias for a shared collection of peerid, multiaddrs
pub type PeerInfoVec = Arc<RwLock<Vec<(PeerId, Multiaddr)>>>;

/// The underlying state of the libp2p network
#[derive(Debug)]
struct Libp2pNetworkInner<T: NodeType> {
    /// this node's public key
    pk: T::SignatureKey,
    /// handle to control the network
    handle: Arc<NetworkNodeHandle<T>>,
    /// Message Receiver
    receiver: Mutex<Receiver<Vec<u8>>>,
    /// Sender for broadcast messages
    sender: Sender<Vec<u8>>,
    /// Sender for node lookup (relevant view number, key of node) (None for shutdown)
    node_lookup_send: Sender<Option<(ViewNumber, T::SignatureKey)>>,
    /// this is really cheating to enable local tests
    /// hashset of (bootstrap_addr, peer_id)
    bootstrap_addrs: PeerInfoVec,
    /// whether or not the network is ready to send
    is_ready: Arc<AtomicBool>,
    /// max time before dropping message due to DHT error
    dht_timeout: Duration,
    /// whether or not we've bootstrapped into the DHT yet
    is_bootstrapped: Arc<AtomicBool>,
    /// The Libp2p metrics we're managing
    metrics: Libp2pMetricsValue,
    /// The list of topics we're subscribed to
    subscribed_topics: HashSet<String>,
    /// the latest view number (for node lookup purposes)
    /// NOTE: supposed to represent a ViewNumber but we
    /// haven't made that atomic yet and we prefer lock-free
    latest_seen_view: Arc<AtomicU64>,
    #[cfg(feature = "hotshot-testing")]
    /// reliability_config
    reliability_config: Option<Box<dyn NetworkReliability>>,
    /// Killswitch sender
    kill_switch: Sender<()>,
}

/// Networking implementation that uses libp2p
/// generic over `M` which is the message type
#[derive(Clone)]
pub struct Libp2pNetwork<T: NodeType> {
    /// holds the state of the libp2p network
    inner: Arc<Libp2pNetworkInner<T>>,
}

#[cfg(feature = "hotshot-testing")]
impl<T: NodeType> TestableNetworkingImplementation<T> for Libp2pNetwork<T> {
    /// Returns a boxed function `f(node_id, public_key) -> Libp2pNetwork`
    /// with the purpose of generating libp2p networks.
    /// Generates `num_bootstrap` bootstrap nodes. The remainder of nodes are normal
    /// nodes with sane defaults.
    /// # Panics
    /// Returned function may panic either:
    /// - An invalid configuration
    ///   (probably an issue with the defaults of this function)
    /// - An inability to spin up the replica's network
    #[allow(clippy::panic, clippy::too_many_lines)]
    fn generator(
        expected_node_count: usize,
        num_bootstrap: usize,
        _network_id: usize,
        da_committee_size: usize,
        reliability_config: Option<Box<dyn NetworkReliability>>,
        _secondary_network_delay: Duration,
    ) -> AsyncGenerator<Arc<Self>> {
        assert!(
            da_committee_size <= expected_node_count,
            "DA committee size must be less than or equal to total # nodes"
        );
        let bootstrap_addrs: PeerInfoVec = Arc::default();
        let node_ids: Arc<RwLock<HashSet<u64>>> = Arc::default();

        // NOTE uncomment this for easier debugging
        // let start_port = 5000;
        Box::pin({
            move |node_id| {
                info!(
                    "GENERATOR: Node id {:?}, is bootstrap: {:?}",
                    node_id,
                    node_id < num_bootstrap as u64
                );

                // pick a free, unused UDP port for testing
                let port = portpicker::pick_unused_port().expect("Could not find an open port");

                let addr =
                    Multiaddr::from_str(&format!("/ip4/127.0.0.1/udp/{port}/quic-v1")).unwrap();

                // We assign node's public key and stake value rather than read from config file since it's a test
                let privkey = T::SignatureKey::generated_from_seed_indexed([0u8; 32], node_id).1;
                let pubkey = T::SignatureKey::from_private(&privkey);

                // Derive the Libp2p keypair from the private key
                let libp2p_keypair = derive_libp2p_keypair::<T::SignatureKey>(&privkey)
                    .expect("Failed to derive libp2p keypair");

                // Sign the lookup record
                let lookup_record_value = RecordValue::new_signed(
                    &RecordKey::new(Namespace::Lookup, pubkey.to_bytes()),
                    libp2p_keypair.public().to_peer_id().to_bytes(),
                    &privkey,
                )
                .expect("Failed to sign DHT lookup record");

                // We want at least 2/3 of the nodes to have any given record in the DHT
                let replication_factor =
                    NonZeroUsize::new((2 * expected_node_count).div_ceil(3)).unwrap();

                // Build the network node configuration
                let config = NetworkNodeConfigBuilder::default()
                    .keypair(libp2p_keypair)
                    .replication_factor(replication_factor)
                    .bind_address(Some(addr))
                    .to_connect_addrs(HashSet::default())
                    .republication_interval(None)
                    .build()
                    .expect("Failed to build network node config");

                let bootstrap_addrs_ref = Arc::clone(&bootstrap_addrs);
                let node_ids_ref = Arc::clone(&node_ids);
                let reliability_config_dup = reliability_config.clone();

                Box::pin(async move {
                    // If it's the second time we are starting this network, clear the bootstrap info
                    let mut write_ids = node_ids_ref.write().await;
                    if write_ids.contains(&node_id) {
                        write_ids.clear();
                        bootstrap_addrs_ref.write().await.clear();
                    }
                    write_ids.insert(node_id);
                    drop(write_ids);
                    Arc::new(
                        match Libp2pNetwork::new(
                            Libp2pMetricsValue::default(),
                            DhtNoPersistence,
                            config,
                            pubkey.clone(),
                            lookup_record_value,
                            bootstrap_addrs_ref,
                            usize::try_from(node_id).unwrap(),
                            #[cfg(feature = "hotshot-testing")]
                            reliability_config_dup,
                        )
                        .await
                        {
                            Ok(network) => network,
                            Err(err) => {
                                panic!("Failed to create libp2p network: {err:?}");
                            },
                        },
                    )
                })
            }
        })
    }

    fn in_flight_message_count(&self) -> Option<usize> {
        None
    }
}

/// Derive a Libp2p keypair from a given private key
///
/// # Errors
/// If we are unable to derive a new `SecretKey` from the `blake3`-derived
/// bytes.
pub fn derive_libp2p_keypair<K: SignatureKey>(
    private_key: &K::PrivateKey,
) -> anyhow::Result<Keypair> {
    // Derive a secondary key from our primary private key
    let derived_key = blake3::derive_key("libp2p key", &private_key.to_bytes());
    let derived_key = SecretKey::try_from_bytes(derived_key)?;

    // Create an `ed25519` keypair from the derived key
    Ok(ed25519::Keypair::from(derived_key).into())
}

/// Derive a Libp2p Peer ID from a given private key
///
/// # Errors
/// If we are unable to derive a Libp2p keypair
pub fn derive_libp2p_peer_id<K: SignatureKey>(
    private_key: &K::PrivateKey,
) -> anyhow::Result<PeerId> {
    // Get the derived keypair
    let keypair = derive_libp2p_keypair::<K>(private_key)?;

    // Return the PeerID derived from the public key
    Ok(PeerId::from_public_key(&keypair.public()))
}

/// Parse a Libp2p Multiaddr from a string. The input string should be in the format
/// `hostname:port` or `ip:port`. This function derives a `Multiaddr` from the input string.
///
/// This borrows from Rust's implementation of `to_socket_addrs` but will only warn if the domain
/// does not yet resolve.
///
/// # Errors
/// - If the input string is not in the correct format
pub fn derive_libp2p_multiaddr(addr: &String) -> anyhow::Result<Multiaddr> {
    // Split the address into the host and port parts
    let (host, port) = match addr.rfind(':') {
        Some(idx) => (&addr[..idx], &addr[idx + 1..]),
        None => return Err(anyhow!("Invalid address format, no port supplied")),
    };

    // Try parsing the host as an IP address
    let ip = host.parse::<IpAddr>();

    // Conditionally build the multiaddr string
    let multiaddr_string = match ip {
        Ok(IpAddr::V4(ip)) => format!("/ip4/{ip}/udp/{port}/quic-v1"),
        Ok(IpAddr::V6(ip)) => format!("/ip6/{ip}/udp/{port}/quic-v1"),
        Err(_) => {
            // Try resolving the host. If it fails, continue but warn the user
            let lookup_result = addr.to_socket_addrs();

            // See if the lookup failed
            let failed = lookup_result
                .map(|result| result.collect::<Vec<_>>().is_empty())
                .unwrap_or(true);

            // If it did, warn the user
            if failed {
                warn!(
                    "Failed to resolve domain name {}, assuming it has not yet been provisioned",
                    host
                );
            }

            format!("/dns/{host}/udp/{port}/quic-v1")
        },
    };

    // Convert the multiaddr string to a `Multiaddr`
    multiaddr_string.parse().with_context(|| {
        format!("Failed to convert Multiaddr string to Multiaddr: {multiaddr_string}",)
    })
}

impl<T: NodeType> Libp2pNetwork<T> {
    /// Create and return a Libp2p network from a network config file
    /// and various other configuration-specific values.
    ///
    /// # Errors
    /// If we are unable to parse a Multiaddress
    ///
    /// # Panics
    /// If we are unable to calculate the replication factor
    #[allow(clippy::too_many_arguments)]
    pub async fn from_config<D: DhtPersistentStorage>(
        mut config: NetworkConfig<T>,
        dht_persistent_storage: D,
        quorum_membership: Arc<RwLock<T::Membership>>,
        gossip_config: GossipConfig,
        request_response_config: RequestResponseConfig,
        bind_address: Multiaddr,
        pub_key: &T::SignatureKey,
        priv_key: &<T::SignatureKey as SignatureKey>::PrivateKey,
        metrics: Libp2pMetricsValue,
    ) -> anyhow::Result<Self> {
        // Try to take our Libp2p config from our broader network config
        let libp2p_config = config
            .libp2p_config
            .take()
            .ok_or(anyhow!("Libp2p config not supplied"))?;

        // Derive our Libp2p keypair from our supplied private key
        let keypair = derive_libp2p_keypair::<T::SignatureKey>(priv_key)?;

        // Build our libp2p configuration
        let mut config_builder = NetworkNodeConfigBuilder::default();

        // Set the gossip configuration
        config_builder.gossip_config(gossip_config.clone());
        config_builder.request_response_config(request_response_config);

        // Construct the auth message
        let auth_message =
            construct_auth_message(pub_key, &keypair.public().to_peer_id(), priv_key)
                .with_context(|| "Failed to construct auth message")?;

        // Set the auth message and stake table
        config_builder
            .membership(Some(quorum_membership))
            .auth_message(Some(auth_message));

        // The replication factor is the minimum of [the default and 2/3 the number of nodes]
        let Some(default_replication_factor) = DEFAULT_REPLICATION_FACTOR else {
            return Err(anyhow!("Default replication factor not supplied"));
        };

        let replication_factor = NonZeroUsize::new(min(
            default_replication_factor.get(),
            config.config.num_nodes_with_stake.get() * 2 / 3,
        ))
        .with_context(|| "Failed to calculate replication factor")?;

        // Sign our DHT lookup record
        let lookup_record_value = RecordValue::new_signed(
            &RecordKey::new(Namespace::Lookup, pub_key.to_bytes()),
            // The value is our Libp2p Peer ID
            keypair.public().to_peer_id().to_bytes(),
            priv_key,
        )
        .with_context(|| "Failed to sign DHT lookup record")?;

        config_builder
            .keypair(keypair)
            .replication_factor(replication_factor)
            .bind_address(Some(bind_address.clone()));

        // Choose `mesh_n` random nodes to connect to for bootstrap
        let bootstrap_nodes = libp2p_config
            .bootstrap_nodes
            .into_iter()
            .choose_multiple(&mut StdRng::from_entropy(), gossip_config.mesh_n);
        config_builder.to_connect_addrs(HashSet::from_iter(bootstrap_nodes.clone()));

        // Build the node's configuration
        let node_config = config_builder.build()?;

        // Calculate all keys so we can keep track of direct message recipients
        let mut all_keys = BTreeSet::new();

        // Insert all known nodes into the set of all keys
        for node in config.config.known_nodes_with_stake {
            all_keys.insert(T::SignatureKey::public_key(&node.stake_table_entry));
        }

        Ok(Libp2pNetwork::new(
            metrics,
            dht_persistent_storage,
            node_config,
            pub_key.clone(),
            lookup_record_value,
            Arc::new(RwLock::new(bootstrap_nodes)),
            usize::try_from(config.node_index)?,
            #[cfg(feature = "hotshot-testing")]
            None,
        )
        .await?)
    }

    /// Returns whether or not the network is currently ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready.load(Ordering::Relaxed)
    }

    /// Returns only when the network is ready.
    pub async fn wait_for_ready(&self) {
        loop {
            if self.is_ready() {
                break;
            }
            sleep(Duration::from_secs(1)).await;
        }
    }

    /// Constructs new network for a node. Note that this network is unconnected.
    /// One must call `connect` in order to connect.
    /// * `config`: the configuration of the node
    /// * `pk`: public key associated with the node
    /// * `bootstrap_addrs`: rwlock containing the bootstrap addrs
    /// # Errors
    /// Returns error in the event that the underlying libp2p network
    /// is unable to create a network.
    ///
    /// # Panics
    ///
    /// This will panic if there are less than 5 bootstrap nodes
    #[allow(clippy::too_many_arguments)]
    pub async fn new<D: DhtPersistentStorage>(
        metrics: Libp2pMetricsValue,
        dht_persistent_storage: D,
        config: NetworkNodeConfig<T>,
        pk: T::SignatureKey,
        lookup_record_value: RecordValue<T::SignatureKey>,
        bootstrap_addrs: BootstrapAddrs,
        id: usize,
        #[cfg(feature = "hotshot-testing")] reliability_config: Option<Box<dyn NetworkReliability>>,
    ) -> Result<Libp2pNetwork<T>, NetworkError> {
        let (mut rx, network_handle) =
            spawn_network_node::<T, D>(config.clone(), dht_persistent_storage, id)
                .await
                .map_err(|e| {
                    NetworkError::ConfigError(format!("failed to spawn network node: {e}"))
                })?;

        // Add our own address to the bootstrap addresses
        let addr = network_handle.listen_addr();
        let pid = network_handle.peer_id();
        bootstrap_addrs.write().await.push((pid, addr));

        let mut pubkey_pid_map = BiHashMap::new();
        pubkey_pid_map.insert(pk.clone(), network_handle.peer_id());

        // Subscribe to the relevant topics
        let subscribed_topics = HashSet::from_iter(vec![QC_TOPIC.to_string()]);

        // unbounded channels may not be the best choice (spammed?)
        // if bounded figure out a way to log dropped msgs
        let (sender, receiver) = channel(1000);
        let (node_lookup_send, node_lookup_recv) = channel(10);
        let (kill_tx, kill_rx) = channel(1);
        rx.set_kill_switch(kill_rx);

        let mut result = Libp2pNetwork {
            inner: Arc::new(Libp2pNetworkInner {
                handle: Arc::new(network_handle),
                receiver: Mutex::new(receiver),
                sender: sender.clone(),
                pk,
                bootstrap_addrs,
                is_ready: Arc::new(AtomicBool::new(false)),
                // This is optimal for 10-30 nodes. TODO: parameterize this for both tests and examples
                dht_timeout: config.dht_timeout.unwrap_or(Duration::from_secs(120)),
                is_bootstrapped: Arc::new(AtomicBool::new(false)),
                metrics,
                subscribed_topics,
                node_lookup_send,
                // Start the latest view from 0. "Latest" refers to "most recent view we are polling for
                // proposals on". We need this because to have consensus info injected we need a working
                // network already. In the worst case, we send a few lookups we don't need.
                latest_seen_view: Arc::new(AtomicU64::new(0)),
                #[cfg(feature = "hotshot-testing")]
                reliability_config,
                kill_switch: kill_tx,
            }),
        };

        // Set the network as not ready
        result.inner.metrics.is_ready.set(0);

        result.handle_event_generator(sender, rx);
        result.spawn_node_lookup(node_lookup_recv);
        result.spawn_connect(id, lookup_record_value);

        Ok(result)
    }

    /// Spawns task for looking up nodes pre-emptively
    #[allow(clippy::cast_sign_loss, clippy::cast_precision_loss)]
    fn spawn_node_lookup(
        &self,
        mut node_lookup_recv: Receiver<Option<(ViewNumber, T::SignatureKey)>>,
    ) {
        let handle = Arc::clone(&self.inner.handle);
        let dht_timeout = self.inner.dht_timeout;
        let latest_seen_view = Arc::clone(&self.inner.latest_seen_view);

        // deals with handling lookup queue. should be infallible
        spawn(async move {
            // cancels on shutdown
            while let Some(Some((view_number, pk))) = node_lookup_recv.recv().await {
                /// defines lookahead threshold based on the constant
                #[allow(clippy::cast_possible_truncation)]
                const THRESHOLD: u64 = (LOOK_AHEAD as f64 * 0.8) as u64;

                trace!("Performing lookup for peer {:?}", pk);

                // only run if we are not too close to the next view number
                if latest_seen_view.load(Ordering::Relaxed) + THRESHOLD <= *view_number {
                    // look up
                    if let Err(err) = handle.lookup_node(&pk.to_bytes(), dht_timeout).await {
                        warn!("Failed to perform lookup for key {:?}: {}", pk, err);
                    };
                }
            }
        });
    }

    /// Initiates connection to the outside world
    fn spawn_connect(&mut self, id: usize, lookup_record_value: RecordValue<T::SignatureKey>) {
        let pk = self.inner.pk.clone();
        let bootstrap_ref = Arc::clone(&self.inner.bootstrap_addrs);
        let handle = Arc::clone(&self.inner.handle);
        let is_bootstrapped = Arc::clone(&self.inner.is_bootstrapped);
        let inner = Arc::clone(&self.inner);

        spawn({
            let is_ready = Arc::clone(&self.inner.is_ready);
            async move {
                let bs_addrs = bootstrap_ref.read().await.clone();

                // Add known peers to the network
                handle.add_known_peers(bs_addrs).unwrap();

                // Begin the bootstrap process
                handle.begin_bootstrap()?;
                while !is_bootstrapped.load(Ordering::Relaxed) {
                    sleep(Duration::from_secs(1)).await;
                    handle.begin_bootstrap()?;
                }

                // Subscribe to the QC topic
                handle.subscribe(QC_TOPIC.to_string()).await.unwrap();

                // Map our staking key to our Libp2p Peer ID so we can properly
                // route direct messages
                while handle
                    .put_record(
                        RecordKey::new(Namespace::Lookup, pk.to_bytes()),
                        lookup_record_value.clone(),
                    )
                    .await
                    .is_err()
                {
                    sleep(Duration::from_secs(1)).await;
                }

                // Wait for the network to connect to the required number of peers
                if let Err(e) = handle.wait_to_connect(4, id).await {
                    error!("Failed to connect to peers: {:?}", e);
                    return Err::<(), NetworkError>(e);
                }
                info!("Connected to required number of peers");

                // Set the network as ready
                is_ready.store(true, Ordering::Relaxed);
                inner.metrics.is_ready.set(1);

                Ok::<(), NetworkError>(())
            }
        });
    }

    /// Handle events
    fn handle_recvd_events(
        &self,
        msg: NetworkEvent,
        sender: &Sender<Vec<u8>>,
    ) -> Result<(), NetworkError> {
        match msg {
            GossipMsg(msg) => {
                sender.try_send(msg).map_err(|err| {
                    NetworkError::ChannelSendError(format!("failed to send gossip message: {err}"))
                })?;
            },
            DirectRequest(msg, _pid, chan) => {
                sender.try_send(msg).map_err(|err| {
                    NetworkError::ChannelSendError(format!(
                        "failed to send direct request message: {err}"
                    ))
                })?;
                if self
                    .inner
                    .handle
                    .direct_response(
                        chan,
                        &bincode::serialize(&Empty { byte: 0u8 }).map_err(|e| {
                            NetworkError::FailedToSerialize(format!(
                                "failed to serialize acknowledgement: {e}"
                            ))
                        })?,
                    )
                    .is_err()
                {
                    error!("failed to ack!");
                };
            },
            DirectResponse(_msg, _) => {},
            NetworkEvent::IsBootstrapped => {
                error!("handle_recvd_events received `NetworkEvent::IsBootstrapped`, which should be impossible.");
            },
            NetworkEvent::ConnectedPeersUpdate(_) => {},
        }
        Ok::<(), NetworkError>(())
    }

    /// task to propagate messages to handlers
    /// terminates on shut down of network
    fn handle_event_generator(&self, sender: Sender<Vec<u8>>, mut network_rx: NetworkNodeReceiver) {
        let handle = self.clone();
        let is_bootstrapped = Arc::clone(&self.inner.is_bootstrapped);
        spawn(async move {
            let Some(mut kill_switch) = network_rx.take_kill_switch() else {
                tracing::error!(
                    "`spawn_handle` was called on a network handle that was already closed"
                );
                return;
            };

            loop {
                select! {
                    msg = network_rx.recv() => {
                        let Ok(message) = msg else {
                            warn!("Network receiver shut down!");
                            return;
                        };

                        match message {
                            NetworkEvent::IsBootstrapped => {
                                is_bootstrapped.store(true, Ordering::Relaxed);
                            }
                            GossipMsg(_) | DirectRequest(_, _, _) | DirectResponse(_, _) => {
                                let _ = handle.handle_recvd_events(message, &sender);
                            }
                            NetworkEvent::ConnectedPeersUpdate(num_peers) => {
                                handle.inner.metrics.num_connected_peers.set(num_peers);
                            }
                        }
                    }

                    _kill_switch = kill_switch.recv() => {
                        warn!("Event Handler shutdown");
                        return;
                    }
                }
            }
        });
    }
}

#[async_trait]
impl<T: NodeType> ConnectedNetwork<T::SignatureKey> for Libp2pNetwork<T> {
    #[instrument(name = "Libp2pNetwork::ready_blocking", skip_all)]
    async fn wait_for_ready(&self) {
        self.wait_for_ready().await;
    }

    fn pause(&self) {
        unimplemented!("Pausing not implemented for the Libp2p network");
    }

    fn resume(&self) {
        unimplemented!("Resuming not implemented for the Libp2p network");
    }

    #[instrument(name = "Libp2pNetwork::shut_down", skip_all)]
    fn shut_down<'a, 'b>(&'a self) -> BoxSyncFuture<'b, ()>
    where
        'a: 'b,
        Self: 'b,
    {
        let closure = async move {
            let _ = self.inner.handle.shutdown().await;
            let _ = self.inner.node_lookup_send.send(None).await;
            let _ = self.inner.kill_switch.send(()).await;
        };
        boxed_sync(closure)
    }

    #[instrument(name = "Libp2pNetwork::broadcast_message", skip_all)]
    async fn broadcast_message(
        &self,
        message: Vec<u8>,
        topic: Topic,
        _broadcast_delay: BroadcastDelay,
    ) -> Result<(), NetworkError> {
        // If we're not ready, return an error
        if !self.is_ready() {
            self.inner.metrics.num_failed_messages.add(1);
            return Err(NetworkError::NotReadyYet);
        };

        // If we are subscribed to the topic,
        let topic = topic.to_string();
        if self.inner.subscribed_topics.contains(&topic) {
            // Short-circuit-send the message to ourselves
            self.inner.sender.try_send(message.clone()).map_err(|_| {
                self.inner.metrics.num_failed_messages.add(1);
                NetworkError::ShutDown
            })?;
        }

        // NOTE: metrics is threadsafe, so clone is fine (and lightweight)
        #[cfg(feature = "hotshot-testing")]
        {
            let metrics = self.inner.metrics.clone();
            if let Some(ref config) = &self.inner.reliability_config {
                let handle = Arc::clone(&self.inner.handle);

                let fut = config.clone().chaos_send_msg(
                    message,
                    Arc::new(move |msg: Vec<u8>| {
                        let topic_2 = topic.clone();
                        let handle_2 = Arc::clone(&handle);
                        let metrics_2 = metrics.clone();
                        boxed_sync(async move {
                            if let Err(e) = handle_2.gossip_no_serialize(topic_2, msg) {
                                metrics_2.num_failed_messages.add(1);
                                warn!("Failed to broadcast to libp2p: {:?}", e);
                            }
                        })
                    }),
                );
                spawn(fut);
                return Ok(());
            }
        }

        if let Err(e) = self.inner.handle.gossip(topic, &message) {
            self.inner.metrics.num_failed_messages.add(1);
            return Err(e);
        }

        Ok(())
    }

    #[instrument(name = "Libp2pNetwork::da_broadcast_message", skip_all)]
    async fn da_broadcast_message(
        &self,
        message: Vec<u8>,
        recipients: Vec<T::SignatureKey>,
        _broadcast_delay: BroadcastDelay,
    ) -> Result<(), NetworkError> {
        // If we're not ready, return an error
        if !self.is_ready() {
            self.inner.metrics.num_failed_messages.add(1);
            return Err(NetworkError::NotReadyYet);
        };

        let future_results = recipients
            .into_iter()
            .map(|r| self.direct_message(message.clone(), r));
        let results = join_all(future_results).await;

        let errors: Vec<_> = results.into_iter().filter_map(|r| r.err()).collect();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(NetworkError::Multiple(errors))
        }
    }

    #[instrument(name = "Libp2pNetwork::direct_message", skip_all)]
    async fn direct_message(
        &self,
        message: Vec<u8>,
        recipient: T::SignatureKey,
    ) -> Result<(), NetworkError> {
        // If we're not ready, return an error
        if !self.is_ready() {
            self.inner.metrics.num_failed_messages.add(1);
            return Err(NetworkError::NotReadyYet);
        };

        // short circuit if we're dming ourselves
        if recipient == self.inner.pk {
            // panic if we already shut down?
            self.inner.sender.try_send(message).map_err(|_x| {
                self.inner.metrics.num_failed_messages.add(1);
                NetworkError::ShutDown
            })?;
            return Ok(());
        }

        let pid = match self
            .inner
            .handle
            .lookup_node(&recipient.to_bytes(), self.inner.dht_timeout)
            .await
        {
            Ok(pid) => pid,
            Err(err) => {
                self.inner.metrics.num_failed_messages.add(1);
                return Err(NetworkError::LookupError(format!(
                    "failed to look up node for direct message: {err}"
                )));
            },
        };

        #[cfg(feature = "hotshot-testing")]
        {
            let metrics = self.inner.metrics.clone();
            if let Some(ref config) = &self.inner.reliability_config {
                let handle = Arc::clone(&self.inner.handle);

                let fut = config.clone().chaos_send_msg(
                    message,
                    Arc::new(move |msg: Vec<u8>| {
                        let handle_2 = Arc::clone(&handle);
                        let metrics_2 = metrics.clone();
                        boxed_sync(async move {
                            if let Err(e) = handle_2.direct_request_no_serialize(pid, msg) {
                                metrics_2.num_failed_messages.add(1);
                                warn!("Failed to broadcast to libp2p: {:?}", e);
                            }
                        })
                    }),
                );
                spawn(fut);
                return Ok(());
            }
        }

        match self.inner.handle.direct_request(pid, &message) {
            Ok(()) => Ok(()),
            Err(e) => {
                self.inner.metrics.num_failed_messages.add(1);
                Err(e)
            },
        }
    }

    /// Receive one or many messages from the underlying network.
    ///
    /// # Errors
    /// If there is a network-related failure.
    #[instrument(name = "Libp2pNetwork::recv_message", skip_all)]
    async fn recv_message(&self) -> Result<Vec<u8>, NetworkError> {
        let result = self
            .inner
            .receiver
            .lock()
            .await
            .recv()
            .await
            .ok_or(NetworkError::ShutDown)?;

        Ok(result)
    }

    #[instrument(name = "Libp2pNetwork::queue_node_lookup", skip_all)]
    #[allow(clippy::type_complexity)]
    fn queue_node_lookup(
        &self,
        view_number: ViewNumber,
        pk: T::SignatureKey,
    ) -> Result<(), TrySendError<Option<(ViewNumber, T::SignatureKey)>>> {
        self.inner
            .node_lookup_send
            .try_send(Some((view_number, pk)))
    }

    /// The libp2p view update is a special operation intrinsic to its internal behavior.
    ///
    /// Libp2p needs to do a lookup because a libp2p address is not related to
    /// hotshot keys. So in libp2p we store a mapping of HotShot key to libp2p address
    /// in a distributed hash table.
    ///
    /// This means to directly message someone on libp2p we need to lookup in the hash
    /// table what their libp2p address is, using their HotShot public key as the key.
    ///
    /// So the logic with libp2p is to prefetch upcoming leaders libp2p address to
    /// save time when we later need to direct message the leader our vote. Hence the
    /// use of the future view and leader to queue the lookups.
    async fn update_view<'a, TYPES>(
        &'a self,
        view: u64,
        epoch: Option<u64>,
        membership_coordinator: EpochMembershipCoordinator<TYPES>,
    ) where
        TYPES: NodeType<SignatureKey = T::SignatureKey> + 'a,
    {
        let future_view = <TYPES as NodeType>::View::new(view) + LOOK_AHEAD;
        let epoch = epoch.map(<TYPES as NodeType>::Epoch::new);

        let membership = match membership_coordinator.membership_for_epoch(epoch).await {
            Ok(m) => m,
            Err(e) => {
                return tracing::warn!(e.message);
            },
        };
        let future_leader = match membership.leader(future_view).await {
            Ok(l) => l,
            Err(e) => {
                return tracing::info!(
                    "Failed to calculate leader for view {:?}: {e}",
                    future_view
                );
            },
        };

        let _ = self
            .queue_node_lookup(ViewNumber::new(*future_view), future_leader)
            .map_err(|err| tracing::warn!("failed to process node lookup request: {err}"));
    }
}

#[cfg(test)]
mod test {
    mod derive_multiaddr {
        use std::net::Ipv6Addr;

        use super::super::*;

        /// Test derivation of a valid IPv4 address -> Multiaddr
        #[test]
        fn test_v4_valid() {
            // Derive a multiaddr from a valid IPv4 address
            let addr = "1.1.1.1:8080".to_string();
            let multiaddr =
                derive_libp2p_multiaddr(&addr).expect("Failed to derive valid multiaddr, {}");

            // Make sure it's the correct (quic) multiaddr
            assert_eq!(multiaddr.to_string(), "/ip4/1.1.1.1/udp/8080/quic-v1");
        }

        /// Test derivation of a valid IPv6 address -> Multiaddr
        #[test]
        fn test_v6_valid() {
            // Derive a multiaddr from a valid IPv6 address
            let ipv6_addr = Ipv6Addr::new(1, 2, 3, 4, 5, 6, 7, 8);
            let addr = format!("{ipv6_addr}:8080");
            let multiaddr =
                derive_libp2p_multiaddr(&addr).expect("Failed to derive valid multiaddr, {}");

            // Make sure it's the correct (quic) multiaddr
            assert_eq!(
                multiaddr.to_string(),
                format!("/ip6/{ipv6_addr}/udp/8080/quic-v1")
            );
        }

        /// Test that an invalid address fails to derive to a Multiaddr
        #[test]
        fn test_no_port() {
            // Derive a multiaddr from an invalid port
            let addr = "1.1.1.1".to_string();
            let multiaddr = derive_libp2p_multiaddr(&addr);

            // Make sure it fails
            assert!(multiaddr.is_err());
        }

        /// Test that an existing domain name resolves to a Multiaddr
        #[test]
        fn test_fqdn_exists() {
            // Derive a multiaddr from a valid FQDN
            let addr = "example.com:8080".to_string();
            let multiaddr =
                derive_libp2p_multiaddr(&addr).expect("Failed to derive valid multiaddr, {}");

            // Make sure it's the correct (quic) multiaddr
            assert_eq!(multiaddr.to_string(), "/dns/example.com/udp/8080/quic-v1");
        }

        /// Test that a non-existent domain name still resolves to a Multiaddr
        #[test]
        fn test_fqdn_does_not_exist() {
            // Derive a multiaddr from an invalid FQDN
            let addr = "libp2p.example.com:8080".to_string();
            let multiaddr =
                derive_libp2p_multiaddr(&addr).expect("Failed to derive valid multiaddr, {}");

            // Make sure it still worked
            assert_eq!(
                multiaddr.to_string(),
                "/dns/libp2p.example.com/udp/8080/quic-v1"
            );
        }

        /// Test that a domain name without a port fails to derive to a Multiaddr
        #[test]
        fn test_fqdn_no_port() {
            // Derive a multiaddr from an invalid port
            let addr = "example.com".to_string();
            let multiaddr = derive_libp2p_multiaddr(&addr);

            // Make sure it fails
            assert!(multiaddr.is_err());
        }
    }
}
