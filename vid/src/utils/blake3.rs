//! BLAKE3-native plumbing for [`jf_merkle_tree::MerkleTree`].
//!
//! `jf_merkle_tree::hasher::GenericHasherMerkleTree<H, ...>` requires
//! `H: digest::Digest` (the RustCrypto trait). The BLAKE3 crate only
//! implements that trait via its `traits-preview` feature, which is pinned
//! to a specific `digest` crate version — it lags behind blake3's release
//! cadence and forces the rest of the workspace onto an old `blake3` line.
//!
//! These helpers let us drive `MerkleTree` directly with `blake3`'s native
//! API. Wire format (32-byte node values, leaf/internal domain separators
//! `b"1"`/`b"0"`) is intentionally byte-identical to
//! `HasherDigestAlgorithm + HasherNode<blake3::Hasher>`, so commitments and
//! proofs do not change.
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, SerializationError, Valid, Validate,
};
use ark_std::io::{Read, Write};
use jf_merkle_tree::{DigestAlgorithm, errors::MerkleTreeError};
use tagged_base64::tagged;

/// Domain separators copied from `jf_merkle_tree::prelude` (which keeps them
/// `pub(crate)`). Must match exactly so commitments stay compatible with
/// trees built by `HasherDigestAlgorithm`.
const LEAF_HASH_DOM_SEP: &[u8; 1] = b"1";
const INTERNAL_HASH_DOM_SEP: &[u8; 1] = b"0";

/// 32-byte BLAKE3 hash, suitable as a `jf_merkle_tree` `NodeValue`.
///
/// Layout and serialization match `jf_merkle_tree::hasher::HasherNode<H>`
/// for any 32-byte hash: 32 raw bytes, no length prefix.
#[derive(Clone, Copy, Debug, Default, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[tagged("HASH")]
#[repr(transparent)]
pub struct Blake3Node([u8; 32]);

impl Blake3Node {
    /// Construct a node directly from its 32-byte payload.
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl From<blake3::Hash> for Blake3Node {
    fn from(h: blake3::Hash) -> Self {
        Self(*h.as_bytes())
    }
}

impl AsRef<[u8]> for Blake3Node {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8; 32]> for Blake3Node {
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

impl CanonicalSerialize for Blake3Node {
    fn serialize_with_mode<W: Write>(
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

impl CanonicalDeserialize for Blake3Node {
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        _compress: Compress,
        _validate: Validate,
    ) -> Result<Self, SerializationError> {
        let mut buf = [0u8; 32];
        reader.read_exact(&mut buf)?;
        Ok(Self(buf))
    }
}

impl Valid for Blake3Node {
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }
}

/// `DigestAlgorithm` for [`Blake3Node`] using `blake3`'s native API.
///
/// Equivalent to `jf_merkle_tree::hasher::HasherDigestAlgorithm` parameterized
/// on `blake3::Hasher`, but does not go through the `digest::Digest` trait.
pub struct Blake3DigestAlgorithm;

impl<E, I> DigestAlgorithm<E, I, Blake3Node> for Blake3DigestAlgorithm
where
    E: jf_merkle_tree::Element + CanonicalSerialize,
    I: jf_merkle_tree::Index + CanonicalSerialize,
{
    fn digest(data: &[Blake3Node]) -> Result<Blake3Node, MerkleTreeError> {
        let mut hasher = blake3::Hasher::new();
        hasher.update(INTERNAL_HASH_DOM_SEP);
        for v in data {
            hasher.update(&v.0);
        }
        Ok(Blake3Node::from(hasher.finalize()))
    }

    fn digest_leaf(pos: &I, elem: &E) -> Result<Blake3Node, MerkleTreeError> {
        let mut hasher = blake3::Hasher::new();
        hasher.update(LEAF_HASH_DOM_SEP);
        let mut adapter = HasherWriter(&mut hasher);
        pos.serialize_uncompressed(&mut adapter)
            .map_err(|_| MerkleTreeError::DigestError("Failed serializing pos".into()))?;
        elem.serialize_uncompressed(&mut adapter)
            .map_err(|_| MerkleTreeError::DigestError("Failed serializing elem".into()))?;
        Ok(Blake3Node::from(hasher.finalize()))
    }
}

/// `ark_std::io::Write` adapter that forwards writes into a `blake3::Hasher`.
///
/// Used for `digest_leaf`, which serializes via `CanonicalSerialize` — that
/// trait writes through `ark_std::io::Write`, which `blake3::Hasher` does
/// not implement directly.
struct HasherWriter<'a>(&'a mut blake3::Hasher);

impl Write for HasherWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> ark_std::io::Result<usize> {
        self.0.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> ark_std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use jf_merkle_tree::{MerkleTreeScheme, append_only::MerkleTree as JfMerkleTree};

    use super::*;

    type Mt = JfMerkleTree<Blake3Node, Blake3DigestAlgorithm, u64, 4, Blake3Node>;

    fn leaf(i: u8) -> Blake3Node {
        let mut buf = [0u8; 32];
        buf[0] = i;
        Blake3Node::new(buf)
    }

    /// Building the same leaves twice yields the same commitment.
    #[test]
    fn deterministic_commitment() {
        let leaves: Vec<Blake3Node> = (0u8..16).map(leaf).collect();
        let a = Mt::from_elems(None, &leaves).unwrap().commitment();
        let b = Mt::from_elems(None, &leaves).unwrap().commitment();
        assert_eq!(a, b);
    }

    /// Round-trip a generated proof through `MerkleTree::verify`.
    #[test]
    fn proof_round_trip() {
        let leaves: Vec<Blake3Node> = (0u8..16).map(leaf).collect();
        let mt = Mt::from_elems(None, &leaves).unwrap();
        let commit = mt.commitment();
        for k in 0u64..16 {
            let (val, proof) = mt.lookup(k).expect_ok().unwrap();
            assert_eq!(*val, leaves[k as usize]);
            assert!(Mt::verify(commit, k, *val, &proof).unwrap().is_ok());
        }
    }

    /// Tampering with a leaf rejects the proof.
    #[test]
    fn proof_rejects_tampered_leaf() {
        let leaves: Vec<Blake3Node> = (0u8..16).map(leaf).collect();
        let mt = Mt::from_elems(None, &leaves).unwrap();
        let commit = mt.commitment();
        let (_, proof) = mt.lookup(0).expect_ok().unwrap();
        let bad = leaf(99);
        assert!(Mt::verify(commit, 0u64, bad, &proof).unwrap().is_err());
    }

    /// CanonicalSerialize round-trip preserves bytes.
    #[test]
    fn canonical_serialize_round_trip() {
        let n = Blake3Node::new([7u8; 32]);
        let mut buf = Vec::new();
        n.serialize_uncompressed(&mut buf).unwrap();
        assert_eq!(buf.len(), 32);
        assert_eq!(buf, vec![7u8; 32]);
        let n2 = Blake3Node::deserialize_uncompressed(&buf[..]).unwrap();
        assert_eq!(n, n2);
    }
}
