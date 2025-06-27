//! Benchmark of VID dispersal
use criterion::{Criterion, criterion_group, criterion_main};
use rand::RngCore;
use vid::{VidScheme, avid_m::AvidMScheme};

fn verify_benchmark(c: &mut Criterion) {
    let param_list = [(6, 10), (50, 100)];
    let payload_bytes_len_list = [1024 * 1024];
    let mut payload = vec![0u8; 32 * 1024 * 1024];
    let distribution = [1u32; 1000];
    jf_utils::test_rng().fill_bytes(&mut payload);

    let mut avidm_group = c.benchmark_group("AvidM");
    avidm_group.sample_size(10);
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
                    "AvidMRecovery_({}, {})_{}",
                    recovery_threshold, num_storage_nodes, payload_bytes_len
                ),
                |b| b.iter(|| AvidMScheme::recover(&param, &commit, &shares[..recovery_threshold])),
            );
        }
    }
    avidm_group.finish();
}

criterion_group!(benches, verify_benchmark);
criterion_main!(benches);
