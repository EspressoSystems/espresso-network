use std::{num::NonZeroUsize, sync::Arc, time::Duration};

use bytes::{Bytes, BytesMut};
use tokio::{
    net::TcpListener,
    sync::{OwnedSemaphorePermit, mpsc},
    time::{Instant, timeout},
};
use tokio_util::sync::CancellationToken;

use crate::{
    Config, Keypair, PublicKey,
    addr::NetAddr,
    connection::Connection,
    delay::DelayQueue,
    error::NetworkError,
    metrics::NoMetrics,
    msg::{MsgId, Slot, Trailer, hello::Hello},
    net::{RetryPolicy, peer::Peer},
    noise::Protocol,
    queue::Queue,
};

// -- Helpers -----------------------------------------------------------------

type Rx = mpsc::UnboundedReceiver<(PublicKey, Bytes, Option<OwnedSemaphorePermit>)>;

fn config(kp: Keypair, recv_timeout: Duration) -> Arc<Config> {
    Arc::new(
        Config::builder()
            .name("test")
            .keypair(kp)
            .bind(NetAddr::from((std::net::Ipv4Addr::LOCALHOST, 0u16)))
            .parties(std::iter::empty::<(PublicKey, NetAddr)>())
            .receive_timeout(recv_timeout)
            .connect_retry_delays(vec![2, 5])
            .send_retry_delays(vec![2, 5])
            .noise_protocols([(1.into(), Protocol::IK_25519_AesGcm_Blake2s)])
            .build(),
    )
}

/// Perform a Noise handshake on both ends and return the two `Connection`s.
async fn connection_pair(
    conf_a: Arc<Config>,
    pkb: PublicKey,
    conf_b: Arc<Config>,
) -> (Connection, Connection) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let addr = NetAddr::from((std::net::Ipv4Addr::LOCALHOST, port));
    let (cb, ca) = tokio::join!(
        async {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = Connection::accept(conf_b, stream).await.unwrap();
            let h = conn.recv_hello().await.unwrap();
            assert!(h.is_ok());
            conn.send_hello(Hello::Ok).await.unwrap();
            conn
        },
        Connection::connect(conf_a, pkb, addr),
    );
    (ca, cb)
}

/// Build a payload with the trailer appended.
fn payload(slot: Slot, id: MsgId, data: &[u8]) -> (RetryPolicy, Bytes) {
    let trailer = Trailer::Std { slot, id }.to_bytes();
    let mut buf = BytesMut::new();
    buf.extend_from_slice(data);
    buf.extend_from_slice(trailer.as_ref());
    (RetryPolicy::Default, buf.freeze())
}

/// Create a `Peer` and its inbound message receiver + outbox + slot sender.
fn make_peer(conf: Arc<Config>, budget: usize) -> (Peer, Rx) {
    let (tx, rx) = mpsc::unbounded_channel();
    let peer = Peer::builder()
        .config(conf.clone())
        .budget(NonZeroUsize::new(budget).unwrap())
        .messages(Queue::new())
        .retry(DelayQueue::new(conf))
        .inbound(tx)
        .metrics(Arc::new(NoMetrics))
        .build();
    (peer, rx)
}

// -- Tests -------------------------------------------------------------------

/// A sends a single message to B.
#[tokio::test]
async fn send_receive() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pka = ka.public_key();
    let pkb = kb.public_key();

    let conf_a = config(ka.clone(), Duration::from_secs(5));
    let conf_b = config(kb.clone(), Duration::from_secs(5));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (mut peer_a, _rx_a) = make_peer(conf_a, 10);
    let (mut peer_b, mut rx_b) = make_peer(conf_b, 10);

    let outbox_a = peer_a.msgs.clone();

    let slot = Slot::new(1);
    let id = MsgId::new(1);
    outbox_a.enqueue(slot, id, payload(slot, id, b"hello"));

    let ha = tokio::spawn(async move { peer_a.start(conn_a, CancellationToken::new()).await });
    let hb = tokio::spawn(async move { peer_b.start(conn_b, CancellationToken::new()).await });

    let (src, data, _permit) = timeout(Duration::from_secs(5), rx_b.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    assert_eq!(src, pka);
    assert_eq!(data.as_ref(), b"hello");

    ha.abort();
    hb.abort();
}

/// A sends multiple messages to B; all arrive in slot/id order.
#[tokio::test]
async fn send_multiple() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = config(ka.clone(), Duration::from_secs(5));
    let conf_b = config(kb.clone(), Duration::from_secs(5));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (mut peer_a, _rx_a) = make_peer(conf_a, 10);
    let (mut peer_b, mut rx_b) = make_peer(conf_b, 10);

    let outbox_a = peer_a.msgs.clone();

    let n = 10u64;
    let slot = Slot::new(1);
    for i in 0..n {
        let id = MsgId::new(i);
        let msg = format!("msg-{i}");
        outbox_a.enqueue(slot, id, payload(slot, id, msg.as_bytes()));
    }

    let ha = tokio::spawn(async move { peer_a.start(conn_a, CancellationToken::new()).await });
    let hb = tokio::spawn(async move { peer_b.start(conn_b, CancellationToken::new()).await });

    for i in 0..n {
        let (_, data, _) = timeout(Duration::from_secs(5), rx_b.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        assert_eq!(data, Bytes::from(format!("msg-{i}")));
    }

    ha.abort();
    hb.abort();
}

/// Both peers send to each other simultaneously.
#[tokio::test]
async fn bidirectional() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pka = ka.public_key();
    let pkb = kb.public_key();

    let conf_a = config(ka.clone(), Duration::from_secs(5));
    let conf_b = config(kb.clone(), Duration::from_secs(5));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (mut peer_a, mut rx_a) = make_peer(conf_a, 10);
    let (mut peer_b, mut rx_b) = make_peer(conf_b, 10);

    let outbox_a = peer_a.msgs.clone();
    let outbox_b = peer_b.msgs.clone();

    let slot = Slot::new(1);
    for i in 0..5u64 {
        let id = MsgId::new(i);
        outbox_a.enqueue(slot, id, payload(slot, id, b"from-a"));
        outbox_b.enqueue(slot, id, payload(slot, id, b"from-b"));
    }

    let ha = tokio::spawn(async move { peer_a.start(conn_a, CancellationToken::new()).await });
    let hb = tokio::spawn(async move { peer_b.start(conn_b, CancellationToken::new()).await });

    for _ in 0..5 {
        let (src, data, _) = timeout(Duration::from_secs(5), rx_b.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        assert_eq!(src, pka);
        assert_eq!(data.as_ref(), b"from-a");
    }

    for _ in 0..5 {
        let (src, data, _) = timeout(Duration::from_secs(5), rx_a.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        assert_eq!(src, pkb);
        assert_eq!(data.as_ref(), b"from-b");
    }

    ha.abort();
    hb.abort();
}

/// A message that spans multiple Noise frames is received intact.
#[tokio::test]
async fn large_message() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = config(ka.clone(), Duration::from_secs(5));
    let conf_b = config(kb.clone(), Duration::from_secs(5));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (mut peer_a, _rx_a) = make_peer(conf_a, 10);
    let (mut peer_b, mut rx_b) = make_peer(conf_b, 10);

    let outbox_a = peer_a.msgs.clone();

    let big: Vec<u8> = (0..200 * 1024).map(|i| (i % 251) as u8).collect();
    let slot = Slot::new(1);
    let id = MsgId::new(0);
    outbox_a.enqueue(slot, id, payload(slot, id, &big));

    let ha = tokio::spawn(async move { peer_a.start(conn_a, CancellationToken::new()).await });
    let hb = tokio::spawn(async move { peer_b.start(conn_b, CancellationToken::new()).await });

    let (_, data, _) = timeout(Duration::from_secs(10), rx_b.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    assert_eq!(data.as_ref(), big.as_slice());

    ha.abort();
    hb.abort();
}

/// With budget = 1, delivery is serialised.
#[tokio::test]
async fn slow_receiver() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = config(ka.clone(), Duration::from_secs(5));
    let conf_b = config(kb.clone(), Duration::from_secs(5));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (mut peer_a, _rx_a) = make_peer(conf_a, 10);
    let (mut peer_b, mut rx_b) = make_peer(conf_b, 1);

    let outbox_a = peer_a.msgs.clone();

    let slot = Slot::new(1);
    for i in 0..3u64 {
        let id = MsgId::new(i);
        let msg = format!("m{i}");
        outbox_a.enqueue(slot, id, payload(slot, id, msg.as_bytes()));
    }

    let ha = tokio::spawn(async move { peer_a.start(conn_a, CancellationToken::new()).await });
    let hb = tokio::spawn(async move { peer_b.start(conn_b, CancellationToken::new()).await });

    for i in 0..3u64 {
        let (_, data, permit) = timeout(Duration::from_secs(5), rx_b.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        assert_eq!(data, Bytes::from(format!("m{i}")));
        drop(permit);
    }

    ha.abort();
    hb.abort();
}

/// Messages with a slot below the threshold are silently discarded.
#[tokio::test]
async fn slot_gc() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = config(ka.clone(), Duration::from_secs(5));
    let conf_b = config(kb.clone(), Duration::from_secs(5));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (mut peer_a, _rx_a) = make_peer(conf_a, 10);
    let (mut peer_b, mut rx_b) = make_peer(conf_b, 10);

    let outbox_a = peer_a.msgs.clone();
    let retry_a = peer_a.retry.clone();

    // Slot 3 (below threshold) — should be discarded by B.
    let old_slot = Slot::new(3);
    let old_id = MsgId::new(0);
    outbox_a.enqueue(old_slot, old_id, payload(old_slot, old_id, b"old"));

    // Slot 7 (above threshold) — should be delivered.
    let new_slot = Slot::new(7);
    let new_id = MsgId::new(0);
    outbox_a.enqueue(new_slot, new_id, payload(new_slot, new_id, b"new"));

    outbox_a.gc(Slot::new(5));
    retry_a.gc(Slot::new(5));

    let ha = tokio::spawn(async move { peer_a.start(conn_a, CancellationToken::new()).await });
    let hb = tokio::spawn(async move { peer_b.start(conn_b, CancellationToken::new()).await });

    let (_, data, _) = timeout(Duration::from_secs(5), rx_b.recv())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(data.as_ref(), b"new");

    assert!(
        timeout(Duration::from_millis(500), rx_b.recv())
            .await
            .is_err(),
        "unexpected extra message"
    );

    ha.abort();
    hb.abort();
}

/// If nothing is received after sending, the peer times out.
#[tokio::test]
async fn receive_timeout() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = config(ka.clone(), Duration::from_millis(500));
    let conf_b = config(kb.clone(), Duration::from_secs(5));

    let (conn_a, _conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (tx, _rx) = mpsc::unbounded_channel();
    let outbox = Queue::new();
    let mut peer_a = Peer::builder()
        .config(conf_a.clone())
        .budget(NonZeroUsize::new(10).unwrap())
        .messages(outbox.clone())
        .retry(DelayQueue::new(conf_a))
        .inbound(tx)
        .metrics(Arc::new(NoMetrics))
        .build();

    let slot = Slot::new(1);
    let id = MsgId::new(0);
    outbox.enqueue(slot, id, payload(slot, id, b"ping"));

    let result = timeout(
        Duration::from_secs(5),
        peer_a.start(conn_a, CancellationToken::new()),
    )
    .await
    .expect("test itself timed out");

    assert!(
        matches!(result, Err(NetworkError::Timeout)),
        "expected Timeout, got {result:?}"
    );
}

/// Advancing the threshold mid-flight suppresses old-slot retries.
#[tokio::test]
async fn threshold_advance_mid_flight() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = config(ka.clone(), Duration::from_secs(5));
    let conf_b = config(kb.clone(), Duration::from_secs(5));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (mut peer_a, _rx_a) = make_peer(conf_a, 10);
    let (mut peer_b, mut rx_b) = make_peer(conf_b, 10);

    let outbox_a = peer_a.msgs.clone();
    let retry_a = peer_a.retry.clone();

    let s1 = Slot::new(1);
    let s3 = Slot::new(3);
    outbox_a.enqueue(s1, MsgId::new(0), payload(s1, MsgId::new(0), b"old"));
    outbox_a.enqueue(s3, MsgId::new(0), payload(s3, MsgId::new(0), b"new"));

    let ha = tokio::spawn(async move { peer_a.start(conn_a, CancellationToken::new()).await });
    let hb = tokio::spawn(async move { peer_b.start(conn_b, CancellationToken::new()).await });

    let (_, data1, _) = timeout(Duration::from_secs(5), rx_b.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    // GC slots below 2 mid-flight, as the Server would.
    outbox_a.gc(Slot::new(2));
    retry_a.gc(Slot::new(2));

    let (_, data2, _) = timeout(Duration::from_secs(5), rx_b.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    let mut received = vec![data1, data2];
    received.sort();
    assert_eq!(received, vec![Bytes::from("new"), Bytes::from("old")]);

    assert!(
        timeout(Duration::from_millis(500), rx_b.recv())
            .await
            .is_err(),
        "unexpected extra message"
    );

    ha.abort();
    hb.abort();
}

/// Dropping the inbound receiver causes the peer to return `ChannelClosed`.
#[tokio::test]
async fn channel_closed() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = config(ka.clone(), Duration::from_secs(5));
    let conf_b = config(kb.clone(), Duration::from_secs(5));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (mut peer_a, _rx_a) = make_peer(conf_a, 10);

    let (tx_b, rx_b) = mpsc::unbounded_channel();
    let outbox_b = Queue::new();
    let mut peer_b = Peer::builder()
        .config(conf_b.clone())
        .budget(NonZeroUsize::new(10).unwrap())
        .messages(outbox_b)
        .retry(DelayQueue::new(conf_b))
        .inbound(tx_b)
        .metrics(Arc::new(NoMetrics))
        .build();
    drop(rx_b);

    let slot = Slot::new(1);
    let id = MsgId::new(0);
    peer_a.msgs.enqueue(slot, id, payload(slot, id, b"data"));

    let ha = tokio::spawn(async move { peer_a.start(conn_a, CancellationToken::new()).await });
    let result = timeout(
        Duration::from_secs(5),
        peer_b.start(conn_b, CancellationToken::new()),
    )
    .await
    .expect("test itself timed out");

    assert!(
        matches!(result, Err(NetworkError::ChannelClosed)),
        "expected ChannelClosed, got {result:?}"
    );

    ha.abort();
}

/// Dropping the remote connection causes an I/O error.
#[tokio::test]
async fn connection_reset() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = config(ka.clone(), Duration::from_secs(5));
    let conf_b = config(kb.clone(), Duration::from_secs(5));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (tx, _rx) = mpsc::unbounded_channel();
    let outbox = Queue::new();
    let mut peer_a = Peer::builder()
        .config(conf_a.clone())
        .budget(NonZeroUsize::new(10).unwrap())
        .messages(outbox)
        .retry(DelayQueue::new(conf_a))
        .inbound(tx)
        .metrics(Arc::new(NoMetrics))
        .build();

    drop(conn_b);

    let result = timeout(
        Duration::from_secs(5),
        peer_a.start(conn_a, CancellationToken::new()),
    )
    .await
    .expect("test itself timed out");

    assert!(
        matches!(result, Err(NetworkError::Io(_))),
        "expected Io error, got {result:?}"
    );
}

/// A zero-byte payload is delivered correctly.
#[tokio::test]
async fn empty_payload() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = config(ka.clone(), Duration::from_secs(5));
    let conf_b = config(kb.clone(), Duration::from_secs(5));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (mut peer_a, _rx_a) = make_peer(conf_a, 10);
    let (mut peer_b, mut rx_b) = make_peer(conf_b, 10);

    let slot = Slot::new(1);
    let id = MsgId::new(0);
    peer_a.msgs.enqueue(slot, id, payload(slot, id, b""));

    let ha = tokio::spawn(async move { peer_a.start(conn_a, CancellationToken::new()).await });
    let hb = tokio::spawn(async move { peer_b.start(conn_b, CancellationToken::new()).await });

    let (_, data, _) = timeout(Duration::from_secs(5), rx_b.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    assert!(data.is_empty());

    ha.abort();
    hb.abort();
}

/// Both peers send large multi-frame messages simultaneously.
#[tokio::test]
async fn interleaved_acks() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = config(ka.clone(), Duration::from_secs(5));
    let conf_b = config(kb.clone(), Duration::from_secs(5));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (mut peer_a, mut rx_a) = make_peer(conf_a, 10);
    let (mut peer_b, mut rx_b) = make_peer(conf_b, 10);

    let outbox_a = peer_a.msgs.clone();
    let outbox_b = peer_b.msgs.clone();

    let big_a: Vec<u8> = (0..100 * 1024).map(|i| (i % 251) as u8).collect();
    let big_b: Vec<u8> = (0..100 * 1024).map(|i| (i % 241) as u8).collect();

    for i in 0..3u64 {
        let slot = Slot::new(1);
        let id = MsgId::new(i);
        outbox_a.enqueue(slot, id, payload(slot, id, &big_a));
        outbox_b.enqueue(slot, id, payload(slot, id, &big_b));
    }

    let ha = tokio::spawn(async move { peer_a.start(conn_a, CancellationToken::new()).await });
    let hb = tokio::spawn(async move { peer_b.start(conn_b, CancellationToken::new()).await });

    for _ in 0..3 {
        let (_, data, _) = timeout(Duration::from_secs(10), rx_b.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        assert_eq!(data.as_ref(), big_a.as_slice());
    }

    for _ in 0..3 {
        let (_, data, _) = timeout(Duration::from_secs(10), rx_a.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        assert_eq!(data.as_ref(), big_b.as_slice());
    }

    ha.abort();
    hb.abort();
}

/// Enqueuing two messages with the same (Slot, MsgId) overwrites the first.
#[tokio::test]
async fn duplicate_message_id() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = config(ka.clone(), Duration::from_secs(5));
    let conf_b = config(kb.clone(), Duration::from_secs(5));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (mut peer_a, _rx_a) = make_peer(conf_a, 10);
    let (mut peer_b, mut rx_b) = make_peer(conf_b, 10);

    let outbox_a = peer_a.msgs.clone();

    let slot = Slot::new(1);
    let id = MsgId::new(0);

    outbox_a.enqueue(slot, id, payload(slot, id, b"first"));
    outbox_a.enqueue(slot, id, payload(slot, id, b"second"));

    let ha = tokio::spawn(async move { peer_a.start(conn_a, CancellationToken::new()).await });
    let hb = tokio::spawn(async move { peer_b.start(conn_b, CancellationToken::new()).await });

    let (_, data, _) = timeout(Duration::from_secs(5), rx_b.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    assert_eq!(data.as_ref(), b"second");

    assert!(
        timeout(Duration::from_millis(500), rx_b.recv())
            .await
            .is_err(),
        "unexpected extra message"
    );

    ha.abort();
    hb.abort();
}

/// A message whose slot equals the threshold is delivered (not off-by-one).
#[tokio::test]
async fn threshold_exact_boundary() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = config(ka.clone(), Duration::from_secs(5));
    let conf_b = config(kb.clone(), Duration::from_secs(5));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (mut peer_a, _rx_a) = make_peer(conf_a, 10);
    let (mut peer_b, mut rx_b) = make_peer(conf_b, 10);

    let outbox_a = peer_a.msgs.clone();
    let retry_a = peer_a.retry.clone();

    let s4 = Slot::new(4);
    outbox_a.enqueue(s4, MsgId::new(0), payload(s4, MsgId::new(0), b"below"));

    let s5 = Slot::new(5);
    outbox_a.enqueue(s5, MsgId::new(0), payload(s5, MsgId::new(0), b"equal"));

    let s6 = Slot::new(6);
    outbox_a.enqueue(s6, MsgId::new(0), payload(s6, MsgId::new(0), b"above"));

    // GC the outbox below slot 5, as the Server would.
    outbox_a.gc(Slot::new(5));
    retry_a.gc(Slot::new(5));

    let ha = tokio::spawn(async move { peer_a.start(conn_a, CancellationToken::new()).await });
    let hb = tokio::spawn(async move { peer_b.start(conn_b, CancellationToken::new()).await });

    let mut received = Vec::new();
    for _ in 0..2 {
        let (_, data, _) = timeout(Duration::from_secs(5), rx_b.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        received.push(data);
    }
    received.sort();
    assert_eq!(received, vec![Bytes::from("above"), Bytes::from("equal")]);

    assert!(
        timeout(Duration::from_millis(500), rx_b.recv())
            .await
            .is_err(),
        "unexpected extra message"
    );

    ha.abort();
    hb.abort();
}

/// Stress test with periodic GC.
#[tokio::test]
async fn many_small_messages() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = config(ka.clone(), Duration::from_secs(30));
    let conf_b = config(kb.clone(), Duration::from_secs(30));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (mut peer_a, _rx_a) = make_peer(conf_a, 500);
    let (mut peer_b, mut rx_b) = make_peer(conf_b, 500);

    let outbox_a = peer_a.msgs.clone();

    let msgs_per_slot = 50u64;
    let num_slots = 100u64;
    let count = msgs_per_slot * num_slots;

    for i in 0..count {
        let slot = Slot::new(i / msgs_per_slot);
        let id = MsgId::new(i % msgs_per_slot);
        let msg = i.to_be_bytes();
        outbox_a.enqueue(slot, id, payload(slot, id, &msg));
    }

    let ha = tokio::spawn(async move { peer_a.start(conn_a, CancellationToken::new()).await });
    let hb = tokio::spawn(async move { peer_b.start(conn_b, CancellationToken::new()).await });

    let mut values = Vec::with_capacity(count as usize);
    for n in 0..count {
        let (_, data, _) = timeout(Duration::from_secs(30), rx_b.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        let bytes: [u8; 8] = data.as_ref().try_into().expect("8 bytes");
        values.push(u64::from_be_bytes(bytes));

        if (n + 1) % 200 == 0 {
            let gc_slot = (n + 1) / msgs_per_slot;
            outbox_a.gc(Slot::new(gc_slot));
        }
    }

    values.sort();
    assert_eq!(values, (0..count).collect::<Vec<_>>());

    ha.abort();
    hb.abort();
}

/// Helper: build a config with custom retry delays.
fn config_with_retry(kp: Keypair, recv_timeout: Duration, retry_delays: Vec<u8>) -> Arc<Config> {
    Arc::new(
        Config::builder()
            .name("test")
            .keypair(kp)
            .bind(NetAddr::from((std::net::Ipv4Addr::LOCALHOST, 0u16)))
            .parties(std::iter::empty::<(PublicKey, NetAddr)>())
            .receive_timeout(recv_timeout)
            .connect_retry_delays(retry_delays.clone())
            .send_retry_delays(retry_delays)
            .noise_protocols([(1.into(), Protocol::IK_25519_AesGcm_Blake2s)])
            .build(),
    )
}

/// A sends a message but B starts late. A's retry delivers.
#[tokio::test]
async fn retry_delivers_on_late_start() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = config_with_retry(ka.clone(), Duration::from_secs(10), vec![1]);
    let conf_b = config(kb.clone(), Duration::from_secs(10));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (mut peer_a, _rx_a) = make_peer(conf_a, 10);

    let slot = Slot::new(1);
    let id = MsgId::new(0);
    peer_a.msgs.enqueue(slot, id, payload(slot, id, b"retried"));

    let ha = tokio::spawn(async move { peer_a.start(conn_a, CancellationToken::new()).await });

    tokio::time::sleep(Duration::from_millis(1500)).await;

    let (mut peer_b, mut rx_b) = make_peer(conf_b, 10);
    let hb = tokio::spawn(async move { peer_b.start(conn_b, CancellationToken::new()).await });

    let (_, data, _) = timeout(Duration::from_secs(5), rx_b.recv())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(data.as_ref(), b"retried");

    ha.abort();
    hb.abort();
}

/// Reconnect preserves retry state.
#[tokio::test]
async fn retry_after_reconnect() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = config_with_retry(ka.clone(), Duration::from_secs(10), vec![1]);
    let conf_b = config(kb.clone(), Duration::from_secs(10));

    let (conn_a1, conn_b1) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (tx_a, _rx_a) = mpsc::unbounded_channel();
    let outbox_a = Queue::new();
    let mut peer_a = Peer::builder()
        .config(conf_a.clone())
        .budget(NonZeroUsize::new(10).unwrap())
        .messages(outbox_a.clone())
        .retry(DelayQueue::new(conf_a.clone()))
        .inbound(tx_a)
        .metrics(Arc::new(NoMetrics))
        .build();

    let slot = Slot::new(1);
    let id = MsgId::new(0);
    outbox_a.enqueue(slot, id, payload(slot, id, b"survive"));

    drop(conn_b1);

    let result = peer_a.start(conn_a1, CancellationToken::new()).await;
    assert!(
        matches!(result, Err(NetworkError::Io(_))),
        "expected Io error, got {result:?}"
    );

    let (conn_a2, conn_b2) = connection_pair(conf_a, pkb, conf_b.clone()).await;

    let (mut peer_b, mut rx_b) = make_peer(conf_b, 10);

    let ha = tokio::spawn(async move { peer_a.start(conn_a2, CancellationToken::new()).await });
    let hb = tokio::spawn(async move { peer_b.start(conn_b2, CancellationToken::new()).await });

    let (_, data, _) = timeout(Duration::from_secs(10), rx_b.recv())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(data.as_ref(), b"survive");

    ha.abort();
    hb.abort();
}

/// Reconnect preserves retry state for *all* pending messages, not just one.
///
/// On reconnect `Peer::start` resets the delay queue, rescheduling every
/// unacknowledged message to the same instant. All of them must be resent.
#[tokio::test]
async fn retry_after_reconnect_multiple() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = config_with_retry(ka.clone(), Duration::from_secs(10), vec![1]);
    let conf_b = config(kb.clone(), Duration::from_secs(10));

    let (conn_a1, conn_b1) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (tx_a, _rx_a) = mpsc::unbounded_channel();
    let outbox_a = Queue::new();
    let mut peer_a = Peer::builder()
        .config(conf_a.clone())
        .budget(NonZeroUsize::new(10).unwrap())
        .messages(outbox_a.clone())
        .retry(DelayQueue::new(conf_a.clone()))
        .inbound(tx_a)
        .metrics(Arc::new(NoMetrics))
        .build();

    let slot = Slot::new(1);
    let n = 5u64;
    for i in 0..n {
        let id = MsgId::new(i);
        outbox_a.enqueue(slot, id, payload(slot, id, format!("m{i}").as_bytes()));
    }

    // First connection dies before B can acknowledge anything.
    drop(conn_b1);
    let result = peer_a.start(conn_a1, CancellationToken::new()).await;
    assert!(
        matches!(result, Err(NetworkError::Io(_))),
        "expected Io error, got {result:?}"
    );

    // Reconnect: every message must be rescheduled and delivered.
    let (conn_a2, conn_b2) = connection_pair(conf_a, pkb, conf_b.clone()).await;
    let (mut peer_b, mut rx_b) = make_peer(conf_b, 10);

    let ha = tokio::spawn(async move { peer_a.start(conn_a2, CancellationToken::new()).await });
    let hb = tokio::spawn(async move { peer_b.start(conn_b2, CancellationToken::new()).await });

    let mut got = Vec::new();
    for _ in 0..n {
        let (_, data, _) = timeout(Duration::from_secs(10), rx_b.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        got.push(data);
    }
    got.sort();
    let expected: Vec<Bytes> = (0..n).map(|i| Bytes::from(format!("m{i}"))).collect();
    assert_eq!(got, expected);

    ha.abort();
    hb.abort();
}

/// Sending pauses when unacknowledged messages reach peer_budget.
#[tokio::test]
async fn backpressure_on_unacked() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = Arc::new(
        Config::builder()
            .name("test")
            .keypair(ka.clone())
            .bind(NetAddr::from((std::net::Ipv4Addr::LOCALHOST, 0u16)))
            .parties(std::iter::empty::<(PublicKey, NetAddr)>())
            .peer_budget(NonZeroUsize::new(3).unwrap())
            .receive_timeout(Duration::from_secs(5))
            .connect_retry_delays(vec![30])
            .send_retry_delays(vec![30])
            .noise_protocols([(1.into(), Protocol::IK_25519_AesGcm_Blake2s)])
            .build(),
    );
    let conf_b = config(kb.clone(), Duration::from_secs(5));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (mut peer_a, _rx_a) = make_peer(conf_a, 10);
    let (mut peer_b, mut rx_b) = make_peer(conf_b, 1);

    let slot = Slot::new(1);
    for i in 0..5u64 {
        let id = MsgId::new(i);
        let msg = format!("msg-{i}");
        peer_a
            .msgs
            .enqueue(slot, id, payload(slot, id, msg.as_bytes()));
    }

    let ha = tokio::spawn(async move { peer_a.start(conn_a, CancellationToken::new()).await });
    let hb = tokio::spawn(async move { peer_b.start(conn_b, CancellationToken::new()).await });

    let mut received = Vec::new();
    for _ in 0..3 {
        let (_, data, permit) = timeout(Duration::from_secs(5), rx_b.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        received.push(data);
        drop(permit);
    }
    assert_eq!(received.len(), 3);

    for _ in 0..2 {
        let (_, data, permit) = timeout(Duration::from_secs(5), rx_b.recv())
            .await
            .expect("timed out")
            .expect("channel closed");
        received.push(data);
        drop(permit);
    }

    received.sort();
    let expected: Vec<Bytes> = (0..5).map(|i| Bytes::from(format!("msg-{i}"))).collect();
    assert_eq!(received, expected);

    assert!(
        timeout(Duration::from_millis(500), rx_b.recv())
            .await
            .is_err(),
        "unexpected extra message"
    );

    ha.abort();
    hb.abort();
}

// -- DelayQueue --------------------------------------------------------------
//
// These exercise the retry schedule directly. Because the schedule is indexed
// by due time, messages sharing a due instant (common: many are enqueued
// between one-second clock ticks) and stale entries left behind by GC are the
// interesting cases, and are hard to hit reliably over a live connection.

fn delay_queue(retry_delays: Vec<u8>) -> DelayQueue {
    let conf = config_with_retry(
        Keypair::generate().unwrap(),
        Duration::from_secs(5),
        retry_delays,
    );
    DelayQueue::new(conf)
}

/// Drain every message that is due at `now`, sorted for comparison.
fn drain_due(q: &DelayQueue, now: Instant) -> Vec<Bytes> {
    let mut out = Vec::new();
    while let Some((bytes, _)) = q.due(now) {
        out.push(bytes);
    }
    out.sort();
    out
}

fn labels(n: u64) -> Vec<Bytes> {
    (0..n).map(|i| Bytes::from(format!("m{i}"))).collect()
}

/// Messages sharing a due instant are each retried, not collapsed into one.
#[tokio::test(start_paused = true)]
async fn delay_queue_retries_all_sharing_a_due_instant() {
    let q = delay_queue(vec![1]);

    let now = Instant::now();
    let slot = Slot::new(1);
    let n = 5;
    for i in 0..n {
        // Same `now` for every message, so all share one due instant.
        let msg = Bytes::from(format!("m{i}"));
        q.add(slot, MsgId::new(i), msg, RetryPolicy::Default, now);
    }
    assert_eq!(q.len(), n as usize);

    tokio::time::advance(Duration::from_secs(1)).await;
    assert!(q.is_due(Instant::now()));

    assert_eq!(drain_due(&q, Instant::now()), labels(n));
}

/// `reset` reschedules every message (all to the same instant), not just one.
#[tokio::test(start_paused = true)]
async fn delay_queue_reset_reschedules_all() {
    let q = delay_queue(vec![1]);

    let slot = Slot::new(1);
    let n = 5;
    for i in 0..n {
        // Distinct due instants, so this isolates `reset` from the `add` path.
        tokio::time::advance(Duration::from_millis(10)).await;
        let msg = Bytes::from(format!("m{i}"));
        q.add(
            slot,
            MsgId::new(i),
            msg,
            RetryPolicy::Default,
            Instant::now(),
        );
    }

    q.reset(Instant::now());

    tokio::time::advance(Duration::from_secs(1)).await;
    assert_eq!(drain_due(&q, Instant::now()), labels(n));
}

/// A stale schedule entry left by GC must not drop a later, not-yet-due message.
#[tokio::test(start_paused = true)]
async fn delay_queue_stale_entry_keeps_pending() {
    let q = delay_queue(vec![1]);

    // `gone` is scheduled, then GC'd from the map — its schedule entry lingers.
    q.add(
        Slot::new(1),
        MsgId::new(0),
        Bytes::from("gone"),
        RetryPolicy::Default,
        Instant::now(),
    );
    q.gc(Slot::new(2));
    assert_eq!(q.len(), 0);

    // `kept` is due strictly after the stale entry.
    tokio::time::advance(Duration::from_millis(500)).await;
    q.add(
        Slot::new(5),
        MsgId::new(0),
        Bytes::from("kept"),
        RetryPolicy::Default,
        Instant::now(),
    );
    assert_eq!(q.len(), 1);

    // At this point the stale entry is due but `kept` is not: the stale entry is
    // skipped and `kept` must stay scheduled rather than being popped and lost.
    tokio::time::advance(Duration::from_millis(500)).await;
    assert!(q.due(Instant::now()).is_none());

    // Once `kept` comes due it is still delivered.
    tokio::time::advance(Duration::from_millis(500)).await;
    assert_eq!(
        q.due(Instant::now()).map(|(bytes, _)| bytes),
        Some(Bytes::from("kept"))
    );
}
