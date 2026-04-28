use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait BlockStateApi {
    type BlockMerklePath: Serialize + Send + Sync;

    async fn get_block_merkle_path(
        &self,
        height: u64,
        key: u64,
    ) -> anyhow::Result<Self::BlockMerklePath>;

    async fn get_block_merkle_path_by_commit(
        &self,
        commit: String,
        key: u64,
    ) -> anyhow::Result<Self::BlockMerklePath>;

    async fn get_block_merkle_height(&self) -> anyhow::Result<usize>;
}
