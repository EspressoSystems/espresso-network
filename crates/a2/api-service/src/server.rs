//! One port for HTTP and gRPC: a request with a gRPC content type goes to the gRPC services, any
//! other to the REST routes.

use axum::{Router, extract::Request, http::header::CONTENT_TYPE, serve::ListenerExt};
use tokio::net::TcpListener;
use tonic::service::RoutesBuilder;

/// Serves `rest` and `grpc`, with gRPC reflection for the services of `espresso-api` and this
/// crate, on `listener` until it fails.
pub(crate) async fn serve(
    listener: TcpListener,
    rest: Router,
    mut grpc: RoutesBuilder,
) -> anyhow::Result<()> {
    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(espresso_api::FILE_DESCRIPTOR_SET)
        .build_v1()?;
    grpc.add_service(reflection);
    let grpc = grpc.routes().into_axum_router();
    let http = rest
        .layer(http_wire::body_limit_layer())
        .layer(http_wire::cors_layer());
    let app = tower::service_fn(move |request: Request| {
        let service = match is_grpc(&request) {
            true => grpc.clone(),
            false => http.clone(),
        };
        tower::ServiceExt::oneshot(service, request)
    });
    tracing::info!(addr = ?listener.local_addr()?, "serving the API");
    // Without TCP_NODELAY a small answer waits for the client's delayed acknowledgement.
    let listener = listener.tap_io(|stream| {
        if let Err(err) = stream.set_nodelay(true) {
            tracing::debug!(%err, "cannot set TCP_NODELAY");
        }
    });
    axum::serve(listener, axum::ServiceExt::into_make_service(app)).await?;
    Ok(())
}

fn is_grpc(request: &Request) -> bool {
    request
        .headers()
        .get(CONTENT_TYPE)
        .is_some_and(|value| value.as_bytes().starts_with(b"application/grpc"))
}
