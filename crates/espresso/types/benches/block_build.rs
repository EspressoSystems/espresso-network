//! The leader's block build (`request_block` in crates/hotshot/new-protocol/src/block.rs), timed
//! step by step.
//!
//! `request_block` runs `from_transactions` on a copy of the cached transactions, then
//! `hotshot_new_protocol::block::block_commitments`, the function the leader calls. A tracing
//! layer records each step span's start and duration; when `CRITERION_HOME` is set they are
//! written to `steps.json` next to the benchmark's criterion output. Step samples start with
//! criterion's warm-up iterations; scripts/bench-block-build keeps one per measured iteration,
//! which assumes every recorded span opens exactly once per iteration.
//!
//! Inputs mirror the network-bench load generator: random 1 MiB transactions round-robin over
//! 16 namespaces in 30 to 70 MiB blocks, protocol version 0.6 (AvidmGf2), 100 nodes. Real stakes
//! are large, so the VID total weight is the `approximate_weights` result for 100 equal stakes
//! (1100).
//!
//! To compare two git refs, run `just bench-block-build BASE HEAD` (scripts/bench-block-build).

use std::{
    collections::BTreeMap,
    env, fs, mem,
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};

use criterion::{BenchmarkId, Criterion, SamplingMode};
use espresso_types::{ChainConfig, NamespaceId, NsTable, Payload, SeqTypes, Transaction};
use hotshot_new_protocol::block::{BlockCommitments, block_commitments};
use hotshot_types::{consensus::PayloadWithMetadata, data::vid_disperse::VID_TARGET_TOTAL_STAKE};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use tracing::{Level, Subscriber, span};
use tracing_subscriber::{
    Layer,
    filter::Targets,
    layer::{Context, SubscriberExt},
    registry::LookupSpan,
};
use versions::NEW_PROTOCOL_VERSION;

const MIB: usize = 1 << 20;
const NAMESPACES: u64 = 16;
const NODES: usize = 100;
const BLOCK_MIB: [usize; 5] = [30, 40, 50, 60, 70];
/// `approximate_weights` total for `NODES` equal stakes above `VID_TARGET_TOTAL_STAKE`.
const VID_WEIGHT: usize = NODES * (VID_TARGET_TOTAL_STAKE as usize / NODES + 1);

fn bench_block_build(c: &mut Criterion) {
    install_step_timer();
    let threads = rayon::current_num_threads();
    let mut group = c.benchmark_group("block_build");
    group
        .sampling_mode(SamplingMode::Flat)
        .sample_size(10)
        .warm_up_time(Duration::from_millis(200))
        .measurement_time(Duration::from_secs(1));
    for block_mib in BLOCK_MIB {
        let label = format!("{block_mib}MiB_t{threads}_n{NODES}");
        let txs = transactions(block_mib);
        group.bench_function(BenchmarkId::new("request_block", &label), |b| {
            b.iter_with_large_drop(|| request_block(&txs))
        });
        write_steps(&format!("block_build/request_block/{label}"));
    }
    group.finish();
}

/// The leader's build task after the transactions are taken; outputs are returned so their drop
/// is untimed.
fn request_block(
    txs: &[Transaction],
) -> (PayloadWithMetadata<SeqTypes>, BlockCommitments<SeqTypes>) {
    let _span = tracing::debug_span!("request_block").entered();
    // `from_transactions` takes owned transactions, so the leader clones its cached ones
    // (`transactions_for` in block.rs).
    let (payload, metadata): (Payload, NsTable) =
        Payload::from_transactions_sync(txs.to_vec(), chain_config())
            .expect("payload construction");
    let payload = PayloadWithMetadata { payload, metadata };
    let commitments = block_commitments(&payload, VID_WEIGHT, NEW_PROTOCOL_VERSION);
    (payload, commitments)
}

fn transactions(block_mib: usize) -> Vec<Transaction> {
    let mut rng = ChaCha20Rng::seed_from_u64(block_mib as u64);
    (0..block_mib)
        .map(|i| {
            let mut body = vec![0u8; MIB];
            rng.fill_bytes(&mut body);
            Transaction::new(NamespaceId::from(10_000 + i as u64 % NAMESPACES), body)
        })
        .collect()
}

fn chain_config() -> ChainConfig {
    ChainConfig {
        max_block_size: (1u64 << 30).into(),
        ..Default::default()
    }
}

/// `[start, duration]` in ms of closed spans, by span name; start is relative to the enclosing
/// `request_block` span.
static STEPS: Mutex<BTreeMap<&'static str, Vec<[f64; 2]>>> = Mutex::new(BTreeMap::new());

/// Start of the current `request_block` span. Steps on rayon threads have no parent span, and
/// iterations run one at a time, so this is their common origin.
static ITERATION_START: Mutex<Option<Instant>> = Mutex::new(None);

/// Records when each span starts and how long it lives into [`STEPS`].
struct StepTimer;

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for StepTimer {
    fn on_new_span(&self, _: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("new span is registered");
        let now = Instant::now();
        if span.name() == "request_block" {
            *ITERATION_START.lock().expect("iteration start lock") = Some(now);
        }
        span.extensions_mut().insert(now);
    }

    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        let end = Instant::now();
        let span = ctx.span(&id).expect("closing span is registered");
        let start = *span
            .extensions()
            .get::<Instant>()
            .expect("start recorded in on_new_span");
        let origin = ITERATION_START
            .lock()
            .expect("iteration start lock")
            .unwrap_or(start);
        STEPS
            .lock()
            .expect("steps lock")
            .entry(span.name())
            .or_default()
            .push([ms(start - origin), ms(end - start)]);
    }
}

fn install_step_timer() {
    let targets = Targets::new()
        .with_target("block_build", Level::DEBUG)
        .with_target("hotshot_new_protocol::block", Level::DEBUG)
        .with_target("espresso_types", Level::DEBUG);
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(StepTimer.with_filter(targets)),
    )
    .expect("no other subscriber");
}

/// Writes the steps recorded since the last call to `$CRITERION_HOME/<id>/steps.json`.
fn write_steps(id: &str) {
    let steps = mem::take(&mut *STEPS.lock().expect("steps lock"));
    // Benchmarks skipped by the command-line filter record nothing.
    let Some(home) = env::var_os("CRITERION_HOME").filter(|_| !steps.is_empty()) else {
        return;
    };
    let path = PathBuf::from(home).join(id).join("steps.json");
    fs::create_dir_all(path.parent().expect("steps path has a parent")).expect("create steps dir");
    fs::write(&path, serde_json::to_vec(&steps).expect("serialize steps"))
        .expect("write steps.json");
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e3
}

criterion::criterion_group!(benches, bench_block_build);
criterion::criterion_main!(benches);
