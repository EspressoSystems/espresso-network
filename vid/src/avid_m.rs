//! This module implements the AVID-M scheme, whose name came after the DispersedLedger paper <https://www.usenix.org/conference/nsdi22/presentation/yang>.
//!
//! To disperse a payload to a number of storage nodes according to a weight
//! distribution, the payload is first converted into field elements and then
//! divided into chunks of `k` elements each, and each chunk is then encoded
//! into `n` field elements using Reed Solomon code. The parameter `n` equals to
//! the total weight of all storage nodes, and `k` is the minimum collective
//! weights required to recover the original payload. After the encoding, it can
//! be viewed as `n` vectors of field elements each of length equals to the
//! number of chunks. The VID commitment is obtained by Merklized these `n`
//! vectors. And for dispersal, each storage node gets some vectors and their
//! Merkle proofs according to its weight.

use std::{collections::HashMap, iter, ops::Range};

use ark_ff::PrimeField;
use ark_poly::{EvaluationDomain, Radix2EvaluationDomain};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{end_timer, start_timer};
use config::AvidMConfig;
use jf_merkle_tree::MerkleTreeScheme;
use jf_utils::canonical;
use p3_maybe_rayon::prelude::{
    IntoParallelIterator, IntoParallelRefIterator, ParallelIterator, ParallelSlice,
};
use serde::{Deserialize, Serialize};
use tagged_base64::tagged;

use crate::{
    utils::bytes_to_field::{self, bytes_to_field, field_to_bytes},
    VidError, VidResult, VidScheme,
};

mod config;

pub mod namespaced;
pub mod proofs;

#[cfg(all(not(feature = "sha256"), not(feature = "keccak256")))]
type Config = config::Poseidon2Config;
#[cfg(feature = "sha256")]
type Config = config::Sha256Config;
#[cfg(feature = "keccak256")]
type Config = config::Keccak256Config;

// Type alias for convenience
type F = <Config as AvidMConfig>::BaseField;
type MerkleTree = <Config as AvidMConfig>::MerkleTree;
type MerkleProof = <MerkleTree as MerkleTreeScheme>::MembershipProof;
type MerkleCommit = <MerkleTree as MerkleTreeScheme>::Commitment;

/// Commit type for AVID-M scheme.
#[derive(
    Clone,
    Copy,
    Debug,
    Hash,
    CanonicalSerialize,
    CanonicalDeserialize,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
)]
#[tagged("AvidMCommit")]
#[repr(C)]
pub struct AvidMCommit {
    /// Root commitment of the Merkle tree.
    pub commit: MerkleCommit,
}

impl AsRef<[u8]> for AvidMCommit {
    fn as_ref(&self) -> &[u8] {
        unsafe {
            ::core::slice::from_raw_parts(
                (self as *const Self) as *const u8,
                ::core::mem::size_of::<Self>(),
            )
        }
    }
}

impl AsRef<[u8; 32]> for AvidMCommit {
    fn as_ref(&self) -> &[u8; 32] {
        unsafe { ::core::slice::from_raw_parts((self as *const Self) as *const u8, 32) }
            .try_into()
            .unwrap()
    }
}

/// Share type to be distributed among the parties.
#[derive(Clone, Debug, Hash, Serialize, Deserialize, Eq, PartialEq)]
pub struct RawAvidMShare {
    /// Range of this share in the encoded payload.
    range: Range<usize>,
    /// Actual share content.
    #[serde(with = "canonical")]
    payload: Vec<Vec<F>>,
    /// Merkle proof of the content.
    #[serde(with = "canonical")]
    mt_proofs: Vec<MerkleProof>,
}

/// Share type to be distributed among the parties.
#[derive(Clone, Debug, Hash, Serialize, Deserialize, Eq, PartialEq)]
pub struct AvidMShare {
    /// Index number of the given share.
    index: u32,
    /// The length of payload in bytes.
    payload_byte_len: usize,
    /// Content of this AvidMShare.
    content: RawAvidMShare,
}

/// Public parameters of the AVID-M scheme.
#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvidMParam {
    /// Total weights of all storage nodes
    pub total_weights: usize,
    /// Minimum collective weights required to recover the original payload.
    pub recovery_threshold: usize,
}

impl AvidMParam {
    /// Construct a new [`AvidMParam`].
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

/// Helper: initialize a FFT domain
#[inline]
fn radix2_domain<F: PrimeField>(domain_size: usize) -> VidResult<Radix2EvaluationDomain<F>> {
    Radix2EvaluationDomain::<F>::new(domain_size).ok_or_else(|| VidError::InvalidParam)
}

/// Dummy struct for AVID-M scheme.
pub struct AvidMScheme;

impl AvidMScheme {
    /// Setup an instance for AVID-M scheme
    pub fn setup(recovery_threshold: usize, total_weights: usize) -> VidResult<AvidMParam> {
        AvidMParam::new(recovery_threshold, total_weights)
    }
}

impl AvidMScheme {
    /// Helper function.
    /// Transform the payload bytes into a list of fields elements.
    /// This function also pads the bytes with a 1 in the end, following by many 0's
    /// until the length of the output is a multiple of `param.recovery_threshold`.
    fn pad_to_fields(param: &AvidMParam, payload: &[u8]) -> Vec<F> {
        // The number of bytes that can be encoded into a single F element.
        let elem_bytes_len = bytes_to_field::elem_byte_capacity::<F>();

        // A "chunk" is a byte slice whose size holds exactly `recovery_threshold`
        // F elements.
        let num_bytes_per_chunk = param.recovery_threshold * elem_bytes_len;

        let remainder = (payload.len() + 1) % num_bytes_per_chunk;
        let pad_num_zeros = (num_bytes_per_chunk - remainder) % num_bytes_per_chunk;

        // Pad the payload with a 1 and many 0's.
        bytes_to_field::<_, F>(
            payload
                .iter()
                .chain(iter::once(&1u8))
                .chain(iter::repeat_n(&0u8, pad_num_zeros)),
        )
        .collect()
    }

    /// Helper function.
    /// Let `k = recovery_threshold` and `n = total_weights`. This function
    /// partition the `payload` into many chunks, each containing `k` field
    /// elements. Then each chunk is encoded into `n` field element with Reed
    /// Solomon erasure code. They are then re-organized as `n` vectors, each
    /// collecting one field element from each chunk. These `n` vectors are
    /// then Merklized for commitment and membership proof generation.
    #[allow(clippy::type_complexity)]
    #[inline]
    fn raw_encode(param: &AvidMParam, payload: &[F]) -> VidResult<(MerkleTree, Vec<Vec<F>>)> {
        let domain = radix2_domain::<F>(param.total_weights)?; // See docs at `domains`.

        let encoding_timer = start_timer!(|| "Encoding payload");

        // RS-encode each chunk
        let codewords: Vec<_> = payload
            .par_chunks(param.recovery_threshold)
            .map(|chunk| {
                let mut fft_vec = domain.fft(chunk); // RS-encode the chunk
                fft_vec.truncate(param.total_weights); // truncate the useless evaluations
                fft_vec
            })
            .collect();
        // Generate `total_weights` raw shares. Each share collects one field element
        // from each encode chunk.
        let raw_shares: Vec<_> = (0..param.total_weights)
            .into_par_iter()
            .map(|i| codewords.iter().map(|v| v[i]).collect::<Vec<F>>())
            .collect();
        end_timer!(encoding_timer);

        let hash_timer = start_timer!(|| "Compressing each raw share");
        let compressed_raw_shares = raw_shares
            .par_iter()
            .map(|v| Config::raw_share_digest(v))
            .collect::<Result<Vec<_>, _>>()?;
        end_timer!(hash_timer);

        let mt_timer = start_timer!(|| "Constructing Merkle tree");
        let mt = MerkleTree::from_elems(None, &compressed_raw_shares)?;
        end_timer!(mt_timer);

        Ok((mt, raw_shares))
    }

    /// Short hand for `pad_to_field` and `raw_encode`.
    fn pad_and_encode(param: &AvidMParam, payload: &[u8]) -> VidResult<(MerkleTree, Vec<Vec<F>>)> {
        let payload = Self::pad_to_fields(param, payload);
        Self::raw_encode(param, &payload)
    }

    /// Consume in the constructed Merkle tree and the raw shares from `raw_encode`, provide the AvidM commitment and shares.
    fn distribute_shares(
        param: &AvidMParam,
        distribution: &[u32],
        mt: MerkleTree,
        raw_shares: Vec<Vec<F>>,
        payload_byte_len: usize,
    ) -> VidResult<(AvidMCommit, Vec<AvidMShare>)> {
        // let payload_byte_len = payload.len();
        let total_weights = distribution.iter().sum::<u32>() as usize;
        if total_weights != param.total_weights {
            return Err(VidError::Argument(
                "Weight distribution is inconsistent with the given param".to_string(),
            ));
        }
        if distribution.contains(&0) {
            return Err(VidError::Argument("Weight cannot be zero".to_string()));
        }

        let distribute_timer = start_timer!(|| "Distribute codewords to the storage nodes");
        // Distribute the raw shares to each storage node according to the weight
        // distribution. For each chunk, storage `i` gets `distribution[i]`
        // consecutive raw shares ranging as `ranges[i]`.
        let ranges: Vec<_> = distribution
            .iter()
            .scan(0, |sum, w| {
                let prefix_sum = *sum;
                *sum += w;
                Some(prefix_sum as usize..*sum as usize)
            })
            .collect();
        let shares: Vec<_> = ranges
            .par_iter()
            .map(|range| {
                range
                    .clone()
                    .map(|k| raw_shares[k].to_owned())
                    .collect::<Vec<_>>()
            })
            .collect();
        end_timer!(distribute_timer);

        let mt_proof_timer = start_timer!(|| "Generate Merkle tree proofs");
        let shares = shares
            .into_iter()
            .enumerate()
            .map(|(i, payload)| AvidMShare {
                index: i as u32,
                payload_byte_len,
                content: RawAvidMShare {
                    range: ranges[i].clone(),
                    payload,
                    mt_proofs: ranges[i]
                        .clone()
                        .map(|k| {
                            mt.lookup(k as u64)
                                .expect_ok()
                                .expect("MT lookup shouldn't fail")
                                .1
                        })
                        .collect::<Vec<_>>(),
                },
            })
            .collect::<Vec<_>>();
        end_timer!(mt_proof_timer);

        let commit = AvidMCommit {
            commit: mt.commitment(),
        };

        Ok((commit, shares))
    }

    pub(crate) fn verify_internal(
        param: &AvidMParam,
        commit: &AvidMCommit,
        share: &RawAvidMShare,
    ) -> VidResult<crate::VerificationResult> {
        if share.range.end > param.total_weights || share.range.len() != share.payload.len() {
            return Err(VidError::InvalidShare);
        }
        for (i, index) in share.range.clone().enumerate() {
            let compressed_payload = Config::raw_share_digest(&share.payload[i])?;
            if MerkleTree::verify(
                commit.commit,
                index as u64,
                compressed_payload,
                &share.mt_proofs[i],
            )?
            .is_err()
            {
                return Ok(Err(()));
            }
        }
        Ok(Ok(()))
    }

    pub(crate) fn recover_fields(param: &AvidMParam, shares: &[AvidMShare]) -> VidResult<Vec<F>> {
        let recovery_threshold: usize = param.recovery_threshold;

        // Each share's payload contains some evaluations from `num_polys`
        // polynomials.
        let num_polys = shares
            .iter()
            .find(|s| !s.content.payload.is_empty())
            .ok_or(VidError::Argument("All shares are empty".to_string()))?
            .content
            .payload[0]
            .len();

        let mut raw_shares = HashMap::new();
        for share in shares {
            if share.content.range.len() != share.content.payload.len()
                || share.content.range.end > param.total_weights
            {
                return Err(VidError::InvalidShare);
            }
            for (i, p) in share.content.range.clone().zip(&share.content.payload) {
                if p.len() != num_polys {
                    return Err(VidError::InvalidShare);
                }
                if raw_shares.contains_key(&i) {
                    return Err(VidError::InvalidShare);
                }
                raw_shares.insert(i, p);
                if raw_shares.len() >= recovery_threshold {
                    break;
                }
            }
            if raw_shares.len() >= recovery_threshold {
                break;
            }
        }

        if raw_shares.len() < recovery_threshold {
            return Err(VidError::InsufficientShares);
        }

        let domain = radix2_domain::<F>(param.total_weights)?;

        // Lagrange interpolation
        // step 1: find all evaluation points and their raw shares
        let (x, raw_shares): (Vec<_>, Vec<_>) = raw_shares
            .into_iter()
            .map(|(i, p)| (domain.element(i), p))
            .unzip();
        // step 2: interpolate each polynomial
        Ok((0..num_polys)
            .into_par_iter()
            .map(|poly_index| {
                jf_utils::reed_solomon_code::reed_solomon_erasure_decode(
                    x.iter().zip(raw_shares.iter().map(|p| p[poly_index])),
                    recovery_threshold,
                )
                .map_err(|err| VidError::Internal(err.into()))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect())
    }
}

impl VidScheme for AvidMScheme {
    type Param = AvidMParam;

    type Share = AvidMShare;

    type Commit = AvidMCommit;

    fn commit(param: &Self::Param, payload: &[u8]) -> VidResult<Self::Commit> {
        let (mt, _) = Self::pad_and_encode(param, payload)?;
        Ok(AvidMCommit {
            commit: mt.commitment(),
        })
    }

    fn disperse(
        param: &Self::Param,
        distribution: &[u32],
        payload: &[u8],
    ) -> VidResult<(Self::Commit, Vec<Self::Share>)> {
        let (mt, raw_shares) = Self::pad_and_encode(param, payload)?;
        Self::distribute_shares(param, distribution, mt, raw_shares, payload.len())
    }

    fn verify_share(
        param: &Self::Param,
        commit: &Self::Commit,
        share: &Self::Share,
    ) -> VidResult<crate::VerificationResult> {
        Self::verify_internal(param, commit, &share.content)
    }

    /// Recover payload data from shares.
    ///
    /// # Requirements
    /// - Total weight of all shares must be at least `recovery_threshold`.
    /// - Each share's `payload` must have equal length.
    /// - All shares must be verified under the given commitment.
    ///
    /// Shares beyond `recovery_threshold` are ignored.
    fn recover(
        param: &Self::Param,
        _commit: &Self::Commit,
        shares: &[Self::Share],
    ) -> VidResult<Vec<u8>> {
        let mut bytes: Vec<u8> = field_to_bytes(Self::recover_fields(param, shares)?).collect();
        // Remove the trimming zeros and the last 1 to get the actual payload bytes.
        // See `pad_to_fields`.
        if let Some(pad_index) = bytes.iter().rposition(|&b| b != 0) {
            if bytes[pad_index] == 1u8 {
                bytes.truncate(pad_index);
                return Ok(bytes);
            }
        }
        Err(VidError::Argument(
            "Malformed payload, cannot find the padding position".to_string(),
        ))
    }
}

/// Unit tests
#[cfg(test)]
pub mod tests {
    use rand::{seq::SliceRandom, RngCore};

    use super::F;
    use crate::{avid_m::AvidMScheme, utils::bytes_to_field, VidScheme};

    #[test]
    fn test_padding() {
        let elem_bytes_len = bytes_to_field::elem_byte_capacity::<F>();
        let param = AvidMScheme::setup(2usize, 5usize).unwrap();
        let bytes = vec![2u8; 1];
        let padded = AvidMScheme::pad_to_fields(&param, &bytes);
        assert_eq!(padded.len(), 2usize);
        assert_eq!(padded, [F::from(2u32 + u8::MAX as u32 + 1), F::from(0)]);

        let bytes = vec![2u8; elem_bytes_len * 2];
        let padded = AvidMScheme::pad_to_fields(&param, &bytes);
        assert_eq!(padded.len(), 4usize);
    }

    #[test]
    fn round_trip() {
        // play with these items
        let params_list = [(2, 4), (3, 9), (5, 6), (15, 16)];
        let payload_byte_lens = [1, 31, 32, 500];

        // more items as a function of the above

        let mut rng = jf_utils::test_rng();

        for (recovery_threshold, num_storage_nodes) in params_list {
            let weights: Vec<u32> = (0..num_storage_nodes)
                .map(|_| rng.next_u32() % 5 + 1)
                .collect();
            let total_weights: u32 = weights.iter().sum();
            let params = AvidMScheme::setup(recovery_threshold, total_weights as usize).unwrap();

            for payload_byte_len in payload_byte_lens {
                println!(
                    "recovery_threshold:: {} num_storage_nodes: {} payload_byte_len: {}",
                    recovery_threshold, num_storage_nodes, payload_byte_len
                );
                println!("weights: {:?}", weights);

                let payload = {
                    let mut bytes_random = vec![0u8; payload_byte_len];
                    rng.fill_bytes(&mut bytes_random);
                    bytes_random
                };

                let (commit, mut shares) =
                    AvidMScheme::disperse(&params, &weights, &payload).unwrap();

                assert_eq!(shares.len(), num_storage_nodes);

                // verify shares
                shares.iter().for_each(|share| {
                    assert!(
                        AvidMScheme::verify_share(&params, &commit, share).is_ok_and(|r| r.is_ok())
                    )
                });

                // test payload recovery on a random subset of shares
                shares.shuffle(&mut rng);
                let mut cumulated_weights = 0;
                let mut cut_index = 0;
                while cumulated_weights <= recovery_threshold {
                    cumulated_weights += shares[cut_index].content.range.len();
                    cut_index += 1;
                }
                let payload_recovered =
                    AvidMScheme::recover(&params, &commit, &shares[..cut_index]).unwrap();
                assert_eq!(payload_recovered, payload);
            }
        }
    }

    #[test]
    #[cfg(feature = "print-trace")]
    fn round_trip_breakdown() {
        use ark_std::{end_timer, start_timer};

        let mut rng = jf_utils::test_rng();

        let params = AvidMScheme::setup(50usize, 200usize).unwrap();
        let weights = vec![2u32; 100usize];
        let payload_byte_len = 1024 * 1024 * 32; // 32MB

        let payload = {
            let mut bytes_random = vec![0u8; payload_byte_len];
            rng.fill_bytes(&mut bytes_random);
            bytes_random
        };

        let disperse_timer = start_timer!(|| format!("Disperse {} bytes", payload_byte_len));
        let (commit, shares) = AvidMScheme::disperse(&params, &weights, &payload).unwrap();
        end_timer!(disperse_timer);

        let recover_timer = start_timer!(|| "Recovery");
        AvidMScheme::recover(&params, &commit, &shares).unwrap();
        end_timer!(recover_timer);
    }
}
