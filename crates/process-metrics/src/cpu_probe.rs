//! One-shot startup probe of CPU features relevant to DRB SHA-256 hashing speed.
//!
//! The DRB computation (`crates/hotshot/types/src/drb.rs`) chain-hashes a 32-byte value up to
//! 5e9 times per epoch transition. On hardware with SHA extensions that takes minutes; without
//! them, `sha2` falls back to a software implementation roughly 8x slower, which can stall
//! consensus at epoch boundaries. External node operators run arbitrary hardware, including VMs
//! whose CPU model (e.g. `qemu64`) masks features the host actually has, so this logs what the
//! process sees once per start. Never fails startup: every part that can error degrades to an
//! absent value.

use std::time::{Duration, Instant};

use humantime::format_duration;
use sha2::{Digest, Sha256};
use sysinfo::{CpuRefreshKind, RefreshKind, System};

/// Iterations of the DRB's chained hash loop to time. ~50ms on a software-SHA host, ~6ms with
/// SHA-NI.
const HASH_SAMPLES: u64 = 125_000;

/// Untimed iterations before timing, so `sha2`'s one-time CPU feature detection and cold caches
/// don't skew the sample.
const WARMUP_SAMPLES: u64 = 5_000;

/// `ns_per_hash` above which DRB computation is noticeably slower than on hardware with SHA
/// extensions (~48ns/hash); software SHA-256 runs at ~400ns/hash.
const SLOW_NS_PER_HASH: u128 = 150;

#[derive(Debug, Default)]
struct CpuFeatures {
    sha: Option<bool>,
    avx2: Option<bool>,
    avx512f: Option<bool>,
    adx: Option<bool>,
    bmi2: Option<bool>,
    aes: Option<bool>,
    pclmulqdq: Option<bool>,
    sha2: Option<bool>,
}

struct CpuProbe {
    model: Option<String>,
    cpus: Option<usize>,
    features: CpuFeatures,
    ns_per_hash: u128,
}

/// Runs the blocking half on the blocking pool, so the ~50ms hash timing doesn't stall the async
/// runtime. Never fails startup: a failed blocking task is logged and skipped.
pub async fn log_cpu_probe(drb_difficulty: Option<u64>) {
    match tokio::task::spawn_blocking(run_probe).await {
        Ok(probe) => probe.log(drb_difficulty),
        Err(err) => tracing::warn!(%err, "cpu probe: probe task failed, skipping"),
    }
}

fn run_probe() -> CpuProbe {
    CpuProbe {
        model: cpu_model(),
        cpus: std::thread::available_parallelism().ok().map(|n| n.get()),
        features: detect_features(),
        ns_per_hash: measure_ns_per_hash(),
    }
}

impl CpuProbe {
    fn log(&self, drb_difficulty: Option<u64>) {
        let drb_estimate =
            drb_difficulty.map(|d| format_duration(drb_duration(self.ns_per_hash, d)).to_string());
        tracing::info!(
            target: "announce",
            model = self.model.as_deref(),
            cpus = self.cpus,
            sha = self.features.sha,
            avx2 = self.features.avx2,
            avx512f = self.features.avx512f,
            adx = self.features.adx,
            bmi2 = self.features.bmi2,
            aes = self.features.aes,
            pclmulqdq = self.features.pclmulqdq,
            sha2 = self.features.sha2,
            ns_per_hash = self.ns_per_hash as u64,
            drb_difficulty,
            drb_estimate = drb_estimate.as_deref(),
            "cpu probe"
        );

        if self.ns_per_hash > SLOW_NS_PER_HASH {
            let estimate = drb_estimate.as_deref().unwrap_or("longer than usual");
            let sha_missing = self.features.sha == Some(false) || self.features.sha2 == Some(false);
            if sha_missing {
                tracing::warn!(
                    "cpu probe: SHA-256 runs in software on this machine ({}ns/hash), the CPU \
                     reports no SHA extensions; each DRB computation takes {estimate} and can \
                     delay epoch transitions. If this is a VM, set the CPU type to host \
                     passthrough.",
                    self.ns_per_hash,
                );
            } else {
                tracing::warn!(
                    "cpu probe: SHA-256 runs slowly on this machine ({}ns/hash) despite SHA \
                     extensions, likely CPU contention, throttling or a non-release build; each \
                     DRB computation takes {estimate} and can delay epoch transitions.",
                    self.ns_per_hash,
                );
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
fn detect_features() -> CpuFeatures {
    CpuFeatures {
        sha: Some(std::arch::is_x86_feature_detected!("sha")),
        avx2: Some(std::arch::is_x86_feature_detected!("avx2")),
        avx512f: Some(std::arch::is_x86_feature_detected!("avx512f")),
        adx: Some(std::arch::is_x86_feature_detected!("adx")),
        bmi2: Some(std::arch::is_x86_feature_detected!("bmi2")),
        aes: Some(std::arch::is_x86_feature_detected!("aes")),
        pclmulqdq: Some(std::arch::is_x86_feature_detected!("pclmulqdq")),
        ..Default::default()
    }
}

#[cfg(target_arch = "aarch64")]
fn detect_features() -> CpuFeatures {
    CpuFeatures {
        sha2: Some(std::arch::is_aarch64_feature_detected!("sha2")),
        aes: Some(std::arch::is_aarch64_feature_detected!("aes")),
        ..Default::default()
    }
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn detect_features() -> CpuFeatures {
    CpuFeatures::default()
}

fn cpu_model() -> Option<String> {
    let system =
        System::new_with_specifics(RefreshKind::nothing().with_cpu(CpuRefreshKind::nothing()));
    system.cpus().first().map(|cpu| cpu.brand().to_string())
}

/// Estimated time of one DRB computation of `difficulty` chained hashes, in whole seconds.
fn drb_duration(ns_per_hash: u128, difficulty: u64) -> Duration {
    let secs = ns_per_hash.saturating_mul(u128::from(difficulty)) / 1_000_000_000;
    Duration::from_secs(u64::try_from(secs).unwrap_or(u64::MAX))
}

/// Times the same chained `Sha256::digest` loop the DRB uses
/// (`crates/hotshot/types/src/drb.rs`), so the result predicts real DRB throughput on this
/// machine.
fn measure_ns_per_hash() -> u128 {
    let hash = hash_chain([0u8; 32], WARMUP_SAMPLES);
    let start = Instant::now();
    std::hint::black_box(hash_chain(hash, HASH_SAMPLES));
    start.elapsed().as_nanos() / u128::from(HASH_SAMPLES)
}

fn hash_chain(mut hash: [u8; 32], iterations: u64) -> [u8; 32] {
    for _ in 0..iterations {
        hash = std::hint::black_box(Sha256::digest(std::hint::black_box(hash)).into());
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drb_duration_scales_with_difficulty() {
        assert_eq!(drb_duration(48, 5_000_000_000), Duration::from_secs(240));
    }
}
