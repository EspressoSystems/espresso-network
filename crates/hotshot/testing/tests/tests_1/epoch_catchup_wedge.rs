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
//! Both tests below drive `EpochMembershipCoordinator` directly with a
//! membership whose stake-table load never completes (or whose catchup task
//! dies), and assert that the coordinator eventually gives up on the stuck
//! attempt and tries again. On unfixed code they fail: exactly one attempt is
//! ever made.

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
    drb::{DrbResult, INITIAL_DRB_RESULT},
    epoch_membership::EpochMembershipCoordinator,
    signature_key::{BLSPubKey, BuilderKey, SchnorrPubKey},
    traits::{
        election::Membership, node_implementation::NodeType, signature_key::StakeTableEntryType,
    },
    upgrade_config::UpgradeConstants,
};

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

type InnerMembership = StrictMembership<WedgeTypes, StaticStakeTable<BLSPubKey, SchnorrPubKey>>;

/// How the stake-table load misbehaves.
#[derive(Clone, Copy, Debug)]
enum LoadBehavior {
    /// Never returns — models `load_stake_table` blocking on the persistence
    /// lock, which is what parked the mainnet nodes inside the discovery loop.
    Hang,
    /// Panics on the first call, killing the spawned catchup task before it can
    /// run `catchup_cleanup`. Models any way the task can die (panic, cancel).
    PanicOnce,
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
}

impl WedgeMembership {
    fn attempts(&self) -> usize {
        self.load_calls.load(Ordering::SeqCst)
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
        _e: EpochNumber,
        _coordinator: &EpochMembershipCoordinator<WedgeTypes>,
    ) -> Result<Leaf2<WedgeTypes>, Self::Error> {
        Err(anyhow!("epoch root unavailable").into())
    }

    async fn get_epoch_drb(
        &self,
        _e: EpochNumber,
        _coordinator: &EpochMembershipCoordinator<WedgeTypes>,
    ) -> Result<DrbResult, Self::Error> {
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
            LoadBehavior::PanicOnce => false,
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
    };
    // Registers stake tables for FIRST_EPOCH and FIRST_EPOCH + 1.
    membership.set_first_epoch(EpochNumber::new(FIRST_EPOCH), INITIAL_DRB_RESULT);

    let coordinator = EpochMembershipCoordinator::new(
        membership.clone(),
        EPOCH_HEIGHT,
        &TestStorage::<WedgeTypes>::default(),
    );
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
