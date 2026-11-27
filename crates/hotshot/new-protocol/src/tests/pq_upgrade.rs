//! SLH-DSA signatures on upgrade votes, carried in a trailer after the message
//! (see [`crate::pq`]): over a real cliquenet, and end to end with every node
//! running a coordinator.

use std::{
    collections::{BTreeMap, HashMap},
    net::Ipv4Addr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    PeerConnectInfo,
    addr::NetAddr,
    data::{EpochNumber, ViewNumber},
    message::UpgradeLock,
    signature_key::BLSPubKey,
    simple_vote::UpgradeVote2,
    slh_dsa::{self, SIGNATURE_LEN},
    traits::{metrics::NoMetrics, signature_key::SignatureKey},
    upgrade_config::UpgradeConfig,
    x25519::Keypair,
};
use tokio::time::timeout;
use versions::{NEW_PROTOCOL_VERSION, Upgrade, version};

use crate::{
    helpers::test_upgrade_lock,
    message::{ConsensusMessage, Message, MessageType, Unchecked, Validated},
    network::Cliquenet,
    pq::{PqKeys, PqTally, PqVoting},
    tests::common::{assertions::node_index_for_key, runner::TestRunner},
    upgrade::expected_upgrade_data,
};

const VIEW: ViewNumber = ViewNumber::new(7);

/// Peer B receives the message peer A broadcast with a trailer, and the
/// signature in it. A receives its own broadcast too, trailer included.
#[tokio::test(flavor = "multi_thread")]
async fn broadcast_with_trailer_delivers_message_and_signature() {
    let (mut a, mut b, message) = connected_pair().await;
    let signature = filler_signature();

    a.sender()
        .broadcast_with_trailer(VIEW, &message, &signature)
        .unwrap();

    let (received, trailer) = receive_with_trailer(&mut b).await;
    assert_eq!(received, message.clone().into_unchecked());
    assert_eq!(trailer, Some(signature.clone()));

    let (own, own_trailer) = receive_with_trailer(&mut a).await;
    assert_eq!(own, message.into_unchecked());
    assert_eq!(own_trailer, Some(signature));
}

/// A message broadcast as today carries no trailer.
#[tokio::test(flavor = "multi_thread")]
async fn plain_broadcast_has_no_trailer() {
    let (a, mut b, message) = connected_pair().await;

    a.sender().broadcast(VIEW, &message).unwrap();

    let (received, trailer) = receive_with_trailer(&mut b).await;
    assert_eq!(received, message.into_unchecked());
    assert_eq!(trailer, None);
}

/// The unchanged `receive` reads a message sent with a trailer as the message
/// alone, as a 0.7.x node does.
#[tokio::test(flavor = "multi_thread")]
async fn old_receive_reads_message_sent_with_trailer() {
    let (a, mut b, message) = connected_pair().await;

    a.sender()
        .broadcast_with_trailer(VIEW, &message, &filler_signature())
        .unwrap();

    let received = timeout(Duration::from_secs(10), b.receive())
        .await
        .expect("the message should arrive")
        .unwrap();
    assert_eq!(received, message.into_unchecked());
}

/// The P2 gate, on a mixed network: nodes 0-2 vote with an SLH-DSA trailer,
/// nodes 3-4 vote without one, as 0.7.x does. The upgrade decides on every
/// node, and every node tallies exactly nodes 0-2 for the decided upgrade's
/// view. That includes the node itself: a node receives its own broadcast.
#[tokio::test(flavor = "multi_thread")]
async fn mixed_network_tallies_exactly_the_slh_dsa_voters() {
    let keypairs: Vec<_> = (0..3)
        .map(|_| Arc::new(slh_dsa::Keypair::generate()))
        .collect();
    let keys = RegisteredKeys::new(keypairs.iter().map(|keypair| keypair.public_key()));
    let tallies: BTreeMap<usize, PqTally<TestTypes>> =
        (0..NUM_NODES).map(|i| (i, PqTally::default())).collect();
    let pq_voting = tallies
        .iter()
        .map(|(&i, tally)| {
            let voting = PqVoting {
                keypair: keypairs.get(i).cloned(),
                keys: keys.clone(),
                tally: tally.clone(),
            };
            (i, voting)
        })
        .collect();

    let mut runner = TestRunner::builder()
        .num_nodes(NUM_NODES)
        .epoch_height(10)
        .target_decisions(45)
        .upgrade(upgrade())
        .upgrade_config(window(5, 15))
        .pq_voting(pq_voting)
        .build();
    runner.run().await.unwrap();

    assert_upgraded(&runner).await;
    assert_tallied(&runner, &tallies, &[0, 1, 2]);
}

/// Node 0 signs its trailer with a key other than the one registered for it,
/// so no node tallies it, while nodes 1 and 2 are tallied. Its BLS vote still
/// counts: node 4's upgrade votes are dropped everywhere, so the upgrade
/// threshold of 4 of 5 needs node 0's vote. Node 3 has `PqVoting` without a
/// keypair; node 4 has none, as 0.7.x.
#[tokio::test(flavor = "multi_thread")]
async fn trailer_from_unregistered_key_is_not_tallied_but_bls_vote_counts() {
    let keypairs: Vec<_> = (0..3)
        .map(|_| Arc::new(slh_dsa::Keypair::generate()))
        .collect();
    let registered_for_node_0 = slh_dsa::Keypair::generate().public_key();
    let keys = RegisteredKeys::new(
        std::iter::once(registered_for_node_0)
            .chain(keypairs[1..].iter().map(|keypair| keypair.public_key())),
    );
    let tallies: BTreeMap<usize, PqTally<TestTypes>> =
        (0..4).map(|i| (i, PqTally::default())).collect();
    let pq_voting = tallies
        .iter()
        .map(|(&i, tally)| {
            let voting = PqVoting {
                keypair: keypairs.get(i).cloned(),
                keys: keys.clone(),
                tally: tally.clone(),
            };
            (i, voting)
        })
        .collect();
    let node_4 = node_key(4);
    let dropped = Arc::new(AtomicUsize::new(0));
    let dropping = dropped.clone();

    let mut runner = TestRunner::builder()
        .num_nodes(NUM_NODES)
        .epoch_height(10)
        .target_decisions(45)
        .upgrade(upgrade())
        .upgrade_config(window(5, 15))
        .pq_voting(pq_voting)
        .drop_inbound(Arc::new(move |_node, message| {
            let drop = matches!(
                &message.message_type,
                MessageType::Consensus(ConsensusMessage::UpgradeVote(vote))
                    if vote.signature.0 == node_4
            );
            if drop {
                dropping.fetch_add(1, Ordering::Relaxed);
            }
            drop
        }))
        .build();
    runner.run().await.unwrap();

    assert!(
        dropped.load(Ordering::Relaxed) > 0,
        "node 4's upgrade votes were never dropped"
    );
    assert_upgraded(&runner).await;
    assert_tallied(&runner, &tallies, &[1, 2]);
}

const NUM_NODES: usize = 5;

/// As in `tests/upgrade.rs`.
fn upgrade() -> Upgrade {
    Upgrade::new(NEW_PROTOCOL_VERSION, version(0, 7))
}

/// Proposing open for views [start, stop); voting and times unrestricted.
fn window(start: u64, stop: u64) -> UpgradeConfig {
    UpgradeConfig {
        start_proposing_view: start,
        stop_proposing_view: stop,
        start_voting_view: 0,
        stop_voting_view: u64::MAX,
        start_proposing_time: 0,
        stop_proposing_time: u64::MAX,
        start_voting_time: 0,
        stop_voting_time: u64::MAX,
    }
}

/// The key `TestRunner` gives node `i`.
fn node_key(i: usize) -> BLSPubKey {
    BLSPubKey::generated_from_seed_indexed([0; 32], i as u64).0
}

/// SLH-DSA keys registered by nodes 0, 1, ... in order, for every epoch.
struct RegisteredKeys(HashMap<BLSPubKey, slh_dsa::PublicKey>);

impl RegisteredKeys {
    fn new(keys: impl IntoIterator<Item = slh_dsa::PublicKey>) -> Arc<Self> {
        Arc::new(Self((0..).map(node_key).zip(keys).collect()))
    }
}

impl PqKeys<TestTypes> for RegisteredKeys {
    fn slh_dsa_key(&self, _epoch: EpochNumber, voter: &BLSPubKey) -> Option<slh_dsa::PublicKey> {
        self.0.get(voter).copied()
    }
}

/// Every node decided the upgrade and persisted its certificate, as
/// `assert_upgraded` in `tests/upgrade.rs` checks.
async fn assert_upgraded(runner: &TestRunner) {
    for (i, lock) in runner.node_locks().iter().enumerate() {
        let lock = lock.as_ref().expect("node ran");
        let cert = lock
            .decided_upgrade_cert()
            .unwrap_or_else(|| panic!("node {i} did not decide the upgrade"));
        assert_eq!(cert.data.old_version, NEW_PROTOCOL_VERSION);
        assert_eq!(cert.data.new_version, version(0, 7));
    }
    for (i, storage) in runner.node_storages().iter().enumerate() {
        assert!(
            storage.decided_upgrade_certificate().await.is_some(),
            "node {i} did not persist the decided upgrade certificate"
        );
    }
}

/// Each node in `tallies` tallied exactly `signers`, each once, for the view
/// of the upgrade it decided.
fn assert_tallied(
    runner: &TestRunner,
    tallies: &BTreeMap<usize, PqTally<TestTypes>>,
    signers: &[u64],
) {
    for (&i, tally) in tallies {
        let view = decided_upgrade_view(runner, i);
        let mut tallied: Vec<u64> = tally.signers(view).iter().map(node_index_for_key).collect();
        tallied.sort_unstable();
        assert_eq!(
            tallied, signers,
            "node {i}, upgrade proposed at view {view}"
        );
    }
}

/// The view of the upgrade proposal whose certificate node `i` decided.
fn decided_upgrade_view(runner: &TestRunner, i: usize) -> ViewNumber {
    runner.node_locks()[i]
        .as_ref()
        .and_then(UpgradeLock::decided_upgrade_cert)
        .unwrap_or_else(|| panic!("node {i} did not decide the upgrade"))
        .view_number
}

/// Two connected networks, A and B, and an upgrade vote of A for `VIEW`.
async fn connected_pair() -> (
    Cliquenet<TestTypes>,
    Cliquenet<TestTypes>,
    Message<TestTypes, Validated>,
) {
    let a = Peer::new(0);
    let b = Peer::new(1);
    let message = upgrade_vote_message(&a);
    let a_party = (a.key, a.info.clone());
    let b_party = (b.key, b.info.clone());
    let a = a.start(b_party).await;
    let b = b.start(a_party).await;
    (a, b, message)
}

/// A network party that is not running yet.
struct Peer {
    key: BLSPubKey,
    private_key: <BLSPubKey as SignatureKey>::PrivateKey,
    keypair: Keypair,
    info: PeerConnectInfo,
}

impl Peer {
    fn new(index: u64) -> Self {
        let (key, private_key) = BLSPubKey::generated_from_seed_indexed([3; 32], index);
        let keypair = Keypair::generate().expect("keypair generation should succeed");
        let port =
            test_utils::reserve_tcp_port().expect("OS should have ephemeral ports available");
        let info = PeerConnectInfo {
            x25519_key: keypair.public_key(),
            p2p_addr: NetAddr::Inet(Ipv4Addr::LOCALHOST.into(), port),
        };
        Self {
            key,
            private_key,
            keypair,
            info,
        }
    }

    async fn start(self, party: (BLSPubKey, PeerConnectInfo)) -> Cliquenet<TestTypes> {
        // Networks handshake only with networks of the same name.
        Cliquenet::create(
            "test-pq-trailer",
            self.key,
            self.keypair,
            self.info.p2p_addr,
            [party],
            None,
            test_upgrade_lock(),
            Box::new(NoMetrics),
        )
        .await
        .expect("cliquenet creation should succeed")
    }
}

/// A BLS-signed upgrade vote of `peer` for the upgrade proposed at `VIEW`.
fn upgrade_vote_message(peer: &Peer) -> Message<TestTypes, Validated> {
    let lock: UpgradeLock<TestTypes> = test_upgrade_lock();
    let data = expected_upgrade_data(&lock.upgrade(), VIEW, EpochNumber::new(1)).unwrap();
    let vote =
        UpgradeVote2::create_signed_vote(data, VIEW, &peer.key, &peer.private_key, &lock).unwrap();
    Message {
        sender: peer.key,
        message_type: MessageType::Consensus(ConsensusMessage::UpgradeVote(vote)),
    }
}

async fn receive_with_trailer(
    network: &mut Cliquenet<TestTypes>,
) -> (Message<TestTypes, Unchecked>, Option<slh_dsa::Signature>) {
    timeout(Duration::from_secs(10), network.receive_with_trailer())
        .await
        .expect("the message should arrive")
        .unwrap()
}

/// A well-formed signature that no key made: the network does not check it.
fn filler_signature() -> slh_dsa::Signature {
    let bytes: Vec<u8> = (0..SIGNATURE_LEN).map(|i| (i % 251) as u8).collect();
    slh_dsa::Signature::try_from(bytes.as_slice()).unwrap()
}
