use committable::Commitment;
use hotshot_types::{
    data::{EpochNumber, Leaf2, VidCommitment2, VidDisperse2, ViewNumber},
    drb::{DrbInput, DrbResult},
    simple_certificate::{TimeoutCertificate2, ViewSyncCommitCertificate2},
    traits::node_implementation::NodeType,
    vote::HasViewNumber,
};

use crate::message::{
    Certificate1, Certificate2, ConsensusMessage, Proposal, ProposalMessage, Vote1, Vote2,
};

pub(crate) struct StateRequest<TYPES: NodeType> {
    pub view: ViewNumber,
    pub parent_view: ViewNumber,
    pub epoch: EpochNumber,
    pub block_number: u64,
    pub proposal: Proposal<TYPES>,
}

pub(crate) struct StateResponse<TYPES: NodeType> {
    pub view: ViewNumber,
    pub commitment: Commitment<Proposal<TYPES>>,
}

pub(crate) struct HeaderRequest {
    pub view: ViewNumber,
    pub parent_view: ViewNumber,
    pub epoch: EpochNumber,
    pub block_number: u64,
}

pub enum Event<TYPES: NodeType> {
    SendMessage(ConsensusMessage<TYPES>),
    RequestState(StateRequest<TYPES>),
    RequestHeader(HeaderRequest),
    StateVerified(StateRequest<TYPES>),
    HeaderCreated(TYPES::BlockHeader),
    StateVerificationFailed(StateRequest<TYPES>),
    HeaderCreationFailed(HeaderRequest),
    RequestVidDisperse(TYPES::BlockPayload),
    VidDisperseCreated(VidCommitment2, VidDisperse2<TYPES>),
    LeafDecided(Vec<Leaf2<TYPES>>),
    RequestProposal(ViewNumber, Commitment<Leaf2<TYPES>>),
    RequestDRB(DrbInput),
    DrbCalculated(DrbResult),
    LockUpdated(Certificate2<TYPES>),
    ViewChanged(ViewNumber, EpochNumber),
    BlockReconstructed(ViewNumber, TYPES::BlockPayload, VidCommitment2),
}

pub enum ConsensusEvent<TYPES: NodeType> {
    Proposal(ProposalMessage<TYPES>),
    Certificate1(Certificate1<TYPES>),
    Certificate2(Certificate2<TYPES>),
    TimeoutCertificate(TimeoutCertificate2<TYPES>),
    ViewSyncCertificate(ViewSyncCommitCertificate2<TYPES>),
    BlockReconstructed(ViewNumber, VidCommitment2),
    StateVerified(StateResponse<TYPES>),
    HeaderCreated(ViewNumber, TYPES::BlockHeader),
    StateVerificationFailed(StateRequest<TYPES>),
    HeaderCreationFailed(HeaderRequest),
    Timeout(ViewNumber),
}

impl<TYPES: NodeType> ConsensusEvent<TYPES> {
    pub fn view_number(&self) -> ViewNumber {
        match self {
            ConsensusEvent::Proposal(proposal) => proposal.view_number(),
            ConsensusEvent::Certificate1(certificate) => certificate.view_number(),
            ConsensusEvent::Certificate2(certificate) => certificate.view_number(),
            ConsensusEvent::TimeoutCertificate(simple_certificate) => {
                simple_certificate.view_number()
            },
            ConsensusEvent::ViewSyncCertificate(simple_certificate) => {
                simple_certificate.view_number()
            },
            ConsensusEvent::BlockReconstructed(view_number, _) => *view_number,
            ConsensusEvent::StateVerified(state_response) => state_response.view,
            ConsensusEvent::HeaderCreated(view_number, _) => *view_number,
            ConsensusEvent::StateVerificationFailed(state_request) => state_request.view,
            ConsensusEvent::HeaderCreationFailed(header_request) => header_request.view,
            ConsensusEvent::Timeout(view_number) => *view_number,
        }
    }
}

pub enum NetworkEvent<TYPES: NodeType> {
    SendMessage(ConsensusMessage<TYPES>),
    ViewChanged(ViewNumber, EpochNumber),
}

pub enum IOEvent<TYPES: NodeType> {
    StorageEvent(StorageEvent<TYPES>),
    NetworkEvent(NetworkEvent<TYPES>),
}

pub enum StorageEvent<TYPES: NodeType> {
    StoreProposal(Proposal<TYPES>),
    StoreCertificate1(Certificate1<TYPES>),
    StoreCertificate2(Certificate2<TYPES>),
    StoreBlock(TYPES::BlockPayload),
    StoreShares(VidDisperse2<TYPES>),
}

pub enum StateEvent<TYPES: NodeType> {
    RequestState(StateRequest<TYPES>),
    RequestHeader(HeaderRequest),
    UpdateState(TYPES::ValidatedState, ViewNumber, Commitment<Leaf2<TYPES>>),
}

pub enum CpuEvent<TYPES: NodeType> {
    DrbRequest(DrbInput),
    VidShare(VidDisperse2<TYPES>),
    Vote1(Vote1<TYPES>),
    Vote2(Vote2<TYPES>),
}
