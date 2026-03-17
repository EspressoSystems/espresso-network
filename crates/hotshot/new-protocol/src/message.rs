use committable::{Commitment, Committable};
use hotshot_types::{
    data::{
        EpochNumber, Leaf2, QuorumProposal2, VidDisperseShare2, ViewChangeEvidence2, ViewNumber,
    },
    drb::DrbResult,
    simple_certificate::{
        LightClientStateUpdateCertificateV2, SimpleCertificate, SuccessThreshold,
        UpgradeCertificate, ViewSyncCommitCertificate2, ViewSyncFinalizeCertificate2,
        ViewSyncPreCommitCertificate2,
    },
    simple_vote::{
        HasEpoch, QuorumData2, QuorumMarker, QuorumVote2, SimpleVote, TimeoutVote2,
        ViewSyncCommitVote2, ViewSyncFinalizeVote2, ViewSyncPreCommitVote2,
    },
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
    vote::HasViewNumber,
};
use serde::{Deserialize, Serialize};

pub struct ProposalMessage<TYPES: NodeType> {
    pub(crate) proposal: hotshot_types::message::Proposal<TYPES, QuorumProposal2<TYPES>>,
    pub(crate) vid_share: hotshot_types::data::VidDisperseShare2<TYPES>,
}

impl<TYPES: NodeType> HasViewNumber for ProposalMessage<TYPES> {
    fn view_number(&self) -> ViewNumber {
        self.proposal.data.view_number
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
/// Data used for a yes vote.
#[serde(bound(deserialize = ""))]
pub struct Vote2Data<TYPES: NodeType> {
    pub leaf_commit: Commitment<Leaf2<TYPES>>,
    pub epoch: EpochNumber,
    pub block_number: u64,
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

pub struct Vote1<TYPES: NodeType> {
    pub vote: QuorumVote2<TYPES>,
    pub vid_share: VidDisperseShare2<TYPES>,
}

pub type Vote2<TYPES> = SimpleVote<TYPES, Vote2Data<TYPES>>;

pub type Certificate1<TYPES> = SimpleCertificate<TYPES, QuorumData2<TYPES>, SuccessThreshold>;
pub type Certificate2<TYPES> = SimpleCertificate<TYPES, Vote2Data<TYPES>, SuccessThreshold>;

impl<TYPES: NodeType> QuorumMarker for Vote2Data<TYPES> {}

pub enum ConsensusMessage<TYPES: NodeType> {
    Proposal(ProposalMessage<TYPES>),
    Vote1(Vote1<TYPES>),
    Vote2(Vote2<TYPES>),
    Certificate1(Certificate1<TYPES>, TYPES::SignatureKey),
    Certificate2(Certificate2<TYPES>, TYPES::SignatureKey),
    TimeoutVote(TimeoutVote2<TYPES>),
    Transactions(Vec<TYPES::Transaction>),
}

pub enum ViewSyncMessage<TYPES: NodeType> {
    ViewSyncPreCommitVote(ViewSyncPreCommitVote2<TYPES>),
    ViewSyncCommitVote(ViewSyncCommitVote2<TYPES>),
    ViewSyncFinalizeVote(ViewSyncFinalizeVote2<TYPES>),
    ViewSyncPreCommitCertificate(ViewSyncPreCommitCertificate2<TYPES>),
    ViewSyncCommitCertificate(ViewSyncCommitCertificate2<TYPES>),
    ViewSyncFinalizeCertificate(ViewSyncFinalizeCertificate2<TYPES>),
}
