use alloy::{
    primitives::Address, providers::Provider, rpc::types::TransactionReceipt,
    sol_types::SolValue as _,
};
use anyhow::Result;
use ark_ec::CurveGroup;
use hotshot_contract_adapter::sol_types::{EdOnBN254PointSol, G1PointSol, G2PointSol, StakeTable};
use jf_signature::constants::CS_ID_BLS_BN254;

use crate::{parse::Commission, BLSKeyPair, StateVerKey};

pub async fn register_validator(
    provider: impl Provider,
    stake_table_addr: Address,
    commission: Commission,
    validator_address: Address,
    bls_key_pair: BLSKeyPair,
    schnorr_vk: StateVerKey,
) -> Result<TransactionReceipt> {
    let stake_table = StakeTable::new(stake_table_addr, &provider);
    let sig = bls_key_pair.sign(&validator_address.abi_encode(), CS_ID_BLS_BN254);

    let bls_vk_sol: G2PointSol = bls_key_pair.ver_key().to_affine().into();
    let schnorr_vk_sol: EdOnBN254PointSol = schnorr_vk.to_affine().into();
    let sig_sol: G1PointSol = sig.sigma.into_affine().into();
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
    use super::*;
    use crate::{deploy::TestSystem, l1::decode_log};

    #[tokio::test]
    async fn test_register_validator() -> Result<()> {
        let system = TestSystem::deploy().await?;

        let validator_address = system.deployer_address;
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
}
