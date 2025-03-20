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
/// It consists of a witness vector, which is the claimed low degree polynomial,
/// and a list of [`MalEncodingProofRawShare`]s. Each raw share claims that, in the Merkle tree committed by the provided commitment,
/// some code word of the witness vector appears at the `index` position.
/// If we have enough raw shares, and the Merkle commitment of the encoding of witness vector doesn't match the provided commitment,
/// we assert that the provided commitment cannot open to be a correct encoding.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MalEncodingProof {
    /// The witness vector
    #[serde(with = "canonical")]
    witness: Vec<F>,
    /// Proof content.
    raw_shares: Vec<MalEncodingProofRawShare>,
}

/// Proof content, see [`MalEncodingProof`].
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MalEncodingProofRawShare {
    /// The index of this raw share
    pub index: usize,
    /// A Merkle tree proof against the provided Merkle tree commitment.
    #[serde(with = "canonical")]
    pub mt_proof: MerkleProof,
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
                return Err(VidError::Argument("Invalid share".to_string()));
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
                    return Err(VidError::Argument("Invalid share".to_string()));
                }
                if visited_indices.contains(&index) {
                    return Err(VidError::Argument("Overlapping shares".to_string()));
                }
                raw_shares.push(MalEncodingProofRawShare {
                    index,
                    mt_proof: mt_proof.clone(),
                });
                visited_indices.insert(index);
                if raw_shares.len() >= param.recovery_threshold {
                    break;
                }
            }
        }
        if raw_shares.len() < param.recovery_threshold {
            return Err(VidError::Argument(
                "Insufficient shares to generate the proof of incorrect encoding.".to_string(),
            ));
        }

        Ok(MalEncodingProof {
            witness,
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
            return Err(VidError::Argument(
                "Insufficient shares to generate the proof of incorrect encoding.".to_string(),
            ));
        }
        if self.raw_shares.len() > param.total_weights {
            return Err(VidError::Argument("To many shares".to_string()));
        }
        let (mt, raw_shares) = AvidMScheme::raw_encode(param, &self.witness)?;
        if mt.commitment() == commit.commit {
            return Ok(Err(()));
        }
        let mut visited_indices = HashSet::new();
        for share in self.raw_shares.iter() {
            if share.index >= param.total_weights || visited_indices.contains(&share.index) {
                return Err(VidError::Argument("Invalid share".to_string()));
            }
            let digest = Config::raw_share_digest(&raw_shares[share.index])?;
            if MerkleTree::verify(&commit.commit, share.index as u64, &digest, &share.mt_proof)?
                .is_err()
            {
                return Ok(Err(()));
            }
            visited_indices.insert(share.index);
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
            config::AvidMConfig,
            proofs::{MalEncodingProof, MalEncodingProofRawShare},
            radix2_domain, AvidMScheme, Config, MerkleTree, F,
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
            witness: witness.clone(),
            raw_shares: shares
                .iter()
                .map(|share| MalEncodingProofRawShare {
                    index: share.index as usize,
                    mt_proof: share.content.mt_proofs[0].clone(),
                })
                .collect(),
        };
        assert!(bad_proof.verify(&param, &commit).unwrap().is_err());

        let mut bad_witness = vec![F::from(0u64); 5];
        bad_witness[0] = shares[0].content.payload[0][0];
        let bad_proof2 = MalEncodingProof {
            witness: bad_witness,
            raw_shares: std::iter::repeat(bad_proof.raw_shares[0].clone())
                .take(6)
                .collect(),
        };
        assert!(bad_proof2.verify(&param, &commit).is_err());
    }
}
