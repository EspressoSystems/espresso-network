use std::marker::PhantomData;

use committable::{Commitment, Committable};
use hotshot_types::{
    data::{EpochNumber, Leaf2, QuorumProposal2, VidDisperseShare2, ViewNumber},
    message::Proposal,
    simple_certificate::{
        SimpleCertificate, SuccessThreshold, ViewSyncCommitCertificate2,
        ViewSyncFinalizeCertificate2, ViewSyncPreCommitCertificate2,
    },
    simple_vote::{
        HasEpoch, QuorumData2, QuorumMarker, QuorumVote2, SimpleVote, TimeoutVote2,
        ViewSyncCommitVote2, ViewSyncFinalizeVote2, ViewSyncPreCommitVote2,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Deserialize)]
pub enum Unchecked {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize)]
pub enum Validated {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = "S: Deserialize<'de>"))]
pub struct ProposalMessage<T: NodeType, S> {
    pub proposal: Proposal<T, QuorumProposal2<T>>,
    pub vid_share: VidDisperseShare2<T>,
    #[serde(skip)]
    _marker: PhantomData<fn() -> S>,
}

impl<T: NodeType> ProposalMessage<T, Validated> {
    pub fn validated(p: Proposal<T, QuorumProposal2<T>>, s: VidDisperseShare2<T>) -> Self {
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

/// Data used for a yes vote.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct Vote2Data<T: NodeType> {
    pub leaf_commit: Commitment<Leaf2<T>>,
    pub epoch: EpochNumber,
    pub block_number: u64,
}

/// Data used for checkpointing.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
pub struct CheckpointData {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
}

impl Committable for CheckpointData {
    fn commit(&self) -> Commitment<Self> {
        committable::RawCommitmentBuilder::new("CheckpointData")
            .u64(*self.view)
            .u64(*self.epoch)
            .finalize()
    }
}

impl HasViewNumber for CheckpointData {
    fn view_number(&self) -> ViewNumber {
        self.view
    }
}

impl HasEpoch for CheckpointData {
    fn epoch(&self) -> Option<EpochNumber> {
        Some(self.epoch)
    }
}

impl QuorumMarker for CheckpointData {}

impl<T: NodeType> HasEpoch for Vote2Data<T> {
    fn epoch(&self) -> Option<EpochNumber> {
        Some(self.epoch)
    }
}

impl<T: NodeType> Committable for Vote2Data<T> {
    fn commit(&self) -> Commitment<Self> {
        committable::RawCommitmentBuilder::new("Vote2Data")
            .var_size_bytes(self.leaf_commit.as_ref())
            .u64(*self.epoch)
            .u64(self.block_number)
            .constant_str("Vote2")
            .finalize()
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

impl<T: NodeType> QuorumMarker for Vote2Data<T> {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = "S: Deserialize<'de>"))]
#[allow(clippy::large_enum_variant)]
pub enum ConsensusMessage<T: NodeType, S> {
    Proposal(ProposalMessage<T, S>),
    Vote1(Vote1<T>),
    Vote2(Vote2<T>),
    Certificate1(Certificate1<T>, T::SignatureKey),
    Certificate2(Certificate2<T>, T::SignatureKey),
    TimeoutVote(TimeoutVote2<T>),
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
            Self::Checkpoint(v) => ConsensusMessage::Checkpoint(v),
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
            Self::TimeoutVote(vote) => vote.view_number(),
            Self::Checkpoint(vote) => vote.view_number(),
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
#[serde(bound(deserialize = ""))]
pub enum ViewSyncMessage<T: NodeType> {
    ViewSyncPreCommitVote(ViewSyncPreCommitVote2<T>),
    ViewSyncCommitVote(ViewSyncCommitVote2<T>),
    ViewSyncFinalizeVote(ViewSyncFinalizeVote2<T>),
    ViewSyncPreCommitCertificate(ViewSyncPreCommitCertificate2<T>),
    ViewSyncCommitCertificate(ViewSyncCommitCertificate2<T>),
    ViewSyncFinalizeCertificate(ViewSyncFinalizeCertificate2<T>),
}

impl<T: NodeType> HasViewNumber for ViewSyncMessage<T> {
    fn view_number(&self) -> ViewNumber {
        match self {
            ViewSyncMessage::ViewSyncPreCommitVote(vote) => vote.view_number(),
            ViewSyncMessage::ViewSyncCommitVote(vote) => vote.view_number(),
            ViewSyncMessage::ViewSyncFinalizeVote(vote) => vote.view_number(),
            ViewSyncMessage::ViewSyncPreCommitCertificate(certificate) => certificate.view_number(),
            ViewSyncMessage::ViewSyncCommitCertificate(certificate) => certificate.view_number(),
            ViewSyncMessage::ViewSyncFinalizeCertificate(certificate) => certificate.view_number(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = "S: Deserialize<'de>"))]
#[allow(clippy::large_enum_variant)]
pub enum MessageType<T: NodeType, S> {
    Consensus(ConsensusMessage<T, S>),
    Block(BlockMessage<T>),
    ViewSync(ViewSyncMessage<T>),
    External(Vec<u8>),
}

impl<T: NodeType, S> MessageType<T, S> {
    #[cfg(test)]
    pub fn into_unchecked(self) -> MessageType<T, Unchecked> {
        match self {
            Self::Consensus(c) => MessageType::Consensus(c.into_unchecked()),
            Self::Block(b) => MessageType::Block(b),
            Self::ViewSync(m) => MessageType::ViewSync(m),
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
            MessageType::ViewSync(view_sync_message) => view_sync_message.view_number(),
            MessageType::External(_) => ViewNumber::new(0), // TODO: This can become a problem
        }
    }
}
