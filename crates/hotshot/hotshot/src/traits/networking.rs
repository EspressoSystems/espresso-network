// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Network access compatibility
//!
//! This module contains a trait abstracting over network access, as well as implementations of that
//! trait. Currently this includes
//! - [`MemoryNetwork`](memory_network::MemoryNetwork), an in memory testing-only implementation
//! - [`Cliquenet`](cliquenet_network::Cliquenet), a fully connected mesh network for production use.

pub mod cliquenet_network;
pub mod combined_network;
pub mod memory_network;
pub mod push_cdn_network;

pub use hotshot_types::traits::network::{NetworkError, NetworkReliability};
