use std::{
    collections::{HashMap, HashSet, hash_map::Entry},
    sync::Arc,
};

use alloy_primitives::U256;
use async_broadcast::{InactiveReceiver, Sender, broadcast};
use committable::Commitment;
use either::Either;
use hotshot_utils::anytrace::{self, Wrap};
use parking_lot::{Mutex, RwLock};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};
use versions::DRB_FIX_VERSION;

use crate::{
    PeerConfig, PeerConnectInfo,
    data::{BlockNumber, EpochNumber, Leaf2, ViewNumber},
    drb::{DrbDifficultySelectorFn, DrbInput, DrbResult, compute_drb_result},
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

type EpochMap<TYPES> = HashMap<EpochNumber, InactiveReceiver<Result<EpochMembership<TYPES>>>>;

type DrbMap = HashSet<EpochNumber>;

/// Cancellation tokens for in-flight DRB computations. When an
/// external source supplies the DRB result for `epoch` (e.g. a decided leaf
/// carrying `next_drb_result`), `supply_drb` fires the token so the local
/// computation can stop early instead of grinding to completion.
type DrbCancelMap = HashMap<EpochNumber, CancellationToken>;

type EpochSender<TYPES> = (EpochNumber, Sender<Result<EpochMembership<TYPES>>>);

/// The per-epoch snapshot type associated with `T::Membership`.
type Snapshot<T> = <<T as NodeType>::Membership as Membership<T>>::Snapshot;

/// The stake-table hash type associated with `T::Membership`'s per-epoch
/// snapshot.
type SnapshotStakeTableHash<T> = <Snapshot<T> as MembershipSnapshot<T>>::StakeTableHash;

/// Struct to Coordinate membership catchup
pub struct EpochMembershipCoordinator<TYPES: NodeType> {
    membership: Arc<TYPES::Membership>,
    catchup_map: Arc<Mutex<EpochMap<TYPES>>>,
    drb_calculation_map: Arc<Mutex<DrbMap>>,
    drb_cancel_map: Arc<Mutex<DrbCancelMap>>,
    epoch_height: BlockNumber,
    store_drb_progress_fn: StoreDrbProgressFn,
    load_drb_progress_fn: LoadDrbProgressFn,
    store_drb_result_fn: StoreDrbResultFn,
    drb_difficulty_selector: Arc<RwLock<Option<DrbDifficultySelectorFn>>>,
}

impl<TYPES: NodeType> Clone for EpochMembershipCoordinator<TYPES> {
    fn clone(&self) -> Self {
        Self {
            membership: Arc::clone(&self.membership),
            catchup_map: Arc::clone(&self.catchup_map),
            drb_calculation_map: Arc::clone(&self.drb_calculation_map),
            drb_cancel_map: Arc::clone(&self.drb_cancel_map),
            epoch_height: self.epoch_height,
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
            catchup_map: Arc::default(),
            drb_calculation_map: Arc::default(),
            drb_cancel_map: Arc::default(),
            epoch_height: epoch_height.into(),
            store_drb_progress_fn: store_drb_progress_fn(storage.clone()),
            load_drb_progress_fn: load_drb_progress_fn(storage.clone()),
            store_drb_result_fn: store_drb_result_fn(storage.clone()),
            drb_difficulty_selector: Arc::new(RwLock::new(None)),
        }
    }

    pub fn epoch_height(&self) -> BlockNumber {
        self.epoch_height
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
        let mut catchup_map = self.catchup_map.lock();
        match catchup_map.entry(epoch) {
            Entry::Occupied(_) => Err(anytrace::warn!(
                "Randomized stake table for epoch {epoch:?} unavailable. Catchup already in \
                 progress"
            )),
            Entry::Vacant(e) => {
                let coordinator = self.clone();
                let (tx, rx) = broadcast(1);
                e.insert(rx.deactivate());
                drop(catchup_map);
                spawn_catchup(coordinator, epoch, tx);
                Err(anytrace::warn!(
                    "Randomized stake table for epoch {epoch:?} unavailable. Starting catchup"
                ))
            },
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
        if let Some(mem) = self.epoch_membership(epoch) {
            return Ok(mem);
        }
        let mut catchup_map = self.catchup_map.lock();
        match catchup_map.entry(epoch) {
            Entry::Occupied(_) => Err(anytrace::warn!(
                "Stake table for epoch {epoch:?} unavailable. Catchup already in progress"
            )),
            Entry::Vacant(e) => {
                let coordinator = self.clone();
                let (tx, rx) = broadcast(1);
                e.insert(rx.deactivate());
                drop(catchup_map);
                spawn_catchup(coordinator, epoch, tx);

                Err(anytrace::warn!(
                    "Stake table for epoch {epoch:?} unavailable. Starting catchup"
                ))
            },
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

    /// Catches the membership up to the epoch passed as an argument.
    ///
    /// Starts at the latest epoch whose stake table we already have and
    /// proceeds one epoch at a time toward `epoch`, which it fetches last,
    /// together with its DRB result.
    ///
    /// `epoch` is a stopping condition, not a starting point.
    ///
    /// Concurrent catchups do not duplicate efforts: they walk the same
    /// epochs and claim them in `catchup_map`, so whichever reaches an epoch
    /// first fetches it while the others wait for its result.
    //
    // Clippy claims "this `MutexGuard` is held across an await point", however
    // the guard is explicitly dropped before. See also:
    // https://github.com/rust-lang/rust-clippy/issues/6446
    //
    // Even more annoying is that the warning can only be disabled on function
    // level, instead of putting this attribute on the expression, see
    // https://github.com/rust-lang/rust-clippy/issues/9047.
    #[allow(clippy::await_holding_lock)]
    async fn catchup(self, epoch: EpochNumber, epoch_tx: Sender<Result<EpochMembership<TYPES>>>) {
        let Some(first_epoch) = self.membership.first_epoch() else {
            let err = anytrace::error!(
                "We got a catchup request for epoch {epoch:?} but the first epoch is not set"
            );
            self.catchup_cleanup(epoch, epoch_tx.clone(), None, err);
            return;
        };

        if epoch <= first_epoch + 1 {
            let err = anytrace::error!(
                "We got a catchup request for epoch {epoch} but the first two epochs are seeded \
                 by `set_first_epoch`, not derived from an epoch root"
            );
            self.catchup_cleanup(epoch, epoch_tx.clone(), None, err);
            return;
        }

        let Some(mut try_epoch) = self.find_walk_start(epoch, first_epoch).await else {
            let err = anytrace::error!(
                "We are trying to catchup to epoch {epoch} but we hold no stake table it can be \
                 derived from! This means the initial stake table is missing!"
            );
            self.catchup_cleanup(epoch, epoch_tx.clone(), None, err);
            return;
        };

        warn!(%epoch, %try_epoch, "catching up to epoch");

        while try_epoch < epoch {
            if self.ensure_stake_table(try_epoch).await {
                try_epoch += 1;
                continue;
            }
            // Lock the catchup map
            let mut map_lock = self.catchup_map.lock();
            match map_lock
                .get(&try_epoch)
                .map(InactiveReceiver::activate_cloned)
            {
                Some(mut rx) => {
                    // Somebody else is already fetching this epoch, drop
                    // the lock and wait for them to finish
                    drop(map_lock);
                    match rx.recv_direct().await {
                        Ok(Ok(_)) => try_epoch += 1,
                        Ok(Err(err)) => {
                            if !self.ensure_stake_table(try_epoch).await {
                                self.catchup_cleanup(epoch, epoch_tx, None, err);
                                return;
                            }
                            try_epoch += 1;
                        },
                        Err(_) => {
                            let mut map_lock = self.catchup_map.lock();
                            if map_lock
                                .get(&try_epoch)
                                .is_some_and(|rx| rx.sender_count() == 0)
                            {
                                map_lock.remove(&try_epoch);
                            }
                        },
                    }
                },
                _ => {
                    if self.membership.snapshot(try_epoch).is_some() {
                        drop(map_lock);
                        try_epoch += 1;
                        continue;
                    }
                    // Nobody else is fetching this epoch. Claim it, fetch it,
                    // and move on to the next one.
                    let (mut tx, rx) = broadcast(1);
                    tx.set_overflow(true);
                    map_lock.insert(try_epoch, rx.deactivate());
                    drop(map_lock);

                    if let Err(err) = self.fetch_and_publish(try_epoch, &tx).await {
                        self.catchup_cleanup(epoch, epoch_tx, Some((try_epoch, tx)), err);
                        return;
                    }
                    // Remove the epoch from the catchup map to indicate that the catchup is complete
                    self.catchup_map.lock().remove(&try_epoch);
                    try_epoch += 1;
                },
            }
        }

        let mut root_leaf = None;
        if !self.ensure_stake_table(epoch).await {
            let Some(leaf) = self.fetch_epoch_root(epoch, &epoch_tx).await else {
                return;
            };
            root_leaf = Some(leaf);
        }

        match self.get_epoch_drb(epoch).await {
            Ok(drb_result) => {
                warn!(
                    ?drb_result,
                    %epoch,
                    "DRB result retrieved from peers. Updating membership."
                );
                self.membership.add_drb_result(epoch, drb_result);
            },
            Err(err) => {
                warn!(
                    %epoch,
                    %err,
                    "Recalculating missing DRB result. Catchup failed with error.",
                );

                let root_leaf = match root_leaf {
                    Some(leaf) => leaf,
                    None => {
                        let Some(leaf) = self.fetch_epoch_root(epoch, &epoch_tx).await else {
                            return;
                        };
                        leaf
                    },
                };

                let result = self.compute_drb_result(epoch, root_leaf).await;

                hotshot_utils::log!(result);

                if let Err(err) = result {
                    self.catchup_cleanup(epoch, epoch_tx.clone(), None, err);
                    return;
                }
            },
        };

        if let Err(err) = self.publish(epoch, &epoch_tx) {
            self.catchup_cleanup(epoch, epoch_tx.clone(), None, err);
            return;
        }

        // Remove the epoch from the catchup map to indicate that the catchup is complete
        self.catchup_map.lock().remove(&epoch);
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
        let maybe_receiver = self
            .catchup_map
            .lock()
            .get(&epoch)
            .map(InactiveReceiver::activate_cloned);
        let Some(mut rx) = maybe_receiver else {
            // There is no catchup in progress, maybe the epoch is already finalized
            if let Some(mem) = self.epoch_membership(epoch) {
                return Ok(mem);
            }
            return Err(anytrace::error!(
                "No catchup in progress for epoch {epoch} and we don't have a stake table for it"
            ));
        };
        if let Ok(result) = rx.recv_direct().await {
            result
        } else {
            self.epoch_membership(epoch)
                .ok_or_else(|| anytrace::error!("Catchup for epoch {epoch} failed"))
        }
    }

    /// Clean up after a failed catchup attempt.
    ///
    /// This method is called when a catchup attempt fails. It cleans up the state of the
    /// `EpochMembershipCoordinator` by removing the failed epochs from the
    /// `catchup_map` and broadcasting the error to any tasks that are waiting for the
    /// catchup to complete.
    fn catchup_cleanup(
        &self,
        req_epoch: EpochNumber,
        epoch_tx: Sender<Result<EpochMembership<TYPES>>>,
        in_flight: Option<EpochSender<TYPES>>,
        err: anytrace::Error,
    ) {
        let cancel_epochs = if let Some(in_flight) = in_flight {
            vec![(req_epoch, epoch_tx), in_flight]
        } else {
            vec![(req_epoch, epoch_tx)]
        };

        error!(
            epoch = %req_epoch,
            %err,
            "catchup failed. canceling catchup for epochs: {:?}",
            cancel_epochs.iter().map(|(e, _)| e).collect::<Vec<_>>()
        );

        {
            let mut map_lock = self.catchup_map.lock();
            for (epoch, _) in cancel_epochs.iter() {
                // Remove the failed epochs from the catchup map
                map_lock.remove(epoch);
            }
        }

        for (cancel_epoch, tx) in cancel_epochs {
            // Signal the other tasks about the failures
            if let Ok(Some(res)) = tx.try_broadcast(Err(err.clone())) {
                warn!(
                    epoch = %cancel_epoch,
                    "The catchup channel overflowed during cleanup, dropped message {:?}",
                    res.map(|em| em.epoch())
                );
            }
        }
    }

    /// The epoch a catchup walk toward `target` starts at, or `None` if we
    /// hold no stake table it can be derived from.
    ///
    /// The walk starts at the epoch after the latest one we have, and never
    /// later than `target`. Deriving a stake table needs the epoch root of
    /// `epoch - 2`, which `fetch_stake_table` resolves in memory, and the
    /// next epoch needs `epoch - 1`, so the start moves to earlier epochs
    /// until both are available, loading them from local storage on the way.
    /// What we hold is normally contiguous, so the first pair wins; a gap
    /// costs one probe per epoch it spans, and the search stops at the epochs
    /// `set_first_epoch` seeds.
    ///
    /// A membership that tracks no latest epoch reports `None` from
    /// `highest_known_epoch`. The walk then starts at the seeded epochs and
    /// skips what we have one probe at a time, fetching the same set.
    ///
    /// `target` can only move the start earlier, so a request for an
    /// unreachable epoch cannot lengthen any of this.
    async fn find_walk_start(
        &self,
        target: EpochNumber,
        first_epoch: EpochNumber,
    ) -> Option<EpochNumber> {
        // The seeded epochs are always available, so they bound a membership
        // that reports no latest epoch and one that reports an earlier epoch.
        let known = self
            .membership
            .highest_known_epoch()
            .filter(|known| *known > first_epoch + 1)
            .unwrap_or(first_epoch + 1);
        let last = EpochNumber::new(target.saturating_sub(1));
        let mut epoch = known.min(last) + 1;
        // Each step needs `epoch - 1` and `epoch - 2`, and one step's
        // `epoch - 2` is the next step's `epoch - 1`, so carry it along
        // instead of probing local storage for it twice.
        let mut later = None;
        while epoch > first_epoch + 1 {
            let has_later = match later {
                Some(known) => known,
                None => self.ensure_stake_table(epoch - 1).await,
            };
            let has_earlier = self.ensure_stake_table(epoch - 2).await;
            if has_later && has_earlier {
                return Some(epoch);
            }
            later = Some(has_earlier);
            epoch = epoch - 1;
        }
        None
    }

    /// Make the stake table for `epoch` available in memory, loading it from
    /// local storage if it is not there already. `false` if it is available
    /// from neither.
    async fn ensure_stake_table(&self, epoch: EpochNumber) -> bool {
        self.membership.snapshot(epoch).is_some() || self.membership.load_stake_table(epoch).await
    }

    /// Fetch the stake table for `epoch` and hand the resulting membership to
    /// the tasks waiting on `tx`.
    async fn fetch_and_publish(
        &self,
        epoch: EpochNumber,
        tx: &Sender<Result<EpochMembership<TYPES>>>,
    ) -> Result<()> {
        self.fetch_stake_table(epoch).await?;
        self.publish(epoch, tx)
    }

    /// Fetch `epoch`'s stake table and return the epoch root it was derived
    /// from. On failure the waiters are notified and the claim released, so
    /// `None` means the caller must stop.
    async fn fetch_epoch_root(
        &self,
        epoch: EpochNumber,
        epoch_tx: &Sender<Result<EpochMembership<TYPES>>>,
    ) -> Option<Leaf2<TYPES>> {
        match self.fetch_stake_table(epoch).await {
            Ok(root_leaf) => Some(root_leaf),
            Err(err) => {
                error!(%epoch, %err, "failed to fetch stake table");
                self.catchup_cleanup(epoch, epoch_tx.clone(), None, err);
                None
            },
        }
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

    /// Hand the membership for `epoch` to the tasks waiting on `tx`.
    ///
    /// The caller has just made the stake table available, so a missing
    /// snapshot is a catchup failure rather than a state to wait out.
    fn publish(
        &self,
        epoch: EpochNumber,
        tx: &Sender<Result<EpochMembership<TYPES>>>,
    ) -> Result<()> {
        let Some(mem) = self.epoch_membership(epoch) else {
            return Err(anytrace::error!(
                "snapshot for epoch {epoch} unavailable after fetching its stake table"
            ));
        };
        if let Ok(Some(res)) = tx.try_broadcast(Ok(mem)) {
            warn!(
                %epoch,
                "The catchup channel overflowed, dropped message {:?}",
                res.map(|em| em.epoch())
            );
        }
        Ok(())
    }

    /// A helper method to the `catchup` method.
    ///
    /// It tries to fetch the requested stake table from the root epoch,
    /// and updates the membership accordingly.
    ///
    /// # Arguments
    ///
    /// * `epoch` - The epoch for which to fetch the stake table.
    ///
    /// # Returns
    ///
    /// * `Ok(Leaf2<TYPES>)` containing the epoch root leaf if successful.
    /// * `Err(Error)` if the root membership or root leaf cannot be found, or if
    ///   updating the membership fails.
    async fn fetch_stake_table(&self, epoch: EpochNumber) -> Result<Leaf2<TYPES>> {
        let root_epoch = EpochNumber::new(epoch.saturating_sub(2));
        // Snapshot eviction can drop the root epoch from memory while a walk
        // is in progress, so load it back from local storage. Deliberately not
        // `stake_table_for_epoch`, which on a miss claims the root epoch and
        // spawns a nested catchup for it -- that is how one walk step turns
        // into a cascade.
        self.ensure_stake_table(root_epoch).await;
        let Some(root_membership) = self.epoch_membership(root_epoch) else {
            return Err(anytrace::error!(
                "We tried to fetch stake table for epoch {epoch:?} but its root epoch \
                 {root_epoch:?} is in neither memory nor local storage"
            ));
        };

        // Get the epoch root headers and update our membership with them, finally sync them
        // Verification of the root is handled in get_epoch_root_and_drb
        let Ok(root_leaf) = root_membership.get_epoch_root().await else {
            return Err(anytrace::error!(
                "get epoch root leaf failed for epoch {root_epoch:?}"
            ));
        };

        self.add_epoch_root(root_leaf.block_header().clone())
            .await
            .map_err(|e| {
                anytrace::error!("Failed to add epoch root for epoch {epoch:?} to membership: {e}")
            })?;

        Ok(root_leaf)
    }

    pub async fn compute_drb_result(
        &self,
        epoch: EpochNumber,
        root_leaf: Leaf2<TYPES>,
    ) -> Result<DrbResult> {
        let cancel_token = {
            let mut drb_calculation_map_lock = self.drb_calculation_map.lock();

            if drb_calculation_map_lock.contains(&epoch) {
                return Err(anytrace::debug!(
                    "DRB calculation for epoch {} already in progress",
                    epoch
                ));
            }
            drb_calculation_map_lock.insert(epoch);

            let token = CancellationToken::new();
            self.drb_cancel_map.lock().insert(epoch, token.clone());
            token
        };

        let Ok(drb_seed_input_vec) = bincode::serialize(&root_leaf.justify_qc().signatures) else {
            self.clear_drb_state(epoch);
            return Err(anytrace::error!(
                "Failed to serialize the QC signature for leaf {root_leaf:?}"
            ));
        };

        let Some(drb_difficulty_selector) = self.drb_difficulty_selector.read().clone() else {
            self.clear_drb_state(epoch);
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

        let drb = match compute_drb_result(
            drb_input,
            store_drb_progress_fn,
            load_drb_progress_fn,
            cancel_token,
        )
        .await
        {
            Some(drb) => drb,
            None => {
                self.clear_drb_state(epoch);
                return self.get_epoch_drb(epoch).await.map_err(|e| {
                    anytrace::error!(
                        "DRB calculation for epoch {epoch} was cancelled but no externally \
                         supplied result is available: {e}"
                    )
                });
            },
        };

        self.clear_drb_state(epoch);

        tracing::info!("Writing drb result from catchup to storage for epoch {epoch}: {drb:?}");
        if let Err(e) = (self.store_drb_result_fn)(epoch, drb).await {
            tracing::warn!("Failed to add drb result to storage: {e}");
        }
        self.membership.add_drb_result(epoch, drb);

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
            tracing::error!(
                "supply_drb called for epoch {epoch} but stake table not yet loaded; dropping \
                 externally-supplied DRB and relying on in-flight catchup"
            );
            return;
        }
        self.membership.add_drb_result(epoch, drb);
        let maybe_token = self.drb_cancel_map.lock().remove(&epoch);
        if let Some(token) = maybe_token {
            token.cancel();
        }
        let store_drb_result_fn = self.store_drb_result_fn.clone();
        tokio::spawn(async move {
            tracing::info!(
                "Writing externally supplied drb result to storage for epoch {epoch}: {drb:?}"
            );
            if let Err(e) = store_drb_result_fn(epoch, drb).await {
                tracing::warn!("Failed to add externally supplied drb result to storage: {e}");
            }
        });
    }

    /// Remove per-epoch DRB bookkeeping after a computation finishes or is
    /// cancelled. Safe to call multiple times.
    fn clear_drb_state(&self, epoch: EpochNumber) {
        self.drb_calculation_map.lock().remove(&epoch);
        self.drb_cancel_map.lock().remove(&epoch);
    }

    /// Cancel all in-flight DRB calculations (e.g. on shutdown).
    pub fn cancel_all_drb(&self) {
        let tokens: Vec<_> = self.drb_cancel_map.lock().drain().map(|(_, t)| t).collect();
        for token in tokens {
            token.cancel();
        }
    }
}

fn spawn_catchup<T: NodeType>(
    coordinator: EpochMembershipCoordinator<T>,
    epoch: EpochNumber,
    epoch_tx: Sender<Result<EpochMembership<T>>>,
) {
    tokio::spawn(async move {
        coordinator.clone().catchup(epoch, epoch_tx).await;
    });
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
