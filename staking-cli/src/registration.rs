use alloy::{
    primitives::{Address, Bytes},
    providers::Provider,
    rpc::types::TransactionReceipt,
};
use anyhow::Result;
use hotshot_contract_adapter::{
    evm::DecodeRevert as _,
    sol_types::{
        EdOnBN254PointSol, G1PointSol, G2PointSol,
        StakeTableV2::{self, StakeTableV2Errors},
    },
    stake_table::{sign_address_bls, sign_address_schnorr},
};
use hotshot_types::{light_client::StateKeyPair, signature_key::BLSKeyPair};

use crate::parse::Commission;

/// The ver_key and signature as types that contract bindings expect
fn prepare_bls_payload(
    bls_key_pair: &BLSKeyPair,
    validator_address: Address,
) -> (G2PointSol, G1PointSol) {
    (
        bls_key_pair.ver_key().to_affine().into(),
        sign_address_bls(bls_key_pair, validator_address),
    )
}

// The ver_key and signature as types that contract bindings expect
fn prepare_schnorr_payload(
    schnorr_key_pair: &StateKeyPair,
    validator_address: Address,
) -> (EdOnBN254PointSol, Bytes) {
    let schnorr_vk_sol: EdOnBN254PointSol = schnorr_key_pair.ver_key().to_affine().into();
    let sig = sign_address_schnorr(schnorr_key_pair, validator_address);
    (schnorr_vk_sol, sig)
}

pub async fn register_validator(
    provider: impl Provider,
    stake_table_addr: Address,
    commission: Commission,
    validator_address: Address,
    bls_key_pair: BLSKeyPair,
    schnorr_key_pair: StateKeyPair,
) -> Result<TransactionReceipt> {
    let stake_table = StakeTableV2::new(stake_table_addr, &provider);
    let (bls_vk, bls_sig) = prepare_bls_payload(&bls_key_pair, validator_address);
    let (schnorr_vk, schnorr_sig) = prepare_schnorr_payload(&schnorr_key_pair, validator_address);

    let version = stake_table.getVersion().call().await?.majorVersion;
    Ok(match version {
        1 => {
            stake_table
                .registerValidator(bls_vk, schnorr_vk, bls_sig.into(), commission.to_evm())
                .send()
                .await
                .maybe_decode_revert::<StakeTableV2Errors>()?
                .get_receipt()
                .await?
        },
        2 => {
            stake_table
                .registerValidatorV2(
                    bls_vk,
                    schnorr_vk,
                    bls_sig.into(),
                    schnorr_sig,
                    commission.to_evm(),
                )
                .send()
                .await
                .maybe_decode_revert::<StakeTableV2Errors>()?
                .get_receipt()
                .await?
        },
        _ => {
            unimplemented!("Unsupported stake table version: {}", version);
        },
    })
}

pub async fn update_consensus_keys(
    provider: impl Provider,
    stake_table_addr: Address,
    validator_address: Address,
    bls_key_pair: BLSKeyPair,
    schnorr_key_pair: StateKeyPair,
) -> Result<TransactionReceipt> {
    let stake_table = StakeTableV2::new(stake_table_addr, &provider);
    let (bls_vk, bls_sig) = prepare_bls_payload(&bls_key_pair, validator_address);
    let (schnorr_vk, schnorr_sig) = prepare_schnorr_payload(&schnorr_key_pair, validator_address);

    let version = stake_table.getVersion().call().await?.majorVersion;
    Ok(match version {
        1 => {
            stake_table
                .updateConsensusKeys(bls_vk, schnorr_vk, bls_sig.into())
                .send()
                .await
                .maybe_decode_revert::<StakeTableV2Errors>()?
                .get_receipt()
                .await?
        },
        2 => {
            stake_table
                .updateConsensusKeysV2(bls_vk, schnorr_vk, bls_sig.into(), schnorr_sig)
                .send()
                .await
                .maybe_decode_revert::<StakeTableV2Errors>()?
                .get_receipt()
                .await?
        },
        _ => {
            unimplemented!("Unsupported stake table version: {}", version);
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

#[cfg(test)]
mod test {
    use rand::{rngs::StdRng, SeedableRng as _};

    use super::*;
    use crate::deploy::TestSystem;

    #[tokio::test]
    async fn test_register_validator() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let validator_address = system.deployer_address;
        let (bls_vk_sol, _) = prepare_bls_payload(&system.bls_key_pair, validator_address);
        let schnorr_vk_sol: EdOnBN254PointSol = system.state_key_pair.ver_key().to_affine().into();

        let receipt = register_validator(
            &system.provider,
            system.stake_table,
            system.commission,
            validator_address,
            system.bls_key_pair,
            system.state_key_pair,
        )
        .await?;
        assert!(receipt.status());

        let event = receipt
            .decoded_log::<StakeTableV2::ValidatorRegisteredV2>()
            .unwrap();
        assert_eq!(event.account, validator_address);
        assert_eq!(event.commission, system.commission.to_evm());

        assert_eq!(event.blsVK, bls_vk_sol);
        assert_eq!(event.schnorrVK, schnorr_vk_sol);

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
        let (bls_vk_sol, _) = prepare_bls_payload(&new_bls, validator_address);
        let (schnorr_vk_sol, _) = prepare_schnorr_payload(&new_schnorr, validator_address);

        let receipt = update_consensus_keys(
            &system.provider,
            system.stake_table,
            validator_address,
            new_bls,
            new_schnorr,
        )
        .await?;
        assert!(receipt.status());

        let event = receipt
            .decoded_log::<StakeTableV2::ConsensusKeysUpdatedV2>()
            .unwrap();
        assert_eq!(event.account, system.deployer_address);

        assert_eq!(event.blsVK, bls_vk_sol);
        assert_eq!(event.schnorrVK, schnorr_vk_sol);

        event.data.authenticate()?;

        Ok(())
    }
}
