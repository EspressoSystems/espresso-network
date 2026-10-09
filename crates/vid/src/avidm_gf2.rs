//! This module implements the AVID-M scheme over GF2

use std::{borrow::Cow, ops::Range, vec};

use anyhow::anyhow;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use jf_merkle_tree::{MerkleTreeScheme, append_only::MerkleTree as JfMerkleTree};
use p3_maybe_rayon::prelude::*;
use reed_solomon_simd::ReedSolomonEncoder;
use serde::{Deserialize, Serialize};
use tagged_base64::tagged;

use crate::{
    VidError, VidResult, VidScheme,
    utils::blake3::{Blake3DigestAlgorithm, Blake3Node},
};

/// Namespaced AvidmGf2 scheme
pub mod namespaced;
/// Namespace proofs for AvidmGf2 scheme
pub mod proofs;

/// Merkle tree scheme used in the VID. Uses BLAKE3 directly via
/// [`Blake3DigestAlgorithm`] rather than going through the
/// `jf_merkle_tree::hasher::HasherDigest` blanket impl, which would pin
/// `blake3` to a `digest 0.10`-compatible release line.
pub(crate) type MerkleTree = JfMerkleTree<Blake3Node, Blake3DigestAlgorithm, u64, 4, Blake3Node>;
/// Membership proof of the VID Merkle tree, one per shard of a share.
pub type MerkleProof = <MerkleTree as MerkleTreeScheme>::MembershipProof;
type MerkleCommit = <MerkleTree as MerkleTreeScheme>::Commitment;

/// Dummy struct for AVID-M scheme over GF2
pub struct AvidmGf2Scheme;

/// VID Parameters
#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvidmGf2Param {
    /// Total weights of all storage nodes
    pub total_weights: usize,
    /// Minimum collective weights required to recover the original payload.
    pub recovery_threshold: usize,
}

impl AvidmGf2Param {
    /// Construct a new [`AvidmGf2Param`].
    pub fn new(recovery_threshold: usize, total_weights: usize) -> VidResult<Self> {
        if recovery_threshold == 0 || total_weights < recovery_threshold {
            return Err(VidError::InvalidParam);
        }
        Ok(Self {
            total_weights,
            recovery_threshold,
        })
    }
}

/// VID Share type to be distributed among the parties.
#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvidmGf2Share {
    /// Range of this share in the encoded payload.
    range: Range<usize>,
    /// Actual share content.
    #[serde(with = "nested_bytes")]
    payload: Vec<Vec<u8>>,
    /// Merkle proof of the content.
    mt_proofs: Vec<MerkleProof>,
}

/// Optimised serialisation of a sequence of `Vec<u8>`s using `serde_bytes`.
mod nested_bytes {
    use serde::{Deserialize, Deserializer, Serializer, ser::SerializeSeq};
    use serde_bytes::{ByteBuf, Bytes};

    pub fn serialize<S>(v: &[Vec<u8>], s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = s.serialize_seq(Some(v.len()))?;
        for inner in v {
            seq.serialize_element(Bytes::new(inner))?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Vec<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v: Vec<ByteBuf> = Deserialize::deserialize(d)?;
        Ok(v.into_iter().map(ByteBuf::into_vec).collect())
    }
}

impl AvidmGf2Share {
    /// Get the weight of this share
    pub fn weight(&self) -> usize {
        self.range.len()
    }

    /// Range of this share in the encoded payload.
    pub fn range(&self) -> &Range<usize> {
        &self.range
    }

    /// One shard of raw bytes per index in [`Self::range`].
    pub fn payload(&self) -> &[Vec<u8>] {
        &self.payload
    }

    /// One Merkle proof per shard of [`Self::payload`].
    pub fn mt_proofs(&self) -> &[MerkleProof] {
        &self.mt_proofs
    }

    /// Validate the share structure.
    pub fn validate(&self) -> bool {
        self.payload.len() == self.range.len() && self.mt_proofs.len() == self.range.len()
    }

    /// The size every shard of this share has, or `None` if the share is
    /// empty or its shards disagree. A dispersal gives all its shards the
    /// size [`AvidmGf2Scheme::shard_bytes`] computes for its payload.
    pub fn shard_bytes(&self) -> Option<usize> {
        let first = self.payload.first()?.len();
        self.payload
            .iter()
            .all(|shard| shard.len() == first)
            .then_some(first)
    }
}

/// VID Commitment type
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Hash,
    CanonicalSerialize,
    CanonicalDeserialize,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
)]
#[tagged("AvidmGf2Commit")]
#[repr(C)]
pub struct AvidmGf2Commit {
    /// VID commitment is the Merkle tree root
    pub commit: MerkleCommit,
}

impl AsRef<[u8]> for AvidmGf2Commit {
    fn as_ref(&self) -> &[u8] {
        self.commit.as_ref()
    }
}

impl AsRef<[u8; 32]> for AvidmGf2Commit {
    fn as_ref(&self) -> &[u8; 32] {
        <Self as AsRef<[u8]>>::as_ref(self)
            .try_into()
            .expect("AvidmGf2Commit is always 32 bytes")
    }
}

impl AvidmGf2Scheme {
    /// Setup an instance for AVID-M scheme
    pub fn setup(recovery_threshold: usize, total_weights: usize) -> VidResult<AvidmGf2Param> {
        AvidmGf2Param::new(recovery_threshold, total_weights)
    }

    /// Size in bytes of every shard of a `payload_byte_len`-byte payload
    /// dispersed into `original_count` original shards: the fewest bytes that
    /// hold the payload plus its pad byte, rounded up to the even size
    /// `reed_solomon_simd` requires.
    ///
    /// `None` if `original_count` is zero or the size overflows, so a verifier
    /// can evaluate it on an untrusted length without panicking.
    pub fn shard_bytes(payload_byte_len: usize, original_count: usize) -> Option<usize> {
        if original_count == 0 {
            return None;
        }
        payload_byte_len
            .checked_add(1)?
            .div_ceil(original_count)
            .checked_next_multiple_of(2)
    }

    fn payload_shard_bytes(param: &AvidmGf2Param, payload_len: usize) -> VidResult<usize> {
        Self::shard_bytes(payload_len, param.recovery_threshold).ok_or_else(|| {
            VidError::Argument("Payload length is too large to disperse".to_string())
        })
    }

    /// Build the `original_count` original shards from `payload`, applying
    /// the AvidM-GF2 bit padding (one `0x01` byte at `payload.len()` followed
    /// by zeros to fill the final shard).
    ///
    /// Full shards borrow from `payload`; only shards touching the padding are allocated.
    fn chunk_and_pad(
        payload: &[u8],
        shard_bytes: usize,
        original_count: usize,
    ) -> VidResult<Vec<Cow<'_, [u8]>>> {
        let padded_len = shard_bytes * original_count;
        if padded_len < payload.len() + 1 {
            return Err(VidError::Argument(
                "Payload length is too large to fit in the given payload length".to_string(),
            ));
        }
        let mut original: Vec<Cow<[u8]>> = Vec::with_capacity(original_count);
        for i in 0..original_count {
            let start = i * shard_bytes;
            let end = start + shard_bytes;
            if end <= payload.len() {
                original.push(Cow::Borrowed(&payload[start..end]));
                continue;
            }
            let mut chunk = vec![0u8; shard_bytes];
            if start < payload.len() {
                let take = payload.len() - start;
                chunk[..take].copy_from_slice(&payload[start..]);
                // Pad byte falls inside this chunk.
                chunk[take] = 1u8;
            } else if start == payload.len() {
                // Payload ended exactly on a chunk boundary, pad byte is the
                // first byte of this all-zero chunk.
                chunk[0] = 1u8;
            }
            original.push(Cow::Owned(chunk));
        }
        Ok(original)
    }

    /// Erasure-code the padded `original` shards, returning only the recovery shards.
    fn encode_recovery(
        param: &AvidmGf2Param,
        original: &[Cow<'_, [u8]>],
    ) -> VidResult<Vec<Vec<u8>>> {
        let recovery_count = param.total_weights - param.recovery_threshold;
        if recovery_count == 0 {
            return Ok(vec![]);
        }
        Ok(reed_solomon_simd::encode(
            param.recovery_threshold,
            recovery_count,
            original,
        )?)
    }

    fn merkle_tree<T: AsRef<[u8]> + Sync>(shards: &[T]) -> VidResult<MerkleTree> {
        let share_digests: Vec<Blake3Node> = shards
            .par_iter()
            .map(|share| Blake3Node::from(blake3::hash(share.as_ref())))
            .collect();
        Ok(MerkleTree::from_elems(None, &share_digests)?)
    }

    /// Merkle tree over all shards of `payload`, hashing recovery shards in place in the encoder.
    fn raw_commit(param: &AvidmGf2Param, payload: &[u8]) -> VidResult<MerkleTree> {
        let original_count = param.recovery_threshold;
        let recovery_count = param.total_weights - original_count;
        let shard_bytes = Self::payload_shard_bytes(param, payload.len())?;
        let original = Self::chunk_and_pad(payload, shard_bytes, original_count)?;
        if recovery_count == 0 {
            return Self::merkle_tree(&original);
        }
        let mut encoder = ReedSolomonEncoder::new(original_count, recovery_count, shard_bytes)?;
        for shard in &original {
            encoder.add_original_shard(shard)?;
        }
        let result = encoder.encode()?;
        let shards: Vec<&[u8]> = original
            .iter()
            .map(AsRef::as_ref)
            .chain(result.recovery_iter())
            .collect();
        Self::merkle_tree(&shards)
    }

    fn raw_disperse(
        param: &AvidmGf2Param,
        payload: &[u8],
    ) -> VidResult<(MerkleTree, Vec<Vec<u8>>)> {
        let shard_bytes = Self::payload_shard_bytes(param, payload.len())?;
        let original = Self::chunk_and_pad(payload, shard_bytes, param.recovery_threshold)?;
        let recovery = Self::encode_recovery(param, &original)?;

        let mut shares: Vec<Vec<u8>> = original.into_iter().map(Cow::into_owned).collect();
        shares.extend(recovery);
        let mt = Self::merkle_tree(&shares)?;
        Ok((mt, shares))
    }

    /// Test-only: disperse `payload` but corrupt the erasure-coded (recovery)
    /// shards, so the committed shard set is **not** a valid codeword. Every
    /// returned share still verifies against the returned commitment — its
    /// merkle proofs are genuine — yet recovering from any threshold-covering
    /// subset and re-committing yields a *different* commitment. This models a
    /// Byzantine disperser that commits to a non-codeword, exercising the
    /// unrecoverable reconstruction path.
    ///
    /// Requires `recovery_threshold < total_weights` so recovery shards exist
    /// to corrupt.
    #[cfg(any(test, feature = "testing"))]
    pub fn disperse_non_codeword(
        param: &AvidmGf2Param,
        distribution: &[u32],
        payload: &[u8],
    ) -> VidResult<(AvidmGf2Commit, Vec<AvidmGf2Share>)> {
        let total_weights = distribution.iter().map(|&w| w as usize).sum::<usize>();
        if total_weights != param.total_weights {
            return Err(VidError::Argument(
                "Weight distribution is inconsistent with the given param".to_string(),
            ));
        }
        if distribution.contains(&0u32) {
            return Err(VidError::Argument("Weight cannot be zero".to_string()));
        }
        let original_count = param.recovery_threshold;
        let (_, mut shards) = Self::raw_disperse(param, payload)?;
        if shards.len() <= original_count {
            return Err(VidError::Argument(
                "Payload has no recovery shards to corrupt".to_string(),
            ));
        }
        // Flip a byte in every recovery shard. The original shards are
        // untouched, so each shard still verifies against the rebuilt tree, but
        // the recovery shards no longer match the Reed-Solomon encoding of the
        // originals: the committed set is not a codeword and cannot re-commit.
        for shard in &mut shards[original_count..] {
            shard[0] ^= 0xff;
        }
        let share_digests: Vec<Blake3Node> = shards
            .iter()
            .map(|shard| Blake3Node::from(blake3::hash(shard)))
            .collect();
        let mt = MerkleTree::from_elems(None, &share_digests)?;
        let commit = AvidmGf2Commit {
            commit: mt.commitment(),
        };
        let ranges: Vec<_> = distribution
            .iter()
            .scan(0usize, |sum, w| {
                let prefix_sum = *sum;
                *sum += *w as usize;
                Some(prefix_sum..*sum)
            })
            .collect();
        let mut shards_iter = shards.into_iter();
        let payloads: Vec<Vec<Vec<u8>>> = ranges
            .iter()
            .map(|range| shards_iter.by_ref().take(range.len()).collect())
            .collect();
        let mut proofs_iter = mt
            .collect_leaves_with_proofs()
            .into_iter()
            .map(|(_, _, proof)| proof);
        let proof_groups: Vec<Vec<MerkleProof>> = ranges
            .iter()
            .map(|range| proofs_iter.by_ref().take(range.len()).collect())
            .collect();
        let shares: Vec<_> = ranges
            .into_iter()
            .zip(payloads)
            .zip(proof_groups)
            .map(|((range, payload), mt_proofs)| AvidmGf2Share {
                range,
                payload,
                mt_proofs,
            })
            .collect();
        Ok((commit, shares))
    }
}

impl VidScheme for AvidmGf2Scheme {
    type Param = AvidmGf2Param;
    type Share = AvidmGf2Share;
    type Commit = AvidmGf2Commit;

    fn commit(param: &Self::Param, payload: &[u8]) -> VidResult<Self::Commit> {
        let mt = Self::raw_commit(param, payload)?;
        Ok(Self::Commit {
            commit: mt.commitment(),
        })
    }

    fn disperse(
        param: &Self::Param,
        distribution: &[u32],
        payload: &[u8],
    ) -> VidResult<(Self::Commit, Vec<Self::Share>)> {
        let total_weights = distribution.iter().map(|&w| w as usize).sum::<usize>();
        if total_weights != param.total_weights {
            return Err(VidError::Argument(
                "Weight distribution is inconsistent with the given param".to_string(),
            ));
        }
        if distribution.contains(&0u32) {
            return Err(VidError::Argument("Weight cannot be zero".to_string()));
        }
        let (mt, shares) = Self::raw_disperse(param, payload)?;
        let commit = AvidmGf2Commit {
            commit: mt.commitment(),
        };

        let ranges: Vec<_> = distribution
            .iter()
            .scan(0usize, |sum, w| {
                let prefix_sum = *sum;
                *sum += *w as usize;
                Some(prefix_sum..*sum)
            })
            .collect();
        // Ranges partition `shares` and `proofs` in order. Consume both via
        // owning iterators instead of `shares[range].to_vec()` /
        // `proofs[range].to_vec()`, which would heap-clone every Vec<u8>
        // payload and every per-leaf proof at high num_ns × total_weights.
        //
        // `mt.collect_leaves_with_proofs()` returns leaves in ascending
        // position order (DFS over children 0..ARITY), so we can drain the
        // iterator directly without an indexed placeholder Vec.
        let mut shares_iter = shares.into_iter();
        let payloads: Vec<Vec<Vec<u8>>> = ranges
            .iter()
            .map(|range| shares_iter.by_ref().take(range.len()).collect())
            .collect();
        let mut proofs_iter = mt
            .collect_leaves_with_proofs()
            .into_iter()
            .map(|(_, _, proof)| proof);
        let proof_groups: Vec<Vec<MerkleProof>> = ranges
            .iter()
            .map(|range| proofs_iter.by_ref().take(range.len()).collect())
            .collect();
        // The map body is just a struct construction over already-prepared
        // owned components — sub-µs per item, smaller than rayon's
        // per-item scheduling overhead. Stay sequential.
        let shares: Vec<_> = ranges
            .into_iter()
            .zip(payloads)
            .zip(proof_groups)
            .map(|((range, payload), mt_proofs)| AvidmGf2Share {
                range,
                payload,
                mt_proofs,
            })
            .collect();
        Ok((commit, shares))
    }

    fn verify_share(
        param: &Self::Param,
        commit: &Self::Commit,
        share: &Self::Share,
    ) -> VidResult<crate::VerificationResult> {
        if !share.validate() || share.range.is_empty() || share.range.end > param.total_weights {
            return Err(VidError::InvalidShare);
        }
        // Each (i, leaf, proof) triple is independent. `find_any` short-
        // circuits on the first failing position and avoids allocating any
        // intermediate collection.
        let start = share.range.start;
        let len = share.range.end - start;
        match (0..len)
            .into_par_iter()
            .map(|i| -> VidResult<crate::VerificationResult> {
                let payload_digest = Blake3Node::from(blake3::hash(&share.payload[i]));
                MerkleTree::verify(
                    commit.commit,
                    (start + i) as u64,
                    payload_digest,
                    &share.mt_proofs[i],
                )
                .map_err(VidError::from)
            })
            .find_any(|r| !matches!(r, Ok(Ok(()))))
        {
            None => Ok(Ok(())),
            Some(Ok(v)) => Ok(v),
            Some(Err(e)) => Err(e),
        }
    }

    fn recover(
        param: &Self::Param,
        _commit: &Self::Commit,
        shares: &[Self::Share],
    ) -> VidResult<Vec<u8>> {
        let original_count = param.recovery_threshold;
        let recovery_count = param.total_weights - param.recovery_threshold;
        // Find the first non-empty share
        let Some(first_share) = shares.iter().find(|s| !s.payload.is_empty()) else {
            return Err(VidError::InsufficientShares);
        };
        let shard_bytes = first_share.payload[0].len();

        // Track references to input original shards; avoids the per-shard
        // `.clone()` the previous version did to populate a
        // `Vec<Option<Vec<u8>>>`. Reconstructed shards come from the decoder
        // and are copied directly into the output buffer below.
        let mut input_orig: Vec<Option<&[u8]>> = vec![None; original_count];

        let mut recovered: Vec<u8> = Vec::with_capacity(original_count * shard_bytes);
        if recovery_count == 0 {
            // Edge case where there are no recovery shares: every original must
            // be supplied as input.
            for share in shares {
                if !share.validate() || share.payload.iter().any(|p| p.len() != shard_bytes) {
                    return Err(VidError::InvalidShare);
                }
                for (i, index) in share.range.clone().enumerate() {
                    if index < original_count {
                        input_orig[index] = Some(&share.payload[i]);
                    }
                }
            }
            for slot in &input_orig {
                let shard = slot
                    .ok_or_else(|| VidError::Internal(anyhow!("Failed to recover the payload.")))?;
                recovered.extend_from_slice(shard);
            }
        } else {
            let mut decoder = reed_solomon_simd::ReedSolomonDecoder::new(
                original_count,
                recovery_count,
                shard_bytes,
            )?;
            for share in shares {
                if !share.validate() || share.payload.iter().any(|p| p.len() != shard_bytes) {
                    return Err(VidError::InvalidShare);
                }
                for (i, index) in share.range.clone().enumerate() {
                    let shard = &share.payload[i];
                    if index < original_count {
                        input_orig[index] = Some(shard);
                        decoder.add_original_shard(index, shard)?;
                    } else {
                        decoder.add_recovery_shard(index - original_count, shard)?;
                    }
                }
            }

            let result = decoder.decode()?;
            for (i, shard) in input_orig.iter().enumerate().take(original_count) {
                let shard: &[u8] = match shard {
                    Some(data) => data,
                    None => result.restored_original(i).ok_or_else(|| {
                        VidError::Internal(anyhow!("Failed to recover the payload."))
                    })?,
                };
                recovered.extend_from_slice(shard);
            }
        }
        match recovered.iter().rposition(|&b| b != 0) {
            Some(pad_index) if recovered[pad_index] == 1u8 => {
                recovered.truncate(pad_index);
                Ok(recovered)
            },
            _ => Err(VidError::Argument(
                "Malformed payload, cannot find the padding position".to_string(),
            )),
        }
    }
}

/// Unit tests
#[cfg(test)]
pub mod tests {
    use rand::{RngCore, seq::SliceRandom};

    use super::AvidmGf2Scheme;
    use crate::VidScheme;

    /// Digest over the commitment and every share's range and shards.
    fn output_digest(rt: usize, tw: usize, len: usize) -> String {
        let payload: Vec<u8> = (0..len).map(|i| (i * 31 + 7) as u8).collect();
        let param = AvidmGf2Scheme::setup(rt, tw).unwrap();
        let commit = AvidmGf2Scheme::commit(&param, &payload).unwrap();
        let distribution: Vec<u32> = (0..tw).map(|_| 1).collect();
        let (dcommit, shares) = AvidmGf2Scheme::disperse(&param, &distribution, &payload).unwrap();
        assert_eq!(commit, dcommit);
        let mut hasher = blake3::Hasher::new();
        hasher.update(commit.as_ref());
        for share in &shares {
            hasher.update(&share.range().start.to_le_bytes());
            hasher.update(&share.range().end.to_le_bytes());
            for shard in share.payload() {
                hasher.update(&(shard.len() as u64).to_le_bytes());
                hasher.update(shard);
            }
        }
        hasher.finalize().to_hex().to_string()
    }

    #[test]
    fn outputs_are_pinned() {
        let mut out = String::new();
        for (rt, tw) in [(1, 1), (1, 3), (3, 10), (4, 12), (5, 5), (7, 20)] {
            for len in [0, 1, 31, 32, 33, 63, 64, 65, 1000, 4097, 100_000] {
                out += &format!("{rt} {tw} {len} {}\n", output_digest(rt, tw, len));
            }
        }
        let got = blake3::hash(out.as_bytes()).to_hex().to_string();
        assert_eq!(
            got, "3c96b583adfdf90d15d80c117f9ab76625995e851743204c4e9c5725b5b7950a",
            "{out}"
        );
    }

    #[test]
    fn round_trip() {
        // play with these items
        let num_storage_nodes_list = [4, 9, 16];
        let payload_byte_lens = [1, 31, 32, 500];

        // more items as a function of the above

        let mut rng = jf_utils::test_rng();

        for num_storage_nodes in num_storage_nodes_list {
            let weights: Vec<u32> = (0..num_storage_nodes)
                .map(|_| rng.next_u32() % 5 + 1)
                .collect();
            let total_weights: u32 = weights.iter().sum();
            let recovery_threshold = total_weights.div_ceil(3) as usize;
            let params = AvidmGf2Scheme::setup(recovery_threshold, total_weights as usize).unwrap();

            for payload_byte_len in payload_byte_lens {
                let payload = {
                    let mut bytes_random = vec![0u8; payload_byte_len];
                    rng.fill_bytes(&mut bytes_random);
                    bytes_random
                };

                let (commit, mut shares) =
                    AvidmGf2Scheme::disperse(&params, &weights, &payload).unwrap();

                assert_eq!(shares.len(), num_storage_nodes);

                // verify shares
                shares.iter().for_each(|share| {
                    assert!(
                        AvidmGf2Scheme::verify_share(&params, &commit, share)
                            .is_ok_and(|r| r.is_ok())
                    )
                });

                // test payload recovery on a random subset of shares
                shares.shuffle(&mut rng);
                let mut cumulated_weights = 0;
                let mut cut_index = 0;
                while cumulated_weights < recovery_threshold {
                    cumulated_weights += shares[cut_index].weight();
                    cut_index += 1;
                }
                let payload_recovered =
                    AvidmGf2Scheme::recover(&params, &commit, &shares[..cut_index]).unwrap();
                assert_eq!(payload_recovered, payload);
            }
        }
    }

    #[test]
    fn round_trip_edge_case() {
        // play with these items
        let num_storage_nodes_list = [4, 9, 16];
        let payload_byte_lens = [1, 31, 32, 500];

        // more items as a function of the above

        let mut rng = jf_utils::test_rng();

        for num_storage_nodes in num_storage_nodes_list {
            let weights: Vec<u32> = (0..num_storage_nodes)
                .map(|_| rng.next_u32() % 5 + 1)
                .collect();
            let total_weights: u32 = weights.iter().sum();
            let recovery_threshold = total_weights as usize;
            let params = AvidmGf2Scheme::setup(recovery_threshold, total_weights as usize).unwrap();

            for payload_byte_len in payload_byte_lens {
                let payload = {
                    let mut bytes_random = vec![0u8; payload_byte_len];
                    rng.fill_bytes(&mut bytes_random);
                    bytes_random
                };

                let (commit, mut shares) =
                    AvidmGf2Scheme::disperse(&params, &weights, &payload).unwrap();

                assert_eq!(shares.len(), num_storage_nodes);

                // verify shares
                shares.iter().for_each(|share| {
                    assert!(
                        AvidmGf2Scheme::verify_share(&params, &commit, share)
                            .is_ok_and(|r| r.is_ok())
                    )
                });

                // test payload recovery on a random subset of shares
                shares.shuffle(&mut rng);
                let payload_recovered =
                    AvidmGf2Scheme::recover(&params, &commit, &shares[..]).unwrap();
                assert_eq!(payload_recovered, payload);
            }
        }
    }

    /// `shard_bytes` is what dispersal actually produces, for every shard of
    /// every share, across payload lengths on both sides of a shard boundary
    /// and on both sides of the even rounding.
    #[test]
    fn shard_bytes_matches_dispersal() {
        let total_weights = 10usize;
        let recovery_threshold = 4;
        let params = AvidmGf2Scheme::setup(recovery_threshold, total_weights).unwrap();
        let weights = vec![2u32; 5];

        for payload_byte_len in [0usize, 1, 3, 4, 7, 8, 31, 32, 100, 101] {
            let payload = vec![1u8; payload_byte_len];
            let expected =
                AvidmGf2Scheme::shard_bytes(payload_byte_len, recovery_threshold).unwrap();
            assert_eq!(expected % 2, 0, "{payload_byte_len}");
            assert!(
                expected * recovery_threshold > payload_byte_len,
                "{payload_byte_len}: shards must hold the payload and its pad byte"
            );
            assert!(
                (expected - 2) * recovery_threshold <= payload_byte_len,
                "{payload_byte_len}: shards must not be larger than needed"
            );
            let (_, shares) = AvidmGf2Scheme::disperse(&params, &weights, &payload).unwrap();
            for share in &shares {
                assert_eq!(share.shard_bytes(), Some(expected), "{payload_byte_len}");
            }
        }

        // Total on inputs dispersal never sees.
        assert_eq!(AvidmGf2Scheme::shard_bytes(100, 0), None);
        assert_eq!(AvidmGf2Scheme::shard_bytes(usize::MAX, 1), None);
        assert_eq!(AvidmGf2Scheme::shard_bytes(usize::MAX - 1, 1), None);
    }

    #[test]
    fn disperse_rejects_inconsistent_distribution() {
        let total_weights = 10usize;
        let recovery_threshold = 4;
        let params = AvidmGf2Scheme::setup(recovery_threshold, total_weights).unwrap();
        let payload = vec![1u8; 100];

        // distribution sums to 12, but param says total_weights=10
        let bad_weights = vec![3u32; 4];
        assert!(
            AvidmGf2Scheme::disperse(&params, &bad_weights, &payload).is_err(),
            "disperse should reject distribution that doesn't sum to total_weights"
        );

        // distribution contains a zero weight
        let zero_weight = vec![0u32, 5, 5];
        assert!(
            AvidmGf2Scheme::disperse(&params, &zero_weight, &payload).is_err(),
            "disperse should reject zero-weight entries"
        );

        // correct distribution should succeed
        let good_weights = vec![2u32; 5];
        assert!(AvidmGf2Scheme::disperse(&params, &good_weights, &payload).is_ok());
    }

    #[test]
    fn verify_share_rejects_out_of_range() {
        let total_weights = 10usize;
        let recovery_threshold = 4;
        let params = AvidmGf2Scheme::setup(recovery_threshold, total_weights).unwrap();
        let payload = vec![1u8; 100];
        let weights = vec![2u32; 5];

        let (commit, shares) = AvidmGf2Scheme::disperse(&params, &weights, &payload).unwrap();

        // valid shares pass
        for share in &shares {
            assert!(AvidmGf2Scheme::verify_share(&params, &commit, share).is_ok_and(|r| r.is_ok()));
        }

        // a share verified against a smaller param should be rejected
        let smaller_params = AvidmGf2Scheme::setup(2, 5).unwrap();
        let last_share = shares.last().unwrap();
        assert!(
            AvidmGf2Scheme::verify_share(&smaller_params, &commit, last_share).is_err(),
            "verify_share should reject share with range.end > param.total_weights"
        );
    }
}
