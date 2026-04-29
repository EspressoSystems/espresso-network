//! Runtime breakdown of AvidM-GF2 dispersal and recovery.
//!
//! Replicates the core steps of `AvidmGf2Scheme::disperse`/`recover` inline
//! with per-phase `Instant::now()` timing, so the attribution is *measured*
//! rather than inferred. Parameterized on the Merkle hasher so we can
//! compare Keccak-256 and BLAKE3 in one run without recompiling.
//!
//! Run with:
//!
//!     RAYON_NUM_THREADS=1 cargo bench --bench avidm_gf2_breakdown
use std::{
    ops::Range,
    time::{Duration, Instant},
};

use jf_merkle_tree::{
    MerkleTreeScheme,
    append_only::MerkleTree as JfMerkleTree,
    hasher::{GenericHasherMerkleTree, HasherNode},
    prelude::MerkleTreeProof,
};
use p3_maybe_rayon::prelude::*;
use rand::{RngCore, SeedableRng, rngs::SmallRng};
use sha2::Digest as _;
use vid::utils::blake3::{Blake3DigestAlgorithm, Blake3Node};

/// Abstract Merkle backend so we can compare two `DigestAlgorithm` paths
/// (HasherDigest+Keccak vs native-blake3) without duplicating the bench.
trait MerkleBackend: 'static {
    type Mt: Send + Sync;
    type Node: Copy + Send + Sync + 'static;
    type Proof: Clone + Send + 'static;

    fn hash_leaf(data: &[u8]) -> Self::Node;
    fn from_leaves(leaves: &[Self::Node]) -> Self::Mt;
    fn empty_proof() -> Self::Proof;
    /// Collect every leaf's proof in one tree traversal, indexed by leaf
    /// position. Mirrors the production path in `AvidmGf2Scheme::disperse`.
    fn collect_proofs(mt: &Self::Mt, total_weights: usize) -> Vec<Self::Proof>;
    fn commitment(mt: &Self::Mt) -> Self::Node;
}

struct KeccakBackend;

impl MerkleBackend for KeccakBackend {
    type Mt = GenericHasherMerkleTree<sha3::Keccak256, HasherNode<sha3::Keccak256>, u64, 4>;
    type Node = HasherNode<sha3::Keccak256>;
    type Proof = <Self::Mt as MerkleTreeScheme>::MembershipProof;

    fn hash_leaf(data: &[u8]) -> Self::Node {
        HasherNode::from(sha3::Keccak256::digest(data))
    }
    fn from_leaves(leaves: &[Self::Node]) -> Self::Mt {
        Self::Mt::from_elems(None, leaves).unwrap()
    }
    fn empty_proof() -> Self::Proof {
        MerkleTreeProof(Vec::new())
    }
    fn collect_proofs(mt: &Self::Mt, total_weights: usize) -> Vec<Self::Proof> {
        let mut proofs = vec![Self::empty_proof(); total_weights];
        for (i, _, proof) in mt.collect_leaves_with_proof() {
            proofs[*i as usize] = proof;
        }
        proofs
    }
    fn commitment(mt: &Self::Mt) -> Self::Node {
        mt.commitment()
    }
}

struct Blake3Backend;

impl MerkleBackend for Blake3Backend {
    type Mt = JfMerkleTree<Blake3Node, Blake3DigestAlgorithm, u64, 4, Blake3Node>;
    type Node = Blake3Node;
    type Proof = <Self::Mt as MerkleTreeScheme>::MembershipProof;

    fn hash_leaf(data: &[u8]) -> Self::Node {
        Blake3Node::from(blake3::hash(data))
    }
    fn from_leaves(leaves: &[Self::Node]) -> Self::Mt {
        Self::Mt::from_elems(None, leaves).unwrap()
    }
    fn empty_proof() -> Self::Proof {
        MerkleTreeProof(Vec::new())
    }
    fn collect_proofs(mt: &Self::Mt, total_weights: usize) -> Vec<Self::Proof> {
        let mut proofs = vec![Self::empty_proof(); total_weights];
        for (i, _, proof) in mt.collect_leaves_with_proof() {
            proofs[*i as usize] = proof;
        }
        proofs
    }
    fn commitment(mt: &Self::Mt) -> Self::Node {
        mt.commitment()
    }
}

// ------------------------------- disperse ----------------------------------

#[derive(Default, Clone, Copy)]
struct DispT {
    pad_chunk: Duration,
    rs_encode: Duration,
    leaf_hash: Duration,
    mt_build: Duration,
    share_assemble: Duration,
    outer_mt: Duration,
}

impl DispT {
    fn add(&mut self, o: &DispT) {
        self.pad_chunk += o.pad_chunk;
        self.rs_encode += o.rs_encode;
        self.leaf_hash += o.leaf_hash;
        self.mt_build += o.mt_build;
        self.share_assemble += o.share_assemble;
        self.outer_mt += o.outer_mt;
    }
    fn div(&mut self, n: u32) {
        self.pad_chunk /= n;
        self.rs_encode /= n;
        self.leaf_hash /= n;
        self.mt_build /= n;
        self.share_assemble /= n;
        self.outer_mt /= n;
    }
    fn total(&self) -> Duration {
        self.pad_chunk
            + self.rs_encode
            + self.leaf_hash
            + self.mt_build
            + self.share_assemble
            + self.outer_mt
    }
}

struct RawShare {
    range: Range<usize>,
    payload: Vec<Vec<u8>>,
}

fn raw_disperse<B: MerkleBackend>(
    recovery_threshold: usize,
    total_weights: usize,
    distribution: &[u32],
    payload: &[u8],
    t: &mut DispT,
) -> (B::Mt, Vec<RawShare>) {
    let orig = recovery_threshold;
    let rec = total_weights - recovery_threshold;

    let t0 = Instant::now();
    let mut shard = (payload.len() + 1).div_ceil(orig);
    if shard % 2 == 1 {
        shard += 1;
    }
    let mut original: Vec<Vec<u8>> = Vec::with_capacity(orig);
    for i in 0..orig {
        let start = i * shard;
        let mut chunk = vec![0u8; shard];
        if start < payload.len() {
            let end = ((i + 1) * shard).min(payload.len());
            let take = end - start;
            chunk[..take].copy_from_slice(&payload[start..end]);
            if take < shard {
                chunk[take] = 1u8;
            }
        } else if start == payload.len() {
            chunk[0] = 1u8;
        }
        original.push(chunk);
    }
    t.pad_chunk += t0.elapsed();

    let t1 = Instant::now();
    let recovery = if rec == 0 {
        vec![]
    } else {
        reed_solomon_simd::encode(orig, rec, &original).unwrap()
    };
    t.rs_encode += t1.elapsed();

    let shares: Vec<Vec<u8>> = [original, recovery].concat();

    let t2 = Instant::now();
    let digests: Vec<B::Node> = shares.par_iter().map(|s| B::hash_leaf(s)).collect();
    t.leaf_hash += t2.elapsed();

    let t3 = Instant::now();
    let mt = B::from_leaves(&digests);
    t.mt_build += t3.elapsed();

    let ranges: Vec<Range<usize>> = distribution
        .iter()
        .scan(0usize, |sum, w| {
            let p = *sum;
            *sum += *w as usize;
            Some(p..*sum)
        })
        .collect();

    // Lever D: consume owned shares via an iterator instead of cloning each
    // per-recipient Vec<u8>. Ranges partition `shares` in order. Same drain
    // pattern for proofs — `B::collect_proofs` walks the tree once instead
    // of `range.len()` separate `mt.lookup(k)` calls per recipient.
    let t4 = Instant::now();
    let total_weights = distribution.iter().map(|&w| w as usize).sum::<usize>();
    let proofs = B::collect_proofs(&mt, total_weights);
    let mut shares_iter = shares.into_iter();
    let payloads: Vec<Vec<Vec<u8>>> = ranges
        .iter()
        .map(|range| shares_iter.by_ref().take(range.len()).collect())
        .collect();
    let mut proofs_iter = proofs.into_iter();
    let proof_groups: Vec<Vec<B::Proof>> = ranges
        .iter()
        .map(|range| proofs_iter.by_ref().take(range.len()).collect())
        .collect();
    let out: Vec<RawShare> = ranges
        .into_iter()
        .zip(payloads)
        .zip(proof_groups)
        .map(|((range, payload), _mt_proofs)| RawShare { range, payload })
        .collect();
    t.share_assemble += t4.elapsed();
    std::hint::black_box(&out);

    (mt, out)
}

fn ns_disperse<B: MerkleBackend>(
    recovery_threshold: usize,
    total_weights: usize,
    distribution: &[u32],
    payload: &[u8],
    num_ns: usize,
) -> (DispT, Vec<Vec<RawShare>>) {
    let mut t = DispT::default();
    let ns_size = payload.len() / num_ns;
    let mut commits: Vec<B::Node> = Vec::with_capacity(num_ns);
    let mut all = Vec::with_capacity(num_ns);
    for i in 0..num_ns {
        let end = if i + 1 == num_ns {
            payload.len()
        } else {
            (i + 1) * ns_size
        };
        let (mt, shares) = raw_disperse::<B>(
            recovery_threshold,
            total_weights,
            distribution,
            &payload[i * ns_size..end],
            &mut t,
        );
        commits.push(B::commitment(&mt));
        all.push(shares);
    }
    let t_outer = Instant::now();
    let outer = B::from_leaves(&commits);
    t.outer_mt = t_outer.elapsed();
    std::hint::black_box(outer);
    (t, all)
}

// ------------------------------- recover ----------------------------------

#[derive(Default, Clone, Copy)]
struct RecT {
    decoder_setup: Duration,
    add_shards: Duration,
    decode: Duration,
    restore: Duration,
    concat_unpad: Duration,
}

impl RecT {
    fn add(&mut self, o: &RecT) {
        self.decoder_setup += o.decoder_setup;
        self.add_shards += o.add_shards;
        self.decode += o.decode;
        self.restore += o.restore;
        self.concat_unpad += o.concat_unpad;
    }
    fn div(&mut self, n: u32) {
        self.decoder_setup /= n;
        self.add_shards /= n;
        self.decode /= n;
        self.restore /= n;
        self.concat_unpad /= n;
    }
    fn total(&self) -> Duration {
        self.decoder_setup + self.add_shards + self.decode + self.restore + self.concat_unpad
    }
}

fn recover_one(
    recovery_threshold: usize,
    total_weights: usize,
    shares: &[RawShare],
    t: &mut RecT,
) -> Vec<u8> {
    let orig = recovery_threshold;
    let rec = total_weights - recovery_threshold;
    let first = shares.iter().find(|s| !s.payload.is_empty()).unwrap();
    let shard_bytes = first.payload[0].len();
    let mut input_orig: Vec<Option<&[u8]>> = vec![None; orig];

    let t0 = Instant::now();
    let mut dec = reed_solomon_simd::ReedSolomonDecoder::new(orig, rec, shard_bytes).unwrap();
    t.decoder_setup += t0.elapsed();

    let t1 = Instant::now();
    for share in shares {
        for (i, idx) in share.range.clone().enumerate() {
            let shard = &share.payload[i];
            if idx < orig {
                input_orig[idx] = Some(shard);
                dec.add_original_shard(idx, shard).unwrap();
            } else {
                dec.add_recovery_shard(idx - orig, shard).unwrap();
            }
        }
    }
    t.add_shards += t1.elapsed();

    let t2 = Instant::now();
    let result = dec.decode().unwrap();
    t.decode += t2.elapsed();

    let t4 = Instant::now();
    let mut recovered: Vec<u8> = Vec::with_capacity(orig * shard_bytes);
    for (i, shard) in input_orig.iter().enumerate().take(orig) {
        let shard: &[u8] = match shard {
            Some(data) => data,
            None => result.restored_original(i).unwrap(),
        };
        recovered.extend_from_slice(shard);
    }
    if let Some(idx) = recovered.iter().rposition(|&b| b != 0)
        && recovered[idx] == 1u8
    {
        recovered.truncate(idx);
    }
    t.concat_unpad += t4.elapsed();
    // `restore` phase no longer exists — reconstructed shards are memcpy'd
    // straight from the decoder into `recovered` above (counted under
    // `concat_unpad`).
    recovered
}

fn ns_recover(
    recovery_threshold: usize,
    total_weights: usize,
    per_ns_shares: &[Vec<RawShare>],
) -> RecT {
    let mut t = RecT::default();
    for shares in per_ns_shares {
        // mimic the bench's "recover from all-recovery shards" worst case:
        // shares[recovery_threshold..2*recovery_threshold]
        let subset: Vec<RawShare> = shares
            .iter()
            .skip(recovery_threshold)
            .take(recovery_threshold)
            .map(|s| RawShare {
                range: s.range.clone(),
                payload: s.payload.clone(),
            })
            .collect();
        recover_one(recovery_threshold, total_weights, &subset, &mut t);
    }
    t
}

// ------------------------------- main -------------------------------------

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

fn pct(part: Duration, whole: Duration) -> String {
    if whole.as_nanos() == 0 {
        "   -  ".into()
    } else {
        format!("{:>5.1}%", 100.0 * part.as_secs_f64() / whole.as_secs_f64())
    }
}

fn print_disp(label: &str, t: &DispT) {
    let tot = t.total();
    println!("  {label:<18}  total {}", human(tot));
    println!(
        "    pad+chunk       {}  ({})",
        human(t.pad_chunk),
        pct(t.pad_chunk, tot)
    );
    println!(
        "    RS encode       {}  ({})",
        human(t.rs_encode),
        pct(t.rs_encode, tot)
    );
    println!(
        "    leaf hash       {}  ({})",
        human(t.leaf_hash),
        pct(t.leaf_hash, tot)
    );
    println!(
        "    MT build        {}  ({})",
        human(t.mt_build),
        pct(t.mt_build, tot)
    );
    println!(
        "    share assemble  {}  ({})",
        human(t.share_assemble),
        pct(t.share_assemble, tot)
    );
    println!(
        "    outer MT        {}  ({})",
        human(t.outer_mt),
        pct(t.outer_mt, tot)
    );
}

fn print_rec(label: &str, t: &RecT) {
    let tot = t.total();
    println!("  {label:<18}  total {}", human(tot));
    println!(
        "    decoder setup   {}  ({})",
        human(t.decoder_setup),
        pct(t.decoder_setup, tot)
    );
    println!(
        "    add shards      {}  ({})",
        human(t.add_shards),
        pct(t.add_shards, tot)
    );
    println!(
        "    decode          {}  ({})",
        human(t.decode),
        pct(t.decode, tot)
    );
    println!(
        "    restore         {}  ({})",
        human(t.restore),
        pct(t.restore, tot)
    );
    println!(
        "    concat + unpad  {}  ({})",
        human(t.concat_unpad),
        pct(t.concat_unpad, tot)
    );
}

fn run<B: MerkleBackend>(
    label: &str,
    recovery_threshold: usize,
    total_weights: usize,
    distribution: &[u32],
    payload: &[u8],
    num_ns: usize,
    trials: u32,
) -> (DispT, RecT) {
    // one warm-up
    let (_, shares) = ns_disperse::<B>(
        recovery_threshold,
        total_weights,
        distribution,
        payload,
        num_ns,
    );
    let _ = ns_recover(recovery_threshold, total_weights, &shares);

    let mut disp = DispT::default();
    let mut rec = RecT::default();
    let mut last_shares = None;
    for _ in 0..trials {
        let (t, shares) = ns_disperse::<B>(
            recovery_threshold,
            total_weights,
            distribution,
            payload,
            num_ns,
        );
        disp.add(&t);
        last_shares = Some(shares);
    }
    let shares = last_shares.unwrap();
    for _ in 0..trials {
        let t = ns_recover(recovery_threshold, total_weights, &shares);
        rec.add(&t);
    }
    disp.div(trials);
    rec.div(trials);
    println!();
    print_disp(&format!("Disperse {label}"), &disp);
    print_rec(&format!("Recover  {label}"), &rec);
    (disp, rec)
}

fn main() {
    // `cargo nextest run --benches` (and `cargo test --benches`) invokes
    // each bench binary with libtest-style flags for test discovery
    // (`--list`, `--ignored`, etc.). This bench has no tests; respond
    // cleanly so the outer test runner doesn't try to parse the bench
    // output. `cargo bench` doesn't pass these flags, so the real bench
    // runs as usual.
    if std::env::args().any(|a| matches!(a.as_str(), "--list" | "--ignored" | "--exact")) {
        return;
    }

    const RECOVERY_THRESHOLD: usize = 340;
    const TOTAL_WEIGHTS: usize = 1000;
    const PAYLOAD_MB: usize = 10;

    let mut payload = vec![0u8; PAYLOAD_MB * 1024 * 1024];
    SmallRng::seed_from_u64(0xCAFEBEEF).fill_bytes(&mut payload);
    let distribution = vec![1u32; TOTAL_WEIGHTS];

    let ns_counts = [1usize, 10, 100];
    let trials = 3u32;

    println!(
        "AvidM-GF2 breakdown — arity 4, recovery_threshold = {}, total_weights = {}, payload = {} \
         MB, trials = {}",
        RECOVERY_THRESHOLD, TOTAL_WEIGHTS, PAYLOAD_MB, trials
    );
    println!("(set RAYON_NUM_THREADS=1 for single-threaded measurement)");

    for &num_ns in &ns_counts {
        println!("\n========================= num_ns = {num_ns} =========================");
        run::<Blake3Backend>(
            "BLAKE3",
            RECOVERY_THRESHOLD,
            TOTAL_WEIGHTS,
            &distribution,
            &payload,
            num_ns,
            trials,
        );
        run::<KeccakBackend>(
            "Keccak",
            RECOVERY_THRESHOLD,
            TOTAL_WEIGHTS,
            &distribution,
            &payload,
            num_ns,
            trials,
        );
    }
}
