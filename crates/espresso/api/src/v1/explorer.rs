//! V1 explorer API.
//!
//! Mirrors the tide-disco endpoints defined in `hotshot-query-service/api/explorer.toml`.

use async_trait::async_trait;
use serde::Serialize;

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
    type BlockDetail: Serialize + Send + Sync + 'static;
    type BlockSummaries: Serialize + Send + Sync + 'static;
    type TransactionDetail: Serialize + Send + Sync + 'static;
    type TransactionSummaries: Serialize + Send + Sync + 'static;
    type ExplorerSummary: Serialize + Send + Sync + 'static;
    type SearchResult: Serialize + Send + Sync + 'static;

    async fn get_block_detail(&self, ident: BlockIdent) -> anyhow::Result<Self::BlockDetail>;

    async fn get_block_summaries(
        &self,
        target: BlockIdent,
        limit: u64,
    ) -> anyhow::Result<Self::BlockSummaries>;

    async fn get_transaction_detail(
        &self,
        ident: TxIdent,
    ) -> anyhow::Result<Self::TransactionDetail>;

    async fn get_transaction_summaries(
        &self,
        target: TxIdent,
        limit: u64,
        filter: TxSummaryFilter,
    ) -> anyhow::Result<Self::TransactionSummaries>;

    async fn get_explorer_summary(&self) -> anyhow::Result<Self::ExplorerSummary>;

    async fn get_search_result(&self, query: String) -> anyhow::Result<Self::SearchResult>;
}
