//! Seeds the post-start epoch catchup from the LC contract.

use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use anyhow::Context;
use espresso_types::{
    BackoffParams, ChainConfig, Header, LcContractFinalizedState, SeqTypes, traits::StateCatchup,
};
use futures::{FutureExt, StreamExt, TryStreamExt, stream};
use hotshot_types::{
    data::EpochNumber,
    epoch_membership::EpochMembershipCoordinator,
    traits::metrics::{Counter, Metrics},
};

/// Most requests of one seed in flight; peers answer 4 requests per key at a time.
const MAX_CONCURRENT_REQUESTS: usize = 3;

/// Retries of one seed request after its first attempt.
const SEED_REQUEST_RETRIES: u32 = 4;

/// The LC-contract seed of the catchup, behind `CatchupSeedFn`; steps in
/// `doc/agents/architecture.md`.
///
/// `F` and `blockCommRoot` come from the SNARK-verified LC contract. Every installed table comes
/// from a header proven against the block Merkle root of a header at `F` that `verify_header`
/// accepted. Proven heights are below `F`, installed epochs above `start`, and every table has a
/// block reward.
pub struct LcContractSeed {
    /// Seed attempts that installed a window.
    seeded: Arc<dyn Counter>,
}

impl LcContractSeed {
    /// `None` when `chain_config` names no stake table contract to read the LC contract from.
    pub fn new(chain_config: &ChainConfig, metrics: &dyn Metrics) -> Option<Self> {
        chain_config.stake_table_contract.map(|_| Self {
            seeded: metrics
                .subgroup("catchup".into())
                .create_counter("lc_contract_seeds".into(), None)
                .into(),
        })
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
    /// when not needed, the last installed epoch when seeded.
    async fn run(
        &self,
        target: EpochNumber,
        start: EpochNumber,
        coordinator: EpochMembershipCoordinator<SeqTypes>,
    ) -> anyhow::Result<EpochNumber> {
        let started = tokio::time::Instant::now();
        let fetcher = coordinator.membership().fetcher();
        let lc_contract_state = fetcher.lc_contract_finalized_state().await?;
        let epoch_height = *coordinator.epoch_height();
        let seed_epoch = lc_contract_state.seed_epoch(epoch_height);
        let log = |outcome: &str| {
            tracing::info!(
                target: "announce::catchup",
                %target,
                held_start = %start,
                lc_contract_height = lc_contract_state.height,
                %seed_epoch,
                outcome,
                duration_ms = started.elapsed().as_millis() as u64,
                "lc contract seed"
            );
        };
        let Some(window) = lc_contract_state.seed_window(
            epoch_height,
            coordinator.membership().first_epoch(),
            start,
            target,
        ) else {
            log("not needed");
            return Ok(start);
        };

        let heights = window.root_heights(epoch_height).collect();
        let (preceding, roots) = fetch_headers(
            fetcher.peers(),
            &lc_contract_state,
            window.preceding_root,
            heights,
        )
        .await?;
        coordinator
            .membership()
            .install_epoch_roots(preceding, roots, &coordinator)
            .await?;

        self.seeded.add(1);
        log("seeded");
        Ok(*window.epochs.end())
    }
}

/// The preceding root (the block time source of the lowest installed reward) split off, and the
/// headers at `proven_heights`, ascending, proven against the root of the header at `F`.
async fn fetch_headers(
    peers: &dyn StateCatchup,
    lc_contract_state: &LcContractFinalizedState,
    preceding_height: Option<u64>,
    proven_heights: Vec<u64>,
) -> anyhow::Result<(Option<Header>, Vec<Header>)> {
    let root = retry_request(|retry| {
        peers.try_fetch_lc_contract_block_merkle_root(retry, lc_contract_state)
    })
    .await
    .with_context(|| {
        format!(
            "fetching the LC contract header at height {}",
            lc_contract_state.height
        )
    })?;
    let heights = preceding_height.into_iter().chain(proven_heights);
    let mut headers: Vec<Header> = stream::iter(heights)
        .map(|height| {
            retry_request(move |retry| {
                peers.try_fetch_proven_header(retry, lc_contract_state.height, root, height)
            })
            .map(move |result| {
                result.with_context(|| format!("fetching the header proof at height {height}"))
            })
        })
        .buffered(MAX_CONCURRENT_REQUESTS)
        .try_collect()
        .await?;
    let preceding = preceding_height.map(|_| headers.remove(0));
    Ok((preceding, headers))
}

/// `request` with up to `SEED_REQUEST_RETRIES` retries; attempt `n` (0-based) is called with `n`.
async fn retry_request<T, Fut>(request: impl Fn(usize) -> Fut) -> anyhow::Result<T>
where
    Fut: Future<Output = anyhow::Result<T>>,
{
    let attempts = AtomicUsize::new(0);
    // Not `peers.backoff()`: disabled on `ParallelStateCatchup`, unreachable on request-response.
    BackoffParams::default()
        .retry_if(
            SEED_REQUEST_RETRIES,
            |_| true,
            || request(attempts.fetch_add(1, Ordering::SeqCst)),
        )
        .await
}

#[cfg(test)]
mod tests {
    use espresso_types::v0::mock::MockStateCatchup;
    use hotshot_types::{data::ViewNumber, traits::block_contents::BlockHeader};
    use light_client::testing::TestClient;
    use rstest::rstest;

    use super::*;

    const EPOCH_HEIGHT: u64 = 10;

    /// The mock serving the headers of `heights` and of `F = 62`, and the contract state at `F`.
    async fn mock_with_headers(heights: &[usize]) -> (MockStateCatchup, LcContractFinalizedState) {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let mut headers = Vec::new();
        for height in heights.iter().copied().chain([62]) {
            headers.push(client.leaf(height).await.leaf().block_header().clone());
        }
        let lc_contract_state = LcContractFinalizedState {
            height: 62,
            block_comm_root: headers
                .last()
                .unwrap()
                .get_light_client_state(ViewNumber::new(0))
                .unwrap()
                .block_comm_root,
        };
        (
            MockStateCatchup::default().with_headers(headers),
            lc_contract_state,
        )
    }

    /// The preceding root is split off and the roots come back in the order asked.
    #[tokio::test(start_paused = true)]
    async fn fetch_headers_ok() {
        let (peers, lc_contract_state) = mock_with_headers(&[5, 15, 25, 35, 45, 55]).await;

        let (preceding, roots) = fetch_headers(
            &peers,
            &lc_contract_state,
            Some(5),
            vec![15, 25, 35, 45, 55],
        )
        .await
        .unwrap();

        let heights: Vec<_> = roots.iter().map(Header::height).collect();
        assert_eq!(heights, [15, 25, 35, 45, 55]);
        assert_eq!(preceding.map(|root| root.height()), Some(5));
    }

    /// A request is retried up to `SEED_REQUEST_RETRIES` times, then the error surfaces.
    #[rstest]
    #[case::retried_ok(2, 3, true)]
    #[case::bounded_fails(usize::MAX, SEED_REQUEST_RETRIES as usize + 1, false)]
    #[tokio::test(start_paused = true)]
    async fn fetch_headers_retries_failed_request(
        #[case] failing_calls: usize,
        #[case] expected_calls: usize,
        #[case] expect_ok: bool,
    ) {
        let (peers, lc_contract_state) = mock_with_headers(&[15]).await;
        let peers = peers.with_failing_seed_calls(failing_calls);

        let result = fetch_headers(&peers, &lc_contract_state, None, vec![15]).await;

        assert_eq!(result.is_ok(), expect_ok, "{result:?}");
        if let Err(err) = result {
            assert!(
                format!("{err:#}").contains("the header proof at height 15"),
                "{err:#}"
            );
        }
        assert_eq!(peers.proven_header_calls(), expected_calls);
        let attempts: Vec<usize> = (0..expected_calls).collect();
        assert_eq!(peers.proven_header_retries(), attempts);
    }
}
