//! Seeds the post-start epoch catchup from the LC contract.

use std::{
    fmt::Display,
    future::Future,
    pin::pin,
    sync::{Arc, OnceLock, Weak},
    time::Duration,
};

use anyhow::{Context, anyhow, bail};
use espresso_types::{
    BlockMerkleCommitment, ChainConfig, Header, LcContractFinalizedState, SeqTypes,
    traits::SequencerPersistence,
};
use futures::{
    FutureExt, StreamExt, TryStreamExt,
    future::{Either, select},
    stream,
};
use hotshot_query_service::node::BlockId;
use hotshot_types::{
    data::EpochNumber,
    epoch_membership::EpochMembershipCoordinator,
    traits::metrics::{Counter, Metrics},
};
use light_client::client::{Client, QueryServiceClient};
use parking_lot::Mutex;
use tokio::time::Instant;
use url::Url;

use crate::request_response::{
    RequestResponseProtocol,
    request::{Request, Response},
};

/// Most requests of one seed in flight; peers answer 4 requests per key at a time.
const MAX_CONCURRENT_REQUESTS: usize = 3;

/// Bound on each peer request of a seed: with no answering peer the seed errors within it.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// The LC-contract seed behind `CatchupSeedFn`; steps in `doc/agents/architecture.md`.
/// Every installed table is proven against the header at `F` that `verify_header` accepted.
#[derive(Clone)]
pub struct LcContractSeed<P: SequencerPersistence> {
    /// Set by `bind_requests` once consensus exists; weak, as `SequencerContext` owns the protocol.
    requests: Arc<OnceLock<Weak<RequestResponseProtocol<P>>>>,
    /// Asked in order once eligible; none, no HTTP.
    state_peers: Arc<StatePeers<QueryServiceClient>>,
    /// How long after a fetch starts the state peers become eligible, at most a third of
    /// `REQUEST_TIMEOUT`.
    http_delay: Duration,
    /// Seed attempts that installed a window.
    seeded: Arc<dyn Counter>,
}

impl<P: SequencerPersistence> LcContractSeed<P> {
    /// A seed without a request-response protocol, `None` when `chain_config` names no stake
    /// table contract to read the LC contract from. Until `bind_requests`, `state_peers` answer
    /// at once, else `http_delay` after a fetch starts.
    pub fn new(
        chain_config: &ChainConfig,
        state_peers: Vec<Url>,
        http_delay: Duration,
        metrics: &dyn Metrics,
    ) -> Option<Self> {
        chain_config.stake_table_contract.map(|_| Self {
            requests: Arc::default(),
            state_peers: Arc::new(StatePeers::new(
                state_peers
                    .into_iter()
                    .map(|url| (url.clone(), QueryServiceClient::new(url)))
                    .collect(),
            )),
            http_delay: http_delay.min(REQUEST_TIMEOUT / 3),
            seeded: metrics
                .subgroup("catchup".into())
                .create_counter("lc_contract_seeds".into(), None)
                .into(),
        })
    }

    /// Binds the request-response protocol, which exists only after consensus is constructed.
    pub fn bind_requests(&self, requests: &Arc<RequestResponseProtocol<P>>) {
        self.requests
            .set(Arc::downgrade(requests))
            .expect("the request-response protocol is bound once");
    }

    /// Sets the catchup hook of `coordinator` to this seed.
    pub fn install(self, coordinator: &EpochMembershipCoordinator<SeqTypes>) {
        let seed = Arc::new(self);
        coordinator.set_catchup_seed(Arc::new(move |target, start, coordinator| {
            let seed = Arc::clone(&seed);
            async move { seed.run(target, start, coordinator).await }.boxed()
        }));
    }

    /// One seed attempt for `catchup(target)` with held pair top `start`: the walk start, `start`
    /// when not needed, the last installed epoch when seeded. The L1 read, the
    /// peer requests and each install are bounded.
    async fn run(
        &self,
        target: EpochNumber,
        start: EpochNumber,
        coordinator: EpochMembershipCoordinator<SeqTypes>,
    ) -> anyhow::Result<EpochNumber> {
        let started = tokio::time::Instant::now();
        let lc = coordinator
            .membership()
            .fetcher()
            .lc_contract_finalized_state()
            .await?;
        let epoch_height = *coordinator.epoch_height();
        let seed_epoch = lc.seed_epoch(epoch_height);
        let log = |outcome: &str| {
            tracing::info!(
                target: "announce::catchup",
                %target,
                held_start = %start,
                lc_contract_height = lc.height,
                %seed_epoch,
                outcome,
                duration_ms = started.elapsed().as_millis() as u64,
                "lc contract seed"
            );
        };
        let Some(window) = lc.seed_window(
            epoch_height,
            coordinator.membership().first_epoch(),
            start,
            target,
        ) else {
            log("not needed");
            return Ok(start);
        };

        let heights = window.root_heights(epoch_height).collect();
        let requests = self.requests.get().and_then(Weak::upgrade);
        let sources = Sources::new(requests.as_ref(), &self.state_peers, self.http_delay);
        let (preceding, roots) =
            fetch_headers(&sources, &lc, window.preceding_root, heights).await?;
        coordinator
            .membership()
            .install_epoch_roots(preceding, roots, &coordinator)
            .await?;

        self.seeded.add(1);
        log("seeded");
        Ok(*window.epochs.end())
    }
}

/// The request-response side of a fetch.
trait PeerRequests: Send + Sync {
    /// The block Merkle root of the header at `F`, accepted by `lc.verify_header`. A rejected
    /// answer is skipped and the request keeps waiting.
    fn root(
        &self,
        lc: &LcContractFinalizedState,
    ) -> impl Send + Future<Output = anyhow::Result<BlockMerkleCommitment>>;

    /// The header at `height`, proven against `root` of the header at `root_height`.
    fn proven_header(
        &self,
        root_height: u64,
        height: u64,
        root: BlockMerkleCommitment,
    ) -> impl Send + Future<Output = anyhow::Result<Header>>;
}

impl<P: SequencerPersistence> PeerRequests for Arc<RequestResponseProtocol<P>> {
    async fn root(&self, lc: &LcContractFinalizedState) -> anyhow::Result<BlockMerkleCommitment> {
        let checked_lc = lc.clone();
        Ok(self
            .request_indefinitely(
                Request::Leaf(lc.height),
                move |_: &Request, response: Response| {
                    let lc = checked_lc.clone();
                    async move {
                        let Response::Leaf(leaves) = response else {
                            bail!("expected leaf response");
                        };
                        let header = leaves.first().context("empty leaf chain")?.block_header();
                        lc.verify_header(header)
                    }
                    .map(|result| {
                        result.inspect_err(|err| tracing::debug!("seed header rejected: {err:#}"))
                    })
                },
            )
            .await?)
    }

    async fn proven_header(
        &self,
        root_height: u64,
        height: u64,
        root: BlockMerkleCommitment,
    ) -> anyhow::Result<Header> {
        Ok(self
            .request_indefinitely(
                Request::HeaderProof {
                    root: root_height,
                    height,
                },
                move |_: &Request, response: Response| {
                    async move {
                        let Response::HeaderProof(proof) = response else {
                            bail!("expected header proof response");
                        };
                        proof.verify_at_height(root, height)
                    }
                    .map(|result| {
                        result.inspect_err(|err| tracing::debug!("header proof rejected: {err:#}"))
                    })
                },
            )
            .await?)
    }
}

/// The state peers in the order they are asked. A peer that failed moves to the back for the rest
/// of the process; a peer that later answers is not promoted.
struct StatePeers<C> {
    /// The peers with the URLs the debug log names them by.
    peers: Vec<(Url, C)>,
    /// Indices into `peers`, in the order they are asked.
    order: Mutex<Vec<usize>>,
}

impl<C: Client> StatePeers<C> {
    fn new(peers: Vec<(Url, C)>) -> Self {
        let order = Mutex::new((0..peers.len()).collect());
        Self { peers, order }
    }

    /// The block Merkle root of the header at `F` from the state peers.
    async fn root(&self, lc: &LcContractFinalizedState) -> anyhow::Result<BlockMerkleCommitment> {
        self.first_accepted(|peer| async move {
            let header = peer
                .get_leaves_in_range(lc.height as usize, lc.height as usize + 1)
                .await?
                .into_iter()
                .next()
                .context("empty leaf range")?
                .header()
                .clone();
            lc.verify_header(&header)
        })
        .await
    }

    /// The header at `height`, proven against `root` of the header at `root_height`.
    async fn proven_header(
        &self,
        root_height: u64,
        height: u64,
        root: BlockMerkleCommitment,
    ) -> anyhow::Result<Header> {
        self.first_accepted(|peer| async move {
            peer.header_proof(root_height, BlockId::Number(height as usize))
                .await?
                .verify_at_height(root, height)
        })
        .await
    }

    /// The first `Ok` of `ask` over the peers, in order. An error or a rejection moves the peer
    /// to the back and on to the next; all failing returns the last error. Spawns nothing.
    async fn first_accepted<'a, T, Fut>(&'a self, ask: impl Fn(&'a C) -> Fut) -> anyhow::Result<T>
    where
        Fut: Future<Output = anyhow::Result<T>>,
    {
        let mut last_err = anyhow!("no state peers");
        let asked = self.order.lock().clone();
        for index in asked {
            let (url, peer) = &self.peers[index];
            match ask(peer).await {
                Ok(value) => return Ok(value),
                Err(err) => {
                    tracing::debug!(%url, "state peer failed: {err:#}");
                    let mut order = self.order.lock();
                    order.retain(|asked| *asked != index);
                    order.push(index);
                    last_err = err;
                },
            }
        }
        Err(last_err)
    }
}

/// Where one fetch asks: request-response, and the state peers from `state_peers_from`.
struct Sources<'a, R, C> {
    /// `None` while the protocol is unavailable.
    requests: Option<&'a R>,
    state_peers: &'a StatePeers<C>,
    /// `http_delay` after the fetch starts, at once without request-response. One instant for
    /// every request of the fetch.
    state_peers_from: Instant,
}

impl<'a, R: PeerRequests, C: Client> Sources<'a, R, C> {
    fn new(requests: Option<&'a R>, state_peers: &'a StatePeers<C>, http_delay: Duration) -> Self {
        let delay = if requests.is_some() {
            http_delay
        } else {
            Duration::ZERO
        };
        Self {
            requests,
            state_peers,
            state_peers_from: Instant::now() + delay,
        }
    }

    /// The block Merkle root of the header at `F`.
    async fn root(&self, lc: &LcContractFinalizedState) -> anyhow::Result<BlockMerkleCommitment> {
        let requests = async {
            self.requests
                .context("the request-response protocol is not available")?
                .root(lc)
                .await
        };
        self.request(
            format!("the LC contract header at height {}", lc.height),
            requests,
            self.state_peers.root(lc),
        )
        .await
    }

    /// The header at `height`, proven against `root` of the header at `root_height`.
    async fn proven_header(
        &self,
        root_height: u64,
        height: u64,
        root: BlockMerkleCommitment,
    ) -> anyhow::Result<Header> {
        let requests = async {
            self.requests
                .context("the request-response protocol is not available")?
                .proven_header(root_height, height, root)
                .await
        };
        self.request(
            format!("the header proof at height {height}"),
            requests,
            self.state_peers.proven_header(root_height, height, root),
        )
        .await
    }

    /// `requests` raced against `state_peers` under `REQUEST_TIMEOUT`; a timeout error names
    /// `what` and the last error of the state peers.
    async fn request<T>(
        &self,
        what: impl Display,
        requests: impl Future<Output = anyhow::Result<T>>,
        state_peers: impl Future<Output = anyhow::Result<T>>,
    ) -> anyhow::Result<T> {
        let last_state_peer_error = Mutex::new(None::<String>);
        let state_peers = async {
            state_peers
                .await
                .inspect_err(|err| *last_state_peer_error.lock() = Some(format!("{err:#}")))
        };
        tokio::time::timeout(
            REQUEST_TIMEOUT,
            race(requests, state_peers, self.state_peers_from),
        )
        .await
        .map_err(|_| {
            let last_error = last_state_peer_error.lock().take();
            match last_error {
                Some(err) => anyhow!(
                    "{what} not received within {REQUEST_TIMEOUT:?}; last state peer error: {err}"
                ),
                None => anyhow!("{what} not received within {REQUEST_TIMEOUT:?}"),
            }
        })?
    }
}

/// The block Merkle root of the header at `F`, then the headers it proves: the preceding root
/// (the block time source of the lowest installed reward) and `proven_heights`, ascending.
async fn fetch_headers<R: PeerRequests, C: Client>(
    sources: &Sources<'_, R, C>,
    lc: &LcContractFinalizedState,
    preceding_height: Option<u64>,
    proven_heights: Vec<u64>,
) -> anyhow::Result<(Option<Header>, Vec<Header>)> {
    let root = sources.root(lc).await?;
    let heights = preceding_height.into_iter().chain(proven_heights);
    let mut headers: Vec<Header> = stream::iter(heights)
        .map(|height| sources.proven_header(lc.height, height, root))
        .buffered(MAX_CONCURRENT_REQUESTS)
        .try_collect()
        .await?;
    let preceding = preceding_height.map(|_| headers.remove(0));
    Ok((preceding, headers))
}

/// The first `Ok` of `primary` and `fallback`; `fallback` starts when `primary` errs or at
/// `fallback_from`. An error of one side waits for the other; both failing returns both reasons.
async fn race<T>(
    primary: impl Future<Output = anyhow::Result<T>>,
    fallback: impl Future<Output = anyhow::Result<T>>,
    fallback_from: Instant,
) -> anyhow::Result<T> {
    let mut primary = pin!(primary);
    tokio::select! {
        result = primary.as_mut() => match result {
            Ok(value) => return Ok(value),
            Err(err) => {
                tracing::debug!("seed request-response side failed: {err:#}");
                return fallback.await.map_err(|fallback_err| both_failed(err, fallback_err));
            },
        },
        () = tokio::time::sleep_until(fallback_from) => {},
    }
    match select(primary, pin!(fallback)).await {
        Either::Left((Ok(value), _)) | Either::Right((Ok(value), _)) => Ok(value),
        Either::Left((Err(err), rest)) => {
            tracing::debug!("seed request-response side failed: {err:#}");
            rest.await
                .map_err(|fallback_err| both_failed(err, fallback_err))
        },
        Either::Right((Err(err), rest)) => {
            tracing::debug!("seed state peers side failed: {err:#}");
            rest.await
                .map_err(|primary_err| both_failed(primary_err, err))
        },
    }
}

/// The error of a fetch whose two sides failed.
fn both_failed(primary_err: anyhow::Error, fallback_err: anyhow::Error) -> anyhow::Error {
    fallback_err.context(format!(
        "request-response failed: {primary_err:#}; state peers"
    ))
}

#[cfg(test)]
mod tests {
    use hotshot_types::{
        data::ViewNumber,
        traits::{block_contents::BlockHeader, metrics::NoMetrics},
    };
    use light_client::testing::TestClient;

    use super::*;
    use crate::persistence::no_storage::NoStorage;

    const EPOCH_HEIGHT: u64 = 10;
    const HTTP_DELAY: Duration = Duration::from_secs(5);

    /// A request-response side that never answers.
    struct Silent;

    impl PeerRequests for Silent {
        async fn root(
            &self,
            _lc: &LcContractFinalizedState,
        ) -> anyhow::Result<BlockMerkleCommitment> {
            std::future::pending().await
        }

        async fn proven_header(
            &self,
            _root_height: u64,
            _height: u64,
            _root: BlockMerkleCommitment,
        ) -> anyhow::Result<Header> {
            std::future::pending().await
        }
    }

    async fn header(client: &TestClient, height: usize) -> Header {
        client.leaf(height).await.leaf().block_header().clone()
    }

    fn finalized_state(header: &Header) -> LcContractFinalizedState {
        LcContractFinalizedState {
            height: header.height(),
            block_comm_root: header
                .get_light_client_state(ViewNumber::new(0))
                .unwrap()
                .block_comm_root,
        }
    }

    fn state_peers(clients: Vec<TestClient>) -> StatePeers<TestClient> {
        StatePeers::new(
            clients
                .into_iter()
                .map(|client| ("http://peer.test".parse().unwrap(), client))
                .collect(),
        )
    }

    /// The instant the state peers become eligible, `HTTP_DELAY` from now.
    fn after_delay() -> Instant {
        Instant::now() + HTTP_DELAY
    }

    /// Answers `value` after `delay`.
    async fn answer_after(delay: Duration, value: u64) -> anyhow::Result<u64> {
        tokio::time::sleep(delay).await;
        Ok(value)
    }

    /// Fails with `reason` after `delay`.
    async fn fail_after(delay: Duration, reason: &'static str) -> anyhow::Result<u64> {
        tokio::time::sleep(delay).await;
        bail!(reason)
    }

    #[tokio::test(start_paused = true)]
    async fn fallback_after_delay_ok() {
        let started = Instant::now();

        let value = race(
            std::future::pending(),
            answer_after(Duration::ZERO, 1),
            after_delay(),
        )
        .await
        .unwrap();

        assert_eq!(value, 1);
        assert_eq!(started.elapsed(), HTTP_DELAY);
    }

    #[tokio::test(start_paused = true)]
    async fn fallback_prefers_primary_ok() {
        let started = Instant::now();

        let value = race(
            answer_after(Duration::ZERO, 1),
            std::future::pending(),
            after_delay(),
        )
        .await
        .unwrap();

        assert_eq!(value, 1);
        assert!(started.elapsed() < HTTP_DELAY);
    }

    /// A primary that errs starts the fallback at once.
    #[tokio::test(start_paused = true)]
    async fn fallback_after_primary_error_at_once_ok() {
        let started = Instant::now();

        let value = race(
            fail_after(Duration::ZERO, "scripted failure"),
            answer_after(Duration::ZERO, 1),
            after_delay(),
        )
        .await
        .unwrap();

        assert_eq!(value, 1);
        assert_eq!(started.elapsed(), Duration::ZERO);
    }

    /// Both sides failing returns both reasons.
    #[tokio::test(start_paused = true)]
    async fn fallback_both_fail_errs() {
        let started = Instant::now();

        let err = race(
            fail_after(Duration::ZERO, "primary reason"),
            fail_after(Duration::ZERO, "fallback reason"),
            after_delay(),
        )
        .await
        .unwrap_err();

        let message = format!("{err:#}");
        assert!(message.contains("primary reason"), "{message}");
        assert!(message.contains("fallback reason"), "{message}");
        assert_eq!(started.elapsed(), Duration::ZERO);
    }

    /// An error of the fallback does not end the request while the primary is still asking.
    #[tokio::test(start_paused = true)]
    async fn fallback_error_waits_for_primary() {
        let late = HTTP_DELAY * 2;
        let started = Instant::now();

        let value = race(
            answer_after(late, 1),
            fail_after(Duration::ZERO, "scripted failure"),
            after_delay(),
        )
        .await
        .unwrap();

        assert_eq!(value, 1);
        assert_eq!(started.elapsed(), late);
    }

    /// No state peers: the fallback errs and the primary still answers.
    #[tokio::test(start_paused = true)]
    async fn no_state_peers_wait_for_primary_ok() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let lc = finalized_state(&header(&client, 62).await);
        let peers = state_peers(vec![]);
        let late = HTTP_DELAY * 2;
        let started = Instant::now();

        let value = race(
            answer_after(late, 1),
            async {
                peers.root(&lc).await?;
                Ok(2)
            },
            after_delay(),
        )
        .await
        .unwrap();

        assert_eq!(value, 1);
        assert_eq!(started.elapsed(), late);
    }

    /// The next state peer is asked when an answer is rejected.
    #[tokio::test]
    async fn state_peers_skip_rejected_answer_ok() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let served = header(&client, 62).await;
        let lc = finalized_state(&served);
        let forging = TestClient::with_epoch_height(EPOCH_HEIGHT);
        forging.return_wrong_leaf(62, 61).await;

        let root = state_peers(vec![forging, client]).root(&lc).await.unwrap();

        assert_eq!(root, served.block_merkle_tree_root());
    }

    #[tokio::test]
    async fn state_peers_all_rejecting_fails() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let lc = finalized_state(&header(&client, 62).await);
        let forging = TestClient::with_epoch_height(EPOCH_HEIGHT);
        forging.return_wrong_leaf(62, 61).await;

        let err = state_peers(vec![forging.clone(), forging])
            .root(&lc)
            .await
            .unwrap_err();

        assert!(format!("{err:#}").contains("expected 62"), "{err:#}");
    }

    /// A failed peer is asked last from then on, and a later answer does not promote it.
    #[tokio::test]
    async fn state_peers_failed_peer_moves_back_ok() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let lc = finalized_state(&header(&client, 62).await);
        let forging = TestClient::with_epoch_height(EPOCH_HEIGHT);
        forging.return_wrong_leaf(62, 61).await;
        let peers = state_peers(vec![forging, client.clone(), client]);

        peers.root(&lc).await.unwrap();
        assert_eq!(*peers.order.lock(), [1, 2, 0]);
        peers.root(&lc).await.unwrap();

        assert_eq!(*peers.order.lock(), [1, 2, 0]);
    }

    /// A proof that fails verification is rejected and the next state peer is asked.
    #[tokio::test]
    async fn state_peers_proven_header_skip_invalid_proof_ok() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let root = header(&client, 62).await.block_merkle_tree_root();
        let other_chain = TestClient::with_epoch_height(EPOCH_HEIGHT);
        other_chain.leaf(62).await;
        other_chain.return_wrong_leaf(15, 14).await;

        let served = state_peers(vec![other_chain, client])
            .proven_header(62, 15, root)
            .await
            .unwrap();

        assert_eq!(served.height(), 15);
    }

    /// A valid proof from the root's chain for another height is rejected on the height.
    #[tokio::test]
    async fn state_peers_proven_header_wrong_height_fails() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let root = header(&client, 62).await.block_merkle_tree_root();
        client.return_wrong_leaf(15, 14).await;

        let err = state_peers(vec![client])
            .proven_header(62, 15, root)
            .await
            .unwrap_err();

        assert!(format!("{err:#}").contains("expected 15"), "{err:#}");
    }

    /// Without request-response, the state peers answer at once, the preceding root is split off
    /// and the roots come back ascending.
    #[tokio::test(start_paused = true)]
    async fn fetch_headers_ok() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let lc = finalized_state(&header(&client, 62).await);
        let peers = state_peers(vec![client]);
        let sources = Sources::new(None::<&Silent>, &peers, HTTP_DELAY);
        let started = Instant::now();

        let (preceding, roots) = fetch_headers(&sources, &lc, Some(5), vec![15, 25, 35, 45, 55])
            .await
            .unwrap();

        let heights: Vec<_> = roots.iter().map(Header::height).collect();
        assert_eq!(heights, [15, 25, 35, 45, 55]);
        assert_eq!(preceding.map(|root| root.height()), Some(5));
        assert_eq!(started.elapsed(), Duration::ZERO);
    }

    /// With request-response present and silent, the state peers answer every request of the
    /// fetch after one `http_delay`, not one per request.
    #[tokio::test(start_paused = true)]
    async fn fetch_headers_silent_requests_wait_one_delay_ok() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let lc = finalized_state(&header(&client, 62).await);
        let peers = state_peers(vec![client]);
        let sources = Sources::new(Some(&Silent), &peers, HTTP_DELAY);
        let started = Instant::now();

        let (preceding, roots) = fetch_headers(&sources, &lc, Some(5), vec![15, 25, 35, 45, 55])
            .await
            .unwrap();

        assert_eq!(roots.len(), 5);
        assert!(preceding.is_some());
        assert_eq!(started.elapsed(), HTTP_DELAY);
    }

    /// The timeout error of a request names the request and the last error of the state peers.
    #[tokio::test(start_paused = true)]
    async fn fetch_headers_timeout_names_the_state_peer_error() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let lc = finalized_state(&header(&client, 62).await);
        let forging = TestClient::with_epoch_height(EPOCH_HEIGHT);
        forging.return_wrong_leaf(62, 61).await;
        let peers = state_peers(vec![forging]);
        let sources = Sources::new(Some(&Silent), &peers, HTTP_DELAY);
        let started = Instant::now();

        let err = fetch_headers(&sources, &lc, None, vec![15])
            .await
            .unwrap_err();

        let message = format!("{err:#}");
        assert!(
            message.contains("the LC contract header at height 62"),
            "{message}"
        );
        assert!(message.contains("expected 62"), "{message}");
        assert_eq!(started.elapsed(), REQUEST_TIMEOUT);
    }

    /// The HTTP delay is at most a third of the request bound.
    #[test]
    fn http_delay_is_at_most_a_third_of_the_request_bound() {
        let chain_config = ChainConfig {
            stake_table_contract: Some(Default::default()),
            ..Default::default()
        };

        let delay = |http_delay| {
            LcContractSeed::<NoStorage>::new(&chain_config, vec![], http_delay, &NoMetrics)
                .unwrap()
                .http_delay
        };

        assert_eq!(delay(Duration::from_secs(3600)), REQUEST_TIMEOUT / 3);
        assert_eq!(delay(Duration::from_secs(2)), Duration::from_secs(2));
    }

    /// A header at `F` whose block commitment root is not the LC contract's is rejected.
    #[tokio::test(start_paused = true)]
    async fn fetch_headers_forged_header_fails() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let mut lc = finalized_state(&header(&client, 62).await);
        lc.block_comm_root = finalized_state(&header(&client, 61).await).block_comm_root;
        let peers = state_peers(vec![client]);
        let sources = Sources::new(None::<&Silent>, &peers, HTTP_DELAY);

        let err = fetch_headers(&sources, &lc, Some(5), vec![15])
            .await
            .unwrap_err();

        assert!(
            format!("{err:#}").contains("block commitment root"),
            "{err:#}"
        );
    }
}
