// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! The consensus layer for hotshot. This currently implements sequencing
//! consensus in an event driven way

/// The task which implements the core state logic of consensus.
pub mod consensus;

/// The task which handles the logic for the quorum vote.
pub mod quorum_vote;

/// The task which implements the main parts of data availability.
pub mod da;

/// The task which implements all transaction handling
pub mod transactions;

/// Defines the events passed between tasks
pub mod events;

/// The task which implements the network.
pub mod network;

/// Defines the types to run unit tests for a task.
pub mod harness;

/// The task which implements view synchronization
pub mod view_sync;

/// The task which implements verifiable information dispersal
pub mod vid;

/// Generic task for collecting votes
pub mod vote_collection;

/// Task for handling upgrades
pub mod upgrade;

/// Implementations for builder client
/// Should contain builder task in the future
pub mod builder;

/// Helper functions used by any task
pub mod helpers;

/// Task which responses to requests from the network
pub mod response;

/// Task for requesting the network for things
pub mod request;

/// Task for handling logic for quorum proposals
pub mod quorum_proposal;

/// Task for handling QuorumProposalRecv events
pub mod quorum_proposal_recv;

/// Task for storing and replaying all received tasks by a node
pub mod rewind;
