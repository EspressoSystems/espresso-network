//! RewardApi trait implementations for espresso-node
//!
//! This module provides implementations for both v1::RewardApi (internal types)
//! and v2::RewardApi (proto types), backed by the same data source.

use alloy::primitives::U256;
use async_trait::async_trait;
use espresso_types::{
    v0::sparse_mt::KeccakNode,
    v0_3::RewardAmount as InternalRewardAmount,
    v0_4::{
        RewardAccountProofV2 as InternalRewardAccountProofV2,
        RewardAccountQueryDataV2 as InternalRewardAccountQueryData, RewardAccountV2,
        RewardMerkleProofV2 as InternalRewardMerkleProofV2,
    },
    v0_6::RewardClaimError,
};
use hotshot_contract_adapter::reward::RewardClaimInput as InternalRewardClaimInput;
use jf_merkle_tree_compat::prelude::{
    MerkleNode as InternalMerkleNode, MerkleProof as InternalMerkleProof,
};
use serde_json;
use serialization_api::v2::{
    self, RewardAccountProofV2, RewardAccountQueryDataV2, RewardBalance, RewardBalances,
    RewardClaimInput, RewardMerkleProofV2, RewardMerkleTreeV2Data, merkle_node,
    reward_merkle_proof_v2::ProofType,
};
use tagged_base64::TaggedBase64;

use super::{RewardMerkleTreeDataSource, RewardMerkleTreeV2Data as InternalRewardTreeData};

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

    /// Convert RewardAccountProofV2 to proto
    fn convert_reward_account_proof_v2(
        &self,
        proof: &InternalRewardAccountProofV2,
    ) -> anyhow::Result<RewardAccountProofV2> {
        Ok(RewardAccountProofV2 {
            account: format!("{:#x}", proof.account),
            proof: Some(self.convert_reward_merkle_proof_v2(&proof.proof)?),
        })
    }

    /// Convert RewardMerkleProofV2 enum to proto
    fn convert_reward_merkle_proof_v2(
        &self,
        proof: &InternalRewardMerkleProofV2,
    ) -> anyhow::Result<RewardMerkleProofV2> {
        let proof_type = match proof {
            InternalRewardMerkleProofV2::Presence(p) => {
                ProofType::Presence(self.convert_merkle_proof(p)?)
            },
            InternalRewardMerkleProofV2::Absence(p) => {
                ProofType::Absence(self.convert_merkle_proof(p)?)
            },
        };

        Ok(RewardMerkleProofV2 {
            proof_type: Some(proof_type),
        })
    }

    /// Convert MerkleProof to proto
    fn convert_merkle_proof(
        &self,
        proof: &InternalMerkleProof<InternalRewardAmount, RewardAccountV2, KeccakNode, 2>,
    ) -> anyhow::Result<v2::MerkleProof> {
        let proof_nodes: Result<Vec<v2::MerkleNode>, _> = proof
            .proof
            .iter()
            .map(|node| self.convert_merkle_node(node))
            .collect();

        Ok(v2::MerkleProof {
            pos: TaggedBase64::new("FIELD", proof.pos.0.as_slice())
                .map_err(|e| anyhow::anyhow!("failed to encode proof pos: {}", e))?
                .to_string(),
            proof: proof_nodes?,
        })
    }

    /// Convert MerkleNode to proto (recursive)
    fn convert_merkle_node(
        &self,
        node: &InternalMerkleNode<InternalRewardAmount, RewardAccountV2, KeccakNode>,
    ) -> anyhow::Result<v2::MerkleNode> {
        let node_type = match node {
            InternalMerkleNode::Empty => merkle_node::NodeType::Empty(v2::Empty {
                dummy: Some(v2::EmptyData {}),
            }),
            InternalMerkleNode::Leaf { pos, elem, value } => {
                merkle_node::NodeType::Leaf(v2::Leaf {
                    pos: TaggedBase64::new("FIELD", pos.0.as_slice())
                        .map_err(|e| anyhow::anyhow!("failed to encode leaf pos: {}", e))?
                        .to_string(),
                    elem: TaggedBase64::new("FIELD", &elem.0.to_le_bytes::<32>())
                        .map_err(|e| anyhow::anyhow!("failed to encode leaf elem: {}", e))?
                        .to_string(),
                    value: TaggedBase64::new("FIELD", &value.0)
                        .map_err(|e| anyhow::anyhow!("failed to encode leaf value: {}", e))?
                        .to_string(),
                })
            },
            InternalMerkleNode::Branch { value, children } => {
                let proto_children: Result<Vec<v2::MerkleNode>, _> = children
                    .iter()
                    .map(|child| self.convert_merkle_node(child))
                    .collect();

                merkle_node::NodeType::Branch(v2::Branch {
                    value: TaggedBase64::new("FIELD", &value.0)
                        .map_err(|e| anyhow::anyhow!("failed to encode branch value: {}", e))?
                        .to_string(),
                    children: proto_children?,
                })
            },
            InternalMerkleNode::ForgettenSubtree { value } => {
                merkle_node::NodeType::ForgottenSubtree(v2::ForgottenSubtree {
                    value: TaggedBase64::new("FIELD", &value.0)
                        .map_err(|e| {
                            anyhow::anyhow!("failed to encode forgotten subtree value: {}", e)
                        })?
                        .to_string(),
                })
            },
        };

        Ok(v2::MerkleNode {
            node_type: Some(node_type),
        })
    }
}

// ============================================================================
// ApiSerializations implementation (conversion layer)
// ============================================================================

impl<D> serialization_api::ApiSerializations for NodeApiStateImpl<D>
where
    D: RewardMerkleTreeDataSource,
{
    // Request types
    type Address = alloy::primitives::Address;

    // Response types (internal types)
    type RewardClaimInput = InternalRewardClaimInput;
    type RewardBalance = U256;
    type RewardAccountQueryData = InternalRewardAccountQueryData;
    type RewardBalances = (Vec<(RewardAccountV2, InternalRewardAmount)>, u64); // (amounts, total)
    type RewardMerkleTreeData = InternalRewardTreeData;

    // Deserialize proto/string types → internal types
    fn deserialize_address(&self, s: &str) -> anyhow::Result<Self::Address> {
        s.parse()
            .map_err(|_| anyhow::anyhow!("invalid ethereum address: {}", s))
    }

    // Serialize internal types → proto types
    fn serialize_reward_claim_input(
        &self,
        address: &str,
        value: &Self::RewardClaimInput,
    ) -> anyhow::Result<RewardClaimInput> {
        // Serialize auth_data directly to match serde's hex encoding
        let auth_data = serde_json::to_value(&value.auth_data)
            .and_then(|v| {
                v.as_str()
                    .ok_or_else(|| serde_json::Error::custom("auth_data not a string"))
                    .map(|s| s.to_string())
            })
            .map_err(|e| anyhow::anyhow!("failed to serialize auth_data: {}", e))?;

        Ok(RewardClaimInput {
            address: address.to_string(),
            lifetime_rewards: format!("{:#x}", value.lifetime_rewards), // Hex for contract
            auth_data,
        })
    }

    fn serialize_reward_balance(
        &self,
        value: &Self::RewardBalance,
    ) -> anyhow::Result<RewardBalance> {
        Ok(RewardBalance {
            amount: value.to_string(), // Decimal string
        })
    }

    fn serialize_reward_account_query_data(
        &self,
        value: &Self::RewardAccountQueryData,
    ) -> anyhow::Result<RewardAccountQueryDataV2> {
        // Convert balance to decimal string
        let balance = value.balance.to_string();

        // Convert the proof
        let proof = Some(self.convert_reward_account_proof_v2(&value.proof)?);

        Ok(RewardAccountQueryDataV2 { balance, proof })
    }

    fn serialize_reward_balances(
        &self,
        value: &Self::RewardBalances,
    ) -> anyhow::Result<RewardBalances> {
        let (amounts_vec, total) = value;

        // Convert each account/amount pair to proto format
        let amounts = amounts_vec
            .iter()
            .map(|(account, amount)| serialization_api::v2::RewardAmount {
                address: format!("{:#x}", account.0),
                amount: amount.0.to_string(), // Decimal string
            })
            .collect();

        Ok(RewardBalances {
            amounts,
            total: *total,
        })
    }

    fn serialize_reward_merkle_tree_data(
        &self,
        value: &Self::RewardMerkleTreeData,
    ) -> anyhow::Result<RewardMerkleTreeV2Data> {
        let bytes = bincode::serialize(value)
            .map_err(|e| anyhow::anyhow!("failed to serialize RewardMerkleTreeV2Data: {}", e))?;
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
        address: Self::Address,
    ) -> anyhow::Result<Self::RewardClaimInput> {
        // Load the latest reward account proof from the data source
        let proof = self
            .data_source
            .load_latest_reward_account_proof_v2(address.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to load latest reward account {:?}: {}",
                    address,
                    err
                )
            })?;

        // Convert the proof to reward claim input and return internal type
        proof.to_reward_claim_input().map_err(|err| match err {
            RewardClaimError::ZeroRewardError => {
                anyhow::anyhow!("zero reward balance for {:?}", address)
            },
            RewardClaimError::ProofConversionError(e) => {
                anyhow::anyhow!("failed to create solidity proof for {:?}: {}", address, e)
            },
        })
    }

    async fn get_reward_balance(
        &self,
        address: Self::Address,
    ) -> anyhow::Result<Self::RewardBalance> {
        // Load the latest reward account proof from the data source
        let proof = self
            .data_source
            .load_latest_reward_account_proof_v2(address.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to load latest reward account {:?}: {}",
                    address,
                    err
                )
            })?;

        // Return internal balance type
        Ok(proof.balance)
    }

    async fn get_reward_account_proof(
        &self,
        address: Self::Address,
    ) -> anyhow::Result<Self::RewardAccountQueryData> {
        // Load the latest reward account proof from the data source and return internal type
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

    async fn get_reward_balances(
        &self,
        height: u64,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Self::RewardBalances> {
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
        let addr: alloy::primitives::Address = address
            .parse()
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
        let addr: alloy::primitives::Address = address
            .parse()
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

    async fn get_latest_reward_balance(
        &self,
        address: String,
    ) -> anyhow::Result<Self::RewardBalance> {
        // Parse the Ethereum address
        let addr: alloy::primitives::Address = address
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid ethereum address: {}", address))?;

        // Load the latest reward account proof from the data source
        let proof = self
            .data_source
            .load_latest_reward_account_proof_v2(addr.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!("failed to load latest reward account {}: {}", address, err)
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
        let addr: alloy::primitives::Address = address
            .parse()
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
        let addr: alloy::primitives::Address = address
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid ethereum address: {}", address))?;

        // Load and return the latest reward account proof directly (internal type)
        let proof = self
            .data_source
            .load_latest_reward_account_proof_v2(addr.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!("failed to load latest reward account {}: {}", address, err)
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
