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
//! while never emitting anything about fetching a stake table or about the
//! catchup failing. That combination means the spawned `catchup()` task was
//! parked while looking for the epochs to fetch, and therefore never released
//! its claim on the requested epoch. Since a claim is what makes
//! `stake_table_for_epoch` and `membership_for_epoch` report a catchup in
//! progress, it pinned the coordinator for the lifetime of the process: no
//! timeout, no eviction, no retry. Every view in which such a node is elected
//! leader then times out.
//!
//! The tests below drive `EpochMembershipCoordinator` directly with a
//! membership that misbehaves in one specific way, and assert that a stuck
//! attempt is eventually given up on and retried — including for the
//! intermediate epochs it had claimed on its way to the requested one — while
//! an attempt that is merely slow is left alone. The last few cover what the
//! walk toward an epoch fetches, which bounds the work an unreachable epoch
//! can cost.

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
/// `catchup()` enters its walk rather than short-circuiting.
const TARGET_EPOCH: u64 = 5;

/// How long a caller is willing to wait for the coordinator to notice that an
/// in-flight catchup is not making progress. Far longer than any legitimate
/// catchup in a unit test.
const RECOVERY_BUDGET: Duration = Duration::from_secs(3);

type InnerMembership = StrictMembership<WedgeTypes, StaticStakeTable<BLSPubKey, SchnorrPubKey>>;

/// How the stake-table load misbehaves.
#[derive(Clone, Copy, Debug)]
enum LoadBehavior {
    /// Never returns — models `load_stake_table` blocking on the persistence
    /// lock, which is what parked the mainnet nodes inside the walk.
    Hang,
    /// Panics on the first call, killing the spawned catchup task before it can
    /// release its claim. Models any way the task can die unexpectedly.
    PanicOnce,
    /// The load itself returns quickly (with "not found"), letting the walk
    /// claim the epoch it is fetching, but the epoch-root fetch then never
    /// returns. Parks the task one step later than [`Hang`](Self::Hang): after
    /// it has taken a claim it will never release on its own.
    HangEpochRoot,
    /// Loads fail fast and epoch roots resolve, but the DRB result has to be
    /// computed locally, with a difficulty calibrated to several step timeouts
    /// of real hashing — modelling mainnet's tens-of-minutes DRB
    /// computation. A healthy-but-slow attempt like this must not be
    /// abandoned.
    SlowDrb,
    /// Loads fail fast and epoch roots resolve (as in
    /// [`SlowDrb`](Self::SlowDrb)), but the attempt dies inside the local DRB
    /// computation — the test installs a difficulty selector that panics on
    /// its first call, which is awaited after the computation has claimed its
    /// `drb_computations` entry. That entry must be released, or no retry
    /// can ever compute this epoch's DRB.
    PanicInDrb,
    /// Loads fail fast, epoch roots resolve, and the local DRB computation
    /// completes instantly — but persisting the result never returns (the
    /// test installs a hung `store_drb_result_fn`). The attempt parks in the
    /// write still holding its `drb_computations` entry, so no retry can
    /// compute the DRB either; the epoch must resolve from the in-memory
    /// result anyway.
    HangDrbStore,
    /// Loads report "not found", epoch roots resolve, and peers serve DRB
    /// results, so a catchup runs to completion. Lets a test look at which
    /// epochs a walk fetched rather than at how it failed.
    Reachable,
    /// Nothing can be served: loads report "not found" and both the
    /// epoch-root and DRB fetches fail at once. Lets a test count what a
    /// catchup toward an epoch nobody has attempts before giving up.
    Unavailable,
}

/// A `Membership` that delegates everything to `StrictMembership` except the
/// stake-table load, which misbehaves per [`LoadBehavior`], and the epoch-root
/// and DRB fetches, which fail cleanly so a retry has a defined outcome.
#[derive(Clone, Debug)]
struct WedgeMembership {
    inner: Arc<InnerMembership>,
    behavior: LoadBehavior,
    /// Number of times `load_stake_table` has been entered. One per catchup
    /// attempt that reached the walk.
    load_calls: Arc<AtomicUsize>,
    /// Number of times `get_epoch_root` has been entered. One per catchup
    /// attempt that got past the walk.
    root_calls: Arc<AtomicUsize>,
    /// Template leaf returned by `get_epoch_root` under
    /// [`SlowDrb`](LoadBehavior::SlowDrb), seeded by the test up front:
    /// `Leaf2::genesis` pays a multi-second one-time global setup cost on
    /// first use, which must not happen inside a step timeout.
    root_leaf: Arc<std::sync::OnceLock<Leaf2<WedgeTypes>>>,
    /// Number of times `get_epoch_drb` has been entered. One per catchup
    /// attempt that got past the epoch-root fetches.
    drb_calls: Arc<AtomicUsize>,
}

impl WedgeMembership {
    fn attempts(&self) -> usize {
        self.load_calls.load(Ordering::SeqCst)
    }

    fn root_fetches(&self) -> usize {
        self.root_calls.load(Ordering::SeqCst)
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

    fn highest_known_epoch(&self) -> Option<EpochNumber> {
        self.inner.highest_known_epoch()
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
            | LoadBehavior::Reachable => {
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
            LoadBehavior::Hang | LoadBehavior::PanicOnce | LoadBehavior::Unavailable => {
                Err(anyhow!("epoch root unavailable").into())
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
        if matches!(self.behavior, LoadBehavior::Reachable) {
            return Ok(INITIAL_DRB_RESULT);
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
            LoadBehavior::PanicOnce
            | LoadBehavior::HangEpochRoot
            | LoadBehavior::SlowDrb
            | LoadBehavior::PanicInDrb
            | LoadBehavior::HangDrbStore
            | LoadBehavior::Reachable
            | LoadBehavior::Unavailable => false,
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
        drb_calls: Arc::default(),
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

/// Build the leaf `get_epoch_root` hands back and seed the membership with it.
///
/// `Leaf2::genesis` pays a one-time global setup cost of several seconds, so
/// tests that care about timing build it before the clock matters.
async fn seed_root_leaf(membership: &WedgeMembership) -> Leaf2<WedgeTypes> {
    let leaf = Leaf2::<WedgeTypes>::genesis(
        &TestValidatedState::default(),
        &TestInstanceState::default(),
        Version { major: 0, minor: 1 },
    )
    .await;
    membership
        .root_leaf
        .set(leaf.clone())
        .expect("root_leaf seeded once");
    leaf
}

/// Register a stake table for `epoch` without a DRB result, the way an epoch
/// root fetched from peers would. Models what `reload_stake` leaves behind.
async fn register_stake_table(
    membership: &WedgeMembership,
    coordinator: &EpochMembershipCoordinator<WedgeTypes>,
    template: &Leaf2<WedgeTypes>,
    epoch: u64,
) {
    let mut header = template.block_header().clone();
    // An epoch root registers the epoch two later than the one it sits in.
    header.block_number = root_block_in_epoch(epoch - 2, EPOCH_HEIGHT);
    membership
        .add_epoch_root(header, coordinator)
        .await
        .expect("add_epoch_root");
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
/// parked in its walk, never removes its claim,
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

    // Wait until the spawned task is actually parked in the walk.
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
        "catchup for {target} was never retried in {RECOVERY_BUDGET:?}: the claim is never \
         evicted when the in-flight attempt hangs, so stake_table_for_epoch answers \"Catchup \
         already in progress\" forever (attempts = {})",
        membership.attempts()
    );
}

/// Same wedge reached a different way: the spawned catchup task dies before it
/// release its claim, so the claim is orphaned. Nothing owns it and nothing
/// will ever remove it.
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
         without releasing its claim (attempts = {})",
        membership.attempts()
    );
}

/// The wedge one level deeper: on its way to the requested epoch, a catchup
/// claims the intermediate epochs it fetches. If the attempt is then
/// abandoned, evicting only the requested epoch's claim leaves the
/// intermediate ones orphaned, and every `stake_table_for_epoch` for those
/// epochs is answered "Catchup already in progress" for the lifetime of the
/// process.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn intermediate_epochs_recover_after_catchup_abandoned() {
    let (membership, coordinator) = setup(LoadBehavior::HangEpochRoot);
    let target = EpochNumber::new(TARGET_EPOCH);
    // The walk claims the epochs it fetches one at a time from the earliest
    // one missing, so the epoch it is parked on is the first gap after the
    // seeded pair.
    let intermediate = EpochNumber::new(FIRST_EPOCH + 2);

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

/// The inverse guarantee: an attempt that is merely *slow* — computing a DRB
/// locally, which on mainnet is 25e9 sequential hashes, i.e. tens of minutes
/// by design — must NOT be abandoned. Abandoning it would spawn retries that
/// re-fetch epoch roots from peers only to die on the "DRB calculation
/// already in progress" guard, a storm of doomed catchups for as long as the
/// real computation runs.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn slow_drb_computation_is_not_abandoned() {
    let (membership, coordinator) = setup(LoadBehavior::SlowDrb);
    // A tighter step timeout for this test: compile profiles skew the calibration
    // probe below against the real hash loop by several ×, and the chain must
    // outlast several windows whichever way the skew goes.
    let step_timeout = Duration::from_millis(200);
    let coordinator = coordinator.with_catchup_timeout(step_timeout);
    // Built up front: the first `Leaf2::genesis` pays a one-time global setup
    // cost of several seconds, which must not eat into a step timeout.
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
    // The slow part must be the hash chain itself — the only step that reports
    // progress often enough for a step timeout to keep waiting. Calibrate a difficulty targeting 8 s
    // of hashing as measured by this probe; the real chain lands anywhere
    // from ~1 s (probe unoptimized, chain optimized) to ~8 s (both
    // optimized), which is several step timeouts either way. The probe is
    // the fastest of three runs: a transient CI load spike can only slow a
    // run, which would deflate the difficulty and let the chain finish
    // inside the step timeout (tripping the elapsed check below), while
    // sustained load slows probe and chain alike and cancels out.
    let probe_iters: u32 = 200_000;
    let mut probe = [0u8; 32];
    let per_iter = (0..3)
        .map(|_| {
            let start = Instant::now();
            for _ in 0..probe_iters {
                probe = Sha256::digest(probe).into();
            }
            start.elapsed() / probe_iters
        })
        .min()
        .expect("three probe runs");
    std::hint::black_box(probe);
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

    // The attempt fetches the epoch roots for 3, 4 and 5, then the root of 5
    // once more to seed the DRB, and sits in the computation. It must be left
    // alone until it resolves the epoch.
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
        started.elapsed() >= 2 * step_timeout,
        "the hash chain finished before a step timeout could fire, so this run proved nothing; \
         raise the calibration target"
    );
    assert_eq!(
        membership.root_fetches(),
        4,
        "a step timeout abandoned the attempt during its legitimate DRB computation: retries were \
         spawned that re-fetched epoch roots only to die on the DRB-in-progress guard"
    );
}

/// An attempt that dies *inside* the local DRB computation must release the
/// `drb_computations` entry it claimed. That entry is what stops a retry
/// from starting a second concurrent hash chain, so if the dead attempt keeps
/// it, every retry fails with "DRB calculation already in progress" and the
/// epoch can never obtain a DRB locally again.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn drb_state_is_released_when_attempt_dies_computing() {
    let (membership, coordinator) = setup(LoadBehavior::PanicInDrb);
    // Built up front: the first `Leaf2::genesis` pays a one-time global setup
    // cost of several seconds, which must not eat into a step timeout.
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
    // its `drb_computations` entry and the hash chain, so a panic in it is
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
    // dead attempt never released its drb_computations entry, so every
    // retry dies on the DRB-in-progress guard and the epoch never resolves.
    let resolved = wait_until(RECOVERY_BUDGET, || {
        coordinator.membership_for_epoch(Some(target)).is_ok()
    })
    .await;
    assert!(
        resolved,
        "no retry could compute the DRB for {target} within {RECOVERY_BUDGET:?}: the attempt that \
         died mid-computation leaked its drb_computations entry (selector calls = {}, root \
         fetches = {})",
        selector_calls.load(Ordering::SeqCst),
        membership.root_fetches()
    );
}

/// A stalled DRB *write* must not stall epoch resolution. The computation
/// itself completes, but persisting the result never returns, so the attempt
/// parks in the write still holding its `drb_computations` entry until the
/// a step timeout abandons it. The computed result must already be in the
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
    // cost of several seconds, which must not eat into a step timeout.
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
    // outlasting a step timeout is the stalled write.
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
    // Abandonment must also release the epoch's DRB claim: the attempt is
    // parked in the write for good, so a claim it keeps is kept forever and
    // every later attempt to compute this epoch's DRB locally dies on the
    // DRB-in-progress guard.
    let released = wait_until(RECOVERY_BUDGET, || {
        coordinator.drb_cancel_token(target).is_none()
    })
    .await;
    assert!(
        released,
        "the abandoned attempt still holds the DRB claim for epoch {target} while parked in the \
         stalled result write"
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

/// Catchup toward an epoch nobody can serve attempts the first epoch this node
/// does not hold and gives up there. The requested epoch arrives on
/// certificates that do not commit to it, so it must not scale the work.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn catchup_to_unreachable_epoch_stops_at_the_first_gap() {
    let (membership, coordinator) = setup(LoadBehavior::Unavailable);
    let target = EpochNumber::new(u64::MAX);

    let result = tokio::time::timeout(RECOVERY_BUDGET, coordinator.wait_for_stake_table(target))
        .await
        .expect("catchup gives up rather than walking every epoch below the target");

    assert!(
        result.is_err(),
        "epoch {target} cannot be served, so this must not succeed"
    );
    assert_eq!(
        membership.root_fetches(),
        1,
        "catchup must attempt the first epoch it is missing and stop there"
    );
}

/// A gap in what this node holds is bridged: the walk resumes from the latest
/// consecutive pair rather than from the latest epoch, and fetches everything
/// between that pair and the target. Persistence with a hole near the tip
/// would otherwise leave a step without the epoch root it derives from.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn catchup_bridges_a_gap_in_what_the_node_holds() {
    let (membership, coordinator) = setup(LoadBehavior::Reachable);
    let template = seed_root_leaf(&membership).await;
    // Holds 1 and 2 (seeded) and 5, but neither 3 nor 4.
    register_stake_table(&membership, &coordinator, &template, 5).await;
    let target = EpochNumber::new(6);

    tokio::time::timeout(RECOVERY_BUDGET, coordinator.wait_for_stake_table(target))
        .await
        .expect("catchup completes")
        .map_err(|err| format!("catchup failed: {err:?}"))
        .unwrap();

    for epoch in [3, 4, 6] {
        assert!(
            membership.snapshot(EpochNumber::new(epoch)).is_some(),
            "epoch {epoch} was not fetched"
        );
    }
    assert_eq!(
        membership.root_fetches(),
        3,
        "the walk must fetch 3, 4 and 6, and pass over the 5 it already holds"
    );
}

/// Two catchups toward different epochs share one attempt per epoch, so an
/// epoch they both walk is fetched once.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn concurrent_catchups_fetch_each_epoch_once() {
    let (membership, coordinator) = setup(LoadBehavior::Reachable);
    seed_root_leaf(&membership).await;
    let (near, far) = (EpochNumber::new(5), EpochNumber::new(6));

    assert!(coordinator.membership_for_epoch(Some(near)).is_err());
    assert!(coordinator.membership_for_epoch(Some(far)).is_err());

    let done = wait_until(RECOVERY_BUDGET, || {
        coordinator.membership_for_epoch(Some(near)).is_ok()
            && coordinator.membership_for_epoch(Some(far)).is_ok()
    })
    .await;
    assert!(done, "both catchups complete");

    assert_eq!(
        membership.root_fetches(),
        4,
        "epochs 3 to 6 must be fetched once each, however the two walks interleave"
    );
}

/// An epoch whose stake table this node already holds but whose DRB result it
/// does not needs no epoch root at all: the walk passes over the epochs it
/// holds and the DRB comes from peers.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn catchup_for_a_held_stake_table_fetches_no_epoch_root() {
    let (membership, coordinator) = setup(LoadBehavior::Reachable);
    let template = seed_root_leaf(&membership).await;
    register_stake_table(&membership, &coordinator, &template, 3).await;
    register_stake_table(&membership, &coordinator, &template, 4).await;
    let target = EpochNumber::new(4);

    // The stake table is there, the DRB result is not.
    assert!(
        coordinator.stake_table_for_epoch(Some(target)).is_ok(),
        "the stake table for epoch {target} was registered above"
    );
    assert!(
        coordinator.membership_for_epoch(Some(target)).is_err(),
        "epoch {target} has no DRB result yet"
    );

    let resolved = wait_until(RECOVERY_BUDGET, || {
        coordinator.membership_for_epoch(Some(target)).is_ok()
    })
    .await;
    assert!(resolved, "catchup did not supply the DRB result");

    assert_eq!(
        membership.root_fetches(),
        0,
        "nothing had to be derived, so no epoch root should have been fetched"
    );
}
