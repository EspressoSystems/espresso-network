use super::*;

// Body is decoded as VBS (binary) or JSON based on Content-Type, matching tide-disco's
// `body_auto`.
async fn submit<S: v1::SubmitApi>(
    State(state): State<S>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let tx: S::Transaction = decode_body(&headers, &body)?;
    let hash = state.submit(tx).await.map_err(ApiError::Internal)?;
    encode_response(&headers, hash)
}

pub(crate) fn router_submit<S>(state: S) -> ApiRouter
where
    S: v1::SubmitApi + Clone + Send + Sync + 'static,
{
    let submit = ApiRouter::new().api_route(
        "/submit",
        post_with(submit::<S>, |op| {
            op.summary("Submit transaction")
                .description("Submit a transaction to the HotShot handle for sequencing.")
        }),
    );

    ApiRouter::new().nest("/submit", submit).with_state(state)
}
