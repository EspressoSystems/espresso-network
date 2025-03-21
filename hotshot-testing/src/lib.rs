// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Testing infrastructure for `HotShot`

/// Helpers for initializing system context handle and building tasks.
pub mod helpers;

///  builder
pub mod test_builder;

/// launcher
pub mod test_launcher;

/// runner
pub mod test_runner;

/// task that's consuming events and asserting safety
pub mod overall_safety_task;

/// task that checks leaves received across all nodes from decide events for consistency
pub mod consistency_task;

/// task that's submitting transactions to the stream
pub mod txn_task;

/// task that decides when things are complete
pub mod completion_task;

/// task to spin nodes up and down
pub mod spinning_task;

/// the `TestTask` struct and associated trait/functions
pub mod test_task;

/// task for checking if view sync got activated
pub mod view_sync_task;

/// Test implementation of block builder
pub mod block_builder;

/// predicates to use in tests
pub mod predicates;

/// scripting harness for tests
pub mod script;

/// view generator for tests
pub mod view_generator;

/// byzantine framework for tests
pub mod byzantine;
