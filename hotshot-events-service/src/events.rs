use axum::{
    Router,
    extract::{State, WebSocketUpgrade},
    http::HeaderMap,
    response::Response,
    routing::get,
};
use derive_more::From;
use disco_types::{request::RequestError, status::StatusCode};
use futures::StreamExt;
use hotshot_types::traits::node_implementation::NodeType;
use http_wire::{
    self as wire, ContentType, body_limit_layer, cors_layer, drive_ws_stream, healthcheck_response,
};
use serde::{Deserialize, Serialize};
use snafu::Snafu;

use crate::events_source::EventsSource;

#[derive(Clone, Debug, Snafu, Deserialize, Serialize)]
#[snafu(visibility(pub))]
pub enum EventError {
    /// The requested resource does not exist or is not known to this hotshot node.
    NotFound,
    /// The requested resource exists but is not currently available.
    Missing,
    /// There was an error while trying to fetch the requested resource.
    #[snafu(display("Failed to fetch requested resource: {message}"))]
    Error { message: String },
}

#[derive(Clone, Debug, From, Snafu, Deserialize, Serialize)]
#[snafu(visibility(pub))]
pub enum Error {
    Request {
        source: RequestError,
    },
    #[snafu(display("error receiving events {resource}: {source}"))]
    #[from(ignore)]
    EventAvailable {
        source: EventError,
        resource: String,
    },
    Custom {
        message: String,
        status: StatusCode,
    },
}

impl disco_types::error::Error for Error {
    fn catch_all(status: StatusCode, msg: String) -> Self {
        Error::Custom {
            message: msg,
            status,
        }
    }
    fn status(&self) -> StatusCode {
        match self {
            Error::Request { .. } => StatusCode::BAD_REQUEST,
            Error::EventAvailable { source, .. } => match source {
                EventError::NotFound => StatusCode::NOT_FOUND,
                EventError::Missing => StatusCode::NOT_FOUND,
                EventError::Error { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            },
            Error::Custom { status, .. } => *status,
        }
    }
}

impl http_client::ClientError for Error {
    fn catch_all(status: http_client::StatusCode, msg: String) -> Self {
        Error::Custom {
            message: msg,
            status: status.into(),
        }
    }

    fn status(&self) -> http_client::StatusCode {
        disco_types::error::Error::status(self).into()
    }
}

async fn startup_info<Types, S>(State(state): State<S>, headers: HeaderMap) -> Response
where
    Types: NodeType,
    S: EventsSource<Types> + Send + Sync + 'static,
{
    let info = state.get_startup_info().await;
    wire::respond(&headers, Ok::<_, Error>(info))
}

/// The events module's routes with current (v1+) semantics: WS `events` streaming [`Event`]s
/// and GET `startup_info`.
///
/// [`Event`]: hotshot_types::event::Event
pub fn events_router<Types, S>(state: S) -> Router
where
    Types: NodeType,
    S: EventsSource<Types> + Clone + Send + Sync + 'static,
{
    let events = |ws: WebSocketUpgrade, State(state): State<S>, headers: HeaderMap| async move {
        let format = ContentType::negotiate(&headers);
        ws.on_upgrade(move |socket| async move {
            tracing::info!("client subscribed to events");
            let stream = state.get_event_stream(None).await;
            drive_ws_stream(socket, stream.boxed(), format).await;
        })
    };
    Router::new()
        .route("/events", get(events))
        .route("/startup_info", get(startup_info::<Types, S>))
        .with_state(state)
}

/// The events module's routes with v0 semantics: WS `events` streaming [`LegacyEvent`]s and
/// GET `startup_info`.
///
/// [`LegacyEvent`]: hotshot_types::event::LegacyEvent
pub fn legacy_events_router<Types, S>(state: S) -> Router
where
    Types: NodeType,
    S: EventsSource<Types> + Clone + Send + Sync + 'static,
{
    let events = |ws: WebSocketUpgrade, State(state): State<S>, headers: HeaderMap| async move {
        let format = ContentType::negotiate(&headers);
        ws.on_upgrade(move |socket| async move {
            tracing::info!("client subscribed to legacy events");
            let stream = state.get_legacy_event_stream(None).await;
            drive_ws_stream(socket, stream.boxed(), format).await;
        })
    };
    Router::new()
        .route("/events", get(events))
        .route("/startup_info", get(startup_info::<Types, S>))
        .with_state(state)
}

async fn healthcheck(headers: HeaderMap) -> Response {
    healthcheck_response(&headers)
}

/// Wraps module routers with the app-level `healthcheck`, a request body limit, and permissive
/// CORS headers. Version mounting is up to the caller, because the events module's semantics
/// depend on the mounted version (v0 streams legacy events).
pub fn app(api: Router) -> Router {
    Router::new()
        .route("/healthcheck", get(healthcheck))
        .merge(api)
        .layer(body_limit_layer())
        .layer(cors_layer())
}
