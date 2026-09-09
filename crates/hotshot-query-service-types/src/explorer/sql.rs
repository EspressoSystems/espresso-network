//! Mappings between explorer types and SQL types.
#![cfg(feature = "sqlx")]

use hotshot_types::traits::{block_contents::BlockHeader, node_implementation::NodeType};
use serde_json::Value;
use sqlx::{ColumnIndex, prelude::*, types::Json};

use crate::{
    Header, Payload, QueryError,
    availability::{BlockQueryData, QueryablePayload, sql::DecodeError},
    explorer::{
        BalanceAmount, BlockDetail, BlockSummary, GetBlockDetailError, GetBlockSummariesError,
        GetExplorerSummaryError, GetSearchResultsError, GetTransactionDetailError,
        GetTransactionSummariesError, monetary_value::MonetaryValue, traits::ExplorerHeader,
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

impl<'r, Types, R> FromRow<'r, R> for BlockSummary<Types>
where
    Types: NodeType,
    Header<Types>: BlockHeader<Types> + ExplorerHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    R: Row,
    for<'a> &'a str: ColumnIndex<R>,
    for<'a> i32: Type<R::Database> + Decode<'a, R::Database>,
    for<'a> Vec<u8>: Type<R::Database> + Decode<'a, R::Database>,
    for<'a> String: Type<R::Database> + Decode<'a, R::Database>,
    for<'a> Json<Value>: Type<R::Database> + Decode<'a, R::Database>,
{
    fn from_row(row: &'r R) -> sqlx::Result<Self> {
        BlockQueryData::<Types>::from_row(row)?
            .try_into()
            .decode_error("malformed block summary")
    }
}

impl<'r, Types, R> FromRow<'r, R> for BlockDetail<Types>
where
    Types: NodeType,
    Header<Types>: BlockHeader<Types> + ExplorerHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    BalanceAmount<Types>: Into<MonetaryValue>,
    R: Row,
    for<'a> &'a str: ColumnIndex<R>,
    for<'a> i32: Type<R::Database> + Decode<'a, R::Database>,
    for<'a> Vec<u8>: Type<R::Database> + Decode<'a, R::Database>,
    for<'a> String: Type<R::Database> + Decode<'a, R::Database>,
    for<'a> Json<Value>: Type<R::Database> + Decode<'a, R::Database>,
{
    fn from_row(row: &'r R) -> sqlx::Result<Self> {
        BlockQueryData::<Types>::from_row(row)?
            .try_into()
            .decode_error("malformed block detail")
    }
}
