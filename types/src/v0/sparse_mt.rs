use std::fmt;

use alloy::{
    hex,
    primitives::{Address, U256},
};
use anyhow::Result;
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, Read, SerializationError, Valid, Validate,
};
use derive_more::From;
use jf_merkle_tree::{DigestAlgorithm, ToTraversalPath};
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

/// Custom Keccak256 node for our merkle tree
#[derive(Default, Eq, PartialEq, Clone, Copy, Ord, PartialOrd, Hash)]
pub struct KeccakNode(pub [u8; 32]);

impl fmt::Debug for KeccakNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("KeccakNode")
            .field(&hex::encode(self.0))
            .finish()
    }
}

impl AsRef<[u8]> for KeccakNode {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl CanonicalSerialize for KeccakNode {
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

impl CanonicalDeserialize for KeccakNode {
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

impl Valid for KeccakNode {
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }
}

/// Keccak256 hasher that matches our Solidity implementation
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Keccak256Hasher;

impl DigestAlgorithm<RewardAmount, RewardAccount, KeccakNode> for Keccak256Hasher {
    fn digest(data: &[KeccakNode]) -> Result<KeccakNode, jf_merkle_tree::MerkleTreeError> {
        let mut hasher = Keccak256::new();

        // Hash the concatenated node data directly (no domain separator)
        for node in data {
            hasher.update(node.as_ref());
        }

        let result = hasher.finalize();
        Ok(KeccakNode(result.into()))
    }

    fn digest_leaf(
        _pos: &RewardAccount,
        elem: &RewardAmount,
    ) -> Result<KeccakNode, jf_merkle_tree::MerkleTreeError> {
        // First hash of the value
        let mut hasher = Keccak256::new();
        hasher.update(elem.0.to_be_bytes::<32>()); // 32-byte value as big-endian
        let first_hash = hasher.finalize();

        // Second hash (double hashing)
        let mut hasher = Keccak256::new();
        hasher.update(first_hash);
        let result = hasher.finalize();

        Ok(KeccakNode(result.into()))
    }
}
