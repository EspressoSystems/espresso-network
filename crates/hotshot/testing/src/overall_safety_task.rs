// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use hotshot_types::traits::node_implementation::NodeType;
use thiserror::Error;
use tracing::error;

/// convenience type alias for state and block
pub type StateAndBlock<S, B> = (Vec<S>, Vec<B>);

/// the status of a view
#[derive(Debug, Clone)]
pub enum ViewStatus<TYPES: NodeType> {
    /// success
    Ok,
    /// failure
    Failed,
    /// safety violation
    Err(OverallSafetyTaskErr<TYPES>),
    /// in progress
    InProgress,
}

/// possible errors
#[derive(Error, Debug, Clone)]
pub enum OverallSafetyTaskErr<TYPES: NodeType> {
    #[error("Mismatched leaf")]
    MismatchedLeaf,

    #[error("Inconsistent blocks")]
    InconsistentBlocks,

    #[error("Inconsistent number of transactions: {map:?}")]
    InconsistentTxnsNum { map: HashMap<u64, usize> },

    #[error("Not enough decides: got: {got}, expected: {expected}")]
    NotEnoughDecides { got: usize, expected: usize },

    #[error("Too many view failures: {0:?}")]
    TooManyFailures(HashSet<TYPES::View>),

    #[error("Inconsistent failed views: expected: {expected_failed_views:?}, actual: {actual_failed_views:?}")]
    InconsistentFailedViews {
        expected_failed_views: Vec<TYPES::View>,
        actual_failed_views: HashSet<TYPES::View>,
    },
    #[error(
        "Not enough round results: results_count: {results_count}, views_count: {views_count}"
    )]
    NotEnoughRoundResults {
        results_count: usize,
        views_count: usize,
    },

    #[error("View timed out")]
    ViewTimeout,
}

/// cross node safety properties
#[derive(Clone, Debug)]
pub struct OverallSafetyPropertiesDescription {
    /// required number of successful views
    pub num_successful_views: usize,
    /// whether or not to check the leaf
    pub check_leaf: bool,
    /// whether or not to check the block
    pub check_block: bool,
    /// whether or not to check that we have threshold amounts of transactions each block
    /// if 0: don't check
    /// if n > 0, check that at least n transactions are decided upon if such information
    /// is available
    pub transaction_threshold: u64,
    /// pass in the views that we expect to fail.
    ///
    /// the test should fail if any view on this list succeeds.
    pub expected_view_failures: Vec<u64>,
    /// pass in the views that may or may not fail.
    pub possible_view_failures: Vec<u64>,
    /// how long to wait between external events before timing out the test
    pub decide_timeout: Duration,
}

impl Default for OverallSafetyPropertiesDescription {
    fn default() -> Self {
        Self {
            num_successful_views: 50,
            check_leaf: false,
            check_block: true,
            transaction_threshold: 0,
            expected_view_failures: vec![],
            possible_view_failures: vec![],
            decide_timeout: Duration::from_secs(4),
        }
    }
}
