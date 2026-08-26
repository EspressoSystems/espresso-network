//! Reproduces the mainnet epoch-catchup wedge of 2026-08-14 against the real
//! production membership (`EpochCommittees`) and the real
//! `EpochMembershipCoordinator`. No production code is modified and no
//! membership is faked — the only test-local piece is a `MembershipPersistence`
//! stand-in for Postgres, and the condition it simulates is a **stalled
//! persistence query**, which is an ordinary production event (connection
//! black-holed, table lock held by a long transaction, pool exhausted).
//!
//! Why that condition wedges the node:
//!
//! * `EpochCommittees::load_stake_table` takes a process-wide
//!   `load_from_storage_lock`, then `fetcher.persistence`, then awaits
//!   `persistence.load_stake(epoch)` — see
//!   `crates/espresso/types/src/v0/impls/committee.rs`.
//! * `EpochMembershipCoordinator::catchup` calls `load_stake_table` from its
//!   epoch-discovery loop (`crates/hotshot/types/src/epoch_membership.rs`),
//!   *before* it logs `Fetching stake tables for epochs: …`.
//! * A parked task there never removes its `catchup_map` entry and never runs
//!   `catchup_cleanup`. Since `stake_table_for_epoch` / `membership_for_epoch`
//!   short-circuit on `Entry::Occupied`, every later request for that epoch is
//!   answered `Catchup already in progress` for the life of the process. There
//!   is no timeout, no eviction and no retry.
//!
//! That is exactly what mainnet validator 79anvi logged for three days: only
//! `Stake table for epoch EpochNumber(N) unavailable. Catchup already in
//! progress`, with no `Fetching stake tables`, no success removal and no
//! `catchup for epoch … failed … Canceling catchup`. Every view it was elected
//! to lead timed out until it was restarted.

use std::{
    collections::HashSet,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use alloy::primitives::{Address, U256};
use async_lock::Mutex as AsyncMutex;
use async_trait::async_trait;
use espresso_types::{
    AuthenticatedValidatorMap, ChainConfig, EpochCommittees, Header, L1Client, SeqTypes,
    StakeTableHash,
    mock::MockStateCatchup,
    traits::{EventsPersistenceRead, MembershipPersistence, StakeTuple},
    v0_3::{EventKey, Fetcher, IndexedStake, RegisteredValidator, RewardAmount, StakeTableEvent},
};
use hotshot_example_types::storage_types::TestStorage;
use hotshot_types::{
    PeerConfig, ValidatorConfig, data::EpochNumber, drb::DrbResult,
    epoch_membership::EpochMembershipCoordinator, signature_key::BLSPubKey,
    traits::election::Membership,
};
use indexmap::IndexMap;
use parking_lot::Mutex;

/// Blocks per epoch, matching the other `EpochCommittees` tests.
const EPOCH_HEIGHT: u64 = 100;

/// `set_first_epoch` seeds stake tables for this epoch and the next, so epochs
/// 1 and 2 resolve locally and everything from 3 up needs catchup.
const FIRST_EPOCH: u64 = 1;

/// Epoch the test asks for. Its discovery loop starts at `TARGET_EPOCH - 1`,
/// which has no local snapshot, so the very first thing `catchup` does is call
/// `load_stake_table` — the await that parks.
const TARGET_EPOCH: u64 = 5;

/// A second, unrelated epoch, used to show the blast radius.
const OTHER_EPOCH: u64 = 8;

/// How long a caller waits for the coordinator to notice that an in-flight
/// catchup is making no progress. Far longer than any real catchup here.
const RECOVERY_BUDGET: Duration = Duration::from_secs(3);

/// Stands in for the node's Postgres. `load_stake` never returns, modelling a
/// query that has stalled; every other method answers immediately.
#[derive(Debug, Default)]
struct StalledStakeStore {
    /// Epochs `load_stake` has been entered for, in call order.
    load_stake_epochs: Mutex<Vec<u64>>,
    load_stake_calls: AtomicUsize,
}

impl StalledStakeStore {
    fn calls(&self) -> usize {
        self.load_stake_calls.load(Ordering::SeqCst)
    }

    fn epochs_seen(&self) -> HashSet<u64> {
        self.load_stake_epochs.lock().iter().copied().collect()
    }

    /// How many times `load_stake` was entered for `epoch`. A single catchup
    /// attempt loads each epoch at most once, so a second load of the same
    /// epoch proves a fresh attempt ran.
    fn loads_of(&self, epoch: u64) -> usize {
        self.load_stake_epochs
            .lock()
            .iter()
            .filter(|e| **e == epoch)
            .count()
    }
}

#[async_trait]
impl MembershipPersistence for StalledStakeStore {
    async fn load_stake(&self, epoch: EpochNumber) -> anyhow::Result<Option<StakeTuple>> {
        self.load_stake_calls.fetch_add(1, Ordering::SeqCst);
        self.load_stake_epochs.lock().push(*epoch);
        tracing::info!(%epoch, "load_stake entered; stalling (models a hung DB query)");
        std::future::pending::<anyhow::Result<Option<StakeTuple>>>().await
    }

    async fn load_latest_stake(&self, _limit: u64) -> anyhow::Result<Option<Vec<IndexedStake>>> {
        Ok(None)
    }

    async fn load_drb_result(&self, _epoch: EpochNumber) -> anyhow::Result<Option<DrbResult>> {
        Ok(None)
    }

    async fn load_epoch_root(&self, _epoch: EpochNumber) -> anyhow::Result<Option<Header>> {
        Ok(None)
    }

    async fn store_stake(
        &self,
        _epoch: EpochNumber,
        _stake: AuthenticatedValidatorMap,
        _block_reward: Option<RewardAmount>,
        _stake_table_hash: Option<StakeTableHash>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn store_events(
        &self,
        _l1_finalized: u64,
        _events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn load_events(
        &self,
        _from_l1_block: u64,
        _l1_finalized: u64,
    ) -> anyhow::Result<(
        Option<EventsPersistenceRead>,
        Vec<(EventKey, StakeTableEvent)>,
    )> {
        Ok((None, vec![]))
    }

    async fn delete_stake_tables(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn store_all_validators(
        &self,
        _epoch: EpochNumber,
        _all_validators: IndexMap<Address, RegisteredValidator<BLSPubKey>>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn load_all_validators(
        &self,
        _epoch: EpochNumber,
        _offset: u64,
        _limit: u64,
    ) -> anyhow::Result<Vec<RegisteredValidator<BLSPubKey>>> {
        Ok(vec![])
    }
}

/// Real `EpochCommittees` + real coordinator, wired to the stalled persistence.
fn setup() -> (Arc<StalledStakeStore>, EpochMembershipCoordinator<SeqTypes>) {
    let store = Arc::new(StalledStakeStore::default());
    let fetcher = Fetcher::new(
        Arc::new(MockStateCatchup::default()),
        Arc::new(AsyncMutex::new(StoreHandle(Arc::clone(&store)))),
        L1Client::new(vec!["http://localhost:3331".parse().unwrap()])
            .expect("Failed to create L1 client"),
        ChainConfig::default(),
    );

    let peers: Vec<PeerConfig<SeqTypes>> = (0..4)
        .map(|i| {
            ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
                [42u8; 32],
                i,
                U256::from(100),
                true,
            )
            .public_config()
        })
        .collect();
    let committees = EpochCommittees::new_stake(peers.clone(), peers, None, fetcher, EPOCH_HEIGHT)
        // Production-sized timeouts would dominate the test budget; the
        // stalled query still stalls far longer than a real read takes.
        .with_storage_read_timeout(Duration::from_millis(300));
    // Seeds snapshots for FIRST_EPOCH and FIRST_EPOCH + 1.
    committees.set_first_epoch(EpochNumber::new(FIRST_EPOCH), [0u8; 32]);

    let coordinator = EpochMembershipCoordinator::new(
        committees,
        EPOCH_HEIGHT,
        &TestStorage::<SeqTypes>::default(),
    );
    (store, coordinator)
}

/// `Fetcher::new` wants an owned `dyn MembershipPersistence`; this forwards to
/// the shared store so the test can read its counters.
#[derive(Debug)]
struct StoreHandle(Arc<StalledStakeStore>);

#[async_trait]
impl MembershipPersistence for StoreHandle {
    async fn load_stake(&self, epoch: EpochNumber) -> anyhow::Result<Option<StakeTuple>> {
        self.0.load_stake(epoch).await
    }
    async fn load_latest_stake(&self, limit: u64) -> anyhow::Result<Option<Vec<IndexedStake>>> {
        self.0.load_latest_stake(limit).await
    }
    async fn load_drb_result(&self, epoch: EpochNumber) -> anyhow::Result<Option<DrbResult>> {
        self.0.load_drb_result(epoch).await
    }
    async fn load_epoch_root(&self, epoch: EpochNumber) -> anyhow::Result<Option<Header>> {
        self.0.load_epoch_root(epoch).await
    }
    async fn store_stake(
        &self,
        epoch: EpochNumber,
        stake: AuthenticatedValidatorMap,
        block_reward: Option<RewardAmount>,
        stake_table_hash: Option<StakeTableHash>,
    ) -> anyhow::Result<()> {
        self.0
            .store_stake(epoch, stake, block_reward, stake_table_hash)
            .await
    }
    async fn store_events(
        &self,
        l1_finalized: u64,
        events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()> {
        self.0.store_events(l1_finalized, events).await
    }
    async fn load_events(
        &self,
        from_l1_block: u64,
        l1_finalized: u64,
    ) -> anyhow::Result<(
        Option<EventsPersistenceRead>,
        Vec<(EventKey, StakeTableEvent)>,
    )> {
        self.0.load_events(from_l1_block, l1_finalized).await
    }
    async fn delete_stake_tables(&self) -> anyhow::Result<()> {
        self.0.delete_stake_tables().await
    }
    async fn store_all_validators(
        &self,
        epoch: EpochNumber,
        all_validators: IndexMap<Address, RegisteredValidator<BLSPubKey>>,
    ) -> anyhow::Result<()> {
        self.0.store_all_validators(epoch, all_validators).await
    }
    async fn load_all_validators(
        &self,
        epoch: EpochNumber,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<RegisteredValidator<BLSPubKey>>> {
        self.0.load_all_validators(epoch, offset, limit).await
    }
}

/// Poll `cond` until it holds or `budget` elapses.
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

/// One stalled persistence query pins the epoch's `catchup_map` entry forever:
/// the coordinator neither serves the epoch nor abandons the dead attempt, so
/// no retry can ever happen.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn catchup_retries_after_persistence_query_stalls() {
    let (store, coordinator) = setup();
    let target = EpochNumber::new(TARGET_EPOCH);

    let Err(err) = coordinator.stake_table_for_epoch(Some(target)) else {
        panic!("epoch {target} has no local snapshot, so this must not succeed")
    };
    assert!(
        format!("{err:?}").contains("Starting catchup"),
        "first request should have started catchup, got: {err:?}"
    );

    // The spawned catchup task is now parked inside load_stake.
    assert!(
        wait_until(RECOVERY_BUDGET, || store.calls() >= 1).await,
        "catchup never reached the persistence layer"
    );
    let Err(err) = coordinator.stake_table_for_epoch(Some(target)) else {
        panic!("epoch {target} must still be unavailable")
    };
    assert!(
        format!("{err:?}").contains("Catchup already in progress"),
        "expected an in-progress error, got: {err:?}"
    );

    // The stalled query will not come back. The attempt must not park on it
    // forever; it has to give up so a fresh attempt can run — a fresh attempt
    // is proven by a *second* load of the same epoch, since one attempt loads
    // each epoch at most once. Otherwise the node is wedged until it is
    // restarted and every view it leads times out.
    let stalled_epoch = TARGET_EPOCH - 1;
    let retried = wait_until(RECOVERY_BUDGET, || {
        let _ = coordinator.stake_table_for_epoch(Some(target));
        store.loads_of(stalled_epoch) >= 2
    })
    .await;

    assert!(
        retried,
        "catchup for {target} was never retried in {RECOVERY_BUDGET:?}: the catchup_map entry is \
         never evicted when load_stake_table parks, so stake_table_for_epoch answers \"Catchup \
         already in progress\" forever (loads of epoch {stalled_epoch} = {})",
        store.loads_of(stalled_epoch)
    );
}

/// Blast radius: `load_stake_table` serializes on a process-wide lock, so the
/// same stalled query also makes every *other* epoch permanently unserviceable
/// — a request for an unrelated epoch cannot even reach storage.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unrelated_epoch_still_reaches_storage_while_one_query_stalls() {
    let (store, coordinator) = setup();
    let target = EpochNumber::new(TARGET_EPOCH);
    let other = EpochNumber::new(OTHER_EPOCH);

    assert!(coordinator.stake_table_for_epoch(Some(target)).is_err());
    assert!(
        wait_until(RECOVERY_BUDGET, || store.calls() >= 1).await,
        "catchup never reached the persistence layer"
    );
    let stalled_epoch = TARGET_EPOCH - 1;
    assert!(
        store.epochs_seen().contains(&stalled_epoch),
        "expected the stall on epoch {stalled_epoch}, saw {:?}",
        store.epochs_seen()
    );

    // A different epoch, whose catchup shares nothing with the first except the
    // storage lock. Its discovery loop starts at OTHER_EPOCH - 1, so a load of
    // that epoch proves the unrelated catchup got through the locks to storage.
    assert!(coordinator.stake_table_for_epoch(Some(other)).is_err());
    let other_walk_epoch = OTHER_EPOCH - 1;
    let progressed = wait_until(RECOVERY_BUDGET, || {
        let _ = coordinator.stake_table_for_epoch(Some(other));
        store.epochs_seen().contains(&other_walk_epoch)
    })
    .await;

    assert!(
        progressed,
        "catchup for the unrelated epoch {other} never reached storage in {RECOVERY_BUDGET:?}: \
         one stalled query holds load_from_storage_lock and fetcher.persistence, so every epoch's \
         catchup is blocked, not just {target} (epochs seen = {:?})",
        store.epochs_seen()
    );
}
