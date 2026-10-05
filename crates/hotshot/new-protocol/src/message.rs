pub mod fetch;
pub mod payload;

use std::marker::PhantomData;

use committable::{Commitment, Committable};
use hotshot_types::{
    data::{
        EpochNumber, UpgradeProposal2, VidDisperseShare2, ViewNumber,
        vid_disperse::AvidmGf2DisperseShareFragment,
    },
    message::{Proposal as SignedProposal, UpgradeLock},
    request_response::ProposalRequestPayload,
    simple_certificate::{TimeoutCertificate2, TimeoutCertificate3, TimeoutEvidence},
    simple_vote::{
        HasEpoch, LightClientStateUpdateVote2, LockView, QuorumData2, QuorumVote2, SimpleVote,
        TimeoutData3, TimeoutVote2, TimeoutVote3, UpgradeVote2, Vote2Data,
    },
    traits::{
        block_contents::BlockHeader, node_implementation::NodeType, signature_key::SignatureKey,
    },
    utils::{epoch_from_block_number, is_last_block},
    vote::{HasViewNumber, Vote},
};
pub use hotshot_types::{
    new_protocol::Proposal,
    simple_certificate::{Certificate1, Certificate2},
};
use serde::{Deserialize, Serialize};

use crate::{
    helpers::proposal_commitment,
    message::payload::PayloadFetchMessage,
    proposal::{
        MalformedProposal, epoch_matches_height, justify_qc_matches_parent,
        state_cert_matches_parent, view_change_evidence_matches_parent,
    },
};

pub type Vote2<T> = SimpleVote<T, Vote2Data<T>>;

#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub enum TimeoutVote<T: NodeType> {
    V2(TimeoutVote2<T>),
    V3(TimeoutBallot<T>),
}

/// A timeout vote that signs its signer's lock, with the certificate of that
/// lock.
///
/// The lock the vote signs is the certificate's, so the two always agree;
/// `None` is a signer locked on nothing but genesis. A ballot can only be
/// signed by its own signer ([`Self::sign`]) or read from a message
/// ([`TimeoutVoteMessage3::ballot`]), and neither checks the signatures.
#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub struct TimeoutBallot<T: NodeType> {
    vote: TimeoutVote3<T>,
    lock: Option<Certificate1<T>>,
}

impl<T: NodeType> TimeoutBallot<T> {
    /// Sign a timeout vote for `view` in `epoch` naming the lock `lock`
    /// certifies.
    pub fn sign(
        view: ViewNumber,
        epoch: EpochNumber,
        lock: Option<Certificate1<T>>,
        public_key: &T::SignatureKey,
        private_key: &<T::SignatureKey as SignatureKey>::PrivateKey,
        upgrade_lock: &UpgradeLock<T>,
    ) -> anyhow::Result<Self> {
        let data = TimeoutData3 {
            view,
            epoch,
            lock: lock.as_ref().map(LockView::of),
        };
        let vote =
            SimpleVote::create_signed_vote(data, view, public_key, private_key, upgrade_lock)
                .map_err(|err| anyhow::anyhow!("failed to sign timeout vote: {err}"))?;
        Ok(Self { vote, lock })
    }

    /// The signed vote.
    pub fn vote(&self) -> &TimeoutVote3<T> {
        &self.vote
    }

    /// The certificate of the lock the vote signs.
    pub fn lock(&self) -> Option<&Certificate1<T>> {
        self.lock.as_ref()
    }
}

impl<T: NodeType> TimeoutVote<T> {
    pub fn binds_epoch(&self) -> bool {
        matches!(self, Self::V3(_))
    }

    pub fn signing_key(&self) -> T::SignatureKey {
        match self {
            Self::V2(vote) => vote.signing_key(),
            Self::V3(ballot) => ballot.vote.signing_key(),
        }
    }

    pub fn is_well_formed(&self) -> bool {
        match self {
            Self::V2(v) => v.view_number() == v.data.view,
            // Its data is built from its view.
            Self::V3(_) => true,
        }
    }
}

impl<T: NodeType> HasViewNumber for TimeoutVote<T> {
    fn view_number(&self) -> ViewNumber {
        match self {
            Self::V2(vote) => vote.view_number(),
            Self::V3(ballot) => ballot.vote.view_number(),
        }
    }
}

impl<T: NodeType> HasEpoch for TimeoutVote<T> {
    fn epoch(&self) -> Option<EpochNumber> {
        match self {
            Self::V2(vote) => vote.data.epoch,
            Self::V3(ballot) => Some(ballot.vote.data.epoch),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Deserialize)]
pub enum Unchecked {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize)]
pub enum Validated {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = "S: Deserialize<'de>"))]
pub struct ProposalMessage<T: NodeType, S> {
    pub proposal: SignedProposal<T, Proposal<T>>,
    #[serde(skip)]
    _marker: PhantomData<fn() -> S>,
}

impl<T: NodeType> ProposalMessage<T, Validated> {
    pub fn validated(p: SignedProposal<T, Proposal<T>>) -> Self {
        Self {
            proposal: p,
            _marker: PhantomData,
        }
    }
}

impl<T: NodeType> ProposalMessage<T, Unchecked> {
    /// Wrap a proposal that has not been validated yet
    pub fn unchecked(p: SignedProposal<T, Proposal<T>>) -> Self {
        Self {
            proposal: p,
            _marker: PhantomData,
        }
    }
}

impl<T: NodeType, S> ProposalMessage<T, S> {
    #[cfg(any(test, feature = "testing"))]
    pub fn into_unchecked(self) -> ProposalMessage<T, Unchecked> {
        ProposalMessage {
            proposal: self.proposal,
            _marker: PhantomData,
        }
    }
}

impl<T: NodeType, S> HasViewNumber for ProposalMessage<T, S> {
    fn view_number(&self) -> ViewNumber {
        self.proposal.data.view_number
    }
}

/// A reassembled, signed VID share.
pub type VidShareMessage<T> = SignedProposal<T, VidDisperseShare2<T>>;

/// A signed per-namespace VID share fragment.
///
/// Unicast by the leader to a replica. A replica collects all of a view's
/// fragments and reassembles them into a [`VidShareMessage`].
pub type VidShareFragmentMessage<T> = SignedProposal<T, AvidmGf2DisperseShareFragment<T>>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct Vote1<T: NodeType> {
    pub vote: QuorumVote2<T>,
    /// Populated only when voting on an epoch-root leaf. Required there; absent otherwise.
    pub state_vote: Option<LightClientStateUpdateVote2<T>>,
}

impl<T: NodeType> HasViewNumber for Vote1<T> {
    fn view_number(&self) -> ViewNumber {
        self.vote.view_number()
    }
}

/// The leader's broadcast of an upgrade proposal for the network to vote on.
pub type UpgradeProposalMessage<T> = SignedProposal<T, UpgradeProposal2>;

/// An upgrade vote, broadcast all-to-all. Its signed data binds the voter's
/// epoch, which selects the stake table under which the vote is tallied.
pub type UpgradeVoteMessage<T> = UpgradeVote2<T>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct TimeoutVoteMessage<T: NodeType> {
    pub vote: TimeoutVote2<T>,
    pub evidence: Option<CatchupEvidence<T>>,
}

impl<T: NodeType> HasViewNumber for TimeoutVoteMessage<T> {
    fn view_number(&self) -> ViewNumber {
        self.vote.view_number()
    }
}

/// A timeout vote that signs its signer's lock, as it travels.
///
/// The signed data is not sent: it is the view, the epoch and the lock of
/// `lock`, so the lock a vote signs and the certificate it comes with cannot
/// disagree, and neither can its view and the view its data names.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct TimeoutVoteMessage3<T: NodeType> {
    pub signer: T::SignatureKey,
    pub signature: <T::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    /// The certificate of the signer's lock; `None` if it is locked on nothing
    /// but genesis. A lock no quorum certified cannot be counted.
    pub lock: Option<Certificate1<T>>,
    pub evidence: Option<CatchupEvidence<T>>,
}

impl<T: NodeType> TimeoutVoteMessage3<T> {
    pub fn new(ballot: TimeoutBallot<T>, evidence: Option<CatchupEvidence<T>>) -> Self {
        let TimeoutBallot { vote, lock } = ballot;
        Self {
            signer: vote.signature.0,
            signature: vote.signature.1,
            view: vote.view_number,
            epoch: vote.data.epoch,
            lock,
            evidence,
        }
    }

    /// The vote, if its lock is a signed certificate no later than the view
    /// it times out.
    pub fn ballot(&self) -> Result<TimeoutBallot<T>, &'static str> {
        if let Some(cert) = &self.lock {
            if cert.view_number() == ViewNumber::genesis() {
                return Err("the genesis certificate is no lock");
            }
            if cert.view_number() > self.view {
                return Err("the lock is later than the view timed out");
            }
        }
        let data = TimeoutData3 {
            view: self.view,
            epoch: self.epoch,
            lock: self.lock.as_ref().map(LockView::of),
        };
        Ok(TimeoutBallot {
            vote: SimpleVote {
                signature: (self.signer.clone(), self.signature.clone()),
                data,
                view_number: self.view,
            },
            lock: self.lock.clone(),
        })
    }
}

impl<T: NodeType> HasViewNumber for TimeoutVoteMessage3<T> {
    fn view_number(&self) -> ViewNumber {
        self.view
    }
}

/// The highest certificate a node holds: its locked QC or its latest timeout
/// certificate, whichever has the higher view. Attached to timeout votes and
/// sent to peers stuck on stale views, so divergent nodes re-converge on the
/// highest justified view.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub enum CatchupEvidence<T: NodeType> {
    Qc(Certificate1<T>),
    Tc(TimeoutCertificate2<T>),
    Tc3(TimeoutCertificate3<T>),
}

impl<T: NodeType> From<&TimeoutEvidence<T>> for CatchupEvidence<T> {
    fn from(evidence: &TimeoutEvidence<T>) -> Self {
        match evidence {
            TimeoutEvidence::V2(cert) => Self::Tc(cert.clone()),
            TimeoutEvidence::V3(cert) => Self::Tc3(cert.clone()),
        }
    }
}

impl<T: NodeType> HasViewNumber for CatchupEvidence<T> {
    fn view_number(&self) -> ViewNumber {
        match self {
            Self::Qc(qc) => qc.view_number(),
            Self::Tc(tc) => tc.view_number(),
            Self::Tc3(tc) => tc.view_number(),
        }
    }
}

/// Message sent at the end of an epoch by the current committee
/// to the next committee.  Both certificates are on the last block of the epoch.
/// The protocol spec only requires the second certificate, but for consistency
/// in the code and with the existing Proposal and Leaf structures
/// We include the Certificate1.  This allows us to use the Certificate1 as the
/// Justify QC on the first proposal.  The Certificate2 also required on that proposal
/// but as next_epoch_justify_qc on the Leaf.
///
/// We include the proposal because the new leader in the next epoch
/// will need it to build a header for the first block of the next epoch.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = "S: Deserialize<'de>"))]
pub struct EpochChangeMessage<T: NodeType, S> {
    pub cert1: Certificate1<T>,
    pub cert2: Certificate2<T>,
    pub proposal: Proposal<T>,
    #[serde(skip)]
    _marker: PhantomData<fn() -> S>,
}

impl<T: NodeType> EpochChangeMessage<T, Validated> {
    /// Wrap certificates this node has verified (or formed itself).
    pub fn validated(
        cert1: Certificate1<T>,
        cert2: Certificate2<T>,
        proposal: Proposal<T>,
    ) -> Self {
        Self {
            cert1,
            cert2,
            proposal,
            _marker: PhantomData,
        }
    }
}

impl<T: NodeType> EpochChangeMessage<T, Unchecked> {
    /// Mark this message's certificates as verified.
    pub(crate) fn into_validated(self) -> EpochChangeMessage<T, Validated> {
        EpochChangeMessage {
            cert1: self.cert1,
            cert2: self.cert2,
            proposal: self.proposal,
            _marker: PhantomData,
        }
    }
}

impl<T: NodeType, S> EpochChangeMessage<T, S> {
    /// Structural validity of the message, independent of signatures.
    ///
    /// Every part must name the same block, and `cert1` and the proposal the
    /// same view. `cert2` may be at a later view: a re-vote commits the last
    /// block of an epoch again at a view of its own, and the first block of
    /// the next epoch still names the block's own `cert1`. A certificate's
    /// own view and block number are covered by the signatures over it, and
    /// the proposal's are covered by the leaf commitment, so agreement between
    /// them otherwise rests on an honest signer being in the quorum. Comparing
    /// them makes the message self-checking. `Proposal::epoch` is not covered
    /// by the commitment at all and has no other check.
    ///
    /// What the embedded proposal claims about itself and its parent is checked
    /// by the same functions the proposal path uses, except for the boundary
    /// Cert2 only the first proposal of an epoch carries.
    pub fn well_formed(&self, epoch_height: u64) -> Result<(), EpochChangeError> {
        let block_number = self.cert2.data.block_number;
        if self.cert1.view_number() > self.cert2.view_number()
            || self.cert1.epoch() != self.cert2.epoch()
            || self.cert1.data.leaf_commit != self.cert2.data.leaf_commit
            || self.cert1.data.block_number != Some(block_number)
        {
            return Err(EpochChangeError::CertificateMismatch);
        }
        if !is_last_block(block_number, epoch_height) {
            return Err(EpochChangeError::NotLastBlock);
        }
        if self.cert2.data.epoch != epoch_from_block_number(block_number, epoch_height).into() {
            return Err(EpochChangeError::WrongEpoch);
        }
        if proposal_commitment(&self.proposal) != self.cert1.data.leaf_commit {
            return Err(EpochChangeError::ProposalMismatch);
        }
        if self.proposal.view_number() != self.cert1.view_number()
            || self.proposal.block_header.block_number() != block_number
        {
            return Err(EpochChangeError::ProposalCertificateMismatch);
        }
        epoch_matches_height(&self.proposal, epoch_height)?;
        justify_qc_matches_parent(&self.proposal, epoch_height)?;
        state_cert_matches_parent(&self.proposal, epoch_height)?;
        view_change_evidence_matches_parent(&self.proposal)?;
        Ok(())
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn into_unchecked(self) -> EpochChangeMessage<T, Unchecked> {
        EpochChangeMessage {
            cert1: self.cert1,
            cert2: self.cert2,
            proposal: self.proposal,
            _marker: PhantomData,
        }
    }
}

/// Reason an [`EpochChangeMessage`] is not [well-formed](EpochChangeMessage::well_formed).
#[derive(Copy, Clone, Debug, thiserror::Error)]
pub enum EpochChangeError {
    #[error(
        "certificates differ in epoch, block number or leaf commitment, or certificate2 is \
         earlier than certificate1"
    )]
    CertificateMismatch,
    #[error("certificate2 is not for the last block of an epoch")]
    NotLastBlock,
    #[error("certificate2's block number does not match its epoch")]
    WrongEpoch,
    #[error("proposal commitment does not match certificate1's leaf commitment")]
    ProposalMismatch,
    #[error("the embedded proposal names a different view or block than the certificates")]
    ProposalCertificateMismatch,
    #[error("the embedded proposal is malformed: {0}")]
    Proposal(#[from] MalformedProposal),
}

impl<T: NodeType, S> HasViewNumber for EpochChangeMessage<T, S> {
    /// The view of the commit: the next epoch starts after it.
    fn view_number(&self) -> ViewNumber {
        self.cert2.view_number()
    }
}

impl<T: NodeType, S> HasEpoch for EpochChangeMessage<T, S> {
    fn epoch(&self) -> Option<EpochNumber> {
        self.cert1.epoch()
    }
}

/// A request by the leader of `view` that the committee of `epoch` vote on
/// the epoch's last block again.
///
/// The last block of an epoch must be committed before the next epoch can
/// start, and its first block needs the commit certificate. If the block has
/// a `Certificate1` but its `Certificate2` can no longer form, because the
/// nodes that did not vote2 for it have timed its view out, the committee
/// votes on the block itself again: a vote1 and then a vote2 over the block's
/// existing vote data, at `view`. No block is built or dispersed.
///
/// The leader asks as soon as it can lock on the block, without waiting for
/// the view to time out: in the view after the block's, or after a timeout
/// certificate for the view before.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct ReVote<T: NodeType> {
    /// The view of the re-vote.
    pub view: ViewNumber,
    /// The epoch whose last block is voted on.
    pub epoch: EpochNumber,
    /// The block's own `Certificate1`, at the block's view.
    pub cert1: Certificate1<T>,
    /// The timeout certificate for the view before `view`, unless that is the
    /// block's view.
    pub timeout: Option<TimeoutCertificate3<T>>,
}

impl<T: NodeType> ReVote<T> {
    /// The data every vote of the re-vote signs: the block's own.
    pub fn vote_data(&self) -> QuorumData2<T> {
        self.cert1.data
    }

    /// Structural validity, independent of signatures.
    pub fn well_formed(&self, epoch_height: u64) -> Result<(), ReVoteError> {
        let data = &self.cert1.data;
        if data.epoch != Some(self.epoch) {
            return Err(ReVoteError::Epoch);
        }
        let Some(block) = data.block_number else {
            return Err(ReVoteError::NotLastBlock);
        };
        if !is_last_block(block, epoch_height)
            || EpochNumber::new(epoch_from_block_number(block, epoch_height)) != self.epoch
        {
            return Err(ReVoteError::NotLastBlock);
        }
        let parent = self.cert1.view_number();
        if parent == ViewNumber::genesis() || parent >= self.view {
            return Err(ReVoteError::ParentNotEarlier);
        }
        match &self.timeout {
            None if parent + 1 != self.view => return Err(ReVoteError::Timeout),
            Some(tc) if tc.view_number + 1 != self.view || tc.epoch != self.epoch => {
                return Err(ReVoteError::Timeout);
            },
            _ => {},
        }
        Ok(())
    }
}

impl<T: NodeType> Committable for ReVote<T> {
    fn commit(&self) -> Commitment<Self> {
        committable::RawCommitmentBuilder::new("Re-vote request")
            .u64_field("view number", *self.view)
            .u64_field("epoch number", *self.epoch)
            .field("certificate1", self.cert1.commit())
            .optional("timeout certificate", &self.timeout)
            .finalize()
    }
}

/// Reason a [`ReVote`] is not [well-formed](ReVote::well_formed).
#[derive(Copy, Clone, Debug, thiserror::Error)]
pub enum ReVoteError {
    #[error("re-vote certificate is not of the re-vote's epoch")]
    Epoch,
    #[error("re-vote certificate is not over the last block of its epoch")]
    NotLastBlock,
    #[error("re-vote certificate is not earlier than the re-vote")]
    ParentNotEarlier,
    #[error(
        "re-vote is neither in the view after its certificate's nor behind a timeout certificate \
         for the view before it, in its epoch"
    )]
    Timeout,
}

/// A [`ReVote`] signed by its view's leader.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = "S: Deserialize<'de>"))]
pub struct ReVoteMessage<T: NodeType, S> {
    pub revote: ReVote<T>,
    pub signature: <T::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    #[serde(skip)]
    _marker: PhantomData<fn() -> S>,
}

impl<T: NodeType> ReVoteMessage<T, Validated> {
    /// Sign a re-vote request.
    pub fn new(
        revote: ReVote<T>,
        private_key: &<T::SignatureKey as SignatureKey>::PrivateKey,
    ) -> Result<Self, <T::SignatureKey as SignatureKey>::SignError> {
        let signature = T::SignatureKey::sign(private_key, revote.commit().as_ref())?;
        Ok(Self {
            revote,
            signature,
            _marker: PhantomData,
        })
    }
}

impl<T: NodeType> ReVoteMessage<T, Unchecked> {
    pub(crate) fn into_validated(self) -> ReVoteMessage<T, Validated> {
        ReVoteMessage {
            revote: self.revote,
            signature: self.signature,
            _marker: PhantomData,
        }
    }
}

impl<T: NodeType, S> ReVoteMessage<T, S> {
    /// Whether `leader` signed this request.
    pub fn signed_by(&self, leader: &T::SignatureKey) -> bool {
        leader.validate(&self.signature, self.revote.commit().as_ref())
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn into_unchecked(self) -> ReVoteMessage<T, Unchecked> {
        ReVoteMessage {
            revote: self.revote,
            signature: self.signature,
            _marker: PhantomData,
        }
    }
}

impl<T: NodeType, S> HasViewNumber for ReVoteMessage<T, S> {
    fn view_number(&self) -> ViewNumber {
        self.revote.view
    }
}

impl<T: NodeType, S> HasEpoch for ReVoteMessage<T, S> {
    fn epoch(&self) -> Option<EpochNumber> {
        Some(self.revote.epoch)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct ProposalFetchRequest<T: NodeType> {
    pub payload: ProposalRequestPayload<T>,
    pub signature: <T::SignatureKey as SignatureKey>::PureAssembledSignatureType,
}

impl<T: NodeType> ProposalFetchRequest<T> {
    pub fn new(
        view_number: ViewNumber,
        key: T::SignatureKey,
        private_key: &<T::SignatureKey as SignatureKey>::PrivateKey,
    ) -> Result<Self, <T::SignatureKey as SignatureKey>::SignError> {
        let payload = ProposalRequestPayload { view_number, key };
        let signature = T::SignatureKey::sign(private_key, payload.commit().as_ref())?;
        Ok(Self { payload, signature })
    }

    pub fn validate_sender(&self, sender: &T::SignatureKey) -> bool {
        &self.payload.key == sender
            && self
                .payload
                .key
                .validate(&self.signature, self.payload.commit().as_ref())
    }
}

impl<T: NodeType> HasViewNumber for ProposalFetchRequest<T> {
    fn view_number(&self) -> ViewNumber {
        self.payload.view_number
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = "S: Deserialize<'de>"))]
#[allow(clippy::large_enum_variant)]
pub enum ConsensusMessage<T: NodeType, S> {
    Proposal(ProposalMessage<T, S>),
    Vote1(Vote1<T>),
    Vote2(Vote2<T>),
    Certificate1(Certificate1<T>, T::SignatureKey),
    Certificate2(Certificate2<T>, T::SignatureKey),
    TimeoutVote(TimeoutVoteMessage<T>),
    TimeoutCertificate(TimeoutCertificate2<T>),
    EpochChange(EpochChangeMessage<T, S>),
    /// The leader's unicast of a per-namespace VID share fragment.
    VidShareFragment(VidShareFragmentMessage<T>),
    /// A node's own VID share, broadcast independently of Vote1.
    VidShareBroadcast(VidDisperseShare2<T>),
    HighQc(Certificate1<T>),
    // Only append new variants: bincode tags variants by index, so reordering
    // or inserting breaks wire compatibility within a protocol version.
    TimeoutVote3(TimeoutVoteMessage3<T>),
    TimeoutCertificate3(TimeoutCertificate3<T>),
    UpgradeProposal(UpgradeProposalMessage<T>),
    UpgradeVote(UpgradeVoteMessage<T>),
    ReVote(ReVoteMessage<T, S>),
}

impl<T: NodeType, S> ConsensusMessage<T, S> {
    #[cfg(any(test, feature = "testing"))]
    pub fn into_unchecked(self) -> ConsensusMessage<T, Unchecked> {
        match self {
            Self::Proposal(p) => ConsensusMessage::Proposal(p.into_unchecked()),
            Self::Vote1(v) => ConsensusMessage::Vote1(v),
            Self::Vote2(v) => ConsensusMessage::Vote2(v),
            Self::Certificate1(c, k) => ConsensusMessage::Certificate1(c, k),
            Self::Certificate2(c, k) => ConsensusMessage::Certificate2(c, k),
            Self::TimeoutVote(v) => ConsensusMessage::TimeoutVote(v),
            Self::TimeoutCertificate(c) => ConsensusMessage::TimeoutCertificate(c),
            Self::EpochChange(c) => ConsensusMessage::EpochChange(c.into_unchecked()),
            Self::VidShareFragment(v) => ConsensusMessage::VidShareFragment(v),
            Self::VidShareBroadcast(v) => ConsensusMessage::VidShareBroadcast(v),
            Self::HighQc(c) => ConsensusMessage::HighQc(c),
            Self::TimeoutVote3(v) => ConsensusMessage::TimeoutVote3(v),
            Self::TimeoutCertificate3(c) => ConsensusMessage::TimeoutCertificate3(c),
            Self::UpgradeProposal(p) => ConsensusMessage::UpgradeProposal(p),
            Self::UpgradeVote(v) => ConsensusMessage::UpgradeVote(v),
            Self::ReVote(r) => ConsensusMessage::ReVote(r.into_unchecked()),
        }
    }
}

impl<T: NodeType, S> HasViewNumber for ConsensusMessage<T, S> {
    fn view_number(&self) -> ViewNumber {
        match self {
            Self::Proposal(proposal) => proposal.view_number(),
            Self::Vote1(vote) => vote.view_number(),
            Self::Vote2(vote) => vote.view_number(),
            Self::Certificate1(certificate, _) => certificate.view_number(),
            Self::Certificate2(certificate, _) => certificate.view_number(),
            Self::TimeoutVote(msg) => msg.view_number(),
            Self::TimeoutCertificate(certificate) => certificate.view_number(),
            Self::EpochChange(epoch_change) => epoch_change.view_number(),
            Self::VidShareFragment(fragment) => fragment.data.view_number(),
            Self::VidShareBroadcast(vid_share) => vid_share.view_number(),
            Self::HighQc(certificate) => certificate.view_number(),
            Self::TimeoutVote3(msg) => msg.view_number(),
            Self::TimeoutCertificate3(certificate) => certificate.view_number(),
            Self::UpgradeProposal(proposal) => proposal.data.view_number(),
            Self::UpgradeVote(vote) => vote.view_number(),
            Self::ReVote(revote) => revote.view_number(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub enum ProposalFetchMessage<T: NodeType> {
    Request(ProposalFetchRequest<T>),
    Response(Box<SignedProposal<T, Proposal<T>>>),
}

impl<T: NodeType> HasViewNumber for ProposalFetchMessage<T> {
    fn view_number(&self) -> ViewNumber {
        match self {
            Self::Request(request) => request.view_number(),
            Self::Response(proposal) => proposal.data.view_number(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct DedupManifest<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub hashes: Vec<Commitment<T::Transaction>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct TransactionMessage<T: NodeType> {
    pub view: ViewNumber,
    pub transactions: Vec<T::Transaction>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub enum BlockMessage<T: NodeType> {
    Transactions(TransactionMessage<T>),
    DedupManifest(DedupManifest<T>),
}

impl<T: NodeType> HasViewNumber for BlockMessage<T> {
    fn view_number(&self) -> ViewNumber {
        match self {
            BlockMessage::Transactions(msg) => msg.view,
            BlockMessage::DedupManifest(msg) => msg.view,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = "S: Deserialize<'de>"))]
#[allow(clippy::large_enum_variant)]
pub enum MessageType<T: NodeType, S> {
    Consensus(ConsensusMessage<T, S>),
    Block(BlockMessage<T>),
    ProposalFetch(ProposalFetchMessage<T>),
    External(#[serde(with = "serde_bytes")] Vec<u8>),
    PayloadFetch(PayloadFetchMessage),
}

impl<T: NodeType, S> MessageType<T, S> {
    #[cfg(any(test, feature = "testing"))]
    pub fn into_unchecked(self) -> MessageType<T, Unchecked> {
        match self {
            Self::Consensus(c) => MessageType::Consensus(c.into_unchecked()),
            Self::Block(b) => MessageType::Block(b),
            Self::ProposalFetch(r) => MessageType::ProposalFetch(r),
            Self::External(v) => MessageType::External(v),
            Self::PayloadFetch(r) => MessageType::PayloadFetch(r),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = "S: Deserialize<'de>"))]
pub struct Message<T: NodeType, S> {
    pub sender: T::SignatureKey,
    pub message_type: MessageType<T, S>,
}

impl<T: NodeType, S> Message<T, S> {
    pub fn is_external(&self) -> bool {
        matches!(self.message_type, MessageType::External(_))
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn into_unchecked(self) -> Message<T, Unchecked> {
        Message {
            sender: self.sender,
            message_type: self.message_type.into_unchecked(),
        }
    }
}

impl<T: NodeType, S> HasViewNumber for Message<T, S> {
    fn view_number(&self) -> ViewNumber {
        match &self.message_type {
            MessageType::Consensus(consensus_message) => consensus_message.view_number(),
            MessageType::Block(block_message) => block_message.view_number(),
            MessageType::ProposalFetch(message) => message.view_number(),
            MessageType::External(_) => ViewNumber::new(1), // TODO: This can become a problem
            MessageType::PayloadFetch(message) => message.view_number(),
        }
    }
}

pub struct OpaqueMessage<K> {
    pub sender: K,
    pub data: Vec<u8>,
}
