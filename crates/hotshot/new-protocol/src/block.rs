mod mempool;

use committable::Commitment;
use hotshot_types::{
    consensus::PayloadWithMetadata,
    data::{EpochNumber, VidCommitment, VidDisperse2, ViewNumber},
    traits::{block_contents::BuilderFee, node_implementation::NodeType},
    utils::BuilderCommitment,
};
pub use mempool::MempoolBuilder;

use crate::{
    consensus::ConsensusInput,
    message::{DedupManifest, Proposal, TransactionMessage},
    state::HeaderRequest,
};

#[derive(Debug, thiserror::Error)]
pub enum BlockError {
    #[error("payload construction failed: {0}")]
    PayloadConstruction(String),
    #[error("stake table unavailable")]
    StakeTableUnavailable,
    #[error("builder signature failed")]
    BuilderSignature,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct BlockAndHeaderRequest<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub parent_proposal: Proposal<T>,
}

pub struct BlockBuilderOutput<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub payload: PayloadWithMetadata<T>,
    pub parent_proposal: Proposal<T>,
    pub builder_commitment: BuilderCommitment,
    pub builder_fee: BuilderFee<T>,
    pub payload_commitment: VidCommitment,
    pub vid_disperse: VidDisperse2<T>,
    pub manifest: DedupManifest<T>,
}

/// Trait for block builders that produce blocks and VID dispersals.
pub trait BlockBuilder<T: NodeType>: Send {
    /// Start building a block for the given view/epoch.
    fn request_block(&mut self, request: BlockAndHeaderRequest<T>);

    /// Poll for the next completed block.
    fn next(
        &mut self,
    ) -> impl std::future::Future<Output = Option<Result<BlockBuilderOutput<T>, BlockError>>> + Send;

    /// Handle forwarded transactions from the network.
    fn on_transactions(&mut self, _msg: TransactionMessage<T>) {}

    /// Handle a dedup manifest from the current view's leader.
    fn on_dedup_manifest(&mut self, _manifest: DedupManifest<T>) {}

    /// Called on view change. Returns retry transactions to re-broadcast.
    fn on_view_changed(&mut self, _view: ViewNumber, _epoch: EpochNumber) -> Vec<T::Transaction> {
        vec![]
    }

    /// Notify that a block was reconstructed.
    fn on_block_reconstructed(&mut self, _tx_commitments: Vec<Commitment<T::Transaction>>) {}

    /// Garbage-collect state for views older than `view`.
    fn gc(&mut self, _view: ViewNumber) {}
}

pub struct BlockBuilderConfig {
    pub max_retry_bytes: u64,
    pub max_leader_bytes: u64,
    pub ttl: u64,
    pub dedup_window_size: u64,
}

impl Default for BlockBuilderConfig {
    fn default() -> Self {
        Self {
            max_retry_bytes: 100 * 1024 * 1024,
            max_leader_bytes: 2 * 1024 * 1024,
            ttl: 50,
            dedup_window_size: 10,
        }
    }
}

impl<T: NodeType> From<&BlockBuilderOutput<T>> for HeaderRequest<T> {
    fn from(output: &BlockBuilderOutput<T>) -> Self {
        HeaderRequest {
            view: output.view,
            epoch: output.epoch,
            parent_proposal: output.parent_proposal.clone(),
            payload_commitment: output.payload_commitment,
            builder_commitment: output.builder_commitment.clone(),
            metadata: output.payload.metadata.clone(),
            builder_fee: output.builder_fee.clone(),
        }
    }
}

impl<T: NodeType> From<BlockBuilderOutput<T>> for ConsensusInput<T> {
    fn from(output: BlockBuilderOutput<T>) -> Self {
        ConsensusInput::BlockBuilt {
            view: output.view,
            epoch: output.epoch,
            payload: output.payload.payload,
            vid_disperse: output.vid_disperse,
        }
    }
}
