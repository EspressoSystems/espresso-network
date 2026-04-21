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
use tracing::error;

use crate::leaf_store::EpochLeafStore;

pub enum EpochRootResult {
    DrbResult(EpochNumber, DrbResult),
    /// The epoch root leaf at the given height is needed but not available
    /// locally.  The coordinator should fetch it from peers and call
    /// [`EpochManager::on_leaf_stored`] when it arrives.
    NeedLeaf(EpochNumber, u64),
}

/// Output of a spawned DRB task.  The leading `EpochNumber` tags the task
/// with the epoch it was working on so [`EpochManager::next`] can update
/// dedup state correctly even on the Err path.
type TaskOutput = (EpochNumber, Result<EpochRootResult, EpochManagerError>);

/// Manager for Epoch specific rules and actions
/// - Adding Epoch Root, DRB Result, and Light Client State Update Certificate
/// - Verififying Stake Table, DRB Result, and Light Client State Update Certificate
/// - Sending Epoch Termination Signals
pub struct EpochManager<T: NodeType> {
    epoch_height: BlockNumber,
    membership_coordinator: EpochMembershipCoordinator<T>,
    tasks: JoinSet<TaskOutput>,
    handles: BTreeMap<EpochNumber, Vec<AbortHandle>>,
    leaf_store: EpochLeafStore<T>,
    /// Epochs for which a `request_drb_result` task is currently in flight.
    /// Prevents duplicate fetch/compute tasks while the first is running.
    pending_drb_requests: BTreeSet<EpochNumber>,
    /// Epochs whose DRB has already been computed and added to membership.
    /// Subsequent `request_drb_result` calls for these epochs are no-ops.
    completed_drb_requests: BTreeSet<EpochNumber>,
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
            completed_drb_requests: BTreeSet::new(),
            pending_leaves: BTreeMap::new(),
        }
    }

    pub async fn next(&mut self) -> Option<Result<EpochRootResult, EpochManagerError>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok((epoch, result))) => {
                    match &result {
                        Ok(EpochRootResult::DrbResult(..)) => {
                            self.pending_drb_requests.remove(&epoch);
                            self.completed_drb_requests.insert(epoch);
                        },
                        Ok(EpochRootResult::NeedLeaf(_, height)) => {
                            // Keep the dedup guard set: we're now waiting on the
                            // leaf to arrive, and don't want each incoming cert to
                            // respawn a redundant task (which would trigger yet
                            // another LeafRequest broadcast).  The guard is cleared
                            // by `on_leaf_stored` when the leaf finally arrives.
                            self.pending_leaves
                                .entry(*height)
                                .or_default()
                                .insert(epoch);
                        },
                        Err(_) => {
                            // Clear the guard so a subsequent call can retry.
                            self.pending_drb_requests.remove(&epoch);
                        },
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
                // Clear the dedup guard (set on NeedLeaf) so the follow-up
                // task that will actually compute the DRB can be spawned.
                self.pending_drb_requests.remove(&epoch);
                self.request_drb_result(epoch);
            }
        }
    }

    pub fn handle_leaf_decided(&mut self, leaf: Leaf2<T>) {
        let block_number = leaf.block_header().block_number();

        // Trigger epoch root + DRB computation for epoch root blocks.  The
        // leaf has already been inserted into `leaf_store` by the coordinator,
        // so `request_drb_result` can pick it up through the same path a
        // late-joining node would.
        if is_epoch_root(block_number, *self.epoch_height) {
            let Some(epoch) = leaf.epoch(*self.epoch_height) else {
                error!("Leaf has no epoch");
                return;
            };
            self.request_drb_result(epoch + 2);
        }
    }

    pub fn gc(&mut self, epoch: EpochNumber) {
        let mut tmp = self.handles.split_off(&epoch);
        swap(&mut tmp, &mut self.handles);
        for handle in tmp.into_values().flatten() {
            handle.abort();
        }
        // Drop tracking entries for epochs we no longer care about.  Keeps
        // `completed_drb_requests` bounded while the protocol runs.
        self.pending_drb_requests = self.pending_drb_requests.split_off(&epoch);
        self.completed_drb_requests = self.completed_drb_requests.split_off(&epoch);
    }

    pub fn request_drb_result(&mut self, epoch: EpochNumber) {
        // Already computed — caller can read the DRB from membership.
        if self.completed_drb_requests.contains(&epoch) {
            return;
        }
        // In-flight task will deliver the result; avoid spawning a duplicate.
        if self.pending_drb_requests.contains(&epoch) {
            return;
        }
        self.pending_drb_requests.insert(epoch);
        let membership_coordinator = self.membership_coordinator.clone();
        let leaf_store = self.leaf_store.clone();
        let epoch_height = self.epoch_height;
        let handles = self.handles.entry(epoch).or_default();

        handles.push(self.tasks.spawn(async move {
            let result = async {
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
                            && let Ok(drb) = stake_table.get_epoch_drb().await
                        {
                            return Ok(EpochRootResult::DrbResult(epoch, drb));
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

                if let Some(transition_entry) = leaf_store.get(transition_height)
                    && let Some(drb) = transition_entry.leaf.next_drb_result
                {
                    membership_coordinator
                        .membership()
                        .write()
                        .await
                        .add_drb_result(epoch, drb);
                    return Ok(EpochRootResult::DrbResult(epoch, drb));
                }

                // Compute the DRB locally from the epoch root leaf.
                membership_coordinator
                    .compute_drb_result(epoch, root_entry.leaf)
                    .await
                    .map_err(EpochManagerError::DrbCompute)
                    .map(|drb| EpochRootResult::DrbResult(epoch, drb))
            }
            .await;
            (epoch, result)
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
