//! Bounded retry wrapper around a [`LogExporter`].
//!
//! The OTel SDK's `BatchLogProcessor` clears each batch unconditionally after
//! `export()` returns — Ok or Err — and never retries. A 5-30s proxy or
//! aggregator restart therefore drops every batch attempted during the window.
//! This wrapper bounces the call up to `MAX_ATTEMPTS` times with exponential
//! backoff so a brief outage (the typical deploy case) stops being a data loss
//! event.
//!
//! Retry policy:
//! - **Retry** on `Timeout` and on transport / 5xx failures, which surface as
//!   `InternalFailure(String)` from `opentelemetry-otlp`.
//! - **Don't retry** on `AlreadyShutdown` or on responses whose error string
//!   contains `Status Code: 4` — auth / bad-payload errors won't recover and
//!   retrying would just amplify stderr noise.
//! - 4xx detection is a string-match. The HTTP exporter formats failures as
//!   `OpenTelemetry logs export failed. Url: ..., Status Code: NNN, ...`; the
//!   match is best-effort and degrades to "retry" if the format ever changes.
//! - HTTP 429 specifically is recognized by string match (`Status Code: 429`)
//!   and surfaces a single shared ERROR via
//!   [`crate::rate_limit::log_rate_limit_once`]. Retry decision is unchanged
//!   (4xx remains non-retryable).
//!
//! Sleep semantics: the SDK's batch processor calls `export()` from a dedicated
//! `std::thread` driven by `futures_executor::block_on` — there's no async
//! runtime in scope. Backoff therefore uses `std::thread::sleep`, which is
//! exactly the right primitive: it pauses the dedicated export thread (which
//! has no other work) until the next attempt.

use std::{
    fmt::Debug,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use opentelemetry::InstrumentationScope;
use opentelemetry_sdk::{
    Resource,
    error::{OTelSdkError, OTelSdkResult},
    logs::{LogBatch, LogExporter, SdkLogRecord},
};

use crate::rate_limit::log_rate_limit_once;

const MAX_ATTEMPTS: u32 = 4;
const BASE_BACKOFF: Duration = Duration::from_millis(250);
const MAX_BACKOFF: Duration = Duration::from_secs(4);

/// Wraps a [`LogExporter`] with bounded exponential-backoff retry.
#[derive(Debug)]
pub(crate) struct RetryingLogExporter<E> {
    inner: E,
    rate_limit_warned: Arc<AtomicBool>,
    telemetry_log_filter: Arc<String>,
}

impl<E> RetryingLogExporter<E> {
    pub(crate) fn new(
        inner: E,
        rate_limit_warned: Arc<AtomicBool>,
        telemetry_log_filter: Arc<String>,
    ) -> Self {
        Self {
            inner,
            rate_limit_warned,
            telemetry_log_filter,
        }
    }
}

impl<E: LogExporter> LogExporter for RetryingLogExporter<E> {
    async fn export(&self, batch: LogBatch<'_>) -> OTelSdkResult {
        // `LogBatch` is logically a slice reference but doesn't derive
        // `Copy`/`Clone`, so reconstruct it on each retry from the original
        // batch's iterator. The underlying records are owned by the
        // BatchLogProcessor and outlive this `export()` call.
        let pairs: Vec<(&SdkLogRecord, &InstrumentationScope)> = batch.iter().collect();
        let mut attempt: u32 = 1;
        loop {
            let result = self.inner.export(LogBatch::new(&pairs)).await;
            match &result {
                Ok(_) => return result,
                Err(e) if !is_retryable(e) => {
                    if is_rate_limited(e) {
                        log_rate_limit_once(
                            &self.rate_limit_warned,
                            &self.telemetry_log_filter,
                            None,
                        );
                    }
                    return result;
                },
                Err(_) => {},
            }
            if attempt >= MAX_ATTEMPTS {
                return result;
            }
            std::thread::sleep(backoff_for(attempt));
            attempt += 1;
        }
    }

    fn shutdown_with_timeout(&self, timeout: Duration) -> OTelSdkResult {
        self.inner.shutdown_with_timeout(timeout)
    }

    fn set_resource(&mut self, resource: &Resource) {
        self.inner.set_resource(resource);
    }
}

fn is_retryable(err: &OTelSdkError) -> bool {
    match err {
        OTelSdkError::Timeout(_) => true,
        OTelSdkError::AlreadyShutdown => false,
        OTelSdkError::InternalFailure(msg) => !msg.contains("Status Code: 4"),
    }
}

/// True iff `err` is the OTLP exporter's HTTP 429 surface form. The HTTP
/// exporter formats failures as `... Status Code: NNN, ...`; we string-match
/// the 429 case so a single shared ERROR is logged on the operator side.
fn is_rate_limited(err: &OTelSdkError) -> bool {
    matches!(err, OTelSdkError::InternalFailure(msg) if msg.contains("Status Code: 429"))
}

fn backoff_for(attempt: u32) -> Duration {
    let scale = 4u64
        .checked_pow(attempt.saturating_sub(1))
        .unwrap_or(u64::MAX);
    let ms = (BASE_BACKOFF.as_millis() as u64).saturating_mul(scale);
    Duration::from_millis(ms).min(MAX_BACKOFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_4xx_as_non_retryable() {
        let e = OTelSdkError::InternalFailure(
            "OpenTelemetry logs export failed. Url: ..., Status Code: 401, Response: ...".into(),
        );
        assert!(!is_retryable(&e));
    }

    #[test]
    fn classifies_5xx_as_retryable() {
        let e = OTelSdkError::InternalFailure(
            "OpenTelemetry logs export failed. Url: ..., Status Code: 503, Response: ...".into(),
        );
        assert!(is_retryable(&e));
    }

    #[test]
    fn classifies_transport_error_as_retryable() {
        let e = OTelSdkError::InternalFailure("connection refused".into());
        assert!(is_retryable(&e));
    }

    #[test]
    fn classifies_timeout_as_retryable() {
        let e = OTelSdkError::Timeout(Duration::from_secs(10));
        assert!(is_retryable(&e));
    }

    #[test]
    fn classifies_already_shutdown_as_non_retryable() {
        assert!(!is_retryable(&OTelSdkError::AlreadyShutdown));
    }

    #[test]
    fn classifies_429_as_rate_limited() {
        let e = OTelSdkError::InternalFailure(
            "OpenTelemetry logs export failed. Url: ..., Status Code: 429, Response: ...".into(),
        );
        assert!(is_rate_limited(&e));
        assert!(!is_retryable(&e));
    }

    #[test]
    fn classifies_other_4xx_as_not_rate_limited() {
        let e = OTelSdkError::InternalFailure(
            "OpenTelemetry logs export failed. Url: ..., Status Code: 401, Response: ...".into(),
        );
        assert!(!is_rate_limited(&e));
    }

    #[test]
    fn backoff_is_exponential_and_capped() {
        assert_eq!(backoff_for(1), Duration::from_millis(250));
        assert_eq!(backoff_for(2), Duration::from_millis(1000));
        assert_eq!(backoff_for(3), MAX_BACKOFF);
        assert_eq!(backoff_for(99), MAX_BACKOFF);
    }
}
