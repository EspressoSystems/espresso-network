use alloy::primitives::{FixedBytes, U256};
use serde::{Deserialize, Serialize};

use crate::sol_types::LifetimeRewardsProofSol;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct RewardClaimInput {
    /// The total lifetime rewards of the claimer
    pub lifetime_rewards: U256,
    /// The proof of inclusion for the rewards merkle tree
    pub proof: LifetimeRewardsProofSol,
    /// The other inputs to the auth root computation
    pub auth_root_inputs: [FixedBytes<32>; 7],
}
