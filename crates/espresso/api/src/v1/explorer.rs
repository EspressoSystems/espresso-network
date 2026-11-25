//! V1 explorer API.
//!
//! Mirrors the endpoints of the legacy `hotshot-query-service` explorer API (its tide-disco
//! `explorer.toml`, since removed).

use async_trait::async_trait;
use espresso_types::SeqTypes;
use hotshot_query_service_types::explorer::response::{
    BlockDetailResponse, BlockSummaryResponse, ExplorerSummaryResponse, SearchResultResponse,
    TransactionDetailResponse, TransactionSummariesResponse,
};

#[derive(Debug, Clone)]
pub enum BlockIdent {
    Height(u64),
    Hash(String),
    Latest,
}

#[derive(Debug, Clone)]
pub enum TxIdent {
    HeightAndOffset(u64, u64),
    Hash(String),
    Latest,
}

#[derive(Debug, Clone)]
pub enum TxSummaryFilter {
    None,
    Block(u64),
    Namespace(i64),
}

#[async_trait]
pub trait ExplorerApi {
    async fn get_block_detail(
        &self,
        ident: BlockIdent,
    ) -> anyhow::Result<BlockDetailResponse<SeqTypes>>;

    async fn get_block_summaries(
        &self,
        target: BlockIdent,
        limit: u64,
    ) -> anyhow::Result<BlockSummaryResponse<SeqTypes>>;

    async fn get_transaction_detail(
        &self,
        ident: TxIdent,
    ) -> anyhow::Result<TransactionDetailResponse<SeqTypes>>;

    async fn get_transaction_summaries(
        &self,
        target: TxIdent,
        limit: u64,
        filter: TxSummaryFilter,
    ) -> anyhow::Result<TransactionSummariesResponse<SeqTypes>>;

    async fn get_explorer_summary(&self) -> anyhow::Result<ExplorerSummaryResponse<SeqTypes>>;

    async fn get_search_result(
        &self,
        query: String,
    ) -> anyhow::Result<SearchResultResponse<SeqTypes>>;
}
