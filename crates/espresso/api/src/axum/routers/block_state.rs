use super::*;

async fn get_block_state_height<S: v1::BlockStateApi>(
    State(state): State<S>,
) -> Result<ApiJson<u64>, ApiError> {
    state
        .get_block_state_height()
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_block_state_path_by_commit<S: v1::BlockStateApi>(
    State(state): State<S>,
    Path((commit, key)): Path<(String, String)>,
) -> Result<ApiJson<S::MerkleProof>, ApiError> {
    state
        .get_block_state_path(v1::Snapshot::Commit(commit), key)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_block_state_path_by_height<S: v1::BlockStateApi>(
    State(state): State<S>,
    Path((height, key)): Path<(u64, String)>,
) -> Result<ApiJson<S::MerkleProof>, ApiError> {
    state
        .get_block_state_path(v1::Snapshot::Height(height), key)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

pub(crate) fn router_block_state<S>(state: S) -> ApiRouter
where
    S: v1::BlockStateApi + Clone + Send + Sync + 'static,
{
    let block_state = ApiRouter::new()
        .api_route(
            "/block-height",
            get_with(get_block_state_height::<S>, |op| {
                op.summary("Get block-state height").description(
                    "Latest block height for which the merklized blocks-Merkle-tree state is \
                     available.",
                )
            }),
        )
        .api_route(
            "/commit/{commit}/{key}",
            get_with(get_block_state_path_by_commit::<S>, |op| {
                op.summary("Get block-state Merkle path by commitment")
                    .description(
                        "Retrieve the Merkle path for a leaf in the blocks Merkle tree, by tree \
                         commitment and key.",
                    )
            }),
        )
        .api_route(
            "/{height}/{key}",
            get_with(get_block_state_path_by_height::<S>, |op| {
                op.summary("Get block-state Merkle path by height")
                    .description(
                        "Retrieve the Merkle path for a leaf in the blocks Merkle tree, by block \
                         height and key.",
                    )
            }),
        );

    ApiRouter::new()
        .nest("/block-state", block_state)
        .with_state(state)
}
