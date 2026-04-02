//! Espresso API server with both Axum (HTTP/JSON) and gRPC endpoints

use serialization_api::v1::{
    ViewNumber, GetViewNumberRequest,
    NamespaceProofQueryData, GetNamespaceProofRequest,
};
use tonic::{Request, Response, Status};

// Generated gRPC service code
pub mod proto {
    tonic::include_proto!("espresso.api.v1");
}

use proto::status_service_server::{StatusService, StatusServiceServer};

/// gRPC service implementation
#[derive(Default)]
pub struct StatusServiceImpl;

#[tonic::async_trait]
impl StatusService for StatusServiceImpl {
    async fn get_view_number(
        &self,
        _request: Request<GetViewNumberRequest>,
    ) -> Result<Response<ViewNumber>, Status> {
        // Return constant value of 1 for skeleton
        Ok(Response::new(ViewNumber { value: 1 }))
    }

    async fn get_namespace_proof(
        &self,
        _request: Request<GetNamespaceProofRequest>,
    ) -> Result<Response<NamespaceProofQueryData>, Status> {
        // Return constant None/empty vec for skeleton
        Ok(Response::new(NamespaceProofQueryData {
            proof: None,
            transactions: vec![],
        }))
    }
}

// Axum HTTP/JSON endpoints with OpenAPI documentation
use aide::{
    axum::{
        routing::{get, get_with},
        ApiRouter,
        IntoApiResponse,
    },
    openapi::{OpenApi, Info},
    scalar::Scalar,
    swagger::Swagger,
};
use axum::{Json, extract::Path, Extension, Router};

/// Get the current view number
async fn get_view_number_http() -> impl IntoApiResponse {
    Json(ViewNumber { value: 1 })
}

/// Query namespace proof and transactions by height and namespace ID
async fn get_namespace_proof_http(
    Path((height, namespace)): Path<(u64, u64)>,
) -> impl IntoApiResponse {
    let _ = (height, namespace);
    Json(NamespaceProofQueryData {
        proof: None,
        transactions: vec![],
    })
}

/// Serve the OpenAPI spec (extracted from Extension)
async fn serve_openapi_spec(Extension(api): Extension<OpenApi>) -> impl IntoApiResponse {
    Json(api)
}

pub fn create_axum_router() -> Router {
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
        .api_route("/view-number", get_with(get_view_number_http, |op| {
            op.description("Get the current HotShot view number")
                .tag("Status")
        }))
        .api_route("/namespace-proof/{height}/{namespace}", get_with(get_namespace_proof_http, |op| {
            op.description("Query namespace proof and transactions by height and namespace ID")
                .tag("Data Availability")
        }))
        .route("/docs/openapi.json", get(serve_openapi_spec))
        .route("/docs", get(Scalar::new("/docs/openapi.json").with_title("Espresso API - Scalar").axum_handler()))
        .route("/swagger-ui", get(Swagger::new("/docs/openapi.json").with_title("Espresso API - Swagger").axum_handler()))
        .finish_api(&mut api)
        .layer(Extension(api))
}

pub fn create_grpc_service() -> StatusServiceServer<StatusServiceImpl> {
    StatusServiceServer::new(StatusServiceImpl::default())
}

/// Start Axum HTTP server
pub async fn serve_axum(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting Axum server on port {}", port);
    let app = create_axum_router();
    let addr = format!("0.0.0.0:{}", port);

    tracing::info!("Binding to {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Axum API server listening on {}", addr);
    axum::serve(listener, app.into_make_service()).await?;

    tracing::info!("Axum server stopped");
    Ok(())
}

/// Start gRPC server
pub async fn serve_grpc(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let service = create_grpc_service();

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
