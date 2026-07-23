use super::*;

async fn get_state_signature<S: v1::StateSignatureApi>(
    State(state): State<S>,
    Path(height): Path<u64>,
) -> Result<ApiJson<S::Signature>, ApiError> {
    state
        .get_state_signature(height)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

pub(crate) fn router_state_signature<S>(state: S) -> ApiRouter
where
    S: v1::StateSignatureApi + Clone + Send + Sync + 'static,
{
    let state_signature = ApiRouter::new().api_route(
        "/block/{height}",
        get_with(get_state_signature::<S>, |op| {
            op.summary("Get light client state signature").description(
                "Get this node's signature for the light client state at the given block height.",
            )
        }),
    );

    ApiRouter::new()
        .nest("/state-signature", state_signature)
        .with_state(state)
}
