//! Espresso API server with both Axum (HTTP/JSON) and gRPC endpoints on a single port

mod axum;
mod grpc;
mod r#trait;

pub mod proto {
    tonic::include_proto!("espresso.api.v1");
}

pub use r#trait::{NodeApi, NodeApiState};

use self::{
    axum::create_axum_router,
    grpc::{create_reward_service, create_status_service},
};

const REFLECTION_DESCRIPTOR: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/reflection_descriptor.bin"));

pub fn build_v2_router<S>(state: S) -> ::axum::Router
where
    S: NodeApi + Clone + Send + Sync + 'static,
{
    let rest_router = create_axum_router(state.clone());

    let status_svc = create_status_service(state.clone());
    let reward_svc = create_reward_service(state);
    let reflection_svc = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(REFLECTION_DESCRIPTOR)
        .build_v1()
        .expect("failed to build gRPC reflection service");

    let grpc_router = tonic::service::Routes::new(status_svc)
        .add_service(reward_svc)
        .add_service(reflection_svc)
        .into_axum_router();

    rest_router.merge(grpc_router)
}

pub async fn serve(port: u16, router: ::axum::Router) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("0.0.0.0:{port}");
    tracing::info!("API v2 server listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    ::axum::serve(listener, router.into_make_service()).await?;
    Ok(())
}
