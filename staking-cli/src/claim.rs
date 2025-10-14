use alloy::{
    network::Ethereum,
    primitives::Address,
    providers::{PendingTransactionBuilder, Provider},
};
use anyhow::{bail, Result};
use hotshot_contract_adapter::{
    evm::DecodeRevert as _,
    reward::RewardClaimInput,
    sol_types::{
        EspTokenV2, LightClientV3, RewardClaim,
        StakeTable::{self, StakeTableErrors},
        StakeTableV2,
    },
};
use url::Url;

pub async fn claim_withdrawal(
    provider: impl Provider,
    stake_table: Address,
    validator_address: Address,
) -> Result<PendingTransactionBuilder<Ethereum>> {
    let st = StakeTable::new(stake_table, provider);
    st.claimWithdrawal(validator_address)
        .send()
        .await
        .maybe_decode_revert::<StakeTableErrors>()
}

pub async fn claim_validator_exit(
    provider: impl Provider,
    stake_table: Address,
    validator_address: Address,
) -> Result<PendingTransactionBuilder<Ethereum>> {
    let st = StakeTable::new(stake_table, provider);
    st.claimValidatorExit(validator_address)
        .send()
        .await
        .maybe_decode_revert::<StakeTableErrors>()
}

pub async fn claim_reward(
    provider: impl Provider + Clone,
    stake_table_address: Address,
    espresso_url: Url,
    claimer_address: Address,
) -> Result<PendingTransactionBuilder<Ethereum>> {
    let stake_table = StakeTableV2::new(stake_table_address, &provider);
    let token_address = stake_table.token().call().await?;

    let esp_token = EspTokenV2::new(token_address, &provider);
    let reward_claim_address = esp_token.rewardClaim().call().await?;

    if reward_claim_address == Address::ZERO {
        bail!("Reward claim contract not set on ESP token");
    }

    let light_client_address = stake_table.lightClient().call().await?;
    let light_client = LightClientV3::new(light_client_address, &provider);
    let finalized_state = light_client.finalizedState().call().await?;
    let block_height = finalized_state.blockHeight;

    let reward_claim_url = format!(
        "{}reward-state-v2/reward-claim-input/{}/{}",
        espresso_url, block_height, claimer_address
    );

    let http_client = reqwest::Client::new();
    let response = http_client
        .get(&reward_claim_url)
        .header("Accept", "application/json")
        .send()
        .await?
        .error_for_status()?;

    let claim_input: RewardClaimInput = response.json().await?;

    let reward_claim = RewardClaim::new(reward_claim_address, provider);
    reward_claim
        .claimRewards(claim_input.lifetime_rewards, claim_input.auth_data.into())
        .send()
        .await
        .map_err(Into::into)
}

#[cfg(test)]
mod test {
    use alloy::primitives::U256;

    use super::*;
    use crate::{deploy::TestSystem, receipt::ReceiptExt};

    #[tokio::test]
    async fn test_claim_withdrawal() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let amount = U256::from(123);
        system.register_validator().await?;
        system.delegate(amount).await?;
        system.undelegate(amount).await?;
        system.warp_to_unlock_time().await?;

        let validator_address = system.deployer_address;
        let receipt = claim_withdrawal(&system.provider, system.stake_table, validator_address)
            .await?
            .assert_success()
            .await?;

        let event = receipt.decoded_log::<StakeTable::Withdrawal>().unwrap();
        assert_eq!(event.amount, amount);

        Ok(())
    }

    #[tokio::test]
    async fn test_claim_validator_exit() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let amount = U256::from(123);
        system.register_validator().await?;
        system.delegate(amount).await?;
        system.deregister_validator().await?;
        system.warp_to_unlock_time().await?;

        let validator_address = system.deployer_address;
        let receipt = claim_validator_exit(&system.provider, system.stake_table, validator_address)
            .await?
            .assert_success()
            .await?;

        let event = receipt.decoded_log::<StakeTable::Withdrawal>().unwrap();
        assert_eq!(event.amount, amount);

        Ok(())
    }
}
