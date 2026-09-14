use std::{
    collections::{HashMap, hash_map::Entry},
    sync::Arc,
    time::Duration,
};

use alloy_primitives::U256;
use committable::Commitment;
use either::Either;
use futures::future::{BoxFuture, FutureExt, Shared};
use hotshot_utils::anytrace::{self, Wrap};
use parking_lot::{Mutex, RwLock};
use sha2::{Digest, Sha256};
use tokio::{spawn, time::timeout};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use versions::DRB_FIX_VERSION;

use crate::{
    PeerConfig, PeerConnectInfo,
    data::{BlockNumber, EpochNumber, Leaf2, ViewNumber},
    drb::{
        DRB_PROGRESS_LOAD_TIMEOUT, DrbDifficultySelectorFn, DrbInput, DrbResult, Heartbeat,
        compute_drb_result,
    },
    traits::{
        block_contents::BlockHeader,
        election::{Membership, MembershipSnapshot, NonEpochMembershipSnapshot},
        node_implementation::NodeType,
        signature_key::StakeTableEntryType,
        storage::{
            LoadDrbProgressFn, Storage, StoreDrbProgressFn, StoreDrbResultFn, load_drb_progress_fn,
            store_drb_progress_fn, store_drb_result_fn,
        },
    },
};

type Result<T> = anytrace::Result<T>;

/// One in-flight attempt, shared by every caller that asks for the same epoch
/// while it runs. Whoever polls it drives it, so it outlives the task that
/// started it as long as somebody is still waiting.
type Attempt = Shared<BoxFuture<'static, Result<()>>>;

/// The in-flight DRB computations, each with the token that stops it. An entry
/// is the claim on its epoch: one computation runs per epoch, and the next may
/// start only once the entry is gone.
///
/// When an external source supplies the DRB result for `epoch` (e.g. a decided
/// leaf carrying `next_drb_result`), `supply_drb` fires the token so the local
/// computation can stop early instead of grinding to completion.
type DrbComputations = Arc<Mutex<HashMap<EpochNumber, CancellationToken>>>;

/// The per-epoch snapshot type associated with `T::Membership`.
type Snapshot<T> = <<T as NodeType>::Membership as Membership<T>>::Snapshot;

/// The stake-table hash type associated with `T::Membership`'s per-epoch
/// snapshot.
type SnapshotStakeTableHash<T> = <Snapshot<T> as MembershipSnapshot<T>>::StakeTableHash;

/// Struct to Coordinate membership catchup
pub struct EpochMembershipCoordinator<TYPES: NodeType> {
    membership: Arc<TYPES::Membership>,
    catchup_map: Attempts,
    stake_table_map: Attempts,
    drb_computations: DrbComputations,
    epoch_height: BlockNumber,
    catchup_timeout: Duration,
    store_drb_progress_fn: StoreDrbProgressFn,
    load_drb_progress_fn: LoadDrbProgressFn,
    store_drb_result_fn: StoreDrbResultFn,
    drb_difficulty_selector: Arc<RwLock<Option<DrbDifficultySelectorFn>>>,
}

impl<TYPES: NodeType> Clone for EpochMembershipCoordinator<TYPES> {
    fn clone(&self) -> Self {
        Self {
            membership: Arc::clone(&self.membership),
            catchup_map: self.catchup_map.clone(),
            stake_table_map: self.stake_table_map.clone(),
            drb_computations: Arc::clone(&self.drb_computations),
            epoch_height: self.epoch_height,
            catchup_timeout: self.catchup_timeout,
            store_drb_progress_fn: Arc::clone(&self.store_drb_progress_fn),
            load_drb_progress_fn: Arc::clone(&self.load_drb_progress_fn),
            store_drb_result_fn: self.store_drb_result_fn.clone(),
            drb_difficulty_selector: Arc::clone(&self.drb_difficulty_selector),
        }
    }
}

impl<TYPES: NodeType> EpochMembershipCoordinator<TYPES> {
    pub fn new<M, S, B>(membership: M, epoch_height: B, storage: &S) -> Self
    where
        M: Into<Arc<TYPES::Membership>>,
        B: Into<BlockNumber>,
        S: Storage<TYPES>,
    {
        Self {
            membership: membership.into(),
            catchup_map: Attempts::default(),
            stake_table_map: Attempts::default(),
            drb_computations: Arc::default(),
            epoch_height: epoch_height.into(),
            catchup_timeout: DEFAULT_CATCHUP_TIMEOUT,
            store_drb_progress_fn: store_drb_progress_fn(storage.clone()),
            load_drb_progress_fn: load_drb_progress_fn(storage.clone()),
            store_drb_result_fn: store_drb_result_fn(storage.clone()),
            drb_difficulty_selector: Arc::new(RwLock::new(None)),
        }
    }

    pub fn epoch_height(&self) -> BlockNumber {
        self.epoch_height
    }

    /// Override the bound on a single catchup step. Mostly for tests.
    pub fn with_catchup_timeout(mut self, timeout: Duration) -> Self {
        self.catchup_timeout = timeout;
        self
    }

    /// Override the callback that persists a computed DRB result. For tests.
    pub fn with_store_drb_result_fn(mut self, f: StoreDrbResultFn) -> Self {
        self.store_drb_result_fn = f;
        self
    }

    /// Get a reference to the membership
    pub fn membership(&self) -> &TYPES::Membership {
        &self.membership
    }

    /// Handles notifications that a new epoch root has been created.
    pub async fn add_epoch_root(
        &self,
        header: TYPES::BlockHeader,
    ) -> std::result::Result<(), <TYPES::Membership as Membership<TYPES>>::Error> {
        self.membership.add_epoch_root(header, self).await
    }

    /// Gets the validated block header and epoch number of the epoch root
    /// at the given block height.
    pub async fn get_epoch_root(
        &self,
        epoch: EpochNumber,
    ) -> std::result::Result<Leaf2<TYPES>, <TYPES::Membership as Membership<TYPES>>::Error> {
        self.membership.get_epoch_root(epoch, self).await
    }

    /// Gets the DRB result for the given epoch.
    pub async fn get_epoch_drb(
        &self,
        epoch: EpochNumber,
    ) -> std::result::Result<DrbResult, <TYPES::Membership as Membership<TYPES>>::Error> {
        self.membership.get_epoch_drb(epoch, self).await
    }

    /// Set the DRB difficulty selector
    pub fn set_drb_difficulty_selector(&self, f: DrbDifficultySelectorFn) {
        let mut drb_difficulty_selector_writer = self.drb_difficulty_selector.write();
        *drb_difficulty_selector_writer = Some(f);
    }

    /// Get a Membership for a given Epoch, which is guaranteed to have a randomized stake
    /// table for the given Epoch
    pub fn membership_for_epoch(
        &self,
        maybe_epoch: Option<EpochNumber>,
    ) -> Result<EpochMembership<TYPES>> {
        let Some(epoch) = maybe_epoch else {
            return Ok(EpochMembership {
                coordinator: self.clone(),
                snapshot: EpochMembershipSnapshot::NonEpoch(self.membership.non_epoch_snapshot()),
            });
        };
        let Some(first_epoch) = self.membership.first_epoch() else {
            return Err(anytrace::error!(
                "membership_for_epoch called with epoch {epoch:?} but first_epoch is unset"
            ));
        };
        if epoch < first_epoch {
            return Err(anytrace::error!(
                "membership_for_epoch called with epoch {epoch:?} before first_epoch {first_epoch}"
            ));
        }
        if let Some(snapshot) = self.membership.snapshot(epoch)
            && snapshot.has_drb()
        {
            return Ok(EpochMembership {
                coordinator: self.clone(),
                snapshot: EpochMembershipSnapshot::Epoch { epoch, snapshot },
            });
        }
        if self.spawn_catchup(epoch) {
            Err(anytrace::warn!(
                "Randomized stake table for epoch {epoch:?} unavailable. Starting catchup"
            ))
        } else {
            Err(anytrace::warn!(
                "Randomized stake table for epoch {epoch:?} unavailable. Catchup already in \
                 progress"
            ))
        }
    }

    /// Get a Membership for a given Epoch, which is guaranteed to have a stake
    /// table for the given Epoch
    pub fn stake_table_for_epoch(&self, e: Option<EpochNumber>) -> Result<EpochMembership<TYPES>> {
        let Some(epoch) = e else {
            return Ok(EpochMembership {
                coordinator: self.clone(),
                snapshot: EpochMembershipSnapshot::NonEpoch(self.membership.non_epoch_snapshot()),
            });
        };
        let Some(first_epoch) = self.membership.first_epoch() else {
            return Err(anytrace::error!(
                "stake_table_for_epoch called with epoch {epoch:?} but first_epoch is unset"
            ));
        };
        if epoch < first_epoch {
            return Err(anytrace::error!(
                "stake_table_for_epoch called with epoch {epoch:?} before first_epoch \
                 {first_epoch}"
            ));
        }
        if let Some(membership) = self.epoch_membership(epoch) {
            return Ok(membership);
        }
        if self.spawn_catchup(epoch) {
            Err(anytrace::warn!(
                "Stake table for epoch {epoch:?} unavailable. Starting catchup"
            ))
        } else {
            Err(anytrace::warn!(
                "Stake table for epoch {epoch:?} unavailable. Catchup already in progress"
            ))
        }
    }

    /// Return the union of the stake table and DA committee for `epoch`,
    /// keyed by signature key. Each entry's `Option<PeerConnectInfo>`
    /// reflects whether the peer has connection info registered.
    ///
    /// For a peer in both, `Some` connect info wins over `None`: the snapshot's DA
    /// committee may be a stale bootstrap copy lacking connect info while the stake
    /// table has fresh L1-derived info (e.g. during CLIQUENET cutover), so plain
    /// last-write-wins would clobber the live data.
    ///
    /// Returns `None` if the stake table for `epoch` is unavailable
    /// (e.g. catchup is still in progress).
    pub fn epoch_peers(
        &self,
        e: Option<EpochNumber>,
    ) -> Option<HashMap<TYPES::SignatureKey, Option<PeerConnectInfo>>> {
        let membership = self.stake_table_for_epoch(e).ok()?;
        let mut out: HashMap<TYPES::SignatureKey, Option<PeerConnectInfo>> = HashMap::new();
        let mut merge =
            |key: TYPES::SignatureKey, info: Option<PeerConnectInfo>| match out.entry(key) {
                Entry::Vacant(slot) => {
                    slot.insert(info);
                },
                Entry::Occupied(mut slot) => {
                    if slot.get().is_none() && info.is_some() {
                        slot.insert(info);
                    }
                },
            };
        if let Some(snap) = membership.snapshot() {
            for m in snap.stake_table().chain(snap.da_stake_table()) {
                merge(m.stake_table_entry.public_key(), m.connect_info.clone());
            }
        } else {
            let snap = membership.non_epoch_snapshot()?;
            for m in snap.stake_table().chain(snap.da_stake_table()) {
                merge(m.stake_table_entry.public_key(), m.connect_info.clone());
            }
        }
        Some(out)
    }

    /// Collect the union of `epoch-1`, `epoch`, and `epoch+1` stake tables
    /// (each merged with its DA committee) as a flat map of peers to dial.
    ///
    /// Newest-wins ordering for `connect_info`: next overrides curr overrides
    /// prev. Entries with no `connect_info` are filtered out.
    ///
    /// Used to seed networks like cliquenet with the same window
    /// `on_epoch_change` would build for `epoch`.
    pub fn window_peers(&self, e: EpochNumber) -> HashMap<TYPES::SignatureKey, PeerConnectInfo> {
        let curr = self.epoch_peers(Some(e)).unwrap_or_default();
        let prev = if *e > 0 {
            self.epoch_peers(Some(e - 1)).unwrap_or_default()
        } else {
            HashMap::new()
        };
        let next = self.epoch_peers(Some(e + 1)).unwrap_or_default();

        // Newest-wins merge: start from prev, overlay curr and next.
        let mut merged: HashMap<TYPES::SignatureKey, Option<PeerConnectInfo>> = prev;
        for (k, v) in curr.into_iter().chain(next) {
            merged.insert(k, v);
        }

        merged
            .into_iter()
            .filter_map(|(k, v)| v.map(|info| (k, info)))
            .collect()
    }

    /// Start a catchup task for `epoch` unless one is already running.
    ///
    /// Returns `true` if this call started one.
    ///
    /// The attempt is owned by the task spawned here: a step that never returns
    /// keeps it alive, but killing that task drops the whole chain of awaits
    /// under it, releasing every lock and claim they held.
    fn spawn_catchup(&self, epoch: EpochNumber) -> bool {
        if self.stake_table_map.contains_attempt(epoch) {
            return false;
        }
        let (attempt, entry) = self.catchup_map.get_or_insert(epoch, || {
            let coordinator = self.clone();
            async move { coordinator.catchup(epoch).await }.boxed()
        });
        let Some(entry) = entry else {
            return false;
        };
        spawn(async move {
            let _entry = entry; // Dropping the entry removes the attempt.
            if let Err(err) = attempt.await {
                error!(%epoch, %err, "catchup failed");
            }
        });
        true
    }

    /// Bring `epoch` to a usable membership: its stake table, then its DRB.
    ///
    /// Walks from the latest consecutive pair of stake tables this node holds
    /// to `epoch`, fetching what is missing one epoch at a time. `epoch` is
    /// where the walk stops, not where it starts, so an epoch nobody can serve
    /// costs one fetch attempt at the first gap rather than a probe of every
    /// earlier epoch.
    ///
    /// Concurrent catchups walk the same epochs and share one attempt per
    /// epoch, so whichever reaches an epoch first fetches it while the others
    /// wait for its result. Every walk awaits attempts in increasing epoch
    /// order and an attempt awaits none, so no two can wait on each other.
    async fn catchup(&self, epoch: EpochNumber) -> Result<()> {
        let Some(first_epoch) = self.membership.first_epoch() else {
            return Err(anytrace::error!(
                "catchup requested for epoch {epoch} but the first epoch is not set"
            ));
        };

        let seeded = first_epoch + 1;

        if epoch <= seeded {
            return Err(anytrace::error!(
                "catchup requested for epoch {epoch}, which `set_first_epoch` seeds rather than \
                 deriving it from an epoch root"
            ));
        }

        let known = self.membership.highest_known_epoch().unwrap_or(seeded);

        let mut start = known
            .min(EpochNumber::new(epoch.saturating_sub(1)))
            .max(seeded);

        while start > seeded {
            if self.stake_table_present(start).await? && self.stake_table_present(start - 1).await?
            {
                break;
            }
            start = start - 1;
        }

        info!(target: "announce::catchup", %epoch, %start, "catching up to epoch");

        let mut try_epoch = start + 1;
        while try_epoch <= epoch {
            let (attempt, _entry) = self.stake_table_map.get_or_insert(try_epoch, || {
                let coordinator = self.clone();
                async move {
                    if coordinator.stake_table_present(try_epoch).await? {
                        return Ok(());
                    }
                    coordinator.fetch_stake_table(try_epoch).await
                }
                .boxed()
            });
            attempt.await.map_err(|err| {
                anytrace::error!("catchup for epoch {epoch} failed at epoch {try_epoch}: {err:?}")
            })?;
            try_epoch += 1;
        }

        self.drb_ready(epoch).await
    }

    /// The membership for `epoch`, if its stake table is in memory.
    fn epoch_membership(&self, epoch: EpochNumber) -> Option<EpochMembership<TYPES>> {
        Some(EpochMembership {
            coordinator: self.clone(),
            snapshot: EpochMembershipSnapshot::Epoch {
                epoch,
                snapshot: self.membership.snapshot(epoch)?,
            },
        })
    }

    /// Whether `epoch`'s stake table is in memory.
    ///
    /// It is read from storage if it is not already in RAM.
    async fn stake_table_present(&self, epoch: EpochNumber) -> Result<bool> {
        if self.membership.snapshot(epoch).is_some() {
            return Ok(true);
        }
        debug!(%epoch, "loading stake table from storage");
        timeout(
            self.catchup_timeout,
            self.membership.load_stake_table(epoch),
        )
        .await
        .map_err(|_| {
            anytrace::error!(
                "loading the stake table for epoch {epoch} from storage exceeded {:?}",
                self.catchup_timeout
            )
        })
    }

    /// Make `epoch`'s DRB result present.
    ///
    /// From peers if they have it, by computing it locally if they do not.
    async fn drb_ready(&self, epoch: EpochNumber) -> Result<()> {
        if self
            .membership
            .snapshot(epoch)
            .is_some_and(|snapshot| snapshot.has_drb())
        {
            return Ok(());
        }
        debug!(%epoch, "fetching drb result from peers");
        let fetched = timeout(self.catchup_timeout, self.get_epoch_drb(epoch))
            .await
            .map_err(|_| {
                anytrace::error!(
                    "fetching the DRB result for epoch {epoch} from peers exceeded {:?}",
                    self.catchup_timeout
                )
            })?;
        match fetched {
            Ok(drb_result) => {
                info!(
                    target: "announce::catchup",
                    %epoch,
                    ?drb_result,
                    "drb result retrieved from peers"
                );
                self.membership.add_drb_result(epoch, drb_result);
                Ok(())
            },
            Err(err) => {
                info!(
                    target: "announce::catchup",
                    %epoch,
                    %err,
                    "recalculating the drb result the peers lack"
                );
                let root_leaf = self.epoch_root_leaf(epoch).await?;
                self.compute_drb_result(epoch, root_leaf).await.map(|_| ())
            },
        }
    }

    /// Get the stake table for `epoch`, blocking on catchup if necessary.
    ///
    /// Unlike `stake_table_for_epoch`, this returns the result rather than
    /// kicking off catchup and immediately returning an error. Used at startup
    /// to drive the existing catchup chain synchronously before consensus is
    /// running.
    pub async fn wait_for_stake_table(&self, epoch: EpochNumber) -> Result<EpochMembership<TYPES>> {
        match self.stake_table_for_epoch(Some(epoch)) {
            Ok(mem) => Ok(mem),
            Err(_) => self.wait_for_catchup(epoch).await,
        }
    }

    /// Call this method if you think catchup is in progress for a given epoch
    /// and you want to wait for it to finish and get the stake table.
    /// If it's not, it will try to return the stake table if already available.
    /// Returns an error if the catchup failed or the catchup is not in progress
    /// and the stake table is not available.
    pub async fn wait_for_catchup(&self, epoch: EpochNumber) -> Result<EpochMembership<TYPES>> {
        let failed = |err| anytrace::error!("Catchup for epoch {epoch} failed: {err:?}");
        if let Some(attempt) = self.catchup_map.get_attempt(epoch) {
            attempt.await.map_err(failed)?;
        } else if let Some(attempt) = self.stake_table_map.get_attempt(epoch) {
            attempt.await.map_err(failed)?;
        } else if let Some(membership) = self.epoch_membership(epoch) {
            return Ok(membership);
        } else {
            return Err(anytrace::error!(
                "No catchup in progress for epoch {epoch} and we don't have a stake table for it"
            ));
        }
        self.epoch_membership(epoch).ok_or_else(|| {
            anytrace::error!("Catchup for epoch {epoch} reported success but left no stake table")
        })
    }

    /// The epoch root leaf `epoch`'s stake table is derived from.
    ///
    /// Fetched from peers and verified against the stake table of `epoch - 2`,
    /// which must already be present. Also seeds the local DRB computation, so
    /// it is fetched on that path too, without a stake table to install.
    async fn epoch_root_leaf(&self, epoch: EpochNumber) -> Result<Leaf2<TYPES>> {
        let root_epoch = EpochNumber::new(epoch.saturating_sub(2));
        // Snapshot eviction can drop the root epoch from memory while a catchup
        // is in progress, so load it back from local storage. Deliberately not
        // `stake_table_for_epoch`, which on a miss claims the root epoch and
        // spawns a nested catchup for it: that is how one step turns into a
        // cascade.
        self.stake_table_present(root_epoch).await?;
        let Some(root_membership) = self.epoch_membership(root_epoch) else {
            return Err(anytrace::error!(
                "the stake table for epoch {epoch} is derived from epoch {root_epoch}, which is \
                 in neither memory nor local storage"
            ));
        };

        // Verification of the root is handled in get_epoch_root_and_drb
        debug!(%epoch, %root_epoch, "fetching epoch root");
        let fetched = timeout(self.catchup_timeout, root_membership.get_epoch_root())
            .await
            .map_err(|_| {
                anytrace::error!(
                    "fetching the epoch root of epoch {epoch} exceeded {:?}",
                    self.catchup_timeout
                )
            })?;
        fetched.map_err(|err| {
            anytrace::error!("get epoch root leaf failed for epoch {root_epoch:?}: {err:#}")
        })
    }

    /// Fetch `epoch`'s stake table and add it to the membership.
    async fn fetch_stake_table(&self, epoch: EpochNumber) -> Result<()> {
        let root_leaf = self.epoch_root_leaf(epoch).await?;

        debug!(%epoch, "adding epoch root to membership");
        timeout(
            self.catchup_timeout,
            self.add_epoch_root(root_leaf.block_header().clone()),
        )
        .await
        .map_err(|_| {
            anytrace::error!(
                "adding the epoch root of epoch {epoch} to membership exceeded {:?}",
                self.catchup_timeout
            )
        })?
        .map_err(|e| {
            anytrace::error!("Failed to add epoch root for epoch {epoch:?} to membership: {e}")
        })
    }

    /// Compute `epoch`'s DRB result locally, claiming the epoch for the
    /// duration so that only one computation runs per epoch.
    ///
    /// Bounded by progress rather than by time: the hash chain is
    /// purposefully long, so it is given as long as it keeps hashing and
    /// abandoned once it stops, which a chain queued behind a saturated
    /// blocking pool does.
    pub async fn compute_drb_result(
        &self,
        epoch: EpochNumber,
        root_leaf: Leaf2<TYPES>,
    ) -> Result<DrbResult> {
        let cancel_token = {
            let mut computations = self.drb_computations.lock();
            if computations.contains_key(&epoch) {
                return Err(anytrace::debug!(
                    "DRB calculation for epoch {epoch} already in progress"
                ));
            }
            let token = CancellationToken::new();
            computations.insert(epoch, token.clone());
            token
        };
        // The single owner of the bookkeeping inserted above: cleared on drop.
        let _drb_state = DrbStateGuard {
            coordinator: self.clone(),
            epoch,
        };

        let Ok(drb_seed_input_vec) = bincode::serialize(&root_leaf.justify_qc().signatures) else {
            return Err(anytrace::error!(
                "Failed to serialize the QC signature for leaf {root_leaf:?}"
            ));
        };

        let Some(drb_difficulty_selector) = self.drb_difficulty_selector.read().clone() else {
            return Err(anytrace::error!(
                "The DRB difficulty selector is missing from the epoch membership coordinator. \
                 This node will not be able to spawn any DRB calculation tasks from catchup."
            ));
        };

        let drb_difficulty = drb_difficulty_selector(root_leaf.block_header().version()).await;

        let mut drb_seed_input = [0u8; 32];

        if root_leaf.block_header().version() >= DRB_FIX_VERSION {
            drb_seed_input = Sha256::digest(&drb_seed_input_vec).into();
        } else {
            let len = drb_seed_input_vec.len().min(32);
            drb_seed_input[..len].copy_from_slice(&drb_seed_input_vec[..len]);
        }

        let drb_input = DrbInput {
            epoch: *epoch,
            iteration: 0,
            value: drb_seed_input,
            difficulty_level: drb_difficulty,
        };

        let store_drb_progress_fn = self.store_drb_progress_fn.clone();
        let load_drb_progress_fn = self.load_drb_progress_fn.clone();

        let heartbeat = Heartbeat::default();
        let Some(computed) = heartbeat
            .while_alive(
                self.catchup_timeout,
                compute_drb_result(
                    drb_input,
                    store_drb_progress_fn,
                    load_drb_progress_fn,
                    DRB_PROGRESS_LOAD_TIMEOUT,
                    heartbeat.clone(),
                    cancel_token,
                ),
            )
            .await
        else {
            return Err(anytrace::error!(
                "the DRB computation for epoch {epoch} reported no progress for {:?}",
                self.catchup_timeout
            ));
        };
        let drb = match computed {
            Some(drb) => drb,
            None => {
                return self.get_epoch_drb(epoch).await.map_err(|e| {
                    anytrace::error!(
                        "DRB calculation for epoch {epoch} was cancelled but no externally \
                         supplied result is available: {e}"
                    )
                });
            },
        };

        // Publish the result in memory before persisting it, mirroring
        // `supply_drb`.
        self.membership.add_drb_result(epoch, drb);

        // The result is already published in memory, so the epoch resolves
        // whether or not the write gets through.
        info!(%epoch, ?drb, "writing the computed drb result to storage");
        match timeout(self.catchup_timeout, (self.store_drb_result_fn)(epoch, drb)).await {
            Ok(Err(err)) => warn!(%epoch, %err, "failed to store the drb result"),
            Ok(Ok(())) => {},
            Err(_) => warn!(
                %epoch,
                timeout = ?self.catchup_timeout,
                "storing the drb result timed out"
            ),
        }

        Ok(drb)
    }

    /// Supply a DRB result obtained from an external source (e.g. a decided
    /// leaf carrying `next_drb_result`). Adds the result to membership,
    /// persists it to storage, and cancels any in-flight local computation
    /// for `epoch`.
    ///
    /// If the stake table for `epoch` has not yet been loaded (e.g. the async
    /// catchup that registers it is still in flight), this logs an error and
    /// returns; the in-flight catchup will compute the DRB itself once it
    /// completes.
    pub fn supply_drb(&self, epoch: EpochNumber, drb: DrbResult) {
        if self.membership.snapshot(epoch).is_none() {
            error!(
                %epoch,
                "supply_drb called before the stake table is loaded; dropping the externally \
                 supplied drb and relying on the in-flight catchup"
            );
            return;
        }
        self.membership.add_drb_result(epoch, drb);
        // Left in place: the claim belongs to the computation, which clears it
        // on its way out.
        let maybe_token = self.drb_computations.lock().get(&epoch).cloned();
        if let Some(token) = maybe_token {
            token.cancel();
        }
        let store_drb_result_fn = self.store_drb_result_fn.clone();
        spawn(async move {
            info!(%epoch, ?drb, "writing the supplied drb result to storage");
            if let Err(e) = store_drb_result_fn(epoch, drb).await {
                warn!(%epoch, %e, "failed to store the supplied drb result");
            }
        });
    }

    /// The cancel token of the in-flight DRB computation for `epoch`, if one
    /// is running. Test observability: lets a test assert that dropping a
    /// computation's future fires its token.
    pub fn drb_cancel_token(&self, epoch: EpochNumber) -> Option<CancellationToken> {
        self.drb_computations.lock().get(&epoch).cloned()
    }

    /// Cancel all in-flight DRB calculations (e.g. on shutdown).
    pub fn cancel_all_drb(&self) {
        // Claims are left for the computations to release as they unwind, so
        // that none of them is replaced by a fresh one on the way out.
        let tokens: Vec<_> = self.drb_computations.lock().values().cloned().collect();
        for token in tokens {
            token.cancel();
        }
    }
}

/// Owns one computation's entry in `drb_computations`. Dropping it releases
/// the claim and fires the token it held.
struct DrbStateGuard<TYPES: NodeType> {
    coordinator: EpochMembershipCoordinator<TYPES>,
    epoch: EpochNumber,
}

impl<TYPES: NodeType> Drop for DrbStateGuard<TYPES> {
    fn drop(&mut self) {
        let token = self.coordinator.drb_computations.lock().remove(&self.epoch);
        if let Some(token) = token {
            token.cancel();
        }
    }
}

/// Default upper bound on a single catchup step.
const DEFAULT_CATCHUP_TIMEOUT: Duration = Duration::from_secs(300);

/// The in-flight attempts of one kind of work, keyed by the epoch they serve.
#[derive(Clone, Default)]
struct Attempts {
    map: Arc<Mutex<HashMap<EpochNumber, Attempt>>>,
}

impl Attempts {
    fn get_or_insert(
        &self,
        epoch: EpochNumber,
        action: impl FnOnce() -> BoxFuture<'static, Result<()>>,
    ) -> (Attempt, Option<AttemptEntry>) {
        let mut map = self.map.lock();
        if let Some(running) = map.get(&epoch) {
            return (running.clone(), None);
        }
        let attempt = action().shared();
        map.insert(epoch, attempt.clone());
        (
            attempt,
            Some(AttemptEntry {
                attempts: self.clone(),
                epoch,
            }),
        )
    }

    fn contains_attempt(&self, epoch: EpochNumber) -> bool {
        self.map.lock().contains_key(&epoch)
    }

    fn get_attempt(&self, epoch: EpochNumber) -> Option<Attempt> {
        self.map.lock().get(&epoch).cloned()
    }
}

/// Holds an epoch's place in an [`Attempts`] map for as long as its owner is driving it.
struct AttemptEntry {
    attempts: Attempts,
    epoch: EpochNumber,
}

impl Drop for AttemptEntry {
    fn drop(&mut self) {
        let removed = self.attempts.map.lock().remove(&self.epoch);
        drop(removed);
    }
}

/// Wrapper around a membership that holds a captured snapshot for a given
/// epoch (or the pre-epoch state). All accessors observe one consistent
/// view because the snapshot is held inline.
pub struct EpochMembership<TYPES: NodeType> {
    /// The captured snapshot, either per-epoch or pre-epoch.
    snapshot: EpochMembershipSnapshot<TYPES>,
    /// Underlying coordinator, retained so navigation methods like
    /// `next_epoch` can construct fresh snapshots.
    pub coordinator: EpochMembershipCoordinator<TYPES>,
}

enum EpochMembershipSnapshot<TYPES: NodeType> {
    Epoch {
        epoch: EpochNumber,
        snapshot: <TYPES::Membership as Membership<TYPES>>::Snapshot,
    },
    NonEpoch(<TYPES::Membership as Membership<TYPES>>::NonEpochSnapshot),
}

impl<TYPES: NodeType> Clone for EpochMembershipSnapshot<TYPES> {
    fn clone(&self) -> Self {
        match self {
            Self::Epoch { epoch, snapshot } => Self::Epoch {
                epoch: *epoch,
                snapshot: snapshot.clone(),
            },
            Self::NonEpoch(s) => Self::NonEpoch(s.clone()),
        }
    }
}

impl<TYPES: NodeType> Clone for EpochMembership<TYPES> {
    fn clone(&self) -> Self {
        Self {
            coordinator: self.coordinator.clone(),
            snapshot: self.snapshot.clone(),
        }
    }
}

impl<TYPES: NodeType> EpochMembership<TYPES> {
    pub fn epoch(&self) -> Option<EpochNumber> {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { epoch, .. } => Some(*epoch),
            EpochMembershipSnapshot::NonEpoch(_) => None,
        }
    }

    pub fn next_epoch(&self) -> Result<Self> {
        let epoch = self
            .epoch()
            .ok_or_else(|| anytrace::error!("No next epoch because epoch is None"))?;
        self.coordinator.membership_for_epoch(Some(epoch + 1))
    }

    pub fn next_epoch_stake_table(&self) -> Result<Self> {
        let epoch = self
            .epoch()
            .ok_or_else(|| anytrace::error!("No next epoch because epoch is None"))?;
        self.coordinator.stake_table_for_epoch(Some(epoch + 1))
    }

    pub fn get_new_epoch(&self, epoch: Option<EpochNumber>) -> Result<Self> {
        self.coordinator.membership_for_epoch(epoch)
    }

    async fn get_epoch_root(&self) -> anyhow::Result<Leaf2<TYPES>> {
        let Some(epoch) = self.epoch() else {
            anyhow::bail!("Cannot get root for None epoch");
        };
        let leaf = self.coordinator.get_epoch_root(epoch).await?;
        Ok(leaf)
    }

    pub async fn get_epoch_drb(&self) -> Result<DrbResult> {
        let Some(epoch) = self.epoch() else {
            return Err(anytrace::warn!("Cannot get drb for None epoch"));
        };
        self.coordinator.get_epoch_drb(epoch).await.wrap()
    }

    /// Borrow the per-epoch snapshot, or `None` for the pre-epoch case.
    pub fn snapshot(&self) -> Option<&<TYPES::Membership as Membership<TYPES>>::Snapshot> {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { snapshot, .. } => Some(snapshot),
            EpochMembershipSnapshot::NonEpoch(_) => None,
        }
    }

    /// Borrow the pre-epoch snapshot, or `None` if this is a per-epoch
    /// membership.
    pub fn non_epoch_snapshot(
        &self,
    ) -> Option<&<TYPES::Membership as Membership<TYPES>>::NonEpochSnapshot> {
        match &self.snapshot {
            EpochMembershipSnapshot::NonEpoch(s) => Some(s),
            EpochMembershipSnapshot::Epoch { .. } => None,
        }
    }

    /// Add the DRB result for this epoch to the membership.
    pub fn add_drb_result(&self, drb_result: DrbResult) {
        if let Some(epoch) = self.epoch() {
            self.coordinator
                .membership
                .add_drb_result(epoch, drb_result);
        }
    }

    // ---------------------------------------------------------------------
    // Single-call convenience accessors. Each delegates to whichever
    // snapshot was captured at construction time, so a single accessor
    // call observes one consistent view. For *sequences* of related reads
    // that must observe the same view, take a snapshot via
    // [`Self::snapshot`] / [`Self::non_epoch_snapshot`] and call methods
    // on it directly.
    // ---------------------------------------------------------------------

    pub fn stake_table(&self) -> impl ExactSizeIterator<Item = &PeerConfig<TYPES>> + Send {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { snapshot, .. } => Either::Left(snapshot.stake_table()),
            EpochMembershipSnapshot::NonEpoch(s) => Either::Right(s.stake_table()),
        }
    }

    pub fn da_stake_table(&self) -> impl ExactSizeIterator<Item = &PeerConfig<TYPES>> + Send {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { snapshot, .. } => {
                Either::Left(snapshot.da_stake_table())
            },
            EpochMembershipSnapshot::NonEpoch(s) => Either::Right(s.da_stake_table()),
        }
    }

    pub fn committee_members(
        &self,
        view: ViewNumber,
    ) -> impl ExactSizeIterator<Item = &TYPES::SignatureKey> + Send {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { snapshot, .. } => {
                Either::Left(snapshot.committee_members(view))
            },
            EpochMembershipSnapshot::NonEpoch(s) => Either::Right(s.committee_members(view)),
        }
    }

    pub fn da_committee_members(
        &self,
        view: ViewNumber,
    ) -> impl ExactSizeIterator<Item = &TYPES::SignatureKey> + Send {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { snapshot, .. } => {
                Either::Left(snapshot.da_committee_members(view))
            },
            EpochMembershipSnapshot::NonEpoch(s) => Either::Right(s.da_committee_members(view)),
        }
    }

    pub fn stake(&self, key: &TYPES::SignatureKey) -> Option<PeerConfig<TYPES>> {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { snapshot, .. } => snapshot.stake(key),
            EpochMembershipSnapshot::NonEpoch(s) => s.stake(key),
        }
    }

    pub fn da_stake(&self, key: &TYPES::SignatureKey) -> Option<PeerConfig<TYPES>> {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { snapshot, .. } => snapshot.da_stake(key),
            EpochMembershipSnapshot::NonEpoch(s) => s.da_stake(key),
        }
    }

    pub fn has_stake(&self, key: &TYPES::SignatureKey) -> bool {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { snapshot, .. } => snapshot.has_stake(key),
            EpochMembershipSnapshot::NonEpoch(s) => s.has_stake(key),
        }
    }

    pub fn has_da_stake(&self, key: &TYPES::SignatureKey) -> bool {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { snapshot, .. } => snapshot.has_da_stake(key),
            EpochMembershipSnapshot::NonEpoch(s) => s.has_da_stake(key),
        }
    }

    /// The leader for `view`, returning a HotShot-internal error type.
    ///
    /// # Errors
    ///
    /// Returns an error if the leader cannot be calculated.
    pub fn leader(&self, view: ViewNumber) -> Result<TYPES::SignatureKey> {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { snapshot, .. } => snapshot.leader(view),
            EpochMembershipSnapshot::NonEpoch(s) => s.leader(view),
        }
    }

    /// The leader for `view`, returning the membership-impl error type.
    ///
    /// # Errors
    ///
    /// Returns the membership-impl error if the leader cannot be calculated.
    pub fn lookup_leader(
        &self,
        view: ViewNumber,
    ) -> std::result::Result<
        TYPES::SignatureKey,
        <<TYPES as NodeType>::Membership as Membership<TYPES>>::Error,
    > {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { snapshot, .. } => snapshot.lookup_leader(view),
            EpochMembershipSnapshot::NonEpoch(s) => s.lookup_leader(view),
        }
    }

    pub fn total_nodes(&self) -> usize {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { snapshot, .. } => snapshot.total_nodes(),
            EpochMembershipSnapshot::NonEpoch(s) => s.total_nodes(),
        }
    }

    pub fn da_total_nodes(&self) -> usize {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { snapshot, .. } => snapshot.da_total_nodes(),
            EpochMembershipSnapshot::NonEpoch(s) => s.da_total_nodes(),
        }
    }

    pub fn success_threshold(&self) -> U256 {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { snapshot, .. } => snapshot.success_threshold(),
            EpochMembershipSnapshot::NonEpoch(s) => s.success_threshold(),
        }
    }

    pub fn da_success_threshold(&self) -> U256 {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { snapshot, .. } => snapshot.da_success_threshold(),
            EpochMembershipSnapshot::NonEpoch(s) => s.da_success_threshold(),
        }
    }

    pub fn failure_threshold(&self) -> U256 {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { snapshot, .. } => snapshot.failure_threshold(),
            EpochMembershipSnapshot::NonEpoch(s) => s.failure_threshold(),
        }
    }

    pub fn upgrade_threshold(&self) -> U256 {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { snapshot, .. } => snapshot.upgrade_threshold(),
            EpochMembershipSnapshot::NonEpoch(s) => s.upgrade_threshold(),
        }
    }

    pub fn stake_table_hash(&self) -> Option<Commitment<SnapshotStakeTableHash<TYPES>>> {
        match &self.snapshot {
            EpochMembershipSnapshot::Epoch { snapshot, .. } => snapshot.stake_table_hash(),
            EpochMembershipSnapshot::NonEpoch(_) => None,
        }
    }
}
