//! Construction time for a single large Merkle tree of `Blake3Node` leaves.
//!
//! Leaves are 32 bytes each (one `Blake3Node`); the count is chosen so the
//! total leaf-byte sum equals the requested `total_mb`. Measures pure
//! `MerkleTree::from_elems` time — no RS encoding, no namespace structure,
//! no proof generation.
//!
//! Run with:
//!
//!     RAYON_NUM_THREADS=1 cargo bench --bench big_mt
use std::time::{Duration, Instant};

use jf_merkle_tree::{MerkleTreeScheme, append_only::MerkleTree as JfMerkleTree};
use rand::{RngCore, SeedableRng, rngs::SmallRng};
use vid::utils::blake3::{Blake3DigestAlgorithm, Blake3Node};

const ARITY: usize = 4;
type Mt = JfMerkleTree<Blake3Node, Blake3DigestAlgorithm, u64, ARITY, Blake3Node>;

fn human(d: Duration) -> String {
    let ns = d.as_nanos();
    if ns >= 1_000_000_000 {
        format!("{:>7.2} s", d.as_secs_f64())
    } else if ns >= 1_000_000 {
        format!("{:>7.2} ms", d.as_secs_f64() * 1e3)
    } else if ns >= 1_000 {
        format!("{:>7.2} µs", d.as_secs_f64() * 1e6)
    } else {
        format!("{:>7} ns", ns)
    }
}

fn run(total_mb: usize, trials: u32) {
    const LEAF_BYTES: usize = 32; // sizeof(Blake3Node)
    let total_bytes = total_mb * 1024 * 1024;
    let n_leaves = total_bytes / LEAF_BYTES;
    let depth = {
        let mut d = 0;
        let mut n = n_leaves;
        while n > 1 {
            n = n.div_ceil(ARITY);
            d += 1;
        }
        d
    };

    println!(
        "\n=== {} MB total, {} leaves of {} B (arity {}, depth {}) ===",
        total_mb, n_leaves, LEAF_BYTES, ARITY, depth
    );

    // Generate leaves once; they are reused across trials so timing is
    // construction-only (no rng cost).
    let mut rng = SmallRng::seed_from_u64(0xCAFEBEEF);
    let mut leaves: Vec<Blake3Node> = Vec::with_capacity(n_leaves);
    for _ in 0..n_leaves {
        let mut buf = [0u8; LEAF_BYTES];
        rng.fill_bytes(&mut buf);
        leaves.push(Blake3Node::new(buf));
    }

    // warmup
    let warm = Mt::from_elems(None, &leaves).unwrap();
    std::hint::black_box(warm.commitment());

    let mut times: Vec<Duration> = Vec::with_capacity(trials as usize);
    for _ in 0..trials {
        let start = Instant::now();
        let mt = Mt::from_elems(None, &leaves).unwrap();
        let elapsed = start.elapsed();
        std::hint::black_box(mt.commitment());
        times.push(elapsed);
    }
    times.sort();
    let median = times[times.len() / 2];
    let min = times[0];
    let max = times[times.len() - 1];

    let bytes_per_sec = total_bytes as f64 / median.as_secs_f64();
    let bytes_per_sec_mb = bytes_per_sec / (1024.0 * 1024.0);

    println!("  median:     {}", human(median));
    println!("  min:        {}", human(min));
    println!("  max:        {}", human(max));
    println!(
        "  throughput: {:.0} MB/s (over leaf-byte sum)",
        bytes_per_sec_mb
    );
    println!(
        "  per-leaf:   {} (incl. internal node share)",
        human(median / n_leaves as u32)
    );
}

fn main() {
    if std::env::args().any(|a| matches!(a.as_str(), "--list" | "--ignored" | "--exact")) {
        return;
    }

    let trials = 5u32;
    println!(
        "MT construction over Blake3Node leaves — Blake3DigestAlgorithm, arity {}, trials = {}",
        ARITY, trials
    );
    println!("(set RAYON_NUM_THREADS=1 for single-threaded measurement)");

    // Primary target: 30 MB.
    run(30, trials);

    // A few neighbouring sizes for context.
    run(10, trials);
    run(60, trials);
}
