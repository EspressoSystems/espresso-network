// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Abstractions over the immutable instance-level state and the global state that blocks modify.
//!
//! This module provides the [`InstanceState`] and [`ValidatedState`] traits, which serve as
//! compatibilities over the current network state, which is modified by the transactions contained
//! within blocks.

use std::{error::Error, fmt::Debug, future::Future};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use vbs::version::Version;

use super::block_contents::TestableBlock;
use crate::{
    data::Leaf2,
    traits::{
        node_implementation::{ConsensusTime, NodeType},
        BlockPayload,
    },
};

/// Instance-level state, which allows us to fetch missing validated state.
pub trait InstanceState: Debug + Clone + Send + Sync {}

/// Application-specific state delta, which will be used to store a list of merkle tree entries.
pub trait StateDelta:
    Debug + PartialEq + Eq + Send + Sync + Serialize + for<'a> Deserialize<'a>
{
}

/// Abstraction over the state that blocks modify
///
/// This trait represents the behaviors that the 'global' ledger state must have:
///   * A defined error type ([`Error`](ValidatedState::Error))
///   * The type of block that modifies this type of state ([`BlockPayload`](`ValidatedStates::
/// BlockPayload`))
///   * The ability to validate that a block header is actually a valid extension of this state and
///     produce a new state, with the modifications from the block applied
///
/// ([`validate_and_apply_header`](`ValidatedState::validate_and_apply_header`))
pub trait ValidatedState<TYPES: NodeType>:
    Serialize + DeserializeOwned + Debug + Default + PartialEq + Eq + Send + Sync + Clone
{
    /// The error type for this particular type of ledger state
    type Error: Error + Debug + Send + Sync;
    /// The type of the instance-level state this state is associated with
    type Instance: InstanceState;
    /// The type of the state delta this state is associated with.
    type Delta: StateDelta;
    /// Time compatibility needed for reward collection
    type Time: ConsensusTime;

    /// Check if the proposed block header is valid and apply it to the state if so.
    ///
    /// Returns the new state and state delta.
    ///
    /// # Arguments
    /// * `instance` - Immutable instance-level state.
    ///
    /// # Errors
    ///
    /// If the block header is invalid or appending it would lead to an invalid state.
    fn validate_and_apply_header(
        &self,
        instance: &Self::Instance,
        parent_leaf: &Leaf2<TYPES>,
        proposed_header: &TYPES::BlockHeader,
        payload_byte_len: u32,
        version: Version,
        view_number: u64,
    ) -> impl Future<Output = Result<(Self, Self::Delta), Self::Error>> + Send;

    /// Construct the state with the given block header.
    ///
    /// This can also be used to rebuild the state for catchup.
    fn from_header(block_header: &TYPES::BlockHeader) -> Self;

    /// Construct a genesis validated state.
    #[must_use]
    fn genesis(instance: &Self::Instance) -> (Self, Self::Delta);

    /// Gets called to notify the persistence backend that this state has been committed
    fn on_commit(&self);
}

/// extra functions required on state to be usable by hotshot-testing
pub trait TestableState<TYPES>: ValidatedState<TYPES>
where
    TYPES: NodeType,
    TYPES::BlockPayload: TestableBlock<TYPES>,
{
    /// Creates random transaction if possible
    /// otherwise panics
    /// `padding` is the bytes of padding to add to the transaction
    fn create_random_transaction(
        state: Option<&Self>,
        rng: &mut dyn rand::RngCore,
        padding: u64,
    ) -> <TYPES::BlockPayload as BlockPayload<TYPES>>::Transaction;
}
