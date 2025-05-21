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

pub(crate) mod currency;
pub(crate) mod data_source;
pub(crate) mod errors;
pub(crate) mod monetary_value;
pub(crate) mod query_data;
pub(crate) mod traits;

use std::{fmt::Display, num::NonZeroUsize, path::Path};

pub use currency::*;
pub use data_source::*;
use futures::FutureExt;
use hotshot_types::traits::node_implementation::NodeType;
pub use monetary_value::*;
pub use query_data::*;
use serde::{Deserialize, Serialize};
use tide_disco::{api::ApiError, method::ReadState, Api, StatusCode};
pub use traits::*;
use vbs::version::StaticVersionType;

use self::errors::InvalidLimit;
use crate::{
    api::load_api,
    availability::{QueryableHeader, QueryablePayload},
    Header, Payload, Transaction,
};

/// [Error] is an enum that represents the various errors that can be returned
/// from the Explorer API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Error {
    GetBlockDetail(GetBlockDetailError),
    GetBlockSummaries(GetBlockSummariesError),
    GetTransactionDetail(GetTransactionDetailError),
    GetTransactionSummaries(GetTransactionSummariesError),
    GetExplorerSummary(GetExplorerSummaryError),
    GetSearchResults(GetSearchResultsError),
}

impl Error {
    pub fn status(&self) -> StatusCode {
        match self {
            Error::GetBlockDetail(e) => e.status(),
            Error::GetBlockSummaries(e) => e.status(),
            Error::GetTransactionDetail(e) => e.status(),
            Error::GetTransactionSummaries(e) => e.status(),
            Error::GetExplorerSummary(e) => e.status(),
            Error::GetSearchResults(e) => e.status(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::GetBlockDetail(e) => e.fmt(f),
            Error::GetBlockSummaries(e) => e.fmt(f),
            Error::GetTransactionDetail(e) => e.fmt(f),
            Error::GetTransactionSummaries(e) => e.fmt(f),
            Error::GetExplorerSummary(e) => e.fmt(f),
            Error::GetSearchResults(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::GetBlockDetail(e) => Some(e),
            Error::GetBlockSummaries(e) => Some(e),
            Error::GetTransactionDetail(e) => Some(e),
            Error::GetTransactionSummaries(e) => Some(e),
            Error::GetExplorerSummary(e) => Some(e),
            Error::GetSearchResults(e) => Some(e),
        }
    }
}

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
    Transaction<Types>: ExplorerTransaction,
{
    pub transaction_summaries: Vec<TransactionSummary<Types>>,
}

impl<Types: NodeType> From<Vec<TransactionSummary<Types>>> for TransactionSummariesResponse<Types>
where
    Header<Types>: ExplorerHeader<Types>,
    Transaction<Types>: ExplorerTransaction,
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
    Transaction<Types>: ExplorerTransaction,
{
    pub explorer_summary: ExplorerSummary<Types>,
}

impl<Types: NodeType> From<ExplorerSummary<Types>> for ExplorerSummaryResponse<Types>
where
    Header<Types>: ExplorerHeader<Types>,
    Transaction<Types>: ExplorerTransaction,
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
    Transaction<Types>: ExplorerTransaction,
{
    pub search_results: SearchResult<Types>,
}

impl<Types: NodeType> From<SearchResult<Types>> for SearchResultResponse<Types>
where
    Header<Types>: ExplorerHeader<Types>,
    Transaction<Types>: ExplorerTransaction,
{
    fn from(search_results: SearchResult<Types>) -> Self {
        Self { search_results }
    }
}

fn validate_limit(
    limit: Result<usize, tide_disco::RequestError>,
) -> Result<NonZeroUsize, InvalidLimit> {
    let num_blocks = match limit {
        Ok(limit) => Ok(limit),
        _ => Err(InvalidLimit {}),
    }?;

    let num_blocks = match NonZeroUsize::new(num_blocks) {
        Some(num_blocks) => Ok(num_blocks),
        None => Err(InvalidLimit {}),
    }?;

    if num_blocks.get() > 100 {
        return Err(InvalidLimit {});
    }

    Ok(num_blocks)
}

/// `define_api` is a function that defines the API endpoints for the Explorer
/// module of the HotShot Query Service. It implements the specification
/// defined in the `explorer.toml` file.
pub fn define_api<State, Types: NodeType, Ver: StaticVersionType + 'static>(
    _: Ver,
    api_ver: semver::Version,
) -> Result<Api<State, Error, Ver>, ApiError>
where
    State: 'static + Send + Sync + ReadState,
    Header<Types>: ExplorerHeader<Types> + QueryableHeader<Types>,
    Transaction<Types>: ExplorerTransaction,
    Payload<Types>: QueryablePayload<Types>,
    <State as ReadState>::State: ExplorerDataSource<Types> + Send + Sync,
{
    let mut api = load_api::<State, Error, Ver>(
        Option::<Box<Path>>::None,
        include_str!("../api/explorer.toml"),
        None,
    )?;

    api.with_version(api_ver)
        .get("get_block_detail", move |req, state| {
            async move {
                let target = match (
                    req.opt_integer_param::<str, usize>("height"),
                    req.opt_blob_param("hash"),
                ) {
                    (Ok(Some(from)), _) => BlockIdentifier::Height(from),
                    (_, Ok(Some(hash))) => BlockIdentifier::Hash(hash),
                    _ => BlockIdentifier::Latest,
                };

                state
                    .get_block_detail(target)
                    .await
                    .map(BlockDetailResponse::from)
                    .map_err(Error::GetBlockDetail)
            }
            .boxed()
        })?
        .get("get_block_summaries", move |req, state| {
            async move {
                let num_blocks = validate_limit(req.integer_param("limit"))
                    .map_err(GetBlockSummariesError::InvalidLimit)
                    .map_err(Error::GetBlockSummaries)?;

                let target = match (
                    req.opt_integer_param::<str, usize>("from"),
                    req.opt_blob_param("hash"),
                ) {
                    (Ok(Some(from)), _) => BlockIdentifier::Height(from),
                    (_, Ok(Some(hash))) => BlockIdentifier::Hash(hash),
                    _ => BlockIdentifier::Latest,
                };

                state
                    .get_block_summaries(GetBlockSummariesRequest(BlockRange {
                        target,
                        num_blocks,
                    }))
                    .await
                    .map(BlockSummaryResponse::from)
                    .map_err(Error::GetBlockSummaries)
            }
            .boxed()
        })?
        .get("get_transaction_detail", move |req, state| {
            async move {
                state
                    .get_transaction_detail(
                        match (
                            req.opt_integer_param("height"),
                            req.opt_integer_param("offset"),
                            req.opt_blob_param("hash"),
                        ) {
                            (Ok(Some(height)), Ok(Some(offset)), _) => {
                                TransactionIdentifier::HeightAndOffset(height, offset)
                            },
                            (_, _, Ok(Some(hash))) => TransactionIdentifier::Hash(hash),
                            _ => TransactionIdentifier::Latest,
                        },
                    )
                    .await
                    .map(TransactionDetailResponse::from)
                    .map_err(Error::GetTransactionDetail)
            }
            .boxed()
        })?
        .get("get_transaction_summaries", move |req, state| {
            async move {
                let num_transactions = validate_limit(req.integer_param("limit"))
                    .map_err(GetTransactionSummariesError::InvalidLimit)
                    .map_err(Error::GetTransactionSummaries)?;

                let filter = match (
                    req.opt_integer_param("block"),
                    req.opt_integer_param("namespace"),
                ) {
                    (Ok(Some(block)), _) => TransactionSummaryFilter::Block(block),
                    (_, Ok(Some(namespace))) => TransactionSummaryFilter::RollUp(namespace),
                    _ => TransactionSummaryFilter::None,
                };

                let target = match (
                    req.opt_integer_param::<str, usize>("height"),
                    req.opt_integer_param::<str, usize>("offset"),
                    req.opt_blob_param("hash"),
                ) {
                    (Ok(Some(height)), Ok(Some(offset)), _) => {
                        TransactionIdentifier::HeightAndOffset(height, offset)
                    },
                    (_, _, Ok(Some(hash))) => TransactionIdentifier::Hash(hash),
                    _ => TransactionIdentifier::Latest,
                };

                state
                    .get_transaction_summaries(GetTransactionSummariesRequest {
                        range: TransactionRange {
                            target,
                            num_transactions,
                        },
                        filter,
                    })
                    .await
                    .map(TransactionSummariesResponse::from)
                    .map_err(Error::GetTransactionSummaries)
            }
            .boxed()
        })?
        .get("get_explorer_summary", move |_req, state| {
            async move {
                state
                    .get_explorer_summary()
                    .await
                    .map(ExplorerSummaryResponse::from)
                    .map_err(Error::GetExplorerSummary)
            }
            .boxed()
        })?
        .get("get_search_result", move |req, state| {
            async move {
                let query = req
                    .tagged_base64_param("query")
                    .map_err(|err| {
                        tracing::error!("query param error: {}", err);
                        GetSearchResultsError::InvalidQuery(errors::BadQuery {})
                    })
                    .map_err(Error::GetSearchResults)?;

                state
                    .get_search_results(query.clone())
                    .await
                    .map(SearchResultResponse::from)
                    .map_err(Error::GetSearchResults)
            }
            .boxed()
        })?;
    Ok(api)
}

#[cfg(test)]
mod test {
    use std::{cmp::min, time::Duration};

    use futures::StreamExt;
    use portpicker::pick_unused_port;
    use surf_disco::Client;
    use tide_disco::App;

    use super::*;
    use crate::{
        availability,
        testing::{
            consensus::{MockNetwork, MockSqlDataSource},
            mocks::{mock_transaction, MockBase, MockTypes, MockVersions},
            setup_test,
        },
        ApiState, Error,
    };

    async fn validate(client: &Client<Error, MockBase>) {
        let explorer_summary_response: ExplorerSummaryResponse<MockTypes> =
            client.get("explorer-summary").send().await.unwrap();

        let ExplorerSummary {
            histograms,
            latest_block,
            latest_blocks,
            latest_transactions,
            genesis_overview,
            ..
        } = explorer_summary_response.explorer_summary;

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
            let block_detail_response: BlockDetailResponse<MockTypes> = client
                .get(format!("block/{}", latest_block.height).as_str())
                .send()
                .await
                .unwrap();
            assert_eq!(block_detail_response.block_detail, latest_block);
        }

        {
            // Retrieve Block Detail using the block hash
            let block_detail_response: BlockDetailResponse<MockTypes> = client
                .get(format!("block/hash/{}", latest_block.hash).as_str())
                .send()
                .await
                .unwrap();
            assert_eq!(block_detail_response.block_detail, latest_block);
        }

        {
            // Retrieve 20 Block Summaries using the block height
            let block_summaries_response: BlockSummaryResponse<MockTypes> = client
                .get(format!("blocks/{}/{}", num_blocks - 1, 20).as_str())
                .send()
                .await
                .unwrap();
            for (a, b) in block_summaries_response
                .block_summaries
                .iter()
                .zip(latest_blocks.iter())
            {
                assert_eq!(a, b);
            }
        }

        {
            let target_num = min(num_blocks as usize, 10);
            // Retrieve the 20 latest block summaries
            let block_summaries_response: BlockSummaryResponse<MockTypes> = client
                .get(format!("blocks/latest/{}", target_num).as_str())
                .send()
                .await
                .unwrap();

            // These blocks aren't guaranteed to have any overlap with what has
            // been previously generated, so we don't know if we can check
            // equality of the set.  However, we **can** check to see if the
            // number of blocks we were asking for get returned.
            assert_eq!(block_summaries_response.block_summaries.len(), target_num);

            // We can also perform a check on the first block to ensure that it
            // is larger than or equal to our `num_blocks` variable.
            assert!(
                block_summaries_response
                    .block_summaries
                    .first()
                    .unwrap()
                    .height
                    >= num_blocks - 1
            );
        }
        let get_search_response: SearchResultResponse<MockTypes> = client
            .get(format!("search/{}", latest_block.hash).as_str())
            .send()
            .await
            .unwrap();

        assert!(!get_search_response.search_results.blocks.is_empty());

        if num_transactions > 0 {
            let last_transaction = latest_transactions.first().unwrap();
            let transaction_detail_response: TransactionDetailResponse<MockTypes> = client
                .get(format!("transaction/hash/{}", last_transaction.hash).as_str())
                .send()
                .await
                .unwrap();

            assert!(
                transaction_detail_response
                    .transaction_detail
                    .details
                    .block_confirmed
            );

            assert_eq!(
                transaction_detail_response.transaction_detail.details.hash,
                last_transaction.hash
            );

            assert_eq!(
                transaction_detail_response
                    .transaction_detail
                    .details
                    .height,
                last_transaction.height
            );

            assert_eq!(
                transaction_detail_response
                    .transaction_detail
                    .details
                    .num_transactions,
                last_transaction.num_transactions
            );

            assert_eq!(
                transaction_detail_response
                    .transaction_detail
                    .details
                    .offset,
                last_transaction.offset
            );
            // assert_eq!(transaction_detail_response.transaction_detail.details.size, last_transaction.size);

            assert_eq!(
                transaction_detail_response.transaction_detail.details.time,
                last_transaction.time
            );

            // Transactions Summaries - No Filter
            let n_txns = num_txns_per_block();

            {
                // Retrieve transactions summaries via hash
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(format!("transactions/hash/{}/{}", last_transaction.hash, 20).as_str())
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
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
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(
                            format!("transactions/from/{}/{}/{}", last_transaction.height, 0, 20)
                                .as_str(),
                        )
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
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
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(
                            format!(
                                "transactions/from/{}/{}/{}",
                                last_transaction.height,
                                n_txns - 1,
                                20
                            )
                            .as_str(),
                        )
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
                    .iter()
                    .zip(
                        latest_transactions
                            .iter()
                            .skip(n_txns - 1)
                            .take(10)
                            .collect::<Vec<_>>(),
                    )
                {
                    assert_eq!(a, b);
                }
            }

            {
                // Retrieve transactions summaries via height and offset (different offset)
                // In this case since we're creating n_txns transactions per
                // block, an offset of n_txns + 1 will ensure that we're
                // outside of the starting block
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(
                            format!(
                                "transactions/from/{}/{}/{}",
                                last_transaction.height,
                                n_txns + 1,
                                20
                            )
                            .as_str(),
                        )
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
                    .iter()
                    .zip(
                        latest_transactions
                            .iter()
                            .skip(6)
                            .take(10)
                            .collect::<Vec<_>>(),
                    )
                {
                    assert_eq!(a, b);
                }
            }

            {
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(format!("transactions/latest/{}", 20).as_str())
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
                    .iter()
                    .zip(latest_transactions.iter().take(10).collect::<Vec<_>>())
                {
                    assert_eq!(a, b);
                }
            }

            // Transactions Summaries - Block Filter

            {
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(
                            format!(
                                "transactions/hash/{}/{}/block/{}",
                                last_transaction.hash, 20, last_transaction.height
                            )
                            .as_str(),
                        )
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
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
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(
                            format!(
                                "transactions/from/{}/{}/{}/block/{}",
                                last_transaction.height, 0, 20, last_transaction.height
                            )
                            .as_str(),
                        )
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
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
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(
                            format!(
                                "transactions/from/{}/{}/{}/block/{}",
                                last_transaction.height,
                                n_txns - 1,
                                20,
                                last_transaction.height
                            )
                            .as_str(),
                        )
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
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
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(
                            format!(
                                "transactions/from/{}/{}/{}/block/{}",
                                last_transaction.height,
                                n_txns + 1,
                                20,
                                last_transaction.height
                            )
                            .as_str(),
                        )
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
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
                let transaction_summaries_response: TransactionSummariesResponse<MockTypes> =
                    client
                        .get(
                            format!(
                                "transactions/latest/{}/block/{}",
                                20, last_transaction.height
                            )
                            .as_str(),
                        )
                        .send()
                        .await
                        .unwrap();

                for (a, b) in transaction_summaries_response
                    .transaction_summaries
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

    #[tokio::test(flavor = "multi_thread")]
    async fn test_api() {
        test_api_helper().await;
    }

    fn num_blocks() -> usize {
        10
    }

    fn num_txns_per_block() -> usize {
        5
    }

    async fn test_api_helper() {
        setup_test();

        // Create the consensus network.
        let mut network = MockNetwork::<MockSqlDataSource, MockVersions>::init().await;
        network.start().await;

        // Start the web server.
        let port = pick_unused_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "explorer",
            define_api(MockBase::instance(), "0.0.1".parse().unwrap()).unwrap(),
        )
        .unwrap();
        app.register_module(
            "availability",
            availability::define_api(
                &availability::Options {
                    fetch_timeout: Duration::from_secs(5),
                    ..Default::default()
                },
                MockBase::instance(),
                "0.0.1".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();

        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{}", port), MockBase::instance()),
        );

        // Start a client.
        let availability_client = Client::<Error, MockBase>::new(
            format!("http://localhost:{}/availability", port)
                .parse()
                .unwrap(),
        );
        let explorer_client = Client::<Error, MockBase>::new(
            format!("http://localhost:{}/explorer", port)
                .parse()
                .unwrap(),
        );

        assert!(
            availability_client
                .connect(Some(Duration::from_secs(60)))
                .await
        );

        let mut blocks = availability_client
            .socket("stream/blocks/0")
            .subscribe::<availability::BlockQueryData<MockTypes>>()
            .await
            .unwrap();

        let n_blocks = num_blocks();
        let n_txns = num_txns_per_block();
        for b in 0..n_blocks {
            for t in 0..n_txns {
                let nonce = b * n_txns + t;
                let txn: hotshot_example_types::block_types::TestTransaction =
                    mock_transaction(vec![nonce as u8]);
                network.submit_transaction(txn).await;
            }

            // Wait for the transaction to be finalized.
            for _ in 0..10 {
                let block = blocks.next().await.unwrap();
                let block = block.unwrap();

                if !block.is_empty() {
                    break;
                }
            }
        }

        assert!(explorer_client.connect(Some(Duration::from_secs(60))).await);

        // sleep a little bit to give some chance for blocks to be generated.
        validate(&explorer_client).await;
        network.shut_down().await;
    }
}
