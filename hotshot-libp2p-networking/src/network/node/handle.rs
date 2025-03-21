// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{collections::HashSet, fmt::Debug, time::Duration};

use hotshot_types::traits::{network::NetworkError, node_implementation::NodeType};
use libp2p::{request_response::ResponseChannel, Multiaddr};
use libp2p_identity::PeerId;
use tokio::{
    sync::mpsc::{Receiver, UnboundedReceiver, UnboundedSender},
    time::{sleep, timeout},
};
use tracing::{debug, info, instrument};

use crate::network::{
    behaviours::dht::{
        record::{Namespace, RecordKey, RecordValue},
        store::persistent::DhtPersistentStorage,
    },
    gen_multiaddr, ClientRequest, NetworkEvent, NetworkNode, NetworkNodeConfig,
};

/// A handle containing:
/// - A reference to the state
/// - Controls for the swarm
#[derive(Debug, Clone)]
pub struct NetworkNodeHandle<T: NodeType> {
    /// network configuration
    network_config: NetworkNodeConfig<T>,

    /// send an action to the networkbehaviour
    send_network: UnboundedSender<ClientRequest>,

    /// the local address we're listening on
    listen_addr: Multiaddr,

    /// the peer id of the networkbehaviour
    peer_id: PeerId,

    /// human readable id
    id: usize,
}

/// internal network node receiver
#[derive(Debug)]
pub struct NetworkNodeReceiver {
    /// the receiver
    receiver: UnboundedReceiver<NetworkEvent>,

    ///kill switch
    recv_kill: Option<Receiver<()>>,
}

impl NetworkNodeReceiver {
    /// recv a network event
    /// # Errors
    /// Errors if the receiver channel is closed
    pub async fn recv(&mut self) -> Result<NetworkEvent, NetworkError> {
        self.receiver
            .recv()
            .await
            .ok_or(NetworkError::ChannelReceiveError(
                "Receiver channel closed".to_string(),
            ))
    }
    /// Add a kill switch to the receiver
    pub fn set_kill_switch(&mut self, kill_switch: Receiver<()>) {
        self.recv_kill = Some(kill_switch);
    }

    /// Take the kill switch to allow killing the receiver task
    pub fn take_kill_switch(&mut self) -> Option<Receiver<()>> {
        self.recv_kill.take()
    }
}

/// Spawn a network node task task and return the handle and the receiver for it
/// # Errors
/// Errors if spawning the task fails
pub async fn spawn_network_node<T: NodeType, D: DhtPersistentStorage>(
    config: NetworkNodeConfig<T>,
    dht_persistent_storage: D,
    id: usize,
) -> Result<(NetworkNodeReceiver, NetworkNodeHandle<T>), NetworkError> {
    let mut network = NetworkNode::new(config.clone(), dht_persistent_storage)
        .await
        .map_err(|e| NetworkError::ConfigError(format!("failed to create network node: {e}")))?;
    // randomly assigned port
    let listen_addr = config
        .bind_address
        .clone()
        .unwrap_or_else(|| gen_multiaddr(0));
    let peer_id = network.peer_id();
    let listen_addr = network.start_listen(listen_addr).await.map_err(|e| {
        NetworkError::ListenError(format!("failed to start listening on Libp2p: {e}"))
    })?;
    // pin here to force the future onto the heap since it can be large
    // in the case of flume
    let (send_chan, recv_chan) = network.spawn_listeners().map_err(|err| {
        NetworkError::ListenError(format!("failed to spawn listeners for Libp2p: {err}"))
    })?;
    let receiver = NetworkNodeReceiver {
        receiver: recv_chan,
        recv_kill: None,
    };

    let handle = NetworkNodeHandle::<T> {
        network_config: config,
        send_network: send_chan,
        listen_addr,
        peer_id,
        id,
    };
    Ok((receiver, handle))
}

impl<T: NodeType> NetworkNodeHandle<T> {
    /// Cleanly shuts down a swarm node
    /// This is done by sending a message to
    /// the swarm itself to spin down
    #[instrument]
    pub async fn shutdown(&self) -> Result<(), NetworkError> {
        self.send_request(ClientRequest::Shutdown)?;
        Ok(())
    }
    /// Notify the network to begin the bootstrap process
    /// # Errors
    /// If unable to send via `send_network`. This should only happen
    /// if the network is shut down.
    pub fn begin_bootstrap(&self) -> Result<(), NetworkError> {
        let req = ClientRequest::BeginBootstrap;
        self.send_request(req)
    }

    /// Get a reference to the network node handle's listen addr.
    #[must_use]
    pub fn listen_addr(&self) -> Multiaddr {
        self.listen_addr.clone()
    }

    /// Print out the routing table used by kademlia
    /// NOTE: only for debugging purposes currently
    /// # Errors
    /// if the client has stopped listening for a response
    pub async fn print_routing_table(&self) -> Result<(), NetworkError> {
        let (s, r) = futures::channel::oneshot::channel();
        let req = ClientRequest::GetRoutingTable(s);
        self.send_request(req)?;
        r.await
            .map_err(|e| NetworkError::ChannelReceiveError(e.to_string()))
    }
    /// Wait until at least `num_peers` have connected
    ///
    /// # Errors
    /// If the channel closes before the result can be sent back
    pub async fn wait_to_connect(
        &self,
        num_required_peers: usize,
        node_id: usize,
    ) -> Result<(), NetworkError> {
        // Wait for the required number of peers to connect
        loop {
            // Get the number of currently connected peers
            let num_connected = self.num_connected().await?;
            if num_connected >= num_required_peers {
                break;
            }

            // Log the number of connected peers
            info!(
                "Node {} connected to {}/{} peers",
                node_id, num_connected, num_required_peers
            );

            // Sleep for a second before checking again
            sleep(Duration::from_secs(1)).await;
        }

        Ok(())
    }

    /// Look up a peer's addresses in kademlia
    /// NOTE: this should always be called before any `request_response` is initiated
    /// # Errors
    /// if the client has stopped listening for a response
    pub async fn lookup_pid(&self, peer_id: PeerId) -> Result<(), NetworkError> {
        let (s, r) = futures::channel::oneshot::channel();
        let req = ClientRequest::LookupPeer(peer_id, s);
        self.send_request(req)?;
        r.await
            .map_err(|err| NetworkError::ChannelReceiveError(err.to_string()))
    }

    /// Looks up a node's `PeerId` by its staking key. Is authenticated through
    /// `get_record` assuming each record should be signed.
    ///
    /// # Errors
    /// If the DHT lookup fails
    pub async fn lookup_node(
        &self,
        key: &[u8],
        dht_timeout: Duration,
    ) -> Result<PeerId, NetworkError> {
        // Create the record key
        let key = RecordKey::new(Namespace::Lookup, key.to_vec());

        // Get the record from the DHT
        let pid = self.get_record_timeout(key, dht_timeout).await?;

        PeerId::from_bytes(&pid).map_err(|err| NetworkError::FailedToDeserialize(err.to_string()))
    }

    /// Insert a record into the kademlia DHT
    /// # Errors
    /// - Will return [`NetworkError::FailedToSerialize`] when unable to serialize the key or value
    pub async fn put_record(
        &self,
        key: RecordKey,
        value: RecordValue<T::SignatureKey>,
    ) -> Result<(), NetworkError> {
        // Serialize the key
        let key = key.to_bytes();

        // Serialize the record
        let value = bincode::serialize(&value)
            .map_err(|e| NetworkError::FailedToSerialize(e.to_string()))?;

        let (s, r) = futures::channel::oneshot::channel();
        let req = ClientRequest::PutDHT {
            key: key.clone(),
            value,
            notify: s,
        };

        self.send_request(req)?;

        r.await.map_err(|_| NetworkError::RequestCancelled)
    }

    /// Receive a record from the kademlia DHT if it exists.
    /// Must be replicated on at least 2 nodes
    /// # Errors
    /// - Will return [`NetworkError::FailedToSerialize`] when unable to serialize the key
    /// - Will return [`NetworkError::FailedToDeserialize`] when unable to deserialize the returned value
    pub async fn get_record(
        &self,
        key: RecordKey,
        retry_count: u8,
    ) -> Result<Vec<u8>, NetworkError> {
        // Serialize the key
        let serialized_key = key.to_bytes();

        let (s, r) = futures::channel::oneshot::channel();
        let req = ClientRequest::GetDHT {
            key: serialized_key.clone(),
            notify: vec![s],
            retry_count,
        };
        self.send_request(req)?;

        // Map the error
        let result = r.await.map_err(|_| NetworkError::RequestCancelled)?;

        // Deserialize the record's value
        let record: RecordValue<T::SignatureKey> = bincode::deserialize(&result)
            .map_err(|e| NetworkError::FailedToDeserialize(e.to_string()))?;

        Ok(record.value().to_vec())
    }

    /// Get a record from the kademlia DHT with a timeout
    /// # Errors
    /// - Will return [`NetworkError::Timeout`] when times out
    /// - Will return [`NetworkError::FailedToSerialize`] when unable to serialize the key or value
    /// - Will return [`NetworkError::ChannelSendError`] when underlying `NetworkNode` has been killed
    pub async fn get_record_timeout(
        &self,
        key: RecordKey,
        timeout_duration: Duration,
    ) -> Result<Vec<u8>, NetworkError> {
        timeout(timeout_duration, self.get_record(key, 3))
            .await
            .map_err(|err| NetworkError::Timeout(err.to_string()))?
    }

    /// Insert a record into the kademlia DHT with a timeout
    /// # Errors
    /// - Will return [`NetworkError::Timeout`] when times out
    /// - Will return [`NetworkError::FailedToSerialize`] when unable to serialize the key or value
    /// - Will return [`NetworkError::ChannelSendError`] when underlying `NetworkNode` has been killed
    pub async fn put_record_timeout(
        &self,
        key: RecordKey,
        value: RecordValue<T::SignatureKey>,
        timeout_duration: Duration,
    ) -> Result<(), NetworkError> {
        timeout(timeout_duration, self.put_record(key, value))
            .await
            .map_err(|err| NetworkError::Timeout(err.to_string()))?
    }

    /// Subscribe to a topic
    /// # Errors
    /// - Will return [`NetworkError::ChannelSendError`] when underlying `NetworkNode` has been killed
    pub async fn subscribe(&self, topic: String) -> Result<(), NetworkError> {
        let (s, r) = futures::channel::oneshot::channel();
        let req = ClientRequest::Subscribe(topic, Some(s));
        self.send_request(req)?;
        r.await
            .map_err(|err| NetworkError::ChannelReceiveError(err.to_string()))
    }

    /// Unsubscribe from a topic
    /// # Errors
    /// - Will return [`NetworkError::ChannelSendError`] when underlying `NetworkNode` has been killed
    pub async fn unsubscribe(&self, topic: String) -> Result<(), NetworkError> {
        let (s, r) = futures::channel::oneshot::channel();
        let req = ClientRequest::Unsubscribe(topic, Some(s));
        self.send_request(req)?;
        r.await
            .map_err(|err| NetworkError::ChannelReceiveError(err.to_string()))
    }

    /// Ignore `peers` when pruning
    /// e.g. maintain their connection
    /// # Errors
    /// - Will return [`NetworkError::ChannelSendError`] when underlying `NetworkNode` has been killed
    pub fn ignore_peers(&self, peers: Vec<PeerId>) -> Result<(), NetworkError> {
        let req = ClientRequest::IgnorePeers(peers);
        self.send_request(req)
    }

    /// Make a direct request to `peer_id` containing `msg`
    /// # Errors
    /// - Will return [`NetworkError::ChannelSendError`] when underlying `NetworkNode` has been killed
    /// - Will return [`NetworkError::FailedToSerialize`] when unable to serialize `msg`
    pub fn direct_request(&self, pid: PeerId, msg: &[u8]) -> Result<(), NetworkError> {
        self.direct_request_no_serialize(pid, msg.to_vec())
    }

    /// Make a direct request to `peer_id` containing `msg` without serializing
    /// # Errors
    /// - Will return [`NetworkError::ChannelSendError`] when underlying `NetworkNode` has been killed
    /// - Will return [`NetworkError::FailedToSerialize`] when unable to serialize `msg`
    pub fn direct_request_no_serialize(
        &self,
        pid: PeerId,
        contents: Vec<u8>,
    ) -> Result<(), NetworkError> {
        let req = ClientRequest::DirectRequest {
            pid,
            contents,
            retry_count: 1,
        };
        self.send_request(req)
    }

    /// Reply with `msg` to a request over `chan`
    /// # Errors
    /// - Will return [`NetworkError::ChannelSendError`] when underlying `NetworkNode` has been killed
    /// - Will return [`NetworkError::FailedToSerialize`] when unable to serialize `msg`
    pub fn direct_response(
        &self,
        chan: ResponseChannel<Vec<u8>>,
        msg: &[u8],
    ) -> Result<(), NetworkError> {
        let req = ClientRequest::DirectResponse(chan, msg.to_vec());
        self.send_request(req)
    }

    /// Forcefully disconnect from a peer
    /// # Errors
    /// If the channel is closed somehow
    /// Shouldnt' happen.
    /// # Panics
    /// If channel errors out
    /// shouldn't happen.
    pub fn prune_peer(&self, pid: PeerId) -> Result<(), NetworkError> {
        let req = ClientRequest::Prune(pid);
        self.send_request(req)
    }

    /// Gossip a message to peers
    /// # Errors
    /// - Will return [`NetworkError::ChannelSendError`] when underlying `NetworkNode` has been killed
    /// - Will return [`NetworkError::FailedToSerialize`] when unable to serialize `msg`
    pub fn gossip(&self, topic: String, msg: &[u8]) -> Result<(), NetworkError> {
        self.gossip_no_serialize(topic, msg.to_vec())
    }

    /// Gossip a message to peers without serializing
    /// # Errors
    /// - Will return [`NetworkError::ChannelSendError`] when underlying `NetworkNode` has been killed
    /// - Will return [`NetworkError::FailedToSerialize`] when unable to serialize `msg`
    pub fn gossip_no_serialize(&self, topic: String, msg: Vec<u8>) -> Result<(), NetworkError> {
        let req = ClientRequest::GossipMsg(topic, msg);
        self.send_request(req)
    }

    /// Tell libp2p about known network nodes
    /// # Errors
    /// - Will return [`NetworkError::ChannelSendError`] when underlying `NetworkNode` has been killed
    pub fn add_known_peers(
        &self,
        known_peers: Vec<(PeerId, Multiaddr)>,
    ) -> Result<(), NetworkError> {
        debug!("Adding {} known peers", known_peers.len());
        let req = ClientRequest::AddKnownPeers(known_peers);
        self.send_request(req)
    }

    /// Send a client request to the network
    ///
    /// # Errors
    /// - Will return [`NetworkError::ChannelSendError`] when underlying `NetworkNode` has been killed
    fn send_request(&self, req: ClientRequest) -> Result<(), NetworkError> {
        self.send_network
            .send(req)
            .map_err(|err| NetworkError::ChannelSendError(err.to_string()))
    }

    /// Returns number of peers this node is connected to
    /// # Errors
    /// If the channel is closed somehow
    /// Shouldnt' happen.
    /// # Panics
    /// If channel errors out
    /// shouldn't happen.
    pub async fn num_connected(&self) -> Result<usize, NetworkError> {
        let (s, r) = futures::channel::oneshot::channel();
        let req = ClientRequest::GetConnectedPeerNum(s);
        self.send_request(req)?;
        Ok(r.await.unwrap())
    }

    /// return hashset of PIDs this node is connected to
    /// # Errors
    /// If the channel is closed somehow
    /// Shouldnt' happen.
    /// # Panics
    /// If channel errors out
    /// shouldn't happen.
    pub async fn connected_pids(&self) -> Result<HashSet<PeerId>, NetworkError> {
        let (s, r) = futures::channel::oneshot::channel();
        let req = ClientRequest::GetConnectedPeers(s);
        self.send_request(req)?;
        Ok(r.await.unwrap())
    }

    /// Get a reference to the network node handle's id.
    #[must_use]
    pub fn id(&self) -> usize {
        self.id
    }

    /// Get a reference to the network node handle's peer id.
    #[must_use]
    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    /// Return a reference to the network config
    #[must_use]
    pub fn config(&self) -> &NetworkNodeConfig<T> {
        &self.network_config
    }
}
