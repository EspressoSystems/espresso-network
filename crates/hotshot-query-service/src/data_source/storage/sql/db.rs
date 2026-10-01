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

/// The underlying database type for a SQL data source.
///
/// Currently, only PostgreSQL and SQLite are supported, with selection based on the "embedded-db" feature flag.
/// - When the "embedded-db" feature is enabled, SQLite is used.
/// - When it’s disabled, PostgreSQL is used.
///
/// ### Design Choice
/// The reason for taking this approach over sqlx's Any database is that we can support SQL types
/// which are implemented for the two backends we care about (Postgres and SQLite) but not for _any_
/// SQL database, such as MySQL. Crucially, JSON types fall in this category.
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
#[cfg(feature = "embedded-db")]
pub type Db = sqlx::Sqlite;
#[cfg(not(feature = "embedded-db"))]
pub type Db = sqlx::Postgres;

#[cfg(feature = "embedded-db")]
pub mod syntax_helpers {
    pub const MAX_FN: &str = "MAX";
    pub const BINARY_TYPE: &str = "BLOB";
}

#[cfg(not(feature = "embedded-db"))]
pub mod syntax_helpers {
    pub const MAX_FN: &str = "GREATEST";
    pub const BINARY_TYPE: &str = "BYTEA";
}
