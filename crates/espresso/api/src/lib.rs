//! Espresso API server with both Axum (HTTP/JSON) and gRPC endpoints

// Module declarations
mod axum;
mod grpc;
mod r#trait;

// Generated gRPC service code
pub mod proto {
    tonic::include_proto!("espresso.api.v1");
}

// Re-exports
pub use r#trait::{NodeApi, NodeApiState};

pub use self::{axum::create_axum_router, grpc::create_grpc_service};

/// Start Axum HTTP server
pub async fn serve_axum(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting Axum server on port {}", port);

    let state = NodeApiState::default();
    let app = create_axum_router(state);
    let addr = format!("0.0.0.0:{}", port);

    tracing::info!("Binding to {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Axum API server listening on {}", addr);
    ::axum::serve(listener, app.into_make_service()).await?;

    tracing::info!("Axum server stopped");
    Ok(())
}

/// Start Tonic gRPC server
pub async fn serve_tonic(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));

    let state = NodeApiState::default();
    let service = create_grpc_service(state);

    // Enable gRPC reflection for tools like grpcurl
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(include_bytes!(concat!(
            env!("OUT_DIR"),
            "/reflection_descriptor.bin"
        )))
        .build_v1()?;

    tracing::info!("gRPC server listening on {}", addr);
    tonic::transport::Server::builder()
        .add_service(service)
        .add_service(reflection_service)
        .serve(addr)
        .await?;

    Ok(())
}
