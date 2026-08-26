//! Benchmark of namespaced AvidM-GF2 dispersal.
//!
//! Companion to `avidm_gf2.rs`: same committee size and total payload, but
//! splits the payload across a varying number of namespaces so we can measure
//! the per-namespace overhead (separate RS encode + Merkle tree per ns, plus
//! the top-level Merkle tree over namespace commits).
//!
//! Use together with `avidm_gf2.rs` to test hypothesis (1) from causes.md:
//! namespaced dispersal is strictly more work than a single flat dispersal of
//! the same byte budget.
use std::ops::Range;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rand::RngCore;
use vid::avidm_gf2::namespaced::NsAvidmGf2Scheme;

/// Split `total_len` into `n` contiguous, near-equal ranges covering the whole payload.
fn equal_ns_table(total_len: usize, n: usize) -> Vec<Range<usize>> {
    assert!(n > 0 && n <= total_len);
    let base = total_len / n;
    let rem = total_len % n;
    let mut table = Vec::with_capacity(n);
    let mut start = 0;
    for i in 0..n {
        let len = base + usize::from(i < rem);
        table.push(start..start + len);
        start += len;
    }
    debug_assert_eq!(start, total_len);
    table
}

fn avidm_gf2_ns_benchmark(c: &mut Criterion) {
    // Match the parameters in `avidm_gf2.rs` so the flat-vs-namespaced
    // comparison is apples-to-apples.
    let recovery_threshold = 340usize;
    let num_storage_nodes = 1000usize;
    let payload_mb = 10usize;
    let payload_len = payload_mb * 1024 * 1024;

    // Namespace counts to sweep. 1 isolates "ns_disperse with a single ns" vs
    // flat `disperse`; the larger values show how fixed per-ns overheads scale.
    let ns_counts = [1usize, 10, 50, 100];

    let mut payload = vec![0u8; payload_len];
    let distribution = vec![1u32; num_storage_nodes];
    jf_utils::test_rng().fill_bytes(&mut payload);

    let param = NsAvidmGf2Scheme::setup(recovery_threshold, num_storage_nodes).unwrap();

    let mut group = c.benchmark_group("AvidM_GF2_NS");

    for &num_ns in &ns_counts {
        let ns_table = equal_ns_table(payload_len, num_ns);

        group.bench_with_input(
            BenchmarkId::new(
                format!("Disperse_({recovery_threshold}, {num_storage_nodes})_{payload_mb}MB"),
                num_ns,
            ),
            &num_ns,
            |b, _| {
                b.iter(|| {
                    NsAvidmGf2Scheme::ns_disperse(
                        &param,
                        &distribution,
                        &payload,
                        ns_table.iter().cloned(),
                    )
                    .unwrap()
                })
            },
        );

        let (commit, common, shares) = NsAvidmGf2Scheme::ns_disperse(
            &param,
            &distribution,
            &payload,
            ns_table.iter().cloned(),
        )
        .unwrap();

        group.bench_with_input(
            BenchmarkId::new(
                format!("Verify_({recovery_threshold}, {num_storage_nodes})_{payload_mb}MB"),
                num_ns,
            ),
            &num_ns,
            |b, _| b.iter(|| NsAvidmGf2Scheme::verify_share(&commit, &common, &shares[0]).unwrap()),
        );

        group.bench_with_input(
            BenchmarkId::new(
                format!("Recovery_({recovery_threshold}, {num_storage_nodes})_{payload_mb}MB"),
                num_ns,
            ),
            &num_ns,
            |b, _| {
                b.iter(|| {
                    NsAvidmGf2Scheme::recover(
                        &common,
                        &shares[recovery_threshold..2 * recovery_threshold],
                    )
                    .unwrap()
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, avidm_gf2_ns_benchmark);
criterion_main!(benches);
