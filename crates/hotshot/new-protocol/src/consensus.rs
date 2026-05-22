use std::{
    cmp::max,
    collections::{BTreeMap, BTreeSet},
    marker::PhantomData,
};

use committable::{Commitment, Committable};
use hotshot::traits::BlockPayload;
use hotshot_contract_adapter::light_client::derive_signed_state_digest;
use hotshot_types::{
    data::{
        BlockNumber, EpochNumber, Leaf2, VidCommitment, VidCommitment2, VidDisperse2,
        VidDisperseShare2, ViewNumber,
    },
    drb::DrbResult,
    epoch_membership::EpochMembershipCoordinator,
    message::{Proposal as SignedProposal, UpgradeLock},
    simple_certificate::{
        LightClientStateUpdateCertificateV2, TimeoutCertificate2,
        check_qc_state_cert_correspondence,
    },
    simple_vote::{
        CheckpointData, HasEpoch, LightClientStateUpdateVote2, QuorumData2, SimpleVote,
        TimeoutData2, TimeoutVote2, Vote2Data,
    },
    stake_table::{HSStakeTable, StakeTableEntries},
    traits::{
        block_contents::BlockHeader,
        node_implementation::NodeType,
        signature_key::{
            LCV2StateSignatureKey, LCV3StateSignatureKey, SignatureKey, StateSignatureKey,
        },
    },
    utils::{epoch_from_block_number, is_epoch_root, is_epoch_transition, is_last_block},
    vote::{self, Certificate, HasViewNumber},
};
use tracing::{debug, instrument, warn};

use crate::{
    block::BlockAndHeaderRequest,
    helpers::proposal_commitment,
    logging::KeyPrefix,
    message::{
        BlockPushMessage, Certificate1, Certificate2, CheckpointVote, EpochChangeMessage, Proposal,
        ProposalFetchRequest, ProposalMessage, Validated, VidShareMessage, Vote1, Vote2,
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
    /// Atomic pair emitted by the `EpochRootVoteCollector` for epoch-root views:
    /// a `Certificate1` and its matching `LightClientStateUpdateCertificateV2`.
    /// Consensus never sees an epoch-root Cert1 without the matching state_cert.
    EpochRootCertificates {
        cert1: Certificate1<T>,
        state_cert: LightClientStateUpdateCertificateV2<T>,
    },
    EpochChange(EpochChangeMessage<T>),
    HeaderCreated(ViewNumber, Commitment<Leaf2<T>>, T::BlockHeader),
    ProposalWithVidShare(
        T::SignatureKey,
        ProposalMessage<T, Validated>,
        VidDisperseShare2<T>,
    ),
    StateValidated(StateResponse<T>),
    StateValidationFailed(StateResponse<T>),
    Timeout(ViewNumber, EpochNumber),
    TimeoutCertificate(TimeoutCertificate2<T>),
    TimeoutOneHonest(ViewNumber, EpochNumber),
    VidDisperseCreated(ViewNumber, VidDisperse2<T>),
    DrbResult(EpochNumber, DrbResult),
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum ConsensusOutput<T: NodeType> {
    RequestBlockAndHeader(BlockAndHeaderRequest<T>),
    RequestState(StateRequest<T>),
    RequestDrbResult(EpochNumber),
    SendProposal(SignedProposal<T, Proposal<T>>),
    SendVidShares(Vec<VidShareMessage<T>>),
    SendBlockToLeader {
        next_leader: T::SignatureKey,
        block: BlockPushMessage<T>,
    },
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
        /// Certificate1 (QC) that certifies the most recent (first) leaf in the chain.
        /// Each older leaf's cert1 is available as the next leaf's `justify_qc`.
        cert1: Certificate1<T>,
        cert2: Option<Certificate2<T>>,
        vid_shares: Vec<Option<SignedProposal<T, VidDisperseShare2<T>>>>,
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
    signed_proposals: BTreeMap<ViewNumber, SignedProposal<T, Proposal<T>>>,
    proposed_views: BTreeSet<ViewNumber>,
    vid_shares: BTreeMap<ViewNumber, VidDisperseShare2<T>>,
    states_verified: BTreeMap<ViewNumber, Commitment<Leaf2<T>>>,
    blocks_reconstructed: BTreeMap<ViewNumber, VidCommitment2>,
    blocks: BTreeMap<ViewNumber, T::BlockPayload>,
    certs: BTreeMap<ViewNumber, Certificate1<T>>,
    certs2: BTreeMap<ViewNumber, Certificate2<T>>,
    timeout_certs: BTreeMap<ViewNumber, TimeoutCertificate2<T>>,
    locked_cert: Option<Certificate1<T>>,
    headers: BTreeMap<(ViewNumber, Commitment<Leaf2<T>>), T::BlockHeader>,
    /// Block-push staging: headers now have complicated keys to be efficiently usable.
    block_push_prep: BTreeMap<ViewNumber, BlockPushPrep<T>>,
    leaves: BTreeMap<ViewNumber, Leaf2<T>>,
    last_decided_view: ViewNumber,
    last_decided_leaf: Leaf2<T>,
    drb_results: BTreeMap<EpochNumber, DrbResult>,

    voted_1_views: BTreeSet<ViewNumber>,
    voted_2_views: BTreeSet<ViewNumber>,
    pushed_block_views: BTreeSet<ViewNumber>,

    /// Certificates whose epoch membership was not yet available when they
    /// arrived.  They are retried when new epoch data becomes available.
    pending_certs1: BTreeMap<ViewNumber, Certificate1<T>>,
    pending_certs2: BTreeMap<ViewNumber, Certificate2<T>>,

    timeout_view: ViewNumber,
    current_view: ViewNumber,
    current_epoch: Option<EpochNumber>,

    // TODO: We need a next epoch stake table to handle the transition
    // And a way to set these stake tables, probably an event from coordinator
    stake_table_coordinator: EpochMembershipCoordinator<T>,

    public_key: T::SignatureKey,
    private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
    state_private_key: <T::StateSignatureKey as StateSignatureKey>::StatePrivateKey,
    stake_table_capacity: usize,
    // TODO: persist state_certs
    state_certs: BTreeMap<EpochNumber, LightClientStateUpdateCertificateV2<T>>,
    node_id: KeyPrefix,
    upgrade_lock: UpgradeLock<T>,

    garbage_collection_interval: BlockNumber,
    pub(crate) epoch_height: BlockNumber,

    pub(crate) tracer: Option<crate::leader_trace::LeaderTracerHandle>,
}

/// Captured at HeaderCreated time so block-push doesn't need to re-fetch the
/// header from `self.headers` (which is keyed by `(view, leaf_commit)` and
/// thus awkward to look up by view alone from `BlockBuilt`/`VidDisperseCreated`
/// where the leaf_commit isn't in scope).
struct BlockPushPrep<T: NodeType> {
    epoch: EpochNumber,
    payload_commitment: VidCommitment,
    metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
    /// Pre-computed: header.block_number() == epoch_height's last block.
    /// If true, the next leader is in epoch + 1; otherwise same epoch.
    is_last_block: bool,
}

/// Protocol flow directive.
enum Protocol {
    /// Stop with further protocol steps.
    Abort,
    /// Continue with protocol.
    Continue,
}

/// Result of attempting to verify a certificate's epoch membership.
enum CertVerification {
    /// Certificate is cryptographically valid.
    Valid,
    /// Certificate is cryptographically invalid (bad signature).
    Invalid,
    /// The epoch's stake table is not yet available (catchup in progress).
    EpochUnavailable,
}

impl<T: NodeType> Consensus<T> {
    #[allow(clippy::too_many_arguments)]
    pub fn new<B>(
        membership_coordinator: EpochMembershipCoordinator<T>,
        public_key: T::SignatureKey,
        private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
        state_private_key: <T::StateSignatureKey as StateSignatureKey>::StatePrivateKey,
        stake_table_capacity: usize,
        upgrade_lock: UpgradeLock<T>,
        genesis_leaf: Leaf2<T>,
        epoch_height: B,
        garbage_collection_interval: B,
    ) -> Self
    where
        B: Into<BlockNumber>,
    {
        Self {
            proposals: BTreeMap::new(),
            signed_proposals: BTreeMap::new(),
            proposed_views: BTreeSet::new(),
            blocks: BTreeMap::new(),
            states_verified: BTreeMap::new(),
            blocks_reconstructed: BTreeMap::new(),
            certs: BTreeMap::new(),
            certs2: BTreeMap::new(),
            timeout_certs: BTreeMap::new(),
            locked_cert: None,
            leaves: BTreeMap::new(),
            last_decided_view: ViewNumber::genesis(),
            last_decided_leaf: genesis_leaf,
            headers: BTreeMap::new(),
            block_push_prep: BTreeMap::new(),
            drb_results: BTreeMap::new(),
            node_id: KeyPrefix::from(&public_key),
            public_key,
            timeout_view: ViewNumber::genesis(),
            current_view: ViewNumber::genesis(),
            current_epoch: None,
            stake_table_coordinator: membership_coordinator,
            voted_1_views: BTreeSet::new(),
            voted_2_views: BTreeSet::new(),
            pushed_block_views: BTreeSet::new(),
            pending_certs1: BTreeMap::new(),
            pending_certs2: BTreeMap::new(),
            private_key,
            state_private_key,
            stake_table_capacity,
            state_certs: BTreeMap::new(),
            upgrade_lock,
            vid_shares: BTreeMap::new(),
            garbage_collection_interval: garbage_collection_interval.into(),
            epoch_height: epoch_height.into(),
            tracer: None,
        }
    }

    /// Install an optional leader-event tracer. Only the bench binary sets this;
    /// production builds leave it None and pay one branch per call site.
    pub fn set_tracer(&mut self, tracer: Option<crate::leader_trace::LeaderTracerHandle>) {
        self.tracer = tracer;
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

    /// Return the Certificate1 (QC) stored at the given view, if any.
    pub fn cert1_at(&self, view: ViewNumber) -> Option<&Certificate1<T>> {
        self.certs.get(&view)
    }

    fn signed_vid_share(
        &self,
        view: ViewNumber,
    ) -> Option<SignedProposal<T, VidDisperseShare2<T>>> {
        self.vid_shares
            .get(&view)?
            .clone()
            .to_proposal(&self.private_key)
    }

    pub fn signed_proposal_fetch_request(
        &self,
        view: ViewNumber,
    ) -> Result<ProposalFetchRequest<T>, <T::SignatureKey as SignatureKey>::SignError> {
        ProposalFetchRequest::new(view, self.public_key.clone(), &self.private_key)
    }

    /// Return the Certificate2 stored at the given view, if any.
    pub fn cert2_at(&self, view: ViewNumber) -> Option<&Certificate2<T>> {
        self.certs2.get(&view)
    }

    /// Apply consensus to the given input and collect protocol outputs.
    #[instrument(level = "debug", skip_all, fields(node = %self.node_id, view = %input.view_number()))]
    pub fn apply(&mut self, input: ConsensusInput<T>, outbox: &mut Outbox<ConsensusOutput<T>>) {
        let drb_epoch = match &input {
            ConsensusInput::DrbResult(epoch, _) => Some(*epoch),
            _ => None,
        };
        // DRB results arrive asynchronously with no specific view attached.
        // Use `current_view` so that the post-apply retries (`maybe_propose`,
        // `maybe_vote_*`) target the view the node is actually on — in
        // particular, a leader blocked on `self.drb_results` for an
        // epoch-transition proposal retries that proposal here.
        let view = if matches!(&input, ConsensusInput::DrbResult(..)) {
            self.current_view
        } else {
            input.view_number()
        };
        let proto = match input {
            ConsensusInput::ProposalWithVidShare(sender, proposal, vid_share) => {
                self.handle_proposal_with_vid_share(sender, proposal, vid_share, outbox)
            },
            ConsensusInput::Certificate1(certificate) => {
                self.handle_certificate1(certificate, outbox)
            },
            ConsensusInput::Certificate2(certificate) => {
                self.handle_certificate2(certificate, outbox)
            },
            ConsensusInput::EpochRootCertificates { cert1, state_cert } => {
                // Store state_cert first so the subsequent Cert1 handler / leader
                // proposer has it on hand. Atomicity invariant: this pair always
                // arrives together; Consensus never sees the Cert1 alone.
                self.state_certs.insert(state_cert.epoch, state_cert);
                self.handle_certificate1(cert1, outbox)
            },
            ConsensusInput::TimeoutCertificate(certificate) => {
                self.handle_timeout_certificate(certificate, outbox)
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
            ConsensusInput::HeaderCreated(view, commitment, header) => {
                crate::trace_leader_event!(
                    self.tracer,
                    view,
                    crate::leader_trace::LeaderEvent::HeaderCreatedApplied
                );
                let epoch_height = *self.epoch_height;
                let block_number = header.block_number();
                self.block_push_prep.insert(
                    view,
                    BlockPushPrep {
                        epoch: EpochNumber::new(epoch_from_block_number(
                            block_number,
                            epoch_height,
                        )),
                        payload_commitment: header.payload_commitment(),
                        metadata: header.metadata().clone(),
                        is_last_block: is_last_block(block_number, epoch_height),
                    },
                );
                self.headers.insert((view, commitment), header);
                self.maybe_send_block_to_next_leader(view, outbox);
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
                crate::trace_leader_event!(
                    self.tracer,
                    view,
                    crate::leader_trace::LeaderEvent::BlockBuiltApplied
                );
                outbox.push_back(ConsensusOutput::RequestVidDisperse {
                    view,
                    epoch,
                    payload: payload.clone(),
                    metadata: metadata.clone(),
                });
                crate::trace_leader_event!(
                    self.tracer,
                    view,
                    crate::leader_trace::LeaderEvent::RequestVidDisperseQueued
                );
                self.blocks.insert(view, payload);
                Protocol::Continue
            },
            ConsensusInput::VidDisperseCreated(view, vid_disperse) => {
                crate::trace_leader_event!(
                    self.tracer,
                    view,
                    crate::leader_trace::LeaderEvent::NsDisperseEnd
                );
                // Push the full payload to L_{V+1} first.
                self.maybe_send_block_to_next_leader(view, outbox);
                // Directly send the VID shares before making a proposal.
                self.send_vid_shares(&view, vid_disperse, outbox);
                Protocol::Continue
            },
            ConsensusInput::DrbResult(epoch, drb_result) => {
                self.drb_results.insert(epoch, drb_result);
                Protocol::Continue
            },
            ConsensusInput::EpochChange(epoch_change) => {
                self.handle_epoch_change(epoch_change, outbox)
            },
        };

        if matches!(proto, Protocol::Abort) {
            debug!("aborting protocol");
            return;
        }

        self.maybe_vote_1(view, outbox);
        self.maybe_vote_2_and_update_lock(view, outbox);
        self.maybe_decide(view, outbox);
        self.maybe_propose(view, outbox);
        // An event from the current view or the previous view can trigger a propose
        self.maybe_propose(view + 1, outbox);

        // When new epoch data arrives (DRB result or epoch change), retry
        // any pending certificates that were deferred because their epoch
        // membership wasn't available yet.
        if drb_epoch.is_some() {
            self.retry_pending_certs(outbox);
        }
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

    pub fn wants_proposal_for_view(&self, view: &ViewNumber) -> bool {
        let locked_too_new = self
            .locked_cert
            .as_ref()
            .is_some_and(|l| l.view_number() > *view);
        // A proposal may already be in `self.proposals` because we received
        // an EpochChangeMessage for it (which carries the proposal but no
        // vid_share and does not trigger state validation).  In that case we
        // still want to process the real proposal message so handle_proposal
        // runs — it populates vid_shares and emits RequestState.
        let fully_processed =
            self.proposals.contains_key(view) && self.vid_shares.contains_key(view);
        !(locked_too_new || fully_processed)
    }

    pub fn signed_proposal(&self, view: &ViewNumber) -> Option<&SignedProposal<T, Proposal<T>>> {
        self.signed_proposals.get(view)
    }

    pub fn gc(&mut self, view: ViewNumber, _epoch: EpochNumber) {
        self.proposed_views = self.proposed_views.split_off(&view);
        self.states_verified = self.states_verified.split_off(&view);
        self.blocks_reconstructed = self.blocks_reconstructed.split_off(&view);
        self.blocks = self.blocks.split_off(&view);
        self.certs = self.certs.split_off(&view);
        self.certs2 = self.certs2.split_off(&view);
        self.pending_certs1 = self.pending_certs1.split_off(&view);
        self.pending_certs2 = self.pending_certs2.split_off(&view);
        self.timeout_certs = self.timeout_certs.split_off(&view);
        self.headers
            .retain(|(header_view, _), _| *header_view >= view);
        self.block_push_prep = self.block_push_prep.split_off(&view);
        self.leaves = self.leaves.split_off(&view);
        self.proposals = self.proposals.split_off(&view);
        self.signed_proposals = self.signed_proposals.split_off(&view);
        self.voted_1_views = self.voted_1_views.split_off(&view);
        self.voted_2_views = self.voted_2_views.split_off(&view);
        self.pushed_block_views = self.pushed_block_views.split_off(&view);
    }

    #[instrument(level = "debug", skip_all)]
    fn handle_proposal_with_vid_share(
        &mut self,
        sender: T::SignatureKey,
        proposal: ProposalMessage<T, Validated>,
        vid_share: VidDisperseShare2<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let view = proposal.view_number();

        if !self.wants_proposal_for_view(&view) {
            warn!(%view, "proposal too old");
            return Protocol::Abort;
        }

        let signed_proposal = proposal.proposal.clone();
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
            if !self.verify_cert(cert2, qc_epoch) {
                warn!(%epoch, "next epoch justify QC not verified");
                return Protocol::Abort;
            }
        }

        let payload_size = vid_share.payload_byte_len();

        // Store the proposal before the DRB check so it is not lost when
        // the DRB is not yet available (e.g. a node catching up after a
        // restart).  Voting is deferred to `maybe_vote_1` which verifies
        // the DRB before casting a vote.
        self.proposals.insert(view, proposal.clone());
        self.signed_proposals.insert(view, signed_proposal.clone());
        self.leaves.insert(view, proposal.clone().into());
        self.vid_shares.insert(view, vid_share);

        // Request the DRB if we don't have it yet.  A mismatching DRB is
        // a hard failure (invalid leader), but a missing DRB is
        // recoverable — the proposal is stored and voting will proceed
        // once the DRB arrives.  Same epoch guard as `maybe_propose`:
        // transitions in epoch >= 2 (`> genesis`) carry `next_drb_result`
        // (the successor epoch's DRB lives in this leaf, so the successor
        // epoch's catchup path unwraps `leaf.next_drb_result`).
        if proposal.epoch > EpochNumber::genesis()
            && is_epoch_transition(block_number, *self.epoch_height)
        {
            if let Some(drb) = self.drb_results.get(&(epoch + 1)) {
                if proposal
                    .next_drb_result
                    .is_none_or(|proposed_drb| drb != &proposed_drb)
                {
                    warn!(%epoch, "DRB result does not match proposal");
                    return Protocol::Abort;
                }
            } else {
                outbox.push_back(ConsensusOutput::RequestDrbResult(epoch + 1));
            }
        }

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

        if self.is_leader(view + 1, epoch) {
            crate::trace_leader_event!(
                self.tracer,
                view,
                crate::leader_trace::LeaderEvent::ProposalValidatedVMinus1
            );
            outbox.push_back(ConsensusOutput::RequestBlockAndHeader(
                BlockAndHeaderRequest {
                    view: view + 1,
                    epoch,
                    parent_proposal: proposal,
                },
            ));
            crate::trace_leader_event!(
                self.tracer,
                view + 1,
                crate::leader_trace::LeaderEvent::RequestBlockHeaderQueued
            );
        }

        Protocol::Continue
    }

    #[instrument(level = "debug", skip_all)]
    fn handle_certificate1(
        &mut self,
        certificate: Certificate1<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
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
        match self.try_verify_cert(&certificate, certificate_epoch) {
            CertVerification::Valid => {},
            CertVerification::Invalid => {
                warn!(%view, "certificate1 not verified");
                return Protocol::Abort;
            },
            CertVerification::EpochUnavailable => {
                debug!(%view, %certificate_epoch, "certificate1 deferred (epoch unavailable)");
                self.pending_certs1.insert(view, certificate);
                outbox.push_back(ConsensusOutput::RequestDrbResult(certificate_epoch));
                return Protocol::Continue;
            },
        }
        self.certs.insert(view, certificate);
        Protocol::Continue
    }

    #[instrument(level = "debug", skip_all)]
    fn handle_certificate2(
        &mut self,
        certificate: Certificate2<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
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
        match self.try_verify_cert(&certificate, certificate_epoch) {
            CertVerification::Valid => {},
            CertVerification::Invalid => {
                warn!(%view, "certificate2 not verified");
                return Protocol::Abort;
            },
            CertVerification::EpochUnavailable => {
                debug!(%view, %certificate_epoch, "certificate2 deferred (epoch unavailable)");
                self.pending_certs2.insert(view, certificate);
                outbox.push_back(ConsensusOutput::RequestDrbResult(certificate_epoch));
                return Protocol::Continue;
            },
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
    fn handle_timeout_certificate(
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
        if !self.is_leader(view, epoch) {
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
    fn handle_epoch_change(
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
        if !self.verify_cert(&cert1, cert1_epoch) {
            warn!("epoch change certificate not verified");
            return Protocol::Abort;
        }
        if !self.verify_cert(&cert2, cert2.data.epoch) {
            warn!("epoch change certificate not verified");
            return Protocol::Abort;
        }
        let next_view = cert2.view_number() + 1;
        let next_epoch = cert2.data.epoch + 1;
        // Change view to the first view of the next epoch
        self.current_epoch = Some(next_epoch);
        outbox.push_back(ConsensusOutput::ViewChanged(next_view, next_epoch));

        // Request block and header if we're the first leader of the next epoch
        if self.is_leader(next_view, next_epoch) {
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

    // Push the full payload to the leader of V+1.
    #[instrument(level = "debug", skip_all)]
    fn maybe_send_block_to_next_leader(
        &mut self,
        view: ViewNumber,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        if self.pushed_block_views.contains(&view) {
            return;
        }
        let Some(prep) = self.block_push_prep.get(&view) else {
            debug!(%view, "block push deferred: no block-push prep");
            return;
        };
        let next_epoch = if prep.is_last_block {
            prep.epoch + 1
        } else {
            prep.epoch
        };
        let next_view = view + 1;
        let Ok(membership) = self
            .stake_table_coordinator
            .membership_for_epoch(Some(next_epoch))
        else {
            debug!(%next_epoch, "block push skipped: next-epoch membership unavailable");
            return;
        };
        let Ok(next_leader) = membership.leader(next_view) else {
            debug!(%next_view, "block push skipped: leader lookup failed");
            return;
        };
        if next_leader == self.public_key {
            debug!(%next_view, "block push skipped: we are the next leader");
            return;
        }
        let Some(payload) = self.blocks.get(&view).cloned() else {
            debug!(%view, "block push deferred: no payload yet");
            return;
        };
        let block = match BlockPushMessage::new(
            view,
            prep.epoch,
            payload,
            prep.metadata.clone(),
            prep.payload_commitment,
            &self.private_key,
        ) {
            Ok(block) => block,
            Err(err) => {
                warn!(%view, %err, "block push skipped: failed to sign");
                return;
            },
        };
        crate::trace_leader_event!(
            self.tracer,
            view,
            crate::leader_trace::LeaderEvent::BlockPushSigned
        );
        outbox.push_back(ConsensusOutput::SendBlockToLeader { next_leader, block });
        crate::trace_leader_event!(
            self.tracer,
            view,
            crate::leader_trace::LeaderEvent::BlockPushQueued
        );
        self.pushed_block_views.insert(view);
    }

    // The leader's own share is also unicast back to itself (cliquenet
    // self-loopback): it is the only way the leader's `handle_proposal_and_vid_share`
    // path runs for its own proposal, populating `proposals`, `leaves`,
    // `states_verified`, and seeding the VID reconstructor's metadata.
    #[instrument(level = "debug", skip_all)]
    fn send_vid_shares(
        &self,
        view: &ViewNumber,
        vid_disperse: VidDisperse2<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        crate::trace_leader_event!(
            self.tracer,
            *view,
            crate::leader_trace::LeaderEvent::ShareSignLoopStart
        );
        let vid_messages = vid_disperse
            .to_shares()
            .into_iter()
            .filter_map(|share| {
                let Some(proposal) = share.to_proposal(&self.private_key) else {
                    warn!(%view, "failed to sign VID share proposal");
                    return None;
                };
                Some(proposal)
            })
            .collect::<Vec<_>>();
        crate::trace_leader_event!(
            self.tracer,
            *view,
            crate::leader_trace::LeaderEvent::ShareSignLoopEnd
        );
        outbox.push_back(ConsensusOutput::SendVidShares(vid_messages));
        crate::trace_leader_event!(
            self.tracer,
            *view,
            crate::leader_trace::LeaderEvent::VidSharesQueued
        );
    }

    #[instrument(level = "debug", skip_all)]
    fn maybe_propose(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        if self.proposed_views.contains(&view) {
            return;
        }
        crate::trace_leader_event!(
            self.tracer,
            view,
            crate::leader_trace::LeaderEvent::MaybeProposeEntered
        );

        let view_change_evidence = self.timeout_certs.get(&view).cloned();
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

        // Key the header lookup by the proposal's leaf commitment, not the
        // cert's `leaf_commit` field b/c genesis cert leaf commit != genesis proposals
        // leaf commitment.
        let parent_commitment = proposal_commitment(proposal);
        let Some(header) = self.headers.get(&(view, parent_commitment)) else {
            debug!("no block header");
            return;
        };
        if !self.blocks.contains_key(&view) {
            debug!("no block");
            return;
        };

        let first_proposal_of_epoch =
            is_last_block(header.block_number().saturating_sub(1), *self.epoch_height);
        let proposal_epoch = if first_proposal_of_epoch {
            proposal.epoch + 1
        } else {
            proposal.epoch
        };
        if !self.is_leader(view, proposal_epoch) {
            warn!(epoch = %proposal_epoch, "not the leader for this view, we should not have a header");
            return;
        }

        // Epoch 1 is the genesis epoch and has no successor that needs a
        // DRB from a transition leaf — `set_first_epoch` pre-loads DRBs for
        // `first_epoch` and `first_epoch + 1`.  Every epoch beyond that
        // communicates its successor's DRB via `next_drb_result` on each
        // leaf in its transition zone; the successor epoch's catchup
        // unwraps that field, so leaving it `None` here would panic
        // peers that fetch this leaf.
        let next_drb_result = if proposal.epoch > EpochNumber::genesis()
            && is_epoch_transition(header.block_number(), *self.epoch_height)
        {
            let Some(drb) = self.drb_results.get(&EpochNumber::new(*proposal.epoch + 1)) else {
                debug!(%proposal.epoch, "no DRB result for epoch");
                // Keep retrying — the epoch manager dedups pending requests,
                // but if an earlier catchup failed (e.g. Leaf2Fetcher
                // timeout under CPU load) nothing else kicks the request.
                outbox.push_back(ConsensusOutput::RequestDrbResult(proposal.epoch + 1));
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

        // If the parent QC is for an epoch-root block, attach the state_cert.
        // By the atomicity invariant (enforced by `EpochRootVoteCollector`),
        // if we hold the epoch-root Cert1 then `state_certs` also holds the
        // matching cert.
        let parent_block_number = parent_cert.data.block_number.unwrap_or(0);
        let state_cert = if is_epoch_root(parent_block_number, *self.epoch_height) {
            let Some(parent_epoch) = parent_cert.data.epoch() else {
                warn!("epoch-root parent QC has no epoch; cannot propose");
                return;
            };
            let Some(sc) = self.state_certs.get(&parent_epoch).cloned() else {
                warn!(
                    %view,
                    "epoch-root parent QC without state_cert — atomicity invariant broken; skipping propose"
                );
                return;
            };
            if !check_qc_state_cert_correspondence(parent_cert, &sc, *self.epoch_height) {
                warn!(%view, "state_cert does not correspond to parent QC; skipping propose");
                return;
            }
            Some(sc)
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
            state_cert,
        };

        // Sign the proposal
        let proposed_leaf: Leaf2<T> = proposal.clone().into();
        let leaf_commit = proposed_leaf.commit();
        crate::trace_leader_event!(
            self.tracer,
            view,
            crate::leader_trace::LeaderEvent::Leaf2CommitComputed
        );
        let signature = match T::SignatureKey::sign(&self.private_key, leaf_commit.as_ref()) {
            Ok(sig) => sig,
            Err(err) => {
                warn!(%view, %err, "failed to sign proposal");
                return;
            },
        };
        crate::trace_leader_event!(
            self.tracer,
            view,
            crate::leader_trace::LeaderEvent::ProposalSigned
        );

        let message = SignedProposal {
            data: proposal,
            signature,
            _pd: PhantomData,
        };

        self.proposed_views.insert(view);
        outbox.push_back(ConsensusOutput::SendProposal(message));
        crate::trace_leader_event!(
            self.tracer,
            view,
            crate::leader_trace::LeaderEvent::ProposalQueued
        );
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
        // Decide requires the block to be locally available and its
        // commitment to match the proposal.
        if !self.block_matches_proposal(view, proposal) {
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
        let mut leaf: Leaf2<T> = proposal.clone().into();
        if let Some(payload) = self.blocks.get(&view) {
            leaf.fill_block_payload_unchecked(payload.clone());
        }
        let new_decided_view = max(self.last_decided_view, leaf.view_number());
        let last_decided_leaf = leaf.clone();
        let mut gc = None;
        if leaf.block_header().block_number() % *self.garbage_collection_interval == 0 {
            gc = Some((leaf.view_number(), leaf.justify_qc().epoch()));
        }
        let mut decided = vec![leaf];
        let mut vid_shares = vec![self.signed_vid_share(view)];

        let mut parent_view = proposal.justify_qc.view_number();
        let mut parent_commit = proposal.justify_qc.data.leaf_commit;

        while parent_view > self.last_decided_view
            && let Some(proposal) = self.proposals.get(&parent_view)
        {
            let proposal_commit = proposal_commitment(proposal);
            if proposal_commit != parent_commit {
                break;
            }
            let mut leaf: Leaf2<T> = proposal.clone().into();
            if let Some(payload) = self.blocks.get(&parent_view) {
                leaf.fill_block_payload_unchecked(payload.clone());
            }
            if gc.is_none()
                && leaf.block_header().block_number() % *self.garbage_collection_interval == 0
            {
                gc = Some((leaf.view_number(), leaf.justify_qc().epoch()));
            }
            vid_shares.push(self.signed_vid_share(parent_view));
            decided.push(leaf);
            parent_view = proposal.justify_qc.view_number();
            parent_commit = proposal.justify_qc.data.leaf_commit;
        }
        self.last_decided_view = new_decided_view;
        self.last_decided_leaf = last_decided_leaf;
        let Some(cert1) = self.certs.get(&view).cloned() else {
            debug!(%view, "cert1 missing");
            return;
        };
        crate::trace_leader_event!(
            self.tracer,
            view,
            crate::leader_trace::LeaderEvent::LeafDecided
        );
        outbox.push_back(ConsensusOutput::LeafDecided {
            leaves: decided,
            cert1,
            cert2: Some(cert2.clone()),
            vid_shares,
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

    /// Build a `LightClientStateUpdateVote2` for an epoch-root leaf.
    ///
    /// Computes the `LightClientState` from the header, fetches the next-epoch
    /// stake-table commitment, and signs both the LCV2 (pre-upgrade, for
    /// backward compatibility with existing relay infrastructure) and LCV3
    /// (current) Schnorr signatures.
    fn build_state_vote(
        &self,
        proposal: &Proposal<T>,
    ) -> anyhow::Result<LightClientStateUpdateVote2<T>> {
        let view_number = proposal.view_number;
        let light_client_state = proposal
            .block_header
            .get_light_client_state(view_number)
            .map_err(|e| anyhow::anyhow!("failed to generate light client state: {e}"))?;
        let auth_root = proposal
            .block_header
            .auth_root()
            .map_err(|e| anyhow::anyhow!("failed to fetch auth root: {e}"))?;
        let membership = self
            .stake_table_coordinator
            .membership_for_epoch(Some(proposal.epoch))
            .map_err(|e| anyhow::anyhow!("membership lookup failed: {e}"))?;
        let next_stake_table = membership
            .next_epoch_stake_table()
            .map_err(|e| anyhow::anyhow!("next-epoch stake table lookup failed: {e}"))?;
        let next_stake_table_state = HSStakeTable::from_iter(next_stake_table.stake_table())
            .commitment(self.stake_table_capacity)
            .map_err(|e| anyhow::anyhow!("failed to compute stake table commitment: {e}"))?;
        let v2_signature = <T::StateSignatureKey as LCV2StateSignatureKey>::sign_state(
            &self.state_private_key,
            &light_client_state,
            &next_stake_table_state,
        )
        .map_err(|e| anyhow::anyhow!("failed to sign LCV2 state: {e}"))?;
        let signed_state_digest =
            derive_signed_state_digest(&light_client_state, &next_stake_table_state, &auth_root);
        let signature = <T::StateSignatureKey as LCV3StateSignatureKey>::sign_state(
            &self.state_private_key,
            signed_state_digest,
        )
        .map_err(|e| anyhow::anyhow!("failed to sign LCV3 state: {e}"))?;
        Ok(LightClientStateUpdateVote2 {
            epoch: proposal.epoch,
            light_client_state,
            next_stake_table_state,
            signature,
            v2_signature,
            auth_root,
            signed_state_digest,
        })
    }

    #[instrument(level = "debug", skip_all)]
    fn maybe_vote_1(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
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

        // Don't vote for epoch-transition proposals until we can verify
        // the attached DRB result.  Same guard as `maybe_propose`:
        // transitions in epoch >= 2 must carry `next_drb_result`.
        let block_number = proposal.block_header.block_number();
        if proposal.epoch > EpochNumber::genesis()
            && is_epoch_transition(block_number, *self.epoch_height)
        {
            let Some(drb) = self.drb_results.get(&(proposal.epoch + 1)) else {
                debug!("DRB result not yet available, deferring vote");
                return;
            };
            if proposal
                .next_drb_result
                .is_none_or(|proposed_drb| drb != &proposed_drb)
            {
                warn!("DRB result does not match proposal, refusing to vote");
                return;
            }
        }

        if !self.staked_in_epoch(proposal.epoch) {
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
            // Parent block must be locally available: either built locally
            // (we proposed it) or reconstructed via shares/push.
            let locally_built = self.blocks.contains_key(&parent_view);
            let reconstructed_commit = self.blocks_reconstructed.get(&parent_view);
            if !locally_built && reconstructed_commit.is_none() {
                debug!(%parent_view, "block commitment not available");
                return;
            }
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
            // Only do cross-checking for the reconstructed block.
            if let Some(commit) = reconstructed_commit
                && commit != &prev_block_commitment
            {
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
            &self.upgrade_lock,
        ) {
            Ok(vote) => vote,
            Err(err) => {
                warn!(%view, %err, "failed to created signed vote for proposal");
                return;
            },
        };

        let state_vote = if is_epoch_root(proposal.block_header.block_number(), *self.epoch_height)
        {
            match self.build_state_vote(proposal) {
                Ok(sv) => Some(sv),
                Err(err) => {
                    warn!(%view, %err, "failed to build state vote for epoch-root leaf; skipping vote1");
                    return;
                },
            }
        } else {
            None
        };

        let vote = Vote1 {
            vote: inner_vote,
            vid_share: vid_share.clone(),
            state_vote,
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
        let Some(cert1) = self.certs.get(&view).cloned() else {
            debug!("cert1 not available");
            return;
        };

        // Vote2 and lock-update share a common prerequisite: we must have a
        // local proposal that matches cert1's leaf commit.
        let Some(proposal) = self.proposals.get(&view) else {
            debug!(%view, "proposal not available; vote2 and lock deferred");
            return;
        };
        let proposal_epoch = proposal.epoch;
        let proposal_commit = proposal_commitment(proposal);
        if cert1.data.leaf_commit != proposal_commit {
            warn!(%view, "cert1 commitment does not match proposal commitment");
            return;
        }

        // Vote2: block not required.
        if !self.voted_2_views.contains(&view) {
            match (cert1.data.epoch, cert1.data.block_number) {
                (Some(epoch), Some(block_number)) if self.staked_in_epoch(epoch) => {
                    match SimpleVote::create_signed_vote(
                        Vote2Data {
                            leaf_commit: cert1.data.leaf_commit,
                            epoch,
                            block_number,
                        },
                        view,
                        &self.public_key,
                        &self.private_key,
                        &self.upgrade_lock,
                    ) {
                        Ok(vote) => {
                            crate::trace_leader_event!(
                                self.tracer,
                                view,
                                crate::leader_trace::LeaderEvent::Vote2VMinus1Signed
                            );
                            outbox.push_back(ConsensusOutput::SendVote2(vote));
                            crate::trace_leader_event!(
                                self.tracer,
                                view,
                                crate::leader_trace::LeaderEvent::Vote2VMinus1Queued
                            );
                            self.voted_2_views.insert(view);
                        },
                        Err(err) => {
                            warn!(%view, %err, "failed to create signed vote2");
                        },
                    }
                },
                (Some(_), Some(_)) => {
                    debug!(%view, "vote2 skipped: not staked in cert1's epoch");
                },
                _ => {
                    debug!(%view, "vote2 skipped: cert1 missing epoch or block_number");
                },
            }
        }

        // Lock update / ViewChanged / SendCertificate1: additionally require
        // a locally available block whose commitment matches the proposal.
        if !self.block_matches_proposal(view, proposal) {
            return;
        }
        if self
            .locked_cert
            .as_mut()
            .is_none_or(|locked_cert| locked_cert.view_number() < cert1.view_number())
        {
            self.locked_cert = Some(cert1.clone());
            self.current_epoch = Some(proposal_epoch);
            outbox.push_back(ConsensusOutput::ViewChanged(view + 1, proposal_epoch));
            outbox.push_back(ConsensusOutput::SendCertificate1(cert1));
        }
    }

    /// Returns true when a payload matching the proposal's commitment is
    /// locally available.
    fn block_matches_proposal(&self, view: ViewNumber, proposal: &Proposal<T>) -> bool {
        if self.blocks.contains_key(&view) {
            return true;
        }
        let VidCommitment::V2(expected) = proposal.block_header.payload_commitment() else {
            warn!(%view, "proposal payload commitment is not a V2 VID commitment");
            return false;
        };
        match self.blocks_reconstructed.get(&view) {
            Some(c) if c == &expected => true,
            Some(_) => {
                warn!(%view, "reconstructed block commitment does not match proposal");
                false
            },
            None => {
                debug!(%view, "block not yet locally available");
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
    fn verify_cert<A, C>(&self, cert: &C, epoch: EpochNumber) -> bool
    where
        C: vote::Certificate<T, A>,
    {
        match self
            .stake_table_coordinator
            .membership_for_epoch(Some(epoch))
        {
            Ok(stake_table) => {
                let entries = StakeTableEntries::from_iter(stake_table.stake_table()).0;
                let threshold = stake_table.success_threshold();
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

    /// Try to verify a certificate, distinguishing between "epoch not available"
    /// and "cryptographically invalid".
    #[instrument(level = "trace", skip_all)]
    fn try_verify_cert<A, C>(&self, cert: &C, epoch: EpochNumber) -> CertVerification
    where
        C: vote::Certificate<T, A>,
    {
        match self
            .stake_table_coordinator
            .membership_for_epoch(Some(epoch))
        {
            Ok(stake_table) => {
                let entries = StakeTableEntries::from_iter(stake_table.stake_table()).0;
                let threshold = stake_table.success_threshold();
                match cert.is_valid_cert(&entries, threshold, &self.upgrade_lock) {
                    Ok(()) => CertVerification::Valid,
                    Err(err) => {
                        warn!(%epoch, %err, "invalid threshold signature");
                        CertVerification::Invalid
                    },
                }
            },
            Err(_) => CertVerification::EpochUnavailable,
        }
    }

    /// Retry verification of pending certificates whose epoch may now be available.
    fn retry_pending_certs(&mut self, outbox: &mut Outbox<ConsensusOutput<T>>) {
        // Retry pending cert1s.
        let pending = std::mem::take(&mut self.pending_certs1);
        for (view, cert) in pending {
            self.handle_certificate1(cert.clone(), outbox);
            self.maybe_vote_2_and_update_lock(view, outbox);
            self.maybe_propose(view, outbox);
        }

        // Retry pending cert2s.
        let pending = std::mem::take(&mut self.pending_certs2);
        for (view, cert) in pending {
            self.handle_certificate2(cert.clone(), outbox);
            self.maybe_decide(view, outbox);
            self.maybe_propose(view, outbox);
        }
    }

    #[instrument(level = "trace", skip_all)]
    fn is_leader(&self, view: ViewNumber, epoch: EpochNumber) -> bool {
        match self
            .stake_table_coordinator
            .membership_for_epoch(Some(epoch))
        {
            Ok(stake_table) => match stake_table.leader(view) {
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

    fn staked_in_epoch(&self, epoch: EpochNumber) -> bool {
        match self
            .stake_table_coordinator
            .membership_for_epoch(Some(epoch))
        {
            Ok(stake_table) => stake_table.has_stake(&self.public_key),
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
            ConsensusInput::EpochRootCertificates { cert1, .. } => cert1.view_number(),
            ConsensusInput::HeaderCreated(view, ..) => *view,
            ConsensusInput::ProposalWithVidShare(_, prop, _) => prop.view_number(),
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
            // DRB results arrive asynchronously and don't belong to any
            // particular view; `apply` handles routing by using
            // `current_view` for this variant.
            ConsensusInput::DrbResult(..) => ViewNumber::genesis(),
            ConsensusInput::EpochChange(epoch_change) => epoch_change.cert1.view_number(),
        }
    }
}
