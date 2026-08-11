// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::sync::Arc;

use async_trait::async_trait;
use committable::Commitment;
use hotshot_types::{
    data::VidCommitment,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
    utils::BuilderCommitment,
};

use super::{
    block_info::{AvailableBlockData, AvailableBlockHeaderInputV1, AvailableBlockInfo},
    builder::{BuildError, TransactionStatus},
};

#[async_trait]
pub trait BuilderDataSource<TYPES: NodeType> {
    /// To get the list of available blocks
    async fn available_blocks(
        &self,
        for_parent: &VidCommitment,
        view_number: u64,
        sender: TYPES::SignatureKey,
        signature: &<TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<Vec<AvailableBlockInfo<TYPES>>, BuildError>;

    /// To claim a block from the list of provided available blocks
    async fn claim_block(
        &self,
        block_hash: &BuilderCommitment,
        view_number: u64,
        sender: TYPES::SignatureKey,
        signature: &<TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<AvailableBlockData<TYPES>, BuildError>;

    /// To claim a block from the list of provided available blocks and provide the number of nodes
    /// information to the builder for VID computation.
    async fn claim_block_with_num_nodes(
        &self,
        block_hash: &BuilderCommitment,
        view_number: u64,
        sender: TYPES::SignatureKey,
        signature: &<TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
        num_nodes: usize,
    ) -> Result<AvailableBlockData<TYPES>, BuildError>;

    /// To claim a block header input
    async fn claim_block_header_input(
        &self,
        block_hash: &BuilderCommitment,
        view_number: u64,
        sender: TYPES::SignatureKey,
        signature: &<TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<AvailableBlockHeaderInputV1<TYPES>, BuildError>;

    /// To get the builder's address
    async fn builder_address(&self) -> Result<TYPES::BuilderSignatureKey, BuildError>;
}

/// Forwarding impl so a source that is not `Clone` can serve the axum routers (router state must
/// be `Clone`) behind an `Arc`.
#[async_trait]
impl<TYPES: NodeType, T: BuilderDataSource<TYPES> + Send + Sync> BuilderDataSource<TYPES>
    for Arc<T>
{
    async fn available_blocks(
        &self,
        for_parent: &VidCommitment,
        view_number: u64,
        sender: TYPES::SignatureKey,
        signature: &<TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<Vec<AvailableBlockInfo<TYPES>>, BuildError> {
        (**self)
            .available_blocks(for_parent, view_number, sender, signature)
            .await
    }

    async fn claim_block(
        &self,
        block_hash: &BuilderCommitment,
        view_number: u64,
        sender: TYPES::SignatureKey,
        signature: &<TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<AvailableBlockData<TYPES>, BuildError> {
        (**self)
            .claim_block(block_hash, view_number, sender, signature)
            .await
    }

    async fn claim_block_with_num_nodes(
        &self,
        block_hash: &BuilderCommitment,
        view_number: u64,
        sender: TYPES::SignatureKey,
        signature: &<TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
        num_nodes: usize,
    ) -> Result<AvailableBlockData<TYPES>, BuildError> {
        (**self)
            .claim_block_with_num_nodes(block_hash, view_number, sender, signature, num_nodes)
            .await
    }

    async fn claim_block_header_input(
        &self,
        block_hash: &BuilderCommitment,
        view_number: u64,
        sender: TYPES::SignatureKey,
        signature: &<TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<AvailableBlockHeaderInputV1<TYPES>, BuildError> {
        (**self)
            .claim_block_header_input(block_hash, view_number, sender, signature)
            .await
    }

    async fn builder_address(&self) -> Result<TYPES::BuilderSignatureKey, BuildError> {
        (**self).builder_address().await
    }
}

#[async_trait]
pub trait AcceptsTxnSubmits<I>
where
    I: NodeType,
{
    async fn submit_txns(
        &self,
        txns: Vec<<I as NodeType>::Transaction>,
    ) -> Result<Vec<Commitment<<I as NodeType>::Transaction>>, BuildError>;

    async fn txn_status(
        &self,
        txn_hash: Commitment<<I as NodeType>::Transaction>,
    ) -> Result<TransactionStatus, BuildError>;
}

/// See the [`BuilderDataSource`] impl for [`Arc`].
#[async_trait]
impl<I: NodeType, T: AcceptsTxnSubmits<I> + Send + Sync> AcceptsTxnSubmits<I> for Arc<T> {
    async fn submit_txns(
        &self,
        txns: Vec<<I as NodeType>::Transaction>,
    ) -> Result<Vec<Commitment<<I as NodeType>::Transaction>>, BuildError> {
        (**self).submit_txns(txns).await
    }

    async fn txn_status(
        &self,
        txn_hash: Commitment<<I as NodeType>::Transaction>,
    ) -> Result<TransactionStatus, BuildError> {
        (**self).txn_status(txn_hash).await
    }
}
