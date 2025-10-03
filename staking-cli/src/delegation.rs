use alloy::{
    eips::BlockId,
    network::Ethereum,
    primitives::{Address, U256},
    providers::{PendingTransactionBuilder, Provider},
};
use anyhow::Result;
use hotshot_contract_adapter::{
    evm::DecodeRevert as _,
    sol_types::{
        EspToken::{self, EspTokenErrors},
        StakeTable::{self, StakeTableErrors},
    },
};

pub async fn approve(
    provider: impl Provider,
    token_addr: Address,
    stake_table_address: Address,
    amount: U256,
) -> Result<PendingTransactionBuilder<Ethereum>> {
    let token = EspToken::new(token_addr, &provider);
    token
        .approve(stake_table_address, amount)
        .block(BlockId::pending())
        .send()
        .await
        .maybe_decode_revert::<EspTokenErrors>()
}

pub async fn delegate(
    provider: impl Provider,
    stake_table: Address,
    validator_address: Address,
    amount: U256,
) -> Result<PendingTransactionBuilder<Ethereum>> {
    let st = StakeTable::new(stake_table, provider);
    st.delegate(validator_address, amount)
        .block(BlockId::pending())
        .send()
        .await
        .maybe_decode_revert::<StakeTableErrors>()
}

pub async fn undelegate(
    provider: impl Provider,
    stake_table: Address,
    validator_address: Address,
    amount: U256,
) -> Result<PendingTransactionBuilder<Ethereum>> {
    let st = StakeTable::new(stake_table, provider);
    st.undelegate(validator_address, amount)
        .block(BlockId::pending())
        .send()
        .await
        .maybe_decode_revert::<StakeTableErrors>()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::deploy::TestSystem;

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
        .await?
        .get_receipt()
        .await?;
        assert!(receipt.status());

        let event = receipt.decoded_log::<StakeTable::Delegated>().unwrap();
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
        .await?
        .get_receipt()
        .await?;
        assert!(receipt.status());

        let event = receipt.decoded_log::<StakeTable::Undelegated>().unwrap();
        assert_eq!(event.validator, validator_address);
        assert_eq!(event.amount, amount);

        Ok(())
    }
}
