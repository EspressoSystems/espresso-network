//! Axum HTTP/JSON API handlers

pub mod routes;

use aide::{
    axum::{routing::get_with, ApiRouter},
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

use crate::r#trait::NodeApi;

/// Serve the OpenAPI spec (extracted from Extension)
async fn serve_openapi_spec(Extension(api): Extension<OpenApi>) -> Json<OpenApi> {
    Json(api)
}

/// Create the Axum router with OpenAPI documentation
pub fn create_axum_router<S>(state: S) -> Router
where
    S: NodeApi + Clone + Send + Sync + 'static,
{
    let mut api = OpenApi {
        info: Info {
            title: "Espresso Sequencer API".to_string(),
            description: Some("HTTP/JSON API for Espresso Network".to_string()),
            version: "0.1.0".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    // Create closures that capture the generic type
    let get_namespace_proof =
        |State(state): State<S>, Path((height, namespace)): Path<(u64, u64)>| async move {
            state
                .get_namespace_proof(height, namespace)
                .await
                .map(Json)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        };

    let get_reward_claim_input =
        |State(state): State<S>, Path((block_height, address)): Path<(u64, String)>| async move {
            state
                .get_reward_claim_input(block_height, address)
                .await
                .map(Json)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        };

    ApiRouter::new()
        .api_route(
            routes::NAMESPACE_PROOF_ROUTE,
            get_with(get_namespace_proof, |op| {
                op.description("Query namespace proof and transactions by height and namespace ID")
                    .tag("Data Availability")
            }),
        )
        .api_route(
            routes::REWARD_CLAIM_INPUT_ROUTE,
            get_with(get_reward_claim_input, |op| {
                op.description("Get reward claim input for L1 contract submission")
                    .tag("Rewards")
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
