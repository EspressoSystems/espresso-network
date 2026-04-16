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
use tokio::{
    sync::{mpsc, oneshot},
    task::{AbortHandle, JoinSet},
};
use tracing::{debug, error, warn};

use crate::leaf_store::{DecidedLeafEntry, EpochLeafStore, LeafFetchRequest};

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
    leaf_store: EpochLeafStore<T>,
    /// Sender cloned into spawned tasks so they can request leaves.
    fetch_tx: mpsc::UnboundedSender<LeafFetchRequest<T>>,
    /// Pending leaf fetch responses, keyed by block height.
    pending_fetches: BTreeMap<u64, Vec<oneshot::Sender<DecidedLeafEntry<T>>>>,
    /// Epochs for which a `request_drb_result` task is already in flight,
    /// used to avoid spawning duplicate fetch tasks.
    pending_drb_requests: BTreeSet<EpochNumber>,
}

impl<T: NodeType> EpochManager<T> {
    /// Create a new `EpochManager`.
    ///
    /// Returns `(self, fetch_rx)`. The caller (coordinator) owns the
    /// `fetch_rx` and polls it in its select loop to send leaf requests
    /// over the network.
    pub fn new<B>(
        epoch_height: B,
        membership_coordinator: EpochMembershipCoordinator<T>,
        leaf_store: EpochLeafStore<T>,
    ) -> (Self, mpsc::UnboundedReceiver<LeafFetchRequest<T>>)
    where
        B: Into<BlockNumber>,
    {
        let (fetch_tx, fetch_rx) = mpsc::unbounded_channel();
        (
            Self {
                epoch_height: epoch_height.into(),
                membership_coordinator,
                tasks: JoinSet::new(),
                handles: BTreeMap::new(),
                leaf_store,
                fetch_tx,
                pending_fetches: BTreeMap::new(),
                pending_drb_requests: BTreeSet::new(),
            },
            fetch_rx,
        )
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

    /// Handle a leaf response received from a peer. Resolves any pending
    /// oneshot senders waiting for this block height.
    pub fn handle_leaf_response(&mut self, entry: DecidedLeafEntry<T>) {
        let height = entry.leaf.height();
        if let Some(senders) = self.pending_fetches.remove(&height) {
            for sender in senders {
                let _ = sender.send(entry.clone());
            }
        }
    }

    pub fn leaf_store(&self) -> &EpochLeafStore<T> {
        &self.leaf_store
    }

    /// Register a pending leaf fetch so that [`handle_leaf_response`] can
    /// resolve it when the response arrives from a peer.
    pub fn register_pending_fetch(
        &mut self,
        height: u64,
        sender: oneshot::Sender<DecidedLeafEntry<T>>,
    ) {
        self.pending_fetches.entry(height).or_default().push(sender);
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
        let fetch_tx = self.fetch_tx.clone();
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
                    // Not in local store -- request from peers.
                    debug!(%epoch, %root_height, "requesting epoch root leaf from peers");
                    fetch_leaf_from_peers(&fetch_tx, root_height)
                        .await
                        .map_err(|_| {
                            EpochManagerError::Catchup(anyhow::anyhow!(
                                "Failed to fetch epoch root leaf at height {root_height}"
                            ))
                        })?
                },
            };

            // Register the epoch root with the membership.
            let membership = membership_coordinator.membership().clone();
            T::Membership::add_epoch_root(membership, root_entry.leaf.block_header().clone())
                .await
                .map_err(EpochManagerError::EpochRoot)?;

            // Try to get the DRB from the transition leaf in the local
            // store (which carries next_drb_result).  We intentionally do
            // NOT fetch the transition leaf from peers here because it may
            // not have been decided yet (the transition block is often the
            // very block we need the DRB to propose), and the 10-second
            // fetch timeout would block catchup.
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

/// Send a leaf fetch request through the channel and await the response.
async fn fetch_leaf_from_peers<T: NodeType>(
    fetch_tx: &mpsc::UnboundedSender<LeafFetchRequest<T>>,
    height: u64,
) -> Result<DecidedLeafEntry<T>, ()> {
    let (response_tx, response_rx) = oneshot::channel();
    fetch_tx
        .send(LeafFetchRequest {
            height,
            response_tx,
        })
        .map_err(|_| {
            warn!(%height, "leaf fetch channel closed");
        })?;
    tokio::time::timeout(std::time::Duration::from_secs(10), response_rx)
        .await
        .map_err(|_| {
            warn!(%height, "leaf fetch timed out");
        })?
        .map_err(|_| {
            warn!(%height, "leaf fetch oneshot dropped");
        })
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
    Catchup(#[source] anyhow::Error),
}
