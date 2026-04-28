use async_trait::async_trait;
use serde::Serialize;

#[derive(Debug, Clone)]
pub enum BlockId {
    Height(u64),
    Hash(String),
    PayloadHash(String),
}

#[async_trait]
pub trait AvailabilityApi {
    type NamespaceProofQueryData: Serialize + Send + Sync;

    type IncorrectEncodingProof: Serialize + Send + Sync;

    type StateCertQueryDataV1: Serialize + Send + Sync;

    type StateCertQueryDataV2: Serialize + Send + Sync;

    async fn get_namespace_proof(
        &self,
        block_id: BlockId,
        namespace: u32,
    ) -> anyhow::Result<Option<Self::NamespaceProofQueryData>>;

    async fn get_namespace_proof_range(
        &self,
        from: u64,
        until: u64,
        namespace: u32,
    ) -> anyhow::Result<Vec<Self::NamespaceProofQueryData>>;

    async fn stream_namespace_proofs(
        &self,
        start_height: u64,
        namespace: u32,
    ) -> anyhow::Result<()> {
        let _ = (start_height, namespace);
        anyhow::bail!("WebSocket streaming not yet implemented")
    }

    async fn get_incorrect_encoding_proof(
        &self,
        block_id: BlockId,
        namespace: u32,
    ) -> anyhow::Result<Self::IncorrectEncodingProof>;

    async fn get_state_cert(&self, epoch: u64) -> anyhow::Result<Self::StateCertQueryDataV1>;

    async fn get_state_cert_v2(&self, epoch: u64) -> anyhow::Result<Self::StateCertQueryDataV2>;
}
