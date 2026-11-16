use std::{
    collections::BTreeMap,
    net::{Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use bytes::Bytes;
use consensus_api::{
    Error as ApiError, PayloadKey, Source,
    proto::{Body, PayloadReady},
};
use hotshot_types::{data::VidCommon, traits::block_contents::EncodeBytes};
use tokio::{
    sync::{broadcast, watch},
    time::timeout,
};

use super::*;
use crate::{
    availability::BlockInfo,
    testing::{chain::MockChain, mocks::MockTypes},
    types::HeightIndexed,
};

/// A decide log in memory, served over the real ingester API.
struct FakeLog {
    entries: Mutex<BTreeMap<u64, Decided>>,
    payloads: Mutex<BTreeMap<u64, Bytes>>,
    next: watch::Sender<u64>,
    ready: broadcast::Sender<PayloadReady>,
    acked: Mutex<u64>,
}

impl FakeLog {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            entries: Mutex::default(),
            payloads: Mutex::default(),
            next: watch::channel(0).0,
            ready: broadcast::channel(4).0,
            acked: Mutex::new(0),
        })
    }

    /// Decides `blocks` of `chain`, oldest first, as one decide: only the newest carries the
    /// certificates.
    fn decide(&self, chain: &MockChain, heights: std::ops::RangeInclusive<usize>) {
        let blocks = &chain.blocks()[heights];
        let newest = blocks.last().unwrap().height();
        for block in blocks {
            let is_newest = block.height() == newest;
            let mut entry = entry(chain, block, is_newest);
            entry.payload_size = Some(block.block.payload().encode().len() as u64);
            self.payloads.lock().unwrap().insert(
                block.height(),
                block.block.payload().encode().to_vec().into(),
            );
            self.entries.lock().unwrap().insert(block.height(), entry);
        }
        self.next.send_modify(|n| *n = (*n).max(newest + 1));
    }

    fn insert(&self, entry: Decided) {
        let height = entry.height;
        self.entries.lock().unwrap().insert(height, entry);
        self.next.send_modify(|n| *n = (*n).max(height + 1));
    }
}

fn entry(chain: &MockChain, block: &crate::testing::chain::MockBlock, newest: bool) -> Decided {
    let leaf = block.leaf.leaf();
    let vid_common = match block.vid_common.common() {
        VidCommon::V2(common) => bincode::serialize(common).unwrap(),
        _ => Vec::new(),
    };
    let ser = |v: Option<Vec<u8>>| v.unwrap_or_default();
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
            leaf: bincode::serialize(leaf).unwrap(),
            cert1: ser(newest.then(|| bincode::serialize(block.leaf.qc()).unwrap())),
            cert2: ser(newest
                .then(|| chain.cert2(block))
                .flatten()
                .map(|c| bincode::serialize(&c).unwrap())),
            vid_common,
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

    fn next_height(&self) -> watch::Receiver<u64> {
        self.next.subscribe()
    }

    async fn ack(&self, height: u64) -> Result<(), ApiError> {
        let mut acked = self.acked.lock().unwrap();
        *acked = (*acked).max(height);
        let mut entries = self.entries.lock().unwrap();
        *entries = entries.split_off(&height);
        Ok(())
    }

    async fn payload(&self, key: PayloadKey) -> Result<Option<Bytes>, ApiError> {
        match key {
            PayloadKey::Height(h) => Ok(self.payloads.lock().unwrap().get(&h).cloned()),
            PayloadKey::Block { .. } => Ok(None),
        }
    }

    fn payloads_ready(&self) -> broadcast::Receiver<PayloadReady> {
        self.ready.subscribe()
    }
}

/// A sink in memory, holding the blocks from `start` on.
struct MemSink {
    start: u64,
    blocks: Mutex<Vec<BlockInfo<MockTypes>>>,
}

impl MemSink {
    fn new(start: u64) -> Arc<Self> {
        Arc::new(Self {
            start,
            blocks: Mutex::default(),
        })
    }

    fn heights(&self) -> Vec<u64> {
        self.blocks
            .lock()
            .unwrap()
            .iter()
            .map(|b| b.height())
            .collect()
    }
}

#[async_trait]
impl Sink<MockTypes> for MemSink {
    async fn append(&self, info: BlockInfo<MockTypes>) -> anyhow::Result<()> {
        self.blocks.lock().unwrap().push(info);
        Ok(())
    }

    async fn block_height(&self) -> anyhow::Result<u64> {
        Ok(self.start + self.blocks.lock().unwrap().len() as u64)
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

async fn wait_for(sink: &MemSink, height: u64) {
    timeout(Duration::from_secs(10), async {
        while sink.heights().last() != Some(&height) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the ingester reaches the height");
}

async fn chain(blocks: usize) -> MockChain {
    let mut chain = MockChain::new(4, 100).await;
    chain.push_empty(blocks).await;
    chain
}

/// Blocks reach the sink with their payload, VID common and certificates, and each one is
/// acknowledged once stored. An older leaf of a decide waits for the next leaf's QC.
#[tokio::test]
async fn ingests_decided_blocks() {
    let chain = chain(4).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=0);
    log.decide(&chain, 1..=2);
    let sink = MemSink::new(0);
    let _ingester = spawn_ingester(serve(log.clone()).await, sink.clone());

    wait_for(&sink, 2).await;
    log.decide(&chain, 3..=4);
    wait_for(&sink, 4).await;

    assert_eq!(sink.heights(), vec![0, 1, 2, 3, 4]);
    {
        let blocks = sink.blocks.lock().unwrap();
        for (stored, expected) in blocks.iter().zip(chain.blocks()) {
            assert_eq!(stored.leaf.leaf().commit(), expected.leaf.leaf().commit());
            assert_eq!(stored.leaf.qc(), expected.leaf.qc());
            assert_eq!(
                stored.block.as_ref().map(|b| b.payload().encode()),
                Some(expected.block.payload().encode())
            );
        }
        assert!(
            blocks[1].cert2.is_none(),
            "only the newest leaf of a decide has a cert2"
        );
        assert_eq!(blocks[2].cert2, chain.cert2(&chain.blocks()[2]));
    }
    timeout(Duration::from_secs(5), async {
        while *log.acked.lock().unwrap() < 5 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("every stored height is acknowledged");
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
    let sink = MemSink::new(0);
    let _ingester = spawn_ingester(serve(log).await, sink.clone());

    wait_for(&sink, 1).await;
    let blocks = sink.blocks.lock().unwrap();
    assert_eq!(blocks[0].vid_common.as_ref(), Some(&genesis.vid_common));
}

/// A restarted ingester continues from what the sink holds, and acknowledges that first.
#[tokio::test]
async fn resumes_from_the_sink() {
    let chain = chain(3).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=3);
    let sink = MemSink::new(2);
    let _ingester = spawn_ingester(serve(log.clone()).await, sink.clone());

    wait_for(&sink, 3).await;
    assert_eq!(sink.heights(), vec![2, 3]);
}

/// A leaf that does not follow the one before it stops the ingester.
#[tokio::test]
async fn a_broken_chain_stops_the_ingester() {
    let chain = chain(2).await;
    let other = chain_with_other_parent(&chain).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=1);
    log.insert(other);
    let sink = MemSink::new(0);
    let ingester = spawn_ingester(serve(log).await, sink.clone());

    let err = timeout(Duration::from_secs(10), ingester)
        .await
        .expect("the ingester stops")
        .unwrap();
    assert!(err.is::<ChainBroken>(), "{err:#}");
    assert_eq!(sink.heights(), vec![0, 1]);
}

/// An entry at height 2 from a chain that differs from `chain` at height 1.
async fn chain_with_other_parent(chain: &MockChain) -> Decided {
    // Diverge at height 1, so the leaf at height 2 names a different parent.
    let mut other = MockChain::new(4, 100).await;
    other
        .push(vec![crate::testing::mocks::MockTransaction::new(vec![7])])
        .await;
    other.push_empty(1).await;
    let block = &other.blocks()[2];
    assert_ne!(
        block.leaf.leaf().parent_commitment(),
        chain.blocks()[1].leaf.leaf().commit()
    );
    entry(&other, block, true)
}

/// Heights consensus no longer holds are skipped, starting from the earliest one it has.
#[tokio::test]
async fn skips_to_the_earliest_entry() {
    let chain = chain(4).await;
    let log = FakeLog::new();
    log.decide(&chain, 0..=4);
    log.ack(3).await.unwrap();
    let sink = MemSink::new(1);
    let _ingester = spawn_ingester(serve(log).await, sink.clone());

    wait_for(&sink, 4).await;
    // The sink's own count is behind; heights 1 and 2 are missing from it.
    assert_eq!(sink.heights(), vec![3, 4]);
}
