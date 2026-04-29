use std::{num::NonZeroUsize, sync::Arc, time::Duration};

use bytes::{Bytes, BytesMut};
use tokio::{
    net::TcpListener,
    sync::{OwnedSemaphorePermit, mpsc, watch},
    time::timeout,
};

use crate::{
    Config, Keypair, PublicKey,
    addr::NetAddr,
    connection::Connection,
    error::NetworkError,
    msg::{MAX_PAYLOAD_SIZE, MsgId, Slot, Trailer, TrailerType},
    net::{RetryPolicy, peer::Peer},
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
            .retry_delays(vec![2, 5])
            .max_retry_delay(Duration::from_secs(10))
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
            Connection::accept(conf_b, stream).await.unwrap()
        },
        Connection::connect(conf_a, pkb, addr),
    );
    (ca, cb)
}

/// Build a payload with the trailer appended.
fn payload(slot: Slot, id: MsgId, data: &[u8]) -> (RetryPolicy, Bytes) {
    let trailer = Trailer::new(TrailerType::Std, slot, id).to_bytes();
    let mut buf = BytesMut::with_capacity(data.len() + Trailer::SIZE);
    buf.extend_from_slice(data);
    buf.extend_from_slice(&trailer);
    (RetryPolicy::Default, buf.freeze())
}

/// Create a `Peer` and its inbound message receiver + outbox + slot sender.
fn make_peer(
    conf: Arc<Config>,
    conn: Connection,
    budget: usize,
) -> (Peer, Rx, Queue<(RetryPolicy, Bytes)>, watch::Sender<Slot>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let (slot_tx, slot_rx) = watch::channel(Slot::MIN);
    let outbox = Queue::new();
    let peer = Peer::builder()
        .config(conf)
        .budget(NonZeroUsize::new(budget).unwrap())
        .messages(outbox.clone())
        .inbound(tx)
        .next_slot(slot_rx)
        .connection(conn)
        .build();
    (peer, rx, outbox, slot_tx)
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

    let (mut peer_a, _rx_a, outbox_a, _slot_a) = make_peer(conf_a, conn_a, 10);
    let (mut peer_b, mut rx_b, _outbox_b, _slot_b) = make_peer(conf_b, conn_b, 10);

    let slot = Slot::new(1);
    let id = MsgId::new(1);
    outbox_a.enqueue(slot, id, payload(slot, id, b"hello"));

    let ha = tokio::spawn(async move { peer_a.start().await });
    let hb = tokio::spawn(async move { peer_b.start().await });

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

    let (mut peer_a, _rx_a, outbox_a, _slot_a) = make_peer(conf_a, conn_a, 10);
    let (mut peer_b, mut rx_b, _outbox_b, _slot_b) = make_peer(conf_b, conn_b, 10);

    let n = 10u64;
    let slot = Slot::new(1);
    for i in 0..n {
        let id = MsgId::new(i);
        let msg = format!("msg-{i}");
        outbox_a.enqueue(slot, id, payload(slot, id, msg.as_bytes()));
    }

    let ha = tokio::spawn(async move { peer_a.start().await });
    let hb = tokio::spawn(async move { peer_b.start().await });

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

    let (mut peer_a, mut rx_a, outbox_a, _slot_a) = make_peer(conf_a, conn_a, 10);
    let (mut peer_b, mut rx_b, outbox_b, _slot_b) = make_peer(conf_b, conn_b, 10);

    let slot = Slot::new(1);
    for i in 0..5u64 {
        let id = MsgId::new(i);
        outbox_a.enqueue(slot, id, payload(slot, id, b"from-a"));
        outbox_b.enqueue(slot, id, payload(slot, id, b"from-b"));
    }

    let ha = tokio::spawn(async move { peer_a.start().await });
    let hb = tokio::spawn(async move { peer_b.start().await });

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

    let (mut peer_a, _rx_a, outbox_a, _slot_a) = make_peer(conf_a, conn_a, 10);
    let (mut peer_b, mut rx_b, _outbox_b, _slot_b) = make_peer(conf_b, conn_b, 10);

    let big: Vec<u8> = (0..200 * 1024).map(|i| (i % 251) as u8).collect();
    let slot = Slot::new(1);
    let id = MsgId::new(0);
    outbox_a.enqueue(slot, id, payload(slot, id, &big));

    let ha = tokio::spawn(async move { peer_a.start().await });
    let hb = tokio::spawn(async move { peer_b.start().await });

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

    let (mut peer_a, _rx_a, outbox_a, _slot_a) = make_peer(conf_a, conn_a, 10);
    let (mut peer_b, mut rx_b, _outbox_b, _slot_b) = make_peer(conf_b, conn_b, 1);

    let slot = Slot::new(1);
    for i in 0..3u64 {
        let id = MsgId::new(i);
        let msg = format!("m{i}");
        outbox_a.enqueue(slot, id, payload(slot, id, msg.as_bytes()));
    }

    let ha = tokio::spawn(async move { peer_a.start().await });
    let hb = tokio::spawn(async move { peer_b.start().await });

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

    let (mut peer_a, _rx_a, outbox_a, slot_a) = make_peer(conf_a, conn_a, 10);
    let (mut peer_b, mut rx_b, _outbox_b, slot_b) = make_peer(conf_b, conn_b, 10);

    // Slot 3 (below threshold) — should be discarded by B.
    let old_slot = Slot::new(3);
    let old_id = MsgId::new(0);
    outbox_a.enqueue(old_slot, old_id, payload(old_slot, old_id, b"old"));

    // Slot 7 (above threshold) — should be delivered.
    let new_slot = Slot::new(7);
    let new_id = MsgId::new(0);
    outbox_a.enqueue(new_slot, new_id, payload(new_slot, new_id, b"new"));

    // Set threshold to 5 on both sides. GC the outbox as the Server would.
    slot_a.send(Slot::new(5)).unwrap();
    slot_b.send(Slot::new(5)).unwrap();
    outbox_a.gc(Slot::new(5));

    let ha = tokio::spawn(async move { peer_a.start().await });
    let hb = tokio::spawn(async move { peer_b.start().await });

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
    let (_slot_tx, slot_rx) = watch::channel(Slot::new(0));
    let outbox = Queue::new();
    let mut peer_a = Peer::builder()
        .config(conf_a)
        .budget(NonZeroUsize::new(10).unwrap())
        .messages(outbox.clone())
        .inbound(tx)
        .next_slot(slot_rx)
        .connection(conn_a)
        .build();

    let slot = Slot::new(1);
    let id = MsgId::new(0);
    outbox.enqueue(slot, id, payload(slot, id, b"ping"));

    let result = timeout(Duration::from_secs(5), peer_a.start())
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

    let (mut peer_a, _rx_a, outbox_a, slot_a) = make_peer(conf_a, conn_a, 10);
    let (mut peer_b, mut rx_b, _outbox_b, _slot_b) = make_peer(conf_b, conn_b, 10);

    let s1 = Slot::new(1);
    let s3 = Slot::new(3);
    outbox_a.enqueue(s1, MsgId::new(0), payload(s1, MsgId::new(0), b"old"));
    outbox_a.enqueue(s3, MsgId::new(0), payload(s3, MsgId::new(0), b"new"));

    let ha = tokio::spawn(async move { peer_a.start().await });
    let hb = tokio::spawn(async move { peer_b.start().await });

    let (_, data1, _) = timeout(Duration::from_secs(5), rx_b.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    // Advance threshold past slot 1.
    slot_a.send(Slot::new(2)).unwrap();

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

    let (mut peer_a, _rx_a, outbox_a, _slot_a) = make_peer(conf_a, conn_a, 10);

    let (tx_b, rx_b) = mpsc::unbounded_channel();
    let (_slot_tx, slot_rx) = watch::channel(Slot::MIN);
    let mut peer_b = Peer::builder()
        .config(conf_b)
        .budget(NonZeroUsize::new(10).unwrap())
        .messages(Queue::new())
        .inbound(tx_b)
        .next_slot(slot_rx)
        .connection(conn_b)
        .build();
    drop(rx_b);

    let slot = Slot::new(1);
    let id = MsgId::new(0);
    outbox_a.enqueue(slot, id, payload(slot, id, b"data"));

    let ha = tokio::spawn(async move { peer_a.start().await });
    let result = timeout(Duration::from_secs(5), peer_b.start())
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
    let (_slot_tx, slot_rx) = watch::channel(Slot::new(0));
    let mut peer_a = Peer::builder()
        .config(conf_a)
        .budget(NonZeroUsize::new(10).unwrap())
        .messages(Queue::new())
        .inbound(tx)
        .next_slot(slot_rx)
        .connection(conn_a)
        .build();

    drop(conn_b);

    let result = timeout(Duration::from_secs(5), peer_a.start())
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

    let (mut peer_a, _rx_a, outbox_a, _slot_a) = make_peer(conf_a, conn_a, 10);
    let (mut peer_b, mut rx_b, _outbox_b, _slot_b) = make_peer(conf_b, conn_b, 10);

    let slot = Slot::new(1);
    let id = MsgId::new(0);
    outbox_a.enqueue(slot, id, payload(slot, id, b""));

    let ha = tokio::spawn(async move { peer_a.start().await });
    let hb = tokio::spawn(async move { peer_b.start().await });

    let (_, data, _) = timeout(Duration::from_secs(5), rx_b.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    assert!(data.is_empty());

    ha.abort();
    hb.abort();
}

/// The largest payload that fits in a single Noise frame.
#[tokio::test]
async fn max_single_frame() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = config(ka.clone(), Duration::from_secs(5));
    let conf_b = config(kb.clone(), Duration::from_secs(5));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (mut peer_a, _rx_a, outbox_a, _slot_a) = make_peer(conf_a, conn_a, 10);
    let (mut peer_b, mut rx_b, _outbox_b, _slot_b) = make_peer(conf_b, conn_b, 10);

    let user_data_len = MAX_PAYLOAD_SIZE - Trailer::SIZE;
    let data: Vec<u8> = (0..user_data_len).map(|i| (i % 199) as u8).collect();
    let slot = Slot::new(1);
    let id = MsgId::new(0);
    outbox_a.enqueue(slot, id, payload(slot, id, &data));

    let ha = tokio::spawn(async move { peer_a.start().await });
    let hb = tokio::spawn(async move { peer_b.start().await });

    let (_, received, _) = timeout(Duration::from_secs(5), rx_b.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    assert_eq!(received.len(), user_data_len);
    assert_eq!(received.as_ref(), data.as_slice());

    ha.abort();
    hb.abort();
}

/// One byte over the single-frame limit forces a second frame.
#[tokio::test]
async fn just_over_single_frame() {
    let ka = Keypair::generate().unwrap();
    let kb = Keypair::generate().unwrap();
    let pkb = kb.public_key();

    let conf_a = config(ka.clone(), Duration::from_secs(5));
    let conf_b = config(kb.clone(), Duration::from_secs(5));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (mut peer_a, _rx_a, outbox_a, _slot_a) = make_peer(conf_a, conn_a, 10);
    let (mut peer_b, mut rx_b, _outbox_b, _slot_b) = make_peer(conf_b, conn_b, 10);

    let user_data_len = MAX_PAYLOAD_SIZE - Trailer::SIZE + 1;
    let data: Vec<u8> = (0..user_data_len).map(|i| (i % 199) as u8).collect();
    let slot = Slot::new(1);
    let id = MsgId::new(0);
    outbox_a.enqueue(slot, id, payload(slot, id, &data));

    let ha = tokio::spawn(async move { peer_a.start().await });
    let hb = tokio::spawn(async move { peer_b.start().await });

    let (_, received, _) = timeout(Duration::from_secs(5), rx_b.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    assert_eq!(received.len(), user_data_len);
    assert_eq!(received.as_ref(), data.as_slice());

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

    let (mut peer_a, mut rx_a, outbox_a, _slot_a) = make_peer(conf_a, conn_a, 10);
    let (mut peer_b, mut rx_b, outbox_b, _slot_b) = make_peer(conf_b, conn_b, 10);

    let big_a: Vec<u8> = (0..100 * 1024).map(|i| (i % 251) as u8).collect();
    let big_b: Vec<u8> = (0..100 * 1024).map(|i| (i % 241) as u8).collect();

    for i in 0..3u64 {
        let slot = Slot::new(1);
        let id = MsgId::new(i);
        outbox_a.enqueue(slot, id, payload(slot, id, &big_a));
        outbox_b.enqueue(slot, id, payload(slot, id, &big_b));
    }

    let ha = tokio::spawn(async move { peer_a.start().await });
    let hb = tokio::spawn(async move { peer_b.start().await });

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

    let (mut peer_a, _rx_a, outbox_a, _slot_a) = make_peer(conf_a, conn_a, 10);
    let (mut peer_b, mut rx_b, _outbox_b, _slot_b) = make_peer(conf_b, conn_b, 10);

    let slot = Slot::new(1);
    let id = MsgId::new(0);

    outbox_a.enqueue(slot, id, payload(slot, id, b"first"));
    outbox_a.enqueue(slot, id, payload(slot, id, b"second"));

    let ha = tokio::spawn(async move { peer_a.start().await });
    let hb = tokio::spawn(async move { peer_b.start().await });

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

    let (mut peer_a, _rx_a, outbox_a, slot_a) = make_peer(conf_a, conn_a, 10);
    let (mut peer_b, mut rx_b, _outbox_b, slot_b) = make_peer(conf_b, conn_b, 10);

    let s4 = Slot::new(4);
    outbox_a.enqueue(s4, MsgId::new(0), payload(s4, MsgId::new(0), b"below"));

    let s5 = Slot::new(5);
    outbox_a.enqueue(s5, MsgId::new(0), payload(s5, MsgId::new(0), b"equal"));

    let s6 = Slot::new(6);
    outbox_a.enqueue(s6, MsgId::new(0), payload(s6, MsgId::new(0), b"above"));

    // Set threshold to 5 on both sides. GC the outbox as the Server would.
    slot_a.send(Slot::new(5)).unwrap();
    slot_b.send(Slot::new(5)).unwrap();
    outbox_a.gc(Slot::new(5));

    let ha = tokio::spawn(async move { peer_a.start().await });
    let hb = tokio::spawn(async move { peer_b.start().await });

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

    let (mut peer_a, _rx_a, outbox_a, slot_a) = make_peer(conf_a, conn_a, 500);
    let (mut peer_b, mut rx_b, _outbox_b, _slot_b) = make_peer(conf_b, conn_b, 500);

    let msgs_per_slot = 50u64;
    let num_slots = 100u64;
    let count = msgs_per_slot * num_slots;

    for i in 0..count {
        let slot = Slot::new(i / msgs_per_slot);
        let id = MsgId::new(i % msgs_per_slot);
        let msg = i.to_be_bytes();
        outbox_a.enqueue(slot, id, payload(slot, id, &msg));
    }

    let ha = tokio::spawn(async move { peer_a.start().await });
    let hb = tokio::spawn(async move { peer_b.start().await });

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
            let _ = slot_a.send(Slot::new(gc_slot));
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
            .retry_delays(retry_delays)
            .max_retry_delay(Duration::from_secs(10))
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

    let (mut peer_a, _rx_a, outbox_a, _slot_a) = make_peer(conf_a, conn_a, 10);

    let slot = Slot::new(1);
    let id = MsgId::new(0);
    outbox_a.enqueue(slot, id, payload(slot, id, b"retried"));

    let ha = tokio::spawn(async move { peer_a.start().await });

    tokio::time::sleep(Duration::from_millis(1500)).await;

    let (mut peer_b, mut rx_b, _outbox_b, _slot_b) = make_peer(conf_b, conn_b, 10);
    let hb = tokio::spawn(async move { peer_b.start().await });

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
    let (_slot_tx, slot_rx) = watch::channel(Slot::new(0));
    let outbox_a = Queue::new();
    let mut peer_a = Peer::builder()
        .config(conf_a.clone())
        .budget(NonZeroUsize::new(10).unwrap())
        .messages(outbox_a.clone())
        .inbound(tx_a)
        .next_slot(slot_rx)
        .connection(conn_a1)
        .build();

    let slot = Slot::new(1);
    let id = MsgId::new(0);
    outbox_a.enqueue(slot, id, payload(slot, id, b"survive"));

    drop(conn_b1);

    let result = peer_a.start().await;
    assert!(
        matches!(result, Err(NetworkError::Io(_))),
        "expected Io error, got {result:?}"
    );

    let (conn_a2, conn_b2) = connection_pair(conf_a, pkb, conf_b.clone()).await;
    peer_a.set_connection(conn_a2);

    let (mut peer_b, mut rx_b, _outbox_b, _slot_b) = make_peer(conf_b, conn_b2, 10);

    let ha = tokio::spawn(async move { peer_a.start().await });
    let hb = tokio::spawn(async move { peer_b.start().await });

    let (_, data, _) = timeout(Duration::from_secs(10), rx_b.recv())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(data.as_ref(), b"survive");

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
            .retry_delays(vec![30])
            .max_retry_delay(Duration::from_secs(30))
            .build(),
    );
    let conf_b = config(kb.clone(), Duration::from_secs(5));

    let (conn_a, conn_b) = connection_pair(conf_a.clone(), pkb, conf_b.clone()).await;

    let (mut peer_a, _rx_a, outbox_a, _slot_a) = make_peer(conf_a, conn_a, 10);
    let (mut peer_b, mut rx_b, _outbox_b, _slot_b) = make_peer(conf_b, conn_b, 1);

    let slot = Slot::new(1);
    for i in 0..5u64 {
        let id = MsgId::new(i);
        let msg = format!("msg-{i}");
        outbox_a.enqueue(slot, id, payload(slot, id, msg.as_bytes()));
    }

    let ha = tokio::spawn(async move { peer_a.start().await });
    let hb = tokio::spawn(async move { peer_b.start().await });

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
