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
use hotshot_types::{data::VidCommitment, traits::node_implementation::NodeType};

use crate::{
    availability::{LeafHash, LeafQueryData, QcHash, StateCertQueryData},
    Payload, VidCommon,
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

/// A request for VID common data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VidCommonRequest(pub VidCommitment);

impl<Types: NodeType> Request<Types> for VidCommonRequest {
    type Response = VidCommon;
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

/// A request for a light client state update certificate with a given epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, From, Into)]
pub struct StateCertRequest(pub u64);

impl<Types: NodeType> Request<Types> for StateCertRequest {
    type Response = StateCertQueryData<Types>;
}
