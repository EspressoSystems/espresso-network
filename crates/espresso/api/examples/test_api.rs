//! Test API server example
//!
//! This example demonstrates the espresso-api by serving a test implementation
//! with mock data. Useful for testing API consumers without running a full node.
//!
//! Run with: cargo run --example test_api --package espresso-api
//! Then visit: http://localhost:5000 for Swagger documentation

use anyhow::Result;
use async_trait::async_trait;
use espresso_api::{v1, v2};
use serde::Serialize;
use serialization_api::ApiSerializations;

/// Port for the test API server
const API_PORT: u16 = 5000;

/// Test API implementation with hardcoded mock data
#[derive(Clone)]
struct TestApi;

// Mock data types
#[derive(Serialize)]
struct MockRewardClaimInput {
    lifetime_rewards: u128,
    auth_data: Vec<u8>,
}

// Implement v1::RewardApi with test data
#[async_trait]
impl v1::RewardApi for TestApi {
    type RewardClaimInput = MockRewardClaimInput;
    type RewardBalance = u128;
    type RewardAccountQueryData = (u128, Vec<u8>);
    type RewardAmounts = (Vec<(u128, u128)>, u64);
    type RewardMerkleTreeData = Vec<u8>;

    async fn get_reward_claim_input(
        &self,
        block_height: u64,
        address: String,
    ) -> Result<Self::RewardClaimInput> {
        tracing::info!(
            "v1: get_reward_claim_input(height={}, address={})",
            block_height,
            address
        );

        Ok(MockRewardClaimInput {
            lifetime_rewards: 1_000_000_000_000_000_000, // 1 ESP in wei
            auth_data: vec![0x01, 0x02, 0x03, 0x04],     // Dummy auth data
        })
    }

    async fn get_reward_balance(
        &self,
        height: u64,
        address: String,
    ) -> Result<Self::RewardBalance> {
        tracing::info!(
            "v1: get_reward_balance(height={}, address={})",
            height,
            address
        );
        Ok(500_000_000_000_000_000) // 0.5 ESP
    }

    async fn get_latest_reward_balance(&self, address: String) -> Result<Self::RewardBalance> {
        tracing::info!("v1: get_latest_reward_balance(address={})", address);
        Ok(750_000_000_000_000_000) // 0.75 ESP
    }

    async fn get_reward_account_proof(
        &self,
        height: u64,
        address: String,
    ) -> Result<Self::RewardAccountQueryData> {
        tracing::info!(
            "v1: get_reward_account_proof(height={}, address={})",
            height,
            address
        );
        Ok((500_000_000_000_000_000, vec![0xde, 0xad, 0xbe, 0xef]))
    }

    async fn get_latest_reward_account_proof(
        &self,
        address: String,
    ) -> Result<Self::RewardAccountQueryData> {
        tracing::info!("v1: get_latest_reward_account_proof(address={})", address);
        Ok((750_000_000_000_000_000, vec![0xca, 0xfe, 0xba, 0xbe]))
    }

    async fn get_reward_amounts(
        &self,
        height: u64,
        offset: u64,
        limit: u64,
    ) -> Result<Self::RewardAmounts> {
        tracing::info!(
            "v1: get_reward_amounts(height={}, offset={}, limit={})",
            height,
            offset,
            limit
        );

        // Return dummy paginated results
        let amounts = vec![
            (0x1234567890abcdef, 100_000_000_000_000_000),
            (0xfedcba0987654321, 200_000_000_000_000_000),
            (0xaaaaaaaaaaaaaaaa, 300_000_000_000_000_000),
        ];

        Ok((amounts, 42)) // 42 total accounts
    }

    async fn get_reward_merkle_tree_v2(&self, height: u64) -> Result<Self::RewardMerkleTreeData> {
        tracing::info!("v1: get_reward_merkle_tree_v2(height={})", height);
        Ok(vec![0x00, 0x11, 0x22, 0x33, 0x44, 0x55])
    }
}

// Implement v2::RewardApi (simplified API - latest-only for claim/balance/proof)
#[async_trait]
impl v2::RewardApi for TestApi {
    async fn get_reward_claim_input(
        &self,
        address: Self::Address,
    ) -> Result<Self::RewardClaimInput> {
        // Delegate to v1's latest implementation
        <Self as v1::RewardApi>::get_latest_reward_balance(self, address.clone()).await?;
        // Return claim input with dummy height
        <Self as v1::RewardApi>::get_reward_claim_input(self, 9999, address).await
    }

    async fn get_reward_balance(&self, address: Self::Address) -> Result<Self::RewardBalance> {
        <Self as v1::RewardApi>::get_latest_reward_balance(self, address).await
    }

    async fn get_reward_account_proof(
        &self,
        address: Self::Address,
    ) -> Result<Self::RewardAccountQueryData> {
        <Self as v1::RewardApi>::get_latest_reward_account_proof(self, address).await
    }

    async fn get_reward_balances(
        &self,
        height: u64,
        offset: u64,
        limit: u64,
    ) -> Result<Self::RewardBalances> {
        <Self as v1::RewardApi>::get_reward_amounts(self, height, offset, limit).await
    }

    async fn get_reward_merkle_tree_v2(&self, height: u64) -> Result<Self::RewardMerkleTreeData> {
        <Self as v1::RewardApi>::get_reward_merkle_tree_v2(self, height).await
    }
}

// Implement ApiSerializations for v2 proto type conversions
impl ApiSerializations for TestApi {
    type Address = String;
    type RewardClaimInput = MockRewardClaimInput;
    type RewardBalance = u128;
    type RewardAccountQueryData = (u128, Vec<u8>);
    type RewardBalances = (Vec<(u128, u128)>, u64);
    type RewardMerkleTreeData = Vec<u8>;

    fn deserialize_address(&self, s: &str) -> Result<Self::Address> {
        // Simple validation: must start with 0x and be hex
        if s.starts_with("0x") && s.len() == 42 {
            Ok(s.to_string())
        } else {
            Err(anyhow::anyhow!(
                "Invalid address format: expected 0x followed by 40 hex characters"
            ))
        }
    }

    fn serialize_reward_claim_input(
        &self,
        address: &str,
        value: &Self::RewardClaimInput,
    ) -> Result<serialization_api::v2::RewardClaimInput> {
        Ok(serialization_api::v2::RewardClaimInput {
            address: address.to_string(),
            lifetime_rewards: format!("{:#x}", value.lifetime_rewards),
            auth_data: format!("0x{}", hex::encode(&value.auth_data)),
        })
    }

    fn serialize_reward_balance(
        &self,
        value: &Self::RewardBalance,
    ) -> Result<serialization_api::v2::RewardBalance> {
        Ok(serialization_api::v2::RewardBalance {
            amount: value.to_string(), // Decimal string
        })
    }

    fn serialize_reward_account_query_data(
        &self,
        value: &Self::RewardAccountQueryData,
    ) -> Result<serialization_api::v2::RewardAccountQueryDataV2> {
        let (balance, _proof_data) = value;

        // Create a minimal dummy proof
        Ok(serialization_api::v2::RewardAccountQueryDataV2 {
            balance: balance.to_string(),
            proof: Some(serialization_api::v2::RewardAccountProofV2 {
                account: "0x1234567890123456789012345678901234567890".to_string(),
                proof: Some(serialization_api::v2::RewardMerkleProofV2 {
                    proof_type: Some(
                        serialization_api::v2::reward_merkle_proof_v2::ProofType::Presence(
                            serialization_api::v2::MerkleProof {
                                pos: "FIELD~dummy_pos".to_string(),
                                proof: vec![],
                            },
                        ),
                    ),
                }),
            }),
        })
    }

    fn serialize_reward_balances(
        &self,
        value: &Self::RewardBalances,
    ) -> Result<serialization_api::v2::RewardBalances> {
        let (amounts_vec, total) = value;

        let amounts = amounts_vec
            .iter()
            .map(|(account, amount)| serialization_api::v2::RewardAmount {
                address: format!("{:#x}", account),
                amount: amount.to_string(),
            })
            .collect();

        Ok(serialization_api::v2::RewardBalances {
            amounts,
            total: *total,
        })
    }

    fn serialize_reward_merkle_tree_data(
        &self,
        value: &Self::RewardMerkleTreeData,
    ) -> Result<serialization_api::v2::RewardMerkleTreeV2Data> {
        Ok(serialization_api::v2::RewardMerkleTreeV2Data {
            data: value.clone(),
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("Starting test API server...");
    tracing::info!("");
    tracing::info!("Serving API at 127.0.0.1:{}", API_PORT);
    tracing::info!("");
    tracing::info!("API documentation: http://localhost:{}/", API_PORT);
    tracing::info!("Swagger UI:        http://localhost:{}/v2", API_PORT);
    tracing::info!("Scalar UI:         http://localhost:{}/v2/scalar", API_PORT);
    tracing::info!("Redoc UI:          http://localhost:{}/v2/redoc", API_PORT);
    tracing::info!(
        "OpenAPI spec:      http://localhost:{}/v2/docs/openapi.json",
        API_PORT
    );
    tracing::info!("");
    tracing::info!("Example API calls:");
    tracing::info!(
        "  V1 balance: curl http://localhost:{}/v1/reward-state-v2/reward-balance/100/0x1234567890123456789012345678901234567890",
        API_PORT
    );
    tracing::info!(
        "  V2 balance: curl http://localhost:{}/v2/rewards/balance/0x1234567890123456789012345678901234567890",
        API_PORT
    );
    tracing::info!(
        "  V2 balances: curl http://localhost:{}/v2/rewards/balances/100/0/10",
        API_PORT
    );
    tracing::info!("");

    let state = TestApi;

    // Start Axum server with combined v1 and v2 APIs
    espresso_api::serve_axum(API_PORT, state).await?;

    Ok(())
}
