use std::collections::HashMap;

use alloy::primitives::U256;

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
