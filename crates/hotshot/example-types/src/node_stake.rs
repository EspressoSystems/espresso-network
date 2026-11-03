use std::collections::HashMap;

use alloy::primitives::U256;
use hotshot_types::{PeerConfig, ValidatorConfig, traits::node_implementation::NodeType};

/// The public configs of `num_staked_nodes` test nodes with keys derived from a fixed seed, and
/// the subset of the first `num_da_nodes` that also sit on the DA committee.
pub fn gen_node_lists<TYPES: NodeType>(
    num_staked_nodes: u64,
    num_da_nodes: u64,
    node_stakes: &TestNodeStakes,
) -> (Vec<PeerConfig<TYPES>>, Vec<PeerConfig<TYPES>>) {
    let mut staked_nodes = Vec::new();
    let mut da_nodes = Vec::new();

    for n in 0..num_staked_nodes {
        let validator_config: ValidatorConfig<TYPES> = ValidatorConfig::generated_from_seed_indexed(
            [0u8; 32],
            n,
            node_stakes.get(n),
            n < num_da_nodes,
        );

        let peer_config = validator_config.public_config();
        staked_nodes.push(peer_config.clone());

        if n < num_da_nodes {
            da_nodes.push(peer_config)
        }
    }

    (staked_nodes, da_nodes)
}

/// Stake per test node id; nodes not listed get the default stake.
#[derive(Clone)]
pub struct TestNodeStakes {
    stakes: HashMap<u64, U256>,
    default_stake: U256,
}

impl TestNodeStakes {
    pub fn new(stakes: HashMap<u64, U256>, default_stake: U256) -> Self {
        Self {
            stakes,
            default_stake,
        }
    }

    pub fn get(&self, node_id: u64) -> U256 {
        self.stakes
            .get(&node_id)
            .cloned()
            .unwrap_or(self.default_stake)
    }

    pub fn with_stake(mut self, node_id: u64, stake: U256) -> Self {
        self.stakes.insert(node_id, stake);
        self
    }
}

impl Default for TestNodeStakes {
    fn default() -> Self {
        Self {
            stakes: HashMap::new(),
            default_stake: U256::from(1),
        }
    }
}
