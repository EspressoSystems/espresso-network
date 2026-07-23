use super::*;

async fn hotshot_config<S: v1::ConfigApi>(
    State(state): State<S>,
) -> Result<ApiJson<S::HotShotConfig>, ApiError> {
    state
        .hotshot_config()
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn env<S: v1::ConfigApi>(State(state): State<S>) -> Result<ApiJson<Vec<String>>, ApiError> {
    state.env().await.map(ApiJson).map_err(ApiError::Internal)
}

async fn runtime_config<S: v1::ConfigApi>(
    State(state): State<S>,
) -> Result<ApiJson<S::RuntimeConfig>, ApiError> {
    state
        .runtime_config()
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

pub(crate) fn router_config<S>(state: S) -> ApiRouter
where
    S: v1::ConfigApi + Clone + Send + Sync + 'static,
{
    let config = ApiRouter::new()
        .api_route(
            "/hotshot",
            get_with(hotshot_config::<S>, |op| {
                op.summary("Get HotShot config")
                    .description("Get the HotShot configuration for the current node.")
            }),
        )
        .api_route(
            "/env",
            get_with(env::<S>, |op| {
                op.summary("Get environment variables").description(
                    "Get all ESPRESSO_ environment variables set for the current node.",
                )
            }),
        )
        .api_route(
            "/runtime",
            get_with(runtime_config::<S>, |op| {
                op.summary("Get runtime config").description(
                    "Get the merged runtime configuration (CLI flags + env vars + defaults); \
                     secrets and L1 RPC URLs are redacted.",
                )
            }),
        );

    ApiRouter::new().nest("/config", config).with_state(state)
}
