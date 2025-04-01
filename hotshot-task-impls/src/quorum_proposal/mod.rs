// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{collections::BTreeMap, sync::Arc, time::Instant};

use async_broadcast::{Receiver, Sender};
use async_lock::RwLock;
use async_trait::async_trait;
use either::Either;
use hotshot_task::{
    dependency::{AndDependency, EventDependency, OrDependency},
    dependency_task::DependencyTask,
    task::TaskState,
};
use hotshot_types::{
    consensus::OuterConsensus,
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    simple_certificate::{NextEpochQuorumCertificate2, QuorumCertificate2, UpgradeCertificate},
    traits::{
        node_implementation::{ConsensusTime, NodeImplementation, NodeType, Versions},
        signature_key::SignatureKey,
        storage::Storage,
    },
    utils::{
        is_epoch_transition, is_middle_transition_block, is_transition_block,
        EpochTransitionIndicator,
    },
    vote::{Certificate, HasViewNumber},
    StakeTableEntries,
};
use hotshot_utils::anytrace::*;
use tokio::task::JoinHandle;
use tracing::instrument;

use self::handlers::{ProposalDependency, ProposalDependencyHandle};
use crate::{events::HotShotEvent, quorum_proposal::handlers::handle_eqc_formed};

mod handlers;

/// The state for the quorum proposal task.
pub struct QuorumProposalTaskState<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> {
    /// Latest view number that has been proposed for.
    pub latest_proposed_view: TYPES::View,

    /// Current epoch
    pub cur_epoch: Option<TYPES::Epoch>,

    /// Table for the in-progress proposal dependency tasks.
    pub proposal_dependencies: BTreeMap<TYPES::View, JoinHandle<()>>,

    /// Formed QCs
    pub formed_quorum_certificates: BTreeMap<TYPES::View, QuorumCertificate2<TYPES>>,

    /// Formed QCs for the next epoch
    pub formed_next_epoch_quorum_certificates:
        BTreeMap<TYPES::View, NextEpochQuorumCertificate2<TYPES>>,

    /// Immutable instance state
    pub instance_state: Arc<TYPES::InstanceState>,

    /// Membership for Quorum Certs/votes
    pub membership_coordinator: EpochMembershipCoordinator<TYPES>,

    /// Our public key
    pub public_key: TYPES::SignatureKey,

    /// Our Private Key
    pub private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,

    /// View timeout from config.
    pub timeout: u64,

    /// This node's storage ref
    pub storage: Arc<RwLock<I::Storage>>,

    /// Shared consensus task state
    pub consensus: OuterConsensus<TYPES>,

    /// The node's id
    pub id: u64,

    /// The most recent upgrade certificate this node formed.
    /// Note: this is ONLY for certificates that have been formed internally,
    /// so that we can propose with them.
    ///
    /// Certificates received from other nodes will get reattached regardless of this fields,
    /// since they will be present in the leaf we propose off of.
    pub formed_upgrade_certificate: Option<UpgradeCertificate<TYPES>>,

    /// Lock for a decided upgrade
    pub upgrade_lock: UpgradeLock<TYPES, V>,

    /// Number of blocks in an epoch, zero means there are no epochs
    pub epoch_height: u64,
}

impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>
    QuorumProposalTaskState<TYPES, I, V>
{
    /// Create an event dependency
    #[instrument(skip_all, fields(id = self.id, latest_proposed_view = *self.latest_proposed_view), name = "Create event dependency", level = "info")]
    fn create_event_dependency(
        &self,
        dependency_type: ProposalDependency,
        view_number: TYPES::View,
        event_receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
    ) -> EventDependency<Arc<HotShotEvent<TYPES>>> {
        let id = self.id;
        EventDependency::new(
            event_receiver,
            Box::new(move |event| {
                let event = event.as_ref();
                let event_view = match dependency_type {
                    ProposalDependency::Qc => {
                        if let HotShotEvent::Qc2Formed(either::Left(qc)) = event {
                            qc.view_number() + 1
                        } else {
                            return false;
                        }
                    },
                    ProposalDependency::TimeoutCert => {
                        if let HotShotEvent::Qc2Formed(either::Right(timeout)) = event {
                            timeout.view_number() + 1
                        } else {
                            return false;
                        }
                    },
                    ProposalDependency::ViewSyncCert => {
                        if let HotShotEvent::ViewSyncFinalizeCertificateRecv(view_sync_cert) = event
                        {
                            view_sync_cert.view_number()
                        } else {
                            return false;
                        }
                    },
                    ProposalDependency::Proposal => {
                        if let HotShotEvent::QuorumProposalPreliminarilyValidated(proposal) = event
                        {
                            proposal.data.view_number() + 1
                        } else {
                            return false;
                        }
                    },
                    ProposalDependency::PayloadAndMetadata => {
                        if let HotShotEvent::SendPayloadCommitmentAndMetadata(
                            _payload_commitment,
                            _builder_commitment,
                            _metadata,
                            view_number,
                            _fee,
                            _auction_result,
                        ) = event
                        {
                            *view_number
                        } else {
                            return false;
                        }
                    },
                    ProposalDependency::VidShare => {
                        if let HotShotEvent::VidDisperseSend(vid_disperse, _) = event {
                            vid_disperse.data.view_number()
                        } else {
                            return false;
                        }
                    },
                };
                let valid = event_view == view_number;
                if valid {
                    tracing::debug!(
                        "Dependency {dependency_type:?} is complete for view {event_view:?}, my id is {id:?}!",
                    );
                }
                valid
            }),
        )
    }

    /// Creates the requisite dependencies for the Quorum Proposal task. It also handles any event forwarding.
    fn create_and_complete_dependencies(
        &self,
        view_number: TYPES::View,
        event_receiver: &Receiver<Arc<HotShotEvent<TYPES>>>,
        event: Arc<HotShotEvent<TYPES>>,
    ) -> AndDependency<Vec<Vec<Arc<HotShotEvent<TYPES>>>>> {
        let mut proposal_dependency = self.create_event_dependency(
            ProposalDependency::Proposal,
            view_number,
            event_receiver.clone(),
        );

        let mut qc_dependency = self.create_event_dependency(
            ProposalDependency::Qc,
            view_number,
            event_receiver.clone(),
        );

        let mut view_sync_dependency = self.create_event_dependency(
            ProposalDependency::ViewSyncCert,
            view_number,
            event_receiver.clone(),
        );

        let mut timeout_dependency = self.create_event_dependency(
            ProposalDependency::TimeoutCert,
            view_number,
            event_receiver.clone(),
        );

        let mut payload_commitment_dependency = self.create_event_dependency(
            ProposalDependency::PayloadAndMetadata,
            view_number,
            event_receiver.clone(),
        );

        let mut vid_share_dependency = self.create_event_dependency(
            ProposalDependency::VidShare,
            view_number,
            event_receiver.clone(),
        );

        match event.as_ref() {
            HotShotEvent::SendPayloadCommitmentAndMetadata(..) => {
                payload_commitment_dependency.mark_as_completed(Arc::clone(&event));
            },
            HotShotEvent::QuorumProposalPreliminarilyValidated(..) => {
                proposal_dependency.mark_as_completed(event);
            },
            HotShotEvent::Qc2Formed(quorum_certificate) => match quorum_certificate {
                Either::Right(_) => {
                    timeout_dependency.mark_as_completed(event);
                },
                Either::Left(_) => {
                    qc_dependency.mark_as_completed(event);
                },
            },
            HotShotEvent::ViewSyncFinalizeCertificateRecv(_) => {
                view_sync_dependency.mark_as_completed(event);
            },
            HotShotEvent::VidDisperseSend(..) => {
                vid_share_dependency.mark_as_completed(event);
            },
            _ => {},
        };

        // We have three cases to consider:
        let mut secondary_deps = vec![
            // 1. A timeout cert was received
            AndDependency::from_deps(vec![timeout_dependency]),
            // 2. A view sync cert was received.
            AndDependency::from_deps(vec![view_sync_dependency]),
        ];
        // 3. A `Qc2Formed`` event (and `QuorumProposalRecv` event)
        if *view_number > 1 {
            secondary_deps.push(AndDependency::from_deps(vec![
                qc_dependency,
                proposal_dependency,
            ]));
        } else {
            secondary_deps.push(AndDependency::from_deps(vec![qc_dependency]));
        }

        let primary_deps = vec![payload_commitment_dependency, vid_share_dependency];

        AndDependency::from_deps(vec![OrDependency::from_deps(vec![
            AndDependency::from_deps(vec![
                OrDependency::from_deps(vec![AndDependency::from_deps(primary_deps)]),
                OrDependency::from_deps(secondary_deps),
            ]),
        ])])
    }

    /// Create and store an [`AndDependency`] combining [`EventDependency`]s associated with the
    /// given view number if it doesn't exist. Also takes in the received `event` to seed a
    /// dependency as already completed. This allows for the task to receive a proposable event
    /// without losing the data that it received, as the dependency task would otherwise have no
    /// ability to receive the event and, thus, would never propose.
    #[instrument(skip_all, fields(id = self.id, latest_proposed_view = *self.latest_proposed_view), name = "Create dependency task", level = "error")]
    async fn create_dependency_task_if_new(
        &mut self,
        view_number: TYPES::View,
        epoch_number: Option<TYPES::Epoch>,
        event_receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
        event_sender: Sender<Arc<HotShotEvent<TYPES>>>,
        event: Arc<HotShotEvent<TYPES>>,
        epoch_transition_indicator: EpochTransitionIndicator,
    ) -> Result<()> {
        let epoch_membership = self
            .membership_coordinator
            .membership_for_epoch(epoch_number)
            .await?;
        let leader_in_current_epoch =
            epoch_membership.leader(view_number).await? == self.public_key;
        // If we are in the epoch transition and we are the leader in the next epoch,
        // we might want to start collecting dependencies for our next epoch proposal.

        let leader_in_next_epoch = epoch_number.is_some()
            && matches!(
                epoch_transition_indicator,
                EpochTransitionIndicator::InTransition
            )
            && epoch_membership
                .next_epoch()
                .await
                .context(warn!(
                    "No Stake Table for Epoch = {:?}",
                    epoch_number.unwrap() + 1
                ))?
                .leader(view_number)
                .await?
                == self.public_key;

        // Don't even bother making the task if we are not entitled to propose anyway.
        ensure!(
            leader_in_current_epoch || leader_in_next_epoch,
            debug!("We are not the leader of the next view")
        );

        // Don't try to propose twice for the same view.
        ensure!(
            view_number > self.latest_proposed_view,
            "We have already proposed for this view"
        );

        tracing::debug!(
            "Attempting to make dependency task for view {view_number:?} and event {event:?}"
        );

        ensure!(
            !self.proposal_dependencies.contains_key(&view_number),
            "Task already exists"
        );

        let dependency_chain =
            self.create_and_complete_dependencies(view_number, &event_receiver, event);

        let dependency_task = DependencyTask::new(
            dependency_chain,
            ProposalDependencyHandle {
                latest_proposed_view: self.latest_proposed_view,
                view_number,
                sender: event_sender,
                receiver: event_receiver,
                membership: epoch_membership,
                public_key: self.public_key.clone(),
                private_key: self.private_key.clone(),
                instance_state: Arc::clone(&self.instance_state),
                consensus: OuterConsensus::new(Arc::clone(&self.consensus.inner_consensus)),
                timeout: self.timeout,
                formed_upgrade_certificate: self.formed_upgrade_certificate.clone(),
                upgrade_lock: self.upgrade_lock.clone(),
                id: self.id,
                view_start_time: Instant::now(),
                epoch_height: self.epoch_height,
            },
        );
        self.proposal_dependencies
            .insert(view_number, dependency_task.run());

        Ok(())
    }

    /// Update the latest proposed view number.
    #[instrument(skip_all, fields(id = self.id, latest_proposed_view = *self.latest_proposed_view), name = "Update latest proposed view", level = "error")]
    async fn update_latest_proposed_view(&mut self, new_view: TYPES::View) -> bool {
        if *self.latest_proposed_view < *new_view {
            tracing::debug!(
                "Updating latest proposed view from {} to {}",
                *self.latest_proposed_view,
                *new_view
            );

            // Cancel the old dependency tasks.
            for view in (*self.latest_proposed_view + 1)..=(*new_view) {
                if let Some(dependency) = self.proposal_dependencies.remove(&TYPES::View::new(view))
                {
                    dependency.abort();
                }
            }

            self.latest_proposed_view = new_view;

            return true;
        }
        false
    }

    async fn update_high_qc(&mut self, qc: QuorumCertificate2<TYPES>) -> Result<()> {
        self.consensus
            .write()
            .await
            .update_high_qc(qc.clone())
            .wrap()
            .context(error!(
                "Failed to update high QC in internal consensus state!"
            ))?;
        let in_epoch_transition = qc
            .data
            .block_number
            .is_some_and(|bn| is_middle_transition_block(bn, self.epoch_height));

        // Don't update storage if we're in the epoch transition
        if !in_epoch_transition {
            tracing::error!(
                "Updating high QC in storage for view {:?} and height {:?}",
                qc.view_number(),
                qc.data.block_number
            );
            // Then update the high QC in storage
            self.storage
                .write()
                .await
                .update_high_qc2(qc)
                .await
                .wrap()
                .context(error!("Failed to update high QC in storage!"))?;
        }
        Ok(())
    }

    async fn update_next_epoch_high_qc(
        &mut self,
        qc: NextEpochQuorumCertificate2<TYPES>,
    ) -> Result<()> {
        self.consensus
            .write()
            .await
            .update_next_epoch_high_qc(qc.clone())
            .wrap()
            .context(error!(
                "Failed to update next epoch high QC in internal consensus state!"
            ))?;
        let in_epoch_transition = qc
            .data
            .block_number
            .is_some_and(|bn| is_middle_transition_block(bn, self.epoch_height));

        // Then update the next epoch high QC in storage
        // Don't update storage if we're in the epoch transition
        if !in_epoch_transition {
            tracing::debug!(
                "Updating next epoch high QC in storage for view {:?} and height {:?}",
                qc.view_number(),
                qc.data.block_number
            );
            self.storage
                .write()
                .await
                .update_next_epoch_high_qc2(qc)
                .await
                .wrap()
                .context(error!("Failed to update next epoch high QC in storage!"))?;
        }
        Ok(())
    }

    /// Hanldles checking that both certificates for an eqc exist and stores if they do.  Also handles storing the QC for cases
    /// where we do not need a next epoch QC.
    async fn check_eqc_and_store(
        &mut self,
        view_number: TYPES::View,
        qc: Either<QuorumCertificate2<TYPES>, NextEpochQuorumCertificate2<TYPES>>,
    ) -> Result<()> {
        let (qc, next_epoch_qc) = match qc {
            Either::Left(qc) => {
                let Some(block_number) = qc.data.block_number else {
                    return self.update_high_qc(qc).await;
                };

                if !self.upgrade_lock.epochs_enabled(view_number).await
                    || !is_epoch_transition(block_number, self.epoch_height)
                {
                    return self.update_high_qc(qc).await;
                }
                let Some(next_epoch_qc) =
                    self.formed_next_epoch_quorum_certificates.get(&view_number)
                else {
                    return Ok(());
                };
                if next_epoch_qc.data.leaf_commit != qc.data.leaf_commit {
                    return Ok(());
                }
                (qc, next_epoch_qc.clone())
            },
            Either::Right(next_epoch_qc) => {
                if !self.upgrade_lock.epochs_enabled(view_number).await {
                    return Ok(());
                }
                let Some(high_qc) = self.formed_quorum_certificates.get(&view_number) else {
                    return Ok(());
                };
                if high_qc.data.leaf_commit != next_epoch_qc.data.leaf_commit {
                    return Ok(());
                }
                (high_qc.clone(), next_epoch_qc)
            },
        };
        // clean up old qcs
        self.formed_next_epoch_quorum_certificates = self
            .formed_next_epoch_quorum_certificates
            .split_off(&view_number);
        self.formed_quorum_certificates = self.formed_quorum_certificates.split_off(&view_number);

        if is_transition_block(qc.data.block_number.unwrap(), self.epoch_height) {
            self.consensus
                .write()
                .await
                .update_transition_qc(qc.clone(), next_epoch_qc.clone());
        }
        // Store the new eqc
        self.update_high_qc(qc).await?;
        self.update_next_epoch_high_qc(next_epoch_qc).await?;
        Ok(())
    }

    /// Handles a consensus event received on the event stream
    #[instrument(skip_all, fields(id = self.id, latest_proposed_view = *self.latest_proposed_view, epoch = self.cur_epoch.map(|x| *x)), name = "handle method", level = "error", target = "QuorumProposalTaskState")]
    pub async fn handle(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        event_receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
        event_sender: Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<()> {
        let epoch_number = self.cur_epoch;
        let epoch_transition_indicator = if self.consensus.read().await.is_high_qc_for_last_block()
        {
            EpochTransitionIndicator::InTransition
        } else {
            EpochTransitionIndicator::NotInTransition
        };
        match event.as_ref() {
            HotShotEvent::UpgradeCertificateFormed(cert) => {
                tracing::debug!(
                    "Upgrade certificate received for view {}!",
                    *cert.view_number
                );
                // Update our current upgrade_cert as long as we still have a chance of reaching a decide on it in time.
                if cert.data.decide_by >= self.latest_proposed_view + 3 {
                    tracing::debug!("Updating current formed_upgrade_certificate");

                    self.formed_upgrade_certificate = Some(cert.clone());
                }
            },
            HotShotEvent::Qc2Formed(cert) => match cert.clone() {
                either::Right(timeout_cert) => {
                    let view_number = timeout_cert.view_number + 1;
                    self.create_dependency_task_if_new(
                        view_number,
                        epoch_number,
                        event_receiver,
                        event_sender,
                        Arc::clone(&event),
                        epoch_transition_indicator,
                    )
                    .await?;
                },
                either::Left(qc) => {
                    // Only update if the qc is from a newer view
                    if qc.view_number <= self.consensus.read().await.high_qc().view_number {
                        tracing::trace!(
                            "Received a QC for a view that was not > than our current high QC"
                        );
                    }

                    self.formed_quorum_certificates
                        .insert(qc.view_number(), qc.clone());

                    self.check_eqc_and_store(qc.view_number(), Either::Left(qc.clone()))
                        .await?;

                    handle_eqc_formed(
                        qc.view_number(),
                        qc.data.leaf_commit,
                        qc.data.block_number,
                        self,
                        &event_sender,
                    )
                    .await;

                    let view_number = qc.view_number() + 1;
                    self.create_dependency_task_if_new(
                        view_number,
                        epoch_number,
                        event_receiver,
                        event_sender,
                        Arc::clone(&event),
                        epoch_transition_indicator,
                    )
                    .await?;
                },
            },
            HotShotEvent::SendPayloadCommitmentAndMetadata(
                _payload_commitment,
                _builder_commitment,
                _metadata,
                view_number,
                _fee,
                _auction_result,
            ) => {
                let view_number = *view_number;

                self.create_dependency_task_if_new(
                    view_number,
                    epoch_number,
                    event_receiver,
                    event_sender,
                    Arc::clone(&event),
                    EpochTransitionIndicator::NotInTransition,
                )
                .await?;
            },
            HotShotEvent::ViewSyncFinalizeCertificateRecv(certificate) => {
                let epoch_number = certificate.data.epoch;
                let epoch_membership = self
                    .membership_coordinator
                    .membership_for_epoch(epoch_number)
                    .await
                    .context(warn!("No Stake Table for Epoch = {:?}", epoch_number))?;

                let membership_stake_table = epoch_membership.stake_table().await;
                let membership_success_threshold = epoch_membership.success_threshold().await;

                certificate
                    .is_valid_cert(
                        StakeTableEntries::<TYPES>::from(membership_stake_table).0,
                        membership_success_threshold,
                        &self.upgrade_lock,
                    )
                    .await
                    .context(|e| {
                        warn!(
                            "View Sync Finalize certificate {:?} was invalid: {}",
                            certificate.data(),
                            e
                        )
                    })?;

                let view_number = certificate.view_number;

                self.create_dependency_task_if_new(
                    view_number,
                    epoch_number,
                    event_receiver,
                    event_sender,
                    event,
                    EpochTransitionIndicator::NotInTransition,
                )
                .await?;
            },
            HotShotEvent::QuorumProposalPreliminarilyValidated(proposal) => {
                let view_number = proposal.data.view_number();
                // All nodes get the latest proposed view as a proxy of `cur_view` of old.
                if !self.update_latest_proposed_view(view_number).await {
                    tracing::trace!("Failed to update latest proposed view");
                }

                self.create_dependency_task_if_new(
                    view_number + 1,
                    epoch_number,
                    event_receiver,
                    event_sender,
                    Arc::clone(&event),
                    epoch_transition_indicator,
                )
                .await?;
            },
            HotShotEvent::QuorumProposalSend(proposal, _) => {
                let view = proposal.data.view_number();

                ensure!(
                    self.update_latest_proposed_view(view).await,
                    "Failed to update latest proposed view"
                );
            },
            HotShotEvent::VidDisperseSend(vid_disperse, _) => {
                let view_number = vid_disperse.data.view_number();
                self.create_dependency_task_if_new(
                    view_number,
                    epoch_number,
                    event_receiver,
                    event_sender,
                    Arc::clone(&event),
                    EpochTransitionIndicator::NotInTransition,
                )
                .await?;
            },
            HotShotEvent::ViewChange(view, epoch) => {
                if epoch > &self.cur_epoch {
                    self.cur_epoch = *epoch;
                }
                let keep_view = TYPES::View::new(view.saturating_sub(1));
                self.cancel_tasks(keep_view);
            },
            HotShotEvent::Timeout(view, ..) => {
                let keep_view = TYPES::View::new(view.saturating_sub(1));
                self.cancel_tasks(keep_view);
            },
            HotShotEvent::NextEpochQc2Formed(Either::Left(next_epoch_qc)) => {
                // Only update if the qc is from a newer view
                let current_next_epoch_qc =
                    self.consensus.read().await.next_epoch_high_qc().cloned();
                ensure!(current_next_epoch_qc.is_none() ||
                    next_epoch_qc.view_number > current_next_epoch_qc.unwrap().view_number,
                    debug!("Received a next epoch QC for a view that was not > than our current next epoch high QC")
                );

                self.formed_next_epoch_quorum_certificates
                    .insert(next_epoch_qc.view_number(), next_epoch_qc.clone());

                self.check_eqc_and_store(
                    next_epoch_qc.view_number(),
                    Either::Right(next_epoch_qc.clone()),
                )
                .await?;

                handle_eqc_formed(
                    next_epoch_qc.view_number(),
                    next_epoch_qc.data.leaf_commit,
                    next_epoch_qc.data.block_number,
                    self,
                    &event_sender,
                )
                .await;
            },
            _ => {},
        }
        Ok(())
    }

    /// Cancel all tasks the consensus tasks has spawned before the given view
    pub fn cancel_tasks(&mut self, view: TYPES::View) {
        let keep = self.proposal_dependencies.split_off(&view);
        while let Some((_, task)) = self.proposal_dependencies.pop_first() {
            task.abort();
        }
        self.proposal_dependencies = keep;
    }
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> TaskState
    for QuorumProposalTaskState<TYPES, I, V>
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
        while let Some((_, handle)) = self.proposal_dependencies.pop_first() {
            handle.abort();
        }
    }
}
