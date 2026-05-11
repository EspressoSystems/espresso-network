//! Reward State (V1) API trait for v1
//!
//! Returns internal espresso-types (legacy, no OpenAPI docs).
//! Uses associated types to avoid importing espresso-types in this crate.
//!
//! This trait covers the `reward-state` URL prefix which uses the V1 reward
//! merkle tree (RewardMerkleTreeV1, arity 256).  The "shared" endpoints
//! (balance, claim-input, amounts, tree-v2, proof/latest) re-use the
//! existing `RewardApi` handlers at the `reward-state` URL prefix.

use async_trait::async_trait;
use serde::Serialize;

/// Reward State V1 API trait - returns internal types
///
/// Exposes the RewardMerkleTreeV1 (arity 256) merklized state queries.
#[async_trait]
pub trait RewardStateApi {
    /// Type for reward V1 merkle proof (must be serializable to JSON)
    type RewardMerklePath: Serialize + Send + Sync;

    /// Type for V1 reward account query data (proof + balance)
    type RewardAccountQueryData: Serialize + Send + Sync;

    /// Get the current reward-state height
    async fn get_reward_state_height(&self) -> anyhow::Result<u64>;

    /// Get merkle path for a reward account at the given snapshot height
    async fn get_reward_state_path(
        &self,
        height: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardMerklePath>;

    /// Get merkle path for a reward account by commitment string
    async fn get_reward_state_path_by_commit(
        &self,
        commit: String,
        address: String,
    ) -> anyhow::Result<Self::RewardMerklePath>;

    /// Get V1 reward account proof at a specific height
    async fn get_reward_state_proof(
        &self,
        height: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryData>;
}
