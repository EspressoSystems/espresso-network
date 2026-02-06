//! Storage abstraction for inner Merkle tree roots.
//!
//! The [`RewardMerkleTreeStorage`] trait allows pluggable storage backends
//! (memory, disk, database). [`CachedInMemoryStorage`] provides fast in-memory
//! storage with single-entry caching.

use std::{borrow::Borrow, collections::HashMap, sync::RwLock};

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use jf_merkle_tree_compat::{
    prelude::{MerkleNode, MerkleProof},
    LookupResult, MerkleTreeError, ToTraversalPath,
};
use serde::{Deserialize, Serialize};

use crate::{
    reward_mt::{RewardMerkleProof, REWARD_MERKLE_TREE_V2_INNER_HEIGHT},
    sparse_mt::{Keccak256Hasher, KeccakNode},
    v0_3::RewardAmount,
    v0_4::{RewardAccountV2, REWARD_MERKLE_TREE_V2_ARITY, REWARD_MERKLE_TREE_V2_HEIGHT},
};

/// Type alias for the root node of an inner Merkle tree
pub type InnerRewardMerkleTreeRoot = MerkleNode<RewardAmount, RewardAccountV2, KeccakNode>;

/// Index type for inner Merkle roots in the outer tree
/// This represents the first 4 bits of the reward account address
#[derive(
    Copy,
    Clone,
    Debug,
    Serialize,
    Deserialize,
    Hash,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    CanonicalSerialize,
    CanonicalDeserialize,
)]
pub struct OuterIndex(pub u8);

impl<const ARITY: usize> ToTraversalPath<ARITY> for OuterIndex {
    fn to_traversal_path(&self, height: usize) -> Vec<usize> {
        <u8 as ToTraversalPath<ARITY>>::to_traversal_path(&self.0, height)
    }
}

impl OuterIndex {
    /// Maximum valid outer index (2^4 - 1 = 15)
    pub const MAX: u8 = 15;

    /// Create a new outer index, returns None if value > 15
    pub fn new(account: &RewardAccountV2) -> Self {
        // Big endian bytes
        Self(account.to_fixed_bytes()[0] >> 4)
    }

    /// Get the inner value
    pub fn value(&self) -> u8 {
        self.0
    }
}

/// Storage trait for inner Merkle roots
/// Implementations provide persistent storage for inner roots
pub trait RewardMerkleTreeStorage {
    /// Execute a function with an immutable reference to a tree
    /// Loads or creates the tree if it doesn't exist
    fn with_tree<F, R>(&self, index: OuterIndex, f: F) -> R
    where
        F: FnOnce(&InnerRewardMerkleTreeRoot) -> R;

    /// Execute a function with a mutable reference to a tree
    /// Loads or creates the tree if it doesn't exist
    fn with_tree_mut<F, R>(&self, index: OuterIndex, f: F) -> R
    where
        F: FnOnce(&mut InnerRewardMerkleTreeRoot) -> R;

    /// Check if an inner tree exists at the given index
    fn exists(&self, index: OuterIndex) -> bool;

    /// Get all stored outer indices
    fn indices(&self) -> Vec<OuterIndex>;

    fn lookup(
        &self,
        pos: impl Borrow<RewardAccountV2>,
    ) -> LookupResult<RewardAmount, RewardMerkleProof, RewardMerkleProof> {
        let pos = pos.borrow();
        let outer_index = OuterIndex::new(pos);
        let path =
            <RewardAccountV2 as ToTraversalPath<REWARD_MERKLE_TREE_V2_ARITY>>::to_traversal_path(
                pos,
                REWARD_MERKLE_TREE_V2_HEIGHT,
            );
        let inner_path = &path[..REWARD_MERKLE_TREE_V2_INNER_HEIGHT];
        self.with_tree(
            outer_index,
            |tree| -> LookupResult<RewardAmount, RewardMerkleProof, RewardMerkleProof> {
                match tree.lookup_internal(REWARD_MERKLE_TREE_V2_INNER_HEIGHT, inner_path) {
                    LookupResult::Ok(value, proof) => {
                        LookupResult::Ok(value, MerkleProof::new(*pos, proof))
                    },
                    LookupResult::NotInMemory => LookupResult::NotInMemory,
                    LookupResult::NotFound(proof) => {
                        LookupResult::NotFound(RewardMerkleProof::new(*pos, proof))
                    },
                }
            },
        )
    }

    #[allow(clippy::type_complexity)]
    fn update_with<F>(
        &mut self,
        pos: impl Borrow<RewardAccountV2>,
        f: F,
    ) -> Result<(LookupResult<RewardAmount, (), ()>, i64, KeccakNode), MerkleTreeError>
    where
        F: FnOnce(Option<&RewardAmount>) -> Option<RewardAmount>,
    {
        let pos = pos.borrow();
        let outer_index = OuterIndex::new(pos);
        let path =
            <RewardAccountV2 as ToTraversalPath<REWARD_MERKLE_TREE_V2_ARITY>>::to_traversal_path(
                pos,
                REWARD_MERKLE_TREE_V2_HEIGHT,
            );
        let inner_path = &path[..REWARD_MERKLE_TREE_V2_INNER_HEIGHT];
        self.with_tree_mut(
            outer_index,
            |tree| -> Result<(LookupResult<RewardAmount, (), ()>, i64, KeccakNode), MerkleTreeError> {
                let (new_root, delta, result) = tree
                    .update_with_internal::<Keccak256Hasher, REWARD_MERKLE_TREE_V2_ARITY, _>(
                        REWARD_MERKLE_TREE_V2_INNER_HEIGHT,
                        pos,
                        inner_path,
                        f,
                    )?;
                *tree = (*new_root).clone();
                Ok((result, delta, tree.value()))
            },
        )
    }

    #[allow(unused)]
    fn forget(
        &self,
        pos: impl Borrow<RewardAccountV2>,
    ) -> LookupResult<RewardAmount, RewardMerkleProof, ()> {
        let pos = pos.borrow();
        let outer_index = OuterIndex::new(pos);
        let path =
            <RewardAccountV2 as ToTraversalPath<REWARD_MERKLE_TREE_V2_ARITY>>::to_traversal_path(
                pos,
                REWARD_MERKLE_TREE_V2_HEIGHT,
            );
        let inner_path = &path[..REWARD_MERKLE_TREE_V2_INNER_HEIGHT];
        self.with_tree_mut(
            outer_index,
            |tree| -> LookupResult<RewardAmount, RewardMerkleProof, ()> {
                let (root, result) =
                    tree.forget_internal(REWARD_MERKLE_TREE_V2_INNER_HEIGHT, inner_path);
                match result {
                    LookupResult::Ok(value, proof) => {
                        *tree = (*root).clone();
                        LookupResult::Ok(value, MerkleProof::new(*pos, proof))
                    },
                    LookupResult::NotInMemory => LookupResult::NotInMemory,
                    LookupResult::NotFound(_) => LookupResult::NotFound(()),
                }
            },
        )
    }
}

/// Cached in-memory storage implementation using a HashMap
/// Uses RwLock for thread-safe interior mutability to allow cache operations through &self
#[derive(Debug)]
pub struct CachedInMemoryStorage {
    /// Persistent storage for all inner roots
    roots: RwLock<HashMap<OuterIndex, InnerRewardMerkleTreeRoot>>,
    /// Cached inner tree with its index (uses RwLock for thread-safe interior mutability)
    cache: RwLock<Option<(OuterIndex, InnerRewardMerkleTreeRoot)>>,
}

// Manual Clone implementation
impl Clone for CachedInMemoryStorage {
    fn clone(&self) -> Self {
        Self {
            roots: RwLock::new(self.roots.read().unwrap().clone()),
            cache: RwLock::new(self.cache.read().unwrap().clone()),
        }
    }
}

// Manual Serialize implementation
impl Serialize for CachedInMemoryStorage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("CachedInMemoryStorage", 2)?;
        state.serialize_field("roots", &*self.roots.read().unwrap())?;
        state.serialize_field("cache", &*self.cache.read().unwrap())?;
        state.end()
    }
}

// Manual Deserialize implementation
impl<'de> Deserialize<'de> for CachedInMemoryStorage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CachedInMemoryStorageData {
            roots: HashMap<OuterIndex, InnerRewardMerkleTreeRoot>,
            cache: Option<(OuterIndex, InnerRewardMerkleTreeRoot)>,
        }

        let data = CachedInMemoryStorageData::deserialize(deserializer)?;
        Ok(Self {
            roots: RwLock::new(data.roots),
            cache: RwLock::new(data.cache),
        })
    }
}

// Manual PartialEq implementation that compares logical content
impl PartialEq for CachedInMemoryStorage {
    fn eq(&self, other: &Self) -> bool {
        // Flush both caches before comparison
        let _ = self.flush_cache();
        let _ = other.flush_cache();
        *self.roots.read().unwrap() == *other.roots.read().unwrap()
    }
}

impl Eq for CachedInMemoryStorage {}

impl CachedInMemoryStorage {
    /// Create a new empty cached storage
    pub fn new() -> Self {
        Self {
            roots: RwLock::new(HashMap::new()),
            cache: RwLock::new(None),
        }
    }

    /// Create storage with a pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            roots: RwLock::new(HashMap::with_capacity(capacity)),
            cache: RwLock::new(None),
        }
    }

    /// Get the number of stored roots (excluding cache)
    pub fn len(&self) -> usize {
        self.roots.read().unwrap().len()
    }

    /// Check if storage is empty (excluding cache)
    pub fn is_empty(&self) -> bool {
        self.roots.read().unwrap().is_empty()
    }

    /// Clear all stored roots and cache
    pub fn clear(&self) {
        self.roots.write().unwrap().clear();
        *self.cache.write().unwrap() = None;
    }

    /// Internal: Load an inner Merkle tree into the cache
    fn load_into_cache(&self, index: OuterIndex) {
        // Check if the requested tree is already cached
        {
            let cache = self.cache.read().unwrap();
            if let Some((cached_index, _)) = &*cache {
                if *cached_index == index {
                    return; // Already cached
                }
            }
        }

        // Flush the current cache if it exists
        let _ = self.flush_cache();

        // Load the tree from storage or create a new one
        let tree = self
            .roots
            .write()
            .unwrap()
            .remove(&index)
            .unwrap_or(InnerRewardMerkleTreeRoot::Empty);

        // Cache the tree
        *self.cache.write().unwrap() = Some((index, tree));
    }

    /// Internal: Flush the cache back to storage
    fn flush_cache(&self) -> Result<(), MerkleTreeError> {
        let mut cache = self.cache.write().unwrap();
        if let Some((index, tree)) = cache.take() {
            self.roots.write().unwrap().insert(index, tree);
        }
        Ok(())
    }
}

impl Default for CachedInMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl RewardMerkleTreeStorage for CachedInMemoryStorage {
    fn with_tree<F, R>(&self, index: OuterIndex, f: F) -> R
    where
        F: FnOnce(&InnerRewardMerkleTreeRoot) -> R,
    {
        // Load into cache if needed
        self.load_into_cache(index);

        // Execute the closure with the cached tree
        let cache = self.cache.read().unwrap();
        let (_, root) = cache.as_ref().expect("Tree should be in cache after load");
        f(root)
    }

    fn with_tree_mut<F, R>(&self, index: OuterIndex, f: F) -> R
    where
        F: FnOnce(&mut InnerRewardMerkleTreeRoot) -> R,
    {
        // Load into cache if needed
        self.load_into_cache(index);

        // Execute the closure with the cached tree
        let mut cache = self.cache.write().unwrap();
        let (_, root) = cache.as_mut().expect("Tree should be in cache after load");
        f(root)
    }

    fn exists(&self, index: OuterIndex) -> bool {
        // Check if it's in the cache first
        {
            let cache = self.cache.read().unwrap();
            if let Some((cached_index, _)) = &*cache {
                if *cached_index == index {
                    return true;
                }
            }
        }
        // Otherwise check persistent storage
        self.roots.read().unwrap().contains_key(&index)
    }

    fn indices(&self) -> Vec<OuterIndex> {
        let roots = self.roots.read().unwrap();
        let mut indices: Vec<_> = roots.keys().copied().collect();

        // Include cached index if it's not in persistent storage
        let cache = self.cache.read().unwrap();
        if let Some((cached_index, _)) = &*cache {
            if !roots.contains_key(cached_index) {
                indices.push(*cached_index);
            }
        }

        indices.sort();
        indices
    }
}
