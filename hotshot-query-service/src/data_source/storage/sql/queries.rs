#![allow(clippy::needless_lifetimes)]
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

//! Immutable query functionality of a SQL database.

use std::ops::{Bound, RangeBounds};

use derivative::Derivative;
pub(super) use hotshot_query_service_types::availability::sql::DecodeError;
use hotshot_query_service_types::availability::sql::{
    BLOCK_COLUMNS, HEADER_COLUMNS, LEAF_COLUMNS, PAYLOAD_COLUMNS, PAYLOAD_METADATA_COLUMNS,
    VID_COMMON_COLUMNS, VID_COMMON_METADATA_COLUMNS, parse_header,
};
use hotshot_types::traits::node_implementation::NodeType;
use sqlx::{Arguments, FromRow};

use super::{Database, Db, Query, QueryAs, Transaction};
use crate::{Header, QueryError, QueryResult, availability::BlockId};

pub(super) mod availability;
pub(super) mod explorer;
pub(super) mod node;
pub(super) mod state;

/// Helper type for programmatically constructing queries.
///
/// This type can be used to bind arguments of various types, similar to [`Query`] or [`QueryAs`].
/// With [`QueryBuilder`], though, the arguments are bound *first* and the SQL statement is given
/// last. Each time an argument is bound, a SQL fragment is returned as a string which can be used
/// to represent that argument in the statement (e.g. `$1` for the first argument bound). This makes
/// it easier to programmatically construct queries where the statement is not a compile time
/// constant.
///
/// # Example
///
/// ```
/// # use hotshot_query_service::{
/// #   data_source::storage::sql::{
/// #       Database, Db, QueryBuilder, Transaction,
/// #   },
/// #   QueryResult,
/// # };
/// # use sqlx::FromRow;
/// async fn search_and_maybe_filter<T, Mode>(
///     tx: &mut Transaction<Mode>,
///     id: Option<i64>,
/// ) -> QueryResult<Vec<T>>
/// where
///     for<'r> T: FromRow<'r, <Db as Database>::Row> + Send + Unpin,
/// {
///     let mut query = QueryBuilder::default();
///     let mut sql = "SELECT * FROM table".into();
///     if let Some(id) = id {
///         sql = format!("{sql} WHERE id = {}", query.bind(id)?);
///     }
///     let results = query
///         .query_as(&sql)
///         .fetch_all(tx.as_mut())
///         .await?;
///     Ok(results)
/// }
/// ```
#[derive(Derivative, Default)]
#[derivative(Debug)]
pub struct QueryBuilder<'a> {
    #[derivative(Debug = "ignore")]
    arguments: <Db as Database>::Arguments<'a>,
}

impl<'q> QueryBuilder<'q> {
    /// Add an argument and return its name as a formal parameter in a SQL prepared statement.
    pub fn bind<T>(&mut self, arg: T) -> QueryResult<String>
    where
        T: 'q + sqlx::Encode<'q, Db> + sqlx::Type<Db>,
    {
        self.arguments.add(arg).map_err(|err| QueryError::Error {
            message: format!("{err:#}"),
        })?;

        Ok(format!("${}", self.arguments.len()))
    }

    /// Finalize the query with a constructed SQL statement.
    pub fn query(self, sql: &'q str) -> Query<'q> {
        sqlx::query_with(sql, self.arguments)
    }

    /// Finalize the query with a constructed SQL statement and a specified output type.
    pub fn query_as<T>(self, sql: &'q str) -> QueryAs<'q, T>
    where
        T: for<'r> FromRow<'r, <Db as Database>::Row>,
    {
        sqlx::query_as_with(sql, self.arguments)
    }
}

impl QueryBuilder<'_> {
    /// Construct a SQL `WHERE` clause which filters for a header exactly matching `id`.
    pub fn header_where_clause<Types: NodeType>(
        &mut self,
        id: BlockId<Types>,
    ) -> QueryResult<String> {
        let clause = match id {
            BlockId::Number(n) => format!("h.height = {}", self.bind(n as i64)?),
            BlockId::Hash(h) => format!("h.hash = {}", self.bind(h.to_string())?),
            BlockId::PayloadHash(h) => format!("h.payload_hash = {}", self.bind(h.to_string())?),
        };
        Ok(clause)
    }

    /// Convert range bounds to a SQL `WHERE` clause constraining a given column.
    pub fn bounds_to_where_clause<R>(&mut self, range: R, column: &str) -> QueryResult<String>
    where
        R: RangeBounds<usize>,
    {
        let mut bounds = vec![];

        match range.start_bound() {
            Bound::Included(n) => {
                bounds.push(format!("{column} >= {}", self.bind(*n as i64)?));
            },
            Bound::Excluded(n) => {
                bounds.push(format!("{column} > {}", self.bind(*n as i64)?));
            },
            Bound::Unbounded => {},
        }
        match range.end_bound() {
            Bound::Included(n) => {
                bounds.push(format!("{column} <= {}", self.bind(*n as i64)?));
            },
            Bound::Excluded(n) => {
                bounds.push(format!("{column} < {}", self.bind(*n as i64)?));
            },
            Bound::Unbounded => {},
        }

        let mut where_clause = bounds.join(" AND ");
        if !where_clause.is_empty() {
            where_clause = format!(" WHERE {where_clause}");
        }

        Ok(where_clause)
    }
}

impl<Mode> Transaction<Mode> {
    /// Load a header from storage.
    ///
    /// This function is similar to `AvailabilityStorage::get_header`, but
    /// * does not require the `QueryablePayload<Types>` bound that that trait impl does
    /// * makes it easier to specify types since the type parameter is on the function and not on a
    ///   trait impl
    /// * allows type conversions for the `id` parameter
    ///
    /// This more ergonomic interface is useful as loading headers is important for many SQL storage
    /// functions, not just the `AvailabilityStorage` interface.
    pub async fn load_header<Types: NodeType>(
        &mut self,
        id: impl Into<BlockId<Types>> + Send,
    ) -> QueryResult<Header<Types>> {
        let mut query = QueryBuilder::default();
        let where_clause = query.header_where_clause(id.into())?;
        // ORDER BY h.height ASC ensures that if there are duplicate blocks (this can happen when
        // selecting by payload ID, as payloads are not unique), we return the first one.
        let sql = format!(
            "SELECT {HEADER_COLUMNS}
               FROM header AS h
              WHERE {where_clause}
              ORDER BY h.height
              LIMIT 1"
        );

        let row = query.query(&sql).fetch_one(self.as_mut()).await?;
        let header = parse_header::<Types, <Db as Database>::Row>(row)?;

        Ok(header)
    }
}
