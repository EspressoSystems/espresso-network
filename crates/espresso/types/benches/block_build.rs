//! Leader block-build stages from `request_block` (crates/hotshot/new-protocol/src/block.rs).
//!
//! Inputs mirror the network-bench load generator: random 1 MB transactions round-robin over
//! 16 namespaces in 30 to 70 MB blocks, protocol version 0.6 (AvidmGf2). VID total weight is
//! the node count (stake 1 per node) and the `approximate_weights` result for large equal
//! stakes (1000 + nodes).
//!
//! `request_block` runs the leader's build task: `from_transactions` on a copy of the cached
//! transactions, then `hotshot_new_protocol::block::block_commitments`, the function the leader
//! calls. Their tracing spans are timed per step; after each `request_block` benchmark the step
//! medians are printed and, when `CRITERION_HOME` is set, written to `steps.json` next to its
//! criterion output. Steps in parallel overlap, so they do not add up to the total.
//! `tx_clone` times a full copy of the transactions for comparison.
//!
//! `RAYON_NUM_THREADS` is read once per process and recorded in the benchmark id.
//!
//! To compare two git refs on the critical path, run `just bench-block-build BASE HEAD`
//! (scripts/bench-block-build), which isolates each benchmark in its own process.

use std::{
    collections::BTreeMap,
    env, fs,
    hint::black_box,
    mem,
    path::PathBuf,
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

use committable::{Commitment, Committable};
use criterion::{BenchmarkGroup, BenchmarkId, Criterion, SamplingMode, measurement::WallTime};
use espresso_types::{ChainConfig, NamespaceId, NsTable, Payload, SeqTypes, Transaction};
use hotshot_new_protocol::block::{BlockCommitments, block_commitments};
use hotshot_types::{
    consensus::PayloadWithMetadata,
    data::{VidCommitment, vid_commitment},
    traits::{BlockPayload, EncodeBytes},
    utils::BuilderCommitment,
};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rayon::prelude::*;
use tracing::{Level, Subscriber, span};
use tracing_subscriber::{
    Layer,
    filter::Targets,
    layer::{Context, SubscriberExt},
    registry::LookupSpan,
};
use versions::NEW_PROTOCOL_VERSION;

const MB: usize = 1_000_000;
const NAMESPACES: u64 = 16;
const NODES: [usize; 2] = [3, 10];
/// `approximate_weights` adds 1 per node on top of `VID_TARGET_TOTAL_STAKE`.
const SCALED_WEIGHT_BASE: usize = 1000;
/// (block size in MB, tx size in bytes)
const BLOCKS: [(usize, usize); 5] = [(30, MB), (40, MB), (50, MB), (60, MB), (70, MB)];

struct Case {
    label: String,
    txs: Vec<Transaction>,
}

struct Encoded {
    payload: Payload,
    ns_table: NsTable,
    payload_bytes: Vec<u8>,
    metadata_bytes: Vec<u8>,
}

impl Encoded {
    fn new(txs: &[Transaction]) -> Self {
        let (payload, ns_table) = build(txs);
        let payload_bytes = payload.encode().to_vec();
        let metadata_bytes = ns_table.encode().to_vec();
        Self {
            payload,
            ns_table,
            payload_bytes,
            metadata_bytes,
        }
    }

    fn vid(&self, weight: usize) -> VidCommitment {
        vid_commitment(
            black_box(&self.payload_bytes),
            black_box(&self.metadata_bytes),
            weight,
            NEW_PROTOCOL_VERSION,
        )
    }

    fn sha(&self) -> BuilderCommitment {
        black_box(&self.payload).builder_commitment(black_box(&self.ns_table))
    }
}

fn transactions(block_mb: usize, tx_size: usize, rng: &mut ChaCha20Rng) -> Vec<Transaction> {
    (0..block_mb * MB / tx_size)
        .map(|i| {
            let mut body = vec![0u8; tx_size];
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

/// `from_transactions` takes owned transactions, so the leader clones its cached ones
/// (`transactions_for` in block.rs); the clone is part of what this times.
fn build(txs: &[Transaction]) -> (Payload, NsTable) {
    Payload::from_transactions_sync(txs.to_vec(), chain_config()).expect("payload construction")
}

/// The leader's build task after the transactions are taken; outputs are returned so their drop
/// is untimed.
fn request_block(
    txs: &[Transaction],
    weight: usize,
) -> (PayloadWithMetadata<SeqTypes>, BlockCommitments<SeqTypes>) {
    let _span = tracing::debug_span!("request_block").entered();
    let (payload, metadata) = build(txs);
    let payload = PayloadWithMetadata { payload, metadata };
    let commitments = block_commitments(&payload, weight, NEW_PROTOCOL_VERSION);
    (payload, commitments)
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
            .push([ms(start - origin), ms(start.elapsed())]);
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

/// Prints each step's median start and duration (`@start+duration` ms) recorded since the last
/// call, and writes all samples to
/// `$CRITERION_HOME/<id>/steps.json` when `CRITERION_HOME` is set.
fn report_steps(id: &str) {
    let steps = mem::take(&mut *STEPS.lock().expect("steps lock"));
    // Benchmarks skipped by the command-line filter record nothing.
    if steps.is_empty() {
        return;
    }
    let medians: Vec<String> = steps
        .iter()
        .map(|(name, runs)| {
            let column = |i: usize| median(&runs.iter().map(|r| r[i]).collect::<Vec<_>>());
            format!("{name}=@{:.2}+{:.2}", column(0), column(1))
        })
        .collect();
    println!("steps {id}: {} ms", medians.join(" "));
    if let Some(home) = env::var_os("CRITERION_HOME") {
        let path = PathBuf::from(home).join(id).join("steps.json");
        fs::create_dir_all(path.parent().expect("steps path has a parent"))
            .expect("create steps dir");
        fs::write(&path, serde_json::to_vec(&steps).expect("serialize steps"))
            .expect("write steps.json");
    }
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e3
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[sorted.len() / 2]
}

fn bench_tx_stages(group: &mut BenchmarkGroup<WallTime>, case: &Case) {
    let label = format!("{}_n-", case.label);
    let txs = &case.txs;
    group.bench_function(BenchmarkId::new("tx_commit_serial", &label), |b| {
        b.iter(|| {
            black_box(txs)
                .iter()
                .map(Committable::commit)
                .collect::<Vec<Commitment<_>>>()
        })
    });
    group.bench_function(BenchmarkId::new("tx_commit_par", &label), |b| {
        b.iter(|| {
            black_box(txs)
                .par_iter()
                .map(Committable::commit)
                .collect::<Vec<Commitment<_>>>()
        })
    });
    group.bench_function(BenchmarkId::new("tx_clone", &label), |b| {
        b.iter_with_large_drop(|| black_box(txs).clone())
    });
    group.bench_function(BenchmarkId::new("from_transactions", &label), |b| {
        b.iter_with_large_drop(|| build(black_box(txs)))
    });

    let enc = Encoded::new(txs);
    group.bench_function(BenchmarkId::new("encode", &label), |b| {
        b.iter_with_large_drop(|| {
            (
                black_box(&enc.payload).encode(),
                black_box(&enc.ns_table).encode(),
            )
        })
    });
    group.bench_function(BenchmarkId::new("from_bytes", &label), |b| {
        b.iter_with_large_drop(|| {
            Payload::from_bytes(black_box(&enc.payload_bytes), black_box(&enc.ns_table))
        })
    });
    let serialized = bincode::serialize(&enc.payload).expect("bincode serialize");
    group.bench_function(BenchmarkId::new("decode", &label), |b| {
        b.iter_with_large_drop(|| {
            bincode::deserialize::<Payload>(black_box(&serialized)).expect("bincode deserialize")
        })
    });
    group.bench_function(BenchmarkId::new("builder_commitment", &label), |b| {
        b.iter(|| enc.sha())
    });
}

fn bench_commit_stages(group: &mut BenchmarkGroup<WallTime>, case: &Case, nodes: usize) {
    let label = format!("{}_n{nodes}", case.label);
    let enc = Encoded::new(&case.txs);

    group.bench_function(BenchmarkId::new("vid_commitment", &label), |b| {
        b.iter(|| enc.vid(nodes))
    });
    group.bench_function(BenchmarkId::new("commitments_join", &label), |b| {
        b.iter(|| rayon::join(|| enc.vid(nodes), || enc.sha()))
    });
    group.bench_function(BenchmarkId::new("commitments_seq", &label), |b| {
        b.iter(|| (enc.vid(nodes), enc.sha()))
    });
    group.bench_function(BenchmarkId::new("commitments_thread", &label), |b| {
        b.iter(|| {
            thread::scope(|s| {
                let sha = s.spawn(|| enc.sha());
                (enc.vid(nodes), sha.join().expect("sha thread"))
            })
        })
    });
    bench_request_block(group, case, &label, nodes);

    let weight = SCALED_WEIGHT_BASE + nodes;
    let label = format!("{}_n{nodes}w{weight}", case.label);
    group.bench_function(BenchmarkId::new("vid_commitment", &label), |b| {
        b.iter(|| enc.vid(weight))
    });
    bench_request_block(group, case, &label, weight);
}

fn bench_request_block(
    group: &mut BenchmarkGroup<WallTime>,
    case: &Case,
    label: &str,
    weight: usize,
) {
    STEPS.lock().expect("steps lock").clear();
    group.bench_function(BenchmarkId::new("request_block", label), |b| {
        b.iter_with_large_drop(|| request_block(&case.txs, weight))
    });
    report_steps(&format!("block_build/request_block/{label}"));
}

fn bench_block_build(c: &mut Criterion) {
    install_step_timer();
    let threads = rayon::current_num_threads();
    let mut rng = ChaCha20Rng::seed_from_u64(42);
    let mut group = c.benchmark_group("block_build");
    group
        .sampling_mode(SamplingMode::Flat)
        .sample_size(10)
        .warm_up_time(Duration::from_millis(200))
        .measurement_time(Duration::from_secs(1));

    for (block_mb, tx_size) in BLOCKS {
        let case = Case {
            label: format!("{block_mb}MB_{}KB_t{threads}", tx_size / 1000),
            txs: transactions(block_mb, tx_size, &mut rng),
        };
        bench_tx_stages(&mut group, &case);
        for nodes in NODES {
            bench_commit_stages(&mut group, &case, nodes);
        }
    }
    group.finish();
}

criterion::criterion_group!(benches, bench_block_build);
criterion::criterion_main!(benches);
