//! The leader's block build (`request_block` in crates/hotshot/new-protocol/src/block.rs) and its
//! VID work (`leader_vid`), timed step by step.
//!
//! `request_block` runs `from_transactions` on the cached transactions, then
//! `hotshot_new_protocol::block::block_commitments`, the function the leader calls. It is the
//! proposal's critical path.
//!
//! `leader_vid` runs `request_block`, then the disperser's work for the built block:
//! `hotshot_new_protocol::vid::disperse_payload` with a `send` that serializes each fragment as
//! the network does. Consensus asks for the dispersal as soon as the block is built, so this is
//! when peers can start receiving their shares, and its CPU is the leader's VID cost per block.
//!
//! A tracing layer records each step's start, completion and CPU time; when `CRITERION_HOME` is
//! set they are written to `steps.json` next to the benchmark's criterion output. Step samples
//! start with criterion's warm-up iterations; scripts/bench-block-build keeps one per measured
//! iteration, which assumes the timer below records every step exactly once per iteration.
//!
//! Like the leader, it passes the pool's hashes of the transactions. Every transaction fits, so
//! the hashes are reused and the `transaction_hashes` step does not run.
//!
//! Inputs mirror the network-bench load generator: random 1 MiB transactions round-robin over
//! 16 namespaces in 30 to 70 MiB blocks, protocol version 0.6 (AvidmGf2), 100 nodes. Real stakes
//! are large, so the nodes have equal stakes of 1 ESP, which `approximate_weights` scales to a VID
//! total weight of 1100.
//!
//! To compare two git refs, run `just bench-block-build BASE HEAD` (scripts/bench-block-build).

use std::{
    collections::BTreeMap,
    env, fs, iter, mem,
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};

use alloy::primitives::U256;
use committable::{Commitment, Committable};
use criterion::{BenchmarkId, Criterion, SamplingMode};
use espresso_types::{ChainConfig, NamespaceId, NsTable, Payload, SeqTypes, Transaction};
use hotshot_new_protocol::{
    block::{BlockCommitments, block_commitments},
    vid::{BuiltDispersal, Dispersal, disperse_payload},
};
use hotshot_types::{
    PeerConfig, ValidatorConfig,
    consensus::PayloadWithMetadata,
    data::{EpochNumber, VidCommitment, VidCommitment2, ViewNumber},
    message::UpgradeLock,
};
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
use versions::{NEW_PROTOCOL_VERSION, Upgrade};

const MIB: usize = 1 << 20;
const NAMESPACES: u64 = 16;
const NODES: usize = 100;
const BLOCK_MIB: [usize; 5] = [30, 40, 50, 60, 70];

fn bench_block_build(c: &mut Criterion) {
    install_step_timer();
    let threads = rayon::current_num_threads();
    let committee = Committee::new();
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
            b.iter_with_large_drop(|| request_block(&txs, &pooled_hashes, &committee))
        });
        write_steps(&format!("block_build/request_block/{label}"));
        group.bench_function(BenchmarkId::new("leader_vid", &label), |b| {
            b.iter_with_large_drop(|| leader_vid(&txs, &pooled_hashes, &committee))
        });
        write_steps(&format!("block_build/leader_vid/{label}"));
    }
    group.finish();
}

/// The leader's build task after the transactions and their pool hashes are taken; outputs are
/// returned so their drop is untimed.
fn request_block(
    txs: &[Transaction],
    pooled_hashes: &[Commitment<Transaction>],
    committee: &Committee,
) -> (PayloadWithMetadata<SeqTypes>, BlockCommitments<SeqTypes>) {
    let _span = tracing::debug_span!("request_block").entered();
    let (payload, metadata): (Payload, NsTable) =
        Payload::from_transactions_sync(txs, chain_config()).expect("payload construction");
    let payload = PayloadWithMetadata { payload, metadata };
    let commitments = block_commitments(
        &payload,
        pooled_hashes,
        &committee.stake_table,
        NEW_PROTOCOL_VERSION,
    )
    .expect("block commitments");
    (payload, commitments)
}

/// [`request_block`], then the disperser's work for the block. Outputs are returned so their drop
/// is untimed.
fn leader_vid(
    txs: &[Transaction],
    pooled_hashes: &[Commitment<Transaction>],
    committee: &Committee,
) -> (
    PayloadWithMetadata<SeqTypes>,
    Vec<Commitment<Transaction>>,
    Vec<Vec<u8>>,
) {
    let _span = tracing::debug_span!("leader_vid").entered();
    let (payload, commitments) = request_block(txs, pooled_hashes, committee);
    let BlockCommitments {
        payload_commitment,
        hashes,
        dispersal,
        ..
    } = commitments;
    let VidCommitment::V2(payload_commitment) = payload_commitment else {
        panic!("protocol 0.6 commits with AvidmGf2");
    };
    let fragments = tracing::debug_span!("vid_disperse")
        .in_scope(|| disperse(dispersal, payload_commitment, committee));
    (payload, hashes, fragments)
}

/// The disperser's work for a built block, until every fragment is serialized as the network
/// does. The serialized fragments are kept, as the network takes them.
fn disperse(
    dispersal: BuiltDispersal<SeqTypes>,
    payload_commitment: VidCommitment2,
    committee: &Committee,
) -> Vec<Vec<u8>> {
    let fragments = Mutex::new(Vec::new());
    let upgrade_lock = &committee.upgrade_lock;
    disperse_payload(
        Dispersal::Built(dispersal),
        ViewNumber::new(1),
        EpochNumber::genesis(),
        payload_commitment,
        &committee.leader.public_key,
        &committee.leader.private_key,
        |_, message| {
            let bytes = tracing::debug_span!("serialize")
                .in_scope(|| upgrade_lock.serialize(message))
                .expect("serialize fragment");
            fragments.lock().expect("fragments lock").push(bytes);
            Ok(())
        },
    )
    .expect("dispersal");
    fragments.into_inner().expect("fragments lock")
}

/// The block's epoch committee as the leader sees it, the leader being its first node.
struct Committee {
    stake_table: Vec<PeerConfig<SeqTypes>>,
    leader: ValidatorConfig<SeqTypes>,
    upgrade_lock: UpgradeLock<SeqTypes>,
}

impl Committee {
    fn new() -> Self {
        let node = |i| {
            ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
                [0; 32],
                i,
                U256::from(10u64.pow(18)),
                false,
            )
        };
        let leader = node(0);
        let stake_table: Vec<_> = iter::once(leader.public_config())
            .chain((1..NODES as u64).map(|i| node(i).public_config()))
            .collect();
        Self {
            stake_table,
            leader,
            upgrade_lock: UpgradeLock::new(Upgrade::trivial(NEW_PROTOCOL_VERSION)),
        }
    }
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
/// relative to the iteration's root span, `request_block` or `leader_vid`. The CPU kind depends
/// on the step:
/// - thread CPU: `from_transactions`, `encode`, `builder_commitment`, `sign`
/// - process CPU, including rayon workers spinning for work: `leader_vid`, `request_block`,
///   `block_commitments`, `vid_disperse`
/// - sum of the leaves' thread CPU: `transaction_hashes` (its `tx_hash` leaves), absent while the
///   pool's hashes are reused; `fragments` and `serialize`
/// - residual upper bound, process CPU of `block_commitments` minus its other steps:
///   `vid_commitment`, which disperses every namespace and folds the commitment from them
static STEPS: Mutex<BTreeMap<&'static str, Vec<[f64; 3]>>> = Mutex::new(BTreeMap::new());

/// The running iteration. Steps on rayon threads have no parent span, and iterations run one at
/// a time, so this is their common state.
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
    /// The span whose exit ends the iteration.
    root: &'static str,
    origin: Instant,
    steps: Vec<Step>,
    /// Leaf spans entered many times per iteration, by name: their first start, last end and
    /// summed thread CPU.
    leaves: BTreeMap<&'static str, Step>,
}

/// Times each step by its own work. A rayon worker that waits inside a step span steals unrelated
/// jobs, so a step's span can last as long as the VID work. Leaf spans run once per transaction,
/// bucket or fragment, make no rayon calls (`tx_hash`, `fragments`, `serialize`) and cost their
/// summed thread CPU. The VID step (`vid_commitment`) costs what the process spent in
/// `block_commitments` beyond its other steps.
struct StepTimer;

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for StepTimer {
    fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
        let at = Instant::now();
        let span = ctx.span(id).expect("entered span is registered");
        let name = span.name();
        let entered = Entered {
            at,
            thread_cpu: cpu_ms(ClockId::ThreadCPUTime),
            process_cpu: wide(name).then(|| cpu_ms(ClockId::ProcessCPUTime)),
        };
        if matches!(name, "request_block" | "leader_vid") {
            ITERATION
                .lock()
                .expect("iteration lock")
                .get_or_insert_with(|| Iteration {
                    root: name,
                    origin: at,
                    steps: Vec::new(),
                    leaves: BTreeMap::new(),
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
        let iteration = guard.as_mut().expect("spans run inside an iteration");
        let rel = |t: Instant| ms(t - iteration.origin);
        let step = Step {
            name,
            start: rel(start),
            end: rel(end),
            cpu: match start_process {
                Some(process) => cpu_ms(ClockId::ProcessCPUTime) - process,
                None => thread_cpu - start_cpu,
            },
        };
        if matches!(name, "tx_hash" | "fragments" | "serialize") {
            iteration
                .leaves
                .entry(name)
                .and_modify(|leaf| {
                    leaf.start = leaf.start.min(step.start);
                    leaf.end = leaf.end.max(step.end);
                    leaf.cpu += step.cpu;
                })
                .or_insert(step);
            return;
        }
        iteration.steps.push(step);
        if name == iteration.root {
            record(guard.take().expect("iteration present"));
        }
    }
}

/// Spans entered outside the rayon pool, whose CPU is read from the process clock.
fn wide(name: &str) -> bool {
    matches!(
        name,
        "leader_vid" | "request_block" | "block_commitments" | "vid_disperse"
    )
}

/// Pushes one row per step of a finished iteration into [`STEPS`].
fn record(iteration: Iteration) {
    let cpu_of = |name: &str| {
        iteration
            .steps
            .iter()
            .chain(iteration.leaves.values())
            .find(|s| s.name == name)
            .map(|s| s.cpu)
    };
    let tx_hashes = iteration.leaves.get("tx_hash");
    // Absent from `NO_BUILDER_COMMITMENT_VERSION` onwards.
    let builder_cpu = cpu_of("builder_commitment").unwrap_or(0.0);
    let vid_cpu = cpu_of("block_commitments").expect("block_commitments ran")
        - tx_hashes.map_or(0.0, |leaves| leaves.cpu)
        - builder_cpu
        - cpu_of("encode").expect("encode ran");
    let mut steps = STEPS.lock().expect("steps lock");
    let leaves = iteration
        .leaves
        .values()
        .filter(|leaf| leaf.name != "tx_hash");
    for step in iteration.steps.iter().chain(leaves) {
        let (end, cpu) = match step.name {
            "transaction_hashes" => {
                let leaves = tx_hashes.expect("a tx_hash span ran");
                (leaves.end, leaves.cpu)
            },
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
        .with_target("hotshot_new_protocol::vid", Level::DEBUG)
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
