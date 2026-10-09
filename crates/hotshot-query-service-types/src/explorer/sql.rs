//! Mappings between explorer types and SQL types.
#![cfg(feature = "sqlx")]

use crate::{
    QueryError,
    explorer::{
        GetBlockDetailError, GetBlockSummariesError, GetExplorerSummaryError,
        GetSearchResultsError, GetTransactionDetailError, GetTransactionSummariesError,
    },
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
