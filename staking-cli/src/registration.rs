use alloy::{
    primitives::Address, providers::Provider, rpc::types::TransactionReceipt,
    sol_types::SolValue as _, transports::Transport,
};
use anyhow::Result;
use ark_ec::CurveGroup;
use contract_bindings_alloy::staketable::{
    EdOnBN254::EdOnBN254Point,
    StakeTable::StakeTableInstance,
    BN254::{G1Point, G2Point},
};
use ethers_conv::ToAlloy;
use hotshot_contract_adapter::{
    jellyfish::ParsedG1Point,
    stake_table::{ParsedEdOnBN254Point, ParsedG2Point},
};
use jf_signature::constants::CS_ID_BLS_BN254;

use crate::{parse::Commission, BLSKeyPair, StateVerKey};

fn to_alloy_g1_point(p: ParsedG1Point) -> G1Point {
    G1Point {
        x: p.x.to_alloy(),
        y: p.y.to_alloy(),
    }
}

fn to_alloy_g2_point(p: ParsedG2Point) -> G2Point {
    G2Point {
        x0: p.x0.to_alloy(),
        x1: p.x1.to_alloy(),
        y0: p.y0.to_alloy(),
        y1: p.y1.to_alloy(),
    }
}

fn to_alloy_ed_on_bn_point(p: ParsedEdOnBN254Point) -> EdOnBN254Point {
    EdOnBN254Point {
        x: p.x.to_alloy(),
        y: p.y.to_alloy(),
    }
}

pub async fn register_validator<P: Provider<T>, T: Transport + Clone>(
    stake_table: StakeTableInstance<T, P>,
    commission: Commission,
    validator_address: Address,
    bls_key_pair: BLSKeyPair,
    schnorr_vk: StateVerKey,
) -> Result<TransactionReceipt> {
    let bls_vk = bls_key_pair.ver_key();

    let sig_parsed: ParsedG2Point = bls_vk.to_affine().into();
    let bls_vk_alloy = to_alloy_g2_point(sig_parsed);

    let sig = bls_key_pair.sign(&validator_address.abi_encode(), CS_ID_BLS_BN254);
    let sig_parsed: ParsedG1Point = sig.sigma.into_affine().into();
    let sig_alloy = to_alloy_g1_point(sig_parsed);

    let schnorr_vk_parsed: ParsedEdOnBN254Point = schnorr_vk.to_affine().into();
    let schnorr_vk_alloy = to_alloy_ed_on_bn_point(schnorr_vk_parsed);

    Ok(stake_table
        .registerValidator(
            bls_vk_alloy,
            schnorr_vk_alloy,
            sig_alloy,
            commission.to_evm(),
        )
        .send()
        .await?
        .get_receipt()
        .await?)
}

pub async fn deregister_validator<P: Provider<T>, T: Transport + Clone>(
    stake_table: StakeTableInstance<T, P>,
) -> Result<TransactionReceipt> {
    Ok(stake_table
        .deregisterValidator()
        .send()
        .await?
        .get_receipt()
        .await?)
}

#[cfg(test)]
mod test {
    use contract_bindings_alloy::staketable::StakeTable;

    use super::*;
    use crate::{deploy::TestSystem, l1::decode_log};

    #[tokio::test]
    async fn test_register_validator() -> Result<()> {
        let system = TestSystem::deploy().await?;

        let validator_address = system.deployer_address;
        let receipt = register_validator(
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

        let receipt = deregister_validator(system.stake_table).await?;
        assert!(receipt.status());

        let event = decode_log::<StakeTable::ValidatorExit>(&receipt).unwrap();
        assert_eq!(event.validator, system.deployer_address);

        Ok(())
    }
}
