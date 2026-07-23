use super::*;

async fn get_reward_claim_input<S: v1::RewardApi>(
    State(state): State<S>,
    Path((height, address)): Path<(u64, String)>,
) -> Result<ApiJson<S::RewardClaimInput>, ApiError> {
    state
        .get_reward_claim_input(height, address)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_reward_balance<S: v1::RewardApi>(
    State(state): State<S>,
    Path((height, address)): Path<(u64, String)>,
) -> Result<ApiJson<S::RewardBalance>, ApiError> {
    state
        .get_reward_balance(height, address)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_latest_reward_balance<S: v1::RewardApi>(
    State(state): State<S>,
    Path(address): Path<String>,
) -> Result<ApiJson<S::RewardBalance>, ApiError> {
    state
        .get_latest_reward_balance(address)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_reward_account_proof<S: v1::RewardApi>(
    State(state): State<S>,
    Path((height, address)): Path<(u64, String)>,
) -> Result<ApiJson<S::RewardAccountQueryData>, ApiError> {
    state
        .get_reward_account_proof(height, address)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_reward_account_proof_v1<S: v1::RewardApi>(
    State(state): State<S>,
    Path((height, address)): Path<(u64, String)>,
) -> Result<ApiJson<S::RewardAccountQueryDataV1>, ApiError> {
    state
        .get_reward_account_proof_v1(height, address)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_latest_reward_account_proof<S: v1::RewardApi>(
    State(state): State<S>,
    Path(address): Path<String>,
) -> Result<ApiJson<S::RewardAccountQueryData>, ApiError> {
    state
        .get_latest_reward_account_proof(address)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_reward_amounts<S: v1::RewardApi>(
    State(state): State<S>,
    Path((height, offset, limit)): Path<(u64, u64, u64)>,
) -> Result<ApiJson<S::RewardAmounts>, ApiError> {
    state
        .get_reward_amounts(height, offset, limit)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_reward_merkle_tree_v2<S: v1::RewardApi>(
    State(state): State<S>,
    Path(height): Path<u64>,
) -> Result<ApiJson<S::RewardMerkleTreeData>, ApiError> {
    state
        .get_reward_merkle_tree_v2(height)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_reward_state_height<S: v1::RewardApi>(
    State(state): State<S>,
) -> Result<ApiJson<u64>, ApiError> {
    state
        .get_reward_state_height()
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_reward_state_v2_height<S: v1::RewardApi>(
    State(state): State<S>,
) -> Result<ApiJson<u64>, ApiError> {
    state
        .get_reward_state_v2_height()
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_reward_state_path_v1_by_height<S: v1::RewardApi>(
    State(state): State<S>,
    Path((height, key)): Path<(u64, String)>,
) -> Result<ApiJson<S::RewardStatePathV1>, ApiError> {
    state
        .get_reward_state_path_v1(v1::Snapshot::Height(height), key)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_reward_state_path_v1_by_commit<S: v1::RewardApi>(
    State(state): State<S>,
    Path((commit, key)): Path<(String, String)>,
) -> Result<ApiJson<S::RewardStatePathV1>, ApiError> {
    state
        .get_reward_state_path_v1(v1::Snapshot::Commit(commit), key)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_reward_state_path_v2_by_height<S: v1::RewardApi>(
    State(state): State<S>,
    Path((height, key)): Path<(u64, String)>,
) -> Result<ApiJson<S::RewardStatePathV2>, ApiError> {
    state
        .get_reward_state_path_v2(v1::Snapshot::Height(height), key)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_reward_state_path_v2_by_commit<S: v1::RewardApi>(
    State(state): State<S>,
    Path((commit, key)): Path<(String, String)>,
) -> Result<ApiJson<S::RewardStatePathV2>, ApiError> {
    state
        .get_reward_state_path_v2(v1::Snapshot::Commit(commit), key)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

pub(crate) fn router_reward<S>(state: S) -> ApiRouter
where
    S: v1::RewardApi + Clone + Send + Sync + 'static,
{
    // `/reward-state-v2` is the primary merklized-reward mount.
    let reward_state_v2 = ApiRouter::new()
        .api_route(
            "/reward-claim-input/{height}/{address}",
            get_with(get_reward_claim_input::<S>, |op| {
                op.summary("Get reward claim input").description(
                    "Returns the RewardClaimInput needed to call claimRewards() on L1: lifetime \
                     rewards, Merkle proof, and auth root inputs, for the account at the given \
                     block height finalized by the light client contract.",
                )
            }),
        )
        .api_route(
            "/reward-balance/{height}/{address}",
            get_with(get_reward_balance::<S>, |op| {
                op.summary("Get reward balance at height").description(
                    "Get balance in reward state at a specific height for an Ethereum address.",
                )
            }),
        )
        .api_route(
            "/reward-balance/latest/{address}",
            get_with(get_latest_reward_balance::<S>, |op| {
                op.summary("Get latest reward balance")
                    .description("Get current balance in reward state for an Ethereum address.")
            }),
        )
        .api_route(
            "/proof/{height}/{address}",
            get_with(get_reward_account_proof::<S>, |op| {
                op.summary("Get reward account proof").description(
                    "Get the Merkle proof for a reward account at a given block height \
                     (RewardAccountProofV1 pre-V4, RewardAccountProofV2 from V4 onward).",
                )
            }),
        )
        .api_route(
            "/proof/latest/{address}",
            get_with(get_latest_reward_account_proof::<S>, |op| {
                op.summary("Get latest reward account proof").description(
                    "Get the Merkle proof (RewardAccountProofV2) for a reward account at the \
                     latest block height finalized by the light client contract.",
                )
            }),
        )
        .api_route(
            "/reward-amounts/{height}/{offset}/{limit}",
            get_with(get_reward_amounts::<S>, |op| {
                op.summary("List reward amounts").description(
                    "Return all RewardMerkleTreeV2 accounts stored for the requested height, \
                     paginated by offset and limit (limit must be <= 10000).",
                )
            }),
        )
        .api_route(
            "/reward-merkle-tree-v2/{height}",
            get_with(get_reward_merkle_tree_v2::<S>, |op| {
                op.summary("Get RewardMerkleTreeV2 snapshot").description(
                    "Get the snapshot of this node's RewardMerkleTreeV2 at the given block \
                     height, serialized as RewardMerkleTreeV2Data.",
                )
            }),
        )
        .api_route(
            "/block-height",
            get_with(get_reward_state_v2_height::<S>, |op| {
                op.summary("Get reward-state-v2 block height").description(
                    "Latest block height for which the merklized reward state (V2) is available.",
                )
            }),
        )
        .api_route(
            "/{height}/{key}",
            get_with(get_reward_state_path_v2_by_height::<S>, |op| {
                op.summary("Get reward-state-v2 Merkle path by height")
                    .description(
                        "Retrieve the Merkle path for the membership proof of a leaf in the \
                         reward-state-v2 tree, by block height and key.",
                    )
            }),
        )
        .api_route(
            "/commit/{commit}/{key}",
            get_with(get_reward_state_path_v2_by_commit::<S>, |op| {
                op.summary("Get reward-state-v2 Merkle path by commitment")
                    .description(
                        "Retrieve the Merkle path for the membership proof of a leaf in the \
                         reward-state-v2 tree, by tree commitment and key.",
                    )
            }),
        );

    // `/reward-state` mirrors the V2 mount: tide-disco shared these handlers across both
    // merklized-state modules, so the same routes are served under this prefix too. The reused
    // handlers differ only in their doc summary (the "(v1 mount)" variants).
    let reward_state = ApiRouter::new()
        .api_route(
            "/block-height",
            get_with(get_reward_state_height::<S>, |op| {
                op.summary("Get reward-state block height").description(
                    "Latest block height for which the merklized reward state (V1) is available.",
                )
            }),
        )
        .api_route(
            "/reward-balance/{height}/{address}",
            get_with(get_reward_balance::<S>, |op| {
                op.summary("Get reward balance at height (v1 mount)")
                    .description(
                        "Same handler as reward-state-v2/reward-balance, registered on the \
                         reward-state mount; tide-disco shared this handler across both \
                         merklized-state mounts.",
                    )
            }),
        )
        .api_route(
            "/proof/{height}/{address}",
            get_with(get_reward_account_proof_v1::<S>, |op| {
                op.summary("Get reward account proof (v1 mount)")
                    .description(
                        "Same handler as reward-state-v2/proof, registered on the reward-state \
                         mount; tide-disco shared this handler across both merklized-state mounts.",
                    )
            }),
        )
        .api_route(
            "/reward-balance/latest/{address}",
            get_with(get_latest_reward_balance::<S>, |op| {
                op.summary("Get latest reward balance (v1 mount)")
                    .description(
                        "Same handler as reward-state-v2/reward-balance/latest, registered on the \
                         reward-state mount; tide-disco shared this handler across both \
                         merklized-state mounts.",
                    )
            }),
        )
        .api_route(
            "/proof/latest/{address}",
            get_with(get_latest_reward_account_proof::<S>, |op| {
                op.summary("Get latest reward account proof (v1 mount)")
                    .description(
                        "Same handler as reward-state-v2/proof/latest, registered on the \
                         reward-state mount; tide-disco shared this handler across both \
                         merklized-state mounts.",
                    )
            }),
        )
        .api_route(
            "/reward-amounts/{height}/{offset}/{limit}",
            get_with(get_reward_amounts::<S>, |op| {
                op.summary("List reward amounts (v1 mount)").description(
                    "Same handler as reward-state-v2/reward-amounts, registered on the \
                     reward-state mount; tide-disco shared this handler across both \
                     merklized-state mounts.",
                )
            }),
        )
        .api_route(
            "/reward-merkle-tree-v2/{height}",
            get_with(get_reward_merkle_tree_v2::<S>, |op| {
                op.summary("Get RewardMerkleTreeV2 snapshot (v1 mount)")
                    .description(
                        "Same handler as reward-state-v2/reward-merkle-tree-v2, registered on the \
                         reward-state mount; tide-disco shared this handler across both \
                         merklized-state mounts.",
                    )
            }),
        )
        .api_route(
            "/{height}/{key}",
            get_with(get_reward_state_path_v1_by_height::<S>, |op| {
                op.summary("Get reward-state Merkle path by height")
                    .description(
                        "Retrieve the Merkle path for the membership proof of a leaf in the \
                         reward-state (V1) tree, by block height and key.",
                    )
            }),
        )
        .api_route(
            "/commit/{commit}/{key}",
            get_with(get_reward_state_path_v1_by_commit::<S>, |op| {
                op.summary("Get reward-state Merkle path by commitment")
                    .description(
                        "Retrieve the Merkle path for the membership proof of a leaf in the \
                         reward-state (V1) tree, by tree commitment and key.",
                    )
            }),
        );

    ApiRouter::new()
        .nest("/reward-state-v2", reward_state_v2)
        .nest("/reward-state", reward_state)
        .with_state(state)
}
