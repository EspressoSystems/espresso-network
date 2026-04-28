//! Espresso API server with both Axum (HTTP/JSON) and gRPC endpoints

// Module declarations
mod axum;
pub mod error;
pub mod handlers;
mod tonic;
pub mod v1;
pub mod v2;

// Generated gRPC service code - committed to git for visibility in code review
pub mod proto {
    include!("espresso.api.v2.rs");
}

// Re-exports
pub use self::{
    axum::{create_combined_router, create_router_v1, create_router_v2, routes},
    tonic::create_reward_service,
};

/// Start Axum HTTP server with combined v1 and v2 APIs
///
/// This serves both APIs at /v1/* and /v2/* from a single state implementation.
pub async fn serve_axum<S>(port: u16, state: S) -> anyhow::Result<()>
where
    S: v1::RewardApi
        + v1::AvailabilityApi
        + v2::RewardApi
        + v2::DataApi
        + v2::ConsensusApi
        + Clone
        + Send
        + Sync
        + 'static,
{
    tracing::info!("Starting Axum server on port {} with v1 and v2 APIs", port);

    let app = create_combined_router(state);
    let addr = format!("0.0.0.0:{}", port);

    tracing::info!("Binding to {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!(
        "Axum API server listening on {} (v1 and v2 routes available)",
        addr
    );
    ::axum::serve(listener, app.into_make_service()).await?;

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
