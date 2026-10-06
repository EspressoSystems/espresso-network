//! Transaction hashing on the leader's path: `Transaction::commit` (Keccak, the external hash)
//! against `Transaction::digest` (BLAKE3, what consensus keys transactions by), plus the raw
//! hashes they are built from.
//!
//! Runs every case once per rayon pool size, so one run covers a machine's whole core range:
//!
//! ```text
//! cargo bench -p espresso-types --bench tx_hash
//! TX_HASH_THREADS=1,4,8,16 TX_HASH_TXS=50 TX_HASH_TX_BYTES=1000000 cargo bench -p espresso-types --bench tx_hash
//! ```
//!
//! Columns are the median wall time in ms. `block` cases hash `TX_HASH_TXS` transactions:
//! `*_forward` hashes a forwarded batch one transaction per task, as `BlockBuilder::on_transactions`
//! does; `payload_*` goes through `Payload`, as block reconstruction does.

use std::{hint::black_box, time::Instant};

use committable::Committable;
use espresso_types::{ChainConfig, NamespaceId, NsTable, Payload, Transaction};
use hotshot_types::traits::{BlockPayload, block_contents::Transaction as _};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rayon::prelude::*;
use sha2::Digest as _;

const MB: usize = 1_000_000;
const NAMESPACES: u64 = 16;
const ITERS: usize = 20;

fn main() {
    let config = Config::from_env();
    let txs = transactions(&config);
    let (payload, ns_table) =
        Payload::from_transactions_sync(txs.clone(), chain_config()).expect("payload construction");

    println!(
        "{} txs x {} bytes, {} cores available, median of {ITERS} runs, ms",
        config.txs,
        config.tx_bytes,
        std::thread::available_parallelism().map_or(0, |n| n.get()),
    );
    let mut header = format!("{:<22}", "case");
    for threads in &config.threads {
        header += &format!("{:>10}", format!("{threads}t"));
    }
    println!("{header}");

    for case in CASES {
        let mut row = format!("{:<22}", case.name);
        for &threads in &config.threads {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .expect("rayon pool");
            let ms = pool.install(|| median_ms(|| (case.run)(&txs, &payload, &ns_table)));
            row += &format!("{ms:>10.2}");
        }
        println!("{row}");
    }
}

struct Case {
    name: &'static str,
    run: fn(&[Transaction], &Payload, &NsTable),
}

const CASES: &[Case] = &[
    Case {
        name: "tx_commit",
        run: |txs, _, _| {
            let _ = black_box(txs[0].commit());
        },
    },
    Case {
        name: "tx_digest",
        run: |txs, _, _| {
            let _ = black_box(txs[0].digest());
        },
    },
    Case {
        name: "commit_forward",
        run: |txs, _, _| {
            let _ = black_box(txs.par_iter().map(|t| t.commit()).collect::<Vec<_>>());
        },
    },
    Case {
        name: "digest_forward",
        run: |txs, _, _| {
            let _ = black_box(txs.par_iter().map(|t| t.digest()).collect::<Vec<_>>());
        },
    },
    Case {
        name: "payload_commitments",
        run: |_, p, ns| {
            let _ = black_box(p.transaction_commitments(ns));
        },
    },
    Case {
        name: "payload_digests",
        run: |_, p, ns| {
            let _ = black_box(p.transaction_digests(ns));
        },
    },
    Case {
        name: "raw_keccak256",
        run: |txs, _, _| {
            let _ = black_box(sha3::Keccak256::digest(txs[0].payload()));
        },
    },
    Case {
        name: "raw_sha256",
        run: |txs, _, _| {
            let _ = black_box(sha2::Sha256::digest(txs[0].payload()));
        },
    },
    Case {
        name: "raw_blake3",
        run: |txs, _, _| {
            let _ = black_box(blake3::hash(txs[0].payload()));
        },
    },
    Case {
        name: "raw_blake3_rayon",
        run: |txs, _, _| {
            let _ = black_box(
                blake3::Hasher::new()
                    .update_rayon(txs[0].payload())
                    .finalize(),
            );
        },
    },
];

struct Config {
    txs: usize,
    tx_bytes: usize,
    threads: Vec<usize>,
}

impl Config {
    fn from_env() -> Self {
        let var = |name: &str| std::env::var(name).ok();
        let parse = |name: &str, s: &str| -> usize {
            s.trim()
                .parse()
                .unwrap_or_else(|_| panic!("{name}: not a number: {s}"))
        };
        Self {
            txs: var("TX_HASH_TXS").map_or(50, |s| parse("TX_HASH_TXS", &s)),
            tx_bytes: var("TX_HASH_TX_BYTES").map_or(MB, |s| parse("TX_HASH_TX_BYTES", &s)),
            threads: var("TX_HASH_THREADS")
                .unwrap_or_else(|| "1,2,4,8,16".into())
                .split(',')
                .map(|s| parse("TX_HASH_THREADS", s))
                .collect(),
        }
    }
}

fn transactions(config: &Config) -> Vec<Transaction> {
    let mut rng = ChaCha20Rng::seed_from_u64(0);
    (0..config.txs)
        .map(|i| {
            let mut body = vec![0u8; config.tx_bytes];
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

fn median_ms(mut f: impl FnMut()) -> f64 {
    f();
    let mut times: Vec<f64> = (0..ITERS)
        .map(|_| {
            let start = Instant::now();
            f();
            start.elapsed().as_secs_f64() * 1e3
        })
        .collect();
    times.sort_by(f64::total_cmp);
    times[ITERS / 2]
}
