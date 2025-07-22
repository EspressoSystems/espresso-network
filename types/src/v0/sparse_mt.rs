use std::{collections::HashMap, fmt};

use alloy::{
    hex,
    primitives::{Address, U256},
};
use anyhow::Result;
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, Read, SerializationError, Valid, Validate,
};
use derive_more::From;
use jf_merkle_tree::{
    prelude::*, universal_merkle_tree::UniversalMerkleTree, DigestAlgorithm, MerkleTreeScheme,
    ToTraversalPath, UniversalMerkleTreeScheme,
};
use sha3::{Digest as _, Keccak256};

use crate::v0_1::{RewardAccount, RewardAmount};

impl From<[u8; 20]> for RewardAccount {
    fn from(bytes: [u8; 20]) -> Self {
        Self(Address::from(bytes))
    }
}

impl AsRef<[u8]> for RewardAccount {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl<const ARITY: usize> ToTraversalPath<ARITY> for RewardAccount {
    fn to_traversal_path(&self, height: usize) -> Vec<usize> {
        let mut result = vec![0; height];

        // Convert 20-byte address to U256
        let mut value = U256::from_be_slice(self.0.as_slice());

        // Extract digits using modulo and division (LSB first)
        for item in result.iter_mut().take(height) {
            let digit = (value % U256::from(ARITY)).to::<usize>();
            *item = digit;
            value /= U256::from(ARITY);
        }

        result
    }
}

/// Value newtype for U256
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Value(pub U256);

impl Valid for Value {
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }
}

impl From<U256> for Value {
    fn from(val: U256) -> Self {
        Self(val)
    }
}

impl CanonicalSerialize for Value {
    fn serialize_with_mode<W: ark_serialize::Write>(
        &self,
        mut writer: W,
        _compress: Compress,
    ) -> Result<(), SerializationError> {
        Ok(writer.write_all(&self.0.to_be_bytes::<32>())?)
    }

    fn serialized_size(&self, _compress: Compress) -> usize {
        core::mem::size_of::<U256>()
    }
}
impl CanonicalDeserialize for Value {
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        _compress: Compress,
        _validate: Validate,
    ) -> Result<Self, SerializationError> {
        let mut bytes = [0u8; core::mem::size_of::<U256>()];
        reader.read_exact(&mut bytes)?;
        let value = U256::from_be_slice(&bytes);
        Ok(Self(value))
    }
}

impl AsRef<[u8]> for Value {
    fn as_ref(&self) -> &[u8] {
        // This implementation should ideally not be used for hashing
        // The digest_leaf function uses u256_to_bytes32 for proper big-endian conversion
        self.0.as_le_slice()
    }
}

/// Custom Keccak256 node for our merkle tree
#[derive(Default, Eq, PartialEq, Clone, Copy, Ord, PartialOrd, Hash)]
pub struct JellyfishKeccakNode(pub [u8; 32]);

impl fmt::Debug for JellyfishKeccakNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("JellyfishKeccakNode")
            .field(&hex::encode(self.0))
            .finish()
    }
}

impl AsRef<[u8]> for JellyfishKeccakNode {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl CanonicalSerialize for JellyfishKeccakNode {
    fn serialize_with_mode<W: ark_serialize::Write>(
        &self,
        mut writer: W,
        _compress: Compress,
    ) -> Result<(), SerializationError> {
        writer.write_all(&self.0)?;
        Ok(())
    }

    fn serialized_size(&self, _compress: Compress) -> usize {
        32
    }
}

impl CanonicalDeserialize for JellyfishKeccakNode {
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        _compress: Compress,
        _validate: Validate,
    ) -> Result<Self, SerializationError> {
        let mut ret = [0u8; 32];
        reader.read_exact(&mut ret)?;
        Ok(Self(ret))
    }
}

impl Valid for JellyfishKeccakNode {
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }
}

/// Keccak256 hasher that matches our Solidity implementation
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct JellyfishKeccak256Hasher;

impl DigestAlgorithm<RewardAmount, RewardAccount, JellyfishKeccakNode>
    for JellyfishKeccak256Hasher
{
    fn digest(
        data: &[JellyfishKeccakNode],
    ) -> Result<JellyfishKeccakNode, jf_merkle_tree::MerkleTreeError> {
        let mut hasher = Keccak256::new();

        // Hash the concatenated node data directly (no domain separator)
        for node in data {
            hasher.update(node.as_ref());
        }

        let result = hasher.finalize();
        Ok(JellyfishKeccakNode(result.into()))
    }

    fn digest_leaf(
        _pos: &RewardAccount,
        elem: &RewardAmount,
    ) -> Result<JellyfishKeccakNode, jf_merkle_tree::MerkleTreeError> {
        // First hash of the value
        let mut hasher = Keccak256::new();
        hasher.update(&elem.0.to_be_bytes::<32>()); // 32-byte value as big-endian
        let first_hash = hasher.finalize();

        // Second hash (double hashing)
        let mut hasher = Keccak256::new();
        hasher.update(&first_hash);
        let result = hasher.finalize();

        Ok(JellyfishKeccakNode(result.into()))
    }
}
