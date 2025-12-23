//! Types for fetching and verifying headers from an untrusted provider.

use anyhow::{anyhow, ensure, Context, Result};
use committable::Committable;
use espresso_types::{BlockMerkleTree, Header};
use jf_merkle_tree_compat::MerkleTreeScheme;
use serde::{Deserialize, Serialize};

/// A proof that a header is finalized, relative to some known-finalized leaf.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HeaderProof {
    header: Header,
    proof: <BlockMerkleTree as MerkleTreeScheme>::MembershipProof,
}

impl HeaderProof {
    /// Construct a [`HeaderProof`] from the header itself and a Merkle inclusion proof.
    pub fn new(
        header: Header,
        proof: <BlockMerkleTree as MerkleTreeScheme>::MembershipProof,
    ) -> Self {
        Self { header, proof }
    }

    /// Verify a [`HeaderProof`] and get the verified header if valid.
    pub fn verify(self, root: <BlockMerkleTree as MerkleTreeScheme>::Commitment) -> Result<Header> {
        self.verify_proof(root)?;
        Ok(self.header)
    }

    /// Verify a [`HeaderProof`] and get a reference to the verified header if valid.
    ///
    /// This is the same as [`verify`](Self::verify), but returns the result as a reference rather
    /// than consuming `self`.
    pub fn verify_ref(
        &self,
        root: <BlockMerkleTree as MerkleTreeScheme>::Commitment,
    ) -> Result<&Header> {
        self.verify_proof(root)?;
        Ok(&self.header)
    }

    fn verify_proof(&self, root: <BlockMerkleTree as MerkleTreeScheme>::Commitment) -> Result<()> {
        // Check that the proof is actually for the correct header, before verifying the proof
        // (which is slightly more expensive).
        ensure!(self.proof.elem() == Some(&self.header.commit()));
        ensure!(self.proof.index() == &self.header.height());

        BlockMerkleTree::verify(root, self.header.height(), &self.proof)
            .context("malformed proof")?
            .map_err(|()| {
                anyhow!(
                    "incorrect proof for element {} relative to {root}",
                    self.header.height()
                )
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use espresso_types::{EpochVersion, BLOCK_MERKLE_TREE_HEIGHT};
    use jf_merkle_tree_compat::{prelude::SHA3MerkleTree, AppendableMerkleTreeScheme};
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::testing::leaf_chain;

    #[tokio::test]
    #[test_log::test]
    async fn test_header_proof_valid() {
        let leaves = leaf_chain::<EpochVersion>(0..=3).await;

        // Use an appendable `MerkleTree` rather than a `BlockMerkleTree` (which is a
        // `LightweightMerkleTree`) so we can look up paths for previously inserted elements.
        let mut mt = SHA3MerkleTree::new(BLOCK_MERKLE_TREE_HEIGHT);
        for (root, leaf) in leaves.iter().enumerate() {
            for (height, expected) in leaves.iter().enumerate().take(root) {
                let proof = mt.lookup(height as u64).expect_ok().unwrap().1;
                let header = expected.header();
                let proof = HeaderProof::new(header.clone(), proof);
                assert_eq!(proof.verify_ref(mt.commitment()).unwrap(), header);
                assert_eq!(proof.verify(mt.commitment()).unwrap(), *header);
            }
            mt.push(leaf.block_hash()).unwrap();
        }
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_header_proof_invalid_wrong_path() {
        let leaves = leaf_chain::<EpochVersion>(0..=1).await;
        let mt = BlockMerkleTree::from_elems(
            Some(BLOCK_MERKLE_TREE_HEIGHT),
            [leaves[0].block_hash(), leaves[1].block_hash()],
        )
        .unwrap();
        let proof = HeaderProof::new(
            leaves[0].header().clone(),
            mt.lookup(1).expect_ok().unwrap().1,
        );
        proof.verify(mt.commitment()).unwrap_err();
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_header_proof_invalid_wrong_height() {
        let leaves = leaf_chain::<EpochVersion>(0..=1).await;
        let mts = [0, 1]
            .into_iter()
            .map(|height_diff| {
                BlockMerkleTree::from_elems(
                    Some(BLOCK_MERKLE_TREE_HEIGHT + height_diff),
                    [leaves[0].block_hash(), leaves[1].block_hash()],
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let proof = HeaderProof::new(
            leaves[0].header().clone(),
            mts[0].lookup(1).expect_ok().unwrap().1,
        );
        proof.verify(mts[1].commitment()).unwrap_err();
    }
}
