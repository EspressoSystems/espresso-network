//! Periodic push of `prometheus::Registry` snapshots to a remote-write endpoint.
//!
//! Mirrors the canonical loop in the espresso-node-telemetry mock-node.
//! `tokio::select!` between the interval tick and a shutdown signal; on shutdown
//! drive one final scrape-and-flush so the last interval's samples aren't lost.
//! Errors are `warn!`-logged and never panic / never block consensus. HTTP 429
//! triggers a single shared ERROR via [`crate::rate_limit::log_rate_limit_once`].

use std::{
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use prometheus::Registry;
use reqwest::{
    Client,
    header::{AUTHORIZATION, CONTENT_TYPE},
};
use tokio::{sync::oneshot, time::MissedTickBehavior};
use url::Url;

use crate::{
    build_write_request, encode_to_snappy,
    rate_limit::log_rate_limit_once,
    remote_write::{Label, WriteRequest},
};

/// Stamp push-time labels (e.g. `service`, `instance`) onto every TimeSeries.
/// Preserves any label the metric already carries — registry-provided labels
/// win. Re-sorts each series so the resulting protobuf stays remote-write 1.0
/// compliant (labels sorted by name).
fn apply_external_labels(request: &mut WriteRequest, external: &[Label]) {
    if external.is_empty() {
        return;
    }
    for series in &mut request.timeseries {
        for label in external {
            if !series.labels.iter().any(|l| l.name == label.name) {
                series.labels.push(label.clone());
            }
        }
        series.labels.sort_by(|a, b| a.name.cmp(&b.name));
    }
}

/// Spawn the periodic push loop. Owns the HTTP client; reuses it across ticks.
///
/// Runs until `shutdown` resolves. On shutdown drives one final flush, then
/// returns. If the HTTP client cannot be built (rare; usually only TLS init
/// failures), logs and returns immediately — the task end is the failure mode.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run(
    registry: Arc<Registry>,
    endpoint: Url,
    jwt: String,
    interval: Duration,
    external_labels: Vec<Label>,
    rate_limit_warned: Arc<AtomicBool>,
    telemetry_log_filter: Arc<String>,
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
                push_once(&client, &url, &jwt, &registry, &external_labels, &rate_limit_warned, &telemetry_log_filter).await;
            }
            _ = &mut shutdown => {
                push_once(&client, &url, &jwt, &registry, &external_labels, &rate_limit_warned, &telemetry_log_filter).await;
                break;
            }
        }
    }
}

async fn push_once(
    client: &Client,
    url: &str,
    jwt: &str,
    registry: &Registry,
    external_labels: &[Label],
    rate_limit_warned: &AtomicBool,
    telemetry_log_filter: &str,
) {
    let families = registry.gather();
    let mut request = match build_write_request(&families) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "telemetry: skipping metrics push: encode failed");
            return;
        },
    };
    apply_external_labels(&mut request, external_labels);
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
        Ok(r) if r.status().as_u16() == 429 => {
            let retry_after = r
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse::<u64>().ok());
            log_rate_limit_once(rate_limit_warned, telemetry_log_filter, retry_after);
        },
        Ok(r) => tracing::warn!(status = %r.status(), "telemetry: metrics push non-2xx"),
        Err(e) => tracing::warn!(error = %e, "telemetry: metrics push failed"),
    }
}
