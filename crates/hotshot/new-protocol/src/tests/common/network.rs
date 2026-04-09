use std::sync::Arc;

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
    traits::{metrics::NoMetrics, network::Topic, signature_key::SignatureKey},
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
    /// with one network instance per node.
    fn create(
        num_nodes: usize,
    ) -> impl std::future::Future<
        Output = (
            Self,
            Vec<<Self::Impl as NodeImplementation<TestTypes>>::Network>,
        ),
    >
    where
        Self: Sized;
}

// -- MemoryNetwork implementation -------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct MemoryNetworkImpl;

impl NodeImplementation<TestTypes> for MemoryNetworkImpl {
    type Network = MemoryNetwork<BLSPubKey>;
    type Storage = TestStorage<TestTypes>;
}

pub struct MemoryTestNetwork {
    #[allow(dead_code)]
    pub group: Arc<MasterMap<BLSPubKey>>,
}

impl TestNetwork for MemoryTestNetwork {
    type Impl = MemoryNetworkImpl;

    async fn create(num_nodes: usize) -> (Self, Vec<MemoryNetwork<BLSPubKey>>) {
        let group: Arc<MasterMap<BLSPubKey>> = MasterMap::new();
        let networks = (0..num_nodes)
            .map(|i| {
                let (pk, _) = BLSPubKey::generated_from_seed_indexed([0; 32], i as u64);
                MemoryNetwork::new(&pk, &group, &[Topic::Global], None)
            })
            .collect();
        (Self { group }, networks)
    }
}

// -- Cliquenet implementation -----------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct CliquenetImpl;

impl NodeImplementation<TestTypes> for CliquenetImpl {
    type Network = Cliquenet<BLSPubKey>;
    type Storage = TestStorage<TestTypes>;
}

pub struct CliquenetTestNetwork;

impl TestNetwork for CliquenetTestNetwork {
    type Impl = CliquenetImpl;

    async fn create(num_nodes: usize) -> (Self, Vec<Cliquenet<BLSPubKey>>) {
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

        // Create each Cliquenet node.
        let mut networks = Vec::with_capacity(num_nodes);
        for (keypair, public_key, addr) in &parties {
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
            networks.push(net);
        }

        (Self, networks)
    }
}
