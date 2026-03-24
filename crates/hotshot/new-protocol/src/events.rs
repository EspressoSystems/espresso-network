use std::sync::Arc;

use committable::Commitment;
use hotshot::traits::{BlockPayload, ValidatedState};
use hotshot_types::{
    data::{
        EpochNumber, Leaf2, QuorumProposal2, VidCommitment, VidCommitment2, VidDisperse2,
        VidDisperseShare2, ViewNumber,
    },
    drb::{DrbInput, DrbResult},
    message::Proposal,
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
pub struct StateRequest<TYPES: NodeType> {
    pub view: ViewNumber,
    pub parent_view: ViewNumber,
    pub epoch: EpochNumber,
    pub block_number: u64,
    pub proposal: QuorumProposal2<TYPES>,
    pub parent_commitment: Commitment<Leaf2<TYPES>>,
    pub payload_size: u32,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct StateResponse<TYPES: NodeType> {
    pub view: ViewNumber,
    pub commitment: Commitment<Leaf2<TYPES>>,
    pub state: Arc<TYPES::ValidatedState>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct BlockAndHeaderRequest<TYPES: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub parent_proposal: QuorumProposal2<TYPES>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct BlockRequest<TYPES: NodeType> {
    pub view: ViewNumber,
    pub parent_proposal: QuorumProposal2<TYPES>,
    pub epoch: EpochNumber,
}

#[derive(Eq, PartialEq, Debug)]
pub struct HeaderRequest<TYPES: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub parent_proposal: QuorumProposal2<TYPES>,
    pub payload_commitment: VidCommitment,
    pub builder_commitment: BuilderCommitment,
    pub metadata: <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    pub builder_fee: BuilderFee<TYPES>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct VidDisperseRequest<TYPES: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub block: TYPES::BlockPayload,
    pub metadata: <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct VidShareInput<TYPES: NodeType> {
    pub share: VidDisperseShare2<TYPES>,
    pub metadata: Option<<TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata>,
}

#[derive(Eq, PartialEq, Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Action<TYPES: NodeType> {
    SendProposal(Proposal<TYPES, QuorumProposal2<TYPES>>, VidDisperse2<TYPES>),
    SendVote1(Vote1<TYPES>),
    SendVote2(Vote2<TYPES>),
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
pub enum Event<TYPES: NodeType> {
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
    Certificate1Formed(Certificate1<TYPES>),
    Certificate2Formed(Certificate2<TYPES>),
    Timeout(ViewNumber),
    TimeoutCertificateReceived(TimeoutCertificate2<TYPES>),
    ViewSyncCertificateReceived(ViewSyncFinalizeCertificate2<TYPES>),
}

#[allow(clippy::large_enum_variant)]
#[derive(Eq, PartialEq, Debug, Clone)]
pub enum ConsensusOutput<TYPES: NodeType> {
    Action(Action<TYPES>),
    Event(Event<TYPES>),
}

impl<T: NodeType> From<Action<T>> for ConsensusOutput<T> {
    fn from(a: Action<T>) -> Self {
        Self::Action(a)
    }
}

impl<T: NodeType> From<Event<T>> for ConsensusOutput<T> {
    fn from(e: Event<T>) -> Self {
        Self::Event(e)
    }
}
#[allow(clippy::large_enum_variant)]
#[derive(Eq, PartialEq, Debug, Clone)]
pub enum ConsensusInput<TYPES: NodeType> {
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
    // TODO: Add checkpoint events
}

impl<TYPES: NodeType> ConsensusInput<TYPES> {
    pub fn view_number(&self) -> ViewNumber {
        match self {
            ConsensusInput::Proposal(proposal) => proposal.view_number(),
            ConsensusInput::Certificate1(certificate) => certificate.view_number(),
            ConsensusInput::Certificate2(certificate) => certificate.view_number(),
            ConsensusInput::TimeoutCertificate(simple_certificate) => {
                // Add one because we are moving to the next view so all event
                // processing is for the next view
                simple_certificate.view_number() + 1
            },
            ConsensusInput::ViewSyncCertificate(simple_certificate) => {
                simple_certificate.view_number()
            },
            ConsensusInput::BlockReconstructed(view_number, _) => *view_number,
            ConsensusInput::StateVerified(state_response) => state_response.view,
            ConsensusInput::HeaderCreated(view_number, _) => *view_number,
            ConsensusInput::StateVerificationFailed(state_request) => state_request.view,
            ConsensusInput::Timeout(view_number) => *view_number,
            ConsensusInput::BlockBuilt(view_number, ..) => *view_number,
            ConsensusInput::VidDisperseCreated(view_number, _) => *view_number,
        }
    }
}

impl<TYPES: NodeType> TryFrom<Event<TYPES>> for ConsensusInput<TYPES> {
    type Error = ();

    fn try_from(update: Event<TYPES>) -> Result<Self, ()> {
        match update {
            Event::MessageReceived(msg) => match msg {
                ConsensusMessage::Proposal(proposal_msg) => {
                    Ok(ConsensusInput::Proposal(proposal_msg))
                },
                ConsensusMessage::Certificate1(cert, _key) => {
                    Ok(ConsensusInput::Certificate1(cert))
                },
                ConsensusMessage::Certificate2(cert, _key) => {
                    Ok(ConsensusInput::Certificate2(cert))
                },
                _ => Err(()),
            },
            Event::BlockReconstructed(view, _payload, vid_commit) => {
                Ok(ConsensusInput::BlockReconstructed(view, vid_commit))
            },
            Event::Timeout(view) => Ok(ConsensusInput::Timeout(view)),
            Event::TimeoutCertificateReceived(cert) => Ok(ConsensusInput::TimeoutCertificate(cert)),
            Event::ViewSyncCertificateReceived(cert) => {
                Ok(ConsensusInput::ViewSyncCertificate(cert))
            },
            Event::StateVerified(request) => {
                let commitment = proposal_commitment(&request.proposal);
                let state = TYPES::ValidatedState::from_header(&request.proposal.block_header);
                Ok(ConsensusInput::StateVerified(StateResponse {
                    view: request.view,
                    commitment,
                    state: Arc::new(state),
                }))
            },
            Event::StateVerificationFailed(request) => {
                let commitment = proposal_commitment(&request.proposal);
                let state = TYPES::ValidatedState::from_header(&request.proposal.block_header);
                Ok(ConsensusInput::StateVerificationFailed(StateResponse {
                    view: request.view,
                    commitment,
                    state: Arc::new(state),
                }))
            },
            Event::HeaderCreated(view, header) => Ok(ConsensusInput::HeaderCreated(view, header)),
            Event::Certificate1Formed(cert) => Ok(ConsensusInput::Certificate1(cert)),
            Event::Certificate2Formed(cert) => Ok(ConsensusInput::Certificate2(cert)),
            Event::VidDisperseCreated(_commitment, disperse) => Ok(
                ConsensusInput::VidDisperseCreated(disperse.view_number, disperse),
            ),
            _ => Err(()),
        }
    }
}

impl<TYPES: NodeType> TryFrom<Event<TYPES>> for CpuEvent<TYPES> {
    type Error = ();

    fn try_from(event: Event<TYPES>) -> Result<Self, ()> {
        match event {
            Event::MessageReceived(msg) => match msg {
                ConsensusMessage::Proposal(proposal) => Ok(CpuEvent::Proposal(proposal)),
                ConsensusMessage::Vote1(vote) => Ok(CpuEvent::Vote1(vote)),
                ConsensusMessage::Vote2(vote) => Ok(CpuEvent::Vote2(vote)),
                _ => Err(()),
            },
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
pub enum IoEvent<TYPES: NodeType> {
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
    Proposal(ProposalMessage<TYPES>),
    VidDisperseRequest(VidDisperseRequest<TYPES>),
    Vote1(Vote1<TYPES>),
    Vote2(Vote2<TYPES>),
}
