//! File system backed storage with persistence.
//!
//! Stores inner tree roots as bincode files (`tree_00.bin` through `tree_0f.bin`).
//! Uses single-entry cache with automatic flush on drop. Survives process restarts.

use std::{
    fs,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    sync::RwLock,
};

use super::storage::{InnerRewardMerkleTreeRoot, OuterIndex, RewardMerkleTreeStorage};

/// File system backed storage implementation for reward Merkle trees.
///
/// Stores each inner tree root as a serialized bincode file on disk.
/// Uses a single-entry cache with RwLock for thread-safe interior mutability.
#[derive(Debug)]
pub struct RewardMerkleTreeFSStorage {
    /// Directory where tree files are stored
    storage_dir: PathBuf,
    /// Cached inner tree with its index (uses RwLock for thread-safe interior mutability)
    cache: RwLock<Option<(OuterIndex, InnerRewardMerkleTreeRoot)>>,
}

impl Default for RewardMerkleTreeFSStorage {
    fn default() -> Self {
        let storage_dir = tempfile::tempdir()
            .expect("Failed to create temporary directory for Reward Merkle Tree FS storage.")
            .keep();
        // fs::create_dir_all(&storage_dir).expect("Failed to create temporary storage dir.");

        Self {
            storage_dir,
            cache: RwLock::new(None),
        }
    }
}

impl RewardMerkleTreeFSStorage {
    /// Create a new file system storage at the given directory
    /// Creates the directory if it doesn't exist
    pub fn new(storage_dir: impl AsRef<Path>) -> io::Result<Self> {
        let storage_dir = storage_dir.as_ref().to_path_buf();
        fs::create_dir_all(&storage_dir)?;

        Ok(Self {
            storage_dir,
            cache: RwLock::new(None),
        })
    }

    /// Get the storage directory path
    pub fn storage_dir(&self) -> &Path {
        &self.storage_dir
    }

    /// Get the file path for a given outer index
    fn file_path(&self, index: OuterIndex) -> PathBuf {
        self.storage_dir
            .join(format!("tree_{:02x}.bin", index.value()))
    }

    /// Internal: Load a tree root from disk, return a brand new tree if not exists.
    fn load_from_disk(&self, index: OuterIndex) -> io::Result<InnerRewardMerkleTreeRoot> {
        let path = self.file_path(index);

        match fs::read(&path) {
            Ok(bytes) => {
                let root: InnerRewardMerkleTreeRoot = bincode::deserialize(&bytes)
                    .map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
                Ok(root)
            },
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(InnerRewardMerkleTreeRoot::Empty),
            Err(e) => Err(e),
        }
    }

    /// Internal: Save a tree root to disk
    fn save_to_disk(&self, index: OuterIndex, root: &InnerRewardMerkleTreeRoot) -> io::Result<()> {
        let path = self.file_path(index);
        let bytes =
            bincode::serialize(root).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
        fs::write(&path, bytes)?;
        Ok(())
    }

    /// Internal: Load a tree into cache
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
        if let Err(e) = self.flush_cache() {
            tracing::error!("Error flushing Reward Merkle tree cache: {}", e);
        }

        // Load the tree from disk or create a new one
        let root = self
            .load_from_disk(index)
            .unwrap_or(InnerRewardMerkleTreeRoot::Empty);

        // Cache the tree
        *self.cache.write().unwrap() = Some((index, root));
    }

    /// Internal: Flush the cache back to disk
    fn flush_cache(&self) -> io::Result<()> {
        let mut cache = self.cache.write().unwrap();
        if let Some((index, root)) = cache.take() {
            self.save_to_disk(index, &root)?;
        }
        Ok(())
    }
}

impl Drop for RewardMerkleTreeFSStorage {
    fn drop(&mut self) {
        // Ensure cache is flushed when storage is dropped
        let _ = self.flush_cache();
    }
}

// Manual Clone implementation
impl Clone for RewardMerkleTreeFSStorage {
    fn clone(&self) -> Self {
        // Flush cache before cloning
        let _ = self.flush_cache();

        Self {
            storage_dir: self.storage_dir.clone(),
            cache: RwLock::new(None),
        }
    }
}

impl RewardMerkleTreeStorage for RewardMerkleTreeFSStorage {
    fn with_tree<F, R>(&self, index: OuterIndex, f: F) -> R
    where
        F: FnOnce(&InnerRewardMerkleTreeRoot) -> R,
    {
        // Load into cache if needed
        self.load_into_cache(index);

        // Execute the closure with the cached tree
        let cache = self.cache.read().unwrap();
        let (_, tree) = cache.as_ref().expect("Tree should be in cache after load");
        f(tree)
    }

    fn with_tree_mut<F, R>(&self, index: OuterIndex, f: F) -> R
    where
        F: FnOnce(&mut InnerRewardMerkleTreeRoot) -> R,
    {
        // Load into cache if needed
        self.load_into_cache(index);

        // Execute the closure with the cached tree
        let mut cache = self.cache.write().unwrap();
        let (_, tree) = cache.as_mut().expect("Tree should be in cache after load");
        f(tree)
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
        // Otherwise check if file exists on disk
        self.file_path(index).exists()
    }

    fn indices(&self) -> Vec<OuterIndex> {
        let mut indices = Vec::new();

        // Read all tree files from the storage directory
        if let Ok(entries) = fs::read_dir(&self.storage_dir) {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string() {
                    // Parse files matching "tree_XX.bin" pattern
                    if file_name.starts_with("tree_") && file_name.ends_with(".bin") {
                        let hex_str = &file_name[5..7]; // Extract "XX" from "tree_XX.bin"
                        if let Ok(value) = u8::from_str_radix(hex_str, 16) {
                            if value <= OuterIndex::MAX {
                                indices.push(OuterIndex(value));
                            }
                        }
                    }
                }
            }
        }

        // Include cached index if it's not on disk yet
        let cache = self.cache.read().unwrap();
        if let Some((cached_index, _)) = &*cache {
            if !self.file_path(*cached_index).exists() {
                indices.push(*cached_index);
            }
        }

        indices.sort();
        indices.dedup();
        indices
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::U256;
    use jf_merkle_tree_compat::{LookupResult, MerkleTreeScheme, UniversalMerkleTreeScheme};
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;

    use super::*;
    use crate::{
        reward_mt::{
            InnerRewardMerkleTreeV2, RewardMerkleTreeV2Impl, REWARD_MERKLE_TREE_V2_HEIGHT,
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
    fn test_fs_storage_creation() {
        let storage = RewardMerkleTreeFSStorage::default();

        assert!(storage.storage_dir().exists());
    }

    #[test]
    fn test_two_level_tree_matches_single_level() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);

        // Create two-level tree with FS storage
        let storage = RewardMerkleTreeFSStorage::default();
        let mut two_level_tree = RewardMerkleTreeV2Impl::new_with_storage(storage);

        // Create single-level tree for comparison
        let mut single_level_tree = InnerRewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);

        // Insert random accounts
        let num_accounts = 100;
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

        let storage = RewardMerkleTreeFSStorage::default();
        let mut tree = RewardMerkleTreeV2Impl::new_with_storage(storage);
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
                    let verification = RewardMerkleTreeV2Impl::<RewardMerkleTreeFSStorage>::verify(
                        tree.commitment(),
                        account,
                        &proof,
                    )
                    .unwrap();
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

        let storage = RewardMerkleTreeFSStorage::default();
        let mut tree = RewardMerkleTreeV2Impl::new_with_storage(storage);

        let account = random_account(&mut rng);
        let amount = random_amount(&mut rng);

        // Insert account
        tree.update(account, amount).unwrap();

        // Test universal lookup for existing account
        match tree.universal_lookup(account) {
            LookupResult::Ok(found_amount, proof) => {
                assert_eq!(found_amount, amount, "Amount should match");

                // Verify membership proof
                let verification = RewardMerkleTreeV2Impl::<RewardMerkleTreeFSStorage>::verify(
                    tree.commitment(),
                    &account,
                    &proof,
                )
                .unwrap();
                assert!(verification.is_ok(), "Membership proof should be valid");
            },
            _ => panic!("Account should be found with membership proof"),
        }

        // Test universal lookup for non-existent account
        let non_existent = random_account(&mut rng);
        match tree.universal_lookup(non_existent) {
            LookupResult::NotFound(proof) => {
                // Verify non-membership proof
                let is_valid =
                    RewardMerkleTreeV2Impl::<RewardMerkleTreeFSStorage>::non_membership_verify(
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

        let storage = RewardMerkleTreeFSStorage::default();
        let mut two_level_tree = RewardMerkleTreeV2Impl::new_with_storage(storage);
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
        let storage = RewardMerkleTreeFSStorage::default();
        let two_level_tree = RewardMerkleTreeV2Impl::from_kv_set_with_storage(
            REWARD_MERKLE_TREE_V2_HEIGHT,
            &kv_pairs,
            storage,
        )
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

        let storage = RewardMerkleTreeFSStorage::default();
        let mut two_level_tree = RewardMerkleTreeV2Impl::new_with_storage(storage);
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

        let storage = RewardMerkleTreeFSStorage::default();
        let mut two_level_tree = RewardMerkleTreeV2Impl::new_with_storage(storage);
        let mut single_level_tree = InnerRewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);

        let account = random_account(&mut rng);
        let amount = random_amount(&mut rng);

        // Insert account
        two_level_tree.update(account, amount).unwrap();
        single_level_tree.update(account, amount).unwrap();
        assert_eq!(two_level_tree.num_leaves(), 1);

        // Remove account by returning None
        two_level_tree.update_with(&account, |_| None).unwrap();
        single_level_tree.update_with(&account, |_| None).unwrap();

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

        let storage = RewardMerkleTreeFSStorage::default();
        let mut two_level_tree = RewardMerkleTreeV2Impl::new_with_storage(storage);
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
}
