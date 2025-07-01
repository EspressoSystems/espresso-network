//! Benchmark of VID dispersal
use criterion::{criterion_group, criterion_main, Criterion};
use rand::RngCore;
use vid::{avid_m::AvidMScheme, VidScheme};

fn dispersal_benchmark(c: &mut Criterion) {
    let param_list = [(50, 100)];
    let payload_bytes_len_list = [32 * 1024 * 1024];
    let mut payload = vec![0u8; 32 * 1024 * 1024];
    let distribution = [1u32; 1000];
    jf_utils::test_rng().fill_bytes(&mut payload);

    let mut avidm_group = c.benchmark_group("AvidM");
    for (recovery_threshold, num_storage_nodes) in param_list {
        let param = AvidMScheme::setup(recovery_threshold, num_storage_nodes).unwrap();
        for payload_bytes_len in payload_bytes_len_list {
            avidm_group.bench_function(
                format!(
                    "AvidMDisperse_({recovery_threshold}, {num_storage_nodes})_{payload_bytes_len}"
                ),
                |b| {
                    b.iter(|| {
                        AvidMScheme::disperse(
                            &param,
                            &distribution[..num_storage_nodes],
                            &payload[..payload_bytes_len],
                        )
                    })
                },
            );
        }
    }
    avidm_group.finish();
}

criterion_group!(benches, dispersal_benchmark);
criterion_main!(benches);
