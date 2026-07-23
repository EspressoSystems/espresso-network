use super::*;

async fn get_fee_state_height<S: v1::FeeStateApi>(
    State(state): State<S>,
) -> Result<ApiJson<u64>, ApiError> {
    state
        .get_fee_state_height()
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_fee_balance_latest<S: v1::FeeStateApi>(
    State(state): State<S>,
    Path(address): Path<String>,
) -> Result<ApiJson<Option<S::FeeAmount>>, ApiError> {
    state
        .get_fee_balance_latest(address)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_fee_state_path_by_commit<S: v1::FeeStateApi>(
    State(state): State<S>,
    Path((commit, key)): Path<(String, String)>,
) -> Result<ApiJson<S::MerkleProof>, ApiError> {
    state
        .get_fee_state_path(v1::Snapshot::Commit(commit), key)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_fee_state_path_by_height<S: v1::FeeStateApi>(
    State(state): State<S>,
    Path((height, key)): Path<(u64, String)>,
) -> Result<ApiJson<S::MerkleProof>, ApiError> {
    state
        .get_fee_state_path(v1::Snapshot::Height(height), key)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

pub(crate) fn router_fee_state<S>(state: S) -> ApiRouter
where
    S: v1::FeeStateApi + Clone + Send + Sync + 'static,
{
    let fee_state = ApiRouter::new()
        .api_route(
            "/block-height",
            get_with(get_fee_state_height::<S>, |op| {
                op.summary("Get fee-state height").description(
                    "Latest block height for which the merklized fee state is available.",
                )
            }),
        )
        .api_route(
            "/fee-balance/latest/{address}",
            get_with(get_fee_balance_latest::<S>, |op| {
                op.summary("Get latest fee balance").description(
                    "Get the latest fee account balance for an address from the fee Merkle tree.",
                )
            }),
        )
        .api_route(
            "/commit/{commit}/{key}",
            get_with(get_fee_state_path_by_commit::<S>, |op| {
                op.summary("Get fee-state Merkle path by commitment")
                    .description(
                        "Retrieve the Merkle path for a leaf in the fee state tree, by tree \
                         commitment and key.",
                    )
            }),
        )
        .api_route(
            "/{height}/{key}",
            get_with(get_fee_state_path_by_height::<S>, |op| {
                op.summary("Get fee-state Merkle path by height")
                    .description(
                        "Retrieve the Merkle path for a leaf in the fee state tree, by block \
                         height and key.",
                    )
            }),
        );

    ApiRouter::new()
        .nest("/fee-state", fee_state)
        .with_state(state)
}
