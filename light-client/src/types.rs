use anyhow::{ensure, Context, Result};
use committable::Committable;
use espresso_types::{Leaf2, SeqTypes};
use hotshot_query_service::availability::LeafQueryData;
use hotshot_types::simple_certificate::QuorumCertificate2;
use serde::{Deserialize, Serialize};

/// A proof that a leaf is finalized.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LeafProof {
    /// A chain of leaves from the requested leaf to a provably finalized leaf.
    ///
    /// The chain is in chronological order, so `leaves[0]` is the requested leaf and
    /// `leaves.last()` is a leaf which is known or can be proven to be finalized. The chain is
    /// joined by `parent_commitment`, so it can be validated by recomputing the commitment of each
    /// leaf and comparing to the parent commitment of the next.
    pub leaves: Vec<Leaf2>,

    /// A chain of quorum certificates proving finality for the last leaf in `leaves`.
    ///
    /// The requirements for checking finality of a leaf given a 2-chain of QCs are:
    /// * `qcs[0].data.leaf_commit == leaf.commit()`
    /// * `qcs[0].view_number == leaf.view_number()`
    /// * `qcs[1].view_number == qcs[0].view_number + 1`
    /// * Both QCs have a valid threshold signature given a stake table
    ///
    /// These QCs are provided only if they are necessary to prove the last leaf in `leaves`
    /// finalized. If the last leaf is the parent of a known finalized leaf (that is, its commitment
    /// is equal to the `parent_commitment` field of a leaf which is already known to be finalized)
    /// these QCs are omitted.
    pub qcs: Option<[QuorumCertificate2<SeqTypes>; 2]>,
}

impl LeafProof {
    /// Verify the proof.
    ///
    /// If successful, returns the leaf which is proven finalized.
    pub fn verify(&self, finalized: Option<&Leaf2>) -> Result<LeafQueryData<SeqTypes>> {
        let mut leaves = self.leaves.iter();
        let leaf = leaves.next().context("empty leaf chain")?;
        let mut opt_qc = None;

        // Verify chaining by recomputing hashes.
        let mut curr = leaf;
        for next in leaves {
            ensure!(Committable::commit(curr) == next.parent_commitment());
            curr = next;

            if opt_qc.is_none() {
                // Get the QC signing `leaf` from the justify QC of the subsequent leaf.
                opt_qc = Some(next.justify_qc().clone());
            }
        }

        // Check that the final leaf is actually finalized.
        let qc;
        if let Some(finalized) = finalized {
            ensure!(Committable::commit(curr) == finalized.parent_commitment());

            // If the final leaf is also the requested leaf, save the QC which proves it finalized.
            qc = opt_qc.unwrap_or_else(|| finalized.justify_qc().clone());
        } else {
            let qcs = self
                .qcs
                .as_ref()
                .context("no finalized leaf and no QC chain provided")?;
            ensure!(qcs[0].view_number == curr.view_number());
            ensure!(qcs[0].data.leaf_commit == Committable::commit(curr));
            ensure!(qcs[1].view_number == qcs[0].view_number + 1);
            // TODO check threshold signatures

            // If the final leaf is also the requested leaf, save the QC which proves it finalized.
            qc = opt_qc.unwrap_or_else(|| qcs[0].clone());
        }

        let info = LeafQueryData::new(leaf.clone(), qc)?;
        Ok(info)
    }

    /// Append a new leaf to the proof's chain.
    ///
    /// Returns `true` if and only if we have enough data to prove at least the first leaf in the
    /// chain finalized.
    pub fn push(&mut self, leaf: LeafQueryData<SeqTypes>) -> bool {
        // Check if the new leaf forms a 2-chain.
        if let Some(last) = self.leaves.last() {
            let justify_qc = leaf.leaf().justify_qc();
            let qc = leaf.qc();
            if qc.view_number == justify_qc.view_number + 1
                && justify_qc.data.leaf_commit == Committable::commit(last)
            {
                self.qcs = Some([justify_qc, qc.clone()]);
                return true;
            }
        }

        self.leaves.push(leaf.leaf().clone());
        false
    }
}
