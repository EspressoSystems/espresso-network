use ark_bn254::G2Affine;
use ark_ec::AffineRepr;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{rand::Rng, UniformRand};
use hotshot_types::{
    light_client::StateVerKey, network::PeerConfigKeys, signature_key::BLSPubKey,
    stake_table::StakeTableEntry, PeerConfig,
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

impl From<EdOnBN254PointSol> for StateVerKey {
    fn from(value: EdOnBN254PointSol) -> Self {
        let point: ark_ed_on_bn254::EdwardsAffine = value.into();
        Self::from(point)
    }
}

impl From<NodeInfoSol> for PeerConfig<BLSPubKey> {
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

impl From<NodeInfoSol> for PeerConfigKeys<BLSPubKey> {
    fn from(value: NodeInfoSol) -> Self {
        Self {
            stake_table_key: value.blsVK.into(),
            state_ver_key: value.schnorrVK.into(),
            stake: 1,
            da: value.isDA,
        }
    }
}

impl From<PeerConfigKeys<BLSPubKey>> for NodeInfoSol {
    fn from(c: PeerConfigKeys<BLSPubKey>) -> Self {
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
