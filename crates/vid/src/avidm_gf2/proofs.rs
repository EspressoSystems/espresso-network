//! This module implements encoding proofs for the Avid-M Scheme.

use std::ops::Range;

use jf_merkle_tree::MerkleTreeScheme;
use serde::{Deserialize, Serialize};

use crate::{
    VerificationResult, VidError, VidResult, VidScheme,
    avidm_gf2::{
        AvidmGf2Commit, AvidmGf2Param, AvidmGf2Scheme, AvidmGf2Share, MerkleProof, MerkleTree,
        namespaced::{NsAvidmGf2Commit, NsAvidmGf2Common, NsAvidmGf2Scheme, NsAvidmGf2Share},
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

/// A proof that a dispersal committed to a non-codeword.
///
/// A Byzantine disperser can commit to shards that no payload encodes to and
/// still hand every node a share with genuine Merkle proofs, so the shares
/// verify and the nodes vote. Any `recovery_threshold` positions of a codeword
/// decode to its payload, and re-committing that payload reproduces the
/// commitment. So shares that verify against the commitment, cover exactly
/// that many positions, and decode to no payload or to one that commits
/// elsewhere show that no payload commits to it.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct AvidmGf2BadEncodingProof {
    /// Shares with pairwise disjoint ranges covering exactly
    /// `recovery_threshold` positions, each verifying against the commitment.
    shares: Vec<AvidmGf2Share>,
}

impl AvidmGf2BadEncodingProof {
    /// The shares the proof decodes.
    pub fn shares(&self) -> &[AvidmGf2Share] {
        &self.shares
    }

    /// Check that the proof shows `commit` to be the commitment of no payload.
    ///
    /// `Ok(Err(()))` means it does not: a share fails to verify against
    /// `commit`, or the shares decode to a payload that commits to it. `Err`
    /// means the proof is malformed.
    pub fn verify(
        &self,
        param: &AvidmGf2Param,
        commit: &AvidmGf2Commit,
    ) -> VidResult<VerificationResult> {
        let mut covered = 0;
        for (i, share) in self.shares.iter().enumerate() {
            if self.shares[..i]
                .iter()
                .any(|taken| overlaps(taken.range(), share.range()))
            {
                return Err(VidError::InvalidShare);
            }
            if AvidmGf2Scheme::verify_share(param, commit, share)?.is_err() {
                return Ok(Err(()));
            }
            covered += share.weight();
        }
        if covered != param.recovery_threshold {
            return Err(VidError::InvalidParam);
        }
        // The shares are the committed shards at threshold-many positions. Were
        // the committed set a codeword, they would decode to its payload, which
        // would re-commit to `commit`. So a decoding failure is the shards'
        // fault too: an honest dispersal gives every shard the same even,
        // positive length and a payload the padding delimits.
        let Ok(payload) = AvidmGf2Scheme::recover(param, commit, &self.shares) else {
            return Ok(Ok(()));
        };
        Ok(if AvidmGf2Scheme::commit(param, &payload)? == *commit {
            Err(())
        } else {
            Ok(())
        })
    }
}

impl AvidmGf2Scheme {
    /// Prove that `commit` is the commitment of no payload, from `shares` that
    /// verify against it and together cover the recovery threshold.
    ///
    /// Shares that fail to verify are ignored, and the proof keeps only as
    /// many positions as the threshold needs.
    pub fn proof_of_incorrect_encoding(
        param: &AvidmGf2Param,
        commit: &AvidmGf2Commit,
        shares: &[AvidmGf2Share],
    ) -> VidResult<AvidmGf2BadEncodingProof> {
        let verified = shares
            .iter()
            .filter(|share| Self::verify_share(param, commit, share).is_ok_and(|r| r.is_ok()));
        let proof = AvidmGf2BadEncodingProof {
            shares: Self::threshold_cover(param, verified)?,
        };
        match proof.verify(param, commit)? {
            Ok(()) => Ok(proof),
            Err(()) => Err(VidError::Argument(
                "Cannot generate the proof of incorrect encoding: encoding is good.".to_string(),
            )),
        }
    }

    /// Trim `shares` to pairwise disjoint ranges covering exactly
    /// `recovery_threshold` positions, skipping any share that overlaps one
    /// already taken.
    fn threshold_cover<'a>(
        param: &AvidmGf2Param,
        shares: impl Iterator<Item = &'a AvidmGf2Share>,
    ) -> VidResult<Vec<AvidmGf2Share>> {
        let mut cover: Vec<AvidmGf2Share> = Vec::new();
        let mut covered = 0;
        for share in shares {
            if covered == param.recovery_threshold {
                break;
            }
            if cover
                .iter()
                .any(|taken| overlaps(taken.range(), share.range()))
            {
                continue;
            }
            let take = share.weight().min(param.recovery_threshold - covered);
            cover.push(share_prefix(share, take));
            covered += take;
        }
        if covered < param.recovery_threshold {
            return Err(VidError::InsufficientShares);
        }
        Ok(cover)
    }
}

fn overlaps(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < b.end && b.start < a.end
}

/// `share` restricted to the first `len` positions of its range.
fn share_prefix(share: &AvidmGf2Share, len: usize) -> AvidmGf2Share {
    AvidmGf2Share {
        range: share.range.start..share.range.start + len,
        payload: share.payload[..len].to_vec(),
        mt_proofs: share.mt_proofs[..len].to_vec(),
    }
}

/// A proof that one namespace of a namespaced dispersal committed to a
/// non-codeword: the namespace's commitment in the namespaced VID commitment,
/// and a bad-encoding proof against it.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct NsAvidmGf2BadEncodingProof {
    /// The index of the namespace.
    pub ns_index: usize,
    /// The namespace's commitment, which the proof's shares verify against.
    pub ns_commit: AvidmGf2Commit,
    /// The Merkle proof of `ns_commit` against the namespaced VID commitment.
    pub ns_mt_proof: MerkleProof,
    /// The proof of incorrect encoding against `ns_commit`.
    pub ns_proof: AvidmGf2BadEncodingProof,
}

impl NsAvidmGf2Scheme {
    /// Prove that namespace `ns_index` of the dispersal `commit` and `common`
    /// describe committed to a non-codeword, from `shares` that verify
    /// against it.
    pub fn proof_of_incorrect_encoding_for_namespace(
        commit: &NsAvidmGf2Commit,
        common: &NsAvidmGf2Common,
        ns_index: usize,
        shares: &[NsAvidmGf2Share],
    ) -> VidResult<NsAvidmGf2BadEncodingProof> {
        if !Self::is_consistent(commit, common) {
            return Err(VidError::InvalidParam);
        }
        if ns_index >= common.ns_commits.len() {
            return Err(VidError::IndexOutOfBound);
        }
        let mt = MerkleTree::from_elems(None, common.ns_commits.iter().map(|c| c.commit))?;
        let (_, ns_mt_proof) = mt
            .lookup(ns_index as u64)
            .expect_ok()
            .expect("MT lookup shouldn't fail");
        let ns_commit = common.ns_commits[ns_index];
        let ns_shares: Vec<_> = shares
            .iter()
            .filter_map(|share| share.inner_ns_share(ns_index))
            .collect();
        Ok(NsAvidmGf2BadEncodingProof {
            ns_index,
            ns_commit,
            ns_mt_proof,
            ns_proof: AvidmGf2Scheme::proof_of_incorrect_encoding(
                &common.param,
                &ns_commit,
                &ns_shares,
            )?,
        })
    }
}

impl NsAvidmGf2BadEncodingProof {
    /// Verify the proof against the namespaced VID commitment. `common`
    /// supplies the erasure parameters, which the commitment does not bind.
    pub fn verify(
        &self,
        commit: &NsAvidmGf2Commit,
        common: &NsAvidmGf2Common,
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
        self.ns_proof.verify(&common.param, &self.ns_commit)
    }
}

#[cfg(test)]
mod tests {
    use rand::seq::SliceRandom;

    use crate::{
        VidError, VidScheme,
        avidm_gf2::{
            AvidmGf2Param, AvidmGf2Scheme, AvidmGf2Share,
            namespaced::{NsAvidmGf2Commit, NsAvidmGf2Common, NsAvidmGf2Scheme, NsAvidmGf2Share},
            proofs::AvidmGf2BadEncodingProof,
        },
    };

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

    const DISTRIBUTION: [u32; 6] = [1, 2, 1, 3, 1, 2];
    const NS_TABLE: [std::ops::Range<usize>; 5] =
        [(0..10), (10..21), (21..33), (33..48), (48..100)];

    /// A four-of-ten dispersal of a hundred-byte payload over five namespaces.
    fn param() -> AvidmGf2Param {
        AvidmGf2Scheme::setup(4, 10).unwrap()
    }

    fn payload() -> Vec<u8> {
        (0..100).map(|i| i as u8).collect()
    }

    fn ns_disperse(
        non_codeword: bool,
    ) -> (NsAvidmGf2Commit, NsAvidmGf2Common, Vec<NsAvidmGf2Share>) {
        let param = param();
        let payload = payload();
        let ns_table = NS_TABLE.iter().cloned();
        if non_codeword {
            NsAvidmGf2Scheme::ns_disperse_non_codeword(&param, &DISTRIBUTION, &payload, ns_table)
        } else {
            NsAvidmGf2Scheme::ns_disperse(&param, &DISTRIBUTION, &payload, ns_table)
        }
        .unwrap()
    }

    fn weight(shares: &[AvidmGf2Share]) -> usize {
        shares.iter().map(AvidmGf2Share::weight).sum()
    }

    #[test]
    fn test_ns_proof_of_incorrect_encoding() {
        let mut rng = jf_utils::test_rng();
        let param = param();
        let (commit, common, mut shares) = ns_disperse(true);
        for share in &shares {
            assert!(
                NsAvidmGf2Scheme::verify_share(&commit, &common, share).is_ok_and(|r| r.is_ok())
            );
        }
        shares.shuffle(&mut rng);

        for ns_index in 0..NS_TABLE.len() {
            let proof = NsAvidmGf2Scheme::proof_of_incorrect_encoding_for_namespace(
                &commit, &common, ns_index, &shares,
            )
            .unwrap();
            assert_eq!(proof.ns_index, ns_index);
            assert_eq!(proof.ns_commit, common.ns_commits[ns_index]);
            assert_eq!(weight(proof.ns_proof.shares()), param.recovery_threshold);
            assert!(proof.verify(&commit, &common).unwrap().is_ok());

            // The proof is pinned to its namespace and its commitment.
            let mut moved = proof.clone();
            moved.ns_index = (ns_index + 1) % NS_TABLE.len();
            assert!(moved.verify(&commit, &common).unwrap().is_err());
            let (other_commit, ..) = ns_disperse(false);
            assert!(proof.verify(&other_commit, &common).unwrap().is_err());
        }

        // Too little weight, an unknown namespace, a common the commitment does
        // not bind: no proof.
        let mut few = vec![];
        for share in &shares {
            if weight(&few) + share.weight() >= param.recovery_threshold {
                break;
            }
            few.push(share.inner_ns_share(0).unwrap());
        }
        assert!(matches!(
            AvidmGf2Scheme::proof_of_incorrect_encoding(&param, &common.ns_commits[0], &few),
            Err(VidError::InsufficientShares)
        ));
        assert!(matches!(
            NsAvidmGf2Scheme::proof_of_incorrect_encoding_for_namespace(
                &commit,
                &common,
                NS_TABLE.len(),
                &shares
            ),
            Err(VidError::IndexOutOfBound)
        ));
        let mut swapped = common.clone();
        swapped.ns_commits.swap(0, 1);
        assert!(matches!(
            NsAvidmGf2Scheme::proof_of_incorrect_encoding_for_namespace(
                &commit, &swapped, 0, &shares
            ),
            Err(VidError::InvalidParam)
        ));
    }

    #[test]
    fn honest_shares_cannot_prove_incorrect_encoding() {
        let param = param();
        let (commit, common, shares) = ns_disperse(false);
        for ns_index in 0..NS_TABLE.len() {
            assert!(matches!(
                NsAvidmGf2Scheme::proof_of_incorrect_encoding_for_namespace(
                    &commit, &common, ns_index, &shares
                ),
                Err(VidError::Argument(_))
            ));
        }
        // A proof assembled by hand from genuine shares fails verification
        // rather than being malformed.
        let ns_shares: Vec<_> = shares
            .iter()
            .map(|share| share.inner_ns_share(0).unwrap())
            .collect();
        let proof = AvidmGf2BadEncodingProof {
            shares: AvidmGf2Scheme::threshold_cover(&param, ns_shares.iter()).unwrap(),
        };
        assert_eq!(weight(proof.shares()), param.recovery_threshold);
        assert_eq!(
            proof.verify(&param, &common.ns_commits[0]).unwrap(),
            Err(())
        );
    }

    #[test]
    fn bad_encoding_proof_rejects_tampering() {
        let param = param();
        let (commit, shares) =
            AvidmGf2Scheme::disperse_non_codeword(&param, &DISTRIBUTION, &payload()).unwrap();
        let proof = AvidmGf2Scheme::proof_of_incorrect_encoding(&param, &commit, &shares).unwrap();
        assert!(proof.verify(&param, &commit).unwrap().is_ok());

        // Overlapping ranges could pass one shard off as several.
        let mut doubled = proof.clone();
        doubled.shares.push(doubled.shares[0].clone());
        assert!(matches!(
            doubled.verify(&param, &commit),
            Err(VidError::InvalidShare)
        ));

        // Fewer positions than the threshold decode to nothing in particular.
        let mut short = proof.clone();
        short.shares.pop();
        assert!(matches!(
            short.verify(&param, &commit),
            Err(VidError::InvalidParam)
        ));

        // A share of another dispersal does not verify against this commitment.
        let (other_commit, other_shares) =
            AvidmGf2Scheme::disperse_non_codeword(&param, &DISTRIBUTION, &[9u8; 100]).unwrap();
        let mut foreign = proof.clone();
        foreign.shares[0] = other_shares
            .iter()
            .find(|share| share.range() == foreign.shares[0].range())
            .unwrap()
            .clone();
        assert_eq!(foreign.verify(&param, &commit).unwrap(), Err(()));
        assert_eq!(proof.verify(&param, &other_commit).unwrap(), Err(()));
    }

    /// Shard sets no payload produces still decide with every share verifying,
    /// so they need a proof too: a codeword whose originals carry no padding
    /// delimiter, and one padded to a shard size other than the payload's.
    #[test]
    fn bad_encoding_proof_covers_shard_sets_no_payload_produces() {
        let param = param();
        let recovery_count = param.total_weights - param.recovery_threshold;

        let originals = vec![vec![0u8; 2]; param.recovery_threshold];
        let recovery =
            reed_solomon_simd::encode(param.recovery_threshold, recovery_count, &originals)
                .unwrap();
        let (commit, shares) =
            AvidmGf2Scheme::disperse_shards(&param, &DISTRIBUTION, [originals, recovery].concat())
                .unwrap();
        assert!(AvidmGf2Scheme::recover(&param, &commit, &shares).is_err());
        let proof = AvidmGf2Scheme::proof_of_incorrect_encoding(&param, &commit, &shares).unwrap();
        assert!(proof.verify(&param, &commit).unwrap().is_ok());

        // Five bytes fill two-byte shards; four-byte shards decode to the same
        // bytes, which re-commit at two.
        let payload = [7u8; 5];
        let originals =
            AvidmGf2Scheme::chunk_and_pad(&payload, 4, param.recovery_threshold).unwrap();
        let recovery =
            reed_solomon_simd::encode(param.recovery_threshold, recovery_count, &originals)
                .unwrap();
        let (commit, shares) =
            AvidmGf2Scheme::disperse_shards(&param, &DISTRIBUTION, [originals, recovery].concat())
                .unwrap();
        assert_eq!(
            AvidmGf2Scheme::recover(&param, &commit, &shares).unwrap(),
            payload
        );
        assert_ne!(AvidmGf2Scheme::commit(&param, &payload).unwrap(), commit);
        let proof = AvidmGf2Scheme::proof_of_incorrect_encoding(&param, &commit, &shares).unwrap();
        assert!(proof.verify(&param, &commit).unwrap().is_ok());
    }
}
