//! Storage abstraction for inner Merkle tree roots.
//!
//! The [`RewardMerkleTreeStorage`] trait allows pluggable storage backends
//! (memory, disk, database). [`CachedInMemoryStorage`] provides fast in-memory
//! storage with single-entry caching.

use std::{borrow::Borrow, collections::HashMap, sync::RwLock};

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use jf_merkle_tree_compat::{
    prelude::{MerkleNode, MerkleProof},
    LookupResult, ToTraversalPath,
};
use serde::{Deserialize, Serialize};

use crate::{
    reward_mt::{RewardMerkleProof, REWARD_MERKLE_TREE_V2_INNER_HEIGHT},
    sparse_mt::{Keccak256Hasher, KeccakNode},
    v0_3::RewardAmount,
    v0_4::{RewardAccountV2, REWARD_MERKLE_TREE_V2_ARITY, REWARD_MERKLE_TREE_V2_HEIGHT},
};

/// Root node of a 156-bit inner Merkle tree.
pub type InnerRewardMerkleTreeRoot = MerkleNode<RewardAmount, RewardAccountV2, KeccakNode>;

/// Outer tree index (0-15) derived from account address high nibble.
///
/// Represents the partition index in the outer tree, calculated as the
/// first 4 bits (high nibble) of an account address:
///
/// ```text
/// Address:     0xABCDEF...
///                ^
///              High nibble = A (hex) = 10 (decimal)
/// OuterIndex:  10
/// ```
///
/// This partitions the 160-bit address space into 16 equal-sized buckets
/// (2^156 addresses each), allowing efficient storage and lookup.
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
    /// Maximum valid outer index (2^4 - 1 = 15).
    pub const MAX: u8 = 15;

    /// Extract outer index from a reward account address.
    ///
    /// Takes the high nibble (first 4 bits) of the account address's first byte.
    /// Since Ethereum addresses are big-endian, this is byte[0] >> 4.
    ///
    /// # Example
    ///
    /// ```text
    /// Account: 0xA1B2C3D4E5F6...
    /// Byte[0]: 0xA1 = 0b1010_0001
    /// >> 4:    0x0A = 0b0000_1010 = 10
    /// Result:  OuterIndex(10)
    /// ```
    ///
    /// # Arguments
    /// * `account` - The reward account (Ethereum address)
    ///
    /// # Returns
    /// An `OuterIndex` in the range [0, 15]
    pub fn new(account: &RewardAccountV2) -> Self {
        // Extract high nibble from first byte (big-endian)
        Self(account.to_fixed_bytes()[0] >> 4)
    }

    /// Get the raw index value (0-15).
    ///
    /// # Returns
    /// The partition index as a u8
    pub fn value(&self) -> u8 {
        self.0
    }
}

/// Storage abstraction for inner Merkle tree roots.
///
/// Defines how inner tree roots (one per partition) are persisted and accessed.
/// Implementations control storage strategy (memory, disk, database) and caching.
///
/// # Design Pattern
///
/// Uses the "loan" pattern with closures to ensure proper cache management:
/// - `with_tree` / `with_tree_mut` load trees on-demand and handle flushing
/// - Callers cannot hold references beyond the closure scope
/// - Storage backend controls when to load/flush/cache
///
/// # Implementations
///
/// - [`CachedInMemoryStorage`]: Fast, non-persistent, single-entry cache
/// - [`fs_storage::RewardMerkleTreeFSStorage`]: File-backed, persistent
///
/// # Thread Safety
///
/// Implementations use `RwLock` for interior mutability, allowing `&self` methods
/// to perform cache operations while maintaining Sync/Send.
pub trait RewardMerkleTreeStorage {
    /// Error type for storage operations.
    ///
    /// - In-memory storage: `std::convert::Infallible` (never fails)
    /// - File storage: `std::io::Error` (disk I/O failures)
    /// - Database storage: Custom error type
    type Error: std::error::Error + Send + Sync + 'static;

    /// Execute a read-only operation on an inner tree.
    ///
    /// Loads the tree from storage (or cache) if needed. Creates an empty tree
    /// if the partition has never been written to.
    ///
    /// # Arguments
    /// * `index` - Partition index (0-15)
    /// * `f` - Closure that receives immutable tree reference
    ///
    /// # Returns
    /// The closure's return value, or storage error
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// storage.with_tree(outer_index, |tree| tree.num_leaves())?
    /// ```
    fn with_tree<F, R>(&self, index: &OuterIndex, f: F) -> Result<R, Self::Error>
    where
        F: FnOnce(&InnerRewardMerkleTreeRoot) -> R;

    /// Execute a mutating operation on an inner tree.
    ///
    /// Loads the tree from storage (or cache) if needed. Creates an empty tree
    /// if the partition has never been written to. Changes are written back when
    /// the cache is flushed.
    ///
    /// # Arguments
    /// * `index` - Partition index (0-15)
    /// * `f` - Closure that receives mutable tree reference
    ///
    /// # Returns
    /// The closure's return value, or storage error
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// storage.with_tree_mut(outer_index, |tree| {
    ///     tree.update(account, amount)
    /// })?
    /// ```
    fn with_tree_mut<F, R>(&self, index: &OuterIndex, f: F) -> Result<R, Self::Error>
    where
        F: FnOnce(&mut InnerRewardMerkleTreeRoot) -> R;

    /// Check if an inner tree exists at the given partition.
    ///
    /// Returns `true` if the partition has any accounts (non-empty tree),
    /// `false` if it's empty or never been used.
    ///
    /// # Arguments
    /// * `index` - Partition index (0-15)
    ///
    /// # Returns
    /// Whether the partition contains data
    fn exists(&self, index: &OuterIndex) -> bool;

    /// Get all partition indices that contain data.
    ///
    /// Returns a sorted, deduplicated list of outer indices (0-15) that have
    /// non-empty inner trees. Used for iteration and statistics.
    ///
    /// # Returns
    /// Vector of partition indices, sorted in ascending order
    fn indices(&self) -> Vec<OuterIndex>;

    /// Look up an account's balance and generate a proof (membership or non-membership).
    ///
    /// This is a provided method that uses `with_tree` to perform the lookup within
    /// the appropriate inner tree. Extracts the outer index from the account address,
    /// loads that partition, and generates an inner proof (156 bits).
    ///
    /// The caller (typically `StorageBackedRewardMerkleTreeV2`) must patch the inner
    /// proof with the outer proof to create a full 160-bit proof.
    ///
    /// # Arguments
    /// * `pos` - Account address to look up
    ///
    /// # Returns
    /// - `Ok(value, proof)` - Account found with balance and inner membership proof
    /// - `NotInMemory` - Account data not available (sparse tree, needs catchup)
    /// - `NotFound(proof)` - Account doesn't exist, includes inner non-membership proof
    ///
    /// # Errors
    /// Returns storage error if IO operation fails
    fn lookup(
        &self,
        pos: impl Borrow<RewardAccountV2>,
    ) -> Result<LookupResult<RewardAmount, RewardMerkleProof, RewardMerkleProof>, Self::Error> {
        let pos = pos.borrow();
        let outer_index = OuterIndex::new(pos);
        let path =
            <RewardAccountV2 as ToTraversalPath<REWARD_MERKLE_TREE_V2_ARITY>>::to_traversal_path(
                pos,
                REWARD_MERKLE_TREE_V2_HEIGHT,
            );
        let inner_path = &path[..REWARD_MERKLE_TREE_V2_INNER_HEIGHT];
        self.with_tree(
            &outer_index,
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

    /// Update an account's balance using a custom function.
    ///
    /// Provided method that uses `with_tree_mut` to modify an inner tree. The function
    /// receives the current balance (if any) and returns the new balance. Returning
    /// `None` removes the account.
    ///
    /// # Arguments
    /// * `pos` - Account address to update
    /// * `f` - Update function: `Option<&RewardAmount> -> Option<RewardAmount>`
    ///   - Input `Some(&amount)` if account exists, `None` otherwise
    ///   - Output `Some(new_amount)` to set balance, `None` to remove
    ///
    /// # Returns
    /// Tuple of:
    /// - `LookupResult` - Previous value (Ok/NotFound/NotInMemory)
    /// - `i64` - Leaf count delta (+1 for insert, -1 for remove, 0 for update)
    /// - `KeccakNode` - New inner tree root hash (or default if tree becomes empty)
    ///
    /// # Errors
    /// Returns error if merkle tree update fails or storage operation fails
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Increment balance by 100
    /// storage.update_with(account, |existing| {
    ///     Some(RewardAmount(existing.map(|a| a.0).unwrap_or(U256::ZERO) + U256::from(100)))
    /// })?;
    ///
    /// // Remove account
    /// storage.update_with(account, |_| None)?;
    /// ```
    #[allow(clippy::type_complexity)]
    fn update_with<F>(
        &mut self,
        pos: impl Borrow<RewardAccountV2>,
        f: F,
    ) -> anyhow::Result<(LookupResult<RewardAmount, (), ()>, i64, KeccakNode)>
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
            &outer_index,
            |tree| -> anyhow::Result<(LookupResult<RewardAmount, (), ()>, i64, KeccakNode)> {
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
        )?
    }

    /// Remove an account from memory (for sparse tree optimization).
    ///
    /// **Note**: Currently not fully implemented. The inner tree's `forget_internal`
    /// is called but trees are not actually made sparse. This is a placeholder for
    /// future sparse tree support where nodes can be pruned from memory while
    /// preserving the root hash.
    ///
    /// # Arguments
    /// * `pos` - Account address to forget
    ///
    /// # Returns
    /// - `Ok(value, proof)` - Account found, returns balance and proof before forgetting
    /// - `NotInMemory` - Already forgotten or never existed
    /// - `NotFound(())` - Account doesn't exist
    ///
    /// # Errors
    /// Returns storage error if IO operation fails
    #[allow(unused)]
    fn forget(
        &self,
        pos: impl Borrow<RewardAccountV2>,
    ) -> Result<LookupResult<RewardAmount, RewardMerkleProof, ()>, Self::Error> {
        let pos = pos.borrow();
        let outer_index = OuterIndex::new(pos);
        let path =
            <RewardAccountV2 as ToTraversalPath<REWARD_MERKLE_TREE_V2_ARITY>>::to_traversal_path(
                pos,
                REWARD_MERKLE_TREE_V2_HEIGHT,
            );
        let inner_path = &path[..REWARD_MERKLE_TREE_V2_INNER_HEIGHT];
        self.with_tree_mut(
            &outer_index,
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

/// In-memory storage with single-entry cache.
///
/// # Design
///
/// Stores all 16 inner tree roots in a HashMap, with a single-entry cache for the
/// most recently accessed partition. This provides excellent performance for
/// sequential access patterns (common during block processing) while keeping
/// memory overhead minimal.
///
/// # Caching Strategy
///
/// - **Cache hit**: Direct access to cached tree (no HashMap lookup)
/// - **Cache miss**: Flush current cache to HashMap, load new tree into cache
/// - **On drop**: Cache is automatically flushed (no data loss)
///
/// # Thread Safety
///
/// Uses `RwLock` for interior mutability, allowing `&self` methods to perform
/// cache operations. This enables shared references to modify the cache while
/// maintaining Sync/Send bounds for concurrent access.
///
/// # Performance
///
/// Best for:
/// - Sequential access (processing blocks in order)
/// - Validators without full state persistence
/// - Testing and development
/// - Reconstructing state from catchup
///
/// Not ideal for:
/// - Random access across many partitions (thrashing)
/// - Long-term persistence (loses state on restart)
#[derive(Debug)]
pub struct CachedInMemoryStorage {
    /// Main storage: HashMap of outer index → inner tree root
    /// All non-cached trees live here
    roots: RwLock<HashMap<OuterIndex, InnerRewardMerkleTreeRoot>>,

    /// Single-entry cache: (partition_index, tree_root)
    /// The most recently accessed partition is kept here for fast access
    cache: RwLock<Option<(OuterIndex, InnerRewardMerkleTreeRoot)>>,
}

// Manual Clone implementation
impl Clone for CachedInMemoryStorage {
    fn clone(&self) -> Self {
        self.flush_cache(); // Ensure cache is flushed before cloning
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
        self.flush_cache();
        other.flush_cache();
        *self.roots.read().unwrap() == *other.roots.read().unwrap()
    }
}

impl Eq for CachedInMemoryStorage {}

impl CachedInMemoryStorage {
    /// Create a new empty storage with no pre-allocated capacity.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let storage = CachedInMemoryStorage::new();
    /// let tree = InMemoryRewardMerkleTreeV2::new_with_storage(storage);
    /// ```
    pub fn new() -> Self {
        Self {
            roots: RwLock::new(HashMap::new()),
            cache: RwLock::new(None),
        }
    }

    /// Create storage with pre-allocated HashMap capacity.
    ///
    /// Useful if you know approximately how many partitions will be used.
    /// Maximum useful capacity is 16 (one per partition).
    ///
    /// # Arguments
    /// * `capacity` - Number of partitions to pre-allocate space for
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Pre-allocate for all 16 partitions
    /// let storage = CachedInMemoryStorage::with_capacity(16);
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            roots: RwLock::new(HashMap::with_capacity(capacity)),
            cache: RwLock::new(None),
        }
    }

    /// Get the number of non-empty partitions (excluding cached partition).
    ///
    /// Returns the count of inner trees stored in the HashMap. The cached tree
    /// (if any) is not included in this count. For total partitions, check if
    /// cache is populated and add 1 if the cached index isn't in roots.
    ///
    /// # Returns
    /// Number of partitions in persistent storage (0-16)
    pub fn len(&self) -> usize {
        self.roots.read().unwrap().len()
    }

    /// Check if storage is empty (no partitions in HashMap, ignoring cache).
    ///
    /// Returns `true` if no inner trees are stored in the HashMap. The cache
    /// may still contain a tree.
    ///
    /// # Returns
    /// Whether persistent storage is empty
    pub fn is_empty(&self) -> bool {
        self.roots.read().unwrap().is_empty()
    }

    /// Clear all stored roots and cache, resetting to empty state.
    ///
    /// Removes all inner trees from both HashMap and cache. After calling this,
    /// the storage is equivalent to freshly created with `new()`.
    pub fn clear(&self) {
        self.roots.write().unwrap().clear();
        *self.cache.write().unwrap() = None;
    }

    /// Load an inner tree into the single-entry cache (internal).
    ///
    /// If the requested tree is already cached, does nothing. Otherwise:
    /// 1. Flushes current cache to HashMap
    /// 2. Loads requested tree from HashMap (or creates Empty if not found)
    /// 3. Stores in cache for fast access
    ///
    /// # Arguments
    /// * `index` - Partition index to load
    fn load_into_cache(&self, index: &OuterIndex) {
        // Check if the requested tree is already cached
        {
            let cache = self.cache.read().unwrap();
            if let Some((cached_index, _)) = &*cache {
                if cached_index == index {
                    return; // Already cached
                }
            }
        }

        // Flush the current cache if it exists
        self.flush_cache();

        // Load the tree from storage or create a new one
        let tree = self
            .roots
            .write()
            .unwrap()
            .remove(index)
            .unwrap_or(InnerRewardMerkleTreeRoot::Empty);

        // Cache the tree
        *self.cache.write().unwrap() = Some((*index, tree));
    }

    /// Write cached tree back to HashMap (internal).
    ///
    /// If cache contains a tree, moves it from cache to HashMap. Called automatically
    /// before loading a different partition or when comparing storage instances.
    fn flush_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        if let Some((index, tree)) = cache.take() {
            self.roots.write().unwrap().insert(index, tree);
        }
    }
}

impl Default for CachedInMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl RewardMerkleTreeStorage for CachedInMemoryStorage {
    type Error = std::convert::Infallible;

    fn with_tree<F, R>(&self, index: &OuterIndex, f: F) -> Result<R, Self::Error>
    where
        F: FnOnce(&InnerRewardMerkleTreeRoot) -> R,
    {
        // Load into cache if needed
        self.load_into_cache(index);

        // Execute the closure with the cached tree
        let cache = self.cache.read().unwrap();
        let (_, root) = cache.as_ref().expect("Tree should be in cache after load");
        Ok(f(root))
    }

    fn with_tree_mut<F, R>(&self, index: &OuterIndex, f: F) -> Result<R, Self::Error>
    where
        F: FnOnce(&mut InnerRewardMerkleTreeRoot) -> R,
    {
        // Load into cache if needed
        self.load_into_cache(index);

        // Execute the closure with the cached tree
        let mut cache = self.cache.write().unwrap();
        let (_, root) = cache.as_mut().expect("Tree should be in cache after load");
        Ok(f(root))
    }

    fn exists(&self, index: &OuterIndex) -> bool {
        // Check if it's in the cache first
        {
            let cache = self.cache.read().unwrap();
            if let Some((cached_index, _)) = &*cache {
                if cached_index == index {
                    return true;
                }
            }
        }
        // Otherwise check persistent storage
        self.roots.read().unwrap().contains_key(index)
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

#[cfg(test)]
mod tests {
    use alloy::primitives::U256;
    use jf_merkle_tree_compat::{MerkleTreeScheme, ToTraversalPath, UniversalMerkleTreeScheme};
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;

    use super::*;
    use crate::{
        reward_mt::{
            InMemoryRewardMerkleTreeV2, InnerRewardMerkleTreeV2, REWARD_MERKLE_TREE_V2_HEIGHT,
        },
        v0_3::RewardAmount,
        v0_4::RewardAccountV2,
    };

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
        let mut two_level_tree = InMemoryRewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);

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

        let mut tree = InMemoryRewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);
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
            match tree.lookup(account).unwrap() {
                LookupResult::Ok(amount, proof) => {
                    assert_eq!(amount, *expected_amount, "Amount should match");

                    // Verify the proof
                    let verification =
                        InMemoryRewardMerkleTreeV2::verify(tree.commitment(), account, &proof)
                            .unwrap();
                    assert!(verification.is_ok(), "Proof should be valid");
                },
                _ => panic!("Account should be found"),
            }
        }

        // Test lookup for non-existent account
        let non_existent = random_account(&mut rng);
        match tree.lookup(non_existent).unwrap() {
            LookupResult::NotFound(_) => {}, // Expected
            _ => panic!("Non-existent account should not be found"),
        }
    }

    #[test]
    fn test_universal_lookup() {
        let mut rng = ChaCha20Rng::seed_from_u64(456);

        let mut tree = InMemoryRewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);

        let account = random_account(&mut rng);
        let amount = random_amount(&mut rng);

        // Insert account
        tree.update(account, amount).unwrap();

        // Test universal lookup for existing account
        match tree.universal_lookup(account).unwrap() {
            LookupResult::Ok(found_amount, proof) => {
                assert_eq!(found_amount, amount, "Amount should match");

                // Verify membership proof
                let verification =
                    InMemoryRewardMerkleTreeV2::verify(tree.commitment(), account, &proof).unwrap();
                assert!(verification.is_ok(), "Membership proof should be valid");
            },
            _ => panic!("Account should be found with membership proof"),
        }

        // Test universal lookup for non-existent account
        let non_existent = random_account(&mut rng);
        match tree.universal_lookup(non_existent).unwrap() {
            LookupResult::NotFound(proof) => {
                // Verify non-membership proof
                let is_valid = InMemoryRewardMerkleTreeV2::non_membership_verify(
                    tree.commitment(),
                    non_existent,
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

        let mut two_level_tree = InMemoryRewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);
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
        match two_level_tree.lookup(account).unwrap() {
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
            InMemoryRewardMerkleTreeV2::from_kv_set(REWARD_MERKLE_TREE_V2_HEIGHT, &kv_pairs)
                .unwrap();

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
            match two_level_tree.lookup(account).unwrap() {
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

        let mut two_level_tree = InMemoryRewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);
        let mut single_level_tree = InnerRewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);

        let account = random_account(&mut rng);
        let initial_amount = RewardAmount(U256::from(100u64));
        let increment = RewardAmount(U256::from(50u64));

        // Initial insert
        two_level_tree.update(account, initial_amount).unwrap();
        single_level_tree.update(account, initial_amount).unwrap();

        // Update using custom function (increment)
        two_level_tree
            .update_with(account, |existing| {
                existing.map(|amt| RewardAmount(amt.0 + increment.0))
            })
            .unwrap();
        single_level_tree
            .update_with(account, |existing| {
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
        match two_level_tree.lookup(account).unwrap() {
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

        let mut two_level_tree = InMemoryRewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);
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
        match two_level_tree.lookup(account).unwrap() {
            LookupResult::NotFound(_) => {}, // Expected
            _ => panic!("Removed account should not be found"),
        }
    }

    #[test]
    fn test_stress_with_many_operations() {
        let mut rng = ChaCha20Rng::seed_from_u64(282930);

        let mut two_level_tree = InMemoryRewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);
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

                    two_level_tree.update_with(account, |_| None).unwrap();
                    single_level_tree.update_with(account, |_| None).unwrap();
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

        let mut tree = InMemoryRewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);
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
