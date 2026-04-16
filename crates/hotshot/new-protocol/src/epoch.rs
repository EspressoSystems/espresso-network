use std::{
    collections::{BTreeMap, BTreeSet},
    mem::swap,
};

use hotshot_types::{
    data::{BlockNumber, EpochNumber, Leaf2},
    drb::DrbResult,
    epoch_membership::EpochMembershipCoordinator,
    traits::{block_contents::BlockHeader, election::Membership, node_implementation::NodeType},
    utils::{is_epoch_root, root_block_in_epoch, transition_block_for_epoch},
};
use hotshot_utils::anytrace;
use tokio::task::{AbortHandle, JoinSet};
use tracing::{debug, error};

use crate::leaf_store::EpochLeafStore;

pub enum EpochRootResult {
    DrbResult(EpochNumber, DrbResult),
    /// The epoch root leaf at the given height is needed but not available
    /// locally.  The coordinator should fetch it from peers and call
    /// [`EpochManager::on_leaf_stored`] when it arrives.
    NeedLeaf(EpochNumber, u64),
}

/// Manager for Epoch specific rules and actions
/// - Adding Epoch Root, DRB Result, and Light Client State Update Certificate
/// - Verififying Stake Table, DRB Result, and Light Client State Update Certificate
/// - Sending Epoch Termination Signals
pub struct EpochManager<T: NodeType> {
    epoch_height: BlockNumber,
    membership_coordinator: EpochMembershipCoordinator<T>,
    tasks: JoinSet<Result<EpochRootResult, EpochManagerError>>,
    handles: BTreeMap<EpochNumber, Vec<AbortHandle>>,
    leaf_store: EpochLeafStore<T>,
    /// Epochs for which a `request_drb_result` task is already in flight,
    /// used to avoid spawning duplicate fetch tasks.
    pending_drb_requests: BTreeSet<EpochNumber>,
    /// Epochs waiting on a leaf at a given height.  Populated when a task
    /// returns [`EpochRootResult::NeedLeaf`]; cleared when the leaf arrives
    /// via [`on_leaf_stored`].
    pending_leaves: BTreeMap<u64, BTreeSet<EpochNumber>>,
}

impl<T: NodeType> EpochManager<T> {
    pub fn new<B>(epoch_height: B, membership_coordinator: EpochMembershipCoordinator<T>) -> Self
    where
        B: Into<BlockNumber>,
    {
        Self {
            epoch_height: epoch_height.into(),
            membership_coordinator,
            tasks: JoinSet::new(),
            handles: BTreeMap::new(),
            leaf_store: EpochLeafStore::new(),
            pending_drb_requests: BTreeSet::new(),
            pending_leaves: BTreeMap::new(),
        }
    }

    pub async fn next(&mut self) -> Option<Result<EpochRootResult, EpochManagerError>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(result)) => {
                    // Clear the dedup guard so the epoch can be retried if
                    // needed (e.g. after a NeedLeaf is fulfilled).
                    if let Ok(ref r) = result {
                        match r {
                            EpochRootResult::DrbResult(epoch, _) => {
                                self.pending_drb_requests.remove(epoch);
                            },
                            EpochRootResult::NeedLeaf(epoch, height) => {
                                self.pending_drb_requests.remove(epoch);
                                self.pending_leaves
                                    .entry(*height)
                                    .or_default()
                                    .insert(*epoch);
                            },
                        }
                    }
                    return Some(result);
                },
                Some(Err(err)) => {
                    if !err.is_cancelled() {
                        error!(%err, "epoch manager task panic")
                    }
                },
                None => return None,
            }
        }
    }

    pub fn leaf_store(&self) -> &EpochLeafStore<T> {
        &self.leaf_store
    }

    /// Called when a leaf becomes available in the store (e.g. from a peer
    /// response).  Re-triggers [`request_drb_result`] for any epochs that
    /// were waiting on this height.
    pub fn on_leaf_stored(&mut self, height: u64) {
        if let Some(epochs) = self.pending_leaves.remove(&height) {
            for epoch in epochs {
                self.request_drb_result(epoch);
            }
        }
    }

    pub fn handle_leaf_decided(&mut self, leaf: Leaf2<T>) {
        let block_number = leaf.block_header().block_number();

        // Trigger epoch root + DRB computation for epoch root blocks.
        if is_epoch_root(block_number, *self.epoch_height) {
            let Some(epoch) = leaf.epoch(*self.epoch_height) else {
                error!("Leaf has no epoch");
                return;
            };

            // Root and DRB apply in 2 epochs,
            let target_epoch = epoch + 2;

            let membership_coordinator = self.membership_coordinator.clone();
            let handles = self.handles.entry(target_epoch).or_default();
            let header = leaf.block_header().clone();
            let membership = membership_coordinator.membership().clone();

            // add_epoch_root must complete before compute_drb_result because
            // add_drb_result asserts that the stake table for the epoch
            // already exists.
            handles.push(self.tasks.spawn(async move {
                T::Membership::add_epoch_root(membership, header.clone())
                    .await
                    .map_err(EpochManagerError::EpochRoot)?;
                membership_coordinator
                    .compute_drb_result(target_epoch, leaf)
                    .await
                    .map_err(EpochManagerError::DrbCompute)
                    .map(|drb| EpochRootResult::DrbResult(target_epoch, drb))
            }));
        }
    }

    pub fn gc(&mut self, epoch: EpochNumber) {
        let mut tmp = self.handles.split_off(&epoch);
        swap(&mut tmp, &mut self.handles);
        for handle in tmp.into_values().flatten() {
            handle.abort();
        }
    }

    pub fn request_drb_result(&mut self, epoch: EpochNumber) {
        // Avoid spawning duplicate fetch tasks for the same epoch.
        if self.pending_drb_requests.contains(&epoch) {
            return;
        }
        self.pending_drb_requests.insert(epoch);
        let membership_coordinator = self.membership_coordinator.clone();
        let leaf_store = self.leaf_store.clone();
        let epoch_height = self.epoch_height;
        let handles = self.handles.entry(epoch).or_default();

        handles.push(self.tasks.spawn(async move {
            // Fast path: DRB already available — check without triggering
            // the membership catchup (which would race with the slow path
            // below).
            {
                let membership = membership_coordinator.membership().read().await;
                if let Ok(true) = membership.has_randomized_stake_table(epoch) {
                    drop(membership);
                    if let Ok(stake_table) = membership_coordinator
                        .membership_for_epoch(Some(epoch))
                        .await
                    {
                        if let Ok(drb) = stake_table.get_epoch_drb().await {
                            return Ok(EpochRootResult::DrbResult(epoch, drb));
                        }
                    }
                }
            }

            // We need the epoch root leaf. The root for epoch E is at
            // root_block_in_epoch(E-2, epoch_height).
            let root_epoch = epoch.saturating_sub(2);
            let root_height = root_block_in_epoch(root_epoch, *epoch_height);

            let root_entry = match leaf_store.get(root_height) {
                Some(entry) => entry,
                None => {
                    // Not in local store — tell the coordinator to fetch it.
                    debug!(%epoch, %root_height, "epoch root leaf not available, requesting from peers");
                    return Ok(EpochRootResult::NeedLeaf(epoch, root_height));
                },
            };

            // Register the epoch root with the membership.
            let membership = membership_coordinator.membership().clone();
            T::Membership::add_epoch_root(membership, root_entry.leaf.block_header().clone())
                .await
                .map_err(EpochManagerError::EpochRoot)?;

            // Try to get the DRB from the transition leaf in the local
            // store (which carries next_drb_result).
            let drb_epoch = epoch.saturating_sub(1);
            let transition_height = transition_block_for_epoch(drb_epoch, *epoch_height);

            if let Some(transition_entry) = leaf_store.get(transition_height) {
                if let Some(drb) = transition_entry.leaf.next_drb_result {
                    membership_coordinator
                        .membership()
                        .write()
                        .await
                        .add_drb_result(epoch, drb);
                    return Ok(EpochRootResult::DrbResult(epoch, drb));
                }
            }

            // Compute the DRB locally from the epoch root leaf.
            membership_coordinator
                .compute_drb_result(epoch, root_entry.leaf)
                .await
                .map_err(EpochManagerError::DrbCompute)
                .map(|drb| EpochRootResult::DrbResult(epoch, drb))
        }));
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EpochManagerError {
    #[error("failed to add epoch root: {0}")]
    EpochRoot(#[source] anyhow::Error),

    #[error("failed to compute drb: {0}")]
    DrbCompute(#[source] anytrace::Error),

    #[error("failed to get drb: {0}")]
    DrbLookup(#[source] anytrace::Error),
}
