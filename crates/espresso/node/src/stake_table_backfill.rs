//! Stores the stake tables of the epochs a catchup seed skipped.

use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, bail};
use async_trait::async_trait;
use espresso_types::{
    BlockMerkleCommitment, EpochCommittees, Header, LcContractFinalizedState, SeqTypes,
    StakeTableStoreOutcome, preceding_root_height, traits::StateCatchup,
};
use hotshot_types::{
    data::EpochNumber,
    epoch_membership::EpochMembershipCoordinator,
    traits::metrics::{Counter, Gauge, Metrics},
    utils::root_block_in_epoch,
};
use tokio::{
    sync::Notify,
    time::{Instant, sleep, timeout},
};

use crate::lc_contract_headers::{fetch_anchor_root, fetch_proven_header};

/// Epochs whose root headers are fetched together.
const CHUNK_EPOCHS: usize = 20;

/// Wait after the first attempt of a fill that leaves retryable epochs; doubles per attempt.
const FIRST_BACKOFF: Duration = Duration::from_secs(5);

/// The longest wait between two attempts of a fill.
const MAX_BACKOFF: Duration = Duration::from_secs(600);

/// Bound on deriving and storing one table: the reward reads L1 once and the storage write is one
/// transaction, so a minute is far above a healthy run.
const EPOCH_DEADLINE: Duration = Duration::from_secs(60);

/// Stores the stake table of every epoch below the highest stored one that storage lacks, from
/// root headers proven against the LC contract. A fill starts at `run` and on each `request_fill`;
/// between fills the task is idle.
#[derive(Clone)]
pub struct StakeTableBackfill {
    /// Starts a fill; a request during a fill or a backoff starts the next attempt at once.
    requested: Arc<Notify>,
    /// Epochs without a stored table, including those with a hash mismatch.
    missing: Arc<dyn Gauge>,
    /// Tables stored.
    stored: Arc<dyn Counter>,
    /// Epochs not stored because the derived hash differs from the decided one.
    hash_mismatches: Arc<dyn Counter>,
    /// Epoch failures that a later attempt retries, counted per attempt.
    failed: Arc<dyn Counter>,
}

impl StakeTableBackfill {
    /// A backfill that registers its metrics under `catchup`.
    pub fn new(metrics: &dyn Metrics) -> Self {
        let catchup = metrics.subgroup("catchup".into());
        Self {
            requested: Arc::new(Notify::new()),
            missing: catchup
                .create_gauge("stake_table_backfill_missing_epochs".into(), None)
                .into(),
            stored: catchup
                .create_counter("stake_table_backfill_stored".into(), None)
                .into(),
            hash_mismatches: catchup
                .create_counter("stake_table_backfill_hash_mismatches".into(), None)
                .into(),
            failed: catchup
                .create_counter("stake_table_backfill_failed".into(), None)
                .into(),
        }
    }

    /// Asks the task for a fill; a request during a fill starts another one after it.
    pub fn request_fill(&self) {
        self.requested.notify_one();
    }

    /// Whether a request is pending; takes it.
    #[cfg(test)]
    pub(crate) fn fill_requested(&self) -> bool {
        use futures::FutureExt;
        self.requested.notified().now_or_never().is_some()
    }

    /// Fills at once, then on each request. Start it once the catchup provider is complete.
    pub async fn run(self, coordinator: EpochMembershipCoordinator<SeqTypes>) {
        let membership = coordinator.membership();
        let layout = EpochLayout {
            first_epoch: membership.first_epoch(),
            epoch_height: *coordinator.epoch_height(),
        };
        self.serve(membership, membership.fetcher().peers(), layout)
            .await
    }

    /// The loop of `run`. `mismatched` outlives the fills: those epochs are not retried.
    async fn serve(
        &self,
        store: &dyn StakeTableStore,
        peers: &dyn StateCatchup,
        layout: EpochLayout,
    ) -> ! {
        let mut mismatched = BTreeSet::new();
        loop {
            self.fill(store, peers, layout, &mut mismatched).await;
            self.requested.notified().await;
        }
    }

    /// Runs attempts until one leaves no retryable epoch, with a doubling backoff between them.
    async fn fill(
        &self,
        store: &dyn StakeTableStore,
        peers: &dyn StateCatchup,
        layout: EpochLayout,
        mismatched: &mut BTreeSet<EpochNumber>,
    ) {
        let started = Instant::now();
        let mut report = FillReport::default();
        let mut backoff = FIRST_BACKOFF;
        while self
            .attempt(store, peers, layout, mismatched, &mut report)
            .await
            == AttemptOutcome::Retry
        {
            tokio::select! {
                () = sleep(backoff) => {},
                () = self.requested.notified() => {},
            }
            backoff = (backoff * 2).min(MAX_BACKOFF);
        }
        tracing::info!(
            target: "announce::catchup",
            stored = report.stored,
            hash_mismatches = report.hash_mismatches,
            attempts = report.attempts,
            duration_ms = started.elapsed().as_millis() as u64,
            "stake table backfill"
        );
    }

    /// One run through the missing epochs; a failure is logged once, with counts.
    async fn attempt(
        &self,
        store: &dyn StakeTableStore,
        peers: &dyn StateCatchup,
        layout: EpochLayout,
        mismatched: &mut BTreeSet<EpochNumber>,
        report: &mut FillReport,
    ) -> AttemptOutcome {
        report.attempts += 1;
        let stored_before = report.stored;
        match self
            .store_missing(store, peers, layout, mismatched, report)
            .await
        {
            Ok(()) => AttemptOutcome::Complete,
            Err(err) => {
                tracing::warn!(
                    attempt = report.attempts,
                    stored = report.stored - stored_before,
                    "stake table backfill attempt incomplete: {err:#}"
                );
                AttemptOutcome::Retry
            },
        }
    }

    /// Stores the missing tables that the LC contract proves. Errors when an epoch or a read
    /// before the epochs failed; a retry can fix those.
    async fn store_missing(
        &self,
        store: &dyn StakeTableStore,
        peers: &dyn StateCatchup,
        layout: EpochLayout,
        mismatched: &mut BTreeSet<EpochNumber>,
        report: &mut FillReport,
    ) -> anyhow::Result<()> {
        let stored = store
            .stored_stake_epochs()
            .await
            .context("listing the epochs with a stored stake table")?;
        let missing = missing_epochs(layout, &stored);
        self.missing.set(missing.len());
        let mut remaining = missing.len();
        let retryable: Vec<_> = missing
            .into_iter()
            .filter(|epoch| !mismatched.contains(epoch))
            .collect();
        if retryable.is_empty() {
            return Ok(());
        }

        let lc_contract_state = store
            .lc_contract_finalized_state()
            .await
            .context("reading the LC contract state")?;
        let provable: Vec<_> = retryable
            .into_iter()
            .filter(|epoch| layout.root_heights(*epoch).next < lc_contract_state.height)
            .collect();
        if provable.is_empty() {
            return Ok(());
        }
        let anchor = fetch_anchor_root(peers, &lc_contract_state).await?;

        let mut failures = Vec::new();
        for chunk in provable.chunks(CHUNK_EPOCHS) {
            let headers =
                fetch_chunk_headers(peers, &lc_contract_state, anchor, layout, chunk).await;
            for &epoch in chunk {
                match store_epoch(store, layout, epoch, &headers).await {
                    Ok(StakeTableStoreOutcome::Stored) => {
                        tracing::debug!(%epoch, "stored stake table");
                        self.stored.add(1);
                        report.stored += 1;
                        remaining -= 1;
                        self.missing.set(remaining);
                    },
                    Ok(StakeTableStoreOutcome::HashMismatch { decided, derived }) => {
                        tracing::error!(
                            %epoch,
                            ?decided,
                            ?derived,
                            "stake table hash differs from the decided hash, epoch skipped"
                        );
                        mismatched.insert(epoch);
                        self.hash_mismatches.add(1);
                        report.hash_mismatches += 1;
                    },
                    Err(err) => {
                        self.failed.add(1);
                        failures.push((epoch, err));
                    },
                }
            }
        }
        match failures.first() {
            Some((epoch, err)) => bail!(
                "{} of {} epochs failed, the first is epoch {epoch}: {err:#}",
                failures.len(),
                provable.len()
            ),
            None => Ok(()),
        }
    }
}

/// Whether an attempt left epochs that a later attempt can store.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AttemptOutcome {
    /// Every provable missing epoch is stored or has a hash mismatch.
    Complete,
    /// A read or an epoch failed.
    Retry,
}

/// What the attempts of one fill did.
#[derive(Debug, Default, PartialEq)]
struct FillReport {
    /// Tables stored.
    stored: usize,
    /// Epochs found with a hash mismatch.
    hash_mismatches: usize,
    /// Attempts made.
    attempts: usize,
}

/// Where the backfill reads and writes the tables; the membership in production.
#[async_trait]
trait StakeTableStore: Sync {
    /// The epochs that have a stored stake table.
    async fn stored_stake_epochs(&self) -> anyhow::Result<BTreeSet<EpochNumber>>;

    /// The LC contract's finalized state.
    async fn lc_contract_finalized_state(&self) -> anyhow::Result<LcContractFinalizedState>;

    /// Stores the table of the epoch `root` defines; see `EpochCommittees::store_derived_stake_table`.
    async fn store_derived_stake_table(
        &self,
        root: &Header,
        previous_root: Option<&Header>,
        next_root: &Header,
    ) -> anyhow::Result<StakeTableStoreOutcome>;
}

#[async_trait]
impl StakeTableStore for EpochCommittees {
    async fn stored_stake_epochs(&self) -> anyhow::Result<BTreeSet<EpochNumber>> {
        self.stored_stake_epochs().await
    }

    async fn lc_contract_finalized_state(&self) -> anyhow::Result<LcContractFinalizedState> {
        self.fetcher().lc_contract_finalized_state().await
    }

    async fn store_derived_stake_table(
        &self,
        root: &Header,
        previous_root: Option<&Header>,
        next_root: &Header,
    ) -> anyhow::Result<StakeTableStoreOutcome> {
        Ok(self
            .store_derived_stake_table(root, previous_root, next_root)
            .await?)
    }
}

/// The epoch numbering of the chain.
#[derive(Clone, Copy, Debug)]
struct EpochLayout {
    /// The first epoch of the membership.
    first_epoch: EpochNumber,
    /// Blocks per epoch.
    epoch_height: u64,
}

impl EpochLayout {
    /// The heights of the epoch roots that the table of `epoch` reads.
    fn root_heights(&self, epoch: EpochNumber) -> EpochRootHeights {
        EpochRootHeights {
            root: root_block_in_epoch(*epoch - 2, self.epoch_height),
            next: root_block_in_epoch(*epoch - 1, self.epoch_height),
            previous: preceding_root_height(epoch, self.first_epoch, self.epoch_height),
        }
    }
}

/// The epoch roots behind the table of one epoch.
struct EpochRootHeights {
    /// The root that defines the table.
    root: u64,
    /// The root one epoch above, which decides the table's hash.
    next: u64,
    /// The root one epoch below `root`, when the block reward reads it.
    previous: Option<u64>,
}

/// The epochs from the first one an epoch root defines up to the highest in `stored` that are
/// not in `stored`, ascending. Pure.
fn missing_epochs(layout: EpochLayout, stored: &BTreeSet<EpochNumber>) -> Vec<EpochNumber> {
    let Some(highest) = stored.last() else {
        return Vec::new();
    };
    // Epoch 3 is the first table an epoch root defines; the first two epochs are genesis tables.
    let lowest = (*layout.first_epoch + 2).max(3);
    (lowest..=**highest)
        .map(EpochNumber::new)
        .filter(|epoch| !stored.contains(epoch))
        .collect()
}

/// The root headers of `chunk` by height, each fetched once and one at a time, so that the
/// requests of a backfill and a concurrent seed stay within the peers' limit per key. A header
/// that cannot be fetched fails only the epochs that need it.
async fn fetch_chunk_headers(
    peers: &dyn StateCatchup,
    lc_contract_state: &LcContractFinalizedState,
    anchor: BlockMerkleCommitment,
    layout: EpochLayout,
    chunk: &[EpochNumber],
) -> HashMap<u64, anyhow::Result<Header>> {
    let heights: BTreeSet<u64> = chunk
        .iter()
        .flat_map(|epoch| {
            let heights = layout.root_heights(*epoch);
            [Some(heights.root), Some(heights.next), heights.previous]
        })
        .flatten()
        .collect();
    let mut headers = HashMap::new();
    for height in heights {
        let header = fetch_proven_header(peers, lc_contract_state, anchor, height).await;
        headers.insert(height, header);
    }
    headers
}

/// Stores the table of `epoch` from its roots in `headers`, within `EPOCH_DEADLINE`.
async fn store_epoch(
    store: &dyn StakeTableStore,
    layout: EpochLayout,
    epoch: EpochNumber,
    headers: &HashMap<u64, anyhow::Result<Header>>,
) -> anyhow::Result<StakeTableStoreOutcome> {
    let header = |height: u64| match headers.get(&height) {
        Some(Ok(header)) => Ok(header),
        Some(Err(err)) => bail!("no root header at height {height}: {err:#}"),
        None => bail!("root header at height {height} was not fetched"),
    };
    let heights = layout.root_heights(epoch);
    let root = header(heights.root)?;
    let next_root = header(heights.next)?;
    let previous_root = heights.previous.map(header).transpose()?;
    timeout(
        EPOCH_DEADLINE,
        store.store_derived_stake_table(root, previous_root, next_root),
    )
    .await
    .with_context(|| format!("storing the table exceeded {EPOCH_DEADLINE:?}"))?
}

#[cfg(test)]
mod tests {
    use committable::Commitment;
    use hotshot_query_service::metrics::PrometheusMetrics;
    use hotshot_types::utils::epoch_from_block_number;
    use parking_lot::Mutex;
    use rstest::rstest;

    use super::*;
    use crate::lc_contract_headers::testing::{EPOCH_HEIGHT, mock_with_headers};

    fn layout() -> EpochLayout {
        EpochLayout {
            first_epoch: EpochNumber::new(1),
            epoch_height: EPOCH_HEIGHT,
        }
    }

    fn epochs(numbers: impl IntoIterator<Item = u64>) -> BTreeSet<EpochNumber> {
        numbers.into_iter().map(EpochNumber::new).collect()
    }

    /// Epochs below the highest stored one are missing from the first one a root defines.
    #[rstest]
    #[case::empty_storage(layout(), &[], &[])]
    #[case::complete(layout(), &[3, 4, 5, 6], &[])]
    #[case::gap(layout(), &[7], &[3, 4, 5, 6])]
    #[case::later_first_epoch(
        EpochLayout { first_epoch: EpochNumber::new(5), epoch_height: EPOCH_HEIGHT },
        &[10],
        &[7, 8, 9]
    )]
    fn missing_epochs_cases(
        #[case] layout: EpochLayout,
        #[case] stored: &[u64],
        #[case] missing: &[u64],
    ) {
        let stored = epochs(stored.iter().copied());

        let expected: Vec<_> = missing.iter().copied().map(EpochNumber::new).collect();
        assert_eq!(missing_epochs(layout, &stored), expected);
    }

    /// The root below a table's root is read from the sixth epoch on, after `first_epoch + 4`.
    #[rstest]
    #[case(5, None)]
    #[case(6, Some(25))]
    #[case(7, Some(35))]
    fn root_heights_cases(#[case] epoch: u64, #[case] previous: Option<u64>) {
        let heights = layout().root_heights(EpochNumber::new(epoch));

        assert_eq!(heights.root, 10 * (epoch - 2) - 5);
        assert_eq!(heights.next, 10 * (epoch - 1) - 5);
        assert_eq!(heights.previous, previous);
    }

    /// The store of a test: `stored` is storage and a store adds its epoch.
    struct MockStakeTableStore {
        stored: Mutex<BTreeSet<EpochNumber>>,
        /// `(epoch, previous root height, next root height)` of each store call.
        calls: Mutex<Vec<(u64, Option<u64>, u64)>>,
        /// When storage was listed, once per attempt that got that far.
        listings: Mutex<Vec<Instant>>,
        /// Epochs whose derived hash differs from the decided one.
        mismatched: BTreeSet<u64>,
        /// Epochs whose store fails.
        failing: BTreeSet<u64>,
        /// Epochs whose store never returns.
        hanging: BTreeSet<u64>,
        lc_contract_state: LcContractFinalizedState,
    }

    impl MockStakeTableStore {
        fn new(stored: &[u64], lc_contract_state: LcContractFinalizedState) -> Self {
            Self {
                stored: Mutex::new(epochs(stored.iter().copied())),
                calls: Mutex::default(),
                listings: Mutex::default(),
                mismatched: BTreeSet::new(),
                failing: BTreeSet::new(),
                hanging: BTreeSet::new(),
                lc_contract_state,
            }
        }

        fn calls_of(&self, epoch: u64) -> usize {
            self.calls.lock().iter().filter(|c| c.0 == epoch).count()
        }

        fn is_stored(&self, epochs: &[u64]) -> bool {
            let stored = self.stored.lock();
            epochs
                .iter()
                .all(|e| stored.contains(&EpochNumber::new(*e)))
        }
    }

    #[async_trait]
    impl StakeTableStore for MockStakeTableStore {
        async fn stored_stake_epochs(&self) -> anyhow::Result<BTreeSet<EpochNumber>> {
            self.listings.lock().push(Instant::now());
            Ok(self.stored.lock().clone())
        }

        async fn lc_contract_finalized_state(&self) -> anyhow::Result<LcContractFinalizedState> {
            Ok(self.lc_contract_state.clone())
        }

        async fn store_derived_stake_table(
            &self,
            root: &Header,
            previous_root: Option<&Header>,
            next_root: &Header,
        ) -> anyhow::Result<StakeTableStoreOutcome> {
            let epoch = epoch_from_block_number(root.height(), EPOCH_HEIGHT) + 2;
            self.calls
                .lock()
                .push((epoch, previous_root.map(Header::height), next_root.height()));
            if self.hanging.contains(&epoch) {
                std::future::pending::<()>().await;
            }
            anyhow::ensure!(!self.failing.contains(&epoch), "injected failure");
            if self.mismatched.contains(&epoch) {
                return Ok(StakeTableStoreOutcome::HashMismatch {
                    decided: Commitment::from_raw([1; 32]),
                    derived: Commitment::from_raw([2; 32]),
                });
            }
            self.stored.lock().insert(EpochNumber::new(epoch));
            Ok(StakeTableStoreOutcome::Stored)
        }
    }

    fn metric(metrics: &PrometheusMetrics, name: &str) -> usize {
        let catchup = metrics.get_subgroup(["catchup"]).unwrap();
        match name {
            "stake_table_backfill_missing_epochs" => catchup.get_gauge(name).unwrap().get(),
            _ => catchup.get_counter(name).unwrap().get(),
        }
    }

    /// Roots at heights 5 to 55 are provable below `F = 62`: epochs 3 to 7.
    async fn peers() -> (
        espresso_types::v0::mock::MockStateCatchup,
        LcContractFinalizedState,
    ) {
        mock_with_headers(&[5, 15, 25, 35, 45, 55]).await
    }

    /// Runs `backfill` until `test` ends; in paused time `test` steps through the waits.
    async fn serve_while(
        backfill: &StakeTableBackfill,
        store: &MockStakeTableStore,
        peers: &dyn StateCatchup,
        test: impl Future<Output = ()>,
    ) {
        tokio::select! {
            never = backfill.serve(store, peers, layout()) => never,
            () = test => {},
        }
    }

    /// Steps virtual time until `condition` holds.
    async fn wait_until(condition: impl Fn() -> bool) {
        timeout(Duration::from_secs(7200), async {
            while !condition() {
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("the condition did not hold within two virtual hours");
    }

    /// The startup fill stores the epochs below the highest stored one, each root fetched once,
    /// and a request after a seed installed a higher table fills the gap below it.
    #[tokio::test(start_paused = true)]
    async fn startup_fill_and_a_request_after_a_partial_install() {
        let (peers, lc_contract_state) = peers().await;
        let metrics = PrometheusMetrics::default();
        let backfill = StakeTableBackfill::new(&metrics);
        let store = MockStakeTableStore::new(&[4], lc_contract_state);

        serve_while(&backfill, &store, &peers, async {
            wait_until(|| store.is_stored(&[3])).await;
            assert_eq!(peers.proven_header_calls(), 2);

            store.stored.lock().insert(EpochNumber::new(7));
            backfill.request_fill();
            wait_until(|| store.is_stored(&[3, 4, 5, 6, 7])).await;
        })
        .await;

        let expected_calls = [(3, None, 15), (5, None, 35), (6, Some(25), 45)];
        assert_eq!(*store.calls.lock(), expected_calls);
        assert_eq!(metric(&metrics, "stake_table_backfill_stored"), 3);
        assert_eq!(metric(&metrics, "stake_table_backfill_missing_epochs"), 0);
    }

    /// A header that cannot be fetched fails only the epochs that need it: the others are stored
    /// in the same attempt, the rest after the backoff once the proof is served.
    #[tokio::test(start_paused = true)]
    async fn unavailable_proof_fails_only_dependent_epochs() {
        let (peers, lc_contract_state) = peers().await;
        peers.set_unavailable_heights([15]);
        let metrics = PrometheusMetrics::default();
        let backfill = StakeTableBackfill::new(&metrics);
        let store = MockStakeTableStore::new(&[7], lc_contract_state);

        serve_while(&backfill, &store, &peers, async {
            wait_until(|| store.is_stored(&[5, 6])).await;
            assert!(!store.is_stored(&[3]) && !store.is_stored(&[4]));
            assert_eq!(metric(&metrics, "stake_table_backfill_failed"), 2);
            assert_eq!(metric(&metrics, "stake_table_backfill_missing_epochs"), 2);

            peers.set_unavailable_heights([]);
            wait_until(|| store.is_stored(&[3, 4])).await;
        })
        .await;

        assert_eq!(store.listings.lock().len(), 2);
        assert_eq!(metric(&metrics, "stake_table_backfill_stored"), 4);
        assert_eq!(metric(&metrics, "stake_table_backfill_missing_epochs"), 0);
    }

    /// A failed anchor fetch ends the attempt before any epoch work; the fill retries.
    #[tokio::test(start_paused = true)]
    async fn anchor_failure_ends_the_attempt() {
        let (peers, lc_contract_state) = peers().await;
        let peers = peers.with_failing_anchor_calls(5);
        let backfill = StakeTableBackfill::new(&PrometheusMetrics::default());
        let store = MockStakeTableStore::new(&[4], lc_contract_state);

        serve_while(&backfill, &store, &peers, async {
            wait_until(|| peers.anchor_calls() == 5).await;
            assert!(store.calls.lock().is_empty());
            assert_eq!(peers.proven_header_calls(), 0);

            wait_until(|| store.is_stored(&[3])).await;
        })
        .await;

        assert_eq!(peers.anchor_calls(), 6);
    }

    /// After the last retryable epoch is stored, no request is issued however long the task runs.
    #[tokio::test(start_paused = true)]
    async fn idle_after_the_fill_completes() {
        let (peers, lc_contract_state) = peers().await;
        let backfill = StakeTableBackfill::new(&PrometheusMetrics::default());
        let store = MockStakeTableStore::new(&[6], lc_contract_state);

        serve_while(&backfill, &store, &peers, async {
            wait_until(|| store.is_stored(&[3, 4, 5])).await;
            let (listings, headers) = (store.listings.lock().len(), peers.proven_header_calls());

            sleep(MAX_BACKOFF * 3).await;

            assert_eq!(store.listings.lock().len(), listings);
            assert_eq!(peers.proven_header_calls(), headers);
            assert_eq!(peers.anchor_calls(), 1);
        })
        .await;
    }

    /// A mismatch is not retried, by the fill or by a later one, and stays in the gauge.
    #[tokio::test(start_paused = true)]
    async fn mismatch_is_not_retried() {
        let (peers, lc_contract_state) = peers().await;
        let metrics = PrometheusMetrics::default();
        let backfill = StakeTableBackfill::new(&metrics);
        let mut store = MockStakeTableStore::new(&[7], lc_contract_state);
        store.mismatched = BTreeSet::from([4]);

        serve_while(&backfill, &store, &peers, async {
            wait_until(|| store.is_stored(&[3, 5, 6])).await;
            sleep(MAX_BACKOFF * 3).await;
            backfill.request_fill();
            sleep(MAX_BACKOFF).await;

            assert_eq!(store.listings.lock().len(), 2);
        })
        .await;

        assert_eq!(store.calls_of(4), 1);
        assert_eq!(metric(&metrics, "stake_table_backfill_hash_mismatches"), 1);
        assert_eq!(metric(&metrics, "stake_table_backfill_missing_epochs"), 1);
        assert_eq!(metric(&metrics, "stake_table_backfill_failed"), 0);
    }

    /// The wait between attempts doubles from `FIRST_BACKOFF` up to `MAX_BACKOFF`.
    #[tokio::test(start_paused = true)]
    async fn backoff_doubles_and_caps() {
        let (peers, lc_contract_state) = peers().await;
        let backfill = StakeTableBackfill::new(&PrometheusMetrics::default());
        let mut store = MockStakeTableStore::new(&[4], lc_contract_state);
        store.failing = BTreeSet::from([3]);

        serve_while(&backfill, &store, &peers, async {
            wait_until(|| store.listings.lock().len() == 11).await;
        })
        .await;

        let listings = store.listings.lock();
        let waits: Vec<u64> = listings
            .windows(2)
            .map(|w| (w[1] - w[0]).as_secs())
            .collect();
        assert_eq!(waits, [5, 10, 20, 40, 80, 160, 320, 600, 600, 600]);
    }

    /// A request during a backoff starts the next attempt at once.
    #[tokio::test(start_paused = true)]
    async fn request_during_backoff_starts_the_next_attempt() {
        let (peers, lc_contract_state) = peers().await;
        let backfill = StakeTableBackfill::new(&PrometheusMetrics::default());
        let mut store = MockStakeTableStore::new(&[4], lc_contract_state);
        store.failing = BTreeSet::from([3]);

        serve_while(&backfill, &store, &peers, async {
            wait_until(|| store.listings.lock().len() == 2).await;
            sleep(Duration::from_secs(1)).await;
            backfill.request_fill();
            wait_until(|| store.listings.lock().len() == 3).await;
        })
        .await;

        let listings = store.listings.lock();
        assert!(listings[2] - listings[1] < Duration::from_secs(5));
    }

    /// A store that does not return fails its epoch at the deadline and not the others.
    #[tokio::test(start_paused = true)]
    async fn store_past_the_deadline_fails_only_its_epoch() {
        let (peers, lc_contract_state) = peers().await;
        let metrics = PrometheusMetrics::default();
        let backfill = StakeTableBackfill::new(&metrics);
        let mut store = MockStakeTableStore::new(&[6], lc_contract_state);
        store.hanging = BTreeSet::from([4]);

        serve_while(&backfill, &store, &peers, async {
            wait_until(|| store.is_stored(&[3, 5])).await;
            wait_until(|| store.listings.lock().len() == 2).await;
        })
        .await;

        assert!(!store.is_stored(&[4]));
        assert_eq!(metric(&metrics, "stake_table_backfill_failed"), 1);
    }
}
