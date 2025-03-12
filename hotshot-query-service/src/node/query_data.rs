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

use derivative::Derivative;
use serde::{Deserialize, Serialize};

pub use crate::availability::{BlockHash, BlockId};
use crate::types::HeightIndexed;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct SyncStatus {
    pub missing_blocks: usize,
    pub missing_leaves: usize,
    pub missing_vid_common: usize,
    pub missing_vid_shares: usize,
    pub pruned_height: Option<usize>,
}

impl SyncStatus {
    pub fn fully_synced() -> Self {
        Self {
            missing_blocks: 0,
            missing_leaves: 0,
            missing_vid_common: 0,
            missing_vid_shares: 0,
            pruned_height: None,
        }
    }

    pub fn is_fully_synced(&self) -> bool {
        *self == Self::fully_synced()
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
