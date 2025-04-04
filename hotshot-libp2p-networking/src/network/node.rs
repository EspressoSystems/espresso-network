// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

/// configuration for the libp2p network (e.g. how it should be built)
mod config;

/// libp2p network handle
/// allows for control over the libp2p network
mod handle;

use std::{
    collections::{HashMap, HashSet},
    iter,
    num::{NonZeroU32, NonZeroUsize},
    time::Duration,
};

use futures::{channel::mpsc, SinkExt, StreamExt};
use hotshot_types::{
    constants::KAD_DEFAULT_REPUB_INTERVAL_SEC, traits::node_implementation::NodeType,
};
use libp2p::{
    autonat,
    core::transport::ListenerId,
    gossipsub::{
        Behaviour as Gossipsub, ConfigBuilder as GossipsubConfigBuilder, Event as GossipEvent,
        Message as GossipsubMessage, MessageAuthenticity, MessageId, Topic, ValidationMode,
    },
    identify::{
        Behaviour as IdentifyBehaviour, Config as IdentifyConfig, Event as IdentifyEvent,
        Info as IdentifyInfo,
    },
    identity::Keypair,
    kad::{store::MemoryStore, Behaviour, Config, Mode, Record},
    request_response::{
        Behaviour as RequestResponse, Config as Libp2pRequestResponseConfig, ProtocolSupport,
    },
    swarm::SwarmEvent,
    Multiaddr, StreamProtocol, Swarm, SwarmBuilder,
};
use libp2p_identity::PeerId;
use rand::{prelude::SliceRandom, thread_rng};
use tokio::{
    select, spawn,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};
use tracing::{debug, error, info, info_span, instrument, warn, Instrument};

pub use self::{
    config::{
        GossipConfig, NetworkNodeConfig, NetworkNodeConfigBuilder, NetworkNodeConfigBuilderError,
        RequestResponseConfig, DEFAULT_REPLICATION_FACTOR,
    },
    handle::{spawn_network_node, NetworkNodeHandle, NetworkNodeReceiver},
};
use super::{
    behaviours::dht::{
        bootstrap::{DHTBootstrapTask, InputEvent},
        store::{
            persistent::{DhtPersistentStorage, PersistentStore},
            validated::ValidatedStore,
        },
    },
    cbor::Cbor,
    gen_transport, BoxedTransport, ClientRequest, NetworkDef, NetworkError, NetworkEvent,
    NetworkEventInternal,
};
use crate::network::behaviours::{
    dht::{DHTBehaviour, DHTProgress, KadPutQuery, NUM_REPLICATED_TO_TRUST},
    direct_message::{DMBehaviour, DMRequest},
    exponential_backoff::ExponentialBackoff,
};

/// Maximum size of a message
pub const MAX_GOSSIP_MSG_SIZE: usize = 2_000_000_000;

/// Wrapped num of connections
pub const ESTABLISHED_LIMIT: NonZeroU32 = NonZeroU32::new(ESTABLISHED_LIMIT_UNWR).unwrap();
/// Number of connections to a single peer before logging an error
pub const ESTABLISHED_LIMIT_UNWR: u32 = 10;

/// Network definition
#[derive(derive_more::Debug)]
pub struct NetworkNode<T: NodeType, D: DhtPersistentStorage> {
    /// peer id of network node
    peer_id: PeerId,
    /// the swarm of networkbehaviours
    #[debug(skip)]
    swarm: Swarm<NetworkDef<T::SignatureKey, D>>,
    /// the listener id we are listening on, if it exists
    listener_id: Option<ListenerId>,
    /// Handler for direct messages
    direct_message_state: DMBehaviour,
    /// Handler for DHT Events
    dht_handler: DHTBehaviour<T::SignatureKey, D>,
    /// Channel to resend requests, set to Some when we call `spawn_listeners`
    resend_tx: Option<UnboundedSender<ClientRequest>>,
}

impl<T: NodeType, D: DhtPersistentStorage> NetworkNode<T, D> {
    /// Returns number of peers this node is connected to
    pub fn num_connected(&self) -> usize {
        self.swarm.connected_peers().count()
    }

    /// return hashset of PIDs this node is connected to
    pub fn connected_pids(&self) -> HashSet<PeerId> {
        self.swarm.connected_peers().copied().collect()
    }

    /// starts the swarm listening on `listen_addr`
    /// and optionally dials into peer `known_peer`
    /// returns the address the swarm is listening upon
    #[instrument(skip(self))]
    pub async fn start_listen(
        &mut self,
        listen_addr: Multiaddr,
    ) -> Result<Multiaddr, NetworkError> {
        self.listener_id = Some(self.swarm.listen_on(listen_addr).map_err(|err| {
            NetworkError::ListenError(format!("failed to listen for Libp2p: {err}"))
        })?);
        let addr = loop {
            if let Some(SwarmEvent::NewListenAddr { address, .. }) = self.swarm.next().await {
                break address;
            }
        };
        info!("Libp2p listening on {:?}", addr);
        Ok(addr)
    }

    /// initialize the DHT with known peers
    /// add the peers to kademlia and then
    /// the `spawn_listeners` function
    /// will start connecting to peers
    #[instrument(skip(self))]
    pub fn add_known_peers(&mut self, known_peers: &[(PeerId, Multiaddr)]) {
        debug!("Adding {} known peers", known_peers.len());
        let behaviour = self.swarm.behaviour_mut();
        let mut bs_nodes = HashMap::<PeerId, HashSet<Multiaddr>>::new();
        let mut shuffled = known_peers.iter().collect::<Vec<_>>();
        shuffled.shuffle(&mut thread_rng());
        for (peer_id, addr) in shuffled {
            if *peer_id != self.peer_id {
                behaviour.dht.add_address(peer_id, addr.clone());
                behaviour.autonat.add_server(*peer_id, Some(addr.clone()));
                bs_nodes.insert(*peer_id, iter::once(addr.clone()).collect());
            }
        }
    }

    /// Creates a new `Network` with the given settings.
    ///
    /// Currently:
    ///   * Generates a random key pair and associated [`PeerId`]
    ///   * Launches a hopefully production ready transport: QUIC v1 (RFC 9000) + DNS
    ///   * Generates a connection to the "broadcast" topic
    ///   * Creates a swarm to manage peers and events
    ///
    /// # Errors
    /// - If we fail to generate the transport or any of the behaviours
    ///
    /// # Panics
    /// If 5 < 0
    #[allow(clippy::too_many_lines)]
    pub async fn new(
        config: NetworkNodeConfig<T>,
        dht_persistent_storage: D,
    ) -> Result<Self, NetworkError> {
        // Generate a random `KeyPair` if one is not specified
        let keypair = config
            .keypair
            .clone()
            .unwrap_or_else(Keypair::generate_ed25519);

        // Get the `PeerId` from the `KeyPair`
        let peer_id = PeerId::from(keypair.public());

        // Generate the transport from the keypair, membership, and auth message
        let transport: BoxedTransport = gen_transport::<T>(
            keypair.clone(),
            config.membership.clone(),
            config.auth_message.clone(),
        )
        .await?;

        // Generate the swarm
        let mut swarm: Swarm<NetworkDef<T::SignatureKey, D>> = {
            // Use the `Blake3` hash of the message's contents as the ID
            let message_id_fn = |message: &GossipsubMessage| {
                let hash = blake3::hash(&message.data);
                MessageId::from(hash.as_bytes().to_vec())
            };

            // Derive a `Gossipsub` config from our gossip config
            let gossipsub_config = GossipsubConfigBuilder::default()
                .message_id_fn(message_id_fn) // Use the (blake3) hash of a message as its ID
                .validation_mode(ValidationMode::Strict) // Force all messages to have valid signatures
                .heartbeat_interval(config.gossip_config.heartbeat_interval) // Time between gossip heartbeats
                .history_gossip(config.gossip_config.history_gossip) // Number of heartbeats to gossip about
                .history_length(config.gossip_config.history_length) // Number of heartbeats to remember the full message for
                .mesh_n(config.gossip_config.mesh_n) // Target number of mesh peers
                .mesh_n_high(config.gossip_config.mesh_n_high) // Upper limit of mesh peers
                .mesh_n_low(config.gossip_config.mesh_n_low) // Lower limit of mesh peers
                .mesh_outbound_min(config.gossip_config.mesh_outbound_min) // Minimum number of outbound peers in mesh
                .max_transmit_size(config.gossip_config.max_transmit_size) // Maximum size of a message
                .max_ihave_length(config.gossip_config.max_ihave_length) // Maximum number of messages to include in an IHAVE message
                .max_ihave_messages(config.gossip_config.max_ihave_messages) // Maximum number of IHAVE messages to accept from a peer within a heartbeat
                .published_message_ids_cache_time(
                    config.gossip_config.published_message_ids_cache_time,
                ) // Cache duration for published message IDs
                .iwant_followup_time(config.gossip_config.iwant_followup_time) // Time to wait for a message requested through IWANT following an IHAVE advertisement
                .max_messages_per_rpc(config.gossip_config.max_messages_per_rpc) // The maximum number of messages we will process in a given RPC
                .gossip_retransimission(config.gossip_config.gossip_retransmission) // Controls how many times we will allow a peer to request the same message id through IWANT gossip before we start ignoring them.
                .flood_publish(config.gossip_config.flood_publish) // If enabled newly created messages will always be sent to all peers that are subscribed to the topic and have a good enough score.
                .duplicate_cache_time(config.gossip_config.duplicate_cache_time) // The time period that messages are stored in the cache
                .fanout_ttl(config.gossip_config.fanout_ttl) // Time to live for fanout peers
                .heartbeat_initial_delay(config.gossip_config.heartbeat_initial_delay) // Initial delay in each heartbeat
                .gossip_factor(config.gossip_config.gossip_factor) // Affects how many peers we will emit gossip to at each heartbeat
                .gossip_lazy(config.gossip_config.gossip_lazy) // Minimum number of peers to emit gossip to during a heartbeat
                .build()
                .map_err(|err| {
                    NetworkError::ConfigError(format!("error building gossipsub config: {err:?}"))
                })?;

            // - Build a gossipsub network behavior
            let gossipsub: Gossipsub = Gossipsub::new(
                MessageAuthenticity::Signed(keypair.clone()),
                gossipsub_config,
            )
            .map_err(|err| {
                NetworkError::ConfigError(format!("error building gossipsub behaviour: {err:?}"))
            })?;

            //   Build a identify network behavior needed for own
            //   node connection information
            //   E.g. this will answer the question: how are other nodes
            //   seeing the peer from behind a NAT
            let identify_cfg =
                IdentifyConfig::new("HotShot/identify/1.0".to_string(), keypair.public());
            let identify = IdentifyBehaviour::new(identify_cfg);

            // - Build DHT needed for peer discovery
            let mut kconfig = Config::new(StreamProtocol::new("/ipfs/kad/1.0.0"));
            // 8 hours by default
            let record_republication_interval = config
                .republication_interval
                .unwrap_or(Duration::from_secs(KAD_DEFAULT_REPUB_INTERVAL_SEC));
            let ttl = Some(config.ttl.unwrap_or(16 * record_republication_interval));
            kconfig
                .set_parallelism(NonZeroUsize::new(5).unwrap())
                .set_provider_publication_interval(Some(record_republication_interval))
                .set_publication_interval(Some(record_republication_interval))
                .set_record_ttl(ttl);

            // allowing panic here because something is very wrong if this fales
            #[allow(clippy::panic)]
            if let Some(factor) = config.replication_factor {
                kconfig.set_replication_factor(factor);
            } else {
                panic!("Replication factor not set");
            }

            // Create the DHT behaviour with the given persistent storage
            let mut kadem = Behaviour::with_config(
                peer_id,
                PersistentStore::new(
                    ValidatedStore::new(MemoryStore::new(peer_id)),
                    dht_persistent_storage,
                    5,
                )
                .await,
                kconfig,
            );
            kadem.set_mode(Some(Mode::Server));

            let rrconfig = Libp2pRequestResponseConfig::default();

            // Create a new `cbor` codec with the given request and response sizes
            let cbor = Cbor::new(
                config.request_response_config.request_size_maximum,
                config.request_response_config.response_size_maximum,
            );

            let direct_message: super::cbor::Behaviour<Vec<u8>, Vec<u8>> =
                RequestResponse::with_codec(
                    cbor,
                    [(
                        StreamProtocol::new("/HotShot/direct_message/1.0"),
                        ProtocolSupport::Full,
                    )]
                    .into_iter(),
                    rrconfig.clone(),
                );

            let autonat_config = autonat::Config {
                only_global_ips: false,
                ..Default::default()
            };

            let network = NetworkDef::new(
                gossipsub,
                kadem,
                identify,
                direct_message,
                autonat::Behaviour::new(peer_id, autonat_config),
            );

            // build swarm
            let swarm = SwarmBuilder::with_existing_identity(keypair.clone());
            let swarm = swarm.with_tokio();

            swarm
                .with_other_transport(|_| transport)
                .unwrap()
                .with_behaviour(|_| network)
                .unwrap()
                .build()
        };
        for (peer, addr) in &config.to_connect_addrs {
            if peer != swarm.local_peer_id() {
                swarm.behaviour_mut().add_address(peer, addr.clone());
            }
        }

        Ok(Self {
            peer_id,
            swarm,
            listener_id: None,
            direct_message_state: DMBehaviour::default(),
            dht_handler: DHTBehaviour::new(
                peer_id,
                config
                    .replication_factor
                    .unwrap_or(NonZeroUsize::new(4).unwrap()),
            ),
            resend_tx: None,
        })
    }

    /// Publish a key/value to the record store.
    ///
    /// # Panics
    /// If the default replication factor is `None`
    pub fn put_record(&mut self, mut query: KadPutQuery) {
        let record = Record::new(query.key.clone(), query.value.clone());
        match self.swarm.behaviour_mut().dht.put_record(
            record,
            libp2p::kad::Quorum::N(
                NonZeroUsize::try_from(self.dht_handler.replication_factor().get() / 2)
                    .expect("replication factor should be bigger than 0"),
            ),
        ) {
            Err(e) => {
                // failed try again later
                query.progress = DHTProgress::NotStarted;
                query.backoff.start_next(false);
                error!("Error publishing to DHT: {e:?} for peer {:?}", self.peer_id);
            },
            Ok(qid) => {
                debug!("Published record to DHT with qid {:?}", qid);
                let query = KadPutQuery {
                    progress: DHTProgress::InProgress(qid),
                    ..query
                };
                self.dht_handler.put_record(qid, query);
            },
        }
    }

    /// event handler for client events
    /// currently supported actions include
    /// - shutting down the swarm
    /// - gossipping a message to known peers on the `global` topic
    /// - returning the id of the current peer
    /// - subscribing to a topic
    /// - unsubscribing from a toipc
    /// - direct messaging a peer
    #[instrument(skip(self))]
    async fn handle_client_requests(
        &mut self,
        msg: Option<ClientRequest>,
    ) -> Result<bool, NetworkError> {
        let behaviour = self.swarm.behaviour_mut();
        match msg {
            Some(msg) => {
                match msg {
                    ClientRequest::BeginBootstrap => {
                        debug!("Beginning Libp2p bootstrap");
                        let _ = self.swarm.behaviour_mut().dht.bootstrap();
                    },
                    ClientRequest::LookupPeer(pid, chan) => {
                        let id = self.swarm.behaviour_mut().dht.get_closest_peers(pid);
                        self.dht_handler
                            .in_progress_get_closest_peers
                            .insert(id, chan);
                    },
                    ClientRequest::GetRoutingTable(chan) => {
                        self.dht_handler
                            .print_routing_table(&mut self.swarm.behaviour_mut().dht);
                        if chan.send(()).is_err() {
                            warn!("Tried to notify client but client not tracking anymore");
                        }
                    },
                    ClientRequest::PutDHT { key, value, notify } => {
                        let query = KadPutQuery {
                            progress: DHTProgress::NotStarted,
                            notify,
                            key,
                            value,
                            backoff: ExponentialBackoff::default(),
                        };
                        self.put_record(query);
                    },
                    ClientRequest::GetConnectedPeerNum(s) => {
                        if s.send(self.num_connected()).is_err() {
                            error!("error sending peer number to client");
                        }
                    },
                    ClientRequest::GetConnectedPeers(s) => {
                        if s.send(self.connected_pids()).is_err() {
                            error!("error sending peer set to client");
                        }
                    },
                    ClientRequest::GetDHT {
                        key,
                        notify,
                        retry_count,
                    } => {
                        self.dht_handler.get_record(
                            key,
                            notify,
                            NonZeroUsize::new(NUM_REPLICATED_TO_TRUST).unwrap(),
                            ExponentialBackoff::default(),
                            retry_count,
                            &mut self.swarm.behaviour_mut().dht,
                        );
                    },
                    ClientRequest::IgnorePeers(_peers) => {
                        // NOTE used by test with conductor only
                    },
                    ClientRequest::Shutdown => {
                        if let Some(listener_id) = self.listener_id {
                            self.swarm.remove_listener(listener_id);
                        }

                        return Ok(true);
                    },
                    ClientRequest::GossipMsg(topic, contents) => {
                        behaviour.publish_gossip(Topic::new(topic.clone()), contents.clone());
                    },
                    ClientRequest::Subscribe(t, chan) => {
                        behaviour.subscribe_gossip(&t);
                        if let Some(chan) = chan {
                            if chan.send(()).is_err() {
                                error!("finished subscribing but response channel dropped");
                            }
                        }
                    },
                    ClientRequest::Unsubscribe(t, chan) => {
                        behaviour.unsubscribe_gossip(&t);
                        if let Some(chan) = chan {
                            if chan.send(()).is_err() {
                                error!("finished unsubscribing but response channel dropped");
                            }
                        }
                    },
                    ClientRequest::DirectRequest {
                        pid,
                        contents,
                        retry_count,
                    } => {
                        debug!("Sending direct request to {:?}", pid);
                        let id = behaviour.add_direct_request(pid, contents.clone());
                        let req = DMRequest {
                            peer_id: pid,
                            data: contents,
                            backoff: ExponentialBackoff::default(),
                            retry_count,
                        };
                        self.direct_message_state.add_direct_request(req, id);
                    },
                    ClientRequest::DirectResponse(chan, msg) => {
                        behaviour.add_direct_response(chan, msg);
                    },
                    ClientRequest::AddKnownPeers(peers) => {
                        self.add_known_peers(&peers);
                    },
                    ClientRequest::Prune(pid) => {
                        if self.swarm.disconnect_peer_id(pid).is_err() {
                            warn!("Could not disconnect from {:?}", pid);
                        }
                    },
                }
            },
            None => {
                error!("Error receiving msg in main behaviour loop: channel closed");
            },
        }
        Ok(false)
    }

    /// event handler for events emitted from the swarm
    #[allow(clippy::type_complexity)]
    #[instrument(skip(self))]
    async fn handle_swarm_events(
        &mut self,
        event: SwarmEvent<NetworkEventInternal>,
        send_to_client: &UnboundedSender<NetworkEvent>,
    ) -> Result<(), NetworkError> {
        // Make the match cleaner
        debug!("Swarm event observed {:?}", event);

        #[allow(deprecated)]
        match event {
            SwarmEvent::ConnectionEstablished {
                connection_id: _,
                peer_id,
                endpoint,
                num_established,
                concurrent_dial_errors,
                established_in: _established_in,
            } => {
                if num_established > ESTABLISHED_LIMIT {
                    error!(
                        "Num concurrent connections to a single peer exceeding {:?} at {:?}!",
                        ESTABLISHED_LIMIT, num_established
                    );
                } else {
                    debug!(
                        "Connection established with {:?} at {:?} with {:?} concurrent dial errors",
                        peer_id, endpoint, concurrent_dial_errors
                    );
                }

                // Send the number of connected peers to the client
                send_to_client
                    .send(NetworkEvent::ConnectedPeersUpdate(self.num_connected()))
                    .map_err(|err| NetworkError::ChannelSendError(err.to_string()))?;
            },
            SwarmEvent::ConnectionClosed {
                connection_id: _,
                peer_id,
                endpoint,
                num_established,
                cause,
            } => {
                if num_established > ESTABLISHED_LIMIT_UNWR {
                    error!(
                        "Num concurrent connections to a single peer exceeding {:?} at {:?}!",
                        ESTABLISHED_LIMIT, num_established
                    );
                } else {
                    debug!(
                        "Connection closed with {:?} at {:?} due to {:?}",
                        peer_id, endpoint, cause
                    );
                }

                // Send the number of connected peers to the client
                send_to_client
                    .send(NetworkEvent::ConnectedPeersUpdate(self.num_connected()))
                    .map_err(|err| NetworkError::ChannelSendError(err.to_string()))?;
            },
            SwarmEvent::Dialing {
                peer_id,
                connection_id: _,
            } => {
                debug!("Attempting to dial {:?}", peer_id);
            },
            SwarmEvent::ListenerClosed {
                listener_id: _,
                addresses: _,
                reason: _,
            }
            | SwarmEvent::NewListenAddr {
                listener_id: _,
                address: _,
            }
            | SwarmEvent::ExpiredListenAddr {
                listener_id: _,
                address: _,
            }
            | SwarmEvent::NewExternalAddrCandidate { .. }
            | SwarmEvent::ExternalAddrExpired { .. }
            | SwarmEvent::IncomingConnection {
                connection_id: _,
                local_addr: _,
                send_back_addr: _,
            } => {},
            SwarmEvent::Behaviour(b) => {
                let maybe_event = match b {
                    NetworkEventInternal::DHTEvent(e) => self
                        .dht_handler
                        .dht_handle_event(e, self.swarm.behaviour_mut().dht.store_mut()),
                    NetworkEventInternal::IdentifyEvent(e) => {
                        // NOTE feed identified peers into kademlia's routing table for peer discovery.
                        if let IdentifyEvent::Received {
                            peer_id,
                            info:
                                IdentifyInfo {
                                    listen_addrs,
                                    protocols: _,
                                    public_key: _,
                                    protocol_version: _,
                                    agent_version: _,
                                    observed_addr: _,
                                },
                            connection_id: _,
                        } = *e
                        {
                            let behaviour = self.swarm.behaviour_mut();

                            // into hashset to delete duplicates (I checked: there are duplicates)
                            for addr in listen_addrs.iter().collect::<HashSet<_>>() {
                                behaviour.dht.add_address(&peer_id, addr.clone());
                            }
                        }
                        None
                    },
                    NetworkEventInternal::GossipEvent(e) => match *e {
                        GossipEvent::Message {
                            propagation_source: _peer_id,
                            message_id: _id,
                            message,
                        } => Some(NetworkEvent::GossipMsg(message.data)),
                        GossipEvent::Subscribed { peer_id, topic } => {
                            debug!("Peer {:?} subscribed to topic {:?}", peer_id, topic);
                            None
                        },
                        GossipEvent::Unsubscribed { peer_id, topic } => {
                            debug!("Peer {:?} unsubscribed from topic {:?}", peer_id, topic);
                            None
                        },
                        GossipEvent::GossipsubNotSupported { peer_id } => {
                            warn!("Peer {:?} does not support gossipsub", peer_id);
                            None
                        },
                    },
                    NetworkEventInternal::DMEvent(e) => self
                        .direct_message_state
                        .handle_dm_event(e, self.resend_tx.clone()),
                    NetworkEventInternal::AutonatEvent(e) => {
                        match e {
                            autonat::Event::InboundProbe(_) => {},
                            autonat::Event::OutboundProbe(e) => match e {
                                autonat::OutboundProbeEvent::Request { .. }
                                | autonat::OutboundProbeEvent::Response { .. } => {},
                                autonat::OutboundProbeEvent::Error {
                                    probe_id: _,
                                    peer,
                                    error,
                                } => {
                                    warn!(
                                        "AutoNAT Probe failed to peer {:?} with error: {:?}",
                                        peer, error
                                    );
                                },
                            },
                            autonat::Event::StatusChanged { old, new } => {
                                debug!("AutoNAT Status changed. Old: {:?}, New: {:?}", old, new);
                            },
                        };
                        None
                    },
                };

                if let Some(event) = maybe_event {
                    // forward messages directly to Client
                    send_to_client
                        .send(event)
                        .map_err(|err| NetworkError::ChannelSendError(err.to_string()))?;
                }
            },
            SwarmEvent::OutgoingConnectionError {
                connection_id: _,
                peer_id,
                error,
            } => {
                warn!("Outgoing connection error to {:?}: {:?}", peer_id, error);
            },
            SwarmEvent::IncomingConnectionError {
                connection_id: _,
                local_addr: _,
                send_back_addr: _,
                error,
            } => {
                warn!("Incoming connection error: {:?}", error);
            },
            SwarmEvent::ListenerError {
                listener_id: _,
                error,
            } => {
                warn!("Listener error: {:?}", error);
            },
            SwarmEvent::ExternalAddrConfirmed { address } => {
                let my_id = *self.swarm.local_peer_id();
                self.swarm
                    .behaviour_mut()
                    .dht
                    .add_address(&my_id, address.clone());
            },
            SwarmEvent::NewExternalAddrOfPeer { peer_id, address } => {
                self.swarm
                    .behaviour_mut()
                    .dht
                    .add_address(&peer_id, address.clone());
            },
            _ => {
                debug!("Unhandled swarm event {:?}", event);
            },
        }
        Ok(())
    }

    /// Spawn a task to listen for requests on the returned channel
    /// as well as any events produced by libp2p
    ///
    /// # Errors
    /// - If we fail to create the channels or the bootstrap channel
    pub fn spawn_listeners(
        mut self,
    ) -> Result<
        (
            UnboundedSender<ClientRequest>,
            UnboundedReceiver<NetworkEvent>,
        ),
        NetworkError,
    > {
        let (s_input, mut s_output) = unbounded_channel::<ClientRequest>();
        let (r_input, r_output) = unbounded_channel::<NetworkEvent>();
        let (mut bootstrap_tx, bootstrap_rx) = mpsc::channel(100);
        self.resend_tx = Some(s_input.clone());
        self.dht_handler.set_bootstrap_sender(bootstrap_tx.clone());

        DHTBootstrapTask::run(bootstrap_rx, s_input.clone());
        spawn(
            async move {
                loop {
                    select! {
                        event = self.swarm.next() => {
                            debug!("peerid {:?}\t\thandling maybe event {:?}", self.peer_id, event);
                            if let Some(event) = event {
                                debug!("peerid {:?}\t\thandling event {:?}", self.peer_id, event);
                                self.handle_swarm_events(event, &r_input).await?;
                            }
                        },
                        msg = s_output.recv() => {
                            debug!("peerid {:?}\t\thandling msg {:?}", self.peer_id, msg);
                            let shutdown = self.handle_client_requests(msg).await?;
                            if shutdown {
                                let _ = bootstrap_tx.send(InputEvent::ShutdownBootstrap).await;
                                break
                            }
                        }
                    }
                }
                Ok::<(), NetworkError>(())
            }
            .instrument(info_span!("Libp2p NetworkBehaviour Handler")),
        );
        Ok((s_input, r_output))
    }

    /// Get a reference to the network node's peer id.
    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }
}
