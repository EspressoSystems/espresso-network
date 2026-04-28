//! Test API server example
//!
//! This example demonstrates the espresso-api by serving a test implementation
//! with mock data. Useful for testing API consumers without running a full node.
//!
//! Run with: cargo run --example test_api --package espresso-api
//! Then visit: http://localhost:5000 for Swagger documentation

use anyhow::Result;
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
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

// Implement v1::AvailabilityApi with test data
#[async_trait]
impl v1::AvailabilityApi for TestApi {
    type NamespaceProofQueryData = (Vec<u8>, Option<Vec<u8>>); // (transactions, proof)
    type IncorrectEncodingProof = Vec<u8>;
    type StateCertQueryDataV1 = Vec<u8>;
    type StateCertQueryDataV2 = Vec<u8>;

    async fn get_namespace_proof(
        &self,
        block_id: v1::availability::BlockId,
        namespace: u32,
    ) -> Result<Option<Self::NamespaceProofQueryData>> {
        tracing::info!(
            "v1: get_namespace_proof(block_id={:?}, namespace={})",
            block_id,
            namespace
        );
        Ok(Some((vec![0xaa, 0xbb, 0xcc], Some(vec![0x11, 0x22, 0x33]))))
    }

    async fn get_namespace_proof_range(
        &self,
        from: u64,
        until: u64,
        namespace: u32,
    ) -> Result<Vec<Self::NamespaceProofQueryData>> {
        tracing::info!(
            "v1: get_namespace_proof_range(from={}, until={}, namespace={})",
            from,
            until,
            namespace
        );
        Ok(vec![(vec![0xaa, 0xbb], Some(vec![0x11, 0x22]))])
    }

    async fn get_incorrect_encoding_proof(
        &self,
        block_id: v1::availability::BlockId,
        namespace: u32,
    ) -> Result<Self::IncorrectEncodingProof> {
        tracing::info!(
            "v1: get_incorrect_encoding_proof(block_id={:?}, namespace={})",
            block_id,
            namespace
        );
        Ok(vec![0xde, 0xad, 0xbe, 0xef])
    }

    async fn get_state_cert(&self, epoch: u64) -> Result<Self::StateCertQueryDataV1> {
        tracing::info!("v1: get_state_cert(epoch={})", epoch);
        Ok(vec![0x01, 0x02, 0x03])
    }

    async fn get_state_cert_v2(&self, epoch: u64) -> Result<Self::StateCertQueryDataV2> {
        tracing::info!("v1: get_state_cert_v2(epoch={})", epoch);
        Ok(vec![0x04, 0x05, 0x06])
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

// Implement v2::DataApi with test data
#[async_trait]
impl v2::DataApi for TestApi {
    async fn get_namespace_proof(
        &self,
        namespace_id: u32,
        block_height: u64,
    ) -> Result<Self::NamespaceProof> {
        tracing::info!(
            "v2: get_namespace_proof(namespace_id={}, block_height={})",
            namespace_id,
            block_height
        );
        // Return (transactions as Vec<Vec<u8>>, optional proof)
        Ok((vec![vec![0xaa, 0xbb, 0xcc]], Some(vec![0x11, 0x22, 0x33])))
    }

    async fn get_namespace_proof_range(
        &self,
        namespace_id: u32,
        from: u64,
        until: u64,
    ) -> Result<Vec<Self::NamespaceProof>> {
        tracing::info!(
            "v2: get_namespace_proof_range(namespace_id={}, from={}, until={})",
            namespace_id,
            from,
            until
        );
        Ok(vec![
            (vec![vec![0xaa, 0xbb]], Some(vec![0x11, 0x22])),
            (vec![vec![0xcc, 0xdd]], Some(vec![0x33, 0x44])),
        ])
    }

    async fn get_incorrect_encoding_proof(
        &self,
        namespace_id: u32,
        block_height: u64,
    ) -> Result<Self::IncorrectEncodingProof> {
        tracing::info!(
            "v2: get_incorrect_encoding_proof(namespace_id={}, block_height={})",
            namespace_id,
            block_height
        );
        Ok(vec![0xde, 0xad, 0xbe, 0xef])
    }
}

// Implement v2::ConsensusApi with test data
#[async_trait]
impl v2::ConsensusApi for TestApi {
    async fn get_state_certificate(&self, epoch: u64) -> Result<Self::StateCertificate> {
        tracing::info!("v2: get_state_certificate(epoch={})", epoch);
        Ok(vec![0x01, 0x02, 0x03, 0x04])
    }

    async fn get_stake_table(&self, epoch: u64) -> Result<Self::StakeTable> {
        tracing::info!("v2: get_stake_table(epoch={})", epoch);
        // Return Vec<Vec<u8>> - each entry represents a peer
        Ok(vec![
            vec![0x05, 0x06, 0x07, 0x08],
            vec![0x09, 0x0a, 0x0b, 0x0c],
        ])
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

    // Data API types
    type NamespaceProof = (Vec<Vec<u8>>, Option<Vec<u8>>); // (transactions, proof)
    type IncorrectEncodingProof = Vec<u8>;

    // Consensus API types
    type StateCertificate = Vec<u8>;
    type StakeTable = Vec<Vec<u8>>;

    // Helper conversion types (dummy types for test)
    type PeerConfig = Vec<u8>;
    type LightClientCert = Vec<u8>;
    type NsProof = Vec<u8>;

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

    // Data API serialization methods

    fn serialize_namespace_proof(
        &self,
        value: &Self::NamespaceProof,
    ) -> Result<serialization_api::v2::NamespaceProofResponse> {
        let (transactions, proof_bytes) = value;

        // Convert transactions to Transaction messages
        let txs = transactions
            .iter()
            .map(|tx| serialization_api::v2::Transaction {
                namespace: 0,
                payload: STANDARD.encode(tx),
            })
            .collect();

        // Convert proof bytes to NsProof if present
        let proof = proof_bytes.as_ref().map(|bytes| {
            // Create a dummy NsProof for testing
            serialization_api::v2::NsProof {
                proof_version: Some(serialization_api::v2::ns_proof::ProofVersion::V0(
                    serialization_api::v2::AdvzNsProof {
                        namespace_id: 0,
                        ns_payload: String::new(),
                        ns_proof: Some(STANDARD.encode(bytes)),
                    },
                )),
            }
        });

        Ok(serialization_api::v2::NamespaceProofResponse {
            transactions: txs,
            proof,
        })
    }

    fn serialize_incorrect_encoding_proof(
        &self,
        value: &Self::IncorrectEncodingProof,
    ) -> Result<serialization_api::v2::IncorrectEncodingProofResponse> {
        Ok(serialization_api::v2::IncorrectEncodingProofResponse {
            proof: Some(serialization_api::v2::AvidMIncorrectEncodingNsProof {
                proof_data: STANDARD.encode(value),
            }),
        })
    }

    // Consensus API serialization methods

    fn serialize_state_certificate(
        &self,
        value: &Self::StateCertificate,
    ) -> Result<serialization_api::v2::StateCertificateResponse> {
        Ok(serialization_api::v2::StateCertificateResponse {
            certificate: Some(serialization_api::v2::LightClientStateUpdateCertificateV2 {
                epoch: 0,
                light_client_state: String::new(),
                next_stake_table_state: String::new(),
                signatures: vec![],
                auth_root: STANDARD.encode(value),
            }),
        })
    }

    fn serialize_stake_table(
        &self,
        value: &Self::StakeTable,
    ) -> Result<serialization_api::v2::StakeTableResponse> {
        // Convert each entry to a PeerConfig
        let peers = value
            .iter()
            .map(|peer_bytes| serialization_api::v2::PeerConfig {
                stake_table_entry: Some(serialization_api::v2::StakeTableEntry {
                    stake_key: Some(serialization_api::v2::BlsPublicKey {
                        key: STANDARD.encode(peer_bytes),
                    }),
                    stake_amount: "1000000".to_string(),
                }),
                state_ver_key: Some(serialization_api::v2::SchnorrPublicKey {
                    key: STANDARD.encode(peer_bytes),
                }),
                connect_info: None,
            })
            .collect();

        Ok(serialization_api::v2::StakeTableResponse { peers })
    }

    fn serialize_peer_config(
        &self,
        _peer: &Self::PeerConfig,
    ) -> Result<serialization_api::v2::PeerConfig> {
        Ok(serialization_api::v2::PeerConfig {
            stake_table_entry: None,
            state_ver_key: None,
            connect_info: None,
        })
    }

    fn serialize_light_client_cert(
        &self,
        _cert: &Self::LightClientCert,
    ) -> Result<serialization_api::v2::LightClientStateUpdateCertificateV2> {
        Ok(serialization_api::v2::LightClientStateUpdateCertificateV2 {
            epoch: 0,
            light_client_state: String::new(),
            next_stake_table_state: String::new(),
            signatures: vec![],
            auth_root: String::new(),
        })
    }

    fn serialize_ns_proof(&self, _proof: &Self::NsProof) -> Result<serialization_api::v2::NsProof> {
        Ok(serialization_api::v2::NsProof {
            proof_version: None,
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
