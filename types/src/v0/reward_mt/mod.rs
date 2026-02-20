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
    prelude::{MerkleNode, MerkleProof},
    universal_merkle_tree::UniversalMerkleTree,
    DigestAlgorithm, ForgetableMerkleTreeScheme, LookupResult, MerkleCommitment,
    MerkleTreeCommitment, MerkleTreeError, MerkleTreeScheme, UniversalMerkleTreeScheme,
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
/// # Architecture
///
/// This data structure implements a 160-bit Merkle tree (matching Ethereum's address space)
/// using a two-level approach for efficient storage and retrieval:
///
/// - **Outer tree**: 4-bit tree (16 partitions) that stores the roots of inner trees
/// - **Inner trees**: 16 separate 156-bit trees, one for each partition
///
/// The outer index is derived from the first 4 bits (most significant nibble) of an
/// Ethereum address, allowing accounts to be distributed across 16 partitions.
///
/// # Storage Backend
///
/// The storage backend determines how inner tree roots are persisted:
/// - [`storage::CachedInMemoryStorage`]: Fast in-memory storage with single-entry cache
/// - [`fs_storage::RewardMerkleTreeFSStorage`]: File system backed, survives restarts
///
/// # Benefits
///
/// - **Sparse storage**: Only non-empty partitions consume storage
/// - **Efficient lookups**: Operations only need to load one inner tree at a time
/// - **Parallelization**: Different partitions can be processed independently
/// - **Solidity compatibility**: Hashing matches on-chain verification
///
/// # Example
///
/// ```rust,ignore
/// let mut tree = InMemoryRewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);
/// tree.update(&account, &amount)?;
/// let LookupResult::Ok(balance, proof) = tree.lookup(&account)?;
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageBackedRewardMerkleTreeV2<S: RewardMerkleTreeStorage> {
    /// Outer tree storing roots of 16 inner trees (4-bit indexed)
    outer: OuterRewardMerkleTreeV2,
    /// Total number of accounts with non-zero balances across all inner trees
    num_leaves: u64,
    /// Storage backend for persisting inner tree roots
    storage: S,
}

// Manual Hash implementation that only hashes the outer tree
// The storage is an implementation detail and doesn't affect logical equality
impl<S: RewardMerkleTreeStorage> std::hash::Hash for StorageBackedRewardMerkleTreeV2<S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.outer.hash(state);
    }
}

/// Merkle tree commitment (root hash).
///
/// Contains the Keccak256 digest of the tree root along with metadata
/// about tree height and number of leaves. Used for efficient verification.
pub type RewardMerkleCommitmentV2 = MerkleTreeCommitment<KeccakNode>;

/// Membership proof for an account's balance in the tree.
///
/// Proves that a specific account has a specific balance at a given tree root.
/// Contains the merkle path from leaf to root with all sibling hashes needed
/// for verification. This proof can be verified on-chain in Solidity.
pub type RewardMerkleProof =
    MerkleProof<RewardAmount, RewardAccountV2, KeccakNode, REWARD_MERKLE_TREE_V2_ARITY>;

/// Non-membership proof for an account not in the tree.
///
/// Proves that a specific account does NOT exist in the tree. In a universal
/// Merkle tree, this is structurally identical to a membership proof, showing
/// the path to where the account would be if it existed (an empty position).
pub type RewardNonMembershipProof = RewardMerkleProof;

/// Membership proof for an inner tree root in the outer tree.
///
/// Internal type used when patching inner proofs with outer proofs. Proves
/// that a specific inner tree root exists at a specific partition index.
type OuterRewardMerkleProof =
    MerkleProof<KeccakNode, OuterIndex, KeccakNode, REWARD_MERKLE_TREE_V2_ARITY>;

/// Merkle node containing a reward amount leaf.
///
/// Internal representation of tree nodes. Can be Empty, Leaf (account + amount),
/// Branch (internal node with children), or ForgettenSubtree (sparse placeholder).
type RewardMerkleNode = MerkleNode<RewardAmount, RewardAccountV2, KeccakNode>;

/// Height of outer tree (4 bits = 16 partitions)
const REWARD_MERKLE_TREE_V2_OUTER_HEIGHT: usize = 4;

/// Height of each inner tree (156 bits)
const REWARD_MERKLE_TREE_V2_INNER_HEIGHT: usize =
    REWARD_MERKLE_TREE_V2_HEIGHT - REWARD_MERKLE_TREE_V2_OUTER_HEIGHT;

/// Inner tree type: 156-bit universal Merkle tree for account balances.
///
/// Each inner tree corresponds to one of the 16 partitions (indexed by the first
/// 4 bits of the account address). Stores reward amounts keyed by full account addresses.
/// Uses standard Keccak256 hashing for both leaves and internal nodes.
type InnerRewardMerkleTreeV2 = UniversalMerkleTree<
    RewardAmount,
    Keccak256Hasher,
    RewardAccountV2,
    REWARD_MERKLE_TREE_V2_ARITY,
    KeccakNode,
>;

/// Expected type alias for single-level reward Merkle tree.
///
/// Used in tests and comparisons to verify that the two-level implementation
/// produces identical commitments to a single-level tree.
pub type ExpectedRewardMerkleTreeV2 = InnerRewardMerkleTreeV2;

/// Outer tree type: 4-bit universal Merkle tree storing inner tree roots.
///
/// Contains up to 16 entries (2^4), each storing the root hash of one inner tree.
/// Uses a custom hasher (OuterKeccak256Hasher) that treats leaves as pre-hashed values.
type OuterRewardMerkleTreeV2 = UniversalMerkleTree<
    KeccakNode,
    OuterKeccak256Hasher,
    OuterIndex,
    REWARD_MERKLE_TREE_V2_ARITY,
    KeccakNode,
>;

/// Keccak256 hasher for outer tree nodes.
///
/// Implements Solidity-compatible hashing with special handling for leaves:
///
/// - **Leaf nodes**: Pass-through (no additional hashing) because inner tree roots
///   are already Keccak256 hashes. Adding another hash layer would break compatibility.
/// - **Internal nodes**: Standard `keccak256(left || right)` of concatenated children,
///   matching Solidity's abi.encodePacked + keccak256.
///
/// This ensures that proofs generated here can be verified on-chain using
/// the RewardClaim contract's merkle verification logic.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct OuterKeccak256Hasher;

impl DigestAlgorithm<KeccakNode, OuterIndex, KeccakNode> for OuterKeccak256Hasher {
    /// Hash internal nodes by concatenating children and applying Keccak256.
    ///
    /// Implements `keccak256(child[0] || child[1] || ... || child[n])` matching
    /// Solidity's `keccak256(abi.encodePacked(...))`.
    ///
    /// # Arguments
    /// * `data` - Slice of child node hashes to combine
    ///
    /// # Returns
    /// The Keccak256 hash of the concatenated children
    fn digest(data: &[KeccakNode]) -> Result<KeccakNode, jf_merkle_tree_compat::MerkleTreeError> {
        let mut hasher = Keccak256::new();

        // Concatenate all child hashes without domain separator
        for node in data {
            hasher.update(node.as_ref());
        }

        let result = hasher.finalize();
        Ok(KeccakNode(result.into()))
    }

    /// Pass-through for leaf nodes (inner tree roots are already hashed).
    ///
    /// Unlike typical Merkle trees where leaves are hashed, the outer tree's
    /// "leaves" are inner tree roots that are already Keccak256 hashes.
    /// Hashing them again would add an unnecessary layer and break Solidity
    /// verification compatibility.
    ///
    /// # Arguments
    /// * `_pos` - Outer index (0-15), unused
    /// * `elem` - Inner tree root hash (already a Keccak256 digest)
    ///
    /// # Returns
    /// The input hash unchanged
    fn digest_leaf(
        _pos: &OuterIndex,
        elem: &KeccakNode,
    ) -> Result<KeccakNode, jf_merkle_tree_compat::MerkleTreeError> {
        Ok(*elem)
    }
}

impl<S: RewardMerkleTreeStorage> StorageBackedRewardMerkleTreeV2<S> {
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
    ) -> anyhow::Result<Self>
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
            Self::update(&mut mt, key.borrow(), value.borrow())?;
        }
        Ok(mt)
    }

    /// Merge inner tree proof with outer tree proof into a single 160-bit proof.
    ///
    /// # Two-Level Proof Construction
    ///
    /// The two-level tree generates proofs in two stages:
    /// 1. **Inner proof** (156 bits): Proves account balance within its partition
    /// 2. **Outer proof** (4 bits): Proves partition root in outer tree
    ///
    /// This method combines them into a single 160-bit proof that can be verified
    /// against the tree's root commitment, matching the format expected by Solidity.
    ///
    /// # Implementation Details
    ///
    /// - Skips the first outer proof node (it's the inner tree root, already in inner proof)
    /// - Appends remaining outer proof nodes to the inner proof path
    /// - Converts outer tree nodes to inner tree node format (ForgettenSubtree placeholders)
    ///
    /// # Arguments
    /// * `proof` - Inner proof (156-bit) to extend (modified in place)
    /// * `outer_proof` - Outer proof (4-bit) to append
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

/// Two-level reward Merkle tree with in-memory storage (default).
///
/// Fast, non-persistent storage suitable for:
/// - Validators that don't store full reward state
/// - Testing and development
/// - Reconstructing state from block headers + catchup
///
/// Uses [`storage::CachedInMemoryStorage`] with single-entry cache for efficiency.
pub type InMemoryRewardMerkleTreeV2 =
    StorageBackedRewardMerkleTreeV2<storage::CachedInMemoryStorage>;

/// Two-level reward Merkle tree with file system storage.
///
/// Persistent storage that survives process restarts, suitable for:
/// - Nodes that maintain full reward state
/// - Archival nodes
/// - Long-running sequencers
///
/// Uses [`fs_storage::RewardMerkleTreeFSStorage`] with bincode serialization.
pub type FileBackedRewardMerkleTreeV2 =
    StorageBackedRewardMerkleTreeV2<fs_storage::RewardMerkleTreeFSStorage>;

/// Canonical reward Merkle tree type (single-level, 160-bit).
///
/// This is the "reference" implementation used for:
/// - Testing two-level tree correctness (commitments must match)
/// - Understanding the logical tree structure
/// - Generating test vectors
///
/// Production code uses [`InMemoryRewardMerkleTreeV2`] or [`FileBackedRewardMerkleTreeV2`]
/// for better performance and storage efficiency.
pub type RewardMerkleTreeV2 = InnerRewardMerkleTreeV2;

// Convenience methods for the default cached storage implementation
impl InMemoryRewardMerkleTreeV2 {
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
    ) -> anyhow::Result<Self>
    where
        BI: Borrow<RewardAccountV2>,
        BE: Borrow<RewardAmount>,
    {
        Self::from_kv_set_with_storage(height, data, storage::CachedInMemoryStorage::new())
    }
}

/// Verification result for merkle proofs.
///
/// - `Ok(())` - Proof is valid, account exists with claimed balance
/// - `Err(())` - Proof is invalid (wrong proof data or commitment)
///
/// Used by [`StorageBackedRewardMerkleTreeV2::verify`] to check membership proofs.
pub type VerificationResult = Result<(), ()>;

/// Core Merkle tree operations
impl<S: RewardMerkleTreeStorage> StorageBackedRewardMerkleTreeV2<S> {
    /// Tree arity (binary tree)
    pub const ARITY: usize = REWARD_MERKLE_TREE_V2_ARITY;

    /// Returns the tree height (160 bits for Ethereum address space)
    pub fn height(&self) -> usize {
        REWARD_MERKLE_TREE_V2_HEIGHT
    }

    /// Returns the tree capacity (2^160 possible accounts)
    pub fn capacity(&self) -> bigdecimal::num_bigint::BigUint {
        bigdecimal::num_bigint::BigUint::from_slice(&[2]).pow(160)
    }

    /// Returns the number of accounts with non-zero balances
    pub fn num_leaves(&self) -> u64 {
        self.num_leaves
    }

    /// Returns the root commitment (hash) of the tree
    pub fn commitment(&self) -> RewardMerkleCommitmentV2 {
        MerkleTreeCommitment::new(
            self.outer.commitment().digest(),
            REWARD_MERKLE_TREE_V2_HEIGHT,
            self.num_leaves,
        )
    }

    /// Look up an account's balance and generate a membership proof
    ///
    /// Returns the balance and proof if the account exists, NotFound otherwise.
    ///
    /// # Errors
    /// Returns storage error if IO operation fails.
    pub fn lookup(
        &self,
        index: impl Borrow<RewardAccountV2>,
    ) -> Result<LookupResult<RewardAmount, RewardMerkleProof, ()>, S::Error> {
        let outer_index = OuterIndex::new(index.borrow());
        let outer_proof = match self.outer.lookup(outer_index) {
            LookupResult::Ok(_, proof) => proof,
            LookupResult::NotInMemory => {
                unreachable!("Outer reward merkle tree will never be forgetten.")
            },
            LookupResult::NotFound(_) => return Ok(LookupResult::NotFound(())),
        };
        match self.storage.lookup(index)? {
            LookupResult::Ok(value, mut proof) => {
                Self::patch_membership_proof(&mut proof, &outer_proof);
                Ok(LookupResult::Ok(value, proof))
            },
            LookupResult::NotInMemory => Ok(LookupResult::NotInMemory),
            LookupResult::NotFound(_) => Ok(LookupResult::NotFound(())),
        }
    }

    /// Verify a membership proof against a root commitment
    ///
    /// Returns Ok(Ok(())) if proof is valid, Ok(Err(())) if invalid.
    pub fn verify(
        root: impl Borrow<RewardMerkleCommitmentV2>,
        index: impl Borrow<RewardAccountV2>,
        proof: impl Borrow<RewardMerkleProof>,
    ) -> Result<VerificationResult, MerkleTreeError> {
        InnerRewardMerkleTreeV2::verify(root, index, proof)
    }

    /// Update an account's balance using a custom function
    ///
    /// The function receives the current balance (if any) and returns the new balance.
    /// Returning None removes the account. Updates both inner and outer trees.
    ///
    /// # Errors
    /// If the merkle tree update fails.
    pub fn update_with<F>(
        &mut self,
        pos: impl Borrow<RewardAccountV2>,
        f: F,
    ) -> anyhow::Result<LookupResult<RewardAmount, (), ()>>
    where
        F: FnOnce(Option<&RewardAmount>) -> Option<RewardAmount>,
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

    /// Convenience method to update an account's balance directly
    ///
    /// # Errors
    /// Returns `MerkleTreeError` if the merkle tree update fails.
    /// Panics if storage operation fails.
    pub fn update(
        &mut self,
        pos: impl Borrow<RewardAccountV2>,
        value: impl Borrow<RewardAmount>,
    ) -> anyhow::Result<LookupResult<RewardAmount, (), ()>> {
        self.update_with(pos, |_| Some(*value.borrow()))
    }

    /// Look up an account and generate proof (membership or non-membership)
    ///
    /// Returns membership proof if account exists, non-membership proof otherwise.
    ///
    /// # Errors
    /// Returns storage error if IO operation fails.
    pub fn universal_lookup(
        &self,
        index: impl Borrow<RewardAccountV2>,
    ) -> Result<LookupResult<RewardAmount, RewardMerkleProof, RewardMerkleProof>, S::Error> {
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
                return Ok(LookupResult::NotFound(proof));
            },
        };
        match self.storage.lookup(index)? {
            LookupResult::Ok(value, mut proof) => {
                Self::patch_membership_proof(&mut proof, &outer_proof);
                Ok(LookupResult::Ok(value, proof))
            },
            LookupResult::NotInMemory => Ok(LookupResult::NotInMemory),
            LookupResult::NotFound(mut proof) => {
                Self::patch_membership_proof(&mut proof, &outer_proof);
                Ok(LookupResult::NotFound(proof))
            },
        }
    }

    /// Verify a non-membership proof against a root commitment
    ///
    /// Returns true if proof is valid (account doesn't exist), false otherwise.
    pub fn non_membership_verify(
        root: impl Borrow<RewardMerkleCommitmentV2>,
        index: impl Borrow<RewardAccountV2>,
        proof: impl Borrow<RewardMerkleProof>,
    ) -> Result<bool, MerkleTreeError> {
        InnerRewardMerkleTreeV2::non_membership_verify(root, index, proof)
    }

    /// Remove an account from memory (not yet implemented, currently just performs lookup)
    ///
    /// Returns the account's balance and proof if it exists.
    ///
    /// # Errors
    /// Returns storage error if IO operation fails.
    pub fn forget(
        &mut self,
        index: impl Borrow<RewardAccountV2>,
    ) -> Result<LookupResult<RewardAmount, RewardMerkleProof, ()>, S::Error> {
        self.lookup(index)
    }

    /// Restore an account to memory using a proof (currently a no-op)
    ///
    /// Restores an account that was previously forgotten. Currently does nothing.
    pub fn remember(
        &mut self,
        _pos: impl Borrow<RewardAccountV2>,
        _element: impl Borrow<RewardAmount>,
        _proof: impl Borrow<RewardMerkleProof>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// Remove an account and generate proof (membership or non-membership)
    ///
    /// Currently just performs universal_lookup without actually removing from memory.
    ///
    /// # Errors
    /// Returns storage error if IO operation fails.
    pub fn universal_forget(
        &mut self,
        pos: RewardAccountV2,
    ) -> Result<LookupResult<RewardAmount, RewardMerkleProof, RewardMerkleProof>, S::Error> {
        self.universal_lookup(pos)
    }

    /// Restore non-membership information using a proof (currently a no-op)
    ///
    /// Restores non-membership information for an account. Currently does nothing.
    pub fn non_membership_remember(
        &mut self,
        _pos: RewardAccountV2,
        _proof: impl Borrow<RewardMerkleProof>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Consumes the tree and returns an iterator over all (account, balance) pairs.
///
/// # Performance Note
///
/// This operation loads all 16 inner trees (even if sparse) and traverses them
/// recursively to collect leaf entries. For trees with many accounts, this can
/// be memory-intensive. Consider using storage-level iteration if available.
///
/// # Example
///
/// ```rust,ignore
/// let tree = InMemoryRewardMerkleTreeV2::from_kv_set(height, accounts)?;
/// for (account, balance) in tree {
///     println!("{:?}: {}", account, balance);
/// }
/// ```
impl<S: RewardMerkleTreeStorage> IntoIterator for StorageBackedRewardMerkleTreeV2<S> {
    type Item = (RewardAccountV2, RewardAmount);

    type IntoIter = <std::vec::Vec<(RewardAccountV2, RewardAmount)> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        // Collect all entries from all inner trees
        let indices = self.storage.indices();
        let mut all_entries = Vec::new();

        for outer_idx in indices {
            self.storage
                .with_tree(&outer_idx, |root| {
                    // Traverse the MerkleNode tree to collect all leaf entries
                    collect_merkle_leaves(root, &mut all_entries);
                })
                .expect("Storage operation failed during iteration");
        }

        all_entries.into_iter()
    }
}

/// Recursively traverses a merkle tree node and collects all leaf entries.
///
/// Performs depth-first traversal of the tree structure, extracting account-balance
/// pairs from leaf nodes. Skips empty nodes and forgotten subtrees.
///
/// # Arguments
/// * `node` - Root node to traverse (can be Empty, Leaf, Branch, or ForgettenSubtree)
/// * `entries` - Mutable vector to append discovered (account, balance) pairs
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
