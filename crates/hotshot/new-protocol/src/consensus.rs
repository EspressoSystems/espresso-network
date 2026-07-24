use std::{
    cmp::max,
    collections::{BTreeMap, BTreeSet},
    marker::PhantomData,
    sync::Arc,
};

use committable::{Commitment, CommitmentBoundsArkless, Committable};
use hotshot::traits::BlockPayload;
use hotshot_contract_adapter::light_client::derive_signed_state_digest;
use hotshot_types::{
    data::{
        BlockNumber, EpochNumber, Leaf2, VidCommitment, VidCommitment2, VidDisperseShare2,
        ViewChangeEvidence2, ViewNumber,
    },
    drb::DrbResult,
    epoch_membership::EpochMembershipCoordinator,
    message::{Proposal as SignedProposal, UpgradeLock},
    simple_certificate::{
        LightClientStateUpdateCertificateV2, QuorumCertificate2, TimeoutCertificate2,
        check_qc_state_cert_correspondence,
    },
    simple_vote::{
        HasEpoch, LightClientStateUpdateVote2, QuorumData2, SimpleVote, TimeoutData2, TimeoutVote2,
        Vote2Data,
    },
    stake_table::HSStakeTable,
    traits::{
        block_contents::BlockHeader,
        node_implementation::NodeType,
        signature_key::{
            LCV2StateSignatureKey, LCV3StateSignatureKey, SignatureKey, StateSignatureKey,
        },
    },
    utils::{epoch_from_block_number, is_epoch_root, is_epoch_transition, is_last_block},
    vote::{Certificate, HasViewNumber},
};
use hotshot_utils::anytrace;
use tracing::{debug, info, instrument, warn};

use crate::{
    block::BlockAndHeaderRequest,
    cert_verifier::ValidCert,
    coordinator::{GcScope, VID_RECONSTRUCT_GC_MARGIN},
    helpers::proposal_commitment,
    logging::KeyPrefix,
    message::{
        CatchupEvidence, Certificate1, Certificate2, EpochChangeMessage, Proposal,
        ProposalFetchRequest, ProposalMessage, Validated, Vote1, Vote2,
    },
    outbox::Outbox,
    state::{StateRequest, StateResponse},
    storage::{ActionKind, StorageOutput},
};

/// Inputs to [`Consensus::apply_pre_cutover_seed`].
///
/// Carries everything the new protocol needs to take over from the legacy
/// stack at a decided upgrade boundary: the highest legacy-decided leaf,
/// the legacy undecided chain above it, the legacy `high_qc` (if any),
/// the validated states for those leaves, and the upgrade certificate's
/// `new_version_first_view`.
#[derive(Clone, Debug)]
pub struct PreCutoverSeed<T: NodeType> {
    /// Highest leaf legacy decided. Anchors `last_decided_view`.
    pub decided_anchor: Leaf2<T>,
    /// Legacy undecided chain above the anchor, oldest-first.
    pub undecided: Vec<Leaf2<T>>,
    /// Legacy `high_qc`. `None` is allowed for cold-start tests; production
    /// seed extraction always supplies one.
    pub high_qc: Option<QuorumCertificate2<T>>,
    /// Validated states keyed by view, for the anchor and every undecided leaf.
    pub validated_states: BTreeMap<ViewNumber, Arc<T::ValidatedState>>,
    /// `upgrade_cert.new_version_first_view`. `current_view`/`timeout_view`
    /// are advanced to `cutover_view - 1` so the new protocol's normal
    /// proposal/timeout machinery takes over at `cutover_view`.
    pub cutover_view: ViewNumber,
}

#[derive(Eq, PartialEq, Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum ConsensusInput<T: NodeType> {
    BlockBuilt {
        view: ViewNumber,
        epoch: EpochNumber,
        payload: T::BlockPayload,
        metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
        payload_commitment: VidCommitment,
    },
    BlockReconstructed(ViewNumber, VidCommitment2),
    Certificate1(ValidCert<Certificate1<T>>),
    Certificate2(ValidCert<Certificate2<T>>),
    /// A quorum certificate allows us to advance our view.
    ///
    /// Used to help divergent nodes re-converge on restart.
    AdvanceView(ValidCert<Certificate1<T>>),
    /// Atomic pair emitted by the `EpochRootTally` for epoch-root views:
    /// a `Certificate1` and its matching `LightClientStateUpdateCertificateV2`.
    /// Consensus never sees an epoch-root Cert1 without the matching state_cert.
    EpochRootCertificates {
        cert1: ValidCert<Certificate1<T>>,
        state_cert: LightClientStateUpdateCertificateV2<T>,
    },
    EpochChange(EpochChangeMessage<T, Validated>),
    HeaderCreated(ViewNumber, Commitment<Leaf2<T>>, T::BlockHeader),
    /// A validated proposal. Consensus parks it until this node's VID share
    /// for the same payload arrives ([`ConsensusInput::VidShare`]) and only
    /// processes the two together.
    Proposal(T::SignatureKey, ProposalMessage<T, Validated>),
    /// This node's validated VID share.
    VidShare(VidDisperseShare2<T>),
    FetchedProposal(ProposalMessage<T, Validated>),
    StateValidated(StateResponse<T>),
    StateValidationFailed(StateResponse<T>),
    Stored(StorageOutput<T>),
    Timeout(ViewNumber, EpochNumber),
    TimeoutCertificate(ValidCert<TimeoutCertificate2<T>>),
    TimeoutOneHonest(ViewNumber, EpochNumber),
    VidDisperseCreated(ViewNumber, VidCommitment2),
    DrbResult(EpochNumber, DrbResult),
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum ConsensusOutput<T: NodeType> {
    RequestBlockAndHeader(BlockAndHeaderRequest<T>),
    RequestState(StateRequest<T>),
    RequestDrbResult(EpochNumber),
    RecordAction(ViewNumber, Option<EpochNumber>, ActionKind),
    PersistProposal(SignedProposal<T, Proposal<T>>),
    SendProposal(SignedProposal<T, Proposal<T>>),
    SendTimeoutVote(TimeoutVote2<T>, Option<CatchupEvidence<T>>),
    SendVote1(Vote1<T>),
    SendVote2(Vote2<T>),
    /// Persist the locked QC before the matching phase-2 vote is released.
    PersistHighQc(Certificate1<T>),
    SendTimeoutCertificate(TimeoutCertificate2<T>, ViewNumber, EpochNumber),
    SendCertificate1(Certificate1<T>),
    /// Broadcast a first-obtained Cert2 so peers that could not assemble it
    /// from votes can still decide. Mirrors `SendCertificate1`.
    SendCertificate2(Certificate2<T>),
    SendEpochChange(EpochChangeMessage<T, Validated>),
    RequestVidDisperse {
        view: ViewNumber,
        epoch: EpochNumber,
        payload: T::BlockPayload,
        metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
        payload_commitment: VidCommitment2,
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
    /// A view timed out with a timeout certificate.
    ViewTimedOut(ViewNumber),
    /// A validated proposal met this node's VID share.
    ProposalPaired {
        proposal: SignedProposal<T, Proposal<T>>,
        vid_share: VidDisperseShare2<T>,
    },
    ProposalValidated {
        proposal: SignedProposal<T, Proposal<T>>,
        sender: T::SignatureKey,
    },
    RequestMissingProposal {
        view: ViewNumber,
        leaf_commit: Commitment<Leaf2<T>>,
    },
    /// Emitted when a node has reconstructed a block payload from VID shares.
    /// Notifies downstream consumers (e.g. the query service) so they can store
    /// the payload even if the corresponding view has already been decided
    /// without a payload in the decide event.
    BlockPayloadReconstructed {
        view: ViewNumber,
        header: T::BlockHeader,
        payload: T::BlockPayload,
    },
    /// Broadcast our own VID share so peers can reconstruct the block. Emitted
    /// right after `SendVote1` so it never delays the cert-forming vote.
    BroadcastVidShare(VidDisperseShare2<T>),
}

type UnpairedProposals<T> = BTreeMap<
    (ViewNumber, VidCommitment2),
    (<T as NodeType>::SignatureKey, ProposalMessage<T, Validated>),
>;

type UnpairedVidShares<T> = BTreeMap<(ViewNumber, VidCommitment2), VidDisperseShare2<T>>;

/// Views to retain decide inputs (`proposals`, `certs`, `certs2`) behind the
/// decided view, letting a late-broadcast Cert2 decide an older gap view.
pub(crate) const DECIDE_BUFFER: u64 = 20;

// The decide buffer retains the proposals the VID reconstructor reads.
const _: () = assert!(DECIDE_BUFFER >= VID_RECONSTRUCT_GC_MARGIN);

pub struct Consensus<T: NodeType> {
    proposals: BTreeMap<ViewNumber, Proposal<T>>,
    signed_proposals: BTreeMap<ViewNumber, SignedProposal<T, Proposal<T>>>,
    proposed_views: BTreeSet<ViewNumber>,
    vid_shares: BTreeMap<ViewNumber, VidDisperseShare2<T>>,
    unpaired_proposals: UnpairedProposals<T>,
    unpaired_vid_shares: UnpairedVidShares<T>,
    states_verified: BTreeMap<ViewNumber, Commitment<Leaf2<T>>>,
    blocks_reconstructed: BTreeSet<(ViewNumber, VidCommitment2)>,
    blocks: BTreeMap<(ViewNumber, VidCommitment2), T::BlockPayload>,
    certs: BTreeMap<ViewNumber, Certificate1<T>>,
    certs2: BTreeMap<ViewNumber, Certificate2<T>>,
    timeout_certs: BTreeMap<ViewNumber, TimeoutCertificate2<T>>,
    locked_cert: Option<Certificate1<T>>,
    headers: BTreeMap<(ViewNumber, Commitment<Leaf2<T>>), T::BlockHeader>,
    leaves: BTreeMap<ViewNumber, Leaf2<T>>,
    /// Views actually emitted in a `LeafDecided`; once views can decide late,
    /// `last_decided_view` is only a high-water mark.
    decided_views: BTreeSet<ViewNumber>,
    /// Hard lower bound for deciding, pinned to the anchor on restart/cutover:
    /// `decided_views` is not persisted, so a replayed certificate pair could
    /// otherwise re-decide pre-anchor views.
    decide_floor_view: ViewNumber,
    last_decided_view: ViewNumber,
    last_decided_leaf: Leaf2<T>,
    drb_results: BTreeMap<EpochNumber, DrbResult>,

    voted_1_views: BTreeSet<ViewNumber>,
    voted_2_views: BTreeSet<ViewNumber>,

    /// Storage confirmations; sends are gated on these facts.
    stored_proposals: BTreeMap<ViewNumber, Vec<Commitment<Leaf2<T>>>>,
    stored_vids: BTreeSet<ViewNumber>,
    stored_actions: BTreeSet<(ViewNumber, ActionKind)>,
    requested_actions: BTreeSet<(ViewNumber, ActionKind)>,
    /// Highest locked-QC view confirmed persisted; gates release of phase-2 votes.
    stored_high_qc: Option<ViewNumber>,

    /// Messages constructed and accounted for in `voted_*_views` /
    /// `proposed_views`, awaiting their storage confirmations. A pending vote2
    /// also records the locked-QC view it must see persisted before release.
    pending_vote1: BTreeMap<ViewNumber, Vote1<T>>,
    pending_vote2: BTreeMap<ViewNumber, (Vote2<T>, ViewNumber)>,
    pending_proposal: BTreeMap<ViewNumber, SignedProposal<T, Proposal<T>>>,

    /// Skipped by `maybe_vote_2_and_update_lock` (V1 AvidM dispersal).
    pre_cutover_views: BTreeSet<ViewNumber>,

    timeout_view: ViewNumber,
    /// Highest view this node may have acted in before a restart (from the
    /// persisted action log). Bars re-*recording* a view's Vote action, not
    /// re-casting the phase-2 vote itself (see [`Self::vote2_persisted`]).
    restart_barred_view: ViewNumber,
    current_view: ViewNumber,
    current_epoch: Option<EpochNumber>,

    // TODO: We need a next epoch stake table to handle the transition
    // And a way to set these stake tables, probably an event from coordinator
    stake_table_coordinator: EpochMembershipCoordinator<T>,

    public_key: T::SignatureKey,
    private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
    state_private_key: <T::StateSignatureKey as StateSignatureKey>::StatePrivateKey,
    stake_table_capacity: usize,
    state_certs: BTreeMap<EpochNumber, LightClientStateUpdateCertificateV2<T>>,
    node_id: KeyPrefix,
    upgrade_lock: UpgradeLock<T>,

    pub(crate) epoch_height: BlockNumber,
}

/// Protocol flow directive.
enum Protocol {
    /// Stop with further protocol steps.
    Abort,
    /// Continue with protocol.
    Continue,
}

/// Reason a proposal failed the safety/liveness rule.
#[derive(Debug, thiserror::Error)]
enum SafetyError {
    #[error(
        "leaf commitment at locked view does not match locked certificate \
         locked_commit={locked_commit} proposal_commit={proposal_commit}"
    )]
    LockedViewCommitmentMismatch {
        locked_commit: String,
        proposal_commit: String,
    },
    #[error(
        "justify qc neither extends nor is newer than the locked certificate \
         locked_view={locked_view} parent_commit={parent_commit} locked_commit={locked_commit}"
    )]
    UnsafeProposal {
        locked_view: ViewNumber,
        parent_commit: String,
        locked_commit: String,
    },
    #[error("failed to compute justify qc data commitment: {0}")]
    JustifyQcCommitment(#[source] anytrace::Error),
    #[error("failed to compute locked certificate data commitment: {0}")]
    LockedCertCommitment(#[source] anytrace::Error),
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
    ) -> Self
    where
        B: Into<BlockNumber>,
    {
        let last_decided_view = genesis_leaf.view_number();
        Self {
            proposals: BTreeMap::new(),
            signed_proposals: BTreeMap::new(),
            proposed_views: BTreeSet::new(),
            blocks: BTreeMap::new(),
            states_verified: BTreeMap::new(),
            blocks_reconstructed: BTreeSet::new(),
            certs: BTreeMap::new(),
            certs2: BTreeMap::new(),
            timeout_certs: BTreeMap::new(),
            locked_cert: None,
            leaves: BTreeMap::new(),
            decided_views: BTreeSet::from([last_decided_view]),
            decide_floor_view: ViewNumber::genesis(),
            last_decided_view,
            last_decided_leaf: genesis_leaf,
            headers: BTreeMap::new(),
            drb_results: BTreeMap::new(),
            node_id: KeyPrefix::from(&public_key),
            public_key,
            timeout_view: ViewNumber::genesis(),
            restart_barred_view: ViewNumber::genesis(),
            current_view: ViewNumber::genesis(),
            current_epoch: None,
            stake_table_coordinator: membership_coordinator,
            voted_1_views: BTreeSet::new(),
            voted_2_views: BTreeSet::new(),
            stored_proposals: BTreeMap::new(),
            stored_vids: BTreeSet::new(),
            stored_actions: BTreeSet::new(),
            requested_actions: BTreeSet::new(),
            stored_high_qc: None,
            pending_vote1: BTreeMap::new(),
            pending_vote2: BTreeMap::new(),
            pending_proposal: BTreeMap::new(),
            pre_cutover_views: BTreeSet::new(),
            private_key,
            state_private_key,
            stake_table_capacity,
            state_certs: BTreeMap::new(),
            upgrade_lock,
            vid_shares: BTreeMap::new(),
            unpaired_proposals: BTreeMap::new(),
            unpaired_vid_shares: BTreeMap::new(),
            epoch_height: epoch_height.into(),
        }
    }

    /// Seed a parent certificate and proposal so the leader of the *next* view
    /// can propose without any external bootstrap injection.
    /// Sets the locked certificate and current epoch. After calling this, a
    /// subsequent `apply` that triggers `maybe_propose` will find the
    /// parent cert and proposal it needs.
    ///
    /// `reconstructed` are `(view, V2 commitment)` pairs to record as
    /// already reconstructed blocks. During normal operation this set is
    /// populated as VID shares arrive; on restart it starts empty, but the
    /// persisted leaf and proposals correspond to blocks this node had already
    /// reconstructed in the previous process. Seeding them lets a restarted
    /// leader satisfy the `parent_block_reconstructed` check for its first
    /// proposal/vote instead of stalling.
    pub fn seed_parent(
        &mut self,
        cert1: Certificate1<T>,
        proposal: Proposal<T>,
        reconstructed: impl IntoIterator<Item = (ViewNumber, VidCommitment2)>,
    ) {
        self.current_epoch = Some(proposal.epoch);
        // The seed cert comes from persistent storage, so its lock is already persisted.
        self.bump_stored_high_qc(cert1.view_number());
        self.certs.insert(cert1.view_number(), cert1.clone());
        self.locked_cert = Some(cert1);
        self.proposals.insert(proposal.view_number, proposal);
        for (view, commitment) in reconstructed {
            self.blocks_reconstructed.insert((view, commitment));
        }
    }

    /// Seed proposals loaded from storage on restart so the decide chain-walk
    /// can follow `justify_qc` back through views the node had already seen,
    /// and so `maybe_vote_1`/`maybe_propose` can find the parent of the first
    /// post-restart proposal (otherwise never re-fetched).
    pub fn seed_proposals(&mut self, proposals: impl IntoIterator<Item = Proposal<T>>) {
        for proposal in proposals {
            let view = proposal.view_number;
            self.leaves.insert(view, proposal.clone().into());
            self.proposals.insert(view, proposal);
        }
    }

    /// Restore the locked QC persisted on a prior run. Called after
    /// `seed_parent`: the persisted lock can be newer than the decided-anchor
    /// QC, and restoring it keeps the node from voting against a block it had
    /// already locked. The loaded value is persisted, so it also advances the
    /// persistence watermark.
    pub fn seed_locked_cert(&mut self, cert1: Certificate1<T>) {
        let view = cert1.view_number();
        self.bump_stored_high_qc(view);
        self.certs.entry(view).or_insert_with(|| cert1.clone());
        if self
            .locked_cert
            .as_ref()
            .is_none_or(|locked| locked.view_number() < view)
        {
            self.locked_cert = Some(cert1);
        }
    }

    /// Advance the locked-QC persistence watermark to `view` if it is newer.
    fn bump_stored_high_qc(&mut self, view: ViewNumber) {
        if self.stored_high_qc.is_none_or(|cur| cur < view) {
            self.stored_high_qc = Some(view);
        }
    }

    /// Whether the locked QC at `required` view is persisted.
    fn high_qc_persisted(&self, required: ViewNumber) -> bool {
        self.stored_high_qc.is_some_and(|stored| stored >= required)
    }

    /// Whether the parent block counts as reconstructed for voting: either we
    /// hold its payload, or our locked QC certifies exactly this parent leaf.
    /// A lock is only ever taken on a reconstructed block, so a lock matching
    /// the parent is itself proof of reconstruction — letting a restarted node
    /// vote on the first proposal built on its restored lock.
    fn parent_reconstructed(
        &self,
        parent_view: ViewNumber,
        parent_block_commitment: VidCommitment2,
        parent_leaf: Commitment<Leaf2<T>>,
    ) -> bool {
        self.blocks_reconstructed
            .contains(&(parent_view, parent_block_commitment))
            || self.locked_cert.as_ref().is_some_and(|lock| {
                lock.view_number() == parent_view && lock.data().leaf_commit == parent_leaf
            })
    }

    /// Seed a state certificate loaded from storage on restart, so a leader
    /// proposing on an epoch-root parent QC right after a restart does not
    /// stall on a missing state_cert.
    pub fn seed_state_cert(&mut self, state_cert: LightClientStateUpdateCertificateV2<T>) {
        self.state_certs.insert(state_cert.epoch, state_cert);
    }

    /// Apply a [`PreCutoverSeed`] to bridge legacy state into the new
    /// protocol. Performs the four operations the seed describes
    /// atomically: anchor the decided view, install the undecided
    /// leaves so they can be decided via Cert2, register the legacy
    /// high_qc, and advance `current_view`/`timeout_view` to the
    /// pre-cutover frontier.
    ///
    /// Idempotent: calling with the same seed twice (or with an older
    /// seed) does not regress decided/locked state.
    pub fn apply_pre_cutover_seed(&mut self, seed: PreCutoverSeed<T>) {
        let view = seed.decided_anchor.view_number();
        if view > self.last_decided_view {
            self.last_decided_view = view;
            self.last_decided_leaf = seed.decided_anchor.clone();
            self.decided_views.insert(view);
        }
        if view > self.decide_floor_view {
            self.decide_floor_view = view;
        }

        let mut highest_seeded_block: u64 = seed.decided_anchor.block_header().block_number();

        for leaf in seed.undecided {
            let view = leaf.view_number();
            let justify_qc = leaf.justify_qc().clone();
            self.register_legacy_qc(&justify_qc);

            let block_number = leaf.block_header().block_number();
            let epoch = EpochNumber::new(epoch_from_block_number(block_number, *self.epoch_height));
            if block_number > highest_seeded_block {
                highest_seeded_block = block_number;
            }

            let view_change_evidence = leaf.view_change_evidence.clone().and_then(|e| match e {
                ViewChangeEvidence2::Timeout(tc) => Some(tc),
                ViewChangeEvidence2::ViewSync(_) => None,
            });
            let proposal = Proposal {
                block_header: leaf.block_header().clone(),
                view_number: view,
                epoch,
                justify_qc,
                next_epoch_justify_qc: None,
                upgrade_certificate: leaf.upgrade_certificate().clone(),
                view_change_evidence,
                next_drb_result: leaf.next_drb_result,
                state_cert: None,
            };

            self.leaves.insert(view, leaf);
            self.proposals.insert(view, proposal);
            self.pre_cutover_views.insert(view);

            self.proposed_views.insert(view);
            self.voted_1_views.insert(view);
            self.voted_2_views.insert(view);
        }

        if let Some(high_qc) = &seed.high_qc {
            self.register_legacy_qc(high_qc);
        }

        let cutover_view = seed.cutover_view;
        if cutover_view == ViewNumber::genesis() {
            return;
        }
        let last_pre_cutover = cutover_view - 1;
        if last_pre_cutover > self.timeout_view {
            self.timeout_view = last_pre_cutover;
        }
        if last_pre_cutover > self.current_view {
            self.current_view = last_pre_cutover;
        }
        let seeded_epoch = EpochNumber::new(epoch_from_block_number(
            highest_seeded_block,
            *self.epoch_height,
        ));
        if self.current_epoch.is_none_or(|cur| cur < seeded_epoch) {
            self.current_epoch = Some(seeded_epoch);
        }
    }

    /// Register `justify_qc` as Cert1 for its parent view (idempotent)
    /// and bump `locked_cert` if newer.
    pub(crate) fn register_legacy_qc(&mut self, justify_qc: &Certificate1<T>) {
        let parent_view = justify_qc.view_number();
        self.certs
            .entry(parent_view)
            .or_insert_with(|| justify_qc.clone());
        if self
            .locked_cert
            .as_ref()
            .is_none_or(|locked| locked.view_number() < parent_view)
        {
            self.locked_cert = Some(justify_qc.clone());
        }
    }

    /// Return the proposal stored at the given view, if any.
    pub fn proposal_at(&self, view: ViewNumber) -> Option<&Proposal<T>> {
        self.proposals.get(&view)
    }

    /// Return the Certificate1 (QC) stored at the given view, if any.
    pub fn cert1_at(&self, view: ViewNumber) -> Option<&Certificate1<T>> {
        self.certs.get(&view)
    }

    /// The highest certificate we hold: the locked QC or the latest timeout
    /// certificate, whichever has the higher view (ties go to the timeout
    /// certificate).
    pub fn catchup_evidence(&self) -> Option<CatchupEvidence<T>> {
        let tc = self.timeout_certs.last_key_value().map(|(_, tc)| tc);
        match (tc, self.locked_cert.as_ref()) {
            (Some(tc), Some(qc)) if qc.view_number() > tc.view_number() => {
                Some(CatchupEvidence::Qc(qc.clone()))
            },
            (Some(tc), _) => Some(CatchupEvidence::Tc(tc.clone())),
            (None, Some(qc)) => Some(CatchupEvidence::Qc(qc.clone())),
            (None, None) => None,
        }
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

    /// Return the TimeoutCertificate2 that advanced consensus to `view`, if
    /// any. Keyed by the view it advanced *into* (i.e. one greater than the
    /// view it certified as timed out).
    pub fn timeout_cert_at(&self, view: ViewNumber) -> Option<&TimeoutCertificate2<T>> {
        self.timeout_certs.get(&view)
    }

    /// Return the view of the locked certificate, if set.
    pub fn locked_view(&self) -> Option<ViewNumber> {
        self.locked_cert.as_ref().map(|c| c.view_number())
    }

    /// Newest view that can no longer be decided (and below which decide
    /// inputs are dropped): slides [`DECIDE_BUFFER`] behind the watermark,
    /// pinned at the restart/cutover anchor.
    pub(crate) fn decide_floor(&self) -> ViewNumber {
        max(
            self.last_decided_view.saturating_sub(DECIDE_BUFFER).into(),
            self.decide_floor_view,
        )
    }

    /// Apply consensus to the given input and collect protocol outputs.
    #[instrument(level = "debug", skip_all, fields(node = %self.node_id, view = %input.view_number()))]
    pub fn apply(&mut self, input: ConsensusInput<T>, outbox: &mut Outbox<ConsensusOutput<T>>) {
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
            ConsensusInput::Proposal(sender, proposal) => {
                debug!(
                    sender = %KeyPrefix::from(&sender),
                    block = %proposal.proposal.data.block_header.block_number(),
                    epoch = %proposal.proposal.data.epoch,
                    "apply: proposal"
                );
                self.pair_proposal(sender, proposal, outbox)
            },
            ConsensusInput::VidShare(vid_share) => {
                debug!("apply: vid share");
                self.pair_vid_share(vid_share, outbox)
            },
            ConsensusInput::FetchedProposal(message) => {
                debug!(
                    view = %message.proposal.data.view_number,
                    "apply: fetched proposal"
                );
                self.handle_fetched_proposal(message, outbox);
                // The fetched proposal itself may now be decidable (e.g. cert2
                // arrived first and triggered the fetch).
                self.maybe_decide(view, outbox);
                // Views extending the fetched one may be blocked on it
                let views_extending_fetched: Vec<ViewNumber> = self
                    .proposals
                    .range(view + 1..)
                    .filter(|(_, proposal)| proposal.justify_qc.view_number() == view)
                    .map(|(extending_view, _)| *extending_view)
                    .collect();
                for extending_view in views_extending_fetched {
                    self.maybe_vote_1(extending_view, outbox);
                    self.maybe_vote_2_and_update_lock(extending_view, outbox);
                    self.maybe_decide(extending_view, outbox);
                }
                self.maybe_propose(view + 1, outbox);
                return;
            },
            ConsensusInput::Certificate1(certificate) => {
                debug!(epoch = %certificate.epoch(), "apply: certificate1");
                self.handle_certificate1(certificate)
            },
            ConsensusInput::Certificate2(certificate) => {
                debug!(epoch = %certificate.epoch(), "apply: certificate2");
                self.handle_certificate2(certificate, outbox)
            },
            ConsensusInput::AdvanceView(certificate) => {
                debug!(
                    view  = %certificate.view_number(),
                    epoch = %certificate.epoch(),
                    "apply: advance view"
                );
                self.handle_advance_view(certificate, outbox)
            },
            ConsensusInput::EpochRootCertificates { cert1, state_cert } => {
                info!(
                    epoch = %state_cert.epoch,
                    "apply: epoch root certificates"
                );
                // Store state_cert first so the subsequent Cert1 handler / leader
                // proposer has it on hand. Atomicity invariant: this pair always
                // arrives together; Consensus never sees the Cert1 alone.
                self.state_certs.insert(state_cert.epoch, state_cert);
                self.handle_certificate1(cert1)
            },
            ConsensusInput::TimeoutCertificate(certificate) => {
                let timed_out_view = certificate.view_number();
                let leader = self.leader_label(timed_out_view, certificate.epoch());
                warn!(
                    view = %timed_out_view,
                    epoch = %certificate.epoch(),
                    %leader,
                    "apply: timeout certificate"
                );
                self.handle_timeout_certificate(certificate, outbox)
            },
            ConsensusInput::BlockReconstructed(view, vid_commitment) => {
                debug!(%view, "apply: block reconstructed");
                self.blocks_reconstructed.insert((view, vid_commitment));
                // Retry the child whose vote1 is gated on this parent reconstruction.
                self.maybe_vote_1(view + 1, outbox);
                Protocol::Continue
            },
            ConsensusInput::StateValidated(state_response) => {
                debug!(view = %state_response.view, "apply: state validated");
                self.states_verified
                    .insert(state_response.view, state_response.commitment);
                Protocol::Continue
            },
            ConsensusInput::HeaderCreated(view, commitment, header) => {
                debug!(%view, block = %header.block_number(), "apply: header created");
                self.headers.insert((view, commitment), header);
                Protocol::Continue
            },
            ConsensusInput::Stored(stored) => {
                debug!(?stored, "apply: stored");
                self.handle_stored(stored, outbox);
                Protocol::Continue
            },
            ConsensusInput::StateValidationFailed(state_response) => {
                let view = state_response.view;
                let stored_proposal = self.proposals.get(&view);
                if let Some(proposal) = stored_proposal {
                    let matches = proposal_commitment(proposal) == state_response.commitment;
                    warn!(
                        %view,
                        block = %proposal.block_header.block_number(),
                        epoch = %proposal.epoch,
                        qc_view = %proposal.justify_qc.view_number(),
                        qc_epoch = ?proposal.justify_qc.epoch(),
                        commitment_matches = matches,
                        "apply: state validation failed"
                    );
                    if !matches {
                        return;
                    }
                } else {
                    warn!(%view, "apply: state validation failed (no stored proposal)");
                }
                self.proposals.remove(&view);
                self.leaves.remove(&view);
                self.vid_shares.remove(&view);
                return;
            },
            ConsensusInput::Timeout(view, epoch) => {
                let leader = self.leader_label(view, epoch);
                warn!(%view, %epoch, %leader, "apply: timeout");
                self.handle_timeout(view, epoch, outbox)
            },
            ConsensusInput::TimeoutOneHonest(view, epoch) => {
                let leader = self.leader_label(view, epoch);
                warn!(%view, %epoch, %leader, "apply: timeout (one honest)");
                self.handle_timeout(view, epoch, outbox)
            },
            ConsensusInput::BlockBuilt {
                view,
                epoch,
                payload,
                metadata,
                payload_commitment,
            } => {
                debug!(%view, %epoch, "apply: block built");
                if let VidCommitment::V2(payload_commitment) = payload_commitment {
                    outbox.push_back(ConsensusOutput::RequestVidDisperse {
                        view,
                        epoch,
                        payload: payload.clone(),
                        metadata,
                        payload_commitment,
                    });
                    self.blocks.insert((view, payload_commitment), payload);
                } else {
                    warn!(%view, %epoch, "block built with non-V2 payload commitment; ignoring");
                }
                Protocol::Continue
            },
            ConsensusInput::VidDisperseCreated(view, payload_commitment) => {
                debug!(%view, "apply: vid disperse created");
                self.blocks_reconstructed.insert((view, payload_commitment));
                Protocol::Continue
            },
            ConsensusInput::DrbResult(epoch, drb_result) => {
                info!(%epoch, "apply: drb result");
                self.drb_results.insert(epoch, drb_result);
                Protocol::Continue
            },
            ConsensusInput::EpochChange(epoch_change) => {
                info!(
                    view = %epoch_change.cert1.view_number(),
                    epoch = ?epoch_change.cert1.epoch().map(|e| *e),
                    "apply: epoch change"
                );
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

    #[cfg(test)]
    pub fn set_view(&mut self, view: ViewNumber, epoch: EpochNumber) {
        self.current_view = view;
        self.current_epoch = Some(epoch);
    }

    /// On restart, bar voting and proposing in every view this node may have
    /// acted in by raising `timeout_view` (vote1, propose) and
    /// `restart_barred_view` (vote-action re-recording), pin the decide floor
    /// at the anchor, and place the view cursor just past the high QC.
    /// Forward-only: never regresses any view.
    pub fn resume_from_restart(
        &mut self,
        anchor_view: ViewNumber,
        restart_view: ViewNumber,
        last_actioned_view: ViewNumber,
    ) {
        let first_allowed = max(anchor_view + 1, max(restart_view, last_actioned_view + 1));
        let last_barred = first_allowed - 1;
        if last_barred > self.timeout_view {
            self.timeout_view = last_barred;
        }
        if last_barred > self.restart_barred_view {
            self.restart_barred_view = last_barred;
        }
        if anchor_view > self.decide_floor_view {
            self.decide_floor_view = anchor_view;
        }
        // `Coordinator::start` enters `current_view + 1`, so parking the cursor
        // at the high QC makes the node re-enter at `high_qc + 1`.
        let resume_view = self.stored_high_qc.unwrap_or(anchor_view + 1);
        if resume_view > self.current_view {
            self.current_view = resume_view;
        }
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

    /// Garbage-collect per-view state.
    ///
    /// The decide inputs (`proposals`, `certs`, `certs2`, deferred certs,
    /// `decided_views`) survive down to [`Self::decide_floor`] so a late
    /// Cert2 can still decide a gap view.
    pub fn gc(&mut self, scope: GcScope) {
        match scope {
            GcScope::Local(view) => {
                let c = Commitment::default_commitment_no_preimage();
                let vc = VidCommitment2::default();
                self.headers = self.headers.split_off(&(view, c));
                self.unpaired_proposals = self.unpaired_proposals.split_off(&(view, vc));
                self.unpaired_vid_shares = self.unpaired_vid_shares.split_off(&(view, vc));
                self.proposed_views = self.proposed_views.split_off(&view);
                self.states_verified = self.states_verified.split_off(&view);
                self.timeout_certs = self.timeout_certs.split_off(&view);
                self.voted_1_views = self.voted_1_views.split_off(&view);
                self.voted_2_views = self.voted_2_views.split_off(&view);
            },
            GcScope::Decided(view) => {
                let vc = VidCommitment2::default();
                self.blocks = self.blocks.split_off(&(view, vc));
                self.blocks_reconstructed = self.blocks_reconstructed.split_off(&(view, vc));
                let keep_from = self.decide_floor();
                self.certs = self.certs.split_off(&keep_from);
                self.certs2 = self.certs2.split_off(&keep_from);
                self.decided_views = self.decided_views.split_off(&keep_from);
                self.proposals = self.proposals.split_off(&keep_from);
                self.leaves = self.leaves.split_off(&view);
                self.signed_proposals = self.signed_proposals.split_off(&view);
                self.vid_shares = self.vid_shares.split_off(&view);
                self.stored_proposals = self.stored_proposals.split_off(&view);
                self.stored_vids = self.stored_vids.split_off(&view);
                self.stored_actions = self.stored_actions.split_off(&(view, ActionKind::Vote));
                self.requested_actions =
                    self.requested_actions.split_off(&(view, ActionKind::Vote));
                self.pending_vote1 = self.pending_vote1.split_off(&view);
                self.pending_vote2 = self.pending_vote2.split_off(&view);
                self.pending_proposal = self.pending_proposal.split_off(&view);
                if let Some(epoch) = self.current_epoch {
                    let epoch = EpochNumber::new(epoch.saturating_sub(1));
                    self.drb_results = self.drb_results.split_off(&epoch);
                    self.state_certs = self.state_certs.split_off(&epoch);
                }
            },
            GcScope::Timeout(view) => {
                // A view holding a certificate is likely to decide soon; keep
                // its payload for the decide event.
                if self.certs.contains_key(&view) || self.certs2.contains_key(&view) {
                    return;
                }
                self.vid_shares.remove(&view);
                let vc = VidCommitment2::default();
                self.blocks
                    .extract_if((view, vc)..(view + 1, vc), |_, _| true)
                    .for_each(drop);
            },
        }
    }

    /// Test-only: forcibly replace the proposal stored at `view`.
    ///
    /// Used to simulate the scenario where `self.proposals[parent_view]`
    /// diverges from `self.certs[parent_view].data.leaf_commit` (e.g. a
    /// byzantine leader sent two safe proposals at the same view, the cert
    /// formed for the first but a later overwrite landed in the proposals
    /// map).  No production code should ever do this.
    #[cfg(test)]
    pub(crate) fn force_set_proposal(&mut self, view: ViewNumber, proposal: Proposal<T>) {
        self.proposals.insert(view, proposal);
    }

    /// Pair a validated proposal with this node's VID share for the same payload.
    ///
    /// The half arriving first is parked, keyed by (view, payload commitment).
    fn pair_proposal(
        &mut self,
        sender: T::SignatureKey,
        proposal: ProposalMessage<T, Validated>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let view = proposal.view_number();
        let VidCommitment::V2(commit) = proposal.proposal.data.block_header.payload_commitment()
        else {
            warn!(%view, "proposal payload commitment is not V2, discarding");
            return Protocol::Abort;
        };
        let Some(vid_share) = self.unpaired_vid_shares.remove(&(view, commit)) else {
            self.unpaired_proposals
                .insert((view, commit), (sender, proposal));
            return Protocol::Abort;
        };
        self.on_proposal_paired(sender, proposal, vid_share, outbox)
    }

    /// Pair this node's VID share with a validated proposal for the same payload.
    ///
    /// The half arriving first is parked, keyed by (view, payload commitment).
    fn pair_vid_share(
        &mut self,
        vid_share: VidDisperseShare2<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let key = (vid_share.view_number(), vid_share.payload_commitment);
        let Some((sender, proposal)) = self.unpaired_proposals.remove(&key) else {
            self.unpaired_vid_shares.insert(key, vid_share);
            return Protocol::Abort;
        };
        self.on_proposal_paired(sender, proposal, vid_share, outbox)
    }

    fn on_proposal_paired(
        &mut self,
        sender: T::SignatureKey,
        proposal: ProposalMessage<T, Validated>,
        vid_share: VidDisperseShare2<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        // Parked halves for this and older views can no longer pair.
        let view = proposal.view_number();
        let vc = VidCommitment2::default();
        self.unpaired_proposals = self.unpaired_proposals.split_off(&(view + 1, vc));
        self.unpaired_vid_shares = self.unpaired_vid_shares.split_off(&(view + 1, vc));
        outbox.push_back(ConsensusOutput::ProposalPaired {
            proposal: proposal.proposal.clone(),
            vid_share: vid_share.clone(),
        });
        self.handle_proposal_with_vid_share(sender, proposal, vid_share, outbox)
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
        let proposer = KeyPrefix::from(&sender);
        let block_number = proposal.proposal.data.block_header.block_number();
        let qc_view = proposal.proposal.data.justify_qc.view_number();

        if !self.wants_proposal_for_view(&view) {
            warn!(
                %view, %proposer, block = %block_number,
                epoch = %proposal.proposal.data.epoch, %qc_view,
                "proposal too old"
            );
            return Protocol::Abort;
        }

        let signed_proposal = proposal.proposal.clone();
        let proposal = proposal.proposal.data;
        let epoch = proposal.epoch;
        // QC can be for a different epoch
        let Some(qc_epoch) = proposal.justify_qc.epoch() else {
            warn!(
                %view, %proposer, block = %block_number, %epoch, %qc_view,
                "proposal has no epoch number"
            );
            return Protocol::Abort;
        };

        if let Err(err) = self.is_safe(&proposal) {
            warn!(
                %view, %proposer, block = %block_number, %epoch, %qc_view, %qc_epoch, %err,
                "proposal not safe"
            );
            return Protocol::Abort;
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
        self.adopt_certified_drb(view);

        self.request_parent_proposal_if_missing(&proposal, outbox);

        if let Some(state_cert) = &proposal.state_cert {
            self.state_certs
                .entry(state_cert.epoch)
                .or_insert_with(|| state_cert.clone());
        }

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
                    warn!(
                        %view, %proposer, block = %block_number, %epoch, %qc_view, %qc_epoch,
                        "DRB result does not match proposal"
                    );
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
    fn handle_fetched_proposal(
        &mut self,
        message: ProposalMessage<T, Validated>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        let signed_proposal = message.proposal;
        let proposal = signed_proposal.data.clone();
        let view = proposal.view_number;
        if view <= self.last_decided_view {
            debug!(%view, "fetched proposal at or below decided view; discarding");
            return;
        }
        if self.proposals.contains_key(&view) {
            debug!(%view, "fetched proposal already present; discarding");
            return;
        }
        self.leaves.insert(view, proposal.clone().into());
        self.signed_proposals.insert(view, signed_proposal);
        self.request_parent_proposal_if_missing(&proposal, outbox);
        self.proposals.insert(view, proposal);
        self.adopt_certified_drb(view);
    }

    fn request_parent_proposal_if_missing(
        &self,
        proposal: &Proposal<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        let parent_view = proposal.justify_qc.view_number();
        if parent_view > self.last_decided_view && !self.proposals.contains_key(&parent_view) {
            warn!(
                view = %proposal.view_number,
                %parent_view,
                "parent proposal missing; requesting fetch"
            );
            outbox.push_back(ConsensusOutput::RequestMissingProposal {
                view: parent_view,
                leaf_commit: proposal.justify_qc.data().leaf_commit,
            });
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn handle_certificate1(&mut self, certificate: ValidCert<Certificate1<T>>) -> Protocol {
        let view = certificate.view_number();
        if view <= self.decide_floor() {
            return Protocol::Continue;
        }
        self.certs.entry(view).or_insert(certificate.into_cert());
        self.adopt_certified_drb(view);
        Protocol::Continue
    }

    #[instrument(level = "debug", skip_all)]
    fn handle_certificate2(
        &mut self,
        certificate: ValidCert<Certificate2<T>>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let view = certificate.view_number();
        if view <= self.decide_floor() {
            return Protocol::Continue;
        }
        if self.certs2.contains_key(&view) {
            return Protocol::Continue;
        }
        // Relay a first-obtained Cert2 so peers that missed the vote2s can
        // still decide. Skip decided views so a GC'd-then-re-received Cert2
        // cannot ping-pong between nodes forever.
        if !self.decided_views.contains(&view) {
            outbox.push_back(ConsensusOutput::SendCertificate2(
                certificate.cert().clone(),
            ));
        }
        if view > self.last_decided_view && !self.proposals.contains_key(&view) {
            warn!(%view, "have certificate2 but no proposal; requesting fetch");
            outbox.push_back(ConsensusOutput::RequestMissingProposal {
                view,
                leaf_commit: certificate.data.leaf_commit,
            });
        }
        self.certs2.insert(view, certificate.into_cert());
        Protocol::Continue
    }

    /// Adopt the successor-epoch DRB carried by the transition leaf at `view`,
    /// once we hold both its proposal and a QC certifying it. A Cert1 over the
    /// leaf is a quorum's endorsement of its `next_drb_result`, so a catching-up
    /// node can use it without waiting on a separate successor-epoch catchup.
    fn adopt_certified_drb(&mut self, view: ViewNumber) {
        let Some(proposal) = self.proposals.get(&view) else {
            return;
        };
        // Only transition leaves in epoch >= 1 carry the next epoch's DRB.
        if proposal.epoch <= EpochNumber::genesis()
            || !is_epoch_transition(proposal.block_header.block_number(), *self.epoch_height)
        {
            return;
        }
        let Some(drb) = proposal.next_drb_result else {
            return;
        };
        let next_epoch = proposal.epoch + 1;
        if self.drb_results.contains_key(&next_epoch) {
            return;
        }
        // Adopt only from the exact leaf the QC certified (hashes the leaf).
        let Some(cert) = self.certs.get(&view) else {
            return;
        };
        if proposal_commitment(proposal) != cert.data.leaf_commit {
            return;
        }
        self.drb_results.insert(next_epoch, drb);
        debug!(%view, %next_epoch, "adopted quorum-certified next_drb_result");
    }

    /// Advance our view based on a quorum certificate.
    #[instrument(level = "debug", skip_all)]
    fn handle_advance_view(
        &mut self,
        cert1: ValidCert<Certificate1<T>>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let view = cert1.view_number();

        if view < self.current_view {
            return Protocol::Continue;
        }

        let epoch = cert1.epoch();

        self.certs.entry(view).or_insert(cert1.into_cert());
        self.adopt_certified_drb(view);

        // Ensure we submit a vote2 if we can:
        self.maybe_vote_2_and_update_lock(view, outbox);

        let next_view = view + 1;

        if next_view > self.current_view {
            self.current_view = next_view;
            self.current_epoch = Some(epoch);
            outbox.push_back(ConsensusOutput::ViewChanged(next_view, epoch));
        }

        Protocol::Continue
    }

    #[instrument(level = "debug", skip_all)]
    fn handle_timeout(
        &mut self,
        view: ViewNumber,
        epoch: EpochNumber,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        if view < self.current_view {
            debug!(
                %view,
                current_view = %self.current_view,
                "ignoring timeout for stale view"
            );
            return Protocol::Abort;
        }
        let we_were_leader = self.is_leader(view, epoch);
        if we_were_leader {
            if self.proposed_views.contains(&view) {
                warn!(%view, %epoch, "timeout: we were the leader and did propose for this view");
            } else {
                let missing = self.missing_for_propose(view);
                let missing_str = if missing.is_empty() {
                    "none".to_string()
                } else {
                    missing.join(",")
                };
                warn!(
                    %view, %epoch, missing = %missing_str,
                    "timeout: we were the leader but did not propose"
                );
            }
        }

        // Vote-side diagnostics fire when we weren't the leader, or when we
        // were the leader and did propose: in either case the next thing
        // expected of us was voting on a proposal we received.
        if !we_were_leader || self.proposed_views.contains(&view) {
            if self.voted_1_views.contains(&view) {
                warn!(%view, %epoch, "timeout: we did vote1 for this view");
            } else {
                let missing = self.missing_for_vote1(view);
                let missing_str = if missing.is_empty() {
                    "none".to_string()
                } else {
                    missing.join(",")
                };
                warn!(
                    %view, %epoch, missing = %missing_str,
                    "timeout: we did not vote1 for this view"
                );
            }
        }

        // If a cert1 already formed for this view, the holdup is one step
        // further along: we need the reconstructed block to match the
        // proposal's payload commitment before we can vote2 and update lock.
        if self.certs.contains_key(&view) {
            let proposal_commit = self
                .proposals
                .get(&view)
                .map(|p| p.block_header.payload_commitment());
            match proposal_commit {
                Some(VidCommitment::V2(prop))
                    if self.blocks_reconstructed.contains(&(view, prop)) =>
                {
                    warn!(%view, %epoch, "timeout: have cert1 and matching reconstructed block");
                },
                Some(VidCommitment::V2(_)) => {
                    warn!(
                        %view, %epoch,
                        "timeout: have cert1, but no reconstructed block matching the proposal"
                    );
                },
                Some(_) => {
                    // Non-V2 commitment shouldn't happen at this point in the
                    // protocol; log it loudly if it does.
                    warn!(
                        %view, %epoch,
                        "timeout: have cert1 but proposal payload commitment is not V2"
                    );
                },
                None => {
                    warn!(%view, %epoch, "timeout: have cert1 but no proposal stored");
                },
            }
        }
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
            self.catchup_evidence(),
        ));
        Protocol::Abort
    }

    #[instrument(level = "debug", skip_all)]
    fn handle_timeout_certificate(
        &mut self,
        certificate: ValidCert<TimeoutCertificate2<T>>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let view = certificate.view_number() + 1;
        if view < self.current_view {
            debug!(
                %view,
                current_view = %self.current_view,
                "ignoring stale timeout certificate"
            );
            return Protocol::Abort;
        }
        if self.timeout_certs.contains_key(&view) {
            return Protocol::Continue;
        }
        let epoch = certificate.epoch();
        self.timeout_certs.insert(view, certificate.cert().clone());
        self.current_view = self.current_view.max(view);
        self.current_epoch = Some(epoch);
        outbox.push_back(ConsensusOutput::ViewChanged(view, epoch));
        outbox.push_back(ConsensusOutput::ViewTimedOut(certificate.view_number()));
        outbox.push_back(ConsensusOutput::SendTimeoutCertificate(
            certificate.into_cert(),
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
        epoch_change: EpochChangeMessage<T, Validated>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let EpochChangeMessage {
            cert1,
            cert2,
            proposal,
            ..
        } = epoch_change;
        // Compare epochs (not views) so a node that timed out past a boundary
        // it never saw can still recover via a genuinely new epoch change.
        if self
            .current_epoch
            .is_some_and(|current| cert2.data.epoch < current)
        {
            debug!(
                view = %cert2.view_number(),
                epoch = %cert2.data.epoch,
                current_epoch = ?self.current_epoch.map(|e| *e),
                "ignoring stale epoch change for an epoch we have already entered"
            );
            return Protocol::Abort;
        }
        // Check if this epoch change is new
        if self
            .locked_cert
            .as_ref()
            .is_some_and(|locked_cert| locked_cert.view_number() > cert1.view_number())
        {
            warn!("locked certificate is newer than epoch change certificate1");
            return Protocol::Abort;
        }
        let next_view = cert2.view_number() + 1;
        let next_epoch = cert2.data.epoch + 1;
        // Change view to the first view of the next epoch
        self.current_view = self.current_view.max(next_view);
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

        let boundary_view = cert2.view_number();
        self.proposals.insert(boundary_view, proposal);
        self.certs.insert(cert1.view_number(), cert1);
        self.certs2.insert(boundary_view, cert2);
        self.adopt_certified_drb(boundary_view);
        Protocol::Continue
    }

    #[instrument(level = "debug", skip_all)]
    fn maybe_propose(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        if view <= self.timeout_view {
            return;
        }
        if self.proposed_views.contains(&view) {
            return;
        }

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

        // Key the header lookup by the cert's `leaf_commit`, NOT
        // `proposal_commitment(proposal)`.  `self.proposals` is keyed by view
        // and can be overwritten by any later-arriving safe proposal at the
        // same view, so it may not match the cert.  Using the cert's
        // leaf_commit pins the lookup to the leaf the QC actually certified
        // and prevents the leader from grabbing a header that was built for a
        // different (same-view) parent and therefore carries a wrong
        // block_number.
        //
        // Genesis is the one special case: the synthetic genesis proposal
        // carries a non-null justify_qc, so the leaf derived from it has a
        // different commitment than the anchor leaf the genesis cert was
        // built over.  For view 1 we fall back to the proposal's commit.
        let parent_commitment = if parent_view == ViewNumber::genesis() {
            proposal_commitment(proposal)
        } else if proposal_commitment(proposal) != parent_cert.data.leaf_commit {
            warn!(
                %parent_view,
                "stored proposal at parent_view does not match parent cert's leaf_commit; \
                 refusing to propose with mismatched parent"
            );
            return;
        } else {
            parent_cert.data.leaf_commit
        };
        let Some(header) = self.headers.get(&(view, parent_commitment)) else {
            // The header request issued on the TC targeted the lock held at
            // that moment; if the lock moved since (bridged legacy QC at
            // cutover), re-request. The block builder dedups by (view, parent).
            if view_change_evidence.is_some() {
                let request_epoch =
                    if is_last_block(proposal.block_header.block_number(), *self.epoch_height) {
                        proposal.epoch + 1
                    } else {
                        proposal.epoch
                    };
                if self.is_leader(view, request_epoch) {
                    outbox.push_back(ConsensusOutput::RequestBlockAndHeader(
                        BlockAndHeaderRequest {
                            view,
                            epoch: request_epoch,
                            parent_proposal: proposal.clone(),
                        },
                    ));
                }
            }
            debug!("no block header");
            return;
        };
        let VidCommitment::V2(block_commitment) = header.payload_commitment() else {
            debug!("header payload commitment is not V2");
            return;
        };
        if !self.blocks.contains_key(&(view, block_commitment)) {
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
        // By the atomicity invariant (enforced by `EpochRootTally`),
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
        outbox.push_back(ConsensusOutput::PersistProposal(message.clone()));
        self.request_action(view, Some(proposal_epoch), ActionKind::Propose, outbox);
        self.pending_proposal.insert(view, message);
    }

    #[instrument(level = "debug", skip_all)]
    fn maybe_decide(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        // Any still-undecided view above the floor can decide, even one older
        // than the watermark (a gap).
        let floor = self.decide_floor();
        if view <= floor || self.decided_views.contains(&view) {
            return;
        }
        let Some(cert2) = self.certs2.get(&view) else {
            debug!(%view, "cert2 not available");
            return;
        };
        let Some(proposal) = self.proposals.get(&view) else {
            debug!(%view, "proposal not available");
            return;
        };
        let block = proposal.block_header.block_number();
        let epoch = proposal.epoch;
        let qc_view = proposal.justify_qc.view_number();
        let qc_epoch = proposal.justify_qc.epoch();
        let proposal_commit = proposal_commitment(proposal);
        if cert2.data.leaf_commit != proposal_commit {
            debug!(
                %view, %block, %epoch, %qc_view, ?qc_epoch,
                "cert2 commitment does not match proposal commitment"
            );
            return;
        }
        // A Cert2 can arrive before its Cert1; require both before mutating any
        // decided state.
        let Some(cert1) = self.certs.get(&view).cloned() else {
            debug!(%view, "cert1 missing");
            return;
        };
        // Handle Epoch Change by broadcasting the epoch change message if we have
        // all the data we need.
        if is_last_block(proposal.block_header.block_number(), *self.epoch_height)
            && cert1.data.leaf_commit == proposal_commit
        {
            let epoch_change =
                EpochChangeMessage::validated(cert1.clone(), cert2.clone(), proposal.clone());
            outbox.push_back(ConsensusOutput::SendEpochChange(epoch_change));
        }
        // we have a second certificate, and matching proposal, it is decided.
        let mut leaf: Leaf2<T> = proposal.clone().into();
        if let VidCommitment::V2(pc) = proposal.block_header.payload_commitment()
            && let Some(payload) = self.blocks.get(&(view, pc))
        {
            leaf.fill_block_payload_unchecked(payload.clone());
        }
        let mut decided = vec![leaf];
        let mut vid_shares = vec![self.signed_vid_share(view)];

        let mut parent_view = proposal.justify_qc.view_number();
        let mut parent_commit = proposal.justify_qc.data.leaf_commit;

        // A missing ancestor is a gap; a later Cert2 for it fills it in.
        while parent_view > floor
            && !self.decided_views.contains(&parent_view)
            && let Some(proposal) = self.proposals.get(&parent_view)
        {
            let proposal_commit = proposal_commitment(proposal);
            if proposal_commit != parent_commit {
                break;
            }
            let mut leaf: Leaf2<T> = proposal.clone().into();
            if let VidCommitment::V2(pc) = proposal.block_header.payload_commitment()
                && let Some(payload) = self.blocks.get(&(parent_view, pc))
            {
                leaf.fill_block_payload_unchecked(payload.clone());
            }
            vid_shares.push(self.signed_vid_share(parent_view));
            decided.push(leaf);
            parent_view = proposal.justify_qc.view_number();
            parent_commit = proposal.justify_qc.data.leaf_commit;
        }
        self.decided_views
            .extend(decided.iter().map(|l| l.view_number()));
        // A gap-fill decide of an older view must not move the watermark backward.
        if view > self.last_decided_view {
            self.last_decided_view = view;
            self.last_decided_leaf = decided[0].clone();
        }
        outbox.push_back(ConsensusOutput::LeafDecided {
            leaves: decided,
            cert1,
            cert2: Some(cert2.clone()),
            vid_shares,
        });
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

    fn handle_stored(&mut self, stored: StorageOutput<T>, outbox: &mut Outbox<ConsensusOutput<T>>) {
        let view = stored.view_number();
        match stored {
            StorageOutput::Proposal(view, commitment) => {
                self.stored_proposals
                    .entry(view)
                    .or_default()
                    .push(commitment);
            },
            StorageOutput::Vid(view) => {
                self.stored_vids.insert(view);
            },
            StorageOutput::Action(view, kind) => {
                self.stored_actions.insert((view, kind));
            },
            StorageOutput::HighQc(view) => {
                self.bump_stored_high_qc(view);
                // A newly persisted lock can unblock vote2 across many views; re-check all.
                let pending: Vec<ViewNumber> = self.pending_vote2.keys().copied().collect();
                for view in pending {
                    self.release_vote2(view, outbox);
                }
                return;
            },
        }
        self.release_vote1(view, outbox);
        self.release_vote2(view, outbox);
        self.release_proposal(view, outbox);
    }

    fn release_vote1(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        let Some(vote1) = self.pending_vote1.get(&view) else {
            return;
        };
        if !self.stored_actions.contains(&(view, ActionKind::Vote))
            || !self.is_proposal_stored(view, &vote1.vote.data.leaf_commit)
        {
            return;
        }
        let vote1 = self.pending_vote1.remove(&view).expect("checked above");
        if view <= self.timeout_view {
            debug!(%view, "dropping pending vote1 for timed-out view");
            return;
        }
        let vid_share = self.vid_shares.get(&view).cloned();
        outbox.push_back(ConsensusOutput::SendVote1(vote1));
        if let Some(vid_share) = vid_share {
            outbox.push_back(ConsensusOutput::BroadcastVidShare(vid_share));
        } else {
            debug!(%view, "vid share gone for released vote1; skipping broadcast");
        }
    }

    fn release_vote2(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        let Some((_, required)) = self.pending_vote2.get(&view) else {
            return;
        };
        if self.certs2.contains_key(&view) {
            self.pending_vote2.remove(&view);
            return;
        }
        let required = *required;
        if !self.vote2_persisted(view) || !self.high_qc_persisted(required) {
            return;
        }
        let (vote2, _) = self.pending_vote2.remove(&view).expect("checked above");
        outbox.push_back(ConsensusOutput::SendVote2(vote2));
    }

    /// Whether the view's Vote action and VID share are persisted, gating the
    /// phase-2 vote. A restart-barred view persisted both before the crash:
    /// its vote is re-cast, but its Vote action is never re-recorded.
    fn vote2_persisted(&self, view: ViewNumber) -> bool {
        view <= self.restart_barred_view
            || (self.stored_actions.contains(&(view, ActionKind::Vote))
                && self.stored_vids.contains(&view))
    }

    fn release_proposal(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        let Some(message) = self.pending_proposal.get(&view) else {
            return;
        };
        if !self.stored_actions.contains(&(view, ActionKind::Propose))
            || !self.is_proposal_stored(view, &proposal_commitment(&message.data))
        {
            return;
        }
        let message = self.pending_proposal.remove(&view).expect("checked above");
        outbox.push_back(ConsensusOutput::SendProposal(message));
    }

    fn is_proposal_stored(&self, view: ViewNumber, commitment: &Commitment<Leaf2<T>>) -> bool {
        self.stored_proposals
            .get(&view)
            .is_some_and(|commitments| commitments.contains(commitment))
    }

    fn request_action(
        &mut self,
        view: ViewNumber,
        epoch: Option<EpochNumber>,
        kind: ActionKind,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        if self.requested_actions.insert((view, kind)) {
            outbox.push_back(ConsensusOutput::RecordAction(view, epoch, kind));
        }
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
            debug!(%view, "state commitment not available");
            return;
        };
        let Some(proposal) = self.proposals.get(&view) else {
            debug!(%view, "proposal not available");
            return;
        };
        let Some(vid_share) = self.vid_shares.get(&view) else {
            debug!(%view, "vid share not available");
            return;
        };

        let block_number = proposal.block_header.block_number();
        let epoch = proposal.epoch;
        let qc_view = proposal.justify_qc.view_number();
        let qc_epoch = proposal.justify_qc.epoch();

        // Don't vote for epoch-transition proposals until we can verify
        // the attached DRB result.  Same guard as `maybe_propose`:
        // transitions in epoch >= 2 must carry `next_drb_result`.
        if proposal.epoch > EpochNumber::genesis()
            && is_epoch_transition(block_number, *self.epoch_height)
        {
            let Some(drb) = self.drb_results.get(&(proposal.epoch + 1)) else {
                debug!(%view, block = %block_number, %epoch, "DRB result not yet available, deferring vote");
                return;
            };
            if proposal
                .next_drb_result
                .is_none_or(|proposed_drb| drb != &proposed_drb)
            {
                warn!(
                    %view, block = %block_number, %epoch, %qc_view, ?qc_epoch,
                    "DRB result does not match proposal, refusing to vote"
                );
                return;
            }
        }

        if !self.staked_in_epoch(proposal.epoch) {
            return;
        }

        // Verify parent chain unless justify_qc is the genesis QC
        let parent_view = proposal.justify_qc.view_number();

        // Pre-cutover parents are V1 AvidM, not V2-reconstructable.
        let parent_is_pre_cutover = self.pre_cutover_views.contains(&parent_view);
        if parent_view != ViewNumber::genesis()
            && !is_last_block(
                proposal.block_header.block_number().saturating_sub(1),
                *self.epoch_height,
            )
        {
            let Some(prev_proposal) = self.proposals.get(&parent_view) else {
                debug!(%view, %parent_view, "proposal not available");
                return;
            };
            let parent_block = prev_proposal.block_header.block_number();
            let parent_epoch = prev_proposal.epoch;

            if !parent_is_pre_cutover {
                let VidCommitment::V2(prev_block_commitment) =
                    prev_proposal.block_header.payload_commitment()
                else {
                    warn! {
                        %view, block = %block_number, %epoch,
                        %parent_view, %parent_block, %parent_epoch,
                        "prev. proposal payload commitment is not a V2 VID commitment"
                    }
                    return;
                };
                // Parent must be reconstructed (see `parent_reconstructed`).
                if !self.parent_reconstructed(
                    parent_view,
                    prev_block_commitment,
                    proposal_commitment(prev_proposal),
                ) {
                    debug!(
                        %view, block = %block_number, %epoch,
                        %parent_view, %parent_block, %parent_epoch,
                        "no reconstructed block matching the parent block commitment"
                    );
                    return;
                }
            }

            if proposal.justify_qc.data().leaf_commit != proposal_commitment(prev_proposal) {
                debug!(
                    %view, block = %block_number, %epoch,
                    %parent_view, %parent_block, %parent_epoch,
                    "justify qc commitment does not match proposal commitment"
                );
                return;
            }
        }

        let proposal_commit = proposal_commitment(proposal);

        // Verify the state commitment matches the proposal
        if state_commitment != &proposal_commit {
            debug!(
                %view, block = %block_number, %epoch, %qc_view, ?qc_epoch,
                "state commitment does not match proposal commitment"
            );
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
            state_vote,
        };
        let can_send = self.stored_actions.contains(&(view, ActionKind::Vote))
            && self.is_proposal_stored(view, &proposal_commit);
        let vid_share = can_send.then(|| vid_share.clone());
        self.voted_1_views.insert(view);
        if let Some(vid_share) = vid_share {
            outbox.push_back(ConsensusOutput::SendVote1(vote));
            outbox.push_back(ConsensusOutput::BroadcastVidShare(vid_share));
        } else {
            self.request_action(view, Some(epoch), ActionKind::Vote, outbox);
            self.pending_vote1.insert(view, vote);
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn maybe_vote_2_and_update_lock(
        &mut self,
        view: ViewNumber,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        // V1 AvidM dispersal cannot be re-voted under V2.
        if self.pre_cutover_views.contains(&view) {
            return;
        }
        if self.voted_2_views.contains(&view) {
            return;
        }
        let Some(cert1) = self.certs.get(&view) else {
            debug!(%view, "cert1 not available");
            return;
        };
        let Some(proposal) = self.proposals.get(&view) else {
            debug!(%view, "proposal not available");
            return;
        };
        let proposal_epoch = proposal.epoch;
        let block = proposal.block_header.block_number();
        let qc_view = proposal.justify_qc.view_number();
        let qc_epoch = proposal.justify_qc.epoch();

        let proposal_commit = proposal_commitment(proposal);

        // The certificate must match the proposal
        if cert1.data.leaf_commit != proposal_commit {
            warn!(
                %view, %block, epoch = %proposal_epoch, %qc_view, ?qc_epoch,
                "cert1 commitment does not match proposal commitment"
            );
            return;
        }
        let VidCommitment::V2(proposal_block_commitment) =
            proposal.block_header.payload_commitment()
        else {
            warn!(
                %view, %block, epoch = %proposal_epoch, %qc_view, ?qc_epoch,
                "proposal payload commitment is not a V2 VID commitment"
            );
            return;
        };
        if !self
            .blocks_reconstructed
            .contains(&(view, proposal_block_commitment))
        {
            debug!(
                %view, %block, epoch = %proposal_epoch, %qc_view, ?qc_epoch,
                "no reconstructed block matching the proposal commitment"
            );
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
            self.current_view = self.current_view.max(view + 1);
            self.current_epoch = Some(proposal_epoch);
            outbox.push_back(ConsensusOutput::ViewChanged(view + 1, proposal_epoch));
            outbox.push_back(ConsensusOutput::SendCertificate1(cert1.clone()));
            // Persist the new lock; `release_vote2` gates the phase-2 vote on it.
            outbox.push_back(ConsensusOutput::PersistHighQc(cert1.clone()));
        }

        if self.certs2.contains_key(&view)
            || self.decided_views.contains(&view)
            || view <= self.decide_floor()
        {
            return;
        }

        if !self.staked_in_epoch(proposal_epoch) {
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
        self.voted_2_views.insert(view);
        // Lock is set above and >= view; the vote waits until it is persisted.
        let required = self
            .locked_view()
            .expect("locked_cert is set before voting in phase 2");
        if self.vote2_persisted(view) && self.high_qc_persisted(required) {
            outbox.push_back(ConsensusOutput::SendVote2(vote));
        } else {
            if view > self.restart_barred_view {
                self.request_action(view, Some(proposal_epoch), ActionKind::Vote, outbox);
            }
            self.pending_vote2.insert(view, (vote, required));
        }
    }

    #[instrument(level = "trace", skip_all)]
    fn is_safe(&self, proposal: &Proposal<T>) -> Result<(), SafetyError> {
        let Some(locked_cert) = self.locked_cert.as_ref() else {
            // Locked certificate is not set which means it is at genesis
            debug!("at genesis");
            return Ok(());
        };

        // cert1 + block arrived before proposal
        if locked_cert.view_number() == proposal.view_number() {
            let locked_commit = locked_cert.data.leaf_commit;
            let proposal_commit = proposal_commitment(proposal);
            if locked_commit != proposal_commit {
                return Err(SafetyError::LockedViewCommitmentMismatch {
                    locked_commit: locked_commit.to_string(),
                    proposal_commit: proposal_commit.to_string(),
                });
            }
            return Ok(());
        }

        let parent_commit = proposal
            .justify_qc
            .data_commitment(&self.upgrade_lock)
            .map_err(SafetyError::JustifyQcCommitment)?;
        let locked_commit = locked_cert
            .data_commitment(&self.upgrade_lock)
            .map_err(SafetyError::LockedCertCommitment)?;

        let safety = parent_commit == locked_commit;
        let liveness = proposal.justify_qc.view_number() > locked_cert.view_number();
        if safety || liveness {
            return Ok(());
        }

        Err(SafetyError::UnsafeProposal {
            locked_view: locked_cert.view_number(),
            parent_commit: parent_commit.to_string(),
            locked_commit: locked_commit.to_string(),
        })
    }

    /// Format the leader's key prefix for the given view/epoch, or `"unknown"`
    /// when the stake table is not available.  Used by timeout logging so a
    /// reader can immediately see which validator failed to make progress.
    fn leader_label(&self, view: ViewNumber, epoch: EpochNumber) -> String {
        match self
            .stake_table_coordinator
            .membership_for_epoch(Some(epoch))
        {
            Ok(stake_table) => match stake_table.leader(view) {
                Ok(leader) => KeyPrefix::from(&leader).to_string(),
                Err(_) => "unknown".to_string(),
            },
            Err(_) => "unknown".to_string(),
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

    /// Used for logging.  Returns a list of checks that failed for the given trying to vote1
    fn missing_for_vote1(&self, view: ViewNumber) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.states_verified.contains_key(&view) {
            missing.push("state_validation");
        }
        let proposal = self.proposals.get(&view);
        if proposal.is_none() {
            missing.push("proposal");
        }
        if !self.vid_shares.contains_key(&view) {
            missing.push("vid_share");
        }
        if let Some(proposal) = proposal {
            let block_number = proposal.block_header.block_number();
            if proposal.epoch > EpochNumber::genesis()
                && is_epoch_transition(block_number, *self.epoch_height)
                && !self.drb_results.contains_key(&(proposal.epoch + 1))
            {
                missing.push("drb_result_for_next_epoch");
            }
            // Parent-chain reconstruction is only required for non-genesis,
            // non-last-block-of-epoch proposals (matches the gate in
            // `maybe_vote_1`).
            let parent_view = proposal.justify_qc.view_number();
            if parent_view != ViewNumber::genesis()
                && !is_last_block(block_number.saturating_sub(1), *self.epoch_height)
            {
                let reconstructed = self.proposals.get(&parent_view).is_some_and(|p| {
                    let VidCommitment::V2(c) = p.block_header.payload_commitment() else {
                        return false;
                    };
                    self.parent_reconstructed(parent_view, c, proposal_commitment(p))
                });
                if !reconstructed {
                    missing.push("parent_block_reconstructed");
                }
            }
        }
        missing
    }

    /// Used for logging.  Returns a list of checks that failed for the given trying to propose
    fn missing_for_propose(&self, view: ViewNumber) -> Vec<&'static str> {
        let mut missing = Vec::new();

        let view_change_evidence = self.timeout_certs.get(&view);
        let parent_cert = if view_change_evidence.is_some() {
            match self.locked_cert.as_ref() {
                Some(c) => c,
                None => {
                    missing.push("locked_cert");
                    return missing;
                },
            }
        } else {
            match self.certs.get(&ViewNumber::from(view.saturating_sub(1))) {
                Some(c) => c,
                None => {
                    missing.push("parent_cert");
                    return missing;
                },
            }
        };
        let parent_view = parent_cert.view_number();
        let Some(parent_proposal) = self.proposals.get(&parent_view) else {
            missing.push("parent_proposal");
            return missing;
        };

        let parent_commitment = proposal_commitment(parent_proposal);
        let header = self.headers.get(&(view, parent_commitment));
        if header.is_none() {
            missing.push("block_header");
        }
        let block_present = header
            .and_then(|h| {
                if let VidCommitment::V2(c) = h.payload_commitment() {
                    Some(c)
                } else {
                    None
                }
            })
            .is_some_and(|c| self.blocks.contains_key(&(view, c)));
        if !block_present {
            missing.push("block_payload");
        }

        // The epoch-transition checks below all key off the proposed block
        // number, so we can only evaluate them once we have the header.
        if let Some(header) = header {
            let first_proposal_of_epoch =
                is_last_block(header.block_number().saturating_sub(1), *self.epoch_height);

            if parent_proposal.epoch > EpochNumber::genesis()
                && is_epoch_transition(header.block_number(), *self.epoch_height)
                && !self
                    .drb_results
                    .contains_key(&EpochNumber::new(*parent_proposal.epoch + 1))
            {
                missing.push("drb_result_for_next_epoch");
            }

            if first_proposal_of_epoch && !self.certs2.contains_key(&parent_view) {
                missing.push("next_epoch_justify_qc");
            }
        }

        let parent_block_number = parent_cert.data.block_number.unwrap_or(0);
        if is_epoch_root(parent_block_number, *self.epoch_height) {
            match parent_cert.data.epoch() {
                None => missing.push("parent_cert_epoch"),
                Some(parent_epoch) => {
                    if !self.state_certs.contains_key(&parent_epoch) {
                        missing.push("state_cert");
                    }
                },
            }
        }

        missing
    }
}

impl<T: NodeType> ConsensusInput<T> {
    fn view_number(&self) -> ViewNumber {
        match self {
            ConsensusInput::BlockBuilt { view, .. } => *view,
            ConsensusInput::BlockReconstructed(view, _) => *view,
            ConsensusInput::Certificate1(cert) => cert.view_number(),
            ConsensusInput::Certificate2(cert) => cert.view_number(),
            // We advance from the certificate's view v to v + 1:
            ConsensusInput::AdvanceView(cert) => cert.view_number() + 1,
            ConsensusInput::EpochRootCertificates { cert1, .. } => cert1.view_number(),
            ConsensusInput::HeaderCreated(view, ..) => *view,
            ConsensusInput::Proposal(_, prop) => prop.view_number(),
            ConsensusInput::VidShare(share) => share.view_number(),
            ConsensusInput::FetchedProposal(prop) => prop.view_number(),
            ConsensusInput::StateValidated(response) => response.view,
            ConsensusInput::StateValidationFailed(request) => request.view,
            ConsensusInput::Stored(stored) => stored.view_number(),
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
