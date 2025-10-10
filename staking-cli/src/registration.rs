use alloy::{primitives::Address, providers::Provider, rpc::types::TransactionReceipt};
use anyhow::Result;
use hotshot_contract_adapter::{
    evm::DecodeRevert as _,
    sol_types::StakeTableV2::{self, StakeTableV2Errors},
    stake_table::StakeTableContractVersion,
};

use crate::{
    parse::Commission,
    signature::{NodeSignatures, NodeSignaturesSol},
};

pub async fn register_validator(
    provider: impl Provider,
    stake_table_addr: Address,
    commission: Commission,
    payload: NodeSignatures,
) -> Result<TransactionReceipt> {
    // NOTE: the StakeTableV2 ABI is a superset of the V1 ABI because the V2 inherits from V1 so we
    // can always use the V2 bindings for calling functions and decoding events, even if we are
    // connected to the V1 contract.
    let stake_table = StakeTableV2::new(stake_table_addr, &provider);
    let sol_payload = NodeSignaturesSol::from(payload);

    let version = stake_table.getVersion().call().await?.try_into()?;
    // There is a race-condition here if the contract is upgraded while this transactions is waiting
    // to be mined. We're very unlikely to hit this in practice, and since we only perform the
    // upgrade on decaf this is acceptable.
    Ok(match version {
        StakeTableContractVersion::V1 => {
            stake_table
                .registerValidator(
                    sol_payload.bls_vk,
                    sol_payload.schnorr_vk,
                    sol_payload.bls_signature.into(),
                    commission.to_evm(),
                )
                .send()
                .await
                .maybe_decode_revert::<StakeTableV2Errors>()?
                .get_receipt()
                .await?
        },
        StakeTableContractVersion::V2 => {
            stake_table
                .registerValidatorV2(
                    sol_payload.bls_vk,
                    sol_payload.schnorr_vk,
                    sol_payload.bls_signature.into(),
                    sol_payload.schnorr_signature.into(),
                    commission.to_evm(),
                )
                .send()
                .await
                .maybe_decode_revert::<StakeTableV2Errors>()?
                .get_receipt()
                .await?
        },
    })
}

pub async fn update_consensus_keys(
    provider: impl Provider,
    stake_table_addr: Address,
    payload: NodeSignatures,
) -> Result<TransactionReceipt> {
    // NOTE: the StakeTableV2 ABI is a superset of the V1 ABI because the V2 inherits from V1 so we
    // can always use the V2 bindings for calling functions and decoding events, even if we are
    // connected to the V1 contract.
    let stake_table = StakeTableV2::new(stake_table_addr, &provider);
    let sol_payload = NodeSignaturesSol::from(payload);

    // There is a race-condition here if the contract is upgraded while this transactions is waiting
    // to be mined. We're very unlikely to hit this in practice, and since we only perform the
    // upgrade on decaf this is acceptable.
    let version = stake_table.getVersion().call().await?.try_into()?;
    Ok(match version {
        StakeTableContractVersion::V1 => {
            stake_table
                .updateConsensusKeys(
                    sol_payload.bls_vk,
                    sol_payload.schnorr_vk,
                    sol_payload.bls_signature.into(),
                )
                .send()
                .await
                .maybe_decode_revert::<StakeTableV2Errors>()?
                .get_receipt()
                .await?
        },
        StakeTableContractVersion::V2 => {
            stake_table
                .updateConsensusKeysV2(
                    sol_payload.bls_vk,
                    sol_payload.schnorr_vk,
                    sol_payload.bls_signature.into(),
                    sol_payload.schnorr_signature.into(),
                )
                .send()
                .await
                .maybe_decode_revert::<StakeTableV2Errors>()?
                .get_receipt()
                .await?
        },
    })
}

pub async fn deregister_validator(
    provider: impl Provider,
    stake_table_addr: Address,
) -> Result<TransactionReceipt> {
    let stake_table = StakeTableV2::new(stake_table_addr, &provider);
    Ok(stake_table
        .deregisterValidator()
        .send()
        .await
        .maybe_decode_revert::<StakeTableV2Errors>()?
        .get_receipt()
        .await?)
}

pub async fn update_commission(
    provider: impl Provider,
    stake_table_addr: Address,
    new_commission: Commission,
) -> Result<TransactionReceipt> {
    let stake_table = StakeTableV2::new(stake_table_addr, &provider);
    Ok(stake_table
        .updateCommission(new_commission.to_evm())
        .send()
        .await
        .maybe_decode_revert::<StakeTableV2Errors>()?
        .get_receipt()
        .await?)
}

pub async fn fetch_commission(
    provider: impl Provider,
    stake_table_addr: Address,
    validator: Address,
) -> Result<Commission> {
    let stake_table = StakeTableV2::new(stake_table_addr, &provider);
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
    use espresso_contract_deployer::build_provider;
    use espresso_types::{
        v0_3::{Fetcher, StakeTableEvent},
        L1Client,
    };
    use hotshot_contract_adapter::{
        sol_types::{EdOnBN254PointSol, G1PointSol, G2PointSol},
        stake_table::{sign_address_bls, sign_address_schnorr, StateSignatureSol},
    };
    use rand::{rngs::StdRng, SeedableRng as _};

    use super::*;
    use crate::deploy::TestSystem;

    #[tokio::test]
    async fn test_register_validator() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let validator_address = system.deployer_address;
        let payload = NodeSignatures::create(
            validator_address,
            &system.bls_key_pair,
            &system.state_key_pair,
        );

        let receipt = register_validator(
            &system.provider,
            system.stake_table,
            system.commission,
            payload,
        )
        .await?;
        assert!(receipt.status());

        let event = receipt
            .decoded_log::<StakeTableV2::ValidatorRegisteredV2>()
            .unwrap();
        assert_eq!(event.account, validator_address);
        assert_eq!(event.commission, system.commission.to_evm());

        assert_eq!(event.blsVK, system.bls_key_pair.ver_key().into());
        assert_eq!(event.schnorrVK, system.state_key_pair.ver_key().into());

        event.data.authenticate()?;
        Ok(())
    }

    #[tokio::test]
    async fn test_deregister_validator() -> Result<()> {
        let system = TestSystem::deploy().await?;
        system.register_validator().await?;

        let receipt = deregister_validator(&system.provider, system.stake_table).await?;
        assert!(receipt.status());

        let event = receipt
            .decoded_log::<StakeTableV2::ValidatorExit>()
            .unwrap();
        assert_eq!(event.validator, system.deployer_address);

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

        let receipt = update_consensus_keys(&system.provider, system.stake_table, payload).await?;
        assert!(receipt.status());

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
        let receipt = stake_table
            .setMinCommissionUpdateInterval(U256::from(1)) // 1 second
            .send()
            .await?
            .get_receipt()
            .await?;
        assert!(receipt.status());

        system.register_validator().await?;
        let validator_address = system.deployer_address;
        let new_commission = Commission::try_from("10.50")?;

        // Wait 2 seconds to ensure we're past the interval
        system.anvil_increase_time(U256::from(2)).await?;

        let receipt =
            update_commission(&system.provider, system.stake_table, new_commission).await?;
        assert!(receipt.status());

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

    /// The GCL must remove stake table events with incorrect signatures. This test verifies that a
    /// validator registered event with incorrect schnorr signature is removed before the stake
    /// table is computed.
    #[tokio::test]
    async fn test_integration_unauthenticated_validator_registered_events_removed() -> Result<()> {
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
            )
            .send()
            .await
            .maybe_decode_revert::<StakeTableV2Errors>()?
            .get_receipt()
            .await?;
        assert!(receipt.status());

        let l1 = L1Client::new(vec![system.rpc_url])?;
        let events = Fetcher::fetch_events_from_contract(
            l1,
            system.stake_table,
            Some(0),
            receipt.block_number.unwrap(),
        )
        .await?;

        // verify that we only have the first RegisterV2 event
        assert_eq!(events.len(), 1);
        match events[0].1.clone() {
            StakeTableEvent::RegisterV2(event) => {
                assert_eq!(event.account, system.deployer_address);
            },
            _ => panic!("expected RegisterV2 event"),
        }
        Ok(())
    }

    /// The GCL must remove stake table events with incorrect signatures. This test verifies that a
    /// consensus keys update event with incorrect schnorr signature is removed before the stake
    /// table is computed.
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
            .get_receipt()
            .await?;
        assert!(receipt.status());

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
        match events[0].1.clone() {
            StakeTableEvent::RegisterV2(event) => {
                assert_eq!(event.account, system.deployer_address);
            },
            _ => panic!("expected RegisterV2 event"),
        }

        println!("Events: {events:?}");

        Ok(())
    }
}
