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

use std::{
    collections::{BTreeMap, HashMap},
    iter,
    ops::Range,
};

use ark_ff::{batch_inversion, AdditiveGroup, Field, PrimeField};
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
    Default,
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

/// Compute E(ω^j) for each j ∈ received, where E is the erasure locator.
/// Uses E(ω^j) = N·ω^{-j} / R'(ω^j) with R'(ω^j) = ∏_{i∈R,i≠j}(ω^j - ω^i).
/// O(k²) where k = |received|.
fn compute_e_evals_at_received<F: PrimeField>(
    domain: &Radix2EvaluationDomain<F>,
    received: &[usize],
) -> Vec<F> {
    let n_fft = domain.size();
    let n_field = domain.size_as_field_element();

    // For each j ∈ R, compute R'(ω^j) = ∏_{i∈R, i≠j}(ω^j - ω^i)
    let mut r_prime_at_received: Vec<F> = received
        .iter()
        .map(|&j| {
            let omega_j = domain.element(j);
            received
                .iter()
                .filter(|&&i| i != j)
                .map(|&i| omega_j - domain.element(i))
                .product()
        })
        .collect();

    // batch invert R'(ω^j)
    batch_inversion(&mut r_prime_at_received);

    // E(ω^j) = N · ω^{-j} / R'(ω^j)
    received
        .iter()
        .zip(r_prime_at_received)
        .map(|(&j, inv_r_prime)| {
            let omega_neg_j = domain.element(if j == 0 { 0 } else { n_fft - j });
            n_field * omega_neg_j * inv_r_prime
        })
        .collect()
}

/// Compute the formal derivative of a polynomial in coefficient form.
/// `D(∑ aᵢxⁱ) = ∑ i·aᵢx^{i-1}`
fn formal_derivative<F: Field>(coeffs: &[F]) -> Vec<F> {
    if coeffs.len() <= 1 {
        return vec![F::ZERO];
    }
    coeffs[1..]
        .iter()
        .enumerate()
        .map(|(i, &c)| c * F::from((i + 1) as u64))
        .collect()
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
    /// Strip the `0x01` padding marker and trailing zeros from recovered field bytes.
    /// Inverse of `pad_to_fields`.
    fn unpad_recovered_bytes(fields: Vec<F>) -> VidResult<Vec<u8>> {
        let mut bytes: Vec<u8> = field_to_bytes(fields).collect();
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
        if distribution.contains(&0u32) {
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
        if share.range.end > param.total_weights
            || share.range.len() != share.payload.len()
            || share.range.len() != share.mt_proofs.len()
        {
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

    /// Collect and validate raw shares from AvidM shares, returning the number
    /// of polynomials and a map from evaluation index to share data.
    fn collect_raw_shares<'a>(
        param: &AvidMParam,
        shares: &'a [AvidMShare],
    ) -> VidResult<(usize, BTreeMap<usize, &'a Vec<F>>)> {
        let recovery_threshold = param.recovery_threshold;

        let num_polys = shares
            .iter()
            .find(|s| !s.content.payload.is_empty())
            .ok_or(VidError::Argument("All shares are empty".to_string()))?
            .content
            .payload[0]
            .len();

        let mut raw_shares = BTreeMap::new();
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

        Ok((num_polys, raw_shares))
    }

    pub(crate) fn recover_fields(param: &AvidMParam, shares: &[AvidMShare]) -> VidResult<Vec<F>> {
        let recovery_threshold: usize = param.recovery_threshold;
        let (num_polys, raw_shares) = Self::collect_raw_shares(param, shares)?;

        let domain = radix2_domain::<F>(param.total_weights)?;
        let n_fft = domain.size();

        // Determine received (R) and erased (Ω) index sets.
        // Ω includes both missing shares within 0..total_weights and unused
        // domain points total_weights..n_fft.
        let received: Vec<usize> = raw_shares.keys().copied().collect();
        let received_set: std::collections::HashSet<usize> = received.iter().copied().collect();
        let erased: Vec<usize> = (0..n_fft).filter(|i| !received_set.contains(i)).collect();

        // === One-time setup (shared across all polynomials) ===
        // Directly compute E's evaluations at received points via the product
        // formula, then IFFT to get E in coefficient form. This avoids building
        // the received locator R(x) in coefficient form entirely.
        let setup_timer = start_timer!(|| "Erasure locator setup");

        // 1. Compute E's evaluations at all N domain points — O(k²)
        //    E(ω^j) = 0 for j ∈ Ω (by definition)
        //    E(ω^j) = N·ω^{-j}/R'(ω^j) for j ∈ R (from x^N-1 = E·R identity)
        let e_at_received = compute_e_evals_at_received(&domain, &received);
        let mut e_evals = vec![F::ZERO; n_fft];
        for (idx, &j) in received.iter().enumerate() {
            e_evals[j] = e_at_received[idx];
        }

        // 2. IFFT E_evals → E_coeffs — O(N log N)
        let e_coeffs = domain.ifft(&e_evals);

        // 3. E'(x) via formal derivative, then FFT — O(N log N)
        // formal_derivative may return a short vec (e.g. [F::ZERO] for constant input),
        // but ark_poly's FFT/IFFT zero-pads short inputs to domain size automatically.
        let e_prime_coeffs = formal_derivative(&e_coeffs);
        let e_prime_evals = domain.fft(&e_prime_coeffs);

        // 4. Precompute 1/E'(ω^j) for j ∈ Ω via batch inversion
        let mut inv_e_prime_vals: Vec<F> = erased.iter().map(|&j| e_prime_evals[j]).collect();
        batch_inversion(&mut inv_e_prime_vals);
        let inv_e_prime_map: HashMap<usize, F> =
            erased.iter().copied().zip(inv_e_prime_vals).collect();

        end_timer!(setup_timer);

        // === Per-polynomial recovery (parallelized) ===
        let recover_timer = start_timer!(|| "Per-polynomial FFT recovery");
        let result: Vec<F> = (0..num_polys)
            .into_par_iter()
            .map(|poly_index| {
                // 1. Build P(ω^j) = C(ω^j)·E(ω^j) for j∈R, 0 for j∈Ω
                let mut p_evals = vec![F::ZERO; n_fft];
                for (&j, &raw_share) in &raw_shares {
                    p_evals[j] = raw_share[poly_index] * e_evals[j];
                }

                // 2. IFFT P → coefficient form
                let p_coeffs = domain.ifft(&p_evals);

                // 3. Formal derivative P'(x)
                // See note above: formal_derivative output may be shorter than domain size;
                // ark_poly's FFT handles this via zero-padding.
                let p_prime_coeffs = formal_derivative(&p_coeffs);

                // 4. FFT P' → evaluations
                let p_prime_evals = domain.fft(&p_prime_coeffs);

                // 5. Recover erased values: C(ω^j) = P'(ω^j) / E'(ω^j)
                let mut c_evals = vec![F::ZERO; n_fft];
                for (&j, raw_share) in &raw_shares {
                    c_evals[j] = raw_share[poly_index];
                }
                for &j in &erased {
                    c_evals[j] = p_prime_evals[j] * inv_e_prime_map[&j];
                }

                // 6. IFFT → take first k coefficients
                let coeffs = domain.ifft(&c_evals);
                coeffs[..recovery_threshold].to_vec()
            })
            .flatten()
            .collect();
        end_timer!(recover_timer);

        Ok(result)
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
        Self::unpad_recovered_bytes(Self::recover_fields(param, shares)?)
    }
}

impl AvidMScheme {
    /// Recover payload fields using per-polynomial Lagrange interpolation (O(k²) per polynomial).
    ///
    /// This is the original recovery approach, kept for benchmarking comparison
    /// against the FFT-based `recover_fields`.
    #[doc(hidden)]
    pub fn recover_fields_lagrange(param: &AvidMParam, shares: &[AvidMShare]) -> VidResult<Vec<F>> {
        let recovery_threshold: usize = param.recovery_threshold;
        let (num_polys, raw_shares) = Self::collect_raw_shares(param, shares)?;

        let domain = radix2_domain::<F>(param.total_weights)?;

        // Collect indices and share data
        let (indices, share_data): (Vec<usize>, Vec<&Vec<F>>) = raw_shares.into_iter().unzip();

        // Per-polynomial Lagrange interpolation
        Ok((0..num_polys)
            .into_par_iter()
            .map(|poly_index| {
                let evals: Vec<_> = indices
                    .iter()
                    .zip(share_data.iter())
                    .map(|(&idx, &p)| (idx, p[poly_index]))
                    .collect();
                jf_utils::reed_solomon_code::reed_solomon_erasure_decode_rou(
                    evals.iter().map(|(idx, val)| (*idx, *val)),
                    recovery_threshold,
                    &domain,
                )
                .map_err(|err| VidError::Internal(err.into()))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect())
    }

    /// Recover payload using per-polynomial Lagrange interpolation.
    ///
    /// This is the original recovery approach, kept for benchmarking comparison
    /// against the FFT-based `recover`.
    #[doc(hidden)]
    pub fn recover_lagrange(
        param: &AvidMParam,
        _commit: &AvidMCommit,
        shares: &[AvidMShare],
    ) -> VidResult<Vec<u8>> {
        Self::unpad_recovered_bytes(Self::recover_fields_lagrange(param, shares)?)
    }
}

/// Unit tests
#[cfg(test)]
pub mod tests {
    use rand::{seq::SliceRandom, RngCore};

    use super::F;
    use crate::{avidm::AvidMScheme, utils::bytes_to_field, VidScheme};

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
                    "recovery_threshold:: {recovery_threshold} num_storage_nodes: \
                     {num_storage_nodes} payload_byte_len: {payload_byte_len}"
                );
                println!("weights: {weights:?}");

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
    fn round_trip_lagrange() {
        let params_list = [(2, 4), (3, 9), (5, 6), (15, 16)];
        let payload_byte_lens = [1, 31, 32, 500];
        let mut rng = jf_utils::test_rng();

        for (recovery_threshold, num_storage_nodes) in params_list {
            let weights: Vec<u32> = (0..num_storage_nodes)
                .map(|_| rng.next_u32() % 5 + 1)
                .collect();
            let total_weights: u32 = weights.iter().sum();
            let params = AvidMScheme::setup(recovery_threshold, total_weights as usize).unwrap();

            for payload_byte_len in payload_byte_lens {
                let payload = {
                    let mut bytes_random = vec![0u8; payload_byte_len];
                    rng.fill_bytes(&mut bytes_random);
                    bytes_random
                };

                let (commit, mut shares) =
                    AvidMScheme::disperse(&params, &weights, &payload).unwrap();

                shares.shuffle(&mut rng);
                let mut cumulated_weights = 0;
                let mut cut_index = 0;
                while cumulated_weights <= recovery_threshold {
                    cumulated_weights += shares[cut_index].content.range.len();
                    cut_index += 1;
                }

                // Both recovery paths should produce identical output
                let fft_recovered =
                    AvidMScheme::recover(&params, &commit, &shares[..cut_index]).unwrap();
                let lagrange_recovered =
                    AvidMScheme::recover_lagrange(&params, &commit, &shares[..cut_index]).unwrap();
                assert_eq!(fft_recovered, payload);
                assert_eq!(lagrange_recovered, payload);
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
