//! NodeApi trait implementation for espresso-node

use alloy::primitives::Address;
use async_trait::async_trait;
use espresso_api::NodeApi;
use espresso_types::v0_6::RewardClaimError;
use serialization_api::v1::{NamespaceProofQueryData, RewardClaimInput};

use super::RewardMerkleTreeDataSource;

/// Node API state implementation wrapping a data source
///
/// This struct provides the real implementation of the NodeApi trait,
/// accessing sequencer state through the data source.
#[derive(Clone)]
pub struct NodeApiStateImpl<D> {
    data_source: D,
}

impl<D> NodeApiStateImpl<D> {
    pub fn new(data_source: D) -> Self {
        Self { data_source }
    }
}

#[async_trait]
impl<D> NodeApi for NodeApiStateImpl<D>
where
    D: RewardMerkleTreeDataSource,
{
    async fn get_namespace_proof(
        &self,
        _height: u64,
        _namespace: u64,
    ) -> anyhow::Result<NamespaceProofQueryData> {
        // TODO: Implement namespace proof retrieval
        // This requires access to block data and VID proofs
        Ok(NamespaceProofQueryData {
            proof: None,
            transactions: vec![],
        })
    }

    async fn get_reward_claim_input(
        &self,
        block_height: u64,
        address: String,
    ) -> anyhow::Result<RewardClaimInput> {
        // Parse the Ethereum address
        let addr: Address = address
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid reward address: {}", address))?;

        // Load the reward account proof from the data source
        let proof = self
            .data_source
            .load_reward_account_proof_v2(block_height, addr.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to load reward account {} at height {}: {}",
                    address,
                    block_height,
                    err
                )
            })?;

        // Convert the proof to reward claim input
        let claim_input = proof.to_reward_claim_input().map_err(|err| match err {
            RewardClaimError::ZeroRewardError => {
                anyhow::anyhow!(
                    "zero reward balance for {} at height {}",
                    address,
                    block_height
                )
            },
            RewardClaimError::ProofConversionError(e) => {
                anyhow::anyhow!(
                    "failed to create solidity proof for {} at height {}: {}",
                    address,
                    block_height,
                    e
                )
            },
        })?;

        // Convert hotshot_contract_adapter::RewardClaimInput to our proto RewardClaimInput
        Ok(RewardClaimInput {
            address,
            lifetime_rewards: claim_input.lifetime_rewards.to_string(),
            auth_data: bincode::serialize(&claim_input.auth_data)
                .map_err(|e| anyhow::anyhow!("failed to serialize auth_data: {}", e))?,
        })
    }
}
