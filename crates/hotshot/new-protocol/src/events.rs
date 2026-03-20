use std::sync::Arc;

use committable::Commitment;
use hotshot::traits::{BlockPayload, ValidatedState};
use hotshot_types::{
    data::{
        EpochNumber, Leaf2, QuorumProposal2, VidCommitment, VidCommitment2, VidDisperse2,
        ViewNumber,
    },
    drb::{DrbInput, DrbResult},
    simple_certificate::{TimeoutCertificate2, ViewSyncFinalizeCertificate2},
    traits::{block_contents::BuilderFee, node_implementation::NodeType},
    utils::BuilderCommitment,
    vote::HasViewNumber,
};

use crate::{
    helpers::proposal_commitment,
    message::{Certificate1, Certificate2, ConsensusMessage, ProposalMessage, Vote1, Vote2},
};

#[derive(Eq, PartialEq, Debug, Clone)]
pub(crate) struct StateRequest<TYPES: NodeType> {
    pub view: ViewNumber,
    pub parent_view: ViewNumber,
    pub epoch: EpochNumber,
    pub block_number: u64,
    pub proposal: QuorumProposal2<TYPES>,
    pub parent_commitment: Commitment<Leaf2<TYPES>>,
    pub payload_size: u32,
}

#[derive(Eq, PartialEq, Debug)]
pub(crate) struct StateResponse<TYPES: NodeType> {
    pub view: ViewNumber,
    pub commitment: Commitment<Leaf2<TYPES>>,
    pub state: Arc<TYPES::ValidatedState>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub(crate) struct BlockAndHeaderRequest<TYPES: NodeType> {
    pub view: ViewNumber,
    pub parent_proposal: QuorumProposal2<TYPES>,
    pub epoch: EpochNumber,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub(crate) struct BlockRequest<TYPES: NodeType> {
    pub view: ViewNumber,
    pub parent_proposal: QuorumProposal2<TYPES>,
    pub epoch: EpochNumber,
}

#[derive(Eq, PartialEq, Debug)]
pub(crate) struct HeaderRequest<TYPES: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub parent_proposal: QuorumProposal2<TYPES>,
    pub payload_commitment: VidCommitment,
    pub builder_commitment: BuilderCommitment,
    pub metadata: <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    pub builder_fee: BuilderFee<TYPES>,
}

#[derive(Eq, PartialEq, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum RequestMessageSender<TYPES: NodeType> {
    Proposal(QuorumProposal2<TYPES>, VidDisperse2<TYPES>),
    Vote1(Vote1<TYPES>),
    Vote2(Vote2<TYPES>),
}

#[derive(Eq, PartialEq, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Action<TYPES: NodeType> {
    SendMessage(RequestMessageSender<TYPES>),
    RequestState(StateRequest<TYPES>),
    RequestBlockAndHeader(BlockAndHeaderRequest<TYPES>),
    RequestVidDisperse(
        ViewNumber,
        EpochNumber,
        TYPES::BlockPayload,
        <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    ),
    RequestProposal(ViewNumber, Commitment<Leaf2<TYPES>>),
    RequestDRB(DrbInput),
    Shutdown,
}

#[derive(Clone, Eq, PartialEq, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Update<TYPES: NodeType> {
    MessageReceived(ConsensusMessage<TYPES>),
    StateVerified(StateRequest<TYPES>),
    HeaderCreated(ViewNumber, TYPES::BlockHeader),
    StateVerificationFailed(StateRequest<TYPES>),
    HeaderCreationFailed(BlockAndHeaderRequest<TYPES>),
    VidDisperseCreated(VidCommitment2, VidDisperse2<TYPES>),
    LeafDecided(Vec<Leaf2<TYPES>>),
    DrbCalculated(DrbResult),
    LockUpdated(Certificate2<TYPES>),
    ViewChanged(ViewNumber, EpochNumber),
    BlockReconstructed(ViewNumber, TYPES::BlockPayload, VidCommitment2),
    Timeout(ViewNumber),
    TimeoutCertificateReceived(TimeoutCertificate2<TYPES>),
    ViewSyncCertificateReceived(ViewSyncFinalizeCertificate2<TYPES>),
}

#[allow(clippy::large_enum_variant)]
#[derive(Eq, PartialEq, Debug)]
pub enum Event<TYPES: NodeType> {
    Action(Action<TYPES>),
    Update(Update<TYPES>),
}

#[allow(clippy::large_enum_variant)]
pub enum ConsensusEvent<TYPES: NodeType> {
    Proposal(ProposalMessage<TYPES>),
    Certificate1(Certificate1<TYPES>),
    Certificate2(Certificate2<TYPES>),
    TimeoutCertificate(TimeoutCertificate2<TYPES>),
    ViewSyncCertificate(ViewSyncFinalizeCertificate2<TYPES>),
    BlockReconstructed(ViewNumber, VidCommitment2),
    BlockBuilt(
        ViewNumber,
        EpochNumber,
        TYPES::BlockPayload,
        <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    ),
    VidDisperseCreated(ViewNumber, VidDisperse2<TYPES>),
    StateVerified(StateResponse<TYPES>),
    HeaderCreated(ViewNumber, TYPES::BlockHeader),
    StateVerificationFailed(StateResponse<TYPES>),
    Timeout(ViewNumber),
    Shutdown,
    // TODO: Add checkpoint events
}

impl<TYPES: NodeType> ConsensusEvent<TYPES> {
    pub fn view_number(&self) -> ViewNumber {
        match self {
            ConsensusEvent::Proposal(proposal) => proposal.view_number(),
            ConsensusEvent::Certificate1(certificate) => certificate.view_number(),
            ConsensusEvent::Certificate2(certificate) => certificate.view_number(),
            ConsensusEvent::TimeoutCertificate(simple_certificate) => {
                // Add one because we are moving to the next view so all event
                // processing is for the next view
                simple_certificate.view_number() + 1
            },
            ConsensusEvent::ViewSyncCertificate(simple_certificate) => {
                simple_certificate.view_number()
            },
            ConsensusEvent::BlockReconstructed(view_number, _) => *view_number,
            ConsensusEvent::StateVerified(state_response) => state_response.view,
            ConsensusEvent::HeaderCreated(view_number, _) => *view_number,
            ConsensusEvent::StateVerificationFailed(state_request) => state_request.view,
            ConsensusEvent::Timeout(view_number) => *view_number,
            ConsensusEvent::BlockBuilt(view_number, ..) => *view_number,
            ConsensusEvent::VidDisperseCreated(view_number, _) => *view_number,
            ConsensusEvent::Shutdown => ViewNumber::genesis(),
        }
    }
}

impl<TYPES: NodeType> TryFrom<Update<TYPES>> for ConsensusEvent<TYPES> {
    type Error = ();

    fn try_from(update: Update<TYPES>) -> Result<Self, ()> {
        match update {
            Update::MessageReceived(msg) => match msg {
                ConsensusMessage::Proposal(proposal_msg) => {
                    Ok(ConsensusEvent::Proposal(proposal_msg))
                },
                ConsensusMessage::Certificate1(cert, _key) => {
                    Ok(ConsensusEvent::Certificate1(cert))
                },
                ConsensusMessage::Certificate2(cert, _key) => {
                    Ok(ConsensusEvent::Certificate2(cert))
                },
                _ => Err(()),
            },
            Update::BlockReconstructed(view, _payload, vid_commit) => {
                Ok(ConsensusEvent::BlockReconstructed(view, vid_commit))
            },
            Update::Timeout(view) => Ok(ConsensusEvent::Timeout(view)),
            Update::TimeoutCertificateReceived(cert) => {
                Ok(ConsensusEvent::TimeoutCertificate(cert))
            },
            Update::ViewSyncCertificateReceived(cert) => {
                Ok(ConsensusEvent::ViewSyncCertificate(cert))
            },
            Update::StateVerified(request) => {
                let commitment = proposal_commitment(&request.proposal);
                let state = TYPES::ValidatedState::from_header(&request.proposal.block_header);
                Ok(ConsensusEvent::StateVerified(StateResponse {
                    view: request.view,
                    commitment,
                    state: Arc::new(state),
                }))
            },
            Update::StateVerificationFailed(request) => {
                let commitment = proposal_commitment(&request.proposal);
                let state = TYPES::ValidatedState::from_header(&request.proposal.block_header);
                Ok(ConsensusEvent::StateVerificationFailed(StateResponse {
                    view: request.view,
                    commitment,
                    state: Arc::new(state),
                }))
            },
            Update::HeaderCreated(view, header) => Ok(ConsensusEvent::HeaderCreated(view, header)),
            _ => Err(()),
        }
    }
}

#[allow(clippy::large_enum_variant)]
pub enum NetworkEvent<TYPES: NodeType> {
    SendMessage(ConsensusMessage<TYPES>),
    ViewChanged(ViewNumber, EpochNumber),
}

#[allow(clippy::large_enum_variant)]
pub enum IOEvent<TYPES: NodeType> {
    StorageEvent(StorageEvent<TYPES>),
    NetworkEvent(NetworkEvent<TYPES>),
}

#[allow(clippy::large_enum_variant)]
pub enum StorageEvent<TYPES: NodeType> {
    StoreProposal(QuorumProposal2<TYPES>),
    StoreCertificate1(Certificate1<TYPES>),
    StoreCertificate2(Certificate2<TYPES>),
    StoreBlock(TYPES::BlockPayload),
    StoreShares(VidDisperse2<TYPES>),
}

#[allow(clippy::large_enum_variant)]
pub enum StateEvent<TYPES: NodeType> {
    RequestState(StateRequest<TYPES>),
    RequestHeader(HeaderRequest<TYPES>),
    UpdateState(TYPES::ValidatedState, ViewNumber, Leaf2<TYPES>),
}

impl<TYPES: NodeType> HasViewNumber for StateEvent<TYPES> {
    fn view_number(&self) -> ViewNumber {
        match self {
            StateEvent::RequestState(request) => request.view,
            StateEvent::RequestHeader(request) => request.view,
            StateEvent::UpdateState(_, view, _) => *view,
        }
    }
}

#[allow(clippy::large_enum_variant)]
pub enum CpuEvent<TYPES: NodeType> {
    DrbRequest(DrbInput),
    VidShare(VidDisperse2<TYPES>),
    Vote1(Vote1<TYPES>),
    Vote2(Vote2<TYPES>),
}
