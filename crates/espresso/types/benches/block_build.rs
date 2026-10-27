//! The leader's block build (`request_block` in crates/hotshot/new-protocol/src/block.rs), timed
//! step by step.
//!
//! `request_block` runs `from_transactions` on the cached transactions, then
//! `hotshot_new_protocol::block::block_commitments`, the function the leader calls. A tracing
//! layer records each step's start, completion and CPU time; when `CRITERION_HOME` is set they are
//! written to `steps.json` next to the benchmark's criterion output. Step samples start with
//! criterion's warm-up iterations; scripts/bench-block-build keeps one per measured iteration,
//! which assumes every step span except `tx_hash` is entered and exited exactly once per
//! iteration, by one thread.
//!
//! Like the leader, it passes the pool's hashes of the transactions. Every transaction fits, so
//! the hashes are reused and the `transaction_hashes` step does not run.
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

use committable::{Commitment, Committable};
use criterion::{BenchmarkId, Criterion, SamplingMode};
use espresso_types::{ChainConfig, NamespaceId, NsTable, Payload, SeqTypes, Transaction};
use hotshot_new_protocol::block::{BlockCommitments, block_commitments};
use hotshot_types::{consensus::PayloadWithMetadata, data::vid_disperse::VID_TARGET_TOTAL_STAKE};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rustix::time::{ClockId, clock_gettime};
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
        let pooled_hashes: Vec<_> = txs.iter().map(Committable::commit).collect();
        group.bench_function(BenchmarkId::new("request_block", &label), |b| {
            b.iter_with_large_drop(|| request_block(&txs, &pooled_hashes))
        });
        write_steps(&format!("block_build/request_block/{label}"));
    }
    group.finish();
}

/// The leader's build task after the transactions and their pool hashes are taken; outputs are
/// returned so their drop is untimed.
fn request_block(
    txs: &[Transaction],
    pooled_hashes: &[Commitment<Transaction>],
) -> (PayloadWithMetadata<SeqTypes>, BlockCommitments<SeqTypes>) {
    let _span = tracing::debug_span!("request_block").entered();
    let (payload, metadata): (Payload, NsTable) =
        Payload::from_transactions_sync(txs, chain_config()).expect("payload construction");
    let payload = PayloadWithMetadata { payload, metadata };
    let commitments = block_commitments(&payload, pooled_hashes, VID_WEIGHT, NEW_PROTOCOL_VERSION);
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

/// `[start, completion - start, cpu]` in ms per step and iteration, by step name; start is
/// relative to the enclosing `request_block` span. The CPU kind depends on the step:
/// - thread CPU: `from_transactions`, `encode`, `builder_commitment`
/// - process CPU, including rayon workers spinning for work: `request_block`, `block_commitments`
/// - sum of the `tx_hash` leaves' thread CPU: `transaction_hashes`, absent while the pool's hashes
///   are reused
/// - residual upper bound, process CPU of `block_commitments` minus the steps above:
///   `vid_commitment`
static STEPS: Mutex<BTreeMap<&'static str, Vec<[f64; 3]>>> = Mutex::new(BTreeMap::new());

/// The running `request_block` iteration. Steps on rayon threads have no parent span, and
/// iterations run one at a time, so this is their common state.
static ITERATION: Mutex<Option<Iteration>> = Mutex::new(None);

/// Wall and CPU readings at a span's most recent enter.
struct Entered {
    at: Instant,
    thread_cpu: f64,
    process_cpu: Option<f64>,
}

/// A step span that has exited, in ms relative to the iteration start.
struct Step {
    name: &'static str,
    start: f64,
    end: f64,
    cpu: f64,
}

struct Iteration {
    origin: Instant,
    steps: Vec<Step>,
    /// Sum of `tx_hash` thread CPU.
    leaf_cpu: f64,
    /// Latest `tx_hash` exit, when `transaction_hashes` completes.
    leaves_end: Option<f64>,
}

/// Times each step by its own work. A rayon worker that waits inside a step span steals unrelated
/// jobs, so a step's span can last as long as the VID work. The `tx_hash` spans make no rayon
/// calls: the hashing step completes at the last leaf exit and costs the leaves' summed thread
/// CPU. The VID step's CPU is what the process spent in `block_commitments` beyond the other steps.
struct StepTimer;

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for StepTimer {
    fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
        let at = Instant::now();
        let span = ctx.span(id).expect("entered span is registered");
        let entered = Entered {
            at,
            thread_cpu: cpu_ms(ClockId::ThreadCPUTime),
            process_cpu: wide(span.name()).then(|| cpu_ms(ClockId::ProcessCPUTime)),
        };
        if span.name() == "request_block" {
            *ITERATION.lock().expect("iteration lock") = Some(Iteration {
                origin: at,
                steps: Vec::new(),
                leaf_cpu: 0.0,
                leaves_end: None,
            });
        }
        span.extensions_mut().replace(entered);
    }

    fn on_exit(&self, id: &span::Id, ctx: Context<'_, S>) {
        let end = Instant::now();
        let thread_cpu = cpu_ms(ClockId::ThreadCPUTime);
        let span = ctx.span(id).expect("exited span is registered");
        let name = span.name();
        let (start, start_cpu, start_process) = {
            let ext = span.extensions();
            let entered = ext.get::<Entered>().expect("recorded in on_enter");
            (entered.at, entered.thread_cpu, entered.process_cpu)
        };
        let mut guard = ITERATION.lock().expect("iteration lock");
        let iteration = guard.as_mut().expect("spans run inside request_block");
        let rel = |t: Instant| ms(t - iteration.origin);
        if name == "tx_hash" {
            iteration.leaf_cpu += thread_cpu - start_cpu;
            iteration.leaves_end = Some(iteration.leaves_end.map_or(rel(end), |e| e.max(rel(end))));
            return;
        }
        let cpu = match start_process {
            Some(process) => cpu_ms(ClockId::ProcessCPUTime) - process,
            None => thread_cpu - start_cpu,
        };
        iteration.steps.push(Step {
            name,
            start: rel(start),
            end: rel(end),
            cpu,
        });
        if name == "request_block" {
            record(guard.take().expect("iteration present"));
        }
    }
}

/// Spans entered outside the rayon pool, whose CPU is read from the process clock.
fn wide(name: &str) -> bool {
    matches!(name, "request_block" | "block_commitments")
}

/// Pushes one row per step of a finished iteration into [`STEPS`].
fn record(iteration: Iteration) {
    let cpu_of = |name: &str| {
        iteration
            .steps
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.cpu)
    };
    // Absent from `NO_BUILDER_COMMITMENT_VERSION` onwards.
    let builder_cpu = cpu_of("builder_commitment").unwrap_or(0.0);
    let vid_cpu = cpu_of("block_commitments").expect("block_commitments ran")
        - iteration.leaf_cpu
        - builder_cpu
        - cpu_of("encode").expect("encode ran");
    let mut steps = STEPS.lock().expect("steps lock");
    for step in &iteration.steps {
        let (end, cpu) = match step.name {
            "transaction_hashes" => (
                iteration.leaves_end.expect("a tx_hash span ran"),
                iteration.leaf_cpu,
            ),
            "vid_commitment" => (step.end, vid_cpu),
            _ => (step.end, step.cpu),
        };
        steps
            .entry(step.name)
            .or_default()
            .push([step.start, end - step.start, cpu]);
    }
}

fn cpu_ms(clock: ClockId) -> f64 {
    let ts = clock_gettime(clock);
    ts.tv_sec as f64 * 1e3 + ts.tv_nsec as f64 / 1e6
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
