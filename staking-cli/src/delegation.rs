use alloy::{
    primitives::{Address, U256},
    providers::Provider,
    rpc::types::TransactionReceipt,
};
use anyhow::Result;
use hotshot_contract_adapter::sol_types::{
    EspToken::{self, EspTokenErrors},
    StakeTable::{self, StakeTableErrors},
};

use crate::l1::DecodeRevertError as _;

pub async fn approve(
    provider: impl Provider,
    token_addr: Address,
    stake_table_address: Address,
    amount: U256,
) -> Result<TransactionReceipt> {
    let token = EspToken::new(token_addr, &provider);
    Ok(token
        .approve(stake_table_address, amount)
        .send()
        .await
        .maybe_decode_revert::<EspTokenErrors>()?
        .get_receipt()
        .await?)
}

pub async fn delegate(
    provider: impl Provider,
    stake_table: Address,
    validator_address: Address,
    amount: U256,
) -> Result<TransactionReceipt> {
    let st = StakeTable::new(stake_table, provider);
    Ok(st
        .delegate(validator_address, amount)
        .send()
        .await
        .maybe_decode_revert::<StakeTableErrors>()?
        .get_receipt()
        .await?)
}

pub async fn undelegate(
    provider: impl Provider,
    stake_table: Address,
    validator_address: Address,
    amount: U256,
) -> Result<TransactionReceipt> {
    let st = StakeTable::new(stake_table, provider);
    Ok(st
        .undelegate(validator_address, amount)
        .send()
        .await
        .maybe_decode_revert::<StakeTableErrors>()?
        .get_receipt()
        .await?)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{deploy::TestSystem, l1::decode_log};

    #[tokio::test]
    async fn test_delegate() -> Result<()> {
        let system = TestSystem::deploy().await?;
        system.register_validator().await?;
        let validator_address = system.deployer_address;

        let amount = U256::from(123);
        let receipt = delegate(
            &system.provider,
            system.stake_table,
            validator_address,
            amount,
        )
        .await?;
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
        let receipt = undelegate(
            &system.provider,
            system.stake_table,
            validator_address,
            amount,
        )
        .await?;
        assert!(receipt.status());

        let event = decode_log::<StakeTable::Undelegated>(&receipt).unwrap();
        assert_eq!(event.validator, validator_address);
        assert_eq!(event.amount, amount);

        Ok(())
    }
}
