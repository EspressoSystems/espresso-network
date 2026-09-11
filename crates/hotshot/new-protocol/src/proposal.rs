use std::sync::Arc;

use committable::Committable;
use hotshot::types::SignatureKey;
use hotshot_contract_adapter::light_client::validate_light_client_state_update_certificate;
use hotshot_types::{
    data::{EpochNumber, Leaf2, VidDisperseShare2, ViewNumber, vid_disperse::vid_total_weight},
    epoch_membership::{EpochMembership, EpochMembershipCoordinator},
    message::{Proposal as SignedProposal, UpgradeLock},
    simple_certificate::{SimpleCertificate, SuccessThreshold, check_qc_state_cert_correspondence},
    simple_vote::{HasEpoch, QuorumMarker, Voteable},
    stake_table::StakeTableEntries,
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    utils::{epoch_from_block_number, is_epoch_root, is_last_block},
    vote::{Certificate, HasViewNumber},
};
use hotshot_utils::anytrace;
use tokio::task::JoinSet;
use tracing::error;

use crate::message::{
    Certificate2, Proposal, ProposalMessage, TimeoutCertificate, Unchecked, Validated,
    VidShareMessage,
};

type Result<T, E = ValidationError> = std::result::Result<T, E>;

/// A validated proposal.
pub struct ValidatedProposal<T: NodeType> {
    pub sender: T::SignatureKey,
    pub message: ProposalMessage<T, Validated>,
    /// True if this proposal was fetched from a peer
    pub fetched: bool,
}

/// A proposal validator checks proposal signature and integrity.
pub struct ProposalValidator<T: NodeType> {
    /// Validation tasks.
    tasks: JoinSet<Result<ValidatedProposal<T>>>,

    /// The actual validation logic.
    validator: Arc<Validator<T>>,
}

/// A validator dedicated to VID share messages.
///
/// Lives as a separate field on `Coordinator` so that its `next()` and
/// `ProposalValidator::next()` can be polled in the same `tokio::select!`
/// without conflicting `&mut self` borrows on a single field.
pub struct VidShareValidator<T: NodeType> {
    /// VID share validation tasks.
    tasks: JoinSet<Result<VidDisperseShare2<T>>>,

    /// Shared validation logic (same shape as `ProposalValidator`'s).
    validator: Arc<Validator<T>>,
}

struct Validator<T: NodeType> {
    membership_coordinator: EpochMembershipCoordinator<T>,
    epoch_height: u64,
    upgrade_lock: UpgradeLock<T>,
}

impl<T: NodeType> ProposalValidator<T> {
    pub fn new(
        c: EpochMembershipCoordinator<T>,
        epoch_height: u64,
        upgrade_lock: UpgradeLock<T>,
    ) -> Self {
        Self {
            tasks: JoinSet::new(),
            validator: Arc::new(Validator {
                membership_coordinator: c,
                epoch_height,
                upgrade_lock,
            }),
        }
    }

    pub fn validate(&mut self, p: ProposalMessage<T, Unchecked>) {
        self.spawn_validation(p, false)
    }

    /// Validate a proposal fetched from a peer.
    /// The coordinator routes it into the decide
    /// backfill path instead of the vote path.
    pub fn validate_fetched(&mut self, p: ProposalMessage<T, Unchecked>) {
        self.spawn_validation(p, true)
    }

    fn spawn_validation(&mut self, p: ProposalMessage<T, Unchecked>, fetched: bool) {
        let v = self.validator.clone();
        self.tasks.spawn(async move {
            let parts = well_formed(&p.proposal.data, v.epoch_height)?;
            let sender = v.signature(&p.proposal).await?;
            v.certificates(&p.proposal.data, &parts).await?;
            v.state_cert(&p.proposal.data).await?;
            let validated_proposal = ValidatedProposal {
                sender,
                message: ProposalMessage::validated(p.proposal),
                fetched,
            };
            Ok(validated_proposal)
        });
    }

    pub async fn next(&mut self) -> Option<Result<ValidatedProposal<T>>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(prop)) => return Some(prop),
                Some(Err(err)) => {
                    error!(%err, "proposal validation task panic");
                },
                None => return None,
            }
        }
    }
}

impl<T: NodeType> VidShareValidator<T> {
    pub fn new(
        c: EpochMembershipCoordinator<T>,
        epoch_height: u64,
        upgrade_lock: UpgradeLock<T>,
    ) -> Self {
        Self {
            tasks: JoinSet::new(),
            validator: Arc::new(Validator {
                membership_coordinator: c,
                epoch_height,
                upgrade_lock,
            }),
        }
    }

    pub fn validate(&mut self, share: VidShareMessage<T>) {
        let v = self.validator.clone();
        self.tasks.spawn(async move {
            v.vid_share_proposal(&share).await?;
            Ok(share.data)
        });
    }

    pub async fn next(&mut self) -> Option<Result<VidDisperseShare2<T>>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(share)) => return Some(share),
                Some(Err(err)) => {
                    error!(%err, "vid share validation task panic");
                },
                None => return None,
            }
        }
    }
}

/// Everything a proposal claims about its own block and its parent, checked
/// without signatures.
///
/// Returns the certificates whose signatures a validator verifies next, and the
/// epoch to resolve their committee through.
pub(crate) fn well_formed<T: NodeType>(
    proposal: &Proposal<T>,
    epoch_height: u64,
) -> Result<Parts<'_, T>, MalformedProposal> {
    epoch_matches_height(proposal, epoch_height)?;
    let justify_qc_epoch = justify_qc_matches_parent(proposal, epoch_height)?;
    Ok(Parts {
        justify_qc_epoch,
        next_epoch_justify_qc: next_epoch_justify_qc_matches_parent(
            proposal,
            epoch_height,
            justify_qc_epoch,
        )?,
        view_change_evidence: view_change_evidence_matches_parent(proposal)?,
    })
}

/// The certificates of a [well-formed](well_formed) proposal.
pub(crate) struct Parts<'a, T: NodeType> {
    /// The justify QC's epoch.
    pub(crate) justify_qc_epoch: EpochNumber,

    /// The boundary block's Cert2.
    pub(crate) next_epoch_justify_qc: Option<&'a Certificate2<T>>,

    /// The timeout certificate.
    pub(crate) view_change_evidence: Option<&'a TimeoutCertificate<T>>,
}

/// The proposal's epoch must be the one its block number falls in.
pub(crate) fn epoch_matches_height<T: NodeType>(
    proposal: &Proposal<T>,
    epoch_height: u64,
) -> Result<(), MalformedProposal> {
    let block_number = proposal.block_header.block_number();
    let expected = EpochNumber::new(epoch_from_block_number(block_number, epoch_height));
    if proposal.epoch != expected {
        return Err(MalformedProposal::Epoch {
            view: proposal.view_number(),
            block_number,
            expected,
            claimed: proposal.epoch,
        });
    }
    Ok(())
}

/// The justify QC must certify the block before this proposal, in the epoch that
/// block's height falls in.
///
/// A certificate's epoch selects the committee whose stake table and threshold
/// its signatures are weighed against. Unlike the proposal's own epoch, it is
/// covered by those signatures, so a mismatch is not something a proposer can
/// produce by relabelling a genuine certificate.
///
/// Returns that epoch, which the QC checks resolve their membership through.
pub(crate) fn justify_qc_matches_parent<T: NodeType>(
    proposal: &Proposal<T>,
    epoch_height: u64,
) -> Result<EpochNumber, MalformedProposal> {
    let view = proposal.view_number();
    let Some(claimed_epoch) = proposal.justify_qc.epoch() else {
        return Err(MalformedProposal::JustifyQcWithoutEpoch(view));
    };
    let parent_block = proposal.block_header.block_number().saturating_sub(1);
    let expected_epoch = EpochNumber::new(epoch_from_block_number(parent_block, epoch_height));
    if claimed_epoch != expected_epoch {
        return Err(MalformedProposal::JustifyQcEpoch {
            view,
            parent_block,
            expected: expected_epoch,
            claimed: claimed_epoch,
        });
    }
    let Some(claimed_block) = proposal.justify_qc.data.block_number else {
        return Err(MalformedProposal::JustifyQcWithoutBlockNumber(view));
    };
    if claimed_block != parent_block {
        return Err(MalformedProposal::JustifyQcBlockNumber {
            view,
            expected: parent_block,
            claimed: claimed_block,
        });
    }
    Ok(claimed_epoch)
}

/// The first proposal of an epoch must carry the boundary block's Cert2, which
/// certifies the same block as its justify QC.
///
/// Returns the certificate to verify the signatures on, or `None` when the
/// proposal does not follow a boundary block and carries none.
///
/// `justify_qc_epoch` is the justify QC's own, as returned by
/// [`justify_qc_matches_parent`]. The certificate names the same view, epoch
/// and block as that QC because both certify the same leaf, and the leaf
/// commitment covers neither, so agreement otherwise rests on an honest signer
/// being in the quorum that formed it.
pub(crate) fn next_epoch_justify_qc_matches_parent<T: NodeType>(
    proposal: &Proposal<T>,
    epoch_height: u64,
    justify_qc_epoch: EpochNumber,
) -> Result<Option<&Certificate2<T>>, MalformedProposal> {
    let parent_block = proposal.block_header.block_number().saturating_sub(1);
    if !is_last_block(parent_block, epoch_height) {
        return Ok(None);
    }
    let view = proposal.view_number();
    let Some(cert2) = proposal.next_epoch_justify_qc.as_ref() else {
        return Err(MalformedProposal::NextEpochJustifyQcMissing(view));
    };
    if cert2.data.leaf_commit != proposal.justify_qc.data.leaf_commit {
        return Err(MalformedProposal::NextEpochJustifyQcLeafCommit(view));
    }
    let parent_view = proposal.justify_qc.view_number();
    if cert2.view_number() != parent_view
        || cert2.data.epoch != justify_qc_epoch
        || cert2.data.block_number != parent_block
    {
        return Err(MalformedProposal::NextEpochJustifyQcParent {
            view,
            claimed_view: cert2.view_number(),
            claimed_epoch: cert2.data.epoch,
            claimed_block: cert2.data.block_number,
            parent_view,
            parent_epoch: justify_qc_epoch,
            parent_block,
        });
    }
    Ok(Some(cert2))
}

/// A proposal extends an earlier view, and skips views only with a timeout
/// certificate for the view immediately before it.
///
/// Returns the certificate to verify the signatures on, or `None` when the
/// proposal follows its parent directly and needs none.
///
/// The parent view being earlier is what the walks over stored proposals
/// descend on, and, like the rest of what a proposal says about its parent, it
/// is covered by no signature of the proposer's own.
pub(crate) fn view_change_evidence_matches_parent<T: NodeType>(
    proposal: &Proposal<T>,
) -> Result<Option<&TimeoutCertificate<T>>, MalformedProposal> {
    let view = proposal.view_number();
    let parent_view = proposal.justify_qc.view_number();
    if parent_view >= view {
        return Err(MalformedProposal::ParentNotEarlier { view, parent_view });
    }
    if parent_view + 1 == view {
        return Ok(None);
    }
    let Some(tc) = proposal.view_change_evidence.as_ref() else {
        return Err(MalformedProposal::ViewChangeEvidenceMissing(view));
    };
    // The timeout certificate must certify the immediately preceding view.
    if tc.data.view + 1 != view {
        return Err(MalformedProposal::ViewChangeEvidenceView {
            view,
            evidence_view: tc.data.view,
        });
    }
    Ok(Some(tc))
}

impl<T: NodeType> Validator<T> {
    /// Verify the proposal signature and return the leader
    async fn signature(
        &self,
        proposal: &SignedProposal<T, Proposal<T>>,
    ) -> Result<T::SignatureKey> {
        let view = proposal.data.view_number();
        let epoch = proposal.data.epoch;
        let membership = self.membership(epoch).await?;
        let leader = match membership.leader(view) {
            Ok(leader) => leader,
            Err(err) => return Err(ValidationError::NoLeader(view, epoch, err)),
        };
        let leaf: Leaf2<T> = proposal.data.clone().into();
        if leader.validate(&proposal.signature, leaf.commit().as_ref()) {
            Ok(leader)
        } else {
            Err(ValidationError::InvalidProposalSignature)
        }
    }

    async fn vid_share_proposal(
        &self,
        vid_proposal: &SignedProposal<T, VidDisperseShare2<T>>,
    ) -> Result<()> {
        let view = vid_proposal.data.view_number();
        let epoch = vid_proposal
            .data
            .epoch
            .ok_or(ValidationError::MissingEpoch(view, "vid share"))?;
        let membership = self.membership(epoch).await?;
        let stake_table = membership.stake_table();
        let leader = match membership.leader(view) {
            Ok(leader) => leader,
            Err(err) => return Err(ValidationError::NoLeader(view, epoch, err)),
        };
        // TODO(Chengyu): this also check the consistency of vid common and vid commitment.
        let total_weight = vid_total_weight(stake_table, Some(epoch));
        if !leader.validate(
            &vid_proposal.signature,
            vid_proposal.data.payload_commitment.as_ref(),
        ) {
            return Err(ValidationError::InvalidVidShareProposalSignature);
        }
        if vid_proposal.data.verify(total_weight) {
            Ok(())
        } else {
            Err(ValidationError::VidShareNotVerified)
        }
    }

    /// Verify the signatures on the certificates the proposal carries.
    ///
    /// `parts` is what [`well_formed`] found the proposal must carry, and the
    /// epoch whose committee formed its justify QC and the boundary Cert2
    /// beside it. The view-change evidence names its own epoch, covered by the
    /// signatures over it.
    async fn certificates(&self, proposal: &Proposal<T>, parts: &Parts<'_, T>) -> Result<()> {
        let epoch = parts.justify_qc_epoch;
        self.verify_cert(
            &proposal.justify_qc,
            epoch,
            ValidationError::InvalidJustifyQc,
        )
        .await?;
        if let Some(cert2) = parts.next_epoch_justify_qc {
            self.verify_cert(cert2, epoch, ValidationError::InvalidNextEpochJustifyQc)
                .await?;
        }
        if let Some(tc) = parts.view_change_evidence {
            let view = proposal.view_number();
            let Some(tc_epoch) = tc.epoch() else {
                return Err(ValidationError::MissingEpoch(view, "view_change_evidence"));
            };
            self.verify_cert(tc, tc_epoch, ValidationError::InvalidViewChangeEvidence)
                .await?;
        }
        Ok(())
    }

    /// Verify a certificate's signatures against the stake table and threshold
    /// of `epoch`'s committee, labelling an invalid one with `invalid`.
    async fn verify_cert<D>(
        &self,
        cert: &SimpleCertificate<T, D, SuccessThreshold>,
        epoch: EpochNumber,
        invalid: fn(anytrace::Error) -> ValidationError,
    ) -> Result<()>
    where
        D: Voteable<T> + QuorumMarker + 'static,
    {
        let membership = self.membership(epoch).await?;
        let entries = StakeTableEntries::from_iter(membership.stake_table()).0;
        cert.is_valid_cert(&entries, membership.success_threshold(), &self.upgrade_lock)
            .map_err(invalid)
    }

    /// Validate the state_cert on an epoch-root proposal.
    ///
    /// If the justify_qc points at an epoch-root block, the proposal MUST
    /// carry a matching `LightClientStateUpdateCertificateV2`.
    async fn state_cert(&self, proposal: &Proposal<T>) -> Result<()> {
        let Some(qc_block_number) = proposal.justify_qc.data.block_number else {
            return Ok(());
        };
        if !is_epoch_root(qc_block_number, self.epoch_height) {
            // Non-epoch-root parent → no state_cert required.
            return Ok(());
        }
        let Some(state_cert) = proposal.state_cert.as_ref() else {
            return Err(ValidationError::MissingStateCert);
        };
        if !check_qc_state_cert_correspondence(&proposal.justify_qc, state_cert, self.epoch_height)
        {
            return Err(ValidationError::StateCertCorrespondence);
        }
        validate_light_client_state_update_certificate(
            state_cert,
            &self.membership_coordinator,
            &self.upgrade_lock,
        )
        .await
        .map_err(ValidationError::InvalidStateCert)
    }

    async fn membership(&self, epoch: EpochNumber) -> Result<EpochMembership<T>> {
        match self
            .membership_coordinator
            .membership_for_epoch(Some(epoch))
        {
            Ok(m) => Ok(m),
            Err(_) => self
                .membership_coordinator
                .wait_for_catchup(epoch) // TODO: timeout?
                .await
                .map_err(|e| ValidationError::NoMembershipForEpoch(epoch, e)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error(transparent)]
    Malformed(#[from] MalformedProposal),

    #[error("invalid proposal signature")]
    InvalidProposalSignature,

    #[error("invalid proposal justify qc: {0}")]
    InvalidJustifyQc(#[source] anytrace::Error),

    #[error("invalid next_epoch_justify_qc: {0}")]
    InvalidNextEpochJustifyQc(#[source] anytrace::Error),

    #[error("vid share does not match proposal")]
    VidCommitmentDoesNotMatchProposal,

    #[error("failed to verify vid share")]
    VidShareNotVerified,

    #[error("vid commitment not v2")]
    InvalidVidCommitmentVersion,

    #[error("missing epoch number in view {0} ({1})")]
    MissingEpoch(ViewNumber, &'static str),

    #[error("failed to get membership for epoch {0}: {1}")]
    NoMembershipForEpoch(EpochNumber, #[source] anytrace::Error),

    #[error("failed to get leader for view {0}, epoch {1}: {2}")]
    NoLeader(ViewNumber, EpochNumber, #[source] anytrace::Error),

    #[error("proposal justify_qc is epoch-root but state_cert is missing")]
    MissingStateCert,

    #[error("state_cert does not correspond to justify_qc")]
    StateCertCorrespondence,

    #[error("state_cert signature validation failed: {0}")]
    InvalidStateCert(#[source] anytrace::Error),

    #[error("invalid vid share proposal signature")]
    InvalidVidShareProposalSignature,

    #[error("view-change evidence (timeout certificate) is invalid: {0}")]
    InvalidViewChangeEvidence(#[source] anytrace::Error),
}

/// Reason a proposal is not [well-formed](well_formed).
#[derive(Copy, Clone, Debug, thiserror::Error)]
pub enum MalformedProposal {
    #[error(
        "proposal at view {view} claims epoch {claimed}, but block number {block_number} falls in \
         epoch {expected}"
    )]
    Epoch {
        view: ViewNumber,
        block_number: u64,
        expected: EpochNumber,
        claimed: EpochNumber,
    },

    #[error("justify_qc of proposal at view {0} names no epoch")]
    JustifyQcWithoutEpoch(ViewNumber),

    #[error("justify_qc of proposal at view {0} names no block number")]
    JustifyQcWithoutBlockNumber(ViewNumber),

    #[error(
        "justify_qc of proposal at view {view} claims epoch {claimed}, but its parent block \
         {parent_block} falls in epoch {expected}"
    )]
    JustifyQcEpoch {
        view: ViewNumber,
        parent_block: u64,
        expected: EpochNumber,
        claimed: EpochNumber,
    },

    #[error(
        "justify_qc of proposal at view {view} certifies block {claimed}, not its parent block \
         {expected}"
    )]
    JustifyQcBlockNumber {
        view: ViewNumber,
        expected: u64,
        claimed: u64,
    },

    #[error("first proposal of an epoch at view {0} is missing next_epoch_justify_qc")]
    NextEpochJustifyQcMissing(ViewNumber),

    #[error(
        "next_epoch_justify_qc of proposal at view {0} certifies another leaf than its justify_qc"
    )]
    NextEpochJustifyQcLeafCommit(ViewNumber),

    #[error(
        "next_epoch_justify_qc of proposal at view {view} certifies view {claimed_view} of epoch \
         {claimed_epoch} at block {claimed_block}, not view {parent_view} of epoch {parent_epoch} \
         at block {parent_block}"
    )]
    NextEpochJustifyQcParent {
        view: ViewNumber,
        claimed_view: ViewNumber,
        claimed_epoch: EpochNumber,
        claimed_block: u64,
        parent_view: ViewNumber,
        parent_epoch: EpochNumber,
        parent_block: u64,
    },

    #[error(
        "proposal at view {view} has a justify_qc for view {parent_view}, which is not earlier"
    )]
    ParentNotEarlier {
        view: ViewNumber,
        parent_view: ViewNumber,
    },

    #[error("proposal at view {0} skips views but carries no view-change evidence")]
    ViewChangeEvidenceMissing(ViewNumber),

    #[error(
        "view-change evidence for proposal at view {view} certifies view {evidence_view}, not the \
         immediately preceding view"
    )]
    ViewChangeEvidenceView {
        view: ViewNumber,
        evidence_view: ViewNumber,
    },
}
