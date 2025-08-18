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

//! Data for the [`merklized_state`](super) API.
//!
//! This module facilitates storing the state of a Merkle Tree at a specific point in time
//! and provides methods for querying and reconstructing the snapshot.
//!

use std::{cmp::Ordering, fmt::Debug, str::FromStr};

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use async_trait::async_trait;
use derivative::Derivative;
use derive_more::with_trait::Display;
use hotshot_types::traits::node_implementation::NodeType;
use jf_merkle_tree::{
    prelude::MerkleProof, DigestAlgorithm, Element, ForgetableMerkleTreeScheme, Index,
    MerkleCommitment, NodeValue, ToTraversalPath,
};
use serde::{de::DeserializeOwned, Serialize};
use tagged_base64::TaggedBase64;

use crate::QueryResult;

/// This trait defines methods that a data source should implement
/// It enables retrieval of the membership path for a leaf node, which can be used to reconstruct the Merkle tree state.
#[async_trait]
pub trait MerklizedStateDataSource<Types, State, const ARITY: usize>
where
    Types: NodeType,
    State: MerklizedState<Types, ARITY>,
{
    async fn get_path(
        &self,
        snapshot: Snapshot<Types, State, ARITY>,
        key: State::Key,
    ) -> QueryResult<MerkleProof<State::Entry, State::Key, State::T, ARITY>>;
}

/// This trait defines methods for updating the storage with the merkle tree state.
#[async_trait]
pub trait UpdateStateData<Types: NodeType, State: MerklizedState<Types, ARITY>, const ARITY: usize>:
    Send + Sync
{
    async fn set_last_state_height(&mut self, height: usize) -> anyhow::Result<()>;
    async fn insert_merkle_nodes(
        &mut self,
        path: MerkleProof<State::Entry, State::Key, State::T, ARITY>,
        traversal_path: Vec<usize>,
        block_number: u64,
    ) -> anyhow::Result<()>;
}

#[async_trait]
pub trait MerklizedStateHeightPersistence {
    async fn get_last_state_height(&self) -> QueryResult<usize>;
}

type StateCommitment<Types, T, const ARITY: usize> = <T as MerklizedState<Types, ARITY>>::Commit;

/// Snapshot can be queried by block height (index) or merkle tree commitment
#[derive(Derivative, Display)]
#[derivative(Ord = "feature_allow_slow_enum")]
#[derivative(
    Copy(bound = ""),
    Debug(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = ""),
    Ord(bound = ""),
    Hash(bound = "")
)]
pub enum Snapshot<Types: NodeType, T: MerklizedState<Types, ARITY>, const ARITY: usize> {
    #[display("{_0}")]
    Commit(StateCommitment<Types, T, ARITY>),
    #[display("{_0}")]
    Index(u64),
}

impl<T: MerklizedState<Types, ARITY>, Types: NodeType, const ARITY: usize> Clone
    for Snapshot<Types, T, ARITY>
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: MerklizedState<Types, ARITY>, Types: NodeType, const ARITY: usize> PartialOrd
    for Snapshot<Types, T, ARITY>
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// This trait should be implemented by the MerkleTree that the module is initialized for.
/// It defines methods utilized by the module.
pub trait MerklizedState<Types, const ARITY: usize>:
    ForgetableMerkleTreeScheme<Commitment = Self::Commit> + Send + Sync + Clone + 'static
where
    Types: NodeType,
{
    type Key: Index
        + Send
        + Sync
        + Serialize
        + ToTraversalPath<ARITY>
        + FromStr
        + DeserializeOwned
        + Display
        + CanonicalSerialize
        + CanonicalDeserialize;
    type Entry: Element
        + Send
        + Sync
        + Serialize
        + DeserializeOwned
        + CanonicalSerialize
        + CanonicalDeserialize;
    type T: NodeValue + Send;
    type Commit: MerkleCommitment<Self::T>
        + Send
        + for<'a> TryFrom<&'a TaggedBase64>
        + Display
        + Debug
        + Into<TaggedBase64>;
    type Digest: DigestAlgorithm<Self::Entry, Self::Key, Self::T>;

    /// Retrieves the name of the state being queried.
    fn state_type() -> &'static str;

    /// Retrieves the field in the header containing the Merkle tree commitment
    /// for the state implementing this trait.
    fn header_state_commitment_field() -> &'static str;

    /// Get the height of the tree
    fn tree_height() -> usize;

    /// Insert a forgotten path into the tree.
    fn insert_path(
        &mut self,
        key: Self::Key,
        proof: &MerkleProof<Self::Entry, Self::Key, Self::T, ARITY>,
    ) -> anyhow::Result<()>;
}
