// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

pub(crate) mod data_source;
pub(crate) mod query_data;

pub use currency::*;
pub use data_source::*;
pub use hotshot_query_service_types::explorer::Error;
use hotshot_types::traits::node_implementation::NodeType;
pub use query_data::*;
use serde::{Deserialize, Serialize};
pub use traits::*;

use crate::{Header, Transaction};

/// [BlockDetailResponse] is a struct that represents the response from the
/// `get_block_detail` endpoint.
#[derive(Debug, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct BlockDetailResponse<Types: NodeType>
where
    Header<Types>: ExplorerHeader<Types>,
{
    pub block_detail: BlockDetail<Types>,
}

impl<Types: NodeType> From<BlockDetail<Types>> for BlockDetailResponse<Types>
where
    Header<Types>: ExplorerHeader<Types>,
{
    fn from(block_detail: BlockDetail<Types>) -> Self {
        Self { block_detail }
    }
}

/// [BlockSummaryResponse] is a struct that represents the response from the
/// `get_block_summaries` endpoint.
#[derive(Debug, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct BlockSummaryResponse<Types: NodeType>
where
    Header<Types>: ExplorerHeader<Types>,
{
    pub block_summaries: Vec<BlockSummary<Types>>,
}

impl<Types: NodeType> From<Vec<BlockSummary<Types>>> for BlockSummaryResponse<Types>
where
    Header<Types>: ExplorerHeader<Types>,
{
    fn from(block_summaries: Vec<BlockSummary<Types>>) -> Self {
        Self { block_summaries }
    }
}

/// [TransactionDetailResponse] is a struct that represents the response from the
/// `get_transaction_detail` endpoint.
#[derive(Debug, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct TransactionDetailResponse<Types: NodeType> {
    pub transaction_detail: query_data::TransactionDetailResponse<Types>,
}

impl<Types: NodeType> From<query_data::TransactionDetailResponse<Types>>
    for TransactionDetailResponse<Types>
{
    fn from(transaction_detail: query_data::TransactionDetailResponse<Types>) -> Self {
        Self { transaction_detail }
    }
}

/// [TransactionSummariesResponse] is a struct that represents the response from the
/// `get_transaction_summaries` endpoint.
#[derive(Debug, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct TransactionSummariesResponse<Types: NodeType>
where
    Header<Types>: ExplorerHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
{
    pub transaction_summaries: Vec<TransactionSummary<Types>>,
}

impl<Types: NodeType> From<Vec<TransactionSummary<Types>>> for TransactionSummariesResponse<Types>
where
    Header<Types>: ExplorerHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
{
    fn from(transaction_summaries: Vec<TransactionSummary<Types>>) -> Self {
        Self {
            transaction_summaries,
        }
    }
}

/// [ExplorerSummaryResponse] is a struct that represents the response from the
/// `get_explorer_summary` endpoint.
#[derive(Debug, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct ExplorerSummaryResponse<Types: NodeType>
where
    Header<Types>: ExplorerHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
{
    pub explorer_summary: ExplorerSummary<Types>,
}

impl<Types: NodeType> From<ExplorerSummary<Types>> for ExplorerSummaryResponse<Types>
where
    Header<Types>: ExplorerHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
{
    fn from(explorer_summary: ExplorerSummary<Types>) -> Self {
        Self { explorer_summary }
    }
}

/// [SearchResultResponse] is a struct that represents the response from the
/// `get_search_result` endpoint.
#[derive(Debug, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct SearchResultResponse<Types: NodeType>
where
    Header<Types>: ExplorerHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
{
    pub search_results: SearchResult<Types>,
}

impl<Types: NodeType> From<SearchResult<Types>> for SearchResultResponse<Types>
where
    Header<Types>: ExplorerHeader<Types>,
    Transaction<Types>: ExplorerTransaction<Types>,
{
    fn from(search_results: SearchResult<Types>) -> Self {
        Self { search_results }
    }
}
