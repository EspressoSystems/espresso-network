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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct ProposalMessage<T: NodeType> {
    pub proposal: Proposal<T, QuorumProposal2<T>>,
    pub vid_share: VidDisperseShare2<T>,
}

impl<T: NodeType> HasViewNumber for ProposalMessage<T> {
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

/// Data used .
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
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
#[serde(bound(deserialize = ""))]
#[allow(clippy::large_enum_variant)]
pub enum ConsensusMessage<T: NodeType> {
    Proposal(ProposalMessage<T>),
    Vote1(Vote1<T>),
    Vote2(Vote2<T>),
    Certificate1(Certificate1<T>, T::SignatureKey),
    Certificate2(Certificate2<T>, T::SignatureKey),
    TimeoutVote(TimeoutVote2<T>),
    Checkpoint(CheckpointVote<T>),
}

impl<T: NodeType> HasViewNumber for ConsensusMessage<T> {
    fn view_number(&self) -> ViewNumber {
        match self {
            ConsensusMessage::Proposal(proposal) => proposal.view_number(),
            ConsensusMessage::Vote1(vote) => vote.view_number(),
            ConsensusMessage::Vote2(vote) => vote.view_number(),
            ConsensusMessage::Certificate1(certificate, _) => certificate.view_number(),
            ConsensusMessage::Certificate2(certificate, _) => certificate.view_number(),
            ConsensusMessage::TimeoutVote(vote) => vote.view_number(),
            ConsensusMessage::Checkpoint(vote) => vote.view_number(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct DedupManifest<T: NodeType> {
    pub(crate) view: ViewNumber,
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
#[serde(bound(deserialize = ""))]
pub enum MessageType<T: NodeType> {
    Consensus(ConsensusMessage<T>),
    Block(BlockMessage<T>),
    ViewSync(ViewSyncMessage<T>),
    External(Vec<u8>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct Message<T: NodeType> {
    pub sender: T::SignatureKey,
    pub message_type: MessageType<T>,
}

impl<T: NodeType> Message<T> {
    pub fn is_external(&self) -> bool {
        matches!(self.message_type, MessageType::External(_))
    }
}

impl<T: NodeType> HasViewNumber for Message<T> {
    fn view_number(&self) -> ViewNumber {
        match &self.message_type {
            MessageType::Consensus(consensus_message) => consensus_message.view_number(),
            MessageType::Block(block_message) => block_message.view_number(),
            MessageType::ViewSync(view_sync_message) => view_sync_message.view_number(),
            MessageType::External(_) => ViewNumber::new(0), // TODO: This can become a problem
        }
    }
}
