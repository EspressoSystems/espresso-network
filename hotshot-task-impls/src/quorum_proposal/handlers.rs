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
use async_lock::RwLock;
use committable::{Commitment, Committable};
use hotshot_task::dependency_task::HandleDepOutput;
use hotshot_types::{
    consensus::{CommitmentAndMetadata, OuterConsensus},
    data::{Leaf2, QuorumProposal2, QuorumProposalWrapper, VidDisperse, ViewChangeEvidence2},
    epoch_membership::EpochMembership,
    message::Proposal,
    simple_certificate::{QuorumCertificate2, UpgradeCertificate},
    traits::{
        block_contents::BlockHeader,
        node_implementation::{NodeImplementation, NodeType},
        signature_key::SignatureKey,
    },
    utils::{is_last_block_in_epoch, option_epoch_from_block_number},
    vote::HasViewNumber,
};
use hotshot_utils::anytrace::*;
use tracing::instrument;
use vbs::version::StaticVersionType;

use crate::{
    events::HotShotEvent,
    helpers::{
        broadcast_event, parent_leaf_and_state, validate_qc_and_next_epoch_qc,
        wait_for_next_epoch_qc,
    },
    quorum_proposal::{QuorumProposalTaskState, UpgradeLock, Versions},
};

/// Proposal dependency types. These types represent events that precipitate a proposal.
#[derive(PartialEq, Debug)]
pub(crate) enum ProposalDependency {
    /// For the `SendPayloadCommitmentAndMetadata` event.
    PayloadAndMetadata,

    /// For the `Qc2Formed` event.
    Qc,

    /// For the `ViewSyncFinalizeCertificateRecv` event.
    ViewSyncCert,

    /// For the `Qc2Formed` event timeout branch.
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
    ) -> Option<QuorumCertificate2<TYPES>> {
        while let Ok(event) = rx.recv_direct().await {
            if let HotShotEvent::HighQcRecv(qc, maybe_next_epoch_qc, _sender) = event.as_ref() {
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
                    return Some(qc.clone());
                }
            }
        }
        None
    }
    /// Waits for the configured timeout for nodes to send HighQc messages to us.  We'll
    /// then propose with the highest QC from among these proposals.
    async fn wait_for_highest_qc(&self) -> Result<QuorumCertificate2<TYPES>> {
        tracing::debug!("waiting for QC");
        // If we haven't upgraded to Hotstuff 2 just return the high qc right away
        ensure!(
            self.upgrade_lock.epochs_enabled(self.view_number).await,
            error!("Epochs are not enabled yet we tried to wait for Highest QC.")
        );

        let mut highest_qc = self.consensus.read().await.high_qc().clone();

        let wait_duration = Duration::from_millis(self.timeout / 2);

        let mut rx = self.receiver.clone();

        // drain any qc off the queue
        while let Ok(event) = rx.try_recv() {
            if let HotShotEvent::HighQcRecv(qc, maybe_next_epoch_qc, _sender) = event.as_ref() {
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
                    && qc.view_number() > highest_qc.view_number()
                {
                    highest_qc = qc.clone();
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
            let Ok(maybe_qc) =
                tokio::time::timeout(time_left, self.wait_for_qc_event(rx.clone())).await
            else {
                tracing::info!("Some nodes did not respond with their HighQc in time. Continuing with the highest QC that we received: {highest_qc:?}");
                return Ok(highest_qc);
            };
            let Some(qc) = maybe_qc else {
                continue;
            };
            if qc.view_number() > highest_qc.view_number() {
                highest_qc = qc;
            }
        }
        Ok(highest_qc.clone())
    }
    /// Publishes a proposal given the [`CommitmentAndMetadata`], [`VidDisperse`]
    /// and high qc [`hotshot_types::simple_certificate::QuorumCertificate`],
    /// with optional [`ViewChangeEvidence`].
    #[instrument(skip_all, fields(id = self.id, view_number = *self.view_number, latest_proposed_view = *self.latest_proposed_view))]
    async fn publish_proposal(
        &self,
        commitment_and_metadata: CommitmentAndMetadata<TYPES>,
        _vid_share: Proposal<TYPES, VidDisperse<TYPES>>,
        view_change_evidence: Option<ViewChangeEvidence2<TYPES>>,
        formed_upgrade_certificate: Option<UpgradeCertificate<TYPES>>,
        decided_upgrade_certificate: Arc<RwLock<Option<UpgradeCertificate<TYPES>>>>,
        parent_qc: QuorumCertificate2<TYPES>,
    ) -> Result<()> {
        let (parent_leaf, state) = parent_leaf_and_state(
            &self.sender,
            &self.receiver,
            self.membership.coordinator.clone(),
            self.public_key.clone(),
            self.private_key.clone(),
            OuterConsensus::new(Arc::clone(&self.consensus.inner_consensus)),
            &self.upgrade_lock,
            parent_qc.view_number(),
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
            if cert
                .is_relevant(self.view_number, Arc::clone(&decided_upgrade_certificate))
                .await
                .is_err()
            {
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

        let block_header = if version >= V::Epochs::VERSION
            && self.consensus.read().await.is_qc_forming_eqc(&parent_qc)
        {
            tracing::info!("Reached end of epoch. Proposing the same block again to form an eQC.");
            let block_header = parent_leaf.block_header().clone();
            tracing::debug!(
                "Proposing block no. {} to form the eQC.",
                block_header.block_number()
            );
            block_header
        } else if version < V::Marketplace::VERSION {
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
        let is_high_qc_for_last_block = if let Some(block_number) = parent_qc.data.block_number {
            is_last_block_in_epoch(block_number, self.epoch_height)
        } else {
            false
        };
        let next_epoch_qc = if self.upgrade_lock.epochs_enabled(self.view_number).await
            && is_high_qc_for_last_block
        {
            wait_for_next_epoch_qc(
                &parent_qc,
                &self.consensus,
                self.timeout,
                self.view_start_time,
                &self.receiver,
            )
            .await
        } else {
            None
        };
        let next_drb_result =
            if is_last_block_in_epoch(block_header.block_number(), self.epoch_height) {
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
        tracing::debug!(
            "Sending proposal for view {:?}",
            proposed_leaf.view_number(),
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
    async fn handle_dep_result(self, res: Self::Output) {
        let mut commit_and_metadata: Option<CommitmentAndMetadata<TYPES>> = None;
        let mut timeout_certificate = None;
        let mut view_sync_finalize_cert = None;
        let mut vid_share = None;
        let mut parent_qc = None;
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
                HotShotEvent::ViewSyncFinalizeCertificateRecv(cert) => {
                    view_sync_finalize_cert = Some(cert.clone());
                },
                HotShotEvent::VidDisperseSend(share, _) => {
                    vid_share = Some(share.clone());
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
        let parent_qc = if let Some(qc) = parent_qc {
            qc
        } else if version < V::Epochs::VERSION {
            self.consensus.read().await.high_qc().clone()
        } else {
            match self.wait_for_highest_qc().await {
                Ok(qc) => qc,
                Err(e) => {
                    tracing::error!("Error while waiting for highest QC: {:?}", e);
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

        let proposal_cert = if let Some(view_sync_cert) = view_sync_finalize_cert {
            Some(ViewChangeEvidence2::ViewSync(view_sync_cert))
        } else {
            timeout_certificate.map(ViewChangeEvidence2::Timeout)
        };

        if let Err(e) = self
            .publish_proposal(
                commit_and_metadata.unwrap(),
                vid_share.unwrap(),
                proposal_cert,
                self.formed_upgrade_certificate.clone(),
                Arc::clone(&self.upgrade_lock.decided_upgrade_certificate),
                parent_qc,
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
    task_state: &QuorumProposalTaskState<TYPES, I, V>,
    event_sender: &Sender<Arc<HotShotEvent<TYPES>>>,
) {
    if !task_state.upgrade_lock.epochs_enabled(cert_view).await {
        tracing::debug!("QC2 formed but epochs not enabled. Do nothing");
        return;
    }
    if !task_state
        .consensus
        .read()
        .await
        .is_leaf_extended(leaf_commit)
    {
        tracing::debug!("We formed QC but not eQC. Do nothing");
        return;
    }

    let consensus_reader = task_state.consensus.read().await;
    let current_epoch_qc = consensus_reader.high_qc();
    let Some(next_epoch_qc) = consensus_reader.next_epoch_high_qc() else {
        tracing::debug!("We formed the eQC but we don't have the next epoch eQC at all.");
        return;
    };
    if current_epoch_qc.view_number() != next_epoch_qc.view_number()
        || current_epoch_qc.data != *next_epoch_qc.data
    {
        tracing::debug!(
            "We formed the eQC but the current and next epoch QCs do not correspond to each other."
        );
        return;
    }
    let current_epoch_qc_clone = current_epoch_qc.clone();
    drop(consensus_reader);

    broadcast_event(
        Arc::new(HotShotEvent::ExtendedQc2Formed(current_epoch_qc_clone)),
        event_sender,
    )
    .await;
}
