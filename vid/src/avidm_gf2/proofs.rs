//! This module implements encoding proofs for the Avid-M Scheme.

use jf_merkle_tree::MerkleTreeScheme;
use serde::{Deserialize, Serialize};

use crate::{
    VerificationResult, VidError, VidResult, VidScheme,
    avidm_gf2::{
        AvidmGf2Scheme, MerkleProof, MerkleTree,
        namespaced::{NsAvidmGf2Commit, NsAvidmGf2Common, NsAvidmGf2Scheme},
    },
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

        let mt = common.merkle_tree()?;
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
    ///
    /// `common` may come from an untrusted source: it is first checked for
    /// consistency against `commit` (which binds all of it), so a forged
    /// `param` or `ns_lens` fails verification outright.
    pub fn verify_namespace_proof(
        commit: &NsAvidmGf2Commit,
        common: &NsAvidmGf2Common,
        proof: &NsProof,
    ) -> VidResult<VerificationResult> {
        if !Self::is_consistent(commit, common) {
            return Ok(Err(()));
        }
        // Excludes the final binding leaf of the commitment's merkle tree,
        // which is not a namespace commitment.
        if proof.ns_index >= common.ns_commits.len() {
            return Ok(Err(()));
        }
        let ns_commit = AvidmGf2Scheme::commit(&common.param, &proof.ns_payload)?;
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
    use crate::avidm_gf2::{AvidmGf2Scheme, namespaced::NsAvidmGf2Scheme};

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

    #[test]
    fn ns_proof_rejects_tampered_common() {
        let param = AvidmGf2Scheme::setup(5usize, 10usize).unwrap();
        let payload = vec![1u8; 100];
        let ns_table = vec![(0..10), (10..100)];
        let (commit, common) = NsAvidmGf2Scheme::commit(&param, &payload, ns_table).unwrap();
        let proof = NsAvidmGf2Scheme::namespace_proof(&common, &payload, 1).unwrap();
        assert!(
            NsAvidmGf2Scheme::verify_namespace_proof(&commit, &common, &proof)
                .unwrap()
                .is_ok()
        );

        // A common with forged params does not belong to this commitment.
        let mut tampered = common.clone();
        tampered.param.recovery_threshold = 1;
        assert!(
            NsAvidmGf2Scheme::verify_namespace_proof(&commit, &tampered, &proof)
                .unwrap()
                .is_err()
        );

        // The binding leaf's index cannot be claimed as a namespace.
        let mut out_of_range = proof.clone();
        out_of_range.ns_index = common.ns_commits.len();
        assert!(
            NsAvidmGf2Scheme::verify_namespace_proof(&commit, &common, &out_of_range)
                .unwrap()
                .is_err()
        );
    }
}
