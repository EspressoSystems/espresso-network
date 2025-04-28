// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{marker::PhantomData, sync::Arc, time::SystemTime};

use async_broadcast::{Receiver, Sender};
use async_trait::async_trait;
use committable::Committable;
use hotshot_task::task::TaskState;
use hotshot_types::{
    consensus::OuterConsensus,
    data::UpgradeProposal,
    epoch_membership::EpochMembershipCoordinator,
    event::{Event, EventType},
    message::{Proposal, UpgradeLock},
    simple_certificate::UpgradeCertificate,
    simple_vote::{UpgradeProposalData, UpgradeVote},
    traits::{
        block_contents::BlockHeader,
        node_implementation::{ConsensusTime, NodeType, Versions},
        signature_key::SignatureKey,
    },
    utils::{epoch_from_block_number, EpochTransitionIndicator},
    vote::HasViewNumber,
};
use hotshot_utils::anytrace::*;
use tracing::instrument;
use vbs::version::StaticVersionType;

use crate::{
    events::HotShotEvent,
    helpers::broadcast_event,
    vote_collection::{handle_vote, VoteCollectorsMap},
};

/// Tracks state of an upgrade task
pub struct UpgradeTaskState<TYPES: NodeType, V: Versions> {
    /// Output events to application
    pub output_event_stream: async_broadcast::Sender<Event<TYPES>>,

    /// View number this view is executing in.
    pub cur_view: TYPES::View,

    /// Epoch number this node is executing in.
    pub cur_epoch: Option<TYPES::Epoch>,

    /// Membership for Quorum Certs/votes
    pub membership_coordinator: EpochMembershipCoordinator<TYPES>,

    /// A map of `UpgradeVote` collector tasks
    pub vote_collectors: VoteCollectorsMap<TYPES, UpgradeVote<TYPES>, UpgradeCertificate<TYPES>, V>,

    /// This Nodes public key
    pub public_key: TYPES::SignatureKey,

    /// This Nodes private key
    pub private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,

    /// This state's ID
    pub id: u64,

    /// Target block for the epoch upgrade
    pub epoch_start_block: u64,

    /// View to start proposing an upgrade
    pub start_proposing_view: u64,

    /// View to stop proposing an upgrade
    pub stop_proposing_view: u64,

    /// View to start voting on an upgrade
    pub start_voting_view: u64,

    /// View to stop voting on an upgrade
    pub stop_voting_view: u64,

    /// Unix time in seconds at which we start proposing an upgrade
    pub start_proposing_time: u64,

    /// Unix time in seconds at which we stop proposing an upgrade
    pub stop_proposing_time: u64,

    /// Unix time in seconds at which we start voting on an upgrade
    pub start_voting_time: u64,

    /// Unix time in seconds at which we stop voting on an upgrade
    pub stop_voting_time: u64,

    /// Lock for a decided upgrade
    pub upgrade_lock: UpgradeLock<TYPES, V>,

    /// Reference to consensus. The replica will require a write lock on this.
    pub consensus: OuterConsensus<TYPES>,

    /// Number of blocks in an epoch, zero means there are no epochs
    pub epoch_height: u64,
}

impl<TYPES: NodeType, V: Versions> UpgradeTaskState<TYPES, V> {
    /// Check if we have decided on an upgrade certificate
    async fn upgraded(&self) -> bool {
        self.upgrade_lock
            .decided_upgrade_certificate
            .read()
            .await
            .is_some()
    }

    /// main task event handler
    #[instrument(skip_all, fields(id = self.id, view = *self.cur_view, epoch = self.cur_epoch.map(|x| *x)), name = "Upgrade Task", level = "error")]
    pub async fn handle(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        tx: Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<()> {
        match event.as_ref() {
            HotShotEvent::UpgradeProposalRecv(proposal, sender) => {
                tracing::info!("Received upgrade proposal: {:?}", proposal);

                let view = *proposal.data.view_number();

                // Skip voting if the version has already been upgraded.
                ensure!(
                    !self.upgraded().await,
                    info!("Already upgraded to {:?}; not voting.", V::Upgrade::VERSION)
                );

                let time = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .wrap()
                    .context(error!(
                        "Failed to calculate duration. This should never happen."
                    ))?
                    .as_secs();

                ensure!(
                    time >= self.start_voting_time && time < self.stop_voting_time,
                    "Refusing to vote because we are no longer in the configured vote time window."
                );

                ensure!(
                    view >= self.start_voting_view && view < self.stop_voting_view,
                    "Refusing to vote because we are no longer in the configured vote view window."
                );

                // If the proposal does not match our upgrade target, we immediately exit.
                ensure!(
                    proposal.data.upgrade_proposal.new_version_hash == V::UPGRADE_HASH
                        && proposal.data.upgrade_proposal.old_version == V::Base::VERSION
                        && proposal.data.upgrade_proposal.new_version == V::Upgrade::VERSION,
                    "Proposal does not match our upgrade target"
                );

                // If we have an upgrade target, we validate that the proposal is relevant for the current view.
                tracing::info!(
                    "Upgrade proposal received for view: {:?}",
                    proposal.data.view_number()
                );

                let epoch_upgrade_checks = if V::Upgrade::VERSION == V::Epochs::VERSION {
                    let consensus_reader = self.consensus.read().await;

                    let Some((_, last_proposal)) =
                        consensus_reader.last_proposals().last_key_value()
                    else {
                        tracing::error!("No recent quorum proposals in consensus state -- skipping upgrade proposal vote.");
                        return Err(error!("No recent quorum proposals in consensus state -- skipping upgrade proposal vote."));
                    };

                    let last_proposal_view: u64 = *last_proposal.data.view_number();
                    let last_proposal_block: u64 = last_proposal.data.block_header().block_number();

                    drop(consensus_reader);

                    let target_start_epoch =
                        epoch_from_block_number(self.epoch_start_block, self.epoch_height);
                    let last_proposal_epoch =
                        epoch_from_block_number(last_proposal_block, self.epoch_height);
                    let upgrade_finish_epoch = epoch_from_block_number(
                        last_proposal_block
                            + (*proposal.data.upgrade_proposal.new_version_first_view
                                - last_proposal_view)
                            + 10,
                        self.epoch_height,
                    );

                    target_start_epoch == last_proposal_epoch
                        && last_proposal_epoch == upgrade_finish_epoch
                } else {
                    true
                };

                ensure!(
                    epoch_upgrade_checks,
                    error!("Epoch upgrade safety check failed! Refusing to vote on upgrade.")
                );

                let view = proposal.data.view_number();

                // At this point, we could choose to validate
                // that the proposal was issued by the correct leader
                // for the indicated view.
                //
                // We choose not to, because we don't gain that much from it.
                // The certificate itself is only useful to the leader for that view anyway,
                // and from the node's perspective it doesn't matter who the sender is.
                // All we'd save is the cost of signing the vote, and we'd lose some flexibility.

                // Allow an upgrade proposal that is one view older, in case we have voted on a quorum
                // proposal and updated the view.
                // `self.cur_view` should be at least 1 since there is a view change before getting
                // the `UpgradeProposalRecv` event. Otherwise, the view number subtraction below will
                // cause an overflow error.
                // TODO Come back to this - we probably don't need this, but we should also never receive a UpgradeCertificate where this fails, investigate block ready so it doesn't make one for the genesis block
                ensure!(
                    self.cur_view != TYPES::View::genesis() && *view >= self.cur_view.saturating_sub(1),
                    warn!(
                      "Discarding old upgrade proposal; the proposal is for view {:?}, but the current view is {:?}.",
                      view,
                      self.cur_view
                    )
                );

                // We then validate that the proposal was issued by the leader for the view.
                let view_leader_key = self
                    .membership_coordinator
                    .membership_for_epoch(self.cur_epoch)
                    .await?
                    .leader(view)
                    .await?;
                ensure!(
                    view_leader_key == *sender,
                    info!(
                        "Upgrade proposal doesn't have expected leader key for view {} \n Upgrade proposal is: {:?}", *view, proposal.data.clone()
                    )
                );

                // At this point, we've checked that:
                //   * the proposal was expected,
                //   * the proposal is valid, and
                // so we notify the application layer
                broadcast_event(
                    Event {
                        view_number: self.cur_view,
                        event: EventType::UpgradeProposal {
                            proposal: proposal.clone(),
                            sender: sender.clone(),
                        },
                    },
                    &self.output_event_stream,
                )
                .await;

                // If everything is fine up to here, we generate and send a vote on the proposal.
                let vote = UpgradeVote::create_signed_vote(
                    proposal.data.upgrade_proposal.clone(),
                    view,
                    &self.public_key,
                    &self.private_key,
                    &self.upgrade_lock,
                )
                .await?;

                tracing::debug!("Sending upgrade vote {:?}", vote.view_number());
                broadcast_event(Arc::new(HotShotEvent::UpgradeVoteSend(vote)), &tx).await;
            },
            HotShotEvent::UpgradeVoteRecv(ref vote) => {
                tracing::debug!("Upgrade vote recv, Main Task {:?}", vote.view_number());

                // Check if we are the leader.
                let view = vote.view_number();
                let epoch_membership = self
                    .membership_coordinator
                    .membership_for_epoch(self.cur_epoch)
                    .await?;
                ensure!(
                    epoch_membership.leader(view).await? == self.public_key,
                    debug!(
                        "We are not the leader for view {} are we leader for next view? {}",
                        *view,
                        epoch_membership.leader(view + 1).await? == self.public_key
                    )
                );

                handle_vote(
                    &mut self.vote_collectors,
                    vote,
                    self.public_key.clone(),
                    &epoch_membership,
                    self.id,
                    &event,
                    &tx,
                    &self.upgrade_lock,
                    EpochTransitionIndicator::NotInTransition,
                )
                .await?;
            },
            HotShotEvent::ViewChange(new_view, epoch_number) => {
                if *epoch_number > self.cur_epoch {
                    self.cur_epoch = *epoch_number;
                }
                ensure!(self.cur_view < *new_view || *self.cur_view == 0);

                self.cur_view = *new_view;

                let view: u64 = *self.cur_view;
                let time = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .wrap()
                    .context(error!(
                        "Failed to calculate duration. This should never happen."
                    ))?
                    .as_secs();

                let leader = self
                    .membership_coordinator
                    .membership_for_epoch(self.cur_epoch)
                    .await?
                    .leader(TYPES::View::new(
                        view + TYPES::UPGRADE_CONSTANTS.propose_offset,
                    ))
                    .await?;

                let old_version_last_view = view + TYPES::UPGRADE_CONSTANTS.begin_offset;
                let new_version_first_view = view + TYPES::UPGRADE_CONSTANTS.finish_offset;
                let decide_by = view + TYPES::UPGRADE_CONSTANTS.decide_by_offset;

                let epoch_upgrade_checks = if V::Upgrade::VERSION == V::Epochs::VERSION {
                    let consensus_reader = self.consensus.read().await;

                    let Some((_, last_proposal)) =
                        consensus_reader.last_proposals().last_key_value()
                    else {
                        tracing::error!("No recent quorum proposals in consensus state -- skipping upgrade proposal.");
                        return Err(error!("No recent quorum proposals in consensus state -- skipping upgrade proposal."));
                    };

                    let last_proposal_view: u64 = *last_proposal.data.view_number();
                    let last_proposal_block: u64 = last_proposal.data.block_header().block_number();

                    drop(consensus_reader);

                    let target_start_epoch =
                        epoch_from_block_number(self.epoch_start_block, self.epoch_height);
                    let last_proposal_epoch =
                        epoch_from_block_number(last_proposal_block, self.epoch_height);
                    let upgrade_finish_epoch = epoch_from_block_number(
                        last_proposal_block + (new_version_first_view - last_proposal_view) + 10,
                        self.epoch_height,
                    );

                    target_start_epoch == last_proposal_epoch
                        && last_proposal_epoch == upgrade_finish_epoch
                } else {
                    true
                };

                // We try to form a certificate 5 views before we're leader.
                if view >= self.start_proposing_view
                    && view < self.stop_proposing_view
                    && time >= self.start_proposing_time
                    && time < self.stop_proposing_time
                    && !self.upgraded().await
                    && epoch_upgrade_checks
                    && leader == self.public_key
                {
                    let upgrade_proposal_data = UpgradeProposalData {
                        old_version: V::Base::VERSION,
                        new_version: V::Upgrade::VERSION,
                        new_version_hash: V::UPGRADE_HASH.to_vec(),
                        old_version_last_view: TYPES::View::new(old_version_last_view),
                        new_version_first_view: TYPES::View::new(new_version_first_view),
                        decide_by: TYPES::View::new(decide_by),
                    };

                    let upgrade_proposal = UpgradeProposal {
                        upgrade_proposal: upgrade_proposal_data.clone(),
                        view_number: TYPES::View::new(
                            view + TYPES::UPGRADE_CONSTANTS.propose_offset,
                        ),
                    };

                    let signature = TYPES::SignatureKey::sign(
                        &self.private_key,
                        upgrade_proposal_data.commit().as_ref(),
                    )
                    .expect("Failed to sign upgrade proposal commitment!");

                    tracing::warn!("Sending upgrade proposal:\n\n {:?}", upgrade_proposal);

                    let message = Proposal {
                        data: upgrade_proposal,
                        signature,
                        _pd: PhantomData,
                    };

                    broadcast_event(
                        Arc::new(HotShotEvent::UpgradeProposalSend(
                            message,
                            self.public_key.clone(),
                        )),
                        &tx,
                    )
                    .await;
                }
            },
            _ => {},
        }
        Ok(())
    }
}

#[async_trait]
/// task state implementation for the upgrade task
impl<TYPES: NodeType, V: Versions> TaskState for UpgradeTaskState<TYPES, V> {
    type Event = HotShotEvent<TYPES>;

    async fn handle_event(
        &mut self,
        event: Arc<Self::Event>,
        sender: &Sender<Arc<Self::Event>>,
        _receiver: &Receiver<Arc<Self::Event>>,
    ) -> Result<()> {
        self.handle(event, sender.clone()).await?;

        Ok(())
    }

    fn cancel_subtasks(&mut self) {}
}
