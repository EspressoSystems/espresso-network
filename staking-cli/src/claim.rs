use alloy::{
    primitives::{Address, U256},
    providers::Provider,
};
use anyhow::{bail, Context as _, Result};
use hotshot_contract_adapter::{
    reward::RewardClaimInput,
    sol_types::{EspTokenV2, LightClientV3, RewardClaim, StakeTableV2},
};
use url::Url;

use crate::transaction::Transaction;

struct RewardClaimData {
    reward_claim_address: Address,
    claim_input: RewardClaimInput,
}

async fn fetch_reward_claim_data(
    provider: impl Provider,
    stake_table_address: Address,
    espresso_url: &Url,
    claimer_address: Address,
) -> Result<Option<RewardClaimData>> {
    let stake_table = StakeTableV2::new(stake_table_address, &provider);
    let token_address = stake_table
        .token()
        .call()
        .await
        .context("Failed to get token address from stake table")?;

    let esp_token = EspTokenV2::new(token_address, &provider);
    let reward_claim_address = esp_token
        .rewardClaim()
        .call()
        .await
        .context("Failed to get reward claim address from token contract")?;

    if reward_claim_address == Address::ZERO {
        bail!("Reward claim contract not set on ESP token");
    }

    let light_client_address = stake_table
        .lightClient()
        .call()
        .await
        .context("Failed to get light client address from stake table")?;
    let light_client = LightClientV3::new(light_client_address, &provider);

    let auth_root = light_client
        .authRoot()
        .call()
        .await
        .context("Failed to get auth root from light client")?;

    if auth_root == U256::ZERO {
        return Ok(None);
    }

    let finalized_state = light_client
        .finalizedState()
        .call()
        .await
        .context("Failed to get finalized state from light client")?;
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
        .await
        .context("Failed to fetch reward claim input from Espresso API")?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }

    let response = response
        .error_for_status()
        .context("Espresso API returned error status")?;

    let claim_input: RewardClaimInput = response
        .json()
        .await
        .context("Failed to parse reward claim input from API response")?;

    Ok(Some(RewardClaimData {
        reward_claim_address,
        claim_input,
    }))
}

pub async fn unclaimed_rewards(
    provider: impl Provider,
    stake_table_address: Address,
    espresso_url: Url,
    claimer_address: Address,
) -> Result<U256> {
    let Some(data) = fetch_reward_claim_data(
        &provider,
        stake_table_address,
        &espresso_url,
        claimer_address,
    )
    .await?
    else {
        return Ok(U256::ZERO);
    };

    let reward_claim = RewardClaim::new(data.reward_claim_address, &provider);
    let already_claimed = reward_claim.claimedRewards(claimer_address).call().await?;

    let unclaimed = data
        .claim_input
        .lifetime_rewards
        .checked_sub(already_claimed)
        .unwrap_or(U256::ZERO);

    Ok(unclaimed)
}

/// Fetch reward claim inputs from the Espresso API and return a Transaction for claiming rewards
pub async fn fetch_claim_rewards_inputs(
    provider: impl Provider,
    stake_table_address: Address,
    espresso_url: &Url,
    claimer_address: Address,
) -> Result<Option<Transaction>> {
    let Some(data) = fetch_reward_claim_data(
        &provider,
        stake_table_address,
        espresso_url,
        claimer_address,
    )
    .await?
    else {
        return Ok(None);
    };

    Ok(Some(Transaction::ClaimRewards {
        reward_claim: data.reward_claim_address,
        lifetime_rewards: data.claim_input.lifetime_rewards,
        auth_data: data.claim_input.auth_data.into(),
    }))
}

#[cfg(test)]
mod test {
    use alloy::primitives::{utils::parse_ether, U256};
    use hotshot_contract_adapter::sol_types::{RewardClaim, StakeTableV2};
    use warp::Filter as _;

    use super::*;
    use crate::{deploy::TestSystem, receipt::ReceiptExt as _, transaction::Transaction};

    #[tokio::test]
    async fn test_claim_withdrawal() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let amount = parse_ether("1.23")?;
        system.register_validator().await?;
        system.delegate(amount).await?;
        system.undelegate(amount).await?;
        system.warp_to_unlock_time().await?;

        let validator_address = system.deployer_address;
        let tx = Transaction::ClaimWithdrawal {
            stake_table: system.stake_table,
            validator: validator_address,
        };
        let receipt = tx.send(&system.provider).await?.assert_success().await?;

        let event = receipt
            .decoded_log::<StakeTableV2::WithdrawalClaimed>()
            .unwrap();
        assert_eq!(event.amount, amount);

        Ok(())
    }

    #[tokio::test]
    async fn test_claim_validator_exit() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let amount = parse_ether("1.23")?;
        system.register_validator().await?;
        system.delegate(amount).await?;
        system.deregister_validator().await?;
        system.warp_to_unlock_time().await?;

        let validator_address = system.deployer_address;
        let tx = Transaction::ClaimValidatorExit {
            stake_table: system.stake_table,
            validator: validator_address,
        };
        let receipt = tx.send(&system.provider).await?.assert_success().await?;

        let event = receipt
            .decoded_log::<StakeTableV2::ValidatorExitClaimed>()
            .unwrap();
        assert_eq!(event.amount, amount);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_claim_reward() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let reward_balance = U256::from(1000000);

        let espresso_url = system.setup_reward_claim_mock(reward_balance).await?;

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let balance_before = system.balance(system.deployer_address).await?;

        let tx = fetch_claim_rewards_inputs(
            &system.provider,
            system.stake_table,
            &espresso_url,
            system.deployer_address,
        )
        .await?
        .expect("claim inputs should be available");

        let receipt = tx.send(&system.provider).await?.assert_success().await?;

        let event = receipt
            .decoded_log::<RewardClaim::RewardsClaimed>()
            .unwrap();
        assert_eq!(event.amount, reward_balance);

        let balance_after = system.balance(system.deployer_address).await?;
        assert_eq!(balance_after, balance_before + reward_balance);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_unclaimed_rewards_not_found() -> Result<()> {
        let system = TestSystem::deploy().await?;

        let port = portpicker::pick_unused_port().expect("No ports available");

        let route = warp::path!("reward-state-v2" / "reward-claim-input" / u64 / String)
            .map(|_, _| warp::reply::with_status(warp::reply(), warp::http::StatusCode::NOT_FOUND));

        tokio::spawn(warp::serve(route).run(([127, 0, 0, 1], port)));

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let espresso_url = format!("http://localhost:{}/", port).parse()?;

        let unclaimed = unclaimed_rewards(
            &system.provider,
            system.stake_table,
            espresso_url,
            system.deployer_address,
        )
        .await?;

        assert_eq!(unclaimed, U256::ZERO);

        Ok(())
    }
}
