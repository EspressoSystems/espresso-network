//! Type conversions for API v2
//!
//! This module centralizes all conversions between internal espresso-types
//! and serialization-api proto types.

use alloy::primitives::{Address, U256};
use anyhow::Result;
use espresso_types::{
    v0::sparse_mt::KeccakNode,
    v0_3::RewardAmount as InternalRewardAmount,
    v0_4::{
        RewardAccountProofV2 as InternalRewardAccountProofV2, RewardAccountQueryDataV2,
        RewardAccountV2, RewardMerkleProofV2 as InternalRewardMerkleProofV2,
    },
};
use hotshot_contract_adapter::reward::RewardClaimInput as InternalRewardClaimInput;
use jf_merkle_tree_compat::prelude::{
    MerkleNode as InternalMerkleNode, MerkleProof as InternalMerkleProof,
};
use serialization_api::v2::{
    self, RewardAccountProofV2, RewardMerkleProofV2, merkle_node,
};
use tagged_base64::TaggedBase64;

/// Utility functions for type conversions
pub mod util {
    use super::*;

    /// Convert U256 to hex string with "0x" prefix
    ///
    /// # Example
    /// ```ignore
    /// let value = U256::from(12345u64);
    /// assert_eq!(u256_to_hex_string(value), "0x3039");
    /// ```
    pub fn u256_to_hex_string(value: U256) -> String {
        format!("{:#x}", value)
    }

    /// Parse hex string to U256
    ///
    /// Accepts strings with or without "0x" prefix
    pub fn hex_string_to_u256(s: &str) -> Result<U256> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        U256::from_str_radix(s, 16).map_err(|e| anyhow::anyhow!("invalid hex string: {}", e))
    }
}

/// Conversions from internal types to proto types
pub mod to_proto {
    use super::*;

    /// Convert U256 balance to proto RewardBalance
    pub fn reward_balance(balance: U256) -> v2::RewardBalance {
        v2::RewardBalance {
            amount: util::u256_to_hex_string(balance),
        }
    }

    /// Convert internal RewardClaimInput to proto
    pub fn reward_claim_input(
        address: String,
        claim_input: &InternalRewardClaimInput,
    ) -> Result<v2::RewardClaimInput> {
        Ok(v2::RewardClaimInput {
            address,
            lifetime_rewards: claim_input.lifetime_rewards.to_string(),
            auth_data: bincode::serialize(&claim_input.auth_data)
                .map_err(|e| anyhow::anyhow!("failed to serialize auth_data: {}", e))?,
        })
    }

    /// Convert RewardAccountQueryDataV2 to proto (full expansion)
    pub fn reward_account_query_data_v2(
        data: &RewardAccountQueryDataV2,
    ) -> Result<v2::RewardAccountQueryDataV2> {
        Ok(v2::RewardAccountQueryDataV2 {
            balance: util::u256_to_hex_string(data.balance),
            proof: Some(reward_account_proof_v2(&data.proof)?),
        })
    }

    /// Convert RewardAccountProofV2 to proto
    fn reward_account_proof_v2(
        proof: &InternalRewardAccountProofV2,
    ) -> Result<RewardAccountProofV2> {
        Ok(RewardAccountProofV2 {
            account: format!("{:#x}", proof.account),
            proof: Some(reward_merkle_proof_v2(&proof.proof)?),
        })
    }

    /// Convert RewardMerkleProofV2 enum to proto
    fn reward_merkle_proof_v2(proof: &InternalRewardMerkleProofV2) -> Result<RewardMerkleProofV2> {
        use serialization_api::v2::reward_merkle_proof_v2::ProofType;

        let proof_type = match proof {
            InternalRewardMerkleProofV2::Presence(p) => ProofType::Presence(merkle_proof(p)?),
            InternalRewardMerkleProofV2::Absence(p) => ProofType::Absence(merkle_proof(p)?),
        };

        Ok(RewardMerkleProofV2 {
            proof_type: Some(proof_type),
        })
    }

    /// Convert MerkleProof to proto
    fn merkle_proof(
        proof: &InternalMerkleProof<InternalRewardAmount, RewardAccountV2, KeccakNode, 2>,
    ) -> Result<v2::MerkleProof> {
        let proof_nodes: Result<Vec<v2::MerkleNode>> =
            proof.proof.iter().map(|node| merkle_node(node)).collect();

        Ok(v2::MerkleProof {
            pos: TaggedBase64::new("FIELD", proof.pos.0.as_slice())
                .map_err(|e| anyhow::anyhow!("failed to encode proof pos: {}", e))?
                .to_string(),
            proof: proof_nodes?,
        })
    }

    /// Convert MerkleNode to proto (recursive)
    fn merkle_node(
        node: &InternalMerkleNode<InternalRewardAmount, RewardAccountV2, KeccakNode>,
    ) -> Result<v2::MerkleNode> {
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
                let proto_children: Result<Vec<v2::MerkleNode>> =
                    children.iter().map(|child| merkle_node(child)).collect();

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

    /// Convert single account/amount pair to proto RewardAmount
    pub fn reward_amount(
        account: &RewardAccountV2,
        amount: &InternalRewardAmount,
    ) -> v2::RewardAmount {
        v2::RewardAmount {
            address: format!("{:#x}", account.0),
            amount: util::u256_to_hex_string(amount.0),
        }
    }

    /// Convert slice of account/amount pairs to vec of proto RewardAmount
    pub fn reward_amounts(
        balances: &[(RewardAccountV2, InternalRewardAmount)],
    ) -> Vec<v2::RewardAmount> {
        balances
            .iter()
            .map(|(account, amount)| reward_amount(account, amount))
            .collect()
    }
}

/// Conversions from proto types to internal types
pub mod from_proto {
    use super::*;

    /// Parse Ethereum address string
    ///
    /// Accepts hex strings with or without "0x" prefix
    pub fn parse_address(s: &str) -> Result<Address> {
        s.parse()
            .map_err(|_| anyhow::anyhow!("invalid ethereum address: {}", s))
    }
}
