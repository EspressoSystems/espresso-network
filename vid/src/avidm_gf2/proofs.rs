//! This module implements encoding proofs for the Avid-M Scheme.

use jf_merkle_tree::{MerkleTreeScheme, RangeProofMerkleTreeScheme};
use serde::{Deserialize, Serialize};

use crate::{
    avidm_gf2::{
        namespaced::{NsAvidmGf2Commit, NsAvidmGf2Common, NsAvidmGf2Scheme},
        AvidmGf2Commit, AvidmGf2Param, AvidmGf2Scheme, MerkleProof, MerkleRangeProof, MerkleTree,
    },
    VerificationResult, VidError, VidResult, VidScheme,
};

/// A proof of a namespace payload.
/// It consists of the index of the namespace, the namespace payload, and a merkle proof
/// of the namespace payload against the namespaced VID commitment.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct NsProof {
    /// The index of the namespace.
    pub ns_index: usize,
    /// The namespace payload.
    #[serde(with = "base64_bytes")]
    pub ns_payload: Vec<u8>,
    /// The merkle proof of the namespace payload against the namespaced VID commitment.
    pub ns_proof: MerkleProof,
}

impl NsAvidmGf2Scheme {
    /// Generate a proof of inclusion for a namespace payload.
    pub fn namespace_proof(
        common: &NsAvidmGf2Common,
        payload: &[u8],
        ns_index: usize,
    ) -> VidResult<NsProof> {
        if common.ns_commits.len() != common.ns_lens.len() {
            return Err(VidError::Internal(anyhow::anyhow!(
                "Inconsistent common data"
            )));
        }
        if ns_index >= common.ns_lens.len() {
            return Err(VidError::IndexOutOfBound);
        }
        let ns_payload_range_start = common.ns_lens[..ns_index].iter().sum::<usize>();
        let ns_payload_range_end = ns_payload_range_start + common.ns_lens[ns_index];
        if ns_payload_range_end > payload.len() {
            return Err(VidError::Internal(anyhow::anyhow!(
                "Payload length is inconsistent with namespace lengths"
            )));
        }

        let mt = MerkleTree::from_elems(None, common.ns_commits.iter().map(|c| c.commit))?;
        Ok(NsProof {
            ns_index,
            ns_payload: payload[ns_payload_range_start..ns_payload_range_end].to_vec(),
            ns_proof: mt
                .lookup(ns_index as u64)
                .expect_ok()
                .expect("MT lookup shouldn't fail")
                .1,
        })
    }

    /// Verify a namespace proof against a namespaced VID commitment.
    pub fn verify_namespace_proof(
        commit: &NsAvidmGf2Commit,
        common: &NsAvidmGf2Common,
        proof: &NsProof,
    ) -> VidResult<VerificationResult> {
        let ns_commit = AvidmGf2Scheme::commit(&common.param, &proof.ns_payload)?;
        Ok(MerkleTree::verify(
            &commit.commit,
            proof.ns_index as u64,
            &ns_commit.commit,
            &proof.ns_proof,
        )?)
    }
}

/// A proof of a transaction payload.
/// It consists of the range of the transaction in bytes, the transaction payload, and a merkle proof
/// of the transaction payload against the namespaced VID commitment.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct BytesInclusionProof {
    /// The range of the transaction in bytes.
    pub bytes_range: std::ops::Range<u64>,
    /// The transaction payload.
    #[serde(with = "base64_bytes")]
    pub bytes: Vec<u8>,
    /// The merkle proof of the transaction payload against the namespace commitment.
    pub proof: MerkleRangeProof,
}

impl AvidmGf2Scheme {
    /// Generate a proof of a transaction payload.
    pub fn bytes_inclusion_proof(
        param: &AvidmGf2Param,
        payload: &[u8],
        bytes_range: std::ops::Range<u64>,
    ) -> VidResult<BytesInclusionProof> {
        let (mt, _) = AvidmGf2Scheme::raw_disperse(param, payload)?;
        Ok(BytesInclusionProof {
            bytes_range: bytes_range.clone(),
            bytes: payload[bytes_range.start as usize..bytes_range.end as usize].to_vec(),
            proof: mt
                .range_lookup(bytes_range.start, bytes_range.end - 1)
                .expect_ok()
                .expect("Range proof shouldn't fail")
                .1,
        })
    }

    /// Verify a transaction proof against a commitment.
    pub fn verify_bytes_inclusion_proof(
        param: &AvidmGf2Param,
        commit: &AvidmGf2Commit,
        proof: &BytesInclusionProof,
    ) -> VidResult<VerificationResult> {
        todo!()
        // let indices: Vec<u64> = proof.tx_bytes_range.clone().collect();
        // Ok(MerkleTree::verify_range_proof(
        //     &commit.commit,
        //     &indices,
        //     &proof.tx_payload,
        //     &proof.tx_proof,
        // )?)
    }
}

/// A proof of a transaction payload within a namespace.
pub struct NsTxProof {
    /// The index of the namespace.
    pub ns_index: usize,
    /// The commitment of the namespaced VID.
    pub ns_commit: AvidmGf2Commit,
    /// The merkle proof of the namespace against the namespaced VID commitment.
    pub ns_proof: MerkleProof,
    /// The proof of the transaction payload.
    pub tx_proof: BytesInclusionProof,
}

#[cfg(test)]
mod tests {
    use crate::avidm_gf2::{namespaced::NsAvidmGf2Scheme, AvidmGf2Scheme};

    #[test]
    fn test_ns_proof() {
        let param = AvidmGf2Scheme::setup(5usize, 10usize).unwrap();
        let payload = vec![1u8; 100];
        let ns_table = vec![(0..10), (10..21), (21..33), (33..48), (48..100)];
        let (commit, common) =
            NsAvidmGf2Scheme::commit(&param, &payload, ns_table.clone()).unwrap();

        for (i, _) in ns_table.iter().enumerate() {
            let proof = NsAvidmGf2Scheme::namespace_proof(&common, &payload, i).unwrap();
            assert!(
                NsAvidmGf2Scheme::verify_namespace_proof(&commit, &common, &proof)
                    .unwrap()
                    .is_ok()
            );
        }
        let mut proof = NsAvidmGf2Scheme::namespace_proof(&common, &payload, 1).unwrap();
        proof.ns_index = 0;
        assert!(
            NsAvidmGf2Scheme::verify_namespace_proof(&commit, &common, &proof)
                .unwrap()
                .is_err()
        );
        proof.ns_index = 1;
        proof.ns_payload[0] = 0u8;
        assert!(
            NsAvidmGf2Scheme::verify_namespace_proof(&commit, &common, &proof)
                .unwrap()
                .is_err()
        );
        proof.ns_index = 100;
        assert!(
            NsAvidmGf2Scheme::verify_namespace_proof(&commit, &common, &proof)
                .unwrap()
                .is_err()
        );
    }
}
