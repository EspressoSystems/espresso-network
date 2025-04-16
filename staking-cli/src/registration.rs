use alloy::{
    primitives::Address, providers::Provider, rpc::types::TransactionReceipt,
    sol_types::SolValue as _,
};
use anyhow::Result;
use ark_ec::CurveGroup;
use hotshot_contract_adapter::sol_types::{EdOnBN254PointSol, G1PointSol, G2PointSol, StakeTable};
use jf_signature::constants::CS_ID_BLS_BN254;

use crate::{parse::Commission, BLSKeyPair, StateVerKey};

fn prepare_bls_payload(
    bls_key_pair: &BLSKeyPair,
    validator_address: Address,
) -> (G2PointSol, G1PointSol) {
    let bls_vk_sol: G2PointSol = bls_key_pair.ver_key().to_affine().into();
    let sig_sol: G1PointSol = bls_key_pair
        .sign(&validator_address.abi_encode(), CS_ID_BLS_BN254)
        .sigma
        .into_affine()
        .into();
    (bls_vk_sol, sig_sol)
}

pub async fn register_validator(
    provider: impl Provider,
    stake_table_addr: Address,
    commission: Commission,
    validator_address: Address,
    bls_key_pair: BLSKeyPair,
    schnorr_vk: StateVerKey,
) -> Result<TransactionReceipt> {
    let stake_table = StakeTable::new(stake_table_addr, &provider);
    let (bls_vk_sol, sig_sol) = prepare_bls_payload(&bls_key_pair, validator_address);
    let schnorr_vk_sol: EdOnBN254PointSol = schnorr_vk.to_affine().into();
    Ok(stake_table
        .registerValidator(
            bls_vk_sol.into(),
            schnorr_vk_sol.into(),
            sig_sol.into(),
            commission.to_evm(),
        )
        .send()
        .await?
        .get_receipt()
        .await?)
}

pub async fn update_consensus_keys(
    provider: impl Provider,
    stake_table_addr: Address,
    validator_address: Address,
    bls_key_pair: BLSKeyPair,
    schnorr_vk: StateVerKey,
) -> Result<TransactionReceipt> {
    let stake_table = StakeTable::new(stake_table_addr, &provider);
    let (bls_vk_sol, sig_sol) = prepare_bls_payload(&bls_key_pair, validator_address);
    let schnorr_vk_sol: EdOnBN254PointSol = schnorr_vk.to_affine().into();
    Ok(stake_table
        .updateConsensusKeys(bls_vk_sol.into(), schnorr_vk_sol.into(), sig_sol.into())
        .send()
        .await?
        .get_receipt()
        .await?)
}

pub async fn deregister_validator(
    provider: impl Provider,
    stake_table_addr: Address,
) -> Result<TransactionReceipt> {
    let stake_table = StakeTable::new(stake_table_addr, &provider);
    Ok(stake_table
        .deregisterValidator()
        .send()
        .await?
        .get_receipt()
        .await?)
}

#[cfg(test)]
mod test {
    use rand::{rngs::StdRng, SeedableRng as _};

    use super::*;
    use crate::{deploy::TestSystem, l1::decode_log};

    #[tokio::test]
    async fn test_register_validator() -> Result<()> {
        let system = TestSystem::deploy().await?;
        let validator_address = system.deployer_address;
        let (bls_vk_sol, _) = prepare_bls_payload(&system.bls_key_pair, validator_address);
        let schnorr_vk_sol: EdOnBN254PointSol =
            system.schnorr_key_pair.ver_key().to_affine().into();

        let receipt = register_validator(
            &system.provider,
            system.stake_table,
            system.commission,
            validator_address,
            system.bls_key_pair,
            system.schnorr_key_pair.ver_key(),
        )
        .await?;
        assert!(receipt.status());

        let event = decode_log::<StakeTable::ValidatorRegistered>(&receipt).unwrap();
        assert_eq!(event.account, validator_address);
        assert_eq!(event.commission, system.commission.to_evm());

        assert_eq!(event.blsVk, bls_vk_sol.into());
        assert_eq!(event.schnorrVk, schnorr_vk_sol.into());

        // TODO verify we can parse keys and verify signature
        Ok(())
    }

    #[tokio::test]
    async fn test_deregister_validator() -> Result<()> {
        let system = TestSystem::deploy().await?;
        system.register_validator().await?;

        let receipt = deregister_validator(&system.provider, system.stake_table).await?;
        assert!(receipt.status());

        let event = decode_log::<StakeTable::ValidatorExit>(&receipt).unwrap();
        assert_eq!(event.validator, system.deployer_address);

        Ok(())
    }

    #[tokio::test]
    async fn test_update_consensus_keys() -> Result<()> {
        let system = TestSystem::deploy().await?;
        system.register_validator().await?;
        let validator_address = system.deployer_address;
        let mut rng = StdRng::from_seed([43u8; 32]);
        let (new_bls, new_schnorr) = TestSystem::gen_consensus_keys(&mut rng);
        let (bls_vk_sol, _) = prepare_bls_payload(&new_bls, validator_address);
        let schnorr_vk_sol: EdOnBN254PointSol = new_schnorr.ver_key().to_affine().into();

        let receipt = update_consensus_keys(
            &system.provider,
            system.stake_table,
            validator_address,
            new_bls,
            new_schnorr.ver_key(),
        )
        .await?;
        assert!(receipt.status());

        let event = decode_log::<StakeTable::ConsensusKeysUpdated>(&receipt).unwrap();
        assert_eq!(event.account, system.deployer_address);

        assert_eq!(event.blsVK, bls_vk_sol.into());
        assert_eq!(event.schnorrVK, schnorr_vk_sol.into());

        Ok(())
    }
}
