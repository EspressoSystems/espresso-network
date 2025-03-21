// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Common traits for the `HotShot` protocol
pub mod auction_results_provider;
pub mod block_contents;
pub mod consensus_api;
pub mod election;
pub mod metrics;
pub mod network;
pub mod node_implementation;
pub mod qc;
pub mod signature_key;
pub mod stake_table;
pub mod states;
pub mod storage;

pub use block_contents::{BlockPayload, EncodeBytes};
pub use states::ValidatedState;
