//! V1 catchup API.
//!
//! Mirrors the tide-disco endpoints defined in `crates/espresso/node/api/catchup.toml`.

use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait CatchupApi {
    type AccountQueryData: Serialize + Send + Sync + 'static;
    type FeeMerkleTree: Serialize + Send + Sync + 'static;
    type BlocksFrontier: Serialize + Send + Sync + 'static;
    type ChainConfig: Serialize + Send + Sync + 'static;
    type LeafChain: Serialize + Send + Sync + 'static;
    type Cert2: Serialize + Send + Sync + 'static;
    type RewardAccountQueryDataV1: Serialize + Send + Sync + 'static;
    type RewardMerkleTreeV1: Serialize + Send + Sync + 'static;
    type RewardAccountQueryDataV2: Serialize + Send + Sync + 'static;
    type RewardMerkleTreeV2Data: Serialize + Send + Sync + 'static;
    type StateCert: Serialize + Send + Sync + 'static;

    async fn get_account(
        &self,
        height: u64,
        view: u64,
        address: String,
    ) -> anyhow::Result<Self::AccountQueryData>;

    async fn get_accounts(
        &self,
        height: u64,
        view: u64,
        accounts: Vec<String>,
    ) -> anyhow::Result<Self::FeeMerkleTree>;

    async fn get_blocks_frontier(
        &self,
        height: u64,
        view: u64,
    ) -> anyhow::Result<Self::BlocksFrontier>;

    async fn get_chain_config(&self, commitment: String) -> anyhow::Result<Self::ChainConfig>;

    async fn get_leaf_chain(&self, height: u64) -> anyhow::Result<Self::LeafChain>;

    async fn get_cert2(&self, height: u64) -> anyhow::Result<Self::Cert2>;

    async fn get_reward_account_v1(
        &self,
        height: u64,
        view: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryDataV1>;

    async fn get_reward_accounts_v1(
        &self,
        height: u64,
        view: u64,
        accounts: Vec<String>,
    ) -> anyhow::Result<Self::RewardMerkleTreeV1>;

    async fn get_reward_account_v2(
        &self,
        height: u64,
        view: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryDataV2>;

    async fn get_reward_merkle_tree_v2(
        &self,
        height: u64,
        view: u64,
    ) -> anyhow::Result<Self::RewardMerkleTreeV2Data>;

    async fn get_state_cert(&self, epoch: u64) -> anyhow::Result<Self::StateCert>;
}
