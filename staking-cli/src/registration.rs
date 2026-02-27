use alloy::{
    network::Ethereum,
    primitives::Address,
    providers::{PendingTransactionBuilder, Provider},
};
use anyhow::Result;
use hotshot_contract_adapter::{
    evm::DecodeRevert as _,
    sol_types::StakeTableV2::{self, StakeTableV2Errors},
    stake_table::StakeTableContractVersion,
};

use crate::parse::Commission;

/// Update validator commission rate.
///
/// Used by sequencer tests.
pub async fn update_commission(
    provider: impl Provider,
    stake_table_addr: Address,
    new_commission: Commission,
) -> Result<PendingTransactionBuilder<Ethereum>> {
    let stake_table = StakeTableV2::new(stake_table_addr, provider);
    stake_table
        .updateCommission(new_commission.to_evm())
        .send()
        .await
        .maybe_decode_revert::<StakeTableV2Errors>()
}

/// Fetch validator commission rate.
///
/// Used by sequencer tests.
pub async fn fetch_commission(
    provider: impl Provider,
    stake_table_addr: Address,
    validator: Address,
) -> Result<Commission> {
    let stake_table = StakeTableV2::new(stake_table_addr, provider);
    let version: StakeTableContractVersion = stake_table.getVersion().call().await?.try_into()?;
    if matches!(version, StakeTableContractVersion::V1) {
        anyhow::bail!("fetching commission is not supported with stake table V1");
    }
    Ok(stake_table
        .commissionTracking(validator)
        .call()
        .await?
        .commission
        .try_into()?)
}

#[cfg(test)]
mod test {
    use alloy::{primitives::U256, providers::WalletProvider as _};
    use anyhow::Result;
    use espresso_contract_deployer::build_provider;
    use espresso_types::{
        v0_3::{Fetcher, StakeTableEvent},
        L1Client,
    };
    use hotshot_contract_adapter::{
        evm::DecodeRevert as _,
        sol_types::{EdOnBN254PointSol, G1PointSol, G2PointSol, StakeTableV2::StakeTableV2Errors},
        stake_table::{sign_address_bls, sign_address_schnorr, StateSignatureSol},
    };
    use rand::{rngs::StdRng, SeedableRng as _};
    use rstest::rstest;

    use super::*;
    use crate::{
        deploy::TestSystem, metadata::MetadataUri, receipt::ReceiptExt as _,
        signature::NodeSignatures, transaction::Transaction,
    };

    #[tokio::test]
    async fn test_register_validator() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let validator_address = system.deployer_address;
        let payload = NodeSignatures::create(
            validator_address,
            &system.bls_key_pair,
            &system.state_key_pair,
        );

        let metadata_uri = "https://example.com/metadata".parse()?;
        let receipt = Transaction::RegisterValidator {
            stake_table: system.stake_table,
            commission: system.commission,
            metadata_uri,
            payload,
            version: StakeTableContractVersion::V2,
        }
        .send(&system.provider)
        .await?
        .assert_success()
        .await?;

        let event = receipt
            .decoded_log::<StakeTableV2::ValidatorRegisteredV2>()
            .unwrap();
        assert_eq!(event.account, validator_address);
        assert_eq!(event.commission, system.commission.to_evm());
        assert_eq!(event.metadataUri, "https://example.com/metadata");

        assert_eq!(event.blsVK, system.bls_key_pair.ver_key().into());
        assert_eq!(event.schnorrVK, system.state_key_pair.ver_key().into());

        event.data.authenticate()?;
        Ok(())
    }

    #[rstest]
    #[case(StakeTableContractVersion::V1)]
    #[case(StakeTableContractVersion::V2)]
    #[tokio::test]
    async fn test_deregister_validator(#[case] version: StakeTableContractVersion) -> Result<()> {
        let system = TestSystem::deploy_version(version).await?;
        system.register_validator().await?;

        let receipt = Transaction::DeregisterValidator {
            stake_table: system.stake_table,
        }
        .send(&system.provider)
        .await?
        .assert_success()
        .await?;

        match version {
            StakeTableContractVersion::V1 => {
                let event = receipt
                    .decoded_log::<StakeTableV2::ValidatorExit>()
                    .unwrap();
                assert_eq!(event.validator, system.deployer_address);
            },
            StakeTableContractVersion::V2 => {
                let event = receipt
                    .decoded_log::<StakeTableV2::ValidatorExitV2>()
                    .unwrap();
                assert_eq!(event.validator, system.deployer_address);
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
    async fn test_update_consensus_keys() -> Result<()> {
        let system = TestSystem::deploy().await?;
        system.register_validator().await?;
        let validator_address = system.deployer_address;
        let mut rng = StdRng::from_seed([43u8; 32]);
        let (_, new_bls, new_schnorr) = TestSystem::gen_keys(&mut rng);
        let payload = NodeSignatures::create(validator_address, &new_bls, &new_schnorr);

        let receipt = Transaction::UpdateConsensusKeys {
            stake_table: system.stake_table,
            payload,
            version: StakeTableContractVersion::V2,
        }
        .send(&system.provider)
        .await?
        .assert_success()
        .await?;

        let event = receipt
            .decoded_log::<StakeTableV2::ConsensusKeysUpdatedV2>()
            .unwrap();
        assert_eq!(event.account, system.deployer_address);

        assert_eq!(event.blsVK, new_bls.ver_key().into());
        assert_eq!(event.schnorrVK, new_schnorr.ver_key().into());

        event.data.authenticate()?;

        Ok(())
    }

    #[tokio::test]
    async fn test_update_commission() -> Result<()> {
        let system = TestSystem::deploy().await?;

        // Set commission update interval to 1 second for testing
        let stake_table = StakeTableV2::new(system.stake_table, &system.provider);
        stake_table
            .setMinCommissionUpdateInterval(U256::from(1)) // 1 second
            .send()
            .await?
            .assert_success()
            .await?;

        system.register_validator().await?;
        let validator_address = system.deployer_address;
        let new_commission = Commission::try_from("10.50")?;

        // Wait 2 seconds to ensure we're past the interval
        system.anvil_increase_time(U256::from(2)).await?;

        let receipt = Transaction::UpdateCommission {
            stake_table: system.stake_table,
            new_commission,
        }
        .send(&system.provider)
        .await?
        .assert_success()
        .await?;

        let event = receipt
            .decoded_log::<StakeTableV2::CommissionUpdated>()
            .unwrap();
        assert_eq!(event.validator, validator_address);
        assert_eq!(event.newCommission, new_commission.to_evm());

        let fetched_commission =
            fetch_commission(&system.provider, system.stake_table, validator_address).await?;
        assert_eq!(fetched_commission, new_commission);

        Ok(())
    }

    /// Unauthenticated validators (with incorrect schnorr signature) are kept because the contract
    /// allows staking transactions targeting these validators.
    #[tokio::test]
    async fn test_integration_unauthenticated_validator_registered_events_kept() -> Result<()> {
        let system = TestSystem::deploy().await?;

        // register a validator with correct signature
        system.register_validator().await?;

        // NOTE: we can't register a validator with a bad BLS signature because the contract will revert

        let provider = build_provider(
            "test test test test test test test test test test test junk",
            1,
            system.rpc_url.clone(),
            /* polling_interval */ None,
        );
        let validator_address = provider.default_signer_address();
        let (_, bls_key_pair, schnorr_key_pair) =
            TestSystem::gen_keys(&mut StdRng::from_seed([1u8; 32]));
        let (_, _, other_schnorr_key_pair) =
            TestSystem::gen_keys(&mut StdRng::from_seed([2u8; 32]));

        let bls_vk = G2PointSol::from(bls_key_pair.ver_key());
        let bls_sig = G1PointSol::from(sign_address_bls(&bls_key_pair, validator_address));
        let schnorr_vk = EdOnBN254PointSol::from(schnorr_key_pair.ver_key());

        // create a valid schnorr signature with the *wrong* key
        let schnorr_sig_other_key = StateSignatureSol::from(sign_address_schnorr(
            &other_schnorr_key_pair,
            validator_address,
        ));

        let stake_table = StakeTableV2::new(system.stake_table, provider);

        // register a validator with the schnorr sig from another key
        let receipt = stake_table
            .registerValidatorV2(
                bls_vk,
                schnorr_vk,
                bls_sig.into(),
                schnorr_sig_other_key.into(),
                Commission::try_from("12.34")?.to_evm(),
                "https://example.com/metadata".to_string(),
            )
            .send()
            .await
            .maybe_decode_revert::<StakeTableV2Errors>()?
            .assert_success()
            .await?;

        let l1 = L1Client::new(vec![system.rpc_url])?;
        let events = Fetcher::fetch_events_from_contract(
            l1,
            system.stake_table,
            Some(0),
            receipt.block_number.unwrap(),
        )
        .await?;

        // verify that both RegisterV2 events are kept
        assert_eq!(events.len(), 2);
        for event in &events {
            assert!(matches!(event.1, StakeTableEvent::RegisterV2(_)));
        }
        Ok(())
    }

    /// Unauthenticated consensus key updates (with incorrect schnorr signature) are removed.
    #[tokio::test]
    async fn test_integration_unauthenticated_update_consensus_keys_events_removed() -> Result<()> {
        let system = TestSystem::deploy().await?;

        // register a validator with correct signature
        system.register_validator().await?;
        let validator_address = system.deployer_address;

        // NOTE: we can't register a validator with a bad BLS signature because the contract will revert

        let (_, new_bls_key_pair, new_schnorr_key_pair) =
            TestSystem::gen_keys(&mut StdRng::from_seed([1u8; 32]));
        let (_, _, other_schnorr_key_pair) =
            TestSystem::gen_keys(&mut StdRng::from_seed([2u8; 32]));

        let bls_vk = G2PointSol::from(new_bls_key_pair.ver_key());
        let bls_sig = G1PointSol::from(sign_address_bls(&new_bls_key_pair, validator_address));
        let schnorr_vk = EdOnBN254PointSol::from(new_schnorr_key_pair.ver_key());

        // create a valid schnorr signature with the *wrong* key
        let schnorr_sig_other_key = StateSignatureSol::from(sign_address_schnorr(
            &other_schnorr_key_pair,
            validator_address,
        ))
        .into();

        let stake_table = StakeTableV2::new(system.stake_table, system.provider);

        // update consensus keys with the schnorr sig from another key
        let receipt = stake_table
            .updateConsensusKeysV2(bls_vk, schnorr_vk, bls_sig.into(), schnorr_sig_other_key)
            .send()
            .await
            .maybe_decode_revert::<StakeTableV2Errors>()?
            .assert_success()
            .await?;

        let l1 = L1Client::new(vec![system.rpc_url])?;
        let events = Fetcher::fetch_events_from_contract(
            l1,
            system.stake_table,
            Some(0),
            receipt.block_number.unwrap(),
        )
        .await?;

        // verify that we only have the RegisterV2 event
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].1, StakeTableEvent::RegisterV2(_)));

        Ok(())
    }

    #[tokio::test]
    async fn test_update_metadata_uri() -> Result<()> {
        let system = TestSystem::deploy().await?;
        system.register_validator().await?;

        let new_uri: MetadataUri = "https://example.com/updated".parse()?;
        let receipt = Transaction::UpdateMetadataUri {
            stake_table: system.stake_table,
            metadata_uri: new_uri.clone(),
        }
        .send(&system.provider)
        .await?
        .assert_success()
        .await?;

        let event = receipt
            .decoded_log::<StakeTableV2::MetadataUriUpdated>()
            .unwrap();
        assert_eq!(event.validator, system.deployer_address);
        assert_eq!(event.metadataUri, new_uri.to_string());

        Ok(())
    }

    #[tokio::test]
    async fn test_register_validator_with_empty_metadata_uri() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let validator_address = system.deployer_address;
        let payload = NodeSignatures::create(
            validator_address,
            &system.bls_key_pair,
            &system.state_key_pair,
        );

        let metadata_uri = MetadataUri::empty();
        let receipt = Transaction::RegisterValidator {
            stake_table: system.stake_table,
            commission: system.commission,
            metadata_uri,
            payload,
            version: StakeTableContractVersion::V2,
        }
        .send(&system.provider)
        .await?
        .assert_success()
        .await?;

        let event = receipt
            .decoded_log::<StakeTableV2::ValidatorRegisteredV2>()
            .unwrap();
        assert_eq!(event.account, validator_address);
        assert_eq!(event.commission, system.commission.to_evm());
        assert_eq!(event.metadataUri, "");

        Ok(())
    }

    #[tokio::test]
    async fn test_update_metadata_uri_to_empty() -> Result<()> {
        let system = TestSystem::deploy().await?;
        system.register_validator().await?;

        let metadata_uri = MetadataUri::empty();
        let receipt = Transaction::UpdateMetadataUri {
            stake_table: system.stake_table,
            metadata_uri,
        }
        .send(&system.provider)
        .await?
        .assert_success()
        .await?;

        let event = receipt
            .decoded_log::<StakeTableV2::MetadataUriUpdated>()
            .unwrap();
        assert_eq!(event.validator, system.deployer_address);
        assert_eq!(event.metadataUri, "");

        Ok(())
    }
}
