//! Classification of L1 RPC errors into flavors that later stages can react to.
//!
//! Providers reject `eth_getLogs` calls in ways that are deterministic (a range or result-count
//! cap, an inactive API key) and will never succeed no matter how many times the node retries.
//! `classify` turns the opaque [`RpcError`] into an [`RpcErrorKind`] so callers can stop retrying
//! the impossible instead of burning their retry budget on it.

use std::time::Duration;

use alloy::transports::{HttpError, RpcError, TransportErrorKind};

/// The flavor of an L1 RPC error, as distinguished by JSON-RPC code and message content.
///
/// `-32600` (Invalid Request) is reused by providers for both [`Self::RangeTooLarge`] (alchemy's
/// free-tier `eth_getLogs` cap) and [`Self::AuthFailed`] (alchemy's inactive-key rejection), so the
/// message content, not the code alone, drives the decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RpcErrorKind {
    /// The requested block range exceeds a provider-side cap. `suggested` is the cap the
    /// provider's message names, when it names one.
    RangeTooLarge { suggested: Option<u64> },
    /// The query matched more results than the provider will return, independent of block range.
    TooManyResults,
    /// The provider is throttling this key. `retry_after` is the provider's hinted backoff.
    RateLimited { retry_after: Option<Duration> },
    /// The provider rejected the request because the API key/app is disabled. Never transient:
    /// alchemy's `App is inactive` is 50,358 of the observed fleet failures and none of them
    /// ever succeeded on retry.
    AuthFailed,
    /// A generic, presumably short-lived, server or gateway failure worth retrying.
    Transient,
    /// Recognized as an error but not classifiable into any of the above.
    Unknown,
}

impl RpcErrorKind {
    /// Every label `label()` can return, for pre-registering one metric per kind.
    pub(crate) const ALL_LABELS: [&'static str; 6] = [
        "range_too_large",
        "too_many_results",
        "rate_limited",
        "auth_failed",
        "transient",
        "unknown",
    ];

    /// Stable label used as the `kind` value of the `consensus_l1_errors` metric.
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::RangeTooLarge { .. } => "range_too_large",
            Self::TooManyResults => "too_many_results",
            Self::RateLimited { .. } => "rate_limited",
            Self::AuthFailed => "auth_failed",
            Self::Transient => "transient",
            Self::Unknown => "unknown",
        }
    }
}

/// Classify an L1 RPC error. Pure: no network access, no allocation beyond the small string
/// searches needed to inspect the provider's message.
pub(crate) fn classify(err: &RpcError<TransportErrorKind>) -> RpcErrorKind {
    match err {
        RpcError::ErrorResp(e) => classify_json_rpc(e.code, &e.message),
        RpcError::Transport(kind) => classify_transport(kind),
        _ => RpcErrorKind::Unknown,
    }
}

/// Providers commonly reject non-2xx `eth_getLogs` calls with the JSON-RPC error object embedded
/// in the HTTP body rather than in a parsed [`RpcError::ErrorResp`], since alloy's HTTP transport
/// only produces `ErrorResp` for 2xx responses. Recover the same code/message pair from the body
/// when possible, and fall back to the HTTP status alone otherwise.
fn classify_transport(kind: &TransportErrorKind) -> RpcErrorKind {
    let TransportErrorKind::HttpError(http_err) = kind else {
        return RpcErrorKind::Transient;
    };
    match extract_json_rpc_error(http_err) {
        Some((code, message)) => classify_json_rpc(code, &message),
        None => classify_http_status(http_err.status),
    }
}

/// Parses `{"error": {"code": ..., "message": ...}}` or a bare `{"code": ..., "message": ...}`
/// out of an HTTP error body.
fn extract_json_rpc_error(http_err: &HttpError) -> Option<(i64, String)> {
    let value: serde_json::Value = serde_json::from_str(&http_err.body).ok()?;
    let error = value.get("error").unwrap_or(&value);
    let code = error.get("code")?.as_i64()?;
    let message = error.get("message")?.as_str()?.to_owned();
    Some((code, message))
}

fn classify_http_status(status: u16) -> RpcErrorKind {
    match status {
        429 => RpcErrorKind::RateLimited { retry_after: None },
        403 => RpcErrorKind::AuthFailed,
        408 | 502 | 503 | 504 => RpcErrorKind::Transient,
        _ => RpcErrorKind::Unknown,
    }
}

fn classify_json_rpc(code: i64, message: &str) -> RpcErrorKind {
    let lower = message.to_ascii_lowercase();

    if lower.contains("block range") {
        return RpcErrorKind::RangeTooLarge {
            suggested: suggested_range(&lower),
        };
    }
    if lower.contains("app is inactive") {
        return RpcErrorKind::AuthFailed;
    }
    if lower.contains("too many requests")
        || lower.contains("request limit reached")
        || matches!(code, 429 | -32005 | -32007)
    {
        return RpcErrorKind::RateLimited {
            retry_after: parse_retry_after(&lower),
        };
    }
    if lower.contains("more than") && lower.contains("results") {
        return RpcErrorKind::TooManyResults;
    }
    if lower.contains("temporarily unavailable")
        || lower.contains("timed out")
        || lower.contains("no available upstreams")
        || code == -32603
    {
        return RpcErrorKind::Transient;
    }
    RpcErrorKind::Unknown
}

/// Extracts the block-range hint a provider embeds in its own message, e.g. "up to a 10 block
/// range" -> `Some(10)`. Providers that only say the range is "too large" give no number.
fn suggested_range(lower_message: &str) -> Option<u64> {
    let before = &lower_message[..lower_message.find("block range")?];
    before.split_whitespace().last()?.parse().ok()
}

/// Parses a provider-supplied backoff hint, e.g. "try again in 4ms" or "try again in 2s".
fn parse_retry_after(lower_message: &str) -> Option<Duration> {
    let after = lower_message.split_once("try again in ")?.1.trim_start();
    let digits_len = after.bytes().take_while(u8::is_ascii_digit).count();
    let (digits, unit) = after.split_at(digits_len);
    let value: u64 = digits.parse().ok()?;
    match unit
        .trim()
        .trim_end_matches(|c: char| c.is_ascii_punctuation())
    {
        "ms" => Some(Duration::from_millis(value)),
        "s" => Some(Duration::from_secs(value)),
        _ => None,
    }
}

#[cfg(test)]
mod test {
    use alloy::rpc::json_rpc::ErrorPayload;

    use super::*;

    /// A `RpcError::ErrorResp`, as produced for a 2xx response whose body carries a JSON-RPC
    /// error object.
    fn error_resp(code: i64, message: &str) -> RpcError<TransportErrorKind> {
        RpcError::ErrorResp(ErrorPayload {
            code,
            message: message.to_owned().into(),
            data: None,
        })
    }

    /// A `RpcError::Transport(HttpError)`, as produced for a non-2xx response. `body` is the raw
    /// HTTP response body, which providers commonly fill with a JSON-RPC error object even though
    /// alloy does not parse it as one at this status.
    fn http_error(status: u16, body: impl Into<String>) -> RpcError<TransportErrorKind> {
        TransportErrorKind::http_error(status, body.into())
    }

    fn json_rpc_body(code: i64, message: &str) -> String {
        format!(r#"{{"jsonrpc":"2.0","id":1,"error":{{"code":{code},"message":"{message}"}}}}"#)
    }

    // Verbatim fleet telemetry, see doc/l1-robustness.md A2.3.
    #[test]
    fn classify_alchemy_free_tier_range() {
        let msg = "Under the Free tier plan, you can make eth_getLogs requests with up to a 10 \
                   block range";
        let err = http_error(400, json_rpc_body(-32600, msg));
        assert_eq!(
            classify(&err),
            RpcErrorKind::RangeTooLarge {
                suggested: Some(10)
            }
        );
    }

    #[test]
    fn classify_getblock_range_too_large() {
        let err = error_resp(-32062, "Block range is too large");
        assert_eq!(
            classify(&err),
            RpcErrorKind::RangeTooLarge { suggested: None }
        );
    }

    #[test]
    fn classify_alchemy_app_inactive() {
        let msg = "App is inactive. Please create a new app";
        let err = http_error(403, json_rpc_body(-32600, msg));
        assert_eq!(classify(&err), RpcErrorKind::AuthFailed);
    }

    #[test]
    fn classify_infura_too_many_requests() {
        let err = http_error(429, json_rpc_body(-32005, "Too Many Requests"));
        assert!(matches!(classify(&err), RpcErrorKind::RateLimited { .. }));
    }

    #[test]
    fn classify_quicknode_request_limit() {
        let msg = "50/second request limit reached";
        let err = http_error(429, json_rpc_body(-32007, msg));
        assert!(matches!(classify(&err), RpcErrorKind::RateLimited { .. }));
    }

    #[test]
    fn classify_infura_service_unavailable() {
        let err = error_resp(-32603, "service temporarily unavailable");
        assert_eq!(classify(&err), RpcErrorKind::Transient);
    }

    #[test]
    fn classify_request_timed_out() {
        let err = http_error(408, json_rpc_body(-32009, "Request timed out"));
        assert_eq!(classify(&err), RpcErrorKind::Transient);
    }

    #[test]
    fn classify_no_available_upstreams() {
        let err = error_resp(1, "no available upstreams to process a request");
        assert_eq!(classify(&err), RpcErrorKind::Transient);
    }

    #[test]
    fn classify_bad_gateway() {
        let err = http_error(502, "Bad Gateway");
        assert_eq!(classify(&err), RpcErrorKind::Transient);
    }

    #[test]
    fn classify_unknown_block() {
        let err = http_error(400, json_rpc_body(26, "Unknown block"));
        assert_eq!(classify(&err), RpcErrorKind::Unknown);
    }
}
