//! Espresso API server with both Axum (HTTP/JSON) and gRPC endpoints

// Module declarations
mod axum;
pub mod error;
pub mod handlers;
mod tonic;
pub mod v1;
pub mod v2;

use tower::Layer;

// Generated gRPC service code - committed to git for visibility in code review
pub mod proto {
    include!("espresso.api.v2.rs");
}

// Re-exports
pub use self::{
    axum::{
        create_combined_router, create_router_v1, create_router_v2, healthcheck_response, routes,
    },
    tonic::create_reward_service,
};

/// Build a full request URL from a server base URL and a path produced by one of the
/// `routes::v1::*` (or `routes::v2::*`) builders.
///
/// Use this from test/CLI sites that have a `url::Url` pointing at the API server and want
/// the absolute URL for a single request. Internally this is just `base.join(path)`; the
/// helper exists so the (path-const, builder, joiner) trio reads as one chain.
pub fn url(base: &::url::Url, path: impl AsRef<str>) -> ::url::Url {
    base.join(path.as_ref())
        .expect("path produced by routes::*::path_fn is always a valid relative URL")
}

/// Start Axum HTTP server with combined v1 and v2 APIs
///
/// This serves both APIs at /v1/* and /v2/* from a single state implementation.
///
/// `catchup`, like the query-service modules (`status`, `availability`, `node`, `token`,
/// `block-state`, `fee-state`, `reward-state`, `database`) and `v2`, is always on: tide-disco's
/// SQL mode registered it unconditionally. `submit`, `config`, `explorer`, `light-client`, and
/// `hotshot-events` follow `Options`, matching `Options::init_with_query_module_sql`.
pub async fn serve_axum<S>(
    port: u16,
    state: S,
    modules: OptionalModules,
    max_connections: Option<usize>,
) -> anyhow::Result<()>
where
    S: v1::RewardApi
        + v1::AvailabilityApi
        + v1::HotShotAvailabilityApi
        + v1::BlockStateApi
        + v1::FeeStateApi
        + v1::StatusApi
        + v1::ConfigApi
        + v1::NodeApi
        + v1::CatchupApi
        + v1::SubmitApi
        + v1::StateSignatureApi
        + v1::HotShotEventsApi
        + v1::LightClientApi
        + v1::ExplorerApi
        + v1::TokenApi
        + v1::DatabaseApi
        + v2::RewardApi
        + v2::DataApi
        + v2::ConsensusApi
        + Clone
        + Send
        + Sync
        + 'static,
{
    let mut router = axum::router_reward(state.clone())
        .merge(axum::router_availability(state.clone()))
        .merge(axum::router_block_state(state.clone()))
        .merge(axum::router_fee_state(state.clone()))
        .merge(axum::router_status(state.clone()))
        .merge(axum::router_node(state.clone()))
        .merge(axum::router_catchup(state.clone()))
        .merge(axum::router_state_signature(state.clone()))
        .merge(axum::router_token(state.clone()))
        .merge(axum::router_database(state.clone()))
        .merge(axum::create_router_v2(state.clone()));
    if modules.submit {
        router = router.merge(axum::router_submit(state.clone()));
    }
    if modules.config {
        router = router.merge(axum::router_config(state.clone()));
    }
    if modules.explorer {
        router = router.merge(axum::router_explorer(state.clone()));
    }
    if modules.light_client {
        router = router.merge(axum::router_light_client(state.clone()));
    }
    if modules.hotshot_events {
        router = router.merge(axum::router_hotshot_events(state));
    }
    serve_router(port, "v1 and v2", router, max_connections).await
}

/// Which of the optional API modules to serve, for modes that make them conditional
/// (mirroring `Options::submit`/`Options::config`/`Options::explorer`/`Options::light_client`/
/// `Options::hotshot_events`).
#[derive(Default, Clone, Copy, Debug)]
pub struct OptionalModules {
    pub submit: bool,
    pub catchup: bool,
    pub config: bool,
    pub hotshot_events: bool,
    pub explorer: bool,
    pub light_client: bool,
}

/// Serve the query API used by the filesystem-backed storage mode: status, availability, node,
/// token, catchup, and state-signature are always on (tide registered them unconditionally);
/// submit, config, and hotshot-events follow `Options`. Filesystem storage doesn't implement the
/// reward/merklized-state/explorer/database traits, so those modules aren't served (a request to
/// one of their routes 404s, matching tide).
pub async fn serve_axum_fs<S>(
    port: u16,
    state: S,
    modules: OptionalModules,
    max_connections: Option<usize>,
) -> anyhow::Result<()>
where
    S: v1::StatusApi
        + v1::AvailabilityApi
        + v1::HotShotAvailabilityApi
        + v1::NodeApi
        + v1::TokenApi
        + v1::CatchupApi
        + v1::SubmitApi
        + v1::StateSignatureApi
        + v1::ConfigApi
        + v1::HotShotEventsApi
        + Clone
        + Send
        + Sync
        + 'static,
{
    let mut router = axum::router_status(state.clone())
        .merge(axum::router_availability(state.clone()))
        .merge(axum::router_node(state.clone()))
        .merge(axum::router_token(state.clone()))
        .merge(axum::router_catchup(state.clone()))
        .merge(axum::router_state_signature(state.clone()));
    if modules.submit {
        router = router.merge(axum::router_submit(state.clone()));
    }
    if modules.config {
        router = router.merge(axum::router_config(state.clone()));
    }
    if modules.hotshot_events {
        router = router.merge(axum::router_hotshot_events(state));
    }
    serve_router(port, "fs", router, max_connections).await
}

/// Serve the status-only API: no availability/node/token data source is available, so only
/// status and the HotShot modules (submit, catchup, state-signature, config, hotshot-events) can
/// be served. State-signature is always on; the rest follow `Options`.
pub async fn serve_axum_status<S>(
    port: u16,
    state: S,
    modules: OptionalModules,
    max_connections: Option<usize>,
) -> anyhow::Result<()>
where
    S: v1::StatusApi
        + v1::SubmitApi
        + v1::CatchupApi
        + v1::StateSignatureApi
        + v1::ConfigApi
        + v1::HotShotEventsApi
        + Clone
        + Send
        + Sync
        + 'static,
{
    let mut router =
        axum::router_status(state.clone()).merge(axum::router_state_signature(state.clone()));
    router = merge_hotshot_modules(router, &state, modules);
    serve_router(port, "status", router, max_connections).await
}

/// Serve the bare API (no query or status module): only the HotShot modules are available,
/// since the only app state is the HotShot handle. State-signature is always on; the rest follow
/// `Options`, matching `Options::init_hotshot_modules`.
pub async fn serve_axum_bare<S>(
    port: u16,
    state: S,
    modules: OptionalModules,
    max_connections: Option<usize>,
) -> anyhow::Result<()>
where
    S: v1::SubmitApi
        + v1::CatchupApi
        + v1::StateSignatureApi
        + v1::ConfigApi
        + v1::HotShotEventsApi
        + Clone
        + Send
        + Sync
        + 'static,
{
    let router = axum::router_state_signature(state.clone());
    let router = merge_hotshot_modules(router, &state, modules);
    serve_router(port, "bare", router, max_connections).await
}

fn merge_hotshot_modules<S>(
    mut router: ::axum::Router,
    state: &S,
    modules: OptionalModules,
) -> ::axum::Router
where
    S: v1::SubmitApi
        + v1::CatchupApi
        + v1::ConfigApi
        + v1::HotShotEventsApi
        + Clone
        + Send
        + Sync
        + 'static,
{
    if modules.submit {
        router = router.merge(axum::router_submit(state.clone()));
    }
    if modules.catchup {
        router = router.merge(axum::router_catchup(state.clone()));
    }
    if modules.config {
        router = router.merge(axum::router_config(state.clone()));
    }
    if modules.hotshot_events {
        router = router.merge(axum::router_hotshot_events(state.clone()));
    }
    router
}

/// Add the reserved top-level routes, apply the optional concurrency limit, rewrite legacy URIs,
/// and bind/serve the router. Shared by all `serve_axum*` entry points.
///
/// `max_connections` matches tide-disco's `RateLimitListener` semantics: at most that many
/// requests are in flight at once, and excess requests fail immediately with 429 Too Many
/// Requests instead of queueing. The load-shed layer converts the concurrency limit's "not
/// ready" into an error, which `HandleErrorLayer` maps to the 429 response.
async fn serve_router(
    port: u16,
    mode: &str,
    router: ::axum::Router,
    max_connections: Option<usize>,
) -> anyhow::Result<()> {
    tracing::info!("Starting Axum server on port {} ({} mode)", port, mode);

    let mut router = axum::with_top_level_routes(router);
    if let Some(limit) = max_connections {
        router = router.layer(
            tower::ServiceBuilder::new()
                .layer(::axum::error_handling::HandleErrorLayer::new(
                    |_: tower::BoxError| async { ::axum::http::StatusCode::TOO_MANY_REQUESTS },
                ))
                .layer(tower::load_shed::LoadShedLayer::new())
                .layer(tower::limit::GlobalConcurrencyLimitLayer::new(limit)),
        );
    }
    // `Router::layer` middleware runs after routing, so it can't rewrite a URI to match a
    // different route. Wrapping the whole router with `MapRequestLayer` instead runs the
    // rewrite before routing, per the axum-documented pattern for this case.
    let router = tower::util::MapRequestLayer::new(axum::rewrite_legacy_uri).layer(router);
    let addr = format!("0.0.0.0:{}", port);

    tracing::info!("Binding to {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Axum API server listening on {} ({} mode)", addr, mode);
    ::axum::serve(listener, ::axum::ServiceExt::into_make_service(router)).await?;

    tracing::info!("Axum server stopped");
    Ok(())
}

/// Start Tonic gRPC server
pub async fn serve_tonic<S>(port: u16, state: S) -> anyhow::Result<()>
where
    S: v2::RewardApi + Clone + Send + Sync + 'static,
{
    use ::tonic::transport::Server;

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));

    let reward_service = create_reward_service(state);

    // Enable gRPC reflection for tools like grpcurl
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/reflection_descriptor.bin"
        )))
        .build_v1()?;

    tracing::info!("gRPC server listening on {}", addr);
    Server::builder()
        .add_service(reward_service)
        .add_service(reflection_service)
        .serve(addr)
        .await?;

    Ok(())
}
