use std::{collections::BTreeMap, mem::swap};

use hotshot_types::{
    data::{BlockNumber, EpochNumber, Leaf2},
    drb::DrbResult,
    epoch_membership::EpochMembershipCoordinator,
    traits::{block_contents::BlockHeader, election::Membership, node_implementation::NodeType},
    utils::is_epoch_root,
};
use hotshot_utils::anytrace;
use tokio::task::{AbortHandle, JoinSet};
use tracing::{error, info};

pub enum EpochRootResult {
    DrbResult(EpochNumber, DrbResult),
    RootAdded(EpochNumber),
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
        }
    }

    pub async fn next(&mut self) -> Option<Result<EpochRootResult, EpochManagerError>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(result)) => return Some(result),
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
        if !is_epoch_root(block_number, *self.epoch_height) {
            return;
        }

        let Some(epoch) = leaf.epoch(*self.epoch_height) else {
            error!("Leaf has no epoch");
            return;
        };

        // Root and DRB apply in 2 epochs.
        let target_epoch = epoch + 2;

        tracing::debug!(
            block_number,
            ?epoch,
            ?target_epoch,
            "spawning add_epoch_root and compute_drb_result"
        );

        let membership_coordinator = self.membership_coordinator.clone();
        let handles = self.handles.entry(target_epoch).or_default();
        let header = leaf.block_header().clone();
        let membership = membership_coordinator.membership().clone();

        handles.push(self.tasks.spawn(async move {
            let result = T::Membership::add_epoch_root(membership, header)
                .await
                .map_err(EpochManagerError::EpochRoot)
                .map(|_| EpochRootResult::RootAdded(target_epoch));
            info!(
                ?target_epoch,
                is_ok = result.is_ok(),
                "add_epoch_root completed"
            );
            result
        }));

        handles.push(self.tasks.spawn(async move {
            let result = membership_coordinator
                .compute_drb_result(target_epoch, leaf)
                .await
                .map_err(EpochManagerError::DrbCompute)
                .map(|drb| EpochRootResult::DrbResult(target_epoch, drb));
            info!(
                ?target_epoch,
                is_ok = result.is_ok(),
                "compute_drb_result completed"
            );
            result
        }));
    }

    pub fn gc(&mut self, epoch: EpochNumber) {
        let mut tmp = self.handles.split_off(&epoch);
        swap(&mut tmp, &mut self.handles);
        for handle in tmp.into_values().flatten() {
            handle.abort();
        }
    }
    pub fn request_drb_result(&mut self, epoch: EpochNumber) {
        let membership_coordinator = self.membership_coordinator.clone();
        let handles = self.handles.entry(epoch).or_default();
        handles.push(self.tasks.spawn(async move {
            // Trigger catchup for the epoch or get the full stake table
            match membership_coordinator
                .stake_table_for_epoch(Some(epoch))
                .await
            {
                Ok(stake_table) => stake_table
                    .get_epoch_drb()
                    .await
                    .map_err(EpochManagerError::DrbLookup)
                    .map(|drb| EpochRootResult::DrbResult(epoch, drb)),
                Err(_) => membership_coordinator
                    .wait_for_catchup(epoch)
                    .await
                    .map_err(EpochManagerError::Catchup)?
                    .get_epoch_drb()
                    .await
                    .map_err(EpochManagerError::DrbLookup)
                    .map(|drb| EpochRootResult::DrbResult(epoch, drb)),
            }
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

    #[error("failed to wait for membership catchup: {0}")]
    Catchup(#[source] anytrace::Error),
}
