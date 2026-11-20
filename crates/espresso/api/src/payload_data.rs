//! The raw bytes of a payload, addressed by its VID commitment.
//!
//! Streams send a block's payload by reference ([`proto::PayloadRef`]): a URL of this route, the
//! payload's BLAKE3 hash and its size. The route serves the bytes as they are, never changing for
//! a URL, so they can be cached by anyone, or moved to an object store behind the same reference.

use std::sync::Arc;

use axum::{
    Router,
    extract::{Query, State},
    http::header,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use tonic_rest::RestError;

use crate::proto::{self, availability_service_server::AvailabilityService};

/// The route, which takes the payload's VID commitment as `hash`.
pub const PAYLOAD_DATA_ROUTE: &str = "/v2/availability/payload-data";

/// How long a client may cache a payload: a year, since a URL's content never changes.
const CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

/// The URL of the payload whose VID commitment is `hash`, relative to the API serving it.
pub fn url(hash: &str) -> String {
    format!("{PAYLOAD_DATA_ROUTE}?hash={hash}")
}

/// The reference to `raw`, the raw bytes of the payload whose VID commitment is `hash`.
pub fn reference(hash: &str, raw: &[u8]) -> proto::PayloadRef {
    proto::PayloadRef {
        url: url(hash),
        blake3: blake3::hash(raw).to_hex().to_string(),
        size: raw.len() as u64,
    }
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

#[derive(Debug, Deserialize)]
struct PayloadDataQuery {
    hash: Option<String>,
}

async fn payload_data<S>(
    State(state): State<Arc<S>>,
    Query(query): Query<PayloadDataQuery>,
) -> Response
where
    S: AvailabilityService + Send + Sync + 'static,
{
    let Some(hash) = query.hash else {
        return RestError::from(tonic::Status::invalid_argument("hash is required"))
            .into_response();
    };
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
    let etag = format!("\"{}\"", blake3::hash(&raw).to_hex());
    (
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::CACHE_CONTROL, CACHE_CONTROL.to_string()),
            (header::ETAG, etag),
        ],
        raw,
    )
        .into_response()
}
