// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Regression tests for a permanently wedged epoch stake-table catchup.
//!
//! Observed on mainnet 2026-08-14: several validators stopped advancing at an
//! epoch boundary and never recovered without a restart. For days they logged
//! nothing but
//!
//! ```text
//! WARN : Stake table for epoch EpochNumber(N) unavailable. Catchup already in progress
//! ```
//!
//! while never emitting `Fetching stake tables for epochs: …`, the success
//! removals, or `catchup for epoch … failed … Canceling catchup`. That
//! combination means the spawned `catchup()` task was parked inside the
//! epoch-discovery loop and therefore never removed its `catchup_map` entry.
//! Because `stake_table_for_epoch` / `membership_for_epoch` short-circuit on
//! `Entry::Occupied`, the entry pins the coordinator for the lifetime of the
//! process: no timeout, no eviction, no retry. Every view in which such a node
//! is elected leader then times out.
//!
//! The tests below drive `EpochMembershipCoordinator` directly with a
//! membership whose stake-table load never completes (or whose catchup task
//! dies), and assert that the coordinator eventually gives up on the stuck
//! attempt and tries again — including for the intermediate epochs the stuck
//! attempt had claimed on its way to the requested one. On unfixed code they
//! fail: exactly one attempt is ever made.

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::anyhow;
use hotshot_example_types::{
    block_types::{TestBlockHeader, TestBlockPayload, TestTransaction},
    membership::{
        TestableMembership,
        static_committee::StaticStakeTable,
        strict_membership::{
            StrictEpochSnapshot, StrictMembership, StrictMembershipError, StrictNonEpochSnapshot,
        },
    },
    state_types::{TestInstanceState, TestValidatedState},
    storage_types::TestStorage,
};
use hotshot_testing::{node_stake::TestNodeStakes, test_builder::gen_node_lists};
use hotshot_types::{
    PeerConfig,
    constants::TEST_UPGRADE_CONSTANTS,
    data::{EpochNumber, Leaf2},
    drb::{DrbDifficultySelectorFn, DrbResult, INITIAL_DRB_RESULT},
    epoch_membership::EpochMembershipCoordinator,
    signature_key::{BLSPubKey, BuilderKey, SchnorrPubKey},
    traits::{
        election::Membership, node_implementation::NodeType, signature_key::StakeTableEntryType,
        storage::StoreDrbResultFn,
    },
    upgrade_config::UpgradeConstants,
    utils::root_block_in_epoch,
};
use sha2::{Digest, Sha256};
use vbs::version::Version;

/// Blocks per epoch. Irrelevant to the wedge, but the coordinator needs one.
const EPOCH_HEIGHT: u64 = 10;

/// The first epoch with a stake table. `set_first_epoch` also registers
/// `FIRST_EPOCH + 1`, so epochs 1 and 2 resolve locally and everything from 3
/// up has to come from catchup.
const FIRST_EPOCH: u64 = 1;

/// The epoch the test asks for. Two above the highest locally known epoch, so
/// `catchup()` enters its epoch-discovery loop rather than short-circuiting.
const TARGET_EPOCH: u64 = 5;

/// How long a caller is willing to wait for the coordinator to notice that an
/// in-flight catchup is not making progress. Far longer than any legitimate
/// catchup in a unit test.
const RECOVERY_BUDGET: Duration = Duration::from_secs(3);

/// How long [`HangOncePastWatch`](LoadBehavior::HangOncePastWatch) stalls the
/// first load: past the watchdog budget (500 ms in `setup`) plus its
/// post-abandonment watch (another 500 ms), with margin for slow CI, so the
/// attempt resumes only once nothing is watching it any more.
const LATE_LOAD_STALL: Duration = Duration::from_secs(2);

type InnerMembership = StrictMembership<WedgeTypes, StaticStakeTable<BLSPubKey, SchnorrPubKey>>;

/// How the stake-table load misbehaves.
#[derive(Clone, Copy, Debug)]
enum LoadBehavior {
    /// Never returns — models `load_stake_table` blocking on the persistence
    /// lock, which is what parked the mainnet nodes inside the discovery loop.
    Hang,
    /// Panics on the first call, killing the spawned catchup task before it can
    /// run `catchup_cleanup`. Models any way the task can die unexpectedly.
    PanicOnce,
    /// The load itself returns quickly (with "not found"), letting the
    /// epoch-discovery loop claim `catchup_map` entries for the intermediate
    /// epochs, but the epoch-root fetch then never returns. Parks the task
    /// one step later than [`Hang`](Self::Hang): after it has claimed entries
    /// it will never release on its own.
    HangEpochRoot,
    /// Loads fail fast and epoch roots resolve, but the DRB result has to be
    /// computed locally, with a difficulty calibrated to several watchdog
    /// periods of real hashing — modelling mainnet's tens-of-minutes DRB
    /// computation. A healthy-but-slow attempt like this must not be
    /// abandoned.
    SlowDrb,
    /// The first load stalls long enough to outlive both the watchdog budget
    /// and its post-abandonment watch, then reports "not found" — modelling a
    /// stalled query that resolves only after the attempt was abandoned. The
    /// resumed attempt then reaches the discovery loop's vacant branch while
    /// abandoned, where it must not insert a fresh `catchup_map` entry.
    HangOncePastWatch,
    /// The first epoch-root fetch parks until the test fires
    /// [`WedgeMembership::release_root`], then fails; later fetches park
    /// forever. Lets the test hold an attempt parked past its abandonment and
    /// then drive it into a failure path — i.e. into `catchup_cleanup` —
    /// while a second attempt owns the `catchup_map` entries.
    HangEpochRootUntilReleased,
    /// Loads fail fast and epoch roots resolve (as in
    /// [`SlowDrb`](Self::SlowDrb)), but the attempt dies inside the local DRB
    /// computation — the test installs a difficulty selector that panics on
    /// its first call, which is awaited after the computation has claimed its
    /// `drb_calculation_map` entry. That entry must be released, or no retry
    /// can ever compute this epoch's DRB.
    PanicInDrb,
    /// Loads fail fast, epoch roots resolve, and the local DRB computation
    /// completes instantly — but persisting the result never returns (the
    /// test installs a hung `store_drb_result_fn`). The attempt parks in the
    /// write still holding its `drb_calculation_map` entry, so no retry can
    /// compute the DRB either; the epoch must resolve from the in-memory
    /// result anyway.
    HangDrbStore,
    /// Loads fail fast and epoch roots resolve, but the peer DRB fetch parks
    /// until the test fires [`WedgeMembership::release_drb`], then fails.
    /// Holds an attempt at the last checkpoint before the local DRB
    /// computation until well past its abandonment, so the test can verify
    /// the resumed attempt does not enter the hash chain as an unsupervised
    /// zombie.
    HangDrbFetchUntilReleased,
}

/// A `Membership` that delegates everything to `StrictMembership` except the
/// stake-table load, which misbehaves per [`LoadBehavior`], and the epoch-root
/// and DRB fetches, which fail cleanly so a retry has a defined outcome.
#[derive(Clone, Debug)]
struct WedgeMembership {
    inner: Arc<InnerMembership>,
    behavior: LoadBehavior,
    /// Number of times `load_stake_table` has been entered. One per catchup
    /// attempt that reached the epoch-discovery loop.
    load_calls: Arc<AtomicUsize>,
    /// Number of times `get_epoch_root` has been entered. One per catchup
    /// attempt that got past the epoch-discovery loop.
    root_calls: Arc<AtomicUsize>,
    /// Template leaf returned by `get_epoch_root` under
    /// [`SlowDrb`](LoadBehavior::SlowDrb), seeded by the test up front:
    /// `Leaf2::genesis` pays a multi-second one-time global setup cost on
    /// first use, which must not happen inside a watchdog window.
    root_leaf: Arc<std::sync::OnceLock<Leaf2<WedgeTypes>>>,
    /// Unparks the first epoch-root fetch under
    /// [`HangEpochRootUntilReleased`](LoadBehavior::HangEpochRootUntilReleased).
    release_root: Arc<tokio::sync::Notify>,
    /// Number of times `get_epoch_drb` has been entered. One per catchup
    /// attempt that got past the epoch-root fetches.
    drb_calls: Arc<AtomicUsize>,
    /// Unparks the peer DRB fetch under
    /// [`HangDrbFetchUntilReleased`](LoadBehavior::HangDrbFetchUntilReleased).
    release_drb: Arc<tokio::sync::Notify>,
}

impl WedgeMembership {
    fn attempts(&self) -> usize {
        self.load_calls.load(Ordering::SeqCst)
    }

    fn root_fetches(&self) -> usize {
        self.root_calls.load(Ordering::SeqCst)
    }

    fn drb_fetches(&self) -> usize {
        self.drb_calls.load(Ordering::SeqCst)
    }
}

impl Membership<WedgeTypes> for WedgeMembership {
    type Error = StrictMembershipError;
    type Snapshot = StrictEpochSnapshot<WedgeTypes, StaticStakeTable<BLSPubKey, SchnorrPubKey>>;
    type NonEpochSnapshot =
        StrictNonEpochSnapshot<WedgeTypes, StaticStakeTable<BLSPubKey, SchnorrPubKey>>;

    fn snapshot(&self, epoch: EpochNumber) -> Option<Self::Snapshot> {
        self.inner.snapshot(epoch)
    }

    fn non_epoch_snapshot(&self) -> Self::NonEpochSnapshot {
        self.inner.non_epoch_snapshot()
    }

    fn first_epoch(&self) -> Option<EpochNumber> {
        self.inner.first_epoch()
    }

    async fn get_epoch_root(
        &self,
        epoch: EpochNumber,
        _coordinator: &EpochMembershipCoordinator<WedgeTypes>,
    ) -> Result<Leaf2<WedgeTypes>, Self::Error> {
        let calls = self.root_calls.fetch_add(1, Ordering::SeqCst) + 1;
        tracing::info!(%epoch, calls, "get_epoch_root entered");
        match self.behavior {
            LoadBehavior::HangEpochRoot => {
                std::future::pending::<()>().await;
                unreachable!("pending() never resolves")
            },
            LoadBehavior::SlowDrb
            | LoadBehavior::PanicInDrb
            | LoadBehavior::HangDrbStore
            | LoadBehavior::HangDrbFetchUntilReleased => {
                // A usable root: `StrictMembership::add_epoch_root` registers
                // the stake table for `epoch_from_block_number(height) + 2`,
                // so a block inside `epoch` (the root epoch this is fetched
                // from) registers `epoch + 2` — the epoch being fetched.
                let mut leaf = self
                    .root_leaf
                    .get()
                    .cloned()
                    .expect("this behavior requires the test to seed root_leaf");
                leaf.block_header_mut().block_number = root_block_in_epoch(*epoch, EPOCH_HEIGHT);
                Ok(leaf)
            },
            LoadBehavior::Hang | LoadBehavior::PanicOnce | LoadBehavior::HangOncePastWatch => {
                Err(anyhow!("epoch root unavailable").into())
            },
            LoadBehavior::HangEpochRootUntilReleased if calls == 1 => {
                self.release_root.notified().await;
                Err(anyhow!("epoch root unavailable").into())
            },
            LoadBehavior::HangEpochRootUntilReleased => {
                std::future::pending::<()>().await;
                unreachable!("pending() never resolves")
            },
        }
    }

    async fn get_epoch_drb(
        &self,
        epoch: EpochNumber,
        _coordinator: &EpochMembershipCoordinator<WedgeTypes>,
    ) -> Result<DrbResult, Self::Error> {
        let calls = self.drb_calls.fetch_add(1, Ordering::SeqCst) + 1;
        tracing::info!(%epoch, calls, "get_epoch_drb entered");
        if matches!(self.behavior, LoadBehavior::HangDrbFetchUntilReleased) {
            self.release_drb.notified().await;
        }
        Err(anyhow!("drb unavailable").into())
    }

    async fn add_epoch_root(
        &self,
        h: TestBlockHeader,
        coordinator: &EpochMembershipCoordinator<WedgeTypes>,
    ) -> Result<(), Self::Error> {
        self.inner.add_epoch_root(h, coordinator).await
    }

    fn add_drb_result(&self, e: EpochNumber, d: DrbResult) {
        self.inner.add_drb_result(e, d);
    }

    async fn load_stake_table(&self, epoch: EpochNumber) -> bool {
        let calls = self.load_calls.fetch_add(1, Ordering::SeqCst) + 1;
        tracing::info!(%epoch, calls, "load_stake_table entered");
        match self.behavior {
            LoadBehavior::Hang => std::future::pending::<bool>().await,
            LoadBehavior::PanicOnce if calls == 1 => {
                panic!("simulated catchup task death while loading epoch {epoch}")
            },
            LoadBehavior::HangOncePastWatch if calls == 1 => {
                tokio::time::sleep(LATE_LOAD_STALL).await;
                false
            },
            LoadBehavior::PanicOnce
            | LoadBehavior::HangEpochRoot
            | LoadBehavior::SlowDrb
            | LoadBehavior::PanicInDrb
            | LoadBehavior::HangDrbStore
            | LoadBehavior::HangDrbFetchUntilReleased
            | LoadBehavior::HangOncePastWatch
            | LoadBehavior::HangEpochRootUntilReleased => false,
        }
    }

    fn set_first_epoch(&self, e: EpochNumber, r: DrbResult) {
        self.inner.set_first_epoch(e, r);
    }

    fn add_da_committee(
        &self,
        first_epoch: EpochNumber,
        da_committee: Vec<PeerConfig<WedgeTypes>>,
    ) {
        self.inner.add_da_committee(first_epoch, da_committee);
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
struct WedgeTypes;

impl NodeType for WedgeTypes {
    const UPGRADE_CONSTANTS: UpgradeConstants = TEST_UPGRADE_CONSTANTS;

    type BlockHeader = TestBlockHeader;
    type BlockPayload = TestBlockPayload;
    type SignatureKey = BLSPubKey;
    type Transaction = TestTransaction;
    type ValidatedState = TestValidatedState;
    type InstanceState = TestInstanceState;
    type Membership = WedgeMembership;
    type BuilderSignatureKey = BuilderKey;
    type StateSignatureKey = SchnorrPubKey;
}

fn setup(behavior: LoadBehavior) -> (WedgeMembership, EpochMembershipCoordinator<WedgeTypes>) {
    let (quorum, da) = gen_node_lists::<WedgeTypes>(4, 4, &TestNodeStakes::default());
    let public_key = quorum[0].stake_table_entry.public_key();

    let inner = <InnerMembership as TestableMembership<WedgeTypes>>::new(
        quorum,
        da,
        public_key,
        EPOCH_HEIGHT,
    );
    let membership = WedgeMembership {
        inner: Arc::new(inner),
        behavior,
        load_calls: Arc::default(),
        root_calls: Arc::default(),
        root_leaf: Arc::default(),
        release_root: Arc::default(),
        drb_calls: Arc::default(),
        release_drb: Arc::default(),
    };
    // Registers stake tables for FIRST_EPOCH and FIRST_EPOCH + 1.
    membership.set_first_epoch(EpochNumber::new(FIRST_EPOCH), INITIAL_DRB_RESULT);

    let coordinator = EpochMembershipCoordinator::new(
        membership.clone(),
        EPOCH_HEIGHT,
        &TestStorage::<WedgeTypes>::default(),
    )
    .with_catchup_timeout(Duration::from_millis(500));
    (membership, coordinator)
}

/// Poll `cond` until it holds, or `budget` elapses.
async fn wait_until(budget: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + budget;
    loop {
        if cond() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// The mainnet failure: the stake-table load never returns, so `catchup()` is
/// parked in its epoch-discovery loop, never removes its `catchup_map` entry,
/// and every later request is answered "Catchup already in progress" forever.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn catchup_retries_after_stake_table_load_hangs() {
    let (membership, coordinator) = setup(LoadBehavior::Hang);
    let target = EpochNumber::new(TARGET_EPOCH);

    // First request has no snapshot to return, so it starts catchup.
    let Err(err) = coordinator.stake_table_for_epoch(Some(target)) else {
        panic!("epoch {target} is not locally known, so this must not succeed")
    };
    assert!(
        format!("{err:?}").contains("Starting catchup"),
        "first request should have started catchup, got: {err:?}"
    );

    // Wait until the spawned task is actually parked in the discovery loop.
    assert!(
        wait_until(RECOVERY_BUDGET, || membership.attempts() >= 1).await,
        "catchup task never reached load_stake_table"
    );

    // While the attempt is in flight, callers are told so. That much is fine.
    let Err(err) = coordinator.stake_table_for_epoch(Some(target)) else {
        panic!("epoch {target} must still be unavailable")
    };
    assert!(
        format!("{err:?}").contains("Catchup already in progress"),
        "expected an in-progress error, got: {err:?}"
    );

    // The attempt will never finish. The coordinator must eventually abandon
    // it so a fresh attempt can run; otherwise the node is wedged until it is
    // restarted, and every view it leads times out.
    let retried = wait_until(RECOVERY_BUDGET, || {
        let _ = coordinator.stake_table_for_epoch(Some(target));
        membership.attempts() >= 2
    })
    .await;

    assert!(
        retried,
        "catchup for {target} was never retried in {RECOVERY_BUDGET:?}: the catchup_map entry is \
         never evicted when the in-flight attempt hangs, so stake_table_for_epoch answers \
         \"Catchup already in progress\" forever (attempts = {})",
        membership.attempts()
    );
}

/// Same wedge reached a different way: the spawned catchup task dies before it
/// can run `catchup_cleanup`, so its `catchup_map` entry is orphaned. Nothing
/// owns it and nothing will ever remove it.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn catchup_retries_after_catchup_task_dies() {
    let (membership, coordinator) = setup(LoadBehavior::PanicOnce);
    let target = EpochNumber::new(TARGET_EPOCH);

    assert!(
        coordinator.stake_table_for_epoch(Some(target)).is_err(),
        "epoch {target} is not locally known, so this must not succeed"
    );

    // The task ran and died. Its map entry is now orphaned.
    assert!(
        wait_until(RECOVERY_BUDGET, || membership.attempts() >= 1).await,
        "catchup task never reached load_stake_table"
    );

    let retried = wait_until(RECOVERY_BUDGET, || {
        let _ = coordinator.stake_table_for_epoch(Some(target));
        membership.attempts() >= 2
    })
    .await;

    assert!(
        retried,
        "catchup for {target} was never retried in {RECOVERY_BUDGET:?}: the catchup task died \
         without running catchup_cleanup, leaking its catchup_map entry (attempts = {})",
        membership.attempts()
    );
}

/// The wedge one level deeper: on its way to the requested epoch, the
/// discovery loop claims `catchup_map` entries for the intermediate epochs it
/// plans to fetch. If the attempt is then abandoned, evicting only the
/// requested epoch's entry leaves the intermediate ones orphaned, and every
/// `stake_table_for_epoch` for those epochs is answered "Catchup already in
/// progress" for the lifetime of the process.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn intermediate_epochs_recover_after_catchup_abandoned() {
    let (membership, coordinator) = setup(LoadBehavior::HangEpochRoot);
    let target = EpochNumber::new(TARGET_EPOCH);
    // Claimed by the discovery loop on the way to `target`, then orphaned
    // when the parked attempt is abandoned.
    let intermediate = EpochNumber::new(TARGET_EPOCH - 1);

    assert!(
        coordinator.stake_table_for_epoch(Some(target)).is_err(),
        "epoch {target} is not locally known, so this must not succeed"
    );

    // Wait until the attempt has claimed the intermediate epochs and parked
    // itself in the epoch-root fetch.
    assert!(
        wait_until(RECOVERY_BUDGET, || membership.root_fetches() >= 1).await,
        "catchup task never reached get_epoch_root"
    );

    // While the attempt is parked it owns the intermediate epoch's entry, so
    // callers are told a catchup is in flight. That much is fine.
    let Err(err) = coordinator.stake_table_for_epoch(Some(intermediate)) else {
        panic!("epoch {intermediate} must still be unavailable")
    };
    assert!(
        format!("{err:?}").contains("Catchup already in progress"),
        "expected an in-progress error, got: {err:?}"
    );

    // Once the attempt is abandoned, its intermediate entry must be evicted
    // too, so a fresh catchup can be started for that epoch.
    let recovered = wait_until(RECOVERY_BUDGET, || {
        matches!(
            coordinator.stake_table_for_epoch(Some(intermediate)),
            Err(e) if format!("{e:?}").contains("Starting catchup")
        )
    })
    .await;

    assert!(
        recovered,
        "the abandoned catchup's entry for intermediate epoch {intermediate} was never evicted: \
         stake_table_for_epoch answers \"Catchup already in progress\" forever"
    );
}

/// An abandoned attempt that later resumes must not claim new epochs. Here
/// the first load stalls past the watchdog's post-abandonment watch, then
/// returns "not found"; the resumed attempt reaches the vacant branch of the
/// discovery loop while abandoned. On unfixed code it inserts a `catchup_map`
/// entry for the intermediate epoch there and — with the watchdog gone — the
/// orphaned entry answers "Catchup already in progress" until some unrelated
/// cleanup happens to sweep it.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn abandoned_attempt_claims_no_new_epochs() {
    let (membership, coordinator) = setup(LoadBehavior::HangOncePastWatch);
    let target = EpochNumber::new(TARGET_EPOCH);
    let intermediate = EpochNumber::new(TARGET_EPOCH - 1);

    assert!(
        coordinator.stake_table_for_epoch(Some(target)).is_err(),
        "epoch {target} is not locally known, so this must not succeed"
    );
    assert!(
        wait_until(RECOVERY_BUDGET, || membership.attempts() >= 1).await,
        "catchup task never reached load_stake_table"
    );

    // Sleep past the stalled load's return, deliberately NOT requesting
    // `intermediate` in the meantime: a request would claim the epoch
    // legitimately and mask the bug.
    tokio::time::sleep(LATE_LOAD_STALL + Duration::from_millis(400)).await;

    // The resumed-but-abandoned attempt must have left no entry behind, so a
    // fresh catchup for the intermediate epoch must start immediately.
    let Err(err) = coordinator.stake_table_for_epoch(Some(intermediate)) else {
        panic!("epoch {intermediate} has no stake table, so this must not succeed")
    };
    assert!(
        format!("{err:?}").contains("Starting catchup"),
        "the abandoned attempt left an orphaned catchup_map entry for epoch {intermediate}: \
         {err:?}"
    );
}

/// An abandoned attempt that later resumes and *fails* must not run its
/// cleanup. Here the first attempt parks in the epoch-root fetch until the
/// test releases it, well after the watchdog abandoned it and a second
/// attempt claimed the same epochs. When the released fetch then fails, the
/// late attempt reaches `catchup_cleanup` — which must be a no-op: on unfixed
/// code it evicts the second attempt's `catchup_map` entries (they share the
/// keys) and fails its waiters, spawning a needless third attempt.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn late_failure_of_abandoned_attempt_does_not_evict_retry() {
    let (membership, coordinator) = setup(LoadBehavior::HangEpochRootUntilReleased);
    // A roomier watchdog than `setup`'s: the assertions below must all land
    // inside the *second* attempt's watchdog budget, whose legitimate
    // abandonment would also answer "Starting catchup".
    let watchdog = Duration::from_secs(1);
    let coordinator = coordinator.with_catchup_timeout(watchdog);
    let target = EpochNumber::new(TARGET_EPOCH);

    assert!(
        coordinator.stake_table_for_epoch(Some(target)).is_err(),
        "epoch {target} is not locally known, so this must not succeed"
    );
    // Attempt 1 claims the intermediate epochs and parks in the root fetch.
    assert!(
        wait_until(RECOVERY_BUDGET, || membership.root_fetches() >= 1).await,
        "catchup task never reached get_epoch_root"
    );

    // Wait out attempt 1's abandonment. The first request answered with
    // "Starting catchup" also spawns attempt 2, which re-claims the same
    // epochs and parks in root fetch #2.
    let retried = wait_until(RECOVERY_BUDGET, || {
        matches!(
            coordinator.stake_table_for_epoch(Some(target)),
            Err(e) if format!("{e:?}").contains("Starting catchup")
        )
    })
    .await;
    assert!(retried, "attempt 1 was never abandoned");
    assert!(
        wait_until(RECOVERY_BUDGET, || membership.root_fetches() >= 2).await,
        "the second attempt never reached get_epoch_root"
    );

    // Release attempt 1: its root fetch fails and it hits `catchup_cleanup`
    // while abandoned.
    membership.release_root.notify_one();

    // Attempt 2 owns the map entries now, and the late failure must not have
    // evicted them: every poll must keep answering "Catchup already in
    // progress". Poll long enough for the unguarded cleanup to have certainly
    // run, but well inside attempt 2's own watchdog budget.
    let deadline = Instant::now() + Duration::from_millis(250);
    while Instant::now() < deadline {
        let Err(err) = coordinator.stake_table_for_epoch(Some(target)) else {
            panic!("epoch {target} must still be unavailable")
        };
        assert!(
            format!("{err:?}").contains("Catchup already in progress"),
            "the abandoned attempt's late failure evicted the second attempt's catchup_map entry: \
             {err:?}"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// The inverse guarantee: an attempt that is merely *slow* — computing a DRB
/// locally, which on mainnet is 25e9 sequential hashes, i.e. tens of minutes
/// by design — must NOT be abandoned. Abandoning it would spawn retries that
/// re-fetch epoch roots from peers only to die on the "DRB calculation
/// already in progress" guard, a storm of doomed catchups for as long as the
/// real computation runs.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn slow_drb_computation_is_not_abandoned() {
    let (membership, coordinator) = setup(LoadBehavior::SlowDrb);
    // A tighter watchdog for this test: compile profiles skew the calibration
    // probe below against the real hash loop by several ×, and the chain must
    // outlast several windows whichever way the skew goes.
    let watchdog = Duration::from_millis(200);
    let coordinator = coordinator.with_catchup_timeout(watchdog);
    // Built up front: the first `Leaf2::genesis` pays a one-time global setup
    // cost of several seconds, which must not eat into a watchdog window.
    let root_leaf = Leaf2::<WedgeTypes>::genesis(
        &TestValidatedState::default(),
        &TestInstanceState::default(),
        Version { major: 0, minor: 1 },
    )
    .await;
    membership
        .root_leaf
        .set(root_leaf)
        .expect("root_leaf seeded once");
    // The slow part must be the hash chain itself — the only step the
    // watchdog is expected to wait out. Calibrate a difficulty targeting 8 s
    // of hashing as measured by this probe; the real chain lands anywhere
    // from ~1 s (probe unoptimized, chain optimized) to ~8 s (both
    // optimized), which is several watchdog windows either way.
    let probe_iters: u32 = 200_000;
    let start = Instant::now();
    let mut probe = [0u8; 32];
    for _ in 0..probe_iters {
        probe = Sha256::digest(probe).into();
    }
    std::hint::black_box(probe);
    let per_iter = start.elapsed() / probe_iters;
    let difficulty =
        (Duration::from_secs(8).as_nanos() / per_iter.as_nanos().max(1)).max(1_000_000) as u64;
    let selector: DrbDifficultySelectorFn = Arc::new(move |_| Box::pin(async move { difficulty }));
    coordinator.set_drb_difficulty_selector(selector);
    let target = EpochNumber::new(TARGET_EPOCH);

    // Starts the one and only catchup attempt.
    let started = Instant::now();
    assert!(
        coordinator.membership_for_epoch(Some(target)).is_err(),
        "epoch {target} is not locally known, so this must not succeed"
    );

    // The attempt fetches the epoch roots for 3, 4 and 5, then sits in the
    // DRB computation. It must be left alone until it resolves the epoch.
    let resolved = wait_until(Duration::from_secs(20), || {
        coordinator.membership_for_epoch(Some(target)).is_ok()
    })
    .await;
    assert!(
        resolved,
        "catchup for {target} did not complete: either the slow DRB computation was abandoned or \
         it never ran (root fetches = {})",
        membership.root_fetches()
    );
    assert!(
        started.elapsed() >= 2 * watchdog,
        "the hash chain finished before the watchdog could fire, so this run proved nothing; \
         raise the calibration target"
    );
    assert_eq!(
        membership.root_fetches(),
        3,
        "the watchdog abandoned the attempt during its legitimate DRB computation: retries were \
         spawned that re-fetched epoch roots only to die on the DRB-in-progress guard"
    );
}

/// An attempt that dies *inside* the local DRB computation must release the
/// `drb_calculation_map` entry it claimed. That entry is what stops a retry
/// from starting a second concurrent hash chain, so if the dead attempt keeps
/// it, every retry fails with "DRB calculation already in progress" and the
/// epoch can never obtain a DRB locally again.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn drb_state_is_released_when_attempt_dies_computing() {
    let (membership, coordinator) = setup(LoadBehavior::PanicInDrb);
    // Built up front: the first `Leaf2::genesis` pays a one-time global setup
    // cost of several seconds, which must not eat into a watchdog window.
    let root_leaf = Leaf2::<WedgeTypes>::genesis(
        &TestValidatedState::default(),
        &TestInstanceState::default(),
        Version { major: 0, minor: 1 },
    )
    .await;
    membership
        .root_leaf
        .set(root_leaf)
        .expect("root_leaf seeded once");
    // The difficulty selector is awaited between the computation's claim of
    // its `drb_calculation_map` entry and the hash chain, so a panic in it is
    // a death inside exactly the window where the entry could leak. Panic on
    // the first attempt; compute a trivial chain on retries.
    let selector_calls = Arc::new(AtomicUsize::new(0));
    let selector: DrbDifficultySelectorFn = {
        let selector_calls = Arc::clone(&selector_calls);
        Arc::new(move |_| {
            let calls = selector_calls.fetch_add(1, Ordering::SeqCst) + 1;
            Box::pin(async move {
                if calls == 1 {
                    panic!("simulated catchup task death while computing the DRB");
                }
                10
            })
        })
    };
    coordinator.set_drb_difficulty_selector(selector);
    let target = EpochNumber::new(TARGET_EPOCH);

    // Starts the first attempt, which dies inside the DRB phase.
    assert!(
        coordinator.membership_for_epoch(Some(target)).is_err(),
        "epoch {target} is not locally known, so this must not succeed"
    );

    // A retry must be able to compute the DRB itself. On unfixed code the
    // dead attempt never released its drb_calculation_map entry, so every
    // retry dies on the DRB-in-progress guard and the epoch never resolves.
    let resolved = wait_until(RECOVERY_BUDGET, || {
        coordinator.membership_for_epoch(Some(target)).is_ok()
    })
    .await;
    assert!(
        resolved,
        "no retry could compute the DRB for {target} within {RECOVERY_BUDGET:?}: the attempt that \
         died mid-computation leaked its drb_calculation_map entry (selector calls = {}, root \
         fetches = {})",
        selector_calls.load(Ordering::SeqCst),
        membership.root_fetches()
    );
}

/// A stalled DRB *write* must not stall epoch resolution. The computation
/// itself completes, but persisting the result never returns, so the attempt
/// parks in the write still holding its `drb_calculation_map` entry until the
/// watchdog abandons it. The computed result must already be in the
/// membership by then: on unfixed code it is only added after the write
/// returns, so the epoch stays unresolved and every retry re-fetches epoch
/// roots from peers only to die on the DRB-in-progress guard for as long as
/// the write is stalled.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn stalled_drb_result_write_does_not_block_epoch_resolution() {
    let (membership, coordinator) = setup(LoadBehavior::HangDrbStore);
    let hung_store: StoreDrbResultFn = Arc::new(Box::new(|_, _| Box::pin(std::future::pending())));
    let coordinator = coordinator.with_store_drb_result_fn(hung_store);
    // Built up front: the first `Leaf2::genesis` pays a one-time global setup
    // cost of several seconds, which must not eat into a watchdog window.
    let root_leaf = Leaf2::<WedgeTypes>::genesis(
        &TestValidatedState::default(),
        &TestInstanceState::default(),
        Version { major: 0, minor: 1 },
    )
    .await;
    membership
        .root_leaf
        .set(root_leaf)
        .expect("root_leaf seeded once");
    // A trivial difficulty: the computation must be instant so the only thing
    // outlasting the watchdog is the stalled write.
    let selector: DrbDifficultySelectorFn = Arc::new(|_| Box::pin(async { 10 }));
    coordinator.set_drb_difficulty_selector(selector);
    let target = EpochNumber::new(TARGET_EPOCH);

    // Starts the attempt that computes the DRB and then parks in the write.
    assert!(
        coordinator.membership_for_epoch(Some(target)).is_err(),
        "epoch {target} is not locally known, so this must not succeed"
    );

    let resolved = wait_until(RECOVERY_BUDGET, || {
        coordinator.membership_for_epoch(Some(target)).is_ok()
    })
    .await;
    assert!(
        resolved,
        "epoch {target} never resolved within {RECOVERY_BUDGET:?} while the DRB result write was \
         stalled: the computed result reaches the membership only after the write returns, so \
         every retry re-fetched epoch roots only to die on the DRB-in-progress guard (root \
         fetches = {})",
        membership.root_fetches()
    );
}

/// A DRB computation whose future is dropped mid-flight — new-protocol's
/// `EpochManager::gc` aborts in-flight DRB tasks — must fire its cancel token
/// as it cleans up: a hash batch left running on the blocking pool holds a
/// clone of that token and checks it between chunks, and the cleanup removes
/// the token from the coordinator's maps, so an unfired token means the
/// orphaned batch grinds through up to a full checkpoint interval of hashing
/// that `supply_drb` and `cancel_all_drb` can no longer stop.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn aborted_drb_computation_fires_its_cancel_token() {
    // The membership behavior is irrelevant here: the computation is driven
    // directly, not through catchup.
    let (_membership, coordinator) = setup(LoadBehavior::SlowDrb);
    let root_leaf = Leaf2::<WedgeTypes>::genesis(
        &TestValidatedState::default(),
        &TestInstanceState::default(),
        Version { major: 0, minor: 1 },
    )
    .await;
    // The difficulty selector is awaited after the computation has claimed
    // its maps and cancel token; parking there gives a deterministic
    // in-flight point to abort at, with no hashing involved.
    let entered = Arc::new(tokio::sync::Notify::new());
    let selector: DrbDifficultySelectorFn = {
        let entered = Arc::clone(&entered);
        Arc::new(move |_| {
            let entered = Arc::clone(&entered);
            Box::pin(async move {
                entered.notify_one();
                std::future::pending::<u64>().await
            })
        })
    };
    coordinator.set_drb_difficulty_selector(selector);
    let epoch = EpochNumber::new(TARGET_EPOCH);

    let computation = tokio::spawn({
        let coordinator = coordinator.clone();
        async move { coordinator.compute_drb_result(epoch, root_leaf).await }
    });
    entered.notified().await;
    let token = coordinator
        .drb_cancel_token(epoch)
        .expect("the computation claims its cancel token before awaiting the selector");

    computation.abort();

    let fired = wait_until(RECOVERY_BUDGET, || token.is_cancelled()).await;
    assert!(
        fired,
        "dropping the DRB computation's future did not fire its cancel token within \
         {RECOVERY_BUDGET:?}: an orphaned hash batch on the blocking pool would grind on, \
         uncancellable, because the cleanup already removed the token from the coordinator's maps"
    );
}

/// An attempt abandoned during its peer DRB fetch must not resume into the
/// local DRB computation: the hash chain is exempt from the watchdog budget
/// by design, so a resumed-but-abandoned attempt would compute for tens of
/// minutes as an unsupervised zombie while every retry bounces off the
/// DRB-in-progress guard. The guard at the computation's entrance cannot
/// close the race — abandonment can also land mid-computation — but an
/// attempt that is already abandoned when it gets there must stop.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn abandoned_attempt_does_not_enter_drb_computation() {
    let (membership, coordinator) = setup(LoadBehavior::HangDrbFetchUntilReleased);
    let root_leaf = Leaf2::<WedgeTypes>::genesis(
        &TestValidatedState::default(),
        &TestInstanceState::default(),
        Version { major: 0, minor: 1 },
    )
    .await;
    membership
        .root_leaf
        .set(root_leaf)
        .expect("root_leaf seeded once");
    // Counts entries into the local computation: the difficulty selector is
    // awaited right after the computation claims its DRB maps.
    let selector_calls = Arc::new(AtomicUsize::new(0));
    let selector: DrbDifficultySelectorFn = {
        let selector_calls = Arc::clone(&selector_calls);
        Arc::new(move |_| {
            selector_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { 10 })
        })
    };
    coordinator.set_drb_difficulty_selector(selector);
    let target = EpochNumber::new(TARGET_EPOCH);

    // Starts the only attempt; it fetches the epoch roots, then parks in the
    // peer DRB fetch. No retries are ever requested, so the selector counter
    // can only be moved by this attempt.
    assert!(
        coordinator.membership_for_epoch(Some(target)).is_err(),
        "epoch {target} is not locally known, so this must not succeed"
    );
    assert!(
        wait_until(RECOVERY_BUDGET, || membership.drb_fetches() >= 1).await,
        "catchup never reached the peer DRB fetch"
    );

    // Block until the watchdog abandons the parked attempt: abandonment
    // broadcasts an error on the epoch's channel, which is exactly what
    // `wait_for_catchup` listens to — and it never spawns attempts, so it
    // cannot mask the zombie with a legitimate retry's computation.
    let _ = coordinator.wait_for_catchup(target).await;

    // Unpark the abandoned attempt: its DRB fetch fails, leaving it at the
    // entrance of the local computation.
    membership.release_drb.notify_one();

    // The resumed attempt must stop instead of computing.
    let entered = wait_until(Duration::from_millis(500), || {
        selector_calls.load(Ordering::SeqCst) >= 1
    })
    .await;
    assert!(
        !entered,
        "an abandoned catchup attempt entered the local DRB computation: it would hash \
         unsupervised (the chain is exempt from the watchdog budget) while every retry dies on \
         the DRB-in-progress guard"
    );
}
