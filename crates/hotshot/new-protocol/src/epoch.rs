use std::{
    collections::{BTreeMap, BTreeSet},
    mem::swap,
};

use hotshot_types::{
    data::{BlockNumber, EpochNumber, Leaf2},
    drb::DrbResult,
    epoch_membership::EpochMembershipCoordinator,
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    utils::is_epoch_root,
};
use hotshot_utils::anytrace;
use tokio::task::{AbortHandle, JoinSet};
use tracing::error;

pub enum EpochRootResult {
    DrbResult(EpochNumber, DrbResult),
}

/// Epoch + error for the Err path so retries can re-kick the task.
pub struct EpochFailure {
    pub epoch: EpochNumber,
    pub error: EpochManagerError,
}

/// Output of a spawned DRB task.  The leading `EpochNumber` tags the task
/// with the epoch it was working on so [`EpochManager::next`] can update
/// dedup state correctly even on the Err path.
type TaskOutput = (EpochNumber, Result<EpochRootResult, EpochManagerError>);

/// Manager for Epoch specific rules and actions.
///
/// Delegates all catchup (stake-table walk-back, epoch root fetch, DRB
/// compute) to [`EpochMembershipCoordinator::membership_for_epoch`].  This
/// manager exists only to issue the request and dedup concurrent callers.
pub struct EpochManager<T: NodeType> {
    epoch_height: BlockNumber,
    membership_coordinator: EpochMembershipCoordinator<T>,
    tasks: JoinSet<TaskOutput>,
    handles: BTreeMap<EpochNumber, Vec<AbortHandle>>,
    /// Epochs for which a `request_drb_result` task is currently in flight.
    /// Prevents duplicate fetch/compute tasks while the first is running.
    pending_drb_requests: BTreeSet<EpochNumber>,
    /// Epochs whose DRB has already been computed and added to membership.
    /// Subsequent `request_drb_result` calls for these epochs are no-ops.
    completed_drb_requests: BTreeSet<EpochNumber>,
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
            pending_drb_requests: BTreeSet::new(),
            completed_drb_requests: BTreeSet::new(),
        }
    }

    pub async fn next(&mut self) -> Option<Result<EpochRootResult, EpochFailure>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok((epoch, result))) => {
                    match result {
                        Ok(root @ EpochRootResult::DrbResult(..)) => {
                            self.pending_drb_requests.remove(&epoch);
                            self.completed_drb_requests.insert(epoch);
                            return Some(Ok(root));
                        },
                        Err(error) => {
                            // Clear the guard so a subsequent call can retry.
                            self.pending_drb_requests.remove(&epoch);
                            return Some(Err(EpochFailure { epoch, error }));
                        },
                    }
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

    pub fn handle_leaf_decided(&mut self, leaf: Leaf2<T>) {
        let block_number = leaf.block_header().block_number();

        // At every epoch root, trigger DRB computation for the epoch that
        // will use this root (epoch + 2).
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
        let handles = self.handles.entry(epoch).or_default();

        handles.push(self.tasks.spawn(async move {
            let result = async {
                // Kick the membership coordinator.  If the stake table is
                // already ready, this returns it immediately; otherwise it
                // spawns a catchup task and returns a "catchup in progress"
                // error.  Either way, `wait_for_catchup` resolves once the
                // stake table + DRB are both in place.
                let membership = match membership_coordinator
                    .membership_for_epoch(Some(epoch))
                    .await
                {
                    Ok(m) => m,
                    Err(_) => membership_coordinator
                        .wait_for_catchup(epoch)
                        .await
                        .map_err(EpochManagerError::DrbLookup)?,
                };
                let drb = membership
                    .get_epoch_drb()
                    .await
                    .map_err(EpochManagerError::DrbLookup)?;
                Ok(EpochRootResult::DrbResult(epoch, drb))
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
