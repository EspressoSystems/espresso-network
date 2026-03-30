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

use alloy::primitives::{FixedBytes, Keccak256};
use derive_more::{From, Into};
use hotshot_types::{
    data::{VidCommitment, VidCommon},
    traits::node_implementation::NodeType,
};

use crate::{
    Payload,
    availability::{
        BlockQueryData, LeafHash, LeafQueryData, QcHash, QueryableHeader, VidCommonQueryData,
    },
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

/// A request for a consecutive range of objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RangeRequest {
    /// The first block in the requested range.
    pub start: u64,

    /// The first block after the requested range.
    pub end: u64,

    /// The Keccak256 hash of the concatenation of the expected payload commitments.
    ///
    /// This can be used to verify the fetched data. We use the hash rather than passing in the full
    /// list of expected payload commitments because a [`Request`] is expected to be small and easy
    /// to copy and pass around.
    pub expected_hash: FixedBytes<32>,
}

impl RangeRequest {
    /// A request for a range of data corresponding to a range of headers.
    pub fn from_headers<Types: NodeType>(
        headers: &NonEmptyRange<impl QueryableHeader<Types>>,
    ) -> Self {
        let expected_hash =
            Self::hash_payloads(headers.iter().map(|header| header.payload_commitment()));
        Self {
            start: headers.start(),
            end: headers.end(),
            expected_hash,
        }
    }

    /// Compute the expected hash of a range of payload commitments.
    pub fn hash_payloads(
        payload_commitments: impl IntoIterator<Item = VidCommitment>,
    ) -> FixedBytes<32> {
        let mut hasher = Keccak256::new();
        for comm in payload_commitments {
            hasher.update(comm);
        }
        hasher.finalize()
    }
}

/// A request for a consecutive range of blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, From, Into)]
pub struct BlockRangeRequest(RangeRequest);

impl<Types: NodeType> Request<Types> for BlockRangeRequest {
    type Response = NonEmptyRange<BlockQueryData<Types>>;
}

impl BlockRangeRequest {
    pub fn from_headers<Types: NodeType>(
        headers: &NonEmptyRange<impl QueryableHeader<Types>>,
    ) -> Self {
        RangeRequest::from_headers(headers).into()
    }
}

/// A request for VID common data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VidCommonRequest(pub VidCommitment);

impl<Types: NodeType> Request<Types> for VidCommonRequest {
    type Response = VidCommon;
}

/// A request for a consecutive range of VID common.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, From, Into)]
pub struct VidCommonRangeRequest(RangeRequest);

impl<Types: NodeType> Request<Types> for VidCommonRangeRequest {
    type Response = NonEmptyRange<VidCommonQueryData<Types>>;
}

impl VidCommonRangeRequest {
    pub fn from_headers<Types: NodeType>(
        headers: &NonEmptyRange<impl QueryableHeader<Types>>,
    ) -> Self {
        RangeRequest::from_headers(headers).into()
    }
}

/// A request for a leaf with a given height.
///
/// The expected hash and QC hash are also provided, so that the request can be verified against a
/// response from an untrusted provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, From, Into)]
pub struct LeafRequest<Types: NodeType> {
    pub height: u64,
    pub expected_leaf: LeafHash<Types>,
    pub expected_qc: QcHash<Types>,
}

impl<Types: NodeType> LeafRequest<Types> {
    pub fn new(height: u64, expected_leaf: LeafHash<Types>, expected_qc: QcHash<Types>) -> Self {
        Self {
            height,
            expected_leaf,
            expected_qc,
        }
    }
}

impl<Types: NodeType> Request<Types> for LeafRequest<Types> {
    type Response = LeafQueryData<Types>;
}

/// A request for a consecutive range of VID common.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LeafRangeRequest<Types: NodeType> {
    /// The first block in the requested range.
    pub start: u64,

    /// The first block after the requested range.
    pub end: u64,

    /// The expected hash of the last leaf in the chain.
    ///
    /// Earlier leaves can be verified based on the subsequent leaf.
    pub last_leaf: LeafHash<Types>,

    /// The expected hash of the last QC in the chain.
    ///
    /// Earlier QCs can be verified based on the subsequent leaf.
    pub last_qc: QcHash<Types>,
}

impl<Types: NodeType> Request<Types> for LeafRangeRequest<Types> {
    type Response = NonEmptyRange<LeafQueryData<Types>>;
}
