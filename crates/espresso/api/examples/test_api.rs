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

    async fn stream_namespace_proofs(
        &self,
        _from: usize,
        _namespace: u32,
    ) -> Result<futures::stream::BoxStream<'static, Self::NamespaceProofQueryData>> {
        Ok(Box::pin(futures::stream::empty()))
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

// Stub HotShotAvailabilityApi for the example — returns empty/unit data.
#[async_trait]
impl v1::HotShotAvailabilityApi for TestApi {
    type Leaf = serde_json::Value;
    type Block = serde_json::Value;
    type Header = serde_json::Value;
    type Payload = serde_json::Value;
    type VidCommon = serde_json::Value;
    type Transaction = serde_json::Value;
    type TransactionWithProof = serde_json::Value;
    type BlockSummary = serde_json::Value;
    type Limits = serde_json::Value;
    type Cert2 = serde_json::Value;

    async fn get_leaf(&self, _id: v1::LeafId) -> Result<Self::Leaf> {
        Ok(serde_json::json!({}))
    }
    async fn get_leaf_range(&self, _from: usize, _until: usize) -> Result<Vec<Self::Leaf>> {
        Ok(vec![])
    }
    async fn get_header(&self, _id: v1::BlockId) -> Result<Self::Header> {
        Ok(serde_json::json!({}))
    }
    async fn get_header_range(&self, _from: usize, _until: usize) -> Result<Vec<Self::Header>> {
        Ok(vec![])
    }
    async fn get_block(&self, _id: v1::BlockId) -> Result<Self::Block> {
        Ok(serde_json::json!({}))
    }
    async fn get_block_range(&self, _from: usize, _until: usize) -> Result<Vec<Self::Block>> {
        Ok(vec![])
    }
    async fn get_payload(&self, _id: v1::PayloadId) -> Result<Self::Payload> {
        Ok(serde_json::json!({}))
    }
    async fn get_payload_range(&self, _from: usize, _until: usize) -> Result<Vec<Self::Payload>> {
        Ok(vec![])
    }
    async fn get_vid_common(&self, _id: v1::BlockId) -> Result<Self::VidCommon> {
        Ok(serde_json::json!({}))
    }
    async fn get_vid_common_range(
        &self,
        _from: usize,
        _until: usize,
    ) -> Result<Vec<Self::VidCommon>> {
        Ok(vec![])
    }
    async fn get_transaction_by_position(
        &self,
        _height: u64,
        _index: u64,
    ) -> Result<Self::Transaction> {
        Ok(serde_json::json!({}))
    }
    async fn get_transaction_by_hash(&self, _hash: String) -> Result<Self::Transaction> {
        Ok(serde_json::json!({}))
    }
    async fn get_transaction_proof_by_position(
        &self,
        _height: u64,
        _index: u64,
    ) -> Result<Self::TransactionWithProof> {
        Ok(serde_json::json!({}))
    }
    async fn get_transaction_proof_by_hash(
        &self,
        _hash: String,
    ) -> Result<Self::TransactionWithProof> {
        Ok(serde_json::json!({}))
    }
    async fn get_block_summary(&self, _height: usize) -> Result<Self::BlockSummary> {
        Ok(serde_json::json!({}))
    }
    async fn get_block_summary_range(
        &self,
        _from: usize,
        _until: usize,
    ) -> Result<Vec<Self::BlockSummary>> {
        Ok(vec![])
    }
    async fn get_limits(&self) -> Result<Self::Limits> {
        Ok(serde_json::json!({"small_object_range_limit": 500, "large_object_range_limit": 100}))
    }
    async fn get_cert2(&self, _height: u64) -> Result<Option<Self::Cert2>> {
        Ok(None)
    }
    async fn stream_leaves(
        &self,
        _from: usize,
    ) -> Result<futures::stream::BoxStream<'static, Self::Leaf>> {
        Ok(Box::pin(futures::stream::empty()))
    }
    async fn stream_headers(
        &self,
        _from: usize,
    ) -> Result<futures::stream::BoxStream<'static, Self::Header>> {
        Ok(Box::pin(futures::stream::empty()))
    }
    async fn stream_blocks(
        &self,
        _from: usize,
    ) -> Result<futures::stream::BoxStream<'static, Self::Block>> {
        Ok(Box::pin(futures::stream::empty()))
    }
    async fn stream_payloads(
        &self,
        _from: usize,
    ) -> Result<futures::stream::BoxStream<'static, Self::Payload>> {
        Ok(Box::pin(futures::stream::empty()))
    }
    async fn stream_vid_common(
        &self,
        _from: usize,
    ) -> Result<futures::stream::BoxStream<'static, Self::VidCommon>> {
        Ok(Box::pin(futures::stream::empty()))
    }
    async fn stream_transactions(
        &self,
        _from: usize,
        _namespace: Option<u32>,
    ) -> Result<futures::stream::BoxStream<'static, Self::Transaction>> {
        Ok(Box::pin(futures::stream::empty()))
    }
}

#[async_trait]
impl v1::BlockStateApi for TestApi {
    type MerkleProof = serde_json::Value;

    async fn get_block_state_path(
        &self,
        _snapshot: v1::Snapshot,
        _key: String,
    ) -> Result<Self::MerkleProof> {
        Ok(serde_json::Value::Null)
    }

    async fn get_block_state_height(&self) -> Result<u64> {
        Ok(0)
    }
}

#[async_trait]
impl v1::FeeStateApi for TestApi {
    type MerkleProof = serde_json::Value;
    type FeeAmount = u128;

    async fn get_fee_state_path(
        &self,
        _snapshot: v1::Snapshot,
        _key: String,
    ) -> Result<Self::MerkleProof> {
        Ok(serde_json::Value::Null)
    }

    async fn get_fee_state_height(&self) -> Result<u64> {
        Ok(0)
    }

    async fn get_fee_balance_latest(&self, _address: String) -> Result<Option<Self::FeeAmount>> {
        Ok(None)
    }
}

#[async_trait]
impl v1::StatusApi for TestApi {
    async fn block_height(&self) -> Result<u64> {
        Ok(0)
    }
    async fn success_rate(&self) -> Result<f64> {
        Ok(1.0)
    }
    async fn time_since_last_decide(&self) -> Result<u64> {
        Ok(0)
    }
    async fn metrics(&self) -> Result<String> {
        Ok(String::new())
    }
}

#[async_trait]
impl v1::ConfigApi for TestApi {
    type HotShotConfig = serde_json::Value;
    type RuntimeConfig = serde_json::Value;

    async fn hotshot_config(&self) -> Result<Self::HotShotConfig> {
        Ok(serde_json::Value::Null)
    }
    async fn env(&self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
    async fn runtime_config(&self) -> Result<Self::RuntimeConfig> {
        Ok(serde_json::Value::Null)
    }
}

#[async_trait]
impl v1::NodeApi for TestApi {
    type VidShare = serde_json::Value;
    type SyncStatus = serde_json::Value;
    type HeaderWindow = serde_json::Value;
    type Limits = serde_json::Value;
    type StakeTable = serde_json::Value;
    type StakeTableCurrent = serde_json::Value;
    type Validators = serde_json::Value;
    type AllValidators = serde_json::Value;
    type Participation = serde_json::Value;
    type BlockReward = serde_json::Value;
    type Block = serde_json::Value;
    type Leaf = serde_json::Value;

    async fn block_height(&self) -> Result<u64> {
        Ok(0)
    }
    async fn count_transactions(
        &self,
        _from: Option<u64>,
        _to: Option<u64>,
        _namespace: Option<u32>,
    ) -> Result<u64> {
        Ok(0)
    }
    async fn payload_size(
        &self,
        _from: Option<u64>,
        _to: Option<u64>,
        _namespace: Option<u32>,
    ) -> Result<u64> {
        Ok(0)
    }
    async fn get_vid_share(&self, _id: v1::VidShareId) -> Result<Self::VidShare> {
        Ok(serde_json::Value::Null)
    }
    async fn sync_status(&self) -> Result<Self::SyncStatus> {
        Ok(serde_json::Value::Null)
    }
    async fn get_header_window(
        &self,
        _start: v1::HeaderWindowStart,
        _end: u64,
    ) -> Result<Self::HeaderWindow> {
        Ok(serde_json::Value::Null)
    }
    async fn limits(&self) -> Result<Self::Limits> {
        Ok(serde_json::Value::Null)
    }
    async fn stake_table(&self, _epoch: u64) -> Result<Self::StakeTable> {
        Ok(serde_json::Value::Null)
    }
    async fn stake_table_current(&self) -> Result<Self::StakeTableCurrent> {
        Ok(serde_json::Value::Null)
    }
    async fn da_stake_table(&self, _epoch: u64) -> Result<Self::StakeTable> {
        Ok(serde_json::Value::Null)
    }
    async fn da_stake_table_current(&self) -> Result<Self::StakeTableCurrent> {
        Ok(serde_json::Value::Null)
    }
    async fn get_validators(&self, _epoch: u64) -> Result<Self::Validators> {
        Ok(serde_json::Value::Null)
    }
    async fn get_all_validators(
        &self,
        _epoch: u64,
        _offset: u64,
        _limit: u64,
    ) -> Result<Self::AllValidators> {
        Ok(serde_json::Value::Null)
    }
    async fn current_proposal_participation(&self) -> Result<Self::Participation> {
        Ok(serde_json::Value::Null)
    }
    async fn proposal_participation(&self, _epoch: u64) -> Result<Self::Participation> {
        Ok(serde_json::Value::Null)
    }
    async fn current_vote_participation(&self) -> Result<Self::Participation> {
        Ok(serde_json::Value::Null)
    }
    async fn vote_participation(&self, _epoch: u64) -> Result<Self::Participation> {
        Ok(serde_json::Value::Null)
    }
    async fn get_block_reward(&self, _epoch: Option<u64>) -> Result<Self::BlockReward> {
        Ok(serde_json::Value::Null)
    }
    async fn get_oldest_block(&self) -> Result<Option<Self::Block>> {
        Ok(None)
    }
    async fn get_oldest_leaf(&self) -> Result<Option<Self::Leaf>> {
        Ok(None)
    }
}

#[async_trait]
impl v1::CatchupApi for TestApi {
    type AccountQueryData = serde_json::Value;
    type FeeMerkleTree = serde_json::Value;
    type BlocksFrontier = serde_json::Value;
    type ChainConfig = serde_json::Value;
    type LeafChain = serde_json::Value;
    type Cert2 = serde_json::Value;
    type RewardAccountQueryDataV1 = serde_json::Value;
    type RewardMerkleTreeV1 = serde_json::Value;
    type RewardAccountQueryDataV2 = serde_json::Value;
    type RewardMerkleTreeV2Data = serde_json::Value;
    type StateCert = serde_json::Value;

    async fn get_account(
        &self,
        _height: u64,
        _view: u64,
        _address: String,
    ) -> Result<Self::AccountQueryData> {
        Ok(serde_json::Value::Null)
    }
    async fn get_accounts(
        &self,
        _height: u64,
        _view: u64,
        _accounts: Vec<String>,
    ) -> Result<Self::FeeMerkleTree> {
        Ok(serde_json::Value::Null)
    }
    async fn get_blocks_frontier(&self, _h: u64, _v: u64) -> Result<Self::BlocksFrontier> {
        Ok(serde_json::Value::Null)
    }
    async fn get_chain_config(&self, _c: String) -> Result<Self::ChainConfig> {
        Ok(serde_json::Value::Null)
    }
    async fn get_leaf_chain(&self, _h: u64) -> Result<Self::LeafChain> {
        Ok(serde_json::Value::Null)
    }
    async fn get_cert2(&self, _h: u64) -> Result<Self::Cert2> {
        Ok(serde_json::Value::Null)
    }
    async fn get_reward_account_v1(
        &self,
        _height: u64,
        _view: u64,
        _address: String,
    ) -> Result<Self::RewardAccountQueryDataV1> {
        Ok(serde_json::Value::Null)
    }
    async fn get_reward_accounts_v1(
        &self,
        _height: u64,
        _view: u64,
        _accounts: Vec<String>,
    ) -> Result<Self::RewardMerkleTreeV1> {
        Ok(serde_json::Value::Null)
    }
    async fn get_reward_account_v2(
        &self,
        _height: u64,
        _view: u64,
        _address: String,
    ) -> Result<Self::RewardAccountQueryDataV2> {
        Ok(serde_json::Value::Null)
    }
    async fn get_reward_merkle_tree_v2(
        &self,
        _height: u64,
        _view: u64,
    ) -> Result<Self::RewardMerkleTreeV2Data> {
        Ok(serde_json::Value::Null)
    }
    async fn get_state_cert(&self, _epoch: u64) -> Result<Self::StateCert> {
        Ok(serde_json::Value::Null)
    }
}

#[async_trait]
impl v1::SubmitApi for TestApi {
    type Transaction = serde_json::Value;
    type TxHash = serde_json::Value;

    async fn submit(&self, _tx: Self::Transaction) -> Result<Self::TxHash> {
        Ok(serde_json::Value::Null)
    }
}

#[async_trait]
impl v1::StateSignatureApi for TestApi {
    type Signature = serde_json::Value;

    async fn get_state_signature(&self, _height: u64) -> Result<Self::Signature> {
        Ok(serde_json::Value::Null)
    }
}

#[async_trait]
impl v1::HotShotEventsApi for TestApi {
    type Event = serde_json::Value;
    type StartupInfo = serde_json::Value;

    async fn startup_info(&self) -> Result<Self::StartupInfo> {
        Ok(serde_json::Value::Null)
    }
    async fn events(&self) -> Result<futures::stream::BoxStream<'static, Self::Event>> {
        Ok(Box::pin(futures::stream::empty()))
    }
}

#[async_trait]
impl v1::LightClientApi for TestApi {
    type LeafProof = serde_json::Value;
    type HeaderProof = serde_json::Value;
    type StakeTableEvents = serde_json::Value;
    type PayloadProof = serde_json::Value;
    type NamespaceProof = serde_json::Value;

    async fn get_leaf_proof(
        &self,
        _query: v1::LeafQuery,
        _finalized: Option<u64>,
    ) -> Result<Self::LeafProof> {
        Ok(serde_json::Value::Null)
    }
    async fn get_header_proof(
        &self,
        _root: u64,
        _requested: v1::HeaderQuery,
    ) -> Result<Self::HeaderProof> {
        Ok(serde_json::Value::Null)
    }
    async fn get_light_client_stake_table(&self, _epoch: u64) -> Result<Self::StakeTableEvents> {
        Ok(serde_json::Value::Null)
    }
    async fn get_payload_proof(&self, _height: u64) -> Result<Self::PayloadProof> {
        Ok(serde_json::Value::Null)
    }
    async fn get_payload_proof_range(
        &self,
        _start: u64,
        _end: u64,
    ) -> Result<Vec<Self::PayloadProof>> {
        Ok(vec![])
    }
    async fn get_lc_namespace_proof(
        &self,
        _height: u64,
        _namespace: u64,
    ) -> Result<Self::NamespaceProof> {
        Ok(serde_json::Value::Null)
    }
    async fn get_lc_namespace_proof_range(
        &self,
        _start: u64,
        _end: u64,
        _namespace: u64,
    ) -> Result<Vec<Self::NamespaceProof>> {
        Ok(vec![])
    }
}

#[async_trait]
impl v1::ExplorerApi for TestApi {
    type BlockDetail = serde_json::Value;
    type BlockSummaries = serde_json::Value;
    type TransactionDetail = serde_json::Value;
    type TransactionSummaries = serde_json::Value;
    type ExplorerSummary = serde_json::Value;
    type SearchResult = serde_json::Value;

    async fn get_block_detail(&self, _ident: v1::BlockIdent) -> Result<Self::BlockDetail> {
        Ok(serde_json::Value::Null)
    }
    async fn get_block_summaries(
        &self,
        _target: v1::BlockIdent,
        _limit: u64,
    ) -> Result<Self::BlockSummaries> {
        Ok(serde_json::Value::Null)
    }
    async fn get_transaction_detail(&self, _ident: v1::TxIdent) -> Result<Self::TransactionDetail> {
        Ok(serde_json::Value::Null)
    }
    async fn get_transaction_summaries(
        &self,
        _target: v1::TxIdent,
        _limit: u64,
        _filter: v1::TxSummaryFilter,
    ) -> Result<Self::TransactionSummaries> {
        Ok(serde_json::Value::Null)
    }
    async fn get_explorer_summary(&self) -> Result<Self::ExplorerSummary> {
        Ok(serde_json::Value::Null)
    }
    async fn get_search_result(&self, _query: String) -> Result<Self::SearchResult> {
        Ok(serde_json::Value::Null)
    }
}

#[async_trait]
impl v1::TokenApi for TestApi {
    async fn total_minted_supply(&self) -> Result<String> {
        Ok("0".to_string())
    }
    async fn circulating_supply(&self) -> Result<String> {
        Ok("0".to_string())
    }
    async fn circulating_supply_ethereum(&self) -> Result<String> {
        Ok("0".to_string())
    }
    async fn total_issued_supply(&self) -> Result<String> {
        Ok("0".to_string())
    }
    async fn total_reward_distributed(&self) -> Result<String> {
        Ok("0".to_string())
    }
}

#[async_trait]
impl v1::DatabaseApi for TestApi {
    type TableSizes = serde_json::Value;

    async fn get_table_sizes(&self) -> Result<Self::TableSizes> {
        Ok(serde_json::Value::Null)
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
        namespace_id: u64,
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
        namespace_id: u64,
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
        namespace_id: u64,
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
