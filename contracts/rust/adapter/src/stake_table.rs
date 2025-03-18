use std::str::FromStr;

use ark_ec::{
    short_weierstrass,
    twisted_edwards::{self, Affine, TECurveConfig},
    AffineRepr,
};
use ark_ed_on_bn254::EdwardsConfig;
use ark_ff::{BigInteger, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::rand::{Rng, RngCore};
use contract_bindings_alloy::permissionedstaketable::{
    EdOnBN254::EdOnBN254Point as EdOnBN254PointAlloy,
    PermissionedStakeTable::NodeInfo as NodeInfoAlloy, BN254::G2Point as G2PointAlloy,
};
use contract_bindings_ethers::permissioned_stake_table::{self, EdOnBN254Point, NodeInfo};
pub use diff_test_bn254::ParsedG2Point;
use ethers::{
    abi::AbiDecode,
    prelude::{AbiError, EthAbiCodec, EthAbiType},
    types::U256,
};
use ethers_conv::{ToAlloy, ToEthers};
use hotshot_types::{
    light_client::{StateKeyPair, StateVerKey},
    network::PeerConfigKeys,
    signature_key::BLSPubKey,
    stake_table::StakeTableEntry,
    traits::signature_key::SignatureKey as _,
    PeerConfig,
};
use serde::{Deserialize, Serialize};

use crate::jellyfish::u256_to_field;

// TODO: (alex) maybe move these commonly shared util to a crate
/// convert a field element to U256, panic if field size is larger than 256 bit
pub fn field_to_u256<F: PrimeField>(f: F) -> U256 {
    if F::MODULUS_BIT_SIZE > 256 {
        panic!("Shouldn't convert a >256-bit field to U256");
    }
    U256::from_little_endian(&f.into_bigint().to_bytes_le())
}

/// an intermediate representation of `EdOnBN254Point` in solidity.
#[derive(Clone, PartialEq, Eq, Debug, EthAbiType, EthAbiCodec)]
pub struct ParsedEdOnBN254Point {
    /// x coordinate of affine repr
    pub x: U256,
    /// y coordinate of affine repr
    pub y: U256,
}

// this is convention from BN256 precompile
impl Default for ParsedEdOnBN254Point {
    fn default() -> Self {
        Self {
            x: U256::from(0),
            y: U256::from(0),
        }
    }
}

impl From<ParsedEdOnBN254Point> for EdOnBN254Point {
    fn from(value: ParsedEdOnBN254Point) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

impl From<ParsedEdOnBN254Point> for EdOnBN254PointAlloy {
    fn from(value: ParsedEdOnBN254Point) -> Self {
        Self {
            x: value.x.to_alloy(),
            y: value.y.to_alloy(),
        }
    }
}

impl From<EdOnBN254Point> for ParsedEdOnBN254Point {
    fn from(value: EdOnBN254Point) -> Self {
        let EdOnBN254Point { x, y } = value;
        Self { x, y }
    }
}

impl From<EdOnBN254PointAlloy> for ParsedEdOnBN254Point {
    fn from(value: EdOnBN254PointAlloy) -> Self {
        let EdOnBN254PointAlloy { x, y } = value;
        Self {
            x: x.to_ethers(),
            y: y.to_ethers(),
        }
    }
}

impl FromStr for ParsedEdOnBN254Point {
    type Err = AbiError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parsed: (Self,) = AbiDecode::decode_hex(s)?;
        Ok(parsed.0)
    }
}

impl<P: TECurveConfig> From<Affine<P>> for ParsedEdOnBN254Point
where
    P::BaseField: PrimeField,
{
    fn from(p: Affine<P>) -> Self {
        if p.is_zero() {
            // this convention is from the BN precompile
            Self {
                x: U256::from(0),
                y: U256::from(0),
            }
        } else {
            Self {
                x: field_to_u256::<P::BaseField>(*p.x().unwrap()),
                y: field_to_u256::<P::BaseField>(*p.y().unwrap()),
            }
        }
    }
}

impl<P: TECurveConfig> From<ParsedEdOnBN254Point> for Affine<P>
where
    P::BaseField: PrimeField,
{
    fn from(p: ParsedEdOnBN254Point) -> Self {
        if p == ParsedEdOnBN254Point::default() {
            Self::default()
        } else {
            Self::new_unchecked(
                u256_to_field::<P::BaseField>(p.x),
                u256_to_field::<P::BaseField>(p.y),
            )
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeInfoJf {
    pub stake_table_key: BLSPubKey,
    pub state_ver_key: StateVerKey,
    pub da: bool,
}

impl NodeInfoJf {
    pub fn random(rng: &mut impl RngCore) -> Self {
        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed);

        let (stake_table_key, _) = BLSPubKey::generated_from_seed_indexed(seed, 0);
        let state_key_pair = StateKeyPair::generate_from_seed_indexed(seed, 0);
        Self {
            stake_table_key,
            state_ver_key: state_key_pair.ver_key(),
            da: rng.gen(),
        }
    }

    pub fn stake_table_key_alloy(&self) -> G2PointAlloy {
        bls_jf_to_alloy(self.stake_table_key)
    }
}

impl From<NodeInfoJf> for NodeInfo {
    fn from(value: NodeInfoJf) -> Self {
        let NodeInfoJf {
            stake_table_key,
            state_ver_key,
            da,
        } = value;
        let ParsedG2Point { x0, x1, y0, y1 } = stake_table_key.to_affine().into();
        let schnorr: ParsedEdOnBN254Point = state_ver_key.to_affine().into();
        Self {
            bls_vk: permissioned_stake_table::G2Point {
                x_0: x0,
                x_1: x1,
                y_0: y0,
                y_1: y1,
            },
            schnorr_vk: schnorr.into(),
            is_da: da,
        }
    }
}

impl From<NodeInfoJf> for NodeInfoAlloy {
    fn from(value: NodeInfoJf) -> Self {
        let NodeInfoJf {
            stake_table_key,
            state_ver_key,
            da,
        } = value;
        let ParsedG2Point { x0, x1, y0, y1 } = stake_table_key.to_affine().into();
        let schnorr: ParsedEdOnBN254Point = state_ver_key.to_affine().into();
        Self {
            blsVK: G2PointAlloy {
                x0: x0.to_alloy(),
                x1: x1.to_alloy(),
                y0: y0.to_alloy(),
                y1: y1.to_alloy(),
            },
            schnorrVK: schnorr.into(),
            isDA: da,
        }
    }
}

impl From<NodeInfoJf> for StakeTableEntry<BLSPubKey> {
    fn from(value: NodeInfoJf) -> Self {
        StakeTableEntry {
            stake_key: value.stake_table_key,
            stake_amount: U256::from(1), // dummy stake amount
        }
    }
}

impl From<NodeInfoJf> for PeerConfig<BLSPubKey> {
    fn from(value: NodeInfoJf) -> Self {
        Self {
            stake_table_entry: StakeTableEntry {
                stake_key: value.stake_table_key,
                stake_amount: U256::from(1), // dummy stake amount
            },
            state_ver_key: value.state_ver_key,
        }
    }
}

impl From<NodeInfo> for NodeInfoJf {
    fn from(value: NodeInfo) -> Self {
        let NodeInfo {
            bls_vk,
            schnorr_vk,
            is_da,
        } = value;
        let stake_table_key = bls_sol_to_jf(bls_vk);
        let state_ver_key = {
            let g1_point: ParsedEdOnBN254Point = schnorr_vk.into();
            let state_sk_affine = twisted_edwards::Affine::<EdwardsConfig>::from(g1_point);
            StateVerKey::from(state_sk_affine)
        };
        Self {
            stake_table_key,
            state_ver_key,
            da: is_da,
        }
    }
}

impl From<NodeInfoAlloy> for NodeInfoJf {
    fn from(value: NodeInfoAlloy) -> Self {
        let NodeInfoAlloy {
            blsVK,
            schnorrVK,
            isDA,
        } = value;
        let stake_table_key = {
            let g2 = diff_test_bn254::ParsedG2Point {
                x0: blsVK.x0.to_ethers(),
                x1: blsVK.x1.to_ethers(),
                y0: blsVK.y0.to_ethers(),
                y1: blsVK.y1.to_ethers(),
            };
            let g2_affine = short_weierstrass::Affine::<ark_bn254::g2::Config>::from(g2);
            let mut bytes = vec![];
            // TODO: remove serde round-trip once jellyfin provides a way to
            // convert from Affine representation to VerKey.
            //
            // Serialization and de-serialization shouldn't fail.
            g2_affine
                .into_group()
                .serialize_compressed(&mut bytes)
                .unwrap();
            BLSPubKey::deserialize_compressed(&bytes[..]).unwrap()
        };
        let state_ver_key = {
            let g1_point: ParsedEdOnBN254Point = schnorrVK.into();
            let state_sk_affine = twisted_edwards::Affine::<EdwardsConfig>::from(g1_point);
            StateVerKey::from(state_sk_affine)
        };
        Self {
            stake_table_key,
            state_ver_key,
            da: isDA,
        }
    }
}

impl From<PeerConfigKeys<BLSPubKey>> for NodeInfoJf {
    fn from(value: PeerConfigKeys<BLSPubKey>) -> Self {
        let PeerConfigKeys {
            stake_table_key,
            state_ver_key,
            da,
            ..
        } = value;
        Self {
            stake_table_key,
            state_ver_key,
            da,
        }
    }
}

pub fn bls_jf_to_sol(bls_vk: BLSPubKey) -> permissioned_stake_table::G2Point {
    let ParsedG2Point { x0, x1, y0, y1 } = bls_vk.to_affine().into();
    permissioned_stake_table::G2Point {
        x_0: x0,
        x_1: x1,
        y_0: y0,
        y_1: y1,
    }
}

fn bls_conv_helper(g2: diff_test_bn254::ParsedG2Point) -> BLSPubKey {
    let g2_affine = short_weierstrass::Affine::<ark_bn254::g2::Config>::from(g2);
    let mut bytes = vec![];
    // TODO: remove serde round-trip once jellyfin provides a way to
    // convert from Affine representation to VerKey.
    //
    // Serialization and de-serialization shouldn't fail.
    g2_affine
        .into_group()
        .serialize_compressed(&mut bytes)
        .unwrap();
    BLSPubKey::deserialize_compressed(&bytes[..]).unwrap()
}

pub fn bls_sol_to_jf(bls_vk: permissioned_stake_table::G2Point) -> BLSPubKey {
    let g2 = diff_test_bn254::ParsedG2Point {
        x0: bls_vk.x_0,
        x1: bls_vk.x_1,
        y0: bls_vk.y_0,
        y1: bls_vk.y_1,
    };
    bls_conv_helper(g2)
}

pub fn bls_alloy_to_jf(bls_vk: G2PointAlloy) -> BLSPubKey {
    let g2 = diff_test_bn254::ParsedG2Point {
        x0: bls_vk.x0.to_ethers(),
        x1: bls_vk.x1.to_ethers(),
        y0: bls_vk.y0.to_ethers(),
        y1: bls_vk.y1.to_ethers(),
    };
    bls_conv_helper(g2)
}

pub fn bls_jf_to_alloy(bls_vk: BLSPubKey) -> G2PointAlloy {
    let ParsedG2Point { x0, x1, y0, y1 } = bls_vk.to_affine().into();
    G2PointAlloy {
        x0: x0.to_alloy(),
        x1: x1.to_alloy(),
        y0: y0.to_alloy(),
        y1: y1.to_alloy(),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_node_info_round_trip() {
        let mut rng = rand::thread_rng();
        for _ in 0..20 {
            let jf = NodeInfoJf::random(&mut rng);
            let sol: NodeInfo = jf.clone().into();
            let jf2: NodeInfoJf = sol.into();
            assert_eq!(jf2, jf);
        }
    }
}
