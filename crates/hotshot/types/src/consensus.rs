// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Provides the core consensus types

use std::{
    collections::{BTreeMap, HashMap},
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use async_lock::{RwLock, RwLockReadGuard, RwLockUpgradableReadGuard, RwLockWriteGuard};
use committable::{Commitment, Committable};
use hotshot_utils::anytrace::*;
use tracing::instrument;
use vec1::Vec1;

pub use crate::utils::{View, ViewInner};
use crate::{
    data::{Leaf2, QuorumProposalWrapper, VidCommitment, VidDisperse, VidDisperseShare},
    drb::DrbResults,
    epoch_membership::EpochMembershipCoordinator,
    error::HotShotError,
    event::{HotShotAction, LeafInfo},
    message::{Proposal, UpgradeLock},
    simple_certificate::{
        DaCertificate2, LightClientStateUpdateCertificate, NextEpochQuorumCertificate2,
        QuorumCertificate2,
    },
    simple_vote::HasEpoch,
    traits::{
        block_contents::{BlockHeader, BuilderFee},
        metrics::{Counter, Gauge, Histogram, Metrics, NoMetrics},
        node_implementation::{ConsensusTime, NodeType, Versions},
        signature_key::SignatureKey,
        BlockPayload, ValidatedState,
    },
    utils::{
        epoch_from_block_number, is_epoch_root, is_epoch_transition, is_ge_epoch_root,
        is_last_block, is_transition_block, option_epoch_from_block_number, BuilderCommitment,
        LeafCommitment, StateAndDelta, Terminator,
    },
    vote::{Certificate, HasViewNumber},
};

/// A type alias for `HashMap<Commitment<T>, T>`
pub type CommitmentMap<T> = HashMap<Commitment<T>, T>;

/// A type alias for `BTreeMap<T::Time, HashMap<T::SignatureKey, BTreeMap<T::Epoch, Proposal<T, VidDisperseShare<T>>>>>`
pub type VidShares<TYPES> = BTreeMap<
    <TYPES as NodeType>::View,
    HashMap<
        <TYPES as NodeType>::SignatureKey,
        BTreeMap<Option<<TYPES as NodeType>::Epoch>, Proposal<TYPES, VidDisperseShare<TYPES>>>,
    >,
>;

/// Type alias for consensus state wrapped in a lock.
pub type LockedConsensusState<TYPES> = Arc<RwLock<Consensus<TYPES>>>;

/// A thin wrapper around `LockedConsensusState` that helps debugging locks
#[derive(Clone, Debug)]
pub struct OuterConsensus<TYPES: NodeType> {
    /// Inner `LockedConsensusState`
    pub inner_consensus: LockedConsensusState<TYPES>,
}

impl<TYPES: NodeType> OuterConsensus<TYPES> {
    /// Create a new instance of `OuterConsensus`, hopefully uniquely named
    pub fn new(consensus: LockedConsensusState<TYPES>) -> Self {
        Self {
            inner_consensus: consensus,
        }
    }

    /// Locks inner consensus for reading and leaves debug traces
    #[instrument(skip_all, target = "OuterConsensus")]
    pub async fn read(&self) -> ConsensusReadLockGuard<'_, TYPES> {
        tracing::trace!("Trying to acquire read lock on consensus");
        let ret = self.inner_consensus.read().await;
        tracing::trace!("Acquired read lock on consensus");
        ConsensusReadLockGuard::new(ret)
    }

    /// Locks inner consensus for writing and leaves debug traces
    #[instrument(skip_all, target = "OuterConsensus")]
    pub async fn write(&self) -> ConsensusWriteLockGuard<'_, TYPES> {
        tracing::trace!("Trying to acquire write lock on consensus");
        let ret = self.inner_consensus.write().await;
        tracing::trace!("Acquired write lock on consensus");
        ConsensusWriteLockGuard::new(ret)
    }

    /// Tries to acquire write lock on inner consensus and leaves debug traces
    #[instrument(skip_all, target = "OuterConsensus")]
    pub fn try_write(&self) -> Option<ConsensusWriteLockGuard<'_, TYPES>> {
        tracing::trace!("Trying to acquire write lock on consensus");
        let ret = self.inner_consensus.try_write();
        if let Some(guard) = ret {
            tracing::trace!("Acquired write lock on consensus");
            Some(ConsensusWriteLockGuard::new(guard))
        } else {
            tracing::trace!("Failed to acquire write lock");
            None
        }
    }

    /// Acquires upgradable read lock on inner consensus and leaves debug traces
    #[instrument(skip_all, target = "OuterConsensus")]
    pub async fn upgradable_read(&self) -> ConsensusUpgradableReadLockGuard<'_, TYPES> {
        tracing::trace!("Trying to acquire upgradable read lock on consensus");
        let ret = self.inner_consensus.upgradable_read().await;
        tracing::trace!("Acquired upgradable read lock on consensus");
        ConsensusUpgradableReadLockGuard::new(ret)
    }

    /// Tries to acquire read lock on inner consensus and leaves debug traces
    #[instrument(skip_all, target = "OuterConsensus")]
    pub fn try_read(&self) -> Option<ConsensusReadLockGuard<'_, TYPES>> {
        tracing::trace!("Trying to acquire read lock on consensus");
        let ret = self.inner_consensus.try_read();
        if let Some(guard) = ret {
            tracing::trace!("Acquired read lock on consensus");
            Some(ConsensusReadLockGuard::new(guard))
        } else {
            tracing::trace!("Failed to acquire read lock");
            None
        }
    }
}

/// A thin wrapper around `RwLockReadGuard` for `Consensus` that leaves debug traces when the lock is freed
pub struct ConsensusReadLockGuard<'a, TYPES: NodeType> {
    /// Inner `RwLockReadGuard`
    lock_guard: RwLockReadGuard<'a, Consensus<TYPES>>,
}

impl<'a, TYPES: NodeType> ConsensusReadLockGuard<'a, TYPES> {
    /// Creates a new instance of `ConsensusReadLockGuard` with the same name as parent `OuterConsensus`
    #[must_use]
    pub fn new(lock_guard: RwLockReadGuard<'a, Consensus<TYPES>>) -> Self {
        Self { lock_guard }
    }
}

impl<TYPES: NodeType> Deref for ConsensusReadLockGuard<'_, TYPES> {
    type Target = Consensus<TYPES>;
    fn deref(&self) -> &Self::Target {
        &self.lock_guard
    }
}

impl<TYPES: NodeType> Drop for ConsensusReadLockGuard<'_, TYPES> {
    #[instrument(skip_all, target = "ConsensusReadLockGuard")]
    fn drop(&mut self) {
        tracing::trace!("Read lock on consensus dropped");
    }
}

/// A thin wrapper around `RwLockWriteGuard` for `Consensus` that leaves debug traces when the lock is freed
pub struct ConsensusWriteLockGuard<'a, TYPES: NodeType> {
    /// Inner `RwLockWriteGuard`
    lock_guard: RwLockWriteGuard<'a, Consensus<TYPES>>,
}

impl<'a, TYPES: NodeType> ConsensusWriteLockGuard<'a, TYPES> {
    /// Creates a new instance of `ConsensusWriteLockGuard` with the same name as parent `OuterConsensus`
    #[must_use]
    pub fn new(lock_guard: RwLockWriteGuard<'a, Consensus<TYPES>>) -> Self {
        Self { lock_guard }
    }
}

impl<TYPES: NodeType> Deref for ConsensusWriteLockGuard<'_, TYPES> {
    type Target = Consensus<TYPES>;
    fn deref(&self) -> &Self::Target {
        &self.lock_guard
    }
}

impl<TYPES: NodeType> DerefMut for ConsensusWriteLockGuard<'_, TYPES> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.lock_guard
    }
}

impl<TYPES: NodeType> Drop for ConsensusWriteLockGuard<'_, TYPES> {
    #[instrument(skip_all, target = "ConsensusWriteLockGuard")]
    fn drop(&mut self) {
        tracing::debug!("Write lock on consensus dropped");
    }
}

/// A thin wrapper around `RwLockUpgradableReadGuard` for `Consensus` that leaves debug traces when the lock is freed or upgraded
pub struct ConsensusUpgradableReadLockGuard<'a, TYPES: NodeType> {
    /// Inner `RwLockUpgradableReadGuard`
    lock_guard: ManuallyDrop<RwLockUpgradableReadGuard<'a, Consensus<TYPES>>>,
    /// A helper bool to indicate whether inner lock has been unsafely taken or not
    taken: bool,
}

impl<'a, TYPES: NodeType> ConsensusUpgradableReadLockGuard<'a, TYPES> {
    /// Creates a new instance of `ConsensusUpgradableReadLockGuard` with the same name as parent `OuterConsensus`
    #[must_use]
    pub fn new(lock_guard: RwLockUpgradableReadGuard<'a, Consensus<TYPES>>) -> Self {
        Self {
            lock_guard: ManuallyDrop::new(lock_guard),
            taken: false,
        }
    }

    /// Upgrades the inner `RwLockUpgradableReadGuard` and leaves debug traces
    #[instrument(skip_all, target = "ConsensusUpgradableReadLockGuard")]
    pub async fn upgrade(mut guard: Self) -> ConsensusWriteLockGuard<'a, TYPES> {
        let inner_guard = unsafe { ManuallyDrop::take(&mut guard.lock_guard) };
        guard.taken = true;
        tracing::debug!("Trying to upgrade upgradable read lock on consensus");
        let ret = RwLockUpgradableReadGuard::upgrade(inner_guard).await;
        tracing::debug!("Upgraded upgradable read lock on consensus");
        ConsensusWriteLockGuard::new(ret)
    }
}

impl<TYPES: NodeType> Deref for ConsensusUpgradableReadLockGuard<'_, TYPES> {
    type Target = Consensus<TYPES>;

    fn deref(&self) -> &Self::Target {
        &self.lock_guard
    }
}

impl<TYPES: NodeType> Drop for ConsensusUpgradableReadLockGuard<'_, TYPES> {
    #[instrument(skip_all, target = "ConsensusUpgradableReadLockGuard")]
    fn drop(&mut self) {
        if !self.taken {
            unsafe { ManuallyDrop::drop(&mut self.lock_guard) }
            tracing::debug!("Upgradable read lock on consensus dropped");
        }
    }
}

/// A bundle of views that we have most recently performed some action
#[derive(Debug, Clone, Copy)]
struct HotShotActionViews<T: ConsensusTime> {
    /// View we last proposed in to the Quorum
    proposed: T,
    /// View we last voted in for a QuorumProposal
    voted: T,
    /// View we last proposed to the DA committee
    da_proposed: T,
    /// View we lasted voted for DA proposal
    da_vote: T,
}

impl<T: ConsensusTime> Default for HotShotActionViews<T> {
    fn default() -> Self {
        let genesis = T::genesis();
        Self {
            proposed: genesis,
            voted: genesis,
            da_proposed: genesis,
            da_vote: genesis,
        }
    }
}
impl<T: ConsensusTime> HotShotActionViews<T> {
    /// Create HotShotActionViews from a view number
    fn from_view(view: T) -> Self {
        Self {
            proposed: view,
            voted: view,
            da_proposed: view,
            da_vote: view,
        }
    }
}
/// A reference to the consensus algorithm
///
/// This will contain the state of all rounds.
#[derive(derive_more::Debug, Clone)]
pub struct Consensus<TYPES: NodeType> {
    /// The validated states that are currently loaded in memory.
    validated_state_map: BTreeMap<TYPES::View, View<TYPES>>,

    /// All the VID shares we've received for current and future views.
    vid_shares: VidShares<TYPES>,

    /// All the DA certs we've received for current and future views.
    /// view -> DA cert
    saved_da_certs: HashMap<TYPES::View, DaCertificate2<TYPES>>,

    /// View number that is currently on.
    cur_view: TYPES::View,

    /// Epoch number that is currently on.
    cur_epoch: Option<TYPES::Epoch>,

    /// Last proposals we sent out, None if we haven't proposed yet.
    /// Prevents duplicate proposals, and can be served to those trying to catchup
    last_proposals: BTreeMap<TYPES::View, Proposal<TYPES, QuorumProposalWrapper<TYPES>>>,

    /// last view had a successful decide event
    last_decided_view: TYPES::View,

    /// The `locked_qc` view number
    locked_view: TYPES::View,

    /// Map of leaf hash -> leaf
    /// - contains undecided leaves
    /// - includes the MOST RECENT decided leaf
    saved_leaves: CommitmentMap<Leaf2<TYPES>>,

    /// Bundle of views which we performed the most recent action
    /// visibible to the network.  Actions are votes and proposals
    /// for DA and Quorum
    last_actions: HotShotActionViews<TYPES::View>,

    /// Saved payloads.
    ///
    /// Encoded transactions for every view if we got a payload for that view.
    saved_payloads: BTreeMap<TYPES::View, Arc<PayloadWithMetadata<TYPES>>>,

    /// the highqc per spec
    high_qc: QuorumCertificate2<TYPES>,

    /// The high QC for the next epoch
    next_epoch_high_qc: Option<NextEpochQuorumCertificate2<TYPES>>,

    /// A reference to the metrics trait
    pub metrics: Arc<ConsensusMetricsValue>,

    /// Number of blocks in an epoch, zero means there are no epochs
    pub epoch_height: u64,

    /// Tables for the DRB seeds and results.
    pub drb_results: DrbResults<TYPES>,

    /// The transition QC for the current epoch
    transition_qc: Option<(
        QuorumCertificate2<TYPES>,
        NextEpochQuorumCertificate2<TYPES>,
    )>,

    /// The highest block number that we have seen
    pub highest_block: u64,
    /// The light client state update certificate
    pub state_cert: Option<LightClientStateUpdateCertificate<TYPES>>,
}

/// This struct holds a payload and its metadata
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct PayloadWithMetadata<TYPES: NodeType> {
    pub payload: TYPES::BlockPayload,
    pub metadata: <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
}

/// Contains several `ConsensusMetrics` that we're interested in from the consensus interfaces
#[derive(Clone, Debug)]
pub struct ConsensusMetricsValue {
    /// The number of last synced block height
    pub last_synced_block_height: Box<dyn Gauge>,
    /// The number of last decided view
    pub last_decided_view: Box<dyn Gauge>,
    /// The number of the last voted view
    pub last_voted_view: Box<dyn Gauge>,
    /// Number of timestamp for the last decided time
    pub last_decided_time: Box<dyn Gauge>,
    /// The current view
    pub current_view: Box<dyn Gauge>,
    /// Number of views that are in-flight since the last decided view
    pub number_of_views_since_last_decide: Box<dyn Gauge>,
    /// Number of views that are in-flight since the last anchor view
    pub number_of_views_per_decide_event: Box<dyn Histogram>,
    /// Duration of views as leader
    pub view_duration_as_leader: Box<dyn Histogram>,
    /// Number of invalid QCs we've seen since the last commit.
    pub invalid_qc: Box<dyn Gauge>,
    /// Number of outstanding transactions
    pub outstanding_transactions: Box<dyn Gauge>,
    /// Memory size in bytes of the serialized transactions still outstanding
    pub outstanding_transactions_memory_size: Box<dyn Gauge>,
    /// Number of views that timed out
    pub number_of_timeouts: Box<dyn Counter>,
    /// Number of views that timed out as leader
    pub number_of_timeouts_as_leader: Box<dyn Counter>,
    /// The number of empty blocks that have been proposed
    pub number_of_empty_blocks_proposed: Box<dyn Counter>,
    /// Number of events in the hotshot event queue
    pub internal_event_queue_len: Box<dyn Gauge>,
}

impl ConsensusMetricsValue {
    /// Create a new instance of this [`ConsensusMetricsValue`] struct, setting all the counters and gauges
    #[must_use]
    pub fn new(metrics: &dyn Metrics) -> Self {
        Self {
            last_synced_block_height: metrics
                .create_gauge(String::from("last_synced_block_height"), None),
            last_decided_view: metrics.create_gauge(String::from("last_decided_view"), None),
            last_voted_view: metrics.create_gauge(String::from("last_voted_view"), None),
            last_decided_time: metrics.create_gauge(String::from("last_decided_time"), None),
            current_view: metrics.create_gauge(String::from("current_view"), None),
            number_of_views_since_last_decide: metrics
                .create_gauge(String::from("number_of_views_since_last_decide"), None),
            number_of_views_per_decide_event: metrics
                .create_histogram(String::from("number_of_views_per_decide_event"), None),
            view_duration_as_leader: metrics
                .create_histogram(String::from("view_duration_as_leader"), None),
            invalid_qc: metrics.create_gauge(String::from("invalid_qc"), None),
            outstanding_transactions: metrics
                .create_gauge(String::from("outstanding_transactions"), None),
            outstanding_transactions_memory_size: metrics
                .create_gauge(String::from("outstanding_transactions_memory_size"), None),
            number_of_timeouts: metrics.create_counter(String::from("number_of_timeouts"), None),
            number_of_timeouts_as_leader: metrics
                .create_counter(String::from("number_of_timeouts_as_leader"), None),
            number_of_empty_blocks_proposed: metrics
                .create_counter(String::from("number_of_empty_blocks_proposed"), None),
            internal_event_queue_len: metrics
                .create_gauge(String::from("internal_event_queue_len"), None),
        }
    }
}

impl Default for ConsensusMetricsValue {
    fn default() -> Self {
        Self::new(&*NoMetrics::boxed())
    }
}

impl<TYPES: NodeType> Consensus<TYPES> {
    /// Constructor.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        validated_state_map: BTreeMap<TYPES::View, View<TYPES>>,
        vid_shares: Option<VidShares<TYPES>>,
        cur_view: TYPES::View,
        cur_epoch: Option<TYPES::Epoch>,
        locked_view: TYPES::View,
        last_decided_view: TYPES::View,
        last_actioned_view: TYPES::View,
        last_proposals: BTreeMap<TYPES::View, Proposal<TYPES, QuorumProposalWrapper<TYPES>>>,
        saved_leaves: CommitmentMap<Leaf2<TYPES>>,
        saved_payloads: BTreeMap<TYPES::View, Arc<PayloadWithMetadata<TYPES>>>,
        high_qc: QuorumCertificate2<TYPES>,
        next_epoch_high_qc: Option<NextEpochQuorumCertificate2<TYPES>>,
        metrics: Arc<ConsensusMetricsValue>,
        epoch_height: u64,
        state_cert: Option<LightClientStateUpdateCertificate<TYPES>>,
    ) -> Self {
        let transition_qc = if let Some(ref next_epoch_high_qc) = next_epoch_high_qc {
            if high_qc
                .data
                .block_number
                .is_some_and(|bn| is_transition_block(bn, epoch_height))
            {
                if high_qc.data.leaf_commit == next_epoch_high_qc.data.leaf_commit {
                    Some((high_qc.clone(), next_epoch_high_qc.clone()))
                } else {
                    tracing::error!("Next epoch high QC has different leaf commit to high QC");
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        Consensus {
            validated_state_map,
            vid_shares: vid_shares.unwrap_or_default(),
            saved_da_certs: HashMap::new(),
            cur_view,
            cur_epoch,
            last_decided_view,
            last_proposals,
            last_actions: HotShotActionViews::from_view(last_actioned_view),
            locked_view,
            saved_leaves,
            saved_payloads,
            high_qc,
            next_epoch_high_qc,
            metrics,
            epoch_height,
            drb_results: DrbResults::new(),
            transition_qc,
            highest_block: 0,
            state_cert,
        }
    }

    /// Get the current view.
    pub fn cur_view(&self) -> TYPES::View {
        self.cur_view
    }

    /// Get the current epoch.
    pub fn cur_epoch(&self) -> Option<TYPES::Epoch> {
        self.cur_epoch
    }

    /// Get the last decided view.
    pub fn last_decided_view(&self) -> TYPES::View {
        self.last_decided_view
    }

    /// Get the locked view.
    pub fn locked_view(&self) -> TYPES::View {
        self.locked_view
    }

    /// Get the high QC.
    pub fn high_qc(&self) -> &QuorumCertificate2<TYPES> {
        &self.high_qc
    }

    /// Get the transition QC.
    pub fn transition_qc(
        &self,
    ) -> Option<&(
        QuorumCertificate2<TYPES>,
        NextEpochQuorumCertificate2<TYPES>,
    )> {
        self.transition_qc.as_ref()
    }

    ///Update the highest block number
    pub fn update_highest_block(&mut self, block_number: u64) {
        if block_number > self.highest_block {
            self.highest_block = block_number;
            return;
        }

        if is_epoch_transition(block_number, self.epoch_height) {
            let new_epoch = epoch_from_block_number(block_number, self.epoch_height);
            let high_epoch = epoch_from_block_number(self.highest_block, self.epoch_height);
            if new_epoch >= high_epoch {
                self.highest_block = block_number;
            }
        }
    }

    /// Update the transition QC.
    pub fn update_transition_qc(
        &mut self,
        qc: QuorumCertificate2<TYPES>,
        next_epoch_qc: NextEpochQuorumCertificate2<TYPES>,
    ) {
        if next_epoch_qc.data.leaf_commit != qc.data().leaf_commit {
            tracing::error!(
                "Next epoch QC for view {:?} has different leaf commit {:?} to {:?}",
                qc.view_number(),
                next_epoch_qc.data.leaf_commit,
                qc.data().leaf_commit
            );
            return;
        }
        if let Some((transition_qc, _)) = &self.transition_qc {
            if transition_qc.view_number() >= qc.view_number() {
                return;
            }
        }
        self.transition_qc = Some((qc, next_epoch_qc));
    }

    /// Get the current light client state certificate
    pub fn state_cert(&self) -> Option<&LightClientStateUpdateCertificate<TYPES>> {
        self.state_cert.as_ref()
    }

    /// Get the next epoch high QC.
    pub fn next_epoch_high_qc(&self) -> Option<&NextEpochQuorumCertificate2<TYPES>> {
        self.next_epoch_high_qc.as_ref()
    }

    /// Get the validated state map.
    pub fn validated_state_map(&self) -> &BTreeMap<TYPES::View, View<TYPES>> {
        &self.validated_state_map
    }

    /// Get the saved leaves.
    pub fn saved_leaves(&self) -> &CommitmentMap<Leaf2<TYPES>> {
        &self.saved_leaves
    }

    /// Get the saved payloads.
    pub fn saved_payloads(&self) -> &BTreeMap<TYPES::View, Arc<PayloadWithMetadata<TYPES>>> {
        &self.saved_payloads
    }

    /// Get the vid shares.
    pub fn vid_shares(&self) -> &VidShares<TYPES> {
        &self.vid_shares
    }

    /// Get the saved DA certs.
    pub fn saved_da_certs(&self) -> &HashMap<TYPES::View, DaCertificate2<TYPES>> {
        &self.saved_da_certs
    }

    /// Get the map of our recent proposals
    pub fn last_proposals(
        &self,
    ) -> &BTreeMap<TYPES::View, Proposal<TYPES, QuorumProposalWrapper<TYPES>>> {
        &self.last_proposals
    }

    /// Update the current view.
    /// # Errors
    /// Can return an error when the new view_number is not higher than the existing view number.
    pub fn update_view(&mut self, view_number: TYPES::View) -> Result<()> {
        ensure!(
            view_number > self.cur_view,
            debug!("New view isn't newer than the current view.")
        );
        self.cur_view = view_number;
        Ok(())
    }

    /// Get the parent Leaf Info from a given leaf and our public key.
    /// Returns None if we don't have the data in out state
    pub async fn parent_leaf_info(
        &self,
        leaf: &Leaf2<TYPES>,
        public_key: &TYPES::SignatureKey,
    ) -> Option<LeafInfo<TYPES>> {
        let parent_view_number = leaf.justify_qc().view_number();
        let parent_epoch = leaf.justify_qc().epoch();
        let parent_leaf = self
            .saved_leaves
            .get(&leaf.justify_qc().data().leaf_commit)?;
        let parent_state_and_delta = self.state_and_delta(parent_view_number);
        let (Some(state), delta) = parent_state_and_delta else {
            return None;
        };

        let parent_vid = self
            .vid_shares()
            .get(&parent_view_number)
            .and_then(|key_map| key_map.get(public_key).cloned())
            .and_then(|epoch_map| epoch_map.get(&parent_epoch).cloned())
            .map(|prop| prop.data);

        let state_cert = if parent_leaf.with_epoch
            && is_epoch_root(parent_leaf.block_header().block_number(), self.epoch_height)
        {
            match self.state_cert() {
                // Sanity check that the state cert is for the same view as the parent leaf
                Some(state_cert)
                    if state_cert.light_client_state.view_number == parent_view_number.u64() =>
                {
                    Some(state_cert.clone())
                },
                _ => None,
            }
        } else {
            None
        };

        Some(LeafInfo {
            leaf: parent_leaf.clone(),
            state,
            delta,
            vid_share: parent_vid,
            state_cert,
        })
    }

    /// Update the current epoch.
    /// # Errors
    /// Can return an error when the new epoch_number is not higher than the existing epoch number.
    pub fn update_epoch(&mut self, epoch_number: TYPES::Epoch) -> Result<()> {
        ensure!(
            self.cur_epoch.is_none() || Some(epoch_number) > self.cur_epoch,
            debug!("New epoch isn't newer than the current epoch.")
        );
        tracing::trace!(
            "Updating epoch from {:?} to {}",
            self.cur_epoch,
            epoch_number
        );
        self.cur_epoch = Some(epoch_number);
        Ok(())
    }

    /// Update the last actioned view internally for votes and proposals
    ///
    /// Returns true if the action is for a newer view than the last action of that type
    pub fn update_action(&mut self, action: HotShotAction, view: TYPES::View) -> bool {
        let old_view = match action {
            HotShotAction::Vote => &mut self.last_actions.voted,
            HotShotAction::Propose => &mut self.last_actions.proposed,
            HotShotAction::DaPropose => &mut self.last_actions.da_proposed,
            HotShotAction::DaVote => {
                if view > self.last_actions.da_vote {
                    self.last_actions.da_vote = view;
                }
                // TODO Add logic to prevent double voting.  For now the simple check if
                // the last voted view is less than the view we are trying to vote doesn't work
                // because the leader of view n + 1 may propose to the DA (and we would vote)
                // before the leader of view n.
                return true;
            },
            _ => return true,
        };
        if view > *old_view {
            *old_view = view;
            return true;
        }
        false
    }

    /// reset last actions to genesis so we can resend events in tests
    pub fn reset_actions(&mut self) {
        self.last_actions = HotShotActionViews::default();
    }

    /// Update the last proposal.
    ///
    /// # Errors
    /// Can return an error when the new view_number is not higher than the existing proposed view number.
    pub fn update_proposed_view(
        &mut self,
        proposal: Proposal<TYPES, QuorumProposalWrapper<TYPES>>,
    ) -> Result<()> {
        ensure!(
            proposal.data.view_number()
                > self
                    .last_proposals
                    .last_key_value()
                    .map_or(TYPES::View::genesis(), |(k, _)| { *k }),
            debug!("New view isn't newer than the previously proposed view.")
        );
        self.last_proposals
            .insert(proposal.data.view_number(), proposal);
        Ok(())
    }

    /// Update the last decided view.
    ///
    /// # Errors
    /// Can return an error when the new view_number is not higher than the existing decided view number.
    pub fn update_last_decided_view(&mut self, view_number: TYPES::View) -> Result<()> {
        ensure!(
            view_number > self.last_decided_view,
            debug!("New view isn't newer than the previously decided view.")
        );
        self.last_decided_view = view_number;
        Ok(())
    }

    /// Update the locked view.
    ///
    /// # Errors
    /// Can return an error when the new view_number is not higher than the existing locked view number.
    pub fn update_locked_view(&mut self, view_number: TYPES::View) -> Result<()> {
        ensure!(
            view_number > self.locked_view,
            debug!("New view isn't newer than the previously locked view.")
        );
        self.locked_view = view_number;
        Ok(())
    }

    /// Update the validated state map with a new view_number/view combo.
    ///
    /// # Errors
    /// Can return an error when the new view contains less information than the existing view
    /// with the same view number.
    pub fn update_da_view(
        &mut self,
        view_number: TYPES::View,
        epoch: Option<TYPES::Epoch>,
        payload_commitment: VidCommitment,
    ) -> Result<()> {
        let view = View {
            view_inner: ViewInner::Da {
                payload_commitment,
                epoch,
            },
        };
        self.update_validated_state_map(view_number, view)
    }

    /// Update the validated state map with a new view_number/view combo.
    ///
    /// # Errors
    /// Can return an error when the new view contains less information than the existing view
    /// with the same view number.
    pub fn update_leaf(
        &mut self,
        leaf: Leaf2<TYPES>,
        state: Arc<TYPES::ValidatedState>,
        delta: Option<Arc<<TYPES::ValidatedState as ValidatedState<TYPES>>::Delta>>,
    ) -> Result<()> {
        let view_number = leaf.view_number();
        let epoch = option_epoch_from_block_number::<TYPES>(
            leaf.with_epoch,
            leaf.height(),
            self.epoch_height,
        );
        let view = View {
            view_inner: ViewInner::Leaf {
                leaf: leaf.commit(),
                state,
                delta,
                epoch,
            },
        };
        self.update_validated_state_map(view_number, view)?;
        self.update_saved_leaves(leaf);
        Ok(())
    }

    /// Update the validated state map with a new view_number/view combo.
    ///
    /// # Errors
    /// Can return an error when the new view contains less information than the existing view
    /// with the same view number.
    fn update_validated_state_map(
        &mut self,
        view_number: TYPES::View,
        new_view: View<TYPES>,
    ) -> Result<()> {
        if let Some(existing_view) = self.validated_state_map().get(&view_number) {
            if let ViewInner::Leaf {
                delta: ref existing_delta,
                ..
            } = existing_view.view_inner
            {
                if let ViewInner::Leaf {
                    delta: ref new_delta,
                    ..
                } = new_view.view_inner
                {
                    ensure!(
                         new_delta.is_some() || existing_delta.is_none(),
                         debug!("Skipping the state update to not override a `Leaf` view with `Some` state delta.")
                     );
                } else {
                    bail!("Skipping the state update to not override a `Leaf` view with a non-`Leaf` view.");
                }
            }
        }
        self.validated_state_map.insert(view_number, new_view);
        Ok(())
    }

    /// Update the saved leaves with a new leaf.
    fn update_saved_leaves(&mut self, leaf: Leaf2<TYPES>) {
        self.saved_leaves.insert(leaf.commit(), leaf);
    }

    /// Update the saved payloads with a new encoded transaction.
    ///
    /// # Errors
    /// Can return an error when there's an existing payload corresponding to the same view number.
    pub fn update_saved_payloads(
        &mut self,
        view_number: TYPES::View,
        payload: Arc<PayloadWithMetadata<TYPES>>,
    ) -> Result<()> {
        ensure!(
            !self.saved_payloads.contains_key(&view_number),
            "Payload with the same view already exists."
        );
        self.saved_payloads.insert(view_number, payload);
        Ok(())
    }

    /// Update the high QC if given a newer one.
    /// # Errors
    /// Can return an error when the provided high_qc is not newer than the existing entry.
    pub fn update_high_qc(&mut self, high_qc: QuorumCertificate2<TYPES>) -> Result<()> {
        if self.high_qc == high_qc {
            return Ok(());
        }
        // make sure the we don't update the high QC unless is't a higher view
        ensure!(
            high_qc.view_number > self.high_qc.view_number,
            debug!("High QC with an equal or higher view exists.")
        );
        tracing::debug!("Updating high QC");
        self.high_qc = high_qc;

        Ok(())
    }

    /// Update the next epoch high QC if given a newer one.
    /// # Errors
    /// Can return an error when the provided high_qc is not newer than the existing entry.
    /// # Panics
    /// It can't actually panic. If the option is None, we will not call unwrap on it.
    pub fn update_next_epoch_high_qc(
        &mut self,
        high_qc: NextEpochQuorumCertificate2<TYPES>,
    ) -> Result<()> {
        if self.next_epoch_high_qc.as_ref() == Some(&high_qc) {
            return Ok(());
        }
        if let Some(next_epoch_high_qc) = self.next_epoch_high_qc() {
            ensure!(
                high_qc.view_number > next_epoch_high_qc.view_number,
                debug!("Next epoch high QC with an equal or higher view exists.")
            );
        }
        tracing::debug!("Updating next epoch high QC");
        self.next_epoch_high_qc = Some(high_qc);

        Ok(())
    }

    /// Resets high qc and next epoch qc to the provided transition qc.
    /// # Errors
    /// Can return an error when the provided high_qc is not newer than the existing entry.
    pub fn reset_high_qc(
        &mut self,
        high_qc: QuorumCertificate2<TYPES>,
        next_epoch_qc: NextEpochQuorumCertificate2<TYPES>,
    ) -> Result<()> {
        ensure!(
            high_qc.data.leaf_commit == next_epoch_qc.data.leaf_commit,
            error!("High QC's and next epoch QC's leaf commits do not match.")
        );
        if self.high_qc == high_qc {
            return Ok(());
        }
        let same_epoch = high_qc.data.block_number.is_some_and(|bn| {
            let current_qc = self.high_qc();
            let Some(high_bn) = current_qc.data.block_number else {
                return false;
            };
            epoch_from_block_number(bn + 1, self.epoch_height)
                == epoch_from_block_number(high_bn + 1, self.epoch_height)
        });
        ensure!(
            high_qc
                .data
                .block_number
                .is_some_and(|bn| is_transition_block(bn, self.epoch_height))
                && same_epoch,
            error!("Provided QC is not a transition QC.")
        );
        tracing::debug!("Resetting high QC and next epoch high QC");
        self.high_qc = high_qc;
        self.next_epoch_high_qc = Some(next_epoch_qc);

        Ok(())
    }

    /// Update the light client state update certificate if given a newer one.
    /// # Errors
    /// Can return an error when the provided state_cert is not newer than the existing entry.
    pub fn update_state_cert(
        &mut self,
        state_cert: LightClientStateUpdateCertificate<TYPES>,
    ) -> Result<()> {
        if let Some(existing_state_cert) = &self.state_cert {
            ensure!(
                state_cert.epoch > existing_state_cert.epoch,
                debug!(
                    "Light client state update certification with an equal or higher epoch exists."
                )
            );
        }
        tracing::debug!("Updating light client state update certification");
        self.state_cert = Some(state_cert);

        Ok(())
    }

    /// Add a new entry to the vid_shares map.
    pub fn update_vid_shares(
        &mut self,
        view_number: TYPES::View,
        disperse: Proposal<TYPES, VidDisperseShare<TYPES>>,
    ) {
        self.vid_shares
            .entry(view_number)
            .or_default()
            .entry(disperse.data.recipient_key().clone())
            .or_default()
            .insert(disperse.data.target_epoch(), disperse);
    }

    /// Add a new entry to the da_certs map.
    pub fn update_saved_da_certs(&mut self, view_number: TYPES::View, cert: DaCertificate2<TYPES>) {
        self.saved_da_certs.insert(view_number, cert);
    }

    /// gather information from the parent chain of leaves
    /// # Errors
    /// If the leaf or its ancestors are not found in storage
    pub fn visit_leaf_ancestors<F>(
        &self,
        start_from: TYPES::View,
        terminator: Terminator<TYPES::View>,
        ok_when_finished: bool,
        mut f: F,
    ) -> std::result::Result<(), HotShotError<TYPES>>
    where
        F: FnMut(
            &Leaf2<TYPES>,
            Arc<<TYPES as NodeType>::ValidatedState>,
            Option<Arc<<<TYPES as NodeType>::ValidatedState as ValidatedState<TYPES>>::Delta>>,
        ) -> bool,
    {
        let mut next_leaf = if let Some(view) = self.validated_state_map.get(&start_from) {
            view.leaf_commitment().ok_or_else(|| {
                HotShotError::InvalidState(format!(
                    "Visited failed view {start_from:?} leaf. Expected successful leaf"
                ))
            })?
        } else {
            return Err(HotShotError::InvalidState(format!(
                "View {start_from:?} leaf does not exist in state map "
            )));
        };

        while let Some(leaf) = self.saved_leaves.get(&next_leaf) {
            let view = leaf.view_number();
            if let (Some(state), delta) = self.state_and_delta(view) {
                if let Terminator::Exclusive(stop_before) = terminator {
                    if stop_before == view {
                        if ok_when_finished {
                            return Ok(());
                        }
                        break;
                    }
                }
                next_leaf = leaf.parent_commitment();
                if !f(leaf, state, delta) {
                    return Ok(());
                }
                if let Terminator::Inclusive(stop_after) = terminator {
                    if stop_after == view {
                        if ok_when_finished {
                            return Ok(());
                        }
                        break;
                    }
                }
            } else {
                return Err(HotShotError::InvalidState(format!(
                    "View {view:?} state does not exist in state map"
                )));
            }
        }
        Err(HotShotError::MissingLeaf(next_leaf))
    }

    /// Garbage collects based on state change right now, this removes from both the
    /// `saved_payloads` and `validated_state_map` fields of `Consensus`.
    /// # Panics
    /// On inconsistent stored entries
    pub fn collect_garbage(&mut self, old_anchor_view: TYPES::View, new_anchor_view: TYPES::View) {
        // Nothing to collect
        if new_anchor_view <= old_anchor_view {
            return;
        }
        let gc_view = TYPES::View::new(new_anchor_view.saturating_sub(1));
        // state check
        let anchor_entry = self
            .validated_state_map
            .iter()
            .next()
            .expect("INCONSISTENT STATE: anchor leaf not in state map!");
        if **anchor_entry.0 != old_anchor_view.saturating_sub(1) {
            tracing::info!(
                "Something about GC has failed. Older leaf exists than the previous anchor leaf."
            );
        }
        // perform gc
        self.saved_da_certs
            .retain(|view_number, _| *view_number >= old_anchor_view);
        self.validated_state_map
            .range(old_anchor_view..gc_view)
            .filter_map(|(_view_number, view)| view.leaf_commitment())
            .for_each(|leaf| {
                self.saved_leaves.remove(&leaf);
            });
        self.validated_state_map = self.validated_state_map.split_off(&gc_view);
        self.saved_payloads = self.saved_payloads.split_off(&gc_view);
        self.vid_shares = self.vid_shares.split_off(&gc_view);
        self.last_proposals = self.last_proposals.split_off(&gc_view);
    }

    /// Gets the last decided leaf.
    ///
    /// # Panics
    /// if the last decided view's leaf does not exist in the state map or saved leaves, which
    /// should never happen.
    #[must_use]
    pub fn decided_leaf(&self) -> Leaf2<TYPES> {
        let decided_view_num = self.last_decided_view;
        let view = self.validated_state_map.get(&decided_view_num).unwrap();
        let leaf = view
            .leaf_commitment()
            .expect("Decided leaf not found! Consensus internally inconsistent");
        self.saved_leaves.get(&leaf).unwrap().clone()
    }

    pub fn undecided_leaves(&self) -> Vec<Leaf2<TYPES>> {
        self.saved_leaves.values().cloned().collect::<Vec<_>>()
    }

    /// Gets the validated state with the given view number, if in the state map.
    #[must_use]
    pub fn state(&self, view_number: TYPES::View) -> Option<&Arc<TYPES::ValidatedState>> {
        match self.validated_state_map.get(&view_number) {
            Some(view) => view.state(),
            None => None,
        }
    }

    /// Gets the validated state and state delta with the given view number, if in the state map.
    #[must_use]
    pub fn state_and_delta(&self, view_number: TYPES::View) -> StateAndDelta<TYPES> {
        match self.validated_state_map.get(&view_number) {
            Some(view) => view.state_and_delta(),
            None => (None, None),
        }
    }

    /// Gets the last decided validated state.
    ///
    /// # Panics
    /// If the last decided view's state does not exist in the state map, which should never
    /// happen.
    #[must_use]
    pub fn decided_state(&self) -> Arc<TYPES::ValidatedState> {
        let decided_view_num = self.last_decided_view;
        self.state_and_delta(decided_view_num)
            .0
            .expect("Decided state not found! Consensus internally inconsistent")
    }

    /// Associated helper function:
    /// Takes `LockedConsensusState` which will be updated; locks it for read and write accordingly.
    /// Calculates `VidDisperse` based on the view, the txns and the membership,
    /// and updates `vid_shares` map with the signed `VidDisperseShare` proposals.
    /// Returned `Option` indicates whether the update has actually happened or not.
    #[instrument(skip_all, target = "Consensus", fields(view = *view))]
    pub async fn calculate_and_update_vid<V: Versions>(
        consensus: OuterConsensus<TYPES>,
        view: <TYPES as NodeType>::View,
        target_epoch: Option<<TYPES as NodeType>::Epoch>,
        membership_coordinator: EpochMembershipCoordinator<TYPES>,
        private_key: &<TYPES::SignatureKey as SignatureKey>::PrivateKey,
        upgrade_lock: &UpgradeLock<TYPES, V>,
    ) -> Option<()> {
        let payload_with_metadata = Arc::clone(consensus.read().await.saved_payloads().get(&view)?);
        let epoch = consensus
            .read()
            .await
            .validated_state_map()
            .get(&view)?
            .view_inner
            .epoch()?;

        let vid = VidDisperse::calculate_vid_disperse::<V>(
            &payload_with_metadata.payload,
            &membership_coordinator,
            view,
            target_epoch,
            epoch,
            &payload_with_metadata.metadata,
            upgrade_lock,
        )
        .await
        .ok()?;

        let shares = VidDisperseShare::from_vid_disperse(vid);
        let mut consensus_writer = consensus.write().await;
        for share in shares {
            if let Some(prop) = share.to_proposal(private_key) {
                consensus_writer.update_vid_shares(view, prop);
            }
        }

        Some(())
    }
    /// Returns true if a given leaf is for the epoch transition
    pub fn is_epoch_transition(&self, leaf_commit: LeafCommitment<TYPES>) -> bool {
        let Some(leaf) = self.saved_leaves.get(&leaf_commit) else {
            tracing::trace!("We don't have a leaf corresponding to the leaf commit");
            return false;
        };
        let block_height = leaf.height();
        is_epoch_transition(block_height, self.epoch_height)
    }

    /// Returns true if our high QC is for the last block in the epoch
    pub fn is_high_qc_for_last_block(&self) -> bool {
        let Some(leaf) = self.saved_leaves.get(&self.high_qc().data.leaf_commit) else {
            tracing::trace!("We don't have a leaf corresponding to the high QC");
            return false;
        };
        let block_height = leaf.height();
        is_epoch_transition(block_height, self.epoch_height)
    }

    /// Returns true if the `parent_leaf` formed an eQC for the previous epoch to the `proposed_leaf`
    pub fn check_eqc(&self, proposed_leaf: &Leaf2<TYPES>, parent_leaf: &Leaf2<TYPES>) -> bool {
        if parent_leaf.view_number() == TYPES::View::genesis() {
            return true;
        }
        let new_epoch = epoch_from_block_number(proposed_leaf.height(), self.epoch_height);
        let old_epoch = epoch_from_block_number(parent_leaf.height(), self.epoch_height);

        new_epoch - 1 == old_epoch && is_last_block(parent_leaf.height(), self.epoch_height)
    }

    /// Returns true if our high QC is for the block equal or greater than the root epoch block
    pub fn is_high_qc_ge_root_block(&self) -> bool {
        let Some(leaf) = self.saved_leaves.get(&self.high_qc().data.leaf_commit) else {
            tracing::trace!("We don't have a leaf corresponding to the high QC");
            return false;
        };
        let block_height = leaf.height();
        is_ge_epoch_root(block_height, self.epoch_height)
    }
}

/// Alias for the block payload commitment and the associated metadata. The primary data
/// needed in order to submit a proposal.
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct CommitmentAndMetadata<TYPES: NodeType> {
    /// Vid Commitment
    pub commitment: VidCommitment,
    /// Builder Commitment
    pub builder_commitment: BuilderCommitment,
    /// Metadata for the block payload
    pub metadata: <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    /// Builder fee data
    pub fees: Vec1<BuilderFee<TYPES>>,
    /// View number this block is for
    pub block_view: TYPES::View,
    /// auction result that the block was produced from, if any
    pub auction_result: Option<TYPES::AuctionResult>,
}
