//! The raw bytes of a payload, addressed by its VID commitment.
//!
//! Streams send blocks and payloads without the payload's bytes, but with their BLAKE3 hash. The
//! bytes are served at this route of the same API, under the payload's VID commitment, which a
//! block's header and a payload response name. A URL's content never changes, so it can be cached
//! by anyone, and a server may redirect to another place that holds the bytes, such as an object
//! store.
//!
//! A request with a `Range` header gets that range of the bytes only, as a client that reads one
//! namespace of a large payload does.

use std::{ops::Range, sync::Arc};

use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use bytes::Bytes;
use tonic_rest::RestError;

use crate::proto::{self, availability_service_server::AvailabilityService};

/// The route, which takes the payload's VID commitment as `hash`.
pub const PAYLOAD_DATA_ROUTE: &str = "/v2/availability/payload-data/{hash}";

/// How long a client may cache a payload: a year, since a URL's content never changes.
const CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

/// The path of the payload whose VID commitment is `hash`.
pub fn path(hash: &str) -> String {
    format!("/v2/availability/payload-data/{hash}")
}

/// The route, answered from `state`'s `GetPayload`.
pub fn router<S>(state: Arc<S>) -> Router
where
    S: AvailabilityService + Send + Sync + 'static,
{
    Router::new()
        .route(PAYLOAD_DATA_ROUTE, get(payload_data::<S>))
        .with_state(state)
}

async fn payload_data<S>(
    State(state): State<Arc<S>>,
    Path(hash): Path<String>,
    headers: HeaderMap,
) -> Response
where
    S: AvailabilityService + Send + Sync + 'static,
{
    serve(state.as_ref(), hash, ByteRange::of(&headers)).await
}

/// The byte range a request asks for with its `Range` header: `bytes=a-b`, `bytes=a-` or
/// `bytes=-n`. A header with several ranges or another unit is ignored, and the whole payload is
/// served, as HTTP allows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByteRange {
    /// From the first byte on, up to and including the last, or to the end.
    From { first: u64, last: Option<u64> },
    /// The last bytes, this many.
    Suffix(u64),
}

impl ByteRange {
    pub fn of(headers: &HeaderMap) -> Option<Self> {
        let value = headers.get(header::RANGE)?.to_str().ok()?;
        let spec = value.trim().strip_prefix("bytes=")?;
        if spec.contains(',') {
            return None;
        }
        let (first, last) = spec.split_once('-')?;
        let (first, last) = (first.trim(), last.trim());
        if first.is_empty() {
            return last.parse().ok().map(Self::Suffix);
        }
        let first = first.parse().ok()?;
        let last = match last {
            "" => None,
            last => Some(last.parse().ok()?),
        };
        if last.is_some_and(|last| last < first) {
            return None;
        }
        Some(Self::From { first, last })
    }

    /// The bytes of the range within `len` bytes, or `None` if it holds none of them.
    pub fn within(self, len: u64) -> Option<Range<u64>> {
        let range = match self {
            Self::From { first, last } => {
                first..last.map_or(len, |last| last.saturating_add(1).min(len))
            },
            Self::Suffix(n) => len.saturating_sub(n)..len,
        };
        (range.start < range.end).then_some(range)
    }
}

/// A 206 answer with `bytes`, which are `range` of a payload of `len` bytes, if known.
pub fn partial(bytes: Bytes, range: Range<u64>, len: Option<u64>) -> Response {
    let len = len.map_or_else(|| "*".to_string(), |len| len.to_string());
    let content_range = format!("bytes {}-{}/{len}", range.start, range.end - 1);
    (
        StatusCode::PARTIAL_CONTENT,
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::CACHE_CONTROL, CACHE_CONTROL.to_string()),
            (header::CONTENT_RANGE, content_range),
        ],
        bytes,
    )
        .into_response()
}

/// The answer to a request for the payload whose VID commitment is `hash`, or for `range` of it:
/// its raw bytes, from `state`'s `GetPayload`, for a server that serves the route itself.
pub async fn serve<S>(state: &S, hash: String, range: Option<ByteRange>) -> Response
where
    S: AvailabilityService + Send + Sync + 'static,
{
    let request = proto::GetPayloadRequest {
        hash: Some(hash),
        ..Default::default()
    };
    let payload = match state.get_payload(tonic::Request::new(request)).await {
        Ok(response) => response.into_inner(),
        Err(status) => return RestError::from(status).into_response(),
    };
    let raw = payload
        .data
        .map(|data| data.raw_payload)
        .unwrap_or_default();
    answer(raw, range)
}

/// The answer with the payload `raw`, or with `range` of it.
pub fn answer(raw: Bytes, range: Option<ByteRange>) -> Response {
    let len = raw.len() as u64;
    if let Some(range) = range {
        return match range.within(len) {
            Some(range) => partial(
                raw.slice(range.start as usize..range.end as usize),
                range,
                Some(len),
            ),
            None => (
                StatusCode::RANGE_NOT_SATISFIABLE,
                [(header::CONTENT_RANGE, format!("bytes */{len}"))],
            )
                .into_response(),
        };
    }
    let etag = format!("\"{}\"", blake3::hash(&raw).to_hex());
    (
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::CACHE_CONTROL, CACHE_CONTROL.to_string()),
            (header::ETAG, etag),
            (header::ACCEPT_RANGES, "bytes".to_string()),
        ],
        raw,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn range(value: &str) -> Option<ByteRange> {
        let mut headers = HeaderMap::new();
        headers.insert(header::RANGE, HeaderValue::from_str(value).unwrap());
        ByteRange::of(&headers)
    }

    #[test]
    fn reads_the_ranges_it_serves_and_ignores_the_others() {
        assert_eq!(range("bytes=2-5").and_then(|r| r.within(10)), Some(2..6));
        assert_eq!(range("bytes=7-").and_then(|r| r.within(10)), Some(7..10));
        assert_eq!(range("bytes=-3").and_then(|r| r.within(10)), Some(7..10));
        assert_eq!(range("bytes=8-100").and_then(|r| r.within(10)), Some(8..10));
        assert_eq!(range("bytes=10-12").and_then(|r| r.within(10)), None);
        assert_eq!(range("bytes=5-2"), None);
        assert_eq!(range("bytes=0-1,4-5"), None);
        assert_eq!(range("items=0-1"), None);
    }
}
