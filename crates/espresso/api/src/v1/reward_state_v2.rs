//! Reward State V2 API trait for v1
//!
//! Returns internal espresso-types (legacy, no OpenAPI docs).

use alloy::primitives::Address;
use async_trait::async_trait;
use espresso_types::{
    v0::sparse_mt::KeccakNode,
    v0_3::{RewardAccountQueryDataV1, RewardAccountV1, RewardAmount, RewardMerkleTreeV1},
    v0_4::{RewardAccountQueryDataV2, RewardAccountV2, RewardMerkleTreeV2},
};
use hotshot_contract_adapter::reward::RewardClaimInput;
use jf_merkle_tree_compat::{
    MerkleTreeScheme,
    prelude::{MerkleProof, Sha3Node},
};

use crate::v1::merklized_state::Snapshot;

pub type RewardStatePathV1 = MerkleProof<
    RewardAmount,
    RewardAccountV1,
    Sha3Node,
    { <RewardMerkleTreeV1 as MerkleTreeScheme>::ARITY },
>;

pub type RewardStatePathV2 = MerkleProof<
    RewardAmount,
    RewardAccountV2,
    KeccakNode,
    { <RewardMerkleTreeV2 as MerkleTreeScheme>::ARITY },
>;

#[async_trait]
pub trait RewardApi {
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
    ) -> anyhow::Result<RewardAccountQueryDataV1>;

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
    ) -> anyhow::Result<RewardClaimInput>;

    /// Get reward balance at a specific height
    ///
    /// # Arguments
    /// * `height` - Block height to query
    /// * `address` - Ethereum address to query rewards for
    async fn get_reward_balance(
        &self,
        height: u64,
        address: String,
    ) -> anyhow::Result<RewardAmount>;

    /// Get latest reward balance at the most recent finalized height
    ///
    /// # Arguments
    /// * `address` - Ethereum address to query rewards for
    async fn get_latest_reward_balance(&self, address: String) -> anyhow::Result<RewardAmount>;

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
    ) -> anyhow::Result<RewardAccountQueryDataV2>;

    /// Get Merkle proof for a reward account at the latest finalized height
    ///
    /// Returns complete query data with balance and expanded merkle proof
    ///
    /// # Arguments
    /// * `address` - Ethereum address to query proof for
    async fn get_latest_reward_account_proof(
        &self,
        address: String,
    ) -> anyhow::Result<RewardAccountQueryDataV2>;

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
    ) -> anyhow::Result<Vec<(Address, RewardAmount)>>;

    /// Get raw RewardMerkleTreeV2 snapshot at a given height
    ///
    /// Returns the serialized merkle tree data
    ///
    /// # Arguments
    /// * `height` - Block height to query
    async fn get_reward_merkle_tree_v2(&self, height: u64) -> anyhow::Result<Vec<u8>>;

    /// Get the Merkle path for a key in the reward-state (RewardMerkleTreeV1) tree
    ///
    /// Mirrors the legacy `merklized_state` `get_path`, inherited by the reward-state mount
    /// from `hotshot-query-service`'s merklized-state base routes (same as block-state/fee-state).
    ///
    /// # Arguments
    /// * `snapshot` - Height or commitment identifying the tree snapshot
    /// * `key` - Reward account address to query
    async fn get_reward_state_path_v1(
        &self,
        snapshot: Snapshot,
        key: String,
    ) -> anyhow::Result<RewardStatePathV1>;

    /// Get the Merkle path for a key in the reward-state-v2 (RewardMerkleTreeV2) tree
    ///
    /// # Arguments
    /// * `snapshot` - Height or commitment identifying the tree snapshot
    /// * `key` - Reward account address to query
    async fn get_reward_state_path_v2(
        &self,
        snapshot: Snapshot,
        key: String,
    ) -> anyhow::Result<RewardStatePathV2>;
}
