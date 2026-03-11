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

//! Explorer storage implementation for a database query engine.

use std::{collections::VecDeque, num::NonZeroUsize};

use async_trait::async_trait;
use committable::{Commitment, Committable};
use futures::stream::{self, StreamExt, TryStreamExt};
use hotshot_types::traits::node_implementation::NodeType;
use sqlx::{FromRow, Row};
use tagged_base64::{Tagged, TaggedBase64};

use super::{
    super::transaction::{Transaction, TransactionMode},
    DecodeError, BLOCK_COLUMNS,
};
use crate::{
    availability::{BlockQueryData, QueryableHeader, QueryablePayload},
    data_source::storage::{ExplorerStorage, NodeStorage},
    explorer::{
        self,
        errors::{self, NotFound},
        query_data::TransactionDetailResponse,
        traits::ExplorerHeader,
        BalanceAmount, BlockDetail, BlockIdentifier, BlockRange, BlockSummary, ExplorerHistograms,
        ExplorerSummary, GenesisOverview, GetBlockDetailError, GetBlockSummariesError,
        GetBlockSummariesRequest, GetExplorerSummaryError, GetSearchResultsError,
        GetTransactionDetailError, GetTransactionSummariesError, GetTransactionSummariesRequest,
        SearchResult, TransactionIdentifier, TransactionRange, TransactionSummary,
        TransactionSummaryFilter,
    },
    types::HeightIndexed,
    with_backend, Header, Payload, QueryError, Transaction as HotshotTransaction,
};

impl From<sqlx::Error> for GetExplorerSummaryError {
    fn from(err: sqlx::Error) -> Self {
        Self::from(QueryError::from(err))
    }
}

impl From<sqlx::Error> for GetTransactionDetailError {
    fn from(err: sqlx::Error) -> Self {
        Self::from(QueryError::from(err))
    }
}

impl From<sqlx::Error> for GetTransactionSummariesError {
    fn from(err: sqlx::Error) -> Self {
        Self::from(QueryError::from(err))
    }
}

impl From<sqlx::Error> for GetBlockDetailError {
    fn from(err: sqlx::Error) -> Self {
        Self::from(QueryError::from(err))
    }
}

impl From<sqlx::Error> for GetBlockSummariesError {
    fn from(err: sqlx::Error) -> Self {
        Self::from(QueryError::from(err))
    }
}

impl From<sqlx::Error> for GetSearchResultsError {
    fn from(err: sqlx::Error) -> Self {
        Self::from(QueryError::from(err))
    }
}

lazy_static::lazy_static! {
    static ref GET_BLOCK_SUMMARIES_QUERY_FOR_LATEST: String = {
        format!(
            "SELECT {BLOCK_COLUMNS}
                FROM header AS h
                JOIN payload AS p ON h.height = p.height
                ORDER BY h.height DESC
                LIMIT $1"
            )
    };

    static ref GET_BLOCK_SUMMARIES_QUERY_FOR_HEIGHT: String = {
        format!(
            "SELECT {BLOCK_COLUMNS}
                FROM header AS h
                JOIN payload AS p ON h.height = p.height
                WHERE h.height <= $1
                ORDER BY h.height DESC
                LIMIT $2"
        )
    };

    // We want to match the blocks starting with the given hash, and working backwards
    // until we have returned up to the number of requested blocks.  The hash for a
    // block should be unique, so we should just need to start with identifying the
    // block height with the given hash, and return all blocks with a height less than
    // or equal to that height, up to the number of requested blocks.
    static ref GET_BLOCK_SUMMARIES_QUERY_FOR_HASH: String = {
        format!(
            "SELECT {BLOCK_COLUMNS}
                FROM header AS h
                JOIN payload AS p ON h.height = p.height
                WHERE h.height <= (SELECT h1.height FROM header AS h1 WHERE h1.hash = $1)
                ORDER BY h.height DESC
                LIMIT $2",
        )
    };

    static ref GET_BLOCK_DETAIL_QUERY_FOR_LATEST: String = {
        format!(
            "SELECT {BLOCK_COLUMNS}
                FROM header AS h
                JOIN payload AS p ON h.height = p.height
                ORDER BY h.height DESC
                LIMIT 1"
        )
    };

    static ref GET_BLOCK_DETAIL_QUERY_FOR_HEIGHT: String = {
        format!(
            "SELECT {BLOCK_COLUMNS}
                FROM header AS h
                JOIN payload AS p ON h.height = p.height
                WHERE h.height = $1
                ORDER BY h.height DESC
                LIMIT 1"
        )
    };

    static ref GET_BLOCK_DETAIL_QUERY_FOR_HASH: String = {
        format!(
            "SELECT {BLOCK_COLUMNS}
                FROM header AS h
                JOIN payload AS p ON h.height = p.height
                WHERE h.hash = $1
                ORDER BY h.height DESC
                LIMIT 1"
        )
    };


    static ref GET_BLOCKS_CONTAINING_TRANSACTIONS_NO_FILTER_QUERY: String = {
        format!(
            "SELECT {BLOCK_COLUMNS}
               FROM header AS h
               JOIN payload AS p ON h.height = p.height
               WHERE h.height IN (
                   SELECT t.block_height
                       FROM transactions AS t
                       WHERE (t.block_height, t.ns_id, t.position) <= ($1, $2, $3)
                       ORDER BY t.block_height DESC, t.ns_id DESC, t.position DESC
                       LIMIT $4
               )
               ORDER BY h.height DESC"
        )
    };

    static ref GET_BLOCKS_CONTAINING_TRANSACTIONS_IN_NAMESPACE_QUERY: String = {
        format!(
            "SELECT {BLOCK_COLUMNS}
               FROM header AS h
               JOIN payload AS p ON h.height = p.height
               WHERE h.height IN (
                   SELECT t.block_height
                       FROM transactions AS t
                       WHERE (t.block_height, t.ns_id, t.position) <= ($1, $2, $3)
                         AND t.ns_id = $5
                       ORDER BY t.block_height DESC, t.ns_id DESC, t.position DESC
                       LIMIT $4
               )
               ORDER BY h.height DESC"
        )
    };

    static ref GET_TRANSACTION_SUMMARIES_QUERY_FOR_BLOCK: String = {
        format!(
            "SELECT {BLOCK_COLUMNS}
                FROM header AS h
                JOIN payload AS p ON h.height = p.height
                WHERE  h.height = $1
                ORDER BY h.height DESC"
        )
    };

    static ref GET_TRANSACTION_DETAIL_QUERY_FOR_LATEST: String = {
        format!(
            "SELECT {BLOCK_COLUMNS}
                FROM header AS h
                JOIN payload AS p ON h.height = p.height
                WHERE h.height = (
                    SELECT MAX(t1.block_height)
                        FROM transactions AS t1
                )
                ORDER BY h.height DESC"
        )
    };

    static ref GET_TRANSACTION_DETAIL_QUERY_FOR_HEIGHT_AND_OFFSET: String = {
        format!(
            "SELECT {BLOCK_COLUMNS}
                FROM header AS h
                JOIN payload AS p ON h.height = p.height
                WHERE h.height = (
                    SELECT t1.block_height
                        FROM transactions AS t1
                        WHERE t1.block_height = $1
                        ORDER BY t1.block_height, t1.ns_id, t1.position
                        LIMIT 1
                        OFFSET $2

                )
                ORDER BY h.height DESC",
        )
    };

    static ref GET_TRANSACTION_DETAIL_QUERY_FOR_HASH: String = {
        format!(
            "SELECT {BLOCK_COLUMNS}
                FROM header AS h
                JOIN payload AS p ON h.height = p.height
                WHERE h.height = (
                    SELECT t1.block_height
                        FROM transactions AS t1
                        WHERE t1.hash = $1
                        ORDER BY t1.block_height DESC, t1.ns_id DESC, t1.position DESC
                        LIMIT 1
                )
                ORDER BY h.height DESC"
        )
    };
}

/// [EXPLORER_SUMMARY_HISTOGRAM_NUM_ENTRIES] is the number of entries we want
/// to return in our histogram summary.
const EXPLORER_SUMMARY_HISTOGRAM_NUM_ENTRIES: usize = 50;

/// [EXPLORER_SUMMARY_NUM_BLOCKS] is the number of blocks we want to return in
/// our explorer summary.
const EXPLORER_SUMMARY_NUM_BLOCKS: usize = 10;

/// [EXPLORER_SUMMARY_NUM_TRANSACTIONS] is the number of transactions we want
/// to return in our explorer summary.
const EXPLORER_SUMMARY_NUM_TRANSACTIONS: usize = 10;

#[async_trait]
impl<Mode, Types> ExplorerStorage<Types> for Transaction<Mode>
where
    Mode: TransactionMode,
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
    Header<Types>: QueryableHeader<Types> + ExplorerHeader<Types>,
    crate::Transaction<Types>: explorer::traits::ExplorerTransaction<Types>,
    BalanceAmount<Types>: Into<explorer::monetary_value::MonetaryValue>,
{
    async fn get_block_summaries(
        &mut self,
        request: GetBlockSummariesRequest<Types>,
    ) -> Result<Vec<BlockSummary<Types>>, GetBlockSummariesError> {
        let request = &request.0;

        let blocks: Vec<BlockSummary<Types>> = with_backend!(self, |tx| {
            let query_stmt = match request.target {
                BlockIdentifier::Latest => sqlx::query(&GET_BLOCK_SUMMARIES_QUERY_FOR_LATEST)
                    .bind(request.num_blocks.get() as i64),
                BlockIdentifier::Height(height) => {
                    sqlx::query(&GET_BLOCK_SUMMARIES_QUERY_FOR_HEIGHT)
                        .bind(height as i64)
                        .bind(request.num_blocks.get() as i64)
                },
                BlockIdentifier::Hash(hash) => sqlx::query(&GET_BLOCK_SUMMARIES_QUERY_FOR_HASH)
                    .bind(hash.to_string())
                    .bind(request.num_blocks.get() as i64),
            };

            // Convert each row to BlockSummary inline, avoiding a Vec<BlockQueryData>.
            query_stmt
                .fetch(tx.as_mut())
                .map(|row| {
                    let block = BlockQueryData::from_row(&row?)?;
                    block.try_into().decode_error("malformed block summary")
                })
                .try_collect()
                .await
        })?;

        Ok(blocks)
    }

    async fn get_block_detail(
        &mut self,
        request: BlockIdentifier<Types>,
    ) -> Result<BlockDetail<Types>, GetBlockDetailError> {
        let block: BlockQueryData<Types> = with_backend!(self, |tx| {
            let query_stmt = match request {
                BlockIdentifier::Latest => sqlx::query(&GET_BLOCK_DETAIL_QUERY_FOR_LATEST),
                BlockIdentifier::Height(height) => {
                    sqlx::query(&GET_BLOCK_DETAIL_QUERY_FOR_HEIGHT).bind(height as i64)
                },
                BlockIdentifier::Hash(hash) => {
                    sqlx::query(&GET_BLOCK_DETAIL_QUERY_FOR_HASH).bind(hash.to_string())
                },
            };

            let query_result = query_stmt.fetch_one(tx.as_mut()).await?;
            BlockQueryData::from_row(&query_result)
        })?;

        block
            .try_into()
            .decode_error("malformed block detail")
            .map_err(GetBlockDetailError::from)
    }

    async fn get_transaction_summaries(
        &mut self,
        request: GetTransactionSummariesRequest<Types>,
    ) -> Result<Vec<TransactionSummary<Types>>, GetTransactionSummariesError> {
        let range = &request.range;
        let target = &range.target;
        let filter = &request.filter;

        // We need to figure out the transaction target we are going to start
        // returned results based on.
        let Some((block_height, namespace, position)) = with_backend!(self, |tx| {
            let transaction_target_query = match target {
                TransactionIdentifier::Latest => sqlx::query(
                    "SELECT block_height AS height, ns_id, position FROM transactions ORDER BY \
                     block_height DESC, ns_id DESC, position DESC LIMIT 1",
                ),
                TransactionIdentifier::HeightAndOffset(height, _) => sqlx::query(
                    "SELECT block_height AS height, ns_id, position FROM transactions WHERE \
                     block_height = $1 ORDER BY ns_id DESC, position DESC LIMIT 1",
                )
                .bind(*height as i64),
                TransactionIdentifier::Hash(hash) => sqlx::query(
                    "SELECT block_height AS height, ns_id, position FROM transactions WHERE hash \
                     = $1 ORDER BY block_height DESC, ns_id DESC, position DESC LIMIT 1",
                )
                .bind(hash.to_string()),
            };
            let row = transaction_target_query.fetch_optional(tx.as_mut()).await?;
            Ok::<_, sqlx::Error>(row.map(|r| {
                let height = r.get::<i64, _>("height") as usize;
                let ns_id = r.get::<i64, _>("ns_id");
                let pos = r.get::<i64, _>("position");
                (height, ns_id, pos)
            }))
        })?
        else {
            // If nothing is found, then we want to return an empty summary list as it means there
            // is either no transaction, or the targeting criteria fails to identify any transaction
            return Ok(vec![]);
        };

        let offset = if let TransactionIdentifier::HeightAndOffset(_, offset) = target {
            *offset
        } else {
            0
        };

        // Our block_stream is more-or-less always the same, the only difference
        // is a an additional filter on the identified transactions being found
        // In general, we use our `transaction_target` to identify the starting
        // `block_height` and `namespace`, and `position`, and we grab up to `limit`
        // transactions from that point.  We then grab only the blocks for those
        // identified transactions, as only those blocks are needed to pull all
        // of the relevant transactions.
        let transaction_summary_vec: Vec<TransactionSummary<Types>> = with_backend!(self, |tx| {
            let query_stmt = match filter {
                TransactionSummaryFilter::RollUp(ns) => {
                    sqlx::query(&GET_BLOCKS_CONTAINING_TRANSACTIONS_IN_NAMESPACE_QUERY)
                        .bind(block_height as i64)
                        .bind(namespace)
                        .bind(position)
                        .bind((range.num_transactions.get() + offset) as i64)
                        .bind((*ns).into())
                },
                TransactionSummaryFilter::None => {
                    sqlx::query(&GET_BLOCKS_CONTAINING_TRANSACTIONS_NO_FILTER_QUERY)
                        .bind(block_height as i64)
                        .bind(namespace)
                        .bind(position)
                        .bind((range.num_transactions.get() + offset) as i64)
                },

                TransactionSummaryFilter::Block(block) => {
                    sqlx::query(&GET_TRANSACTION_SUMMARIES_QUERY_FOR_BLOCK).bind(*block as i64)
                },
            };

            // Stream-process each block into TransactionSummaries, avoiding
            // a Vec<BlockQueryData>. Each block is expanded via flat_map into
            // its matching transactions, then dropped.
            //
            // Pipeline per block row:
            //   1. block.enumerate() -- payload method yielding (TransactionIndex, Txn)
            //   2. filter_map -- keep only matching namespace, convert to summary
            //   3. .enumerate() -- Iterator::enumerate for offset within filtered set
            //   4. collect + rev -- reverse to newest-first (needs collect for rev)
            query_stmt
                .fetch(tx.as_mut())
                .map(|row| BlockQueryData::from_row(&row?).map_err(QueryError::from))
                .flat_map(|row: Result<BlockQueryData<Types>, QueryError>| match row {
                    Ok(block) => {
                        tracing::info!(height = block.height(), "selected block");
                        stream::iter(
                            block
                                .enumerate()
                                .filter_map(|(ix, txn)| {
                                    if let TransactionSummaryFilter::RollUp(ns) = filter {
                                        let tx_ns = QueryableHeader::<Types>::namespace_id(
                                            block.header(),
                                            &ix.ns_index,
                                        );
                                        if tx_ns.as_ref() != Some(ns) {
                                            return None;
                                        }
                                    }
                                    Some(txn)
                                })
                                .enumerate()
                                .map(|(index, txn)| {
                                    TransactionSummary::try_from((&block, index, txn)).map_err(
                                        |err| QueryError::Error {
                                            message: err.to_string(),
                                        },
                                    )
                                })
                                .collect::<Vec<_>>()
                                .into_iter()
                                .rev()
                                .collect::<Vec<_>>(),
                        )
                    },
                    Err(err) => stream::iter(vec![Err(err)]),
                })
                .try_collect()
                .await
        })?;

        Ok(transaction_summary_vec
            .into_iter()
            .skip(offset)
            .skip_while(|txn| {
                if let TransactionIdentifier::Hash(hash) = target {
                    txn.hash != *hash
                } else {
                    false
                }
            })
            .take(range.num_transactions.get())
            .collect::<Vec<TransactionSummary<Types>>>())
    }

    async fn get_transaction_detail(
        &mut self,
        request: TransactionIdentifier<Types>,
    ) -> Result<TransactionDetailResponse<Types>, GetTransactionDetailError> {
        let target = request;

        let block: BlockQueryData<Types> = with_backend!(self, |tx| {
            let query_stmt = match target {
                TransactionIdentifier::Latest => {
                    sqlx::query(&GET_TRANSACTION_DETAIL_QUERY_FOR_LATEST)
                },
                TransactionIdentifier::HeightAndOffset(height, offset) => {
                    sqlx::query(&GET_TRANSACTION_DETAIL_QUERY_FOR_HEIGHT_AND_OFFSET)
                        .bind(height as i64)
                        .bind(offset as i64)
                },
                TransactionIdentifier::Hash(hash) => {
                    sqlx::query(&GET_TRANSACTION_DETAIL_QUERY_FOR_HASH).bind(hash.to_string())
                },
            };

            let query_row = query_stmt.fetch_one(tx.as_mut()).await?;
            BlockQueryData::from_row(&query_row)
        })?;

        let txns = block.enumerate().map(|(_, txn)| txn).collect::<Vec<_>>();

        let (offset, txn) = match target {
            TransactionIdentifier::Latest => txns.into_iter().enumerate().next_back().ok_or(
                GetTransactionDetailError::TransactionNotFound(NotFound {
                    key: "Latest".to_string(),
                }),
            ),
            TransactionIdentifier::HeightAndOffset(height, offset) => {
                txns.into_iter().enumerate().nth(offset).ok_or(
                    GetTransactionDetailError::TransactionNotFound(NotFound {
                        key: format!("at {height} and {offset}"),
                    }),
                )
            },
            TransactionIdentifier::Hash(hash) => txns
                .into_iter()
                .enumerate()
                .find(|(_, txn)| txn.commit() == hash)
                .ok_or(GetTransactionDetailError::TransactionNotFound(NotFound {
                    key: format!("hash {hash}"),
                })),
        }?;

        Ok(TransactionDetailResponse::try_from((&block, offset, txn))?)
    }

    async fn get_explorer_summary(
        &mut self,
    ) -> Result<ExplorerSummary<Types>, GetExplorerSummaryError> {
        let histograms = {
            let histogram_query_result: Vec<(i64, i64, Option<i64>, Option<i32>, i32)> =
                with_backend!(self, |tx| {
                    sqlx::query(
                        "SELECT
                            h.height AS height,
                            h.timestamp AS timestamp,
                            h.timestamp - lead(timestamp) OVER (ORDER BY h.height DESC) AS time,
                            p.size AS size,
                            p.num_transactions AS transactions
                        FROM header AS h
                        JOIN payload AS p ON
                            p.height = h.height
                        WHERE
                            h.height IN (SELECT height FROM header ORDER BY height DESC LIMIT $1)
                        ORDER BY h.height
                        ",
                    )
                    .bind((EXPLORER_SUMMARY_HISTOGRAM_NUM_ENTRIES + 1) as i64)
                    .fetch(tx.as_mut())
                    .map(|row_result| {
                        row_result.map(|row| {
                            let height: i64 = row.try_get("height").unwrap();
                            let timestamp: i64 = row.try_get("timestamp").unwrap();
                            let time: Option<i64> = row.try_get("time").unwrap();
                            let size: Option<i32> = row.try_get("size").unwrap();
                            let num_transactions: i32 = row.try_get("transactions").unwrap();
                            (height, timestamp, time, size, num_transactions)
                        })
                    })
                    .try_collect()
                    .await
                })?;

            let mut histograms = ExplorerHistograms {
                block_time: VecDeque::with_capacity(EXPLORER_SUMMARY_HISTOGRAM_NUM_ENTRIES),
                block_size: VecDeque::with_capacity(EXPLORER_SUMMARY_HISTOGRAM_NUM_ENTRIES),
                block_transactions: VecDeque::with_capacity(EXPLORER_SUMMARY_HISTOGRAM_NUM_ENTRIES),
                block_heights: VecDeque::with_capacity(EXPLORER_SUMMARY_HISTOGRAM_NUM_ENTRIES),
            };

            for (height, _timestamp, time, size, num_transactions) in histogram_query_result {
                histograms.block_time.push_back(time.map(|i| i as u64));
                histograms.block_size.push_back(size.map(|i| i as u64));
                histograms
                    .block_transactions
                    .push_back(num_transactions as u64);
                histograms.block_heights.push_back(height as u64);
            }

            while histograms.block_time.len() > EXPLORER_SUMMARY_HISTOGRAM_NUM_ENTRIES {
                histograms.block_time.pop_front();
                histograms.block_size.pop_front();
                histograms.block_transactions.pop_front();
                histograms.block_heights.pop_front();
            }

            histograms
        };

        let genesis_overview = {
            let blocks = NodeStorage::<Types>::block_height(self).await? as u64;
            let transactions =
                NodeStorage::<Types>::count_transactions_in_range(self, .., None).await? as u64;
            GenesisOverview {
                rollups: 0,
                transactions,
                blocks,
            }
        };

        let latest_block: BlockDetail<Types> =
            self.get_block_detail(BlockIdentifier::Latest).await?;

        let latest_blocks: Vec<BlockSummary<Types>> = self
            .get_block_summaries(GetBlockSummariesRequest(BlockRange {
                target: BlockIdentifier::Latest,
                num_blocks: NonZeroUsize::new(EXPLORER_SUMMARY_NUM_BLOCKS).unwrap(),
            }))
            .await?;

        let latest_transactions: Vec<TransactionSummary<Types>> = self
            .get_transaction_summaries(GetTransactionSummariesRequest {
                range: TransactionRange {
                    target: TransactionIdentifier::Latest,
                    num_transactions: NonZeroUsize::new(EXPLORER_SUMMARY_NUM_TRANSACTIONS).unwrap(),
                },
                filter: TransactionSummaryFilter::None,
            })
            .await?;

        Ok(ExplorerSummary {
            genesis_overview,
            latest_block,
            latest_transactions,
            latest_blocks,
            histograms,
        })
    }

    async fn get_search_results(
        &mut self,
        search_query: TaggedBase64,
    ) -> Result<SearchResult<Types>, GetSearchResultsError> {
        let search_tag = search_query.tag();
        let header_tag = Commitment::<Header<Types>>::tag();
        let tx_tag = Commitment::<HotshotTransaction<Types>>::tag();

        if search_tag != header_tag && search_tag != tx_tag {
            return Err(GetSearchResultsError::InvalidQuery(errors::BadQuery {}));
        }

        let search_query_string = search_query.to_string();
        if search_tag == header_tag {
            let block_query = format!(
                "SELECT {BLOCK_COLUMNS}
                    FROM header AS h
                    JOIN payload AS p ON h.height = p.height
                    WHERE h.hash = $1
                    ORDER BY h.height DESC
                    LIMIT 1"
            );
            let block: BlockQueryData<Types> = with_backend!(self, |tx| {
                let row = sqlx::query(block_query.as_str())
                    .bind(&search_query_string)
                    .fetch_one(tx.as_mut())
                    .await?;
                BlockQueryData::from_row(&row)
            })?;

            let block: BlockSummary<Types> = block
                .try_into()
                .decode_error("malformed block summary")
                .map_err(GetSearchResultsError::from)?;

            Ok(SearchResult {
                blocks: vec![block],
                transactions: Vec::new(),
            })
        } else {
            let transactions_query = format!(
                "SELECT {BLOCK_COLUMNS}
                    FROM header AS h
                    JOIN payload AS p ON h.height = p.height
                    JOIN transactions AS t ON h.height = t.block_height
                    WHERE t.hash = $1
                    ORDER BY h.height DESC
                    LIMIT 5"
            );
            // Stream-process each block, expanding matching transactions via
            // flat_map. Each BlockQueryData is dropped after expansion.
            //
            // Pipeline per block row:
            //   1. block.enumerate() -- payload method yielding (TransactionIndex, Txn)
            //   2. .enumerate() -- Iterator::enumerate for offset in block (before filter)
            //   3. filter_map -- keep txn matching the search hash, convert to summary
            //
            // Note: enumerate before filter so offset reflects position in the
            // full block, not the filtered subset.
            let transactions_query_result: Vec<TransactionSummary<Types>> =
                with_backend!(self, |tx| {
                    sqlx::query(transactions_query.as_str())
                        .bind(&search_query_string)
                        .fetch(tx.as_mut())
                        .map(|row| BlockQueryData::from_row(&row?).map_err(QueryError::from))
                        .flat_map(|row: Result<BlockQueryData<Types>, QueryError>| match row {
                            Ok(block) => stream::iter(
                                block
                                    .enumerate()
                                    .enumerate()
                                    .filter_map(|(offset, (_, txn))| {
                                        if txn.commit().to_string() != search_query_string {
                                            return None;
                                        }
                                        Some(
                                            TransactionSummary::try_from((&block, offset, txn))
                                                .map_err(|err| QueryError::Error {
                                                    message: err.to_string(),
                                                }),
                                        )
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                            Err(err) => stream::iter(vec![Err(err)]),
                        })
                        .try_collect()
                        .await
                })?;

            Ok(SearchResult {
                blocks: Vec::new(),
                transactions: transactions_query_result,
            })
        }
    }
}
