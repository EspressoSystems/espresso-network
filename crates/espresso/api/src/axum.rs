//! Axum HTTP/JSON API handlers

use aide::{
    axum::{
        routing::{get_with},
        ApiRouter,
    },
    openapi::{Info, OpenApi},
    scalar::Scalar,
    swagger::Swagger,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Extension, Json, Router,
};
use serialization_api::v1::{NamespaceProofQueryData, ViewNumber};

use crate::r#trait::{NodeApi, NodeApiState};

/// Get the current view number
async fn get_view_number_http(
    State(state): State<NodeApiState>,
) -> Result<Json<ViewNumber>, StatusCode> {
    state
        .get_view_number()
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Query namespace proof and transactions by height and namespace ID
async fn get_namespace_proof_http(
    State(state): State<NodeApiState>,
    Path((height, namespace)): Path<(u64, u64)>,
) -> Result<Json<NamespaceProofQueryData>, StatusCode> {
    state
        .get_namespace_proof(height, namespace)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Serve the OpenAPI spec (extracted from Extension)
async fn serve_openapi_spec(Extension(api): Extension<OpenApi>) -> Json<OpenApi> {
    Json(api)
}

/// Create the Axum router with OpenAPI documentation
pub fn create_axum_router(state: NodeApiState) -> Router {
    let mut api = OpenApi {
        info: Info {
            title: "Espresso Sequencer API".to_string(),
            description: Some("HTTP/JSON API for Espresso Network".to_string()),
            version: "0.1.0".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    ApiRouter::new()
        .api_route(
            "/view-number",
            get_with(get_view_number_http, |op| {
                op.description("Get the current HotShot view number")
                    .tag("Status")
            }),
        )
        .api_route(
            "/namespace-proof/{height}/{namespace}",
            get_with(get_namespace_proof_http, |op| {
                op.description("Query namespace proof and transactions by height and namespace ID")
                    .tag("Data Availability")
            }),
        )
        .finish_api(&mut api)
        .route("/docs/openapi.json", get(serve_openapi_spec))
        .route(
            "/docs",
            get(Scalar::new("/docs/openapi.json")
                .with_title("Espresso API - Scalar")
                .axum_handler()),
        )
        .route(
            "/swagger-ui",
            get(Swagger::new("/docs/openapi.json")
                .with_title("Espresso API - Swagger")
                .axum_handler()),
        )
        .layer(Extension(api))
        .with_state(state)
}
