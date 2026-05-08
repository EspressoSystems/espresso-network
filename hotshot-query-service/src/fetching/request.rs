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
    data::{VidCommitment, VidCommon, ViewNumber},
    simple_vote::QuorumData2,
    traits::node_implementation::NodeType,
};

use crate::{
    Payload,
    availability::{
        BlockQueryData, Certificate2, LeafHash, LeafQueryData, QueryableHeader, VidCommonQueryData,
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
/// The expected leaf hash, the QC's view, and the certifying QC's `data` are provided
/// so the request can be verified against a response from an untrusted provider.
///
/// We compare the QC's `(view, data)` instead of its full commit because under the new protocol
/// we store leaves alongside a `cert1` taken from the decide event, and that `cert1` is
/// not necessarily the same `cert1` that the next leaf's `justify_qc` will reference. Both
/// certify the same `(view, leaf)` but can be assembled from different voting set
/// so their aggregated signatures (and commits) differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, From, Into)]
pub struct LeafRequest<Types: NodeType> {
    pub height: u64,
    pub expected_leaf: LeafHash<Types>,
    pub expected_qc_view: ViewNumber,
    pub expected_qc_data: QuorumData2<Types>,
}

impl<Types: NodeType> LeafRequest<Types> {
    pub fn new(
        height: u64,
        expected_leaf: LeafHash<Types>,
        expected_qc_view: ViewNumber,
        expected_qc_data: QuorumData2<Types>,
    ) -> Self {
        Self {
            height,
            expected_leaf,
            expected_qc_view,
            expected_qc_data,
        }
    }
}

impl<Types: NodeType> Request<Types> for LeafRequest<Types> {
    type Response = LeafQueryData<Types>;
}

/// A request for a consecutive range of leaves.
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

    /// The expected view of the QC certifying the last leaf in the chain.
    ///
    /// See [`LeafRequest`] for why we compare `(view, data)` rather than full commit.
    pub last_qc_view: ViewNumber,

    /// The expected `data` of the QC certifying the last leaf in the chain.
    pub last_qc_data: QuorumData2<Types>,
}

impl<Types: NodeType> Request<Types> for LeafRangeRequest<Types> {
    type Response = NonEmptyRange<LeafQueryData<Types>>;
}

/// A request for a cert2 at a given height.
///
/// The response is `Option<Certificate2>` since not every height has a cert2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Certificate2Request {
    pub height: u64,
}

impl<Types: NodeType> Request<Types> for Certificate2Request {
    type Response = Option<Certificate2<Types>>;
}
