use async_trait::async_trait;
use serialization_api::ApiSerializations;

#[async_trait]
pub trait RewardApi: ApiSerializations {
    async fn get_reward_claim_input(
        &self,
        address: Self::Address,
    ) -> anyhow::Result<Self::RewardClaimInput>;

    async fn get_reward_balance(
        &self,
        address: Self::Address,
    ) -> anyhow::Result<Self::RewardBalance>;

    async fn get_reward_account_proof(
        &self,
        address: Self::Address,
    ) -> anyhow::Result<Self::RewardAccountQueryData>;

    async fn get_reward_balances(
        &self,
        height: u64,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Self::RewardBalances>;

    async fn get_reward_merkle_tree_v2(
        &self,
        height: u64,
    ) -> anyhow::Result<Self::RewardMerkleTreeData>;
}
