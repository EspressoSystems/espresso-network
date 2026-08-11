//! The `availability` and `node` modules this service serves, both from
//! `hotshot-query-service`'s own routers, plus the paths tide-disco exposed around them.

use axum::{Router, http::HeaderMap, routing::get};
use espresso_node::api::sql::DataSource;
use espresso_types::SeqTypes;
use hotshot_query_service::{
    availability::{self, router::availability_router},
    node::{self, router::node_router},
};
use http_wire::{cors_layer, healthcheck_response};

/// Builds the full router: `healthcheck`, plus the `availability` and `node` modules served both
/// unversioned and under `/v1`, matching the paths tide-disco exposed for this service (which
/// only ever registered API version `1.0.0`).
pub fn router(ds: DataSource) -> Router {
    // This service serves no OpenAPI spec, so both mounts drop the routers' documentation.
    let api = Router::new()
        .nest(
            "/availability",
            Router::from(availability_router::<SeqTypes, DataSource>(
                &availability::Options::default(),
                ds.clone(),
            )),
        )
        .nest(
            "/node",
            Router::from(node_router::<SeqTypes, DataSource>(
                &node::Options::default(),
                ds,
            )),
        );
    Router::new()
        .route(
            "/healthcheck",
            get(|headers: HeaderMap| async move { healthcheck_response(&headers) }),
        )
        .merge(api.clone())
        .nest("/v1", api)
        .layer(cors_layer())
}
