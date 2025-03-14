// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{collections::BTreeMap, sync::Arc};

use async_broadcast::{InactiveReceiver, Receiver, Sender};
use async_lock::RwLock;
use async_trait::async_trait;
use committable::Committable;
use hotshot_task::{
    dependency::{AndDependency, EventDependency},
    dependency_task::{DependencyTask, HandleDepOutput},
    task::TaskState,
};
use hotshot_types::{
    consensus::{ConsensusMetricsValue, OuterConsensus},
    data::{vid_disperse::vid_total_weight, Leaf2, QuorumProposalWrapper},
    drb::DrbComputation,
    epoch_membership::EpochMembershipCoordinator,
    event::Event,
    message::{Proposal, UpgradeLock},
    simple_certificate::UpgradeCertificate,
    simple_vote::HasEpoch,
    traits::{
        block_contents::BlockHeader,
        node_implementation::{ConsensusTime, NodeImplementation, NodeType, Versions},
        signature_key::SignatureKey,
        storage::Storage,
    },
    utils::{epoch_from_block_number, option_epoch_from_block_number},
    vote::{Certificate, HasViewNumber},
    StakeTableEntries,
};
use hotshot_utils::anytrace::*;
use tokio::task::JoinHandle;
use tracing::instrument;
use vbs::version::StaticVersionType;

use crate::{
    events::HotShotEvent,
    helpers::broadcast_event,
    quorum_vote::handlers::{handle_quorum_proposal_validated, submit_vote, update_shared_state},
};

/// Event handlers for `QuorumProposalValidated`.
mod handlers;

/// Vote dependency types.
#[derive(Debug, PartialEq)]
enum VoteDependency {
    /// For the `QuorumProposalValidated` event after validating `QuorumProposalRecv`.
    QuorumProposal,
    /// For the `DaCertificateRecv` event.
    Dac,
    /// For the `VidShareRecv` event.
    Vid,
}

/// Handler for the vote dependency.
pub struct VoteDependencyHandle<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> {
    /// Public key.
    pub public_key: TYPES::SignatureKey,

    /// Private Key.
    pub private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,

    /// Reference to consensus. The replica will require a write lock on this.
    pub consensus: OuterConsensus<TYPES>,

    /// Immutable instance state
    pub instance_state: Arc<TYPES::InstanceState>,

    /// Membership for Quorum certs/votes.
    pub membership_coordinator: EpochMembershipCoordinator<TYPES>,

    /// Reference to the storage.
    pub storage: Arc<RwLock<I::Storage>>,

    /// View number to vote on.
    pub view_number: TYPES::View,

    /// Event sender.
    pub sender: Sender<Arc<HotShotEvent<TYPES>>>,

    /// Event receiver.
    pub receiver: InactiveReceiver<Arc<HotShotEvent<TYPES>>>,

    /// Lock for a decided upgrade
    pub upgrade_lock: UpgradeLock<TYPES, V>,

    /// The consensus metrics
    pub consensus_metrics: Arc<ConsensusMetricsValue>,

    /// The node's id
    pub id: u64,

    /// Number of blocks in an epoch, zero means there are no epochs
    pub epoch_height: u64,
}

impl<TYPES: NodeType, I: NodeImplementation<TYPES> + 'static, V: Versions> HandleDepOutput
    for VoteDependencyHandle<TYPES, I, V>
{
    type Output = Vec<Arc<HotShotEvent<TYPES>>>;

    #[allow(clippy::too_many_lines)]
    #[instrument(skip_all, fields(id = self.id, view = *self.view_number))]
    async fn handle_dep_result(self, res: Self::Output) {
        let mut payload_commitment = None;
        let mut next_epoch_payload_commitment = None;
        let mut leaf = None;
        let mut vid_share = None;
        let mut parent_view_number = None;
        for event in res {
            match event.as_ref() {
                #[allow(unused_assignments)]
                HotShotEvent::QuorumProposalValidated(proposal, parent_leaf) => {
                    let version = match self.upgrade_lock.version(self.view_number).await {
                        Ok(version) => version,
                        Err(e) => {
                            tracing::error!("{e:#}");
                            return;
                        },
                    };
                    let proposal_payload_comm = proposal.data.block_header().payload_commitment();
                    let parent_commitment = parent_leaf.commit();
                    let proposed_leaf = Leaf2::from_quorum_proposal(&proposal.data);

                    if version >= V::Epochs::VERSION
                        && self
                            .consensus
                            .read()
                            .await
                            .is_leaf_forming_eqc(proposal.data.justify_qc().data.leaf_commit)
                    {
                        tracing::debug!("Do not vote here. Voting for this case is handled in QuorumVoteTaskState");
                        return;
                    } else if let Some(ref comm) = payload_commitment {
                        if proposal_payload_comm != *comm {
                            tracing::error!("Quorum proposal has inconsistent payload commitment with DAC or VID.");
                            return;
                        }
                    } else {
                        payload_commitment = Some(proposal_payload_comm);
                    }

                    if proposed_leaf.parent_commitment() != parent_commitment {
                        tracing::warn!("Proposed leaf parent commitment does not match parent leaf payload commitment. Aborting vote.");
                        return;
                    }
                    // Update our persistent storage of the proposal. If we cannot store the proposal return
                    // and error so we don't vote
                    if let Err(e) = self
                        .storage
                        .write()
                        .await
                        .append_proposal_wrapper(proposal)
                        .await
                    {
                        tracing::error!("failed to store proposal, not voting.  error = {e:#}");
                        return;
                    }
                    leaf = Some(proposed_leaf);
                    parent_view_number = Some(parent_leaf.view_number());
                },
                HotShotEvent::DaCertificateValidated(cert) => {
                    let cert_payload_comm = &cert.data().payload_commit;
                    let next_epoch_cert_payload_comm = cert.data().next_epoch_payload_commit;
                    if let Some(ref comm) = payload_commitment {
                        if cert_payload_comm != comm {
                            tracing::error!("DAC has inconsistent payload commitment with quorum proposal or VID.");
                            return;
                        }
                    } else {
                        payload_commitment = Some(*cert_payload_comm);
                    }
                    if next_epoch_payload_commitment.is_some()
                        && next_epoch_payload_commitment != next_epoch_cert_payload_comm
                    {
                        tracing::error!(
                            "DAC has inconsistent next epoch payload commitment with VID."
                        );
                        return;
                    } else {
                        next_epoch_payload_commitment = next_epoch_cert_payload_comm;
                    }
                },
                HotShotEvent::VidShareValidated(share) => {
                    let vid_payload_commitment = &share.data.payload_commitment();
                    vid_share = Some(share.clone());
                    let is_next_epoch_vid = share.data.epoch() != share.data.target_epoch();
                    if is_next_epoch_vid {
                        if let Some(ref comm) = next_epoch_payload_commitment {
                            if vid_payload_commitment != comm {
                                tracing::error!(
                                    "VID has inconsistent next epoch payload commitment with DAC."
                                );
                                return;
                            }
                        } else {
                            next_epoch_payload_commitment = Some(*vid_payload_commitment);
                        }
                    } else if let Some(ref comm) = payload_commitment {
                        if vid_payload_commitment != comm {
                            tracing::error!("VID has inconsistent payload commitment with quorum proposal or DAC.");
                            return;
                        }
                    } else {
                        payload_commitment = Some(*vid_payload_commitment);
                    }
                },
                _ => {},
            }
        }

        let Some(vid_share) = vid_share else {
            tracing::error!(
                "We don't have the VID share for this view {:?}, but we should, because the vote dependencies have completed.",
                self.view_number
            );
            return;
        };

        let Some(leaf) = leaf else {
            tracing::error!(
                "We don't have the leaf for this view {:?}, but we should, because the vote dependencies have completed.",
                self.view_number
            );
            return;
        };

        // Update internal state
        if let Err(e) = update_shared_state::<TYPES, I, V>(
            OuterConsensus::new(Arc::clone(&self.consensus.inner_consensus)),
            self.sender.clone(),
            self.receiver.clone(),
            self.membership_coordinator.clone(),
            self.public_key.clone(),
            self.private_key.clone(),
            self.upgrade_lock.clone(),
            self.view_number,
            Arc::clone(&self.instance_state),
            &leaf,
            &vid_share,
            parent_view_number,
            self.epoch_height,
        )
        .await
        {
            tracing::error!("Failed to update shared consensus state; error = {e:#}");
            return;
        }
        let cur_epoch = option_epoch_from_block_number::<TYPES>(
            leaf.with_epoch,
            leaf.height(),
            self.epoch_height,
        );

        let epoch_membership = match self
            .membership_coordinator
            .membership_for_epoch(cur_epoch)
            .await
        {
            Ok(epoch_membership) => epoch_membership,
            Err(e) => {
                tracing::warn!("{:?}", e);
                return;
            },
        };

        tracing::trace!(
            "Sending ViewChange for view {} and epoch {:?}",
            self.view_number + 1,
            cur_epoch
        );
        broadcast_event(
            Arc::new(HotShotEvent::ViewChange(self.view_number + 1, cur_epoch)),
            &self.sender,
        )
        .await;

        if let Err(e) = submit_vote::<TYPES, I, V>(
            self.sender.clone(),
            epoch_membership,
            self.public_key.clone(),
            self.private_key.clone(),
            self.upgrade_lock.clone(),
            self.view_number,
            Arc::clone(&self.storage),
            leaf,
            vid_share,
            false,
            self.epoch_height,
        )
        .await
        {
            tracing::debug!("Failed to vote; error = {e:#}");
        }
    }
}

/// The state for the quorum vote task.
///
/// Contains all of the information for the quorum vote.
pub struct QuorumVoteTaskState<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> {
    /// Public key.
    pub public_key: TYPES::SignatureKey,

    /// Private Key.
    pub private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,

    /// Reference to consensus. The replica will require a write lock on this.
    pub consensus: OuterConsensus<TYPES>,

    /// Immutable instance state
    pub instance_state: Arc<TYPES::InstanceState>,

    /// Latest view number that has been voted for.
    pub latest_voted_view: TYPES::View,

    /// Table for the in-progress dependency tasks.
    pub vote_dependencies: BTreeMap<TYPES::View, JoinHandle<()>>,

    /// The underlying network
    pub network: Arc<I::Network>,

    /// Membership for Quorum certs/votes and DA committee certs/votes.
    pub membership: EpochMembershipCoordinator<TYPES>,

    /// In-progress DRB computation task.
    pub drb_computation: DrbComputation<TYPES>,

    /// Output events to application
    pub output_event_stream: async_broadcast::Sender<Event<TYPES>>,

    /// The node's id
    pub id: u64,

    /// The consensus metrics
    pub consensus_metrics: Arc<ConsensusMetricsValue>,

    /// Reference to the storage.
    pub storage: Arc<RwLock<I::Storage>>,

    /// Lock for a decided upgrade
    pub upgrade_lock: UpgradeLock<TYPES, V>,

    /// Number of blocks in an epoch, zero means there are no epochs
    pub epoch_height: u64,

    /// Upgrade certificate to enable epochs, staged until we reach the specified block height
    pub staged_epoch_upgrade_certificate: Option<UpgradeCertificate<TYPES>>,

    /// Block height at which to enable the epoch upgrade
    pub epoch_upgrade_block_height: u64,
}

impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> QuorumVoteTaskState<TYPES, I, V> {
    /// Create an event dependency.
    #[instrument(skip_all, fields(id = self.id, latest_voted_view = *self.latest_voted_view), name = "Quorum vote create event dependency", level = "error")]
    fn create_event_dependency(
        &self,
        dependency_type: VoteDependency,
        view_number: TYPES::View,
        event_receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
    ) -> EventDependency<Arc<HotShotEvent<TYPES>>> {
        let id = self.id;
        EventDependency::new(
            event_receiver.clone(),
            Box::new(move |event| {
                let event = event.as_ref();
                let event_view = match dependency_type {
                    VoteDependency::QuorumProposal => {
                        if let HotShotEvent::QuorumProposalValidated(proposal, _) = event {
                            proposal.data.view_number()
                        } else {
                            return false;
                        }
                    },
                    VoteDependency::Dac => {
                        if let HotShotEvent::DaCertificateValidated(cert) = event {
                            cert.view_number
                        } else {
                            return false;
                        }
                    },
                    VoteDependency::Vid => {
                        if let HotShotEvent::VidShareValidated(disperse) = event {
                            disperse.data.view_number()
                        } else {
                            return false;
                        }
                    },
                };
                if event_view == view_number {
                    tracing::trace!(
                        "Vote dependency {:?} completed for view {:?}, my id is {:?}",
                        dependency_type,
                        view_number,
                        id,
                    );
                    return true;
                }
                false
            }),
        )
    }

    /// Create and store an [`AndDependency`] combining [`EventDependency`]s associated with the
    /// given view number if it doesn't exist.
    #[instrument(skip_all, fields(id = self.id, latest_voted_view = *self.latest_voted_view), name = "Quorum vote crete dependency task if new", level = "error")]
    fn create_dependency_task_if_new(
        &mut self,
        view_number: TYPES::View,
        event_receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
        event_sender: &Sender<Arc<HotShotEvent<TYPES>>>,
        event: Arc<HotShotEvent<TYPES>>,
    ) {
        tracing::debug!(
            "Attempting to make dependency task for view {view_number:?} and event {event:?}"
        );

        if self.vote_dependencies.contains_key(&view_number) {
            return;
        }

        let mut quorum_proposal_dependency = self.create_event_dependency(
            VoteDependency::QuorumProposal,
            view_number,
            event_receiver.clone(),
        );
        let dac_dependency =
            self.create_event_dependency(VoteDependency::Dac, view_number, event_receiver.clone());
        let vid_dependency =
            self.create_event_dependency(VoteDependency::Vid, view_number, event_receiver.clone());
        // If we have an event provided to us
        if let HotShotEvent::QuorumProposalValidated(..) = event.as_ref() {
            quorum_proposal_dependency.mark_as_completed(event);
        }

        let deps = vec![quorum_proposal_dependency, dac_dependency, vid_dependency];

        let dependency_chain = AndDependency::from_deps(deps);

        let dependency_task = DependencyTask::new(
            dependency_chain,
            VoteDependencyHandle::<TYPES, I, V> {
                public_key: self.public_key.clone(),
                private_key: self.private_key.clone(),
                consensus: OuterConsensus::new(Arc::clone(&self.consensus.inner_consensus)),
                instance_state: Arc::clone(&self.instance_state),
                membership_coordinator: self.membership.clone(),
                storage: Arc::clone(&self.storage),
                view_number,
                sender: event_sender.clone(),
                receiver: event_receiver.clone().deactivate(),
                upgrade_lock: self.upgrade_lock.clone(),
                id: self.id,
                epoch_height: self.epoch_height,
                consensus_metrics: Arc::clone(&self.consensus_metrics),
            },
        );
        self.vote_dependencies
            .insert(view_number, dependency_task.run());
    }

    /// Update the latest voted view number.
    #[instrument(skip_all, fields(id = self.id, latest_voted_view = *self.latest_voted_view), name = "Quorum vote update latest voted view", level = "error")]
    async fn update_latest_voted_view(&mut self, new_view: TYPES::View) -> bool {
        if *self.latest_voted_view < *new_view {
            tracing::debug!(
                "Updating next vote view from {} to {} in the quorum vote task",
                *self.latest_voted_view,
                *new_view
            );

            // Cancel the old dependency tasks.
            for view in *self.latest_voted_view..(*new_view) {
                if let Some(dependency) = self.vote_dependencies.remove(&TYPES::View::new(view)) {
                    dependency.abort();
                    tracing::debug!("Vote dependency removed for view {:?}", view);
                }
            }

            // Update the metric for the last voted view
            if let Ok(last_voted_view_usize) = usize::try_from(*new_view) {
                self.consensus_metrics
                    .last_voted_view
                    .set(last_voted_view_usize);
            } else {
                tracing::warn!("Failed to convert last voted view to a usize: {}", new_view);
            }

            self.latest_voted_view = new_view;

            return true;
        }
        false
    }

    /// Handle a vote dependent event received on the event stream
    #[instrument(skip_all, fields(id = self.id, latest_voted_view = *self.latest_voted_view), name = "Quorum vote handle", level = "error", target = "QuorumVoteTaskState")]
    pub async fn handle(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        event_receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
        event_sender: Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<()> {
        match event.as_ref() {
            HotShotEvent::QuorumProposalValidated(proposal, parent_leaf) => {
                tracing::trace!(
                    "Received Proposal for view {}",
                    *proposal.data.view_number()
                );

                // Handle the event before creating the dependency task.
                if let Err(e) = handle_quorum_proposal_validated(&proposal.data, self).await {
                    tracing::debug!(
                        "Failed to handle QuorumProposalValidated event; error = {e:#}"
                    );
                }

                ensure!(
                    proposal.data.view_number() > self.latest_voted_view,
                    "We have already voted for this view"
                );

                let version = self
                    .upgrade_lock
                    .version(proposal.data.view_number())
                    .await?;

                let is_justify_qc_forming_eqc = self
                    .consensus
                    .read()
                    .await
                    .is_leaf_forming_eqc(proposal.data.justify_qc().data.leaf_commit);

                if version >= V::Epochs::VERSION && is_justify_qc_forming_eqc {
                    self.handle_eqc_voting(proposal, parent_leaf, event_sender, event_receiver)
                        .await?;
                } else {
                    self.create_dependency_task_if_new(
                        proposal.data.view_number(),
                        event_receiver,
                        &event_sender,
                        Arc::clone(&event),
                    );
                }
            },
            HotShotEvent::DaCertificateRecv(cert) => {
                let view = cert.view_number;

                tracing::trace!("Received DAC for view {}", *view);
                // Do nothing if the DAC is old
                ensure!(
                    view > self.latest_voted_view,
                    "Received DAC for an older view."
                );

                let cert_epoch = cert.data.epoch;

                let epoch_membership = self.membership.membership_for_epoch(cert_epoch).await?;
                let membership_da_stake_table = epoch_membership.da_stake_table().await;
                let membership_da_success_threshold = epoch_membership.da_success_threshold().await;

                // Validate the DAC.
                cert.is_valid_cert(
                    StakeTableEntries::<TYPES>::from(membership_da_stake_table).0,
                    membership_da_success_threshold,
                    &self.upgrade_lock,
                )
                .await
                .context(|e| warn!("Invalid DAC: {}", e))?;

                // Add to the storage.
                self.consensus
                    .write()
                    .await
                    .update_saved_da_certs(view, cert.clone());

                broadcast_event(
                    Arc::new(HotShotEvent::DaCertificateValidated(cert.clone())),
                    &event_sender.clone(),
                )
                .await;
                self.create_dependency_task_if_new(
                    view,
                    event_receiver,
                    &event_sender,
                    Arc::clone(&event),
                );
            },
            HotShotEvent::VidShareRecv(sender, share) => {
                let view = share.data.view_number();
                // Do nothing if the VID share is old
                tracing::trace!("Received VID share for view {}", *view);
                ensure!(
                    view > self.latest_voted_view,
                    "Received VID share for an older view."
                );

                // Validate the VID share.
                let payload_commitment = share.data.payload_commitment_ref();

                // Check that the signature is valid
                ensure!(
                    sender.validate(&share.signature, payload_commitment.as_ref()),
                    "VID share signature is invalid"
                );

                let vid_epoch = share.data.epoch();
                let target_epoch = share.data.target_epoch();
                let membership_reader = self.membership.membership_for_epoch(vid_epoch).await?;
                // ensure that the VID share was sent by a DA member OR the view leader
                ensure!(
                    membership_reader
                        .da_committee_members(view)
                        .await
                        .contains(sender)
                        || *sender == membership_reader.leader(view).await?,
                    "VID share was not sent by a DA member or the view leader."
                );

                let total_weight = vid_total_weight::<TYPES>(
                    self.membership
                        .membership_for_epoch(target_epoch)
                        .await?
                        .stake_table()
                        .await,
                    target_epoch,
                );

                if let Err(()) = share.data.verify_share(total_weight) {
                    bail!("Failed to verify VID share");
                }

                self.consensus
                    .write()
                    .await
                    .update_vid_shares(view, share.clone());

                ensure!(
                    *share.data.recipient_key() == self.public_key,
                    "Got a Valid VID share but it's not for our key"
                );

                broadcast_event(
                    Arc::new(HotShotEvent::VidShareValidated(share.clone())),
                    &event_sender.clone(),
                )
                .await;
                self.create_dependency_task_if_new(
                    view,
                    event_receiver,
                    &event_sender,
                    Arc::clone(&event),
                );
            },
            HotShotEvent::Timeout(view, ..) => {
                let view = TYPES::View::new(view.saturating_sub(1));
                // cancel old tasks
                let current_tasks = self.vote_dependencies.split_off(&view);
                while let Some((_, task)) = self.vote_dependencies.pop_last() {
                    task.abort();
                }
                self.vote_dependencies = current_tasks;
            },
            HotShotEvent::ViewChange(mut view, _) => {
                view = TYPES::View::new(view.saturating_sub(1));
                if !self.update_latest_voted_view(view).await {
                    tracing::debug!("view not updated");
                }
                // cancel old tasks
                let current_tasks = self.vote_dependencies.split_off(&view);
                while let Some((_, task)) = self.vote_dependencies.pop_last() {
                    task.abort();
                }
                self.vote_dependencies = current_tasks;
            },
            _ => {},
        }
        Ok(())
    }

    /// Handles voting for the last block in the epoch to form the Extended QC.
    #[allow(clippy::too_many_lines)]
    async fn handle_eqc_voting(
        &self,
        proposal: &Proposal<TYPES, QuorumProposalWrapper<TYPES>>,
        parent_leaf: &Leaf2<TYPES>,
        event_sender: Sender<Arc<HotShotEvent<TYPES>>>,
        event_receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<()> {
        tracing::info!("Reached end of epoch. Justify QC is for the last block in the epoch.");
        let proposed_leaf = Leaf2::from_quorum_proposal(&proposal.data);
        let parent_commitment = parent_leaf.commit();

        ensure!(
            proposed_leaf.height() == parent_leaf.height() && proposed_leaf.payload_commitment() == parent_leaf.payload_commitment(),
            error!("Justify QC is for the last block but it's not extended and a new block is proposed. Not voting!")
        );

        tracing::info!(
            "Reached end of epoch. Proposed leaf has the same height and payload as its parent."
        );

        let mut consensus_writer = self.consensus.write().await;

        let vid_shares = consensus_writer
            .vid_shares()
            .get(&parent_leaf.view_number())
            .context(warn!(
                "Proposed leaf is the same as its parent but we don't have our VID for it"
            ))?;

        let vid = vid_shares.get(&self.public_key).context(warn!(
            "Proposed leaf is the same as its parent but we don't have our VID for it"
        ))?;

        let mut updated_vid = vid.clone();
        updated_vid
            .data
            .set_view_number(proposal.data.view_number());
        consensus_writer.update_vid_shares(updated_vid.data.view_number(), updated_vid.clone());

        drop(consensus_writer);

        ensure!(
            proposed_leaf.parent_commitment() == parent_commitment,
            warn!("Proposed leaf parent commitment does not match parent leaf payload commitment. Aborting vote.")
        );

        // Update our persistent storage of the proposal. If we cannot store the proposal return
        // and error so we don't vote
        self.storage
            .write()
            .await
            .append_proposal_wrapper(proposal)
            .await
            .wrap()
            .context(|e| error!("failed to store proposal, not voting. error = {}", e))?;

        // Update internal state
        update_shared_state::<TYPES, I, V>(
            OuterConsensus::new(Arc::clone(&self.consensus.inner_consensus)),
            event_sender.clone(),
            event_receiver.clone().deactivate(),
            self.membership.clone(),
            self.public_key.clone(),
            self.private_key.clone(),
            self.upgrade_lock.clone(),
            proposal.data.view_number(),
            Arc::clone(&self.instance_state),
            &proposed_leaf,
            &updated_vid,
            Some(parent_leaf.view_number()),
            self.epoch_height,
        )
        .await
        .context(|e| error!("Failed to update shared consensus state, error = {}", e))?;

        let current_block_number = proposed_leaf.height();
        let current_epoch = TYPES::Epoch::new(epoch_from_block_number(
            current_block_number,
            self.epoch_height,
        ));

        let is_vote_leaf_extended = self
            .consensus
            .read()
            .await
            .is_leaf_extended(proposed_leaf.commit());
        if !is_vote_leaf_extended {
            // We're voting for the proposal that will probably form the eQC. We don't want to change
            // the view here because we will probably change it when we form the eQC.
            // The main reason is to handle view change event only once in the transaction task.
            tracing::trace!(
                "Sending ViewChange for view {} and epoch {}",
                proposal.data.view_number() + 1,
                *current_epoch
            );
            broadcast_event(
                Arc::new(HotShotEvent::ViewChange(
                    proposal.data.view_number() + 1,
                    Some(current_epoch),
                )),
                &event_sender,
            )
            .await;
        }

        submit_vote::<TYPES, I, V>(
            event_sender.clone(),
            self.membership
                .membership_for_epoch(Some(current_epoch))
                .await?,
            self.public_key.clone(),
            self.private_key.clone(),
            self.upgrade_lock.clone(),
            proposal.data.view_number(),
            Arc::clone(&self.storage),
            proposed_leaf,
            updated_vid,
            is_vote_leaf_extended,
            self.epoch_height,
        )
        .await
        .context(|e| debug!("Failed to submit vote; error = {}", e))
    }
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> TaskState
    for QuorumVoteTaskState<TYPES, I, V>
{
    type Event = HotShotEvent<TYPES>;

    async fn handle_event(
        &mut self,
        event: Arc<Self::Event>,
        sender: &Sender<Arc<Self::Event>>,
        receiver: &Receiver<Arc<Self::Event>>,
    ) -> Result<()> {
        self.handle(event, receiver.clone(), sender.clone()).await
    }

    fn cancel_subtasks(&mut self) {
        while let Some((_, handle)) = self.vote_dependencies.pop_last() {
            handle.abort();
        }
    }
}
