use std::ops::Range;

use async_trait::async_trait;
use espresso_types::{
    NamespaceProofQueryData, SeqTypes, StateCertQueryDataV1, StateCertQueryDataV2,
    v0_3::AvidMIncorrectEncodingNsProof,
};
use futures::stream::BoxStream;
use hotshot_query_service_types::{
    Header,
    availability::{
        BlockQueryData, BlockSummaryQueryData, LeafQueryData, Limits, PayloadQueryData,
        TransactionQueryData, TransactionWithProofQueryData, VidCommonQueryData,
    },
};
use hotshot_types::simple_certificate::Certificate2;

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
    async fn get_namespace_proof(
        &self,
        block_id: BlockId,
        namespace: u32,
    ) -> anyhow::Result<NamespaceProofQueryData>;

    async fn get_namespace_proof_range(
        &self,
        from: u64,
        until: u64,
        namespace: u32,
    ) -> anyhow::Result<Vec<NamespaceProofQueryData>>;

    async fn stream_namespace_proofs(
        &self,
        from: usize,
        namespace: u32,
    ) -> anyhow::Result<BoxStream<'static, NamespaceProofQueryData>>;

    async fn get_incorrect_encoding_proof(
        &self,
        block_id: BlockId,
        namespace: u32,
    ) -> anyhow::Result<AvidMIncorrectEncodingNsProof>;

    async fn get_state_cert(&self, epoch: u64) -> anyhow::Result<StateCertQueryDataV1<SeqTypes>>;

    async fn get_state_cert_v2(&self, epoch: u64)
    -> anyhow::Result<StateCertQueryDataV2<SeqTypes>>;
}

/// HotShot core availability API: mirrors the hotshot-query-service availability endpoints.
///
/// Each method corresponds to a hotshot-query-service route, with no path or output changes.
#[async_trait]
pub trait HotShotAvailabilityApi {
    async fn get_leaf(&self, id: LeafId) -> anyhow::Result<LeafQueryData<SeqTypes>>;
    async fn get_leaf_range(
        &self,
        from: usize,
        until: usize,
    ) -> anyhow::Result<Vec<LeafQueryData<SeqTypes>>>;

    async fn get_header(&self, id: BlockId) -> anyhow::Result<Header<SeqTypes>>;
    async fn get_header_range(
        &self,
        from: usize,
        until: usize,
    ) -> anyhow::Result<Vec<Header<SeqTypes>>>;

    async fn get_block(&self, id: BlockId) -> anyhow::Result<BlockQueryData<SeqTypes>>;
    async fn get_block_range(
        &self,
        from: usize,
        until: usize,
    ) -> anyhow::Result<Vec<BlockQueryData<SeqTypes>>>;

    async fn get_payload(&self, id: PayloadId) -> anyhow::Result<PayloadQueryData<SeqTypes>>;
    async fn get_payload_range(
        &self,
        from: usize,
        until: usize,
    ) -> anyhow::Result<Vec<PayloadQueryData<SeqTypes>>>;

    async fn get_vid_common(&self, id: BlockId) -> anyhow::Result<VidCommonQueryData<SeqTypes>>;
    async fn get_vid_common_range(
        &self,
        from: usize,
        until: usize,
    ) -> anyhow::Result<Vec<VidCommonQueryData<SeqTypes>>>;

    /// The objects for a set of half-open height ranges, which must be ascending and disjoint.
    ///
    /// These carry a fragmented set of heights in one request, for peers catching up. Like the
    /// range endpoints they answer in full or not at all: a height the node lacks is fetched from
    /// its own peers, and a 404 means at least one was not available in time.
    async fn get_leaf_ranges(
        &self,
        ranges: Vec<Range<u64>>,
    ) -> anyhow::Result<Vec<LeafQueryData<SeqTypes>>>;
    async fn get_block_ranges(
        &self,
        ranges: Vec<Range<u64>>,
    ) -> anyhow::Result<Vec<BlockQueryData<SeqTypes>>>;
    async fn get_vid_common_ranges(
        &self,
        ranges: Vec<Range<u64>>,
    ) -> anyhow::Result<Vec<VidCommonQueryData<SeqTypes>>>;

    async fn get_transaction_by_position(
        &self,
        height: u64,
        index: u64,
    ) -> anyhow::Result<TransactionQueryData<SeqTypes>>;
    async fn get_transaction_by_hash(
        &self,
        hash: String,
    ) -> anyhow::Result<TransactionQueryData<SeqTypes>>;

    async fn get_transaction_proof_by_position(
        &self,
        height: u64,
        index: u64,
    ) -> anyhow::Result<TransactionWithProofQueryData<SeqTypes>>;
    async fn get_transaction_proof_by_hash(
        &self,
        hash: String,
    ) -> anyhow::Result<TransactionWithProofQueryData<SeqTypes>>;

    async fn get_block_summary(
        &self,
        height: usize,
    ) -> anyhow::Result<BlockSummaryQueryData<SeqTypes>>;
    async fn get_block_summary_range(
        &self,
        from: usize,
        until: usize,
    ) -> anyhow::Result<Vec<BlockSummaryQueryData<SeqTypes>>>;

    async fn get_limits(&self) -> anyhow::Result<Limits>;

    async fn get_cert2(&self, height: u64) -> anyhow::Result<Option<Certificate2<SeqTypes>>>;

    async fn stream_leaves(
        &self,
        from: usize,
    ) -> anyhow::Result<BoxStream<'static, LeafQueryData<SeqTypes>>>;

    async fn stream_headers(
        &self,
        from: usize,
    ) -> anyhow::Result<BoxStream<'static, Header<SeqTypes>>>;

    async fn stream_blocks(
        &self,
        from: usize,
    ) -> anyhow::Result<BoxStream<'static, BlockQueryData<SeqTypes>>>;

    async fn stream_payloads(
        &self,
        from: usize,
    ) -> anyhow::Result<BoxStream<'static, PayloadQueryData<SeqTypes>>>;

    async fn stream_vid_common(
        &self,
        from: usize,
    ) -> anyhow::Result<BoxStream<'static, VidCommonQueryData<SeqTypes>>>;

    async fn stream_transactions(
        &self,
        from: usize,
        namespace: Option<u32>,
    ) -> anyhow::Result<BoxStream<'static, TransactionQueryData<SeqTypes>>>;
}
