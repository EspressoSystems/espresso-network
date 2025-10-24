//! Benchmark of VID dispersal
use criterion::{criterion_group, criterion_main, Criterion};
use rand::RngCore;
use vid::{avidm::AvidMScheme, VidScheme};

fn verify_benchmark(c: &mut Criterion) {
    let param_list = [(6, 10), (50, 100)];
    let payload_bytes_len_list = [32 * 1024 * 1024];
    let mut payload = vec![0u8; 32 * 1024 * 1024];
    let distribution = [1u32; 1000];
    jf_utils::test_rng().fill_bytes(&mut payload);

    let mut avidm_group = c.benchmark_group("AvidM");
    for (recovery_threshold, num_storage_nodes) in param_list {
        let param = AvidMScheme::setup(recovery_threshold, num_storage_nodes).unwrap();
        for payload_bytes_len in payload_bytes_len_list {
            let (commit, shares) = AvidMScheme::disperse(
                &param,
                &distribution[..num_storage_nodes],
                &payload[..payload_bytes_len],
            )
            .unwrap();
            avidm_group.bench_function(
                format!(
                    "AvidMVerify_({recovery_threshold}, {num_storage_nodes})_{payload_bytes_len}"
                ),
                |b| b.iter(|| AvidMScheme::verify_share(&param, &commit, &shares[0])),
            );
        }
    }
    avidm_group.finish();
}

criterion_group!(benches, verify_benchmark);
criterion_main!(benches);
