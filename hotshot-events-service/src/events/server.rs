use axum::{
    Router,
    extract::{State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode as HttpStatusCode},
    response::Response,
    routing::get,
};
use disco_types::status::StatusCode;
use futures::StreamExt;
use hotshot_types::traits::node_implementation::NodeType;
use http_wire::{
    self as wire, ContentType, WireFormat, body_limit_layer, cors_layer, drive_ws_stream,
    healthcheck_response, module_healthcheck_response,
};
use url::Url;
use vbs::version::StaticVersionType;

use super::Error;
use crate::events_source::EventsSource;

/// Wire format of the events API: `Ver` VBS framing and the [`Error`] envelope.
struct EventsWireFormat<Ver>(std::marker::PhantomData<Ver>);

impl<Ver: StaticVersionType + 'static> WireFormat for EventsWireFormat<Ver> {
    type Error = Error;
    type Version = Ver;

    fn status(err: &Error) -> HttpStatusCode {
        HttpStatusCode::from_u16(u16::from(disco_types::error::Error::status(err)))
            .unwrap_or(HttpStatusCode::INTERNAL_SERVER_ERROR)
    }

    fn serialize_failure(message: String) -> Error {
        Error::Custom {
            message,
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

async fn startup_info<Types, S, Ver>(State(state): State<S>, headers: HeaderMap) -> Response
where
    Types: NodeType,
    S: EventsSource<Types> + Send + Sync + 'static,
    Ver: StaticVersionType + 'static,
{
    let info = state.get_startup_info().await;
    wire::respond::<EventsWireFormat<Ver>, _>(&headers, Ok::<_, Error>(info))
}

/// The events module's routes with current (v1+) semantics: WS `events` streaming [`Event`]s
/// and GET `startup_info`.
///
/// [`Event`]: hotshot_types::event::Event
pub fn events_router<Types, S, Ver>(state: S) -> Router
where
    Types: NodeType,
    S: EventsSource<Types> + Clone + Send + Sync + 'static,
    Ver: StaticVersionType + 'static,
{
    let events = |ws: WebSocketUpgrade, State(state): State<S>, headers: HeaderMap| async move {
        let format = ContentType::negotiate(&headers);
        ws.on_upgrade(move |socket| async move {
            tracing::info!("client subscribed to events");
            let stream = state.get_event_stream(None).await;
            drive_ws_stream::<Ver, _>(socket, stream.boxed(), format).await;
        })
    };
    Router::new()
        .route("/events", get(events))
        .route("/startup_info", get(startup_info::<Types, S, Ver>))
        .route("/healthcheck", get(module_healthcheck))
        .with_state(state)
}

/// The events module's routes with v0 semantics: WS `events` streaming [`LegacyEvent`]s and
/// GET `startup_info`.
///
/// [`LegacyEvent`]: hotshot_types::event::LegacyEvent
pub fn legacy_events_router<Types, S, Ver>(state: S) -> Router
where
    Types: NodeType,
    S: EventsSource<Types> + Clone + Send + Sync + 'static,
    Ver: StaticVersionType + 'static,
{
    let events = |ws: WebSocketUpgrade, State(state): State<S>, headers: HeaderMap| async move {
        let format = ContentType::negotiate(&headers);
        ws.on_upgrade(move |socket| async move {
            tracing::info!("client subscribed to legacy events");
            let stream = state.get_legacy_event_stream(None).await;
            drive_ws_stream::<Ver, _>(socket, stream.boxed(), format).await;
        })
    };
    Router::new()
        .route("/events", get(events))
        .route("/startup_info", get(startup_info::<Types, S, Ver>))
        .route("/healthcheck", get(module_healthcheck))
        .with_state(state)
}

async fn healthcheck(headers: HeaderMap) -> Response {
    healthcheck_response(&headers)
}

/// The legacy server answered `/<module>/healthcheck` for the events module; the route lives on
/// the module router so it survives version mounting.
async fn module_healthcheck(headers: HeaderMap) -> Response {
    module_healthcheck_response(&headers)
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

/// Binds `url`'s host and port and serves `router` until the returned handle is aborted.
///
/// # Panics
/// If `url` has no port or the port cannot be bound.
pub fn serve(url: &Url, router: Router) -> tokio::task::JoinHandle<()> {
    wire::spawn_serve(url, router)
}
