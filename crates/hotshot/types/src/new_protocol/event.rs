use crate::{
    data::{Leaf2, VidDisperseShare2, ViewNumber},
    event::Event,
    message::Proposal as SignedProposal,
    new_protocol::Proposal,
    simple_certificate::{SimpleCertificate, SuccessThreshold},
    simple_vote::{QuorumData2, Vote2Data},
    traits::node_implementation::NodeType,
};

/// Decided leaves plus the certificates and VID shares needed by the node adapter
/// to persist and broadcast them. Emitted by the coordinator.
#[derive(Clone, Debug)]
pub struct NewDecideEvent<TYPES: NodeType> {
    pub leaves: Vec<Leaf2<TYPES>>,
    /// Certificate1 (QC) that certifies the most recent (first) leaf in the chain.
    /// Each older leaf's cert1 is the next leaf's `justify_qc`.
    pub cert1: SimpleCertificate<TYPES, QuorumData2<TYPES>, SuccessThreshold>,
    pub cert2: Option<SimpleCertificate<TYPES, Vote2Data<TYPES>, SuccessThreshold>>,
    pub vid_shares: Vec<Option<SignedProposal<TYPES, VidDisperseShare2<TYPES>>>>,
}

/// High-level event emitted by the coordinator adapter. Covers both legacy HotShot
/// events and new-protocol coordinator events.
#[derive(Clone, Debug)]
pub enum CoordinatorEvent<TYPES: NodeType> {
    LegacyEvent(Event<TYPES>),
    NewDecide(NewDecideEvent<TYPES>),
    ViewChanged {
        view_number: ViewNumber,
    },
    QuorumProposal {
        proposal: SignedProposal<TYPES, Proposal<TYPES>>,
        sender: TYPES::SignatureKey,
    },
    ExternalMessageReceived {
        sender: TYPES::SignatureKey,
        data: Vec<u8>,
    },
}

impl<TYPES: NodeType> std::fmt::Display for CoordinatorEvent<TYPES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LegacyEvent(event) => {
                write!(f, "Legacy: {} view={}", event.event, event.view_number)
            },
            Self::NewDecide(event) => {
                let view = event
                    .leaves
                    .first()
                    .map(|leaf| *leaf.view_number())
                    .unwrap_or_default();
                write!(f, "NewDecide: view={view}")
            },
            Self::ViewChanged { view_number } => {
                write!(f, "ViewChanged: view={view_number}")
            },
            Self::QuorumProposal { proposal, .. } => {
                write!(
                    f,
                    "QuorumProposal: view={} epoch={}",
                    proposal.data.view_number, proposal.data.epoch
                )
            },
            Self::ExternalMessageReceived { .. } => {
                write!(f, "ExternalMessageReceived")
            },
        }
    }
}
