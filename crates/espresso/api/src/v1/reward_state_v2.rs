//! Reward State V2 API trait for v1
//!
//! Returns internal espresso-types (legacy, no OpenAPI docs).
//! Uses associated types to avoid importing espresso-types in this crate.

use async_trait::async_trait;
use serde::Serialize;

/// Reward API trait - returns internal types
///
/// Uses associated types to avoid importing espresso-types in this crate.
/// Types must be Serialize for JSON responses, but don't need JsonSchema.
#[async_trait]
pub trait RewardApi {
    /// Type for reward claim input data (must be serializable to JSON)
    type RewardClaimInput: Serialize + Send + Sync;

    /// Type for reward balance queries (must be serializable to JSON)
    type RewardBalance: Serialize + Send + Sync;

    /// Type for reward account proof queries (must be serializable to JSON)
    type RewardAccountQueryData: Serialize + Send + Sync;

    /// Type for paginated reward amounts (must be serializable to JSON)
    type RewardAmounts: Serialize + Send + Sync;

    /// Type for raw merkle tree snapshots (must be serializable to JSON)
    type RewardMerkleTreeData: Serialize + Send + Sync;

    /// Get reward claim input for L1 contract submission
    ///
    /// Returns all data needed to call the claimRewards function on the L1 contract,
    /// including lifetime rewards and the Merkle proof.
    ///
    /// # Arguments
    /// * `block_height` - Must match the height finalized in the Light Client contract
    /// * `address` - Ethereum address to query rewards for (hex format)
    async fn get_reward_claim_input(
        &self,
        block_height: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardClaimInput>;

    /// Get reward balance at a specific height
    ///
    /// # Arguments
    /// * `height` - Block height to query
    /// * `address` - Ethereum address to query rewards for
    async fn get_reward_balance(
        &self,
        height: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardBalance>;

    /// Get latest reward balance at the most recent finalized height
    ///
    /// # Arguments
    /// * `address` - Ethereum address to query rewards for
    async fn get_latest_reward_balance(&self, address: String) -> anyhow::Result<Self::RewardBalance>;

    /// Get Merkle proof for a reward account at a specific height
    ///
    /// Returns complete query data with balance and expanded merkle proof
    ///
    /// # Arguments
    /// * `height` - Block height to query
    /// * `address` - Ethereum address to query proof for
    async fn get_reward_account_proof(
        &self,
        height: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryData>;

    /// Get Merkle proof for a reward account at the latest finalized height
    ///
    /// Returns complete query data with balance and expanded merkle proof
    ///
    /// # Arguments
    /// * `address` - Ethereum address to query proof for
    async fn get_latest_reward_account_proof(
        &self,
        address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryData>;

    /// Get paginated list of reward amounts at a specific height
    ///
    /// # Arguments
    /// * `height` - Block height to query
    /// * `offset` - Starting index for pagination
    /// * `limit` - Maximum number of results (≤ 10000)
    async fn get_reward_amounts(
        &self,
        height: u64,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Self::RewardAmounts>;

    /// Get raw RewardMerkleTreeV2 snapshot at a given height
    ///
    /// Returns the serialized merkle tree data
    ///
    /// # Arguments
    /// * `height` - Block height to query
    async fn get_reward_merkle_tree_v2(
        &self,
        height: u64,
    ) -> anyhow::Result<Self::RewardMerkleTreeData>;
}
