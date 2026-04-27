use std::marker::PhantomData;

use committable::Commitment;
use hotshot_types::{
    data::{
        EpochNumber, Leaf2, QuorumProposal2, QuorumProposalWrapper, VidDisperseShare2,
        ViewChangeEvidence2, ViewNumber,
    },
    drb::DrbResult,
    message::Proposal as SignedProposal,
    simple_certificate::{
        LightClientStateUpdateCertificateV2, OneHonestThreshold, QuorumCertificate2,
        SimpleCertificate, SuccessThreshold, TimeoutCertificate2, UpgradeCertificate,
    },
    simple_vote::{
        CheckpointData, HasEpoch, QuorumData2, QuorumVote2, SimpleVote, TimeoutData2, TimeoutVote2,
        Vote2Data,
    },
    traits::node_implementation::NodeType,
    vote::HasViewNumber,
};
use serde::{Deserialize, Serialize};

pub type Vote2<T> = SimpleVote<T, Vote2Data<T>>;
pub type CheckpointVote<T> = SimpleVote<T, CheckpointData>;
pub type CheckpointCertificate<T> = SimpleCertificate<T, CheckpointData, SuccessThreshold>;
pub type Certificate1<T> = SimpleCertificate<T, QuorumData2<T>, SuccessThreshold>;
pub type Certificate2<T> = SimpleCertificate<T, Vote2Data<T>, SuccessThreshold>;
pub type TimeoutCertificate<T> = SimpleCertificate<T, TimeoutData2, SuccessThreshold>;
pub type TimeoutOneHonest<T> = SimpleCertificate<T, TimeoutData2, OneHonestThreshold>;

/// Proposal to append a block.
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
#[serde(bound(deserialize = ""))]
pub struct Proposal<T: NodeType> {
    /// The block header to append
    pub block_header: T::BlockHeader,

    /// view number for the proposal
    pub view_number: ViewNumber,

    /// The epoch number corresponding to the block number.
    ///
    /// Can be `None` for pre-epoch version.
    pub epoch: EpochNumber,

    /// certificate that the proposal is chaining from
    pub justify_qc: QuorumCertificate2<T>,

    /// certificate proving the last block of the epoch is decided
    pub next_epoch_justify_qc: Option<Certificate2<T>>,

    /// Possible upgrade certificate, which the leader may optionally attach.
    pub upgrade_certificate: Option<UpgradeCertificate<T>>,

    /// Possible timeout certificate.
    ///
    /// If the `justify_qc` is not for a proposal in the immediately preceding
    /// view, then a timeout certificate must be attached.
    pub view_change_evidence: Option<TimeoutCertificate<T>>,

    /// The DRB result for the next epoch.
    ///
    /// This is required only for the last block of the epoch. Nodes will verify
    /// that it's consistent with the result from their computations.
    #[serde(with = "serde_bytes")]
    pub next_drb_result: Option<DrbResult>,

    /// The light client state update certificate for the next epoch.
    /// This is required for the epoch root.
    pub state_cert: Option<LightClientStateUpdateCertificateV2<T>>,
}

impl<T: NodeType> HasViewNumber for Proposal<T> {
    fn view_number(&self) -> ViewNumber {
        self.view_number
    }
}

impl<T: NodeType> HasEpoch for Proposal<T> {
    fn epoch(&self) -> Option<EpochNumber> {
        Some(self.epoch)
    }
}

impl<T: NodeType> From<QuorumProposalWrapper<T>> for Proposal<T> {
    fn from(wrapper: QuorumProposalWrapper<T>) -> Self {
        let qp = wrapper.proposal;
        Self {
            block_header: qp.block_header,
            view_number: qp.view_number,
            epoch: qp.epoch.unwrap_or(EpochNumber::new(0)),
            justify_qc: qp.justify_qc,
            next_epoch_justify_qc: None,
            upgrade_certificate: qp.upgrade_certificate,
            view_change_evidence: qp.view_change_evidence.and_then(|e| match e {
                ViewChangeEvidence2::Timeout(tc) => Some(tc),
                ViewChangeEvidence2::ViewSync(_) => None,
            }),
            next_drb_result: qp.next_drb_result,
            state_cert: qp.state_cert,
        }
    }
}

impl<T: NodeType> From<Proposal<T>> for QuorumProposalWrapper<T> {
    fn from(p: Proposal<T>) -> Self {
        QuorumProposalWrapper::from(QuorumProposal2 {
            block_header: p.block_header,
            view_number: p.view_number,
            epoch: Some(p.epoch),
            justify_qc: p.justify_qc,
            next_epoch_justify_qc: None,
            upgrade_certificate: p.upgrade_certificate,
            view_change_evidence: p.view_change_evidence.map(ViewChangeEvidence2::Timeout),
            next_drb_result: p.next_drb_result,
            state_cert: p.state_cert,
        })
    }
}

impl<T: NodeType> From<Proposal<T>> for Leaf2<T> {
    fn from(p: Proposal<T>) -> Self {
        Self::from_quorum_proposal(&QuorumProposalWrapper::from(p))
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
    pub vid_share: VidDisperseShare2<T>,
    #[serde(skip)]
    _marker: PhantomData<fn() -> S>,
}

impl<T: NodeType> ProposalMessage<T, Validated> {
    pub fn validated(p: SignedProposal<T, Proposal<T>>, s: VidDisperseShare2<T>) -> Self {
        Self {
            proposal: p,
            vid_share: s,
            _marker: PhantomData,
        }
    }
}

impl<T: NodeType, S> ProposalMessage<T, S> {
    #[cfg(test)]
    pub fn into_unchecked(self) -> ProposalMessage<T, Unchecked> {
        ProposalMessage {
            proposal: self.proposal,
            vid_share: self.vid_share,
            _marker: PhantomData,
        }
    }
}

impl<T: NodeType, S> HasViewNumber for ProposalMessage<T, S> {
    fn view_number(&self) -> ViewNumber {
        self.proposal.data.view_number
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct Vote1<T: NodeType> {
    pub vote: QuorumVote2<T>,
    pub vid_share: VidDisperseShare2<T>,
}

impl<T: NodeType> HasViewNumber for Vote1<T> {
    fn view_number(&self) -> ViewNumber {
        self.vote.view_number()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct TimeoutVoteMessage<T: NodeType> {
    pub vote: TimeoutVote2<T>,
    pub lock: Option<Certificate1<T>>,
}

impl<T: NodeType> HasViewNumber for TimeoutVoteMessage<T> {
    fn view_number(&self) -> ViewNumber {
        self.vote.view_number()
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
#[serde(bound(deserialize = ""))]
pub struct EpochChangeMessage<T: NodeType> {
    pub cert1: Certificate1<T>,
    pub cert2: Certificate2<T>,
    pub proposal: Proposal<T>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct ProposalFetchRequest<T: NodeType> {
    pub view_number: ViewNumber,
    #[serde(skip)]
    _marker: PhantomData<fn() -> T>,
}

impl<T: NodeType> ProposalFetchRequest<T> {
    pub fn new(view_number: ViewNumber) -> Self {
        Self {
            view_number,
            _marker: PhantomData,
        }
    }
}

impl<T: NodeType> HasViewNumber for ProposalFetchRequest<T> {
    fn view_number(&self) -> ViewNumber {
        self.view_number
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
    EpochChange(EpochChangeMessage<T>),
    Checkpoint(CheckpointVote<T>),
}

impl<T: NodeType, S> ConsensusMessage<T, S> {
    #[cfg(test)]
    pub fn into_unchecked(self) -> ConsensusMessage<T, Unchecked> {
        match self {
            Self::Proposal(p) => ConsensusMessage::Proposal(p.into_unchecked()),
            Self::Vote1(v) => ConsensusMessage::Vote1(v),
            Self::Vote2(v) => ConsensusMessage::Vote2(v),
            Self::Certificate1(c, k) => ConsensusMessage::Certificate1(c, k),
            Self::Certificate2(c, k) => ConsensusMessage::Certificate2(c, k),
            Self::TimeoutVote(v) => ConsensusMessage::TimeoutVote(v),
            Self::TimeoutCertificate(c) => ConsensusMessage::TimeoutCertificate(c),
            Self::Checkpoint(v) => ConsensusMessage::Checkpoint(v),
            Self::EpochChange(c) => ConsensusMessage::EpochChange(c),
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
            Self::Checkpoint(vote) => vote.view_number(),
            Self::EpochChange(epoch_change) => epoch_change.cert1.view_number(),
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
    pub(crate) view: ViewNumber,
    pub(crate) epoch: EpochNumber,
    pub(crate) hashes: Vec<Commitment<T::Transaction>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct TransactionMessage<T: NodeType> {
    pub(crate) view: ViewNumber,
    pub(crate) transactions: Vec<T::Transaction>,
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
    External(Vec<u8>),
}

impl<T: NodeType, S> MessageType<T, S> {
    #[cfg(test)]
    pub fn into_unchecked(self) -> MessageType<T, Unchecked> {
        match self {
            Self::Consensus(c) => MessageType::Consensus(c.into_unchecked()),
            Self::Block(b) => MessageType::Block(b),
            Self::ProposalFetch(r) => MessageType::ProposalFetch(r),
            Self::External(v) => MessageType::External(v),
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

    #[cfg(test)]
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
            MessageType::External(_) => ViewNumber::new(0), // TODO: This can become a problem
        }
    }
}
