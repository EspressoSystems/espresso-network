//! Espresso API crate: handlers, gRPC services, router, and serve utility.

pub mod data_source;
mod grpc;
pub mod handlers;
mod router;

pub mod proto {
    tonic::include_proto!("espresso.api.v1");
}

pub use data_source::{DataSource, DataSourceError, RewardClaimData};
pub use proto::{
    reward_service_server::{RewardService, RewardServiceServer},
    status_service_server::{StatusService, StatusServiceServer},
};
pub use router::build_router;

pub const REFLECTION_DESCRIPTOR: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/reflection_descriptor.bin"));

pub async fn serve(port: u16, router: axum::Router) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("API v2 server listening on port {port}");
    axum::serve(listener, router.into_make_service()).await?;
    Ok(())
}
