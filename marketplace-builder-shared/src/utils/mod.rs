use std::hash::Hash;

use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{DaProposal2, QuorumProposalWrapper},
    traits::{
        block_contents::BlockHeader, node_implementation::NodeType,
        signature_key::BuilderSignatureKey,
    },
    utils::BuilderCommitment,
};

pub mod rotating_set;
pub use rotating_set::RotatingSet;

pub mod event_service_wrapper;
pub use event_service_wrapper::EventServiceStream;

/// A convenience type alias for a tuple of builder keys
/// `(public_key, private_key)`
pub type BuilderKeys<Types> = (
    <Types as NodeType>::BuilderSignatureKey,
    <<Types as NodeType>::BuilderSignatureKey as BuilderSignatureKey>::BuilderPrivateKey,
);

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ProposalId<Types>
where
    Types: NodeType,
{
    view_number: Types::View,
    builder_commitment: BuilderCommitment,
}

impl<Types> ProposalId<Types>
where
    Types: NodeType,
{
    pub fn from_quorum_proposal(proposal: &QuorumProposalWrapper<Types>) -> Self {
        Self {
            builder_commitment: proposal.block_header().builder_commitment(),
            view_number: proposal.view_number(),
        }
    }

    pub fn from_da_proposal(proposal: &DaProposal2<Types>) -> Self {
        let builder_commitment = <Types::BlockPayload as BlockPayload<Types>>::from_bytes(
            &proposal.encoded_transactions,
            &proposal.metadata,
        )
        .builder_commitment(&proposal.metadata);

        Self {
            builder_commitment,
            view_number: proposal.view_number,
        }
    }

    pub fn view_number(&self) -> <Types as NodeType>::View {
        self.view_number
    }

    pub fn builder_commitment(&self) -> &BuilderCommitment {
        &self.builder_commitment
    }
}
