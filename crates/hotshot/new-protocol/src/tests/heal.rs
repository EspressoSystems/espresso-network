//! Share heal against in-memory peers: a node with an empty share store and an intact chain
//! re-creates its shares from what the other nodes hold.

use std::{
    collections::{BTreeSet, HashMap},
    num::NonZeroUsize,
    sync::Arc,
    time::Duration,
};

use hotshot_example_types::{node_types::TestTypes, storage_types::TestStorage};
use hotshot_types::{
    data::{VidCommitment, VidDisperseShare, VidDisperseShare2, ViewNumber},
    signature_key::BLSPubKey,
    traits::{signature_key::SignatureKey, storage::Storage as _},
    vid::avidm_gf2::AvidmGf2Scheme,
    vote::HasViewNumber,
};
use parking_lot::Mutex;
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
    time::{sleep, timeout},
};

use super::common::utils::{TestData, mock_membership_with_num_nodes};
use crate::{
    heal::{HealConfig, HealCtx, HealMetrics, run, serve::load_response},
    message::{
        fetch::Response,
        share::{ShareFetchRequest, ShareResponseBody},
    },
    storage::NewProtocolStorage,
};

const NODES: usize = 4;
const VIEWS: usize = 4;
const HEALER: usize = 0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Peer {
    Honest,
    /// Answers every request with `NotHeld`.
    Empty,
    /// Answers each view's first request with `Busy`, then honestly.
    BusyOnce,
    /// Answers with a share whose content does not belong to the commitment.
    Garbage,
}

struct Cluster {
    data: TestData,
    keys: Vec<(BLSPubKey, <BLSPubKey as SignatureKey>::PrivateKey)>,
    storages: Vec<TestStorage<TestTypes>>,
    peers: Arc<Mutex<Vec<Peer>>>,
    /// Views requested from peers, in order.
    requested: Arc<Mutex<Vec<ViewNumber>>>,
    load: Arc<Mutex<Load>>,
}

/// Requests peers have received and not yet answered, and the peaks seen.
#[derive(Default)]
struct Load {
    per_peer: HashMap<usize, usize>,
    per_view: HashMap<ViewNumber, usize>,
    max_per_peer: usize,
    max_views: usize,
}

impl Load {
    fn start(&mut self, peer: usize, view: ViewNumber) {
        let current = self.per_peer.entry(peer).or_default();
        *current += 1;
        self.max_per_peer = self.max_per_peer.max(*current);
        *self.per_view.entry(view).or_default() += 1;
        self.max_views = self.max_views.max(self.per_view.len());
    }

    fn end(&mut self, peer: usize, view: ViewNumber) {
        *self.per_peer.get_mut(&peer).unwrap() -= 1;
        let count = self.per_view.get_mut(&view).unwrap();
        *count -= 1;
        if *count == 0 {
            self.per_view.remove(&view);
        }
    }
}

/// Time a peer takes to answer, so requests overlap.
const LATENCY: Duration = Duration::from_millis(20);

/// `(peer, view)` pairs that already got their `Busy`.
type Busied = Arc<Mutex<BTreeSet<(usize, ViewNumber)>>>;

impl Cluster {
    /// Every node but the healer holds its share of every view; the healer's store knows the chain.
    async fn new() -> Self {
        Self::with_views(VIEWS).await
    }

    async fn with_views(views: usize) -> Self {
        let data = TestData::new_with_epoch_height_and_num_nodes(views, 10, NODES).await;
        let keys: Vec<_> = (0..NODES as u64)
            .map(|i| BLSPubKey::generated_from_seed_indexed([0u8; 32], i))
            .collect();
        let storages: Vec<_> = (0..NODES).map(|_| TestStorage::default()).collect();
        for (storage, key) in storages.iter().zip(&keys).skip(HEALER + 1) {
            for view in &data.views {
                let share = VidDisperseShare::V2(view.vid_share_for(&key.0));
                storage
                    .append_vid(&share.to_proposal(&key.1).unwrap())
                    .await
                    .unwrap();
            }
        }
        for view in &data.views {
            storages[HEALER]
                .insert_decided_leaf(view.leaf.clone())
                .await;
        }
        Self {
            data,
            keys,
            storages,
            peers: Arc::new(Mutex::new(vec![Peer::Honest; NODES])),
            requested: Arc::default(),
            load: Arc::default(),
        }
    }

    fn config() -> HealConfig {
        HealConfig {
            w: Duration::from_secs(u32::MAX.into()),
            tip_margin: 0,
            min_remaining: Duration::ZERO,
            scan_interval: Duration::from_millis(100),
            req_timeout: Duration::from_millis(500),
            // The test metadata has no namespace table, so every height costs `usize::MAX`.
            inflight_bytes: usize::MAX,
            ..HealConfig::default()
        }
    }

    /// Start the healer over `storage`, with peers answered from the cluster's stores.
    fn start_healer(&self, storage: TestStorage<TestTypes>) -> JoinHandle<()> {
        self.start_healer_with(storage, Self::config(), ViewNumber::new(1000))
    }

    fn start_healer_with(
        &self,
        storage: TestStorage<TestTypes>,
        config: HealConfig,
        current_view: ViewNumber,
    ) -> JoinHandle<()> {
        let (public_key, private_key) = self.keys[HEALER].clone();
        let (membership, ..) = mock_membership_with_num_nodes(NODES, 10, public_key);
        let (requests, mut requests_rx) = mpsc::channel::<(BLSPubKey, ShareFetchRequest)>(64);
        let (responses_tx, responses) = mpsc::channel(64);
        let (_view_tx, view) = watch::channel(current_view);
        let ctx = HealCtx {
            config,
            storage,
            membership,
            public_key,
            private_key,
            requests,
            view,
            metrics: HealMetrics::new(&hotshot_types::traits::metrics::NoMetrics),
        };

        let (keys, storages) = (self.keys.clone(), self.storages.clone());
        let (peers, requested) = (self.peers.clone(), self.requested.clone());
        let load = self.load.clone();
        let busied = Busied::default();
        let data_shares: HashMap<_, _> = self
            .data
            .views
            .iter()
            .map(|view| (view.view_number, view.vid_shares.clone()))
            .collect();
        tokio::spawn(async move {
            let _view_tx = _view_tx;
            let (keys, storages) = (Arc::new(keys), Arc::new(storages));
            let data_shares = Arc::new(data_shares);
            while let Some((peer, request)) = requests_rx.recv().await {
                let idx = keys.iter().position(|key| key.0 == peer).unwrap();
                let view = request.view_number();
                requested.lock().push(view);
                load.lock().start(idx, view);
                let behavior = peers.lock()[idx];
                let first_busy = behavior == Peer::BusyOnce && busied.lock().insert((idx, view));
                let (storages, data_shares) = (storages.clone(), data_shares.clone());
                let (responses_tx, load) = (responses_tx.clone(), load.clone());
                tokio::spawn(async move {
                    sleep(LATENCY).await;
                    let body = match behavior {
                        Peer::Honest | Peer::BusyOnce if !first_busy => {
                            load_response(&storages[idx], &request, usize::MAX).await.0
                        },
                        Peer::BusyOnce => ShareResponseBody::Busy,
                        Peer::Empty => ShareResponseBody::NotHeld,
                        Peer::Garbage => {
                            let honest =
                                data_shares[&view].iter().find(|s| s.recipient_key == peer);
                            ShareResponseBody::Share(Box::new(garbage(honest.unwrap().clone())))
                        },
                        Peer::Honest => unreachable!(),
                    };
                    load.lock().end(idx, view);
                    // The healer may have ended; a late response is dropped.
                    let _ = responses_tx.send((peer, Response::new(view, body))).await;
                });
            }
        });
        tokio::spawn(run(ctx, responses))
    }

    async fn held(&self, storage: &TestStorage<TestTypes>) -> BTreeSet<ViewNumber> {
        storage.held_vid_views().await.unwrap()
    }

    fn all_views(&self) -> BTreeSet<ViewNumber> {
        self.data.views.iter().map(|v| v.view_number).collect()
    }

    /// Wait for the healer's store to hold exactly `views`.
    async fn wait_held(&self, storage: &TestStorage<TestTypes>, views: &BTreeSet<ViewNumber>) {
        let waited = timeout(Duration::from_secs(10), async {
            while self.held(storage).await != *views {
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await;
        if waited.is_err() {
            panic!(
                "held views never became {views:?}: {:?}",
                self.held(storage).await
            );
        }
    }

    /// The share every view's live path stored for the healer.
    fn live_share(&self, view: ViewNumber) -> VidDisperseShare2<TestTypes> {
        let view = self
            .data
            .views
            .iter()
            .find(|v| v.view_number == view)
            .unwrap();
        view.vid_share_for(&self.keys[HEALER].0)
    }

    async fn assert_byte_equal(
        &self,
        storage: &TestStorage<TestTypes>,
        views: &BTreeSet<ViewNumber>,
    ) {
        for view in views {
            let healed = storage.load_vid_share(*view).await.unwrap().unwrap();
            let live = VidDisperseShare::V2(self.live_share(*view))
                .to_proposal(&self.keys[HEALER].1)
                .unwrap();
            assert_eq!(healed, live, "view {view}");
        }
    }
}

/// The share with its content swapped for another payload's at the same shard range, so it
/// passes intake but fails merkle verification.
fn garbage(mut share: VidDisperseShare2<TestTypes>) -> VidDisperseShare2<TestTypes> {
    let weights = [1u32; NODES];
    let other = vec![0xb7u8; 64];
    let (_, _, shares) = AvidmGf2Scheme::ns_disperse(
        &share.common.param,
        &weights,
        &other,
        std::iter::once(0..other.len()),
    )
    .unwrap();
    let range = share.share.range().unwrap();
    share.share = shares[range.start].clone();
    share
}

#[tokio::test(flavor = "multi_thread")]
async fn heal_in_memory_peers_ok() {
    let cluster = Cluster::new().await;
    let storage = cluster.storages[HEALER].clone();
    assert!(cluster.held(&storage).await.is_empty());

    let healer = cluster.start_healer(storage.clone());
    cluster.wait_held(&storage, &cluster.all_views()).await;
    cluster
        .assert_byte_equal(&storage, &cluster.all_views())
        .await;
    healer.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn heal_bad_share_weeded_ok() {
    let cluster = Cluster::new().await;
    cluster.peers.lock()[1] = Peer::Garbage;
    let storage = cluster.storages[HEALER].clone();

    let healer = cluster.start_healer(storage.clone());
    cluster.wait_held(&storage, &cluster.all_views()).await;
    cluster
        .assert_byte_equal(&storage, &cluster.all_views())
        .await;
    healer.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn heal_below_threshold_retries_ok() {
    let cluster = Cluster::new().await;
    {
        let mut peers = cluster.peers.lock();
        peers[1] = Peer::Empty;
        peers[2] = Peer::Empty;
        peers[3] = Peer::Empty;
    }
    let storage = cluster.storages[HEALER].clone();
    let healer = cluster.start_healer(storage.clone());

    // Several scans ask and find nothing.
    sleep(Duration::from_millis(600)).await;
    assert!(cluster.held(&storage).await.is_empty());
    assert!(cluster.requested.lock().len() > cluster.data.views.len());

    cluster.peers.lock().fill(Peer::Honest);
    cluster.wait_held(&storage, &cluster.all_views()).await;
    healer.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn heal_restart_resumes_ok() {
    let cluster = Cluster::new().await;
    let storage = cluster.storages[HEALER].clone();
    // A previous run healed the newest view before stopping.
    let done = cluster.data.views.last().unwrap().view_number;
    let share = VidDisperseShare::V2(cluster.live_share(done));
    storage
        .append_vid(&share.to_proposal(&cluster.keys[HEALER].1).unwrap())
        .await
        .unwrap();

    let healer = cluster.start_healer(storage.clone());
    cluster.wait_held(&storage, &cluster.all_views()).await;
    healer.abort();
    assert!(!cluster.requested.lock().contains(&done));
}

/// The leader of view 2 committed to a non-codeword: no share set decodes, so the height is
/// skipped while the others heal.
#[tokio::test(flavor = "multi_thread")]
async fn heal_unrecoverable_skipped_ok() {
    let mut cluster = Cluster::new().await;
    let poisoned = cluster.data.views[1].view_number;
    cluster.poison(poisoned).await;
    let storage = cluster.storages[HEALER].clone();

    let healer = cluster.start_healer(storage.clone());
    let mut expected = cluster.all_views();
    expected.remove(&poisoned);
    cluster.wait_held(&storage, &expected).await;
    sleep(Duration::from_millis(500)).await;
    assert_eq!(cluster.held(&storage).await, expected);
    healer.abort();
}

/// With nothing left to heal the task ends, and no later scan reads the wal again.
#[tokio::test(flavor = "multi_thread")]
async fn heal_idles_when_done_ok() {
    let cluster = Cluster::new().await;
    let storage = cluster.storages[HEALER].clone();
    let healer = cluster.start_healer(storage.clone());
    cluster.wait_held(&storage, &cluster.all_views()).await;
    timeout(Duration::from_secs(2), healer)
        .await
        .expect("healer is still scanning")
        .unwrap();
}

/// Missing shares too close to the tip are healed once they are old enough, so the healer keeps
/// scanning while any remain.
#[tokio::test(flavor = "multi_thread")]
async fn heal_keeps_scanning_for_recent_heights_ok() {
    let cluster = Cluster::new().await;
    let storage = cluster.storages[HEALER].clone();
    let config = HealConfig {
        tip_margin: 2,
        ..Cluster::config()
    };
    let healer = cluster.start_healer_with(storage.clone(), config, ViewNumber::new(3));
    let oldest = BTreeSet::from([cluster.data.views[0].view_number]);
    cluster.wait_held(&storage, &oldest).await;
    sleep(Duration::from_millis(600)).await;
    assert!(!healer.is_finished());
    healer.abort();
}

/// A leaf without an epoch is skipped, not healed as if it belonged to the genesis epoch.
#[tokio::test(flavor = "multi_thread")]
async fn heal_leaf_without_epoch_skipped_ok() {
    let cluster = Cluster::new().await;
    let storage = cluster.storages[HEALER].clone();
    let skipped = cluster.data.views[1].view_number;
    let mut leaf = cluster.data.views[1].leaf.clone();
    leaf.with_epoch = false;
    storage.insert_decided_leaf(leaf).await;

    let healer = cluster.start_healer(storage.clone());
    let mut expected = cluster.all_views();
    expected.remove(&skipped);
    cluster.wait_held(&storage, &expected).await;
    sleep(Duration::from_millis(300)).await;
    assert_eq!(cluster.held(&storage).await, expected);
    healer.abort();
}

/// Peers that answer `Busy` are asked again after a backoff, within the same height.
#[tokio::test(flavor = "multi_thread")]
async fn heal_busy_peers_asked_again_ok() {
    let cluster = Cluster::new().await;
    cluster.peers.lock().fill(Peer::BusyOnce);
    let storage = cluster.storages[HEALER].clone();
    let config = HealConfig {
        scan_interval: Duration::from_secs(3600),
        busy_backoff: Duration::from_millis(50),
        ..Cluster::config()
    };
    let healer = cluster.start_healer_with(storage.clone(), config, ViewNumber::new(1000));
    cluster.wait_held(&storage, &cluster.all_views()).await;
    healer.abort();
}

/// A height that cannot be healed is asked for once, not on every scan.
#[tokio::test(flavor = "multi_thread")]
async fn heal_terminal_failure_not_retried_ok() {
    let mut cluster = Cluster::new().await;
    let poisoned = cluster.data.views[1].view_number;
    cluster.poison(poisoned).await;
    let storage = cluster.storages[HEALER].clone();

    let healer = cluster.start_healer(storage.clone());
    let mut expected = cluster.all_views();
    expected.remove(&poisoned);
    cluster.wait_held(&storage, &expected).await;
    sleep(Duration::from_millis(600)).await;
    healer.abort();
    let asked = cluster
        .requested
        .lock()
        .iter()
        .filter(|view| **view == poisoned)
        .count();
    assert!(asked < NODES, "asked {asked} times");
}

/// Many targets heal at once, byte-equal to the live shares, with the per-peer cap held.
#[tokio::test(flavor = "multi_thread")]
async fn heal_many_heights_in_parallel_ok() {
    let cluster = Cluster::with_views(20).await;
    let storage = cluster.storages[HEALER].clone();
    let config = HealConfig {
        heights_inflight: NonZeroUsize::new(8).unwrap(),
        peer_inflight: NonZeroUsize::new(2).unwrap(),
        ..Cluster::config()
    };
    let healer = cluster.start_healer_with(storage.clone(), config, ViewNumber::new(1000));
    cluster.wait_held(&storage, &cluster.all_views()).await;
    cluster
        .assert_byte_equal(&storage, &cluster.all_views())
        .await;
    healer.abort();
    let load = cluster.load.lock();
    assert!(load.max_views > 1, "heights ran one at a time");
    assert!(load.max_views <= 8, "{} heights at once", load.max_views);
    assert!(
        load.max_per_peer <= 2,
        "{} requests to a peer",
        load.max_per_peer
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn heal_peer_cap_respected_ok() {
    let cluster = Cluster::with_views(12).await;
    let storage = cluster.storages[HEALER].clone();
    let config = HealConfig {
        heights_inflight: NonZeroUsize::new(8).unwrap(),
        peer_inflight: NonZeroUsize::MIN,
        ..Cluster::config()
    };
    let healer = cluster.start_healer_with(storage.clone(), config, ViewNumber::new(1000));
    cluster.wait_held(&storage, &cluster.all_views()).await;
    healer.abort();
    assert_eq!(cluster.load.lock().max_per_peer, 1);
}

/// A budget below one height still heals, one height at a time.
#[tokio::test(flavor = "multi_thread")]
async fn heal_byte_budget_serializes_heights_ok() {
    let cluster = Cluster::with_views(8).await;
    let storage = cluster.storages[HEALER].clone();
    let config = HealConfig {
        heights_inflight: NonZeroUsize::new(8).unwrap(),
        inflight_bytes: 0,
        ..Cluster::config()
    };
    let healer = cluster.start_healer_with(storage.clone(), config, ViewNumber::new(1000));
    cluster.wait_held(&storage, &cluster.all_views()).await;
    healer.abort();
    assert_eq!(cluster.load.lock().max_views, 1);
}

/// A live append of a view the healer also heals leaves one identical share.
#[tokio::test(flavor = "multi_thread")]
async fn heal_duplicate_view_ok() {
    let cluster = Cluster::new().await;
    let storage = cluster.storages[HEALER].clone();
    let healer = cluster.start_healer(storage.clone());
    cluster.wait_held(&storage, &cluster.all_views()).await;

    let view = cluster.data.views[0].view_number;
    let live = VidDisperseShare::V2(cluster.live_share(view))
        .to_proposal(&cluster.keys[HEALER].1)
        .unwrap();
    storage.append_vid_stamped(&live, 1).await.unwrap();
    assert_eq!(cluster.held(&storage).await, cluster.all_views());
    cluster
        .assert_byte_equal(&storage, &cluster.all_views())
        .await;
    healer.abort();
}

impl Cluster {
    /// Replace `view`'s dispersal with one committing to a non-codeword, in the chain and in
    /// every peer's store.
    async fn poison(&mut self, view: ViewNumber) {
        let index = self
            .data
            .views
            .iter()
            .position(|v| v.view_number == view)
            .unwrap();
        let template = self.data.views[index].vid_shares[0].clone();
        let weights = [1u32; NODES];
        let payload = vec![0x5au8; 64];
        let (commit, common, shares) = AvidmGf2Scheme::ns_disperse_non_codeword(
            &template.common.param,
            &weights,
            &payload,
            std::iter::once(0..payload.len()),
        )
        .unwrap();

        self.data.views[index]
            .leaf
            .block_header_mut()
            .payload_commitment = VidCommitment::V2(commit);
        let leaf = self.data.views[index].leaf.clone();
        self.storages[HEALER].insert_decided_leaf(leaf).await;
        for (i, share) in shares.into_iter().enumerate() {
            let share = VidDisperseShare2::<TestTypes> {
                payload_commitment: commit,
                share,
                recipient_key: self.keys[i].0,
                common: common.clone(),
                ..template.clone()
            };
            self.data.views[index].vid_shares[i] = share.clone();
            if i != HEALER {
                let proposal = VidDisperseShare::V2(share)
                    .to_proposal(&self.keys[i].1)
                    .unwrap();
                self.storages[i].append_vid(&proposal).await.unwrap();
            }
        }
    }
}
