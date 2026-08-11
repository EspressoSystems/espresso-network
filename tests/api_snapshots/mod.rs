//! Snapshot tests for every HTTP API in the native demo.
//!
//! One test spins up the demo, waits for it to be serving, and compares every endpoint's response
//! body against a committed snapshot. See `README.md` in this directory for how to record and
//! review snapshots.

mod node;
mod services;

use std::{collections::BTreeMap, time::Duration};

use anyhow::{Context, Result, anyhow, bail};
use espresso_types::SeqTypes;
use futures::StreamExt;
use hotshot_query_service::{availability::LeafQueryData, types::HeightIndexed};
use tokio::time::{Instant, sleep};
use tokio_tungstenite::tungstenite::Message;

use crate::common::{
    NativeDemo, TestConfig,
    api_snapshot::{Params, Probe, Service, SnapshotRunner, WsProbe, wait_for_http},
};

/// Services whose API must be up before probing. The node validator is not included: it only
/// serves a socket route, which the stream probe gates on itself.
const REQUIRED_SERVICES: &[Service] = &[
    Service::Node0,
    Service::Node1,
    Service::Node2,
    Service::Node3,
    Service::Node4,
    Service::Orchestrator,
    Service::StateRelay,
    Service::Builder,
    Service::SubmitPublic,
    Service::SubmitPrivate,
];

/// Probe the version the network actually runs. This genesis also sets a short epoch, so the
/// stake table and reward endpoints have real data to serve within a minute of the first block.
const GENESIS_FILE: &str = "data/genesis/demo-drb-header.toml";

/// The demo needs several minutes to deploy contracts and start producing blocks.
const SERVICE_TIMEOUT: Duration = Duration::from_secs(300);
const PROGRESS_TIMEOUT: Duration = Duration::from_secs(300);

/// The light client only serves stake tables from epoch 3 on, and this genesis has 30-block
/// epochs, so wait until the chain is into its fourth epoch before probing anything.
const MIN_HEIGHT: u64 = 95;

#[tokio::test(flavor = "multi_thread")]
async fn test_api_snapshots() -> Result<()> {
    // Spawn the demo before loading `.env`: dotenvy expands `$VAR` references in the file, which
    // corrupts the values process-compose needs to see.
    let _demo = if std::env::var("API_SNAPSHOT_EXTERNAL_DEMO").as_deref() == Ok("1") {
        println!("probing an externally started demo");
        None
    } else {
        Some(NativeDemo::run(
            None,
            // process-compose runs from the root of the repo
            Some(vec![(
                "ESPRESSO_NODE_GENESIS_FILE".to_string(),
                GENESIS_FILE.to_string(),
            )]),
        )?)
    };
    dotenvy::dotenv().ok();

    wait_for_services().await?;
    wait_for_chain_progress().await?;

    let params = resolve_params().await?;
    println!("resolved probe params: {:#?}", params.values);

    let mut runner = SnapshotRunner::new(params)?;
    for probe in http_probes() {
        runner.run(probe).await;
    }
    for probe in ws_probes() {
        runner.run_ws(probe).await;
    }
    runner.finish()
}

fn http_probes() -> impl Iterator<Item = &'static Probe> {
    [
        node::AVAILABILITY_GENESIS,
        node::AVAILABILITY_TRANSACTIONS,
        node::NODE_MODULE,
        node::EPOCHS_AND_REWARDS,
        node::STATE_AND_STATUS,
        node::CATCHUP,
        node::LIGHT_CLIENT,
        node::EXPLORER,
        node::V2,
        node::TOP_LEVEL,
        node::PER_NODE,
        services::ORCHESTRATOR,
        services::STATE_RELAY,
        services::BUILDER,
        services::SUBMIT_TRANSACTIONS,
        services::NODE_METRICS,
    ]
    .into_iter()
    .flatten()
}

fn ws_probes() -> impl Iterator<Item = &'static WsProbe> {
    node::STREAMS.iter()
}

async fn wait_for_services() -> Result<()> {
    let waits = REQUIRED_SERVICES.iter().map(|service| async move {
        let url = format!("{}/healthcheck", service.base_url()?);
        wait_for_http(&url, SERVICE_TIMEOUT)
            .await
            .with_context(|| format!("waiting for {}", service.slug()))
    });
    futures::future::try_join_all(waits).await?;
    println!("all services are serving");
    Ok(())
}

/// Wait until the chain has produced blocks past genesis and the relay has aggregated a signed
/// state, so the endpoints that report live state have something real to report.
async fn wait_for_chain_progress() -> Result<()> {
    let what = format!("block height >= {MIN_HEIGHT}");
    let height = poll_until(PROGRESS_TIMEOUT, &what, || async {
        let height: u64 = get_json(&node_url("/v1/status/block-height")).await?;
        Ok((height >= MIN_HEIGHT).then_some(height))
    })
    .await?;
    println!("chain is at height {height}");

    poll_until(PROGRESS_TIMEOUT, "relay has a signed state", || async {
        let url = format!("{}/api/lateststate", Service::StateRelay.base_url()?);
        let response = reqwest::get(&url).await?;
        Ok(response.status().is_success().then_some(()))
    })
    .await?;
    println!("relay is serving a signed state");
    Ok(())
}

/// Resolve the hashes, heights and identifiers that probe paths refer to, so no probe hard-codes a
/// value that changes from run to run.
async fn resolve_params() -> Result<Params> {
    let mut values = BTreeMap::new();

    let genesis: LeafQueryData<SeqTypes> = get_json(&node_url("/v1/availability/leaf/0")).await?;
    values.insert("leaf_hash", genesis.hash().to_string());
    values.insert("block_hash", genesis.block_hash().to_string());
    values.insert("payload_hash", genesis.payload_hash().to_string());

    let head: u64 = get_json(&node_url("/v1/node/block-height")).await?;
    // Probe a height that is already decided everywhere rather than the very tip.
    let head = head.saturating_sub(2).max(1);
    values.insert("head", head.to_string());

    let recent: LeafQueryData<SeqTypes> =
        get_json(&node_url(&format!("/v1/availability/leaf/{head}"))).await?;
    values.insert("catchup_height", recent.height().to_string());
    values.insert(
        "catchup_view",
        recent.leaf().view_number().u64().to_string(),
    );

    let builder_address: String = get_json(&format!(
        "{}/block_info/builderaddress",
        Service::Builder.base_url()?
    ))
    .await?;
    values.insert("builder_address", builder_address);

    let transaction = first_transaction().await?;
    values.insert("tx_height", transaction.height.to_string());
    values.insert("tx_height_next", (transaction.height + 1).to_string());
    values.insert("tx_index", transaction.index.to_string());
    values.insert("tx_hash", transaction.hash);
    values.insert("ns", transaction.namespace);

    // State signatures are kept for a bounded window of recent heights, so ask the node which of
    // them it can actually serve rather than assuming.
    let signed_height = first_signed_height(head).await?;
    values.insert("signed_height", signed_height.to_string());

    // Epoch-keyed data is pruned as the chain advances, so ask the chain which epoch it is in and
    // walk back from there rather than naming an epoch that will go stale. Resolved through
    // `validators` rather than `stake-table` because the two differ in which epochs they retain.
    let current_epoch = current_epoch().await?;
    let epoch = served_epoch("/v1/node/validators", current_epoch).await?;
    values.insert("epoch", epoch.to_string());

    let lc_epoch = served_epoch("/v1/light-client/stake-table", current_epoch).await?;
    values.insert("lc_epoch", lc_epoch.to_string());

    values.insert(
        "validator_address",
        TestConfig::validator0_address().to_string(),
    );

    Ok(Params { values })
}

/// The epoch the chain is in right now.
async fn current_epoch() -> Result<u64> {
    let current: serde_json::Value = get_json(&node_url("/v1/node/stake-table/current")).await?;
    current
        .get("epoch")
        .and_then(serde_json::Value::as_u64)
        .context("stake-table/current has no epoch")
}

/// Walk back from the last completed epoch to one `route` still answers for. Which epochs a module
/// retains differs between modules and shrinks as the chain advances, so this cannot be derived: the
/// validator set is pruned for old epochs, while a state certificate does not exist for the epoch
/// still in progress. The most recently completed epoch is the one both will serve.
async fn served_epoch(route: &str, current: u64) -> Result<u64> {
    const WINDOW: u64 = 8;
    let newest = current.saturating_sub(1).max(1);
    let oldest = newest.saturating_sub(WINDOW).max(1);
    for epoch in (oldest..=newest).rev() {
        if reqwest::get(&node_url(&format!("{route}/{epoch}")))
            .await?
            .status()
            .is_success()
        {
            return Ok(epoch);
        }
    }
    bail!("{route} serves no epoch in {oldest}..={newest}")
}

struct FoundTransaction {
    height: u64,
    index: u64,
    hash: String,
    namespace: String,
}

/// Read the first transaction the chain has produced from the transaction stream. This is more
/// reliable than scanning blocks, because the stream skips empty blocks for us.
async fn first_transaction() -> Result<FoundTransaction> {
    let url = format!(
        "{}/v1/availability/stream/transactions/0",
        Service::Node0.base_url()?.replacen("http", "ws", 1)
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(&url)
        .await
        .with_context(|| format!("connecting to {url}"))?;

    let deadline = Instant::now() + PROGRESS_TIMEOUT;
    while Instant::now() < deadline {
        let Some(message) = socket.next().await else {
            bail!("transaction stream closed before yielding a transaction");
        };
        let Message::Text(text) = message.context("reading transaction stream")? else {
            continue;
        };
        let value: serde_json::Value = serde_json::from_str(&text)?;
        let field = |name: &str| -> Result<&serde_json::Value> {
            value
                .get(name)
                .ok_or_else(|| anyhow!("transaction has no {name} field: {value}"))
        };
        return Ok(FoundTransaction {
            height: field("block_height")?
                .as_u64()
                .context("block_height is not a number")?,
            index: field("index")?.as_u64().context("index is not a number")?,
            hash: field("hash")?
                .as_str()
                .context("hash is not a string")?
                .to_string(),
            namespace: field("namespace")?.to_string(),
        });
    }
    bail!("no transaction seen within {PROGRESS_TIMEOUT:?}")
}

/// Find a height whose state signature the node still has.
async fn first_signed_height(head: u64) -> Result<u64> {
    let candidates = (1..=head).rev().take(20);
    for height in candidates {
        let url = node_url(&format!("/v1/state-signature/block/{height}"));
        if reqwest::get(&url).await?.status().is_success() {
            return Ok(height);
        }
    }
    bail!("no height at or below {head} has a state signature")
}

fn node_url(path: &str) -> String {
    let base = Service::Node0
        .base_url()
        .expect("node 0 port is set in .env");
    format!("{base}{path}")
}

async fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T> {
    let response = reqwest::get(url)
        .await
        .with_context(|| format!("GET {url}"))?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        bail!("GET {url} returned {status}: {body}");
    }
    serde_json::from_str(&body).with_context(|| format!("parsing response of {url}: {body}"))
}

/// Poll a condition until it yields a value, reporting what we were waiting for on timeout.
async fn poll_until<T, F, Fut>(timeout_after: Duration, what: &str, mut f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<Option<T>>>,
{
    let deadline = Instant::now() + timeout_after;
    let mut last_err = None;
    while Instant::now() < deadline {
        match f().await {
            Ok(Some(value)) => return Ok(value),
            Ok(None) => {},
            Err(err) => {
                tracing::debug!(%err, "still waiting for {what}");
                last_err = Some(err);
            },
        }
        sleep(Duration::from_secs(1)).await;
    }
    Err(anyhow!(
        "timed out after {timeout_after:?} waiting for {what}: {last_err:?}"
    ))
}
