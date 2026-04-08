use axum::Router;

use crate::{grpc::GrpcService, handlers, DataSource, RewardServiceServer, StatusServiceServer};

pub fn build_router<D: DataSource>(state: D) -> Router {
    let grpc_router: Router<()> =
        tonic::service::Routes::new(StatusServiceServer::new(GrpcService::new(state.clone())))
            .add_service(RewardServiceServer::new(GrpcService::new(state.clone())))
            .add_service(
                tonic_reflection::server::Builder::configure()
                    .register_encoded_file_descriptor_set(crate::REFLECTION_DESCRIPTOR)
                    .build_v1()
                    .expect("reflection service"),
            )
            .into_axum_router();

    let rest_router = handlers::rest_router(state);

    rest_router.merge(grpc_router)
}
