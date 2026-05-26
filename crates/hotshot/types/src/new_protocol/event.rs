use crate::{
    event::{Event, LeafInfo},
    message::Proposal as SignedProposal,
    new_protocol::Proposal,
    simple_certificate::{SimpleCertificate, SuccessThreshold},
    simple_vote::{QuorumData2, Vote2Data},
    traits::node_implementation::NodeType,
};

/// High-level event emitted by the coordinator adapter. Covers both legacy HotShot
/// events and new-protocol coordinator events.
#[derive(Clone, Debug)]
pub enum CoordinatorEvent<TYPES: NodeType> {
    LegacyEvent(Event<TYPES>),
    NewDecide {
        leaf_infos: Vec<LeafInfo<TYPES>>,
        /// Certificate1 that certifies the most recent (first) leaf in the chain.
        /// Each older leaf's cert1 is the next leaf's `justify_qc`.
        cert1: SimpleCertificate<TYPES, QuorumData2<TYPES>, SuccessThreshold>,
        /// Cert2 which finalizes the most recent leaf in the chain
        cert2: Option<SimpleCertificate<TYPES, Vote2Data<TYPES>, SuccessThreshold>>,
    },
    QuorumProposal {
        proposal: SignedProposal<TYPES, Proposal<TYPES>>,
        sender: TYPES::SignatureKey,
    },
    ExternalMessageReceived {
        sender: TYPES::SignatureKey,
        data: Vec<u8>,
    },
    /// Emitted when a node has reconstructed a block payload from VID shares.
    /// Lets downstream consumers (e.g. query service) fill in a payload that
    /// was missing when the corresponding view was decided.
    BlockPayloadReconstructed {
        view: ViewNumber,
        header: TYPES::BlockHeader,
        payload: TYPES::BlockPayload,
    },
}

impl<TYPES: NodeType> std::fmt::Display for CoordinatorEvent<TYPES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LegacyEvent(event) => {
                write!(f, "Legacy: {} view={}", event.event, event.view_number)
            },
            Self::NewDecide { leaf_infos, .. } => {
                let view = leaf_infos
                    .first()
                    .map(|info| *info.leaf.view_number())
                    .unwrap_or_default();
                write!(f, "NewDecide: view={view}")
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
            Self::BlockPayloadReconstructed { view, .. } => {
                write!(f, "BlockPayloadReconstructed: view={view}")
            },
        }
    }
}
