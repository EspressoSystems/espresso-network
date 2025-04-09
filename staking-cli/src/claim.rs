use alloy::{primitives::Address, providers::Provider, rpc::types::TransactionReceipt};
use anyhow::Result;
use hotshot_contract_adapter::sol_types::StakeTable;

pub async fn claim_withdrawal(
    provider: impl Provider,
    stake_table: Address,
    validator_address: Address,
) -> Result<TransactionReceipt> {
    let st = StakeTable::new(stake_table, provider);
    // See if there are any logs
    Ok(st
        .claimWithdrawal(validator_address)
        .send()
        .await?
        .get_receipt()
        .await?)
}

pub async fn claim_validator_exit(
    provider: impl Provider,
    stake_table: Address,
    validator_address: Address,
) -> Result<TransactionReceipt> {
    let st = StakeTable::new(stake_table, provider);
    Ok(st
        .claimValidatorExit(validator_address)
        .send()
        .await?
        .get_receipt()
        .await?)
}

#[cfg(test)]
mod test {
    use alloy::primitives::U256;

    use super::*;
    use crate::{deploy::TestSystem, l1::decode_log};

    #[tokio::test]
    async fn test_claim_withdrawal() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let amount = U256::from(123);
        system.register_validator().await?;
        system.delegate(amount).await?;
        system.undelegate(amount).await?;
        system.warp_to_unlock_time().await?;

        let validator_address = system.deployer_address;
        let receipt =
            claim_withdrawal(&system.provider, system.stake_table, validator_address).await?;
        assert!(receipt.status());

        let event = decode_log::<StakeTable::Withdrawal>(&receipt).unwrap();
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
        let receipt =
            claim_validator_exit(&system.provider, system.stake_table, validator_address).await?;
        assert!(receipt.status());

        let event = decode_log::<StakeTable::Withdrawal>(&receipt).unwrap();
        assert_eq!(event.amount, amount);

        Ok(())
    }
}
