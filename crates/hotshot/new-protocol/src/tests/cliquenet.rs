use std::{
    collections::{BTreeMap, BTreeSet},
    net::Ipv4Addr,
    time::Duration,
};

use cliquenet::noise::Protocol;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    PeerConnectInfo,
    addr::NetAddr,
    data::ViewNumber,
    signature_key::BLSPubKey,
    traits::{metrics::NoMetrics, signature_key::SignatureKey},
    x25519::Keypair,
};
use tokio::time::timeout;

use crate::{
    helpers::test_upgrade_lock,
    message::{ConsensusMessage, Message, MessageType, Unchecked},
    network::{CONTROL_LANE_PORT_OFFSET, Cliquenet, ControlLane, NetworkError},
    tests::common::{
        runner::{TestRunner, reserve_ports},
        utils::TestData,
    },
};

#[tokio::test(flavor = "multi_thread")]
async fn five_nodes_decide_same_chain_over_cliquenet() {
    TestRunner::builder().build().run().await.unwrap();
}

/// Proposals travel on the control lane alone, so the network only decides if every node's
/// lane is bound, its peers' lanes derived from their main addresses, and the inbound merge
/// of the two networks delivers both.
#[tokio::test(flavor = "multi_thread")]
async fn five_nodes_decide_over_the_control_lane() {
    TestRunner::builder()
        .control_lane(ControlLane::On)
        .build()
        .run()
        .await
        .unwrap();
}

/// Each epoch's peer update goes to both networks, and epoch changes travel the main one.
#[tokio::test(flavor = "multi_thread")]
async fn epoch_changes_over_the_control_lane() {
    TestRunner::builder()
        .control_lane(ControlLane::On)
        .epoch_height(10)
        .target_decisions(50)
        .build()
        .run()
        .await
        .unwrap();
}

/// With the lane on, a proposal travels the control lane and a VID share the main network,
/// each to the peer's lane at the port above its registered address.
#[tokio::test(flavor = "multi_thread")]
async fn control_lane_carries_proposals_apart_from_shares() {
    let data = TestData::new(1).await;
    let view = &data.views[0];
    let (network, mut peer) = lane_pair().await;

    // The share goes first: on one network the proposal would queue behind it.
    for message in [view.vid_share_broadcast_input(0), view.proposal_input()] {
        network
            .sender()
            .broadcast(view.view_number, &message)
            .unwrap();
    }

    assert!(matches!(
        next_message(&mut peer.control).await,
        MessageType::Consensus(ConsensusMessage::Proposal(_))
    ));
    assert!(matches!(
        next_message(&mut peer.main).await,
        MessageType::Consensus(ConsensusMessage::VidShareBroadcast(_))
    ));
}

/// The lane listens one port above the main network, so a main port with no port above it
/// is refused before anything binds.
#[tokio::test(flavor = "multi_thread")]
async fn control_lane_needs_a_port_above_the_main_one() {
    let (key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], 0);
    let keypair = Keypair::derive_from::<BLSPubKey>(&private_key).unwrap();
    for port in [0, u16::MAX] {
        let result = Cliquenet::<TestTypes>::create(
            "lane-test",
            key,
            keypair.clone(),
            NetAddr::Inet(Ipv4Addr::LOCALHOST.into(), port),
            [],
            None,
            ControlLane::On,
            test_upgrade_lock(),
            Box::new(NoMetrics),
        )
        .await;
        assert!(
            matches!(result, Err(NetworkError::NoControlLanePort(_))),
            "port {port}: {result:?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn three_nodes_decide_over_cliquenet() {
    TestRunner::builder()
        .num_nodes(3)
        .target_decisions(50)
        .build()
        .run()
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn epoch_changes_over_cliquenet() {
    TestRunner::builder()
        .epoch_height(10)
        .target_decisions(50)
        .build()
        .run()
        .await
        .unwrap();
}

/// Nodes 1 and 4 cannot connect to each other; cliquenet does not relay.
/// When one of them leads the other never receives the proposal and cannot
/// vote, leaving 6 of 7 votes against the quorum threshold of 5
/// (`2n/3 + 1`) — one vote of slack, so every view is still required to
/// decide. The pair sits 3 apart in the round-robin leader order, so a
/// connected leader always follows a blocked one and re-propagates its
/// certificates.
#[tokio::test(flavor = "multi_thread")]
async fn blocked_pair_still_decides_over_cliquenet() {
    TestRunner::builder()
        .num_nodes(7)
        .target_decisions(30)
        .epoch_height(10)
        .blocked_pairs(BTreeSet::from([(1, 4)]))
        .build()
        .run()
        .await
        .unwrap();
}

/// As in [`blocked_pair_still_decides_over_cliquenet`], but nodes 1, 4 and
/// 7 of 10 are mutually unreachable: a blocked leader still collects 8 of
/// 10 votes against the quorum threshold of 7 (`2n/3 + 1`) — one vote of
/// slack.
#[tokio::test(flavor = "multi_thread")]
async fn mutually_blocked_nodes_still_decide_over_cliquenet() {
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(30)
        .epoch_height(10)
        .blocked_pairs(BTreeSet::from([(1, 4), (1, 7), (4, 7)]))
        .build()
        .run()
        .await
        .unwrap();
}

/// A node that misses one view's VID share broadcasts recovers by fetching
/// the payload, and the network certifies again.
///
/// Five of seven nodes are up, which is exactly the quorum threshold, so no
/// view can certify without every one of them. Node 1 receives view 2's
/// proposal, its own share fragments and every vote, and so takes full part
/// in that view — but never the other voters' share broadcasts, so it cannot
/// reconstruct the block. From then on it cannot vote on any proposal
/// parented at view 2, which is every later proposal, and the run only
/// completes if it recovers the payload from a peer.
///
/// The views around the deficit are tolerated rather than pinned: how many
/// time out before the fetch lands depends on scheduling. Completing at all
/// is the assertion — without the fetch this run never reaches its target.
#[tokio::test(flavor = "multi_thread")]
async fn share_starved_node_recovers_over_cliquenet() {
    TestRunner::builder()
        .num_nodes(7)
        .down_nodes(BTreeSet::from([5, 6]))
        .starved_of_shares(BTreeMap::from([(1, BTreeSet::from([ViewNumber::new(2)]))]))
        .tolerated_failed_views((2..=8).map(ViewNumber::new).collect())
        .target_decisions(12)
        .view_timeout(std::time::Duration::from_secs(2))
        .build()
        .run()
        .await
        .unwrap();
}

/// A peer of a node with the control lane on, its two networks opened directly so a test sees
/// which one a message arrives on.
struct LanePeer {
    main: cliquenet::Network,
    control: cliquenet::Network,
}

/// A node with the control lane on, and its one peer.
async fn lane_pair() -> (Cliquenet<TestTypes>, LanePeer) {
    let node = |i| {
        let (key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], i);
        let keypair = Keypair::derive_from::<BLSPubKey>(&private_key).unwrap();
        let addr = NetAddr::Inet(Ipv4Addr::LOCALHOST.into(), reserve_ports(ControlLane::On));
        (key, keypair, addr)
    };
    let (key, keypair, addr) = node(0);
    let (peer_key, peer_keypair, peer_addr) = node(1);

    let lane = |addr: &NetAddr| addr.clone().with_offset(CONTROL_LANE_PORT_OFFSET);
    let open = |name: &str, bind: NetAddr, node: NetAddr| {
        cliquenet::Network::create(
            cliquenet::Config::builder()
                .name(name)
                .keypair(peer_keypair.clone().into())
                .bind(bind)
                .parties([(keypair.public_key().into(), node)])
                .noise_protocols([(1.into(), Protocol::IK_25519_AesGcm_Blake2s)])
                .build(),
        )
    };
    let peer = LanePeer {
        main: open("lane-test", peer_addr.clone(), addr.clone())
            .await
            .unwrap(),
        control: open("lane-test-control", lane(&peer_addr), lane(&addr))
            .await
            .unwrap(),
    };

    let peer_info = PeerConnectInfo {
        x25519_key: peer_keypair.public_key(),
        p2p_addr: peer_addr,
    };
    let network = Cliquenet::create(
        "lane-test",
        key,
        keypair,
        addr,
        [(peer_key, peer_info)],
        None,
        ControlLane::On,
        test_upgrade_lock(),
        Box::new(NoMetrics),
    )
    .await
    .unwrap();
    (network, peer)
}

/// The next message `network` delivers.
async fn next_message(network: &mut cliquenet::Network) -> MessageType<TestTypes, Unchecked> {
    let (_, bytes) = timeout(Duration::from_secs(30), network.receive())
        .await
        .expect("a message arrives")
        .expect("the network is open");
    let (message, _) = test_upgrade_lock::<TestTypes>()
        .deserialize::<Message<TestTypes, Unchecked>>(&bytes)
        .unwrap();
    message.message_type
}
