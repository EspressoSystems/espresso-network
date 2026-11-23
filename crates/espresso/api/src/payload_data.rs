//! The raw bytes of a payload, addressed by its VID commitment.
//!
//! Streams send blocks and payloads without the payload's bytes, but with their BLAKE3 hash. The
//! bytes are served at this route of the same API, under the payload's VID commitment, which a
//! block's header and a payload response name. A URL's content never changes, so it can be cached
//! by anyone, and a server may redirect to another place that holds the bytes, such as an object
//! store.

use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
    routing::get,
};
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

async fn payload_data<S>(State(state): State<Arc<S>>, Path(hash): Path<String>) -> Response
where
    S: AvailabilityService + Send + Sync + 'static,
{
    serve(state.as_ref(), hash).await
}

/// The answer to a request for the payload whose VID commitment is `hash`: its raw bytes, from
/// `state`'s `GetPayload`, for a server that serves the route itself.
pub async fn serve<S>(state: &S, hash: String) -> Response
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
