//! Seeds the post-start epoch catchup from the LC contract.

use std::{
    pin::pin,
    sync::{Arc, OnceLock, Weak},
    time::Duration,
};

use anyhow::{Context, anyhow, bail, ensure};
use espresso_types::{
    Header, LcContractFinalizedState, LcContractReader, SeqTypes, traits::SequencerPersistence,
};
use futures::{
    FutureExt, StreamExt, TryStreamExt,
    future::{Either, select},
    stream,
};
use hotshot_query_service::node::BlockId;
use hotshot_types::{
    data::EpochNumber,
    epoch_membership::{CatchupSeedFn, EpochMembershipCoordinator},
};
use light_client::{
    client::{Client, QueryServiceClient},
    consensus::header::HeaderProof,
};
use url::Url;

use crate::request_response::{
    RequestResponseProtocol,
    request::{Request, Response},
};

/// Most requests of one seed in flight; peers answer 4 requests per key at a time.
const MAX_CONCURRENT_REQUESTS: usize = 3;

/// Bound on the peer requests of one seed: with no answering peer the attempt fails within it.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// The LC-contract seed of the catchup, behind `CatchupSeedFn`; steps in
/// `doc/agents/architecture.md`.
///
/// Trust: `F` and `blockCommRoot` come from the SNARK-verified LC contract. A peer's header at `F`
/// is accepted only with that `block_comm_root`; every installed table comes from a header proven
/// against its block Merkle root.
///
/// Invariants:
/// - I1: every header passed to `install_epoch_roots` is proven against the block Merkle root of a
///   header at `F` that `verify_header` accepted.
/// - I2: every proven height is below `F`.
/// - I3: only epochs above `start` are installed; eviction is `install_epoch_roots`' own.
/// - I7: every table the seed installs has a block reward.
pub struct LcContractSeed<P: SequencerPersistence> {
    lc_contract: LcContractReader,
    sources: FallbackSeedSource<RequestResponseProtocol<P>, StatePeerSources<QueryServiceClient>>,
}

/// Late-bound request-response protocol of a seed.
pub(crate) type RequestsCell<P> = Arc<OnceLock<Weak<RequestResponseProtocol<P>>>>;

impl<P: SequencerPersistence> LcContractSeed<P> {
    /// A seed over `lc_contract`. `state_peers` answer over HTTP after `http_delay`; none, no HTTP.
    pub fn new(lc_contract: LcContractReader, state_peers: Vec<Url>, http_delay: Duration) -> Self {
        let fallback = (!state_peers.is_empty()).then(|| {
            StatePeerSources(
                state_peers
                    .into_iter()
                    .map(QueryServiceClient::new)
                    .collect(),
            )
        });
        Self {
            lc_contract,
            sources: FallbackSeedSource {
                primary: Arc::default(),
                fallback,
                http_delay,
            },
        }
    }

    /// The cell to set with the request-response protocol once it exists.
    pub fn requests_cell(&self) -> RequestsCell<P> {
        Arc::clone(&self.sources.primary)
    }

    /// One seed attempt for `catchup(target)` with held pair top `start`: the walk start, `start`
    /// when not needed, `E* + 1` when seeded. The L1 read, the peer requests and each install are
    /// bounded.
    pub async fn run(
        &self,
        target: EpochNumber,
        start: EpochNumber,
        coordinator: EpochMembershipCoordinator<SeqTypes>,
    ) -> anyhow::Result<EpochNumber> {
        let started = tokio::time::Instant::now();
        let lc = self.lc_contract.finalized_state().await?;
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
        match seed_decision(&lc, epoch_height, target, start) {
            SeedDecision::NotNeeded => {
                log("not needed");
                return Ok(start);
            },
            SeedDecision::Seed => {},
        }

        let SeedRoots { preceding, roots } = tokio::time::timeout(
            REQUEST_TIMEOUT,
            fetch_seed_roots(&self.sources, &lc, epoch_height, start),
        )
        .await
        .map_err(|_| {
            anyhow!(
                "seed requests for the LC contract header and proofs exceeded {REQUEST_TIMEOUT:?}"
            )
        })??;
        coordinator
            .membership()
            .install_epoch_roots(preceding, roots, &coordinator)
            .await?;

        log("seeded");
        Ok(seed_epoch + 1)
    }

    /// The hook for `EpochMembershipCoordinator::set_catchup_seed`.
    pub fn into_seed_fn(self) -> CatchupSeedFn<SeqTypes> {
        let seed = Arc::new(self);
        Arc::new(move |target, start, coordinator| {
            let seed = Arc::clone(&seed);
            async move { seed.run(target, start, coordinator).await }.boxed()
        })
    }
}

/// What a seed attempt does for `catchup(target)` with held pair top `start`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SeedDecision {
    /// The seed epoch is at most `start + 1`, or the seeded window cannot serve the target: the
    /// walk runs unchanged.
    NotNeeded,
    /// Install the window of the seed epoch.
    Seed,
}

/// `NotNeeded` if `seed_epoch <= start + 1` or `target < first_installed + 2`, else `Seed`. The DRB
/// of the target needs `ST(target-1)` and `ST(target-2)`, which the window holds from
/// `first_installed + 2`; below that the walk serves the target. Pure.
fn seed_decision(
    lc: &LcContractFinalizedState,
    epoch_height: u64,
    target: EpochNumber,
    start: EpochNumber,
) -> SeedDecision {
    let first_installed = lc.first_installed_epoch(epoch_height, start);
    if lc.seed_epoch(epoch_height) <= start + 1 || target < first_installed + 2 {
        SeedDecision::NotNeeded
    } else {
        SeedDecision::Seed
    }
}

/// The headers a seed installs and the root below them, proven against the block Merkle root of
/// the header at `F`.
#[cfg_attr(test, derive(Debug))]
struct SeedRoots {
    /// `lc.preceding_root_height`: the block time source of the lowest installed reward.
    preceding: Option<Header>,
    /// `lc.proven_root_heights`, ascending.
    roots: Vec<Header>,
}

/// The `SeedRoots` of `lc` above `start`, each proven against the header at `F` that
/// `lc.verify_header` accepted.
async fn fetch_seed_roots(
    source: &impl LcContractSeedSource,
    lc: &LcContractFinalizedState,
    epoch_height: u64,
    start: EpochNumber,
) -> anyhow::Result<SeedRoots> {
    let checked_lc = lc.clone();
    let lc_header = source
        .header_at(lc.height, move |header: &Header| {
            checked_lc.verify_header(header)
        })
        .await?;
    let preceding = lc.preceding_root_height(epoch_height, start);
    let heights = preceding
        .into_iter()
        .chain(lc.proven_root_heights(epoch_height, start));
    let mut headers: Vec<Header> = stream::iter(heights)
        .map(|height| {
            let lc_header = lc_header.clone();
            source.proven_header(lc.height, height, move |proof| {
                verify_header_proof(&lc_header, proof, height)
            })
        })
        .buffered(MAX_CONCURRENT_REQUESTS)
        .try_collect()
        .await?;
    let preceding = preceding.map(|_| headers.remove(0));
    Ok(SeedRoots {
        preceding,
        roots: headers,
    })
}

/// The header at `expected_height`, if `proof` holds against the block Merkle root of
/// `root_header`. `HeaderProof::verify` does not check the height; this does. Pure.
fn verify_header_proof(
    root_header: &Header,
    proof: HeaderProof,
    expected_height: u64,
) -> anyhow::Result<Header> {
    let header = proof.verify(root_header.block_merkle_tree_root())?;
    ensure!(
        header.height() == expected_height,
        "header proof is for height {}, expected {expected_height}",
        header.height()
    );
    Ok(header)
}

/// Where the seed reads headers; every answer goes through the caller's `check`. The returned
/// futures do nothing until polled.
pub(crate) trait LcContractSeedSource: Send + Sync {
    /// The header at `height` from the first answer `check` accepts.
    fn header_at(
        &self,
        height: u64,
        check: impl Fn(&Header) -> anyhow::Result<()> + Clone + Send + Sync + 'static,
    ) -> impl Future<Output = anyhow::Result<Header>> + Send;

    /// The header `check` returns for the first `(root, height)` header proof it accepts.
    fn proven_header(
        &self,
        root: u64,
        height: u64,
        check: impl Fn(HeaderProof) -> anyhow::Result<Header> + Clone + Send + Sync + 'static,
    ) -> impl Future<Output = anyhow::Result<Header>> + Send;
}

/// Cliquenet: a rejected answer is skipped and the request keeps waiting (`request_indefinitely`).
impl<P: SequencerPersistence> LcContractSeedSource for RequestResponseProtocol<P> {
    async fn header_at(
        &self,
        height: u64,
        check: impl Fn(&Header) -> anyhow::Result<()> + Clone + Send + Sync + 'static,
    ) -> anyhow::Result<Header> {
        Ok(self
            .request_indefinitely(
                Request::Leaf(height),
                move |_: &Request, response: Response| {
                    let check = check.clone();
                    async move {
                        let Response::Leaf(leaves) = response else {
                            bail!("expected leaf response");
                        };
                        let header = leaves
                            .first()
                            .context("empty leaf chain")?
                            .block_header()
                            .clone();
                        check(&header)?;
                        Ok(header)
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
        root: u64,
        height: u64,
        check: impl Fn(HeaderProof) -> anyhow::Result<Header> + Clone + Send + Sync + 'static,
    ) -> anyhow::Result<Header> {
        Ok(self
            .request_indefinitely(
                Request::HeaderProof { root, height },
                move |_: &Request, response: Response| {
                    let check = check.clone();
                    async move {
                        let Response::HeaderProof(proof) = response else {
                            bail!("expected header proof response");
                        };
                        check(*proof)
                    }
                    .map(|result| {
                        result.inspect_err(|err| tracing::debug!("header proof rejected: {err:#}"))
                    })
                },
            )
            .await?)
    }
}

/// The state peers, asked in order for each request: the first answer `check` accepts wins; an error
/// or a rejection moves on to the next peer, and all failing returns the last error. Spawns nothing.
pub(crate) struct StatePeerSources<S>(Vec<S>);

impl<S: LcContractSeedSource> LcContractSeedSource for StatePeerSources<S> {
    async fn header_at(
        &self,
        height: u64,
        check: impl Fn(&Header) -> anyhow::Result<()> + Clone + Send + Sync + 'static,
    ) -> anyhow::Result<Header> {
        let mut last_err = anyhow!("no state peers");
        for peer in &self.0 {
            match peer.header_at(height, check.clone()).await {
                Ok(header) => return Ok(header),
                Err(err) => {
                    tracing::debug!("state peer header failed: {err:#}");
                    last_err = err;
                },
            }
        }
        Err(last_err)
    }

    async fn proven_header(
        &self,
        root: u64,
        height: u64,
        check: impl Fn(HeaderProof) -> anyhow::Result<Header> + Clone + Send + Sync + 'static,
    ) -> anyhow::Result<Header> {
        let mut last_err = anyhow!("no state peers");
        for peer in &self.0 {
            match peer.proven_header(root, height, check.clone()).await {
                Ok(header) => return Ok(header),
                Err(err) => {
                    tracing::debug!("state peer header proof failed: {err:#}");
                    last_err = err;
                },
            }
        }
        Err(last_err)
    }
}

/// Any light-client `Client`: one state peer.
impl<C: Client> LcContractSeedSource for C {
    async fn header_at(
        &self,
        height: u64,
        check: impl Fn(&Header) -> anyhow::Result<()> + Clone + Send + Sync + 'static,
    ) -> anyhow::Result<Header> {
        let leaves = self
            .get_leaves_in_range(height as usize, height as usize + 1)
            .await?;
        let header = leaves
            .into_iter()
            .next()
            .context("empty leaf range")?
            .header()
            .clone();
        check(&header)?;
        Ok(header)
    }

    async fn proven_header(
        &self,
        root: u64,
        height: u64,
        check: impl Fn(HeaderProof) -> anyhow::Result<Header> + Clone + Send + Sync + 'static,
    ) -> anyhow::Result<Header> {
        let proof = self
            .header_proof(root, BlockId::Number(height as usize))
            .await?;
        check(proof)
    }
}

/// Request-response first; the fallback starts when the primary errs (protocol not set) or after
/// `http_delay`, whichever is first. The first accepted answer wins; an error from one side waits
/// for the other.
///
/// Invariants:
/// - I4: a source returns a value only after the caller's check accepted it.
/// - I5: the fallback starts no earlier than `http_delay` after the primary unless the primary has
///   erred; without a fallback no HTTP request is made.
/// - I6: no task outlives a call; dropping its future drops both sides.
pub(crate) struct FallbackSeedSource<Primary, Fallback> {
    /// Weak, so the seed owns no path to the coordinator that the protocol holds.
    primary: Arc<OnceLock<Weak<Primary>>>,
    /// `None` without state peers: requests run on the primary alone, with no `race`.
    fallback: Option<Fallback>,
    http_delay: Duration,
}

/// The first `Ok` of `primary` and `fallback`, the latter polled once `primary` errs or `http_delay`
/// passes (`tokio::select!`, then `futures::future::select`). Spawns nothing; the last error wins.
async fn race<T>(
    primary: impl Future<Output = anyhow::Result<T>> + Send,
    fallback: impl Future<Output = anyhow::Result<T>> + Send,
    http_delay: Duration,
) -> anyhow::Result<T> {
    let mut primary = pin!(primary);
    tokio::select! {
        result = primary.as_mut() => match result {
            Ok(value) => return Ok(value),
            Err(err) => {
                tracing::debug!("primary seed source failed: {err:#}");
                return fallback.await;
            },
        },
        () = tokio::time::sleep(http_delay) => {
            tracing::debug!(?http_delay, "starting the HTTP side of a seed request");
        },
    }
    match select(primary, pin!(fallback)).await {
        Either::Left((Ok(value), _)) | Either::Right((Ok(value), _)) => Ok(value),
        Either::Left((Err(err), rest)) => {
            tracing::debug!("primary seed source failed: {err:#}");
            rest.await
        },
        Either::Right((Err(err), rest)) => {
            tracing::debug!("fallback seed source failed: {err:#}");
            rest.await
        },
    }
}

impl<Primary, Fallback> FallbackSeedSource<Primary, Fallback> {
    /// The primary, or an error while its cell is unset or its owner is gone.
    fn primary(&self) -> anyhow::Result<Arc<Primary>> {
        self.primary
            .get()
            .and_then(Weak::upgrade)
            .context("the request-response protocol is not started")
    }
}

/// An unset primary cell is an error of the primary side (`the request-response protocol is not started`).
impl<Primary, Fallback> LcContractSeedSource for FallbackSeedSource<Primary, Fallback>
where
    Primary: LcContractSeedSource,
    Fallback: LcContractSeedSource,
{
    async fn header_at(
        &self,
        height: u64,
        check: impl Fn(&Header) -> anyhow::Result<()> + Clone + Send + Sync + 'static,
    ) -> anyhow::Result<Header> {
        let primary = async {
            let primary = self.primary()?;
            primary.header_at(height, check.clone()).await
        };
        match &self.fallback {
            None => primary.await,
            Some(fallback) => {
                race(
                    primary,
                    fallback.header_at(height, check.clone()),
                    self.http_delay,
                )
                .await
            },
        }
    }

    async fn proven_header(
        &self,
        root: u64,
        height: u64,
        check: impl Fn(HeaderProof) -> anyhow::Result<Header> + Clone + Send + Sync + 'static,
    ) -> anyhow::Result<Header> {
        let primary = async {
            let primary = self.primary()?;
            primary.proven_header(root, height, check.clone()).await
        };
        match &self.fallback {
            None => primary.await,
            Some(fallback) => {
                race(
                    primary,
                    fallback.proven_header(root, height, check.clone()),
                    self.http_delay,
                )
                .await
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use hotshot_types::{data::ViewNumber, traits::block_contents::BlockHeader};
    use light_client::testing::TestClient;
    use rstest::rstest;
    use tokio::time::timeout;

    use super::*;

    const EPOCH_HEIGHT: u64 = 10;
    const HTTP_DELAY: Duration = Duration::from_secs(5);

    fn epoch(epoch: u64) -> EpochNumber {
        EpochNumber::new(epoch)
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

    #[tokio::test]
    async fn verify_header_cases_ok() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let lc = finalized_state(&header(&client, 62).await);

        lc.verify_header(&header(&client, 62).await).unwrap();
        lc.verify_header(&header(&client, 61).await).unwrap_err();

        let mut forged = header(&client, 62).await;
        forged
            .block_merkle_tree_root_mut()
            .clone_from(&header(&client, 61).await.block_merkle_tree_root());
        lc.verify_header(&forged).unwrap_err();
    }

    #[tokio::test]
    async fn fetch_seed_roots_ok() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let lc = finalized_state(&header(&client, 62).await);

        let roots = fetch_seed_roots(&client, &lc, EPOCH_HEIGHT, epoch(2))
            .await
            .unwrap();

        let heights: Vec<_> = roots.roots.iter().map(Header::height).collect();
        assert_eq!(heights, [15, 25, 35, 45, 55]);
        assert_eq!(roots.preceding.map(|root| root.height()), Some(5));
    }

    #[tokio::test]
    async fn fetch_seed_roots_forged_header_fails() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let lc = finalized_state(&header(&client, 62).await);

        let err = fetch_seed_roots(
            &ForgingSource {
                client,
                forging: true,
            },
            &lc,
            EPOCH_HEIGHT,
            epoch(2),
        )
        .await
        .unwrap_err();

        assert!(
            format!("{err:#}").contains("block commitment root"),
            "{err:#}"
        );
    }

    #[tokio::test]
    async fn header_proof_of_another_height_fails() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let root = header(&client, 7).await;
        let proof = client.header_proof(7, BlockId::Number(3)).await.unwrap();

        verify_header_proof(&root, proof.clone(), 3).unwrap();
        verify_header_proof(&root, proof, 4).unwrap_err();
    }

    /// What a `ScriptedSource` does with a request.
    #[derive(Clone, Copy)]
    enum Script {
        /// Never answers.
        Pending,
        /// Answers with its header after the delay.
        AnswerAfter(Duration),
        /// Errors after the delay.
        FailAfter(Duration),
    }

    /// Answers every request with `header` as scripted; sets `dropped` when a request future it
    /// returned is dropped.
    struct ScriptedSource {
        script: Script,
        header: Header,
        dropped: Arc<AtomicBool>,
    }

    struct DropFlag(Arc<AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    impl ScriptedSource {
        async fn new(script: Script) -> Self {
            let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
            Self {
                script,
                header: header(&client, 7).await,
                dropped: Arc::default(),
            }
        }

        fn request(&self) -> impl Future<Output = anyhow::Result<Header>> + Send + use<> {
            let flag = DropFlag(Arc::clone(&self.dropped));
            let (script, header) = (self.script, self.header.clone());
            async move {
                let _flag = flag;
                match script {
                    Script::Pending => std::future::pending().await,
                    Script::AnswerAfter(delay) => {
                        tokio::time::sleep(delay).await;
                        Ok(header)
                    },
                    Script::FailAfter(delay) => {
                        tokio::time::sleep(delay).await;
                        bail!("scripted failure")
                    },
                }
            }
        }

        fn was_dropped(&self) -> bool {
            self.dropped.load(Ordering::SeqCst)
        }
    }

    impl LcContractSeedSource for ScriptedSource {
        fn header_at(
            &self,
            _height: u64,
            _check: impl Fn(&Header) -> anyhow::Result<()> + Clone + Send + Sync + 'static,
        ) -> impl Future<Output = anyhow::Result<Header>> + Send {
            self.request()
        }

        fn proven_header(
            &self,
            _root: u64,
            _height: u64,
            _check: impl Fn(HeaderProof) -> anyhow::Result<Header> + Clone + Send + Sync + 'static,
        ) -> impl Future<Output = anyhow::Result<Header>> + Send {
            self.request()
        }
    }

    /// Serves the headers of `client`; with `forging`, with a changed block Merkle root where it is
    /// asked for one.
    struct ForgingSource {
        client: TestClient,
        forging: bool,
    }

    impl LcContractSeedSource for ForgingSource {
        async fn header_at(
            &self,
            height: u64,
            check: impl Fn(&Header) -> anyhow::Result<()> + Clone + Send + Sync + 'static,
        ) -> anyhow::Result<Header> {
            let mut served = self.client.header_at(height, accept).await?;
            if self.forging {
                served
                    .block_merkle_tree_root_mut()
                    .clone_from(&header(&self.client, 61).await.block_merkle_tree_root());
            }
            check(&served)?;
            Ok(served)
        }

        async fn proven_header(
            &self,
            root: u64,
            height: u64,
            check: impl Fn(HeaderProof) -> anyhow::Result<Header> + Clone + Send + Sync + 'static,
        ) -> anyhow::Result<Header> {
            self.client.proven_header(root, height, check).await
        }
    }

    /// The next state peer is asked when an answer is rejected.
    #[tokio::test]
    async fn state_peers_skip_rejected_answer_ok() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let lc = finalized_state(&header(&client, 62).await);
        let peers = StatePeerSources(vec![
            ForgingSource {
                client: client.clone(),
                forging: true,
            },
            ForgingSource {
                client,
                forging: false,
            },
        ]);

        let served = peers
            .header_at(62, move |header: &Header| lc.verify_header(header))
            .await
            .unwrap();

        assert_eq!(served.height(), 62);
    }

    #[tokio::test]
    async fn state_peers_all_rejecting_fails() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let lc = finalized_state(&header(&client, 62).await);
        let peers = StatePeerSources(
            (0..2)
                .map(|_| ForgingSource {
                    client: client.clone(),
                    forging: true,
                })
                .collect(),
        );

        let err = peers
            .header_at(62, move |header: &Header| lc.verify_header(header))
            .await
            .unwrap_err();

        assert!(
            format!("{err:#}").contains("block commitment root"),
            "{err:#}"
        );
    }

    /// A `FallbackSeedSource` with its primary kept alive for the test.
    struct Sources<Primary, Fallback> {
        source: FallbackSeedSource<Primary, Fallback>,
        primary: Option<Arc<Primary>>,
    }

    impl<Primary, Fallback> std::ops::Deref for Sources<Primary, Fallback> {
        type Target = FallbackSeedSource<Primary, Fallback>;

        fn deref(&self) -> &Self::Target {
            &self.source
        }
    }

    fn sources<Primary, Fallback>(
        primary: Option<Primary>,
        fallback: Option<Fallback>,
    ) -> Sources<Primary, Fallback> {
        let primary = primary.map(Arc::new);
        let cell = OnceLock::new();
        if let Some(primary) = &primary {
            assert!(cell.set(Arc::downgrade(primary)).is_ok());
        }
        Sources {
            source: FallbackSeedSource {
                primary: Arc::new(cell),
                fallback,
                http_delay: HTTP_DELAY,
            },
            primary,
        }
    }

    fn accept(_: &Header) -> anyhow::Result<()> {
        Ok(())
    }

    #[tokio::test(start_paused = true)]
    async fn fallback_after_delay_ok() {
        let source = sources(
            Some(ScriptedSource::new(Script::Pending).await),
            Some(TestClient::default()),
        );
        let started = tokio::time::Instant::now();

        let header = source.header_at(7, accept).await.unwrap();

        assert_eq!(header.height(), 7);
        assert_eq!(started.elapsed(), HTTP_DELAY);
    }

    /// The primary wins after the fallback has started: the fallback future is dropped.
    #[tokio::test(start_paused = true)]
    async fn fallback_drops_loser_ok() {
        let late = HTTP_DELAY + Duration::from_secs(1);
        let source = sources(
            Some(ScriptedSource::new(Script::AnswerAfter(late)).await),
            Some(ScriptedSource::new(Script::Pending).await),
        );

        source.header_at(7, accept).await.unwrap();

        assert!(source.fallback.as_ref().unwrap().was_dropped());
    }

    /// A dropped request drops both sides, as the seed timeout does.
    #[tokio::test(start_paused = true)]
    async fn fallback_drops_both_on_timeout_ok() {
        let source = sources(
            Some(ScriptedSource::new(Script::Pending).await),
            Some(ScriptedSource::new(Script::Pending).await),
        );

        timeout(HTTP_DELAY * 2, source.header_at(7, accept))
            .await
            .unwrap_err();

        assert!(source.primary.as_ref().unwrap().was_dropped());
        assert!(source.fallback.as_ref().unwrap().was_dropped());
    }

    #[tokio::test(start_paused = true)]
    async fn fallback_without_protocol_at_once_ok() {
        let source = sources(None::<TestClient>, Some(TestClient::default()));
        let started = tokio::time::Instant::now();

        source.header_at(7, accept).await.unwrap();

        assert_eq!(started.elapsed(), Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn fallback_prefers_primary_ok() {
        let source = sources(
            Some(TestClient::default()),
            Some(ScriptedSource::new(Script::Pending).await),
        );
        let started = tokio::time::Instant::now();

        source.header_at(7, accept).await.unwrap();

        assert!(started.elapsed() < HTTP_DELAY);
    }

    #[tokio::test(start_paused = true)]
    async fn fallback_without_sources_fails() {
        let source = sources(None::<TestClient>, None::<TestClient>);
        let started = tokio::time::Instant::now();

        source.header_at(7, accept).await.unwrap_err();

        assert_eq!(started.elapsed(), Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn fallback_both_fail_errs() {
        let source = sources(
            None::<TestClient>,
            Some(ScriptedSource::new(Script::FailAfter(Duration::ZERO)).await),
        );
        let started = tokio::time::Instant::now();

        let err = source.header_at(7, accept).await.unwrap_err();

        assert!(format!("{err:#}").contains("scripted failure"), "{err:#}");
        assert_eq!(started.elapsed(), Duration::ZERO);
    }

    /// An error of the fallback does not end the request while the primary is still asking.
    #[tokio::test(start_paused = true)]
    async fn fallback_error_waits_for_primary() {
        let late = HTTP_DELAY * 2;
        let source = sources(
            Some(ScriptedSource::new(Script::AnswerAfter(late)).await),
            Some(ScriptedSource::new(Script::FailAfter(Duration::ZERO)).await),
        );
        let started = tokio::time::Instant::now();

        source.header_at(7, accept).await.unwrap();

        assert_eq!(started.elapsed(), late);
    }

    #[tokio::test(start_paused = true)]
    async fn no_state_peers_uses_primary_only_ok() {
        let source = sources(Some(TestClient::default()), None::<ScriptedSource>);

        let header = source.header_at(7, accept).await.unwrap();

        assert_eq!(header.height(), 7);
    }

    #[rstest]
    // The seed epoch is at most `start + 1`.
    #[case(4, 9, 3, SeedDecision::NotNeeded)]
    #[case(3, 3, 2, SeedDecision::NotNeeded)]
    // The window is `ST(7) ..= ST(11)`: its two lowest tables lack `ST(target-2)`.
    #[case(10, 6, 2, SeedDecision::NotNeeded)]
    #[case(10, 7, 2, SeedDecision::NotNeeded)]
    #[case(10, 8, 2, SeedDecision::NotNeeded)]
    #[case(10, 9, 2, SeedDecision::Seed)]
    #[case(10, 10, 2, SeedDecision::Seed)]
    #[case(10, 15, 2, SeedDecision::Seed)]
    // The held pair is the seeded genesis pair: the window is `ST(3) ..= ST(7)`.
    #[case(6, 4, 2, SeedDecision::NotNeeded)]
    #[case(6, 5, 2, SeedDecision::Seed)]
    #[case(4, 4, 2, SeedDecision::NotNeeded)]
    #[case(4, 5, 2, SeedDecision::Seed)]
    fn seed_decision_cases(
        #[case] seed_epoch: u64,
        #[case] target: u64,
        #[case] start: u64,
        #[case] expected: SeedDecision,
    ) {
        // The last block of `seed_epoch`.
        let lc = LcContractFinalizedState {
            height: seed_epoch * EPOCH_HEIGHT,
            block_comm_root: Default::default(),
        };

        assert_eq!(
            seed_decision(&lc, EPOCH_HEIGHT, epoch(target), epoch(start)),
            expected
        );
    }
}
