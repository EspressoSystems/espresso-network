//! The payload route, `PAYLOAD_DATA_ROUTE`, which serves a payload's raw bytes under its VID
//! commitment.
//!
//! The store answers with the bytes, or, for a large uploaded payload in an object store clients
//! reach, with where it is: the route then redirects there, or, for a URL of the local file
//! system, reads the bytes from there itself. A payload the store cannot serve yet
//! is waited for a moment, as it may still be on its way to the object store.

use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use espresso_api::payload_data::{ByteRange, PAYLOAD_DATA_ROUTE, answer, partial};
use hotshot_types::data::VidCommitment;
use store_api::{PayloadAt, PayloadData, Store};
use tokio::time::{Instant, sleep};
use tonic::Status;
use tonic_rest::RestError;

use crate::{ApiService, request::parse};

/// How long the route waits for a payload the store cannot serve yet.
const WAIT: Duration = Duration::from_millis(500);

pub(crate) fn router(service: Arc<ApiService>) -> Router {
    Router::new()
        .route(PAYLOAD_DATA_ROUTE, get(payload))
        .with_state(service)
}

/// A redirect keeps the request's `Range` header, which the object store answers. A range with
/// both ends is asked of the store alone; any other is cut from the whole payload.
async fn payload(
    State(service): State<Arc<ApiService>>,
    Path(hash): Path<String>,
    headers: HeaderMap,
) -> Response {
    let commitment: VidCommitment = match parse(&hash, "hash") {
        Ok(commitment) => commitment,
        Err(status) => return RestError::from(status).into_response(),
    };
    let at = PayloadAt::Commitment(commitment.to_string());
    let store = service.reader.store();
    let range = ByteRange::of(&headers);
    if let Some(ByteRange::From {
        first,
        last: Some(last),
    }) = range
    {
        let bounded = first..last.saturating_add(1);
        match store.payload(&at, Some(bounded.clone())).await {
            Ok(Some(PayloadData::Url(url))) if redirectable(&url) => {
                return Redirect::temporary(&url).into_response();
            },
            Ok(Some(PayloadData::Url(url))) => {
                match crate::payload::fetch(&service.http, &url, Some(bounded.clone())).await {
                    Ok(Some(bytes)) => return partial(bytes, bounded, None),
                    Ok(None) => {},
                    Err(err) => tracing::debug!(%commitment, "reading {url} failed: {err:#}"),
                }
            },
            Ok(Some(PayloadData::Inline(bytes)))
                if bytes.len() as u64 == bounded.end - bounded.start =>
            {
                return partial(bytes, bounded, None);
            },
            Ok(_) => {},
            Err(err) => tracing::debug!(%commitment, "reading a payload range failed: {err:#}"),
        }
    }
    match until_served(store.as_ref(), &at).await {
        Ok(Some(PayloadData::Url(url))) if redirectable(&url) => {
            Redirect::temporary(&url).into_response()
        },
        Ok(Some(PayloadData::Url(url))) => {
            match crate::payload::fetch(&service.http, &url, None).await {
                Ok(Some(bytes)) => answer(bytes, range),
                Ok(None) => RestError::from(Status::not_found(format!("no payload {commitment}")))
                    .into_response(),
                Err(err) => RestError::from(Status::internal(format!("{err:#}"))).into_response(),
            }
        },
        Ok(Some(PayloadData::Inline(bytes))) => answer(bytes, range),
        Ok(None) => {
            RestError::from(Status::not_found(format!("no payload {commitment}"))).into_response()
        },
        Err(err) => RestError::from(Status::internal(format!("{err:#}"))).into_response(),
    }
}

/// The payload `at` names, asking again, waiting longer each time, for up to [`WAIT`] while the
/// store cannot serve it.
async fn until_served(store: &dyn Store, at: &PayloadAt) -> anyhow::Result<Option<PayloadData>> {
    let deadline = Instant::now() + WAIT;
    let mut delay = Duration::from_millis(20);
    loop {
        let data = store.payload(at, None).await?;
        if data.is_some() || Instant::now() + delay > deadline {
            return Ok(data);
        }
        sleep(delay).await;
        delay *= 2;
    }
}

/// Whether a client can be sent to `url` itself: a URL of the local file system is read by the
/// API instance instead.
fn redirectable(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}
