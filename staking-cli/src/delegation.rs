use alloy::{
    primitives::{Address, U256},
    providers::Provider,
    rpc::types::TransactionReceipt,
    transports::Transport,
};
use anyhow::Result;
use contract_bindings_alloy::staketable::StakeTable::StakeTableInstance;

pub async fn delegate<P: Provider<T>, T: Transport + Clone>(
    stake_table: StakeTableInstance<T, P>,
    validator_address: Address,
    amount: U256,
) -> Result<TransactionReceipt> {
    // TODO: needs alloy 0.12: use err.as_decoded_error::<StakeTableErrors>().unwrap();
    // to provide better error messages in case of failure
    Ok(stake_table
        .delegate(validator_address, amount)
        .send()
        .await?
        .get_receipt()
        .await?)
}

pub async fn undelegate<P: Provider<T>, T: Transport + Clone>(
    stake_table: StakeTableInstance<T, P>,
    validator_address: Address,
    amount: U256,
) -> Result<TransactionReceipt> {
    Ok(stake_table
        .undelegate(validator_address, amount)
        .send()
        .await?
        .get_receipt()
        .await?)
}

#[cfg(test)]
mod test {
    use contract_bindings_alloy::staketable::StakeTable::{self};

    use super::*;
    use crate::{deploy::TestSystem, l1::decode_log};

    #[tokio::test]
    async fn test_delegate() -> Result<()> {
        let system = TestSystem::deploy().await?;
        system.register_validator().await?;
        let validator_address = system.deployer_address;

        let amount = U256::from(123);
        let receipt = delegate(system.stake_table, validator_address, amount).await?;
        assert!(receipt.status());

        let event = decode_log::<StakeTable::Delegated>(&receipt).unwrap();
        assert_eq!(event.validator, validator_address);
        assert_eq!(event.amount, amount);

        Ok(())
    }

    #[tokio::test]
    async fn test_undelegate() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let amount = U256::from(123);
        system.register_validator().await?;
        system.delegate(amount).await?;

        let validator_address = system.deployer_address;
        let receipt = undelegate(system.stake_table, validator_address, amount).await?;
        assert!(receipt.status());

        let event = decode_log::<StakeTable::Undelegated>(&receipt).unwrap();
        assert_eq!(event.validator, validator_address);
        assert_eq!(event.amount, amount);

        Ok(())
    }
}
