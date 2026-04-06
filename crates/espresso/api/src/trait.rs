//! Core API trait definition

use async_trait::async_trait;
use serialization_api::v1::{NamespaceProofQueryData, RewardClaimInput};

/// Node API trait defining the core business logic
#[async_trait]
pub trait NodeApi {
    /// Get namespace proof and transactions by height and namespace ID
    async fn get_namespace_proof(
        &self,
        height: u64,
        namespace: u64,
    ) -> anyhow::Result<NamespaceProofQueryData>;

    /// Get reward claim input for contract submission
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
}

/// State struct for the node API
///
/// This currently has a dummy implementation returning hardcoded values.
/// The real implementation will eventually live in crates/espresso/node.
#[derive(Clone, Default)]
pub struct NodeApiState;

#[async_trait]
impl NodeApi for NodeApiState {
    async fn get_namespace_proof(
        &self,
        _height: u64,
        _namespace: u64,
    ) -> anyhow::Result<NamespaceProofQueryData> {
        Ok(NamespaceProofQueryData {
            proof: None,
            transactions: vec![],
        })
    }

    async fn get_reward_claim_input(
        &self,
        _block_height: u64,
        address: String,
    ) -> anyhow::Result<RewardClaimInput> {
        Ok(RewardClaimInput {
            address,
            lifetime_rewards: "0".to_string(),
            auth_data: vec![],
        })
    }
}
