//! This module implements encoding proofs for the Avid-M Scheme.

use std::{collections::HashSet, ops::Range};

use jf_merkle_tree::MerkleTreeScheme;
use jf_utils::canonical;
use serde::{Deserialize, Serialize};

use crate::{
    avid_m::{
        config::AvidMConfig,
        namespaced::{NsAvidMCommit, NsAvidMScheme, NsAvidMShare},
        AvidMCommit, AvidMParam, AvidMScheme, AvidMShare, Config, MerkleProof, MerkleTree, F,
    },
    VerificationResult, VidError, VidResult, VidScheme,
};

/// A proof of incorrect encoding.
/// When the disperser is malicious, he can disperse an incorrectly encoded block, resulting in a merkle root of
/// a Merkle tree containing invalid share (i.e. inconsistent with shares from correctly encoded block). Disperser
/// would disperse them to all replicas with valid Merkle proof against this incorrect root, or else the replicas
/// won't even vote if the merkle proof is wrong. By the time of reconstruction, replicas can come together with
/// at least `threshold` shares to interpolate back the original block (in polynomial form), and by recomputing the
/// corresponding encoded block on this recovered polynomial, we can derive another merkle root of encoded shares.
/// If the merkle root matches the one dispersed earlier, then the encoding was correct.
/// If not, this mismatch can serve as a proof of incorrect encoding.
///
/// In short, the proof contains the recovered poly (from the received shares) and the merkle proofs (against the wrong root)
/// being distributed by the malicious disperser.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct AvidMBadEncodingProof {
    /// The recovered polynomial from VID shares.
    #[serde(with = "canonical")]
    recovered_poly: Vec<F>,
    /// The Merkle proofs against the original commitment.
    #[serde(with = "canonical")]
    raw_shares: Vec<(usize, MerkleProof)>,
}

impl AvidMScheme {
    /// Generate a proof of incorrect encoding
    /// See [`MalEncodingProof`] for details.
    pub fn proof_of_incorrect_encoding(
        param: &AvidMParam,
        commit: &AvidMCommit,
        shares: &[AvidMShare],
    ) -> VidResult<AvidMBadEncodingProof> {
        // First filter out the invalid shares
        let shares = shares
            .iter()
            .filter(|share| {
                AvidMScheme::verify_share(param, commit, share).is_ok_and(|r| r.is_ok())
            })
            .cloned()
            .collect::<Vec<_>>();
        // Recover the original payload in fields representation.
        // Length of `payload` is always a multiple of `recovery_threshold`.
        let witness = Self::recover_fields(param, &shares)?;
        let (mt, _) = Self::raw_encode(param, &witness)?;
        if mt.commitment() == commit.commit {
            return Err(VidError::Argument(
                "Cannot generate the proof of incorrect encoding: encoding is good.".to_string(),
            ));
        }

        let mut raw_shares = vec![];
        let mut visited_indices = HashSet::new();
        for share in shares {
            for (index, mt_proof) in share
                .content
                .range
                .clone()
                .zip(share.content.mt_proofs.iter())
            {
                if index > param.total_weights {
                    return Err(VidError::InvalidShare);
                }
                if visited_indices.contains(&index) {
                    return Err(VidError::InvalidShare);
                }
                raw_shares.push((index, mt_proof.clone()));
                visited_indices.insert(index);
                if raw_shares.len() == param.recovery_threshold {
                    break;
                }
            }
            if raw_shares.len() == param.recovery_threshold {
                break;
            }
        }
        if raw_shares.len() != param.recovery_threshold {
            return Err(VidError::InsufficientShares);
        }

        Ok(AvidMBadEncodingProof {
            recovered_poly: witness,
            raw_shares,
        })
    }
}

impl AvidMBadEncodingProof {
    /// Verify a proof of incorrect encoding
    pub fn verify(
        &self,
        param: &AvidMParam,
        commit: &AvidMCommit,
    ) -> VidResult<VerificationResult> {
        // A bad encoding proof should have exactly `recovery_threshold` raw shares
        if self.raw_shares.len() != param.recovery_threshold {
            return Err(VidError::InvalidParam);
        }
        if self.recovered_poly.len() > param.recovery_threshold {
            // recovered polynomial should be of low degree
            return Err(VidError::InvalidParam);
        }
        let (mt, raw_shares) = AvidMScheme::raw_encode(param, &self.recovered_poly)?;
        if mt.commitment() == commit.commit {
            return Ok(Err(()));
        }
        let mut visited_indices = HashSet::new();
        for (index, proof) in self.raw_shares.iter() {
            if *index >= param.total_weights || visited_indices.contains(index) {
                return Err(VidError::InvalidShare);
            }
            let digest = Config::raw_share_digest(&raw_shares[*index])?;
            if MerkleTree::verify(&commit.commit, *index as u64, &digest, proof)?.is_err() {
                return Ok(Err(()));
            }
            visited_indices.insert(*index);
        }
        Ok(Ok(()))
    }
}

/// A proof of incorrect encoding for a namespace.
/// It consists of the index of the namespace, the merkle proof of the namespace payload against the namespaced VID commitment,
/// and the proof of incorrect encoding.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct NsAvidMBadEncodingProof {
    /// The index of the namespace.
    pub ns_index: usize,
    /// The commitment of the namespaced VID.
    pub ns_commit: AvidMCommit,
    /// The outer merkle proof of the namespace against the namespaced VID commitment.
    pub ns_mt_proof: MerkleProof,
    /// The proof of incorrect encoding.
    pub ns_proof: AvidMBadEncodingProof,
}

impl NsAvidMScheme {
    /// Generate a proof of incorrect encoding for a namespace.
    pub fn proof_of_incorrect_encoding_for_namespace(
        param: &AvidMParam,
        ns_index: usize,
        commit: &NsAvidMCommit,
        shares: &[NsAvidMShare],
    ) -> VidResult<NsAvidMBadEncodingProof> {
        if shares.is_empty() {
            return Err(VidError::InsufficientShares);
        }
        if shares.iter().any(|share| !share.contains_ns(ns_index)) {
            return Err(VidError::IndexOutOfBound);
        }
        let mt = MerkleTree::from_elems(
            None,
            shares[0].ns_commits().iter().map(|commit| commit.commit),
        )?;
        if mt.commitment() != commit.commit {
            return Err(VidError::InvalidParam);
        }
        let (ns_commit, ns_mt_proof) = mt
            .lookup(ns_index as u64)
            .expect_ok()
            .expect("MT lookup shouldn't fail");
        let ns_commit = AvidMCommit { commit: *ns_commit };
        let shares = shares
            .iter()
            .filter_map(|share| share.inner_ns_share(ns_index))
            .collect::<Vec<_>>();
        Ok(NsAvidMBadEncodingProof {
            ns_index,
            ns_commit,
            ns_mt_proof,
            ns_proof: AvidMScheme::proof_of_incorrect_encoding(param, &ns_commit, &shares)?,
        })
    }

    /// Generate a proof of incorrect encoding.
    pub fn proof_of_incorrect_encoding(
        param: &AvidMParam,
        commit: &NsAvidMCommit,
        shares: &[NsAvidMShare],
    ) -> VidResult<NsAvidMBadEncodingProof> {
        if shares.is_empty() {
            return Err(VidError::InsufficientShares);
        }
        for ns_index in 0..shares[0].ns_commits().len() {
            let result =
                Self::proof_of_incorrect_encoding_for_namespace(param, ns_index, commit, shares);
            // Early break if there's a bad namespace, or if the shares/param are invalid
            if matches!(
                result,
                Ok(_)
                    | Err(VidError::InvalidShare)
                    | Err(VidError::IndexOutOfBound)
                    | Err(VidError::InsufficientShares)
            ) {
                return result;
            }
        }
        Err(VidError::InvalidParam)
    }
}

impl NsAvidMBadEncodingProof {
    /// Verify an incorrect encoding proof.
    pub fn verify(
        &self,
        param: &AvidMParam,
        commit: &NsAvidMCommit,
    ) -> VidResult<VerificationResult> {
        if MerkleTree::verify(
            &commit.commit,
            self.ns_index as u64,
            &self.ns_commit.commit,
            &self.ns_mt_proof,
        )?
        .is_err()
        {
            return Ok(Err(()));
        }
        self.ns_proof.verify(param, &self.ns_commit)
    }
}

/// A proof of a namespace payload.
/// It consists of the index of the namespace, the namespace payload, and a merkle proof
/// of the namespace payload against the namespaced VID commitment.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct NsProof {
    /// The index of the namespace.
    pub ns_index: usize,
    /// The namespace payload.
    pub ns_payload: Vec<u8>,
    /// The merkle proof of the namespace payload against the namespaced VID commitment.
    pub ns_proof: MerkleProof,
}

impl NsAvidMScheme {
    /// Generate a proof of inclusion for a namespace payload.
    /// WARN: for the current implementation, no proof can be generated if any namespace is malformed.
    pub fn namespace_proof(
        param: &AvidMParam,
        payload: &[u8],
        ns_index: usize,
        ns_table: impl IntoIterator<Item = Range<usize>>,
    ) -> VidResult<NsProof> {
        let ns_table = ns_table.into_iter().collect::<Vec<_>>();
        let ns_payload_range = ns_table[ns_index].clone();
        let ns_commits = ns_table
            .into_iter()
            .map(|ns_range| {
                AvidMScheme::commit(param, &payload[ns_range]).map(|commit| commit.commit)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mt = MerkleTree::from_elems(None, &ns_commits)?;
        Ok(NsProof {
            ns_index,
            ns_payload: payload[ns_payload_range].to_vec(),
            ns_proof: mt
                .lookup(ns_index as u64)
                .expect_ok()
                .expect("MT lookup shouldn't fail")
                .1,
        })
    }

    /// Verify a namespace proof against a namespaced VID commitment.
    pub fn verify_namespace_proof(
        param: &AvidMParam,
        commit: &NsAvidMCommit,
        proof: &NsProof,
    ) -> VidResult<VerificationResult> {
        let ns_commit = AvidMScheme::commit(param, &proof.ns_payload)?;
        Ok(MerkleTree::verify(
            &commit.commit,
            proof.ns_index as u64,
            &ns_commit.commit,
            &proof.ns_proof,
        )?)
    }
}

#[cfg(test)]
mod tests {
    use ark_poly::EvaluationDomain;
    use jf_merkle_tree::MerkleTreeScheme;
    use rand::{seq::SliceRandom, Rng};

    use crate::{
        avid_m::{
            config::AvidMConfig,
            namespaced::{NsAvidMCommit, NsAvidMScheme, NsAvidMShare},
            proofs::AvidMBadEncodingProof,
            radix2_domain, AvidMCommit, AvidMScheme, AvidMShare, Config, MerkleTree, RawAvidMShare,
            F,
        },
        utils::bytes_to_field,
        VidScheme,
    };

    #[test]
    fn test_proof_of_incorrect_encoding() {
        let mut rng = jf_utils::test_rng();
        let param = AvidMScheme::setup(5usize, 10usize).unwrap();
        let weights = [1u32; 10];
        let payload_byte_len = bytes_to_field::elem_byte_capacity::<F>() * 4;
        let domain = radix2_domain::<F>(param.total_weights).unwrap();

        let high_degree_polynomial = vec![F::from(1u64); 10];
        let mal_payload: Vec<_> = domain
            .fft(&high_degree_polynomial)
            .into_iter()
            .take(param.total_weights)
            .map(|v| vec![v])
            .collect();

        let mt = MerkleTree::from_elems(
            None,
            mal_payload
                .iter()
                .map(|v| Config::raw_share_digest(v).unwrap()),
        )
        .unwrap();

        let (commit, mut shares) =
            AvidMScheme::distribute_shares(&param, &weights, mt, mal_payload, payload_byte_len)
                .unwrap();

        shares.shuffle(&mut rng);

        // not enough shares
        assert!(AvidMScheme::proof_of_incorrect_encoding(&param, &commit, &shares[..1]).is_err());

        // successful proof generation
        let proof =
            AvidMScheme::proof_of_incorrect_encoding(&param, &commit, &shares[..5]).unwrap();
        assert!(proof.verify(&param, &commit).unwrap().is_ok());

        // proof generation shall not work on good commitment and shares
        let payload = [1u8; 50];
        let (commit, mut shares) = AvidMScheme::disperse(&param, &weights, &payload).unwrap();
        shares.shuffle(&mut rng);
        assert!(AvidMScheme::proof_of_incorrect_encoding(&param, &commit, &shares).is_err());

        let witness = AvidMScheme::pad_to_fields(&param, &payload);
        let bad_proof = AvidMBadEncodingProof {
            recovered_poly: witness.clone(),
            raw_shares: shares
                .iter()
                .map(|share| (share.index as usize, share.content.mt_proofs[0].clone()))
                .collect(),
        };
        assert!(bad_proof.verify(&param, &commit).is_err());

        // duplicate indices may fool the verification
        let mut bad_witness = vec![F::from(0u64); 5];
        bad_witness[0] = shares[0].content.payload[0][0];
        let bad_proof2 = AvidMBadEncodingProof {
            recovered_poly: bad_witness,
            raw_shares: std::iter::repeat_n(bad_proof.raw_shares[0].clone(), 6).collect(),
        };
        assert!(bad_proof2.verify(&param, &commit).is_err());
    }

    #[test]
    fn test_ns_proof() {
        let param = AvidMScheme::setup(5usize, 10usize).unwrap();
        let payload = vec![1u8; 100];
        let ns_table = vec![(0..10), (10..21), (21..33), (33..48), (48..100)];
        let commit = NsAvidMScheme::commit(&param, &payload, ns_table.clone()).unwrap();

        for (i, _) in ns_table.iter().enumerate() {
            let proof =
                NsAvidMScheme::namespace_proof(&param, &payload, i, ns_table.clone()).unwrap();
            assert!(
                NsAvidMScheme::verify_namespace_proof(&param, &commit, &proof)
                    .unwrap()
                    .is_ok()
            );
        }
        let mut proof =
            NsAvidMScheme::namespace_proof(&param, &payload, 1, ns_table.clone()).unwrap();
        proof.ns_index = 0;
        assert!(
            NsAvidMScheme::verify_namespace_proof(&param, &commit, &proof)
                .unwrap()
                .is_err()
        );
        proof.ns_index = 1;
        proof.ns_payload[0] = 0u8;
        assert!(
            NsAvidMScheme::verify_namespace_proof(&param, &commit, &proof)
                .unwrap()
                .is_err()
        );
        proof.ns_index = 100;
        assert!(
            NsAvidMScheme::verify_namespace_proof(&param, &commit, &proof)
                .unwrap()
                .is_err()
        );
    }

    #[test]
    fn test_ns_proof_of_incorrect_encoding() {
        let mut rng = jf_utils::test_rng();
        let param = AvidMScheme::setup(5usize, 10usize).unwrap();
        let mut payload = [1u8; 100];
        rng.fill(&mut payload[..]);
        let distribution = [1u32; 10];
        let ns_table = [(0..10), (10..21), (21..33), (33..48), (48..100)];
        let domain = radix2_domain::<F>(param.total_weights).unwrap();

        // Manually distribute the payload, with second namespace being malicious
        let mut ns_commits = vec![];
        let mut disperses = vec![];
        let mut ns_lens = vec![];
        for ns_range in ns_table.iter() {
            ns_lens.push(ns_range.len());
            if ns_range.start == 10 {
                // second namespace is malicious, commit to a high-degree polynomial
                let high_degree_polynomial = vec![F::from(1u64); 10];
                let mal_payload: Vec<_> = domain
                    .fft(&high_degree_polynomial)
                    .into_iter()
                    .take(param.total_weights)
                    .map(|v| vec![v])
                    .collect();
                let bad_mt = MerkleTree::from_elems(
                    None,
                    mal_payload
                        .iter()
                        .map(|v| Config::raw_share_digest(v).unwrap()),
                )
                .unwrap();
                ns_commits.push(bad_mt.commitment());
                let shares: Vec<_> = mal_payload
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| AvidMShare {
                        index: i as u32,
                        payload_byte_len: ns_range.len(),
                        content: RawAvidMShare {
                            range: (i..i + 1),
                            payload: vec![v],
                            mt_proofs: vec![bad_mt.lookup(i as u64).expect_ok().unwrap().1],
                        },
                    })
                    .collect();
                disperses.push(shares);
            } else {
                let (commit, shares) =
                    AvidMScheme::disperse(&param, &distribution, &payload[ns_range.clone()])
                        .unwrap();
                ns_commits.push(commit.commit);
                disperses.push(shares);
            }
        }
        let commit = NsAvidMCommit {
            commit: MerkleTree::from_elems(None, &ns_commits)
                .unwrap()
                .commitment(),
        };
        let ns_commits: Vec<_> = ns_commits
            .into_iter()
            .map(|comm| AvidMCommit { commit: comm })
            .collect();
        let mut shares = vec![NsAvidMShare::default(); disperses[0].len()];
        shares.iter_mut().for_each(|share| {
            share.index = disperses[0][0].index;
            share.ns_commits = ns_commits.clone();
            share.ns_lens = ns_lens.clone();
        });
        disperses.into_iter().for_each(|ns_disperse| {
            shares
                .iter_mut()
                .zip(ns_disperse)
                .for_each(|(share, ns_share)| share.content.push(ns_share.content))
        });

        // generate bad encoding proof for the second namespace
        let proof =
            NsAvidMScheme::proof_of_incorrect_encoding_for_namespace(&param, 1, &commit, &shares)
                .unwrap();
        assert!(proof.verify(&param, &commit).unwrap().is_ok());

        // Good namespaces
        for ns_index in [0, 2, 3, 4] {
            assert!(NsAvidMScheme::proof_of_incorrect_encoding_for_namespace(
                &param, ns_index, &commit, &shares
            )
            .is_err());
        }

        let proof = NsAvidMScheme::proof_of_incorrect_encoding(&param, &commit, &shares).unwrap();
        assert_eq!(proof.ns_index, 1);
        assert!(proof.verify(&param, &commit).unwrap().is_ok());
    }
}
