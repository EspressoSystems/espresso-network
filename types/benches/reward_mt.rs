//! Benchmarks for two reward Merkle tree implementations:
//! - Vanilla (`RewardMerkleTreeV2`): single-level 160-bit tree
//! - InMemory (`InMemoryRewardMerkleTreeV2`): two-level with cached in-memory storage

use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

use alloy::primitives::U256;
use criterion::Criterion;
use espresso_types::{
    reward_mt::{
        InMemoryRewardMerkleTreeV2, REWARD_MERKLE_TREE_V2_HEIGHT, RewardMerkleTreeV2,
        storage::OuterIndex,
    },
    v0_3::RewardAmount,
    v0_4::RewardAccountV2,
};
use jf_merkle_tree_compat::{MerkleTreeScheme, UniversalMerkleTreeScheme};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;

// ---------------------------------------------------------------------------
// Tracking allocator for memory measurement
// ---------------------------------------------------------------------------

struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static ALLOC: TrackingAllocator = TrackingAllocator;

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            ALLOCATED.fetch_add(layout.size(), Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        ALLOCATED.fetch_sub(layout.size(), Relaxed);
    }
}

fn allocated_bytes() -> usize {
    ALLOCATED.load(Relaxed)
}

// ---------------------------------------------------------------------------
// Constants and helpers
// ---------------------------------------------------------------------------

const NUM_ACCOUNTS: usize = 10_000;
const HEIGHT: usize = REWARD_MERKLE_TREE_V2_HEIGHT;

fn random_account(rng: &mut impl Rng) -> RewardAccountV2 {
    let mut bytes = [0u8; 20];
    rng.fill(&mut bytes);
    RewardAccountV2::from(bytes)
}

fn random_amount(rng: &mut impl Rng) -> RewardAmount {
    RewardAmount(U256::from(rng.r#gen::<u64>()))
}

fn generate_kv_pairs(n: usize) -> Vec<(RewardAccountV2, RewardAmount)> {
    let mut rng = ChaCha20Rng::seed_from_u64(42);
    (0..n)
        .map(|_| (random_account(&mut rng), random_amount(&mut rng)))
        .collect()
}

fn sort_by_partition(
    pairs: &[(RewardAccountV2, RewardAmount)],
) -> Vec<(RewardAccountV2, RewardAmount)> {
    let mut sorted = pairs.to_vec();
    sorted.sort_by_key(|(account, _)| (OuterIndex::new(account).0, *account));
    sorted
}

fn build_vanilla(kv_pairs: &[(RewardAccountV2, RewardAmount)]) -> RewardMerkleTreeV2 {
    let mut tree = RewardMerkleTreeV2::new(HEIGHT);
    for (account, amount) in kv_pairs {
        tree.update(*account, *amount).unwrap();
    }
    tree
}

fn build_in_memory(kv_pairs: &[(RewardAccountV2, RewardAmount)]) -> InMemoryRewardMerkleTreeV2 {
    let mut tree = InMemoryRewardMerkleTreeV2::new(HEIGHT);
    for (account, amount) in kv_pairs {
        tree.update(*account, *amount).unwrap();
    }
    tree
}

// ---------------------------------------------------------------------------
// Memory measurement (printed to stdout before Criterion runs)
// ---------------------------------------------------------------------------

fn measure_memory(
    kv_pairs: &[(RewardAccountV2, RewardAmount)],
    sorted_pairs: &[(RewardAccountV2, RewardAmount)],
) {
    println!("\nMemory usage ({} accounts):", kv_pairs.len());

    // Vanilla
    let before = allocated_bytes();
    let tree = build_vanilla(kv_pairs);
    let after = allocated_bytes();
    let delta = after.saturating_sub(before);
    println!(
        "  vanilla:   {} bytes ({:.2} MB)",
        delta,
        delta as f64 / 1_048_576.0
    );
    drop(tree);

    // InMemory (sorted construction to avoid cache thrashing)
    let before = allocated_bytes();
    let tree = build_in_memory(sorted_pairs);
    let after = allocated_bytes();
    let delta = after.saturating_sub(before);
    println!(
        "  in_memory: {} bytes ({:.2} MB)",
        delta,
        delta as f64 / 1_048_576.0
    );
    drop(tree);

    println!();
}

// ---------------------------------------------------------------------------
// Group 1: Construction benchmarks
// ---------------------------------------------------------------------------

fn bench_construct(c: &mut Criterion) {
    let kv_pairs = generate_kv_pairs(NUM_ACCOUNTS);
    let sorted_pairs = sort_by_partition(&kv_pairs);
    let mut group = c.benchmark_group("reward_mt/construct");

    // Vanilla — order-insensitive (no partitioning), random only
    group.bench_function("vanilla", |b| {
        b.iter(|| {
            let mut tree = RewardMerkleTreeV2::new(HEIGHT);
            for (account, amount) in &kv_pairs {
                tree.update(*account, *amount).unwrap();
            }
            tree
        });
    });

    // InMemory — random order (worst case: ~10K cache evictions)
    group.bench_function("in_memory_random", |b| {
        b.iter(|| {
            let mut tree = InMemoryRewardMerkleTreeV2::new(HEIGHT);
            for (account, amount) in &kv_pairs {
                tree.update(*account, *amount).unwrap();
            }
            tree
        });
    });

    // InMemory — sorted by partition (best case: 16 cache evictions)
    group.bench_function("in_memory_sorted", |b| {
        b.iter(|| {
            let mut tree = InMemoryRewardMerkleTreeV2::new(HEIGHT);
            for (account, amount) in &sorted_pairs {
                tree.update(*account, *amount).unwrap();
            }
            tree
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Group 2: Lookup benchmarks (10K lookups with proof generation)
// ---------------------------------------------------------------------------

fn bench_lookup_all(c: &mut Criterion) {
    let kv_pairs = generate_kv_pairs(NUM_ACCOUNTS);
    let sorted_pairs = sort_by_partition(&kv_pairs);
    let mut group = c.benchmark_group("reward_mt/lookup_all");

    // Vanilla — MerkleTreeScheme::lookup returns LookupResult directly
    group.bench_function("vanilla", |b| {
        b.iter_batched(
            || build_vanilla(&kv_pairs),
            |tree| {
                for (account, _) in &kv_pairs {
                    let _ = black_box(tree.lookup(*account));
                }
            },
            criterion::BatchSize::LargeInput,
        );
    });

    // InMemory — random order (setup uses sorted_pairs for fast construction)
    group.bench_function("in_memory_random", |b| {
        b.iter_batched(
            || build_in_memory(&sorted_pairs),
            |tree| {
                for (account, _) in &kv_pairs {
                    let _ = black_box(tree.lookup(*account));
                }
            },
            criterion::BatchSize::LargeInput,
        );
    });

    // InMemory — sorted by partition
    group.bench_function("in_memory_sorted", |b| {
        b.iter_batched(
            || build_in_memory(&sorted_pairs),
            |tree| {
                for (account, _) in &sorted_pairs {
                    let _ = black_box(tree.lookup(*account));
                }
            },
            criterion::BatchSize::LargeInput,
        );
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Entrypoint
// ---------------------------------------------------------------------------

fn main() {
    let kv_pairs = generate_kv_pairs(NUM_ACCOUNTS);
    let sorted_pairs = sort_by_partition(&kv_pairs);
    measure_memory(&kv_pairs, &sorted_pairs);

    let mut criterion = Criterion::default().sample_size(10).configure_from_args();
    bench_construct(&mut criterion);
    bench_lookup_all(&mut criterion);
    criterion.final_summary();
}
