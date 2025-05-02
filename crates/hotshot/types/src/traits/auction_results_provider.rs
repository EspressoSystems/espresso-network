// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! This module defines the interaction layer with the Solver via the [`AuctionResultsProvider`] trait,
//! which handles connecting to, and fetching the allocation results from, the Solver.

use anyhow::Result;
use async_trait::async_trait;

use super::node_implementation::NodeType;

/// The AuctionResultsProvider trait is the sole source of Solver-originated state and interaction,
/// and returns the results of the Solver's allocation via the associated type. The associated type,
/// `AuctionResult`, also implements the `HasUrls` trait, which requires that the output
/// type has the requisite fields available.
#[async_trait]
pub trait AuctionResultsProvider<TYPES: NodeType>: Send + Sync + Clone {
    /// Fetches the auction result for a view. Does not cache the result,
    /// subsequent calls will invoke additional wasted calls.
    async fn fetch_auction_result(&self, view_number: TYPES::View) -> Result<TYPES::AuctionResult>;
}
