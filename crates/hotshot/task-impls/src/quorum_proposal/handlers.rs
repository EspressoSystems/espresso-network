// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! This module holds the dependency task for the QuorumProposalTask. It is spawned whenever an event that could
//! initiate a proposal occurs.

use std::{
    marker::PhantomData,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{ensure, Context, Result};
use async_broadcast::{Receiver, Sender};
use committable::{Commitment, Committable};
use hotshot_task::dependency_task::HandleDepOutput;
use hotshot_types::{
    consensus::{CommitmentAndMetadata, OuterConsensus},
    data::{Leaf2, QuorumProposal2, QuorumProposalWrapper, VidDisperse, ViewChangeEvidence2},
    epoch_membership::EpochMembership,
    message::Proposal,
    simple_certificate::{
        LightClientStateUpdateCertificate, NextEpochQuorumCertificate2, QuorumCertificate2,
        UpgradeCertificate,
    },
    traits::{
        block_contents::BlockHeader,
        node_implementation::{ConsensusTime, NodeImplementation, NodeType},
        signature_key::SignatureKey,
        BlockPayload,
    },
    utils::{
        epoch_from_block_number, is_epoch_root, is_epoch_transition, is_last_block,
        is_transition_block, option_epoch_from_block_number,
    },
    vote::HasViewNumber,
};
use hotshot_utils::anytrace::*;
use tracing::instrument;
use vbs::version::StaticVersionType;

use crate::{
    events::HotShotEvent,
    helpers::{
        broadcast_event, check_qc_state_cert_correspondence, parent_leaf_and_state,
        validate_light_client_state_update_certificate, validate_qc_and_next_epoch_qc,
    },
    quorum_proposal::{QuorumProposalTaskState, UpgradeLock, Versions},
};

/// Proposal dependency types. These types represent events that precipitate a proposal.
#[derive(PartialEq, Debug)]
pub(crate) enum ProposalDependency {
    /// For the `SendPayloadCommitmentAndMetadata` event.
    PayloadAndMetadata,

    /// For the `Qc2Formed`, `ExtendedQc2Formed`, and `EpochRootQcFormed` event.
    Qc,

    /// For the `ViewSyncFinalizeCertificateRecv` event.
    ViewSyncCert,

    /// For the `Qc2Formed`, `ExtendedQc2Formed`, and `EpochRootQcFormed` event timeout branch.
    TimeoutCert,

    /// For the `QuorumProposalRecv` event.
    Proposal,

    /// For the `VidShareValidated` event.
    VidShare,
}

/// Handler for the proposal dependency
pub struct ProposalDependencyHandle<TYPES: NodeType, V: Versions> {
    /// Latest view number that has been proposed for (proxy for cur_view).
    pub latest_proposed_view: TYPES::View,

    /// The view number to propose for.
    pub view_number: TYPES::View,

    /// The event sender.
    pub sender: Sender<Arc<HotShotEvent<TYPES>>>,

    /// The event receiver.
    pub receiver: Receiver<Arc<HotShotEvent<TYPES>>>,

    /// Immutable instance state
    pub instance_state: Arc<TYPES::InstanceState>,

    /// Membership for Quorum Certs/votes
    pub membership: EpochMembership<TYPES>,

    /// Our public key
    pub public_key: TYPES::SignatureKey,

    /// Our Private Key
    pub private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,

    /// Shared consensus task state
    pub consensus: OuterConsensus<TYPES>,

    /// View timeout from config.
    pub timeout: u64,

    /// The most recent upgrade certificate this node formed.
    /// Note: this is ONLY for certificates that have been formed internally,
    /// so that we can propose with them.
    ///
    /// Certificates received from other nodes will get reattached regardless of this fields,
    /// since they will be present in the leaf we propose off of.
    pub formed_upgrade_certificate: Option<UpgradeCertificate<TYPES>>,

    /// Lock for a decided upgrade
    pub upgrade_lock: UpgradeLock<TYPES, V>,

    /// The node's id
    pub id: u64,

    /// The time this view started
    pub view_start_time: Instant,

    /// Number of blocks in an epoch, zero means there are no epochs
    pub epoch_height: u64,
}

impl<TYPES: NodeType, V: Versions> ProposalDependencyHandle<TYPES, V> {
    /// Return the next HighQc we get from the event stream
    async fn wait_for_qc_event(
        &self,
        mut rx: Receiver<Arc<HotShotEvent<TYPES>>>,
    ) -> Option<(
        QuorumCertificate2<TYPES>,
        Option<NextEpochQuorumCertificate2<TYPES>>,
        Option<LightClientStateUpdateCertificate<TYPES>>,
    )> {
        while let Ok(event) = rx.recv_direct().await {
            let (qc, maybe_next_epoch_qc, mut maybe_state_cert) = match event.as_ref() {
                HotShotEvent::HighQcRecv(qc, maybe_next_epoch_qc, _sender) => {
                    (qc, maybe_next_epoch_qc, None)
                },
                HotShotEvent::EpochRootQcRecv(root_qc, _sender) => {
                    (&root_qc.qc, &None, Some(root_qc.state_cert.clone()))
                },
                _ => continue,
            };
            if validate_qc_and_next_epoch_qc(
                qc,
                maybe_next_epoch_qc.as_ref(),
                &self.consensus,
                &self.membership.coordinator,
                &self.upgrade_lock,
                self.epoch_height,
            )
            .await
            .is_ok()
            {
                if qc
                    .data
                    .block_number
                    .is_some_and(|bn| is_epoch_root(bn, self.epoch_height))
                {
                    // Validate the state cert
                    if let Some(state_cert) = &maybe_state_cert {
                        if validate_light_client_state_update_certificate(
                            state_cert,
                            &self.membership.coordinator,
                        )
                        .await
                        .is_err()
                            || !check_qc_state_cert_correspondence(
                                qc,
                                state_cert,
                                self.epoch_height,
                            )
                        {
                            tracing::error!("Failed to validate state cert");
                            return None;
                        }
                    } else {
                        tracing::error!("Received an epoch root QC but we don't have the corresponding state cert.");
                        return None;
                    }
                } else {
                    maybe_state_cert = None;
                }
                return Some((qc.clone(), maybe_next_epoch_qc.clone(), maybe_state_cert));
            }
        }
        None
    }

    async fn wait_for_transition_qc(
        &self,
    ) -> Result<
        Option<(
            QuorumCertificate2<TYPES>,
            NextEpochQuorumCertificate2<TYPES>,
        )>,
    > {
        ensure!(
            self.upgrade_lock.epochs_enabled(self.view_number).await,
            error!("Epochs are not enabled yet we tried to wait for Highest QC.")
        );

        let mut transition_qc = self.consensus.read().await.transition_qc().cloned();

        let wait_duration = Duration::from_millis(self.timeout / 2);

        let mut rx = self.receiver.clone();

        // drain any qc off the queue
        // We don't watch for EpochRootQcRecv events here because it's not in transition.
        while let Ok(event) = rx.try_recv() {
            if let HotShotEvent::HighQcRecv(qc, maybe_next_epoch_qc, _sender) = event.as_ref() {
                if let Some(block_number) = qc.data.block_number {
                    if !is_transition_block(block_number, self.epoch_height) {
                        continue;
                    }
                } else {
                    continue;
                }
                let Some(next_epoch_qc) = maybe_next_epoch_qc else {
                    continue;
                };
                if validate_qc_and_next_epoch_qc(
                    qc,
                    Some(next_epoch_qc),
                    &self.consensus,
                    &self.membership.coordinator,
                    &self.upgrade_lock,
                    self.epoch_height,
                )
                .await
                .is_ok()
                    && transition_qc
                        .as_ref()
                        .is_none_or(|tqc| qc.view_number() > tqc.0.view_number())
                {
                    transition_qc = Some((qc.clone(), next_epoch_qc.clone()));
                }
            }
        }
        // TODO configure timeout
        while self.view_start_time.elapsed() < wait_duration {
            let time_spent = Instant::now()
            .checked_duration_since(self.view_start_time)
            .ok_or(error!("Time elapsed since the start of the task is negative. This should never happen."))?;
            let time_left = wait_duration
                .checked_sub(time_spent)
                .ok_or(info!("No time left"))?;
            let Ok(Ok(event)) = tokio::time::timeout(time_left, rx.recv_direct()).await else {
                return Ok(transition_qc);
            };
            if let HotShotEvent::HighQcRecv(qc, maybe_next_epoch_qc, _sender) = event.as_ref() {
                if let Some(block_number) = qc.data.block_number {
                    if !is_transition_block(block_number, self.epoch_height) {
                        continue;
                    }
                } else {
                    continue;
                }
                let Some(next_epoch_qc) = maybe_next_epoch_qc else {
                    continue;
                };
                if validate_qc_and_next_epoch_qc(
                    qc,
                    Some(next_epoch_qc),
                    &self.consensus,
                    &self.membership.coordinator,
                    &self.upgrade_lock,
                    self.epoch_height,
                )
                .await
                .is_ok()
                    && transition_qc
                        .as_ref()
                        .is_none_or(|tqc| qc.view_number() > tqc.0.view_number())
                {
                    transition_qc = Some((qc.clone(), next_epoch_qc.clone()));
                }
            }
        }
        Ok(transition_qc)
    }
    /// Waits for the configured timeout for nodes to send HighQc messages to us.  We'll
    /// then propose with the highest QC from among these proposals. A light client state
    /// update certificate is also returned if the highest QC is an epoch root QC.
    async fn wait_for_highest_qc(
        &self,
    ) -> Result<(
        QuorumCertificate2<TYPES>,
        Option<NextEpochQuorumCertificate2<TYPES>>,
        Option<LightClientStateUpdateCertificate<TYPES>>,
    )> {
        tracing::debug!("waiting for QC");
        // If we haven't upgraded to Hotstuff 2 just return the high qc right away
        ensure!(
            self.upgrade_lock.epochs_enabled(self.view_number).await,
            error!("Epochs are not enabled yet we tried to wait for Highest QC.")
        );

        let consensus_reader = self.consensus.read().await;
        let mut highest_qc = consensus_reader.high_qc().clone();
        let mut state_cert = if highest_qc
            .data
            .block_number
            .is_some_and(|bn| is_epoch_root(bn, self.epoch_height))
        {
            consensus_reader.state_cert().cloned()
        } else {
            None
        };
        let mut next_epoch_qc = if highest_qc
            .data
            .block_number
            .is_some_and(|bn| is_last_block(bn, self.epoch_height))
        {
            let maybe_neqc = consensus_reader.next_epoch_high_qc().cloned();
            if maybe_neqc
                .as_ref()
                .is_some_and(|neqc| neqc.data.leaf_commit == highest_qc.data.leaf_commit)
            {
                maybe_neqc
            } else {
                None
            }
        } else {
            None
        };
        drop(consensus_reader);

        let wait_duration = Duration::from_millis(self.timeout / 2);

        let mut rx = self.receiver.clone();

        // drain any qc off the queue
        while let Ok(event) = rx.try_recv() {
            let (qc, maybe_next_epoch_qc, mut maybe_state_cert) = match event.as_ref() {
                HotShotEvent::HighQcRecv(qc, maybe_next_epoch_qc, _sender) => {
                    (qc, maybe_next_epoch_qc, None)
                },
                HotShotEvent::EpochRootQcRecv(root_qc, _sender) => {
                    (&root_qc.qc, &None, Some(root_qc.state_cert.clone()))
                },
                _ => continue,
            };
            if validate_qc_and_next_epoch_qc(
                qc,
                maybe_next_epoch_qc.as_ref(),
                &self.consensus,
                &self.membership.coordinator,
                &self.upgrade_lock,
                self.epoch_height,
            )
            .await
            .is_ok()
            {
                if qc
                    .data
                    .block_number
                    .is_some_and(|bn| is_epoch_root(bn, self.epoch_height))
                {
                    // Validate the state cert
                    if let Some(state_cert) = &maybe_state_cert {
                        if validate_light_client_state_update_certificate(
                            state_cert,
                            &self.membership.coordinator,
                        )
                        .await
                        .is_err()
                            || !check_qc_state_cert_correspondence(
                                qc,
                                state_cert,
                                self.epoch_height,
                            )
                        {
                            tracing::error!("Failed to validate state cert");
                            continue;
                        }
                    } else {
                        tracing::error!("Received an epoch root QC but we don't have the corresponding state cert.");
                        continue;
                    }
                } else {
                    maybe_state_cert = None;
                }
                if qc.view_number() > highest_qc.view_number() {
                    highest_qc = qc.clone();
                    next_epoch_qc = maybe_next_epoch_qc.clone();
                    state_cert = maybe_state_cert;
                }
            }
        }

        // TODO configure timeout
        while self.view_start_time.elapsed() < wait_duration {
            let time_spent = Instant::now()
                .checked_duration_since(self.view_start_time)
                .ok_or(error!("Time elapsed since the start of the task is negative. This should never happen."))?;
            let time_left = wait_duration
                .checked_sub(time_spent)
                .ok_or(info!("No time left"))?;
            let Ok(maybe_qc_state_cert) =
                tokio::time::timeout(time_left, self.wait_for_qc_event(rx.clone())).await
            else {
                tracing::info!("Some nodes did not respond with their HighQc in time. Continuing with the highest QC that we received: {highest_qc:?}");
                return Ok((highest_qc, next_epoch_qc, state_cert));
            };
            let Some((qc, maybe_next_epoch_qc, maybe_state_cert)) = maybe_qc_state_cert else {
                continue;
            };
            if qc.view_number() > highest_qc.view_number() {
                highest_qc = qc;
                next_epoch_qc = maybe_next_epoch_qc;
                state_cert = maybe_state_cert;
            }
        }
        Ok((highest_qc, next_epoch_qc, state_cert))
    }
    /// Publishes a proposal given the [`CommitmentAndMetadata`], [`VidDisperse`]
    /// and high qc [`hotshot_types::simple_certificate::QuorumCertificate`],
    /// with optional [`ViewChangeEvidence`].
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all, fields(id = self.id, view_number = *self.view_number, latest_proposed_view = *self.latest_proposed_view))]
    async fn publish_proposal(
        &self,
        commitment_and_metadata: CommitmentAndMetadata<TYPES>,
        _vid_share: Proposal<TYPES, VidDisperse<TYPES>>,
        view_change_evidence: Option<ViewChangeEvidence2<TYPES>>,
        formed_upgrade_certificate: Option<UpgradeCertificate<TYPES>>,
        parent_qc: QuorumCertificate2<TYPES>,
        maybe_next_epoch_qc: Option<NextEpochQuorumCertificate2<TYPES>>,
        maybe_state_cert: Option<LightClientStateUpdateCertificate<TYPES>>,
    ) -> Result<()> {
        let (parent_leaf, state) = parent_leaf_and_state(
            &self.sender,
            &self.receiver,
            self.membership.coordinator.clone(),
            self.public_key.clone(),
            self.private_key.clone(),
            OuterConsensus::new(Arc::clone(&self.consensus.inner_consensus)),
            &self.upgrade_lock,
            &parent_qc,
            self.epoch_height,
        )
        .await?;

        // In order of priority, we should try to attach:
        //   - the parent certificate if it exists, or
        //   - our own certificate that we formed.
        // In either case, we need to ensure that the certificate is still relevant.
        //
        // Note: once we reach a point of potentially propose with our formed upgrade certificate,
        // we will ALWAYS drop it. If we cannot immediately use it for whatever reason, we choose
        // to discard it.
        //
        // It is possible that multiple nodes form separate upgrade certificates for the some
        // upgrade if we are not careful about voting. But this shouldn't bother us: the first
        // leader to propose is the one whose certificate will be used. And if that fails to reach
        // a decide for whatever reason, we may lose our own certificate, but something will likely
        // have gone wrong there anyway.
        let mut upgrade_certificate = parent_leaf
            .upgrade_certificate()
            .or(formed_upgrade_certificate);

        if let Some(cert) = upgrade_certificate.clone() {
            if cert.is_relevant(self.view_number).await.is_err() {
                upgrade_certificate = None;
            }
        }

        let proposal_certificate = view_change_evidence
            .as_ref()
            .filter(|cert| cert.is_valid_for_view(&self.view_number))
            .cloned();

        ensure!(
            commitment_and_metadata.block_view == self.view_number,
            "Cannot propose because our VID payload commitment and metadata is for an older view."
        );

        let version = self.upgrade_lock.version(self.view_number).await?;

        let builder_commitment = commitment_and_metadata.builder_commitment.clone();
        let metadata = commitment_and_metadata.metadata.clone();

        if version >= V::Epochs::VERSION
            && parent_qc.view_number()
                > self
                    .upgrade_lock
                    .upgrade_view()
                    .await
                    .unwrap_or(TYPES::View::new(0))
        {
            let Some(parent_block_number) = parent_qc.data.block_number else {
                tracing::error!("Parent QC does not have a block number. Do not propose.");
                return Ok(());
            };
            if is_epoch_transition(parent_block_number, self.epoch_height)
                && !is_last_block(parent_block_number, self.epoch_height)
            {
                let (empty_payload, empty_metadata) = <TYPES as NodeType>::BlockPayload::empty();
                tracing::info!("Reached end of epoch.");
                ensure!(
                    builder_commitment == empty_payload.builder_commitment(&metadata)
                        && metadata == empty_metadata,
                    "We're trying to propose non empty block in the epoch transition. Do not propose. View number: {}. Parent Block number: {}",
                    self.view_number,
                    parent_block_number,
                );
            }
            if is_epoch_root(parent_block_number, self.epoch_height) {
                ensure!(
                    maybe_state_cert.as_ref().is_some_and(|state_cert| {
                        check_qc_state_cert_correspondence(&parent_qc, state_cert, self.epoch_height)
                    }),
                    "We are proposing with parent epoch root QC but we don't have the corresponding state cert."
                );
            }
        }
        let block_header = if version < V::Marketplace::VERSION {
            TYPES::BlockHeader::new_legacy(
                state.as_ref(),
                self.instance_state.as_ref(),
                &parent_leaf,
                commitment_and_metadata.commitment,
                builder_commitment,
                metadata,
                commitment_and_metadata.fees.first().clone(),
                version,
                *self.view_number,
            )
            .await
            .wrap()
            .context(warn!("Failed to construct legacy block header"))?
        } else {
            TYPES::BlockHeader::new_marketplace(
                state.as_ref(),
                self.instance_state.as_ref(),
                &parent_leaf,
                commitment_and_metadata.commitment,
                commitment_and_metadata.builder_commitment,
                commitment_and_metadata.metadata,
                commitment_and_metadata.fees.to_vec(),
                *self.view_number,
                commitment_and_metadata.auction_result,
                version,
            )
            .await
            .wrap()
            .context(warn!("Failed to construct marketplace block header"))?
        };

        let epoch = option_epoch_from_block_number::<TYPES>(
            version >= V::Epochs::VERSION,
            block_header.block_number(),
            self.epoch_height,
        );

        let epoch_membership = self
            .membership
            .coordinator
            .membership_for_epoch(epoch)
            .await?;
        // Make sure we are the leader for the view and epoch.
        // We might have ended up here because we were in the epoch transition.
        if epoch_membership.leader(self.view_number).await? != self.public_key {
            tracing::warn!(
                "We are not the leader in the epoch for which we are about to propose. Do not send the quorum proposal."
            );
            return Ok(());
        }
        let is_high_qc_for_transition_block = parent_qc
            .data
            .block_number
            .is_some_and(|block_number| is_epoch_transition(block_number, self.epoch_height));
        let next_epoch_qc = if self.upgrade_lock.epochs_enabled(self.view_number).await
            && is_high_qc_for_transition_block
        {
            ensure!(
                maybe_next_epoch_qc
                    .as_ref()
                    .is_some_and(|neqc| neqc.data.leaf_commit == parent_qc.data.leaf_commit),
                "Jusify QC on our proposal is for an epoch transition block \
                    but we don't have the corresponding next epoch QC. Do not propose."
            );
            maybe_next_epoch_qc
        } else {
            None
        };
        let next_drb_result = if is_epoch_transition(block_header.block_number(), self.epoch_height)
        {
            if let Some(epoch_val) = &epoch {
                self.consensus
                    .read()
                    .await
                    .drb_results
                    .results
                    .get(&(*epoch_val + 1))
                    .copied()
            } else {
                None
            }
        } else {
            None
        };

        let proposal = QuorumProposalWrapper {
            proposal: QuorumProposal2 {
                block_header,
                view_number: self.view_number,
                epoch,
                justify_qc: parent_qc,
                next_epoch_justify_qc: next_epoch_qc,
                upgrade_certificate,
                view_change_evidence: proposal_certificate,
                next_drb_result,
                state_cert: maybe_state_cert,
            },
        };

        let proposed_leaf = Leaf2::from_quorum_proposal(&proposal);
        ensure!(
            proposed_leaf.parent_commitment() == parent_leaf.commit(),
            "Proposed leaf parent does not equal high qc"
        );

        let signature =
            TYPES::SignatureKey::sign(&self.private_key, proposed_leaf.commit().as_ref())
                .wrap()
                .context(error!("Failed to compute proposed_leaf.commit()"))?;

        let message = Proposal {
            data: proposal,
            signature,
            _pd: PhantomData,
        };
        tracing::info!(
            "Sending proposal for view {:?}, height {:?}, justify_qc view: {:?}",
            proposed_leaf.view_number(),
            proposed_leaf.height(),
            proposed_leaf.justify_qc().view_number()
        );

        broadcast_event(
            Arc::new(HotShotEvent::QuorumProposalSend(
                message.clone(),
                self.public_key.clone(),
            )),
            &self.sender,
        )
        .await;

        Ok(())
    }
}

impl<TYPES: NodeType, V: Versions> HandleDepOutput for ProposalDependencyHandle<TYPES, V> {
    type Output = Vec<Vec<Vec<Arc<HotShotEvent<TYPES>>>>>;

    #[allow(clippy::no_effect_underscore_binding, clippy::too_many_lines)]
    #[instrument(skip_all, fields(id = self.id, view_number = *self.view_number, latest_proposed_view = *self.latest_proposed_view))]
    async fn handle_dep_result(self, res: Self::Output) {
        let mut commit_and_metadata: Option<CommitmentAndMetadata<TYPES>> = None;
        let mut timeout_certificate = None;
        let mut view_sync_finalize_cert = None;
        let mut vid_share = None;
        let mut parent_qc = None;
        let mut next_epoch_qc = None;
        let mut state_cert = None;
        for event in res.iter().flatten().flatten() {
            match event.as_ref() {
                HotShotEvent::SendPayloadCommitmentAndMetadata(
                    payload_commitment,
                    builder_commitment,
                    metadata,
                    view,
                    fees,
                    auction_result,
                ) => {
                    commit_and_metadata = Some(CommitmentAndMetadata {
                        commitment: *payload_commitment,
                        builder_commitment: builder_commitment.clone(),
                        metadata: metadata.clone(),
                        fees: fees.clone(),
                        block_view: *view,
                        auction_result: auction_result.clone(),
                    });
                },
                HotShotEvent::Qc2Formed(cert) => match cert {
                    either::Right(timeout) => {
                        timeout_certificate = Some(timeout.clone());
                    },
                    either::Left(qc) => {
                        parent_qc = Some(qc.clone());
                    },
                },
                HotShotEvent::EpochRootQcFormed(root_qc) => {
                    parent_qc = Some(root_qc.qc.clone());
                    state_cert = Some(root_qc.state_cert.clone());
                },
                HotShotEvent::ViewSyncFinalizeCertificateRecv(cert) => {
                    view_sync_finalize_cert = Some(cert.clone());
                },
                HotShotEvent::VidDisperseSend(share, _) => {
                    vid_share = Some(share.clone());
                },
                HotShotEvent::NextEpochQc2Formed(either::Left(qc)) => {
                    next_epoch_qc = Some(qc.clone());
                },
                _ => {},
            }
        }

        let Ok(version) = self.upgrade_lock.version(self.view_number).await else {
            tracing::error!(
                "Failed to get version for view {:?}, not proposing",
                self.view_number
            );
            return;
        };

        let mut maybe_epoch = None;
        let proposal_cert = if let Some(view_sync_cert) = view_sync_finalize_cert {
            maybe_epoch = view_sync_cert.data.epoch;
            Some(ViewChangeEvidence2::ViewSync(view_sync_cert))
        } else {
            match timeout_certificate {
                Some(timeout_cert) => {
                    maybe_epoch = timeout_cert.data.epoch;
                    Some(ViewChangeEvidence2::Timeout(timeout_cert))
                },
                None => None,
            }
        };

        let (parent_qc, maybe_next_epoch_qc, maybe_state_cert) = if let Some(qc) = parent_qc {
            if qc
                .data
                .block_number
                .is_some_and(|bn| is_transition_block(bn, self.epoch_height))
                && next_epoch_qc
                    .as_ref()
                    .is_none_or(|neqc| neqc.data.leaf_commit != qc.data.leaf_commit)
            {
                tracing::error!(
                    "We've formed a transition QC but we haven't formed \
                    the corresponding next epoch QC. Do not propose."
                );
                return;
            }
            (qc, next_epoch_qc, state_cert)
        } else if version < V::Epochs::VERSION {
            (self.consensus.read().await.high_qc().clone(), None, None)
        } else if proposal_cert.is_some() {
            // If we have a view change evidence, we need to wait to propose with the transition QC
            if let Ok(Some((qc, next_epoch_qc))) = self.wait_for_transition_qc().await {
                let Some(epoch) = maybe_epoch else {
                    tracing::error!(
                        "No epoch found on view change evidence, but we are in epoch mode"
                    );
                    return;
                };
                if qc
                    .data
                    .block_number
                    .is_some_and(|bn| epoch_from_block_number(bn, self.epoch_height) == *epoch)
                {
                    (qc, Some(next_epoch_qc), None)
                } else {
                    match self.wait_for_highest_qc().await {
                        Ok((qc, maybe_next_epoch_qc, maybe_state_cert)) => {
                            (qc, maybe_next_epoch_qc, maybe_state_cert)
                        },
                        Err(e) => {
                            tracing::error!("Error while waiting for highest QC: {e:?}");
                            return;
                        },
                    }
                }
            } else {
                let Ok((qc, maybe_next_epoch_qc, maybe_state_cert)) =
                    self.wait_for_highest_qc().await
                else {
                    tracing::error!("Error while waiting for highest QC");
                    return;
                };
                if qc.data.block_number.is_some_and(|bn| {
                    is_epoch_transition(bn, self.epoch_height)
                        && !is_last_block(bn, self.epoch_height)
                }) {
                    tracing::error!("High is in transition but we need to propose with transition QC, do nothing");
                    return;
                }
                (qc, maybe_next_epoch_qc, maybe_state_cert)
            }
        } else {
            match self.wait_for_highest_qc().await {
                Ok((qc, maybe_next_epoch_qc, maybe_state_cert)) => {
                    (qc, maybe_next_epoch_qc, maybe_state_cert)
                },
                Err(e) => {
                    tracing::error!("Error while waiting for highest QC: {e:?}");
                    return;
                },
            }
        };

        if commit_and_metadata.is_none() {
            tracing::error!(
                "Somehow completed the proposal dependency task without a commitment and metadata"
            );
            return;
        }

        if vid_share.is_none() {
            tracing::error!("Somehow completed the proposal dependency task without a VID share");
            return;
        }

        if let Err(e) = self
            .publish_proposal(
                commit_and_metadata.unwrap(),
                vid_share.unwrap(),
                proposal_cert,
                self.formed_upgrade_certificate.clone(),
                parent_qc,
                maybe_next_epoch_qc,
                maybe_state_cert,
            )
            .await
        {
            tracing::error!("Failed to publish proposal; error = {e:#}");
        }
    }
}

pub(super) async fn handle_eqc_formed<
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    V: Versions,
>(
    cert_view: TYPES::View,
    leaf_commit: Commitment<Leaf2<TYPES>>,
    block_number: Option<u64>,
    task_state: &mut QuorumProposalTaskState<TYPES, I, V>,
    event_sender: &Sender<Arc<HotShotEvent<TYPES>>>,
) {
    if !task_state.upgrade_lock.epochs_enabled(cert_view).await {
        tracing::debug!("QC2 formed but epochs not enabled. Do nothing");
        return;
    }
    if !block_number.is_some_and(|bn| is_last_block(bn, task_state.epoch_height)) {
        tracing::debug!("We formed QC but not eQC. Do nothing");
        return;
    }

    let Some(current_epoch_qc) = task_state.formed_quorum_certificates.get(&cert_view) else {
        tracing::debug!("We formed the eQC but we don't have the current epoch QC at all.");
        return;
    };
    if current_epoch_qc.view_number() != cert_view
        || current_epoch_qc.data.leaf_commit != leaf_commit
    {
        tracing::debug!("We haven't yet formed the eQC. Do nothing");
        return;
    }
    let Some(next_epoch_qc) = task_state
        .formed_next_epoch_quorum_certificates
        .get(&cert_view)
    else {
        tracing::debug!("We formed the eQC but we don't have the next epoch eQC at all.");
        return;
    };
    if current_epoch_qc.view_number() != cert_view || current_epoch_qc.data != *next_epoch_qc.data {
        tracing::debug!(
            "We formed the eQC but the current and next epoch QCs do not correspond to each other."
        );
        return;
    }
    let current_epoch_qc_clone = current_epoch_qc.clone();

    let mut consensus_writer = task_state.consensus.write().await;
    let _ = consensus_writer.update_high_qc(current_epoch_qc_clone.clone());
    let _ = consensus_writer.update_next_epoch_high_qc(next_epoch_qc.clone());
    drop(consensus_writer);

    task_state.formed_quorum_certificates =
        task_state.formed_quorum_certificates.split_off(&cert_view);
    task_state.formed_next_epoch_quorum_certificates = task_state
        .formed_next_epoch_quorum_certificates
        .split_off(&cert_view);

    broadcast_event(
        Arc::new(HotShotEvent::ExtendedQc2Formed(current_epoch_qc_clone)),
        event_sender,
    )
    .await;
}
