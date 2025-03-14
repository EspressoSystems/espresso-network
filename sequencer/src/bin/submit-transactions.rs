//! Utility program to submit random transactions to an Espresso Sequencer.

#[cfg(feature = "benchmarking")]
use std::fs::OpenOptions;
#[cfg(feature = "benchmarking")]
use std::num::NonZeroUsize;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use clap::Parser;
use committable::{Commitment, Committable};
#[cfg(feature = "benchmarking")]
use csv::Writer;
use espresso_types::{parse_duration, parse_size, SeqTypes, Transaction};
use futures::{
    channel::mpsc::{self, Sender},
    sink::SinkExt,
    stream::StreamExt,
};
use hotshot_query_service::{availability::BlockQueryData, types::HeightIndexed, Error};
use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaChaRng;
use rand_distr::Distribution;
use sequencer::SequencerApiVersion;
use sequencer_utils::logging;
use surf_disco::{Client, Url};
use tide_disco::{error::ServerError, App};
use tokio::{task::spawn, time::sleep};
use vbs::version::StaticVersionType;

/// Submit random transactions to an Espresso Sequencer.
#[derive(Clone, Debug, Parser)]
struct Options {
    /// Minimum size of transaction to submit.
    ///
    /// The size of each transaction will be chosen uniformly between MIN_SIZE and MAX_SIZE.
    #[arg(long, name = "MIN_SIZE", default_value = "1", value_parser = parse_size, env = "ESPRESSO_SUBMIT_TRANSACTIONS_MIN_SIZE")]
    min_size: u64,

    /// Maximum size of transaction to submit.
    ///
    /// The size of each transaction will be chosen uniformly between MIN_SIZE and MAX_SIZE.
    #[arg(long, name = "MAX_SIZE", default_value = "1kb", value_parser = parse_size, env = "ESPRESSO_SUBMIT_TRANSACTIONS_MAX_SIZE")]
    max_size: u64,

    /// Minimum size of batch of transactions to submit.
    ///
    /// Batches will be a random count between MIN_BATCH_SIZE and MAX_BATCH_SIZE, with a falling distribution favoring smaller batches.
    /// This is by selecting a random size S on each iteration I since last batch, and collecting a batch whenever that S <= I.
    #[arg(long, name = "MIN_BATCH_SIZE", default_value = "1", value_parser = parse_size, env = "ESPRESSO_SUBMIT_TRANSACTIONS_MIN_BATCH_SIZE")]
    min_batch_size: u64,

    /// Maximum size of batch of transactions to submit.
    ///
    /// Batches will be a random count between MIN_BATCH_SIZE and MAX_BATCH_SIZE, with a falling distribution favoring smaller batches.
    #[arg(long, name = "MAX_BATCH_SIZE", default_value = "20", value_parser = parse_size, env = "ESPRESSO_SUBMIT_TRANSACTIONS_MAX_BATCH_SIZE")]
    max_batch_size: u64,

    /// Minimum namespace ID to submit to.
    #[arg(
        long,
        default_value = "10000",
        env = "ESPRESSO_SUBMIT_TRANSACTIONS_MIN_NAMESPACE"
    )]
    min_namespace: u32,

    /// Maximum namespace ID to submit to.
    #[arg(
        long,
        default_value = "10010",
        env = "ESPRESSO_SUBMIT_TRANSACTIONS_MAX_NAMESPACE"
    )]
    max_namespace: u32,

    /// Mean delay between submitting transactions.
    ///
    /// The delay after each transaction will be sampled from an exponential distribution with mean
    /// DELAY.
    #[arg(long, name = "DELAY", value_parser = parse_duration, default_value = "30s", env = "ESPRESSO_SUBMIT_TRANSACTIONS_DELAY")]
    delay: Duration,

    /// Maximum number of unprocessed transaction submissions.
    ///
    /// This can be used to apply backpressure so that the tasks submitting transactions do not get
    /// too far ahead of the task processing results.
    #[arg(
        long,
        default_value = "1000",
        env = "ESPRESSO_SUBMIT_TRANSACTIONS_CHANNEL_BOUND"
    )]
    channel_bound: usize,

    /// Seed for reproducible randomness.
    #[arg(long, env = "ESPRESSO_SUBMIT_TRANSACTIONS_SEED")]
    seed: Option<u64>,

    /// Number of parallel tasks to run.
    #[arg(
        short,
        long,
        default_value = "1",
        env = "ESPRESSO_SUBMIT_TRANSACTIONS_JOBS"
    )]
    jobs: usize,

    /// Number of accumulated pending transactions which should trigger a warning.
    #[arg(
        long,
        default_value = "10",
        env = "ESPRESSO_SUBMIT_TRANSACTIONS_PENDING_TRANSACTIONS_WARNING_THRESHOLD"
    )]
    pending_transactions_warning_threshold: usize,

    /// Duration after which we should warn about a pending transaction.
    #[arg(long, value_parser = parse_duration, default_value = "30s", env = "ESPRESSO_SUBMIT_TRANSACTIONS_SLOW_TRANSACTION_WARNING_THRESHOLD")]
    slow_transaction_warning_threshold: Duration,

    /// Enable an HTTP server with a healthcheck endpoint on this port.
    #[arg(short, long, env = "ESPRESSO_SUBMIT_TRANSACTIONS_PORT")]
    port: Option<u16>,

    /// Alternative URL to submit transactions to, if not the query service URL.
    #[arg(long, env = "ESPRESSO_SUBMIT_TRANSACTIONS_SUBMIT_URL")]
    submit_url: Option<Url>,

    /// URL of the query service.
    #[arg(env = "ESPRESSO_SEQUENCER_URL")]
    url: Url,

    /// Relay num_nodes for benchmark results output
    #[cfg(feature = "benchmarking")]
    #[arg(short, long, env = "ESPRESSO_ORCHESTRATOR_NUM_NODES")]
    num_nodes: NonZeroUsize,

    /// The first block that benchmark starts counting in
    #[cfg(feature = "benchmarking")]
    #[arg(short, long, env = "ESPRESSO_BENCH_START_BLOCK")]
    benchmark_start_block: NonZeroUsize,

    /// The final block that benchmark counts in
    #[cfg(feature = "benchmarking")]
    #[arg(short, long, env = "ESPRESSO_BENCH_END_BLOCK")]
    benchmark_end_block: NonZeroUsize,

    #[command(flatten)]
    logging: logging::Config,
}

impl Options {
    fn submit_url(&self) -> Url {
        self.submit_url
            .clone()
            .unwrap_or_else(|| self.url.join("submit").unwrap())
    }
    fn use_public_mempool(&self) -> bool {
        self.submit_url.is_none()
    }
}

#[tokio::main]
async fn main() {
    let opt = Options::parse();
    opt.logging.init();

    tracing::warn!("starting load generator for sequencer {}", opt.url);

    let (sender, mut receiver) = mpsc::channel(opt.channel_bound);

    let seed = opt.seed.unwrap_or_else(random_seed);
    tracing::info!("PRNG seed: {seed}");
    let mut rng = ChaChaRng::seed_from_u64(seed);

    // Subscribe to block stream so we can check that our transactions are getting sequenced.
    let client = Client::<Error, SequencerApiVersion>::new(opt.url.clone());
    let block_height: usize = client.get("status/block-height").send().await.unwrap();
    let mut blocks = client
        .socket(&format!("availability/stream/blocks/{}", block_height - 1))
        .subscribe()
        .await
        .unwrap();
    tracing::info!("listening for blocks starting at {block_height}");

    // Spawn tasks to submit transactions.
    for _ in 0..opt.jobs {
        spawn(submit_transactions(
            opt.clone(),
            sender.clone(),
            ChaChaRng::from_rng(&mut rng).unwrap(),
            SequencerApiVersion::instance(),
        ));
    }

    // Start healthcheck endpoint once tasks are running.
    if let Some(port) = opt.port {
        spawn(server(port, SequencerApiVersion::instance()));
    }

    // Keep track of the results.
    let mut pending = HashMap::new();
    let mut total_latency = Duration::default();
    let mut total_transactions = 0;

    // Keep track of the latency after warm up for benchmarking
    #[cfg(feature = "benchmarking")]
    let mut num_block = 0;
    #[cfg(feature = "benchmarking")]
    let mut benchmark_total_latency = Duration::default();
    #[cfg(feature = "benchmarking")]
    let mut benchmark_minimum_latency = Duration::default();
    #[cfg(feature = "benchmarking")]
    let mut benchmark_maximum_latency = Duration::default();
    #[cfg(feature = "benchmarking")]
    let mut benchmark_total_transactions = 0;
    #[cfg(feature = "benchmarking")]
    let mut benchmark_finish = false;
    #[cfg(feature = "benchmarking")]
    let mut total_throughput = 0;
    #[cfg(feature = "benchmarking")]
    let mut start: Instant = Instant::now(); // will be re-assign once has_started turned to true
    #[cfg(feature = "benchmarking")]
    let mut has_started: bool = false;

    while let Some(block) = blocks.next().await {
        let block: BlockQueryData<SeqTypes> = match block {
            Ok(block) => block,
            Err(err) => {
                tracing::warn!("error getting block: {err}");
                continue;
            },
        };
        let received_at = Instant::now();
        tracing::debug!("got block {}", block.height());
        #[cfg(feature = "benchmarking")]
        {
            num_block += 1;
            if !has_started && (num_block as usize) >= opt.benchmark_start_block.into() {
                has_started = true;
                start = Instant::now();
            }
        }

        // Get all transactions which were submitted before this block.
        while let Ok(Some(tx)) = receiver.try_next() {
            pending.insert(tx.hash, tx.submitted_at);
        }

        // Clear pending transactions from the block.
        for (_, tx) in block.enumerate() {
            if let Some(submitted_at) = pending.remove(&tx.commit()) {
                let latency = received_at - submitted_at;
                tracing::info!(
                    "got transaction {} in block {}, latency {latency:?}",
                    tx.commit(),
                    block.height()
                );
                total_latency += latency;
                total_transactions += 1;
                tracing::info!("average latency: {:?}", total_latency / total_transactions);
                #[cfg(feature = "benchmarking")]
                {
                    if has_started && !benchmark_finish {
                        benchmark_minimum_latency = if total_transactions == 0 {
                            latency
                        } else {
                            std::cmp::min(benchmark_minimum_latency, latency)
                        };
                        benchmark_maximum_latency = if total_transactions == 0 {
                            latency
                        } else {
                            std::cmp::max(benchmark_maximum_latency, latency)
                        };

                        benchmark_total_latency += latency;
                        benchmark_total_transactions += 1;
                        // Transaction = NamespaceId(u64) + payload(Vec<u8>)
                        let payload_length = tx.into_payload().len();
                        let tx_sz = payload_length * std::mem::size_of::<u8>() // size of payload
                        + std::mem::size_of::<u64>() // size of the namespace
                        + std::mem::size_of::<Transaction>(); // size of the struct wrapper
                        total_throughput += tx_sz;
                    }
                }
            }
        }

        #[cfg(feature = "benchmarking")]
        if !benchmark_finish && (num_block as usize) >= opt.benchmark_end_block.into() {
            let block_range = format!("{}~{}", opt.benchmark_start_block, opt.benchmark_end_block,);
            let transaction_size_range_in_bytes = format!("{}~{}", opt.min_size, opt.max_size,);
            let transactions_per_batch_range = format!(
                "{}~{}",
                (opt.jobs as u64 * opt.min_batch_size),
                (opt.jobs as u64 * opt.max_batch_size),
            );
            let benchmark_average_latency = benchmark_total_latency / benchmark_total_transactions;
            let avg_transaction_size = total_throughput as u32 / benchmark_total_transactions;
            let total_time_elapsed_in_sec = start.elapsed(); // in seconds
            let avg_throughput_bytes_per_sec = (total_throughput as u64)
                / std::cmp::max(total_time_elapsed_in_sec.as_secs(), 1u64);
            // Open the CSV file in append mode
            let results_csv_file = OpenOptions::new()
                .create(true)
                .append(true) // Open in append mode
                .open("scripts/benchmarks_results/results.csv")
                .unwrap();
            // Open a file for writing
            let mut wtr = Writer::from_writer(results_csv_file);
            let mut pub_or_priv_pool = "private";
            if opt.use_public_mempool() {
                pub_or_priv_pool = "public";
            }
            let _ = wtr.write_record([
                "total_nodes",
                "da_committee_size",
                "block_range",
                "transaction_size_range_in_bytes",
                "transaction_per_batch_range",
                "pub_or_priv_pool",
                "avg_latency_in_sec",
                "minimum_latency_in_sec",
                "maximum_latency_in_sec",
                "avg_throughput_bytes_per_sec",
                "total_transactions",
                "avg_transaction_size_in_bytes",
                "total_time_elapsed_in_sec",
            ]);
            let _ = wtr.write_record(&[
                opt.num_nodes.to_string(),
                opt.num_nodes.to_string(),
                block_range,
                transaction_size_range_in_bytes,
                transactions_per_batch_range,
                pub_or_priv_pool.to_string(),
                benchmark_average_latency.as_secs().to_string(),
                benchmark_minimum_latency.as_secs().to_string(),
                benchmark_maximum_latency.as_secs().to_string(),
                avg_throughput_bytes_per_sec.to_string(),
                benchmark_total_transactions.to_string(),
                avg_transaction_size.to_string(),
                total_time_elapsed_in_sec.as_secs().to_string(),
            ]);
            let _ = wtr.flush();
            println!(
                "Latency results successfully saved in scripts/benchmarks_results/results.csv"
            );
            benchmark_finish = true;
        }

        // If a lot of transactions are pending, it might indicate the sequencer is struggling to
        // finalize them. We should warn about this.
        if pending.len() >= opt.pending_transactions_warning_threshold {
            tracing::warn!(
                "transactions are not being finalized or being finalized too slowly, {} pending",
                pending.len()
            );
        } else {
            tracing::debug!("{} transactions still pending", pending.len());

            // Even if we are not accumulating transactions, it is still possible that some
            // individual transactions are not being finalized. Warn about any transaction which has
            // been pending for too long.
            for (tx, submitted_at) in &pending {
                let duration = received_at - *submitted_at;
                if duration >= opt.slow_transaction_warning_threshold {
                    tracing::warn!("transaction {tx} has been pending for {duration:?}");
                }
            }
        }
    }
    tracing::info!(
        "block stream ended with {} transactions still pending",
        pending.len()
    );
}

struct SubmittedTransaction {
    hash: Commitment<Transaction>,
    submitted_at: Instant,
}

async fn submit_transactions<ApiVer: StaticVersionType>(
    opt: Options,
    mut sender: Sender<SubmittedTransaction>,
    mut rng: ChaChaRng,
    _: ApiVer,
) {
    let url = opt.submit_url();
    tracing::info!(%url, "starting load generator task");
    let client = Client::<Error, ApiVer>::new(url);

    // Create an exponential distribution for sampling delay times. The distribution should have
    // mean `opt.delay`, or parameter `\lambda = 1 / opt.delay`.
    let delay_distr = rand_distr::Exp::<f64>::new(1f64 / opt.delay.as_millis() as f64).unwrap();

    let mut txns = Vec::new();
    let mut hashes = Vec::new();
    loop {
        let tx = random_transaction(&opt, &mut rng);
        let hash = tx.commit();
        tracing::info!(
            "Adding transaction {hash} for namespace {} of size {}",
            tx.namespace(),
            tx.payload().len()
        );
        txns.push(tx);
        hashes.push(hash);

        let randomized_batch_size = if opt.use_public_mempool() {
            1
        } else {
            rng.gen_range(opt.min_batch_size..=opt.max_batch_size)
        };
        let txns_batch_count = txns.len() as u64;
        if randomized_batch_size <= txns_batch_count {
            if let Err(err) = if txns_batch_count == 1 {
                // occasionally test the 'submit' endpoint, just for coverage
                client
                    .post::<()>("submit")
                    .body_binary(&txns[0])
                    .unwrap()
                    .send()
                    .await
            } else {
                client
                    .post::<()>("batch")
                    .body_binary(&txns)
                    .unwrap()
                    .send()
                    .await
            } {
                tracing::error!(
                    ?err,
                    "failed to submit batch of {txns_batch_count} transactions"
                );
            } else {
                tracing::info!("submitted batch of {txns_batch_count} transactions");
                let submitted_at = Instant::now();
                for hash in hashes.iter() {
                    sender
                        .send(SubmittedTransaction {
                            hash: *hash,
                            submitted_at,
                        })
                        .await
                        .ok();
                }
            }
            txns.clear();
            hashes.clear();

            let delay = Duration::from_millis(delay_distr.sample(&mut rng) as u64);
            tracing::info!("sleeping for {delay:?}");
            sleep(delay).await;
        }
    }
}

async fn server<ApiVer: StaticVersionType + 'static>(port: u16, bind_version: ApiVer) {
    if let Err(err) = App::<(), ServerError>::with_state(())
        .serve(format!("0.0.0.0:{port}"), bind_version)
        .await
    {
        tracing::error!("web server exited: {err}");
    }
}

fn random_transaction(opt: &Options, rng: &mut ChaChaRng) -> Transaction {
    // TODO instead use NamespaceId::random, but that does not allow us to
    // enforce `gen_range(opt.min_namespace..=opt.max_namespace)`
    let namespace = rng.gen_range(opt.min_namespace..=opt.max_namespace);

    let len = rng.gen_range(opt.min_size..=opt.max_size);
    let mut payload = vec![0; len as usize];
    rng.fill_bytes(&mut payload);

    Transaction::new(namespace.into(), payload)
}

fn random_seed() -> u64 {
    ChaChaRng::from_entropy().next_u64()
}
