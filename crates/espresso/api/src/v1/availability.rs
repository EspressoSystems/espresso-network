use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::Serialize;

#[derive(Debug, Clone)]
pub enum BlockId {
    Height(u64),
    Hash(String),
    PayloadHash(String),
}

#[derive(Debug, Clone)]
pub enum LeafId {
    Height(u64),
    Hash(String),
}

#[derive(Debug, Clone)]
pub enum PayloadId {
    Height(u64),
    Hash(String),
    BlockHash(String),
}

#[async_trait]
pub trait AvailabilityApi {
    type NamespaceProofQueryData: Serialize + Send + Sync + 'static;

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
    ) -> anyhow::Result<BoxStream<'static, Self::NamespaceProofQueryData>>;

    async fn get_incorrect_encoding_proof(
        &self,
        block_id: BlockId,
        namespace: u32,
    ) -> anyhow::Result<Self::IncorrectEncodingProof>;

    async fn get_state_cert(&self, epoch: u64) -> anyhow::Result<Self::StateCertQueryDataV1>;

    async fn get_state_cert_v2(&self, epoch: u64) -> anyhow::Result<Self::StateCertQueryDataV2>;
}

/// HotShot core availability API — mirrors the hotshot-query-service availability endpoints.
///
/// Each method corresponds to a tide-disco route exposed by the hotshot-query-service, copied
/// verbatim to axum with no path or output changes.
#[async_trait]
pub trait HotShotAvailabilityApi {
    type Leaf: Serialize + Send + Sync + 'static;
    type Block: Serialize + Send + Sync + 'static;
    type Header: Serialize + Send + Sync + 'static;
    type Payload: Serialize + Send + Sync + 'static;
    type VidCommon: Serialize + Send + Sync + 'static;
    type Transaction: Serialize + Send + Sync + 'static;
    type TransactionWithProof: Serialize + Send + Sync + 'static;
    type BlockSummary: Serialize + Send + Sync + 'static;
    type Limits: Serialize + Send + Sync + 'static;
    type Cert2: Serialize + Send + Sync + 'static;

    async fn get_leaf(&self, id: LeafId) -> anyhow::Result<Self::Leaf>;
    async fn get_leaf_range(&self, from: usize, until: usize) -> anyhow::Result<Vec<Self::Leaf>>;

    async fn get_header(&self, id: BlockId) -> anyhow::Result<Self::Header>;
    async fn get_header_range(
        &self,
        from: usize,
        until: usize,
    ) -> anyhow::Result<Vec<Self::Header>>;

    async fn get_block(&self, id: BlockId) -> anyhow::Result<Self::Block>;
    async fn get_block_range(&self, from: usize, until: usize) -> anyhow::Result<Vec<Self::Block>>;

    async fn get_payload(&self, id: PayloadId) -> anyhow::Result<Self::Payload>;
    async fn get_payload_range(
        &self,
        from: usize,
        until: usize,
    ) -> anyhow::Result<Vec<Self::Payload>>;

    async fn get_vid_common(&self, id: BlockId) -> anyhow::Result<Self::VidCommon>;
    async fn get_vid_common_range(
        &self,
        from: usize,
        until: usize,
    ) -> anyhow::Result<Vec<Self::VidCommon>>;

    async fn get_transaction_by_position(
        &self,
        height: u64,
        index: u64,
    ) -> anyhow::Result<Self::Transaction>;
    async fn get_transaction_by_hash(&self, hash: String) -> anyhow::Result<Self::Transaction>;

    async fn get_transaction_proof_by_position(
        &self,
        height: u64,
        index: u64,
    ) -> anyhow::Result<Self::TransactionWithProof>;
    async fn get_transaction_proof_by_hash(
        &self,
        hash: String,
    ) -> anyhow::Result<Self::TransactionWithProof>;

    async fn get_block_summary(&self, height: usize) -> anyhow::Result<Self::BlockSummary>;
    async fn get_block_summary_range(
        &self,
        from: usize,
        until: usize,
    ) -> anyhow::Result<Vec<Self::BlockSummary>>;

    async fn get_limits(&self) -> anyhow::Result<Self::Limits>;

    async fn get_cert2(&self, height: u64) -> anyhow::Result<Option<Self::Cert2>>;

    async fn stream_leaves(&self, from: usize) -> anyhow::Result<BoxStream<'static, Self::Leaf>>;

    async fn stream_headers(&self, from: usize)
    -> anyhow::Result<BoxStream<'static, Self::Header>>;

    async fn stream_blocks(&self, from: usize) -> anyhow::Result<BoxStream<'static, Self::Block>>;

    async fn stream_payloads(
        &self,
        from: usize,
    ) -> anyhow::Result<BoxStream<'static, Self::Payload>>;

    async fn stream_vid_common(
        &self,
        from: usize,
    ) -> anyhow::Result<BoxStream<'static, Self::VidCommon>>;

    async fn stream_transactions(
        &self,
        from: usize,
        namespace: Option<u32>,
    ) -> anyhow::Result<BoxStream<'static, Self::Transaction>>;
}
