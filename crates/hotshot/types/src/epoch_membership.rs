use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use alloy::primitives::U256;
use async_broadcast::{broadcast, InactiveReceiver, Sender};
use async_lock::{Mutex, RwLock};
use hotshot_utils::{
    anytrace::{self, Error, Level, Result, Wrap, DEFAULT_LOG_LEVEL},
    ensure, line_info, log, warn,
};

use crate::{
    data::Leaf2,
    drb::{compute_drb_result, DrbInput, DrbResult},
    stake_table::HSStakeTable,
    traits::{
        election::Membership,
        node_implementation::{ConsensusTime, NodeType},
        storage::{
            load_drb_progress_fn, store_drb_progress_fn, store_drb_result_fn, LoadDrbProgressFn,
            Storage, StoreDrbProgressFn, StoreDrbResultFn,
        },
    },
    utils::{root_block_in_epoch, transition_block_for_epoch},
    PeerConfig,
};

type EpochMap<TYPES> =
    HashMap<<TYPES as NodeType>::Epoch, InactiveReceiver<Result<EpochMembership<TYPES>>>>;

type EpochSender<TYPES> = (
    <TYPES as NodeType>::Epoch,
    Sender<Result<EpochMembership<TYPES>>>,
);

/// Struct to Coordinate membership catchup
pub struct EpochMembershipCoordinator<TYPES: NodeType> {
    /// The underlying membhersip
    membership: Arc<RwLock<TYPES::Membership>>,

    /// Any in progress attempts at catching up are stored in this map
    /// Any new callers wantin an `EpochMembership` will await on the signal
    /// alerting them the membership is ready.  The first caller for an epoch will
    /// wait for the actual catchup and allert future callers when it's done
    catchup_map: Arc<Mutex<EpochMap<TYPES>>>,

    /// Number of blocks in an epoch
    pub epoch_height: u64,

    store_drb_progress_fn: StoreDrbProgressFn,

    load_drb_progress_fn: LoadDrbProgressFn,

    /// Callback function to store a drb result in storage when one is calculated during catchup
    store_drb_result_fn: StoreDrbResultFn<TYPES>,
}

impl<TYPES: NodeType> Clone for EpochMembershipCoordinator<TYPES> {
    fn clone(&self) -> Self {
        Self {
            membership: Arc::clone(&self.membership),
            catchup_map: Arc::clone(&self.catchup_map),
            epoch_height: self.epoch_height,
            store_drb_progress_fn: Arc::clone(&self.store_drb_progress_fn),
            load_drb_progress_fn: Arc::clone(&self.load_drb_progress_fn),
            store_drb_result_fn: self.store_drb_result_fn.clone(),
        }
    }
}

impl<TYPES: NodeType> EpochMembershipCoordinator<TYPES>
where
    Self: Send,
{
    /// Create an EpochMembershipCoordinator
    pub fn new<S: Storage<TYPES>>(
        membership: Arc<RwLock<TYPES::Membership>>,
        epoch_height: u64,
        storage: &S,
    ) -> Self {
        Self {
            membership,
            catchup_map: Arc::default(),
            epoch_height,
            store_drb_progress_fn: store_drb_progress_fn(storage.clone()),
            load_drb_progress_fn: load_drb_progress_fn(storage.clone()),
            store_drb_result_fn: store_drb_result_fn(storage.clone()),
        }
    }

    /// Get a reference to the membership
    #[must_use]
    pub fn membership(&self) -> &Arc<RwLock<TYPES::Membership>> {
        &self.membership
    }

    /// Get a Membership for a given Epoch, which is guaranteed to have a randomized stake
    /// table for the given Epoch
    pub async fn membership_for_epoch(
        &self,
        maybe_epoch: Option<TYPES::Epoch>,
    ) -> Result<EpochMembership<TYPES>> {
        let ret_val = EpochMembership {
            epoch: maybe_epoch,
            coordinator: self.clone(),
        };
        let Some(epoch) = maybe_epoch else {
            return Ok(ret_val);
        };
        if self
            .membership
            .read()
            .await
            .has_randomized_stake_table(epoch)
        {
            return Ok(ret_val);
        }
        if self.catchup_map.lock().await.contains_key(&epoch) {
            return Err(warn!(
                "Randomized stake table for epoch {:?} unavailable. Catchup already in progress",
                epoch
            ));
        }
        let coordinator = self.clone();
        let (tx, rx) = broadcast(1);
        self.catchup_map.lock().await.insert(epoch, rx.deactivate());
        spawn_catchup(coordinator, epoch, tx);

        Err(warn!(
            "Randomized stake table for epoch {:?} unavailable. Starting catchup",
            epoch
        ))
    }

    /// Get a Membership for a given Epoch, which is guaranteed to have a stake
    /// table for the given Epoch
    pub async fn stake_table_for_epoch(
        &self,
        maybe_epoch: Option<TYPES::Epoch>,
    ) -> Result<EpochMembership<TYPES>> {
        let ret_val = EpochMembership {
            epoch: maybe_epoch,
            coordinator: self.clone(),
        };
        let Some(epoch) = maybe_epoch else {
            return Ok(ret_val);
        };
        if self.membership.read().await.has_stake_table(epoch) {
            return Ok(ret_val);
        }
        if self.catchup_map.lock().await.contains_key(&epoch) {
            return Err(warn!(
                "Stake table for Epoch {:?} Unavailable. Catch up already in Progress",
                epoch
            ));
        }
        let coordinator = self.clone();
        let (tx, rx) = broadcast(1);
        self.catchup_map.lock().await.insert(epoch, rx.deactivate());
        spawn_catchup(coordinator, epoch, tx);

        Err(warn!(
            "Stake table for Epoch {:?} Unavailable. Starting catchup",
            epoch
        ))
    }

    /// Catches the membership up to the epoch passed as an argument.  
    /// To do this, try to get the stake table for the epoch containing this epoch's root and
    /// the stake table for the epoch containing this epoch's drb result.
    /// If they do not exist, then go one by one back until we find a stake table.
    ///
    /// If there is another catchup in progress this will not duplicate efforts
    /// e.g. if we start with only the first epoch stake table and call catchup for epoch 10, then call catchup for epoch 20
    /// the first caller will actually do the work for to catchup to epoch 10 then the second caller will continue
    /// catching up to epoch 20
    async fn catchup(
        mut self,
        epoch: TYPES::Epoch,
        epoch_tx: Sender<Result<EpochMembership<TYPES>>>,
    ) {
        // We need to fetch the requested epoch, that's for sure
        let mut fetch_epochs = vec![];
        fetch_epochs.push((epoch, epoch_tx));

        let mut try_epoch = TYPES::Epoch::new(epoch.saturating_sub(1));
        let maybe_first_epoch = self.membership.read().await.first_epoch();
        let Some(first_epoch) = maybe_first_epoch else {
            let err = anytrace::error!(
                "We got a catchup request for epoch {:?} but the first epoch is not set",
                epoch
            );
            self.catchup_cleanup(epoch, fetch_epochs, err).await;
            return;
        };

        // First figure out which epochs we need to fetch
        loop {
            let has_stake_table = self.membership.read().await.has_stake_table(try_epoch);
            if has_stake_table {
                // We have this stake table but we need to make sure we have the epoch root of the requested epoch
                if try_epoch <= TYPES::Epoch::new(epoch.saturating_sub(2)) {
                    break;
                }
                try_epoch = TYPES::Epoch::new(try_epoch.saturating_sub(1));
            } else {
                if try_epoch <= first_epoch + 1 {
                    let err = anytrace::error!(
                        "We are trying to catchup to an epoch lower than the second epoch! \
                        This means the initial stake table is missing!"
                    );
                    self.catchup_cleanup(epoch, fetch_epochs, err).await;
                    return;
                }
                // Lock the catchup map
                let mut map_lock = self.catchup_map.lock().await;
                if let Some(mut rx) = map_lock
                    .get(&try_epoch)
                    .map(InactiveReceiver::activate_cloned)
                {
                    // Somebody else is already fetching this epoch, drop the lock and wait for them to finish
                    drop(map_lock);
                    if let Ok(Ok(_)) = rx.recv_direct().await {
                        break;
                    };
                    // If we didn't receive the epoch then we need to try again
                } else {
                    // Nobody else is fetching this epoch. We need to do it. Put it in the map and move on to the next epoch
                    let (mut tx, rx) = broadcast(1);
                    tx.set_overflow(true);
                    map_lock.insert(try_epoch, rx.deactivate());
                    drop(map_lock);
                    fetch_epochs.push((try_epoch, tx));
                    try_epoch = TYPES::Epoch::new(try_epoch.saturating_sub(1));
                }
            };
        }

        // Iterate through the epochs we need to fetch in reverse, i.e. from the oldest to the newest
        while let Some((current_fetch_epoch, tx)) = fetch_epochs.pop() {
            let root_leaf = match self.fetch_stake_table(current_fetch_epoch).await {
                Ok(roof_leaf) => roof_leaf,
                Err(err) => {
                    fetch_epochs.push((current_fetch_epoch, tx));
                    self.catchup_cleanup(epoch, fetch_epochs, err).await;
                    return;
                },
            };

            if let Err(err) = self
                .fetch_or_calc_drb_results(current_fetch_epoch, root_leaf)
                .await
            {
                fetch_epochs.push((current_fetch_epoch, tx));
                self.catchup_cleanup(epoch, fetch_epochs, err).await;
                return;
            }

            // Signal the other tasks about the success
            if let Ok(Some(res)) = tx.try_broadcast(Ok(EpochMembership {
                epoch: Some(current_fetch_epoch),
                coordinator: self.clone(),
            })) {
                tracing::warn!(
                    "The catchup channel for epoch {} was overflown, dropped message {:?}",
                    current_fetch_epoch,
                    res.map(|em| em.epoch)
                );
            }

            // Remove the epoch from the catchup map to indicate that the catchup is complete
            self.catchup_map.lock().await.remove(&current_fetch_epoch);
        }
    }

    /// Call this method if you think catchup is in progress for a given epoch
    /// and you want to wait for it to finish and get the stake table.
    /// If it's not, it will try to return the stake table if already available.
    /// Returns an error if the catchup failed or the catchup is not in progress
    /// and the stake table is not available.
    pub async fn wait_for_catchup(&self, epoch: TYPES::Epoch) -> Result<EpochMembership<TYPES>> {
        let maybe_receiver = self
            .catchup_map
            .lock()
            .await
            .get(&epoch)
            .map(InactiveReceiver::activate_cloned);
        let Some(mut rx) = maybe_receiver else {
            // There is no catchup in progress, maybe the epoch is already finalized
            if self.membership.read().await.has_stake_table(epoch) {
                return Ok(EpochMembership {
                    epoch: Some(epoch),
                    coordinator: self.clone(),
                });
            }
            return Err(anytrace::error!(
                "No catchup in progress for epoch {epoch} and we don't have a stake table for it"
            ));
        };
        let Ok(Ok(mem)) = rx.recv_direct().await else {
            return Err(anytrace::error!("Catchup for epoch {epoch} failed"));
        };
        Ok(mem)
    }

    /// Clean up after a failed catchup attempt.
    ///
    /// This method is called when a catchup attempt fails. It cleans up the state of the
    /// `EpochMembershipCoordinator` by removing the failed epochs from the
    /// `catchup_map` and broadcasting the error to any tasks that are waiting for the
    /// catchup to complete.
    async fn catchup_cleanup(
        &mut self,
        req_epoch: TYPES::Epoch,
        cancel_epochs: Vec<EpochSender<TYPES>>,
        err: Error,
    ) {
        // Cleanup in case of error
        let mut map_lock = self.catchup_map.lock().await;
        for (epoch, _) in cancel_epochs.iter() {
            // Remove the failed epochs from the catchup map
            map_lock.remove(epoch);
        }
        drop(map_lock);
        for (cancel_epoch, tx) in cancel_epochs {
            // Signal the other tasks about the failures
            if let Ok(Some(res)) = tx.try_broadcast(Err(err.clone())) {
                tracing::warn!(
                    "The catchup channel for epoch {} was overflown during cleanup, dropped message {:?}",
                    cancel_epoch,
                    res.map(|em| em.epoch)
                );
            }
        }
        tracing::error!("catchup for epoch {:?} failed: {:?}", req_epoch, err);
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
    /// * `Err(Error)` if the root membership or root leaf cannot be found, or if updating the membership fails.
    async fn fetch_stake_table(&self, epoch: TYPES::Epoch) -> Result<Leaf2<TYPES>> {
        let root_epoch = TYPES::Epoch::new(epoch.saturating_sub(2));
        let Ok(root_membership) = self.stake_table_for_epoch(Some(root_epoch)).await else {
            return Err(anytrace::error!(
                "We tried to fetch stake table for epoch {:?} \
                but we don't have its root epoch {:?}. This should not happen",
                epoch,
                root_epoch
            ));
        };

        // Get the epoch root headers and update our membership with them, finally sync them
        // Verification of the root is handled in get_epoch_root_and_drb
        let Ok(root_leaf) = root_membership
            .get_epoch_root(root_block_in_epoch(*root_epoch, self.epoch_height))
            .await
        else {
            return Err(anytrace::error!(
                "get epoch root leaf failed for epoch {:?}",
                root_epoch
            ));
        };

        let add_epoch_root_updater = {
            let membership_read = self.membership.read().await;
            membership_read
                .add_epoch_root(epoch, root_leaf.block_header().clone())
                .await
        };

        if let Some(updater) = add_epoch_root_updater {
            let mut membership_write = self.membership.write().await;
            updater(&mut *(membership_write));
        };

        Ok(root_leaf)
    }

    /// A helper method to the `catchup` method.
    ///
    /// Fetch or compute the DRB (Distributed Random Beacon) result for a given epoch.
    ///
    /// This method attempts to retrieve the DRB result for the specified epoch. If the DRB
    /// result is not available, it computes the DRB using the provided root leaf and stores
    /// the result in the membership state.
    ///
    /// # Arguments
    ///
    /// * `epoch` - The epoch for which to fetch or compute the DRB result.
    /// * `root_leaf` - The epoch root leaf used for DRB computation if the result is not available.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the DRB result was successfully fetched or computed and stored.
    /// * `Err(Error)` if the DRB result could not be fetched, computed, or stored.
    async fn fetch_or_calc_drb_results(
        &self,
        epoch: TYPES::Epoch,
        root_leaf: Leaf2<TYPES>,
    ) -> Result<()> {
        let root_epoch = TYPES::Epoch::new(epoch.saturating_sub(2));
        let Ok(root_membership) = self.stake_table_for_epoch(Some(root_epoch)).await else {
            return Err(anytrace::error!("We tried to fetch drb result for epoch {:?} but we don't have its root epoch {:?}. This should not happen", epoch, root_epoch));
        };

        let Ok(drb_membership) = root_membership.next_epoch_stake_table().await else {
            return Err(anytrace::error!(
                "get drb stake table failed for epoch {:?}",
                root_epoch
            ));
        };

        // get the DRB from the last block of the epoch right before the one we're catching up to
        // or compute it if it's not available
        let drb = if let Ok(drb) = drb_membership
            .get_epoch_drb(transition_block_for_epoch(
                *(root_epoch + 1),
                self.epoch_height,
            ))
            .await
        {
            drb
        } else {
            let Ok(drb_seed_input_vec) = bincode::serialize(&root_leaf.justify_qc().signatures)
            else {
                return Err(anytrace::error!(
                    "Failed to serialize the QC signature for leaf {:?}",
                    root_leaf
                ));
            };

            let mut drb_seed_input = [0u8; 32];
            let len = drb_seed_input_vec.len().min(32);
            drb_seed_input[..len].copy_from_slice(&drb_seed_input_vec[..len]);
            let drb_input = DrbInput {
                epoch: *epoch,
                iteration: 0,
                value: drb_seed_input,
            };

            let store_drb_progress_fn = self.store_drb_progress_fn.clone();
            let load_drb_progress_fn = self.load_drb_progress_fn.clone();

            compute_drb_result(drb_input, store_drb_progress_fn, load_drb_progress_fn).await
        };

        tracing::info!("Writing drb result from catchup to storage for epoch {epoch}");
        if let Err(e) = (self.store_drb_result_fn)(epoch, drb).await {
            tracing::warn!("Failed to add drb result to storage: {e}");
        }
        self.membership.write().await.add_drb_result(epoch, drb);

        Ok(())
    }
}

fn spawn_catchup<T: NodeType>(
    coordinator: EpochMembershipCoordinator<T>,
    epoch: T::Epoch,
    epoch_tx: Sender<Result<EpochMembership<T>>>,
) {
    tokio::spawn(async move {
        coordinator.clone().catchup(epoch, epoch_tx).await;
    });
}
/// Wrapper around a membership that guarantees that the epoch
/// has a stake table
pub struct EpochMembership<TYPES: NodeType> {
    /// Epoch the `membership` is guaranteed to have a stake table for
    pub epoch: Option<TYPES::Epoch>,
    /// Underlying membership
    pub coordinator: EpochMembershipCoordinator<TYPES>,
}

impl<TYPES: NodeType> Clone for EpochMembership<TYPES> {
    fn clone(&self) -> Self {
        Self {
            coordinator: self.coordinator.clone(),
            epoch: self.epoch,
        }
    }
}

impl<TYPES: NodeType> EpochMembership<TYPES> {
    /// Get the epoch this membership is good for
    pub fn epoch(&self) -> Option<TYPES::Epoch> {
        self.epoch
    }

    /// Get a membership for the next epoch
    pub async fn next_epoch(&self) -> Result<Self> {
        ensure!(
            self.epoch().is_some(),
            "No next epoch because epoch is None"
        );
        self.coordinator
            .membership_for_epoch(self.epoch.map(|e| e + 1))
            .await
    }
    /// Get a membership for the next epoch
    pub async fn next_epoch_stake_table(&self) -> Result<Self> {
        ensure!(
            self.epoch().is_some(),
            "No next epoch because epoch is None"
        );
        self.coordinator
            .stake_table_for_epoch(self.epoch.map(|e| e + 1))
            .await
    }
    pub async fn get_new_epoch(&self, epoch: Option<TYPES::Epoch>) -> Result<Self> {
        self.coordinator.membership_for_epoch(epoch).await
    }

    /// Wraps the same named Membership trait fn
    async fn get_epoch_root(&self, block_height: u64) -> anyhow::Result<Leaf2<TYPES>> {
        let Some(epoch) = self.epoch else {
            anyhow::bail!("Cannot get root for None epoch");
        };
        <TYPES::Membership as Membership<TYPES>>::get_epoch_root(
            self.coordinator.membership.clone(),
            block_height,
            epoch,
        )
        .await
    }

    /// Wraps the same named Membership trait fn
    async fn get_epoch_drb(&self, block_height: u64) -> Result<DrbResult> {
        let Some(epoch) = self.epoch else {
            return Err(anytrace::warn!("Cannot get drb for None epoch"));
        };
        <TYPES::Membership as Membership<TYPES>>::get_epoch_drb(
            self.coordinator.membership.clone(),
            block_height,
            epoch,
        )
        .await
        .wrap()
    }

    /// Get all participants in the committee (including their stake) for a specific epoch
    pub async fn stake_table(&self) -> HSStakeTable<TYPES> {
        self.coordinator
            .membership
            .read()
            .await
            .stake_table(self.epoch)
    }

    /// Get all participants in the committee (including their stake) for a specific epoch
    pub async fn da_stake_table(&self) -> HSStakeTable<TYPES> {
        self.coordinator
            .membership
            .read()
            .await
            .da_stake_table(self.epoch)
    }

    /// Get all participants in the committee for a specific view for a specific epoch
    pub async fn committee_members(
        &self,
        view_number: TYPES::View,
    ) -> BTreeSet<TYPES::SignatureKey> {
        self.coordinator
            .membership
            .read()
            .await
            .committee_members(view_number, self.epoch)
    }

    /// Get all participants in the committee for a specific view for a specific epoch
    pub async fn da_committee_members(
        &self,
        view_number: TYPES::View,
    ) -> BTreeSet<TYPES::SignatureKey> {
        self.coordinator
            .membership
            .read()
            .await
            .da_committee_members(view_number, self.epoch)
    }

    /// Get the stake table entry for a public key, returns `None` if the
    /// key is not in the table for a specific epoch
    pub async fn stake(&self, pub_key: &TYPES::SignatureKey) -> Option<PeerConfig<TYPES>> {
        self.coordinator
            .membership
            .read()
            .await
            .stake(pub_key, self.epoch)
    }

    /// Get the DA stake table entry for a public key, returns `None` if the
    /// key is not in the table for a specific epoch
    pub async fn da_stake(&self, pub_key: &TYPES::SignatureKey) -> Option<PeerConfig<TYPES>> {
        self.coordinator
            .membership
            .read()
            .await
            .da_stake(pub_key, self.epoch)
    }

    /// See if a node has stake in the committee in a specific epoch
    pub async fn has_stake(&self, pub_key: &TYPES::SignatureKey) -> bool {
        self.coordinator
            .membership
            .read()
            .await
            .has_stake(pub_key, self.epoch)
    }

    /// See if a node has stake in the committee in a specific epoch
    pub async fn has_da_stake(&self, pub_key: &TYPES::SignatureKey) -> bool {
        self.coordinator
            .membership
            .read()
            .await
            .has_da_stake(pub_key, self.epoch)
    }

    /// The leader of the committee for view `view_number` in `epoch`.
    ///
    /// Note: this function uses a HotShot-internal error type.
    /// You should implement `lookup_leader`, rather than implementing this function directly.
    ///
    /// # Errors
    /// Returns an error if the leader cannot be calculated.
    pub async fn leader(&self, view: TYPES::View) -> Result<TYPES::SignatureKey> {
        self.coordinator
            .membership
            .read()
            .await
            .leader(view, self.epoch)
    }

    /// The leader of the committee for view `view_number` in `epoch`.
    ///
    /// Note: There is no such thing as a DA leader, so any consumer
    /// requiring a leader should call this.
    ///
    /// # Errors
    /// Returns an error if the leader cannot be calculated
    pub async fn lookup_leader(
        &self,
        view: TYPES::View,
    ) -> std::result::Result<
        TYPES::SignatureKey,
        <<TYPES as NodeType>::Membership as Membership<TYPES>>::Error,
    > {
        self.coordinator
            .membership
            .read()
            .await
            .lookup_leader(view, self.epoch)
    }

    /// Returns the number of total nodes in the committee in an epoch `epoch`
    pub async fn total_nodes(&self) -> usize {
        self.coordinator
            .membership
            .read()
            .await
            .total_nodes(self.epoch)
    }

    /// Returns the number of total DA nodes in the committee in an epoch `epoch`
    pub async fn da_total_nodes(&self) -> usize {
        self.coordinator
            .membership
            .read()
            .await
            .da_total_nodes(self.epoch)
    }

    /// Returns the threshold for a specific `Membership` implementation
    pub async fn success_threshold(&self) -> U256 {
        self.coordinator
            .membership
            .read()
            .await
            .success_threshold(self.epoch)
    }

    /// Returns the DA threshold for a specific `Membership` implementation
    pub async fn da_success_threshold(&self) -> U256 {
        self.coordinator
            .membership
            .read()
            .await
            .da_success_threshold(self.epoch)
    }

    /// Returns the threshold for a specific `Membership` implementation
    pub async fn failure_threshold(&self) -> U256 {
        self.coordinator
            .membership
            .read()
            .await
            .failure_threshold(self.epoch)
    }

    /// Returns the threshold required to upgrade the network protocol
    pub async fn upgrade_threshold(&self) -> U256 {
        self.coordinator
            .membership
            .read()
            .await
            .upgrade_threshold(self.epoch)
    }

    /// Add the epoch result to the membership
    pub async fn add_drb_result(&self, drb_result: DrbResult) {
        if let Some(epoch) = self.epoch() {
            self.coordinator
                .membership
                .write()
                .await
                .add_drb_result(epoch, drb_result);
        }
    }
}
