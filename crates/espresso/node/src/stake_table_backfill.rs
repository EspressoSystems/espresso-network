//! Stores the stake tables of the epochs a catchup seed skipped.

use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use async_trait::async_trait;
use espresso_types::{
    EpochCommittees, Header, LcContractFinalizedState, SeqTypes, StakeTableStoreOutcome,
    preceding_root_height, traits::StateCatchup,
};
use hotshot_types::{
    data::EpochNumber,
    epoch_membership::EpochMembershipCoordinator,
    traits::metrics::{Counter, Gauge, Metrics},
    utils::root_block_in_epoch,
};
use tokio::{sync::Notify, time::Instant};

use crate::lc_contract_seed::{fetch_headers, retry_request};

/// Epochs whose root headers are fetched together.
const CHUNK_EPOCHS: usize = 20;

/// Wait before a pass runs again on its own while the last one left epochs missing.
const RERUN_INTERVAL: Duration = Duration::from_secs(600);

/// The long-lived task that stores the stake table of every epoch below the highest stored one
/// that storage lacks, from root headers proven against the LC contract. A seed requests a pass
/// after it installs tables; the task also runs one at start, which resumes an interrupted fill.
#[derive(Clone)]
pub struct StakeTableBackfill {
    /// Wakes the task for a pass.
    requested: Arc<Notify>,
    /// Epochs without a stored table after the last pass.
    missing: Arc<dyn Gauge>,
    /// Tables stored.
    stored: Arc<dyn Counter>,
    /// Epochs skipped because the derived hash differs from the decided one.
    hash_mismatches: Arc<dyn Counter>,
    /// Epochs given up on after their retries.
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

    /// Asks the task for a pass; one request that arrives during a pass runs after it.
    pub fn request_pass(&self) {
        self.requested.notify_one();
    }

    /// Runs a pass at once, then on each request and every `RERUN_INTERVAL` while epochs are
    /// missing. Start it once the catchup provider is complete.
    pub async fn run(self, coordinator: EpochMembershipCoordinator<SeqTypes>) {
        loop {
            let rerun = match self.pass(&coordinator).await {
                Ok(report) => report.missing_after > 0,
                Err(err) => {
                    tracing::error!("stake table backfill pass failed: {err:#}");
                    true
                },
            };
            tokio::select! {
                () = self.requested.notified() => {},
                () = rerun_after(rerun) => {},
            }
        }
    }

    /// One pass against the LC contract's current finalized state.
    async fn pass(
        &self,
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
    ) -> anyhow::Result<PassReport> {
        let membership = coordinator.membership();
        let fetcher = membership.fetcher();
        let lc_contract_state = fetcher.lc_contract_finalized_state().await?;
        let layout = EpochLayout {
            first_epoch: membership.first_epoch(),
            epoch_height: *coordinator.epoch_height(),
        };
        self.fill(membership, fetcher.peers(), &lc_contract_state, layout)
            .await
    }

    /// Stores the missing tables that `lc_contract_state` proves, ascending, in chunks.
    async fn fill(
        &self,
        store: &dyn StakeTableStore,
        peers: &dyn StateCatchup,
        lc_contract_state: &LcContractFinalizedState,
        layout: EpochLayout,
    ) -> anyhow::Result<PassReport> {
        let started = Instant::now();
        let stored = store
            .stored_epochs()
            .await
            .context("listing the epochs with a stored stake table")?;
        let missing = missing_epochs(layout, &stored, lc_contract_state);
        self.missing.set(missing.total);

        let mut report = PassReport {
            missing_after: missing.total,
            ..Default::default()
        };
        for chunk in missing.provable.chunks(CHUNK_EPOCHS) {
            self.fill_chunk(store, peers, lc_contract_state, layout, chunk, &mut report)
                .await;
        }
        self.missing.set(report.missing_after);

        if !missing.provable.is_empty() {
            tracing::info!(
                target: "announce::catchup",
                stored = report.stored,
                hash_mismatches = report.hash_mismatches,
                failed = report.failed,
                missing_after = report.missing_after,
                duration_ms = started.elapsed().as_millis() as u64,
                "stake table backfill"
            );
        }
        Ok(report)
    }

    /// Fetches the roots of `chunk` once and stores each of its epochs.
    async fn fill_chunk(
        &self,
        store: &dyn StakeTableStore,
        peers: &dyn StateCatchup,
        lc_contract_state: &LcContractFinalizedState,
        layout: EpochLayout,
        chunk: &[EpochNumber],
        report: &mut PassReport,
    ) {
        let headers = match fetch_chunk_headers(peers, lc_contract_state, layout, chunk).await {
            Ok(headers) => headers,
            Err(err) => {
                tracing::error!(
                    first = %chunk[0],
                    last = %chunk[chunk.len() - 1],
                    "stake table backfill gave up on epochs, no root headers: {err:#}"
                );
                self.failed.add(chunk.len());
                report.failed += chunk.len();
                return;
            },
        };
        for &epoch in chunk {
            match store_epoch(store, layout, epoch, &headers).await {
                Ok(StakeTableStoreOutcome::Stored) => {
                    tracing::debug!(%epoch, "stored stake table");
                    self.stored.add(1);
                    report.stored += 1;
                    report.missing_after -= 1;
                },
                Ok(StakeTableStoreOutcome::HashMismatch { decided, derived }) => {
                    tracing::error!(
                        %epoch,
                        ?decided,
                        ?derived,
                        "stake table hash differs from the decided hash, epoch skipped"
                    );
                    self.hash_mismatches.add(1);
                    report.hash_mismatches += 1;
                },
                Err(err) => {
                    tracing::error!(%epoch, "stake table backfill gave up on epoch: {err:#}");
                    self.failed.add(1);
                    report.failed += 1;
                },
            }
        }
    }
}

/// Sleeps `RERUN_INTERVAL` when `rerun`, else never ends.
async fn rerun_after(rerun: bool) {
    if rerun {
        tokio::time::sleep(RERUN_INTERVAL).await;
    } else {
        std::future::pending().await
    }
}

/// What one pass did.
#[derive(Debug, Default, PartialEq)]
struct PassReport {
    /// Tables stored.
    stored: usize,
    /// Epochs skipped because of a hash mismatch.
    hash_mismatches: usize,
    /// Epochs given up on.
    failed: usize,
    /// Epochs still without a stored table at the end of the pass.
    missing_after: usize,
}

/// Where the backfill reads and writes the tables; the membership in production.
#[async_trait]
trait StakeTableStore: Sync {
    /// The epochs that have a stored stake table.
    async fn stored_epochs(&self) -> anyhow::Result<BTreeSet<EpochNumber>>;

    /// Stores the table of the epoch `root` defines; see `EpochCommittees::store_derived_stake_table`.
    async fn store(
        &self,
        root: &Header,
        previous_root: Option<&Header>,
        next_root: &Header,
    ) -> anyhow::Result<StakeTableStoreOutcome>;
}

#[async_trait]
impl StakeTableStore for EpochCommittees {
    async fn stored_epochs(&self) -> anyhow::Result<BTreeSet<EpochNumber>> {
        self.stored_stake_epochs().await
    }

    async fn store(
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

/// The epochs without a stored table below the highest stored one.
#[derive(Debug, PartialEq)]
struct MissingEpochs {
    /// How many there are.
    total: usize,
    /// Those whose roots are all below `F`, ascending.
    provable: Vec<EpochNumber>,
}

/// The epochs from the first one an epoch root defines up to the highest in `stored` that are
/// not in `stored`. Pure.
fn missing_epochs(
    layout: EpochLayout,
    stored: &BTreeSet<EpochNumber>,
    lc_contract_state: &LcContractFinalizedState,
) -> MissingEpochs {
    let Some(highest) = stored.last() else {
        return MissingEpochs {
            total: 0,
            provable: Vec::new(),
        };
    };
    // Epoch 3 is the first table an epoch root defines; the first two epochs are genesis tables.
    let lowest = (*layout.first_epoch + 2).max(3);
    let missing: Vec<EpochNumber> = (lowest..=**highest)
        .map(EpochNumber::new)
        .filter(|epoch| !stored.contains(epoch))
        .collect();
    // Only headers below `F` are provable; the next root is the highest of an epoch's roots.
    let provable = missing
        .iter()
        .copied()
        .filter(|epoch| layout.root_heights(*epoch).next < lc_contract_state.height)
        .collect();
    MissingEpochs {
        total: missing.len(),
        provable,
    }
}

/// The headers at the roots of `chunk`, each height once, by height.
async fn fetch_chunk_headers(
    peers: &dyn StateCatchup,
    lc_contract_state: &LcContractFinalizedState,
    layout: EpochLayout,
    chunk: &[EpochNumber],
) -> anyhow::Result<HashMap<u64, Header>> {
    let heights: BTreeSet<u64> = chunk
        .iter()
        .flat_map(|epoch| {
            let heights = layout.root_heights(*epoch);
            [Some(heights.root), Some(heights.next), heights.previous]
        })
        .flatten()
        .collect();
    let (_, headers) = fetch_headers(
        peers,
        lc_contract_state,
        None,
        heights.into_iter().collect(),
    )
    .await?;
    Ok(headers
        .into_iter()
        .map(|header| (header.height(), header))
        .collect())
}

/// Stores the table of `epoch` from its roots in `headers`, retrying a failed store.
async fn store_epoch(
    store: &dyn StakeTableStore,
    layout: EpochLayout,
    epoch: EpochNumber,
    headers: &HashMap<u64, Header>,
) -> anyhow::Result<StakeTableStoreOutcome> {
    let header = |height: u64| {
        headers
            .get(&height)
            .with_context(|| format!("no root header at height {height}"))
    };
    let heights = layout.root_heights(epoch);
    let root = header(heights.root)?;
    let next_root = header(heights.next)?;
    let previous_root = heights.previous.map(header).transpose()?;
    retry_request(|_| store.store(root, previous_root, next_root)).await
}

#[cfg(test)]
mod tests {
    use committable::Commitment;
    use hotshot_query_service::metrics::PrometheusMetrics;
    use hotshot_types::{light_client::CircuitField, utils::epoch_from_block_number};
    use parking_lot::Mutex;
    use rstest::rstest;

    use super::*;
    use crate::lc_contract_seed::{
        SEED_REQUEST_RETRIES,
        tests::{EPOCH_HEIGHT, mock_with_headers},
    };

    fn layout() -> EpochLayout {
        EpochLayout {
            first_epoch: EpochNumber::new(1),
            epoch_height: EPOCH_HEIGHT,
        }
    }

    fn epochs(numbers: impl IntoIterator<Item = u64>) -> BTreeSet<EpochNumber> {
        numbers.into_iter().map(EpochNumber::new).collect()
    }

    fn state_at(height: u64) -> LcContractFinalizedState {
        LcContractFinalizedState {
            height,
            block_comm_root: CircuitField::default(),
        }
    }

    /// Epochs below the highest stored one are missing from epoch 3, and those whose next root is
    /// at or above `F` wait.
    #[rstest]
    #[case::empty_storage(layout(), &[], 62, &[], 0)]
    #[case::complete(layout(), &[3, 4, 5, 6, 7, 8, 9, 10], 62, &[], 0)]
    #[case::gap(layout(), &[10], 62, &[3, 4, 5, 6, 7], 7)]
    #[case::next_root_at_f(layout(), &[10], 55, &[3, 4, 5, 6], 7)]
    #[case::next_root_below_f(layout(), &[10], 56, &[3, 4, 5, 6, 7], 7)]
    #[case::later_first_epoch(
        EpochLayout { first_epoch: EpochNumber::new(5), epoch_height: EPOCH_HEIGHT },
        &[10],
        62,
        &[7],
        3
    )]
    fn missing_epochs_cases(
        #[case] layout: EpochLayout,
        #[case] stored: &[u64],
        #[case] lc_contract_height: u64,
        #[case] provable: &[u64],
        #[case] total: usize,
    ) {
        let stored = epochs(stored.iter().copied());
        let missing = missing_epochs(layout, &stored, &state_at(lc_contract_height));

        let expected = MissingEpochs {
            total,
            provable: provable.iter().copied().map(EpochNumber::new).collect(),
        };
        assert_eq!(missing, expected);
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

    /// Records the stores; `stored` starts as storage holds it, and a store adds its epoch.
    #[derive(Default)]
    struct MockStakeTableStore {
        stored: Mutex<BTreeSet<EpochNumber>>,
        /// `(epoch, previous root height, next root height)` of each store call.
        calls: Mutex<Vec<(u64, Option<u64>, u64)>>,
        /// Epochs whose derived hash differs from the decided one.
        mismatched: BTreeSet<u64>,
        /// Epochs whose store fails.
        failing: BTreeSet<u64>,
    }

    #[async_trait]
    impl StakeTableStore for MockStakeTableStore {
        async fn stored_epochs(&self) -> anyhow::Result<BTreeSet<EpochNumber>> {
            Ok(self.stored.lock().clone())
        }

        async fn store(
            &self,
            root: &Header,
            previous_root: Option<&Header>,
            next_root: &Header,
        ) -> anyhow::Result<StakeTableStoreOutcome> {
            let epoch = epoch_from_block_number(root.height(), EPOCH_HEIGHT) + 2;
            self.calls
                .lock()
                .push((epoch, previous_root.map(Header::height), next_root.height()));
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

    /// A pass stores the epochs below the highest stored one that `F` proves, fetching each root
    /// once, and the next pass stores those that went missing from storage in the meantime.
    #[tokio::test(start_paused = true)]
    async fn fill_stores_missing_epochs_and_resumes() {
        let (peers, lc_contract_state) = mock_with_headers(&[5, 15, 25, 35, 45, 55]).await;
        let metrics = PrometheusMetrics::default();
        let backfill = StakeTableBackfill::new(&metrics);
        let store = MockStakeTableStore {
            stored: Mutex::new(epochs([10])),
            ..Default::default()
        };

        let report = backfill
            .fill(&store, &peers, &lc_contract_state, layout())
            .await
            .unwrap();

        let expected = PassReport {
            stored: 5,
            missing_after: 2,
            ..Default::default()
        };
        assert_eq!(report, expected);
        assert_eq!(*store.stored.lock(), epochs([3, 4, 5, 6, 7, 10]));
        let calls = store.calls.lock().clone();
        let expected_calls = [
            (3, None, 15),
            (4, None, 25),
            (5, None, 35),
            (6, Some(25), 45),
            (7, Some(35), 55),
        ];
        assert_eq!(calls, expected_calls);
        assert_eq!(peers.proven_header_calls(), 6);
        assert_eq!(metric(&metrics, "stake_table_backfill_stored"), 5);
        assert_eq!(metric(&metrics, "stake_table_backfill_missing_epochs"), 2);

        store
            .stored
            .lock()
            .retain(|epoch| ![4, 5].contains(&**epoch));
        let report = backfill
            .fill(&store, &peers, &lc_contract_state, layout())
            .await
            .unwrap();

        let expected = PassReport {
            stored: 2,
            missing_after: 2,
            ..Default::default()
        };
        assert_eq!(report, expected);
        assert_eq!(*store.stored.lock(), epochs([3, 4, 5, 6, 7, 10]));
        assert_eq!(peers.proven_header_calls(), 9);
        assert_eq!(metric(&metrics, "stake_table_backfill_stored"), 7);
    }

    /// A hash mismatch is skipped without a retry, a failing epoch is retried, then given up, and
    /// the pass goes on with the epochs above.
    #[tokio::test(start_paused = true)]
    async fn fill_skips_a_mismatch_and_gives_up_on_a_failure() {
        let (peers, lc_contract_state) = mock_with_headers(&[5, 15, 25, 35, 45, 55]).await;
        let metrics = PrometheusMetrics::default();
        let backfill = StakeTableBackfill::new(&metrics);
        let store = MockStakeTableStore {
            stored: Mutex::new(epochs([10])),
            mismatched: BTreeSet::from([4]),
            failing: BTreeSet::from([5]),
            ..Default::default()
        };

        let report = backfill
            .fill(&store, &peers, &lc_contract_state, layout())
            .await
            .unwrap();

        let expected = PassReport {
            stored: 3,
            hash_mismatches: 1,
            failed: 1,
            missing_after: 4,
        };
        assert_eq!(report, expected);
        assert_eq!(*store.stored.lock(), epochs([3, 6, 7, 10]));
        let calls_of = |epoch| store.calls.lock().iter().filter(|c| c.0 == epoch).count();
        assert_eq!(calls_of(4), 1);
        assert_eq!(calls_of(5), SEED_REQUEST_RETRIES as usize + 1);
        assert_eq!(metric(&metrics, "stake_table_backfill_hash_mismatches"), 1);
        assert_eq!(metric(&metrics, "stake_table_backfill_failed"), 1);
        assert_eq!(metric(&metrics, "stake_table_backfill_missing_epochs"), 4);
    }
}
