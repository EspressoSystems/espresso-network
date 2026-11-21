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
    changes: watch::Sender<u64>,
    acked: Mutex<Vec<Range<u64>>>,
    stake_tables: Mutex<BTreeMap<u64, Bytes>>,
    records: Mutex<BTreeMap<u64, Vec<Bytes>>>,
}

impl MemSource {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            entries: Mutex::default(),
            payloads: Mutex::default(),
            changes: watch::channel(0).0,
            acked: Mutex::default(),
            stake_tables: Mutex::default(),
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
        self.changes.send_modify(|n| *n += 1);
    }

    /// Adds `payload` to the entry at `height`, decided without it.
    fn add_payload(&self, height: u64, payload: Bytes) {
        self.payloads.lock().unwrap().insert(height, payload);
        self.changes.send_modify(|n| *n += 1);
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

    fn changes(&self) -> watch::Receiver<u64> {
        self.changes.subscribe()
    }

    async fn ack(&self, ranges: Vec<Range<u64>>) -> Result<(), Error> {
        let mut entries = self.entries.lock().unwrap();
        entries.retain(|height, _| !ranges.iter().any(|range| range.contains(height)));
        self.acked.lock().unwrap().extend(ranges);
        Ok(())
    }

    async fn payload(&self, height: u64) -> Result<Option<Bytes>, Error> {
        Ok(self.payloads.lock().unwrap().get(&height).cloned())
    }

    async fn stake_table(&self, epoch: u64) -> Result<Option<Bytes>, Error> {
        Ok(self.stake_tables.lock().unwrap().get(&epoch).cloned())
    }

    async fn service_records(&self, epoch: u64) -> Result<Vec<Bytes>, Error> {
        Ok(self
            .records
            .lock()
            .unwrap()
            .get(&epoch)
            .cloned()
            .unwrap_or_default())
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
    let (service, dispatcher) = server(source, crate::payloads::PayloadQueue::new().0);
    tokio::spawn(dispatcher);
    tokio::spawn(Server::builder().add_service(service).serve(addr));
    for _ in 0..100 {
        if let Ok(client) = DecideLogClient::connect(format!("http://{addr}")).await {
            return client;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("server did not start");
}

/// A subscription, past its `Start`, which names `earliest`.
async fn subscribe(
    client: &mut DecideLogClient<tonic::transport::Channel>,
    earliest: Option<u64>,
) -> tonic::Streaming<SubscribeResponse> {
    let mut stream = client
        .subscribe(SubscribeRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        next_event(&mut stream).await,
        Some(Event::Start(Start {
            earliest_height: earliest
        }))
    );
    stream
}

/// Whether the stream sends nothing but progress for a moment.
async fn quiet(stream: &mut tonic::Streaming<SubscribeResponse>) -> bool {
    loop {
        match timeout(Duration::from_millis(200), any_event(stream)).await {
            Err(_) => return true,
            Ok(Some(Event::Progress(_))) => continue,
            Ok(_) => return false,
        }
    }
}

/// The next event other than progress.
async fn next_event(stream: &mut tonic::Streaming<SubscribeResponse>) -> Option<Event> {
    loop {
        let event = timeout(Duration::from_secs(5), any_event(stream))
            .await
            .expect("an event");
        if !matches!(event, Some(Event::Progress(_))) {
            return event;
        }
    }
}

async fn any_event(stream: &mut tonic::Streaming<SubscribeResponse>) -> Option<Event> {
    stream.next().await.map(|r| r.unwrap().event.unwrap())
}

#[tokio::test]
async fn subscribe_delivers_stored_then_new_entries() {
    let source = MemSource::new();
    source.decide(0, None);
    source.decide(1, None);
    let mut client = serve(source.clone()).await;

    let mut stream = subscribe(&mut client, Some(0)).await;
    for height in [0, 1] {
        assert_eq!(
            next_event(&mut stream).await,
            Some(Event::Decided(entry(height)))
        );
    }

    source.decide(2, None);
    assert_eq!(
        next_event(&mut stream).await,
        Some(Event::Decided(entry(2)))
    );
}

#[tokio::test]
async fn starts_with_the_earliest_entry() {
    let source = MemSource::new();
    source.decide(5, None);
    let mut client = serve(source).await;

    let mut stream = subscribe(&mut client, Some(5)).await;
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

    source.decide(2, None);
    source.decide(3, None);
    let ranges = [0..1, 2..3].map(|range| proto::HeightRange {
        start: range.start,
        end: range.end,
    });
    client
        .ack(AckRequest {
            ranges: ranges.to_vec(),
        })
        .await
        .unwrap();
    assert_eq!(*source.acked.lock().unwrap(), vec![0..1, 2..3]);
    let left: Vec<u64> = source.entries.lock().unwrap().keys().copied().collect();
    assert_eq!(left, vec![1, 3]);
}

/// A payload that joins an entry sent without one follows as `PayloadAdded` and its chunks.
#[tokio::test]
async fn a_late_payload_follows_its_entry() {
    let source = MemSource::new();
    source.decide(3, None);
    let mut client = serve(source.clone()).await;
    let mut stream = subscribe(&mut client, Some(3)).await;
    assert_eq!(
        next_event(&mut stream).await,
        Some(Event::Decided(entry(3)))
    );

    source.add_payload(3, Bytes::from_static(b"late"));
    assert_eq!(
        next_event(&mut stream).await,
        Some(Event::PayloadAdded(PayloadAdded {
            height: 3,
            payload_size: 4
        }))
    );
    assert_eq!(
        next_event(&mut stream).await,
        Some(Event::PayloadChunk(PayloadChunk {
            data: Bytes::from_static(b"late")
        }))
    );
}

/// A height decided after later ones arrives when it is decided.
#[tokio::test]
async fn a_late_entry_fills_its_height() {
    let source = MemSource::new();
    source.decide(3, Some(Bytes::new()));
    source.decide(5, Some(Bytes::new()));
    let mut client = serve(source.clone()).await;
    let mut stream = subscribe(&mut client, Some(3)).await;
    for height in [3, 5] {
        let Some(Event::Decided(decided)) = next_event(&mut stream).await else {
            panic!("expected the entry at {height}");
        };
        assert_eq!(decided.height, height);
    }

    source.decide(4, Some(Bytes::new()));
    let Some(Event::Decided(decided)) = next_event(&mut stream).await else {
        panic!("expected the late entry");
    };
    assert_eq!(decided.height, 4);
}

/// A height before the earliest entry kept can still be decided late, and is sent then.
#[tokio::test]
async fn a_late_entry_before_the_earliest_is_sent() {
    let source = MemSource::new();
    source.decide(11, None);
    let mut client = serve(source.clone()).await;
    let mut stream = subscribe(&mut client, Some(11)).await;
    assert_eq!(
        next_event(&mut stream).await,
        Some(Event::Decided(entry(11)))
    );

    source.decide(10, None);
    assert_eq!(
        next_event(&mut stream).await,
        Some(Event::Decided(entry(10)))
    );
}

/// Nothing joins an entry, or fills a height, once it is `FINALITY_VIEWS` behind the newest.
#[tokio::test]
async fn final_heights_get_nothing_late() {
    let source = MemSource::new();
    source.decide(3, None);
    source.decide(5, Some(Bytes::new()));
    let mut client = serve(source.clone()).await;
    let mut stream = subscribe(&mut client, Some(3)).await;
    for _ in 0..2 {
        next_event(&mut stream).await.expect("an entry");
    }
    // Views are heights plus ten, so this entry makes both heights before final.
    let newest = 5 + FINALITY_VIEWS;
    source.decide(newest, Some(Bytes::new()));
    let Some(Event::Decided(decided)) = next_event(&mut stream).await else {
        panic!("expected the newest entry");
    };
    assert_eq!(decided.height, newest);

    source.add_payload(3, Bytes::from_static(b"late"));
    source.decide(4, Some(Bytes::new()));
    assert!(quiet(&mut stream).await, "a final height gets nothing");
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
    let mut stream = subscribe(&mut client, Some(3)).await;

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
    let mut stream = subscribe(&mut client, Some(3)).await;

    let Some(Event::Decided(decided)) = next_event(&mut stream).await else {
        panic!("expected the entry at 3");
    };
    assert_eq!((decided.height, decided.payload_size), (3, None));
    assert_eq!(
        next_event(&mut stream).await,
        Some(Event::Decided(entry(4)))
    );
}

/// After the entries, the stream sends the stake tables of their epochs up to two after the
/// newest, as the node has them, and the records of the newest epoch, again once there are more.
#[tokio::test]
async fn stake_tables_and_records_follow_the_entries() {
    let source = MemSource::new();
    source.stake_tables.lock().unwrap().extend([
        (2, Bytes::from_static(b"t2")),
        (3, Bytes::from_static(b"t3")),
        (5, Bytes::from_static(b"t5")),
        (6, Bytes::from_static(b"t6")),
    ]);
    source
        .records
        .lock()
        .unwrap()
        .insert(3, vec![Bytes::from_static(b"a3")]);
    let mut decided = entry(0);
    decided.epoch = 3;
    source.entries.lock().unwrap().insert(0, decided.clone());
    let mut client = serve(source.clone()).await;

    let mut stream = subscribe(&mut client, Some(0)).await;
    assert_eq!(next_event(&mut stream).await, Some(Event::Decided(decided)));
    let stake_table = |epoch, table: &'static [u8]| {
        Some(Event::StakeTable(StakeTable {
            epoch,
            table: Bytes::from_static(table),
        }))
    };
    assert_eq!(next_event(&mut stream).await, stake_table(3, b"t3"));
    assert_eq!(next_event(&mut stream).await, stake_table(5, b"t5"));
    assert_eq!(
        next_event(&mut stream).await,
        Some(Event::ServiceRecords(ServiceRecords {
            epoch: 3,
            records: vec![Bytes::from_static(b"a3")],
        }))
    );

    // A stake table built later, and more records, follow with the next change.
    source
        .stake_tables
        .lock()
        .unwrap()
        .insert(4, Bytes::from_static(b"t4"));
    source
        .records
        .lock()
        .unwrap()
        .get_mut(&3)
        .unwrap()
        .push(Bytes::from_static(b"b3"));
    tokio::time::sleep(RECORDS_INTERVAL).await;
    let mut next = entry(1);
    next.epoch = 3;
    source.entries.lock().unwrap().insert(1, next);
    source.changes.send_modify(|n| *n += 1);
    let mut events = Vec::new();
    for _ in 0..3 {
        events.push(next_event(&mut stream).await);
    }
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Some(Event::Decided(d)) if d.height == 1))
    );
    assert!(events.contains(&stake_table(4, b"t4")));
    assert!(events.contains(&Some(Event::ServiceRecords(ServiceRecords {
        epoch: 3,
        records: vec![Bytes::from_static(b"a3"), Bytes::from_static(b"b3")],
    }))));
}

/// Each entry goes to one of the subscriptions, and what a subscription was sent goes to another
/// when it ends before the entries are freed.
#[tokio::test]
async fn subscriptions_share_the_entries() {
    let source = MemSource::new();
    let mut client = serve(source.clone()).await;
    let mut first = subscribe(&mut client, None).await;
    for height in 0..4 {
        source.decide(height, None);
    }
    for height in 0..4 {
        assert_eq!(
            next_event(&mut first).await,
            Some(Event::Decided(entry(height)))
        );
    }
    let mut second = subscribe(&mut client, Some(0)).await;
    assert!(quiet(&mut second).await, "the entries went to the first");

    // Entries 0 and 1 are freed; the others go to the second once the first ends.
    source.ack(std::iter::once(0..2).collect()).await.unwrap();
    drop(first);
    for height in 2..4 {
        assert_eq!(
            next_event(&mut second).await,
            Some(Event::Decided(entry(height)))
        );
    }

    // New entries go to whichever subscription asks; with one left, all go to it.
    source.decide(4, None);
    assert_eq!(
        next_event(&mut second).await,
        Some(Event::Decided(entry(4)))
    );
}

/// Every subscription learns the view of the newest decide.
#[tokio::test]
async fn progress_reaches_every_subscription() {
    let source = MemSource::new();
    let mut client = serve(source.clone()).await;
    let mut streams = [
        subscribe(&mut client, None).await,
        subscribe(&mut client, None).await,
    ];
    source.decide(7, None);
    for stream in &mut streams {
        loop {
            let event = timeout(Duration::from_secs(5), any_event(stream))
                .await
                .expect("an event");
            if let Some(Event::Progress(progress)) = event {
                assert_eq!(progress.newest_view, entry(7).view);
                break;
            }
        }
    }
}

/// Payloads go to one subscription each, a subscriber's acknowledgement reaches consensus, and a
/// payload not acknowledged before its subscription ends goes to another.
#[tokio::test]
async fn payloads_are_taken_and_acknowledged() {
    use proto::payload_offer::Event as OfferEvent;

    use crate::payloads::{Offer, PayloadQueue};

    let (payloads, mut acks) = PayloadQueue::new();
    let port = test_utils::reserve_tcp_port().unwrap();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let (service, dispatcher) = server(MemSource::new(), payloads.clone());
    tokio::spawn(dispatcher);
    tokio::spawn(Server::builder().add_service(service).serve(addr));
    let mut client = loop {
        if let Ok(client) = DecideLogClient::connect(format!("http://{addr}")).await {
            break client;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    let offer = |view: u64| Offer {
        view,
        commitment: format!("C~{view}"),
        ns_table: Bytes::from_static(b"ns"),
        payload: Bytes::from(vec![view as u8; 10]),
    };
    async fn next_offer(stream: &mut tonic::Streaming<PayloadOffer>) -> OfferEvent {
        timeout(Duration::from_secs(5), stream.next())
            .await
            .expect("an offer")
            .unwrap()
            .unwrap()
            .event
            .unwrap()
    }
    async fn take(stream: &mut tonic::Streaming<PayloadOffer>) -> (PayloadHead, Vec<u8>) {
        let OfferEvent::Head(head) = next_offer(stream).await else {
            panic!("expected a head");
        };
        let mut data = Vec::new();
        while (data.len() as u64) < head.size {
            let OfferEvent::Chunk(chunk) = next_offer(stream).await else {
                panic!("expected a chunk");
            };
            data.extend_from_slice(&chunk.data);
        }
        (head, data)
    }

    let mut first = client
        .take_payloads(TakePayloadsRequest {})
        .await
        .unwrap()
        .into_inner();
    payloads.offer(offer(1));
    let (head, data) = take(&mut first).await;
    assert_eq!((head.view, head.commitment.as_str()), (1, "C~1"));
    assert_eq!(data, vec![1; 10]);
    client
        .ack_payload(AckPayloadRequest {
            view: 1,
            commitment: "C~1".into(),
        })
        .await
        .unwrap();
    assert_eq!(acks.recv().await, Some((1, "C~1".to_string())));

    // The first subscription takes the next one and ends without acknowledging it.
    payloads.offer(offer(2));
    let (head, _) = take(&mut first).await;
    assert_eq!(head.view, 2);
    let mut second = client
        .take_payloads(TakePayloadsRequest {})
        .await
        .unwrap()
        .into_inner();
    drop(first);
    let (head, data) = take(&mut second).await;
    assert_eq!((head.view, data), (2, vec![2; 10]));
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
            transaction: Bytes::from_static(&[1, 2, 3]),
        })
        .await;
    assert_eq!(
        refused.err().map(|s| s.code()),
        Some(Code::ResourceExhausted)
    );
}
