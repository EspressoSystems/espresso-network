// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{collections::BTreeMap, fmt::Debug, sync::Arc, time::Duration};

use async_broadcast::{Receiver, Sender};
use async_lock::RwLock;
use async_trait::async_trait;
use hotshot_task::task::TaskState;
use hotshot_types::{
    epoch_membership::{EpochMembership, EpochMembershipCoordinator},
    message::UpgradeLock,
    simple_certificate::{
        ViewSyncCommitCertificate2, ViewSyncFinalizeCertificate2, ViewSyncPreCommitCertificate2,
    },
    simple_vote::{
        HasEpoch, ViewSyncCommitData2, ViewSyncCommitVote2, ViewSyncFinalizeData2,
        ViewSyncFinalizeVote2, ViewSyncPreCommitData2, ViewSyncPreCommitVote2,
    },
    stake_table::StakeTableEntries,
    traits::{
        node_implementation::{ConsensusTime, NodeType, Versions},
        signature_key::SignatureKey,
    },
    utils::EpochTransitionIndicator,
    vote::{Certificate, HasViewNumber, Vote},
};
use hotshot_utils::anytrace::*;
use tokio::{spawn, task::JoinHandle, time::sleep};
use tracing::instrument;

use crate::{
    events::{HotShotEvent, HotShotTaskCompleted},
    helpers::{broadcast_event, broadcast_view_change},
    vote_collection::{
        create_vote_accumulator, AccumulatorInfo, HandleVoteEvent, VoteCollectionTaskState,
    },
};

#[derive(PartialEq, PartialOrd, Clone, Debug, Eq, Hash)]
/// Phases of view sync
pub enum ViewSyncPhase {
    /// No phase; before the protocol has begun
    None,
    /// PreCommit phase
    PreCommit,
    /// Commit phase
    Commit,
    /// Finalize phase
    Finalize,
}

type TaskMap<TYPES, VAL> =
    BTreeMap<Option<<TYPES as NodeType>::Epoch>, BTreeMap<<TYPES as NodeType>::View, VAL>>;

/// Type alias for a map from View Number to Relay to Vote Task
type RelayMap<TYPES, VOTE, CERT, V> =
    TaskMap<TYPES, BTreeMap<u64, VoteCollectionTaskState<TYPES, VOTE, CERT, V>>>;

type ReplicaTaskMap<TYPES, V> = TaskMap<TYPES, ViewSyncReplicaTaskState<TYPES, V>>;

/// Main view sync task state
pub struct ViewSyncTaskState<TYPES: NodeType, V: Versions> {
    /// View HotShot is currently in
    pub cur_view: TYPES::View,

    /// View HotShot wishes to be in
    pub next_view: TYPES::View,

    /// Epoch HotShot is currently in
    pub cur_epoch: Option<TYPES::Epoch>,

    /// Membership for the quorum
    pub membership_coordinator: EpochMembershipCoordinator<TYPES>,

    /// This Nodes Public Key
    pub public_key: TYPES::SignatureKey,

    /// Our Private Key
    pub private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,

    /// Our node id; for logging
    pub id: u64,

    /// How many timeouts we've seen in a row; is reset upon a successful view change
    pub num_timeouts_tracked: u64,

    /// Map of running replica tasks
    pub replica_task_map: RwLock<ReplicaTaskMap<TYPES, V>>,

    /// Map of pre-commit vote accumulates for the relay
    pub pre_commit_relay_map: RwLock<
        RelayMap<TYPES, ViewSyncPreCommitVote2<TYPES>, ViewSyncPreCommitCertificate2<TYPES>, V>,
    >,

    /// Map of commit vote accumulates for the relay
    pub commit_relay_map:
        RwLock<RelayMap<TYPES, ViewSyncCommitVote2<TYPES>, ViewSyncCommitCertificate2<TYPES>, V>>,

    /// Map of finalize vote accumulates for the relay
    pub finalize_relay_map: RwLock<
        RelayMap<TYPES, ViewSyncFinalizeVote2<TYPES>, ViewSyncFinalizeCertificate2<TYPES>, V>,
    >,

    /// Timeout duration for view sync rounds
    pub view_sync_timeout: Duration,

    /// Last view we garbage collected old tasks
    pub last_garbage_collected_view: TYPES::View,

    /// Lock for a decided upgrade
    pub upgrade_lock: UpgradeLock<TYPES, V>,

    /// First view in which epoch version takes effect
    pub first_epoch: Option<(TYPES::View, TYPES::Epoch)>,

    /// Keeps track of the highest finalized view and epoch, used for garbage collection
    pub highest_finalized_epoch_view: (Option<TYPES::Epoch>, TYPES::View),

    pub epoch_height: u64,
}

#[async_trait]
impl<TYPES: NodeType, V: Versions> TaskState for ViewSyncTaskState<TYPES, V> {
    type Event = HotShotEvent<TYPES>;

    async fn handle_event(
        &mut self,
        event: Arc<Self::Event>,
        sender: &Sender<Arc<Self::Event>>,
        _receiver: &Receiver<Arc<Self::Event>>,
    ) -> Result<()> {
        self.handle(event, sender.clone()).await
    }

    fn cancel_subtasks(&mut self) {}
}

/// State of a view sync replica task
pub struct ViewSyncReplicaTaskState<TYPES: NodeType, V: Versions> {
    /// Timeout for view sync rounds
    pub view_sync_timeout: Duration,

    /// Current round HotShot is in
    pub cur_view: TYPES::View,

    /// Round HotShot wishes to be in
    pub next_view: TYPES::View,

    /// The relay index we are currently on
    pub relay: u64,

    /// Whether we have seen a finalized certificate
    pub finalized: bool,

    /// Whether we have already sent a view change event for `next_view`
    pub sent_view_change_event: bool,

    /// Timeout task handle, when it expires we try the next relay
    pub timeout_task: Option<JoinHandle<()>>,

    /// Our node id; for logging
    pub id: u64,

    /// Membership for the quorum
    pub membership_coordinator: EpochMembershipCoordinator<TYPES>,

    /// This Nodes Public Key
    pub public_key: TYPES::SignatureKey,

    /// Our Private Key
    pub private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,

    /// Lock for a decided upgrade
    pub upgrade_lock: UpgradeLock<TYPES, V>,

    /// Epoch HotShot was in when this task was created
    pub cur_epoch: Option<TYPES::Epoch>,

    /// First view in which epoch version takes effect
    pub first_epoch: Option<(TYPES::View, TYPES::Epoch)>,
}

#[async_trait]
impl<TYPES: NodeType, V: Versions> TaskState for ViewSyncReplicaTaskState<TYPES, V> {
    type Event = HotShotEvent<TYPES>;

    async fn handle_event(
        &mut self,
        event: Arc<Self::Event>,
        sender: &Sender<Arc<Self::Event>>,
        _receiver: &Receiver<Arc<Self::Event>>,
    ) -> Result<()> {
        self.handle(event, sender.clone()).await;

        Ok(())
    }

    fn cancel_subtasks(&mut self) {}
}

impl<TYPES: NodeType, V: Versions> ViewSyncTaskState<TYPES, V> {
    #[instrument(skip_all, fields(id = self.id, view = *self.cur_view), name = "View Sync Main Task", level = "error")]
    #[allow(clippy::type_complexity)]
    /// Handles incoming events for the main view sync task
    pub async fn send_to_or_create_replica(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        view: TYPES::View,
        epoch: Option<TYPES::Epoch>,
        sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    ) {
        let mut task_map = self.replica_task_map.write().await;

        if let Some(replica_task) = task_map.get_mut(&epoch).and_then(|x| x.get_mut(&view)) {
            // Forward event then return
            tracing::debug!("Forwarding message");
            let result = replica_task
                .handle(Arc::clone(&event), sender.clone())
                .await;

            if result == Some(HotShotTaskCompleted) {
                // The protocol has finished
                if epoch >= self.highest_finalized_epoch_view.0
                    && view > self.highest_finalized_epoch_view.1
                {
                    self.highest_finalized_epoch_view = (epoch, view);
                } else if view > self.highest_finalized_epoch_view.1 {
                    tracing::error!(
                        "We finalized a higher view but the epoch is lower. This should never \
                         happen. Current highest finalized epoch view: {:?}, new highest \
                         finalized epoch view: {:?}",
                        self.highest_finalized_epoch_view,
                        (epoch, view)
                    );
                }
                task_map.get_mut(&epoch).and_then(|x| x.remove(&view));
                task_map.retain(|_, x| !x.is_empty());
                drop(task_map);

                // Garbage collect old tasks
                self.garbage_collect_tasks().await;
                return;
            }

            return;
        }

        // We do not have a replica task already running, so start one
        let mut replica_state: ViewSyncReplicaTaskState<TYPES, V> = ViewSyncReplicaTaskState {
            cur_view: view,
            next_view: view,
            relay: 0,
            finalized: false,
            sent_view_change_event: false,
            timeout_task: None,
            membership_coordinator: self.membership_coordinator.clone(),
            public_key: self.public_key.clone(),
            private_key: self.private_key.clone(),
            view_sync_timeout: self.view_sync_timeout,
            id: self.id,
            upgrade_lock: self.upgrade_lock.clone(),
            cur_epoch: self.cur_epoch,
            first_epoch: self.first_epoch,
        };

        let result = replica_state
            .handle(Arc::clone(&event), sender.clone())
            .await;

        if result == Some(HotShotTaskCompleted) {
            // The protocol has finished
            return;
        }

        task_map
            .entry(epoch)
            .or_default()
            .insert(view, replica_state);
    }

    #[instrument(skip_all, fields(id = self.id, view = *self.cur_view, epoch = self.cur_epoch.map(|x| *x)), name = "View Sync Main Task", level = "error")]
    #[allow(clippy::type_complexity)]
    /// Handles incoming events for the main view sync task
    pub async fn handle(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        event_stream: Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<()> {
        match event.as_ref() {
            HotShotEvent::ViewSyncPreCommitCertificateRecv(certificate) => {
                tracing::debug!("Received view sync cert for phase {certificate:?}");
                let view = certificate.view_number();
                self.send_to_or_create_replica(
                    Arc::clone(&event),
                    view,
                    certificate.epoch(),
                    &event_stream,
                )
                .await;
            },
            HotShotEvent::ViewSyncCommitCertificateRecv(certificate) => {
                tracing::debug!("Received view sync cert for phase {certificate:?}");
                let view = certificate.view_number();
                self.send_to_or_create_replica(
                    Arc::clone(&event),
                    view,
                    certificate.epoch(),
                    &event_stream,
                )
                .await;
            },
            HotShotEvent::ViewSyncFinalizeCertificateRecv(certificate) => {
                tracing::debug!("Received view sync cert for phase {certificate:?}");
                let view = certificate.view_number();
                self.send_to_or_create_replica(
                    Arc::clone(&event),
                    view,
                    certificate.epoch(),
                    &event_stream,
                )
                .await;
            },
            HotShotEvent::ViewSyncTimeout(view, ..) => {
                tracing::debug!("view sync timeout in main task {view:?}");
                let view = *view;
                self.send_to_or_create_replica(
                    Arc::clone(&event),
                    view,
                    self.cur_epoch,
                    &event_stream,
                )
                .await;
            },

            HotShotEvent::ViewSyncPreCommitVoteRecv(ref vote) => {
                let mut map = self.pre_commit_relay_map.write().await;
                let vote_view = vote.view_number();
                let relay = vote.date().relay;
                let relay_map = map
                    .entry(vote.date().epoch)
                    .or_insert(BTreeMap::new())
                    .entry(vote_view)
                    .or_insert(BTreeMap::new());
                if let Some(relay_task) = relay_map.get_mut(&relay) {
                    tracing::debug!("Forwarding message");

                    // Handle the vote and check if the accumulator has returned successfully
                    if relay_task
                        .handle_vote_event(Arc::clone(&event), &event_stream)
                        .await?
                        .is_some()
                    {
                        map.get_mut(&vote.date().epoch)
                            .and_then(|x| x.remove(&vote_view));
                        map.retain(|_, x| !x.is_empty());
                    }

                    return Ok(());
                }

                let epoch_mem = self
                    .membership_coordinator
                    .membership_for_epoch(vote.date().epoch)
                    .await?;
                // We do not have a relay task already running, so start one
                ensure!(
                    epoch_mem.leader(vote_view + relay).await? == self.public_key,
                    "View sync vote sent to wrong leader"
                );

                let info = AccumulatorInfo {
                    public_key: self.public_key.clone(),
                    membership: epoch_mem,
                    view: vote_view,
                    id: self.id,
                };
                let vote_collector = create_vote_accumulator(
                    &info,
                    event,
                    &event_stream,
                    self.upgrade_lock.clone(),
                    EpochTransitionIndicator::NotInTransition,
                )
                .await?;

                relay_map.insert(relay, vote_collector);
            },

            HotShotEvent::ViewSyncCommitVoteRecv(ref vote) => {
                let mut map = self.commit_relay_map.write().await;
                let vote_view = vote.view_number();
                let relay = vote.date().relay;
                let relay_map = map
                    .entry(vote.date().epoch)
                    .or_insert(BTreeMap::new())
                    .entry(vote_view)
                    .or_insert(BTreeMap::new());
                if let Some(relay_task) = relay_map.get_mut(&relay) {
                    tracing::debug!("Forwarding message");

                    // Handle the vote and check if the accumulator has returned successfully
                    if relay_task
                        .handle_vote_event(Arc::clone(&event), &event_stream)
                        .await?
                        .is_some()
                    {
                        map.get_mut(&vote.date().epoch)
                            .and_then(|x| x.remove(&vote_view));
                        map.retain(|_, x| !x.is_empty());
                    }

                    return Ok(());
                }

                // We do not have a relay task already running, so start one
                let epoch_mem = self
                    .membership_coordinator
                    .membership_for_epoch(vote.date().epoch)
                    .await?;
                ensure!(
                    epoch_mem.leader(vote_view + relay).await? == self.public_key,
                    debug!("View sync vote sent to wrong leader")
                );

                let info = AccumulatorInfo {
                    public_key: self.public_key.clone(),
                    membership: epoch_mem,
                    view: vote_view,
                    id: self.id,
                };

                let vote_collector = create_vote_accumulator(
                    &info,
                    event,
                    &event_stream,
                    self.upgrade_lock.clone(),
                    EpochTransitionIndicator::NotInTransition,
                )
                .await?;
                relay_map.insert(relay, vote_collector);
            },

            HotShotEvent::ViewSyncFinalizeVoteRecv(vote) => {
                let mut map = self.finalize_relay_map.write().await;
                let vote_view = vote.view_number();
                let relay = vote.date().relay;
                let relay_map = map
                    .entry(vote.date().epoch)
                    .or_insert(BTreeMap::new())
                    .entry(vote_view)
                    .or_insert(BTreeMap::new());
                if let Some(relay_task) = relay_map.get_mut(&relay) {
                    tracing::debug!("Forwarding message");

                    // Handle the vote and check if the accumulator has returned successfully
                    if relay_task
                        .handle_vote_event(Arc::clone(&event), &event_stream)
                        .await?
                        .is_some()
                    {
                        map.get_mut(&vote.date().epoch)
                            .and_then(|x| x.remove(&vote_view));
                        map.retain(|_, x| !x.is_empty());
                    }

                    return Ok(());
                }

                let epoch_mem = self
                    .membership_coordinator
                    .membership_for_epoch(vote.date().epoch)
                    .await?;
                // We do not have a relay task already running, so start one
                ensure!(
                    epoch_mem.leader(vote_view + relay).await? == self.public_key,
                    debug!("View sync vote sent to wrong leader")
                );

                let info = AccumulatorInfo {
                    public_key: self.public_key.clone(),
                    membership: epoch_mem,
                    view: vote_view,
                    id: self.id,
                };
                let vote_collector = create_vote_accumulator(
                    &info,
                    event,
                    &event_stream,
                    self.upgrade_lock.clone(),
                    EpochTransitionIndicator::NotInTransition,
                )
                .await;
                if let Ok(vote_task) = vote_collector {
                    relay_map.insert(relay, vote_task);
                }
            },

            &HotShotEvent::ViewChange(new_view, epoch) => {
                if epoch > self.cur_epoch {
                    self.cur_epoch = epoch;
                }
                let new_view = TYPES::View::new(*new_view);
                if self.cur_view < new_view {
                    tracing::debug!(
                        "Change from view {} to view {} in view sync task",
                        *self.cur_view,
                        *new_view
                    );

                    self.cur_view = new_view;
                    self.next_view = self.cur_view;
                    self.num_timeouts_tracked = 0;
                }

                self.garbage_collect_tasks().await;
            },
            HotShotEvent::LeavesDecided(leaves) => {
                let finalized_epoch = self.highest_finalized_epoch_view.0.max(
                    leaves
                        .iter()
                        .map(|leaf| leaf.epoch(self.epoch_height))
                        .max()
                        .unwrap_or(None),
                );
                let finalized_view = self.highest_finalized_epoch_view.1.max(
                    leaves
                        .iter()
                        .map(|leaf| leaf.view_number())
                        .max()
                        .unwrap_or(TYPES::View::new(0)),
                );

                self.highest_finalized_epoch_view = (finalized_epoch, finalized_view);

                self.garbage_collect_tasks().await;
            },
            &HotShotEvent::Timeout(view_number, ..) => {
                // This is an old timeout and we can ignore it
                ensure!(
                    view_number >= self.cur_view,
                    debug!("Discarding old timeout vote.")
                );

                self.num_timeouts_tracked += 1;
                let leader = self
                    .membership_coordinator
                    .membership_for_epoch(self.cur_epoch)
                    .await?
                    .leader(view_number)
                    .await?;
                tracing::warn!(
                    %leader,
                    leader_mnemonic = hotshot_types::utils::mnemonic(&leader),
                    view_number = *view_number,
                    num_timeouts_tracked = self.num_timeouts_tracked,
                    "view timed out",
                );

                if self.num_timeouts_tracked >= 3 {
                    tracing::error!("Too many consecutive timeouts!  This shouldn't happen");
                }

                if self.num_timeouts_tracked >= 2 {
                    tracing::error!("Starting view sync protocol for view {}", *view_number + 1);

                    self.send_to_or_create_replica(
                        Arc::new(HotShotEvent::ViewSyncTrigger(view_number + 1)),
                        view_number + 1,
                        self.cur_epoch,
                        &event_stream,
                    )
                    .await;
                } else {
                    // If this is the first timeout we've seen advance to the next view
                    self.cur_view = view_number + 1;
                    tracing::warn!(
                        "Advancing due to timeout to view {} epoch {:?}",
                        *self.cur_view,
                        self.cur_epoch
                    );
                    broadcast_view_change(
                        &event_stream,
                        self.cur_view,
                        self.cur_epoch,
                        self.first_epoch,
                    )
                    .await;
                }
            },
            HotShotEvent::SetFirstEpoch(view, epoch) => {
                self.first_epoch = Some((*view, *epoch));
            },

            _ => {},
        }
        Ok(())
    }

    /// Garbage collect tasks for epochs older than the highest finalized epoch
    /// or older than the previous epoch, whichever is greater.
    /// Garbage collect views older than the highest finalized view including the highest finalized view.
    async fn garbage_collect_tasks(&self) {
        let previous_epoch = self
            .cur_epoch
            .map(|e| e.saturating_sub(1))
            .map(TYPES::Epoch::new);
        let gc_epoch = self.highest_finalized_epoch_view.0.max(previous_epoch);
        Self::garbage_collect_tasks_helper(
            &self.replica_task_map,
            &gc_epoch,
            &self.highest_finalized_epoch_view.1,
        )
        .await;
        Self::garbage_collect_tasks_helper(
            &self.pre_commit_relay_map,
            &gc_epoch,
            &self.highest_finalized_epoch_view.1,
        )
        .await;
        Self::garbage_collect_tasks_helper(
            &self.commit_relay_map,
            &gc_epoch,
            &self.highest_finalized_epoch_view.1,
        )
        .await;
        Self::garbage_collect_tasks_helper(
            &self.finalize_relay_map,
            &gc_epoch,
            &self.highest_finalized_epoch_view.1,
        )
        .await;
    }

    async fn garbage_collect_tasks_helper<VAL>(
        map: &RwLock<TaskMap<TYPES, VAL>>,
        gc_epoch: &Option<TYPES::Epoch>,
        gc_view: &TYPES::View,
    ) {
        let mut task_map = map.write().await;
        task_map.retain(|e, _| e >= gc_epoch);
        if let Some(view_map) = task_map.get_mut(gc_epoch) {
            view_map.retain(|v, _| v > gc_view)
        };
        task_map.retain(|_, view_map| !view_map.is_empty());
    }
}

impl<TYPES: NodeType, V: Versions> ViewSyncReplicaTaskState<TYPES, V> {
    #[instrument(skip_all, fields(id = self.id, view = *self.cur_view, epoch = self.cur_epoch.map(|x| *x)), name = "View Sync Replica Task", level = "error")]
    /// Handle incoming events for the view sync replica task
    pub async fn handle(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        event_stream: Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Option<HotShotTaskCompleted> {
        match event.as_ref() {
            HotShotEvent::ViewSyncPreCommitCertificateRecv(certificate) => {
                let last_seen_certificate = ViewSyncPhase::PreCommit;

                // Ignore certificate if it is for an older round
                if certificate.view_number() < self.next_view {
                    tracing::warn!("We're already in a higher round");

                    return None;
                }

                let membership = self.membership_for_epoch(certificate.epoch()).await?;
                let membership_stake_table = membership.stake_table().await;
                let membership_failure_threshold = membership.failure_threshold().await;

                // If certificate is not valid, return current state
                if let Err(e) = certificate
                    .is_valid_cert(
                        &StakeTableEntries::<TYPES>::from(membership_stake_table).0,
                        membership_failure_threshold,
                        &self.upgrade_lock,
                    )
                    .await
                {
                    tracing::error!(
                        "Not valid view sync cert! data: {:?}, error: {}",
                        certificate.data(),
                        e
                    );

                    return None;
                }

                // If certificate is for a higher round shutdown this task
                // since another task should have been started for the higher round
                if certificate.view_number() > self.next_view {
                    return Some(HotShotTaskCompleted);
                }

                if certificate.data().relay > self.relay {
                    self.relay = certificate.data().relay;
                }

                let Ok(vote) = ViewSyncCommitVote2::<TYPES>::create_signed_vote(
                    ViewSyncCommitData2 {
                        relay: certificate.data().relay,
                        round: self.next_view,
                        epoch: certificate.data().epoch,
                    },
                    self.next_view,
                    &self.public_key,
                    &self.private_key,
                    &self.upgrade_lock,
                )
                .await
                else {
                    tracing::error!("Failed to sign ViewSyncCommitData!");
                    return None;
                };

                broadcast_event(
                    Arc::new(HotShotEvent::ViewSyncCommitVoteSend(vote)),
                    &event_stream,
                )
                .await;

                if let Some(timeout_task) = self.timeout_task.take() {
                    timeout_task.abort();
                }

                self.timeout_task = Some(spawn({
                    let stream = event_stream.clone();
                    let phase = last_seen_certificate;
                    let relay = self.relay;
                    let next_view = self.next_view;
                    let timeout = self.view_sync_timeout;
                    async move {
                        sleep(timeout).await;
                        tracing::warn!(
                            "Vote sending timed out in ViewSyncPreCommitCertificateRecv, Relay = \
                             {relay}"
                        );

                        broadcast_event(
                            Arc::new(HotShotEvent::ViewSyncTimeout(
                                TYPES::View::new(*next_view),
                                relay,
                                phase,
                            )),
                            &stream,
                        )
                        .await;
                    }
                }));
            },

            HotShotEvent::ViewSyncCommitCertificateRecv(certificate) => {
                let last_seen_certificate = ViewSyncPhase::Commit;

                // Ignore certificate if it is for an older round
                if certificate.view_number() < self.next_view {
                    tracing::warn!("We're already in a higher round");

                    return None;
                }

                let membership = self.membership_for_epoch(certificate.epoch()).await?;
                let membership_stake_table = membership.stake_table().await;
                let membership_success_threshold = membership.success_threshold().await;

                // If certificate is not valid, return current state
                if let Err(e) = certificate
                    .is_valid_cert(
                        &StakeTableEntries::<TYPES>::from(membership_stake_table).0,
                        membership_success_threshold,
                        &self.upgrade_lock,
                    )
                    .await
                {
                    tracing::error!(
                        "Not valid view sync cert! data: {:?}, error: {}",
                        certificate.data(),
                        e
                    );

                    return None;
                }

                // If certificate is for a higher round shutdown this task
                // since another task should have been started for the higher round
                if certificate.view_number() > self.next_view {
                    return Some(HotShotTaskCompleted);
                }

                if certificate.data().relay > self.relay {
                    self.relay = certificate.data().relay;
                }

                let Ok(vote) = ViewSyncFinalizeVote2::<TYPES>::create_signed_vote(
                    ViewSyncFinalizeData2 {
                        relay: certificate.data().relay,
                        round: self.next_view,
                        epoch: certificate.data().epoch,
                    },
                    self.next_view,
                    &self.public_key,
                    &self.private_key,
                    &self.upgrade_lock,
                )
                .await
                else {
                    tracing::error!("Failed to sign view sync finalized vote!");
                    return None;
                };

                broadcast_event(
                    Arc::new(HotShotEvent::ViewSyncFinalizeVoteSend(vote)),
                    &event_stream,
                )
                .await;

                tracing::warn!(
                    "View sync protocol has received view sync evidence to update the view to {}",
                    *self.next_view
                );

                broadcast_view_change(
                    &event_stream,
                    self.next_view,
                    certificate.epoch(),
                    self.first_epoch,
                )
                .await;

                if let Some(timeout_task) = self.timeout_task.take() {
                    timeout_task.abort();
                }
                self.timeout_task = Some(spawn({
                    let stream = event_stream.clone();
                    let phase = last_seen_certificate;
                    let relay = self.relay;
                    let next_view = self.next_view;
                    let timeout = self.view_sync_timeout;
                    async move {
                        sleep(timeout).await;
                        tracing::warn!(
                            "Vote sending timed out in ViewSyncCommitCertificateRecv, relay = \
                             {relay}"
                        );
                        broadcast_event(
                            Arc::new(HotShotEvent::ViewSyncTimeout(
                                TYPES::View::new(*next_view),
                                relay,
                                phase,
                            )),
                            &stream,
                        )
                        .await;
                    }
                }));
            },

            HotShotEvent::ViewSyncFinalizeCertificateRecv(certificate) => {
                // Ignore certificate if it is for an older round
                if certificate.view_number() < self.next_view {
                    tracing::warn!("We're already in a higher round");

                    return None;
                }

                let membership = self.membership_for_epoch(certificate.epoch()).await?;
                let membership_stake_table = membership.stake_table().await;
                let membership_success_threshold = membership.success_threshold().await;

                // If certificate is not valid, return current state
                if let Err(e) = certificate
                    .is_valid_cert(
                        &StakeTableEntries::<TYPES>::from(membership_stake_table).0,
                        membership_success_threshold,
                        &self.upgrade_lock,
                    )
                    .await
                {
                    tracing::error!(
                        "Not valid view sync cert! data: {:?}, error: {}",
                        certificate.data(),
                        e
                    );

                    return None;
                }

                // If certificate is for a higher round shutdown this task
                // since another task should have been started for the higher round
                if certificate.view_number() > self.next_view {
                    return Some(HotShotTaskCompleted);
                }

                if certificate.data().relay > self.relay {
                    self.relay = certificate.data().relay;
                }

                if let Some(timeout_task) = self.timeout_task.take() {
                    timeout_task.abort();
                }

                tracing::warn!(
                    "viewsyncfinalizecertificaterecv: View sync protocol has received view sync \
                     evidence to update the view to {}, epoch {:?}",
                    *self.next_view,
                    certificate.epoch()
                );
                broadcast_view_change(
                    &event_stream,
                    self.next_view,
                    certificate.epoch(),
                    self.first_epoch,
                )
                .await;
                return Some(HotShotTaskCompleted);
            },

            HotShotEvent::ViewSyncTrigger(view_number) => {
                let view_number = *view_number;
                if self.next_view != TYPES::View::new(*view_number) {
                    tracing::error!("Unexpected view number to trigger view sync");
                    return None;
                }

                let Ok(vote) = ViewSyncPreCommitVote2::<TYPES>::create_signed_vote(
                    ViewSyncPreCommitData2 {
                        relay: 0,
                        round: view_number,
                        epoch: self.cur_epoch,
                    },
                    view_number,
                    &self.public_key,
                    &self.private_key,
                    &self.upgrade_lock,
                )
                .await
                else {
                    tracing::error!("Failed to sign pre commit vote!");
                    return None;
                };

                broadcast_event(
                    Arc::new(HotShotEvent::ViewSyncPreCommitVoteSend(vote)),
                    &event_stream,
                )
                .await;

                self.timeout_task = Some(spawn({
                    let stream = event_stream.clone();
                    let relay = self.relay;
                    let next_view = self.next_view;
                    let timeout = self.view_sync_timeout;
                    async move {
                        sleep(timeout).await;
                        tracing::warn!("Vote sending timed out in ViewSyncTrigger");
                        broadcast_event(
                            Arc::new(HotShotEvent::ViewSyncTimeout(
                                TYPES::View::new(*next_view),
                                relay,
                                ViewSyncPhase::None,
                            )),
                            &stream,
                        )
                        .await;
                    }
                }));

                return None;
            },

            HotShotEvent::ViewSyncTimeout(round, relay, last_seen_certificate) => {
                let round = *round;
                // Shouldn't ever receive a timeout for a relay higher than ours
                if TYPES::View::new(*round) == self.next_view && *relay == self.relay {
                    if let Some(timeout_task) = self.timeout_task.take() {
                        timeout_task.abort();
                    }
                    self.relay += 1;
                    match last_seen_certificate {
                        ViewSyncPhase::None | ViewSyncPhase::PreCommit | ViewSyncPhase::Commit => {
                            let Ok(vote) = ViewSyncPreCommitVote2::<TYPES>::create_signed_vote(
                                ViewSyncPreCommitData2 {
                                    relay: self.relay,
                                    round: self.next_view,
                                    epoch: self.cur_epoch,
                                },
                                self.next_view,
                                &self.public_key,
                                &self.private_key,
                                &self.upgrade_lock,
                            )
                            .await
                            else {
                                tracing::error!("Failed to sign ViewSyncPreCommitData!");
                                return None;
                            };

                            broadcast_event(
                                Arc::new(HotShotEvent::ViewSyncPreCommitVoteSend(vote)),
                                &event_stream,
                            )
                            .await;
                        },
                        ViewSyncPhase::Finalize => {
                            // This should never occur
                            unimplemented!()
                        },
                    }

                    self.timeout_task = Some(spawn({
                        let stream = event_stream.clone();
                        let relay = self.relay;
                        let next_view = self.next_view;
                        let timeout = self.view_sync_timeout;
                        let last_cert = last_seen_certificate.clone();
                        async move {
                            sleep(timeout).await;
                            tracing::warn!(
                                "Vote sending timed out in ViewSyncTimeout relay = {relay}"
                            );
                            broadcast_event(
                                Arc::new(HotShotEvent::ViewSyncTimeout(
                                    TYPES::View::new(*next_view),
                                    relay,
                                    last_cert,
                                )),
                                &stream,
                            )
                            .await;
                        }
                    }));

                    return None;
                }
            },
            _ => return None,
        }
        None
    }

    pub async fn membership_for_epoch(
        &self,
        epoch: Option<TYPES::Epoch>,
    ) -> Option<EpochMembership<TYPES>> {
        match self
            .membership_coordinator
            .membership_for_epoch(epoch)
            .await
        {
            Ok(m) => Some(m),
            Err(e) => {
                tracing::warn!(e.message);
                None
            },
        }
    }
}
