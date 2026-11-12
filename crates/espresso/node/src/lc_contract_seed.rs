//! Seeds the post-start epoch catchup from the LC contract.

use std::{
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
use hotshot_types::{data::EpochNumber, epoch_membership::EpochMembershipCoordinator};
use light_client::client::{Client, QueryServiceClient};
use url::Url;

use crate::request_response::{
    RequestResponseProtocol,
    request::{Request, Response},
};

/// Most requests of one seed in flight; peers answer 4 requests per key at a time.
const MAX_CONCURRENT_REQUESTS: usize = 3;

/// Bound on the peer requests of one seed: with no answering peer the attempt fails within it.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// The LC-contract seed behind `CatchupSeedFn`; steps in `doc/agents/architecture.md`.
/// Every installed table is proven against the header at `F` that `verify_header` accepted.
#[derive(Clone)]
pub struct LcContractSeed<P: SequencerPersistence> {
    /// Set by `bind_requests` once consensus exists; weak, so the seed owns no path to the
    /// coordinator that the protocol holds.
    requests: Arc<OnceLock<Weak<RequestResponseProtocol<P>>>>,
    /// Asked in order after `http_delay`; none, no HTTP.
    state_peers: Vec<QueryServiceClient>,
    http_delay: Duration,
}

impl<P: SequencerPersistence> LcContractSeed<P> {
    /// A seed without a request-response protocol, `None` when `chain_config` names no stake
    /// table contract to read the LC contract from. Until `bind_requests`, `state_peers` answer
    /// at once, else after `http_delay`.
    pub fn new(
        chain_config: &ChainConfig,
        state_peers: Vec<Url>,
        http_delay: Duration,
    ) -> Option<Self> {
        chain_config.stake_table_contract.map(|_| Self {
            requests: Arc::default(),
            state_peers: state_peers
                .into_iter()
                .map(QueryServiceClient::new)
                .collect(),
            http_delay,
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
    /// when not needed, `E* + 1` when seeded. The L1 read, the peer requests and each install are
    /// bounded.
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
        if !lc.seeds_target(epoch_height, target, start) {
            log("not needed");
            return Ok(start);
        }

        let requests = self.requests.get().and_then(Weak::upgrade);
        let (preceding, roots) = tokio::time::timeout(
            REQUEST_TIMEOUT,
            fetch_headers(
                requests.as_ref(),
                &self.state_peers,
                self.http_delay,
                &lc,
                epoch_height,
                start,
            ),
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
}

/// The header at `F`, accepted by `lc.verify_header`, then the headers it proves: the preceding
/// root (`lc.preceding_root_height`, the block time source of the lowest installed reward) and
/// `lc.proven_root_heights`, ascending. `requests` is `None` while the protocol is unavailable.
async fn fetch_headers<P: SequencerPersistence, C: Client>(
    requests: Option<&Arc<RequestResponseProtocol<P>>>,
    state_peers: &[C],
    http_delay: Duration,
    lc: &LcContractFinalizedState,
    epoch_height: u64,
    start: EpochNumber,
) -> anyhow::Result<(Option<Header>, Vec<Header>)> {
    let lc_header = race(
        request_header(requests, lc),
        state_peers_header(state_peers, lc),
        http_delay,
    )
    .await?;

    let root = lc_header.block_merkle_tree_root();
    let preceding = lc.preceding_root_height(epoch_height, start);
    let heights = preceding
        .into_iter()
        .chain(lc.proven_root_heights(epoch_height, start));
    let mut headers: Vec<Header> = stream::iter(heights)
        .map(|height| {
            race(
                request_proven_header(requests, lc.height, height, root),
                state_peers_proven_header(state_peers, lc.height, height, root),
                http_delay,
            )
        })
        .buffered(MAX_CONCURRENT_REQUESTS)
        .try_collect()
        .await?;
    let preceding = preceding.map(|_| headers.remove(0));
    Ok((preceding, headers))
}

/// The header at `F` over request-response. A rejected answer is skipped and the request keeps
/// waiting (`request_indefinitely`).
async fn request_header<P: SequencerPersistence>(
    requests: Option<&Arc<RequestResponseProtocol<P>>>,
    lc: &LcContractFinalizedState,
) -> anyhow::Result<Header> {
    let requests = requests.context("the request-response protocol is not available")?;
    let checked_lc = lc.clone();
    Ok(requests
        .request_indefinitely(
            Request::Leaf(lc.height),
            move |_: &Request, response: Response| {
                let lc = checked_lc.clone();
                async move {
                    let Response::Leaf(leaves) = response else {
                        bail!("expected leaf response");
                    };
                    let header = leaves
                        .first()
                        .context("empty leaf chain")?
                        .block_header()
                        .clone();
                    lc.verify_header(&header)?;
                    Ok(header)
                }
                .map(|result| {
                    result.inspect_err(|err| tracing::debug!("seed header rejected: {err:#}"))
                })
            },
        )
        .await?)
}

/// The header at `height`, proven against the block Merkle root `root` of the header at
/// `root_height`, over request-response.
async fn request_proven_header<P: SequencerPersistence>(
    requests: Option<&Arc<RequestResponseProtocol<P>>>,
    root_height: u64,
    height: u64,
    root: BlockMerkleCommitment,
) -> anyhow::Result<Header> {
    let requests = requests.context("the request-response protocol is not available")?;
    Ok(requests
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

/// The header at `F` from the state peers, in order.
async fn state_peers_header<C: Client>(
    peers: &[C],
    lc: &LcContractFinalizedState,
) -> anyhow::Result<Header> {
    first_accepted(peers, |peer| async move {
        let header = peer
            .get_leaves_in_range(lc.height as usize, lc.height as usize + 1)
            .await?
            .into_iter()
            .next()
            .context("empty leaf range")?
            .header()
            .clone();
        lc.verify_header(&header)?;
        Ok(header)
    })
    .await
}

/// The header at `height`, proven against `root` of the header at `root_height`, from the state
/// peers, in order.
async fn state_peers_proven_header<C: Client>(
    peers: &[C],
    root_height: u64,
    height: u64,
    root: BlockMerkleCommitment,
) -> anyhow::Result<Header> {
    first_accepted(peers, |peer| async move {
        peer.header_proof(root_height, BlockId::Number(height as usize))
            .await?
            .verify_at_height(root, height)
    })
    .await
}

/// The first `Ok` of `ask` over `peers`, in order. An error or a rejection moves on to the next
/// peer; all failing returns the last error. Spawns nothing.
async fn first_accepted<'a, C, T, Fut>(
    peers: &'a [C],
    ask: impl Fn(&'a C) -> Fut,
) -> anyhow::Result<T>
where
    Fut: Future<Output = anyhow::Result<T>>,
{
    let mut last_err = anyhow!("no state peers");
    for peer in peers {
        match ask(peer).await {
            Ok(value) => return Ok(value),
            Err(err) => {
                tracing::debug!("state peer failed: {err:#}");
                last_err = err;
            },
        }
    }
    Err(last_err)
}

/// The first `Ok` of `primary` and `fallback`; `fallback` starts when `primary` errs or after
/// `http_delay`. An error of one side waits for the other; each discarded error is logged.
async fn race<T>(
    primary: impl Future<Output = anyhow::Result<T>>,
    fallback: impl Future<Output = anyhow::Result<T>>,
    http_delay: Duration,
) -> anyhow::Result<T> {
    let mut primary = pin!(primary);
    tokio::select! {
        result = primary.as_mut() => match result {
            Ok(value) => return Ok(value),
            Err(err) => {
                tracing::debug!("seed request-response side failed: {err:#}");
                return fallback.await;
            },
        },
        () = tokio::time::sleep(http_delay) => {},
    }
    match select(primary, pin!(fallback)).await {
        Either::Left((Ok(value), _)) | Either::Right((Ok(value), _)) => Ok(value),
        Either::Left((Err(err), rest)) => {
            tracing::debug!("seed request-response side failed: {err:#}");
            rest.await
        },
        Either::Right((Err(err), rest)) => {
            tracing::debug!("seed state peers side failed: {err:#}");
            rest.await
        },
    }
}

#[cfg(test)]
mod tests {
    use hotshot_types::{data::ViewNumber, traits::block_contents::BlockHeader};
    use light_client::testing::TestClient;

    use super::*;
    use crate::persistence::no_storage::NoStorage;

    const EPOCH_HEIGHT: u64 = 10;
    const HTTP_DELAY: Duration = Duration::from_secs(5);

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

    /// Answers `value` after `delay`.
    async fn answer_after(delay: Duration, value: u64) -> anyhow::Result<u64> {
        tokio::time::sleep(delay).await;
        Ok(value)
    }

    /// Fails after `delay`.
    async fn fail_after(delay: Duration) -> anyhow::Result<u64> {
        tokio::time::sleep(delay).await;
        bail!("scripted failure")
    }

    #[tokio::test(start_paused = true)]
    async fn fallback_after_delay_ok() {
        let started = tokio::time::Instant::now();

        let value = race(
            std::future::pending(),
            answer_after(Duration::ZERO, 1),
            HTTP_DELAY,
        )
        .await
        .unwrap();

        assert_eq!(value, 1);
        assert_eq!(started.elapsed(), HTTP_DELAY);
    }

    #[tokio::test(start_paused = true)]
    async fn fallback_prefers_primary_ok() {
        let started = tokio::time::Instant::now();

        let value = race(
            answer_after(Duration::ZERO, 1),
            std::future::pending(),
            HTTP_DELAY,
        )
        .await
        .unwrap();

        assert_eq!(value, 1);
        assert!(started.elapsed() < HTTP_DELAY);
    }

    /// A primary that errs starts the fallback at once.
    #[tokio::test(start_paused = true)]
    async fn fallback_after_primary_error_at_once_ok() {
        let started = tokio::time::Instant::now();

        let value = race(
            fail_after(Duration::ZERO),
            answer_after(Duration::ZERO, 1),
            HTTP_DELAY,
        )
        .await
        .unwrap();

        assert_eq!(value, 1);
        assert_eq!(started.elapsed(), Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn fallback_both_fail_errs() {
        let started = tokio::time::Instant::now();

        let err = race(
            fail_after(Duration::ZERO),
            fail_after(Duration::ZERO),
            HTTP_DELAY,
        )
        .await
        .unwrap_err();

        assert!(format!("{err:#}").contains("scripted failure"), "{err:#}");
        assert_eq!(started.elapsed(), Duration::ZERO);
    }

    /// An error of the fallback does not end the request while the primary is still asking.
    #[tokio::test(start_paused = true)]
    async fn fallback_error_waits_for_primary() {
        let late = HTTP_DELAY * 2;
        let started = tokio::time::Instant::now();

        let value = race(
            answer_after(late, 1),
            fail_after(Duration::ZERO),
            HTTP_DELAY,
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
        let late = HTTP_DELAY * 2;
        let started = tokio::time::Instant::now();

        let value = race(
            answer_after(late, 1),
            async {
                state_peers_header(&[] as &[TestClient], &lc).await?;
                Ok(2)
            },
            HTTP_DELAY,
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
        let lc = finalized_state(&header(&client, 62).await);
        let forging = TestClient::with_epoch_height(EPOCH_HEIGHT);
        forging.return_wrong_leaf(62, 61).await;

        let served = state_peers_header(&[forging, client], &lc).await.unwrap();

        assert_eq!(served.height(), 62);
    }

    #[tokio::test]
    async fn state_peers_all_rejecting_fails() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let lc = finalized_state(&header(&client, 62).await);
        let forging = TestClient::with_epoch_height(EPOCH_HEIGHT);
        forging.return_wrong_leaf(62, 61).await;

        let err = state_peers_header(&[forging.clone(), forging], &lc)
            .await
            .unwrap_err();

        assert!(format!("{err:#}").contains("expected 62"), "{err:#}");
    }

    /// A proof that fails verification is rejected and the next state peer is asked.
    #[tokio::test]
    async fn state_peers_proven_header_skip_invalid_proof_ok() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let root = header(&client, 62).await.block_merkle_tree_root();
        let other_chain = TestClient::with_epoch_height(EPOCH_HEIGHT);
        other_chain.leaf(62).await;
        other_chain.return_wrong_leaf(15, 14).await;

        let served = state_peers_proven_header(&[other_chain, client], 62, 15, root)
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

        let err = state_peers_proven_header(&[client], 62, 15, root)
            .await
            .unwrap_err();

        assert!(format!("{err:#}").contains("expected 15"), "{err:#}");
    }

    /// Without request-response, the preceding root is split off and the roots come back
    /// ascending.
    #[tokio::test]
    async fn fetch_headers_ok() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let lc = finalized_state(&header(&client, 62).await);

        let (preceding, roots) = fetch_headers::<NoStorage, _>(
            None,
            &[client],
            HTTP_DELAY,
            &lc,
            EPOCH_HEIGHT,
            EpochNumber::new(2),
        )
        .await
        .unwrap();

        let heights: Vec<_> = roots.iter().map(Header::height).collect();
        assert_eq!(heights, [15, 25, 35, 45, 55]);
        assert_eq!(preceding.map(|root| root.height()), Some(5));
    }

    /// A header at `F` whose block Merkle root is not the LC contract's is rejected.
    #[tokio::test]
    async fn fetch_headers_forged_header_fails() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let mut lc = finalized_state(&header(&client, 62).await);
        lc.block_comm_root = finalized_state(&header(&client, 61).await).block_comm_root;

        let err = fetch_headers::<NoStorage, _>(
            None,
            &[client],
            HTTP_DELAY,
            &lc,
            EPOCH_HEIGHT,
            EpochNumber::new(2),
        )
        .await
        .unwrap_err();

        assert!(
            format!("{err:#}").contains("block commitment root"),
            "{err:#}"
        );
    }
}
