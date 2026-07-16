//! Reward State V2 API trait for v1
//!
//! Returns internal espresso-types (legacy, no OpenAPI docs).
//! Uses associated types to avoid importing espresso-types in this crate.

use async_trait::async_trait;
use serde::Serialize;

use crate::v1::merklized_state::Snapshot;

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

    /// Type for reward account proof queries against the V1 (RewardMerkleTreeV1) tree
    type RewardAccountQueryDataV1: Serialize + Send + Sync;

    /// Type for a raw Merkle path into the reward-state (RewardMerkleTreeV1) tree
    type RewardStatePathV1: Serialize + Send + Sync;

    /// Type for a raw Merkle path into the reward-state-v2 (RewardMerkleTreeV2) tree
    type RewardStatePathV2: Serialize + Send + Sync;

    /// Get the height of the last persisted reward-state-v1 merklized state snapshot
    async fn get_reward_state_height(&self) -> anyhow::Result<u64>;

    /// Get the height of the last persisted reward-state-v2 merklized state snapshot
    async fn get_reward_state_v2_height(&self) -> anyhow::Result<u64>;

    /// Get Merkle proof for a reward account against the V1 (RewardMerkleTreeV1) tree
    ///
    /// # Arguments
    /// * `height` - Block height to query
    /// * `address` - Ethereum address to query proof for
    async fn get_reward_account_proof_v1(
        &self,
        height: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryDataV1>;

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
    async fn get_latest_reward_balance(
        &self,
        address: String,
    ) -> anyhow::Result<Self::RewardBalance>;

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

    /// Get the Merkle path for a key in the reward-state (RewardMerkleTreeV1) tree
    ///
    /// Mirrors `merklized_state::get_path`, inherited by the reward-state mount from
    /// `hotshot-query-service`'s base `state.toml` routes (same as block-state/fee-state).
    ///
    /// # Arguments
    /// * `snapshot` - Height or commitment identifying the tree snapshot
    /// * `key` - Reward account address to query
    async fn get_reward_state_path_v1(
        &self,
        snapshot: Snapshot,
        key: String,
    ) -> anyhow::Result<Self::RewardStatePathV1>;

    /// Get the Merkle path for a key in the reward-state-v2 (RewardMerkleTreeV2) tree
    ///
    /// # Arguments
    /// * `snapshot` - Height or commitment identifying the tree snapshot
    /// * `key` - Reward account address to query
    async fn get_reward_state_path_v2(
        &self,
        snapshot: Snapshot,
        key: String,
    ) -> anyhow::Result<Self::RewardStatePathV2>;
}
