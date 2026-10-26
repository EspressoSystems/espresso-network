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

//! Queries for node-specific state and uncommitted data.
//!
//! Unlike the [availability](crate::availability) and [node](crate::node) APIs, which deal only
//! with committed data (albeit with different consistency properties), the status API offers a
//! glimpse into internal consensus state and uncommitted data. Here you can find low-level
//! information about a particular node, such as consensus and networking metrics.
//!
//! The status API is intended to be a lightweight way to inspect the activities and health of a
//! consensus node. It is the only API that can be run without any persistent storage, and its
//! memory overhead is also very low. As a consequence, it only serves two types of data:
//! * snapshots of the state right now, with no way to query historical snapshots
//! * summary statistics

pub(crate) mod data_source;

pub use data_source::*;
pub use hotshot_query_service_types::status::Error;
