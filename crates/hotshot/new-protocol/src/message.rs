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

pub type Vote2<TYPES> = SimpleVote<TYPES, Vote2Data<TYPES>>;
pub type Certificate1<TYPES> = SimpleCertificate<TYPES, QuorumData2<TYPES>, SuccessThreshold>;
pub type Certificate2<TYPES> = SimpleCertificate<TYPES, Vote2Data<TYPES>, SuccessThreshold>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct ProposalMessage<TYPES: NodeType> {
    pub(crate) proposal: Proposal<TYPES, QuorumProposal2<TYPES>>,
    pub(crate) vid_share: VidDisperseShare2<TYPES>,
}

impl<TYPES: NodeType> HasViewNumber for ProposalMessage<TYPES> {
    fn view_number(&self) -> ViewNumber {
        self.proposal.data.view_number
    }
}

/// Data used for a yes vote.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct Vote2Data<TYPES: NodeType> {
    pub leaf_commit: Commitment<Leaf2<TYPES>>,
    pub epoch: EpochNumber,
    pub block_number: u64,
}

impl<TYPES: NodeType> HasEpoch for Vote2Data<TYPES> {
    fn epoch(&self) -> Option<EpochNumber> {
        Some(self.epoch)
    }
}

impl<TYPES: NodeType> Committable for Vote2Data<TYPES> {
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
pub struct Vote1<TYPES: NodeType> {
    pub(crate) vote: QuorumVote2<TYPES>,
    pub(crate) vid_share: VidDisperseShare2<TYPES>,
}

impl<TYPES: NodeType> HasViewNumber for Vote1<TYPES> {
    fn view_number(&self) -> ViewNumber {
        self.vote.view_number()
    }
}

impl<TYPES: NodeType> QuorumMarker for Vote2Data<TYPES> {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub enum ConsensusMessage<TYPES: NodeType> {
    Proposal(ProposalMessage<TYPES>),
    Vote1(Vote1<TYPES>),
    Vote2(Vote2<TYPES>),
    Certificate1(Certificate1<TYPES>, TYPES::SignatureKey),
    Certificate2(Certificate2<TYPES>, TYPES::SignatureKey),
    TimeoutVote(TimeoutVote2<TYPES>),
    Transactions(Vec<TYPES::Transaction>, ViewNumber),
    Checkpoint(ViewNumber, EpochNumber),
}

impl<TYPES: NodeType> HasViewNumber for ConsensusMessage<TYPES> {
    fn view_number(&self) -> ViewNumber {
        match self {
            ConsensusMessage::Proposal(proposal) => proposal.view_number(),
            ConsensusMessage::Vote1(vote) => vote.view_number(),
            ConsensusMessage::Vote2(vote) => vote.view_number(),
            ConsensusMessage::Certificate1(certificate, _) => certificate.view_number(),
            ConsensusMessage::Certificate2(certificate, _) => certificate.view_number(),
            ConsensusMessage::TimeoutVote(vote) => vote.view_number(),
            ConsensusMessage::Transactions(_, view_number) => *view_number,
            ConsensusMessage::Checkpoint(view_number, _) => *view_number,
        }
    }
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub enum ViewSyncMessage<TYPES: NodeType> {
    ViewSyncPreCommitVote(ViewSyncPreCommitVote2<TYPES>),
    ViewSyncCommitVote(ViewSyncCommitVote2<TYPES>),
    ViewSyncFinalizeVote(ViewSyncFinalizeVote2<TYPES>),
    ViewSyncPreCommitCertificate(ViewSyncPreCommitCertificate2<TYPES>),
    ViewSyncCommitCertificate(ViewSyncCommitCertificate2<TYPES>),
    ViewSyncFinalizeCertificate(ViewSyncFinalizeCertificate2<TYPES>),
}

impl<TYPES: NodeType> HasViewNumber for ViewSyncMessage<TYPES> {
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
pub enum MessageType<TYPES: NodeType> {
    Consensus(ConsensusMessage<TYPES>),
    ViewSync(ViewSyncMessage<TYPES>),
    External(Vec<u8>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = ""))]
pub struct Message<TYPES: NodeType> {
    pub sender: TYPES::SignatureKey,
    pub message_type: MessageType<TYPES>,
}

impl<TYPES: NodeType> HasViewNumber for Message<TYPES> {
    fn view_number(&self) -> ViewNumber {
        match &self.message_type {
            MessageType::Consensus(consensus_message) => consensus_message.view_number(),
            MessageType::ViewSync(view_sync_message) => view_sync_message.view_number(),
            MessageType::External(_) => ViewNumber::new(0),
        }
    }
}
