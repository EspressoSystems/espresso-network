use super::*;

async fn startup_info<S: v1::HotShotEventsApi>(
    State(state): State<S>,
) -> Result<ApiJson<S::StartupInfo>, ApiError> {
    state
        .startup_info()
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn events<S: v1::HotShotEventsApi + Send + Sync + 'static>(
    State(state): State<S>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let format = ws_format(&headers);
    match state.events().await {
        Ok(stream) => ws
            .on_upgrade(move |socket| async move { drive_ws_stream(socket, stream, format).await }),
        Err(err) => ApiError::Internal(err).into_response(),
    }
}

pub(crate) fn router_hotshot_events<S>(state: S) -> ApiRouter
where
    S: v1::HotShotEventsApi + Clone + Send + Sync + 'static,
{
    let hotshot_events = ApiRouter::new()
        .api_route(
            "/startup_info",
            get_with(startup_info::<S>, |op| {
                op.summary("Get startup info").description(
                    "Get startup info: known nodes with stake and their public keys, and the \
                     count of non-staked nodes.",
                )
            }),
        )
        .api_route(
            "/events",
            get_with(events::<S>, |op| {
                op.summary("Stream HotShot events (websocket)")
                    .description("Websocket endpoint: get legacy HotShot events starting now.")
            }),
        );

    ApiRouter::new()
        .nest("/hotshot-events", hotshot_events)
        .with_state(state)
}
