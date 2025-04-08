// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

#![allow(unused_imports)]

use std::{collections::BTreeMap, sync::Arc};

use async_broadcast::{broadcast, Receiver, Sender};
use async_lock::RwLock;
use async_trait::async_trait;
use either::Either;
use futures::future::{err, join_all};
use hotshot_task::task::{Task, TaskState};
use hotshot_types::{
    consensus::{Consensus, OuterConsensus},
    data::{EpochNumber, Leaf, ViewChangeEvidence2},
    epoch_membership::{self, EpochMembership, EpochMembershipCoordinator},
    event::Event,
    message::UpgradeLock,
    simple_certificate::UpgradeCertificate,
    simple_vote::HasEpoch,
    traits::{
        block_contents::BlockHeader,
        node_implementation::{ConsensusTime, NodeImplementation, NodeType, Versions},
        signature_key::SignatureKey,
    },
    utils::option_epoch_from_block_number,
    vote::{Certificate, HasViewNumber},
};
use hotshot_utils::anytrace::{bail, Result};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, instrument, warn};
use vbs::version::Version;

use self::handlers::handle_quorum_proposal_recv;
use crate::{
    events::{HotShotEvent, ProposalMissing},
    helpers::{broadcast_event, fetch_proposal, parent_leaf_and_state},
};
/// Event handlers for this task.
mod handlers;

/// The state for the quorum proposal task. Contains all of the information for
/// handling [`HotShotEvent::QuorumProposalRecv`] events.
pub struct QuorumProposalRecvTaskState<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> {
    /// Our public key
    pub public_key: TYPES::SignatureKey,

    /// Our Private Key
    pub private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,

    /// Reference to consensus. The replica will require a write lock on this.
    pub consensus: OuterConsensus<TYPES>,

    /// View number this view is executing in.
    pub cur_view: TYPES::View,

    /// Epoch number this node is executing in.
    pub cur_epoch: Option<TYPES::Epoch>,

    /// Membership for Quorum Certs/votes
    pub membership: EpochMembershipCoordinator<TYPES>,

    /// View timeout from config.
    pub timeout: u64,

    /// Output events to application
    pub output_event_stream: async_broadcast::Sender<Event<TYPES>>,

    /// This node's storage ref
    pub storage: Arc<RwLock<I::Storage>>,

    /// Spawned tasks related to a specific view, so we can cancel them when
    /// they are stale
    pub spawned_tasks: BTreeMap<TYPES::View, Vec<JoinHandle<()>>>,

    /// The node's id
    pub id: u64,

    /// Lock for a decided upgrade
    pub upgrade_lock: UpgradeLock<TYPES, V>,

    /// Number of blocks in an epoch, zero means there are no epochs
    pub epoch_height: u64,
}

/// all the info we need to validate a proposal.  This makes it easy to spawn an effemeral task to
/// do all the proposal validation without blocking the long running one
pub(crate) struct ValidationInfo<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> {
    /// The node's id
    pub id: u64,

    /// Our public key
    pub(crate) public_key: TYPES::SignatureKey,

    /// Our Private Key
    pub(crate) private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,

    /// Reference to consensus. The replica will require a write lock on this.
    pub(crate) consensus: OuterConsensus<TYPES>,

    /// Membership for Quorum Certs/votes
    pub membership: EpochMembership<TYPES>,

    /// Output events to application
    pub output_event_stream: async_broadcast::Sender<Event<TYPES>>,

    /// This node's storage ref
    pub(crate) storage: Arc<RwLock<I::Storage>>,

    /// Lock for a decided upgrade
    pub(crate) upgrade_lock: UpgradeLock<TYPES, V>,

    /// Number of blocks in an epoch, zero means there are no epochs
    pub epoch_height: u64,
}

impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>
    QuorumProposalRecvTaskState<TYPES, I, V>
{
    /// Cancel all tasks the consensus tasks has spawned before the given view
    pub fn cancel_tasks(&mut self, view: TYPES::View) {
        let keep = self.spawned_tasks.split_off(&view);
        while let Some((_, tasks)) = self.spawned_tasks.pop_first() {
            for task in tasks {
                task.abort();
            }
        }
        self.spawned_tasks = keep;
    }

    /// Handles all consensus events relating to propose and vote-enabling events.
    #[instrument(skip_all, fields(id = self.id, view = *self.cur_view, epoch = self.cur_epoch.map(|x| *x)), name = "Consensus replica task", level = "error")]
    #[allow(unused_variables)]
    pub async fn handle(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        event_sender: Sender<Arc<HotShotEvent<TYPES>>>,
        event_receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
    ) {
        match event.as_ref() {
            HotShotEvent::QuorumProposalRecv(proposal, sender) => {
                tracing::debug!(
                    "Quorum proposal recv for view {:?}",
                    proposal.data.view_number()
                );
                if self.consensus.read().await.cur_view() > proposal.data.view_number()
                    || self.cur_view > proposal.data.view_number()
                {
                    tracing::warn!(
                        "Throwing away old proposal for view {:?}",
                        proposal.data.view_number()
                    );
                    return;
                }
                let proposal_epoch = option_epoch_from_block_number::<TYPES>(
                    proposal.data.proposal.epoch().is_some(),
                    proposal.data.block_header().block_number(),
                    self.epoch_height,
                );
                let Ok(epoch_membership) =
                    self.membership.membership_for_epoch(proposal_epoch).await
                else {
                    tracing::warn!("No Stake table for epoch = {proposal_epoch:?}");
                    return;
                };
                let validation_info = ValidationInfo::<TYPES, I, V> {
                    id: self.id,
                    public_key: self.public_key.clone(),
                    private_key: self.private_key.clone(),
                    consensus: self.consensus.clone(),
                    membership: epoch_membership,
                    output_event_stream: self.output_event_stream.clone(),
                    storage: Arc::clone(&self.storage),
                    upgrade_lock: self.upgrade_lock.clone(),
                    epoch_height: self.epoch_height,
                };
                match handle_quorum_proposal_recv(
                    proposal,
                    sender,
                    &event_sender,
                    &event_receiver,
                    validation_info,
                )
                .await
                {
                    Ok(()) => {},
                    Err(e) => tracing::error!(?e, "Failed to validate the proposal"),
                }
            },
            HotShotEvent::ViewChange(view, epoch) => {
                if *epoch > self.cur_epoch {
                    self.cur_epoch = *epoch;
                }
                if self.cur_view >= *view {
                    return;
                }
                self.cur_view = *view;
                // cancel task for any view 2 views prior or more.  The view here is the oldest
                // view we want to KEEP tasks for.  We keep the view prior to this because
                // we might still be processing the proposal from view V which caused us
                // to enter view V + 1.
                let oldest_view_to_keep = TYPES::View::new(view.saturating_sub(1));
                self.cancel_tasks(oldest_view_to_keep);
            },
            _ => {},
        }
    }
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> TaskState
    for QuorumProposalRecvTaskState<TYPES, I, V>
{
    type Event = HotShotEvent<TYPES>;

    async fn handle_event(
        &mut self,
        event: Arc<Self::Event>,
        sender: &Sender<Arc<Self::Event>>,
        receiver: &Receiver<Arc<Self::Event>>,
    ) -> Result<()> {
        self.handle(event, sender.clone(), receiver.clone()).await;

        Ok(())
    }

    fn cancel_subtasks(&mut self) {
        while !self.spawned_tasks.is_empty() {
            let Some((_, handles)) = self.spawned_tasks.pop_first() else {
                break;
            };
            for handle in handles {
                handle.abort();
            }
        }
    }
}
