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
    message::{
        Certificate1, Certificate2, ConsensusMessage, ProposalMessage, Vote1, Vote2,
    },
};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct StateRequest<T: NodeType> {
    pub view: ViewNumber,
    pub parent_view: ViewNumber,
    pub epoch: EpochNumber,
    pub block_number: u64,
    pub proposal: QuorumProposal2<T>,
    pub parent_commitment: Commitment<Leaf2<T>>,
    pub payload_size: u32,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct StateResponse<T: NodeType> {
    pub view: ViewNumber,
    pub commitment: Commitment<Leaf2<T>>,
    pub state: Arc<T::ValidatedState>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct BlockAndHeaderRequest<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub parent_proposal: QuorumProposal2<T>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct BlockRequest<T: NodeType> {
    pub view: ViewNumber,
    pub parent_proposal: QuorumProposal2<T>,
    pub epoch: EpochNumber,
}

#[derive(Eq, PartialEq, Debug)]
pub struct HeaderRequest<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub parent_proposal: QuorumProposal2<T>,
    pub payload_commitment: VidCommitment,
    pub builder_commitment: BuilderCommitment,
    pub metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
    pub builder_fee: BuilderFee<T>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct VidDisperseRequest<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub block: T::BlockPayload,
    pub metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct VidShareInput<T: NodeType> {
    pub share: VidDisperseShare2<T>,
    pub metadata: Option<<T::BlockPayload as BlockPayload<T>>::Metadata>,
}

#[derive(Eq, PartialEq, Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Action<T: NodeType> {
    SendProposal(Proposal<T, QuorumProposal2<T>>, VidDisperse2<T>),
    SendVote1(Vote1<T>),
    SendVote2(Vote2<T>),
    SendTransactions(ViewNumber, Vec<T::Transaction>),
    SendDedupManifest(ViewNumber, Vec<Commitment<T::Transaction>>),
    RequestState(StateRequest<T>),
    RequestBlockAndHeader(BlockAndHeaderRequest<T>),
    RequestVidDisperse(
        ViewNumber,
        EpochNumber,
        T::BlockPayload,
        <T::BlockPayload as BlockPayload<T>>::Metadata,
    ),
    RequestProposal(ViewNumber, Commitment<Leaf2<T>>),
    RequestDRB(DrbInput),
}

#[derive(Clone, Eq, PartialEq, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Event<T: NodeType> {
    MessageReceived(ConsensusMessage<T>),
    StateValidated(StateRequest<T>),
    HeaderCreated(ViewNumber, T::BlockHeader),
    StateValidationFailed(StateRequest<T>),
    HeaderCreationFailed(BlockAndHeaderRequest<T>),
    VidDisperseCreated(VidCommitment2, VidDisperse2<T>),
    LeafDecided(Vec<Leaf2<T>>),
    DrbCalculated(DrbResult),
    LockUpdated(Certificate2<T>),
    ViewChanged(ViewNumber, EpochNumber),
    BlockReconstructed(ViewNumber, T::BlockPayload, VidCommitment2),
    Certificate1Formed(Certificate1<T>),
    Certificate2Formed(Certificate2<T>),
    Timeout(ViewNumber),
    TimeoutCertificateReceived(TimeoutCertificate2<T>),
    ViewSyncCertificateReceived(ViewSyncFinalizeCertificate2<T>),
}

#[allow(clippy::large_enum_variant)]
#[derive(Eq, PartialEq, Debug, Clone)]
pub enum ConsensusOutput<T: NodeType> {
    Action(Action<T>),
    Event(Event<T>),
}

impl<T: NodeType> Action<T> {
    pub fn view_number(&self) -> Option<ViewNumber> {
        match self {
            Action::SendProposal(proposal, _) => Some(proposal.data.view_number()),
            Action::SendVote1(vote) => Some(vote.view_number()),
            Action::SendVote2(vote) => Some(vote.view_number()),
            Action::RequestState(req) => Some(req.view),
            Action::RequestBlockAndHeader(req) => Some(req.view),
            Action::RequestVidDisperse(view, ..) => Some(*view),
            Action::RequestProposal(view, _) => Some(*view),
            Action::SendTransactions(view, _) => Some(*view),
            Action::SendDedupManifest(view, _) => Some(*view),
            Action::RequestDRB(_) => None,
        }
    }
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
pub enum ConsensusInput<T: NodeType> {
    Proposal(ProposalMessage<T>),
    Certificate1(Certificate1<T>),
    Certificate2(Certificate2<T>),
    TimeoutCertificate(TimeoutCertificate2<T>),
    ViewSyncCertificate(ViewSyncFinalizeCertificate2<T>),
    BlockReconstructed(ViewNumber, VidCommitment2),
    BlockBuilt(
        ViewNumber,
        EpochNumber,
        T::BlockPayload,
        <T::BlockPayload as BlockPayload<T>>::Metadata,
    ),
    VidDisperseCreated(ViewNumber, VidDisperse2<T>),
    StateValidated(StateResponse<T>),
    HeaderCreated(ViewNumber, T::BlockHeader),
    StateValidationFailed(StateResponse<T>),
    Timeout(ViewNumber),
    // TODO: Add checkpoint events
}

impl<T: NodeType> ConsensusInput<T> {
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
            ConsensusInput::StateValidated(state_response) => state_response.view,
            ConsensusInput::HeaderCreated(view_number, _) => *view_number,
            ConsensusInput::StateValidationFailed(state_request) => state_request.view,
            ConsensusInput::Timeout(view_number) => *view_number,
            ConsensusInput::BlockBuilt(view_number, ..) => *view_number,
            ConsensusInput::VidDisperseCreated(view_number, _) => *view_number,
        }
    }
}

impl<T: NodeType> TryFrom<Event<T>> for ConsensusInput<T> {
    type Error = ();

    fn try_from(update: Event<T>) -> Result<Self, ()> {
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
            Event::StateValidated(request) => {
                let commitment = proposal_commitment(&request.proposal);
                let state = T::ValidatedState::from_header(&request.proposal.block_header);
                Ok(ConsensusInput::StateValidated(StateResponse {
                    view: request.view,
                    commitment,
                    state: Arc::new(state),
                }))
            },
            Event::StateValidationFailed(request) => {
                let commitment = proposal_commitment(&request.proposal);
                let state = T::ValidatedState::from_header(&request.proposal.block_header);
                Ok(ConsensusInput::StateValidationFailed(StateResponse {
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
            Event::HeaderCreationFailed(_)
            | Event::LeafDecided(_)
            | Event::LockUpdated(_)
            | Event::ViewChanged(..)
            | Event::DrbCalculated(_) => Err(()),
        }
    }
}

#[allow(clippy::large_enum_variant)]
pub enum StorageEvent<T: NodeType> {
    StoreProposal(QuorumProposal2<T>),
    StoreCertificate1(Certificate1<T>),
    StoreCertificate2(Certificate2<T>),
    StoreBlock(T::BlockPayload),
    StoreShares(VidDisperse2<T>),
}

#[allow(clippy::large_enum_variant)]
pub enum StateEvent<T: NodeType> {
    RequestState(StateRequest<T>),
    RequestHeader(HeaderRequest<T>),
    UpdateState(T::ValidatedState, ViewNumber, Leaf2<T>),
}

impl<T: NodeType> HasViewNumber for StateEvent<T> {
    fn view_number(&self) -> ViewNumber {
        match self {
            StateEvent::RequestState(request) => request.view,
            StateEvent::RequestHeader(request) => request.view,
            StateEvent::UpdateState(_, view, _) => *view,
        }
    }
}
