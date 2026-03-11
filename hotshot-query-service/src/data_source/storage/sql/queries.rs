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
use futures::stream::BoxStream;
use hotshot_types::{
    simple_certificate::QuorumCertificate2,
    traits::{block_contents::BlockHeader, node_implementation::NodeType, BlockPayload},
};
use sqlx::{Arguments, FromRow, Row};

use super::{db::DbBackend, Transaction};
use crate::{
    availability::{
        BlockId, BlockQueryData, LeafQueryData, PayloadQueryData, QueryableHeader,
        QueryablePayload, VidCommonQueryData,
    },
    data_source::storage::{PayloadMetadata, VidCommonMetadata},
    Header, Leaf2, Payload, QueryError, QueryResult,
};

pub(super) mod availability;
pub(super) mod explorer;
pub(super) mod node;
pub(super) mod state;

/// Backend-dispatched arguments for SQL queries.
pub enum BackendArguments<'a> {
    Postgres(<sqlx::Postgres as sqlx::Database>::Arguments<'a>),
    Sqlite(<sqlx::Sqlite as sqlx::Database>::Arguments<'a>),
}

impl<'a> BackendArguments<'a> {
    fn len(&self) -> usize {
        match self {
            Self::Postgres(args) => args.len(),
            Self::Sqlite(args) => args.len(),
        }
    }
}

/// A SQL query with backend-dispatched arguments. Returned by [`QueryBuilder::query`].
pub enum BackendQuery<'q> {
    Postgres(
        sqlx::query::Query<'q, sqlx::Postgres, <sqlx::Postgres as sqlx::Database>::Arguments<'q>>,
    ),
    Sqlite(sqlx::query::Query<'q, sqlx::Sqlite, <sqlx::Sqlite as sqlx::Database>::Arguments<'q>>),
}

/// Generate `fetch_one`, `fetch_optional`, `fetch_all`, and `fetch` methods for a
/// backend-dispatched query type. `$wrap_pg` and `$wrap_sq` are applied to each row returned
/// by the Postgres and Sqlite variants respectively. `fetch_where` bounds are only added to
/// the `fetch` method (which introduces its own `'e` lifetime).
macro_rules! impl_backend_fetch {
    ($Output:ty, $wrap_pg:expr, $wrap_sq:expr $(, fetch_where $($fetch_extra:tt)+)?) => {
        pub async fn fetch_one<Mode>(
            self,
            tx: &mut Transaction<Mode>,
        ) -> Result<$Output, sqlx::Error> {
            match self {
                Self::Postgres(q) => Ok($wrap_pg(q.fetch_one(tx.inner.as_postgres_mut()).await?)),
                Self::Sqlite(q) => Ok($wrap_sq(q.fetch_one(tx.inner.as_sqlite_mut()).await?)),
            }
        }

        pub async fn fetch_optional<Mode>(
            self,
            tx: &mut Transaction<Mode>,
        ) -> Result<Option<$Output>, sqlx::Error> {
            match self {
                Self::Postgres(q) => Ok(q.fetch_optional(tx.inner.as_postgres_mut()).await?.map($wrap_pg)),
                Self::Sqlite(q) => Ok(q.fetch_optional(tx.inner.as_sqlite_mut()).await?.map($wrap_sq)),
            }
        }

        pub async fn fetch_all<Mode>(
            self,
            tx: &mut Transaction<Mode>,
        ) -> Result<Vec<$Output>, sqlx::Error> {
            match self {
                Self::Postgres(q) => Ok(q
                    .fetch_all(tx.inner.as_postgres_mut())
                    .await?
                    .into_iter()
                    .map($wrap_pg)
                    .collect()),
                Self::Sqlite(q) => Ok(q
                    .fetch_all(tx.inner.as_sqlite_mut())
                    .await?
                    .into_iter()
                    .map($wrap_sq)
                    .collect()),
            }
        }

        pub fn fetch<'e, Mode>(
            self,
            tx: &'e mut Transaction<Mode>,
        ) -> BoxStream<'e, Result<$Output, sqlx::Error>>
        where
            'q: 'e,
            $($($fetch_extra)+)?
        {
            use futures::StreamExt;
            match self {
                Self::Postgres(q) => q
                    .fetch(tx.inner.as_postgres_mut())
                    .map(|r| r.map($wrap_pg))
                    .boxed(),
                Self::Sqlite(q) => q
                    .fetch(tx.inner.as_sqlite_mut())
                    .map(|r| r.map($wrap_sq))
                    .boxed(),
            }
        }
    };
}

impl<'q> BackendQuery<'q> {
    pub fn bind<T>(self, value: T) -> Self
    where
        T: 'q
            + sqlx::Encode<'q, sqlx::Postgres>
            + sqlx::Type<sqlx::Postgres>
            + sqlx::Encode<'q, sqlx::Sqlite>
            + sqlx::Type<sqlx::Sqlite>
            + Send,
    {
        match self {
            Self::Postgres(q) => Self::Postgres(q.bind(value)),
            Self::Sqlite(q) => Self::Sqlite(q.bind(value)),
        }
    }

    pub async fn execute<Mode>(self, tx: &mut Transaction<Mode>) -> Result<u64, sqlx::Error> {
        match self {
            Self::Postgres(q) => Ok(q.execute(tx.inner.as_postgres_mut()).await?.rows_affected()),
            Self::Sqlite(q) => Ok(q.execute(tx.inner.as_sqlite_mut()).await?.rows_affected()),
        }
    }

    impl_backend_fetch!(BackendRow, BackendRow::Postgres, BackendRow::Sqlite);
}

/// A SQL query-as with backend-dispatched arguments. Returned by [`QueryBuilder::query_as`].
pub enum BackendQueryAs<'q, T> {
    Postgres(
        sqlx::query::QueryAs<
            'q,
            sqlx::Postgres,
            T,
            <sqlx::Postgres as sqlx::Database>::Arguments<'q>,
        >,
    ),
    Sqlite(
        sqlx::query::QueryAs<'q, sqlx::Sqlite, T, <sqlx::Sqlite as sqlx::Database>::Arguments<'q>>,
    ),
}

impl<'q, T> BackendQueryAs<'q, T>
where
    T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    T: for<'r> FromRow<'r, sqlx::sqlite::SqliteRow>,
{
    pub fn bind<V>(self, value: V) -> Self
    where
        V: 'q
            + sqlx::Encode<'q, sqlx::Postgres>
            + sqlx::Type<sqlx::Postgres>
            + sqlx::Encode<'q, sqlx::Sqlite>
            + sqlx::Type<sqlx::Sqlite>
            + Send,
    {
        match self {
            Self::Postgres(q) => Self::Postgres(q.bind(value)),
            Self::Sqlite(q) => Self::Sqlite(q.bind(value)),
        }
    }

    impl_backend_fetch!(T, std::convert::identity, std::convert::identity, fetch_where T: 'e);
}

/// A row returned from a backend-dispatched query.
pub enum BackendRow {
    Postgres(sqlx::postgres::PgRow),
    Sqlite(sqlx::sqlite::SqliteRow),
}

impl BackendRow {
    pub fn try_get<'r, T>(&'r self, col: &str) -> sqlx::Result<T>
    where
        T: sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>,
        T: sqlx::Decode<'r, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite>,
    {
        match self {
            Self::Postgres(row) => row.try_get(col),
            Self::Sqlite(row) => row.try_get(col),
        }
    }

    pub fn get<'r, T>(&'r self, col: &str) -> T
    where
        T: sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>,
        T: sqlx::Decode<'r, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite>,
    {
        match self {
            Self::Postgres(row) => row.get(col),
            Self::Sqlite(row) => row.get(col),
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn from_row<T>(&self) -> sqlx::Result<T>
    where
        T: for<'r> FromRow<'r, sqlx::postgres::PgRow>,
        T: for<'r> FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        match self {
            Self::Postgres(row) => T::from_row(row),
            Self::Sqlite(row) => T::from_row(row),
        }
    }
}

pub fn query(backend: DbBackend, sql: &str) -> BackendQuery<'_> {
    match backend {
        DbBackend::Postgres => BackendQuery::Postgres(sqlx::query(sql)),
        DbBackend::Sqlite => BackendQuery::Sqlite(sqlx::query(sql)),
    }
}

pub fn query_as<'q, T>(backend: DbBackend, sql: &'q str) -> BackendQueryAs<'q, T>
where
    T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    T: for<'r> FromRow<'r, sqlx::sqlite::SqliteRow>,
{
    match backend {
        DbBackend::Postgres => BackendQueryAs::Postgres(sqlx::query_as(sql)),
        DbBackend::Sqlite => BackendQueryAs::Sqlite(sqlx::query_as(sql)),
    }
}

/// Helper type for programmatically constructing queries.
///
/// This type can be used to bind arguments of various types. The arguments are bound *first* and
/// the SQL statement is given last. Each time an argument is bound, a SQL fragment is returned as
/// a string which can be used to represent that argument in the statement (e.g. `$1` for the first
/// argument bound). This makes it easier to programmatically construct queries where the statement
/// is not a compile time constant.
///
/// # Example
///
/// ```ignore
/// use hotshot_query_service::{
///     data_source::storage::sql::{QueryBuilder, Transaction},
///     QueryResult,
/// };
///
/// async fn search_and_maybe_filter<Mode>(
///     tx: &mut Transaction<Mode>,
///     id: Option<i64>,
/// ) -> QueryResult<Vec<BackendRow>> {
///     let mut query = QueryBuilder::new(tx.backend());
///     let mut sql = "SELECT * FROM table".to_string();
///     if let Some(id) = id {
///         sql = format!("{sql} WHERE id = {}", query.bind(id)?);
///     }
///     let results = query
///         .query(&sql)
///         .fetch_all(tx)
///         .await?;
///     Ok(results)
/// }
/// ```
#[derive(Derivative)]
#[derivative(Debug)]
pub struct QueryBuilder<'a> {
    #[derivative(Debug = "ignore")]
    arguments: BackendArguments<'a>,
}

impl<'a> QueryBuilder<'a> {
    pub fn new(backend: DbBackend) -> Self {
        let arguments = match backend {
            DbBackend::Postgres => BackendArguments::Postgres(Default::default()),
            DbBackend::Sqlite => BackendArguments::Sqlite(Default::default()),
        };
        Self { arguments }
    }
}

impl<'q> QueryBuilder<'q> {
    /// Add an argument and return its name as a formal parameter in a SQL prepared statement.
    pub fn bind<T>(&mut self, arg: T) -> QueryResult<String>
    where
        T: 'q
            + sqlx::Encode<'q, sqlx::Postgres>
            + sqlx::Type<sqlx::Postgres>
            + sqlx::Encode<'q, sqlx::Sqlite>
            + sqlx::Type<sqlx::Sqlite>,
    {
        match &mut self.arguments {
            BackendArguments::Postgres(args) => {
                args.add(arg).map_err(|err| QueryError::Error {
                    message: format!("{err:#}"),
                })?;
            },
            BackendArguments::Sqlite(args) => {
                args.add(arg).map_err(|err| QueryError::Error {
                    message: format!("{err:#}"),
                })?;
            },
        }

        Ok(format!("${}", self.arguments.len()))
    }

    /// Finalize the query with a constructed SQL statement.
    pub fn query(self, sql: &'q str) -> BackendQuery<'q> {
        match self.arguments {
            BackendArguments::Postgres(args) => BackendQuery::Postgres(sqlx::query_with(sql, args)),
            BackendArguments::Sqlite(args) => BackendQuery::Sqlite(sqlx::query_with(sql, args)),
        }
    }

    /// Finalize the query with a constructed SQL statement and a specified output type.
    pub fn query_as<T>(self, sql: &'q str) -> BackendQueryAs<'q, T>
    where
        T: for<'r> FromRow<'r, sqlx::postgres::PgRow>,
        T: for<'r> FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        match self.arguments {
            BackendArguments::Postgres(args) => {
                BackendQueryAs::Postgres(sqlx::query_as_with(sql, args))
            },
            BackendArguments::Sqlite(args) => {
                BackendQueryAs::Sqlite(sqlx::query_as_with(sql, args))
            },
        }
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

macro_rules! impl_from_row_for_both {
    ($ty:ty, |$row:ident| $body:expr $(, where $($bounds:tt)+)?) => {
        impl<'r, Types> FromRow<'r, sqlx::postgres::PgRow> for $ty
        where
            Types: NodeType,
            $($($bounds)+)?
        {
            fn from_row($row: &'r sqlx::postgres::PgRow) -> sqlx::Result<Self> {
                $body
            }
        }

        impl<'r, Types> FromRow<'r, sqlx::sqlite::SqliteRow> for $ty
        where
            Types: NodeType,
            $($($bounds)+)?
        {
            fn from_row($row: &'r sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
                $body
            }
        }
    };
}

impl_from_row_for_both!(LeafQueryData<Types>, |row| {
    let leaf = row.try_get("leaf")?;
    let leaf: Leaf2<Types> = serde_json::from_value(leaf).decode_error("malformed leaf")?;

    let qc = row.try_get("qc")?;
    let qc: QuorumCertificate2<Types> = serde_json::from_value(qc).decode_error("malformed QC")?;

    Ok(Self { leaf, qc })
});

const BLOCK_COLUMNS: &str =
    "h.hash AS hash, h.data AS header_data, p.size AS payload_size, p.data AS payload_data";

impl_from_row_for_both!(BlockQueryData<Types>, |row| {
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
}, where
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
);

const PAYLOAD_COLUMNS: &str = BLOCK_COLUMNS;

impl_from_row_for_both!(PayloadQueryData<Types>, |row| {
    BlockQueryData::<Types>::from_row(row).map(Self::from)
}, where
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
);

const PAYLOAD_METADATA_COLUMNS: &str = "h.height AS height, h.hash AS hash, h.payload_hash AS \
                                        payload_hash, p.size AS payload_size, p.num_transactions \
                                        AS num_transactions";

impl_from_row_for_both!(PayloadMetadata<Types>, |row| {
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
}, where
    Header<Types>: QueryableHeader<Types>,
);

const VID_COMMON_COLUMNS: &str = "h.height AS height, h.hash AS block_hash, h.payload_hash AS \
                                  payload_hash, v.common AS common_data";

impl_from_row_for_both!(VidCommonQueryData<Types>, |row| {
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
}, where
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
);

const VID_COMMON_METADATA_COLUMNS: &str =
    "h.height AS height, h.hash AS block_hash, h.payload_hash AS payload_hash";

impl_from_row_for_both!(VidCommonMetadata<Types>, |row| {
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
}, where
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
);

const HEADER_COLUMNS: &str = "h.data AS data";

// We can't implement `FromRow` for `Header<Types>` since `Header<Types>` is not actually a type
// defined in this crate; it's just an alias for `Types::BlockHeader`. So this standalone function
// will have to do.
fn parse_header<Types>(row: BackendRow) -> sqlx::Result<Header<Types>>
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
        let mut query = QueryBuilder::new(self.backend());
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

        let row = query.query(&sql).fetch_one(self).await?;
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
