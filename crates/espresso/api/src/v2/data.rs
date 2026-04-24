use async_trait::async_trait;
use serialization_api::ApiSerializations;

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
}
