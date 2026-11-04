//! V1 catchup API.

use async_trait::async_trait;
use espresso_types::{
    AccountQueryData, BlockMerkleTree, FeeAccount, FeeMerkleTree, Leaf2, SeqTypes,
    v0_3::{ChainConfig, RewardAccountQueryDataV1, RewardAccountV1, RewardMerkleTreeV1},
    v0_4::RewardAccountQueryDataV2,
};
use hotshot_types::simple_certificate::{Certificate2, LightClientStateUpdateCertificateV2};
use jf_merkle_tree_compat::MerkleTreeScheme;

#[async_trait]
pub trait CatchupApi {
    async fn get_account(
        &self,
        height: u64,
        view: u64,
        address: String,
    ) -> anyhow::Result<AccountQueryData>;

    async fn get_accounts(
        &self,
        height: u64,
        view: u64,
        accounts: Vec<FeeAccount>,
    ) -> anyhow::Result<FeeMerkleTree>;

    async fn get_blocks_frontier(
        &self,
        height: u64,
        view: u64,
    ) -> anyhow::Result<<BlockMerkleTree as MerkleTreeScheme>::MembershipProof>;

    async fn get_chain_config(&self, commitment: String) -> anyhow::Result<ChainConfig>;

    async fn get_leaf_chain(&self, height: u64) -> anyhow::Result<Vec<Leaf2>>;

    async fn get_cert2(&self, height: u64) -> anyhow::Result<Certificate2<SeqTypes>>;

    async fn get_reward_account_v1(
        &self,
        height: u64,
        view: u64,
        address: String,
    ) -> anyhow::Result<RewardAccountQueryDataV1>;

    async fn get_reward_accounts_v1(
        &self,
        height: u64,
        view: u64,
        accounts: Vec<RewardAccountV1>,
    ) -> anyhow::Result<RewardMerkleTreeV1>;

    async fn get_reward_account_v2(
        &self,
        height: u64,
        view: u64,
        address: String,
    ) -> anyhow::Result<RewardAccountQueryDataV2>;

    async fn get_reward_merkle_tree_v2(
        &self,
        height: u64,
        view: u64,
    ) -> anyhow::Result<serde_json::Value>;

    async fn get_state_cert(
        &self,
        epoch: u64,
    ) -> anyhow::Result<LightClientStateUpdateCertificateV2<SeqTypes>>;
}
