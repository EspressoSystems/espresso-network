//! Submits transactions to an API service at a fixed rate and measures, per transaction, the time
//! from submission until the API service streams it back in a decided block.
//!
//! Each payload starts with the transaction's sequence number and its submission time, so the
//! measurement needs no state shared between the submitting and the reading side.

use std::{
    collections::BTreeMap,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use clap::Parser;
use futures::StreamExt;
use serde_json::{Value, json};
use tokio::time::{MissedTickBehavior, interval};
use url::Url;

#[derive(Debug, Parser)]
struct Options {
    /// The API service to submit to and read from.
    #[arg(long, default_value = "http://127.0.0.1:24000")]
    api: Url,

    /// Transactions per second.
    #[arg(long, default_value_t = 50)]
    rate: u64,

    /// The size of each transaction in bytes, at least 16.
    #[arg(long, default_value_t = 1024)]
    size: usize,

    /// How long to submit for.
    #[arg(long, default_value = "60s", value_parser = humantime::parse_duration)]
    duration: Duration,

    /// The namespace to submit to.
    #[arg(long, default_value_t = 4242)]
    namespace: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = Options::parse();
    if options.size < 16 {
        bail!("transactions need at least 16 bytes for their sequence number and time");
    }
    let http = reqwest::Client::new();
    let from = block_height(&http, &options.api).await?;
    // Read until the last transactions have had time to be decided.
    let until = Instant::now() + options.duration + Duration::from_secs(15);
    let (submitted, received) = tokio::join!(
        submit(&http, &options),
        read(
            http.clone(),
            options.api.clone(),
            options.namespace,
            from,
            until
        ),
    );
    let submitted = submitted?;
    report(&options, submitted, &received);
    Ok(())
}

/// Submits at `options.rate` for `options.duration`, returning how many were accepted.
async fn submit(http: &reqwest::Client, options: &Options) -> anyhow::Result<u64> {
    let url = options.api.join("v2/submit/transaction")?;
    let mut tick = interval(Duration::from_secs_f64(1.0 / options.rate as f64));
    tick.set_missed_tick_behavior(MissedTickBehavior::Burst);
    let end = Instant::now() + options.duration;
    let mut sequence = 0u64;
    let mut accepted = 0;
    let mut pending = futures::stream::FuturesUnordered::new();
    while Instant::now() < end {
        tick.tick().await;
        let mut payload = vec![0u8; options.size];
        payload[..8].copy_from_slice(&sequence.to_be_bytes());
        payload[8..16].copy_from_slice(&now_micros().to_be_bytes());
        let body = json!({
            "namespace": options.namespace.to_string(),
            "payload": STANDARD.encode(&payload),
        });
        pending.push(http.post(url.clone()).json(&body).send());
        sequence += 1;
        // Collect finished submissions without waiting for the slow ones.
        while let Some(Some(result)) = futures::FutureExt::now_or_never(pending.next()) {
            accepted += u64::from(result.is_ok_and(|r| r.status().is_success()));
        }
    }
    while let Some(result) = pending.next().await {
        accepted += u64::from(result.is_ok_and(|r| r.status().is_success()));
    }
    eprintln!("submitted {sequence}, accepted {accepted}");
    Ok(accepted)
}

/// What the stream delivered: per height, the latencies of the transactions in it, in
/// microseconds, and when the block arrived.
#[derive(Default)]
struct Received {
    blocks: BTreeMap<u64, (Instant, Vec<u64>)>,
}

/// Reads the namespace's stream from `from` on until `until`.
async fn read(
    http: reqwest::Client,
    api: Url,
    namespace: u64,
    from: u64,
    until: Instant,
) -> Received {
    let mut received = Received::default();
    let deadline = tokio::time::Instant::from_std(until);
    let reading = read_into(&mut received, http, api, namespace, from);
    let _ = tokio::time::timeout_at(deadline, reading).await;
    received
}

async fn read_into(
    received: &mut Received,
    http: reqwest::Client,
    api: Url,
    namespace: u64,
    from: u64,
) {
    let Ok(url) = api.join(&format!(
        "v2/availability/stream/namespace-proofs?from={from}&namespace={namespace}"
    )) else {
        return;
    };
    let Ok(response) = http
        .get(url)
        .header("Accept", "text/event-stream")
        .send()
        .await
    else {
        return;
    };
    let mut body = response.bytes_stream();
    let mut buffer = String::new();
    let mut height = from;
    while let Some(Ok(chunk)) = body.next().await {
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(end) = buffer.find("\n\n") {
            let event: String = buffer.drain(..end + 2).collect();
            let Some(data) = event.lines().find_map(|line| line.strip_prefix("data:")) else {
                continue;
            };
            let arrived = Instant::now();
            let latencies = latencies(data.trim(), now_micros());
            received.blocks.insert(height, (arrived, latencies));
            height += 1;
        }
    }
}

fn latencies(data: &str, now: u64) -> Vec<u64> {
    let Ok(value) = serde_json::from_str::<Value>(data) else {
        return Vec::new();
    };
    let Some(transactions) = value["transactions"].as_array() else {
        return Vec::new();
    };
    transactions
        .iter()
        .filter_map(|tx| STANDARD.decode(tx["payload"].as_str()?).ok())
        .filter(|payload| payload.len() >= 16)
        .map(|payload| {
            let sent = u64::from_be_bytes(payload[8..16].try_into().unwrap());
            now.saturating_sub(sent)
        })
        .collect()
}

async fn block_height(http: &reqwest::Client, api: &Url) -> anyhow::Result<u64> {
    let value: Value = http
        .get(api.join("v2/status/block-height")?)
        .send()
        .await?
        .json()
        .await?;
    value["height"]
        .as_str()
        .and_then(|h| h.parse().ok())
        .context("a block height")
}

fn now_micros() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}

fn report(options: &Options, submitted: u64, received: &Received) {
    let mut latencies: Vec<u64> = received
        .blocks
        .values()
        .flat_map(|(_, l)| l.iter().copied())
        .collect();
    latencies.sort_unstable();
    let arrivals: Vec<Instant> = received.blocks.values().map(|(at, _)| *at).collect();
    let mut gaps: Vec<f64> = arrivals
        .windows(2)
        .map(|w| (w[1] - w[0]).as_secs_f64())
        .collect();
    gaps.sort_by(f64::total_cmp);
    let pct = |v: &[u64], p: f64| v.get(((v.len() as f64 - 1.0) * p) as usize).copied();
    let ms = |us: Option<u64>| us.map_or("-".into(), |us| format!("{:.0} ms", us as f64 / 1000.0));
    let gap = |p: f64| {
        gaps.get(((gaps.len() as f64 - 1.0) * p) as usize)
            .map_or("-".into(), |s| format!("{:.0} ms", s * 1000.0))
    };
    let seconds = options.duration.as_secs_f64();
    println!(
        "load: {} tx/s of {} bytes for {seconds:.0}s ({:.1} KB/s)",
        options.rate,
        options.size,
        options.rate as f64 * options.size as f64 / 1000.0
    );
    println!(
        "received {} of {submitted} accepted transactions in {} blocks",
        latencies.len(),
        received.blocks.len()
    );
    println!(
        "latency submit -> streamed: p50 {} p90 {} p99 {} max {}",
        ms(pct(&latencies, 0.5)),
        ms(pct(&latencies, 0.9)),
        ms(pct(&latencies, 0.99)),
        ms(latencies.last().copied())
    );
    println!(
        "block interval at the API: p50 {} p90 {} max {}",
        gap(0.5),
        gap(0.9),
        gap(1.0)
    );
}
