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

use std::{
    fmt::Display,
    ops::{Bound, RangeBounds},
};

use derivative::Derivative;
use derive_more::From;
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use tide_disco::StatusCode;

pub use crate::availability::{BlockHash, BlockId};
use crate::{HeightIndexed, QueryError};

/// A status of a set of resources, regarding its presence in the database.
///
/// A single resource or range of consecutive resources may be either:
/// * Present in the database
/// * Missing from the database, but will eventually be recovered via asynchronous fetching
/// * Pruned, meaning it is missing, but intentionally so, and will not be fetched
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize,
)]
pub enum SyncStatus {
    #[default]
    Present,
    Missing,
    Pruned,
}

/// The [`SyncStatus`] describing a range of consecutive objects of a single type.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize,
)]
pub struct SyncStatusRange {
    /// The inclusive starting height for the range.
    pub start: usize,
    /// The exclusive ending height for the range.
    pub end: usize,
    /// The sync status for objects in this range.
    pub status: SyncStatus,
}

impl RangeBounds<usize> for SyncStatusRange {
    fn start_bound(&self) -> Bound<&usize> {
        Bound::Included(&self.start)
    }

    fn end_bound(&self) -> Bound<&usize> {
        Bound::Excluded(&self.end)
    }
}

/// A summary of the [`SyncStatus`] for a single resource (e.g. blocks, or leaves).
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct ResourceSyncStatus {
    /// The number of missing (not including pruned) objects for this resource.
    pub missing: usize,

    /// An ordered list of contiguous ranges of objects of this type with the same sync status.
    pub ranges: Vec<SyncStatusRange>,
}

impl ResourceSyncStatus {
    pub fn is_fully_synced(&self) -> bool {
        self.missing == 0
    }

    /// Extend this [`ResourceSyncStatus`] to additionally cover the range covered by `other`.
    pub fn extend(&mut self, other: Self) {
        self.missing += other.missing;

        let mut ranges = other.ranges.into_iter();

        // Check if the last range of `self` and the first range of `other` can be combined.
        if let Some(last) = self.ranges.last_mut()
            && let Some(next) = ranges.next()
        {
            if last.status == next.status && last.end == next.start {
                last.end = next.end;
            } else {
                self.ranges.push(next);
            }
        }

        self.ranges.extend(ranges);
    }
}

/// [`SyncStatus`] for the entire database.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct SyncStatusQueryData {
    /// Summary of the [`SyncStatus`] of all blocks.
    pub blocks: ResourceSyncStatus,
    /// Summary of the [`SyncStatus`] of all leaves.
    pub leaves: ResourceSyncStatus,
    /// Summary of the [`SyncStatus`] of all VID common objects.
    pub vid_common: ResourceSyncStatus,

    /// The height of the last pruned object.
    ///
    /// Objects below this height are intentionally missing and will never be recovered (unless
    /// pruning settings are changed.)
    pub pruned_height: Option<usize>,
}

impl SyncStatusQueryData {
    pub fn is_fully_synced(&self) -> bool {
        self.blocks.is_fully_synced()
            && self.leaves.is_fully_synced()
            && self.vid_common.is_fully_synced()
    }
}

/// Response to a `/:resource/window` query.
#[derive(Clone, Debug, Derivative, PartialEq, Eq, Serialize, Deserialize)]
#[derivative(Default(bound = ""))]
pub struct TimeWindowQueryData<T> {
    pub window: Vec<T>,
    pub prev: Option<T>,
    pub next: Option<T>,
}

impl<T: HeightIndexed> TimeWindowQueryData<T> {
    /// The block height of the block that starts the window.
    ///
    /// If the window is empty, this is the height of the block that ends the window.
    pub fn from(&self) -> Option<u64> {
        self.window
            .first()
            .or(self.next.as_ref())
            .map(|t| t.height())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Limits {
    pub window_limit: usize,
}

/// Errors exposed to clients of the node API.
#[derive(Clone, Debug, From, Snafu, Deserialize, Serialize)]
#[snafu(visibility(pub))]
pub enum Error {
    Request {
        source: tide_disco::RequestError,
    },
    #[snafu(display("{source}"))]
    Query {
        source: QueryError,
    },
    #[snafu(display("error fetching VID share for block {block}: {source}"))]
    #[from(ignore)]
    QueryVid {
        source: QueryError,
        block: String,
    },
    #[snafu(display(
        "error fetching window starting from {start} and ending at time {end}: {source}"
    ))]
    #[from(ignore)]
    QueryWindow {
        source: QueryError,
        start: String,
        end: u64,
    },
    #[snafu(display("error {status}: {message}"))]
    Custom {
        message: String,
        status: StatusCode,
    },
}

impl Error {
    pub fn internal<M: Display>(message: M) -> Self {
        Self::Custom {
            message: message.to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn status(&self) -> StatusCode {
        match self {
            Self::Request { .. } => StatusCode::BAD_REQUEST,
            Self::Query { source, .. }
            | Self::QueryVid { source, .. }
            | Self::QueryWindow { source, .. } => source.status(),
            Self::Custom { status, .. } => *status,
        }
    }
}
