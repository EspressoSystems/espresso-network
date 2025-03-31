//! This module implements encoding proofs for the Avid-M Scheme.

use std::collections::HashSet;

use jf_merkle_tree::MerkleTreeScheme;
use jf_utils::canonical;
use serde::{Deserialize, Serialize};

use crate::{
    avid_m::{
        config::AvidMConfig, AvidMCommit, AvidMParam, AvidMScheme, AvidMShare, Config, MerkleProof,
        MerkleTree, F,
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
pub struct MalEncodingProof {
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
    ) -> VidResult<MalEncodingProof> {
        // First verify all the shares
        for share in shares.iter() {
            if AvidMScheme::verify_share(param, commit, share)?.is_err() {
                return Err(VidError::InvalidShare);
            }
        }
        // Recover the original payload in fields representation.
        // Length of `payload` is always a multiple of `recovery_threshold`.
        let witness = Self::recover_fields(param, shares)?;
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
                if raw_shares.len() >= param.recovery_threshold {
                    break;
                }
            }
        }
        if raw_shares.len() < param.recovery_threshold {
            return Err(VidError::InsufficientShares);
        }

        Ok(MalEncodingProof {
            recovered_poly: witness,
            raw_shares,
        })
    }
}

impl MalEncodingProof {
    /// Verify a proof of incorrect encoding
    pub fn verify(
        &self,
        param: &AvidMParam,
        commit: &AvidMCommit,
    ) -> VidResult<VerificationResult> {
        // First check that all shares are valid.
        if self.raw_shares.len() < param.recovery_threshold {
            return Err(VidError::InsufficientShares);
        }
        if self.raw_shares.len() > param.total_weights {
            return Err(VidError::InvalidShare);
        }
        let (mt, raw_shares) = AvidMScheme::raw_encode(param, &self.recovered_poly)?;
        if mt.commitment() == commit.commit {
            return Err(VidError::InvalidParam);
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

#[cfg(test)]
mod tests {
    use ark_poly::EvaluationDomain;
    use rand::seq::SliceRandom;

    use crate::{
        avid_m::{
            config::AvidMConfig, proofs::MalEncodingProof, radix2_domain, AvidMScheme, Config,
            MerkleTree, F,
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
        let bad_proof = MalEncodingProof {
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
        let bad_proof2 = MalEncodingProof {
            recovered_poly: bad_witness,
            raw_shares: std::iter::repeat(bad_proof.raw_shares[0].clone())
                .take(6)
                .collect(),
        };
        assert!(bad_proof2.verify(&param, &commit).is_err());
    }
}
