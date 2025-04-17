// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use hotshot::traits::{implementations::PushCdnNetwork, NodeImplementation};
use hotshot_example_types::{
    auction_results_provider_types::TestAuctionResultsProvider, state_types::TestTypes,
    storage_types::TestStorage,
};
use hotshot_types::traits::node_implementation::NodeType;
use serde::{Deserialize, Serialize};

use crate::infra::PushCdnDaRun;

#[derive(Clone, Deserialize, Serialize, Hash, PartialEq, Eq)]
/// Convenience type alias
pub struct NodeImpl {}

/// Convenience type alias
pub type Network = PushCdnNetwork<<TestTypes as NodeType>::SignatureKey>;

impl NodeImplementation<TestTypes> for NodeImpl {
    type Network = Network;
    type Storage = TestStorage<TestTypes>;
    type AuctionResultsProvider = TestAuctionResultsProvider<TestTypes>;
}

/// Convenience type alias
pub type ThisRun = PushCdnDaRun<TestTypes>;
