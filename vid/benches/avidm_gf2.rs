//! Benchmark of VID dispersal
use criterion::{Criterion, criterion_group, criterion_main};
use rand::RngCore;
use vid::{VidScheme, avidm_gf2::AvidmGf2Scheme};

fn avidm_gf2_benchmark(c: &mut Criterion) {
    let param_list = [(3400, 10000)];
    let payload_bytes_len_list = [5]; // in MB
    let mut payload = vec![0u8; 5 * 1024 * 1024];
    let distribution = [1u32; 10000];
    jf_utils::test_rng().fill_bytes(&mut payload);

    let mut avidm_gf2_group = c.benchmark_group("AvidM_GF2");
    for (recovery_threshold, num_storage_nodes) in param_list {
        let param = AvidmGf2Scheme::setup(recovery_threshold, num_storage_nodes).unwrap();
        for payload_bytes_len in payload_bytes_len_list {
            avidm_gf2_group.bench_function(
                format!(
                    "Disperse_({recovery_threshold}, {num_storage_nodes})_{payload_bytes_len}MB"
                ),
                |b| {
                    b.iter(|| {
                        AvidmGf2Scheme::disperse(
                            &param,
                            &distribution[..num_storage_nodes],
                            &payload[..payload_bytes_len * 1024 * 1024],
                        )
                    })
                },
            );

            let (commit, shares) = AvidmGf2Scheme::disperse(
                &param,
                &distribution[..num_storage_nodes],
                &payload[..payload_bytes_len * 1024 * 1024],
            )
            .unwrap();
            avidm_gf2_group.bench_function(
                format!("Verify_({recovery_threshold}, {num_storage_nodes})_{payload_bytes_len}MB"),
                |b| b.iter(|| AvidmGf2Scheme::verify_share(&param, &commit, &shares[0])),
            );

            avidm_gf2_group.bench_function(
                format!(
                    "Recovery_({recovery_threshold}, {num_storage_nodes})_{payload_bytes_len}MB"
                ),
                |b| {
                    b.iter(|| {
                        AvidmGf2Scheme::recover(
                            &param,
                            &commit,
                            &shares[recovery_threshold..2 * recovery_threshold],
                        )
                    })
                },
            );
        }
    }
    avidm_gf2_group.finish();
}

criterion_group!(benches, avidm_gf2_benchmark);
criterion_main!(benches);
