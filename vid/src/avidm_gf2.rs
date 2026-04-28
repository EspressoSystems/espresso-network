//! This module implements the AVID-M scheme over GF2

use std::{ops::Range, vec};

use anyhow::anyhow;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use jf_merkle_tree::{MerkleTreeScheme, append_only::MerkleTree as JfMerkleTree};
use jf_utils::canonical;
use p3_maybe_rayon::prelude::*;
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
type MerkleProof = <MerkleTree as MerkleTreeScheme>::MembershipProof;
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
    #[serde(with = "canonical")]
    payload: Vec<Vec<u8>>,
    /// Merkle proof of the content.
    #[serde(with = "canonical")]
    mt_proofs: Vec<MerkleProof>,
}

impl AvidmGf2Share {
    /// Get the weight of this share
    pub fn weight(&self) -> usize {
        self.range.len()
    }

    /// Validate the share structure.
    pub fn validate(&self) -> bool {
        self.payload.len() == self.range.len() && self.mt_proofs.len() == self.range.len()
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

    /// Build the `original_count` original shards directly from `payload`,
    /// applying the AvidM-GF2 bit padding (one `0x01` byte at
    /// `payload.len()` followed by zeros to fill the final shard).
    ///
    /// Writing the chunks straight out avoids allocating an intermediate
    /// `shard_bytes * original_count`-byte buffer just to re-chunk it.
    fn chunk_and_pad(
        payload: &[u8],
        shard_bytes: usize,
        original_count: usize,
    ) -> VidResult<Vec<Vec<u8>>> {
        let padded_len = shard_bytes * original_count;
        if padded_len < payload.len() + 1 {
            return Err(VidError::Argument(
                "Payload length is too large to fit in the given payload length".to_string(),
            ));
        }
        let mut original: Vec<Vec<u8>> = Vec::with_capacity(original_count);
        for i in 0..original_count {
            let start = i * shard_bytes;
            let mut chunk = vec![0u8; shard_bytes];
            if start < payload.len() {
                let end = ((i + 1) * shard_bytes).min(payload.len());
                let take = end - start;
                chunk[..take].copy_from_slice(&payload[start..end]);
                if take < shard_bytes {
                    // Pad byte falls inside this chunk.
                    chunk[take] = 1u8;
                }
            } else if start == payload.len() {
                // Payload ended exactly on a chunk boundary — pad byte is the
                // first byte of this all-zero chunk.
                chunk[0] = 1u8;
            }
            original.push(chunk);
        }
        Ok(original)
    }

    fn raw_disperse(
        param: &AvidmGf2Param,
        payload: &[u8],
    ) -> VidResult<(MerkleTree, Vec<Vec<u8>>)> {
        let original_count = param.recovery_threshold;
        let recovery_count = param.total_weights - param.recovery_threshold;
        let mut shard_bytes = (payload.len() + 1).div_ceil(original_count);
        if shard_bytes % 2 == 1 {
            shard_bytes += 1;
        }
        let original = Self::chunk_and_pad(payload, shard_bytes, original_count)?;
        let recovery = if recovery_count == 0 {
            vec![]
        } else {
            reed_solomon_simd::encode(original_count, recovery_count, &original)?
        };

        let shares = [original, recovery].concat();
        let share_digests: Vec<Blake3Node> = shares
            .par_iter()
            .map(|share| Blake3Node::from(blake3::hash(share)))
            .collect();
        let mt = MerkleTree::from_elems(None, &share_digests)?;
        Ok((mt, shares))
    }
}

impl VidScheme for AvidmGf2Scheme {
    type Param = AvidmGf2Param;
    type Share = AvidmGf2Share;
    type Commit = AvidmGf2Commit;

    fn commit(param: &Self::Param, payload: &[u8]) -> VidResult<Self::Commit> {
        let (mt, _) = Self::raw_disperse(param, payload)?;
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
        // Ranges partition `shares` in order. Consume the owned shares via a
        // single iterator instead of `shares[range].to_vec()`, which
        // heap-clones every Vec<u8> payload and is a large memcpy bill at
        // high num_ns × total_weights.
        let mut shares_iter = shares.into_iter();
        let payloads: Vec<Vec<Vec<u8>>> = ranges
            .iter()
            .map(|range| shares_iter.by_ref().take(range.len()).collect())
            .collect();
        let shares: Vec<_> = ranges
            .into_par_iter()
            .zip(payloads.into_par_iter())
            .map(|(range, payload)| AvidmGf2Share {
                // TODO(Chengyu): switch to batch proof generation
                mt_proofs: range
                    .clone()
                    .map(|k| {
                        mt.lookup(k as u64)
                            .expect_ok()
                            .expect("MT lookup shouldn't fail")
                            .1
                    })
                    .collect::<Vec<_>>(),
                range,
                payload,
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
