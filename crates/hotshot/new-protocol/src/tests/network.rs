//! Config-pinned cliquenet peers ([`PeerPolicy`]).
//!
//! A forwarder lists an observer as a `Role::Passive` static peer: the
//! observer gets nothing from `broadcast` but everything from
//! `send_to_observers`, stays a peer across stake-table updates, and its own
//! messages are accepted. An observer lists its upstreams as static peers and
//! never dials the stake table.

use std::{net::Ipv4Addr, time::Duration};

use hotshot::types::BLSPubKey;
use hotshot_example_types::{node_types::TestTypes, storage_types::TestStorage};
use hotshot_types::{
    PeerConnectInfo,
    addr::NetAddr,
    data::{EpochNumber, ViewNumber},
    traits::{metrics::NoMetrics, signature_key::SignatureKey},
    x25519::Keypair,
};
use tokio::time::{sleep, timeout};

use crate::{
    helpers::test_upgrade_lock,
    message::{BlockMessage, Message, MessageType, TransactionMessage, Unchecked, Validated},
    network::{Cliquenet, NetworkError, PeerPolicy},
    tests::common::utils::{StakeTableSchedule, mock_membership_with_client_and_schedule},
};

/// Time for TCP connections and noise handshakes to settle.
const SETTLE: Duration = Duration::from_secs(2);
const RECV_TIMEOUT: Duration = Duration::from_secs(10);
/// How long to wait before concluding a message was *not* delivered.
const SILENCE: Duration = Duration::from_millis(500);

/// Identity of a test node: keys from the same seed the mock membership uses,
/// so index `i` here is member `i` in a [`StakeTableSchedule`].
struct Node {
    public_key: BLSPubKey,
    keypair: Keypair,
    addr: NetAddr,
}

impl Node {
    fn new(index: usize) -> Self {
        let (public_key, private_key) =
            BLSPubKey::generated_from_seed_indexed([0u8; 32], index as u64);
        let keypair = Keypair::derive_from::<BLSPubKey>(&private_key).unwrap();
        let port = test_utils::reserve_tcp_port().expect("OS should have ephemeral ports");
        Self {
            public_key,
            keypair,
            addr: NetAddr::Inet(Ipv4Addr::LOCALHOST.into(), port),
        }
    }

    fn peer(&self) -> (BLSPubKey, PeerConnectInfo) {
        (
            self.public_key,
            PeerConnectInfo {
                x25519_key: self.keypair.public_key(),
                p2p_addr: self.addr.clone(),
            },
        )
    }

    async fn network(
        &self,
        parties: Vec<(BLSPubKey, PeerConnectInfo)>,
        policy: PeerPolicy<BLSPubKey>,
    ) -> Result<Cliquenet<TestTypes>, NetworkError> {
        Cliquenet::create(
            "observer-test",
            self.public_key,
            self.keypair.clone(),
            self.addr.clone(),
            parties,
            policy,
            test_upgrade_lock(),
            Box::new(NoMetrics),
        )
        .await
    }
}

fn message(from: &Node, view: u64) -> Message<TestTypes, Validated> {
    Message {
        sender: from.public_key,
        message_type: MessageType::Block(BlockMessage::Transactions(TransactionMessage {
            view: ViewNumber::new(view),
            transactions: vec![],
        })),
    }
}

fn view_of(m: &Message<TestTypes, Unchecked>) -> u64 {
    match &m.message_type {
        MessageType::Block(BlockMessage::Transactions(t)) => *t.view,
        other => panic!("unexpected message {other:?}"),
    }
}

async fn recv(net: &mut Cliquenet<TestTypes>) -> Message<TestTypes, Unchecked> {
    timeout(RECV_TIMEOUT, net.receive())
        .await
        .expect("timed out waiting for message")
        .expect("receive failed")
}

async fn assert_silent(net: &mut Cliquenet<TestTypes>, what: &str) {
    if let Ok(m) = timeout(SILENCE, net.receive()).await {
        panic!("{what}: unexpectedly received {m:?}");
    }
}

/// Forwarder F peers with validator V from the stake table and observer O as
/// a static Passive peer; O peers only with F.
struct Mesh {
    f: Node,
    v: Node,
    o: Node,
    net_f: Cliquenet<TestTypes>,
    net_v: Cliquenet<TestTypes>,
    net_o: Cliquenet<TestTypes>,
}

impl Mesh {
    async fn connect() -> Self {
        crate::logging::init_test_logging();
        let f = Node::new(0);
        let v = Node::new(1);
        let o = Node::new(2);
        let net_f = f
            .network(
                vec![v.peer()],
                PeerPolicy::StakeTable {
                    observers: vec![o.peer()],
                },
            )
            .await
            .unwrap();
        let net_v = v
            .network(vec![f.peer()], PeerPolicy::default())
            .await
            .unwrap();
        let net_o = o
            .network(
                vec![],
                PeerPolicy::StaticOnly {
                    upstreams: vec![f.peer()],
                },
            )
            .await
            .unwrap();
        sleep(SETTLE).await;
        Self {
            f,
            v,
            o,
            net_f,
            net_v,
            net_o,
        }
    }
}

#[tokio::test]
async fn observer_receives_forwarded_messages_only() {
    let mut mesh = Mesh::connect().await;
    let view = ViewNumber::new(1);

    // A broadcast reaches the stake-table peer but not the Passive observer.
    mesh.net_f
        .sender()
        .broadcast(view, &message(&mesh.f, 1))
        .unwrap();
    let got = recv(&mut mesh.net_v).await;
    assert_eq!(got.sender, mesh.f.public_key);
    assert_eq!(view_of(&got), 1);
    // A broadcast is also delivered to the sender itself.
    assert_eq!(view_of(&recv(&mut mesh.net_f).await), 1);
    assert_silent(&mut mesh.net_o, "observer got a broadcast").await;

    // A forwarded message reaches only the observer.
    mesh.net_f
        .sender()
        .send_to_observers(view, &message(&mesh.f, 2))
        .unwrap();
    let got = recv(&mut mesh.net_o).await;
    assert_eq!(got.sender, mesh.f.public_key);
    assert_eq!(view_of(&got), 2);
    assert_silent(&mut mesh.net_v, "validator got an observer message").await;

    // A node without observers can call `send_to_observers` unconditionally.
    mesh.net_v
        .sender()
        .send_to_observers(view, &message(&mesh.v, 3))
        .unwrap();
    assert_silent(
        &mut mesh.net_f,
        "send_to_observers without observers sent something",
    )
    .await;
    assert_silent(
        &mut mesh.net_o,
        "send_to_observers without observers sent something",
    )
    .await;
}

#[tokio::test]
async fn observer_messages_are_accepted_by_upstream() {
    let mut mesh = Mesh::connect().await;
    let view = ViewNumber::new(1);

    // The observer's upstream is Active on its side, so its broadcasts reach F,
    // and F accepts them because O is in its peer table.
    mesh.net_o
        .sender()
        .broadcast(view, &message(&mesh.o, 4))
        .unwrap();
    let got = recv(&mut mesh.net_f).await;
    assert_eq!(got.sender, mesh.o.public_key);
    assert_eq!(view_of(&got), 4);

    mesh.net_o
        .sender()
        .unicast(view, &mesh.f.public_key, &message(&mesh.o, 5))
        .unwrap();
    let got = recv(&mut mesh.net_f).await;
    assert_eq!(view_of(&got), 5);

    // O is not a peer of V at all.
    assert_silent(&mut mesh.net_v, "validator got an observer broadcast").await;
}

#[tokio::test]
async fn apply_epoch_keeps_static_peers() {
    let mut mesh = Mesh::connect().await;

    // Stake table: F and V only; O is never a member.
    let schedule = StakeTableSchedule {
        initial: vec![0, 1],
        changes: vec![],
        addr_overrides: Default::default(),
    };
    let infos = [mesh.f.peer().1, mesh.v.peer().1];
    let (coord, ..) = mock_membership_with_client_and_schedule(
        2,
        10,
        mesh.f.public_key,
        TestStorage::default(),
        &schedule,
        &infos,
    );

    mesh.net_f.apply_epoch(EpochNumber::new(1), &coord).unwrap();
    let mut peers: Vec<_> = mesh.net_f.sender().peers().into_keys().collect();
    peers.sort();
    // Stake-table peers (including this node itself) plus the static observer.
    let mut expected = vec![mesh.f.public_key, mesh.v.public_key, mesh.o.public_key];
    expected.sort();
    assert_eq!(peers, expected, "static observer must survive apply_epoch");

    // The observer records the epoch but does not dial the stake table.
    mesh.net_o.apply_epoch(EpochNumber::new(1), &coord).unwrap();
    let peers = mesh.net_o.sender().peers();
    assert_eq!(peers.len(), 1);
    assert!(peers.contains_key(&mesh.f.public_key));
    assert!(!peers.contains_key(&mesh.v.public_key));

    // Static peers cannot be removed.
    mesh.net_f.remove_peers(vec![&mesh.o.public_key]).unwrap();
    assert!(mesh.net_f.sender().peers().contains_key(&mesh.o.public_key));

    // Forwarding still works after the epoch update.
    let view = ViewNumber::new(2);
    mesh.net_f
        .sender()
        .send_to_observers(view, &message(&mesh.f, 6))
        .unwrap();
    assert_eq!(view_of(&recv(&mut mesh.net_o).await), 6);
}

#[tokio::test]
async fn rejects_self_and_duplicate_static_peers() {
    crate::logging::init_test_logging();
    let me = Node::new(0);
    let other = Node::new(1);

    let err = me
        .network(
            vec![],
            PeerPolicy::StakeTable {
                observers: vec![me.peer()],
            },
        )
        .await
        .expect_err("own key must be rejected");
    assert!(matches!(err, NetworkError::StaticPeers(_)), "{err}");

    let err = me
        .network(
            vec![],
            PeerPolicy::StaticOnly {
                upstreams: vec![other.peer(), other.peer()],
            },
        )
        .await
        .expect_err("duplicate key must be rejected");
    assert!(matches!(err, NetworkError::StaticPeers(_)), "{err}");

    // A static peer may also be listed among the initial parties; the static
    // entry wins and is not a duplicate.
    let net = me
        .network(
            vec![other.peer()],
            PeerPolicy::StakeTable {
                observers: vec![other.peer()],
            },
        )
        .await
        .unwrap();
    assert_eq!(net.sender().static_peer_keys(), vec![other.public_key]);
    assert!(!net.sender().is_observer());
}
