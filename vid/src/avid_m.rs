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

use std::ops::Range;

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
            return Err(VidError::Argument("Invalid Parameter.".to_string()));
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
    Radix2EvaluationDomain::<F>::new(domain_size)
        .ok_or_else(|| VidError::Argument("Invalid Param.".to_string()))
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
    /// Let `k = recovery_threshold` and `n = total_weights`. This function
    /// partition the `payload` into many chunks, each containing `k` field
    /// elements. Then each chunk is encoded into `n` field element with Reed
    /// Solomon erasure code. They are then re-organized as `n` vectors, each
    /// collecting one field element from each chunk. These `n` vectors are
    /// then Merklized for commitment and membership proof generation.
    #[allow(clippy::type_complexity)]
    #[inline]
    fn raw_encode(
        recovery_threshold: usize,
        total_weights: usize,
        payload: &[u8],
    ) -> VidResult<(MerkleTree, Vec<Vec<F>>)> {
        // The number of bytes that can be encoded into a single F element.
        let elem_bytes_len = bytes_to_field::elem_byte_capacity::<F>();

        // A "chunk" is a byte slice whose size holds exactly `recovery_threshold`
        // F elements.
        let num_bytes_per_chunk = recovery_threshold * elem_bytes_len;

        let domain = radix2_domain::<F>(total_weights)?; // See docs at `domains`.

        // For each chunk of payload bytes:
        // 1. Convert those bytes to `recovery_threshold` field elements.
        // 2. Evaluate that polynomial at `total_weights` points.
        let encoding_timer = start_timer!(|| "Encoding payload");

        // RS-encode each chunk
        let codewords: Vec<_> = payload
            .par_chunks(num_bytes_per_chunk)
            .map(|chunk| {
                let mut fft_vec: Vec<F> = bytes_to_field::<_, F>(chunk).collect(); // step 1
                domain.fft_in_place(&mut fft_vec); // step 2
                fft_vec.truncate(total_weights); // truncate the useless evaluations
                fft_vec
            })
            .collect();
        // Generate `total_weights` raw shares. Each share collects one field element
        // from each encode chunk.
        let raw_shares: Vec<_> = (0..total_weights)
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
        let mt = MerkleTree::from_elems(None, &compressed_raw_shares)
            .map_err(|err| VidError::Internal(err.into()))?;
        end_timer!(mt_timer);

        Ok((mt, raw_shares))
    }

    pub(crate) fn verify_internal(
        param: &AvidMParam,
        commit: &AvidMCommit,
        share: &RawAvidMShare,
    ) -> VidResult<crate::VerificationResult> {
        if share.range.end > param.total_weights {
            return Err(VidError::Argument("Invalid share".to_string()));
        }
        for (i, index) in share.range.clone().enumerate() {
            let compressed_payload = Config::raw_share_digest(&share.payload[i])?;
            if MerkleTree::verify(
                commit.commit,
                index as u64,
                compressed_payload,
                &share.mt_proofs[i],
            )
            .map_err(|err| VidError::Internal(err.into()))?
            .is_err()
            {
                return Ok(Err(()));
            }
        }
        Ok(Ok(()))
    }
}

impl VidScheme for AvidMScheme {
    type Param = AvidMParam;

    type Share = AvidMShare;

    type Commit = AvidMCommit;

    fn commit(param: &Self::Param, payload: &[u8]) -> VidResult<Self::Commit> {
        let (mt, _) = Self::raw_encode(param.recovery_threshold, param.total_weights, payload)?;
        Ok(AvidMCommit {
            commit: mt.commitment(),
        })
    }

    fn disperse(
        param: &Self::Param,
        distribution: &[u32],
        payload: &[u8],
    ) -> VidResult<(Self::Commit, Vec<Self::Share>)> {
        let payload_byte_len = payload.len();
        let total_weights = distribution.iter().sum::<u32>() as usize;
        if total_weights != param.total_weights {
            return Err(VidError::Argument(
                "Weight distribution is inconsistent with the given param".to_string(),
            ));
        }
        if distribution.iter().any(|&w| w == 0) {
            return Err(VidError::Argument("Weight cannot be zero".to_string()));
        }

        let disperse_timer = start_timer!(|| format!("Disperse {} bytes", payload_byte_len));

        let (mt, raw_shares) = Self::raw_encode(param.recovery_threshold, total_weights, payload)?;

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

        end_timer!(disperse_timer);
        Ok((commit, shares))
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
    /// - `shares.len()` must be at least `recovery_threshold`.
    /// - Each share's `payload` must have equal length.
    ///
    /// Shares beyond `recovery_threshold` are ignored.
    fn recover(
        param: &Self::Param,
        _commit: &Self::Commit,
        shares: &[Self::Share],
    ) -> VidResult<Vec<u8>> {
        let payload_byte_len = shares[0].payload_byte_len;

        let recovery_threshold: usize = param.recovery_threshold;

        // Each share's payload contains some evaluations from `num_polys`
        // polynomials.
        let num_polys = shares[0].content.payload[0].len();
        let total_weights = param.total_weights;
        let collected_weights: usize = shares.iter().map(|s| s.content.range.len()).sum();

        // check args
        if !shares.iter().all(|s| {
            s.content.payload.iter().all(|p| p.len() == num_polys)
                && s.content.range.end <= total_weights
        }) {
            return Err(VidError::Argument("Malformed shares".to_string()));
        }
        if collected_weights < recovery_threshold {
            return Err(VidError::Argument("Insufficient shares.".to_string()));
        }

        let domain = radix2_domain::<F>(total_weights)?;

        // Lagrange interpolation
        // step 1: find all evaluation points
        let x: Vec<F> = shares
            .iter()
            .flat_map(|s| s.content.range.clone().map(|i| domain.element(i)))
            .collect();
        // step 2: interpolate each polynomial
        let recovered = (0..num_polys)
            .into_par_iter()
            .map(|poly_index| {
                let y = shares
                    .iter()
                    .flat_map(|s| {
                        (0..s.content.range.len()).map(|k| s.content.payload[k][poly_index])
                    })
                    .collect::<Vec<F>>();
                jf_utils::reed_solomon_code::reed_solomon_erasure_decode(
                    x.iter().zip(y),
                    recovery_threshold,
                )
                .map_err(|err| VidError::Internal(err.into()))
            })
            .collect::<Result<Vec<Vec<F>>, _>>()?;

        Ok(field_to_bytes(recovered.into_iter().flatten())
            .take(payload_byte_len)
            .collect())
    }
}

/// Unit tests
#[cfg(test)]
pub mod tests {
    use rand::{seq::SliceRandom, RngCore};

    use crate::{avid_m::AvidMScheme, VidScheme};

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
                    assert!(AvidMScheme::verify_share(&params, &commit, share).is_ok())
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

        let (commit, shares) = AvidMScheme::disperse(&params, &weights, &payload).unwrap();

        let recover_timer = start_timer!(|| "Recovery");
        AvidMScheme::recover(&params, &commit, &shares).unwrap();
        end_timer!(recover_timer);
    }
}
