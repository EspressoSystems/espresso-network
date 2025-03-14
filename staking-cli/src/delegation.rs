use alloy::{
    primitives::{Address, Log, U256},
    providers::Provider,
    rpc::types::TransactionReceipt,
    sol_types::{SolEvent, SolInterface},
    transports::Transport,
};
use anyhow::Result;
use contract_bindings_alloy::staketable::StakeTable::{StakeTableErrors, StakeTableInstance};

pub async fn delegate<P: Provider<T>, T: Transport + Clone>(
    stake_table: StakeTableInstance<T, P>,
    validator_address: Address,
    amount: U256,
) -> Result<TransactionReceipt> {
    Ok(stake_table
        .delegate(validator_address, amount)
        .send()
        .await
        .map_err(|err| {
            // let dec = err.as_decoded_error::<StakeTableErrors>().unwrap();
            // TODO: needs alloy 0.12
            err
        })?
        .get_receipt()
        .await?)
}

#[cfg(test)]
mod test {
    use alloy::providers::WalletProvider as _;
    use contract_bindings_alloy::staketable::StakeTable::{self};

    use super::*;
    use crate::{deploy::TestSystem, l1::decode_log, registration::register_validator};

    #[tokio::test]
    async fn test_delegate() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let validator_address = system.provider.default_signer_address();

        let receipt = register_validator(
            system.stake_table.clone(),
            system.commission,
            validator_address,
            system.bls_key_pair,
            system.schnorr_key_pair.ver_key(),
        )
        .await?;
        assert!(receipt.status());

        let amount = U256::from(123);
        let receipt = delegate(system.stake_table, validator_address, amount).await?;
        assert!(receipt.status());

        let event = decode_log::<StakeTable::Delegated>(&receipt).unwrap();
        assert_eq!(event.validator, validator_address);
        assert_eq!(event.amount, amount);

        Ok(())
    }
}
