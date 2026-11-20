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
use hotshot_types::{data::VidCommon, traits::block_contents::EncodeBytes};
use tokio::{sync::watch, time::timeout};

use super::*;

/// A decide log in memory, served over the real ingester API.
struct FakeLog {
    entries: Mutex<BTreeMap<u64, Decided>>,
    payloads: Mutex<BTreeMap<u64, Bytes>>,
    changes: watch::Sender<u64>,
    acked: Mutex<Vec<Range<u64>>>,
}

impl FakeLog {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            entries: Mutex::default(),
            payloads: Mutex::default(),
            changes: watch::channel(0).0,
            acked: Mutex::default(),
        })
    }

    /// Decides the blocks of `chain` at `heights`, with their payloads; the newest carries the
    /// `Certificate2`.
    fn decide(&self, chain: &MockChain, heights: std::ops::RangeInclusive<usize>) {
        let blocks = &chain.blocks()[heights];
        let newest = blocks.last().unwrap().height();
        for block in blocks {
            let mut entry = entry(chain, block, block.height() == newest);
            entry.payload_size = Some(payload_of(block).len() as u64);
            self.payloads
                .lock()
                .unwrap()
                .insert(block.height(), payload_of(block));
            self.entries.lock().unwrap().insert(block.height(), entry);
        }
        self.changed();
    }

    /// Decides the block of `chain` at `height` without its payload.
    fn decide_without_payload(&self, chain: &MockChain, height: usize) {
        let block = &chain.blocks()[height];
        let entry = entry(chain, block, true);
        self.entries.lock().unwrap().insert(block.height(), entry);
        self.changed();
    }

    /// Adds the payload of the block of `chain` at `height`, decided without it.
    fn add_payload(&self, chain: &MockChain, height: usize) {
        let block = &chain.blocks()[height];
        self.payloads
            .lock()
            .unwrap()
            .insert(block.height(), payload_of(block));
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

fn payload_of(block: &MockBlock) -> Bytes {
    block.block.payload().encode().to_vec().into()
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
        payload_size: None,
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

    async fn payload(&self, height: u64) -> Result<Option<Bytes>, ApiError> {
        Ok(self.payloads.lock().unwrap().get(&height).cloned())
    }

    async fn service_records(
        &self,
        _epoch: Option<u64>,
    ) -> Result<Option<(u64, Vec<Bytes>)>, ApiError> {
        Ok(None)
    }
}

/// A sink in memory, holding the blocks the ingester appends after `before`.
struct MemSink {
    before: Option<Leaf2<MockTypes>>,
    blocks: Mutex<BTreeMap<u64, Block<MockTypes>>>,
    appended: Mutex<Vec<u64>>,
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
            appended: Mutex::default(),
        })
    }

    /// The heights appended, in the order they were.
    fn appended(&self) -> Vec<u64> {
        self.appended.lock().unwrap().clone()
    }

    fn block(&self, height: u64) -> Option<Block<MockTypes>> {
        self.blocks.lock().unwrap().get(&height).cloned()
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

    async fn add_payload(&self, height: u64, payload: Vec<u8>) -> anyhow::Result<()> {
        let mut blocks = self.blocks.lock().unwrap();
        let block = blocks.get_mut(&height).context("no block at the height")?;
        block.payload = Some(payload);
        Ok(())
    }

    async fn newest_leaf(&self) -> anyhow::Result<Option<Leaf2<MockTypes>>> {
        let blocks = self.blocks.lock().unwrap();
        Ok(blocks
            .last_key_value()
            .map(|(_, block)| block.leaf.clone())
            .or_else(|| self.before.clone()))
    }
}

async fn serve(log: Arc<FakeLog>) -> SocketAddr {
    let port = test_utils::reserve_tcp_port().unwrap();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    tokio::spawn(
        tonic::transport::Server::builder()
            .add_service(consensus_api::server(log))
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

fn spawn_ingester(addr: SocketAddr, sink: Arc<MemSink>) -> tokio::task::JoinHandle<anyhow::Error> {
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

/// Blocks reach the sink with their payload, VID common and certificates, and each one is
/// acknowledged once stored.
#[tokio::test]
async fn ingests_decided_blocks() {
    let chain = chain(4).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=0);
    log.decide(&chain, 1..=2);
    let sink = MemSink::new();
    let _ingester = spawn_ingester(serve(log.clone()).await, sink.clone());

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
        assert_eq!(
            stored.payload.as_deref(),
            Some(&*expected.block.payload().encode())
        );
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
/// ingester computes it.
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
    let _ingester = spawn_ingester(serve(log).await, sink.clone());

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
    let _ingester = spawn_ingester(serve(log).await, sink.clone());

    eventually("the block is stored", || sink.block(1).is_some()).await;
    assert_eq!(sink.block(1).unwrap().state_cert, Some(cert));
}

/// A restarted ingester takes again every entry consensus still holds, which it has not
/// acknowledged, and checks them against the newest leaf the sink holds.
#[tokio::test]
async fn takes_the_unacknowledged_entries_again() {
    let chain = chain(3).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=3);
    let sink = MemSink::after(Some(chain.blocks()[1].leaf.leaf().clone()));
    let _ingester = spawn_ingester(serve(log.clone()).await, sink.clone());

    eventually("everything is acknowledged", || {
        log.acked() == vec![0, 1, 2, 3]
    })
    .await;
    assert_eq!(sink.appended(), vec![0, 1, 2, 3]);
}

/// A restarted ingester whose sink holds a leaf of another chain stops at the entry that
/// contradicts it.
#[tokio::test]
async fn resuming_on_another_chain_stops_the_ingester() {
    let chain = chain(3).await;
    let other = other_chain().await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=3);
    let sink = MemSink::after(Some(other.blocks()[1].leaf.leaf().clone()));
    let ingester = spawn_ingester(serve(log).await, sink.clone());

    let err = timeout(Duration::from_secs(10), ingester)
        .await
        .expect("the ingester stops")
        .unwrap();
    assert!(err.is::<ChainBroken>(), "{err:#}");
    assert_eq!(
        sink.appended(),
        vec![0],
        "genesis is the same on both chains"
    );
}

/// A leaf that does not follow the one before it stops the ingester.
#[tokio::test]
async fn a_broken_chain_stops_the_ingester() {
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
    let ingester = spawn_ingester(serve(log).await, sink.clone());

    let err = timeout(Duration::from_secs(10), ingester)
        .await
        .expect("the ingester stops")
        .unwrap();
    assert!(err.is::<ChainBroken>(), "{err:#}");
    assert_eq!(sink.appended(), vec![0, 1]);
}

/// An entry the ingester cannot decode would come again after every reconnect, so it stops the
/// ingester instead.
#[tokio::test]
async fn an_undecodable_entry_stops_the_ingester() {
    let chain = chain(1).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=0);
    let mut bad = entry(&chain, &chain.blocks()[1], true);
    bad.body.as_mut().unwrap().leaf = vec![1, 2, 3].into();
    log.insert(bad);
    let sink = MemSink::new();
    let ingester = spawn_ingester(serve(log).await, sink.clone());

    let err = timeout(Duration::from_secs(10), ingester)
        .await
        .expect("the ingester stops")
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
    let ingester = spawn_ingester(serve(log.clone()).await, sink.clone());

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
    assert!(!ingester.is_finished());
}

/// A payload added after its entry reaches the sink, and the height is acknowledged only then.
#[tokio::test]
async fn takes_a_payload_added_later() {
    let chain = chain(2).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=0);
    log.decide_without_payload(&chain, 1);
    log.decide(&chain, 2..=2);
    let sink = MemSink::new();
    let _ingester = spawn_ingester(serve(log.clone()).await, sink.clone());

    eventually("the complete heights are acknowledged", || {
        log.acked() == vec![0, 2]
    })
    .await;
    assert_eq!(sink.block(1).unwrap().payload, None);

    log.add_payload(&chain, 1);
    eventually("the late payload is acknowledged", || {
        log.acked() == vec![0, 1, 2]
    })
    .await;
    assert_eq!(
        sink.block(1).unwrap().payload.as_deref(),
        Some(&*chain.blocks()[1].block.payload().encode())
    );
}

/// A height without its payload is acknowledged once it is final, and the payload is left to
/// the API service.
#[tokio::test]
async fn acknowledges_a_final_height_without_its_payload() {
    let blocks = FINALITY_VIEWS as usize + 2;
    let chain = chain(blocks).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=0);
    log.decide_without_payload(&chain, 1);
    let sink = MemSink::new();
    let _ingester = spawn_ingester(serve(log.clone()).await, sink.clone());

    eventually("the first height is acknowledged", || {
        log.acked() == vec![0]
    })
    .await;
    log.decide(&chain, 2..=blocks);
    eventually("the final height is acknowledged", || {
        log.acked().contains(&1)
    })
    .await;
    assert_eq!(sink.block(1).unwrap().payload, None);
}

/// Heights consensus no longer holds are not waited for.
#[tokio::test]
async fn starts_at_the_earliest_entry() {
    let chain = chain(4).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=4);
    log.ack(vec![Range { start: 0, end: 3 }]).await.unwrap();
    let sink = MemSink::after(Some(chain.blocks()[0].leaf.leaf().clone()));
    let _ingester = spawn_ingester(serve(log.clone()).await, sink.clone());

    eventually("the held heights are stored", || sink.appended().len() == 2).await;
    assert_eq!(sink.appended(), vec![3, 4]);
}
