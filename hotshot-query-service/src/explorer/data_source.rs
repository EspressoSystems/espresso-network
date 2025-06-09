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

use async_trait::async_trait;
use hotshot_types::traits::{block_contents::BlockHeader, node_implementation::NodeType};
use tagged_base64::TaggedBase64;

use super::{
    query_data::{
        BlockDetail, BlockIdentifier, BlockSummary, ExplorerSummary, GetBlockDetailError,
        GetBlockSummariesError, GetBlockSummariesRequest, GetExplorerSummaryError,
        GetSearchResultsError, GetTransactionDetailError, GetTransactionSummariesError,
        GetTransactionSummariesRequest, SearchResult, TransactionDetailResponse,
        TransactionIdentifier, TransactionSummary,
    },
    traits::{ExplorerHeader, ExplorerTransaction},
};
use crate::{availability::QueryablePayload, Header, Payload, Transaction};

/// An interface for querying Data and Statistics from the HotShot Blockchain.
///
/// This interface provides methods that allows the enabling of querying data
/// concerning the blockchain from the stored data for use with a
/// block explorer.  It does not provide the same guarantees as the
/// Availability data source with data fetching.  It is not concerned with
/// being up-to-date or having all of the data required, but rather it is
/// concerned with providing the requested data as quickly as possible, and in
/// a way that can be easily cached.
#[async_trait]
pub trait ExplorerDataSource<Types>
where
    Types: NodeType,
    Header<Types>: ExplorerHeader<Types> + BlockHeader<Types>,
    Transaction<Types>: ExplorerTransaction,
    Payload<Types>: QueryablePayload<Types>,
{
    /// `get_block_detail` is a method that retrieves the details of a specific
    /// block from the blockchain.  The block is identified by the given
    /// [BlockIdentifier].
    async fn get_block_detail(
        &self,
        request: BlockIdentifier<Types>,
    ) -> Result<BlockDetail<Types>, GetBlockDetailError>;

    /// `get_block_summaries` is a method that retrieves a list of block
    /// summaries from the blockchain.  The list is generated from the given
    /// [GetBlockSummariesRequest].
    async fn get_block_summaries(
        &self,
        request: GetBlockSummariesRequest<Types>,
    ) -> Result<Vec<BlockSummary<Types>>, GetBlockSummariesError>;

    /// `get_transaction_detail` is a method that retrieves the details of a
    /// specific transaction from the blockchain.  The transaction is identified
    /// by the given [TransactionIdentifier].
    async fn get_transaction_detail(
        &self,
        request: TransactionIdentifier<Types>,
    ) -> Result<TransactionDetailResponse<Types>, GetTransactionDetailError>;

    /// `get_transaction_summaries` is a method that retrieves a list of
    /// transaction summaries from the blockchain.  The list is generated from
    /// the given [GetTransactionSummariesRequest].
    async fn get_transaction_summaries(
        &self,
        request: GetTransactionSummariesRequest<Types>,
    ) -> Result<Vec<TransactionSummary<Types>>, GetTransactionSummariesError>;

    /// `get_explorer_summary` is a method that retrieves a summary overview of
    /// the blockchain.  This is useful for displaying information that
    /// indicates the overall status of the block chain.
    async fn get_explorer_summary(&self)
        -> Result<ExplorerSummary<Types>, GetExplorerSummaryError>;

    /// `get_search_results` is a method that retrieves the results of a search
    /// query against the blockchain.  The results are generated from the given
    /// query string.
    async fn get_search_results(
        &self,
        query: TaggedBase64,
    ) -> Result<SearchResult<Types>, GetSearchResultsError>;
}
