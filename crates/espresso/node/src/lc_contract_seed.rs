//! Seeds the post-start epoch catchup from the LC contract.

use std::sync::Arc;

use espresso_types::{
    ChainConfig, Header, LcContractFinalizedState, SeqTypes, traits::StateCatchup,
};
use futures::{FutureExt, StreamExt, TryStreamExt, stream};
use hotshot_types::{
    data::EpochNumber,
    epoch_membership::EpochMembershipCoordinator,
    traits::metrics::{Counter, Metrics},
};

use crate::{
    lc_contract_headers::{fetch_anchor_root, fetch_proven_header},
    stake_table_backfill::StakeTableBackfill,
};

/// Most requests of one seed in flight; peers answer 4 requests per key at a time.
const MAX_CONCURRENT_REQUESTS: usize = 3;

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
    /// Asked to fill the epochs below the window after each install.
    backfill: StakeTableBackfill,
}

impl LcContractSeed {
    /// `None` when `chain_config` names no stake table contract to read the LC contract from.
    /// `backfill` is asked for a fill after each install; the caller runs it.
    pub fn new(
        chain_config: &ChainConfig,
        metrics: &dyn Metrics,
        backfill: StakeTableBackfill,
    ) -> Option<Self> {
        chain_config.stake_table_contract.map(|_| Self {
            seeded: metrics
                .subgroup("catchup".into())
                .create_counter("lc_contract_seeds".into(), None)
                .into(),
            backfill,
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
        self.install_roots(&coordinator, preceding, roots).await?;

        self.seeded.add(1);
        log("seeded");
        Ok(*window.epochs.end())
    }

    /// Installs `roots`, then asks for a fill whether or not the install failed: a failed batch
    /// keeps the tables it installed.
    async fn install_roots(
        &self,
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
        preceding: Option<Header>,
        roots: Vec<Header>,
    ) -> anyhow::Result<()> {
        let installed = coordinator
            .membership()
            .install_epoch_roots(preceding, roots, coordinator)
            .await;
        self.backfill.request_fill();
        Ok(installed?)
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
    let anchor = fetch_anchor_root(peers, lc_contract_state).await?;
    let heights = preceding_height.into_iter().chain(proven_heights);
    let mut headers: Vec<Header> = stream::iter(heights)
        .map(|height| fetch_proven_header(peers, lc_contract_state, anchor, height))
        .buffered(MAX_CONCURRENT_REQUESTS)
        .try_collect()
        .await?;
    let preceding = preceding_height.map(|_| headers.remove(0));
    Ok((preceding, headers))
}

#[cfg(test)]
mod tests {
    use alloy::primitives::Address;
    use espresso_types::{EpochCommittees, v0_3::Fetcher};
    use hotshot_example_types::storage_types::TestStorage;
    use hotshot_types::traits::metrics::NoMetrics;
    use light_client::testing::TestClient;

    use super::*;
    use crate::lc_contract_headers::testing::{EPOCH_HEIGHT, mock_with_headers};

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

    /// A failed install returns its error and asks for a fill.
    #[tokio::test]
    async fn install_roots_error_is_returned_and_requests_a_fill() {
        let backfill = StakeTableBackfill::new(&NoMetrics);
        let chain_config = ChainConfig {
            stake_table_contract: Some(Address::ZERO),
            ..Default::default()
        };
        let seed = LcContractSeed::new(&chain_config, &NoMetrics, backfill.clone()).unwrap();
        let committees = EpochCommittees::new(
            vec![],
            vec![],
            None,
            Fetcher::mock(),
            EPOCH_HEIGHT,
            EpochNumber::new(1),
        )
        .unwrap();
        let coordinator =
            EpochMembershipCoordinator::new(committees, EPOCH_HEIGHT, &TestStorage::default());
        let not_a_root = TestClient::with_epoch_height(EPOCH_HEIGHT)
            .leaf(3)
            .await
            .leaf()
            .block_header()
            .clone();
        assert!(!backfill.fill_requested());

        let result = seed
            .install_roots(&coordinator, None, vec![not_a_root])
            .await;

        assert!(result.is_err());
        assert!(backfill.fill_requested());
    }
}
