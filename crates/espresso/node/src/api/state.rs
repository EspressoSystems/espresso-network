//! RewardApi trait implementations for espresso-node
//!
//! This module provides implementations for both v1::RewardApi (internal types)
//! and v2::RewardApi (proto types), backed by the same data source.

use alloy::primitives::U256;
use async_trait::async_trait;
use espresso_types::v0_3::RewardAmount as InternalRewardAmount;
use espresso_types::v0_4::{
    RewardAccountQueryDataV2 as InternalRewardAccountQueryData,
    RewardAccountV2,
};
use espresso_types::v0_6::RewardClaimError;
use hotshot_contract_adapter::reward::RewardClaimInput as InternalRewardClaimInput;
use serialization_api::v2::{
    RewardAccountQueryDataV2, RewardAmounts, RewardBalance, RewardClaimInput,
    RewardMerkleTreeV2Data,
};

use super::{
    RewardMerkleTreeDataSource, RewardMerkleTreeV2Data as InternalRewardTreeData,
    conversions::v2::{from_proto, to_proto},
};

/// Node API state implementation
///
/// This struct implements both v1::RewardApi (internal types) and v2::RewardApi (proto types).
#[derive(Clone)]
pub struct NodeApiStateImpl<D> {
    data_source: D,
}

impl<D> NodeApiStateImpl<D> {
    pub fn new(data_source: D) -> Self {
        Self { data_source }
    }
}

// ============================================================================
// EspressoSerializations implementation (conversion layer)
// ============================================================================

impl<D> serialization_api::EspressoSerializations for NodeApiStateImpl<D>
where
    D: RewardMerkleTreeDataSource,
{
    // Request types
    type Address = alloy::primitives::Address;

    // Response types (internal types)
    type RewardClaimInput = InternalRewardClaimInput;
    type RewardBalance = U256;
    type RewardAccountQueryData = InternalRewardAccountQueryData;
    type RewardAmounts = (Vec<(RewardAccountV2, InternalRewardAmount)>, u64); // (amounts, total)
    type RewardMerkleTreeData = InternalRewardTreeData;

    // Deserialize proto/string types → internal types
    fn deserialize_address(&self, s: &str) -> anyhow::Result<Self::Address> {
        from_proto::parse_address(s)
    }

    // Serialize internal types → proto types
    fn serialize_reward_claim_input(
        &self,
        address: &str,
        value: &Self::RewardClaimInput,
    ) -> anyhow::Result<RewardClaimInput> {
        to_proto::reward_claim_input(address.to_string(), value)
    }

    fn serialize_reward_balance(
        &self,
        value: &Self::RewardBalance,
    ) -> anyhow::Result<RewardBalance> {
        Ok(to_proto::reward_balance(*value))
    }

    fn serialize_reward_account_query_data(
        &self,
        value: &Self::RewardAccountQueryData,
    ) -> anyhow::Result<RewardAccountQueryDataV2> {
        to_proto::reward_account_query_data_v2(value)
    }

    fn serialize_reward_amounts(
        &self,
        value: &Self::RewardAmounts,
    ) -> anyhow::Result<RewardAmounts> {
        let (amounts_vec, total) = value;
        let amounts = to_proto::reward_amounts(amounts_vec);
        Ok(RewardAmounts {
            amounts,
            total: *total,
        })
    }

    fn serialize_reward_merkle_tree_data(
        &self,
        value: &Self::RewardMerkleTreeData,
    ) -> anyhow::Result<RewardMerkleTreeV2Data> {
        let bytes = bincode::serialize(value).map_err(|e| {
            anyhow::anyhow!("failed to serialize RewardMerkleTreeV2Data: {}", e)
        })?;
        Ok(RewardMerkleTreeV2Data { data: bytes })
    }
}

// ============================================================================
// RewardApiV2 implementation (business logic)
// ============================================================================

#[async_trait]
impl<D> espresso_api::v2::RewardApi for NodeApiStateImpl<D>
where
    D: RewardMerkleTreeDataSource,
{

    async fn get_reward_claim_input(
        &self,
        block_height: u64,
        address: Self::Address,
    ) -> anyhow::Result<Self::RewardClaimInput> {
        // Load the reward account proof from the data source
        let proof = self
            .data_source
            .load_reward_account_proof_v2(block_height, address.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to load reward account {:?} at height {}: {}",
                    address,
                    block_height,
                    err
                )
            })?;

        // Convert the proof to reward claim input and return internal type
        proof.to_reward_claim_input().map_err(|err| match err {
            RewardClaimError::ZeroRewardError => {
                anyhow::anyhow!(
                    "zero reward balance for {:?} at height {}",
                    address,
                    block_height
                )
            },
            RewardClaimError::ProofConversionError(e) => {
                anyhow::anyhow!(
                    "failed to create solidity proof for {:?} at height {}: {}",
                    address,
                    block_height,
                    e
                )
            },
        })
    }

    async fn get_reward_balance(
        &self,
        height: u64,
        address: Self::Address,
    ) -> anyhow::Result<Self::RewardBalance> {
        // Load the reward account proof from the data source
        let proof = self
            .data_source
            .load_reward_account_proof_v2(height, address.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to load reward account {:?} at height {}: {}",
                    address,
                    height,
                    err
                )
            })?;

        // Return internal balance type
        Ok(proof.balance)
    }

    async fn get_latest_reward_balance(&self, address: Self::Address) -> anyhow::Result<Self::RewardBalance> {
        // Load the latest reward account proof
        let proof = self
            .data_source
            .load_latest_reward_account_proof_v2(address.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to load latest reward account for {:?}: {}",
                    address,
                    err
                )
            })?;

        // Return internal balance type
        Ok(proof.balance)
    }

    async fn get_reward_account_proof(
        &self,
        height: u64,
        address: Self::Address,
    ) -> anyhow::Result<Self::RewardAccountQueryData> {
        // Load the reward account proof from the data source and return internal type
        self.data_source
            .load_reward_account_proof_v2(height, address.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to load reward account proof for {:?} at height {}: {}",
                    address,
                    height,
                    err
                )
            })
    }

    async fn get_latest_reward_account_proof(
        &self,
        address: Self::Address,
    ) -> anyhow::Result<Self::RewardAccountQueryData> {
        // Load the latest reward account proof and return internal type
        self.data_source
            .load_latest_reward_account_proof_v2(address.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to load latest reward account proof for {:?}: {}",
                    address,
                    err
                )
            })
    }

    async fn get_reward_amounts(
        &self,
        height: u64,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Self::RewardAmounts> {
        // Validate limit (from reward.toml: limit <= 10000)
        if limit > 10000 {
            return Err(anyhow::anyhow!(
                "limit {} exceeds maximum allowed value of 10000",
                limit
            ));
        }

        // Load the merkle tree at the given height
        let tree_bytes = self.data_source.load_tree(height).await.map_err(|err| {
            anyhow::anyhow!("failed to load reward tree at height {}: {}", height, err)
        })?;

        // Deserialize the tree into internal format
        let tree_data: InternalRewardTreeData =
            bincode::deserialize(&tree_bytes).map_err(|err| {
                anyhow::anyhow!(
                    "failed to deserialize RewardMerkleTreeV2Data at height {}: {}",
                    height,
                    err
                )
            })?;

        let offset_usize = offset as usize;
        let limit_usize = limit as usize;
        let end = std::cmp::min(offset_usize + limit_usize, tree_data.balances.len());

        let total = tree_data.balances.len() as u64;

        // Get the slice
        let slice = tree_data
            .balances
            .get(offset_usize..end)
            .ok_or_else(|| anyhow::anyhow!("offset {} out of bounds", offset))?;

        // Reverse order (matching Tide implementation) and return internal type with total
        let reversed: Vec<_> = slice.iter().rev().copied().collect();
        Ok((reversed, total))
    }

    async fn get_reward_merkle_tree_v2(
        &self,
        height: u64,
    ) -> anyhow::Result<Self::RewardMerkleTreeData> {
        // Load the raw merkle tree bytes
        let tree_bytes = self.data_source.load_tree(height).await.map_err(|err| {
            anyhow::anyhow!("failed to load reward tree at height {}: {}", height, err)
        })?;

        // Deserialize and return internal type
        bincode::deserialize(&tree_bytes).map_err(|err| {
            anyhow::anyhow!(
                "failed to deserialize RewardMerkleTreeV2Data at height {}: {}",
                height,
                err
            )
        })
    }
}

// ============================================================================
// RewardApiV1 implementation (internal types, no proto conversion)
// ============================================================================

#[async_trait]
impl<D> espresso_api::v1::RewardApi for NodeApiStateImpl<D>
where
    D: RewardMerkleTreeDataSource,
{
    type RewardClaimInput = InternalRewardClaimInput;
    type RewardBalance = U256;
    type RewardAccountQueryData = InternalRewardAccountQueryData;
    type RewardAmounts = Vec<(alloy::primitives::Address, U256)>;
    type RewardMerkleTreeData = InternalRewardTreeData;

    async fn get_reward_claim_input(
        &self,
        block_height: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardClaimInput> {
        // Parse the Ethereum address
        let addr: alloy::primitives::Address = address.parse()
            .map_err(|_| anyhow::anyhow!("invalid ethereum address: {}", address))?;

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

        // Convert the proof to reward claim input (internal type)
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

        Ok(claim_input)
    }

    async fn get_reward_balance(
        &self,
        height: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardBalance> {
        // Parse the Ethereum address
        let addr: alloy::primitives::Address = address.parse()
            .map_err(|_| anyhow::anyhow!("invalid ethereum address: {}", address))?;

        // Load the reward account proof from the data source
        let proof = self
            .data_source
            .load_reward_account_proof_v2(height, addr.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to load reward account {} at height {}: {}",
                    address,
                    height,
                    err
                )
            })?;

        // Return the balance directly (U256)
        Ok(proof.balance)
    }

    async fn get_latest_reward_balance(&self, address: String) -> anyhow::Result<Self::RewardBalance> {
        // Parse the Ethereum address
        let addr: alloy::primitives::Address = address.parse()
            .map_err(|_| anyhow::anyhow!("invalid ethereum address: {}", address))?;

        // Load the latest reward account proof from the data source
        let proof = self
            .data_source
            .load_latest_reward_account_proof_v2(addr.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to load latest reward account {}: {}",
                    address,
                    err
                )
            })?;

        // Return the balance directly (U256)
        Ok(proof.balance)
    }

    async fn get_reward_account_proof(
        &self,
        height: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryData> {
        // Parse the Ethereum address
        let addr: alloy::primitives::Address = address.parse()
            .map_err(|_| anyhow::anyhow!("invalid ethereum address: {}", address))?;

        // Load and return the reward account proof directly (internal type)
        let proof = self
            .data_source
            .load_reward_account_proof_v2(height, addr.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to load reward account {} at height {}: {}",
                    address,
                    height,
                    err
                )
            })?;

        Ok(proof)
    }

    async fn get_latest_reward_account_proof(
        &self,
        address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryData> {
        // Parse the Ethereum address
        let addr: alloy::primitives::Address = address.parse()
            .map_err(|_| anyhow::anyhow!("invalid ethereum address: {}", address))?;

        // Load and return the latest reward account proof directly (internal type)
        let proof = self
            .data_source
            .load_latest_reward_account_proof_v2(addr.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to load latest reward account {}: {}",
                    address,
                    err
                )
            })?;

        Ok(proof)
    }

    async fn get_reward_amounts(
        &self,
        height: u64,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Self::RewardAmounts> {
        // Validate limit (from reward.toml: limit <= 10000)
        if limit > 10000 {
            return Err(anyhow::anyhow!(
                "limit {} exceeds maximum allowed value of 10000",
                limit
            ));
        }

        // Load the merkle tree at the given height
        let tree_bytes = self.data_source.load_tree(height).await.map_err(|err| {
            anyhow::anyhow!("failed to load reward tree at height {}: {}", height, err)
        })?;

        // Deserialize the tree into internal format
        let tree_data: InternalRewardTreeData =
            bincode::deserialize(&tree_bytes).map_err(|err| {
                anyhow::anyhow!(
                    "failed to deserialize RewardMerkleTreeV2Data at height {}: {}",
                    height,
                    err
                )
            })?;

        let offset_usize = offset as usize;
        let limit_usize = limit as usize;
        let end = std::cmp::min(offset_usize + limit_usize, tree_data.balances.len());

        // Get the slice
        let slice = tree_data
            .balances
            .get(offset_usize..end)
            .ok_or_else(|| anyhow::anyhow!("offset {} out of bounds", offset))?;

        // Reverse order (matching Tide implementation) and convert to (Address, U256)
        let result: Vec<(alloy::primitives::Address, U256)> = slice
            .iter()
            .rev()
            .map(|(account, amount)| (account.0, amount.0))
            .collect();

        Ok(result)
    }

    async fn get_reward_merkle_tree_v2(
        &self,
        height: u64,
    ) -> anyhow::Result<Self::RewardMerkleTreeData> {
        // Load the raw merkle tree bytes
        let tree_bytes = self.data_source.load_tree(height).await.map_err(|err| {
            anyhow::anyhow!("failed to load reward tree at height {}: {}", height, err)
        })?;

        // Deserialize to internal RewardMerkleTreeV2Data
        let tree_data: InternalRewardTreeData =
            bincode::deserialize(&tree_bytes).map_err(|err| {
                anyhow::anyhow!(
                    "failed to deserialize RewardMerkleTreeV2Data at height {}: {}",
                    height,
                    err
                )
            })?;

        Ok(tree_data)
    }
}
