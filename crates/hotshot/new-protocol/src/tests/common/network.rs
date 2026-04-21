use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use hotshot::{
    traits::{
        NodeImplementation,
        implementations::{Cliquenet, MasterMap, MemoryNetwork},
    },
    types::BLSPubKey,
};
use hotshot_example_types::{node_types::TestTypes, storage_types::TestStorage};
use hotshot_types::{
    PeerConnectInfo,
    addr::NetAddr,
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        metrics::NoMetrics,
        network::{ConnectedNetwork, Topic},
        signature_key::SignatureKey,
    },
    x25519::Keypair,
};
use serde::{Deserialize, Serialize};

/// Abstracts creation of a connected set of test networks.
///
/// Implementations produce `num_nodes` interconnected network instances.  The
/// runner calls [`TestNetwork::create`] once at startup and passes each network
/// to [`build_test_coordinator`](super::coordinator_builder::build_test_coordinator).
pub trait TestNetwork {
    type Impl: NodeImplementation<TestTypes>;

    /// Create `num_nodes` interconnected networks.
    ///
    /// Returns `Self` (which may hold shared state like a `MasterMap`) together
    /// with one network instance per node.  Nodes in `skip_nodes` are not
    /// created; their position in the returned `Vec` is `None`.
    #[allow(clippy::type_complexity)]
    fn create(
        num_nodes: usize,
        skip_nodes: &BTreeSet<usize>,
    ) -> impl std::future::Future<
        Output = (
            Self,
            Vec<Option<<Self::Impl as NodeImplementation<TestTypes>>::Network>>,
        ),
    >
    where
        Self: Sized;

    /// Create a "client" network that can broadcast messages to the nodes
    /// but does not participate in consensus.
    fn create_client(
        &self,
    ) -> impl std::future::Future<Output = <Self::Impl as NodeImplementation<TestTypes>>::Network>;

    /// Create a network for a single node.  Used to bring a node online
    /// mid-test (restart or late-start).
    fn create_node(
        &self,
        node_index: usize,
    ) -> impl std::future::Future<Output = <Self::Impl as NodeImplementation<TestTypes>>::Network>;

    /// Shut down a previously created node's network so its internal
    /// buffers don't block broadcasts.  Must be called before
    /// [`create_node`](Self::create_node) when restarting a node.
    fn shutdown_node(&self, node_index: usize) -> impl std::future::Future<Output = ()>;

    /// Create an independent membership coordinator whose internal
    /// fetcher is connected to this test network so epoch catchup can
    /// reach peers.
    fn create_membership(
        &self,
        num_nodes: usize,
        epoch_height: u64,
    ) -> impl std::future::Future<Output = EpochMembershipCoordinator<TestTypes>>;
}

// -- MemoryNetwork implementation -------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct MemoryNetworkImpl;

impl NodeImplementation<TestTypes> for MemoryNetworkImpl {
    type Network = MemoryNetwork<BLSPubKey>;
    type Storage = TestStorage<TestTypes>;
}

pub struct MemoryTestNetwork {
    pub group: Arc<MasterMap<BLSPubKey>>,
    /// Clones of created node networks, keyed by index.  Used to call
    /// `shut_down` on old networks before replacing them during restarts.
    node_networks: Mutex<BTreeMap<usize, MemoryNetwork<BLSPubKey>>>,
}

impl TestNetwork for MemoryTestNetwork {
    type Impl = MemoryNetworkImpl;

    async fn create(
        num_nodes: usize,
        skip_nodes: &BTreeSet<usize>,
    ) -> (Self, Vec<Option<MemoryNetwork<BLSPubKey>>>) {
        let group: Arc<MasterMap<BLSPubKey>> = MasterMap::new();
        let mut stored = BTreeMap::new();

        let networks = (0..num_nodes)
            .map(|i| {
                let (pk, _) = BLSPubKey::generated_from_seed_indexed([0; 32], i as u64);
                let topics: &[Topic] = if skip_nodes.contains(&i) {
                    &[]
                } else {
                    &[Topic::Global]
                };
                let net = MemoryNetwork::new(&pk, &group, topics, None);
                if skip_nodes.contains(&i) {
                    None
                } else {
                    stored.insert(i, net.clone());
                    Some(net)
                }
            })
            .collect();
        (
            Self {
                group,
                node_networks: Mutex::new(stored),
            },
            networks,
        )
    }

    async fn create_client(&self) -> MemoryNetwork<BLSPubKey> {
        let (pk, _) = BLSPubKey::generated_from_seed_indexed([1; 32], 9999);
        MemoryNetwork::new(&pk, &self.group, &[], None)
    }

    async fn create_node(&self, node_index: usize) -> MemoryNetwork<BLSPubKey> {
        let (pk, _) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index as u64);
        let net = MemoryNetwork::new(&pk, &self.group, &[Topic::Global], None);
        self.node_networks
            .lock()
            .unwrap()
            .insert(node_index, net.clone());
        net
    }

    async fn shutdown_node(&self, node_index: usize) {
        let net = self.node_networks.lock().unwrap().remove(&node_index);
        if let Some(net) = net {
            net.shut_down().await;
        }
    }

    async fn create_membership(
        &self,
        num_nodes: usize,
        epoch_height: u64,
    ) -> EpochMembershipCoordinator<TestTypes> {
        super::utils::mock_membership_with_network(
            num_nodes,
            epoch_height,
            Some(self.group.clone()),
        )
        .await
    }
}

// -- Cliquenet implementation -----------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct CliquenetImpl;

impl NodeImplementation<TestTypes> for CliquenetImpl {
    type Network = Cliquenet<BLSPubKey>;
    type Storage = TestStorage<TestTypes>;
}

pub struct CliquenetTestNetwork {
    peer_infos: Vec<(BLSPubKey, PeerConnectInfo)>,
    /// Live networks keyed by node index.  On restart we shut these down
    /// (freeing the listener port) before rebinding to the same address.
    node_networks: tokio::sync::Mutex<BTreeMap<usize, Cliquenet<BLSPubKey>>>,
}

impl TestNetwork for CliquenetTestNetwork {
    type Impl = CliquenetImpl;

    async fn create(
        num_nodes: usize,
        skip_nodes: &BTreeSet<usize>,
    ) -> (Self, Vec<Option<Cliquenet<BLSPubKey>>>) {
        // Generate keys and addresses for all parties.
        let parties: Vec<(Keypair, BLSPubKey, NetAddr)> = (0..num_nodes)
            .map(|i| {
                let (public_key, private_key) =
                    BLSPubKey::generated_from_seed_indexed([0u8; 32], i as u64);
                let keypair = Keypair::derive_from::<BLSPubKey>(&private_key);
                let port = test_utils::reserve_tcp_port()
                    .expect("OS should have ephemeral ports available");
                let addr = NetAddr::Inet(std::net::Ipv4Addr::LOCALHOST.into(), port);
                (keypair, public_key, addr)
            })
            .collect();

        // Build peer info list for connecting nodes to each other.
        let peer_infos: Vec<(BLSPubKey, PeerConnectInfo)> = parties
            .iter()
            .map(|(kp, pk, addr)| {
                (
                    *pk,
                    PeerConnectInfo {
                        x25519_key: kp.public_key(),
                        p2p_addr: addr.clone(),
                    },
                )
            })
            .collect();

        // Create each Cliquenet node (skip down nodes — sends to them
        // fail gracefully over TCP).
        let mut networks = Vec::with_capacity(num_nodes);
        let mut stored = BTreeMap::new();
        for (i, (keypair, public_key, addr)) in parties.iter().enumerate() {
            if skip_nodes.contains(&i) {
                networks.push(None);
                continue;
            }
            let net = Cliquenet::create(
                "test",
                *public_key,
                keypair.clone(),
                addr.clone(),
                peer_infos.clone(),
                Box::new(NoMetrics),
            )
            .await
            .expect("cliquenet creation should succeed");
            stored.insert(i, net.clone());
            networks.push(Some(net));
        }

        (
            Self {
                peer_infos,
                node_networks: tokio::sync::Mutex::new(stored),
            },
            networks,
        )
    }

    async fn create_client(&self) -> Cliquenet<BLSPubKey> {
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([1u8; 32], 9999);
        let keypair = Keypair::derive_from::<BLSPubKey>(&private_key);
        let port =
            test_utils::reserve_tcp_port().expect("OS should have ephemeral ports available");
        let addr = NetAddr::Inet(std::net::Ipv4Addr::LOCALHOST.into(), port);
        Cliquenet::create(
            "test-client",
            public_key,
            keypair,
            addr,
            self.peer_infos.clone(),
            Box::new(NoMetrics),
        )
        .await
        .expect("cliquenet client creation should succeed")
    }

    async fn create_node(&self, node_index: usize) -> Cliquenet<BLSPubKey> {
        let (public_key, private_key) =
            BLSPubKey::generated_from_seed_indexed([0u8; 32], node_index as u64);
        let keypair = Keypair::derive_from::<BLSPubKey>(&private_key);
        // Reuse the node's original address so peers (whose parties list
        // was fixed at startup) can still reach the restarted node.
        let addr = self.peer_infos[node_index].1.p2p_addr.clone();
        let net = Cliquenet::create(
            "test",
            public_key,
            keypair,
            addr,
            self.peer_infos.clone(),
            Box::new(NoMetrics),
        )
        .await
        .expect("cliquenet node creation should succeed");
        self.node_networks
            .lock()
            .await
            .insert(node_index, net.clone());
        net
    }

    async fn shutdown_node(&self, node_index: usize) {
        let net = self.node_networks.lock().await.remove(&node_index);
        if let Some(net) = net {
            // Await shutdown so the TCP listener releases the port
            // before `create_node` rebinds to the same address.
            net.shut_down().await;
        }
    }

    async fn create_membership(
        &self,
        num_nodes: usize,
        epoch_height: u64,
    ) -> EpochMembershipCoordinator<TestTypes> {
        // Cliquenet doesn't have a MasterMap; use an isolated network.
        super::utils::mock_membership_with_num_nodes(num_nodes, epoch_height).await
    }
}
