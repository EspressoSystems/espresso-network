//! Minimal HTTP server exposing the light client contract address, shared by the v1/v2/v3
//! prover services.

use alloy::primitives::Address;
use axum::{Json, Router, http::HeaderMap, response::Response, routing::get};
use espresso_wire::{cors_layer, healthcheck_response};

/// Serves the light client contract address at the paths tide-disco used to expose it:
/// `/v0/api/lightclient_contract` directly, and `/api/lightclient_contract` (which tide-disco
/// served via a redirect to the versioned path). Also serves `/healthcheck`. Like tide-disco,
/// every response carries permissive CORS headers.
fn router(light_client_address: Address) -> Router {
    Router::new()
        .route(
            "/api/lightclient_contract",
            get(move || async move { Json(light_client_address) }),
        )
        .route(
            "/v0/api/lightclient_contract",
            get(move || async move { Json(light_client_address) }),
        )
        .route("/healthcheck", get(healthcheck))
        .layer(cors_layer())
}

/// Runs [`router`] until the process exits; bind failures are logged, not propagated, since this
/// server only provides a healthcheck ahead of the prover's (fallible) main loop.
pub(crate) fn start_light_client_contract_server(port: u16, light_client_address: Address) {
    let router = router(light_client_address);

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

async fn healthcheck(headers: HeaderMap) -> Response {
    healthcheck_response(&headers)
}

#[cfg(test)]
mod tests {
    use axum::http::{Request, StatusCode, header};

    use super::*;

    async fn get(uri: &str) -> axum::http::Response<axum::body::Body> {
        let req = Request::builder()
            .uri(uri)
            .header(header::ORIGIN, "https://example.com")
            .body(axum::body::Body::empty())
            .unwrap();
        tower::ServiceExt::oneshot(router(Address::ZERO), req)
            .await
            .unwrap()
    }

    /// tide-disco served the module at `/v0/api/...` and redirected the unversioned path there.
    #[tokio::test]
    async fn both_contract_address_paths_route() {
        for uri in ["/api/lightclient_contract", "/v0/api/lightclient_contract"] {
            assert_eq!(get(uri).await.status(), StatusCode::OK, "{uri}");
        }
    }

    /// The prover's tide-disco `App` registered a named `api` module, so it was not a singleton
    /// app and `/healthcheck` served the app-level `AppHealth` object, not a bare status.
    #[tokio::test]
    async fn healthcheck_serves_app_health() {
        let resp = get("/healthcheck").await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], br#"{"status":"available","modules":{}}"#);
    }

    /// Like tide-disco, every response carries permissive CORS headers.
    #[tokio::test]
    async fn responses_carry_cors_headers() {
        for uri in [
            "/healthcheck",
            "/v0/api/lightclient_contract",
            "/no/such/route",
        ] {
            assert_eq!(
                get(uri)
                    .await
                    .headers()
                    .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                    .unwrap_or_else(|| panic!("no CORS header on {uri}")),
                "*",
                "{uri}"
            );
        }
    }
}
