use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use alloy::primitives::U256;
use async_broadcast::{broadcast, InactiveReceiver};
use async_lock::{Mutex, RwLock};
use hotshot_utils::{
    anytrace::{self, Error, Level, Result, Wrap, DEFAULT_LOG_LEVEL},
    ensure, line_info, log, warn,
};

use crate::{
    data::Leaf2,
    drb::{compute_drb_result, DrbResult},
    traits::{
        election::Membership,
        node_implementation::{ConsensusTime, NodeType},
        storage::StorageAddDrbResultFn,
    },
    utils::{root_block_in_epoch, transition_block_for_epoch},
    PeerConfig,
};

type EpochMap<TYPES> =
    HashMap<<TYPES as NodeType>::Epoch, InactiveReceiver<Result<EpochMembership<TYPES>>>>;

/// Struct to Coordinate membership catchup
pub struct EpochMembershipCoordinator<TYPES: NodeType> {
    /// The underlying membhersip
    membership: Arc<RwLock<TYPES::Membership>>,

    /// Any in progress attempts at catching up are stored in this map
    /// Any new callers wantin an `EpochMembership` will await on the signal
    /// alerting them the membership is ready.  The first caller for an epoch will
    /// wait for the actual catchup and allert future callers when it's done
    catchup_map: Arc<Mutex<EpochMap<TYPES>>>,

    /// Callback function to store a drb result when one is calculated during catchup
    storage_add_drb_result_fn: Option<StorageAddDrbResultFn<TYPES>>,

    /// Number of blocks in an epoch
    pub epoch_height: u64,
}

impl<TYPES: NodeType> Clone for EpochMembershipCoordinator<TYPES> {
    fn clone(&self) -> Self {
        Self {
            membership: Arc::clone(&self.membership),
            catchup_map: Arc::clone(&self.catchup_map),
            storage_add_drb_result_fn: self.storage_add_drb_result_fn.clone(),
            epoch_height: self.epoch_height,
        }
    }
}
// async fn catchup_membership(coordinator: EpochMembershipCoordinator<TYPES>) {

// }

impl<TYPES: NodeType> EpochMembershipCoordinator<TYPES>
where
    Self: Send,
{
    /// Create an EpochMembershipCoordinator
    pub fn new(
        membership: Arc<RwLock<TYPES::Membership>>,
        storage_add_drb_result_fn: Option<StorageAddDrbResultFn<TYPES>>,
        epoch_height: u64,
    ) -> Self {
        Self {
            membership,
            catchup_map: Arc::default(),
            storage_add_drb_result_fn,
            epoch_height,
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
        spawn_catchup(coordinator, epoch);

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
        spawn_catchup(coordinator, epoch);

        Err(warn!(
            "Stake table for Epoch {:?} Unavailable. Starting catchup",
            epoch
        ))
    }

    /// Catches the membership up to the epoch passed as an argument.  
    /// To do this try to get the stake table for the epoch containing this epoch's root
    /// if the root does not exist recursively catchup until you've found it
    ///
    /// If there is another catchup in progress this will not duplicate efforts
    /// e.g. if we start with only epoch 0 stake table and call catchup for epoch 10, then call catchup for epoch 20
    /// the first caller will actually do the work for to catchup to epoch 10 then the second caller will continue
    /// catching up to epoch 20
    async fn catchup(self, epoch: TYPES::Epoch) -> Result<EpochMembership<TYPES>> {
        // recursively catchup until we have a stake table for the epoch containing our root
        ensure!(
            *epoch != 0 && *epoch != 1,
            "We are trying to catchup to epoch 0! This means the initial stake table is missing!"
        );
        let root_epoch = TYPES::Epoch::new(*epoch - 2);

        let root_membership = if self.membership.read().await.has_stake_table(root_epoch) {
            EpochMembership {
                epoch: Some(root_epoch),
                coordinator: self.clone(),
            }
        } else {
            Box::pin(self.wait_for_catchup(root_epoch)).await?
        };

        // Get the epoch root headers and update our membership with them, finally sync them
        // Verification of the root is handled in get_epoch_root_and_drb
        let Ok(root_leaf) = root_membership
            .get_epoch_root(root_block_in_epoch(*root_epoch, self.epoch_height))
            .await
        else {
            anytrace::bail!("get epoch root failed for epoch {:?}", root_epoch);
        };

        let updater = self
            .membership
            .read()
            .await
            .add_epoch_root(epoch, root_leaf.block_header().clone())
            .await
            .ok_or(anytrace::warn!("add epoch root failed"))?;
        updater(&mut *(self.membership.write().await));

        let drb_membership = match root_membership.next_epoch_stake_table().await {
            Ok(drb_membership) => drb_membership,
            Err(_) => Box::pin(self.wait_for_catchup(root_epoch + 1)).await?,
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
                return Err(anytrace::error!("Failed to serialize the QC signature."));
            };

            let mut drb_seed_input = [0u8; 32];
            let len = drb_seed_input_vec.len().min(32);
            drb_seed_input[..len].copy_from_slice(&drb_seed_input_vec[..len]);

            tokio::task::spawn_blocking(move || compute_drb_result::<TYPES>(drb_seed_input))
                .await
                .unwrap()
        };

        if let Some(cb) = &self.storage_add_drb_result_fn {
            tracing::info!("Writing drb result from catchup to storage for epoch {epoch}");
            if let Err(e) = cb(epoch, drb).await {
                tracing::warn!("Failed to add drb result to storage: {e}");
            }
        }

        self.membership.write().await.add_drb_result(epoch, drb);
        Ok(EpochMembership {
            epoch: Some(epoch),
            coordinator: self.clone(),
        })
    }

    pub async fn wait_for_catchup(&self, epoch: TYPES::Epoch) -> Result<EpochMembership<TYPES>> {
        let Some(mut rx) = self
            .catchup_map
            .lock()
            .await
            .get(&epoch)
            .map(InactiveReceiver::activate_cloned)
        else {
            return self.clone().catchup(epoch).await;
        };
        let Ok(Ok(mem)) = rx.recv_direct().await else {
            return self.clone().catchup(epoch).await;
        };
        Ok(mem)
    }
}

fn spawn_catchup<T: NodeType>(coordinator: EpochMembershipCoordinator<T>, epoch: T::Epoch) {
    tokio::spawn(async move {
        let tx = {
            let mut map = coordinator.catchup_map.lock().await;
            if map.contains_key(&epoch) {
                return;
            }
            let (tx, rx) = broadcast(1);
            map.insert(epoch, rx.deactivate());
            tx
        };
        // do catchup

        let result = coordinator.clone().catchup(epoch).await;
        let _ = tx.broadcast_direct(result.clone()).await;

        if let Err(err) = result {
            tracing::warn!("failed to catchup for epoch={epoch:?}. err={err:#}");
            coordinator.catchup_map.lock().await.remove(&epoch);
        }
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
    pub async fn stake_table(&self) -> Vec<PeerConfig<TYPES>> {
        self.coordinator
            .membership
            .read()
            .await
            .stake_table(self.epoch)
    }

    /// Get all participants in the committee (including their stake) for a specific epoch
    pub async fn da_stake_table(&self) -> Vec<PeerConfig<TYPES>> {
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
