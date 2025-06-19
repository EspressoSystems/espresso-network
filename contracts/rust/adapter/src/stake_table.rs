use alloy::{
    primitives::{Address, Bytes},
    sol_types::SolValue,
};
use ark_bn254::G2Affine;
use ark_ec::{AffineRepr, CurveGroup as _};
use ark_ed_on_bn254::EdwardsConfig;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use hotshot_types::{
    light_client::{hash_bytes_to_field, StateKeyPair, StateVerKey},
    signature_key::{BLSKeyPair, BLSPubKey, BLSSignature},
    traits::signature_key::SignatureKey,
};
use jf_signature::{
    constants::{CS_ID_BLS_BN254, CS_ID_SCHNORR},
    schnorr,
};

use crate::sol_types::{
    StakeTableV2::{getVersionReturn, ConsensusKeysUpdatedV2, ValidatorRegisteredV2},
    *,
};

#[derive(Debug, Clone, Copy, Default)]
pub enum StakeTableContractVersion {
    V1,
    #[default]
    V2,
}

impl TryFrom<getVersionReturn> for StakeTableContractVersion {
    type Error = anyhow::Error;

    fn try_from(value: getVersionReturn) -> anyhow::Result<Self> {
        match value.majorVersion {
            1 => Ok(StakeTableContractVersion::V1),
            2 => Ok(StakeTableContractVersion::V2),
            _ => anyhow::bail!("Unsupported stake table contract version: {:?}", value),
        }
    }
}

impl From<G2PointSol> for BLSPubKey {
    fn from(value: G2PointSol) -> Self {
        let point: G2Affine = value.into();
        let mut bytes = vec![];
        point
            .into_group()
            .serialize_uncompressed(&mut bytes)
            .unwrap();
        Self::deserialize_uncompressed(&bytes[..]).unwrap()
    }
}

impl From<EdOnBN254PointSol> for StateVerKey {
    fn from(value: EdOnBN254PointSol) -> Self {
        let point: ark_ed_on_bn254::EdwardsAffine = value.into();
        Self::from(point)
    }
}

pub fn sign_address_bls(bls_key_pair: &BLSKeyPair, address: Address) -> G1PointSol {
    bls_key_pair
        .sign(&address.abi_encode(), CS_ID_BLS_BN254)
        .sigma
        .into_affine()
        .into()
}

pub fn sign_address_schnorr(schnorr_key_pair: &StateKeyPair, address: Address) -> Bytes {
    let msg = [hash_bytes_to_field(&address.abi_encode()).expect("hash to field works")];
    let mut buf = vec![];
    schnorr_key_pair
        .sign(&msg, CS_ID_SCHNORR)
        .serialize_compressed(&mut buf)
        .expect("serialize works");
    buf.into()
}

// Helper function useful for unit tests.
fn authenticate_schnorr_sig(
    schnorr_vk: &StateVerKey,
    address: Address,
    schnorr_sig: &[u8],
) -> Result<(), StakeTableSolError> {
    let msg = [hash_bytes_to_field(&address.abi_encode()).expect("hash to field works")];
    let sig = schnorr::Signature::<EdwardsConfig>::deserialize_compressed(schnorr_sig)?;
    schnorr_vk.verify(&msg, &sig, CS_ID_SCHNORR)?;
    Ok(())
}

// Helper function useful for unit tests.
fn authenticate_bls_sig(
    bls_vk: &BLSPubKey,
    address: Address,
    bls_sig: &G1PointSol,
) -> Result<(), StakeTableSolError> {
    let msg = address.abi_encode();
    let sig = {
        let sigma_affine: ark_bn254::G1Affine = (*bls_sig).into();
        BLSSignature {
            sigma: sigma_affine.into_group(),
        }
    };
    if !bls_vk.validate(&sig, &msg) {
        return Err(StakeTableSolError::InvalidBlsSignature);
    }
    Ok(())
}

fn authenticate_stake_table_validator_event(
    account: Address,
    bls_vk: G2PointSol,
    schnorr_vk: EdOnBN254PointSol,
    bls_sig: G1PointSol,
    schnorr_sig: &[u8],
) -> Result<(), StakeTableSolError> {
    // TODO(alex): simplify this once jellyfish has `VerKey::from_affine()`
    let bls_vk = {
        let bls_vk_inner: ark_bn254::G2Affine = bls_vk.into();
        let bls_vk_inner = bls_vk_inner.into_group();

        // the two unwrap are safe since it's BLSPubKey is just a wrapper around G2Projective
        let mut ser_bytes: Vec<u8> = Vec::new();
        bls_vk_inner.serialize_uncompressed(&mut ser_bytes).unwrap();
        BLSPubKey::deserialize_uncompressed(&ser_bytes[..]).unwrap()
    };
    authenticate_bls_sig(&bls_vk, account, &bls_sig)?;

    let schnorr_vk: StateVerKey = schnorr_vk.into();
    authenticate_schnorr_sig(&schnorr_vk, account, schnorr_sig)?;
    Ok(())
}

/// Errors encountered when processing stake table events
#[derive(Debug, thiserror::Error)]
pub enum StakeTableSolError {
    #[error("Failed to deserialize Schnorr signature")]
    SchnorrSigDeserializationError(#[from] ark_serialize::SerializationError),
    #[error("BLS signature invalid")]
    InvalidBlsSignature,
    #[error("Schnorr signature invalid")]
    InvalidSchnorrSignature(#[from] jf_signature::SignatureError),
}

impl ValidatorRegisteredV2 {
    /// verified the BLS and Schnorr signatures in the event
    pub fn authenticate(&self) -> Result<(), StakeTableSolError> {
        authenticate_stake_table_validator_event(
            self.account,
            self.blsVK,
            self.schnorrVK,
            self.blsSig.into(),
            &self.schnorrSig,
        )?;
        Ok(())
    }
}

impl ConsensusKeysUpdatedV2 {
    /// verified the BLS and Schnorr signatures in the event
    pub fn authenticate(&self) -> Result<(), StakeTableSolError> {
        authenticate_stake_table_validator_event(
            self.account,
            self.blsVK,
            self.schnorrVK,
            self.blsSig.into(),
            &self.schnorrSig,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use alloy::primitives::{Address, U256};
    use hotshot_types::{
        light_client::StateKeyPair,
        signature_key::{BLSKeyPair, BLSPrivKey, BLSPubKey},
    };

    use super::{
        authenticate_bls_sig, authenticate_schnorr_sig, sign_address_bls, sign_address_schnorr,
    };
    use crate::sol_types::G2PointSol;

    fn check_round_trip(pk: BLSPubKey) {
        let g2: G2PointSol = pk.to_affine().into();
        let pk2: BLSPubKey = g2.into();
        assert_eq!(pk2, pk, "Failed to roundtrip G2PointSol to BLSPubKey: {pk}");
    }

    #[test]
    fn test_bls_g2_point_roundtrip() {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let pk = (&BLSPrivKey::generate(&mut rng)).into();
            check_round_trip(pk);
        }
    }

    #[test]
    fn test_bls_g2_point_alloy_migration_regression() {
        // This pubkey fails the roundtrip if "serialize_{un,}compressed" are mixed
        let s = "BLS_VER_KEY~JlRLUrn0T_MltAJXaaojwk_CnCgd0tyPny_IGdseMBLBPv9nWabIPAaS-aHmn0ARu5YZHJ7mfmGQ-alW42tkJM663Lse-Is80fyA1jnRxPsHcJDnO05oW1M1SC5LeE8sXITbuhmtG2JdTAgmLqWOxbMRmVIqS1AQXqvGGXdo5qpd";
        let pk: BLSPubKey = s.parse().unwrap();
        check_round_trip(pk);
    }

    #[test]
    fn test_schnorr_sigs() {
        for _ in 0..10 {
            let key_pair = StateKeyPair::generate();
            let address = Address::random();
            let sig = sign_address_schnorr(&key_pair, address);
            authenticate_schnorr_sig(key_pair.ver_key_ref(), address, &sig).unwrap();

            // signed with wrong key
            let sig = sign_address_schnorr(&StateKeyPair::generate(), address);
            assert!(authenticate_schnorr_sig(key_pair.ver_key_ref(), address, &sig).is_err());

            // manipulate one byte
            let mut bad_sig: Vec<u8> = sig.to_vec();
            bad_sig[0] = bad_sig[0].wrapping_add(1);
            assert!(authenticate_schnorr_sig(key_pair.ver_key_ref(), address, &bad_sig).is_err());
        }
    }

    #[test]
    fn test_bls_sigs() {
        let key_pair = BLSKeyPair::generate(&mut rand::thread_rng());
        let address = Address::random();
        let sig = sign_address_bls(&key_pair, address);
        authenticate_bls_sig(key_pair.ver_key_ref(), address, &sig).unwrap();

        // signed with wrong key
        assert!(authenticate_bls_sig(
            key_pair.ver_key_ref(),
            address,
            &sign_address_bls(&BLSKeyPair::generate(&mut rand::thread_rng()), address)
        )
        .is_err());

        // tamper with the signature
        let mut sig = sig;
        sig.x = sig.x.wrapping_add(U256::from(1));
        assert!(authenticate_bls_sig(key_pair.ver_key_ref(), address, &sig).is_err());
    }
}
