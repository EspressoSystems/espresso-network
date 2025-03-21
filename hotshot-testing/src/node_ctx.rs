// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{collections::HashMap, sync::Arc};

use hotshot::{traits::TestableNodeImplementation, HotShotError};
use hotshot_types::traits::node_implementation::NodeType;

/// context for a round
// TODO eventually we want these to just be futures
// that we poll when things are event driven
// this context will be passed around
#[derive(Debug, Clone)]
pub struct NodeCtx<TYPES: NodeType, I: TestableNodeImplementation<TYPES::ConsensusType, TYPES>> {
    /// results from previous rounds
    pub round_results: HashMap<TYPES::Time, ViewStatus<TYPES, I>>,
}

impl<TYPES: NodeType, I: TestableNodeImplementation<TYPES::ConsensusType, TYPES>> Default
    for NodeCtx<TYPES, I>
{
    fn default() -> Self {
        Self {
            round_results: Default::default(),
        }
    }
}

/// Status of a view.
#[derive(Debug, Clone)]
pub enum ViewStatus<TYPES: NodeType, I: TestableNodeImplementation<TYPES::ConsensusType, TYPES>> {
    /// The view is in progress.
    InProgress(InProgress),
    /// The view is failed.
    ViewFailed(ViewFailed<TYPES>),
    /// The view is a success.
    ViewSuccess(ViewSuccess<TYPES>),
}

/// In-progress status of a view.
#[derive(Debug, Clone)]
pub struct InProgress {}

/// Failed status of a view.
#[derive(Debug, Clone)]
pub struct ViewFailed<TYPES: NodeType>(pub Arc<HotShotError<TYPES>>);

/// Success status of a view.
#[derive(Debug, Clone)]
pub struct ViewSuccess<TYPES: NodeType> {
    /// state after decide event
    pub agreed_state: (),

    /// block after decide event
    pub agreed_block: LeafBlockPayload<Leaf<TYPES>>,

    /// leaf after decide event
    pub agreed_leaf: Leaf<TYPES>,
}
