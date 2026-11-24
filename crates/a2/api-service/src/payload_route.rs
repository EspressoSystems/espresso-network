//! The payload route, `PAYLOAD_DATA_ROUTE`, which serves a payload's raw bytes under its VID
//! commitment.
//!
//! Where clients reach the object store the payloads live in, the route redirects to the
//! payload's object, whose URL follows from the payload's VID commitment and is valid before the
//! object exists: a client that finds nothing there yet tries again, or asks another API service.
//! Otherwise the API instance serves the payload itself, or the byte range a request asks for.

use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use espresso_api::payload_data::{ByteRange, PAYLOAD_DATA_ROUTE, partial, serve};
use hotshot_types::data::VidCommitment;
use tonic_rest::RestError;

use crate::{ApiService, request::parse};

pub(crate) fn router(service: Arc<ApiService>) -> Router {
    Router::new()
        .route(PAYLOAD_DATA_ROUTE, get(payload))
        .with_state(service)
}

/// A redirect keeps the request's `Range` header, which the object store answers. A range with
/// both ends is read from the store without the rest of the payload; any other is cut from the
/// whole payload.
async fn payload(
    State(service): State<Arc<ApiService>>,
    Path(hash): Path<String>,
    headers: HeaderMap,
) -> Response {
    let commitment: VidCommitment = match parse(&hash, "hash") {
        Ok(commitment) => commitment,
        Err(status) => return RestError::from(status).into_response(),
    };
    let commitment = commitment.to_string();
    let store = service.reader.store();
    if let Some(url) = store
        .payload_store()
        .and_then(|payloads| payloads.url(&commitment))
    {
        return Redirect::temporary(&url).into_response();
    }
    let range = ByteRange::of(&headers);
    if let Some(ByteRange::From {
        first,
        last: Some(last),
    }) = range
    {
        let range = first..last.saturating_add(1);
        match store
            .payload_range_by_commitment(&commitment, range.clone())
            .await
        {
            Ok(Some(bytes)) if bytes.len() as u64 == range.end - range.start => {
                return partial(bytes, range, None);
            },
            Ok(_) => {},
            Err(err) => tracing::debug!(%commitment, "reading a payload range failed: {err:#}"),
        }
    }
    serve(service.as_ref(), hash, range).await
}
