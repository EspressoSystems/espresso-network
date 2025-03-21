// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Error type for `HotShot`
//!
//! This module provides [`HotShotError`], which is an enum representing possible faults that can
//! occur while interacting with this crate.

use committable::Commitment;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{data::Leaf2, traits::node_implementation::NodeType};

/// Error type for `HotShot`
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum HotShotError<TYPES: NodeType> {
    /// The consensus state machine is in an invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Leaf was not present in storage
    #[error("Missing leaf with commitment: {0}")]
    MissingLeaf(Commitment<Leaf2<TYPES>>),

    /// Failed to serialize data
    #[error("Failed to serialize: {0}")]
    FailedToSerialize(String),

    /// Failed to deserialize data
    #[error("Failed to deserialize: {0}")]
    FailedToDeserialize(String),

    /// The view timed out
    #[error("View {view_number} timed out: {state:?}")]
    ViewTimedOut {
        /// The view number that timed out
        view_number: TYPES::View,
        /// The state that the round was in when it timed out
        state: RoundTimedoutState,
    },
}

/// Contains information about what the state of the hotshot-consensus was when a round timed out
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RoundTimedoutState {
    /// Leader is in a Prepare phase and is waiting for a HighQc
    LeaderWaitingForHighQc,
    /// Leader is in a Prepare phase and timed out before the round min time is reached
    LeaderMinRoundTimeNotReached,
    /// Leader is waiting for prepare votes
    LeaderWaitingForPrepareVotes,
    /// Leader is waiting for precommit votes
    LeaderWaitingForPreCommitVotes,
    /// Leader is waiting for commit votes
    LeaderWaitingForCommitVotes,

    /// Replica is waiting for a prepare message
    ReplicaWaitingForPrepare,
    /// Replica is waiting for a pre-commit message
    ReplicaWaitingForPreCommit,
    /// Replica is waiting for a commit message
    ReplicaWaitingForCommit,
    /// Replica is waiting for a decide message
    ReplicaWaitingForDecide,

    /// HotShot-testing tried to collect round events, but it timed out
    TestCollectRoundEventsTimedOut,
}
