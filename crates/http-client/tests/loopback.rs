//! Loopback tests against a small axum server, covering the response-decode paths of
//! `Request::send`, `Request::bytes`, healthcheck polling, and the frame-decode and
//! redirect-following paths of `SocketRequest`.

use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use axum::{
    Json, Router,
    extract::{
        State,
        ws::{Message as WsMessage, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::{get, post},
};
use futures::{SinkExt, StreamExt};
use http_client::{Client, ClientError, error::ClientErr, healthcheck::HealthStatus};
use tokio::net::TcpListener;
use vbs::{BinarySerializer, Serializer, version::StaticVersion};

type Ver01 = StaticVersion<0, 1>;

async fn spawn_server(app: Router) -> url::Url {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}/").parse().unwrap()
}

async fn get_json() -> impl IntoResponse {
    Json("response".to_owned())
}

async fn get_binary() -> impl IntoResponse {
    let body = Serializer::<Ver01>::serialize(&"response".to_owned()).unwrap();
    (
        [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
        body,
    )
}

async fn get_error() -> impl IntoResponse {
    let err = ClientErr::catch_all(reqwest::StatusCode::NOT_FOUND, "not found".to_owned());
    (axum::http::StatusCode::NOT_FOUND, Json(err))
}

async fn get_binary_error() -> impl IntoResponse {
    let err = ClientErr::catch_all(
        reqwest::StatusCode::SERVICE_UNAVAILABLE,
        "try later".to_owned(),
    );
    (
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
        Serializer::<Ver01>::serialize(&err).unwrap(),
    )
}

async fn ws_redirect() -> impl IntoResponse {
    (
        axum::http::StatusCode::TEMPORARY_REDIRECT,
        [(axum::http::header::LOCATION, "/ws_naturals")],
    )
}

async fn ws_naturals(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        for i in 0u64..3 {
            let bytes = Serializer::<Ver01>::serialize(&i).unwrap();
            if socket.send(WsMessage::Binary(bytes.into())).await.is_err() {
                return;
            }
        }
        let _ = socket.close().await;
    })
}

#[tokio::test]
async fn send_decodes_json_response() {
    let app = Router::new().route("/get_json", get(get_json));
    let base_url = spawn_server(app).await;

    let client = Client::<ClientErr, Ver01>::new(base_url);
    let res: String = client.get("get_json").send().await.unwrap();
    assert_eq!(res, "response");
}

#[tokio::test]
async fn send_decodes_binary_response() {
    let app = Router::new().route("/get_binary", get(get_binary));
    let base_url = spawn_server(app).await;

    let client = Client::<ClientErr, Ver01>::new(base_url);
    let res: String = client.get("get_binary").send().await.unwrap();
    assert_eq!(res, "response");
}

#[tokio::test]
async fn send_decodes_error_body() {
    let app = Router::new().route("/get_error", get(get_error));
    let base_url = spawn_server(app).await;

    let client = Client::<ClientErr, Ver01>::new(base_url);
    let err = client.get::<String>("get_error").send().await.unwrap_err();
    assert_eq!(err.status, reqwest::StatusCode::NOT_FOUND);
    assert_eq!(err.message, "not found");
}

#[tokio::test]
async fn send_decodes_binary_error_body() {
    let app = Router::new().route("/get_binary_error", get(get_binary_error));
    let base_url = spawn_server(app).await;

    let client = Client::<ClientErr, Ver01>::new(base_url);
    let err = client
        .get::<String>("get_binary_error")
        .send()
        .await
        .unwrap_err();
    assert_eq!(err.status, reqwest::StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(err.message, "try later");
}

#[tokio::test]
async fn bytes_returns_raw_body() {
    let app = Router::new().route("/get_binary", get(get_binary));
    let base_url = spawn_server(app).await;

    let client = Client::<ClientErr, Ver01>::new(base_url);
    let bytes = client.get::<()>("get_binary").bytes().await.unwrap();
    assert_eq!(
        bytes,
        Serializer::<Ver01>::serialize(&"response".to_owned()).unwrap()
    );
}

#[tokio::test]
async fn bytes_reports_error_status_with_utf8_body_as_message() {
    let app = Router::new().route("/get_error", get(get_error));
    let base_url = spawn_server(app).await;

    let client = Client::<ClientErr, Ver01>::new(base_url);
    let err = client.get::<()>("get_error").bytes().await.unwrap_err();
    assert_eq!(err.status, reqwest::StatusCode::NOT_FOUND);
    assert_eq!(err.message, r#"{"status":404,"message":"not found"}"#);
}

#[tokio::test]
async fn module_prefixes_routes() {
    let app = Router::new().route("/mod/get_json", get(get_json));
    let base_url = spawn_server(app).await;

    let client = Client::<ClientErr, Ver01>::new(base_url);
    let module = client.module::<ClientErr>("mod/").unwrap();
    let res: String = module.get("get_json").send().await.unwrap();
    assert_eq!(res, "response");
}

#[tokio::test]
async fn socket_subscribe_decodes_binary_frames() {
    let app = Router::new().route("/ws_naturals", get(ws_naturals));
    let base_url = spawn_server(app).await;

    let client = Client::<ClientErr, Ver01>::new(base_url);
    let naturals: Vec<_> = client
        .socket("ws_naturals")
        .subscribe::<u64>()
        .await
        .unwrap()
        .collect()
        .await;
    assert_eq!(naturals, (0u64..3).map(Ok).collect::<Vec<_>>());
}

#[tokio::test]
async fn socket_connect_follows_redirects() {
    let app = Router::new()
        .route("/ws_redirect", get(ws_redirect))
        .route("/ws_naturals", get(ws_naturals));
    let base_url = spawn_server(app).await;

    let client = Client::<ClientErr, Ver01>::new(base_url);
    let naturals: Vec<_> = client
        .socket("ws_redirect")
        .subscribe::<u64>()
        .await
        .unwrap()
        .collect()
        .await;
    assert_eq!(naturals, (0u64..3).map(Ok).collect::<Vec<_>>());
}

#[tokio::test]
async fn socket_connect_gives_up_after_redirect_loop() {
    async fn ws_redirect_self() -> impl IntoResponse {
        (
            axum::http::StatusCode::TEMPORARY_REDIRECT,
            [(axum::http::header::LOCATION, "/ws_redirect_self")],
        )
    }
    let app = Router::new().route("/ws_redirect_self", get(ws_redirect_self));
    let base_url = spawn_server(app).await;

    let client = Client::<ClientErr, Ver01>::new(base_url);
    let err = client
        .socket("ws_redirect_self")
        .subscribe::<u64>()
        .await
        .unwrap_err();
    assert!(err.message.contains("redirect"), "got: {err}");
}

#[tokio::test]
async fn healthcheck_connect_and_wait_for_health() {
    async fn healthcheck(State(state): State<Arc<RwLock<HealthStatus>>>) -> impl IntoResponse {
        Json(*state.read().unwrap())
    }
    async fn init(State(state): State<Arc<RwLock<HealthStatus>>>) -> impl IntoResponse {
        *state.write().unwrap() = HealthStatus::Available;
        Json(())
    }

    let state = Arc::new(RwLock::new(HealthStatus::Initializing));
    let app = Router::new()
        .route("/healthcheck", get(healthcheck))
        .route("/init", post(init))
        .with_state(state);
    let base_url = spawn_server(app).await;

    let client = Client::<ClientErr, Ver01>::new(base_url);
    // Server is up: succeeds on the first probe, no retry sleep.
    assert!(client.connect(None).await);
    assert_eq!(
        client.healthcheck::<HealthStatus>().await.unwrap(),
        HealthStatus::Initializing
    );

    // Expired deadline: returns None without waiting out the 10s retry interval.
    assert_eq!(
        client
            .wait_for_health::<HealthStatus>(
                |h| *h == HealthStatus::Available,
                Some(Duration::ZERO),
            )
            .await,
        None
    );

    client.post::<()>("init").send().await.unwrap();
    assert_eq!(
        client
            .wait_for_health::<HealthStatus>(|h| *h == HealthStatus::Available, None)
            .await,
        Some(HealthStatus::Available)
    );
}

#[tokio::test]
async fn connect_returns_false_when_server_is_down() {
    // Reserve a port, then drop the listener so nothing is serving on it.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let client = Client::<ClientErr, Ver01>::new(format!("http://{addr}/").parse().unwrap());
    assert!(!client.connect(Some(Duration::ZERO)).await);
}
