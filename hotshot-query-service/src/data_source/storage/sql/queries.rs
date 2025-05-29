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

use std::{
    fmt::Display,
    ops::{Bound, RangeBounds},
};

use anyhow::Context;
use derivative::Derivative;
use hotshot_types::{
    simple_certificate::{LightClientStateUpdateCertificate, QuorumCertificate2},
    traits::{
        block_contents::{BlockHeader, BlockPayload},
        node_implementation::NodeType,
    },
};
use sqlx::{Arguments, FromRow, Row};

use super::{Database, Db, Query, QueryAs, Transaction};
use crate::{
    availability::{
        BlockId, BlockQueryData, LeafQueryData, PayloadQueryData, QueryablePayload,
        StateCertQueryData, VidCommonQueryData,
    },
    data_source::storage::{PayloadMetadata, VidCommonMetadata},
    Header, Leaf2, Payload, QueryError, QueryResult,
};

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

const LEAF_COLUMNS: &str = "leaf, qc";

impl<'r, Types> FromRow<'r, <Db as Database>::Row> for LeafQueryData<Types>
where
    Types: NodeType,
{
    fn from_row(row: &'r <Db as Database>::Row) -> sqlx::Result<Self> {
        let leaf = row.try_get("leaf")?;
        let leaf: Leaf2<Types> = serde_json::from_value(leaf).decode_error("malformed leaf")?;

        let qc = row.try_get("qc")?;
        let qc: QuorumCertificate2<Types> =
            serde_json::from_value(qc).decode_error("malformed QC")?;

        Ok(Self { leaf, qc })
    }
}

const BLOCK_COLUMNS: &str =
    "h.hash AS hash, h.data AS header_data, p.size AS payload_size, p.data AS payload_data";

impl<'r, Types> FromRow<'r, <Db as Database>::Row> for BlockQueryData<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    fn from_row(row: &'r <Db as Database>::Row) -> sqlx::Result<Self> {
        // First, check if we have the payload for this block yet.
        let size: Option<i32> = row.try_get("payload_size")?;
        let payload_data: Option<Vec<u8>> = row.try_get("payload_data")?;
        let (size, payload_data) = size.zip(payload_data).ok_or(sqlx::Error::RowNotFound)?;
        let size = size as u64;

        // Reconstruct the full header.
        let header_data = row.try_get("header_data")?;
        let header: Header<Types> =
            serde_json::from_value(header_data).decode_error("malformed header")?;

        // Reconstruct the full block payload.
        let payload = Payload::<Types>::from_bytes(&payload_data, header.metadata());

        // Reconstruct the query data by adding metadata.
        let hash: String = row.try_get("hash")?;
        let hash = hash.parse().decode_error("malformed block hash")?;

        Ok(Self {
            num_transactions: payload.len(header.metadata()) as u64,
            header,
            payload,
            size,
            hash,
        })
    }
}

const PAYLOAD_COLUMNS: &str = BLOCK_COLUMNS;

impl<'r, Types> FromRow<'r, <Db as Database>::Row> for PayloadQueryData<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    fn from_row(row: &'r <Db as Database>::Row) -> sqlx::Result<Self> {
        <BlockQueryData<Types> as FromRow<<Db as Database>::Row>>::from_row(row).map(Self::from)
    }
}

const PAYLOAD_METADATA_COLUMNS: &str =
    "h.height AS height, h.hash AS hash, h.payload_hash AS payload_hash, p.size AS payload_size, p.num_transactions AS num_transactions";

impl<'r, Types> FromRow<'r, <Db as Database>::Row> for PayloadMetadata<Types>
where
    Types: NodeType,
{
    fn from_row(row: &'r <Db as Database>::Row) -> sqlx::Result<Self> {
        Ok(Self {
            height: row.try_get::<i64, _>("height")? as u64,
            block_hash: row
                .try_get::<String, _>("hash")?
                .parse()
                .decode_error("malformed block hash")?,
            hash: row
                .try_get::<String, _>("payload_hash")?
                .parse()
                .decode_error("malformed payload hash")?,
            size: row
                .try_get::<Option<i32>, _>("payload_size")?
                .ok_or(sqlx::Error::RowNotFound)? as u64,
            num_transactions: row
                .try_get::<Option<i32>, _>("num_transactions")?
                .ok_or(sqlx::Error::RowNotFound)? as u64,

            // Per-namespace info must be loaded in a separate query.
            namespaces: Default::default(),
        })
    }
}

const VID_COMMON_COLUMNS: &str = "h.height AS height, h.hash AS block_hash, h.payload_hash AS payload_hash, v.common AS common_data";

impl<'r, Types> FromRow<'r, <Db as Database>::Row> for VidCommonQueryData<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    fn from_row(row: &'r <Db as Database>::Row) -> sqlx::Result<Self> {
        let height = row.try_get::<i64, _>("height")? as u64;
        let block_hash: String = row.try_get("block_hash")?;
        let block_hash = block_hash.parse().decode_error("malformed block hash")?;
        let payload_hash: String = row.try_get("payload_hash")?;
        let payload_hash = payload_hash
            .parse()
            .decode_error("malformed payload hash")?;
        let common_data: Vec<u8> = row.try_get("common_data")?;
        let common =
            bincode::deserialize(&common_data).decode_error("malformed VID common data")?;
        Ok(Self {
            height,
            block_hash,
            payload_hash,
            common,
        })
    }
}

const VID_COMMON_METADATA_COLUMNS: &str =
    "h.height AS height, h.hash AS block_hash, h.payload_hash AS payload_hash";

impl<'r, Types> FromRow<'r, <Db as Database>::Row> for VidCommonMetadata<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    fn from_row(row: &'r <Db as Database>::Row) -> sqlx::Result<Self> {
        let height = row.try_get::<i64, _>("height")? as u64;
        let block_hash: String = row.try_get("block_hash")?;
        let block_hash = block_hash.parse().decode_error("malformed block hash")?;
        let payload_hash: String = row.try_get("payload_hash")?;
        let payload_hash = payload_hash
            .parse()
            .decode_error("malformed payload hash")?;
        Ok(Self {
            height,
            block_hash,
            payload_hash,
        })
    }
}

const HEADER_COLUMNS: &str = "h.data AS data";

// We can't implement `FromRow` for `Header<Types>` since `Header<Types>` is not actually a type
// defined in this crate; it's just an alias for `Types::BlockHeader`. So this standalone function
// will have to do.
fn parse_header<Types>(row: <Db as Database>::Row) -> sqlx::Result<Header<Types>>
where
    Types: NodeType,
{
    // Reconstruct the full header.
    let data = row.try_get("data")?;
    serde_json::from_value(data).decode_error("malformed header")
}

impl From<sqlx::Error> for QueryError {
    fn from(err: sqlx::Error) -> Self {
        if matches!(err, sqlx::Error::RowNotFound) {
            Self::NotFound
        } else {
            Self::Error {
                message: err.to_string(),
            }
        }
    }
}

const STATE_CERT_COLUMNS: &str = "state_cert";

impl<'r, Types> FromRow<'r, <Db as Database>::Row> for StateCertQueryData<Types>
where
    Types: NodeType,
{
    fn from_row(row: &'r <Db as Database>::Row) -> sqlx::Result<Self> {
        let state_cert: LightClientStateUpdateCertificate<Types> =
            bincode::deserialize(row.try_get("state_cert")?)
                .decode_error("malformed state cert")?;
        Ok(state_cert.into())
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
        let header = parse_header::<Types>(row)?;

        Ok(header)
    }
}

pub(super) trait DecodeError {
    type Ok;
    fn decode_error(self, msg: impl Display) -> sqlx::Result<Self::Ok>;
}

impl<T, E> DecodeError for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    type Ok = T;
    fn decode_error(self, msg: impl Display) -> sqlx::Result<<Self as DecodeError>::Ok> {
        self.context(msg.to_string())
            .map_err(|err| sqlx::Error::Decode(err.into()))
    }
}
