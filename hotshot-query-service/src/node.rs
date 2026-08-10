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

//! A node's view of a HotShot chain
//!
//! The node API provides a subjective view of the HotShot blockchain, from the perspective of
//! one particular node. It provides access to information that the
//! [availability](crate::availability) API does not, because this information depends on the
//! perspective of the node observing it, and may be subject to eventual consistency. For example,
//! `/node/block-height` may return smaller counts than expected, if the node being queried is not
//! fully synced with the entire history of the chain. However, the node will _eventually_ sync and
//! return the expected counts.

pub(crate) mod data_source;
pub(crate) mod query_data;
pub use data_source::*;
pub use hotshot_query_service_types::node::*;

#[derive(Debug)]
pub struct Options {
    /// The maximum number of headers which can be loaded in a single `header/window` query.
    pub window_limit: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self { window_limit: 500 }
    }
}
