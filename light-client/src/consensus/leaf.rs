use std::sync::Arc;

use anyhow::{ensure, Context, Result};
use committable::Committable;
use espresso_types::{EpochVersion, Leaf2, SeqTypes};
use hotshot_query_service::availability::LeafQueryData;
use hotshot_types::vote::HasViewNumber;
use serde::{Deserialize, Serialize};
use vbs::version::StaticVersionType;

use super::quorum::{Certificate, Quorum};

/// Data sufficient to convince a client that a certain leaf is finalized.
///
/// There are different types of proofs for different scenarios and protocol versions. New proof
/// types can be added while remaining compatible with old serialized proofs.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum FinalityProof {
    /// The client has stated that they already trust the finality of this particular leaf.
    #[default]
    Assumption,

    /// The finality follows from a 2-chain of QCs using the HotStuff2 commit rule.
    ///
    /// The requirements for checking finality of a leaf given a 2-chain of QCs are:
    /// * The leaf has a protocol version indicating it was created via HotStuff2
    /// * `committing_qc.leaf_commit() == leaf.commit()`
    /// * `committing_qc.view_number() == leaf.view_number()`
    /// * `deciding_qc.view_number() == committing_qc.view_number() + 1`
    /// * Both QCs have a valid threshold signature given a stake table
    HotStuff2 {
        committing_qc: Arc<Certificate>,
        deciding_qc: Arc<Certificate>,
    },

    /// The finality follows from a 3-chain of QCs using the original HotStuff commit rule.
    ///
    /// The requirements for checking finality of a leaf via the 3-chain rule are similar to the
    /// `HotStuff2` finality rule, but an extra QC is required with a consecutive view number.
    HotStuff {
        precommit_qc: Arc<Certificate>,
        committing_qc: Arc<Certificate>,
        deciding_qc: Arc<Certificate>,
    },
}

/// A proof that a leaf is finalized.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LeafProof {
    /// A chain of leaves from the requested leaf to a provably finalized leaf.
    ///
    /// The chain is in chronological order, so `leaves[0]` is the requested leaf and
    /// `leaves.last()` is a leaf which is known or can be proven to be finalized. The chain is
    /// joined by `parent_commitment`, so it can be validated by recomputing the commitment of each
    /// leaf and comparing to the parent commitment of the next.
    leaves: Vec<Leaf2>,

    /// Some extra data proving finality for the last leaf in `leaves`.
    proof: FinalityProof,
}

impl LeafProof {
    /// Verify the proof.
    ///
    /// If successful, returns the leaf which is proven finalized.
    pub async fn verify(
        &self,
        quorum: &impl Quorum,
        finalized: Option<&Leaf2>,
    ) -> Result<LeafQueryData<SeqTypes>> {
        let mut leaves = self.leaves.iter();
        let leaf = leaves.next().context("empty leaf chain")?;

        // The QC signing `leaf`. This either comes from the next leaf in the chain, or from the
        // final QC chain in case the leaf chain contains only this single leaf.
        let mut opt_qc = None;

        // Verify chaining by recomputing hashes.
        let mut curr = leaf;
        for next in leaves {
            ensure!(curr.commit() == next.parent_commitment());
            curr = next;

            if opt_qc.is_none() {
                // Get the QC signing `leaf` from the justify QC of the subsequent leaf.
                opt_qc = Some(next.justify_qc().clone());
            }
        }

        // Check that the final leaf is actually finalized and get the QC which signs it.
        let final_qc = match &self.proof {
            FinalityProof::Assumption => {
                // The prover claims that we already have a finalized leaf whose parent is the
                // current leaf.
                let finalized = finalized.context("no finalized leaf and no QC chain provided")?;
                ensure!(finalized.parent_commitment() == curr.commit());

                finalized.justify_qc()
            },
            FinalityProof::HotStuff2 {
                committing_qc,
                deciding_qc,
            } => {
                // Check that the given QCs form a 2-chain, which proves `curr` finalized under
                // HotStuff2.
                let version = quorum
                    .verify_qc_chain_and_get_version(curr, [&**committing_qc, &**deciding_qc])
                    .await?;

                // Check that HotStuff2 is the appropriate commit rule to use. HotStuff2 commit rule
                // was introduced with the epochs version of HotShot.
                ensure!(version >= EpochVersion::version());

                // If the current leaf is also the requested leaf, return the QC which signs it.
                committing_qc.qc().clone()
            },
            FinalityProof::HotStuff {
                precommit_qc,
                committing_qc,
                deciding_qc,
            } => {
                // Check that the given QCs form a 3-chain, which proves `curr` finalized under
                // HotStuff.
                let version = quorum
                    .verify_qc_chain_and_get_version(
                        curr,
                        [&**precommit_qc, &**committing_qc, &**deciding_qc],
                    )
                    .await?;

                // Check that HotStuff is the appropriate commit rule to use. HotStuff commit rule
                // was deprecated with the epochs version of HotShot.
                ensure!(version < EpochVersion::version());

                // If the current leaf is also the requested leaf, save the QC which signs it.
                precommit_qc.qc().clone()
            },
        };

        // Take the QC which signs the requested leaf, either the one we saved earlier, or the one
        // signing the latest leaf in case the latest leaf is also the requested leaf.
        let qc = opt_qc.unwrap_or(final_qc);

        let info = LeafQueryData::new(leaf.clone(), qc)?;
        Ok(info)
    }

    /// Append a new leaf to the proof's chain.
    ///
    /// Returns `true` if and only if we have enough data to prove at least the first leaf in the
    /// chain finalized.
    pub fn push(&mut self, new_leaf: LeafQueryData<SeqTypes>) -> bool {
        let len = self.leaves.len();

        // Check if the new leaf plus the last saved leaf contain justifying QCs that form a
        // HotStuff2 QC chain for the leaf before.
        if len >= 2 && self.leaves[len - 2].block_header().version() >= EpochVersion::version() {
            let committing_qc = Certificate::for_parent(&self.leaves[len - 1]);
            let deciding_qc = Certificate::for_parent(new_leaf.leaf());
            if committing_qc.view_number() == self.leaves[len - 2].view_number()
                && deciding_qc.view_number() == committing_qc.view_number() + 1
            {
                self.proof = FinalityProof::HotStuff2 {
                    committing_qc: Arc::new(committing_qc),
                    deciding_qc: Arc::new(deciding_qc),
                };

                // We don't actually need the last leaf in the chain, we just needed it for its
                // extra justifying QC.
                self.leaves.pop();

                return true;
            }
        }

        // Check if the new leaf plus the last saved leaf contain QCs that form a legacy HotStuff
        // QC chain for the leaf before.
        if len >= 3 && self.leaves[len - 3].block_header().version() < EpochVersion::version() {
            let precommit_qc = Certificate::for_parent(&self.leaves[len - 2]);
            let committing_qc = Certificate::for_parent(&self.leaves[len - 1]);
            let deciding_qc = Certificate::for_parent(new_leaf.leaf());
            if precommit_qc.view_number() == self.leaves[len - 3].view_number()
                && committing_qc.view_number() == precommit_qc.view_number() + 1
                && deciding_qc.view_number() == committing_qc.view_number() + 1
            {
                self.proof = FinalityProof::HotStuff {
                    precommit_qc: Arc::new(precommit_qc),
                    committing_qc: Arc::new(committing_qc),
                    deciding_qc: Arc::new(deciding_qc),
                };

                // We don't actually need the last two leaves in the chain, we just needed them for
                // their extra justifying QCs,.
                self.leaves.pop();
                self.leaves.pop();

                return true;
            }
        }

        // Nothing is finalized yet, just save the new leaf.
        self.leaves.push(new_leaf.leaf().clone());
        false
    }

    /// Complete a finality proof by appending 2 QCs which extend from the last pushed leaf.
    ///
    /// This is meant to be called by the prover and so it is assumed that the provided QCs
    /// correctly form a 2-chain and that the protocol version is HotStuff2. If these conditions are
    /// met, this function will not fail but may produce a proof which fails to verify.
    pub fn add_qc_chain(&mut self, committing_qc: Arc<Certificate>, deciding_qc: Arc<Certificate>) {
        self.proof = FinalityProof::HotStuff2 {
            committing_qc,
            deciding_qc,
        };
    }

    /// Inspect the raw finality proof within the larger proof.
    pub fn proof(&self) -> &FinalityProof {
        &self.proof
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::testing::{leaf_chain, AlwaysFalseQuorum, AlwaysTrueQuorum, LegacyVersion};

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_hotstuff2() {
        let mut proof = LeafProof::default();

        // Insert some leaves, forming a chain.
        let leaves = leaf_chain::<EpochVersion>(1..=3).await;
        assert!(!proof.push(leaves[0].clone()));
        assert!(!proof.push(leaves[1].clone()));
        assert!(proof.push(leaves[2].clone()));
        assert_eq!(
            proof.verify(&AlwaysTrueQuorum, None).await.unwrap(),
            leaves[0]
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_invalid_qc() {
        let mut proof = LeafProof::default();

        // Insert some leaves, forming a chain.
        let leaves = leaf_chain::<EpochVersion>(1..=3).await;
        assert!(!proof.push(leaves[0].clone()));
        assert!(!proof.push(leaves[1].clone()));
        assert!(proof.push(leaves[2].clone()));

        // The proof is otherwise valid...
        assert_eq!(
            proof.verify(&AlwaysTrueQuorum, None).await.unwrap(),
            leaves[0]
        );
        // ...but fails if the signatures are not valid.
        proof.verify(&AlwaysFalseQuorum, None).await.unwrap_err();
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_assumption() {
        let mut proof = LeafProof::default();

        // Insert a single leaf. We will not be able to provide proofs ending in a leaf chain, but
        // we can return a leaf if the leaf after it is already known to be finalized.
        let leaves = leaf_chain::<EpochVersion>(1..=2).await;
        assert!(!proof.push(leaves[0].clone()));
        assert_eq!(
            proof
                .verify(&AlwaysTrueQuorum, Some(leaves[1].leaf()))
                .await
                .unwrap(),
            leaves[0]
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_no_chain() {
        let mut proof = LeafProof::default();

        // Insert multiple leaves that don't chain. We will not be able to prove these are
        // finalized.
        let leaves = leaf_chain::<EpochVersion>(1..=4).await;
        assert!(!proof.push(leaves[0].clone()));
        assert!(!proof.push(leaves[2].clone()));

        // Even if we start from a finalized leave that extends one of the leaves we do have (4,
        // extends 3) we fail to generate a proof because we can't generate a chain from the
        // requested leaf (1) to the finalized leaf (4), since leaf 2 is missing.
        proof
            .verify(&AlwaysTrueQuorum, Some(leaves[3].leaf()))
            .await
            .unwrap_err();
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_final_qcs() {
        let mut proof = LeafProof::default();

        // Insert a single leaf, plus an extra QC chain proving it finalized.
        let leaves = leaf_chain::<EpochVersion>(1..=3).await;
        assert!(!proof.push(leaves[0].clone()));
        proof.add_qc_chain(
            Arc::new(Certificate::for_parent(leaves[1].leaf())),
            Arc::new(Certificate::for_parent(leaves[2].leaf())),
        );
        assert_eq!(
            proof.verify(&AlwaysTrueQuorum, None,).await.unwrap(),
            leaves[0]
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_legacy_hotstuff_three_chain() {
        let mut proof = LeafProof::default();

        // Insert some leaves, forming a chain.
        let leaves = leaf_chain::<LegacyVersion>(1..=4).await;
        assert!(!proof.push(leaves[0].clone()));
        assert!(!proof.push(leaves[1].clone()));
        assert!(!proof.push(leaves[2].clone()));
        assert!(proof.push(leaves[3].clone()));
        assert_eq!(
            proof.verify(&AlwaysTrueQuorum, None,).await.unwrap(),
            leaves[0]
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_legacy_hotstuff_two_chain_only() {
        let mut proof = LeafProof::default();

        // Insert some leaves, forming a 2-chain but not the 3-chain required to decide in legacy
        // HotStuff.
        let leaves = leaf_chain::<LegacyVersion>(1..=3).await;
        assert!(!proof.push(leaves[0].clone()));
        assert!(!proof.push(leaves[1].clone()));
        assert!(!proof.push(leaves[2].clone()));
        proof.verify(&AlwaysTrueQuorum, None).await.unwrap_err();
    }
}
