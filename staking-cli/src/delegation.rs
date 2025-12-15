use alloy::{
    network::Ethereum,
    primitives::{utils::format_ether, Address, U256},
    providers::{PendingTransactionBuilder, Provider},
};
use anyhow::{bail, Result};
use hotshot_contract_adapter::{
    evm::DecodeRevert as _,
    sol_types::{
        EspToken::{self, EspTokenErrors},
        StakeTableV2::{self, StakeTableV2Errors},
    },
    stake_table::StakeTableContractVersion,
};

pub async fn approve(
    provider: impl Provider,
    token_addr: Address,
    stake_table_address: Address,
    amount: U256,
) -> Result<PendingTransactionBuilder<Ethereum>> {
    tracing::info!(
        "approve {} ESP for {stake_table_address}",
        format_ether(amount)
    );
    let token = EspToken::new(token_addr, provider);
    token
        .approve(stake_table_address, amount)
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
    tracing::info!(
        "delegate {} ESP to {validator_address}",
        format_ether(amount)
    );
    let st = StakeTableV2::new(stake_table, provider);

    let version: StakeTableContractVersion = st.getVersion().call().await?.try_into()?;
    if let StakeTableContractVersion::V2 = version {
        let min_amount = st.minDelegateAmount().call().await?;
        if amount < min_amount {
            bail!(
                "delegation amount {} ESP is below minimum of {} ESP",
                format_ether(amount),
                format_ether(min_amount)
            );
        }
    }

    st.delegate(validator_address, amount)
        .send()
        .await
        .maybe_decode_revert::<StakeTableV2Errors>()
}

pub async fn undelegate(
    provider: impl Provider,
    stake_table: Address,
    validator_address: Address,
    amount: U256,
) -> Result<PendingTransactionBuilder<Ethereum>> {
    tracing::info!(
        "undelegate {} ESP from {validator_address}",
        format_ether(amount)
    );
    let st = StakeTableV2::new(stake_table, provider);
    st.undelegate(validator_address, amount)
        .send()
        .await
        .maybe_decode_revert::<StakeTableV2Errors>()
}

#[cfg(test)]
mod test {
    use alloy::primitives::utils::parse_ether;
    use rstest::rstest;

    use super::*;
    use crate::{deploy::TestSystem, receipt::ReceiptExt};

    #[rstest]
    #[case(StakeTableContractVersion::V1)]
    #[case(StakeTableContractVersion::V2)]
    #[tokio::test]
    async fn test_delegate(#[case] version: StakeTableContractVersion) -> Result<()> {
        let system = TestSystem::deploy_version(version).await?;
        system.register_validator().await?;
        let validator_address = system.deployer_address;

        let amount = parse_ether("1.23")?;
        let receipt = delegate(
            &system.provider,
            system.stake_table,
            validator_address,
            amount,
        )
        .await?
        .assert_success()
        .await?;

        let event = receipt.decoded_log::<StakeTableV2::Delegated>().unwrap();
        assert_eq!(event.validator, validator_address);
        assert_eq!(event.amount, amount);

        Ok(())
    }

    #[rstest]
    #[case(StakeTableContractVersion::V1)]
    #[case(StakeTableContractVersion::V2)]
    #[tokio::test]
    async fn test_undelegate(#[case] version: StakeTableContractVersion) -> Result<()> {
        let system = TestSystem::deploy_version(version).await?;
        let amount = parse_ether("1.23")?;
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
        .assert_success()
        .await?;

        match version {
            StakeTableContractVersion::V1 => {
                let event = receipt.decoded_log::<StakeTableV2::Undelegated>().unwrap();
                assert_eq!(event.validator, validator_address);
                assert_eq!(event.amount, amount);
            },
            StakeTableContractVersion::V2 => {
                let event = receipt
                    .decoded_log::<StakeTableV2::UndelegatedV2>()
                    .unwrap();
                assert_eq!(event.validator, validator_address);
                assert_eq!(event.amount, amount);
                let block = system
                    .provider
                    .get_block_by_number(receipt.block_number.unwrap().into())
                    .await?
                    .unwrap();
                let expected_unlock = block.header.timestamp + system.exit_escrow_period.as_secs();
                assert_eq!(event.unlocksAt, U256::from(expected_unlock));
            },
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_delegate_below_minimum_amount() -> Result<()> {
        let system = TestSystem::deploy().await?;
        system.register_validator().await?;
        let validator_address = system.deployer_address;

        let amount = U256::from(123);
        let err = delegate(
            &system.provider,
            system.stake_table,
            validator_address,
            amount,
        )
        .await
        .expect_err("should fail with amount below minimum");

        let err_msg = err.to_string();
        assert!(
            err_msg.contains("below minimum"),
            "error should mention below minimum: {err_msg}"
        );
        assert!(
            err_msg.contains("1.000000000000000000 ESP"),
            "error should include min amount: {err_msg}"
        );

        Ok(())
    }
}
