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

use sqlx::pool::Pool;

/// Identifies which concrete database backend is in use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DbBackend {
    Postgres,
    Sqlite,
}

/// A connection pool that dispatches to either PostgreSQL or SQLite at runtime.
///
/// Currently, only PostgreSQL and SQLite are supported. The backend is selected at runtime based on
/// the connection URL, with enum variants dispatching to the concrete sqlx pool types.
///
/// ### Design Choice
/// The reason for using concrete backend types (dispatched via enum) over sqlx's Any database is
/// that we can support SQL types which are implemented for the two backends we care about (Postgres
/// and SQLite) but not for _any_ SQL database, such as MySQL. Crucially, JSON types fall in this
/// category.
///
/// The reason for taking this approach rather than writing all of our code to be generic over the
/// Database implementation is that sqlx does not have the necessary trait bounds on all of the
/// associated types (e.g. Database::Connection does not implement Executor for all possible
/// databases, the Executor impl lives on each concrete connection type) and Rust does not provide
/// a good way of encapsulating a collection of trait bounds on associated types. Thus, our function
/// signatures become untenably messy with bounds like
///
/// ```rust
/// # use sqlx::{Database, Encode, Executor, IntoArguments, Type};
/// fn foo<DB: Database>()
/// where
///     for<'a> &'a mut DB::Connection: Executor<'a>,
///     for<'q> DB::Arguments<'q>: IntoArguments<'q, DB>,
///     for<'a> i64: Type<DB> + Encode<'a, DB>,
/// {}
/// ```
///
/// Instead, we use concrete types for each backend and dispatch at runtime via this enum and the
/// corresponding [`BackendTransaction`] and [`BackendPoolConnection`] enums. The [`with_backend!`]
/// macro provides ergonomic access to the inner concrete transaction type.
#[derive(Clone, Debug)]
pub enum SqlPool {
    Postgres(Pool<sqlx::Postgres>),
    Sqlite(Pool<sqlx::Sqlite>),
}

/// An active database transaction, dispatching to the concrete backend transaction type.
///
/// See [`SqlPool`] for why we use concrete types rather than `sqlx::Any` or generics.
pub enum BackendTransaction {
    Postgres(sqlx::Transaction<'static, sqlx::Postgres>),
    Sqlite(sqlx::Transaction<'static, sqlx::Sqlite>),
}

impl std::fmt::Debug for BackendTransaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Postgres(_) => f.debug_tuple("Postgres").finish(),
            Self::Sqlite(_) => f.debug_tuple("Sqlite").finish(),
        }
    }
}

/// Backend-specific SQL syntax differences (e.g. function names, type names).
pub struct SyntaxHelpers {
    pub max_fn: &'static str,
    pub binary_type: &'static str,
}

pub mod syntax_helpers {
    pub const POSTGRES: super::SyntaxHelpers = super::SyntaxHelpers {
        max_fn: "GREATEST",
        binary_type: "BYTEA",
    };

    pub const SQLITE: super::SyntaxHelpers = super::SyntaxHelpers {
        max_fn: "MAX",
        binary_type: "BLOB",
    };

    pub static MAX_FN: &str = "GREATEST";
    pub static BINARY_TYPE: &str = "BYTEA";
}

impl SqlPool {
    pub fn backend(&self) -> DbBackend {
        match self {
            Self::Postgres(_) => DbBackend::Postgres,
            Self::Sqlite(_) => DbBackend::Sqlite,
        }
    }

    pub fn syntax(&self) -> &'static SyntaxHelpers {
        match self {
            Self::Postgres(_) => &syntax_helpers::POSTGRES,
            Self::Sqlite(_) => &syntax_helpers::SQLITE,
        }
    }

    pub async fn begin(&self) -> anyhow::Result<BackendTransaction> {
        match self {
            Self::Postgres(pool) => {
                let tx = pool.begin().await?;
                Ok(BackendTransaction::Postgres(tx))
            },
            Self::Sqlite(pool) => {
                let tx = pool.begin().await?;
                Ok(BackendTransaction::Sqlite(tx))
            },
        }
    }

    pub async fn acquire(&self) -> anyhow::Result<BackendPoolConnection> {
        match self {
            Self::Postgres(pool) => {
                let conn = pool.acquire().await?;
                Ok(BackendPoolConnection::Postgres(conn))
            },
            Self::Sqlite(pool) => {
                let conn = pool.acquire().await?;
                Ok(BackendPoolConnection::Sqlite(conn))
            },
        }
    }
}

/// A pooled database connection, dispatching to the concrete backend connection type.
///
/// See [`SqlPool`] for why we use concrete types rather than `sqlx::Any` or generics.
pub enum BackendPoolConnection {
    Postgres(sqlx::pool::PoolConnection<sqlx::Postgres>),
    Sqlite(sqlx::pool::PoolConnection<sqlx::Sqlite>),
}

impl BackendTransaction {
    pub fn backend(&self) -> DbBackend {
        match self {
            Self::Postgres(_) => DbBackend::Postgres,
            Self::Sqlite(_) => DbBackend::Sqlite,
        }
    }

    pub async fn commit(self) -> anyhow::Result<()> {
        match self {
            Self::Postgres(tx) => tx.commit().await?,
            Self::Sqlite(tx) => tx.commit().await?,
        }
        Ok(())
    }

    pub async fn rollback(self) -> anyhow::Result<()> {
        match self {
            Self::Postgres(tx) => tx.rollback().await?,
            Self::Sqlite(tx) => tx.rollback().await?,
        }
        Ok(())
    }
}

/// Dispatch on the concrete backend transaction type.
///
/// Provides a binding to the inner `sqlx::Transaction` regardless of which backend is active,
/// allowing callers to write backend-agnostic code that still operates on concrete types.
macro_rules! with_backend {
    ($self:expr, |$tx:ident| $body:expr) => {
        match &mut $self.inner {
            $crate::data_source::storage::sql::db::BackendTransaction::Postgres($tx) => $body,
            $crate::data_source::storage::sql::db::BackendTransaction::Sqlite($tx) => $body,
        }
    };
}

pub(crate) use with_backend;
