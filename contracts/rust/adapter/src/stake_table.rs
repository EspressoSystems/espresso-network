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
        Self::deserialize_compressed(&bytes[..]).unwrap()
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
