use alloy::{
    primitives::{Address, Bytes},
    sol_types::SolValue,
};
use ark_bn254::G2Affine;
use ark_ec::{AffineRepr, CurveGroup as _};
use ark_ed_on_bn254::EdwardsConfig;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use hotshot_types::{
    light_client::{hash_bytes_to_field, StateKeyPair, StateSignature, StateVerKey},
    signature_key::{BLSKeyPair, BLSPubKey, BLSSignature},
    traits::signature_key::SignatureKey,
};
use jf_signature::{
    bls_over_bn254,
    constants::{CS_ID_BLS_BN254, CS_ID_SCHNORR},
    schnorr,
};

use crate::sol_types::{
    StakeTableV2::{getVersionReturn, ConsensusKeysUpdatedV2, ValidatorRegisteredV2},
    *,
};

// Allows us to implement From on existing Bytes type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateSignatureSol(pub Bytes);

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

impl From<BLSPubKey> for G2PointSol {
    fn from(value: BLSPubKey) -> Self {
        value.to_affine().into()
    }
}

impl From<EdOnBN254PointSol> for StateVerKey {
    fn from(value: EdOnBN254PointSol) -> Self {
        let point: ark_ed_on_bn254::EdwardsAffine = value.into();
        Self::from(point)
    }
}

impl From<bls_over_bn254::Signature> for G1PointSol {
    fn from(sig: bls_over_bn254::Signature) -> Self {
        sig.sigma.into_affine().into()
    }
}

impl From<StateVerKey> for EdOnBN254PointSol {
    fn from(ver_key: StateVerKey) -> Self {
        ver_key.to_affine().into()
    }
}

impl From<StateSignature> for StateSignatureSol {
    fn from(sig: StateSignature) -> Self {
        let mut buf = vec![];
        sig.serialize_compressed(&mut buf).expect("serialize works");
        Self(buf.into())
    }
}

impl From<StateSignatureSol> for Bytes {
    fn from(sig_sol: StateSignatureSol) -> Self {
        sig_sol.0
    }
}

pub fn sign_address_bls(bls_key_pair: &BLSKeyPair, address: Address) -> bls_over_bn254::Signature {
    bls_key_pair.sign(&address.abi_encode(), CS_ID_BLS_BN254)
}

pub fn sign_address_schnorr(schnorr_key_pair: &StateKeyPair, address: Address) -> StateSignature {
    let msg = [hash_bytes_to_field(&address.abi_encode()).expect("hash to field works")];
    schnorr_key_pair.sign(&msg, CS_ID_SCHNORR)
}

/// Authenticate a Schnorr signature over an Ethereum address
pub fn authenticate_schnorr_sig(
    schnorr_vk: &StateVerKey,
    address: Address,
    schnorr_sig: &StateSignature,
) -> Result<(), StakeTableSolError> {
    let msg = [hash_bytes_to_field(&address.abi_encode()).expect("hash to field works")];
    schnorr_vk.verify(&msg, schnorr_sig, CS_ID_SCHNORR)?;
    Ok(())
}

/// Authenticate a BLS signature over an Ethereum address
pub fn authenticate_bls_sig(
    bls_vk: &BLSPubKey,
    address: Address,
    bls_sig: &BLSSignature,
) -> Result<(), StakeTableSolError> {
    let msg = address.abi_encode();
    if !bls_vk.validate(bls_sig, &msg) {
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
    let bls_sig_jellyfish = {
        let sigma_affine: ark_bn254::G1Affine = bls_sig.into();
        BLSSignature {
            sigma: sigma_affine.into_group(),
        }
    };
    authenticate_bls_sig(&bls_vk, account, &bls_sig_jellyfish)?;

    let schnorr_vk: StateVerKey = schnorr_vk.into();
    let schnorr_sig_jellyfish =
        schnorr::Signature::<EdwardsConfig>::deserialize_compressed(schnorr_sig)?;
    authenticate_schnorr_sig(&schnorr_vk, account, &schnorr_sig_jellyfish)?;
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

impl From<StakeTable::ValidatorRegistered> for StakeTableV2::InitialCommission {
    fn from(value: StakeTable::ValidatorRegistered) -> Self {
        Self {
            validator: value.account,
            commission: value.commission,
        }
    }
}

#[cfg(test)]
mod test {
    use alloy::primitives::Address;
    use hotshot_types::{
        light_client::StateKeyPair,
        signature_key::{BLSKeyPair, BLSPrivKey, BLSPubKey},
    };

    use super::{sign_address_bls, sign_address_schnorr, StateSignatureSol};
    use crate::sol_types::{
        G1PointSol, G2PointSol,
        StakeTableV2::{ConsensusKeysUpdatedV2, ValidatorRegisteredV2},
    };

    fn check_round_trip(pk: BLSPubKey) {
        let g2 = G2PointSol::from(pk);
        let pk2 = BLSPubKey::from(g2);
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
    fn test_validator_registered_event_authentication() {
        for _ in 0..10 {
            let bls_key_pair = BLSKeyPair::generate(&mut rand::thread_rng());
            let schnorr_key_pair = StateKeyPair::generate();
            let address = Address::random();

            let bls_sig = sign_address_bls(&bls_key_pair, address);
            let schnorr_sig = sign_address_schnorr(&schnorr_key_pair, address);

            let valid_event = ValidatorRegisteredV2 {
                account: address,
                blsVK: bls_key_pair.ver_key().into(),
                schnorrVK: schnorr_key_pair.ver_key().into(),
                commission: 1000, // 10%
                blsSig: G1PointSol::from(bls_sig.clone()).into(),
                schnorrSig: StateSignatureSol::from(schnorr_sig.clone()).into(),
            };
            assert!(valid_event.authenticate().is_ok());

            let wrong_bls_sig =
                sign_address_bls(&BLSKeyPair::generate(&mut rand::thread_rng()), address);
            let mut bad_bls_event = valid_event.clone();
            bad_bls_event.blsSig = G1PointSol::from(wrong_bls_sig).into();
            assert!(bad_bls_event.authenticate().is_err());

            let wrong_schnorr_sig = sign_address_schnorr(&StateKeyPair::generate(), address);
            let mut bad_schnorr_event = valid_event.clone();
            bad_schnorr_event.schnorrSig = StateSignatureSol::from(wrong_schnorr_sig).into();
            assert!(bad_schnorr_event.authenticate().is_err());
        }
    }

    #[test]
    fn test_consensus_keys_updated_event_authentication() {
        for _ in 0..10 {
            let bls_key_pair = BLSKeyPair::generate(&mut rand::thread_rng());
            let schnorr_key_pair = StateKeyPair::generate();
            let address = Address::random();

            let bls_sig = sign_address_bls(&bls_key_pair, address);
            let schnorr_sig = sign_address_schnorr(&schnorr_key_pair, address);

            let valid_event = ConsensusKeysUpdatedV2 {
                account: address,
                blsVK: bls_key_pair.ver_key().into(),
                schnorrVK: schnorr_key_pair.ver_key().into(),
                blsSig: G1PointSol::from(bls_sig.clone()).into(),
                schnorrSig: StateSignatureSol::from(schnorr_sig.clone()).into(),
            };
            assert!(valid_event.authenticate().is_ok());

            let wrong_bls_sig =
                sign_address_bls(&BLSKeyPair::generate(&mut rand::thread_rng()), address);
            let mut bad_bls_event = valid_event.clone();
            bad_bls_event.blsSig = G1PointSol::from(wrong_bls_sig).into();
            assert!(bad_bls_event.authenticate().is_err());

            let wrong_schnorr_sig = sign_address_schnorr(&StateKeyPair::generate(), address);
            let mut bad_schnorr_event = valid_event.clone();
            bad_schnorr_event.schnorrSig = StateSignatureSol::from(wrong_schnorr_sig).into();
            assert!(bad_schnorr_event.authenticate().is_err());
        }
    }
}
