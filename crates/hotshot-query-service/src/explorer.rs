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

#[cfg(test)]
mod test {
    use std::{cmp::min, num::NonZeroUsize};

    use futures::StreamExt;

    use super::*;
    use crate::{
        availability::AvailabilityDataSource,
        testing::{
            consensus::{MockNetwork, MockSqlDataSource},
            mocks::{MockTypes, mock_transaction},
        },
    };

    fn num_blocks() -> usize {
        10
    }

    fn num_txns_per_block() -> usize {
        5
    }

    fn block_summaries(
        target: BlockIdentifier<MockTypes>,
        num_blocks: usize,
    ) -> GetBlockSummariesRequest<MockTypes> {
        GetBlockSummariesRequest(BlockRange {
            target,
            num_blocks: NonZeroUsize::new(num_blocks).unwrap(),
        })
    }

    fn transaction_summaries(
        target: TransactionIdentifier<MockTypes>,
        num_transactions: usize,
        filter: TransactionSummaryFilter<MockTypes>,
    ) -> GetTransactionSummariesRequest<MockTypes> {
        GetTransactionSummariesRequest {
            range: TransactionRange {
                target,
                num_transactions: NonZeroUsize::new(num_transactions).unwrap(),
            },
            filter,
        }
    }

    async fn validate(ds: &MockSqlDataSource) {
        let ExplorerSummary {
            histograms,
            latest_block,
            latest_blocks,
            latest_transactions,
            genesis_overview,
            ..
        } = ds.get_explorer_summary().await.unwrap();

        let GenesisOverview {
            blocks: num_blocks,
            transactions: num_transactions,
            ..
        } = genesis_overview;

        assert!(num_blocks > 0);
        assert_eq!(histograms.block_heights.len(), min(num_blocks as usize, 50));
        assert_eq!(histograms.block_size.len(), histograms.block_heights.len());
        assert_eq!(histograms.block_time.len(), histograms.block_heights.len());
        assert_eq!(
            histograms.block_transactions.len(),
            histograms.block_heights.len()
        );

        assert_eq!(latest_block.height, num_blocks - 1);
        assert_eq!(latest_blocks.len(), min(num_blocks as usize, 10));
        assert_eq!(
            latest_transactions.len(),
            min(num_transactions as usize, 10)
        );

        {
            // Retrieve Block Detail using the block height
            let block_detail = ds
                .get_block_detail(BlockIdentifier::Height(latest_block.height as usize))
                .await
                .unwrap();
            assert_eq!(block_detail, latest_block);
        }

        {
            // Retrieve Block Detail using the block hash
            let block_detail = ds
                .get_block_detail(BlockIdentifier::Hash(latest_block.hash))
                .await
                .unwrap();
            assert_eq!(block_detail, latest_block);
        }

        {
            // Retrieve 20 Block Summaries using the block height
            let summaries = ds
                .get_block_summaries(block_summaries(
                    BlockIdentifier::Height((num_blocks - 1) as usize),
                    20,
                ))
                .await
                .unwrap();
            for (a, b) in summaries.iter().zip(latest_blocks.iter()) {
                assert_eq!(a, b);
            }
        }

        {
            let target_num = min(num_blocks as usize, 10);
            // Retrieve the latest block summaries
            let summaries = ds
                .get_block_summaries(block_summaries(BlockIdentifier::Latest, target_num))
                .await
                .unwrap();

            // These blocks aren't guaranteed to have any overlap with what has
            // been previously generated, so we don't know if we can check
            // equality of the set.  However, we **can** check to see if the
            // number of blocks we were asking for get returned.
            assert_eq!(summaries.len(), target_num);

            // We can also perform a check on the first block to ensure that it
            // is larger than or equal to our `num_blocks` variable.
            assert!(summaries.first().unwrap().height >= num_blocks - 1);
        }

        let search_results = ds
            .get_search_results(latest_block.hash.to_string().parse().unwrap())
            .await
            .unwrap();
        assert!(!search_results.blocks.is_empty());

        if num_transactions > 0 {
            let last_transaction = latest_transactions.first().unwrap();
            let transaction_detail = ds
                .get_transaction_detail(TransactionIdentifier::Hash(last_transaction.hash))
                .await
                .unwrap();

            assert!(transaction_detail.details.block_confirmed);
            assert_eq!(transaction_detail.details.hash, last_transaction.hash);
            assert_eq!(transaction_detail.details.height, last_transaction.height);
            assert_eq!(
                transaction_detail.details.num_transactions,
                last_transaction.num_transactions
            );
            assert_eq!(transaction_detail.details.offset, last_transaction.offset);
            // assert_eq!(transaction_detail.details.size, last_transaction.size);
            assert_eq!(transaction_detail.details.time, last_transaction.time);

            // Transactions Summaries - No Filter
            let n_txns = num_txns_per_block();

            {
                // Retrieve transactions summaries via hash
                let summaries = ds
                    .get_transaction_summaries(transaction_summaries(
                        TransactionIdentifier::Hash(last_transaction.hash),
                        20,
                        TransactionSummaryFilter::None,
                    ))
                    .await
                    .unwrap();

                for (a, b) in summaries
                    .iter()
                    .zip(latest_transactions.iter().take(10).collect::<Vec<_>>())
                {
                    assert_eq!(a, b);
                }
            }

            {
                // Retrieve transactions summaries via height and offset
                // No offset, which should indicate the most recent transaction
                // within the targeted block.
                let summaries = ds
                    .get_transaction_summaries(transaction_summaries(
                        TransactionIdentifier::HeightAndOffset(last_transaction.height as usize, 0),
                        20,
                        TransactionSummaryFilter::None,
                    ))
                    .await
                    .unwrap();

                for (a, b) in summaries
                    .iter()
                    .zip(latest_transactions.iter().take(10).collect::<Vec<_>>())
                {
                    assert_eq!(a, b);
                }
            }

            {
                // Retrieve transactions summaries via height and offset (different offset)
                // In this case since we're creating n_txns transactions per
                // block, an offset of n_txns - 1 will ensure that we're still
                // within the same starting target block.
                let summaries = ds
                    .get_transaction_summaries(transaction_summaries(
                        TransactionIdentifier::HeightAndOffset(
                            last_transaction.height as usize,
                            n_txns - 1,
                        ),
                        20,
                        TransactionSummaryFilter::None,
                    ))
                    .await
                    .unwrap();

                for (a, b) in summaries.iter().zip(
                    latest_transactions
                        .iter()
                        .skip(n_txns - 1)
                        .take(10)
                        .collect::<Vec<_>>(),
                ) {
                    assert_eq!(a, b);
                }
            }

            {
                // Retrieve transactions summaries via height and offset (different offset)
                // In this case since we're creating n_txns transactions per
                // block, an offset of n_txns + 1 will ensure that we're
                // outside of the starting block
                let summaries = ds
                    .get_transaction_summaries(transaction_summaries(
                        TransactionIdentifier::HeightAndOffset(
                            last_transaction.height as usize,
                            n_txns + 1,
                        ),
                        20,
                        TransactionSummaryFilter::None,
                    ))
                    .await
                    .unwrap();

                for (a, b) in summaries.iter().zip(
                    latest_transactions
                        .iter()
                        .skip(6)
                        .take(10)
                        .collect::<Vec<_>>(),
                ) {
                    assert_eq!(a, b);
                }
            }

            {
                let summaries = ds
                    .get_transaction_summaries(transaction_summaries(
                        TransactionIdentifier::Latest,
                        20,
                        TransactionSummaryFilter::None,
                    ))
                    .await
                    .unwrap();

                for (a, b) in summaries
                    .iter()
                    .zip(latest_transactions.iter().take(10).collect::<Vec<_>>())
                {
                    assert_eq!(a, b);
                }
            }

            // Transactions Summaries - Block Filter

            let block_filter = TransactionSummaryFilter::Block(last_transaction.height as usize);

            {
                let summaries = ds
                    .get_transaction_summaries(transaction_summaries(
                        TransactionIdentifier::Hash(last_transaction.hash),
                        20,
                        block_filter.clone(),
                    ))
                    .await
                    .unwrap();

                for (a, b) in summaries
                    .iter()
                    .take_while(|t: &&TransactionSummary<MockTypes>| {
                        t.height == last_transaction.height
                    })
                    .zip(latest_transactions.iter().take(10).collect::<Vec<_>>())
                {
                    assert_eq!(a, b);
                }
            }

            {
                // With an offset of 0, we should start at the most recent
                // transaction within the specified block.
                let summaries = ds
                    .get_transaction_summaries(transaction_summaries(
                        TransactionIdentifier::HeightAndOffset(last_transaction.height as usize, 0),
                        20,
                        block_filter.clone(),
                    ))
                    .await
                    .unwrap();

                for (a, b) in summaries
                    .iter()
                    .take_while(|t: &&TransactionSummary<MockTypes>| {
                        t.height == last_transaction.height
                    })
                    .zip(latest_transactions.iter().take(10).collect::<Vec<_>>())
                {
                    assert_eq!(a, b);
                }
            }

            {
                // In this case, since we're creating n_txns transactions per
                // block, an offset of n_txns - 1 will ensure that we're still
                // within the same starting target block.
                let summaries = ds
                    .get_transaction_summaries(transaction_summaries(
                        TransactionIdentifier::HeightAndOffset(
                            last_transaction.height as usize,
                            n_txns - 1,
                        ),
                        20,
                        block_filter.clone(),
                    ))
                    .await
                    .unwrap();

                for (a, b) in summaries
                    .iter()
                    .skip(n_txns - 1)
                    .take_while(|t: &&TransactionSummary<MockTypes>| {
                        t.height == last_transaction.height
                    })
                    .zip(latest_transactions.iter().take(10).collect::<Vec<_>>())
                {
                    assert_eq!(a, b);
                }
            }

            {
                // In this case, since we're creating n_txns transactions per
                // block, an offset of n_txns + 1 will ensure that we're
                // outside of the starting target block
                let summaries = ds
                    .get_transaction_summaries(transaction_summaries(
                        TransactionIdentifier::HeightAndOffset(
                            last_transaction.height as usize,
                            n_txns + 1,
                        ),
                        20,
                        block_filter.clone(),
                    ))
                    .await
                    .unwrap();

                for (a, b) in summaries
                    .iter()
                    .skip(n_txns + 1)
                    .take_while(|t: &&TransactionSummary<MockTypes>| {
                        t.height == last_transaction.height
                    })
                    .zip(latest_transactions.iter().take(10).collect::<Vec<_>>())
                {
                    assert_eq!(a, b);
                }
            }

            {
                let summaries = ds
                    .get_transaction_summaries(transaction_summaries(
                        TransactionIdentifier::Latest,
                        20,
                        block_filter.clone(),
                    ))
                    .await
                    .unwrap();

                for (a, b) in summaries
                    .iter()
                    .take_while(|t: &&TransactionSummary<MockTypes>| {
                        t.height == last_transaction.height
                    })
                    .zip(latest_transactions.iter().take(10).collect::<Vec<_>>())
                {
                    assert_eq!(a, b);
                }
            }
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_api() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockSqlDataSource>::init().await;
        network.start().await;

        let ds = network.data_source();
        let mut blocks = ds.subscribe_blocks(0).await;

        let n_blocks = num_blocks();
        let n_txns = num_txns_per_block();
        for b in 0..n_blocks {
            for t in 0..n_txns {
                let nonce = b * n_txns + t;
                network
                    .submit_transaction(mock_transaction(vec![nonce as u8]))
                    .await;
            }

            // Wait for the transactions to be finalized.
            for _ in 0..10 {
                if !blocks.next().await.unwrap().is_empty() {
                    break;
                }
            }
        }

        validate(&ds).await;
        network.shut_down().await;
    }
}
