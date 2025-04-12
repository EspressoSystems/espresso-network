use ark_bn254::G2Affine;
use ark_ec::AffineRepr;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{rand::Rng, UniformRand};
use hotshot_types::{
    light_client::StateVerKey,
    network::PeerConfigKeys,
    signature_key::{BLSPubKey, SchnorrPubKey},
    stake_table::StakeTableEntry,
    traits::node_implementation::NodeType,
    PeerConfig,
};

use crate::{sol_types::*, *};

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

// unfortunate excessive impl due to missing shared-types in alloy bindings, read `sol_types` module doc
impl From<staketable::BN254::G2Point> for BLSPubKey {
    fn from(value: staketable::BN254::G2Point) -> Self {
        let v: G2PointSol = value.into();
        v.into()
    }
}

impl From<EdOnBN254PointSol> for StateVerKey {
    fn from(value: EdOnBN254PointSol) -> Self {
        let point: ark_ed_on_bn254::EdwardsAffine = value.into();
        Self::from(point)
    }
}

// unfortunate excessive impl due to missing shared-types in alloy bindings, read `sol_types` module doc
impl From<staketable::EdOnBN254::EdOnBN254Point> for StateVerKey {
    fn from(value: staketable::EdOnBN254::EdOnBN254Point) -> Self {
        let v: EdOnBN254PointSol = value.into();
        v.into()
    }
}

impl<TYPES> From<NodeInfoSol> for PeerConfig<TYPES>
where
    TYPES: NodeType<SignatureKey = BLSPubKey, StateSignatureKey = SchnorrPubKey>,
{
    fn from(value: NodeInfoSol) -> Self {
        Self {
            stake_table_entry: StakeTableEntry {
                stake_key: value.blsVK.into(),
                stake_amount: U256::from(1),
            },
            state_ver_key: value.schnorrVK.into(),
        }
    }
}

impl<TYPES> From<NodeInfoSol> for PeerConfigKeys<TYPES>
where
    TYPES: NodeType<SignatureKey = BLSPubKey, StateSignatureKey = SchnorrPubKey>,
{
    fn from(value: NodeInfoSol) -> Self {
        Self {
            stake_table_key: value.blsVK.into(),
            state_ver_key: value.schnorrVK.into(),
            stake: 1,
            da: value.isDA,
        }
    }
}

impl<TYPES> From<PeerConfigKeys<TYPES>> for NodeInfoSol
where
    TYPES: NodeType<SignatureKey = BLSPubKey, StateSignatureKey = SchnorrPubKey>,
{
    fn from(c: PeerConfigKeys<TYPES>) -> Self {
        Self {
            blsVK: c.stake_table_key.to_affine().into(),
            schnorrVK: c.state_ver_key.to_affine().into(),
            isDA: c.da,
        }
    }
}

impl NodeInfoSol {
    /// Generate a random staker
    pub fn rand<R: Rng>(rng: &mut R) -> Self {
        Self {
            blsVK: ark_bn254::G2Affine::rand(rng).into(),
            schnorrVK: ark_ed_on_bn254::EdwardsAffine::rand(rng).into(),
            isDA: rng.gen_bool(0.2),
        }
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
