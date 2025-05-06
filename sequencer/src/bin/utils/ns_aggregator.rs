use std::cmp::min;

use anyhow::{ensure, Context};
use clap::Parser;
use espresso_types::{Header, NamespaceId, NamespaceProofQueryData};
use futures::{future::try_join_all, stream::StreamExt};
use sequencer::SequencerApiVersion;
use surf_disco::Url;

/// Count transactions and bytes confirmed in a given namespace.
#[derive(Debug, Parser)]
pub struct Options {
    /// Block to start counting from.
    #[clap(short, long, default_value = "0")]
    from_block: usize,

    /// Last block to count (inclusive).
    ///
    /// If not specified, will count until the end of the chain at the moment the program is
    /// invoked.
    #[clap(short, long)]
    to_block: Option<usize>,

    /// Namespace to aggregate.
    #[clap(short, long)]
    namespace: u64,

    /// Number of parallel tasks to run.
    ///
    /// Since the process is mostly I/O bound, increasing this can dramatically speed up
    /// aggregation.
    #[clap(short, long, default_value = "1")]
    jobs: usize,

    /// Espresso query service URL.
    url: Url,
}

pub async fn run(opt: Options) -> anyhow::Result<()> {
    let ns = NamespaceId::from(opt.namespace);
    let client = surf_disco::Client::<hotshot_query_service::Error, SequencerApiVersion>::new(
        opt.url.clone(),
    );

    // Convert optional closed [from, to] interval to a semi-open [start, end) interval, which makes
    // arithmetic simpler later on.
    let start = opt.from_block;
    let end = match opt.to_block {
        Some(to) => to + 1,
        None => client.get("status/block-height").send().await?,
    };
    ensure!(end > start, "to-block < from-block");

    let tasks = (0..opt.jobs).map(|i| {
        let chunk_size = (end - start) / opt.jobs;
        let chunk_start = start + i * chunk_size;
        let chunk_end = min(chunk_start + chunk_size, end);
        process_chunk(opt.url.clone(), ns, chunk_start, chunk_end)
    });
    let chunks = try_join_all(tasks).await?;

    let mut num_txs = 0;
    let mut bytes = 0;
    for (chunk_txs, chunk_bytes) in chunks {
        num_txs += chunk_txs;
        bytes += chunk_bytes;
    }

    println!("Scanned range [{start}, {end}) for namespace {ns}");
    println!("{num_txs} transactions");
    println!("{bytes} bytes");

    Ok(())
}

async fn process_chunk(
    url: Url,
    ns: NamespaceId,
    start: usize,
    end: usize,
) -> anyhow::Result<(usize, usize)> {
    let client = surf_disco::Client::<hotshot_query_service::Error, SequencerApiVersion>::new(url);
    let mut headers = client
        .socket(&format!("availability/stream/headers/{start}"))
        .subscribe::<Header>()
        .await?
        .take(end - start);

    let mut num_txs = 0;
    let mut bytes = 0;
    while let Some(header) = headers.next().await {
        let header = header?;
        let height = header.height();

        if header.ns_table().find_ns_id(&ns).is_none() {
            tracing::debug!(height, "trivial block");
            continue;
        }
        tracing::info!(height, "non-trivial block");

        let payload: NamespaceProofQueryData = client
            .get(&format!("availability/block/{height}/namespace/{ns}",))
            .send()
            .await
            .context(format!("requesting namespace for block {height}"))?;
        num_txs += payload.transactions.len();
        bytes += payload
            .transactions
            .iter()
            .map(|tx| tx.payload().len())
            .sum::<usize>();
    }

    Ok((num_txs, bytes))
}
