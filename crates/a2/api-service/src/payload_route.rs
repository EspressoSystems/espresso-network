//! The payload route, `PAYLOAD_DATA_ROUTE`, which the payload references in responses name.
//!
//! Where clients reach the object store the payloads live in, the route redirects to the
//! payload's object, whose URL follows from the payload's VID commitment and is valid before the
//! object exists: a client that finds nothing there yet tries again, or asks another API service.
//! Otherwise the API instance serves the payload itself.

use std::sync::Arc;

use axum::{
    Router,
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use espresso_api::payload_data::{PAYLOAD_DATA_ROUTE, serve};
use hotshot_types::data::VidCommitment;
use serde::Deserialize;
use tonic_rest::RestError;

use crate::{ApiService, request::parse};

pub(crate) fn router(service: Arc<ApiService>) -> Router {
    Router::new()
        .route(PAYLOAD_DATA_ROUTE, get(payload))
        .with_state(service)
}

#[derive(Debug, Deserialize)]
struct PayloadQuery {
    hash: Option<String>,
}

async fn payload(
    State(service): State<Arc<ApiService>>,
    Query(query): Query<PayloadQuery>,
) -> Response {
    let Some(hash) = query.hash else {
        return RestError::from(tonic::Status::invalid_argument("hash is required"))
            .into_response();
    };
    let commitment: VidCommitment = match parse(&hash, "hash") {
        Ok(commitment) => commitment,
        Err(status) => return RestError::from(status).into_response(),
    };
    let url = service
        .reader
        .store()
        .payload_store()
        .and_then(|payloads| payloads.url(&commitment.to_string()));
    match url {
        Some(url) => Redirect::temporary(&url).into_response(),
        None => serve(service.as_ref(), hash).await,
    }
}
