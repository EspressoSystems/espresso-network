//! Rewards API trait for v2
//!
//! Accepts parsed request types and returns implementation-defined types.
//! Implementations must also implement EspressoSerializations for conversions.

use async_trait::async_trait;
use serialization_api::EspressoSerializations;

/// Reward API trait - accepts parsed request types, returns internal types
///
/// Implementations define their own types and provide conversions via EspressoSerializations.
/// Handlers in espresso-api handle parsing proto requests and converting responses.
#[async_trait]
pub trait RewardApi: EspressoSerializations {

    /// Get reward claim input for L1 contract submission
    ///
    /// Returns all data needed to call the claimRewards function on the L1 contract,
    /// including lifetime rewards and the Merkle proof.
    ///
    /// # Arguments
    /// * `block_height` - Must match the height finalized in the Light Client contract
    /// * `address` - Parsed Ethereum address (already validated)
    async fn get_reward_claim_input(
        &self,
        block_height: u64,
        address: Self::Address,
    ) -> anyhow::Result<Self::RewardClaimInput>;

    /// Get reward balance at a specific height
    ///
    /// # Arguments
    /// * `height` - Block height to query
    /// * `address` - Parsed Ethereum address (already validated)
    async fn get_reward_balance(
        &self,
        height: u64,
        address: Self::Address,
    ) -> anyhow::Result<Self::RewardBalance>;

    /// Get latest reward balance at the most recent finalized height
    ///
    /// # Arguments
    /// * `address` - Parsed Ethereum address (already validated)
    async fn get_latest_reward_balance(&self, address: Self::Address) -> anyhow::Result<Self::RewardBalance>;

    /// Get Merkle proof for a reward account at a specific height
    ///
    /// Returns complete query data with balance and expanded merkle proof
    ///
    /// # Arguments
    /// * `height` - Block height to query
    /// * `address` - Parsed Ethereum address (already validated)
    async fn get_reward_account_proof(
        &self,
        height: u64,
        address: Self::Address,
    ) -> anyhow::Result<Self::RewardAccountQueryData>;

    /// Get Merkle proof for a reward account at the latest finalized height
    ///
    /// Returns complete query data with balance and expanded merkle proof
    ///
    /// # Arguments
    /// * `address` - Parsed Ethereum address (already validated)
    async fn get_latest_reward_account_proof(
        &self,
        address: Self::Address,
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
