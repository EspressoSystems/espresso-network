//! Two-level Merkle tree for reward account balances.
//!
//! Uses an outer tree (4 bits, 16 partitions) of inner trees (156 bits each) for efficient
//! storage and retrieval across a 160-bit address space.
//!
//! # Storage Implementations
//!
//! - [`storage::CachedInMemoryStorage`] - In-memory with cache (default)
//! - [`fs_storage::RewardMerkleTreeFSStorage`] - File system backed, persistent
//!
//! # Example
//!
//! ```rust,ignore
//! let mut tree = RewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);
//! tree.update(&account, &amount)?;
//! let result = tree.lookup(&account);
//! ```

use std::{borrow::Borrow, sync::Arc};

use jf_merkle_tree_compat::{
    internal::MerkleTreeIter,
    prelude::{MerkleNode, MerkleProof},
    universal_merkle_tree::UniversalMerkleTree,
    DigestAlgorithm, ForgetableMerkleTreeScheme, ForgetableUniversalMerkleTreeScheme, LookupResult,
    MerkleCommitment, MerkleTreeCommitment, MerkleTreeError, MerkleTreeScheme,
    UniversalMerkleTreeScheme,
};
use serde::{Deserialize, Serialize};
use sha3::{Digest as _, Keccak256};

use crate::{
    reward_mt::storage::{OuterIndex, RewardMerkleTreeStorage},
    sparse_mt::Keccak256Hasher,
    v0::{sparse_mt::KeccakNode, v0_3::RewardAmount, v0_4::RewardAccountV2},
};

pub mod fs_storage;
pub mod storage;

/// Total height of the reward Merkle tree (160 bits = Ethereum address space)
pub const REWARD_MERKLE_TREE_V2_HEIGHT: usize = 160;

/// Arity of the Merkle tree (binary tree)
pub const REWARD_MERKLE_TREE_V2_ARITY: usize = 2;

/// Two-level reward Merkle tree with pluggable storage backend.
///
/// The tree uses an outer tree (4 bits) to index 16 inner trees (156 bits each).
/// Storage determines persistence strategy (in-memory, file system, etc).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RewardMerkleTreeV2Impl<S: RewardMerkleTreeStorage> {
    /// Outer tree storing roots of 16 inner trees
    outer: OuterRewardMerkleTreeV2,
    /// Total number of accounts across all inner trees
    num_leaves: u64,
    /// Storage backend for inner tree roots
    storage: S,
}

// Manual Hash implementation that only hashes the outer tree
// The storage is an implementation detail and doesn't affect logical equality
impl<S: RewardMerkleTreeStorage> std::hash::Hash for RewardMerkleTreeV2Impl<S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.outer.hash(state);
    }
}

/// Merkle tree commitment
pub type RewardMerkleCommitmentV2 = MerkleTreeCommitment<KeccakNode>;

/// Membership proof for account balance in the tree
pub type RewardMerkleProof =
    MerkleProof<RewardAmount, RewardAccountV2, KeccakNode, REWARD_MERKLE_TREE_V2_ARITY>;

/// Membership proof for inner tree root in outer tree
type OuterRewardMerkleProof =
    MerkleProof<KeccakNode, OuterIndex, KeccakNode, REWARD_MERKLE_TREE_V2_ARITY>;

/// Merkle node containing reward amount
type RewardMerkleNode = MerkleNode<RewardAmount, RewardAccountV2, KeccakNode>;

/// Height of outer tree (4 bits = 16 partitions)
const REWARD_MERKLE_TREE_V2_OUTER_HEIGHT: usize = 4;

/// Height of each inner tree (156 bits)
const REWARD_MERKLE_TREE_V2_INNER_HEIGHT: usize =
    REWARD_MERKLE_TREE_V2_HEIGHT - REWARD_MERKLE_TREE_V2_OUTER_HEIGHT;

/// Inner tree type: stores account balances within a partition
type InnerRewardMerkleTreeV2 = UniversalMerkleTree<
    RewardAmount,
    Keccak256Hasher,
    RewardAccountV2,
    REWARD_MERKLE_TREE_V2_ARITY,
    KeccakNode,
>;

/// Outer tree type: stores roots of 16 inner trees
type OuterRewardMerkleTreeV2 = UniversalMerkleTree<
    KeccakNode,
    OuterKeccak256Hasher,
    OuterIndex,
    REWARD_MERKLE_TREE_V2_ARITY,
    KeccakNode,
>;

/// Keccak256 hasher for outer tree (inner tree roots).
///
/// Implements Solidity-compatible hashing:
/// - Leaves: pass-through (already hashed inner tree roots)
/// - Internal nodes: keccak256 of concatenated children
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct OuterKeccak256Hasher;

impl DigestAlgorithm<KeccakNode, OuterIndex, KeccakNode> for OuterKeccak256Hasher {
    fn digest(data: &[KeccakNode]) -> Result<KeccakNode, jf_merkle_tree_compat::MerkleTreeError> {
        let mut hasher = Keccak256::new();

        // Hash the concatenated node data directly (no domain separator)
        for node in data {
            hasher.update(node.as_ref());
        }

        let result = hasher.finalize();
        Ok(KeccakNode(result.into()))
    }

    fn digest_leaf(
        _pos: &OuterIndex,
        elem: &KeccakNode,
    ) -> Result<KeccakNode, jf_merkle_tree_compat::MerkleTreeError> {
        // Do not hash the root of the inner Merkle tree
        Ok(*elem)
    }
}

impl<S: RewardMerkleTreeStorage> RewardMerkleTreeV2Impl<S> {
    /// Tree arity (binary tree)
    pub const ARITY: usize = REWARD_MERKLE_TREE_V2_ARITY;

    /// Create a new empty tree with the given storage backend
    pub fn new_with_storage(storage: S) -> Self {
        Self {
            outer: OuterRewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_OUTER_HEIGHT),
            num_leaves: 0,
            storage,
        }
    }

    /// Reconstruct tree from a commitment (root hash) with given storage backend.
    ///
    /// Creates a sparse tree that can be populated with remember operations.
    pub fn from_commitment_with_storage(
        commitment: impl Borrow<RewardMerkleCommitmentV2>,
        storage: S,
    ) -> Self {
        let num_leaves = commitment.borrow().size();
        Self {
            outer: OuterRewardMerkleTreeV2::from_commitment(commitment),
            num_leaves,
            storage,
        }
    }

    /// Build tree from key-value pairs with given storage backend.
    ///
    /// # Arguments
    /// * `height` - Must be [`REWARD_MERKLE_TREE_V2_HEIGHT`] (160)
    /// * `data` - Iterator of (account, amount) pairs
    /// * `storage` - Storage backend for inner tree roots
    pub fn from_kv_set_with_storage<BI, BE>(
        height: usize,
        data: impl IntoIterator<Item = impl Borrow<(BI, BE)>>,
        storage: S,
    ) -> Result<Self, MerkleTreeError>
    where
        BI: Borrow<RewardAccountV2>,
        BE: Borrow<RewardAmount>,
    {
        assert_eq!(
            height, REWARD_MERKLE_TREE_V2_HEIGHT,
            "Height must be {}",
            REWARD_MERKLE_TREE_V2_HEIGHT
        );
        let mut mt = Self::new_with_storage(storage);
        for tuple in data.into_iter() {
            let (key, value) = tuple.borrow();
            UniversalMerkleTreeScheme::update(&mut mt, key.borrow(), value.borrow())?;
        }
        Ok(mt)
    }

    /// Merge inner tree proof with outer tree proof into a single proof.
    ///
    /// The inner proof verifies the account in its partition, and the outer proof
    /// verifies that partition's root in the outer tree.
    fn patch_membership_proof(proof: &mut RewardMerkleProof, outer_proof: &OuterRewardMerkleProof) {
        outer_proof
            .proof
            .iter()
            .skip(1)
            .for_each(|node| match node {
                MerkleNode::Branch { value, children } => proof.proof.push(MerkleNode::Branch {
                    value: *value,
                    children: children
                        .iter()
                        .map(|node| {
                            Arc::new(if matches!(**node, MerkleNode::Empty) {
                                RewardMerkleNode::Empty
                            } else {
                                RewardMerkleNode::ForgettenSubtree {
                                    value: node.value(),
                                }
                            })
                        })
                        .collect::<Vec<_>>(),
                }),
                MerkleNode::Empty => proof.proof.push(RewardMerkleNode::Empty),
                _ => unreachable!("Proof nodes in outer tree should be branches or empty"),
            });
    }
}

/// Default reward Merkle tree with in-memory storage
pub type RewardMerkleTreeV2 = RewardMerkleTreeV2Impl<storage::CachedInMemoryStorage>;

// Convenience methods for the default cached storage implementation
impl RewardMerkleTreeV2 {
    /// Create a new empty tree with default in-memory storage
    ///
    /// # Arguments
    /// * `height` - Must be [`REWARD_MERKLE_TREE_V2_HEIGHT`] (160)
    pub fn new(height: usize) -> Self {
        assert_eq!(
            height, REWARD_MERKLE_TREE_V2_HEIGHT,
            "Height must be {}",
            REWARD_MERKLE_TREE_V2_HEIGHT
        );
        Self::new_with_storage(storage::CachedInMemoryStorage::new())
    }

    /// Reconstruct tree from a commitment with default in-memory storage
    ///
    /// Creates a sparse tree that can be populated with remember operations.
    pub fn from_commitment(commitment: impl Borrow<RewardMerkleCommitmentV2>) -> Self {
        Self::from_commitment_with_storage(commitment, storage::CachedInMemoryStorage::new())
    }

    /// Build tree from key-value pairs with default in-memory storage
    ///
    /// Build a universal merkle tree from a key-value set.
    /// * `height` - height of the merkle tree (must be REWARD_MERKLE_TREE_V2_HEIGHT = 160)
    /// * `data` - an iterator of key-value pairs. Could be a hashmap or simply
    ///   an array or a slice of (key, value) pairs
    pub fn from_kv_set<BI, BE>(
        height: usize,
        data: impl IntoIterator<Item = impl Borrow<(BI, BE)>>,
    ) -> Result<Self, MerkleTreeError>
    where
        BI: Borrow<RewardAccountV2>,
        BE: Borrow<RewardAmount>,
    {
        Self::from_kv_set_with_storage(height, data, storage::CachedInMemoryStorage::new())
    }
}

/// Verification result: Ok(()) means valid, Err(()) means invalid
pub type VerificationResult = Result<(), ()>;

/// Core Merkle tree operations: lookup, verification, commitment
impl<S: RewardMerkleTreeStorage> MerkleTreeScheme for RewardMerkleTreeV2Impl<S> {
    type Element = RewardAmount;
    type Index = RewardAccountV2;
    type MembershipProof = <InnerRewardMerkleTreeV2 as MerkleTreeScheme>::MembershipProof;
    type Commitment = RewardMerkleCommitmentV2;
    type NodeValue = KeccakNode;
    type BatchMembershipProof = ();

    const ARITY: usize = REWARD_MERKLE_TREE_V2_ARITY;

    /// Returns the tree height (160 bits for Ethereum address space)
    fn height(&self) -> usize {
        160
    }

    /// Returns the tree capacity (2^160 possible accounts)
    fn capacity(&self) -> bigdecimal::num_bigint::BigUint {
        bigdecimal::num_bigint::BigUint::from_slice(&[2]).pow(160)
    }

    /// Returns the number of accounts with non-zero balances
    fn num_leaves(&self) -> u64 {
        self.commitment().size()
    }

    /// Returns the root commitment (hash) of the tree
    fn commitment(&self) -> Self::Commitment {
        MerkleTreeCommitment::new(
            self.outer.commitment().digest(),
            REWARD_MERKLE_TREE_V2_HEIGHT,
            self.num_leaves,
        )
    }

    /// Look up an account's balance and generate a membership proof
    ///
    /// Returns the balance and proof if the account exists, NotFound otherwise.
    fn lookup(
        &self,
        index: impl Borrow<Self::Index>,
    ) -> LookupResult<Self::Element, Self::MembershipProof, ()> {
        let outer_index = OuterIndex::new(index.borrow());
        let outer_proof = match self.outer.lookup(outer_index) {
            LookupResult::Ok(_, proof) => proof,
            LookupResult::NotInMemory => {
                unreachable!("Outer reward merkle tree will never be forgetten.")
            },
            LookupResult::NotFound(_) => return LookupResult::NotFound(()),
        };
        match self.storage.lookup(index) {
            LookupResult::Ok(value, mut proof) => {
                Self::patch_membership_proof(&mut proof, &outer_proof);
                LookupResult::Ok(value, proof)
            },
            LookupResult::NotInMemory => LookupResult::NotInMemory,
            LookupResult::NotFound(_) => LookupResult::NotFound(()),
        }
    }

    /// Verify a membership proof against a root commitment
    ///
    /// Returns Ok(Ok(())) if proof is valid, Ok(Err(())) if invalid.
    fn verify(
        root: impl Borrow<Self::Commitment>,
        index: impl Borrow<Self::Index>,
        proof: impl Borrow<Self::MembershipProof>,
    ) -> Result<VerificationResult, MerkleTreeError> {
        InnerRewardMerkleTreeV2::verify(root, index, proof)
    }

    /// Iterate over all accounts and balances (not yet implemented)
    ///
    /// Note: This method is challenging to implement efficiently for the two-level tree structure
    /// because MerkleTreeIter requires borrowing from a single tree. Use `into_iter()` instead
    /// to consume the tree and iterate over all entries.
    fn iter(&'_ self) -> MerkleTreeIter<'_, Self::Element, Self::Index, Self::NodeValue> {
        todo!()
    }
}

/// Iterator over all accounts and balances
impl<S: RewardMerkleTreeStorage> IntoIterator for RewardMerkleTreeV2Impl<S> {
    type Item = (RewardAccountV2, RewardAmount);

    type IntoIter = <std::vec::Vec<(RewardAccountV2, RewardAmount)> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        // Collect all entries from all inner trees
        let indices = self.storage.indices();
        let mut all_entries = Vec::new();

        for outer_idx in indices {
            self.storage.with_tree(outer_idx, |root| {
                // Traverse the MerkleNode tree to collect all leaf entries
                collect_merkle_leaves(root, &mut all_entries);
            });
        }

        all_entries.into_iter()
    }
}

/// Helper function to recursively collect all leaf entries from a MerkleNode
fn collect_merkle_leaves(
    node: &storage::InnerRewardMerkleTreeRoot,
    entries: &mut Vec<(RewardAccountV2, RewardAmount)>,
) {
    match node {
        MerkleNode::Leaf { pos, elem, .. } => {
            entries.push((*pos, *elem));
        },
        MerkleNode::Branch { children, .. } => {
            for child in children.iter() {
                collect_merkle_leaves(child, entries);
            }
        },
        MerkleNode::Empty | MerkleNode::ForgettenSubtree { .. } => {
            // No leaves to collect
        },
    }
}

/// Universal Merkle tree operations: update, non-membership proofs
///
/// Supports proving both membership (account exists) and non-membership (account doesn't exist).
impl<S: RewardMerkleTreeStorage> UniversalMerkleTreeScheme for RewardMerkleTreeV2Impl<S> {
    type NonMembershipProof =
        <InnerRewardMerkleTreeV2 as UniversalMerkleTreeScheme>::NonMembershipProof;
    type BatchNonMembershipProof = ();

    /// Update an account's balance using a custom function
    ///
    /// The function receives the current balance (if any) and returns the new balance.
    /// Returning None removes the account. Updates both inner and outer trees.
    fn update_with<F>(
        &mut self,
        pos: impl Borrow<Self::Index>,
        f: F,
    ) -> Result<LookupResult<Self::Element, (), ()>, MerkleTreeError>
    where
        F: FnOnce(Option<&Self::Element>) -> Option<Self::Element>,
    {
        let outer_index = OuterIndex::new(pos.borrow());
        let (result, delta, root) = self.storage.update_with(pos, f)?;
        self.num_leaves = (self.num_leaves as i64 + delta) as u64;
        if root == KeccakNode::default() {
            self.outer.update_with(outer_index, |_| None)?;
        } else {
            self.outer.update(outer_index, root)?;
        }
        Ok(result)
    }

    /// Look up an account and generate proof (membership or non-membership)
    ///
    /// Returns membership proof if account exists, non-membership proof otherwise.
    fn universal_lookup(
        &self,
        index: impl Borrow<Self::Index>,
    ) -> LookupResult<Self::Element, Self::MembershipProof, Self::NonMembershipProof> {
        let index = index.borrow();
        let outer_index = OuterIndex::new(index);
        let outer_proof = match self.outer.universal_lookup(outer_index) {
            LookupResult::Ok(_, proof) => proof,
            LookupResult::NotInMemory => {
                unreachable!("Outer reward merkle tree will never be forgetten.")
            },
            LookupResult::NotFound(outer_proof) => {
                let mut proof = RewardMerkleProof::new(
                    *index,
                    vec![MerkleNode::Empty; REWARD_MERKLE_TREE_V2_INNER_HEIGHT + 1],
                );
                Self::patch_membership_proof(&mut proof, &outer_proof);
                return LookupResult::NotFound(proof);
            },
        };
        match self.storage.lookup(index) {
            LookupResult::Ok(value, mut proof) => {
                Self::patch_membership_proof(&mut proof, &outer_proof);
                LookupResult::Ok(value, proof)
            },
            LookupResult::NotInMemory => LookupResult::NotInMemory,
            LookupResult::NotFound(mut proof) => {
                Self::patch_membership_proof(&mut proof, &outer_proof);
                LookupResult::NotFound(proof)
            },
        }
    }

    /// Verify a non-membership proof against a root commitment
    ///
    /// Returns true if proof is valid (account doesn't exist), false otherwise.
    fn non_membership_verify(
        root: impl Borrow<Self::Commitment>,
        index: impl Borrow<Self::Index>,
        proof: impl Borrow<Self::NonMembershipProof>,
    ) -> Result<bool, MerkleTreeError> {
        InnerRewardMerkleTreeV2::non_membership_verify(root, index, proof)
    }
}

/// Forgetable operations: prune tree to reduce memory usage
///
/// Allows removing account data from memory while maintaining the root hash.
/// Accounts can be restored later with remember operations.
impl<S: RewardMerkleTreeStorage + Default> ForgetableMerkleTreeScheme
    for RewardMerkleTreeV2Impl<S>
{
    /// Reconstruct a sparse tree from a commitment with default storage
    ///
    /// Creates an empty tree with the given root. Accounts can be populated with remember.
    fn from_commitment(commitment: impl Borrow<Self::Commitment>) -> Self {
        let num_leaves = commitment.borrow().size();
        Self {
            outer: OuterRewardMerkleTreeV2::from_commitment(commitment),
            num_leaves,
            storage: S::default(),
        }
    }

    /// Remove an account from memory (not yet implemented, currently just performs lookup)
    ///
    /// Returns the account's balance and proof if it exists.
    fn forget(
        &mut self,
        index: impl Borrow<Self::Index>,
    ) -> LookupResult<Self::Element, Self::MembershipProof, ()> {
        self.lookup(index)
        // let outer_index = OuterIndex::new(index.borrow());
        // let outer_proof = match self.outer.lookup(outer_index) {
        //     LookupResult::Ok(_, proof) => proof,
        //     LookupResult::NotInMemory => {
        //         unreachable!("Outer reward merkle tree will never be forgetten.")
        //     },
        //     LookupResult::NotFound(_) => return LookupResult::NotFound(()),
        // };
        // match self.storage.forget(index) {
        //     LookupResult::Ok(value, mut proof) => {
        //         Self::patch_membership_proof(&mut proof, &outer_proof);
        //         LookupResult::Ok(value, proof)
        //     },
        //     LookupResult::NotInMemory => LookupResult::NotInMemory,
        //     LookupResult::NotFound(_) => LookupResult::NotFound(()),
        // }
    }

    /// Restore an account to memory using a proof (currently a no-op)
    ///
    /// Restores an account that was previously forgotten. Currently does nothing.
    fn remember(
        &mut self,
        _pos: impl Borrow<Self::Index>,
        _element: impl Borrow<Self::Element>,
        _proof: impl Borrow<Self::MembershipProof>,
    ) -> Result<(), MerkleTreeError> {
        Ok(())
    }
}

/// Forgetable universal operations: forget with non-membership proofs
impl<S: RewardMerkleTreeStorage + Default> ForgetableUniversalMerkleTreeScheme
    for RewardMerkleTreeV2Impl<S>
{
    /// Remove an account and generate proof (membership or non-membership)
    ///
    /// Currently just performs universal_lookup without actually removing from memory.
    fn universal_forget(
        &mut self,
        pos: Self::Index,
    ) -> LookupResult<Self::Element, Self::MembershipProof, Self::NonMembershipProof> {
        self.universal_lookup(pos)
    }

    /// Restore non-membership information using a proof (currently a no-op)
    ///
    /// Restores non-membership information for an account. Currently does nothing.
    fn non_membership_remember(
        &mut self,
        _pos: Self::Index,
        _proof: impl Borrow<Self::NonMembershipProof>,
    ) -> Result<(), MerkleTreeError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::U256;
    use jf_merkle_tree_compat::ToTraversalPath;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;

    use super::*;

    /// Generate a random reward account address
    fn random_account(rng: &mut impl Rng) -> RewardAccountV2 {
        let mut bytes = [0u8; 20];
        rng.fill(&mut bytes);
        RewardAccountV2::from(bytes)
    }

    /// Generate a random reward amount
    fn random_amount(rng: &mut impl Rng) -> RewardAmount {
        RewardAmount(U256::from(rng.gen::<u64>()))
    }

    #[test]
    fn test_to_traversal_path() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let account = random_account(&mut rng);
        let full_path = <RewardAccountV2 as ToTraversalPath<2>>::to_traversal_path(&account, 160);
        let outer_index = OuterIndex::new(&account);
        let outer_path = <OuterIndex as ToTraversalPath<2>>::to_traversal_path(&outer_index, 4);
        assert_eq!(
            &outer_path,
            &full_path[REWARD_MERKLE_TREE_V2_INNER_HEIGHT..]
        );
    }

    #[test]
    fn test_two_level_tree_matches_single_level() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);

        // Create two-level tree (our implementation)
        let mut two_level_tree = RewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);

        // Create single-level tree for comparison
        let mut single_level_tree = InnerRewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);

        // Insert random accounts
        let num_accounts = 20;
        let mut test_accounts = Vec::new();

        for _ in 0..num_accounts {
            let account = random_account(&mut rng);
            let amount = random_amount(&mut rng);
            test_accounts.push((account, amount));

            // Insert into both trees
            two_level_tree.update(account, amount).unwrap();
            single_level_tree.update(account, amount).unwrap();

            // Verify commitments match after each insertion
            assert_eq!(
                two_level_tree.commitment(),
                single_level_tree.commitment(),
                "Commitments should match after insertion"
            );
        }

        // Verify final state
        assert_eq!(
            two_level_tree.num_leaves(),
            single_level_tree.num_leaves(),
            "Number of leaves should match"
        );
    }

    #[test]
    fn test_lookup_and_proof_verification() {
        let mut rng = ChaCha20Rng::seed_from_u64(123);

        let mut tree = RewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);
        let mut accounts = Vec::new();

        // Insert several accounts
        for _ in 0..10 {
            let account = random_account(&mut rng);
            let amount = random_amount(&mut rng);
            accounts.push((account, amount));
            tree.update(account, amount).unwrap();
        }

        // Test lookup for each inserted account
        for (account, expected_amount) in &accounts {
            match tree.lookup(account) {
                LookupResult::Ok(amount, proof) => {
                    assert_eq!(amount, *expected_amount, "Amount should match");

                    // Verify the proof
                    let verification =
                        RewardMerkleTreeV2::verify(tree.commitment(), account, &proof).unwrap();
                    assert!(verification.is_ok(), "Proof should be valid");
                },
                _ => panic!("Account should be found"),
            }
        }

        // Test lookup for non-existent account
        let non_existent = random_account(&mut rng);
        match tree.lookup(non_existent) {
            LookupResult::NotFound(_) => {}, // Expected
            _ => panic!("Non-existent account should not be found"),
        }
    }

    #[test]
    fn test_universal_lookup() {
        let mut rng = ChaCha20Rng::seed_from_u64(456);

        let mut tree = RewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);

        let account = random_account(&mut rng);
        let amount = random_amount(&mut rng);

        // Insert account
        tree.update(account, amount).unwrap();

        // Test universal lookup for existing account
        match tree.universal_lookup(account) {
            LookupResult::Ok(found_amount, proof) => {
                assert_eq!(found_amount, amount, "Amount should match");

                // Verify membership proof
                let verification =
                    RewardMerkleTreeV2::verify(tree.commitment(), &account, &proof).unwrap();
                assert!(verification.is_ok(), "Membership proof should be valid");
            },
            _ => panic!("Account should be found with membership proof"),
        }

        // Test universal lookup for non-existent account
        let non_existent = random_account(&mut rng);
        match tree.universal_lookup(non_existent) {
            LookupResult::NotFound(proof) => {
                // Verify non-membership proof
                let is_valid = RewardMerkleTreeV2::non_membership_verify(
                    tree.commitment(),
                    &non_existent,
                    &proof,
                )
                .unwrap();
                assert!(is_valid, "Non-membership proof should be valid");
            },
            _ => panic!("Non-existent account should return non-membership proof"),
        }
    }

    #[test]
    fn test_update_existing_account() {
        let mut rng = ChaCha20Rng::seed_from_u64(101112);

        let mut two_level_tree = RewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);
        let mut single_level_tree = InnerRewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);

        let account = random_account(&mut rng);
        let amount1 = random_amount(&mut rng);
        let amount2 = random_amount(&mut rng);

        // Initial insert
        two_level_tree.update(account, amount1).unwrap();
        single_level_tree.update(account, amount1).unwrap();
        assert_eq!(two_level_tree.commitment(), single_level_tree.commitment());

        // Update existing account
        two_level_tree.update(account, amount2).unwrap();
        single_level_tree.update(account, amount2).unwrap();
        assert_eq!(
            two_level_tree.commitment(),
            single_level_tree.commitment(),
            "Commitments should match after update"
        );

        // Verify the updated value
        match two_level_tree.lookup(account) {
            LookupResult::Ok(amount, _) => {
                assert_eq!(amount, amount2, "Updated amount should match");
            },
            _ => panic!("Account should be found"),
        }

        // Number of leaves should remain the same (updated, not inserted new)
        assert_eq!(two_level_tree.num_leaves(), 1);
    }

    #[test]
    fn test_from_kv_set() {
        let mut rng = ChaCha20Rng::seed_from_u64(161718);

        // Generate test data
        let mut kv_pairs = Vec::new();
        for _ in 0..15 {
            let account = random_account(&mut rng);
            let amount = random_amount(&mut rng);
            kv_pairs.push((account, amount));
        }

        // Build two-level tree from kv set
        let two_level_tree =
            RewardMerkleTreeV2::from_kv_set(REWARD_MERKLE_TREE_V2_HEIGHT, &kv_pairs).unwrap();

        // Build single-level tree from same kv set
        let single_level_tree =
            InnerRewardMerkleTreeV2::from_kv_set(REWARD_MERKLE_TREE_V2_HEIGHT, &kv_pairs).unwrap();

        // Verify commitments match
        assert_eq!(
            two_level_tree.commitment(),
            single_level_tree.commitment(),
            "Commitments should match when built from same kv set"
        );

        // Verify all accounts can be looked up
        for (account, expected_amount) in &kv_pairs {
            match two_level_tree.lookup(account) {
                LookupResult::Ok(amount, _) => {
                    assert_eq!(amount, *expected_amount, "Amount should match");
                },
                _ => panic!("Account should be found"),
            }
        }
    }

    #[test]
    fn test_update_with_custom_function() {
        let mut rng = ChaCha20Rng::seed_from_u64(222324);

        let mut two_level_tree = RewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);
        let mut single_level_tree = InnerRewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);

        let account = random_account(&mut rng);
        let initial_amount = RewardAmount(U256::from(100u64));
        let increment = RewardAmount(U256::from(50u64));

        // Initial insert
        two_level_tree.update(account, initial_amount).unwrap();
        single_level_tree.update(account, initial_amount).unwrap();

        // Update using custom function (increment)
        two_level_tree
            .update_with(&account, |existing| {
                existing.map(|amt| RewardAmount(amt.0 + increment.0))
            })
            .unwrap();
        single_level_tree
            .update_with(&account, |existing| {
                existing.map(|amt| RewardAmount(amt.0 + increment.0))
            })
            .unwrap();

        // Verify commitments match
        assert_eq!(
            two_level_tree.commitment(),
            single_level_tree.commitment(),
            "Commitments should match after update_with"
        );

        // Verify the updated value
        match two_level_tree.lookup(account) {
            LookupResult::Ok(amount, _) => {
                assert_eq!(
                    amount,
                    RewardAmount(initial_amount.0 + increment.0),
                    "Amount should be incremented"
                );
            },
            _ => panic!("Account should be found"),
        }
    }

    #[test]
    fn test_remove_account() {
        let mut rng = ChaCha20Rng::seed_from_u64(252627);

        let mut two_level_tree = RewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);
        let mut single_level_tree = InnerRewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);

        let account = random_account(&mut rng);
        let amount = random_amount(&mut rng);

        // Insert account
        two_level_tree.update(account, amount).unwrap();
        single_level_tree.update(account, amount).unwrap();
        assert_eq!(two_level_tree.num_leaves(), 1);

        // Remove account by returning None
        two_level_tree.update_with(account, |_| None).unwrap();
        single_level_tree.update_with(account, |_| None).unwrap();

        // Verify commitments match
        assert_eq!(
            two_level_tree.commitment(),
            single_level_tree.commitment(),
            "Commitments should match after removal"
        );

        // Verify account is gone
        assert_eq!(two_level_tree.num_leaves(), 0);
        match two_level_tree.lookup(account) {
            LookupResult::NotFound(_) => {}, // Expected
            _ => panic!("Removed account should not be found"),
        }
    }

    #[test]
    fn test_stress_with_many_operations() {
        let mut rng = ChaCha20Rng::seed_from_u64(282930);

        let mut two_level_tree = RewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);
        let mut single_level_tree = InnerRewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);

        let mut known_accounts = Vec::new();

        // Perform many random operations
        for i in 0..50 {
            let op = rng.gen_range(0..3);

            match op {
                0 => {
                    // Insert new account
                    let account = random_account(&mut rng);
                    let amount = random_amount(&mut rng);
                    known_accounts.push((account, amount));

                    two_level_tree.update(account, amount).unwrap();
                    single_level_tree.update(account, amount).unwrap();
                },
                1 if !known_accounts.is_empty() => {
                    // Update existing account
                    let idx = rng.gen_range(0..known_accounts.len());
                    let (account, _) = known_accounts[idx];
                    let new_amount = random_amount(&mut rng);
                    known_accounts[idx].1 = new_amount;

                    two_level_tree.update(account, new_amount).unwrap();
                    single_level_tree.update(account, new_amount).unwrap();
                },
                2 if !known_accounts.is_empty() => {
                    // Remove account
                    let idx = rng.gen_range(0..known_accounts.len());
                    let (account, _) = known_accounts.remove(idx);

                    two_level_tree.update_with(&account, |_| None).unwrap();
                    single_level_tree.update_with(&account, |_| None).unwrap();
                },
                _ => continue,
            }

            // Verify commitments match after each operation
            assert_eq!(
                two_level_tree.commitment(),
                single_level_tree.commitment(),
                "Commitments should match after operation {}",
                i
            );
        }

        // Final verification
        assert_eq!(
            two_level_tree.num_leaves(),
            known_accounts.len() as u64,
            "Number of leaves should match known accounts"
        );
    }

    #[test]
    fn test_into_iter() {
        let mut rng = ChaCha20Rng::seed_from_u64(424344);

        let mut tree = RewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);
        let mut expected_entries = std::collections::HashMap::new();

        // Insert accounts across multiple partitions
        for _ in 0..20 {
            let account = random_account(&mut rng);
            let amount = random_amount(&mut rng);
            expected_entries.insert(account, amount);
            tree.update(account, amount).unwrap();
        }

        // Collect entries from iterator
        let collected_entries: std::collections::HashMap<_, _> = tree.into_iter().collect();

        // Verify all expected entries are present
        assert_eq!(
            collected_entries.len(),
            expected_entries.len(),
            "Iterator should return all entries"
        );

        for (account, expected_amount) in &expected_entries {
            let collected_amount = collected_entries
                .get(account)
                .expect("Account should be in iterator results");
            assert_eq!(
                collected_amount, expected_amount,
                "Amount should match for account {:?}",
                account
            );
        }
    }
}
