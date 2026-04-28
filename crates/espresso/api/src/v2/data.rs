use async_trait::async_trait;
use serialization_api::{v2::GetBlockMerklePathRequest, ApiSerializations};

#[async_trait]
pub trait DataApi: ApiSerializations {
    async fn get_namespace_proof(
        &self,
        namespace_id: u32,
        block_height: u64,
    ) -> anyhow::Result<Self::NamespaceProof>;

    async fn get_namespace_proof_range(
        &self,
        namespace_id: u32,
        from: u64,
        until: u64,
    ) -> anyhow::Result<Vec<Self::NamespaceProof>>;

    async fn get_incorrect_encoding_proof(
        &self,
        namespace_id: u32,
        block_height: u64,
    ) -> anyhow::Result<Self::IncorrectEncodingProof>;

    async fn get_block_merkle_path(
        &self,
        request: GetBlockMerklePathRequest,
    ) -> anyhow::Result<Self::BlockMerklePath>
    where
        Self::BlockMerklePath: Sized;

    async fn get_block_merkle_height(&self) -> anyhow::Result<u64>;
}
