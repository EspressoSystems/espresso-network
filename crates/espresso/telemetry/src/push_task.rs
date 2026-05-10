//! Periodic push of `prometheus::Registry` snapshots to a remote-write endpoint.
//!
//! Mirrors the canonical loop in the espresso-node-telemetry mock-node.
//! `tokio::select!` between the interval tick and a shutdown signal; on shutdown
//! drive one final scrape-and-flush so the last interval's samples aren't lost.
//! Errors are `warn!`-logged and never panic / never block consensus.

use std::{sync::Arc, time::Duration};

use prometheus::Registry;
use reqwest::{
    Client,
    header::{AUTHORIZATION, CONTENT_TYPE},
};
use tokio::{sync::oneshot, time::MissedTickBehavior};
use url::Url;

use crate::{build_write_request, encode_to_snappy};

/// Spawn the periodic push loop. Owns the HTTP client; reuses it across ticks.
///
/// Runs until `shutdown` resolves. On shutdown drives one final flush, then
/// returns. If the HTTP client cannot be built (rare; usually only TLS init
/// failures), logs and returns immediately — the task end is the failure mode.
pub(crate) async fn run(
    registry: Arc<Registry>,
    endpoint: Url,
    jwt: String,
    interval: Duration,
    mut shutdown: oneshot::Receiver<()>,
) {
    let url = format!("{}/api/v1/write", endpoint.as_str().trim_end_matches('/'));
    let client = match Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "telemetry: metrics http client init failed; metrics push disabled");
            return;
        },
    };

    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    // First tick fires immediately; skip it so we don't push an empty initial scrape.
    ticker.tick().await;

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                push_once(&client, &url, &jwt, &registry).await;
            }
            _ = &mut shutdown => {
                push_once(&client, &url, &jwt, &registry).await;
                break;
            }
        }
    }
}

async fn push_once(client: &Client, url: &str, jwt: &str, registry: &Registry) {
    let families = registry.gather();
    let request = match build_write_request(&families) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "telemetry: skipping metrics push: encode failed");
            return;
        },
    };
    let body = match encode_to_snappy(&request) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(error = %e, "telemetry: skipping metrics push: snappy compress failed");
            return;
        },
    };

    let resp = client
        .post(url)
        .header(AUTHORIZATION, format!("Bearer {jwt}"))
        .header(CONTENT_TYPE, "application/x-protobuf")
        .header("Content-Encoding", "snappy")
        .body(body)
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {},
        Ok(r) => tracing::warn!(status = %r.status(), "telemetry: metrics push non-2xx"),
        Err(e) => tracing::warn!(error = %e, "telemetry: metrics push failed"),
    }
}
