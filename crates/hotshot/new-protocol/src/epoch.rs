use std::collections::HashMap;

use hotshot_types::{
    data::{EpochNumber, Leaf2},
    drb::DrbResult,
    epoch_membership::EpochMembershipCoordinator,
    traits::{block_contents::BlockHeader, election::Membership, node_implementation::NodeType},
    utils::is_epoch_root,
};
use tokio::task::{AbortHandle, JoinSet};
use tracing::{error, warn};

pub enum EpochRootResult {
    DrbResult(EpochNumber, DrbResult),
    RootAdded(EpochNumber),
}

/// Manager for Epoch specific rules and actions
/// - Adding Epoch Root, DRB Result, and Light Client State Update Certificate
/// - Verififying Stake Table, DRB Result, and Light Client State Update Certificate
/// - Sending Epoch Termination Signals
pub struct EpochManager<T: NodeType> {
    epoch_height: u64,
    membership_coordinator: EpochMembershipCoordinator<T>,
    tasks: JoinSet<Result<EpochRootResult, ()>>,
    handles: HashMap<EpochNumber, Vec<AbortHandle>>,
}

impl<T: NodeType> EpochManager<T> {
    pub fn new(epoch_height: u64, membership_coordinator: EpochMembershipCoordinator<T>) -> Self {
        Self {
            epoch_height,
            membership_coordinator,
            tasks: JoinSet::new(),
            handles: HashMap::new(),
        }
    }

    pub async fn next(&mut self) -> Option<Result<EpochRootResult, ()>> {
        match self.tasks.join_next().await {
            Some(Ok(result)) => Some(result),
            Some(Err(_)) => None,
            None => None,
        }
    }

    pub fn handle_leaf_decided(&mut self, leaf: Leaf2<T>) {
        if !is_epoch_root(leaf.block_header().block_number(), self.epoch_height) {
            return;
        }

        let membership = self.membership_coordinator.clone();

        let Some(epoch) = leaf.epoch(self.epoch_height) else {
            error!("Leaf has no epoch");
            return;
        };

        let handles = self.handles.entry(epoch).or_default();
        let header = leaf.block_header().clone();

        handles.push(self.tasks.spawn(async move {
            let mem = membership.membership().clone();
            T::Membership::add_epoch_root(mem, header.clone())
                .await
                .map_err(|e| warn!("Failed to add epoch root: {}", e))
                .map(|_| EpochRootResult::RootAdded(epoch))
        }));

        let membership_coordinator = self.membership_coordinator.clone();
        handles.push(self.tasks.spawn(async move {
            let drb = membership_coordinator.compute_drb_result(epoch, leaf).await;
            drb.map_err(|e| warn!("Failed to compute DRB: {}", e))
                .map(|drb| EpochRootResult::DrbResult(epoch, drb))
        }));
    }

    pub fn gc(&mut self, epoch: EpochNumber) {
        let handles = self.handles.remove(&epoch).unwrap_or_default();
        for handle in handles {
            handle.abort();
        }
    }
    pub fn request_drb_result(&mut self, epoch: EpochNumber) {
        let membership_coordinator = self.membership_coordinator.clone();
        self.tasks.spawn(async move {
            match membership_coordinator
                .stake_table_for_epoch(Some(epoch))
                .await
            {
                Ok(stake_table) => stake_table
                    .get_epoch_drb()
                    .await
                    .map(|drb| EpochRootResult::DrbResult(epoch, drb))
                    .map_err(|e| warn!("Failed to get DRB result: {}", e)),
                Err(_) => {
                    let stake_table = membership_coordinator
                        .wait_for_catchup(epoch)
                        .await
                        .map_err(|e| warn!("Failed to wait for catchup: {}", e))?;
                    stake_table
                        .get_epoch_drb()
                        .await
                        .map(|drb| EpochRootResult::DrbResult(epoch, drb))
                        .map_err(|e| warn!("Failed to get DRB result: {}", e))
                },
            }
        });
    }
}
