//! Utility program to verify properties of headers sequenced by HotShot.

use async_compatibility_layer::logging::{setup_backtrace, setup_logging};
use async_std::{sync::Arc, task::sleep};
use clap::Parser;
use ethers::prelude::*;
use futures::future::join_all;
use itertools::Itertools;
use sequencer::{Header, L1BlockInfo};
use std::cmp::max;
use std::process::exit;
use std::time::Duration;
use surf_disco::Url;
use versioned_binary_serialization::version::StaticVersionType;

/// Utility program to verify properties of headers sequenced by HotShot.
#[derive(Clone, Debug, Parser)]
struct Options {
    /// Start counting from block FROM.
    #[clap(long, name = "FROM")]
    from: Option<usize>,

    /// Stop counting at block TO.
    #[clap(long, name = "TO")]
    to: Option<usize>,

    /// Skip verifying header timestamps.
    #[clap(long)]
    no_timestamps: bool,

    /// Skip verifying L1 head numbers.
    #[clap(long)]
    no_l1_heads: bool,

    /// Skip verifying L1 finalized references.
    #[clap(long)]
    no_l1_finalized: bool,

    /// L1 RPC URL
    ///
    /// This will be used to verify the hashes and timestamps of L1 finalized references. If not
    /// provided, only block numbers will be verified.
    #[clap(long)]
    l1: Option<Url>,

    /// Number of parallel tasks to run.
    #[clap(short, long, default_value = "1")]
    jobs: usize,

    /// URL of the HotShot query service.
    url: Url,
}

type SequencerClient<Ver> = surf_disco::Client<hotshot_query_service::Error, Ver>;

async fn verify_header<Ver: StaticVersionType>(
    opt: &Options,
    seq: &SequencerClient<Ver>,
    l1: Option<&Provider<Http>>,
    parent: Option<Header>,
    height: usize,
) -> (Header, bool) {
    tracing::debug!("checking header {height}");
    let header = get_header(seq, height).await;

    // Check invariants relative to parent. If this is the genesis header, there is no parent and
    // nothing to do.
    if height == 0 {
        return (header, true);
    }

    let parent = match parent {
        Some(parent) => parent,
        None => get_header(seq, height - 1).await,
    };

    let mut ok = true;

    if !opt.no_timestamps && header.timestamp < parent.timestamp {
        tracing::error!(
            "header {height} has decreasing timestamp: {} -> {}",
            parent.timestamp,
            header.timestamp
        );
        ok = false;
    }
    if !opt.no_l1_heads && header.l1_head < parent.l1_head {
        tracing::error!(
            "header {height} has decreasing L1 head: {} -> {}",
            parent.l1_head,
            header.l1_head
        );
        ok = false;
    }
    if !opt.no_l1_finalized && header.l1_finalized < parent.l1_finalized {
        tracing::error!(
            "header {height} has decreasing L1 finalized: {:?} -> {:?}",
            parent.l1_finalized,
            header.l1_finalized
        );
        ok = false;

        if let (Some(l1), Some(l1_finalized)) = (l1, &header.l1_finalized) {
            let l1_block = get_l1_block(l1, l1_finalized.number).await;
            if *l1_finalized != l1_block {
                tracing::error!(
                    "header {height} has wrong L1 finalized info: {l1_finalized:?}, expected {l1_block:?}"
                );
                ok = false;
            }
        }
    }

    (header, ok)
}

async fn get_header<Ver: StaticVersionType>(seq: &SequencerClient<Ver>, height: usize) -> Header {
    loop {
        match seq
            .get(&format!("availability/header/{height}"))
            .send()
            .await
        {
            Ok(header) => break header,
            Err(err) => {
                tracing::warn!("error fetching header {height}: {err}");

                // Back off a bit and then retry.
                sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

async fn get_l1_block(l1: &Provider<Http>, height: u64) -> L1BlockInfo {
    loop {
        let block = match l1.get_block(height).await {
            Ok(Some(block)) => block,
            Ok(None) => {
                tracing::warn!("L1 block {height} not yet available");
                sleep(Duration::from_secs(1)).await;
                continue;
            }
            Err(err) => {
                tracing::warn!("error fetching L1 block {height}: {err}");
                sleep(Duration::from_millis(100)).await;
                continue;
            }
        };

        let Some(hash) = block.hash else {
            tracing::warn!("L1 block {height} has no hash, might not be finalized yet");
            sleep(Duration::from_secs(1)).await;
            continue;
        };

        return L1BlockInfo {
            number: height,
            hash,
            timestamp: block.timestamp,
        };
    }
}

#[async_std::main]
async fn main() {
    setup_logging();
    setup_backtrace();

    let opt = Arc::new(Options::parse());
    let seq = Arc::new(SequencerClient::<es_version::SequencerVersion>::new(
        opt.url.clone(),
    ));

    let block_height: usize = seq.get("status/latest_block_height").send().await.unwrap();
    let from = opt.from.unwrap_or(0);
    let to = max(opt.to.unwrap_or(block_height), from + 1);

    tracing::info!("checking {} headers in [{from}, {to})", to - from);
    let ok = join_all(
        (from..to)
            .chunks((to - from) / opt.jobs)
            .into_iter()
            .map(|heights| {
                let opt = opt.clone();
                let seq = seq.clone();
                let l1 = opt
                    .l1
                    .as_ref()
                    .map(|url| Provider::try_from(url.to_string()).unwrap());
                async move {
                    let mut ok = true;

                    let mut parent = None;
                    for height in heights {
                        let (header, header_ok) =
                            verify_header(&opt, &seq, l1.as_ref(), parent, height).await;
                        ok &= header_ok;
                        parent = Some(header);
                    }

                    ok
                }
            }),
    )
    .await
    .into_iter()
    .all(|ok| ok);

    if ok {
        tracing::info!("all headers in [{from}, {to}) ok")
    } else {
        tracing::error!("some headers were invalid");
        exit(1);
    }
}
