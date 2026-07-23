use super::*;

async fn block_height<S: v1::StatusApi>(State(state): State<S>) -> Result<ApiJson<u64>, ApiError> {
    state
        .block_height()
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn success_rate<S: v1::StatusApi>(State(state): State<S>) -> Result<ApiJson<f64>, ApiError> {
    state
        .success_rate()
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn time_since_last_decide<S: v1::StatusApi>(
    State(state): State<S>,
) -> Result<ApiJson<u64>, ApiError> {
    state
        .time_since_last_decide()
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

// Returns the Prometheus text exposition format directly, not JSON.
async fn metrics<S: v1::StatusApi>(State(state): State<S>) -> Response {
    match state.metrics().await {
        Ok(text) => (
            [(
                axum::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            )],
            text,
        )
            .into_response(),
        Err(e) => ApiError::Internal(e).into_response(),
    }
}

pub(crate) fn router_status<S>(state: S) -> ApiRouter
where
    S: v1::StatusApi + Clone + Send + Sync + 'static,
{
    let status = ApiRouter::new()
        .api_route(
            "/block-height",
            get_with(block_height::<S>, |op| {
                op.summary("Get latest committed block height")
                    .description("Get the height of the latest committed block.")
            }),
        )
        .api_route(
            "/success-rate",
            get_with(success_rate::<S>, |op| {
                op.summary("Get view success rate")
                    .description("Get the fraction of views which resulted in a committed block.")
            }),
        )
        .api_route(
            "/time-since-last-decide",
            get_with(time_since_last_decide::<S>, |op| {
                op.summary("Get time since last decide")
                    .description("Get the time elapsed in seconds since the last decided view.")
            }),
        )
        .api_route(
            "/metrics",
            get_with(metrics::<S>, |op| {
                op.summary("Get Prometheus metrics")
                    .description("Prometheus endpoint exposing consensus-related metrics.")
            }),
        );

    ApiRouter::new().nest("/status", status).with_state(state)
}
