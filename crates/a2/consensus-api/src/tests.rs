use std::{
    collections::BTreeMap,
    net::{Ipv4Addr, SocketAddr},
    sync::Mutex,
    time::Duration,
};

use futures::StreamExt;
use tokio::time::timeout;
use tonic::{Code, transport::Server};

use super::*;

/// A source over entries and payloads held in memory.
struct MemSource {
    entries: Mutex<BTreeMap<u64, Decided>>,
    payloads: Mutex<BTreeMap<u64, Bytes>>,
    next: watch::Sender<u64>,
    acked: Mutex<Vec<u64>>,
    records: Mutex<BTreeMap<u64, Vec<Bytes>>>,
}

impl MemSource {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            entries: Mutex::default(),
            payloads: Mutex::default(),
            next: watch::channel(0).0,
            acked: Mutex::default(),
            records: Mutex::default(),
        })
    }

    fn decide(&self, height: u64, payload: Option<Bytes>) {
        let mut decided = entry(height);
        decided.payload_size = payload.as_ref().map(|p| p.len() as u64);
        self.entries.lock().unwrap().insert(height, decided);
        if let Some(payload) = payload {
            self.payloads.lock().unwrap().insert(height, payload);
        }
        self.next
            .send_modify(|next| *next = (*next).max(height + 1));
    }
}

#[async_trait]
impl Source for MemSource {
    async fn decided(&self, from: u64, limit: usize) -> Result<Vec<Decided>, Error> {
        Ok(self
            .entries
            .lock()
            .unwrap()
            .range(from..)
            .take(limit)
            .map(|(_, e)| e.clone())
            .collect())
    }

    async fn earliest(&self) -> Result<Option<u64>, Error> {
        Ok(self.entries.lock().unwrap().keys().next().copied())
    }

    fn next_height(&self) -> watch::Receiver<u64> {
        self.next.subscribe()
    }

    async fn ack(&self, height: u64) -> Result<(), Error> {
        self.acked.lock().unwrap().push(height);
        let mut entries = self.entries.lock().unwrap();
        *entries = entries.split_off(&height);
        Ok(())
    }

    async fn payload(&self, height: u64) -> Result<Option<Bytes>, Error> {
        Ok(self.payloads.lock().unwrap().get(&height).cloned())
    }

    async fn service_records(
        &self,
        epoch: Option<u64>,
    ) -> Result<Option<(u64, Vec<Bytes>)>, Error> {
        let records = self.records.lock().unwrap();
        let found = match epoch {
            Some(epoch) => records.get_key_value(&epoch),
            None => records.last_key_value(),
        };
        Ok(found.map(|(epoch, records)| (*epoch, records.clone())))
    }
}

fn entry(height: u64) -> Decided {
    Decided {
        height,
        view: height + 10,
        ..Default::default()
    }
}

async fn serve(source: Arc<MemSource>) -> DecideLogClient<tonic::transport::Channel> {
    let port = test_utils::reserve_tcp_port().unwrap();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    tokio::spawn(Server::builder().add_service(server(source)).serve(addr));
    for _ in 0..100 {
        if let Ok(client) = DecideLogClient::connect(format!("http://{addr}")).await {
            return client;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("server did not start");
}

async fn next_event(stream: &mut tonic::Streaming<SubscribeResponse>) -> Option<Event> {
    timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("an event")
        .map(|r| r.unwrap().event.unwrap())
}

#[tokio::test]
async fn subscribe_delivers_stored_then_new_entries() {
    let source = MemSource::new();
    source.decide(0, None);
    source.decide(1, None);
    let mut client = serve(source.clone()).await;

    let mut stream = client
        .subscribe(SubscribeRequest { from_height: 1 })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        next_event(&mut stream).await,
        Some(Event::Decided(entry(1)))
    );

    source.decide(2, None);
    assert_eq!(
        next_event(&mut stream).await,
        Some(Event::Decided(entry(2)))
    );
}

#[tokio::test]
async fn subscribe_below_the_earliest_entry_starts_with_it() {
    let source = MemSource::new();
    source.decide(5, None);
    let mut client = serve(source).await;

    let mut stream = client
        .subscribe(SubscribeRequest { from_height: 2 })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        next_event(&mut stream).await,
        Some(Event::Start(Start { earliest_height: 5 }))
    );
    assert_eq!(
        next_event(&mut stream).await,
        Some(Event::Decided(entry(5)))
    );
}

#[tokio::test]
async fn ack_reaches_the_source() {
    let source = MemSource::new();
    source.decide(0, None);
    source.decide(1, None);
    let mut client = serve(source.clone()).await;

    client.ack(AckRequest { height: 1 }).await.unwrap();
    assert_eq!(*source.acked.lock().unwrap(), vec![1]);
    assert_eq!(source.earliest().await.unwrap(), Some(1));
}

/// An entry with a payload is followed by the payload in chunks; an empty payload has none.
#[tokio::test]
async fn payloads_follow_their_entries_in_chunks() {
    let source = MemSource::new();
    let payload: Bytes = (0..(2 * CHUNK_SIZE + 7)).map(|i| i as u8).collect();
    source.decide(3, Some(payload.clone()));
    source.decide(4, Some(Bytes::new()));
    source.decide(5, None);
    let mut client = serve(source).await;
    let mut stream = client
        .subscribe(SubscribeRequest { from_height: 3 })
        .await
        .unwrap()
        .into_inner();

    let Some(Event::Decided(decided)) = next_event(&mut stream).await else {
        panic!("expected the entry at 3");
    };
    assert_eq!(decided.payload_size, Some(payload.len() as u64));
    let mut data = Vec::new();
    for _ in 0..3 {
        let Some(Event::PayloadChunk(chunk)) = next_event(&mut stream).await else {
            panic!("expected a payload chunk");
        };
        assert!(chunk.data.len() <= CHUNK_SIZE);
        data.extend(chunk.data);
    }
    assert_eq!(data, payload);

    let Some(Event::Decided(decided)) = next_event(&mut stream).await else {
        panic!("expected the entry at 4");
    };
    assert_eq!((decided.height, decided.payload_size), (4, Some(0)));
    let Some(Event::Decided(decided)) = next_event(&mut stream).await else {
        panic!("expected the entry at 5");
    };
    assert_eq!((decided.height, decided.payload_size), (5, None));
}

/// A payload freed after its entry was read leaves the entry without one, so the subscriber does
/// not wait for chunks that never come.
#[tokio::test]
async fn a_freed_payload_is_not_announced() {
    let source = MemSource::new();
    source.decide(3, Some(Bytes::from_static(b"payload")));
    source.payloads.lock().unwrap().clear();
    source.decide(4, None);
    let mut client = serve(source).await;
    let mut stream = client
        .subscribe(SubscribeRequest { from_height: 3 })
        .await
        .unwrap()
        .into_inner();

    let Some(Event::Decided(decided)) = next_event(&mut stream).await else {
        panic!("expected the entry at 3");
    };
    assert_eq!((decided.height, decided.payload_size), (3, None));
    assert_eq!(
        next_event(&mut stream).await,
        Some(Event::Decided(entry(4)))
    );
}

/// Records come back for the epoch asked for, or for the latest one, and an epoch without any is
/// not found.
#[tokio::test]
async fn service_records_by_epoch() {
    let source = MemSource::new();
    source.records.lock().unwrap().extend([
        (
            3,
            vec![Bytes::from_static(b"a3"), Bytes::from_static(b"b3")],
        ),
        (4, vec![Bytes::from_static(b"a4")]),
    ]);
    let mut client = serve(source.clone()).await;

    let response = client
        .get_service_records(ServiceRecordsRequest { epoch: Some(3) })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(response.epoch, 3);
    assert_eq!(response.records, vec![b"a3".to_vec(), b"b3".to_vec()]);

    let latest = client
        .get_service_records(ServiceRecordsRequest { epoch: None })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(latest.epoch, 4);

    let missing = client
        .get_service_records(ServiceRecordsRequest { epoch: Some(5) })
        .await;
    assert_eq!(missing.err().map(|s| s.code()), Some(Code::NotFound));
}

/// A node that takes no transactions now.
struct FullNode;

#[async_trait]
impl Node for FullNode {
    async fn submit_transaction(&self, _transaction: Bytes) -> Result<(), SubmitError> {
        Err(SubmitError::Overloaded("not taking transactions".into()))
    }
}

/// A transaction the node cannot take now comes back as resource exhausted, so the caller can
/// tell it from an invalid one.
#[tokio::test]
async fn submit_reports_an_overloaded_node() {
    let port = test_utils::reserve_tcp_port().unwrap();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    tokio::spawn(
        Server::builder()
            .add_service(node_server(Arc::new(FullNode)))
            .serve(addr),
    );
    let mut client = None;
    for _ in 0..100 {
        if let Ok(connected) = ConsensusNodeClient::connect(format!("http://{addr}")).await {
            client = Some(connected);
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let mut client = client.expect("server did not start");
    let refused = client
        .submit_transaction(crate::proto::SubmitTransactionRequest {
            transaction: vec![1, 2, 3],
        })
        .await;
    assert_eq!(
        refused.err().map(|s| s.code()),
        Some(Code::ResourceExhausted)
    );
}
