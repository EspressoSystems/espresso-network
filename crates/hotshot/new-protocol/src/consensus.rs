use std::{
    cmp::max,
    collections::{BTreeMap, BTreeSet},
    marker::PhantomData,
};

use committable::{Commitment, Committable};
use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{
        BlockNumber, EpochNumber, Leaf2, VidCommitment, VidCommitment2, VidDisperse2,
        VidDisperseShare2, ViewChangeEvidence2, ViewNumber,
    },
    drb::DrbResult,
    epoch_membership::EpochMembershipCoordinator,
    message::{Proposal as SignedProposal, UpgradeLock},
    simple_certificate::{TimeoutCertificate2, ViewSyncFinalizeCertificate2},
    simple_vote::{
        CheckpointData, HasEpoch, QuorumData2, SimpleVote, TimeoutData2, TimeoutVote2, Vote2Data,
    },
    stake_table::StakeTableEntries,
    traits::{
        block_contents::BlockHeader, node_implementation::NodeType, signature_key::SignatureKey,
    },
    utils::{is_epoch_transition, is_last_block},
    vote::{self, Certificate, HasViewNumber},
};
use tracing::{debug, instrument, warn};

use crate::{
    block::BlockAndHeaderRequest,
    helpers::proposal_commitment,
    logging::KeyPrefix,
    message::{
        Certificate1, Certificate2, CheckpointVote, EpochChangeMessage, Proposal, ProposalMessage,
        Validated, Vote1, Vote2,
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
    EpochChange(EpochChangeMessage<T>),
    HeaderCreated(ViewNumber, T::BlockHeader),
    Proposal(T::SignatureKey, ProposalMessage<T, Validated>),
    StateValidated(StateResponse<T>),
    StateValidationFailed(StateResponse<T>),
    Timeout(ViewNumber, EpochNumber),
    TimeoutCertificate(TimeoutCertificate2<T>),
    TimeoutOneHonest(ViewNumber, EpochNumber),
    VidDisperseCreated(ViewNumber, VidDisperse2<T>),
    ViewSyncCertificate(ViewSyncFinalizeCertificate2<T>),
    DrbResult(EpochNumber, DrbResult),
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum ConsensusOutput<T: NodeType> {
    RequestBlockAndHeader(BlockAndHeaderRequest<T>),
    RequestProposal(ViewNumber, Commitment<Leaf2<T>>),
    RequestState(StateRequest<T>),
    RequestDrbResult(EpochNumber),
    SendProposal(SignedProposal<T, Proposal<T>>, VidDisperse2<T>),
    SendCheckpointVote(CheckpointVote<T>),
    SendTimeoutVote(TimeoutVote2<T>, Option<Certificate1<T>>),
    SendVote1(Vote1<T>),
    SendVote2(Vote2<T>),
    SendTimeoutCertificate(TimeoutCertificate2<T>, ViewNumber, EpochNumber),
    SendCertificate1(Certificate1<T>),
    SendEpochChange(EpochChangeMessage<T>),
    RequestVidDisperse {
        view: ViewNumber,
        epoch: EpochNumber,
        payload: T::BlockPayload,
        metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
    },
    LeafDecided {
        leaves: Vec<Leaf2<T>>,
        cert2: Certificate2<T>,
    },
    LockUpdated(Certificate2<T>),
    ViewChanged(ViewNumber, EpochNumber),
    ProposalValidated {
        proposal: SignedProposal<T, Proposal<T>>,
        sender: T::SignatureKey,
    },
}

pub struct Consensus<T: NodeType> {
    proposals: BTreeMap<ViewNumber, Proposal<T>>,
    proposed_views: BTreeSet<ViewNumber>,
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
    leaves: BTreeMap<ViewNumber, Leaf2<T>>,
    last_decided_view: ViewNumber,
    last_decided_leaf: Leaf2<T>,
    drb_results: BTreeMap<EpochNumber, DrbResult>,

    voted_1_views: BTreeSet<ViewNumber>,
    voted_2_views: BTreeSet<ViewNumber>,

    timeout_view: ViewNumber,
    current_view: ViewNumber,
    current_epoch: Option<EpochNumber>,

    // TODO: We need a next epoch stake table to handle the transition
    // And a way to set these stake tables, probably an event from coordinator
    stake_table_coordinator: EpochMembershipCoordinator<T>,

    public_key: T::SignatureKey,
    private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
    node_id: KeyPrefix,

    garbage_collection_interval: BlockNumber,
    epoch_height: BlockNumber,
    upgrade_lock: UpgradeLock<T>,
}

/// Protocol flow directive.
enum Protocol {
    /// Stop with further protocol steps.
    Abort,
    /// Continue with protocol.
    Continue,
}

impl<T: NodeType> Consensus<T> {
    pub fn new<B>(
        membership_coordinator: EpochMembershipCoordinator<T>,
        public_key: T::SignatureKey,
        private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
        genesis_leaf: Leaf2<T>,
        epoch_height: B,
        upgrade_lock: UpgradeLock<T>,
    ) -> Self
    where
        B: Into<BlockNumber>,
    {
        Self {
            proposals: BTreeMap::new(),
            proposed_views: BTreeSet::new(),
            vid_disperses: BTreeMap::new(),
            blocks: BTreeMap::new(),
            states_verified: BTreeMap::new(),
            blocks_reconstructed: BTreeMap::new(),
            certs: BTreeMap::new(),
            certs2: BTreeMap::new(),
            timeout_certs: BTreeMap::new(),
            view_sync_certs: BTreeMap::new(),
            locked_cert: None,
            leaves: BTreeMap::new(),
            last_decided_view: ViewNumber::genesis(),
            last_decided_leaf: genesis_leaf,
            headers: BTreeMap::new(),
            drb_results: BTreeMap::new(),
            node_id: KeyPrefix::from(&public_key),
            public_key,
            timeout_view: ViewNumber::genesis(),
            current_view: ViewNumber::genesis(),
            current_epoch: None,
            stake_table_coordinator: membership_coordinator,
            voted_1_views: BTreeSet::new(),
            voted_2_views: BTreeSet::new(),
            private_key,
            vid_shares: BTreeMap::new(),
            // TODO: make this configurable or Constant
            garbage_collection_interval: 100.into(),
            epoch_height: epoch_height.into(),
            upgrade_lock,
        }
    }

    /// Seed the genesis state so that the view-1 leader can propose without
    /// any external bootstrap injection.
    ///
    /// Stores a genesis certificate and proposal at view 0, sets the locked
    /// certificate, and sets the current epoch.  After calling this, a
    /// subsequent `apply` that triggers `maybe_propose(view=1)` will find the
    /// parent cert and proposal it needs.
    pub fn seed_genesis(&mut self, genesis_cert1: Certificate1<T>, genesis_proposal: Proposal<T>) {
        self.current_epoch = Some(genesis_proposal.epoch);
        self.certs
            .insert(ViewNumber::genesis(), genesis_cert1.clone());
        self.locked_cert = Some(genesis_cert1);
        self.proposals
            .insert(ViewNumber::genesis(), genesis_proposal);
    }

    /// Return the proposal stored at the given view, if any.
    pub fn proposal_at(&self, view: ViewNumber) -> Option<&Proposal<T>> {
        self.proposals.get(&view)
    }

    /// Apply consensus to the given input and collect protocol outputs.
    #[instrument(level = "debug", skip_all, fields(node = %self.node_id, view = %input.view_number()))]
    pub async fn apply(
        &mut self,
        input: ConsensusInput<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        let view = input.view_number();
        let proto = match input {
            ConsensusInput::Proposal(sender, proposal) => {
                self.handle_proposal(sender, proposal, outbox).await
            },
            ConsensusInput::Certificate1(certificate) => {
                self.handle_certificate1(certificate).await
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
                if let Some(proposal) = self.proposals.get(&state_response.view)
                    && proposal_commitment(proposal) != state_response.commitment
                {
                    return;
                }
                self.proposals.remove(&state_response.view);
                self.leaves.remove(&state_response.view);
                self.vid_shares.remove(&state_response.view);
                return;
            },
            ConsensusInput::Timeout(view, epoch)
            | ConsensusInput::TimeoutOneHonest(view, epoch) => {
                self.handle_timeout(view, epoch, outbox)
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
            ConsensusInput::DrbResult(epoch, drb_result) => {
                self.drb_results.insert(epoch, drb_result);
                Protocol::Continue
            },
            ConsensusInput::EpochChange(epoch_change) => {
                self.handle_epoch_change(epoch_change, outbox).await
            },
        };

        if matches!(proto, Protocol::Abort) {
            debug!("aborting protocol");
            return;
        }

        self.maybe_vote_1(view, outbox).await;
        self.maybe_vote_2_and_update_lock(view, outbox).await;
        self.maybe_decide(view, outbox);
        self.maybe_propose(view, outbox).await;
        // An event from the current view or the previous view can trigger a propose
        self.maybe_propose(view + 1, outbox).await;
    }

    pub fn last_decided_view(&self) -> ViewNumber {
        self.last_decided_view
    }

    pub fn last_decided_leaf(&self) -> &Leaf2<T> {
        &self.last_decided_leaf
    }

    pub fn undecided_leaves(&self) -> impl Iterator<Item = &Leaf2<T>> {
        self.leaves
            .range((
                std::ops::Bound::Excluded(self.last_decided_view),
                std::ops::Bound::Unbounded,
            ))
            .map(|(_, leaf)| leaf)
    }

    pub fn current_view(&self) -> ViewNumber {
        self.current_view
    }

    pub fn current_epoch(&self) -> Option<EpochNumber> {
        self.current_epoch
    }

    pub fn set_view(&mut self, view: ViewNumber, epoch: EpochNumber) {
        self.current_view = view;
        self.current_epoch = Some(epoch);
    }

    pub fn wants_proposal<S>(&self, p: &ProposalMessage<T, S>) -> bool {
        !(self
            .locked_cert
            .as_ref()
            .is_some_and(|l| l.view_number() > p.view_number())
            || self.proposals.contains_key(&p.view_number()))
    }

    pub fn gc(&mut self, view: ViewNumber, _epoch: EpochNumber) {
        self.proposed_views = self.proposed_views.split_off(&view);
        self.states_verified = self.states_verified.split_off(&view);
        self.blocks_reconstructed = self.blocks_reconstructed.split_off(&view);
        self.blocks = self.blocks.split_off(&view);
        self.vid_disperses = self.vid_disperses.split_off(&view);
        self.certs = self.certs.split_off(&view);
        self.certs2 = self.certs2.split_off(&view);
        self.timeout_certs = self.timeout_certs.split_off(&view);
        self.view_sync_certs = self.view_sync_certs.split_off(&view);
        self.headers = self.headers.split_off(&view);
        self.leaves = self.leaves.split_off(&view);
        self.voted_1_views = self.voted_1_views.split_off(&view);
        self.voted_2_views = self.voted_2_views.split_off(&view);
        self.last_decided_view = self.last_decided_view.max(view);
    }

    #[instrument(level = "debug", skip_all)]
    async fn handle_proposal(
        &mut self,
        sender: T::SignatureKey,
        proposal: ProposalMessage<T, Validated>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let view = proposal.view_number();

        if !self.wants_proposal(&proposal) {
            warn!(%view, "proposal too old");
            return Protocol::Abort;
        }

        let signed_proposal = proposal.proposal.clone();
        let vid_share = proposal.vid_share;
        let proposal = proposal.proposal.data;
        let epoch = proposal.epoch;
        // QC can be for a different epoch
        let Some(qc_epoch) = proposal.justify_qc.epoch() else {
            warn!(%view, "proposal has no epoch number");
            return Protocol::Abort;
        };
        let block_number = proposal.block_header.block_number();

        if !self.is_safe(&proposal) {
            debug!("proposal not safe");
            return Protocol::Abort;
        }

        // Validate the epoch transition rules.
        // - DRB result must be attached and match our calculated result.
        // - Next epoch justify QC (deciding QC) must be attached to the first proposal of the new epoch
        if proposal.epoch > EpochNumber::genesis() + 1
            && is_epoch_transition(block_number, *self.epoch_height)
        {
            let Some(drb) = self.drb_results.get(&(epoch + 1)) else {
                debug!(%epoch, "no DRB result for epoch");
                outbox.push_back(ConsensusOutput::RequestDrbResult(epoch + 1));
                return Protocol::Abort;
            };
            if proposal
                .next_drb_result
                .is_none_or(|proposed_drb| drb != &proposed_drb)
            {
                warn!(%epoch, "DRB result does not match proposal");
                return Protocol::Abort;
            }
        }

        // if the previous block is the last block of the epoch, this proposal is the first proposal of the new epoch
        if is_last_block(block_number.saturating_sub(1), *self.epoch_height) {
            let Some(cert2) = &proposal.next_epoch_justify_qc else {
                warn!(%epoch, "no next epoch justify QC");
                return Protocol::Abort;
            };
            if cert2.data.leaf_commit != proposal.justify_qc.data().leaf_commit {
                warn!(%epoch, "next epoch justify QC does not match proposal");
                return Protocol::Abort;
            }
            if !self.verify_cert(cert2, qc_epoch).await {
                warn!(%epoch, "next epoch justify QC not verified");
                return Protocol::Abort;
            }
        }

        let payload_size = vid_share.payload_byte_len();

        self.proposals.insert(view, proposal.clone());
        self.leaves.insert(view, proposal.clone().into());
        self.vid_shares.insert(view, vid_share);

        outbox.push_back(ConsensusOutput::RequestState(StateRequest {
            view: proposal.view_number(),
            parent_view: proposal.justify_qc.view_number(),
            epoch,
            block: proposal.block_header.block_number().into(),
            proposal: proposal.clone(),
            parent_commitment: proposal.justify_qc.data().leaf_commit,
            payload_size,
        }));

        let epoch = if is_last_block(block_number, *self.epoch_height) {
            epoch + 1
        } else {
            epoch
        };

        outbox.push_back(ConsensusOutput::ProposalValidated {
            proposal: signed_proposal,
            sender,
        });

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
    async fn handle_certificate1(&mut self, certificate: Certificate1<T>) -> Protocol {
        let view = certificate.view_number();
        if self.certs.contains_key(&view) {
            return Protocol::Continue;
        }
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
        self.certs.insert(view, certificate);
        Protocol::Continue
    }

    #[instrument(level = "debug", skip_all)]
    async fn handle_certificate2(&mut self, certificate: Certificate2<T>) -> Protocol {
        let view = certificate.view_number();
        if self.certs2.contains_key(&view) {
            return Protocol::Continue;
        }
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
    fn handle_timeout(
        &mut self,
        view: ViewNumber,
        epoch: EpochNumber,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        self.timeout_view = max(self.timeout_view, view);
        let data = TimeoutData2 {
            view,
            epoch: Some(epoch),
        };
        let vote = match SimpleVote::create_signed_vote(
            data,
            view,
            &self.public_key,
            &self.private_key,
            &self.upgrade_lock,
        ) {
            Ok(vote) => vote,
            Err(err) => {
                warn!(%view, %err, "failed to create timeout vote");
                return Protocol::Abort;
            },
        };
        outbox.push_back(ConsensusOutput::SendTimeoutVote(
            vote,
            self.locked_cert.clone(),
        ));
        Protocol::Abort
    }

    #[instrument(level = "debug", skip_all)]
    async fn handle_timeout_certificate(
        &mut self,
        certificate: TimeoutCertificate2<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let view = certificate.view_number() + 1;
        if self.timeout_certs.contains_key(&view) {
            return Protocol::Continue;
        }
        let Some(epoch) = certificate.epoch() else {
            warn!(view = %certificate.view_number(), "timeout certificate has no epoch number");
            return Protocol::Abort;
        };
        self.timeout_certs.insert(view, certificate.clone());
        self.current_epoch = Some(epoch);
        outbox.push_back(ConsensusOutput::ViewChanged(view, epoch));
        outbox.push_back(ConsensusOutput::SendTimeoutCertificate(
            certificate,
            view,
            epoch,
        ));
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
        // Note: We don't handle epoch change on timeout certificate, because
        // we can't change epoch after a timeout
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
        self.current_epoch = Some(epoch);
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
    async fn handle_epoch_change(
        &mut self,
        epoch_change: EpochChangeMessage<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let EpochChangeMessage {
            cert1,
            cert2,
            proposal,
        } = epoch_change;
        // Check if this epoch change is new
        if self
            .locked_cert
            .as_ref()
            .is_some_and(|locked_cert| locked_cert.view_number() > cert1.view_number())
        {
            warn!("locked certificate is newer than epoch change certificate1");
            return Protocol::Abort;
        }
        // Check if the certificates match
        if cert1.view_number() != cert2.view_number()
            || cert1.epoch() != cert2.epoch()
            || cert1.data.leaf_commit != cert2.data.leaf_commit
        {
            warn!("epoch change certificates do not match");
            return Protocol::Abort;
        }
        // check if it's the last block for the correct epoch
        if !is_last_block(cert2.data.block_number, *self.epoch_height) {
            warn!("epoch change certificate2 is not the last block of the epoch");
            return Protocol::Abort;
        }
        if cert2.data.block_number / *self.epoch_height != *cert2.data.epoch {
            warn!("epoch change certificate2 is not for the correct epoch");
            return Protocol::Abort;
        }
        // Check if the proposal matches the certificate1
        if proposal_commitment(&proposal) != cert1.data.leaf_commit {
            warn!("epoch change proposal commitment does not match certificate1 leaf commitment");
            return Protocol::Abort;
        }
        // Verify the certificates
        let Some(cert1_epoch) = cert1.epoch() else {
            warn!("epoch change certificate1 has no epoch number");
            return Protocol::Abort;
        };
        if !self.verify_cert(&cert1, cert1_epoch).await {
            warn!("epoch change certificate not verified");
            return Protocol::Abort;
        }
        if !self.verify_cert(&cert2, cert2.data.epoch).await {
            warn!("epoch change certificate not verified");
            return Protocol::Abort;
        }
        let next_view = cert2.view_number() + 1;
        let next_epoch = cert2.data.epoch + 1;
        // Change view to the first view of the next epoch
        self.current_epoch = Some(next_epoch);
        outbox.push_back(ConsensusOutput::ViewChanged(next_view, next_epoch));

        // Request block and header if we're the first leader of the next epoch
        if self.is_leader(next_view, next_epoch).await {
            outbox.push_back(ConsensusOutput::RequestBlockAndHeader(
                BlockAndHeaderRequest {
                    view: next_view,
                    epoch: next_epoch,
                    parent_proposal: proposal.clone(),
                },
            ));
        }

        self.proposals.insert(cert2.view_number(), proposal);
        self.certs.insert(cert1.view_number(), cert1);
        self.certs2.insert(cert2.view_number(), cert2);
        Protocol::Continue
    }

    #[instrument(level = "debug", skip_all)]
    async fn maybe_propose(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        if self.proposed_views.contains(&view) {
            return;
        }

        let mut view_change_evidence = None;
        if let Some(view_sync_cert) = self.view_sync_certs.get(&view) {
            view_change_evidence = Some(ViewChangeEvidence2::ViewSync(view_sync_cert.clone()));
        } else if let Some(timeout_cert) = self.timeout_certs.get(&view) {
            view_change_evidence = Some(ViewChangeEvidence2::Timeout(timeout_cert.clone()));
        };
        let parent_cert = if view_change_evidence.is_some() {
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

        let first_proposal_of_epoch =
            is_last_block(header.block_number().saturating_sub(1), *self.epoch_height);
        let proposal_epoch = if first_proposal_of_epoch {
            proposal.epoch + 1
        } else {
            proposal.epoch
        };
        if !self.is_leader(view, proposal_epoch).await {
            warn!(epoch = %proposal_epoch, "not the leader for this view, we should not have a header");
            return;
        }

        // TODO: Handle epoch change state cert
        // The first two epochs have no prior DRB computation so we skip the
        // requirement — matching the same guard in handle_proposal.
        let next_drb_result = if proposal.epoch > EpochNumber::genesis() + 1
            && is_epoch_transition(header.block_number(), *self.epoch_height)
        {
            let Some(drb) = self.drb_results.get(&EpochNumber::new(*proposal.epoch + 1)) else {
                debug!(%proposal.epoch, "no DRB result for epoch");
                return;
            };
            Some(*drb)
        } else {
            None
        };
        let next_epoch_justify_qc = if first_proposal_of_epoch {
            let Some(next_epoch_justify_qc) = self.certs2.get(&parent_view) else {
                debug!("no next epoch justify QC");
                return;
            };
            Some(next_epoch_justify_qc.clone())
        } else {
            None
        };
        let proposal = Proposal::<T> {
            block_header: header.clone(),
            view_number: view,
            epoch: proposal_epoch,
            justify_qc: parent_cert.clone(),
            next_epoch_justify_qc,
            upgrade_certificate: None,
            view_change_evidence,
            next_drb_result,
            state_cert: None,
        };

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

        self.proposed_views.insert(view);
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
        // Handle Epoch Change by broadcasting the epoch change message if we have
        // all the data we need.
        if is_last_block(proposal.block_header.block_number(), *self.epoch_height)
            && let Some(cert1) = self.certs.get(&view)
            && cert1.data.leaf_commit == proposal_commit
        {
            let epoch_change = EpochChangeMessage {
                cert1: cert1.clone(),
                cert2: cert2.clone(),
                proposal: proposal.clone(),
            };
            outbox.push_back(ConsensusOutput::SendEpochChange(epoch_change));
        }
        // we have a second certificate, and matching proposal, it is decided.
        let leaf: Leaf2<T> = proposal.clone().into();
        self.last_decided_view = max(self.last_decided_view, leaf.view_number());
        self.last_decided_leaf = leaf.clone();
        let mut gc = None;
        if leaf.block_header().block_number() % *self.garbage_collection_interval == 0 {
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
                && leaf.block_header().block_number() % *self.garbage_collection_interval == 0
            {
                gc = Some((leaf.view_number(), leaf.justify_qc().epoch()));
            }
            decided.push(leaf);
            parent_view = proposal.justify_qc.view_number();
            parent_commit = proposal.justify_qc.data.leaf_commit;
        }
        outbox.push_back(ConsensusOutput::LeafDecided {
            leaves: decided,
            cert2: cert2.clone(),
        });
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
                &self.upgrade_lock,
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
    async fn maybe_vote_1(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        if view <= self.timeout_view {
            return;
        }
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

        if !self.staked_in_epoch(proposal.epoch).await {
            return;
        }

        // Verify parent chain unless justify_qc is the genesis QC
        let parent_view = proposal.justify_qc.view_number();

        // We don't need the genesis block or the last block of the epoch to be reconstructed or verified
        // or the genesis qc to be verified
        if parent_view != ViewNumber::genesis()
            && !is_last_block(
                proposal.block_header.block_number().saturating_sub(1),
                *self.epoch_height,
            )
        {
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
            &&self.upgrade_lock,
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
    async fn maybe_vote_2_and_update_lock(
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
        // We can now update the lock, change view and vote
        if self
            .locked_cert
            .as_mut()
            .is_none_or(|locked_cert| locked_cert.view_number() < cert1.view_number())
        {
            self.locked_cert = Some(cert1.clone());
            self.current_epoch = Some(proposal_epoch);
            outbox.push_back(ConsensusOutput::ViewChanged(view + 1, proposal_epoch));
            outbox.push_back(ConsensusOutput::SendCertificate1(cert1.clone()));
        }

        if !self.staked_in_epoch(proposal_epoch).await {
            return;
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
            &self.upgrade_lock,
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

    #[instrument(level = "trace", skip_all)]
    fn is_safe(&self, proposal: &Proposal<T>) -> bool {
        let Some(locked_cert) = self.locked_cert.as_ref() else {
            // Locked certificate is not set which means it is at genesis
            debug!("at genesis");
            return true;
        };

        // cert1 + block arrived before proposal
        if locked_cert.view_number() == proposal.view_number() {
            return locked_cert.data.leaf_commit == proposal_commitment(proposal);
        }

        let liveness_check = proposal.justify_qc.view_number() > locked_cert.view_number();
        let parent_commit = match proposal.justify_qc.data_commitment(&self.upgrade_lock) {
            Ok(c) => c,
            Err(err) => {
                warn!(%err, "failed to compute justify qc data commitment");
                return false;
            },
        };
        let locked_commit = match locked_cert.data_commitment(&self.upgrade_lock) {
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
                match cert.is_valid_cert(&entries, threshold, &self.upgrade_lock) {
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

    #[instrument(level = "trace", skip_all)]
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

    async fn staked_in_epoch(&self, epoch: EpochNumber) -> bool {
        match self
            .stake_table_coordinator
            .membership_for_epoch(Some(epoch))
            .await
        {
            Ok(stake_table) => stake_table.has_stake(&self.public_key).await,
            Err(err) => {
                warn!(%epoch, %err, "failed to get stake table");
                false
            },
        }
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
            ConsensusInput::Proposal(_, prop) => prop.view_number(),
            ConsensusInput::StateValidated(response) => response.view,
            ConsensusInput::StateValidationFailed(request) => request.view,
            ConsensusInput::Timeout(view, _) => *view,
            ConsensusInput::TimeoutOneHonest(view, _) => *view,
            ConsensusInput::TimeoutCertificate(cert) => {
                // Add one because we are moving to the next view so all event
                // processing is for the next view
                cert.view_number() + 1
            },
            ConsensusInput::VidDisperseCreated(view, _) => *view,
            ConsensusInput::ViewSyncCertificate(cert) => cert.view_number(),
            // TODO: where else can this cause problems?
            ConsensusInput::DrbResult(..) => ViewNumber::genesis(),
            ConsensusInput::EpochChange(epoch_change) => epoch_change.cert1.view_number(),
        }
    }
}
