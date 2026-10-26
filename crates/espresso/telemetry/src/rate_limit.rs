//! Single-shot ERROR on the first HTTP 429 from the telemetry proxy, which caps
//! each node's hourly byte budget. Only the metrics push observes 429 (logs
//! retry via opentelemetry-otlp's `experimental-http-retry`); `compare_exchange`
//! on the shared latch keeps it to one ERROR per process.

use std::sync::atomic::{AtomicBool, Ordering};

/// Log a single ERROR on the first 429, deduped via `flag`. `env_filter` is the
/// active `ESPRESSO_NODE_TELEMETRY_LOG`; `retry_after_secs` is the parsed
/// `Retry-After` header, if numeric.
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
             Your current ESPRESSO_NODE_TELEMETRY_LOG is \"{env_filter}\": narrow it (e.g. \
             \"warn\", or \"warn,hotshot=info\"). Retry-After: {retry}. This message is logged \
             once per process."
        );
    }
}
