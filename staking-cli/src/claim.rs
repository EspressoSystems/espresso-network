use alloy::{
    primitives::Address, providers::Provider, rpc::types::TransactionReceipt, transports::Transport,
};
use anyhow::Result;
use contract_bindings_alloy::staketable::StakeTable::StakeTableInstance;

pub async fn claim_withdrawal<P: Provider<T>, T: Transport + Clone>(
    stake_table: StakeTableInstance<T, P>,
    validator_address: Address,
) -> Result<TransactionReceipt> {
    // See if there are any logs
    Ok(stake_table
        .claimWithdrawal(validator_address)
        .send()
        .await?
        .get_receipt()
        .await?)
}

pub async fn claim_validator_exit<P: Provider<T>, T: Transport + Clone>(
    stake_table: StakeTableInstance<T, P>,
    validator_address: Address,
) -> Result<TransactionReceipt> {
    Ok(stake_table
        .claimValidatorExit(validator_address)
        .send()
        .await?
        .get_receipt()
        .await?)
}

#[cfg(test)]
mod test {
    use alloy::primitives::U256;
    use contract_bindings_alloy::staketable::StakeTable::{self};

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
        let receipt = claim_withdrawal(system.stake_table, validator_address).await?;
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
        let receipt = claim_validator_exit(system.stake_table, validator_address).await?;
        assert!(receipt.status());

        let event = decode_log::<StakeTable::Withdrawal>(&receipt).unwrap();
        assert_eq!(event.amount, amount);

        Ok(())
    }
}
