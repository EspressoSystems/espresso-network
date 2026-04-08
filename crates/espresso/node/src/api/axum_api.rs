use alloy::primitives::Address;
use async_trait::async_trait;
use espresso_api::{DataSource, DataSourceError, RewardClaimData};
use espresso_types::v0_6::RewardClaimError;

use super::RewardMerkleTreeDataSource;

/// Newtype wrapper that bridges `RewardMerkleTreeDataSource` to `espresso_api::DataSource`.
/// Required because of the orphan rule (both trait and type parameter are foreign).
#[derive(Clone)]
pub(crate) struct ApiDataSource<D>(pub D);

#[async_trait]
impl<D> DataSource for ApiDataSource<D>
where
    D: RewardMerkleTreeDataSource,
{
    async fn get_reward_claim_input(
        &self,
        block_height: u64,
        address: &str,
    ) -> Result<RewardClaimData, DataSourceError> {
        let addr: Address = address
            .parse()
            .map_err(|_| DataSourceError::BadRequest(format!("invalid address: {address}")))?;

        let proof = self
            .0
            .load_reward_account_proof_v2(block_height, addr.into())
            .await
            .map_err(|e| DataSourceError::Internal(format!("failed to load proof: {e}")))?;

        let claim_input = proof.to_reward_claim_input().map_err(|err| match err {
            RewardClaimError::ZeroRewardError => DataSourceError::NotFound(format!(
                "zero rewards for {address} at height {block_height}"
            )),
            RewardClaimError::ProofConversionError(e) => {
                DataSourceError::Internal(format!("proof conversion failed: {e}"))
            },
        })?;

        let auth_bytes: alloy::primitives::Bytes = claim_input.auth_data.into();
        Ok(RewardClaimData {
            lifetime_rewards: claim_input.lifetime_rewards.to_string(),
            auth_data_hex: format!("{auth_bytes}"),
        })
    }
}
