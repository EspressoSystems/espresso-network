use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::Serialize;

#[derive(Debug, Clone)]
pub enum BlockId {
    Height(u64),
    Hash(String),
    PayloadHash(String),
}

/// Espresso's extensions to the availability API.
///
/// The base availability surface (leaves, headers, blocks, payloads, VID common, transactions,
/// limits, cert2 and their streams) is served by `hotshot_query_service::availability::router`,
/// which the binary mounts alongside these routes; see [`crate::create_router_v1`].
#[async_trait]
pub trait AvailabilityApiExtension {
    type NamespaceProofQueryData: Serialize + Send + Sync + 'static;

    type IncorrectEncodingProof: Serialize + Send + Sync;

    type StateCertQueryDataV1: Serialize + Send + Sync;

    type StateCertQueryDataV2: Serialize + Send + Sync;

    async fn get_namespace_proof(
        &self,
        block_id: BlockId,
        namespace: u32,
    ) -> anyhow::Result<Self::NamespaceProofQueryData>;

    async fn get_namespace_proof_range(
        &self,
        from: u64,
        until: u64,
        namespace: u32,
    ) -> anyhow::Result<Vec<Self::NamespaceProofQueryData>>;

    async fn stream_namespace_proofs(
        &self,
        from: usize,
        namespace: u32,
    ) -> anyhow::Result<BoxStream<'static, Self::NamespaceProofQueryData>>;

    async fn get_incorrect_encoding_proof(
        &self,
        block_id: BlockId,
        namespace: u32,
    ) -> anyhow::Result<Self::IncorrectEncodingProof>;

    async fn get_state_cert(&self, epoch: u64) -> anyhow::Result<Self::StateCertQueryDataV1>;

    async fn get_state_cert_v2(&self, epoch: u64) -> anyhow::Result<Self::StateCertQueryDataV2>;
}
