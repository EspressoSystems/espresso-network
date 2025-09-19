//! This module configures base fields, Merkle tree, etc for the AVID-M scheme

use ark_ff::FftField;
use ark_serialize::CanonicalSerialize;
use jf_crhf::CRHF;
use jf_merkle_tree::hasher::HasherNode;
use jf_poseidon2::{
    constants::bn254::Poseidon2ParamsBn3, crhf::FixedLenPoseidon2Hash,
    permutation::Poseidon2Permutation,
};
use sha2::Digest;

use crate::{VidError, VidResult};

pub trait AvidMConfig {
    type BaseField: FftField;

    type Digest: jf_merkle_tree::NodeValue;

    type MerkleTree: jf_merkle_tree::MerkleTreeScheme<
        Element = Self::Digest,
        Commitment = Self::Digest,
    >;

    /// Digest the raw shares into the element type for Merkle tree.
    ///
    /// # Errors
    ///
    /// This function will return an error if digest function fails.
    fn raw_share_digest(raw_shares: &[Self::BaseField]) -> VidResult<Self::Digest>;
}

/// Configuration of Poseidon2 based AVID-M scheme
pub struct Poseidon2Config;

type Poseidon2SpongeStateBnN3R1 = Poseidon2Permutation<ark_bn254::Fr, 3, 1, Poseidon2ParamsBn3>;

impl AvidMConfig for Poseidon2Config {
    type BaseField = ark_bn254::Fr;

    type Digest = ark_bn254::Fr;

    type MerkleTree = jf_merkle_tree::append_only::MerkleTree<
        Self::Digest,
        FixedLenPoseidon2Hash<Self::BaseField, Poseidon2SpongeStateBnN3R1, 3, 1>,
        u64,
        3,
        Self::Digest,
    >;

    fn raw_share_digest(raw_shares: &[Self::BaseField]) -> VidResult<Self::Digest> {
        jf_poseidon2::crhf::VariableLenPoseidon2Hash::<Self::BaseField, Poseidon2SpongeStateBnN3R1, 1>::evaluate(
            raw_shares,
        )
        .map(|v| v[0])
        .map_err(|err| VidError::Internal(err.into()))
    }
}

/// Configuration of Sha256 based AVID-M scheme
pub struct Sha256Config;

impl AvidMConfig for Sha256Config {
    type BaseField = ark_bn254::Fr;

    type Digest = HasherNode<sha2::Sha256>;

    type MerkleTree = jf_merkle_tree::hasher::HasherMerkleTree<sha2::Sha256, Self::Digest>;

    fn raw_share_digest(raw_shares: &[Self::BaseField]) -> VidResult<Self::Digest> {
        let mut hasher = sha2::Sha256::new();
        raw_shares
            .serialize_uncompressed(&mut hasher)
            .map_err(|err| VidError::Internal(err.into()))?;
        Ok(HasherNode::from(hasher.finalize()))
    }
}

/// Configuration of Keccak256 based AVID-M scheme
pub struct Keccak256Config;

impl AvidMConfig for Keccak256Config {
    type BaseField = ark_bn254::Fr;

    type Digest = HasherNode<sha3::Keccak256>;

    type MerkleTree = jf_merkle_tree::hasher::HasherMerkleTree<sha3::Keccak256, Self::Digest>;

    fn raw_share_digest(raw_shares: &[Self::BaseField]) -> VidResult<Self::Digest> {
        let mut hasher = sha3::Keccak256::new();
        raw_shares
            .serialize_uncompressed(&mut hasher)
            .map_err(|err| VidError::Internal(err.into()))?;
        Ok(HasherNode::from(hasher.finalize()))
    }
}
