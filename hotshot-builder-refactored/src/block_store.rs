use std::marker::PhantomData;

use hotshot::traits::BlockPayload;
use hotshot_builder_api::v0_1::block_info::AvailableBlockInfo;
use hotshot_types::traits::{node_implementation::NodeType, signature_key::BuilderSignatureKey};
use builder_shared::{
    block::{BlockId, BuilderStateId},
    coordinator::tiered_view_map::TieredViewMap,
    error::Error,
    utils::BuilderKeys,
};

// It holds all the necessary information for a block
#[derive(Debug, Clone)]
pub struct BlockInfo<Types: NodeType> {
    pub block_payload: Types::BlockPayload,
    pub metadata: <<Types as NodeType>::BlockPayload as BlockPayload<Types>>::Metadata,
    pub block_size: u64,
    pub offered_fee: u64,
    // Could we have included more transactions with this block, but chose not to?
    pub truncated: bool,
}

impl<Types: NodeType> BlockInfo<Types> {
    pub fn signed_response(
        &self,
        keys: &BuilderKeys<Types>,
    ) -> Result<AvailableBlockInfo<Types>, Error<Types>> {
        let block_hash = self.block_payload.builder_commitment(&self.metadata);
        let signature = <Types as NodeType>::BuilderSignatureKey::sign_block_info(
            &keys.1,
            self.block_size,
            self.offered_fee,
            &block_hash,
        )
        .map_err(Error::Signing)?;
        Ok(AvailableBlockInfo {
            block_hash,
            block_size: self.block_size,
            offered_fee: self.offered_fee,
            signature,
            sender: keys.0.clone(),
            _phantom: PhantomData,
        })
    }
}

#[derive(Default)]
pub struct BlockStore<Types: NodeType> {
    pub(crate) blocks: TieredViewMap<BlockId<Types>, BlockInfo<Types>>,
    pub(crate) block_cache: TieredViewMap<BuilderStateId<Types>, BlockId<Types>>,
}

impl<Types: NodeType> BlockStore<Types> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(
        &mut self,
        built_by: BuilderStateId<Types>,
        block_id: BlockId<Types>,
        block_info: BlockInfo<Types>,
    ) {
        self.blocks.insert(block_id.clone(), block_info);
        self.block_cache.insert(built_by, block_id);
    }

    pub fn get_cached(&self, builder_id: &BuilderStateId<Types>) -> Option<&BlockInfo<Types>> {
        let block_id = self.block_cache.get(builder_id)?;
        self.blocks.get(block_id)
    }

    pub fn get_block(&self, block_id: &BlockId<Types>) -> Option<&BlockInfo<Types>> {
        self.blocks.get(block_id)
    }

    pub fn prune(&mut self, cutoff: Types::View) {
        self.blocks.prune(cutoff);
        self.block_cache.prune(cutoff);
    }
}
