use std::marker::PhantomData;

use committable::{Commitment, Committable};
pub use hotshot_types::new_protocol::Proposal;
use hotshot_types::{
    data::{EpochNumber, VidDisperseShare2, ViewNumber},
    message::Proposal as SignedProposal,
    request_response::ProposalRequestPayload,
    simple_certificate::{
        OneHonestThreshold, SimpleCertificate, SuccessThreshold, TimeoutCertificate2,
    },
    simple_vote::{
        CheckpointData, LightClientStateUpdateVote2, QuorumData2, QuorumVote2, SimpleVote,
        TimeoutData2, TimeoutVote2, Vote2Data,
    },
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
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
    /// Populated only when voting on an epoch-root leaf. Required there; absent otherwise.
    pub state_vote: Option<LightClientStateUpdateVote2<T>>,
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
