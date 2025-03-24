use std::cmp::min;

use anyhow::ensure;
use clap::Parser;
use espresso_types::{Header, NamespaceId, NamespaceProofQueryData};
use futures::{future::try_join_all, stream::StreamExt};
use sequencer::SequencerApiVersion;
use surf_disco::Url;

#[derive(Debug, Parser)]
pub struct Options {
    #[clap(short, long, default_value = "0")]
    from_block: usize,

    #[clap(short, long)]
    to_block: Option<usize>,

    #[clap(short, long)]
    namespace: u64,

    #[clap(short, long, default_value = "1")]
    jobs: usize,

    url: Url,
}

pub async fn run(opt: Options) -> anyhow::Result<()> {
    let ns = NamespaceId::from(opt.namespace);
    let client = surf_disco::Client::<hotshot_query_service::Error, SequencerApiVersion>::new(
        opt.url.clone(),
    );

    let from = opt.from_block;
    let to = match opt.to_block {
        Some(to) => to,
        None => client.get("status/block-height").send().await?,
    };
    ensure!(to >= from, "to-block < from-block");

    let tasks = (0..opt.jobs).map(|i| {
        let chunk_size = (to - from) / opt.jobs;
        let start = from + i * chunk_size;
        let end = min(start + chunk_size, to);
        process_chunk(opt.url.clone(), ns, start, end)
    });
    let chunks = try_join_all(tasks).await?;

    let mut num_txs = 0;
    let mut bytes = 0;
    for (chunk_txs, chunk_bytes) in chunks {
        num_txs += chunk_txs;
        bytes += chunk_bytes;
    }

    println!("Scanned range [{from}, {to}) for namespace {ns}");
    println!("{num_txs} transactions");
    println!("{bytes} bytes");

    Ok(())
}

async fn process_chunk(
    url: Url,
    ns: NamespaceId,
    from: usize,
    to: usize,
) -> anyhow::Result<(usize, usize)> {
    let client = surf_disco::Client::<hotshot_query_service::Error, SequencerApiVersion>::new(url);
    let mut headers = client
        .socket(&format!("availability/stream/headers/{from}"))
        .subscribe::<Header>()
        .await?
        .take(to - from);

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
            .await?;
        num_txs += payload.transactions.len();
        bytes += payload
            .transactions
            .iter()
            .map(|tx| tx.payload().len())
            .sum::<usize>();
    }

    Ok((num_txs, bytes))
}
