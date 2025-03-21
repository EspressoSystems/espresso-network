// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use anyhow::{bail, Result};
use async_trait::async_trait;
use hotshot_types::traits::{
    auction_results_provider::AuctionResultsProvider,
    node_implementation::{HasUrls, NodeType},
};
use serde::{Deserialize, Serialize};
use url::Url;

/// A mock result for the auction solver. This type is just a pointer to a URL.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, Default)]
pub struct TestAuctionResult {
    /// The URL of the builder to reach out to.
    pub urls: Vec<Url>,
}

impl HasUrls for TestAuctionResult {
    fn urls(&self) -> Vec<Url> {
        self.urls.clone()
    }
}

/// The test auction results type is used to mimic the results from the Solver.
#[derive(Clone, Debug, Default)]
pub struct TestAuctionResultsProvider<TYPES: NodeType> {
    /// We intentionally allow for the results to be pre-cooked for the unit test to guarantee a
    /// particular outcome is met.
    pub solver_results: TYPES::AuctionResult,

    /// A canned type to ensure that an error is thrown in absence of a true fault-injectible
    /// system for logical tests. This will guarantee that `fetch_auction_result` always throws an
    /// error.
    pub should_return_err: bool,

    /// The broadcast URL that the solver is running on. This type allows for the url to be
    /// optional, where `None` means to just return whatever `solver_results` contains, and `Some`
    /// means that we have a `FakeSolver` instance available to query.
    pub broadcast_url: Option<Url>,
}

#[async_trait]
impl<TYPES: NodeType> AuctionResultsProvider<TYPES> for TestAuctionResultsProvider<TYPES> {
    /// Mock fetching the auction results, with optional error injection to simulate failure cases
    /// in the solver.
    async fn fetch_auction_result(&self, view_number: TYPES::View) -> Result<TYPES::AuctionResult> {
        if let Some(url) = &self.broadcast_url {
            let resp =
                reqwest::get(url.join(&format!("/v0/api/auction_results/{}", *view_number))?)
                    .await?
                    .json::<TYPES::AuctionResult>()
                    .await?;

            Ok(resp)
        } else {
            if self.should_return_err {
                bail!("Something went wrong")
            }

            // Otherwise, return our pre-made results
            Ok(self.solver_results.clone())
        }
    }
}
