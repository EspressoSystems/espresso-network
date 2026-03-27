use std::{
    cmp::max,
    collections::{BTreeMap, BTreeSet},
    marker::PhantomData,
};

use committable::{Commitment, Committable};
use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{
        EpochNumber, Leaf2, VidCommitment, VidCommitment2, VidDisperse2, VidDisperseShare2,
        ViewChangeEvidence2, ViewNumber, vid_disperse::vid_total_weight,
    },
    drb::DrbInput,
    epoch_membership::EpochMembershipCoordinator,
    message::Proposal as SignedProposal,
    simple_certificate::{TimeoutCertificate2, ViewSyncFinalizeCertificate2},
    simple_vote::{HasEpoch, QuorumData2, SimpleVote, TimeoutVote2},
    stake_table::StakeTableEntries,
    traits::{
        block_contents::BlockHeader, node_implementation::NodeType, signature_key::SignatureKey,
    },
    vote::{self, Certificate, HasViewNumber},
};
use tracing::{debug, instrument, warn};

use crate::{
    block::BlockAndHeaderRequest,
    helpers::{proposal_commitment, upgrade_lock},
    message::{
        Certificate1, Certificate2, CheckpointData, CheckpointVote, Proposal, ProposalMessage,
        Vote1, Vote2, Vote2Data,
    },
    outbox::Outbox,
    state::{StateRequest, StateResponse},
};

#[derive(Eq, PartialEq, Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum ConsensusInput<T: NodeType> {
    BlockBuilt {
        view: ViewNumber,
        epoch: EpochNumber,
        payload: T::BlockPayload,
        metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
    },
    BlockReconstructed(ViewNumber, VidCommitment2),
    Certificate1(Certificate1<T>),
    Certificate2(Certificate2<T>),
    HeaderCreated(ViewNumber, T::BlockHeader),
    Proposal(ProposalMessage<T>),
    StateValidated(StateResponse<T>),
    StateValidationFailed(StateResponse<T>),
    Timeout(ViewNumber),
    TimeoutCertificate(TimeoutCertificate2<T>),
    VidDisperseCreated(ViewNumber, VidDisperse2<T>),
    ViewSyncCertificate(ViewSyncFinalizeCertificate2<T>),
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum ConsensusOutput<T: NodeType> {
    RequestBlockAndHeader(BlockAndHeaderRequest<T>),
    RequestDRB(DrbInput),
    RequestProposal(ViewNumber, Commitment<Leaf2<T>>),
    RequestState(StateRequest<T>),
    SendProposal(SignedProposal<T, Proposal<T>>, VidDisperse2<T>),
    SendCheckpointVote(CheckpointVote<T>),
    SendTimeoutVote(TimeoutVote2<T>),
    SendVote1(Vote1<T>),
    SendVote2(Vote2<T>),
    RequestVidDisperse {
        view: ViewNumber,
        epoch: EpochNumber,
        payload: T::BlockPayload,
        metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
    },
    Certificate1Formed(Certificate1<T>),
    Certificate2Formed(Certificate2<T>),
    LeafDecided(Vec<Leaf2<T>>),
    LockUpdated(Certificate2<T>),
    TimeoutCertificateReceived(TimeoutCertificate2<T>),
    ViewChanged(ViewNumber, EpochNumber),
    ViewSyncCertificateReceived(ViewSyncFinalizeCertificate2<T>),
}

pub struct Consensus<T: NodeType> {
    proposals: BTreeMap<ViewNumber, Proposal<T>>,
    vid_shares: BTreeMap<ViewNumber, VidDisperseShare2<T>>,
    states_verified: BTreeMap<ViewNumber, Commitment<Leaf2<T>>>,
    blocks_reconstructed: BTreeMap<ViewNumber, VidCommitment2>,
    blocks: BTreeMap<ViewNumber, T::BlockPayload>,
    vid_disperses: BTreeMap<ViewNumber, VidDisperse2<T>>,
    certs: BTreeMap<ViewNumber, Certificate1<T>>,
    certs2: BTreeMap<ViewNumber, Certificate2<T>>,
    timeout_certs: BTreeMap<ViewNumber, TimeoutCertificate2<T>>,
    view_sync_certs: BTreeMap<ViewNumber, ViewSyncFinalizeCertificate2<T>>,
    locked_cert: Option<Certificate1<T>>,
    headers: BTreeMap<ViewNumber, T::BlockHeader>,
    last_decided_view: ViewNumber,

    voted_1_views: BTreeSet<ViewNumber>,
    voted_2_views: BTreeSet<ViewNumber>,

    timeout_view: ViewNumber,

    // TODO: We need a next epoch stake table to handle the transition
    // And a way to set these stake tables, probably an event from coordinator
    stake_table_coordinator: EpochMembershipCoordinator<T>,

    public_key: T::SignatureKey,
    private_key: <T::SignatureKey as SignatureKey>::PrivateKey,

    garbage_collection_interval: u64,
}

/// Protocol flow directive.
enum Protocol {
    /// Stop with further protocol steps.
    Abort,
    /// Continue with protocol.
    Continue,
}

impl<T: NodeType> Consensus<T> {
    pub fn new(
        membership_coordinator: EpochMembershipCoordinator<T>,
        public_key: T::SignatureKey,
        private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
    ) -> Self {
        Self {
            proposals: BTreeMap::new(),
            vid_disperses: BTreeMap::new(),
            blocks: BTreeMap::new(),
            states_verified: BTreeMap::new(),
            blocks_reconstructed: BTreeMap::new(),
            certs: BTreeMap::new(),
            certs2: BTreeMap::new(),
            timeout_certs: BTreeMap::new(),
            view_sync_certs: BTreeMap::new(),
            locked_cert: None,
            last_decided_view: ViewNumber::genesis(),
            headers: BTreeMap::new(),
            public_key,
            timeout_view: ViewNumber::genesis(),
            stake_table_coordinator: membership_coordinator,
            voted_1_views: BTreeSet::new(),
            voted_2_views: BTreeSet::new(),
            private_key,
            vid_shares: BTreeMap::new(),
            // TODO: make this configurable or Constant
            garbage_collection_interval: 100,
        }
    }

    /// Apply consensus to the given input and collect protocol outputs.
    #[instrument(level = "debug", skip_all, fields(view = %input.view_number()))]
    pub async fn apply(
        &mut self,
        input: ConsensusInput<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        let view = input.view_number();
        if view <= self.timeout_view {
            return;
        }
        let proto = match input {
            ConsensusInput::Proposal(proposal) => self.handle_proposal(proposal, outbox).await,
            ConsensusInput::Certificate1(certificate) => {
                self.handle_certificate1(certificate, outbox).await
            },
            ConsensusInput::Certificate2(certificate) => {
                self.handle_certificate2(certificate).await
            },
            ConsensusInput::TimeoutCertificate(certificate) => {
                self.handle_timeout_certificate(certificate, outbox).await
            },
            ConsensusInput::ViewSyncCertificate(certificate) => {
                self.handle_view_sync_certificate(certificate, outbox).await
            },
            ConsensusInput::BlockReconstructed(view, vid_commitment) => {
                self.blocks_reconstructed.insert(view, vid_commitment);
                Protocol::Continue
            },
            ConsensusInput::StateValidated(state_response) => {
                self.states_verified
                    .insert(state_response.view, state_response.commitment);
                Protocol::Continue
            },
            ConsensusInput::HeaderCreated(view, header) => {
                self.headers.insert(view, header);
                Protocol::Continue
            },
            ConsensusInput::StateValidationFailed(state_response) => {
                if let Some(proposal) = self.proposals.remove(&state_response.view)
                    && proposal_commitment(&proposal) != state_response.commitment
                {
                    // Proposal we stored didn't match the failed state verification, put it back
                    self.proposals.insert(state_response.view, proposal);
                    return;
                }
                self.vid_shares.remove(&state_response.view);
                return;
            },
            ConsensusInput::Timeout(view) => {
                self.handle_timeout(view);
                // we are done after timeout, don't try to vote, decide, or propose
                return;
            },
            ConsensusInput::BlockBuilt {
                view,
                epoch,
                payload,
                metadata,
            } => {
                outbox.push_back(ConsensusOutput::RequestVidDisperse {
                    view,
                    epoch,
                    payload: payload.clone(),
                    metadata,
                });
                self.blocks.insert(view, payload);
                Protocol::Continue
            },
            ConsensusInput::VidDisperseCreated(view, vid_disperse) => {
                self.vid_disperses.insert(view, vid_disperse);
                Protocol::Continue
            },
        };

        if matches!(proto, Protocol::Abort) {
            debug!("aborting protocol");
            return;
        }

        self.maybe_vote_1(view, outbox);
        self.maybe_vote_2_and_update_lock(view, outbox);
        self.maybe_decide(view, outbox);
        self.maybe_propose(view, outbox).await;
        // An event from the current view or the previous view can trigger a propose
        self.maybe_propose(view + 1, outbox).await;
    }

    pub fn gc(&mut self, view: ViewNumber, _epoch: EpochNumber) {
        self.states_verified = self.states_verified.split_off(&view);
        self.blocks_reconstructed = self.blocks_reconstructed.split_off(&view);
        self.blocks = self.blocks.split_off(&view);
        self.vid_disperses = self.vid_disperses.split_off(&view);
        self.certs = self.certs.split_off(&view);
        self.certs2 = self.certs2.split_off(&view);
        self.timeout_certs = self.timeout_certs.split_off(&view);
        self.view_sync_certs = self.view_sync_certs.split_off(&view);
        self.locked_cert = self
            .locked_cert
            .take()
            .filter(|cert| cert.view_number() > view);
        self.headers = self.headers.split_off(&view);
        self.voted_1_views = self.voted_1_views.split_off(&view);
        self.voted_2_views = self.voted_2_views.split_off(&view);
        self.last_decided_view = self.last_decided_view.max(view);
    }

    #[instrument(level = "debug", skip_all)]
    async fn handle_proposal(
        &mut self,
        proposal: ProposalMessage<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let view = proposal.view_number();

        // TODO: This signature check is slow (> 1ms).  We should consider
        // if this should be done off the main thread.
        if !self.validate_proposal_signature(&proposal.proposal).await {
            warn!(%view, "invalid proposal signature");
            return Protocol::Abort;
        }

        let vid_share = proposal.vid_share;
        let proposal = proposal.proposal.data;
        let epoch = proposal.epoch;

        if !vid_matches_proposal(&vid_share, &proposal) {
            debug!("vid share does not match proposal");
            return Protocol::Abort;
        }

        if !vid_share.is_consistent() {
            debug!("vid share is not consistent");
            return Protocol::Abort;
        }

        if !self.verify_vid_share(&vid_share, epoch).await {
            debug!("vid share not verified");
            return Protocol::Abort;
        }

        if !self.is_safe(&proposal) {
            debug!("proposal not safe");
            return Protocol::Abort;
        }

        let payload_size = vid_share.payload_byte_len();

        self.proposals.insert(view, proposal.clone());
        self.vid_shares.insert(view, vid_share);

        outbox.push_back(ConsensusOutput::RequestState(StateRequest {
            view: proposal.view_number(),
            parent_view: proposal.view_number().saturating_sub(1).into(),
            epoch,
            block: proposal.block_header.block_number().into(),
            proposal: proposal.clone(),
            parent_commitment: proposal.justify_qc.data().leaf_commit,
            payload_size,
        }));

        if self.is_leader(view + 1, epoch).await {
            outbox.push_back(ConsensusOutput::RequestBlockAndHeader(
                BlockAndHeaderRequest {
                    view: view + 1,
                    epoch,
                    parent_proposal: proposal,
                },
            ));
        }

        Protocol::Continue
    }

    #[instrument(level = "debug", skip_all)]
    async fn handle_certificate1(
        &mut self,
        certificate: Certificate1<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let view = certificate.view_number();
        let Some(certificate_epoch) = certificate.epoch() else {
            warn!(%view, "certificate1 has no epoch number");
            return Protocol::Abort;
        };
        // TODO: This signature check is slow (> 1ms).  We should consider
        // if this should be done off the main thread.
        if !self.verify_cert(&certificate, certificate_epoch).await {
            warn!(%view, "certificate1 not verified");
            return Protocol::Abort;
        }
        outbox.push_back(ConsensusOutput::ViewChanged(view + 1, certificate_epoch));
        self.certs.insert(view, certificate);
        Protocol::Continue
    }

    #[instrument(level = "debug", skip_all)]
    async fn handle_certificate2(&mut self, certificate: Certificate2<T>) -> Protocol {
        let view = certificate.view_number();
        let Some(certificate_epoch) = certificate.epoch() else {
            warn!(%view, "certificate2 has no epoch number");
            return Protocol::Abort;
        };
        // TODO: This signature check is slow (> 1ms).  We should consider
        // if this should be done off the main thread.
        if !self.verify_cert(&certificate, certificate_epoch).await {
            warn!(%view, "certificate2 not verified");
            return Protocol::Abort;
        }
        self.certs2.insert(view, certificate);
        Protocol::Continue
    }

    #[instrument(level = "debug", skip_all)]
    async fn handle_timeout_certificate(
        &mut self,
        certificate: TimeoutCertificate2<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let view = certificate.view_number() + 1;
        let Some(epoch) = certificate.epoch() else {
            warn!(view = %certificate.view_number(), "timeout certificate has no epoch number");
            return Protocol::Abort;
        };
        self.timeout_certs.insert(view, certificate.clone());
        outbox.push_back(ConsensusOutput::TimeoutCertificateReceived(certificate));
        outbox.push_back(ConsensusOutput::ViewChanged(view, epoch));
        if !self.is_leader(view, epoch).await {
            debug!(%epoch, "not leader");
            return Protocol::Abort;
        }

        // If we are the leader of the next view, try to get a block to propose
        // after forming the TC
        let Some(locked_view) = self.locked_cert.as_ref().map(|cert| cert.view_number()) else {
            debug!("locked certificate not available");
            return Protocol::Abort;
        };
        let Some(proposal) = self.proposals.get(&locked_view) else {
            debug!(%locked_view, "proposal not available");
            return Protocol::Abort;
        };
        outbox.push_back(ConsensusOutput::RequestBlockAndHeader(
            BlockAndHeaderRequest {
                view,
                epoch,
                parent_proposal: proposal.clone(),
            },
        ));
        Protocol::Continue
    }

    #[instrument(level = "debug", skip_all)]
    async fn handle_view_sync_certificate(
        &mut self,
        certificate: ViewSyncFinalizeCertificate2<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let view = certificate.view_number();
        let Some(epoch) = certificate.epoch() else {
            warn!(%view, "view-sync certificate has no epoch number");
            return Protocol::Abort;
        };
        self.view_sync_certs.insert(view, certificate.clone());
        outbox.push_back(ConsensusOutput::ViewSyncCertificateReceived(certificate));
        outbox.push_back(ConsensusOutput::ViewChanged(view, epoch));
        if !self.is_leader(view, epoch).await {
            debug!(%epoch, "not leader");
            return Protocol::Abort;
        }
        let Some(locked_view) = self.locked_cert.as_ref().map(|cert| cert.view_number()) else {
            debug!("locked qc not available");
            return Protocol::Abort;
        };
        let Some(proposal) = self.proposals.get(&locked_view) else {
            debug!(%locked_view, "proposal not available");
            return Protocol::Abort;
        };
        outbox.push_back(ConsensusOutput::RequestBlockAndHeader(
            BlockAndHeaderRequest {
                view,
                epoch,
                parent_proposal: proposal.clone(),
            },
        ));
        Protocol::Continue
    }

    #[instrument(level = "debug", skip_all)]
    fn handle_timeout(&mut self, view: ViewNumber) {
        self.timeout_view = view;
        // TODO: clear_view(view);
    }

    #[instrument(level = "debug", skip(self, outbox))]
    async fn maybe_propose(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        let is_after_timeout =
            self.view_sync_certs.contains_key(&view) || self.timeout_certs.contains_key(&view);
        let parent_cert = if is_after_timeout {
            let Some(cert) = &self.locked_cert else {
                debug!("no locked qc");
                return;
            };
            cert
        } else {
            let Some(cert) = self.certs.get(&ViewNumber::from(view.saturating_sub(1))) else {
                debug!("no parent certificate");
                return;
            };
            cert
        };
        let parent_view = parent_cert.view_number();
        let Some(proposal) = self.proposals.get(&parent_view) else {
            debug!(parent = %parent_view, "no proposal for parent view");
            return;
        };
        let proposal_epoch = proposal.epoch;
        if !self.is_leader(view, proposal_epoch).await {
            debug!(epoch = %proposal_epoch, "not a leader of proposal");
            return;
        }

        let Some(header) = self.headers.get(&view) else {
            debug!("no block header");
            return;
        };
        if !self.blocks.contains_key(&view) {
            debug!("no block");
            return;
        };
        let Some(vid_disperse) = self.vid_disperses.get(&view) else {
            debug!("no vid disperse");
            return;
        };

        // TODO: Handle epoch change and properly set next epoch qc drb result and state cert
        let mut proposal = Proposal::<T> {
            block_header: header.clone(),
            view_number: view,
            epoch: proposal.epoch,
            justify_qc: parent_cert.clone(),
            next_epoch_justify_qc: None,
            upgrade_certificate: None,
            view_change_evidence: None,
            next_drb_result: None,
            state_cert: None,
        };

        // add View Change Evidence if we are after timeout
        if is_after_timeout {
            if let Some(view_sync_cert) = self.view_sync_certs.get(&view) {
                proposal.view_change_evidence =
                    Some(ViewChangeEvidence2::ViewSync(view_sync_cert.clone()));
            } else if let Some(timeout_cert) = self.timeout_certs.get(&view) {
                proposal.view_change_evidence =
                    Some(ViewChangeEvidence2::Timeout(timeout_cert.clone()));
            } else {
                warn!(%view, "after timeout, but no view-sync nor timeout certificate");
                return;
            }
        }

        // TODO: Handle epoch change here

        // Sign the proposal
        let proposed_leaf: Leaf2<T> = proposal.clone().into();
        let signature =
            match T::SignatureKey::sign(&self.private_key, proposed_leaf.commit().as_ref()) {
                Ok(sig) => sig,
                Err(err) => {
                    warn!(%view, %err, "failed to sign proposal");
                    return;
                },
            };

        let message = SignedProposal {
            data: proposal,
            signature,
            _pd: PhantomData,
        };

        outbox.push_back(ConsensusOutput::SendProposal(message, vid_disperse.clone()));
    }

    #[instrument(level = "debug", skip_all)]
    fn maybe_decide(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        if view <= self.last_decided_view {
            return;
        }
        let Some(cert2) = self.certs2.get(&view) else {
            debug!("cert2 not available");
            return;
        };
        let Some(proposal) = self.proposals.get(&view) else {
            debug!("proposal not available");
            return;
        };
        let proposal_commit = proposal_commitment(proposal);
        if cert2.data.leaf_commit != proposal_commit {
            debug!("cert2 commitment does not match proposal commitment");
            return;
        }
        // we have a second certificate, and matching proposal, it is decided.
        let leaf: Leaf2<T> = proposal.clone().into();
        self.last_decided_view = max(self.last_decided_view, leaf.view_number());
        let mut gc = None;
        if leaf.block_header().block_number() % self.garbage_collection_interval == 0 {
            gc = Some((leaf.view_number(), leaf.justify_qc().epoch()));
        }
        let mut decided = vec![leaf];

        let mut parent_view = proposal.justify_qc.view_number();
        let mut parent_commit = proposal.justify_qc.data.leaf_commit;

        while let Some(proposal) = self.proposals.get(&parent_view) {
            let proposal_commit = proposal_commitment(proposal);
            if proposal_commit != parent_commit {
                break;
            }
            let leaf: Leaf2<T> = proposal.clone().into();
            if gc.is_none()
                && leaf.block_header().block_number() % self.garbage_collection_interval == 0
            {
                gc = Some((leaf.view_number(), leaf.justify_qc().epoch()));
            }
            decided.push(leaf);
            parent_view = proposal.justify_qc.view_number();
            parent_commit = proposal.justify_qc.data.leaf_commit;
        }
        outbox.push_back(ConsensusOutput::LeafDecided(decided));
        if let Some(gc) = gc {
            let gc_data = CheckpointData {
                view: gc.0,
                epoch: gc.1.unwrap_or_default(),
            };
            let vote = match SimpleVote::create_signed_vote(
                gc_data,
                view,
                &self.public_key,
                &self.private_key,
                &upgrade_lock::<T>(),
            ) {
                Ok(vote) => vote,
                Err(err) => {
                    warn!(%view, %err, "failed to create signed checkpoint vote");
                    return;
                },
            };
            outbox.push_back(ConsensusOutput::SendCheckpointVote(vote));
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn maybe_vote_1(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        if self.voted_1_views.contains(&view) {
            return;
        }
        let Some(state_commitment) = self.states_verified.get(&view) else {
            debug!("state commitment not available");
            return;
        };
        let Some(proposal) = self.proposals.get(&view) else {
            debug!("proposal not available");
            return;
        };
        let Some(vid_share) = self.vid_shares.get(&view) else {
            debug!("vid share not available");
            return;
        };

        // Verify parent chain unless justify_qc is the genesis QC
        let parent_view = proposal.justify_qc.view_number();

        // We don't need the genesis block to be reconstructed or verified
        // or the genesis qc to be verified
        if parent_view != ViewNumber::genesis() {
            // Verify we have the block for the QC on this commitment
            let Some(block_commitment) = self.blocks_reconstructed.get(&parent_view) else {
                debug!(%parent_view, "block commitment not available");
                return;
            };
            let Some(prev_proposal) = self.proposals.get(&parent_view) else {
                debug!(%parent_view, "proposal not available");
                return;
            };
            let VidCommitment::V2(prev_block_commitment) =
                prev_proposal.block_header.payload_commitment()
            else {
                warn! {
                    %view,
                    %parent_view,
                    "prev. proposal payload commitment is not a V2 VID commitment"
                }
                return;
            };
            if block_commitment != &prev_block_commitment {
                debug!(%parent_view, "parent block commitment does not match prev. block commitment");
                return;
            }

            if proposal.justify_qc.data().leaf_commit != proposal_commitment(prev_proposal) {
                debug!(%parent_view, "justify qc commitment does not match proposal commitment");
                return;
            }
        }

        let proposal_commit = proposal_commitment(proposal);

        // Verify the state commitment matches the proposal
        if state_commitment != &proposal_commit {
            debug!("state commitment does not match proposal commitment");
            return;
        }

        let inner_vote = match SimpleVote::create_signed_vote(
            QuorumData2 {
                leaf_commit: proposal_commit,
                epoch: proposal.epoch(),
                block_number: Some(proposal.block_header.block_number()),
            },
            view,
            &self.public_key,
            &self.private_key,
            &upgrade_lock::<T>(),
        ) {
            Ok(vote) => vote,
            Err(err) => {
                warn!(%view, %err, "failed to created signed vote for proposal");
                return;
            },
        };
        let vote = Vote1 {
            vote: inner_vote,
            vid_share: vid_share.clone(),
        };
        outbox.push_back(ConsensusOutput::SendVote1(vote));
        self.voted_1_views.insert(view);
    }

    #[instrument(level = "debug", skip_all)]
    fn maybe_vote_2_and_update_lock(
        &mut self,
        view: ViewNumber,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        if self.voted_2_views.contains(&view) {
            return;
        }
        let Some(reconstructed_block_commitment) = self.blocks_reconstructed.get(&view) else {
            debug!("reconstructed block commitment not available");
            return;
        };
        let Some(cert1) = self.certs.get(&view) else {
            debug!("cert1 not available");
            return;
        };
        let Some(proposal) = self.proposals.get(&view) else {
            debug!("proposal not available");
            return;
        };
        let proposal_epoch = proposal.epoch;

        let proposal_commit = proposal_commitment(proposal);

        // The certificate must match the proposal
        if cert1.data.leaf_commit != proposal_commit {
            warn!(%view, "cert1 commitment does not match proposal commitment");
            return;
        }
        // The proposal block commitment must match the reconstructed block commitment
        let VidCommitment::V2(proposal_block_commitment) =
            proposal.block_header.payload_commitment()
        else {
            warn!(%view, "proposal payload commitment is not a V2 VID commitment");
            return;
        };
        if &proposal_block_commitment != reconstructed_block_commitment {
            warn!(%view, "proposal commitment does not match reconstructed block commitment");
            return;
        }

        // We have a valid certificate, proposal, and reconstructed block
        // We can now update the lock and vote
        if self
            .locked_cert
            .as_mut()
            .is_none_or(|locked_cert| locked_cert.view_number() < cert1.view_number())
        {
            self.locked_cert = Some(cert1.clone());
        }

        let vote = match SimpleVote::create_signed_vote(
            Vote2Data {
                leaf_commit: proposal_commit,
                epoch: proposal_epoch,
                block_number: proposal.block_header.block_number(),
            },
            view,
            &self.public_key,
            &self.private_key,
            &upgrade_lock::<T>(),
        ) {
            Ok(vote) => vote,
            Err(err) => {
                warn!(%view, %err, "failed to created signed vote2");
                return;
            },
        };
        outbox.push_back(ConsensusOutput::SendVote2(vote));
        self.voted_2_views.insert(view);
    }

    #[instrument(level = "trace", skip(self, share))]
    async fn verify_vid_share(&self, share: &VidDisperseShare2<T>, epoch: EpochNumber) -> bool {
        match self
            .stake_table_coordinator
            .membership_for_epoch(Some(epoch))
            .await
        {
            Ok(stake_table) => {
                let total_weight = vid_total_weight(&stake_table.stake_table().await, Some(epoch));
                share.verify(total_weight)
            },
            Err(err) => {
                warn!(%epoch, %err, "failed to get membership for epoch");
                false
            },
        }
    }

    #[instrument(level = "trace", skip_all)]
    fn is_safe(&self, proposal: &Proposal<T>) -> bool {
        let Some(locked_cert) = self.locked_cert.as_ref() else {
            // Locked certificate is not set which means it is at genesis
            debug!("at genesis");
            return true;
        };
        let liveness_check = proposal.justify_qc.view_number() > locked_cert.view_number();
        let parent_commit = match proposal.justify_qc.data_commitment(&upgrade_lock::<T>()) {
            Ok(c) => c,
            Err(err) => {
                warn!(%err, "failed to compute justify qc data commitment");
                return false;
            },
        };
        let locked_commit = match locked_cert.data_commitment(&upgrade_lock::<T>()) {
            Ok(c) => c,
            Err(err) => {
                warn!(%err, "failed to compute locked certificate data");
                return false;
            },
        };

        let safety_check = parent_commit == locked_commit;

        safety_check || liveness_check
    }

    #[instrument(level = "trace", skip_all)]
    async fn verify_cert<A, C>(&self, cert: &C, epoch: EpochNumber) -> bool
    where
        C: vote::Certificate<T, A>,
    {
        match self
            .stake_table_coordinator
            .membership_for_epoch(Some(epoch))
            .await
        {
            Ok(stake_table) => {
                let entries = StakeTableEntries::<T>::from(stake_table.stake_table().await).0;
                let threshold = stake_table.success_threshold().await;
                match cert.is_valid_cert(&entries, threshold, &upgrade_lock::<T>()) {
                    Ok(()) => true,
                    Err(err) => {
                        warn!(%epoch, %err, "invalid threshold signature");
                        false
                    },
                }
            },
            Err(err) => {
                warn!(%epoch, %err, "failed to get stake table");
                false
            },
        }
    }

    #[instrument(level = "trace", skip(self))]
    async fn is_leader(&self, view: ViewNumber, epoch: EpochNumber) -> bool {
        match self
            .stake_table_coordinator
            .membership_for_epoch(Some(epoch))
            .await
        {
            Ok(stake_table) => match stake_table.leader(view).await {
                Ok(leader) => leader == self.public_key,
                Err(err) => {
                    warn!(%view, %epoch, %err, "failed to get leader from stake table");
                    false
                },
            },
            Err(err) => {
                warn!(%view, %epoch, %err, "failed to get stake table");
                false
            },
        }
    }
    async fn validate_proposal_signature(&self, proposal: &SignedProposal<T, Proposal<T>>) -> bool {
        let view = proposal.data.view_number();
        let epoch = proposal.data.epoch;
        let membership = match self
            .stake_table_coordinator
            .membership_for_epoch(Some(epoch))
            .await
        {
            Ok(membership) => membership,
            Err(err) => {
                warn!(%epoch, %err, "failed to get stake table");
                return false;
            },
        };
        let view_leader_key = match membership.leader(view).await {
            Ok(leader) => leader,
            Err(err) => {
                warn!(%view, %epoch, %err, "failed to get leader from stake table");
                return false;
            },
        };
        let proposed_leaf: Leaf2<T> = proposal.data.clone().into();
        let signature = &proposal.signature;
        view_leader_key.validate(signature, proposed_leaf.commit().as_ref())
    }
}

fn vid_matches_proposal<T>(share: &VidDisperseShare2<T>, proposal: &Proposal<T>) -> bool
where
    T: NodeType,
{
    if let VidCommitment::V2(vid_comm) = proposal.block_header.payload_commitment() {
        vid_comm == share.payload_commitment
    } else {
        false
    }
}

impl<T: NodeType> ConsensusInput<T> {
    fn view_number(&self) -> ViewNumber {
        match self {
            ConsensusInput::BlockBuilt { view, .. } => *view,
            ConsensusInput::BlockReconstructed(view, _) => *view,
            ConsensusInput::Certificate1(cert) => cert.view_number(),
            ConsensusInput::Certificate2(cert) => cert.view_number(),
            ConsensusInput::HeaderCreated(view, _) => *view,
            ConsensusInput::Proposal(prop) => prop.view_number(),
            ConsensusInput::StateValidated(response) => response.view,
            ConsensusInput::StateValidationFailed(request) => request.view,
            ConsensusInput::Timeout(view) => *view,
            ConsensusInput::TimeoutCertificate(cert) => {
                // Add one because we are moving to the next view so all event
                // processing is for the next view
                cert.view_number() + 1
            },
            ConsensusInput::VidDisperseCreated(view, _) => *view,
            ConsensusInput::ViewSyncCertificate(cert) => cert.view_number(),
        }
    }
}
