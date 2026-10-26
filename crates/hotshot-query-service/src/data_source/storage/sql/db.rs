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

use std::{collections::HashMap, fmt::Debug, future::Future, str::FromStr};

use futures::stream::TryStreamExt;
use hotshot_types::traits::node_implementation::NodeType;
use sqlx::{
    ConnectOptions, Database, Row as _,
    pool::{Pool, PoolOptions},
};

use super::{
    Db, Migration, QueryBuilder, Transaction, Write,
    queries::state::{Node, build_hash_batch_insert},
};
use crate::{QueryResult, merklized_state::MerklizedState};

/// The database a [`Backend`] runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DbKind {
    Postgres,
    Sqlite,
}

/// The database a copy of the SQL storage runs on.
///
/// The SQL storage is compiled once per backend, with [`Db`] naming that backend's
/// [`sqlx::Database`], and each backend implements this trait for its own [`Db`]. Code branches on
/// [`EMBEDDED`](Self::EMBEDDED) where both branches compile for either database, and calls a
/// method of this trait where a branch would only compile for one.
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
///
/// Code matches on [`KIND`](Self::KIND) where both arms compile for either database, and calls a
/// method of this trait where an arm would only compile for one. Each backend implements it in its
/// own module.
pub trait Backend: Database {
    type ConnectOptions: ConnectOptions<Connection = <Self as Database>::Connection>
        + Clone
        + Debug
        + FromStr<Err = sqlx::Error>;

    const KIND: DbKind;

    /// The SQL function returning the greater of its arguments.
    const MAX_FN: &'static str;

    /// The connection options [`Config::default`](super::Config) starts from.
    fn default_connect_options() -> Self::ConnectOptions;

    /// The migrations that build the default schema, in no particular order.
    fn migrations() -> Vec<Migration>;

    /// Delete the database file, for a backend that keeps the database in one.
    fn delete_file(opts: &Self::ConnectOptions) -> std::io::Result<()>;

    /// Hooks every pooled connection needs on this backend, for a database using `schema`.
    fn session_hooks(pool: PoolOptions<Self>, schema: String) -> PoolOptions<Self>;

    /// Log the sessions and locks behind a serialization conflict in `op`, in the background.
    fn log_conflict_sessions(pool: Pool<Self>, op: &'static str);
}

/// Merklized state queries with a portable form and a faster form that only Postgres can run.
///
/// The provided methods are the portable form, written against [`Db`] rather than `Self` so they
/// type check in every copy of the SQL storage. Postgres overrides them.
pub(crate) trait StateQueries: Backend {
    fn hashes_by_id(
        conn: &mut <Db as Database>::Connection,
        ids: Vec<i64>,
    ) -> impl Future<Output = QueryResult<HashMap<i64, Vec<u8>>>> + Send {
        async move {
            let (query, sql) = super::build_where_in("SELECT id, value FROM hash", "id", ids)?;
            Ok(query
                .query_as(&sql)
                .fetch(conn)
                .try_collect::<HashMap<i64, Vec<u8>>>()
                .await?)
        }
    }

    fn insert_hashes(
        tx: &mut Transaction<Write>,
        hashes: Vec<Vec<u8>>,
    ) -> impl Future<Output = QueryResult<HashMap<Vec<u8>, i64>>> + Send {
        async move {
            let mut hash_ids = HashMap::with_capacity(hashes.len());
            for hash_chunk in hashes.chunks(20) {
                let (query, sql) = build_hash_batch_insert(hash_chunk)?;
                let chunk_ids: HashMap<Vec<u8>, i64> = query
                    .query_as(&sql)
                    .fetch(tx.as_mut())
                    .try_collect()
                    .await?;
                hash_ids.extend(chunk_ids);
            }
            Ok(hash_ids)
        }
    }

    fn upsert_nodes<Types, State, const ARITY: usize>(
        nodes: Vec<Node>,
        tx: &mut Transaction<Write>,
    ) -> impl Future<Output = anyhow::Result<()>> + Send
    where
        Types: NodeType,
        State: MerklizedState<Types, ARITY>,
    {
        async move {
            let name = State::state_type();
            for node_chunk in nodes.chunks(20) {
                let rows: Vec<_> = node_chunk
                    .iter()
                    .map(|n| {
                        let children_bitvec: Option<String> = n
                            .children_bitvec
                            .clone()
                            .map(|b| b.iter().map(|bit| if bit { '1' } else { '0' }).collect());

                        (
                            n.path.clone(),
                            n.created,
                            n.hash_id,
                            n.children.clone(),
                            children_bitvec,
                            n.idx.clone(),
                            n.entry.clone(),
                        )
                    })
                    .collect();

                tx.upsert(
                    name,
                    [
                        "path",
                        "created",
                        "hash_id",
                        "children",
                        "children_bitvec",
                        "idx",
                        "entry",
                    ],
                    ["path", "created"],
                    rows,
                )
                .await?;
            }
            Ok(())
        }
    }

    fn node_from_row(row: <Db as Database>::Row) -> Node {
        let bit_string: Option<String> = row.get_unchecked("children_bitvec");
        let children_bitvec = bit_string.map(|b| b.chars().map(|c| c == '1').collect());

        Node {
            path: row.get_unchecked("path"),
            created: row.get_unchecked("created"),
            hash_id: row.get_unchecked("hash_id"),
            children: row.get_unchecked("children"),
            children_bitvec,
            idx: row.get_unchecked("idx"),
            entry: row.get_unchecked("entry"),
        }
    }

    /// Build a query selecting, for each of `paths`, the newest node at that path created at or
    /// before `created`, ordered from the leaf to the root.
    fn get_path_query<'q, Types, State, const ARITY: usize>(
        paths: Vec<serde_json::Value>,
        created: i64,
    ) -> QueryResult<(QueryBuilder<'q>, String)>
    where
        Types: NodeType,
        State: MerklizedState<Types, ARITY>,
    {
        let table = State::state_type();
        let mut query = QueryBuilder::default();
        let created_param = query.bind(created)?;
        // SQLite has no native array binding, so we keep the UNION form. Each subquery is a point
        // lookup by the composite (path, created) primary key.
        let mut sub_queries = Vec::with_capacity(paths.len());
        for path in paths {
            let node_path = query.bind(path)?;
            sub_queries.push(format!(
                "SELECT * FROM (SELECT * FROM {table} WHERE path = {node_path} AND created <= \
                 {created_param} ORDER BY path DESC, created DESC LIMIT 1) AS latest_node",
            ));
        }
        let sql = format!(
            "SELECT * FROM ({}) as t ORDER BY length(t.path) DESC",
            sub_queries.join(" UNION ")
        );
        Ok((query, sql))
    }
}
