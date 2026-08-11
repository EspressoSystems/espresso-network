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

//! Axum router serving the merklized-state API wire protocol.
//!
//! Route paths, response forms, status codes and the wire error envelope (the crate-level
//! [`Error`](crate::Error)) match the old tide-disco handlers and the `state.toml` route specs, so
//! existing clients keep working unchanged. One exception, deliberate: an unparseable `{key}` is a
//! 400, not the 500 the tide handler served, since a malformed path parameter is not a server
//! fault.
//!
//! The router is an [`ApiRouter`] so that the OpenAPI documentation travels with the routes:
//! an application mounting this module gets the summaries and descriptions without restating
//! them. Use [`From`] to get a plain [`Router`] where the docs are not wanted.

use std::sync::Arc;

use aide::axum::{ApiRouter, routing::get_with};
use axum::{
    Router,
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
    routing::get,
};
use disco_types::{request::RequestError, status::StatusCode};
use hotshot_types::traits::node_implementation::NodeType;
use http_wire::{self as wire, body_limit_layer, cors_layer, healthcheck_response};
use jf_merkle_tree_compat::prelude::MerkleProof;
use serde::Serialize;
use tagged_base64::TaggedBase64;

use super::{
    Error, MerklizedState, MerklizedStateDataSource, MerklizedStateHeightPersistence, Options,
    Snapshot,
};
use crate::Error as AppError;

/// One merklized tree's routes: the Merkle path to a key in a snapshot of the tree, addressed
/// either by block height or by tree commitment, and the height of the latest snapshot stored.
///
/// `Tree` selects which tree the mount serves, so an application with several merklized trees
/// instantiates this once per tree and nests each at its own prefix. The height route reads the
/// storage-wide merklized-state height, which advances for all trees together, so every mount
/// reports the same number.
pub fn merklized_state_router<Types, Tree, S, const ARITY: usize>(
    options: &Options,
    data_source: S,
) -> ApiRouter
where
    Types: NodeType,
    Tree: MerklizedState<Types, ARITY>,
    S: MerklizedStateDataSource<Types, Tree, ARITY>
        + MerklizedStateHeightPersistence
        + Send
        + Sync
        + 'static,
{
    let tree = Tree::state_type();
    ApiRouter::new()
        .api_route(
            "/{height}/{key}",
            get_with(get_path_by_height::<Types, Tree, S, ARITY>, move |op| {
                op.summary(&format!("Get a {tree} Merkle path by height"))
                    .description(
                        "Get the Merkle path proving membership of the entry at `key` in the \
                         snapshot of the tree taken at block `height`. The path is absent if the \
                         snapshot is incomplete, which happens when any parent state is missing.",
                    )
            }),
        )
        .api_route(
            "/commit/{commit}/{key}",
            get_with(get_path_by_commit::<Types, Tree, S, ARITY>, move |op| {
                op.summary(&format!("Get a {tree} Merkle path by commitment"))
                    .description(
                        "Get the Merkle path proving membership of the entry at `key` in the \
                         snapshot of the tree with the given commitment.",
                    )
            }),
        )
        .api_route(
            "/block-height",
            get_with(get_height::<S>, move |op| {
                op.summary(&format!("Get the latest {tree} snapshot height"))
                    .description(
                        "Get the latest block height for which merklized state is stored. This \
                         lags the height reported by the `status` and `node` APIs, since \
                         merklized state is written asynchronously.",
                    )
            }),
        )
        .with_state(RouterState::new(options, data_source))
}

/// Wraps a merklized-state router with the app-level `healthcheck`, a request body limit, and
/// permissive CORS headers. Mounting the module prefix is up to the caller.
pub fn app(api: Router) -> Router {
    Router::new()
        .route(
            "/healthcheck",
            get(|headers: HeaderMap| async move { healthcheck_response(&headers) }),
        )
        .merge(api)
        .layer(body_limit_layer())
        .layer(cors_layer())
}

/// Encode a handler result, wrapping the module error in the crate-level
/// [`Error`](crate::Error) envelope the old tide app served.
fn respond<T: Serialize>(headers: &HeaderMap, result: Result<T, Error>) -> Response {
    wire::respond::<AppError, _>(headers, result.map_err(AppError::from))
}

/// Handler context: the data source. [`Options`] carries no settings yet; the router takes it for
/// symmetry with the other modules, and a setting added later lands here rather than in every
/// handler.
struct RouterState<S> {
    data_source: S,
}

impl<S> RouterState<S> {
    fn new(_options: &Options, data_source: S) -> Arc<Self> {
        Arc::new(Self { data_source })
    }
}

/// Parses a TaggedBase64 path parameter the way tide-disco's `blob_param` did, reporting a failure
/// as the request error that produced tide's 400.
fn tb64_param<T>(value: &str, field: &str) -> Result<T, Error>
where
    T: for<'a> TryFrom<&'a TaggedBase64>,
{
    let err = || Error::Request {
        source: RequestError::TaggedBase64 {
            reason: format!("invalid tagged base 64 for {field}"),
        },
    };
    let tb64: TaggedBase64 = value.parse().map_err(|_| err())?;
    T::try_from(&tb64).map_err(|_| err())
}

/// Look up one Merkle path. `Tree::Key`'s parse error has no [`Display`](std::fmt::Display) bound,
/// so a malformed key is reported by the route's own message rather than the parser's.
async fn load_path<Types, Tree, S, const ARITY: usize>(
    state: &RouterState<S>,
    snapshot: Snapshot<Types, Tree, ARITY>,
    key: &str,
) -> Result<MerkleProof<Tree::Entry, Tree::Key, Tree::T, ARITY>, Error>
where
    Types: NodeType,
    Tree: MerklizedState<Types, ARITY>,
    S: MerklizedStateDataSource<Types, Tree, ARITY> + Send + Sync + 'static,
{
    let key = key.parse::<Tree::Key>().map_err(|_| Error::Custom {
        message: "failed to parse Key param".to_string(),
        status: StatusCode::BAD_REQUEST,
    })?;
    state
        .data_source
        .get_path(snapshot, key)
        .await
        .map_err(|source| Error::Query { source })
}

async fn get_path_by_height<Types, Tree, S, const ARITY: usize>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((height, key)): Path<(u64, String)>,
) -> Response
where
    Types: NodeType,
    Tree: MerklizedState<Types, ARITY>,
    S: MerklizedStateDataSource<Types, Tree, ARITY> + Send + Sync + 'static,
{
    let result = load_path(&state, Snapshot::Index(height), &key).await;
    respond(&headers, result)
}

async fn get_path_by_commit<Types, Tree, S, const ARITY: usize>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
    Path((commit, key)): Path<(String, String)>,
) -> Response
where
    Types: NodeType,
    Tree: MerklizedState<Types, ARITY>,
    S: MerklizedStateDataSource<Types, Tree, ARITY> + Send + Sync + 'static,
{
    let result = async {
        let commit = tb64_param(&commit, "commit")?;
        load_path(&state, Snapshot::Commit(commit), &key).await
    }
    .await;
    respond(&headers, result)
}

async fn get_height<S>(State(state): State<Arc<RouterState<S>>>, headers: HeaderMap) -> Response
where
    S: MerklizedStateHeightPersistence + Send + Sync + 'static,
{
    let height = state.data_source.get_last_state_height().await;
    respond(&headers, height.map_err(|source| Error::Query { source }))
}

#[cfg(test)]
mod test {
    use async_trait::async_trait;
    use jf_merkle_tree_compat::{
        ForgetableMerkleTreeScheme, ForgetableUniversalMerkleTreeScheme,
        prelude::{Sha3Digest, Sha3Node},
        universal_merkle_tree::UniversalMerkleTree,
    };

    use super::*;
    use crate::{
        QueryResult,
        testing::mocks::{MockMerkleTree, MockTypes},
    };

    /// A second tree, differing from [`MockMerkleTree`] in arity and in `state_type`, so a test can
    /// mount two instantiations of the generic router in one router tree.
    type NarrowMerkleTree = UniversalMerkleTree<usize, Sha3Digest, usize, 3, Sha3Node>;

    impl MerklizedState<MockTypes, 3> for NarrowMerkleTree {
        type Key = usize;
        type Entry = usize;
        type T = Sha3Node;
        type Commit = Self::Commitment;
        type Digest = Sha3Digest;

        fn state_type() -> &'static str {
            "narrow_test_tree"
        }

        fn header_state_commitment_field() -> &'static str {
            "narrow_test_merkle_tree_root"
        }

        fn tree_height() -> usize {
            4
        }

        fn insert_path(
            &mut self,
            key: Self::Key,
            proof: &MerkleProof<Self::Entry, Self::Key, Self::T, 3>,
        ) -> anyhow::Result<()> {
            match proof.elem() {
                Some(elem) => self.remember(key, elem, proof)?,
                None => self.non_membership_remember(key, proof)?,
            }
            Ok(())
        }
    }

    /// Registers every route with no storage behind it, so the documentation tests need no
    /// database: they inspect the routes the router declares and call no handler.
    struct UnimplementedDataSource;

    #[async_trait]
    impl<Tree, const ARITY: usize> MerklizedStateDataSource<MockTypes, Tree, ARITY>
        for UnimplementedDataSource
    where
        Tree: MerklizedState<MockTypes, ARITY>,
    {
        async fn get_path(
            &self,
            _snapshot: Snapshot<MockTypes, Tree, ARITY>,
            _key: Tree::Key,
        ) -> QueryResult<MerkleProof<Tree::Entry, Tree::Key, Tree::T, ARITY>> {
            unimplemented!()
        }
    }

    #[async_trait]
    impl MerklizedStateHeightPersistence for UnimplementedDataSource {
        async fn get_last_state_height(&self) -> QueryResult<usize> {
            unimplemented!()
        }
    }

    const ROUTES: [&str; 3] = ["/{height}/{key}", "/commit/{commit}/{key}", "/block-height"];

    /// Applications mount this router and serve its documentation as part of their own OpenAPI
    /// spec, so every route it registers must carry a summary.
    #[tokio::test]
    async fn router_documents_every_route() {
        let mut api = aide::openapi::OpenApi::default();
        let _ = merklized_state_router::<MockTypes, MockMerkleTree, _, 8>(
            &Options::default(),
            UnimplementedDataSource,
        )
        .finish_api(&mut api);

        let paths = api.paths.expect("router registered paths");
        for route in ROUTES {
            let aide::openapi::ReferenceOr::Item(item) = &paths.paths[route] else {
                panic!("{route} is a reference, not an operation");
            };
            let op = item
                .get
                .as_ref()
                .unwrap_or_else(|| panic!("{route} has no GET"));
            assert!(op.summary.is_some(), "{route} has no summary");
            assert!(op.description.is_some(), "{route} has no description");
        }
    }

    /// An application with several merklized trees mounts this router once per tree. Two
    /// instantiations differing in tree and arity must document independently: the same paths under
    /// different prefixes, each summary naming its own tree, and no operation ID to collide on.
    #[tokio::test]
    async fn two_mounts_document_independently() {
        let mut api = aide::openapi::OpenApi::default();
        let _ = ApiRouter::new()
            .nest(
                "/test-tree",
                merklized_state_router::<MockTypes, MockMerkleTree, _, 8>(
                    &Options::default(),
                    UnimplementedDataSource,
                ),
            )
            .nest(
                "/narrow-tree",
                merklized_state_router::<MockTypes, NarrowMerkleTree, _, 3>(
                    &Options::default(),
                    UnimplementedDataSource,
                ),
            )
            .finish_api(&mut api);

        let paths = api.paths.expect("router registered paths");
        let summary = |route: String| {
            let aide::openapi::ReferenceOr::Item(item) = &paths.paths[&route] else {
                panic!("{route} is a reference, not an operation");
            };
            let op = item
                .get
                .as_ref()
                .unwrap_or_else(|| panic!("{route} has no GET"));
            assert!(
                op.operation_id.is_none(),
                "{route} declares an operation ID"
            );
            op.summary.clone().expect("route has a summary")
        };

        for route in ROUTES {
            let mounted = summary(format!("/test-tree{route}"));
            let other = summary(format!("/narrow-tree{route}"));
            assert!(mounted.contains("test_tree"), "{route}: {mounted}");
            assert!(other.contains("narrow_test_tree"), "{route}: {other}");
        }
    }
}
