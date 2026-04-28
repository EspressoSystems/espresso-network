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

//! Requests for fetching resources.

use std::{fmt::Debug, hash::Hash};

use derive_more::{From, Into};
use hotshot_types::{
    data::{VidCommitment, VidCommon},
    traits::node_implementation::NodeType,
};

use crate::{
    Payload,
    availability::{BlockQueryData, LeafQueryData, VidCommonQueryData},
    fetching::NonEmptyRange,
};

/// A request for a resource.
pub trait Request<Types>: Copy + Debug + Eq + Hash + Send {
    /// The type of resource that will be returned as a successful response to this request.
    type Response: Clone + Send;
}

/// A request for a payload with a given commitment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PayloadRequest(pub VidCommitment);

impl<Types: NodeType> Request<Types> for PayloadRequest {
    type Response = Payload<Types>;
}

/// A request for a consecutive range of blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, From, Into)]
pub struct BlockRangeRequest {
    pub start: u64,
    pub end: u64,
}

impl<Types: NodeType> Request<Types> for BlockRangeRequest {
    type Response = NonEmptyRange<BlockQueryData<Types>>;
}

/// A request for VID common data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VidCommonRequest(pub VidCommitment);

impl<Types: NodeType> Request<Types> for VidCommonRequest {
    type Response = VidCommon;
}

/// A request for a consecutive range of VID common.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, From, Into)]
pub struct VidCommonRangeRequest {
    pub start: u64,
    pub end: u64,
}

impl<Types: NodeType> Request<Types> for VidCommonRangeRequest {
    type Response = NonEmptyRange<VidCommonQueryData<Types>>;
}

/// A request for a leaf with a given height.
///
/// The expected hash and QC hash are also provided, so that the request can be verified against a
/// response from an untrusted provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, From, Into)]
pub struct LeafRequest {
    pub height: u64,
}

impl LeafRequest {
    pub fn new(height: u64) -> Self {
        Self { height }
    }
}

impl<Types: NodeType> Request<Types> for LeafRequest {
    type Response = LeafQueryData<Types>;
}

/// A request for a consecutive range of VID common.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LeafRangeRequest {
    /// The first block in the requested range.
    pub start: u64,

    /// The first block after the requested range.
    pub end: u64,
}

impl<Types: NodeType> Request<Types> for LeafRangeRequest {
    type Response = NonEmptyRange<LeafQueryData<Types>>;
}
