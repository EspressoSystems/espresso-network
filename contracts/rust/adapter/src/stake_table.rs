use alloy::sol_types::SolValue;
use ark_bn254::G2Affine;
use ark_ec::AffineRepr;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use hotshot_types::{
    light_client::StateVerKey,
    signature_key::{BLSPubKey, BLSSignature},
    traits::signature_key::SignatureKey,
};

use crate::sol_types::{StakeTableV2::ConsensusKeysUpdatedV2, *};

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

/// Stake-table related error from the contract
#[derive(Debug, thiserror::Error)]
pub enum StakeTableSolError {
    #[error("v1 err: {0:?}")]
    V1(StakeTable::StakeTableErrors),
    #[error("v2 err: {0:?}")]
    V2(StakeTableV2::StakeTableV2Errors),
    #[error("BLS signature invalid")]
    InvalidBlsSignature,
    #[error("Schnorr signature invalid")]
    InvalidSchnorrSignature,
}
impl ConsensusKeysUpdatedV2 {
    /// by checking the `blsSig` is indeed produced by the `blsVK` field,
    /// we authenticate the claimed consensus key
    pub fn authenticate(&self) -> Result<(), StakeTableSolError> {
        // TODO(alex): simplify this once jellyfish has `VerKey::from_affine()`
        let bls_vk = {
            let bls_vk_inner: ark_bn254::G2Affine = self.blsVK.clone().into();
            let bls_vk_inner = bls_vk_inner.into_group();

            // the two unwrap are safe since it's BLSPubKey is just a wrapper around G2Projective
            let mut ser_bytes: Vec<u8> = Vec::new();
            bls_vk_inner.serialize_uncompressed(&mut ser_bytes).unwrap();
            BLSPubKey::deserialize_uncompressed(&ser_bytes[..]).unwrap()
        };
        let msg = self.account.abi_encode();
        let sig = {
            let sigma_sol: G1PointSol = self.blsSig.clone().into();
            let sigma_affine: ark_bn254::G1Affine = sigma_sol.into();
            BLSSignature {
                sigma: sigma_affine.into_group(),
            }
        };
        if !bls_vk.validate(&sig, &msg) {
            return Err(StakeTableSolError::InvalidBlsSignature);
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use hotshot_types::signature_key::{BLSPrivKey, BLSPubKey};

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
}
