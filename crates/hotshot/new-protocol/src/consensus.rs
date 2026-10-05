mod proposals;

use std::{
    cmp::max,
    collections::{BTreeMap, BTreeSet},
    marker::PhantomData,
    num::NonZeroU64,
    sync::Arc,
};

use committable::{Commitment, CommitmentBoundsArkless, Committable};
use hotshot::traits::BlockPayload;
use hotshot_contract_adapter::light_client::derive_signed_state_digest;
use hotshot_types::{
    data::{
        BlockNumber, EpochNumber, Leaf2, VidCommitment, VidCommitment2, VidDisperseShare2,
        ViewNumber,
    },
    drb::DrbResult,
    epoch_membership::EpochMembershipCoordinator,
    message::{Proposal as SignedProposal, UpgradeLock},
    simple_certificate::{
        LightClientStateUpdateCertificateV2, TimeoutEvidence, UpgradeCertificate,
        UpgradeCertificate2, check_qc_state_cert_correspondence,
    },
    simple_vote::{
        HasEpoch, LightClientStateUpdateVote2, LockView, QuorumData2, SimpleVote, TimeoutData2,
        Vote2Data,
    },
    stake_table::HSStakeTable,
    traits::{
        block_contents::{BlockHeader, EncodeBytes},
        node_implementation::NodeType,
        signature_key::{
            LCV2StateSignatureKey, LCV3StateSignatureKey, SignatureKey, StateSignatureKey,
        },
    },
    utils::{is_epoch_root, is_epoch_transition, is_last_block},
    vote::{Certificate, HasViewNumber},
};
use hotshot_utils::anytrace;
use proposals::Proposals;
use tracing::{debug, info, instrument, warn};

use crate::{
    block::BlockAndHeaderRequest,
    cert_verifier::ValidCert,
    coordinator::{GcScope, VID_RECONSTRUCT_GC_MARGIN},
    helpers::{ViewEpoch, proposal_commitment, validated_state_cert},
    logging::KeyPrefix,
    message::{
        CatchupEvidence, Certificate1, Certificate2, EpochChangeMessage, Proposal,
        ProposalFetchRequest, ProposalMessage, ReVote, ReVoteMessage, TimeoutBallot, TimeoutVote,
        Validated, Vote1, Vote2,
    },
    outbox::Outbox,
    state::{StateRequest, StateResponse},
    storage::{ActionKind, StorageOutput},
    vid::ns_lens_match_metadata,
};

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
    /// This node obtained a proposal's payload, by reconstructing it from VID
    /// shares or fetching it from a peer. Consensus keeps the payload so a
    /// decide of that view carries it.
    BlockReconstructed {
        view: ViewNumber,
        payload_commitment: VidCommitment2,
        payload: T::BlockPayload,
    },
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
    Timeout(ViewNumber),
    TimeoutCertificate(ValidCert<TimeoutEvidence<T>>),
    TimeoutOneHonest(ViewNumber),
    VidDisperseCreated(ViewNumber, VidCommitment2),
    DrbResult(EpochNumber, DrbResult),
    /// An `UpgradeCertificate2` assembled from broadcast upgrade votes.
    UpgradeCertificateFormed(ValidCert<UpgradeCertificate2<T>>),
    ReVote(ReVoteMessage<T, Validated>),
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum ConsensusOutput<T: NodeType> {
    RequestBlockAndHeader(BlockAndHeaderRequest<T>),
    RequestState(StateRequest<T>),
    RequestDrbResult(EpochNumber),
    RecordAction(ViewNumber, Option<EpochNumber>, ActionKind),
    PersistProposal(SignedProposal<T, Proposal<T>>),
    SendProposal(SignedProposal<T, Proposal<T>>),
    SendTimeoutVote(TimeoutVote<T>, Option<CatchupEvidence<T>>),
    SendReVote(ReVoteMessage<T, Validated>),
    SendVote1(Vote1<T>),
    SendVote2(Vote2<T>),
    /// Persist the locked QC before the matching phase-2 vote is released.
    PersistHighQc(Certificate1<T>),
    PersistBoundaryQc(Certificate1<T>),
    SendTimeoutCertificate(TimeoutEvidence<T>, ViewNumber, EpochNumber),
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
    LockUpdated(ViewNumber),
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
    RequestMissingPayload {
        view: ViewNumber,
        payload_commitment: VidCommitment2,
    },
    /// Emitted when a node has reconstructed a block payload from VID shares.
    /// A payload obtained before its view decides rides along in
    /// `LeafDecided`; this event covers the rest, so downstream consumers
    /// (e.g. the query service) can store a payload that was still missing
    /// when the view decided.
    BlockPayloadReconstructed {
        view: ViewNumber,
        header: T::BlockHeader,
        payload: Arc<T::BlockPayload>,
    },
    /// Broadcast our own VID share so peers can reconstruct the block. Emitted
    /// right after `SendVote1` so it never delays the cert-forming vote.
    BroadcastVidShare(VidDisperseShare2<T>),
    /// The `UpgradeLock` has been updated; the certificate must be persisted.
    UpgradeDecided(UpgradeCertificate<T>),
}

/// What a proposal and this node's VID share for it must agree on to pair.
///
/// A share's epoch is supplied by its disperser and covered by no signature,
/// so keying on it means a share naming an epoch other than its proposal's
/// never pairs: it cannot be the one this node votes on, stores, or
/// broadcasts, and the honest share for the view can still arrive and pair.
///
/// That makes the share's epoch exactly as trustworthy as the proposal's, and
/// no more.
type PairingKey = (ViewNumber, Option<EpochNumber>, VidCommitment2);

type UnpairedProposals<T> =
    BTreeMap<PairingKey, (<T as NodeType>::SignatureKey, ProposalMessage<T, Validated>)>;

type UnpairedVidShares<T> = BTreeMap<PairingKey, VidDisperseShare2<T>>;

type ProposalKey<T> = (ViewNumber, Commitment<Leaf2<T>>);

/// Views to retain decide inputs (`proposals`, `certs`, `certs2`) behind the
/// decided view, letting a late-broadcast Cert2 decide an older gap view.
pub(crate) const DECIDE_BUFFER: u64 = 20;

/// Views earlier than the current view at which the transport stops
/// retransmitting: the bound the coordinator gives the network's collection,
/// and so the earliest a missing payload is worth fetching.
pub(crate) const GC_MARGIN_VIEWS: NonZeroU64 = NonZeroU64::new(2).expect("2 > 0");

// The decide buffer retains the proposals the VID reconstructor reads.
const _: () = assert!(DECIDE_BUFFER >= VID_RECONSTRUCT_GC_MARGIN);

fn cert1_key<T: NodeType>(cert: &Certificate1<T>) -> ViewEpoch {
    ViewEpoch(cert.view_number(), LockView::of(cert).epoch)
}

fn cert2_key<T: NodeType>(cert: &Certificate2<T>) -> ViewEpoch {
    ViewEpoch(cert.view_number(), cert.data.epoch)
}

pub struct Consensus<T: NodeType> {
    proposals: Proposals<T>,
    signed_proposals: BTreeMap<ProposalKey<T>, SignedProposal<T, Proposal<T>>>,
    proposed_views: BTreeSet<ViewEpoch>,
    vid_shares: BTreeMap<ViewNumber, VidDisperseShare2<T>>,
    unpaired_proposals: UnpairedProposals<T>,
    unpaired_vid_shares: UnpairedVidShares<T>,
    states_verified: BTreeSet<ProposalKey<T>>,
    blocks_reconstructed: BTreeSet<(ViewNumber, VidCommitment2)>,
    /// Payloads this node built or obtained, which a decide attaches to its
    /// leaves. Also gates proposing: the leader's header must have its block.
    blocks: BTreeMap<(ViewNumber, VidCommitment2), T::BlockPayload>,
    certs1: BTreeMap<ViewEpoch, Certificate1<T>>,
    certs2: BTreeMap<ViewEpoch, Certificate2<T>>,
    timeout_certs: BTreeMap<ViewNumber, TimeoutEvidence<T>>,
    locked_cert: Option<Certificate1<T>>,
    headers: BTreeMap<(ViewNumber, Commitment<Leaf2<T>>), T::BlockHeader>,
    leaves: BTreeMap<ProposalKey<T>, Leaf2<T>>,
    /// Views actually emitted in a `LeafDecided`; once views can decide late,
    /// `last_decided_view` is only a high-water mark.
    decided_views: BTreeSet<ViewNumber>,
    /// Hard lower bound for deciding, pinned to the anchor on restart:
    /// `decided_views` is not persisted, so a replayed certificate pair could
    /// otherwise re-decide pre-anchor views.
    decide_floor_view: ViewNumber,
    last_decided_view: ViewNumber,
    last_decided_leaf: Leaf2<T>,
    drb_results: BTreeMap<EpochNumber, DrbResult>,

    voted_1_views: BTreeSet<ViewEpoch>,
    voted_2_views: BTreeSet<ViewEpoch>,
    sent_timeout_votes: BTreeSet<ViewEpoch>,

    /// For each view this node cast a vote1 in, the view its proposal was justified at.
    vote1_parent: BTreeMap<ViewNumber, ViewNumber>,

    /// Storage confirmations; sends are gated on these facts.
    stored_proposals: BTreeMap<ViewNumber, Vec<Commitment<Leaf2<T>>>>,
    stored_vids: BTreeSet<ViewNumber>,
    stored_actions: BTreeSet<(ViewNumber, ActionKind)>,
    requested_actions: BTreeSet<(ViewNumber, ActionKind)>,
    /// Highest locked QC confirmed persisted; gates release of phase-2 votes.
    stored_high_qc: Option<LockView>,

    /// Messages constructed and accounted for in `voted_*_views` /
    /// `proposed_views`, awaiting their storage confirmations. A pending vote2
    /// also records the locked-QC view it must see persisted before release.
    pending_vote1: BTreeMap<ViewEpoch, Vote1<T>>,
    pending_vote2: BTreeMap<ViewEpoch, PendingVote2<T>>,
    pending_proposal: BTreeMap<ViewEpoch, SignedProposal<T, Proposal<T>>>,
    pending_revote: BTreeMap<ViewEpoch, ReVoteMessage<T, Validated>>,
    pending_timeout_vote: BTreeMap<ViewNumber, PendingTimeoutVote<T>>,

    /// The highest view this node has sent, or may have sent before a
    /// restart, a lock-carrying timeout vote for.
    ///
    /// Under the certificate rule a node never votes2 at or before it: its
    /// timeout vote signed a lock earlier than the commit that vote2 would help
    /// form, and a proposal skipping that commit could use the certificate.
    timeout_vote_bar: Option<ViewNumber>,

    /// The epoch of the re-vote commit processed at each view.
    ///
    /// Not in `decided_views`: the next epoch may have a block of its own at
    /// a re-vote's view.
    revote_commits: BTreeMap<ViewNumber, EpochNumber>,

    /// Validated re-vote requests by view (see [`ReVote`]).
    revotes: BTreeMap<ViewNumber, ReVote<T>>,

    /// What was requested for each re-vote view, so it is requested once.
    revote_fetches: BTreeSet<(ViewNumber, RevoteFetch)>,

    /// An assembled or adopted upgrade certificate, kept until a leader turn
    /// attaches it, the upgrade decides, or `decide_by` passes.
    formed_upgrade_certificate: Option<ValidCert<UpgradeCertificate2<T>>>,

    /// View of the decided leaf whose upgrade certificate is in the
    /// `UpgradeLock` (see [`Self::maybe_decide_upgrade`]).
    decided_upgrade_carrier: Option<ViewNumber>,

    timeout_view: ViewNumber,
    /// Highest view this node may have acted in before a restart (from the
    /// persisted action log). Bars re-*recording* a view's Vote action, not
    /// re-casting the phase-2 vote itself (see [`Self::vote2_persisted`]).
    restart_barred_view: ViewNumber,
    current_view: ViewNumber,
    current_epoch: EpochNumber,

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

/// What a re-vote can lack and fetch: the voted block's proposal, or its payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum RevoteFetch {
    Proposal,
    Payload,
}

/// A phase-2 vote awaiting its storage confirmations.
struct PendingVote2<T: NodeType> {
    vote: Vote2<T>,
    /// The lock that must be persisted before the vote is released.
    lock: LockView,
    /// The view the voted block was proposed at; a re-vote's is earlier.
    block_view: ViewNumber,
}

/// A timeout vote awaiting its storage confirmation.
type PendingTimeoutVote<T> = (TimeoutVote<T>, Option<CatchupEvidence<T>>);

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
            proposals: Proposals::new(),
            signed_proposals: BTreeMap::new(),
            proposed_views: BTreeSet::new(),
            blocks: BTreeMap::new(),
            states_verified: BTreeSet::new(),
            blocks_reconstructed: BTreeSet::new(),
            certs1: BTreeMap::new(),
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
            current_epoch: EpochNumber::genesis(),
            stake_table_coordinator: membership_coordinator,
            voted_1_views: BTreeSet::new(),
            sent_timeout_votes: BTreeSet::new(),
            voted_2_views: BTreeSet::new(),
            vote1_parent: BTreeMap::new(),
            stored_proposals: BTreeMap::new(),
            stored_vids: BTreeSet::new(),
            stored_actions: BTreeSet::new(),
            requested_actions: BTreeSet::new(),
            stored_high_qc: None,
            pending_vote1: BTreeMap::new(),
            pending_vote2: BTreeMap::new(),
            pending_proposal: BTreeMap::new(),
            pending_revote: BTreeMap::new(),
            pending_timeout_vote: BTreeMap::new(),
            timeout_vote_bar: None,
            revote_commits: BTreeMap::new(),
            revotes: BTreeMap::new(),
            revote_fetches: BTreeSet::new(),
            formed_upgrade_certificate: None,
            decided_upgrade_carrier: None,
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

    pub fn public_key(&self) -> &T::SignatureKey {
        &self.public_key
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
        // A decided last block ends its epoch.
        if is_last_block(proposal.block_header.block_number(), *self.epoch_height) {
            self.set_current_epoch_max(proposal.epoch + 1);
        } else {
            self.set_current_epoch_max(proposal.epoch);
        }
        // The seed cert comes from persistent storage, so its lock is already persisted.
        self.bump_stored_high_qc(LockView::of(&cert1));
        assert_eq! {
            self.last_decided_leaf.view_number(),
            proposal.view_number,
            "the anchor proposal must be rebuilt from the leaf consensus was created with"
        }
        let anchor = self.last_decided_leaf.commit();
        self.hold_proposal_under(proposal, anchor);
        self.insert_cert1(cert1.clone());
        self.locked_cert = Some(cert1);
        for (view, commitment) in reconstructed {
            self.blocks_reconstructed.insert((view, commitment));
        }
    }

    /// Seed proposals loaded from storage on restart, keeping each one's signature.
    ///
    /// Fetches are answered only with signed proposals, such as the epoch's last block
    /// a re-vote needs.
    pub fn seed_signed_proposals(
        &mut self,
        proposals: impl IntoIterator<Item = SignedProposal<T, Proposal<T>>>,
    ) {
        for signed in proposals {
            let key = (signed.data.view_number, proposal_commitment(&signed.data));
            self.seed_proposals([signed.data.clone()]);
            self.signed_proposals.insert(key, signed);
        }
    }

    /// Seed proposals loaded from storage on restart so the decide chain-walk
    /// can follow `justify_qc` back through views the node had already seen,
    /// and so `maybe_vote_1`/`maybe_propose` can find the parent of the first
    /// post-restart proposal (otherwise never re-fetched).
    pub(crate) fn seed_proposals(&mut self, proposals: impl IntoIterator<Item = Proposal<T>>) {
        for proposal in proposals {
            self.hold_proposal(proposal);
        }
    }

    /// Restore the locked QC persisted on a prior run. Called after
    /// `seed_parent`: the persisted lock can be newer than the decided-anchor
    /// QC, and restoring it keeps the node from voting against a block it had
    /// already locked. The loaded value is persisted, so it also advances the
    /// persistence watermark.
    pub fn seed_locked_cert(&mut self, cert1: Certificate1<T>) {
        let lock = LockView::of(&cert1);
        self.bump_stored_high_qc(lock);
        self.insert_cert1(cert1.clone());
        if self
            .locked_cert
            .as_ref()
            .is_none_or(|locked| LockView::of(locked) < lock)
        {
            // A node is at least in its lock's epoch, which may be later than the anchor's.
            self.set_current_epoch_max(lock.epoch);
            self.locked_cert = Some(cert1);
        }
    }

    /// Seed the boundary certificate persisted on a prior run (see
    /// [`ConsensusOutput::PersistBoundaryQc`]), so that a restarted node can
    /// still lead a re-vote of the boundary block or the next epoch's first
    /// block.
    pub fn seed_boundary_cert(&mut self, cert1: Certificate1<T>) {
        if cert1.view_number() > self.decide_floor() {
            self.insert_cert1(cert1);
        }
    }

    /// Seed the `Certificate2` persisted for the anchor on a prior run, so that
    /// a node restarted on the last block of an epoch can lead the next
    /// epoch's first block.
    pub fn seed_anchor_cert2(&mut self, cert2: Certificate2<T>) {
        if cert2.data.leaf_commit == self.last_decided_leaf.commit() {
            self.insert_cert2(cert2);
        }
    }

    fn insert_cert1(&mut self, cert: Certificate1<T>) {
        self.certs1.entry(cert1_key(&cert)).or_insert(cert);
    }

    /// Insert a cert2 and returns if it is new for its view and epoch.
    fn insert_cert2(&mut self, cert: Certificate2<T>) -> bool {
        match self.certs2.entry(cert2_key(&cert)) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(cert);
                true
            },
            std::collections::btree_map::Entry::Occupied(_) => false,
        }
    }

    fn proposed_at(&self, view: ViewNumber, epoch: EpochNumber) -> bool {
        if self.upgrade_lock.certificate_rule(view) {
            self.proposed_views.contains(&ViewEpoch(view, epoch))
        } else {
            self.proposed_views
                .range(ViewEpoch::at_view(view))
                .next()
                .is_some()
        }
    }

    fn voted_1_at(&self, view: ViewNumber, epoch: EpochNumber) -> bool {
        if self.upgrade_lock.certificate_rule(view) {
            self.voted_1_views.contains(&ViewEpoch(view, epoch))
        } else {
            self.voted_1_views
                .range(ViewEpoch::at_view(view))
                .next()
                .is_some()
        }
    }

    fn voted_2_at(&self, view: ViewNumber, epoch: EpochNumber) -> bool {
        if self.upgrade_lock.certificate_rule(view) {
            self.voted_2_views.contains(&ViewEpoch(view, epoch))
        } else {
            self.voted_2_views
                .range(ViewEpoch::at_view(view))
                .next()
                .is_some()
        }
    }

    /// Advance the locked-QC persistence watermark to `lock` if it is newer.
    fn bump_stored_high_qc(&mut self, lock: LockView) {
        if self.stored_high_qc.is_none_or(|cur| cur < lock) {
            self.stored_high_qc = Some(lock);
        }
    }

    /// Whether the locked QC `required` is persisted.
    fn high_qc_persisted(&self, required: LockView) -> bool {
        self.stored_high_qc.is_some_and(|stored| stored >= required)
    }

    /// Whether a certificate at `view` is of an epoch earlier than this node's,
    /// under the certificate rule: one of the outgoing committee's late
    /// re-votes, which must not move this node through the incoming
    /// committee's views.
    fn behind_current_epoch(&self, view: ViewNumber, epoch: EpochNumber) -> bool {
        self.upgrade_lock.certificate_rule(view) && epoch < self.current_epoch
    }

    pub(crate) fn lock_view(&self) -> Option<LockView> {
        self.locked_cert.as_ref().map(LockView::of)
    }

    /// The proposal a certificate at `view` over `leaf_commit` certifies, with
    /// the view it was proposed at.
    ///
    /// That is `view` itself, except for a re-vote, which certifies the last
    /// block of an epoch again at a later view.
    fn certified_proposal(
        &self,
        view: ViewNumber,
        leaf_commit: Commitment<Leaf2<T>>,
        block_number: Option<u64>,
    ) -> Option<(ViewNumber, &Proposal<T>)> {
        if let Some(proposal) = self.proposals.get(view, leaf_commit) {
            return Some((view, proposal));
        }
        if !block_number.is_some_and(|bn| is_last_block(bn, *self.epoch_height)) {
            return None;
        }
        self.proposals
            .latest_before(view, leaf_commit)
            .map(|proposal| (proposal.view_number, proposal))
    }

    /// A `Certificate2` this node holds over `leaf_commit`, at `from` or later.
    fn cert2_over(
        &self,
        from: ViewNumber,
        leaf_commit: Commitment<Leaf2<T>>,
    ) -> Option<&Certificate2<T>> {
        self.certs2
            .range(ViewEpoch::start(from)..)
            .map(|(_, cert2)| cert2)
            .find(|cert2| cert2.data.leaf_commit == leaf_commit)
    }

    /// Whether the parent block counts as reconstructed for voting: this node
    /// holds its payload, or its lock certifies the parent leaf.
    ///
    /// A lock is taken only on a reconstructed block, so a restarted node can
    /// vote on the first proposal built on its restored lock. The exception is
    /// an epoch's last block taken from an epoch change, whose only child opens
    /// the next epoch and needs no parent payload. The leaf, which covers the
    /// block's view, is compared rather than the view, since a re-vote's
    /// certificate is at a later view than its block.
    fn parent_reconstructed(
        &self,
        parent_view: ViewNumber,
        parent_block_commitment: VidCommitment2,
        parent_leaf: Commitment<Leaf2<T>>,
    ) -> bool {
        self.blocks_reconstructed
            .contains(&(parent_view, parent_block_commitment))
            || self
                .locked_cert
                .as_ref()
                .is_some_and(|lock| lock.data().leaf_commit == parent_leaf)
    }

    /// Seed a state certificate loaded from storage on restart, so a leader
    /// proposing on an epoch-root parent QC right after a restart does not
    /// stall on a missing state_cert.
    pub fn seed_state_cert(&mut self, state_cert: LightClientStateUpdateCertificateV2<T>) {
        self.state_certs.insert(state_cert.epoch, state_cert);
    }

    #[cfg(test)]
    pub(crate) fn state_cert_for_epoch(
        &self,
        epoch: EpochNumber,
    ) -> Option<&LightClientStateUpdateCertificateV2<T>> {
        self.state_certs.get(&epoch)
    }

    /// The proposal with the highest view below `view`, if any.
    pub fn last_proposal_before(&self, view: ViewNumber) -> Option<&Proposal<T>> {
        let v = self.proposals.range(..view).last()?.view_number;
        self.proposal_at(v)
    }

    /// The `Certificate1` of the latest epoch this node holds one of at `view`.
    pub fn cert1_at(&self, view: ViewNumber) -> Option<&Certificate1<T>> {
        self.certs1
            .range(ViewEpoch::at_view(view))
            .next_back()
            .map(|(_, c)| c)
    }

    /// The `Certificate2` of the latest epoch this node holds one of at `view`.
    pub fn cert2_at(&self, view: ViewNumber) -> Option<&Certificate2<T>> {
        self.certs2
            .range(ViewEpoch::at_view(view))
            .next_back()
            .map(|(_, c)| c)
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
            (Some(tc), _) => Some(CatchupEvidence::from(tc)),
            (None, Some(qc)) => Some(CatchupEvidence::Qc(qc.clone())),
            (None, None) => None,
        }
    }

    /// This node's signed VID share of `proposal`'s payload, if it holds one.
    ///
    /// The share held for a view is the one of the proposal the node paired it
    /// with, which need not be `proposal` when the leader equivocated.
    fn signed_vid_share(&self, p: &Proposal<T>) -> Option<SignedProposal<T, VidDisperseShare2<T>>> {
        self.vid_shares
            .get(&p.view_number)
            .filter(|share| {
                VidCommitment::V2(share.payload_commitment) == p.block_header.payload_commitment()
            })?
            .clone()
            .to_proposal(&self.private_key)
    }

    pub fn signed_proposal_fetch_request(
        &self,
        view: ViewNumber,
    ) -> Result<ProposalFetchRequest<T>, <T::SignatureKey as SignatureKey>::SignError> {
        ProposalFetchRequest::new(view, self.public_key.clone(), &self.private_key)
    }

    /// Return the timeout certificate that advanced consensus to `view`, if
    /// any. Keyed by the view it advanced *into* (i.e. one greater than the
    /// view it certified as timed out).
    pub fn timeout_cert_at(&self, view: ViewNumber) -> Option<&TimeoutEvidence<T>> {
        self.timeout_certs.get(&view)
    }

    pub fn is_reconstructed(&self, view: ViewNumber, payload_commitment: VidCommitment2) -> bool {
        self.blocks_reconstructed
            .contains(&(view, payload_commitment))
    }

    /// Newest view that can no longer be decided (and below which decide
    /// inputs are dropped): slides [`DECIDE_BUFFER`] behind the watermark,
    /// pinned at the restart anchor.
    pub(crate) fn decide_floor(&self) -> ViewNumber {
        max(
            self.last_decided_view.saturating_sub(DECIDE_BUFFER).into(),
            self.decide_floor_view,
        )
    }

    /// Apply consensus to the given input and collect protocol outputs.
    #[instrument(level = "debug", skip_all, fields(node = %self.node_id, view = %input.view_number()))]
    pub fn apply(&mut self, input: ConsensusInput<T>, outbox: &mut Outbox<ConsensusOutput<T>>) {
        // DRB results and formed upgrade certificates arrive asynchronously
        // with no specific view attached. Use `current_view` so that the
        // post-apply retries (`maybe_propose`, `maybe_vote_*`) target the view
        // the node is actually on.
        let view = if matches!(
            &input,
            ConsensusInput::DrbResult(..) | ConsensusInput::UpgradeCertificateFormed(..)
        ) {
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
                self.adopt_certified_proposal(message, outbox);
                self.adopt_timeout_lock(self.current_view, outbox);
                self.retry_revotes(outbox);
                return;
            },
            ConsensusInput::Certificate1(certificate) => {
                debug!(epoch = %certificate.epoch(), "apply: certificate1");
                let protocol = self.handle_certificate1(certificate);
                self.retry_later_certificates(view, outbox);
                protocol
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
            ConsensusInput::BlockReconstructed {
                view,
                payload_commitment,
                payload,
            } => {
                debug!(%view, "apply: block reconstructed");
                self.blocks_reconstructed.insert((view, payload_commitment));
                self.blocks.insert((view, payload_commitment), payload);
                // Retry the votable children whose vote1 is gated on this
                // parent's reconstruction. More than `view + 1` can be
                // waiting: while a view's payload was missing, every later
                // proposal extends it, and of those only the ones later than
                // the timeout bar can still be voted. Latest first, stopping
                // at the one we vote for: a vote at an earlier child adds
                // nothing and costs its vote2, which the later vote1 skips.
                let children: BTreeSet<ViewNumber> = self
                    .proposals
                    .range(view.max(self.timeout_view) + 1..)
                    .filter(|p| p.justify_qc.view_number() == view)
                    .map(|p| p.view_number)
                    .collect();
                for child in children.into_iter().rev() {
                    self.maybe_vote_1(child, outbox);
                    if self
                        .voted_1_views
                        .range(ViewEpoch::at_view(child))
                        .next()
                        .is_some()
                    {
                        break;
                    }
                }
                self.retry_later_certificates(view, outbox);
                Protocol::Continue
            },
            ConsensusInput::StateValidated(state_response) => {
                debug!(view = %state_response.view, "apply: state validated");
                self.states_verified
                    .insert((state_response.view, state_response.commitment));
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
                let stored_proposal = self.proposals.get(view, state_response.commitment);
                if let Some(proposal) = stored_proposal {
                    warn!(
                        %view,
                        block = %proposal.block_header.block_number(),
                        epoch = %proposal.epoch,
                        qc_view = %proposal.justify_qc.view_number(),
                        qc_epoch = ?proposal.justify_qc.epoch(),
                        "apply: state validation failed"
                    );
                } else {
                    warn!(%view, "apply: state validation failed (no stored proposal)");
                }
                return;
            },
            ConsensusInput::Timeout(view) => {
                let epoch = self.current_epoch;
                let leader = self.leader_label(view, epoch);
                warn!(%view, %epoch, %leader, "apply: timeout");
                self.handle_timeout(view, outbox)
            },
            ConsensusInput::TimeoutOneHonest(view) => {
                let epoch = self.current_epoch;
                let leader = self.leader_label(view, epoch);
                warn!(%view, %epoch, %leader, "apply: timeout (one honest)");
                self.handle_timeout(view, outbox)
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
            ConsensusInput::UpgradeCertificateFormed(cert) => {
                info!(
                    view = %cert.view_number(),
                    new_version = %cert.data.new_version,
                    "apply: upgrade certificate formed"
                );
                self.handle_upgrade_certificate_formed(cert)
            },
            ConsensusInput::ReVote(message) => {
                info!(
                    epoch = %message.revote.epoch,
                    block_view = %message.revote.cert1.view_number(),
                    "apply: re-vote request"
                );
                self.handle_revote(message, outbox)
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
        // A lock that moved, or a block that arrived, can unblock the leader
        // of the current view and a pending re-vote.
        self.maybe_propose(self.current_view, outbox);
        self.retry_revotes(outbox);
    }

    pub fn last_decided_view(&self) -> ViewNumber {
        self.last_decided_view
    }

    pub fn last_decided_leaf(&self) -> &Leaf2<T> {
        &self.last_decided_leaf
    }

    pub fn undecided_leaves(&self) -> impl Iterator<Item = &Leaf2<T>> {
        let first = (
            self.last_decided_view + 1,
            Commitment::default_commitment_no_preimage(),
        );
        self.leaves
            .range(first..)
            .filter(|((v, c), _)| {
                self.proposal_at(*v)
                    .is_some_and(|p| proposal_commitment(p) == *c)
            })
            .map(|(_, leaf)| leaf)
    }

    pub fn current_view(&self) -> ViewNumber {
        self.current_view
    }

    pub fn upgrade_lock(&self) -> &UpgradeLock<T> {
        &self.upgrade_lock
    }

    pub fn current_epoch(&self) -> Option<EpochNumber> {
        Some(self.current_epoch)
    }

    #[cfg(test)]
    pub fn set_view(&mut self, view: ViewNumber, epoch: EpochNumber) {
        self.current_view = view;
        self.current_epoch = epoch;
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
        if self.upgrade_lock.timeout_epoch_bound(last_barred) {
            self.timeout_vote_bar = max(self.timeout_vote_bar, Some(last_barred));
        }
        // Which views this node submitted its vote1 in is not persisted, only how far
        // it acted. A vote1 needs its proposal stored, so treat every seeded proposal
        // at or below the bar as one, this node may have voted for. Without that, a
        // vote2 re-cast after the restart could land in a view whose branch this node
        // had already left behind. Where a view holds more than one, which of them the
        // vote was for is not known either, so the earliest justify view is kept: it
        // is the one that bars the most vote2s.
        for proposal in self.proposals.range(..=last_barred) {
            let parent_view = proposal.justify_qc.view_number();
            self.vote1_parent
                .entry(proposal.view_number)
                .and_modify(|recorded| *recorded = (*recorded).min(parent_view))
                .or_insert(parent_view);
        }
        // `Coordinator::start` enters `current_view + 1`, so parking the cursor
        // at the high QC makes the node re-enter at `high_qc + 1`.
        let resume_view = self
            .stored_high_qc
            .map_or(anchor_view + 1, |lock| lock.view);
        self.set_current_view_max(resume_view);
    }

    /// Whether a proposal for `view` in `epoch` can still matter to this node.
    ///
    /// Locks are ordered by epoch, then view, so a proposal of a later epoch
    /// is wanted even if a lock of an earlier one is at a later view.
    pub fn wants_proposal_for_view(&self, view: &ViewNumber, epoch: EpochNumber) -> bool {
        let locked_too_new = self
            .lock_view()
            .is_some_and(|lock| lock > LockView { epoch, view: *view });
        // A proposal may already be in `self.proposals` because we received
        // an EpochChangeMessage for it (which carries the proposal but no
        // vid_share and does not trigger state validation).  In that case we
        // still want to process the real proposal message so handle_proposal
        // runs — it populates vid_shares and emits RequestState.
        let fully_processed =
            self.proposals.live(*view).is_some() && self.vid_shares.contains_key(view);
        !(locked_too_new || fully_processed)
    }

    pub fn proposals(&self) -> &Proposals<T> {
        &self.proposals
    }

    /// The proposal held at `view` that matters most, if any.
    ///
    /// The one a held Cert2 names, else the one a held Cert1 names, else the one
    /// this node paired with its own VID share, else any. A view normally holds
    /// one proposal; the order only matters when a leader equivocated.
    pub fn proposal_at(&self, view: ViewNumber) -> Option<&Proposal<T>> {
        self.cert2_at(view)
            .and_then(|c2| self.proposals.get(view, c2.data.leaf_commit))
            .or_else(|| {
                self.cert1_at(view)
                    .and_then(|c1| self.proposals.get(view, c1.data.leaf_commit))
            })
            .or_else(|| self.proposals.live(view))
            .or_else(|| self.proposals.at(view).next())
    }

    pub fn proposal_with_payload(&self, v: ViewNumber, c: VidCommitment2) -> Option<&Proposal<T>> {
        self.proposal_at(v)
            .filter(|p| p.block_header.payload_commitment() == VidCommitment::V2(c))
            .or_else(|| {
                self.proposals
                    .at(v)
                    .find(|p| p.block_header.payload_commitment() == VidCommitment::V2(c))
            })
    }

    pub fn signed_proposal(
        &self,
        v: ViewNumber,
        c: Commitment<Leaf2<T>>,
    ) -> Option<&SignedProposal<T, Proposal<T>>> {
        self.signed_proposals.get(&(v, c))
    }

    pub fn signed_proposal_at(&self, v: ViewNumber) -> Option<&SignedProposal<T, Proposal<T>>> {
        let c = proposal_commitment(self.proposal_at(v)?);
        self.signed_proposal(v, c)
    }

    /// The epoch change that ended `epoch`, if this node holds it.
    pub fn epoch_change_ending(
        &self,
        epoch: EpochNumber,
    ) -> Option<EpochChangeMessage<T, Validated>> {
        self.certs2
            .iter()
            .find_map(|(ViewEpoch(view, cert2_epoch), cert2)| {
                let block_number = cert2.data.block_number;
                if *cert2_epoch != epoch || !is_last_block(block_number, *self.epoch_height) {
                    return None;
                }
                let (block_view, proposal) =
                    self.certified_proposal(*view, cert2.data.leaf_commit, Some(block_number))?;
                let cert1 = self
                    .certs1
                    .get(&ViewEpoch(block_view, epoch))
                    .filter(|cert1| cert1.data.leaf_commit == cert2.data.leaf_commit)?;
                Some(EpochChangeMessage::validated(
                    cert1.clone(),
                    cert2.clone(),
                    proposal.clone(),
                ))
            })
    }

    /// The proposals to answer a fetch for `view` with.
    ///
    /// That is the one proposed at `view`, and the blocks `view`'s
    /// certificates certify if they are others. A re-vote's certificate
    /// certifies a block of an earlier view, and a node holding only the
    /// certificate knows no other view to ask for.
    pub fn signed_proposals_for_fetch(
        &self,
        view: &ViewNumber,
    ) -> Vec<&SignedProposal<T, Proposal<T>>> {
        let certified = self
            .certs1
            .range(ViewEpoch::at_view(*view))
            .map(|(_, cert)| (cert.data.leaf_commit, cert.data.block_number))
            .chain(
                self.certs2
                    .range(ViewEpoch::at_view(*view))
                    .map(|(_, cert)| (cert.data.leaf_commit, Some(cert.data.block_number))),
            )
            .filter_map(|(leaf_commit, block_number)| {
                let (block_view, _) = self.certified_proposal(*view, leaf_commit, block_number)?;
                self.signed_proposals.get(&(block_view, leaf_commit))
            });
        let mut proposals: Vec<_> = self.signed_proposal_at(*view).into_iter().collect();
        for proposal in certified {
            if !proposals.iter().any(|p| p.data == proposal.data) {
                proposals.push(proposal);
            }
        }
        proposals
    }

    /// Garbage-collect per-view state.
    ///
    /// The decide inputs (`proposals`, `certs`, `certs2`, deferred certs,
    /// `decided_views`) survive down to [`Self::decide_floor`] so a late
    /// Cert2 can still decide a gap view. Parked proposal halves survive as
    /// long, so such a Cert2 can adopt one instead of fetching.
    pub fn gc(&mut self, scope: GcScope) {
        match scope {
            GcScope::Local(view) => {
                let c = Commitment::default_commitment_no_preimage();
                let floor = self.decide_floor();
                self.headers = self.headers.split_off(&(view, c));
                self.proposed_views = self.proposed_views.split_off(&ViewEpoch::start(floor));
                self.states_verified = self.states_verified.split_off(&(view, c));
                self.timeout_certs = self.timeout_certs.split_off(&view);
                self.voted_1_views = self.voted_1_views.split_off(&ViewEpoch::start(floor));
                self.sent_timeout_votes =
                    self.sent_timeout_votes.split_off(&ViewEpoch::start(floor));
                self.voted_2_views = self.voted_2_views.split_off(&ViewEpoch::start(floor));
                if self
                    .formed_upgrade_certificate
                    .as_ref()
                    .is_some_and(|cert| view > cert.data.decide_by)
                {
                    self.formed_upgrade_certificate = None;
                }
            },
            GcScope::Decided(view) => {
                let vc = VidCommitment2::default();
                self.blocks = self.blocks.split_off(&(view, vc));
                self.blocks_reconstructed = self.blocks_reconstructed.split_off(&(view, vc));
                let keep_from = self.decide_floor();
                self.certs1 = self.certs1.split_off(&ViewEpoch::start(keep_from));
                self.certs2 = self.certs2.split_off(&ViewEpoch::start(keep_from));
                self.decided_views = self.decided_views.split_off(&keep_from);
                self.proposals.retain_from(keep_from);
                self.unpaired_proposals = self.unpaired_proposals.split_off(&(keep_from, None, vc));
                self.unpaired_vid_shares =
                    self.unpaired_vid_shares.split_off(&(keep_from, None, vc));
                self.vote1_parent = self.vote1_parent.split_off(&keep_from);
                let c = Commitment::default_commitment_no_preimage();
                self.leaves = self.leaves.split_off(&(view, c));
                self.signed_proposals = self.signed_proposals.split_off(&(view, c));
                self.vid_shares = self.vid_shares.split_off(&view);
                self.stored_proposals = self.stored_proposals.split_off(&view);
                self.stored_vids = self.stored_vids.split_off(&view);
                self.stored_actions = self.stored_actions.split_off(&(view, ActionKind::Vote));
                self.requested_actions =
                    self.requested_actions.split_off(&(view, ActionKind::Vote));
                self.pending_vote1 = self.pending_vote1.split_off(&ViewEpoch::start(view));
                self.pending_vote2 = self.pending_vote2.split_off(&ViewEpoch::start(view));
                self.pending_proposal = self.pending_proposal.split_off(&ViewEpoch::start(view));
                self.pending_revote = self.pending_revote.split_off(&ViewEpoch::start(view));
                self.pending_timeout_vote = self.pending_timeout_vote.split_off(&view);
                self.revotes = self.revotes.split_off(&keep_from);
                self.revote_commits = self.revote_commits.split_off(&keep_from);
                self.revote_fetches = self
                    .revote_fetches
                    .split_off(&(keep_from, RevoteFetch::Proposal));
                let epoch = EpochNumber::new(self.current_epoch.saturating_sub(1));
                self.drb_results = self.drb_results.split_off(&epoch);
                self.state_certs = self.state_certs.split_off(&epoch);
            },
            GcScope::Timeout(view) => {
                if self.keeps_payload_on_timeout(view) {
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

    /// Whether a view that timed out keeps its payload and shares.
    ///
    /// A view holding a certificate is likely to decide soon. So is the last
    /// block of an epoch: a re-vote commits it at a later view.
    pub(crate) fn keeps_payload_on_timeout(&self, view: ViewNumber) -> bool {
        self.cert1_at(view).is_some()
            || self.cert2_at(view).is_some()
            || self.proposals.at(view).any(|proposal| {
                is_last_block(proposal.block_header.block_number(), *self.epoch_height)
            })
    }

    pub fn leader_of(&self, view: ViewNumber, epoch: EpochNumber) -> Option<T::SignatureKey> {
        match self
            .stake_table_coordinator
            .membership_for_epoch(Some(epoch))
        {
            Ok(stake_table) => match stake_table.leader(view) {
                Ok(leader) => Some(leader),
                Err(err) => {
                    warn!(%view, %epoch, %err, "failed to get leader from stake table");
                    None
                },
            },
            Err(err) => {
                warn!(%view, %epoch, %err, "failed to get stake table");
                None
            },
        }
    }

    /// Test-only: forcibly replace the proposal stored at `view`.
    ///
    /// Used to simulate a node that holds only a proposal no certificate names
    /// at `view`, the certified one having never arrived. No production code
    /// should ever do this.
    #[cfg(test)]
    pub(crate) fn force_set_proposal(&mut self, view: ViewNumber, proposal: Proposal<T>) {
        debug_assert_eq!(view, proposal.view_number);
        self.proposals.replace_view(proposal);
    }

    /// Pair a validated proposal with this node's VID share for the same payload.
    ///
    /// The half arriving first is parked under its [`PairingKey`].
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
        let key = (view, Some(proposal.proposal.data.epoch), commit);
        let Some(vid_share) = self.unpaired_vid_shares.remove(&key) else {
            self.unpaired_proposals.insert(key, (sender, proposal));
            return Protocol::Abort;
        };
        self.on_proposal_paired(sender, proposal, vid_share, outbox)
    }

    /// Pair this node's VID share with a validated proposal for the same payload.
    ///
    /// The half arriving first is parked under its [`PairingKey`].
    fn pair_vid_share(
        &mut self,
        vid_share: VidDisperseShare2<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let key = (
            vid_share.view_number(),
            vid_share.epoch,
            vid_share.payload_commitment,
        );
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
        // Refuse the share before `ProposalPaired` persists it and seeds the
        // reconstructor with its common.
        if !ns_lens_match_metadata(
            &vid_share.common,
            &proposal.proposal.data.block_header.metadata().encode(),
        ) {
            warn!(
                view = %proposal.view_number(), proposer = %KeyPrefix::from(&sender),
                "VID share namespace lengths disagree with the proposal's namespace table"
            );
            return Protocol::Abort;
        }
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

        if !self.wants_proposal_for_view(&view, proposal.proposal.data.epoch) {
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

        // Under the certificate rule a vote1 is checked against the proposal's
        // timeout certificate, not against this node's lock (`maybe_vote_1`).
        if !self.upgrade_lock.certificate_rule(view)
            && let Err(err) = self.is_safe(&proposal)
        {
            warn!(
                %view, %proposer, block = %block_number, %epoch, %qc_view, %qc_epoch, %err,
                "proposal not safe"
            );
            return Protocol::Abort;
        }

        let payload_size = Some(vid_share.payload_byte_len());

        // Store the proposal before the DRB check so it is not lost when
        // the DRB is not yet available (e.g. a node catching up after a
        // restart).  Voting is deferred to `maybe_vote_1` which verifies
        // the DRB before casting a vote.
        let commit = self.hold_proposal(proposal.clone());
        self.proposals.set_live(view, commit);
        self.signed_proposals
            .insert((view, commit), signed_proposal.clone());
        self.vid_shares.insert(view, vid_share);
        self.adopt_certified_drb(view);
        self.adopt_proposal_upgrade_certificate(&proposal);
        // The first block of an epoch carries its parent's commit, verified
        // with the proposal, which decides the previous epoch's last block.
        if let Some(cert2) = proposal.next_epoch_justify_qc.clone() {
            let commit_view = cert2.view_number();
            if commit_view > self.decide_floor() && self.insert_cert2(cert2) {
                self.maybe_decide(commit_view, outbox);
            }
        }
        // The parent's certificate and the timeout certificate were verified
        // with the proposal, and put this node in the proposal's view.
        let parent_epoch = LockView::of(&proposal.justify_qc).epoch;
        self.advance_on(proposal.justify_qc.clone(), parent_epoch, outbox);
        if let Some(tc) = proposal.view_change_evidence.clone()
            && !self.timeout_certs.contains_key(&view)
        {
            let tc_epoch = HasEpoch::epoch(&tc).unwrap_or(proposal.epoch);
            self.handle_timeout_certificate(ValidCert::new(tc, tc_epoch), outbox);
        }

        self.request_parent_proposal_if_missing(&proposal, outbox);

        if let Some(state_cert) = validated_state_cert(&proposal, *self.epoch_height) {
            self.state_certs
                .insert(state_cert.epoch, state_cert.clone());
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

        self.request_state(&proposal, payload_size, outbox);

        outbox.push_back(ConsensusOutput::ProposalValidated {
            proposal: signed_proposal,
            sender,
        });

        self.request_block_and_header_if_next_leader(&proposal, outbox);

        Protocol::Continue
    }

    /// Take in a proposal this node did not receive live together with its
    /// VID share: one fetched from a peer, or one parked in
    /// `unpaired_proposals` that a certificate has since vouched for. Its
    /// parked ancestors come with it, oldest first, so each finds its parent
    /// stored rather than fetching it.
    #[instrument(level = "debug", skip_all)]
    fn adopt_certified_proposal(
        &mut self,
        message: ProposalMessage<T, Validated>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        let chain = self.parked_ancestors(message);
        if chain.len() > 1 {
            debug!(ancestors = chain.len() - 1, "adopting parked ancestors");
        }
        for message in chain.into_iter().rev() {
            let view = message.proposal.data.view_number;
            self.store_certified_proposal(message, outbox);
            // The proposal itself may now be decidable (e.g. cert2 arrived first
            // and triggered the fetch).
            self.maybe_decide(view, outbox);
            // Views extending it may be blocked on it.
            let extending_views: BTreeSet<ViewNumber> = self
                .proposals
                .range(view + 1..)
                .filter(|proposal| proposal.justify_qc.view_number() == view)
                .map(|proposal| proposal.view_number)
                .collect();
            for extending_view in extending_views {
                self.maybe_vote_1(extending_view, outbox);
                self.maybe_vote_2_and_update_lock(extending_view, outbox);
                self.maybe_decide(extending_view, outbox);
            }
            self.retry_later_certificates(view, outbox);
            self.maybe_propose(view + 1, outbox);
        }
    }

    /// `message` followed by its parked ancestors, nearest first, up to the
    /// first parent this node already holds or has nothing parked for.
    ///
    /// A loop, not recursion through `request_parent_proposal_if_missing`: the
    /// chain is as long as the run of views parked without a share.
    fn parked_ancestors(
        &self,
        message: ProposalMessage<T, Validated>,
    ) -> Vec<ProposalMessage<T, Validated>> {
        let mut chain = vec![message];
        loop {
            let proposal = &chain.last().expect("starts non-empty").proposal.data;
            let view = proposal.view_number;
            let parent_view = proposal.justify_qc.view_number();
            let leaf_commit = proposal.justify_qc.data().leaf_commit;
            if parent_view >= view
                || parent_view <= self.last_decided_view
                || self.proposals.contains(parent_view, leaf_commit)
            {
                break;
            }
            let Some(parent) = self.parked_proposal(parent_view, leaf_commit) else {
                break;
            };
            chain.push(parent);
        }
        chain
    }

    fn store_certified_proposal(
        &mut self,
        message: ProposalMessage<T, Validated>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        let signed_proposal = message.proposal;
        let proposal = signed_proposal.data.clone();
        let view = proposal.view_number;
        if view <= self.last_decided_view {
            debug!(%view, "certified proposal at or below decided view; discarding");
            return;
        }
        let commit = proposal_commitment(&proposal);
        if self.proposals.contains(view, commit) {
            debug!(%view, "certified proposal already present; discarding");
            return;
        }
        self.signed_proposals
            .insert((view, commit), signed_proposal);
        self.hold_proposal(proposal.clone());
        // Parked ancestors are stored ahead of this proposal
        // (`adopt_certified_proposal`), so this only fetches a parent nothing
        // parked can supply.
        self.request_parent_proposal_if_missing(&proposal, outbox);
        self.adopt_certified_drb(view);
        self.adopt_proposal_upgrade_certificate(&proposal);

        let payload_size = self.payload_size_for(&proposal);
        self.request_state(&proposal, payload_size, outbox);
        self.request_block_and_header_if_next_leader(&proposal, outbox);
    }

    /// Adopt the live proposal parked for `view` awaiting this node's VID
    /// share, if it is the leaf `leaf_commit` names. Returns whether one was adopted.
    ///
    /// The proposal stays parked: a share arriving later still pairs with it and
    /// runs the live path, the only one that stores the share for `maybe_vote_1`.
    fn adopt_unpaired_proposal(
        &mut self,
        view: ViewNumber,
        leaf_commit: Commitment<Leaf2<T>>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> bool {
        let Some(message) = self.parked_proposal(view, leaf_commit) else {
            return false;
        };
        debug!(%view, "adopting live proposal parked without a vid share");
        self.adopt_certified_proposal(message, outbox);
        true
    }

    /// The live proposal parked for `view` awaiting this node's VID share, if
    /// it is the leaf `leaf_commit` names.
    fn parked_proposal(
        &self,
        view: ViewNumber,
        leaf_commit: Commitment<Leaf2<T>>,
    ) -> Option<ProposalMessage<T, Validated>> {
        let vc = VidCommitment2::default();
        let range = (view, None, vc)..(view + 1, None, vc);
        self.unpaired_proposals
            .range(range)
            .map(|(_, (_, message))| message)
            .find(|message| proposal_commitment(&message.proposal.data) == leaf_commit)
            .cloned()
    }

    fn request_state(
        &self,
        proposal: &Proposal<T>,
        payload_size: Option<u32>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        outbox.push_back(ConsensusOutput::RequestState(StateRequest {
            view: proposal.view_number(),
            parent_view: proposal.justify_qc.view_number(),
            epoch: proposal.epoch,
            block: proposal.block_header.block_number().into(),
            proposal: proposal.clone(),
            parent_commitment: proposal.justify_qc.data().leaf_commit,
            payload_size,
        }));
    }

    /// The payload size of an adopted proposal, if a share for it has arrived.
    /// `None` makes state validation skip the checks that need the size; a
    /// quorum already certified the proposal, so they have been run elsewhere.
    fn payload_size_for(&self, proposal: &Proposal<T>) -> Option<u32> {
        let view = proposal.view_number();
        let VidCommitment::V2(commitment) = proposal.block_header.payload_commitment() else {
            return None;
        };
        if let Some(share) = self.vid_shares.get(&view)
            && share.payload_commitment == commitment
        {
            return Some(share.payload_byte_len());
        }
        self.unpaired_vid_shares
            .get(&(view, Some(proposal.epoch), commitment))
            .map(|share| share.payload_byte_len())
    }

    fn request_block_and_header_if_next_leader(
        &self,
        proposal: &Proposal<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        let view = proposal.view_number();
        let epoch = if is_last_block(proposal.block_header.block_number(), *self.epoch_height) {
            proposal.epoch + 1
        } else {
            proposal.epoch
        };
        if self.is_leader(view + 1, epoch) {
            outbox.push_back(ConsensusOutput::RequestBlockAndHeader(
                BlockAndHeaderRequest {
                    view: view + 1,
                    epoch,
                    parent_proposal: proposal.clone(),
                },
            ));
        }
    }

    /// Ask for the payload of the view every proposal we could vote for is
    /// parented at.
    ///
    /// Leaders build on their lock, so the parent of the latest proposal we
    /// hold is that view, and it is the one view peers still retain: a node
    /// holding the payload of a certified later view would have locked there
    /// itself, so no peer keeps one. Its certificate is the proposal's
    /// `justify_qc`, which is also the only record of it for a node that
    /// missed the view's votes and the certificate broadcast that followed
    /// them. The QC is read, not stored, so `certs` keeps meaning
    /// certificates that arrived as certificates.
    ///
    /// Views at or earlier than the lock cannot be what blocks us: safety
    /// pins the justify_qc of any proposal we may still vote for at or later
    /// than the lock, and a parent equal to the lock counts as reconstructed.
    /// Later than `current_view - GC_MARGIN_VIEWS` a missing share broadcast
    /// may still arrive on its own, and fetching would race it for a whole
    /// payload.
    fn request_missing_payloads(&self, outbox: &mut Outbox<ConsensusOutput<T>>) {
        let earliest = self
            .lock_view()
            .map_or_else(ViewNumber::genesis, |lock| lock.view)
            + 1;
        let latest = ViewNumber::from(self.current_view.saturating_sub(GC_MARGIN_VIEWS.get()));

        let Some(child) = self.proposals.last() else {
            return;
        };

        let view = child.justify_qc.view_number();

        if view < earliest || view > latest {
            return;
        }

        // A view decided as an ancestor of a later one needs no vote from us,
        // so its payload no longer blocks anything. Its proposal outlives the
        // decide by the decide buffer, which would otherwise keep it a
        // candidate for as long as the lock stays put.
        if self.decided_views.contains(&view) {
            return;
        }

        let Some(proposal) = self.proposals.get(view, child.justify_qc.data.leaf_commit) else {
            return;
        };

        let VidCommitment::V2(payload_commitment) = proposal.block_header.payload_commitment()
        else {
            return;
        };

        if self
            .blocks_reconstructed
            .contains(&(view, payload_commitment))
        {
            return;
        }

        outbox.push_back(ConsensusOutput::RequestMissingPayload {
            view,
            payload_commitment,
        });
    }

    fn request_parent_proposal_if_missing(
        &mut self,
        proposal: &Proposal<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        let parent_view = proposal.justify_qc.view_number();
        let leaf_commit = proposal.justify_qc.data().leaf_commit;
        if parent_view <= self.last_decided_view
            || self.proposals.contains(parent_view, leaf_commit)
        {
            return;
        }
        // Only a parent behind the proposal is adopted; adopting for a forward
        // QC could re-enter adoption without end.
        if parent_view < proposal.view_number
            && self.adopt_unpaired_proposal(parent_view, leaf_commit, outbox)
        {
            return;
        }
        warn!(
            view = %proposal.view_number,
            %parent_view,
            "parent proposal missing; requesting fetch"
        );
        outbox.push_back(ConsensusOutput::RequestMissingProposal {
            view: parent_view,
            leaf_commit,
        });
    }

    #[instrument(level = "debug", skip_all)]
    fn handle_certificate1(&mut self, certificate: ValidCert<Certificate1<T>>) -> Protocol {
        let view = certificate.view_number();
        if view <= self.decide_floor() {
            return Protocol::Continue;
        }
        self.insert_cert1(certificate.into_cert());
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
        if self.certs2.contains_key(&cert2_key(certificate.cert())) {
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
        let leaf_commit = certificate.data.leaf_commit;
        let block_number = Some(certificate.data.block_number);
        self.insert_cert2(certificate.into_cert());
        if view > self.last_decided_view
            && self
                .certified_proposal(view, leaf_commit, block_number)
                .is_none()
            && !self.adopt_unpaired_proposal(view, leaf_commit, outbox)
        {
            warn!(%view, "have certificate2 but no proposal; requesting fetch");
            outbox.push_back(ConsensusOutput::RequestMissingProposal { view, leaf_commit });
        }
        Protocol::Continue
    }

    /// Adopt the successor-epoch DRB carried by the transition leaf at `view`,
    /// once we hold both its proposal and a QC certifying it. A Cert1 over the
    /// leaf is a quorum's endorsement of its `next_drb_result`, so a catching-up
    /// node can use it without waiting on a separate successor-epoch catchup.
    fn adopt_certified_drb(&mut self, view: ViewNumber) {
        let Some(proposal) = self
            .certs1
            .range(ViewEpoch::at_view(view))
            .rev()
            .find_map(|(_, cert)| self.proposals.get(view, cert.data.leaf_commit))
        else {
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
        self.drb_results.insert(next_epoch, drb);
        debug!(%view, %next_epoch, "adopted quorum-certified next_drb_result");
    }

    fn handle_upgrade_certificate_formed(
        &mut self,
        cert: ValidCert<UpgradeCertificate2<T>>,
    ) -> Protocol {
        if self.upgrade_lock.decided_upgrade_cert().is_some() {
            return Protocol::Continue;
        }
        if self.current_view > cert.data.decide_by {
            debug!(
                view = %cert.view_number(),
                decide_by = %cert.data.decide_by,
                "upgrade certificate formed past its decide deadline; dropping"
            );
            return Protocol::Continue;
        }
        if self
            .formed_upgrade_certificate
            .as_ref()
            .is_none_or(|held| held.view_number() < cert.view_number())
        {
            self.formed_upgrade_certificate = Some(cert);
        }
        Protocol::Continue
    }

    /// Lift the upgrade certificate attached to a stored proposal, so a node
    /// that never saw the upgrade votes can still re-attach it when leading.
    fn adopt_proposal_upgrade_certificate(&mut self, proposal: &Proposal<T>) {
        let Some(cert) = &proposal.upgrade_certificate else {
            return;
        };
        if self.upgrade_lock.decided_upgrade_cert().is_some() {
            return;
        }
        if self
            .formed_upgrade_certificate
            .as_ref()
            .is_some_and(|held| held.view_number() >= cert.view_number())
        {
            return;
        }
        debug!(
            view = %proposal.view_number,
            cert_view = %cert.view_number(),
            "adopted upgrade certificate from proposal"
        );
        self.formed_upgrade_certificate = Some(ValidCert::new(cert.clone(), cert.data.epoch));
    }

    /// The upgrade certificate to attach to this node's proposal at `view`.
    /// The `Leaf2` carries it without its epoch, so validators require it to
    /// bind the carrying proposal's epoch.
    fn upgrade_certificate_to_attach(
        &self,
        view: ViewNumber,
        epoch: EpochNumber,
    ) -> Option<UpgradeCertificate2<T>> {
        let cert = self.formed_upgrade_certificate.as_ref()?;
        let attachable = self.upgrade_lock.decided_upgrade_cert().is_none()
            && view <= cert.data.decide_by
            && cert.epoch() == epoch;
        attachable.then(|| cert.cert().clone())
    }

    /// Decide the upgrade when a decided leaf carries a certificate within
    /// its `decide_by` deadline: flip the shared `UpgradeLock` and emit
    /// [`ConsensusOutput::UpgradeDecided`] so the certificate is persisted.
    ///
    /// The chain can carry more than one certificate and gap fills decide
    /// carriers out of order, so the *earliest* carrying leaf wins: an
    /// earlier carrier overrides a later one.
    fn maybe_decide_upgrade(
        &mut self,
        decided: &[Leaf2<T>],
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        // A certificate restored from storage has no known carrier view; its
        // carrier is at or below the decided anchor, so never override it.
        if self.decided_upgrade_carrier.is_none()
            && self.upgrade_lock.decided_upgrade_cert().is_some()
        {
            return;
        }
        // `decided` is ordered newest first.
        for leaf in decided.iter().rev() {
            let Some(cert) = leaf.upgrade_certificate() else {
                continue;
            };
            if leaf.view_number() > cert.data.decide_by {
                warn!(
                    view = %leaf.view_number(),
                    decide_by = %cert.data.decide_by,
                    "decided leaf carries an expired upgrade certificate; ignoring"
                );
                continue;
            }
            if self
                .decided_upgrade_carrier
                .is_some_and(|carrier| carrier <= leaf.view_number())
            {
                continue;
            }
            info!(
                target: "announce",
                view = %leaf.view_number(),
                new_version = %cert.data.new_version,
                first_view = %cert.data.new_version_first_view,
                "upgrade decided"
            );
            self.decided_upgrade_carrier = Some(leaf.view_number());
            self.upgrade_lock.set_decided_upgrade_cert(cert.clone());
            self.formed_upgrade_certificate = None;
            outbox.push_back(ConsensusOutput::UpgradeDecided(cert.clone()));
        }
    }

    /// Advance our view based on a quorum certificate.
    #[instrument(level = "debug", skip_all)]
    fn handle_advance_view(
        &mut self,
        cert1: ValidCert<Certificate1<T>>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let epoch = cert1.epoch();
        self.advance_on(cert1.into_cert(), epoch, outbox);
        Protocol::Continue
    }

    /// Keep `cert1` of `epoch`, vote2 and lock on it if this node can, and
    /// move to the view after it.
    fn advance_on(
        &mut self,
        cert1: Certificate1<T>,
        epoch: EpochNumber,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        let view = cert1.view_number();
        if view < self.current_view && view <= self.decide_floor() {
            return;
        }

        self.insert_cert1(cert1);
        self.adopt_certified_drb(view);

        // Ensure we submit a vote2 if we can:
        self.maybe_vote_2_and_update_lock(view, outbox);
        self.maybe_decide(view, outbox);

        let curr_view = self.current_view;
        let next_view = view + 1;

        if !self.behind_current_epoch(view, epoch) {
            self.set_current_view_max(next_view);
        }
        self.set_current_epoch_max(epoch);

        if self.current_view != curr_view {
            outbox.push_back(ConsensusOutput::ViewChanged(
                self.current_view,
                self.current_epoch,
            ));
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn handle_timeout(
        &mut self,
        view: ViewNumber,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let epoch = self.current_epoch;
        if view < self.current_view {
            debug!(
                %view,
                current_view = %self.current_view,
                current_epoch = %self.current_epoch,
                "ignoring timeout for stale view"
            );
            return Protocol::Abort;
        }
        let we_were_leader = self.is_leader(view, epoch);
        let proposed = self
            .proposed_views
            .range(ViewEpoch::at_view(view))
            .next()
            .is_some();
        if we_were_leader {
            if proposed {
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
        if !we_were_leader || proposed {
            if self
                .voted_1_views
                .range(ViewEpoch::at_view(view))
                .next()
                .is_some()
            {
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
        if let Some(cert1) = self.cert1_at(view) {
            let proposal_commit = self
                .proposals
                .get(view, cert1.data.leaf_commit)
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
        self.request_missing_payloads(outbox);

        if !self.staked_in_epoch(epoch) {
            return Protocol::Abort;
        }

        if self.upgrade_lock.timeout_epoch_bound(view) {
            self.send_timeout_vote3(view, epoch, outbox);
            return Protocol::Abort;
        }
        let vote = SimpleVote::create_signed_vote(
            TimeoutData2 {
                view,
                epoch: Some(epoch),
            },
            view,
            &self.public_key,
            &self.private_key,
            &self.upgrade_lock,
        )
        .map(TimeoutVote::V2);
        let vote = match vote {
            Ok(vote) => vote,
            Err(err) => {
                warn!(%view, %err, "failed to create timeout vote");
                return Protocol::Abort;
            },
        };
        let evidence = self.catchup_evidence();
        self.send_timeout_vote(vote, evidence, outbox);
        Protocol::Abort
    }

    /// Sign a timeout vote that carries this node's lock, and send it once
    /// its action is recorded.
    ///
    /// The bar is raised first, so from now on this node does not vote2 at
    /// `view` or before it, and recording the action keeps the bar where it
    /// is across a restart (see [`Self::timeout_vote_bar`]).
    fn send_timeout_vote3(
        &mut self,
        view: ViewNumber,
        epoch: EpochNumber,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        self.timeout_vote_bar = max(self.timeout_vote_bar, Some(view));
        // The genesis QC is no lock: nothing earlier could be skipped.
        let lock = self
            .locked_cert
            .clone()
            .filter(|cert| cert.view_number() != ViewNumber::genesis());
        let vote = match TimeoutBallot::sign(
            view,
            epoch,
            lock,
            &self.public_key,
            &self.private_key,
            &self.upgrade_lock,
        ) {
            Ok(ballot) => TimeoutVote::V3(ballot),
            Err(err) => {
                warn!(%view, %err, "failed to create timeout vote");
                return;
            },
        };
        let evidence = self.catchup_evidence();
        // Counted from signing: a vote held until its action is stored still goes out.
        self.sent_timeout_votes.insert(ViewEpoch(view, epoch));
        if self.stored_actions.contains(&(view, ActionKind::Timeout)) {
            self.send_timeout_vote(vote, evidence, outbox);
        } else {
            self.request_action(view, Some(epoch), ActionKind::Timeout, outbox);
            self.pending_timeout_vote.insert(view, (vote, evidence));
        }
    }

    fn release_timeout_vote(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        if !self.stored_actions.contains(&(view, ActionKind::Timeout)) {
            return;
        }
        if let Some((vote, evidence)) = self.pending_timeout_vote.remove(&view) {
            self.send_timeout_vote(vote, evidence, outbox);
        }
    }

    fn send_timeout_vote(
        &mut self,
        vote: TimeoutVote<T>,
        evidence: Option<CatchupEvidence<T>>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        let epoch = vote.epoch().unwrap_or(self.current_epoch);
        self.sent_timeout_votes
            .insert(ViewEpoch(vote.view_number(), epoch));
        outbox.push_back(ConsensusOutput::SendTimeoutVote(vote, evidence));
    }

    /// Whether this node signed a timeout vote for `view` in `epoch`, or in any
    /// epoch for a certificate that does not bind one.
    fn sent_timeout_vote(&self, view: ViewNumber, epoch: Option<EpochNumber>) -> bool {
        match epoch {
            Some(epoch) => self.sent_timeout_votes.contains(&ViewEpoch(view, epoch)),
            None => self
                .sent_timeout_votes
                .range(ViewEpoch::at_view(view))
                .next()
                .is_some(),
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn handle_timeout_certificate(
        &mut self,
        certificate: ValidCert<TimeoutEvidence<T>>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let timed_out_view = certificate.view_number();
        let view = timed_out_view + 1;
        if view < self.current_view {
            debug!(
                %view,
                current_view = %self.current_view,
                "ignoring stale timeout certificate"
            );
            return Protocol::Abort;
        }
        if certificate.binds_epoch() != self.upgrade_lock.timeout_epoch_bound(timed_out_view) {
            warn!(
                %timed_out_view,
                "timeout certificate has the wrong form for its version"
            );
            return Protocol::Abort;
        }
        if let Some(stored) = self.timeout_certs.get(&view).map(HasEpoch::epoch) {
            if certificate.binds_epoch() && stored < Some(certificate.epoch()) {
                self.set_current_epoch_max(certificate.epoch());
                debug!(
                    %view,
                    epoch = %self.current_epoch,
                    "adopting the epoch of a later certificate"
                );
                self.timeout_certs.insert(view, certificate.into_cert());
                self.adopt_timeout_lock(view, outbox);
            } else {
                debug!(%view, "duplicate timeout certificate; already applied");
            }
            return Protocol::Continue;
        }
        // A node that voted in the view answered the one-honest indication
        // with its vote, which the others count.
        let forward = self.current_view <= timed_out_view
            && !self.sent_timeout_vote(
                timed_out_view,
                certificate.binds_epoch().then(|| certificate.epoch()),
            );
        self.timeout_certs.insert(view, certificate.cert().clone());
        self.set_current_view_max(view);
        if certificate.binds_epoch() {
            self.set_current_epoch_max(certificate.epoch());
        }
        self.request_missing_payloads(outbox);
        outbox.push_back(ConsensusOutput::ViewChanged(view, self.current_epoch));
        outbox.push_back(ConsensusOutput::ViewTimedOut(timed_out_view));
        if forward {
            outbox.push_back(ConsensusOutput::SendTimeoutCertificate(
                certificate.into_cert(),
                view,
                self.current_epoch,
            ));
        }
        self.adopt_timeout_lock(view, outbox);
        Protocol::Continue
    }

    /// Take in the lock the timeout certificate entering `view` carries.
    ///
    /// The lock may be later than this node's own: a leader must build on a
    /// block no earlier than it, and this node may have to lock on it to vote.
    /// Its block and payload are fetched if missing.
    fn adopt_timeout_lock(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        let Some((lock, cert)) = self
            .timeout_certs
            .get(&view)
            .and_then(|tc| tc.lock())
            .map(|(lock, cert)| (lock, cert.clone()))
        else {
            return;
        };
        if lock.view <= self.decide_floor() || self.lock_view().is_some_and(|own| own >= lock) {
            return;
        }
        let leaf_commit = cert.data.leaf_commit;
        let block_number = cert.data.block_number;
        self.insert_cert1(cert);
        self.adopt_certified_drb(lock.view);
        match self.certified_proposal(lock.view, leaf_commit, block_number) {
            None => {
                if !self.adopt_unpaired_proposal(lock.view, leaf_commit, outbox) {
                    debug!(%view, %lock, "timeout certificate lock without its block; requesting fetch");
                    outbox.push_back(ConsensusOutput::RequestMissingProposal {
                        view: lock.view,
                        leaf_commit,
                    });
                }
            },
            Some((block_view, proposal)) => {
                if let VidCommitment::V2(payload_commitment) =
                    proposal.block_header.payload_commitment()
                    && !self
                        .blocks_reconstructed
                        .contains(&(block_view, payload_commitment))
                {
                    debug!(%view, %lock, "timeout certificate lock without its payload; requesting fetch");
                    outbox.push_back(ConsensusOutput::RequestMissingPayload {
                        view: block_view,
                        payload_commitment,
                    });
                }
            },
        }
        self.maybe_vote_2_and_update_lock(lock.view, outbox);
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

        // The block is at its own certificate's view; a re-vote commits it at a
        // later view, after which the next epoch starts.
        let block_view = cert1.view_number();
        let commit_view = cert2.view_number();
        let boundary_epoch = cert2.data.epoch;
        let leaf_commit = cert2.data.leaf_commit;

        self.hold_proposal(proposal.clone());
        outbox.push_back(ConsensusOutput::PersistBoundaryQc(cert1.clone()));
        self.insert_cert1(cert1.clone());
        self.insert_cert2(cert2);
        self.adopt_certified_drb(block_view);

        // Compare epochs (not views) so a node that timed out past a boundary
        // it never saw can still recover via a genuinely new epoch change.
        if boundary_epoch < self.current_epoch {
            debug!(
                view = %commit_view,
                epoch = %boundary_epoch,
                current_epoch = %self.current_epoch,
                "ignoring stale epoch change for an epoch we have already entered"
            );
            return Protocol::Continue;
        }
        // A lock of the next epoch means this node is past the boundary, and
        // a later lock of the same epoch over another block means the message
        // is stale. A later lock over the block itself is a re-vote's.
        let boundary_lock = LockView {
            epoch: boundary_epoch,
            view: commit_view,
        };
        if let Some(locked) = self.locked_cert.as_ref()
            && LockView::of(locked) > boundary_lock
            && (LockView::of(locked).epoch > boundary_epoch
                || locked.data.leaf_commit != leaf_commit)
        {
            warn!("locked certificate is newer than the epoch change");
            return Protocol::Continue;
        }

        // Locked on without the payload, which the commit shows a quorum had,
        // so that a node new to the incoming committee can lock.
        if self
            .lock_view()
            .is_none_or(|lock| lock < LockView::of(&cert1))
        {
            self.locked_cert = Some(cert1.clone());
            outbox.push_back(ConsensusOutput::LockUpdated(block_view));
            outbox.push_back(ConsensusOutput::PersistHighQc(cert1));
        }

        let (curr_view, curr_epoch) = (self.current_view, self.current_epoch);
        let next_view = commit_view + 1;
        let next_epoch = boundary_epoch + 1;

        self.set_current_view_max(next_view);
        self.set_current_epoch_max(next_epoch);

        // The view after the boundary is relabelled with the next epoch even
        // if this node is in it already.
        if self.current_view != curr_view || self.current_epoch != curr_epoch {
            outbox.push_back(ConsensusOutput::ViewChanged(
                self.current_view,
                self.current_epoch,
            ));
        }

        // Request block and header if we're the first leader of the next epoch
        if self.is_leader(next_view, next_epoch) {
            outbox.push_back(ConsensusOutput::RequestBlockAndHeader(
                BlockAndHeaderRequest {
                    view: next_view,
                    epoch: next_epoch,
                    parent_proposal: proposal,
                },
            ));
        }

        Protocol::Continue
    }

    #[instrument(level = "debug", skip_all)]
    fn maybe_propose(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        if view <= self.timeout_view {
            return;
        }

        let rule_b = self.upgrade_lock.certificate_rule(view);
        // Voters refuse a timeout certificate of an epoch earlier than the
        // proposal's, such as a late one of the outgoing committee.
        let view_change_evidence = self
            .timeout_certs
            .get(&view)
            .filter(|tc| !(rule_b && tc.epoch() < Some(self.current_epoch)))
            .cloned();
        let Some(parent_cert) = self.proposal_parent(view, view_change_evidence.as_ref()) else {
            debug!("no parent certificate");
            return;
        };
        let Some((parent_view, proposal)) = self.parent_proposal(&parent_cert) else {
            debug!(parent = %parent_cert.view_number(), "no proposal for parent certificate");
            return;
        };
        let parent_commitment = if parent_view == ViewNumber::genesis() {
            proposal_commitment(&proposal)
        } else {
            parent_cert.data.leaf_commit
        };
        // The first block of an epoch needs a commit of its parent, and names
        // the parent's own certificate.
        let (justify_qc, next_epoch_justify_qc) = if parent_view != ViewNumber::genesis()
            && is_last_block(proposal.block_header.block_number(), *self.epoch_height)
        {
            let Some(cert2) =
                self.boundary_commit(view, parent_view, parent_commitment, &view_change_evidence)
            else {
                if rule_b {
                    self.maybe_request_revote(
                        view,
                        parent_view,
                        &proposal,
                        view_change_evidence.as_ref(),
                        outbox,
                    );
                }
                debug!("no next epoch justify QC");
                return;
            };
            let own = if parent_cert.view_number() == parent_view {
                parent_cert
            } else {
                let Some(own) = self
                    .certs1
                    .get(&ViewEpoch(parent_view, proposal.epoch))
                    .cloned()
                else {
                    debug!(%parent_view, "no certificate1 of the boundary block at its own view");
                    return;
                };
                own
            };
            (own, Some(cert2))
        } else {
            (parent_cert, None)
        };
        let proposal_epoch = if next_epoch_justify_qc.is_some() {
            proposal.epoch + 1
        } else {
            proposal.epoch
        };
        if self.proposed_at(view, proposal_epoch) {
            return;
        }
        if rule_b {
            // Voters refuse a proposal of an epoch earlier than their own, and
            // one whose parent is earlier than its timeout certificate's lock.
            if proposal_epoch < self.current_epoch {
                debug!(%proposal_epoch, current_epoch = %self.current_epoch, "proposal epoch is behind");
                return;
            }
            if let Some(tc) = &view_change_evidence
                && !certificate_rule_admits(&justify_qc, tc, proposal_epoch)
            {
                debug!(
                    parent = %LockView::of(&justify_qc),
                    "parent is earlier than the timeout certificate's lock"
                );
                return;
            }
        }

        let Some(header) = self.headers.get(&(view, parent_commitment)) else {
            // The header request issued on the TC targeted the lock held at
            // that moment; if the lock moved since, re-request. The same goes
            // for the first block of an epoch, whose parent's commit may come
            // after the view started. The block builder dedups by (view, parent).
            if (view_change_evidence.is_some() || next_epoch_justify_qc.is_some())
                && self.is_leader(view, proposal_epoch)
            {
                outbox.push_back(ConsensusOutput::RequestBlockAndHeader(
                    BlockAndHeaderRequest {
                        view,
                        epoch: proposal_epoch,
                        parent_proposal: proposal.clone(),
                    },
                ));
            }
            debug!("no block header");
            return;
        };
        let parent_cert = justify_qc;
        let VidCommitment::V2(block_commitment) = header.payload_commitment() else {
            debug!("header payload commitment is not V2");
            return;
        };
        if !self.blocks.contains_key(&(view, block_commitment)) {
            debug!("no block");
            return;
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
            if !check_qc_state_cert_correspondence(&parent_cert, &sc, *self.epoch_height) {
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
            justify_qc: parent_cert,
            next_epoch_justify_qc,
            upgrade_certificate: self.upgrade_certificate_to_attach(view, proposal_epoch),
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

        self.proposed_views.insert(ViewEpoch(view, proposal_epoch));
        outbox.push_back(ConsensusOutput::PersistProposal(message.clone()));
        self.request_action(view, Some(proposal_epoch), ActionKind::Propose, outbox);
        self.pending_proposal
            .insert(ViewEpoch(view, proposal_epoch), message);
    }

    /// The certificate a proposal at `view` would name as its parent.
    ///
    /// Without a timeout that is the certificate of the view before. After
    /// one it is this node's lock, except under the certificate rule a leader
    /// holding a commit of an epoch's last block builds the next epoch's
    /// first block on it, even when locked on a re-vote's certificate.
    fn proposal_parent(
        &self,
        view: ViewNumber,
        evidence: Option<&TimeoutEvidence<T>>,
    ) -> Option<Certificate1<T>> {
        let previous = ViewNumber::from(view.saturating_sub(1));
        if evidence.is_none() {
            return self.cert1_at(previous).cloned();
        }
        let lock = self.locked_cert.as_ref();
        if self.upgrade_lock.certificate_rule(view) {
            let epoch = lock.map_or_else(EpochNumber::genesis, |lock| LockView::of(lock).epoch);
            if let Some(boundary) = self.committed_boundary(epoch) {
                return Some(boundary);
            }
        }
        lock.cloned()
    }

    /// The own `Certificate1` of the latest last block of an epoch at or
    /// after `epoch` that this node holds a commit of.
    fn committed_boundary(&self, epoch: EpochNumber) -> Option<Certificate1<T>> {
        self.certs2
            .iter()
            .rev()
            .filter(|(_, cert2)| {
                cert2.data.epoch >= epoch
                    && is_last_block(cert2.data.block_number, *self.epoch_height)
            })
            .find_map(|(view, cert2)| {
                let (block_view, _) = self.certified_proposal(
                    view.0,
                    cert2.data.leaf_commit,
                    Some(cert2.data.block_number),
                )?;
                self.certs1
                    .get(&ViewEpoch(block_view, cert2.data.epoch))
                    .filter(|cert1| cert1.data.leaf_commit == cert2.data.leaf_commit)
                    .cloned()
            })
    }

    /// The proposal `parent` certifies and the view it was proposed at.
    fn parent_proposal(&self, parent: &Certificate1<T>) -> Option<(ViewNumber, Proposal<T>)> {
        let view = parent.view_number();
        // Keyed by the cert's `leaf_commit`, NOT by view alone: `self.proposals`
        // can hold another proposal of the same view than the one the cert
        // certified, and a header built on it carries a wrong block number.
        let found = self
            .certified_proposal(view, parent.data.leaf_commit, parent.data.block_number)
            .map(|(view, p)| (view, p.clone()));
        if found.is_none() && self.proposals.at(view).next().is_some() {
            warn!(
                parent_view = %view,
                "stored proposal at parent_view does not match parent cert's leaf_commit; \
                 refusing to propose with mismatched parent"
            );
        }
        found
    }

    /// The commit of the last block of an epoch, at `block_view` over
    /// `leaf_commit`, that the next epoch's first block at `view` can carry:
    /// one at the block's view or later, and before `view`.
    ///
    /// Without a timeout certificate the first block follows its parent
    /// directly, so only the block's own commit can come before it.
    fn boundary_commit(
        &self,
        view: ViewNumber,
        block_view: ViewNumber,
        leaf_commit: Commitment<Leaf2<T>>,
        evidence: &Option<TimeoutEvidence<T>>,
    ) -> Option<Certificate2<T>> {
        if evidence.is_none() && block_view + 1 != view {
            return None;
        }
        self.certs2
            .range(ViewEpoch::start(block_view)..ViewEpoch::start(view))
            .map(|(_, cert2)| cert2)
            .find(|cert2| cert2.data.leaf_commit == leaf_commit)
            .cloned()
    }

    /// Send a re-vote request for the last block of an epoch at `view`, if
    /// this node leads `view` in that epoch.
    ///
    /// `block`, proposed at `block_view`, has a `Certificate1` but no commit
    /// this node knows of. The request goes out in the view after the
    /// block's, not only after a timeout `tc`, so that nodes withholding their
    /// vote2 cannot stall the boundary.
    fn maybe_request_revote(
        &mut self,
        view: ViewNumber,
        block_view: ViewNumber,
        block: &Proposal<T>,
        tc: Option<&TimeoutEvidence<T>>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        let epoch = block.epoch;
        if self.proposed_at(view, epoch)
            || epoch < self.current_epoch
            || !self.is_leader(view, epoch)
        {
            return;
        }
        let timeout = match tc {
            None if block_view + 1 == view => None,
            Some(TimeoutEvidence::V3(timeout)) if timeout.epoch == epoch => Some(timeout.clone()),
            _ => return,
        };
        let leaf_commit = proposal_commitment(block);
        let Some(cert1) = self
            .certs1
            .get(&ViewEpoch(block_view, epoch))
            .filter(|cert1| cert1.data.leaf_commit == leaf_commit)
            .cloned()
        else {
            debug!(%block_view, "no certificate1 of the last block at its own view");
            return;
        };
        if let Some(tc) = tc
            && !certificate_rule_admits(&cert1, tc, epoch)
        {
            debug!(%block_view, "last block is earlier than the timeout certificate's lock");
            return;
        }
        let VidCommitment::V2(payload_commitment) = block.block_header.payload_commitment() else {
            return;
        };
        if !self.parent_reconstructed(block_view, payload_commitment, leaf_commit) {
            debug!(%block_view, "no payload of the last block to re-vote on");
            return;
        }
        let revote = ReVote {
            view,
            epoch,
            cert1,
            timeout,
        };
        let message = match ReVoteMessage::new(revote, &self.private_key) {
            Ok(message) => message,
            Err(err) => {
                warn!(%view, %err, "failed to sign re-vote request");
                return;
            },
        };
        info!(%view, %epoch, %block_view, "re-vote on the last block of the epoch");
        self.proposed_views.insert(ViewEpoch(view, epoch));
        self.request_action(view, Some(epoch), ActionKind::Propose, outbox);
        self.pending_revote.insert(ViewEpoch(view, epoch), message);
        self.release_proposal(view, outbox);
    }

    /// Decide on each `Certificate2` at `view`, earliest epoch first.
    #[instrument(level = "debug", skip_all)]
    fn maybe_decide(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        let epochs: Vec<EpochNumber> = self
            .certs2
            .range(ViewEpoch::at_view(view))
            .map(|(ViewEpoch(_, epoch), _)| *epoch)
            .collect();
        if epochs.is_empty() {
            debug!(%view, "cert2 not available");
        }
        for epoch in epochs {
            self.maybe_decide_at(ViewEpoch(view, epoch), outbox);
        }
    }

    fn maybe_decide_at(&mut self, key: ViewEpoch, outbox: &mut Outbox<ConsensusOutput<T>>) {
        let ViewEpoch(view, epoch) = key;
        // Any still-undecided view above the floor can decide, even one older
        // than the watermark (a gap).
        let floor = self.decide_floor();
        if view <= floor || self.decided_views.contains(&view) {
            return;
        }
        let Some(cert2) = self.certs2.get(&key).cloned() else {
            return;
        };
        let Some((block_view, proposal)) =
            self.certified_proposal(view, cert2.data.leaf_commit, Some(cert2.data.block_number))
        else {
            debug!(%view, "proposal not available");
            return;
        };
        if block_view != view {
            // `view` is not marked decided: the next epoch may have a block
            // of its own there.
            if self.revote_commits.get(&view) == Some(&cert2.data.epoch) {
                return;
            }
            if block_view <= floor || self.decided_views.contains(&block_view) {
                self.revote_commits.insert(view, cert2.data.epoch);
                return;
            }
        }
        let proposal_commit = cert2.data.leaf_commit;
        // A Cert2 can arrive before its Cert1; require both before mutating any
        // decided state. A re-vote's Cert1 stands in for the block's own.
        let Some(cert1) = self
            .certs1
            .get(&ViewEpoch(block_view, epoch))
            .or_else(|| self.certs1.get(&key))
            .filter(|cert1| cert1.data.leaf_commit == proposal_commit)
            .cloned()
        else {
            debug!(%view, "cert1 missing");
            return;
        };
        // Handle Epoch Change by broadcasting the epoch change message if we have
        // all the data we need.
        if is_last_block(proposal.block_header.block_number(), *self.epoch_height)
            && cert1.view_number() == block_view
        {
            let epoch_change =
                EpochChangeMessage::validated(cert1.clone(), cert2.clone(), proposal.clone());
            outbox.push_back(ConsensusOutput::SendEpochChange(epoch_change));
        }
        // we have a second certificate, and matching proposal, it is decided.
        let mut leaf: Leaf2<T> = proposal.clone().into();
        if let VidCommitment::V2(pc) = proposal.block_header.payload_commitment()
            && let Some(payload) = self.blocks.get(&(block_view, pc))
        {
            leaf.fill_block_payload_unchecked(payload.clone());
        }
        let mut decided = vec![leaf];
        let mut vid_shares = vec![self.signed_vid_share(proposal)];

        let mut parent_view = proposal.justify_qc.view_number();
        let mut parent_commit = proposal.justify_qc.data.leaf_commit;

        // A missing ancestor is a gap; a later Cert2 for it fills it in.
        while parent_view > floor
            && !self.decided_views.contains(&parent_view)
            && let Some(proposal) = self.proposals.get(parent_view, parent_commit)
        {
            let mut leaf: Leaf2<T> = proposal.clone().into();
            if let VidCommitment::V2(pc) = proposal.block_header.payload_commitment()
                && let Some(payload) = self.blocks.get(&(parent_view, pc))
            {
                leaf.fill_block_payload_unchecked(payload.clone());
            }
            vid_shares.push(self.signed_vid_share(proposal));
            decided.push(leaf);
            parent_view = proposal.justify_qc.view_number();
            parent_commit = proposal.justify_qc.data.leaf_commit;
        }
        self.decided_views
            .extend(decided.iter().map(|l| l.view_number()));
        if block_view != view {
            self.revote_commits.insert(view, cert2.data.epoch);
        }
        // A gap-fill decide of an older view must not move the watermark backward.
        if block_view > self.last_decided_view {
            self.last_decided_view = block_view;
            self.last_decided_leaf = decided[0].clone();
        }
        self.maybe_decide_upgrade(&decided, outbox);
        outbox.push_back(ConsensusOutput::LeafDecided {
            leaves: decided,
            cert1,
            cert2: Some(cert2),
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
            StorageOutput::HighQc(lock) => {
                self.bump_stored_high_qc(lock);
                // A newly persisted lock can unblock vote2 across many views; re-check all.
                let pending: Vec<ViewEpoch> = self.pending_vote2.keys().copied().collect();
                for key in pending {
                    self.release_vote2_at(key, outbox);
                }
                return;
            },
        }
        self.release_vote1(view, outbox);
        self.release_vote2(view, outbox);
        self.release_proposal(view, outbox);
        self.release_timeout_vote(view, outbox);
    }

    fn release_vote1(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        let pending: Vec<ViewEpoch> = self
            .pending_vote1
            .range(ViewEpoch::at_view(view))
            .map(|(key, _)| *key)
            .collect();
        for key in pending {
            self.release_vote1_at(key, outbox);
        }
    }

    fn release_vote1_at(&mut self, key: ViewEpoch, outbox: &mut Outbox<ConsensusOutput<T>>) {
        let ViewEpoch(view, epoch) = key;
        let Some(vote1) = self.pending_vote1.get(&key) else {
            return;
        };
        // A re-vote's block was proposed, and stored, at an earlier view.
        let is_revote = self.revotes.get(&view).is_some_and(|revote| {
            revote.epoch == epoch && revote.cert1.data.leaf_commit == vote1.vote.data.leaf_commit
        });
        if !self.stored_actions.contains(&(view, ActionKind::Vote))
            || (!is_revote && !self.is_proposal_stored(view, &vote1.vote.data.leaf_commit))
        {
            return;
        }
        let vote1 = self.pending_vote1.remove(&key).expect("checked above");
        if view <= self.timeout_view {
            debug!(%view, "dropping pending vote1 for timed-out view");
            return;
        }
        outbox.push_back(ConsensusOutput::SendVote1(vote1));
        if is_revote {
            return;
        }
        if let Some(vid_share) = self.vid_shares.get(&view).cloned() {
            outbox.push_back(ConsensusOutput::BroadcastVidShare(vid_share));
        } else {
            debug!(%view, "vid share gone for released vote1; skipping broadcast");
        }
    }

    fn release_vote2(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        let pending: Vec<ViewEpoch> = self
            .pending_vote2
            .range(ViewEpoch::at_view(view))
            .map(|(key, _)| *key)
            .collect();
        for key in pending {
            self.release_vote2_at(key, outbox);
        }
    }

    fn release_vote2_at(&mut self, key: ViewEpoch, outbox: &mut Outbox<ConsensusOutput<T>>) {
        let view = key.0;
        let Some(pending) = self.pending_vote2.get(&key) else {
            return;
        };
        if self.certs2.contains_key(&key) {
            self.pending_vote2.remove(&key);
            return;
        }
        if !self.vote2_persisted(view, pending.block_view) || !self.high_qc_persisted(pending.lock)
        {
            return;
        }
        let pending = self.pending_vote2.remove(&key).expect("checked above");
        if self.timed_out_for_vote2(view) {
            warn!(%view, "dropping pending vote2 for a view this node has timed out");
            return;
        }
        if !self.upgrade_lock.certificate_rule(view) && self.voted_for_branch_excluding(view) {
            warn!(%view, "dropping pending vote2 for a view a later vote1 skips");
            return;
        }
        outbox.push_back(ConsensusOutput::SendVote2(pending.vote));
    }

    /// Whether the view's Vote action and VID share are persisted, gating the
    /// phase-2 vote. A restart-barred view persisted both before the crash:
    /// its vote is re-cast, but its Vote action is never re-recorded. A
    /// re-vote disperses nothing, so it has no share of its own to wait for.
    fn vote2_persisted(&self, view: ViewNumber, block_view: ViewNumber) -> bool {
        view <= self.restart_barred_view
            || (self.stored_actions.contains(&(view, ActionKind::Vote))
                && (block_view != view || self.stored_vids.contains(&view)))
    }

    /// See [`Self::timeout_vote_bar`].
    fn timed_out_for_vote2(&self, view: ViewNumber) -> bool {
        self.timeout_vote_bar.is_some_and(|bar| view <= bar)
    }

    /// Hold `proposal` together with its leaf, and return its commitment.
    ///
    /// Every held proposal above the decided view has a leaf, which is what
    /// `undecided_leaves` reads. Leaves are pruned at the decided view,
    /// proposals only at the decide floor.
    fn hold_proposal(&mut self, proposal: Proposal<T>) -> Commitment<Leaf2<T>> {
        let commit = proposal_commitment(&proposal);
        self.hold_proposal_under(proposal, commit);
        commit
    }

    /// Hold `proposal` and its leaf under `commit`, which need not be the
    /// proposal's own commitment; only for the anchor, see `seed_parent`.
    fn hold_proposal_under(&mut self, proposal: Proposal<T>, commit: Commitment<Leaf2<T>>) {
        let view = proposal.view_number;
        let leaf = proposal.clone().into();
        self.proposals.insert_under(proposal, commit);
        self.leaves.insert((view, commit), leaf);
    }

    /// Whether a vote1 in a later view endorsed a branch with no block at `view`.
    ///
    /// This is what keeps one node out of two conflicting quorums. A vote2 certifies
    /// `view` for good, but the certificate and the reconstructed block it waits on
    /// can arrive after the node has submitted vote1 elsewhere. Casting it anyway
    /// would count the node towards a quorum committing `view` and towards one
    /// certifying a branch without it.
    ///
    /// This covers the order vote1-then-vote2. The reverse is the `is_safe`
    /// re-check in `maybe_vote_1`; neither alone is enough.
    fn voted_for_branch_excluding(&self, view: ViewNumber) -> bool {
        self.vote1_parent
            .range(view + 1..)
            .any(|(_, justified_at)| *justified_at < view)
    }

    fn release_proposal(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        if !self.stored_actions.contains(&(view, ActionKind::Propose)) {
            return;
        }
        for (_, message) in self
            .pending_revote
            .extract_if(ViewEpoch::at_view(view), |_, _| true)
        {
            outbox.push_back(ConsensusOutput::SendReVote(message));
        }
        let stored = self.stored_proposals.get(&view);
        for (_, message) in
            self.pending_proposal
                .extract_if(ViewEpoch::at_view(view), |_, message| {
                    stored.is_some_and(|commitments| {
                        commitments.contains(&proposal_commitment(&message.data))
                    })
                })
        {
            outbox.push_back(ConsensusOutput::SendProposal(message));
        }
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
        // A re-vote of an epoch's last block and the next epoch's first block
        // may share a view; each takes the vote1 of its own epoch.
        if self
            .revotes
            .get(&view)
            .is_some_and(|revote| !self.voted_1_at(view, revote.epoch))
        {
            self.maybe_vote_revote(view, outbox);
        }
        if self
            .proposals
            .live(view)
            .is_none_or(|proposal| self.voted_1_at(view, proposal.epoch))
        {
            return;
        }

        let Some(proposal) = self.proposals.live(view) else {
            debug!(%view, "proposal not available");
            return;
        };
        if !self
            .states_verified
            .contains(&(view, proposal_commitment(proposal)))
        {
            debug!(%view, "state commitment not available");
            return;
        }
        let Some(vid_share) = self.vid_shares.get(&view) else {
            debug!(%view, "vid share not available");
            return;
        };

        let block_number = proposal.block_header.block_number();
        let epoch = proposal.epoch;
        let qc_view = proposal.justify_qc.view_number();
        let qc_epoch = proposal.justify_qc.epoch();

        if self.upgrade_lock.certificate_rule(view) {
            if let Err(reason) = self.certificate_rule_vote1(proposal) {
                warn!(
                    %view, block = %block_number, %epoch, %qc_view, ?qc_epoch, %reason,
                    "certificate rule refuses vote1"
                );
                return;
            }
        // The counterpart of `voted_for_branch_excluding`, for the other order.
        // `handle_proposal_with_vid_share` checked this proposal against the lock
        // as it stood on arrival, and state validation can complete after that.
        // A vote2 in the meantime locks this node on a view the proposal's
        // branch may skip, and voting here would then count it towards a quorum
        // certifying that branch and towards the one committing the view.
        } else if let Err(err) = self.is_safe(proposal) {
            warn!(
                %view, block = %block_number, %epoch, %qc_view, ?qc_epoch, %err,
                "proposal no longer safe against the current lock; refusing to vote1"
            );
            return;
        }

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

        if parent_view != ViewNumber::genesis()
            && !is_last_block(
                proposal.block_header.block_number().saturating_sub(1),
                *self.epoch_height,
            )
        {
            let Some(prev_proposal) = self
                .proposals
                .get(parent_view, proposal.justify_qc.data().leaf_commit)
            else {
                debug!(%view, %parent_view, "proposal not available");
                return;
            };
            let parent_block = prev_proposal.block_header.block_number();
            let parent_epoch = prev_proposal.epoch;

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

        let proposal_commit = proposal_commitment(proposal);

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
        self.voted_1_views.insert(ViewEpoch(view, epoch));
        self.vote1_parent.insert(view, qc_view);
        if let Some(vid_share) = vid_share {
            outbox.push_back(ConsensusOutput::SendVote1(vote));
            outbox.push_back(ConsensusOutput::BroadcastVidShare(vid_share));
        } else {
            self.request_action(view, Some(epoch), ActionKind::Vote, outbox);
            self.pending_vote1.insert(ViewEpoch(view, epoch), vote);
        }
    }

    /// Vote2 and lock on each `Certificate1` at `view`, earliest epoch first,
    /// so that the lock ends on the latest.
    #[instrument(level = "debug", skip_all)]
    fn maybe_vote_2_and_update_lock(
        &mut self,
        view: ViewNumber,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        if view == ViewNumber::genesis() {
            return;
        }
        let epochs: Vec<EpochNumber> = self
            .certs1
            .range(ViewEpoch::at_view(view))
            .map(|(ViewEpoch(_, epoch), _)| *epoch)
            .collect();
        if epochs.is_empty() {
            debug!(%view, "cert1 not available");
        }
        for epoch in epochs {
            self.maybe_vote_2_and_update_lock_at(ViewEpoch(view, epoch), outbox);
        }
    }

    fn maybe_vote_2_and_update_lock_at(
        &mut self,
        key: ViewEpoch,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        let ViewEpoch(view, epoch) = key;
        let Some(cert1) = self.certs1.get(&key) else {
            return;
        };
        if self.voted_2_at(view, epoch)
            && self
                .lock_view()
                .is_some_and(|lock| lock >= LockView::of(cert1))
        {
            return;
        }
        // A re-vote certifies a block of an earlier view; its payload is the
        // one at the block's own view.
        let Some((block_view, proposal)) =
            self.certified_proposal(view, cert1.data.leaf_commit, cert1.data.block_number)
        else {
            if self.proposals.at(view).next().is_some() {
                warn!(%view, "cert1 commitment does not match proposal commitment");
            } else {
                debug!(%view, "proposal not available");
            }
            return;
        };
        let proposal_epoch = proposal.epoch;
        let block = proposal.block_header.block_number();
        let qc_view = proposal.justify_qc.view_number();
        let qc_epoch = proposal.justify_qc.epoch();
        let proposal_commit = proposal_commitment(proposal);
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
            .contains(&(block_view, proposal_block_commitment))
        {
            debug!(
                %view, %block, epoch = %proposal_epoch, %qc_view, ?qc_epoch,
                "no reconstructed block matching the proposal commitment"
            );
            return;
        }

        // We have a valid certificate, proposal, and reconstructed block
        // We can now update the lock, change view and vote. A node does not
        // lock on a late re-vote of an epoch it has left: its timeout votes
        // would carry a lock later than their view, which peers drop.
        if !self.behind_current_epoch(view, proposal_epoch)
            && self
                .lock_view()
                .is_none_or(|lock| lock < LockView::of(cert1))
        {
            let cert1 = cert1.clone();
            self.locked_cert = Some(cert1.clone());
            let curr_view = self.current_view;
            self.set_current_view_max(view + 1);
            self.set_current_epoch_max(proposal_epoch);
            // The view the block was proposed at, which payloads are kept by.
            outbox.push_back(ConsensusOutput::LockUpdated(block_view));
            if self.current_view != curr_view {
                outbox.push_back(ConsensusOutput::ViewChanged(
                    self.current_view,
                    self.current_epoch,
                ));
            }
            outbox.push_back(ConsensusOutput::SendCertificate1(cert1.clone()));
            if block_view == view && is_last_block(block, *self.epoch_height) {
                outbox.push_back(ConsensusOutput::PersistBoundaryQc(cert1.clone()));
            }
            // Persist the new lock; `release_vote2` gates the phase-2 vote on it.
            outbox.push_back(ConsensusOutput::PersistHighQc(cert1));
        }

        if self.voted_2_at(view, epoch)
            || self.certs2.contains_key(&key)
            || self.decided_views.contains(&block_view)
            || view <= self.decide_floor()
        {
            return;
        }

        if self.timed_out_for_vote2(view) {
            debug!(%view, %block, "timed out this view; refusing to vote2");
            return;
        }

        // A node votes for no epoch earlier than its own: a re-vote of the
        // epoch it left would otherwise still take its vote2.
        if self.upgrade_lock.certificate_rule(view) && proposal_epoch < self.current_epoch {
            debug!(%view, %block, epoch = %proposal_epoch, "vote2 of an earlier epoch");
            return;
        }

        if !self.upgrade_lock.certificate_rule(view) && self.voted_for_branch_excluding(view) {
            warn!(%view, %qc_view, %block, "a later vote1 skips this view; refusing to vote2");
            return;
        }

        if !self.staked_in_epoch(proposal_epoch) {
            return;
        }

        let vote = match SimpleVote::create_signed_vote(
            Vote2Data {
                leaf_commit: proposal_commit,
                epoch: proposal_epoch,
                block_number: block,
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
        self.voted_2_views.insert(key);
        // Lock is set above and >= view; the vote waits until it is persisted.
        let required = self
            .lock_view()
            .expect("locked_cert is set before voting in phase 2");
        if self.vote2_persisted(view, block_view) && self.high_qc_persisted(required) {
            outbox.push_back(ConsensusOutput::SendVote2(vote));
        } else {
            if view > self.restart_barred_view {
                self.request_action(view, Some(proposal_epoch), ActionKind::Vote, outbox);
            }
            self.pending_vote2.insert(
                key,
                PendingVote2 {
                    vote,
                    lock: required,
                    block_view,
                },
            );
        }
    }

    /// The certificate rule's checks on a vote1 (rule B), in place of the
    /// voter's own votes:
    /// - a node votes for no epoch earlier than its own;
    /// - after a timeout, the parent must be admitted by the timeout
    ///   certificate's lock ([`certificate_rule_admits`]);
    /// - the first block of an epoch names its parent's own `Certificate1`,
    ///   and the node must hold that parent. The commit of it the proposal
    ///   carries was verified with the proposal.
    fn certificate_rule_vote1(&self, proposal: &Proposal<T>) -> Result<(), &'static str> {
        let epoch = proposal.epoch;
        if epoch < self.current_epoch {
            return Err("proposal is of an epoch earlier than this node's");
        }
        if let Some(tc) = &proposal.view_change_evidence
            && !certificate_rule_admits(&proposal.justify_qc, tc, epoch)
        {
            return Err("parent is earlier than the timeout certificate's lock");
        }
        if proposal.next_epoch_justify_qc.is_some() {
            let parent_view = proposal.justify_qc.view_number();
            let holds_parent = self
                .proposals
                .contains(parent_view, proposal.justify_qc.data.leaf_commit);
            if !holds_parent {
                return Err("parent of the first block of the epoch is not held at its own view");
            }
        }
        Ok(())
    }

    /// Take in a validated re-vote request (see [`ReVote`]).
    ///
    /// Its timeout certificate, if any, moves this node into the re-vote's
    /// view, and the block's own certificate is kept and locked on. The vote
    /// is cast by [`Self::maybe_vote_revote`] once the node holds the block.
    fn handle_revote(
        &mut self,
        message: ReVoteMessage<T, Validated>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) -> Protocol {
        let revote = message.revote;
        let view = revote.view;
        if view <= self.decide_floor() {
            return Protocol::Abort;
        }
        // A node votes for no epoch earlier than its own, and an earlier
        // epoch's timeout certificate must not fill the view for its own.
        if revote.epoch < self.current_epoch {
            debug!(%view, epoch = %revote.epoch, "re-vote request of an earlier epoch");
            return Protocol::Abort;
        }
        let block_view = revote.cert1.view_number();
        self.insert_cert1(revote.cert1.clone());
        if let Some(timeout) = &revote.timeout
            && !self.timeout_certs.contains_key(&view)
        {
            let tc = ValidCert::new(TimeoutEvidence::V3(timeout.clone()), revote.epoch);
            self.handle_timeout_certificate(tc, outbox);
        }
        self.revotes.entry(view).or_insert(revote);
        self.maybe_vote_2_and_update_lock(block_view, outbox);
        Protocol::Continue
    }

    /// Vote1 on the re-vote at `view`, with the last block's own vote data.
    ///
    /// The rules are those of any vote1 under the certificate rule. The node
    /// must also hold the block and its payload, which it fetches once if
    /// missing.
    fn maybe_vote_revote(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        let Some(revote) = self.revotes.get(&view) else {
            return;
        };
        if !self.upgrade_lock.certificate_rule(view) {
            return;
        }
        let epoch = revote.epoch;
        let block_view = revote.cert1.view_number();
        let leaf_commit = revote.cert1.data.leaf_commit;
        if epoch < self.current_epoch {
            debug!(%view, %epoch, current_epoch = %self.current_epoch, "re-vote of an earlier epoch");
            return;
        }
        if self.cert2_over(block_view, leaf_commit).is_some() {
            debug!(%view, %block_view, "re-vote on a block already committed");
            return;
        }
        if let Some(timeout) = &revote.timeout
            && !certificate_rule_admits(&revote.cert1, &TimeoutEvidence::V3(timeout.clone()), epoch)
        {
            warn!(%view, %block_view, "re-vote block is earlier than the timeout certificate's lock");
            return;
        }
        let data = revote.vote_data();
        // A request naming a re-vote's certificate, not the block's own, finds
        // the block held at an earlier view.
        if self
            .certified_proposal(block_view, leaf_commit, data.block_number)
            .is_some_and(|(held_at, _)| held_at != block_view)
        {
            warn!(%view, %block_view, "re-vote request does not name the block's own certificate");
            return;
        }
        let Some(block) = self.proposals.get(block_view, leaf_commit) else {
            if self.revote_fetches.insert((view, RevoteFetch::Proposal)) {
                debug!(%view, %block_view, "re-vote proposal missing; requesting fetch");
                outbox.push_back(ConsensusOutput::RequestMissingProposal {
                    view: block_view,
                    leaf_commit,
                });
            }
            return;
        };
        let VidCommitment::V2(payload_commitment) = block.block_header.payload_commitment() else {
            return;
        };
        if !self.parent_reconstructed(block_view, payload_commitment, leaf_commit) {
            if self.revote_fetches.insert((view, RevoteFetch::Payload)) {
                debug!(%view, %block_view, "re-vote payload missing; requesting fetch");
                outbox.push_back(ConsensusOutput::RequestMissingPayload {
                    view: block_view,
                    payload_commitment,
                });
            }
            return;
        }
        if !self.staked_in_epoch(epoch) {
            return;
        }
        let vote = match SimpleVote::create_signed_vote(
            data,
            view,
            &self.public_key,
            &self.private_key,
            &self.upgrade_lock,
        ) {
            Ok(vote) => Vote1 {
                vote,
                state_vote: None,
            },
            Err(err) => {
                warn!(%view, %err, "failed to sign re-vote");
                return;
            },
        };
        info!(%view, %epoch, %block_view, "re-vote on the last block of the epoch");
        self.voted_1_views.insert(ViewEpoch(view, epoch));
        self.vote1_parent.insert(view, block_view);
        if self.stored_actions.contains(&(view, ActionKind::Vote)) {
            outbox.push_back(ConsensusOutput::SendVote1(vote));
        } else {
            self.request_action(view, Some(epoch), ActionKind::Vote, outbox);
            self.pending_vote1.insert(ViewEpoch(view, epoch), vote);
        }
    }

    /// Retry the vote2, lock and decide on the re-vote certificates over the
    /// block proposed at `block_view`, whose block, payload or own
    /// `Certificate1` just arrived.
    ///
    /// A re-vote's `Certificate2` commits the block at a later view, so the
    /// post-apply decide at `block_view` does not reach it.
    fn retry_later_certificates(
        &mut self,
        block_view: ViewNumber,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        let last_blocks: Vec<_> = self
            .proposals
            .at(block_view)
            .filter(|block| is_last_block(block.block_header.block_number(), *self.epoch_height))
            .map(proposal_commitment)
            .collect();
        if last_blocks.is_empty() {
            return;
        }
        let from = ViewEpoch::start(block_view + 1);
        let certs: BTreeSet<ViewNumber> = self
            .certs1
            .range(from..)
            .filter(|(_, cert)| last_blocks.contains(&cert.data.leaf_commit))
            .map(|(ViewEpoch(view, _), _)| *view)
            .chain(
                self.certs2
                    .range(from..)
                    .filter(|(_, cert)| last_blocks.contains(&cert.data.leaf_commit))
                    .map(|(ViewEpoch(view, _), _)| *view),
            )
            .collect();
        for view in certs {
            self.maybe_vote_2_and_update_lock(view, outbox);
            self.maybe_decide(view, outbox);
        }
    }

    /// Retry what a block or payload arriving can unblock for re-votes: the
    /// vote1 on a pending request, and the vote2, lock and decide on a
    /// re-vote's certificates.
    fn retry_revotes(&mut self, outbox: &mut Outbox<ConsensusOutput<T>>) {
        let pending: Vec<ViewNumber> = self
            .revotes
            .range(self.timeout_view + 1..)
            .filter(|(view, revote)| !self.voted_1_at(**view, revote.epoch))
            .map(|(view, _)| *view)
            .collect();
        for view in pending {
            self.maybe_vote_1(view, outbox);
        }
        let floor = self.decide_floor();
        let revoted: Vec<ViewNumber> = self
            .revotes
            .range(floor + 1..)
            .filter(|(view, revote)| {
                let key = ViewEpoch(**view, revote.epoch);
                (self.certs1.contains_key(&key) && !self.voted_2_at(key.0, key.1))
                    || self.certs2.get(&key).is_some_and(|cert2| {
                        self.revote_commits.get(view) != Some(&cert2.data.epoch)
                    })
            })
            .map(|(view, _)| *view)
            .collect();
        for view in revoted {
            self.maybe_vote_2_and_update_lock(view, outbox);
            self.maybe_decide(view, outbox);
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
        self.leader_of(view, epoch).as_ref() == Some(&self.public_key)
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
        let proposal = self.proposals.live(view);
        if proposal.is_some_and(|p| {
            !self
                .states_verified
                .contains(&(view, proposal_commitment(p)))
        }) {
            missing.push("state_validation");
        }
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
                let parent = self
                    .proposals
                    .get(parent_view, proposal.justify_qc.data().leaf_commit);
                let reconstructed = parent.is_some_and(|p| {
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
            match self.cert1_at(ViewNumber::from(view.saturating_sub(1))) {
                Some(c) => c,
                None => {
                    missing.push("parent_cert");
                    return missing;
                },
            }
        };
        let parent_view = parent_cert.view_number();
        let Some(parent_proposal) = self
            .proposals
            .get(parent_view, parent_cert.data.leaf_commit)
        else {
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

            if first_proposal_of_epoch && self.cert2_at(parent_view).is_none() {
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

    fn set_current_epoch_max(&mut self, e: EpochNumber) {
        self.current_epoch = self.current_epoch.max(e)
    }

    fn set_current_view_max(&mut self, v: ViewNumber) {
        self.current_view = self.current_view.max(v)
    }
}

/// Whether a proposal of `epoch` naming `parent` may follow `tc` under the
/// certificate rule.
///
/// The timeout certificate must be of the proposal's epoch, and the parent no
/// earlier than the certificate's lock, ordered by epoch, then view. A parent
/// over the same block as the lock passes at any view: a re-vote certifies a
/// block again at a later view, and the first block of the next epoch names
/// the block's own certificate. So does any parent if the lock is of an
/// earlier epoch than the proposal, which lets the first view of an epoch
/// recover from a timeout.
pub(crate) fn certificate_rule_admits<T: NodeType>(
    parent: &Certificate1<T>,
    tc: &TimeoutEvidence<T>,
    epoch: EpochNumber,
) -> bool {
    if tc.epoch() != Some(epoch) {
        return false;
    }
    let Some((lock, lock_cert)) = tc.lock() else {
        return true;
    };
    LockView::of(parent) >= lock
        || parent.data.leaf_commit == lock_cert.data.leaf_commit
        || lock.epoch < epoch
}

impl<T: NodeType> ConsensusInput<T> {
    /// The epoch this input carries, where it carries one.
    pub fn epoch(&self) -> Option<EpochNumber> {
        match self {
            ConsensusInput::BlockBuilt { epoch, .. } => Some(*epoch),
            ConsensusInput::Certificate1(cert) => Some(cert.epoch()),
            ConsensusInput::Certificate2(cert) => Some(cert.epoch()),
            ConsensusInput::AdvanceView(cert) => Some(cert.epoch()),
            ConsensusInput::EpochRootCertificates { cert1, .. } => Some(cert1.epoch()),
            ConsensusInput::TimeoutCertificate(cert) => Some(cert.epoch()),
            ConsensusInput::Proposal(_, proposal) => Some(proposal.proposal.data.epoch),
            ConsensusInput::FetchedProposal(proposal) => Some(proposal.proposal.data.epoch),
            ConsensusInput::Timeout(..) | ConsensusInput::TimeoutOneHonest(..) => None,
            ConsensusInput::DrbResult(epoch, _) => Some(*epoch),
            ConsensusInput::EpochChange(message) => message.cert1.epoch(),
            ConsensusInput::UpgradeCertificateFormed(cert) => Some(cert.epoch()),
            ConsensusInput::ReVote(message) => Some(message.revote.epoch),
            ConsensusInput::BlockReconstructed { .. }
            | ConsensusInput::HeaderCreated(..)
            | ConsensusInput::VidShare(..)
            | ConsensusInput::StateValidated(..)
            | ConsensusInput::StateValidationFailed(..)
            | ConsensusInput::Stored(..)
            | ConsensusInput::VidDisperseCreated(..) => None,
        }
    }

    pub fn view_number(&self) -> ViewNumber {
        match self {
            ConsensusInput::BlockBuilt { view, .. } => *view,
            ConsensusInput::BlockReconstructed { view, .. } => *view,
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
            ConsensusInput::Timeout(view) => *view,
            ConsensusInput::TimeoutOneHonest(view) => *view,
            ConsensusInput::TimeoutCertificate(cert) => {
                // Add one because we are moving to the next view so all event
                // processing is for the next view
                cert.view_number() + 1
            },
            ConsensusInput::VidDisperseCreated(view, _) => *view,
            // DRB results and formed upgrade certificates arrive
            // asynchronously and don't belong to any particular view; `apply`
            // handles routing by using `current_view` for these variants.
            ConsensusInput::DrbResult(..) => ViewNumber::genesis(),
            ConsensusInput::UpgradeCertificateFormed(..) => ViewNumber::genesis(),
            // The view of the commit, so that the post-apply steps decide it.
            ConsensusInput::EpochChange(epoch_change) => epoch_change.cert2.view_number(),
            ConsensusInput::ReVote(message) => message.revote.view,
        }
    }
}
