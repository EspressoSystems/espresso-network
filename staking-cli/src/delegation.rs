#[cfg(test)]
mod test {
    use alloy::{
        primitives::{utils::parse_ether, U256},
        providers::Provider,
    };
    use anyhow::Result;
    use hotshot_contract_adapter::{
        sol_types::StakeTableV2, stake_table::StakeTableContractVersion,
    };
    use rstest::rstest;

    use crate::{deploy::TestSystem, receipt::ReceiptExt as _, transaction::Transaction};

    #[rstest]
    #[case(StakeTableContractVersion::V1)]
    #[case(StakeTableContractVersion::V2)]
    #[tokio::test]
    async fn test_delegate(#[case] version: StakeTableContractVersion) -> Result<()> {
        let system = TestSystem::deploy_version(version).await?;
        system.register_validator().await?;
        let validator_address = system.deployer_address;

        let amount = parse_ether("1.23")?;
        let tx = Transaction::Delegate {
            stake_table: system.stake_table,
            validator: validator_address,
            amount,
        };
        let receipt = tx.send(&system.provider).await?.assert_success().await?;

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
        let tx = Transaction::Undelegate {
            stake_table: system.stake_table,
            validator: validator_address,
            amount,
        };
        let receipt = tx.send(&system.provider).await?.assert_success().await?;

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
        let tx = Transaction::Delegate {
            stake_table: system.stake_table,
            validator: validator_address,
            amount,
        };
        let err = tx
            .validate_delegate_amount(&system.provider)
            .await
            .expect_err("should fail with amount below minimum");

        let err_msg = err.to_string();
        assert!(
            err_msg.contains("below minimum"),
            "error should mention below minimum: {err_msg}"
        );
        assert!(
            err_msg.contains("1 ESP"),
            "error should include min amount: {err_msg}"
        );

        Ok(())
    }
}
