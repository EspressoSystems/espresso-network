//! Minimal HTTP server exposing the light client contract address, shared by the v1/v2/v3
//! prover services.

use alloy::primitives::Address;
use axum::{Json, Router, routing::get};

/// Serve the light client contract address at the paths tide-disco used to expose it:
/// `/v0/api/lightclient_contract` directly, and `/api/lightclient_contract` (which tide-disco
/// served via a redirect to the versioned path). Also serves `/healthcheck`. Runs until the
/// process exits; bind failures are logged, not propagated, since this server only provides a
/// healthcheck ahead of the prover's (fallible) main loop.
pub(crate) fn start_light_client_contract_server(port: u16, light_client_address: Address) {
    let router = Router::new()
        .route(
            "/api/lightclient_contract",
            get(move || async move { Json(light_client_address) }),
        )
        .route(
            "/v0/api/lightclient_contract",
            get(move || async move { Json(light_client_address) }),
        )
        .route("/healthcheck", get(healthcheck));

    tokio::spawn(async move {
        let addr = format!("0.0.0.0:{port}");
        let listener = match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => listener,
            Err(err) => {
                tracing::error!("Failed to start prover http server on http://{addr} : {err}");
                return;
            },
        };
        if let Err(err) = axum::serve(listener, router).await {
            tracing::error!("Prover http server on http://{addr} stopped: {err}");
        }
    });
}

async fn healthcheck() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "Available" }))
}
