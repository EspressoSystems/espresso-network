//! This module configures base fields, Merkle tree, etc for the AVID-M scheme

use ark_ff::FftField;
use ark_serialize::CanonicalSerialize;
use jf_merkle_tree::hasher::HasherNode;
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
