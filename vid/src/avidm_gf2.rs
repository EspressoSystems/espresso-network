//! This module implements the AVID-M scheme over GF2

use std::{ops::Range, vec};

use anyhow::anyhow;
use jf_merkle_tree::{hasher::HasherNode, MerkleTreeScheme};
use jf_utils::canonical;
use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::{VidError, VidResult, VidScheme};

/// Namespaced AvidMGF2 scheme
pub mod namespaced;
/// Namespace proofs for AvidMGF2 scheme
pub mod proofs;

/// Merkle tree scheme used in the VID
pub(crate) type MerkleTree =
    jf_merkle_tree::hasher::HasherMerkleTree<sha3::Keccak256, HasherNode<sha3::Keccak256>>;
type MerkleProof = <MerkleTree as MerkleTreeScheme>::MembershipProof;
type MerkleCommit = <MerkleTree as MerkleTreeScheme>::Commitment;

/// Dummy struct for AVID-M scheme over GF2
pub struct AvidMGF2Scheme;

/// VID Parameters
#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvidMGF2Param {
    /// Total weights of all storage nodes
    pub total_weights: usize,
    /// Minimum collective weights required to recover the original payload.
    pub recovery_threshold: usize,
}

impl AvidMGF2Param {
    /// Construct a new [`AvidMGF2Param`].
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
pub struct AvidMGF2Share {
    /// Range of this share in the encoded payload.
    range: Range<usize>,
    /// Actual share content.
    #[serde(with = "canonical")]
    payload: Vec<Vec<u8>>,
    /// Merkle proof of the content.
    #[serde(with = "canonical")]
    mt_proofs: Vec<MerkleProof>,
}

impl AvidMGF2Share {
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
#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvidMGF2Commit {
    /// VID commitment is the Merkle tree root
    pub commit: MerkleCommit,
}

impl AsRef<[u8]> for AvidMGF2Commit {
    fn as_ref(&self) -> &[u8] {
        self.commit.as_ref()
    }
}

impl AvidMGF2Scheme {
    /// Setup an instance for AVID-M scheme
    pub fn setup(recovery_threshold: usize, total_weights: usize) -> VidResult<AvidMGF2Param> {
        AvidMGF2Param::new(recovery_threshold, total_weights)
    }

    fn bit_padding(payload: &[u8], payload_len: usize) -> VidResult<Vec<u8>> {
        if payload_len < payload.len() + 1 {
            return Err(VidError::Argument(
                "Payload length is too large to fit in the given payload length".to_string(),
            ));
        }
        let mut padded = vec![0u8; payload_len];
        padded[..payload.len()].copy_from_slice(payload);
        padded[payload.len()] = 1u8;
        Ok(padded)
    }

    fn raw_disperse(
        param: &AvidMGF2Param,
        payload: &[u8],
    ) -> VidResult<(MerkleTree, Vec<Vec<u8>>)> {
        let original_count = param.recovery_threshold;
        let recovery_count = param.total_weights - param.recovery_threshold;
        // Bit padding, we append an 1u8 to the end of the payload.
        let mut shard_bytes = (payload.len() + 1).div_ceil(original_count);
        if shard_bytes % 2 == 1 {
            shard_bytes += 1;
        }
        let payload = Self::bit_padding(payload, shard_bytes * original_count)?;
        let original = payload
            .chunks(shard_bytes)
            .map(|chunk| chunk.to_owned())
            .collect::<Vec<_>>();
        let recovery = reed_solomon_simd::encode(original_count, recovery_count, &original)?;

        let shares = [original, recovery].concat();
        let share_digests: Vec<_> = shares
            .iter()
            .map(|share| HasherNode::from(sha3::Keccak256::digest(share)))
            .collect();
        let mt = MerkleTree::from_elems(None, &share_digests)?;
        Ok((mt, shares))
    }
}

impl VidScheme for AvidMGF2Scheme {
    type Param = AvidMGF2Param;
    type Share = AvidMGF2Share;
    type Commit = AvidMGF2Commit;

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
        let (mt, shares) = Self::raw_disperse(param, payload)?;
        let commit = AvidMGF2Commit {
            commit: mt.commitment(),
        };
        let ranges: Vec<_> = distribution
            .iter()
            .scan(0, |sum, w| {
                let prefix_sum = *sum;
                *sum += w;
                Some(prefix_sum as usize..*sum as usize)
            })
            .collect();
        let shares: Vec<_> = ranges
            .into_iter()
            .map(|range| AvidMGF2Share {
                range: range.clone(),
                payload: shares[range.clone()].to_vec(),
                // TODO(Chengyu): switch to batch proof generation
                mt_proofs: range
                    .map(|k| {
                        mt.lookup(k as u64)
                            .expect_ok()
                            .expect("MT lookup shouldn't fail")
                            .1
                    })
                    .collect::<Vec<_>>(),
            })
            .collect();
        Ok((commit, shares))
    }

    fn verify_share(
        _param: &Self::Param,
        commit: &Self::Commit,
        share: &Self::Share,
    ) -> VidResult<crate::VerificationResult> {
        if !share.validate() {
            return Err(VidError::InvalidShare);
        }
        for (i, index) in share.range.clone().enumerate() {
            let payload_digest = HasherNode::from(sha3::Keccak256::digest(&share.payload[i]));
            // TODO(Chengyu): switch to batch verification
            if MerkleTree::verify(
                commit.commit,
                index as u64,
                payload_digest,
                &share.mt_proofs[i],
            )?
            .is_err()
            {
                return Ok(Err(()));
            }
        }
        Ok(Ok(()))
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

        let mut decoder = reed_solomon_simd::ReedSolomonDecoder::new(
            original_count,
            recovery_count,
            shard_bytes,
        )?;
        let mut original_shares = vec![None; original_count];
        for share in shares {
            if !share.validate() || share.payload.iter().any(|p| p.len() != shard_bytes) {
                return Err(VidError::InvalidShare);
            }
            for (i, index) in share.range.clone().enumerate() {
                if index < original_count {
                    original_shares[index] = Some(share.payload[i].as_ref());
                    decoder.add_original_shard(index, &share.payload[i])?;
                } else {
                    decoder.add_recovery_shard(index - original_count, &share.payload[i])?;
                }
            }
        }
        let result = decoder.decode()?;
        original_shares
            .iter_mut()
            .enumerate()
            .for_each(|(i, share)| {
                if share.is_none() {
                    *share = result.restored_original(i);
                }
            });
        if original_shares.iter().any(|share| share.is_none()) {
            return Err(VidError::Internal(anyhow!(
                "Failed to recover the payload."
            )));
        }
        let mut recovered: Vec<_> = original_shares
            .into_iter()
            .flat_map(|share| share.unwrap().to_vec())
            .collect();
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
    use rand::{seq::SliceRandom, RngCore};

    use super::AvidMGF2Scheme;
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
            let params = AvidMGF2Scheme::setup(recovery_threshold, total_weights as usize).unwrap();

            for payload_byte_len in payload_byte_lens {
                let payload = {
                    let mut bytes_random = vec![0u8; payload_byte_len];
                    rng.fill_bytes(&mut bytes_random);
                    bytes_random
                };

                let (commit, mut shares) =
                    AvidMGF2Scheme::disperse(&params, &weights, &payload).unwrap();

                assert_eq!(shares.len(), num_storage_nodes);

                // verify shares
                shares.iter().for_each(|share| {
                    assert!(AvidMGF2Scheme::verify_share(&params, &commit, share)
                        .is_ok_and(|r| r.is_ok()))
                });

                // test payload recovery on a random subset of shares
                shares.shuffle(&mut rng);
                let mut cumulated_weights = 0;
                let mut cut_index = 0;
                while cumulated_weights <= recovery_threshold {
                    cumulated_weights += shares[cut_index].weight();
                    cut_index += 1;
                }
                let payload_recovered =
                    AvidMGF2Scheme::recover(&params, &commit, &shares[..cut_index]).unwrap();
                assert_eq!(payload_recovered, payload);
            }
        }
    }
}
