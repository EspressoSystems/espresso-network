use std::{net::Ipv4Addr, time::Duration};

use bytes::Bytes;
use cliquenet::{
    Config, Network, Role, Slot,
    error::NetworkError,
    x25519::{Keypair, PublicKey},
};
use tokio::time::{sleep, timeout};

// -- Helpers -----------------------------------------------------------------

const TIMEOUT: Duration = Duration::from_secs(10);

/// Time to wait for connections to establish after creating networks.
const SETTLE: Duration = Duration::from_secs(2);

struct Node {
    key: PublicKey,
    port: u16,
    keypair: Keypair,
}

impl Node {
    fn new(port: u16) -> Self {
        let keypair = Keypair::generate().unwrap();
        let key = keypair.public_key();
        Self { key, port, keypair }
    }

    fn addr(&self) -> (Ipv4Addr, u16) {
        (Ipv4Addr::LOCALHOST, self.port)
    }
}

fn reserve_port() -> u16 {
    let s = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let a = s.local_addr().unwrap();
    let _ = std::net::TcpStream::connect(a).unwrap();
    let _ = s.accept().unwrap();
    a.port()
}

/// Create a Config for a node, listing all nodes as parties.
fn make_config(node: &Node, all: &[&Node]) -> Config {
    Config::builder()
        .name("test")
        .keypair(node.keypair.clone())
        .bind(node.addr().into())
        .parties(all.iter().map(|n| (n.key, n.addr().into())))
        .receive_timeout(Duration::from_secs(5))
        .retry_delays(vec![1, 3])
        .max_retry_delay(Duration::from_secs(5))
        .build()
}

/// Create a two-node network and wait for connections.
async fn two_nodes() -> (Network, Network, PublicKey, PublicKey) {
    let a = Node::new(reserve_port());
    let b = Node::new(reserve_port());
    let pka = a.key;
    let pkb = b.key;
    let all = [&a, &b];

    let net_a = Network::create(make_config(&a, &all)).await.unwrap();
    let net_b = Network::create(make_config(&b, &all)).await.unwrap();

    sleep(SETTLE).await;

    (net_a, net_b, pka, pkb)
}

// -- Tests -------------------------------------------------------------------

/// Unicast from A to B.
#[tokio::test]
async fn unicast() {
    let (mut net_a, mut net_b, pka, pkb) = two_nodes().await;

    net_a.unicast(Slot::MIN, pkb, b"hello".to_vec()).unwrap();

    let (src, data) = timeout(TIMEOUT, net_b.receive())
        .await
        .expect("timed out")
        .expect("channel closed");

    assert_eq!(src, pka);
    assert_eq!(data, Bytes::from("hello"));
}

/// Unicast multiple messages, all arrive in order.
#[tokio::test]
async fn unicast_multiple() {
    let (mut net_a, mut net_b, _pka, pkb) = two_nodes().await;

    let n = 10usize;
    for i in 0..n {
        net_a
            .unicast(Slot::MIN, pkb, format!("msg-{i}").into_bytes())
            .unwrap();
    }

    for i in 0..n {
        let (_, data) = timeout(TIMEOUT, net_b.receive())
            .await
            .expect("timed out")
            .expect("channel closed");
        assert_eq!(data, Bytes::from(format!("msg-{i}")));
    }
}

/// Broadcast delivers to all active parties including self.
#[tokio::test]
async fn broadcast() {
    let (mut net_a, mut net_b, pka, _pkb) = two_nodes().await;

    net_a.broadcast(Slot::MIN, b"bcast".to_vec()).unwrap();

    // Collect both messages (self + remote, order not guaranteed).
    let mut msgs = Vec::new();
    for _ in 0..2 {
        let (src, data) = timeout(TIMEOUT, async {
            tokio::select! {
                m = net_a.receive() => m,
                m = net_b.receive() => m,
            }
        })
        .await
        .expect("timed out")
        .expect("channel closed");
        assert_eq!(src, pka);
        msgs.push(data);
    }

    msgs.sort();
    assert_eq!(msgs, vec![Bytes::from("bcast"), Bytes::from("bcast")]);
}

/// Bidirectional: both nodes send to each other.
#[tokio::test]
async fn bidirectional() {
    let (mut net_a, mut net_b, pka, pkb) = two_nodes().await;

    net_a.unicast(Slot::MIN, pkb, b"from-a".to_vec()).unwrap();
    net_b.unicast(Slot::MIN, pka, b"from-b".to_vec()).unwrap();

    let (src, data) = timeout(TIMEOUT, net_b.receive())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(src, pka);
    assert_eq!(data, Bytes::from("from-a"));

    let (src, data) = timeout(TIMEOUT, net_a.receive())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(src, pkb);
    assert_eq!(data, Bytes::from("from-b"));
}

/// Multicast delivers to selected peers and self (if included).
#[tokio::test]
async fn multicast() {
    let a = Node::new(reserve_port());
    let b = Node::new(reserve_port());
    let c = Node::new(reserve_port());
    let pka = a.key;
    let pkb = b.key;
    let all = [&a, &b, &c];

    let mut net_a = Network::create(make_config(&a, &all)).await.unwrap();
    let mut net_b = Network::create(make_config(&b, &all)).await.unwrap();
    let mut net_c = Network::create(make_config(&c, &all)).await.unwrap();

    sleep(SETTLE).await;

    net_a
        .multicast(Slot::MIN, [pka, pkb], b"multi".to_vec())
        .unwrap();

    // A receives (self).
    let (_, data) = timeout(TIMEOUT, net_a.receive())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(data, Bytes::from("multi"));

    // B receives.
    let (_, data) = timeout(TIMEOUT, net_b.receive())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(data, Bytes::from("multi"));

    // C should NOT receive.
    assert!(
        timeout(Duration::from_millis(500), net_c.receive())
            .await
            .is_err(),
        "C received unexpected message"
    );
}

/// Self-unicast delivers locally without going through the network.
#[tokio::test]
async fn self_unicast() {
    let (mut net_a, _net_b, pka, _pkb) = two_nodes().await;

    net_a.unicast(Slot::MIN, pka, b"self".to_vec()).unwrap();

    let (src, data) = timeout(TIMEOUT, net_a.receive())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(src, pka);
    assert_eq!(data, Bytes::from("self"));
}

/// Large message spanning multiple Noise frames.
#[tokio::test]
async fn large_message() {
    let (mut net_a, mut net_b, _pka, pkb) = two_nodes().await;

    let big: Vec<u8> = (0..200 * 1024).map(|i| (i % 251) as u8).collect();
    net_a.unicast(Slot::MIN, pkb, big.clone()).unwrap();

    let (_, data) = timeout(TIMEOUT, net_b.receive())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(data.as_ref(), big.as_slice());
}

/// Message too large is rejected at the API level.
#[tokio::test]
async fn message_too_large() {
    let (mut net_a, _net_b, _pka, pkb) = two_nodes().await;

    let huge = vec![0u8; 11 * 1024 * 1024]; // > 10 MiB default
    let result = net_a.unicast(Slot::MIN, pkb, huge);
    assert!(matches!(result, Err(NetworkError::MessageTooLarge)));
}

/// Dropping the controller shuts down the network.
#[tokio::test]
async fn shutdown_on_drop() {
    let (net_a, _net_b, _pka, _pkb) = two_nodes().await;

    let (ctrl_a, mut rx_a) = net_a.split_into();
    drop(ctrl_a);

    let result = timeout(Duration::from_secs(2), rx_a.receive()).await;
    match result {
        Ok(None) => {},
        Ok(Some(_)) => panic!("expected None after shutdown"),
        Err(_) => panic!("timed out waiting for shutdown"),
    }
}

/// Three nodes: broadcast from each, all receive from all.
#[tokio::test]
async fn three_node_broadcast() {
    let a = Node::new(reserve_port());
    let b = Node::new(reserve_port());
    let c = Node::new(reserve_port());
    let all = [&a, &b, &c];

    let mut net_a = Network::create(make_config(&a, &all)).await.unwrap();
    let mut net_b = Network::create(make_config(&b, &all)).await.unwrap();
    let mut net_c = Network::create(make_config(&c, &all)).await.unwrap();

    sleep(SETTLE).await;

    net_a.broadcast(Slot::MIN, b"from-a".to_vec()).unwrap();
    net_b.broadcast(Slot::MIN, b"from-b".to_vec()).unwrap();
    net_c.broadcast(Slot::MIN, b"from-c".to_vec()).unwrap();

    for net in [&mut net_a, &mut net_b, &mut net_c] {
        let mut msgs = Vec::new();
        for _ in 0..3 {
            let (_, data) = timeout(TIMEOUT, net.receive())
                .await
                .expect("timed out")
                .expect("channel closed");
            msgs.push(data);
        }
        msgs.sort();
        assert_eq!(
            msgs,
            vec![
                Bytes::from("from-a"),
                Bytes::from("from-b"),
                Bytes::from("from-c"),
            ]
        );
    }
}

/// Empty payload is delivered correctly.
#[tokio::test]
async fn empty_payload() {
    let (mut net_a, mut net_b, _pka, pkb) = two_nodes().await;

    net_a.unicast(Slot::MIN, pkb, vec![]).unwrap();

    let (_, data) = timeout(TIMEOUT, net_b.receive())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert!(data.is_empty());
}

/// GC discards old-slot messages while current-slot messages are delivered.
#[tokio::test]
async fn gc() {
    let (mut net_a, mut net_b, _pka, pkb) = two_nodes().await;

    // Send messages in slots 1 and 3.
    net_a.unicast(Slot::new(1), pkb, b"old".to_vec()).unwrap();
    net_a.unicast(Slot::new(3), pkb, b"new".to_vec()).unwrap();

    // GC up to slots 2 — slot 1 messages should be discarded.
    net_a.gc(Slot::new(2)).unwrap();

    // Only slot 3 should arrive.
    let (_, data) = timeout(TIMEOUT, net_b.receive())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(data, Bytes::from("new"));

    assert!(
        timeout(Duration::from_millis(500), net_b.receive())
            .await
            .is_err(),
        "unexpected extra message"
    );
}

/// Adding a peer mid-session allows sending messages to it.
#[tokio::test]
async fn add_peer() {
    let a = Node::new(reserve_port());
    let b = Node::new(reserve_port());
    let c = Node::new(reserve_port());
    let pkc = c.key;

    // Start A and B knowing only each other.
    let all_ab = [&a, &b];
    let mut net_a = Network::create(make_config(&a, &all_ab)).await.unwrap();
    let _net_b = Network::create(make_config(&b, &all_ab)).await.unwrap();

    // Start C knowing A and B.
    let all_abc = [&a, &b, &c];
    let mut net_c = Network::create(make_config(&c, &all_abc)).await.unwrap();

    // A adds C dynamically.
    net_a
        .add_peers(Role::Active, [(pkc, c.addr().into())])
        .unwrap();

    sleep(SETTLE).await;

    net_a.unicast(Slot::MIN, pkc, b"hello-c".to_vec()).unwrap();

    let (_, data) = timeout(TIMEOUT, net_c.receive())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(data, Bytes::from("hello-c"));
}

/// Removing a peer stops delivery to it; messages to remaining peers still work.
#[tokio::test]
async fn remove_peer() {
    let a = Node::new(reserve_port());
    let b = Node::new(reserve_port());
    let c = Node::new(reserve_port());
    let pkb = b.key;
    let all = [&a, &b, &c];

    let mut net_a = Network::create(make_config(&a, &all)).await.unwrap();
    let mut net_b = Network::create(make_config(&b, &all)).await.unwrap();
    let mut net_c = Network::create(make_config(&c, &all)).await.unwrap();

    sleep(SETTLE).await;

    // Remove B from A's view.
    net_a.remove_peers([pkb]).unwrap();

    // Broadcast from A should only reach C (and self).
    net_a
        .broadcast(Slot::MIN, b"after-remove".to_vec())
        .unwrap();

    let (_, data) = timeout(TIMEOUT, net_c.receive())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(data, Bytes::from("after-remove"));

    // Self delivery.
    let (_, data) = timeout(TIMEOUT, net_a.receive())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(data, Bytes::from("after-remove"));

    // B should NOT receive.
    assert!(
        timeout(Duration::from_millis(500), net_b.receive())
            .await
            .is_err(),
        "removed peer received a message"
    );
}

/// Passive peers are excluded from broadcasts but can receive unicasts.
#[tokio::test]
async fn passive_role() {
    let a = Node::new(reserve_port());
    let b = Node::new(reserve_port());
    let pka = a.key;
    let pkb = b.key;
    let all = [&a, &b];

    let mut net_a = Network::create(make_config(&a, &all)).await.unwrap();
    let mut net_b = Network::create(make_config(&b, &all)).await.unwrap();

    sleep(SETTLE).await;

    // Make B passive from A's perspective.
    net_a.assign_peers(Role::Passive, [pkb]).unwrap();

    // Small delay for the command to be processed.
    sleep(Duration::from_millis(100)).await;

    // Broadcast should NOT reach B.
    net_a.broadcast(Slot::MIN, b"bcast".to_vec()).unwrap();

    // Self should still receive.
    let (_, data) = timeout(TIMEOUT, net_a.receive())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(data, Bytes::from("bcast"));

    // B should not get the broadcast.
    assert!(
        timeout(Duration::from_millis(500), net_b.receive())
            .await
            .is_err(),
        "passive peer received broadcast"
    );

    // Unicast to B should still work.
    net_a.unicast(Slot::MIN, pkb, b"direct".to_vec()).unwrap();

    let (src, data) = timeout(TIMEOUT, net_b.receive())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(src, pka);
    assert_eq!(data, Bytes::from("direct"));
}

/// Two nodes connecting simultaneously stabilise with one connection per pair.
#[tokio::test]
async fn simultaneous_connect() {
    let a = Node::new(reserve_port());
    let b = Node::new(reserve_port());
    let pka = a.key;
    let pkb = b.key;
    let all = [&a, &b];

    // Create both networks at the same time to maximise chance of
    // simultaneous connect attempts.
    let (net_a, net_b) = tokio::join!(
        Network::create(make_config(&a, &all)),
        Network::create(make_config(&b, &all)),
    );
    let mut net_a = net_a.unwrap();
    let mut net_b = net_b.unwrap();

    sleep(SETTLE).await;

    // Verify bidirectional communication works (proves exactly one
    // connection survived in each direction).
    for i in 0..5u32 {
        net_a
            .unicast(Slot::MIN, pkb, format!("a-{i}").into_bytes())
            .unwrap();
        net_b
            .unicast(Slot::MIN, pka, format!("b-{i}").into_bytes())
            .unwrap();
    }

    for i in 0..5u32 {
        let (src, data) = timeout(TIMEOUT, net_b.receive())
            .await
            .expect("timed out")
            .expect("channel closed");
        assert_eq!(src, pka);
        assert_eq!(data, Bytes::from(format!("a-{i}")));
    }

    for i in 0..5u32 {
        let (src, data) = timeout(TIMEOUT, net_a.receive())
            .await
            .expect("timed out")
            .expect("channel closed");
        assert_eq!(src, pkb);
        assert_eq!(data, Bytes::from(format!("b-{i}")));
    }
}

/// A node that restarts eventually receives messages queued by its peer.
#[tokio::test]
async fn reconnect_after_restart() {
    let a = Node::new(reserve_port());
    let b_port = reserve_port();
    let b = Node::new(b_port);
    let pka = a.key;
    let pkb = b.key;

    // B gets a different port for its second life so we don't hit TIME_WAIT.
    let b2_port = reserve_port();
    let b2 = Node {
        key: b.key,
        port: b2_port,
        keypair: b.keypair.clone(),
    };

    // A knows B at its second port so reconnection works.
    let all_a = [&a, &b2];
    // B knows itself and A.
    let all_b = [&a, &b];

    let mut net_a = Network::create(make_config(&a, &all_a)).await.unwrap();
    let net_b = Network::create(make_config(&b, &all_b)).await.unwrap();

    sleep(SETTLE).await;

    // Drop B to simulate a crash.
    drop(net_b);

    // A sends a message while B is down.
    net_a.unicast(Slot::MIN, pkb, b"queued".to_vec()).unwrap();

    // B comes back on a new port with the same keys.
    let all_b2 = [&a, &b2];
    let mut net_b = Network::create(make_config(&b2, &all_b2)).await.unwrap();

    // The message should eventually be delivered via retry.
    let (src, data) = timeout(Duration::from_secs(15), net_b.receive())
        .await
        .expect("timed out waiting for delivery after reconnect")
        .expect("channel closed");
    assert_eq!(src, pka);
    assert_eq!(data, Bytes::from("queued"));
}

/// When a noise-capable peer completes the handshake but its public key is
/// unknown to the server, the server replies with `Hello::backoff` telling
/// the peer to wait before reconnecting. The peer honours this by sleeping
/// the requested duration inside `Connection::connect` instead of retrying
/// on the normal (short) schedule.
///
/// We configure A with a 60 s backoff and C with a 1 s retry delay. After
/// the first handshake, C sleeps 60 s. Since A never adds C, C has no peer
/// and cannot deliver the queued message within the 5 s window.
#[tokio::test]
async fn unknown_peer_backs_off() {
    let a = Node::new(reserve_port());
    let c = Node::new(reserve_port());

    // A does not know C. Unknown peers are told to back off for 60 s.
    let mut net_a = Network::create(
        Config::builder()
            .name("test")
            .keypair(a.keypair.clone())
            .bind(a.addr().into())
            .parties([(a.key, a.addr().into())])
            .receive_timeout(Duration::from_secs(5))
            .retry_delays(vec![1])
            .max_retry_delay(Duration::from_secs(1))
            .backoff_duration(Duration::from_secs(60))
            .build(),
    )
    .await
    .unwrap();

    // C knows A and connects. Without the backoff, C's 1 s retry would
    // cause it to reconnect immediately after each rejection. With the
    // backoff, C sleeps 60 s after the first hello exchange.
    let mut net_c = Network::create(
        Config::builder()
            .name("test")
            .keypair(c.keypair.clone())
            .bind(c.addr().into())
            .parties([(a.key, a.addr().into()), (c.key, c.addr().into())])
            .receive_timeout(Duration::from_secs(5))
            .retry_delays(vec![1])
            .max_retry_delay(Duration::from_secs(1))
            .build(),
    )
    .await
    .unwrap();

    // C connects to A (~0–1 s), handshake succeeds, A replies with
    // backoff(60 s). C's connect loop sleeps instead of retrying.
    sleep(SETTLE).await;

    // C queues a message for A but has no peer (connect is sleeping the
    // backoff), so the message cannot be delivered.
    net_c.unicast(Slot::MIN, a.key, b"hello".to_vec()).unwrap();

    assert!(
        timeout(Duration::from_secs(5), net_a.receive())
            .await
            .is_err(),
        "message arrived despite backoff — peer should not have reconnected yet"
    );
}

/// Updating a party's address via `add_peers` reconnects to the new address
/// and preserves the peer's retry state (messages queued before the update
/// are still delivered after reconnecting).
#[tokio::test]
async fn update_party_address() {
    let a = Node::new(reserve_port());
    let b1 = Node::new(reserve_port());
    let pka = a.key;
    let pkb = b1.key;

    // A knows B at its first address.
    let all = [&a, &b1];
    let mut net_a = Network::create(make_config(&a, &all)).await.unwrap();
    let mut net_b1 = Network::create(make_config(&b1, &all)).await.unwrap();

    sleep(SETTLE).await;

    // Verify the connection works.
    net_a.unicast(Slot::MIN, pkb, b"before".to_vec()).unwrap();
    let (src, data) = timeout(TIMEOUT, net_b1.receive())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(src, pka);
    assert_eq!(data, Bytes::from("before"));

    // B shuts down and comes back on a new port (same keys).
    drop(net_b1);

    let b2_port = reserve_port();
    let b2 = Node {
        key: b1.key,
        port: b2_port,
        keypair: b1.keypair.clone(),
    };
    let all_b2 = [&a, &b2];
    let mut net_b2 = Network::create(make_config(&b2, &all_b2)).await.unwrap();

    // Send a message while B is down — it goes into A's retry queue.
    net_a.unicast(Slot::MIN, pkb, b"during".to_vec()).unwrap();

    // Tell A that B has moved to the new address.
    net_a
        .add_peers(Role::Active, [(pkb, b2.addr().into())])
        .unwrap();

    // The message queued before the address update should be delivered
    // via retry on the new connection (peer state preserved).
    let (src, data) = timeout(Duration::from_secs(15), net_b2.receive())
        .await
        .expect("timed out — peer state may have been lost on address update")
        .expect("channel closed");
    assert_eq!(src, pka);
    assert_eq!(data, Bytes::from("during"));

    // New messages should also work.
    net_a.unicast(Slot::MIN, pkb, b"after".to_vec()).unwrap();
    let (src, data) = timeout(TIMEOUT, net_b2.receive())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(src, pka);
    assert_eq!(data, Bytes::from("after"));
}
