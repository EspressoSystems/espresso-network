//! Single-shot ERROR log on first HTTP 429 from the telemetry proxy.
//!
//! The proxy applies a per-node hourly byte budget and returns 429 once the
//! budget is exhausted. Both telemetry pipelines (OTLP/HTTP logs and Prometheus
//! remote-write metrics) can observe the rejection. We want exactly one
//! operator-facing ERROR per process, regardless of which pipeline saw it
//! first or how many subsequent rejections fire.
//!
//! The `Arc<AtomicBool>` is owned by the `TelemetryHandle` and shared with both
//! pipelines so dedup is process-wide. `compare_exchange` guarantees only the
//! first observer logs.

use std::sync::atomic::{AtomicBool, Ordering};

/// Log a single ERROR on the first 429 observed by any telemetry pipeline.
///
/// `flag` is the shared dedup latch; `env_filter` is the operator's resolved
/// `ESPRESSO_TELEMETRY_LOG` value at startup; `retry_after_secs` is the parsed
/// `Retry-After` header value when available (None when the header is missing
/// or non-numeric).
pub(crate) fn log_rate_limit_once(
    flag: &AtomicBool,
    env_filter: &str,
    retry_after_secs: Option<u64>,
) {
    if flag
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        let retry = retry_after_secs
            .map(|s| format!("{s}s"))
            .unwrap_or_else(|| "unknown".to_string());
        tracing::error!(
            telemetry_log = env_filter,
            retry_after = %retry,
            "telemetry rate limit hit (HTTP 429). The proxy capped this node's hourly byte budget. \
             Your current ESPRESSO_TELEMETRY_LOG is \"{env_filter}\" — narrow it (e.g. \"warn\", \
             or \"warn,hotshot=info\") or reduce metric cardinality. Retry-After: {retry}. \
             This message is logged once per process."
        );
    }
}
