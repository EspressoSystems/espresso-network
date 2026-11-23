use std::{
    collections::BTreeMap,
    net::{Ipv4Addr, SocketAddr},
    ops::Range,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use bytes::Bytes;
use consensus_api::{Error as ApiError, Source, proto::Body};
use hotshot_query_service::testing::{
    chain::{MockBlock, MockChain},
    mocks::MockTypes,
};
use hotshot_types::data::VidCommon;
use tokio::{sync::watch, time::timeout};

use super::*;

/// A decide log in memory, served over the real consensus API.
struct FakeLog {
    entries: Mutex<BTreeMap<u64, Decided>>,
    changes: watch::Sender<u64>,
    acked: Mutex<Vec<Range<u64>>>,
    stake_tables: Mutex<BTreeMap<u64, Bytes>>,
    records: Mutex<BTreeMap<u64, Vec<Bytes>>>,
}

impl FakeLog {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            entries: Mutex::default(),
            changes: watch::channel(0).0,
            acked: Mutex::default(),
            stake_tables: Mutex::default(),
            records: Mutex::default(),
        })
    }

    /// Decides the blocks of `chain` at `heights`; the newest carries the `Certificate2`.
    fn decide(&self, chain: &MockChain, heights: std::ops::RangeInclusive<usize>) {
        let blocks = &chain.blocks()[heights];
        let newest = blocks.last().unwrap().height();
        for block in blocks {
            let entry = entry(chain, block, block.height() == newest);
            self.entries.lock().unwrap().insert(block.height(), entry);
        }
        self.changed();
    }

    fn insert(&self, entry: Decided) {
        self.entries.lock().unwrap().insert(entry.height, entry);
        self.changed();
    }

    fn changed(&self) {
        self.changes.send_modify(|n| *n += 1);
    }

    fn acked(&self) -> Vec<u64> {
        let acked = self.acked.lock().unwrap();
        let mut heights: Vec<u64> = acked.iter().flat_map(Clone::clone).collect();
        heights.sort();
        heights
    }
}

fn entry(chain: &MockChain, block: &MockBlock, newest: bool) -> Decided {
    let leaf = block.leaf.leaf();
    let vid_common = match block.vid_common.common() {
        VidCommon::V2(common) => bincode::serialize(common).unwrap().into(),
        _ => Default::default(),
    };
    Decided {
        height: block.height(),
        view: *leaf.view_number(),
        epoch: 0,
        leaf_commitment: leaf.commit().to_string(),
        parent_commitment: leaf.parent_commitment().to_string(),
        payload_commitment: leaf.payload_commitment().to_string(),
        body: Some(Body {
            format: BODY_FORMAT,
            leaf: bincode::serialize(leaf).unwrap().into(),
            cert1: bincode::serialize(block.leaf.qc()).unwrap().into(),
            cert2: newest
                .then(|| chain.cert2(block))
                .flatten()
                .map(|cert| bincode::serialize(&cert).unwrap().into())
                .unwrap_or_default(),
            vid_common,
            state_cert: Default::default(),
            state: None,
        }),
    }
}

#[async_trait]
impl Source for FakeLog {
    async fn decided(&self, from: u64, limit: usize) -> Result<Vec<Decided>, ApiError> {
        let entries = self.entries.lock().unwrap();
        Ok(entries
            .range(from..)
            .take(limit)
            .map(|(_, e)| e.clone())
            .collect())
    }

    async fn earliest(&self) -> Result<Option<u64>, ApiError> {
        Ok(self.entries.lock().unwrap().keys().next().copied())
    }

    fn changes(&self) -> watch::Receiver<u64> {
        self.changes.subscribe()
    }

    async fn ack(&self, ranges: Vec<Range<u64>>) -> Result<(), ApiError> {
        let mut entries = self.entries.lock().unwrap();
        entries.retain(|height, _| !ranges.iter().any(|range| range.contains(height)));
        self.acked.lock().unwrap().extend(ranges);
        Ok(())
    }

    async fn stake_table(&self, epoch: u64) -> Result<Option<Bytes>, ApiError> {
        Ok(self.stake_tables.lock().unwrap().get(&epoch).cloned())
    }

    async fn service_records(&self, epoch: u64) -> Result<Vec<Bytes>, ApiError> {
        Ok(self
            .records
            .lock()
            .unwrap()
            .get(&epoch)
            .cloned()
            .unwrap_or_default())
    }
}

/// A sink in memory, holding the blocks the instance appends after `before`, and the payloads of
/// all of them but those it is told it lacks.
struct MemSink {
    before: Option<Leaf2<MockTypes>>,
    blocks: Mutex<BTreeMap<u64, Block<MockTypes>>>,
    lacking_payloads: Mutex<std::collections::BTreeSet<u64>>,
    appended: Mutex<Vec<u64>>,
    stake_tables: Mutex<BTreeMap<u64, Bytes>>,
    records: Mutex<BTreeMap<u64, Vec<Bytes>>>,
}

impl MemSink {
    fn new() -> Arc<Self> {
        Self::after(None)
    }

    /// A sink that already holds `leaf` as its newest.
    fn after(before: Option<Leaf2<MockTypes>>) -> Arc<Self> {
        Arc::new(Self {
            before,
            blocks: Mutex::default(),
            lacking_payloads: Mutex::default(),
            appended: Mutex::default(),
            stake_tables: Mutex::default(),
            records: Mutex::default(),
        })
    }

    /// The heights appended, in the order they were.
    fn appended(&self) -> Vec<u64> {
        self.appended.lock().unwrap().clone()
    }

    fn block(&self, height: u64) -> Option<Block<MockTypes>> {
        self.blocks.lock().unwrap().get(&height).cloned()
    }

    /// Makes the sink lack the payload of `height`, which no storage instance stored yet.
    fn lack_payload(&self, height: u64) {
        self.lacking_payloads.lock().unwrap().insert(height);
    }
}

#[async_trait]
impl Sink<MockTypes> for MemSink {
    async fn append(&self, block: Block<MockTypes>) -> anyhow::Result<()> {
        let height = block.leaf.height();
        self.blocks.lock().unwrap().insert(height, block);
        self.appended.lock().unwrap().push(height);
        Ok(())
    }

    async fn newest_leaf(&self) -> anyhow::Result<Option<Leaf2<MockTypes>>> {
        let blocks = self.blocks.lock().unwrap();
        Ok(blocks
            .last_key_value()
            .map(|(_, block)| block.leaf.clone())
            .or_else(|| self.before.clone()))
    }

    async fn leaf(&self, height: u64) -> anyhow::Result<Option<Leaf2<MockTypes>>> {
        let stored = self.block(height).map(|block| block.leaf);
        let before = self.before.clone().filter(|leaf| leaf.height() == height);
        Ok(stored.or(before))
    }

    async fn has_payload(&self, height: u64) -> anyhow::Result<bool> {
        Ok(
            self.block(height).is_some()
                && !self.lacking_payloads.lock().unwrap().contains(&height),
        )
    }

    async fn put_stake_table(&self, epoch: u64, table: Bytes) -> anyhow::Result<()> {
        self.stake_tables.lock().unwrap().insert(epoch, table);
        Ok(())
    }

    async fn put_service_records(&self, epoch: u64, records: Vec<Bytes>) -> anyhow::Result<()> {
        self.records.lock().unwrap().insert(epoch, records);
        Ok(())
    }
}

async fn serve(log: Arc<FakeLog>) -> SocketAddr {
    let port = test_utils::reserve_tcp_port().unwrap();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let (service, dispatcher) =
        consensus_api::server(log, consensus_api::payloads::PayloadQueue::new().0);
    tokio::spawn(dispatcher);
    tokio::spawn(
        tonic::transport::Server::builder()
            .add_service(service)
            .serve(addr),
    );
    for _ in 0..100 {
        if DecideLogClient::connect(format!("http://{addr}"))
            .await
            .is_ok()
        {
            return addr;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("server did not start");
}

fn spawn_instance(addr: SocketAddr, sink: Arc<MemSink>) -> tokio::task::JoinHandle<anyhow::Error> {
    tokio::spawn(async move { run::<MockTypes>(addr, sink.as_ref()).await })
}

/// Waits until `done` holds.
async fn eventually(what: &str, done: impl Fn() -> bool) {
    timeout(Duration::from_secs(10), async {
        while !done() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("{what}"));
}

async fn chain(blocks: usize) -> MockChain {
    let mut chain = MockChain::new(4, 100).await;
    chain.push_empty(blocks).await;
    chain
}

/// Blocks reach the sink with their VID common and certificates, and each one is acknowledged
/// once stored, the store holding its payload.
#[tokio::test]
async fn ingests_decided_blocks() {
    let chain = chain(4).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=0);
    log.decide(&chain, 1..=2);
    let sink = MemSink::new();
    let _instance = spawn_instance(serve(log.clone()).await, sink.clone());

    eventually("the first blocks are stored", || sink.appended().len() == 3).await;
    log.decide(&chain, 3..=4);
    eventually("every block is acknowledged", || {
        log.acked() == vec![0, 1, 2, 3, 4]
    })
    .await;

    assert_eq!(sink.appended(), vec![0, 1, 2, 3, 4]);
    for expected in chain.blocks() {
        let stored = sink.block(expected.height()).unwrap();
        assert_eq!(stored.leaf.commit(), expected.leaf.leaf().commit());
        assert_eq!(&stored.cert1, expected.leaf.qc());
    }
    assert!(
        sink.block(1).unwrap().cert2.is_none(),
        "only the newest leaf of a decide has a cert2"
    );
    assert_eq!(
        sink.block(2).unwrap().cert2,
        chain.cert2(&chain.blocks()[2])
    );
}

/// Consensus does not disperse the genesis block, so its entry has no VID common data and the
/// instance computes it.
#[tokio::test]
async fn computes_the_genesis_vid_common() {
    let chain = chain(1).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=1);
    let genesis = &chain.blocks()[0];
    assert_eq!(*genesis.leaf.leaf().view_number(), 0);
    log.entries
        .lock()
        .unwrap()
        .get_mut(&0)
        .unwrap()
        .body
        .as_mut()
        .unwrap()
        .vid_common
        .clear();
    let sink = MemSink::new();
    let _instance = spawn_instance(serve(log).await, sink.clone());

    eventually("genesis is stored", || sink.block(0).is_some()).await;
    assert_eq!(
        sink.block(0).unwrap().vid_common.as_ref(),
        Some(genesis.vid_common.common())
    );
}

/// The state certificate an entry carries reaches the sink with its block.
#[tokio::test]
async fn passes_the_state_certificate() {
    let chain = chain(1).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=1);
    let cert = LightClientStateUpdateCertificateV2::<MockTypes> {
        epoch: hotshot_types::data::EpochNumber::new(3),
        light_client_state: Default::default(),
        next_stake_table_state: Default::default(),
        signatures: Vec::new(),
        auth_root: Default::default(),
    };
    log.entries
        .lock()
        .unwrap()
        .get_mut(&1)
        .unwrap()
        .body
        .as_mut()
        .unwrap()
        .state_cert = bincode::serialize(&cert).unwrap().into();
    let sink = MemSink::new();
    let _instance = spawn_instance(serve(log).await, sink.clone());

    eventually("the block is stored", || sink.block(1).is_some()).await;
    assert_eq!(sink.block(1).unwrap().state_cert, Some(cert));
}

/// The state consensus sends with an entry reaches the sink with its block, and the stake tables
/// and records the stream carries reach it too.
#[tokio::test]
async fn passes_the_state_stake_tables_and_records() {
    let chain = chain(1).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=1);
    let state = State {
        frontier: Bytes::from_static(b"frontier"),
        chain_config: Bytes::from_static(b"config"),
        reward_changes: Bytes::new(),
    };
    log.entries
        .lock()
        .unwrap()
        .get_mut(&1)
        .unwrap()
        .body
        .as_mut()
        .unwrap()
        .state = Some(state.clone());
    log.stake_tables
        .lock()
        .unwrap()
        .insert(0, Bytes::from_static(b"table"));
    log.records
        .lock()
        .unwrap()
        .insert(0, vec![Bytes::from_static(b"record")]);
    let sink = MemSink::new();
    let _instance = spawn_instance(serve(log).await, sink.clone());

    eventually("the records are stored", || {
        sink.records.lock().unwrap().contains_key(&0)
    })
    .await;
    assert_eq!(sink.block(1).unwrap().state, Some(state));
    assert_eq!(sink.block(0).unwrap().state, None);
    assert_eq!(
        sink.stake_tables.lock().unwrap().get(&0),
        Some(&Bytes::from_static(b"table"))
    );
}

/// A restarted instance takes again every entry consensus still holds, which it has not
/// acknowledged, and checks them against the newest leaf the sink holds.
#[tokio::test]
async fn takes_the_unacknowledged_entries_again() {
    let chain = chain(3).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=3);
    let sink = MemSink::after(Some(chain.blocks()[1].leaf.leaf().clone()));
    let _instance = spawn_instance(serve(log.clone()).await, sink.clone());

    eventually("everything is acknowledged", || {
        log.acked() == vec![0, 1, 2, 3]
    })
    .await;
    assert_eq!(sink.appended(), vec![0, 1, 2, 3]);
}

/// A restarted instance whose sink holds a leaf of another chain stops at the entry that
/// contradicts it.
#[tokio::test]
async fn resuming_on_another_chain_stops_ingesting() {
    let chain = chain(3).await;
    let other = other_chain().await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=3);
    let sink = MemSink::after(Some(other.blocks()[1].leaf.leaf().clone()));
    let instance = spawn_instance(serve(log).await, sink.clone());

    let err = timeout(Duration::from_secs(10), instance)
        .await
        .expect("ingesting stops")
        .unwrap();
    assert!(err.is::<ChainBroken>(), "{err:#}");
    assert_eq!(
        sink.appended(),
        vec![0],
        "genesis is the same on both chains"
    );
}

/// A leaf that does not follow the one before it stops ingesting.
#[tokio::test]
async fn a_broken_chain_stops_ingesting() {
    let chain = chain(2).await;
    let other = other_chain().await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=1);
    let block = &other.blocks()[2];
    assert_ne!(
        block.leaf.leaf().parent_commitment(),
        chain.blocks()[1].leaf.leaf().commit()
    );
    log.insert(entry(&other, block, true));
    let sink = MemSink::new();
    let instance = spawn_instance(serve(log).await, sink.clone());

    let err = timeout(Duration::from_secs(10), instance)
        .await
        .expect("ingesting stops")
        .unwrap();
    assert!(err.is::<ChainBroken>(), "{err:#}");
    assert_eq!(sink.appended(), vec![0, 1]);
}

/// An entry the instance cannot decode would come again after every reconnect, so it stops the
/// instance instead.
#[tokio::test]
async fn an_undecodable_entry_stops_ingesting() {
    let chain = chain(1).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=0);
    let mut bad = entry(&chain, &chain.blocks()[1], true);
    bad.body.as_mut().unwrap().leaf = vec![1, 2, 3].into();
    log.insert(bad);
    let sink = MemSink::new();
    let instance = spawn_instance(serve(log).await, sink.clone());

    let err = timeout(Duration::from_secs(10), instance)
        .await
        .expect("ingesting stops")
        .unwrap();
    assert!(err.is::<BadEntry>(), "{err:#}");
    assert_eq!(sink.appended(), vec![0]);
}

/// A chain that differs from those of [`chain`] at height 1, so its leaf at height 2 names a
/// different parent.
async fn other_chain() -> MockChain {
    let mut other = MockChain::new(4, 100).await;
    other
        .push(vec![
            hotshot_query_service::testing::mocks::MockTransaction::new(vec![7]),
        ])
        .await;
    other.push_empty(1).await;
    other
}

/// Heights decided after later ones are stored when they arrive, and a range acknowledged
/// never spans a height not stored.
#[tokio::test]
async fn stores_heights_decided_late() {
    let chain = chain(5).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=1);
    log.decide(&chain, 4..=5);
    let sink = MemSink::new();
    let instance = spawn_instance(serve(log.clone()).await, sink.clone());

    eventually("the decided heights are acknowledged", || {
        log.acked() == vec![0, 1, 4, 5]
    })
    .await;
    assert!(
        log.acked
            .lock()
            .unwrap()
            .iter()
            .all(|range| !range.contains(&2)),
        "no range spans the missing heights"
    );

    log.decide(&chain, 2..=3);
    eventually("the late heights are acknowledged", || {
        log.acked() == vec![0, 1, 2, 3, 4, 5]
    })
    .await;
    assert_eq!(sink.appended(), vec![0, 1, 4, 5, 2, 3]);
    assert!(!instance.is_finished());
}

/// A height without its payload is acknowledged once it is final, and the payload is left to
/// the API service.
#[tokio::test]
async fn acknowledges_a_final_height_without_its_payload() {
    let blocks = FINALITY_VIEWS as usize + 2;
    let chain = chain(blocks).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=1);
    let sink = MemSink::new();
    sink.lack_payload(1);
    let _instance = spawn_instance(serve(log.clone()).await, sink.clone());

    eventually("the first height is acknowledged", || {
        log.acked() == vec![0]
    })
    .await;
    log.decide(&chain, 2..=blocks);
    eventually("the final height is acknowledged", || {
        log.acked().contains(&1)
    })
    .await;
}

/// Heights consensus no longer holds are not waited for.
#[tokio::test]
async fn starts_at_the_earliest_entry() {
    let chain = chain(4).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=4);
    log.ack(vec![Range { start: 0, end: 3 }]).await.unwrap();
    let sink = MemSink::after(Some(chain.blocks()[0].leaf.leaf().clone()));
    let _instance = spawn_instance(serve(log.clone()).await, sink.clone());

    eventually("the held heights are stored", || sink.appended().len() == 2).await;
    assert_eq!(sink.appended(), vec![3, 4]);
}

/// Two instances over one store share the decide log: every block is stored and acknowledged.
#[tokio::test]
async fn instances_share_the_decide_log() {
    let chain = chain(6).await;
    let log = FakeLog::new();
    let addr = serve(log.clone()).await;
    let sink = MemSink::new();
    let _first = spawn_instance(addr, sink.clone());
    let _second = spawn_instance(addr, sink.clone());

    log.decide(&chain, 0..=6);
    eventually("every block is acknowledged", || {
        let mut acked = log.acked();
        acked.dedup();
        acked == (0..=6).collect::<Vec<_>>()
    })
    .await;
    for block in chain.blocks() {
        assert!(sink.block(block.height()).is_some(), "the block is stored");
    }
}

/// An offered payload is stored by its commitment and acknowledged.
#[tokio::test]
async fn takes_and_acknowledges_offered_payloads() {
    use chain_store::{ChainStore, payloads::PayloadStore};
    use consensus_api::payloads::{Offer, PayloadQueue};

    let (payloads, mut acks) = PayloadQueue::new();
    let port = test_utils::reserve_tcp_port().unwrap();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let (service, dispatcher) = consensus_api::server(FakeLog::new(), payloads.clone());
    tokio::spawn(dispatcher);
    tokio::spawn(
        tonic::transport::Server::builder()
            .add_service(service)
            .serve(addr),
    );
    let objects = PayloadStore::new(Arc::new(object_store::memory::InMemory::new()), "p", None);
    let store =
        ChainStore::new(chain_store::mem::MemBackend::default()).with_payloads(objects.clone());
    let taker = store.clone();
    tokio::spawn(async move { super::payloads::run(addr, &taker).await });

    payloads.offer(Offer {
        view: 3,
        commitment: "C~3".into(),
        ns_table: Bytes::new(),
        payload: Bytes::from_static(b"offered"),
    });
    let acked = timeout(Duration::from_secs(10), acks.recv())
        .await
        .expect("an acknowledgement");
    assert_eq!(acked, Some((3, "C~3".to_string())));
    assert_eq!(
        objects.get("C~3").await.unwrap().as_deref(),
        Some(&b"offered"[..])
    );
}

/// An instance in the consensus node's process ingests the decide log by calling it directly.
#[tokio::test]
async fn ingests_a_decide_log_in_the_same_process() {
    let chain = chain(2).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=2);
    let (service, dispatcher) =
        consensus_api::service(log.clone(), consensus_api::payloads::PayloadQueue::new().0);
    tokio::spawn(dispatcher);
    let sink = MemSink::new();
    let instance = sink.clone();
    let service = Arc::new(service);
    tokio::spawn(async move { run_local::<MockTypes, _>(service, instance.as_ref()).await });

    eventually("every block is acknowledged", || {
        log.acked() == vec![0, 1, 2]
    })
    .await;
    assert_eq!(sink.appended(), vec![0, 1, 2]);
}

/// An instance in the consensus node's process takes payloads from its queue, and acknowledges
/// each once stored.
#[tokio::test]
async fn takes_payloads_in_the_same_process() {
    use chain_store::{ChainStore, payloads::PayloadStore};
    use consensus_api::payloads::{Offer, PayloadQueue};

    let (payloads, mut acks) = PayloadQueue::new();
    let objects = PayloadStore::new(Arc::new(object_store::memory::InMemory::new()), "p", None);
    let store =
        ChainStore::new(chain_store::mem::MemBackend::default()).with_payloads(objects.clone());
    let queue = payloads.clone();
    tokio::spawn(async move { super::payloads::run_local(queue, &store).await });

    payloads.offer(Offer {
        view: 3,
        commitment: "C~3".into(),
        ns_table: Bytes::new(),
        payload: Bytes::from_static(b"offered"),
    });
    let acked = timeout(Duration::from_secs(10), acks.recv())
        .await
        .expect("an acknowledgement");
    assert_eq!(acked, Some((3, "C~3".to_string())));
    assert_eq!(
        objects.get("C~3").await.unwrap().as_deref(),
        Some(&b"offered"[..])
    );
}
