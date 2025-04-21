// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use hotshot_types::traits::node_implementation::NodeType;
use serde::{Deserialize, Serialize};

use super::block_info::AvailableBlockInfo;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(bound = "")]
pub struct AvailableBlocksQueryData<I: NodeType> {
    pub blocks: Vec<AvailableBlockInfo<I>>,
}
