use super::*;

async fn get_account<S: v1::CatchupApi>(
    State(state): State<S>,
    Path((height, view, address)): Path<(u64, u64, String)>,
) -> Result<ApiJson<S::AccountQueryData>, ApiError> {
    state
        .get_account(height, view, address)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_accounts<S: v1::CatchupApi>(
    State(state): State<S>,
    Path((height, view)): Path<(u64, u64)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let accounts: Vec<S::FeeAccount> = decode_body(&headers, &body)?;
    let tree = state
        .get_accounts(height, view, accounts)
        .await
        .map_err(classify_availability_error)?;
    encode_response(&headers, tree)
}

async fn get_blocks_frontier<S: v1::CatchupApi>(
    State(state): State<S>,
    Path((height, view)): Path<(u64, u64)>,
) -> Result<ApiJson<S::BlocksFrontier>, ApiError> {
    state
        .get_blocks_frontier(height, view)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_chain_config<S: v1::CatchupApi>(
    State(state): State<S>,
    Path(commitment): Path<String>,
) -> Result<ApiJson<S::ChainConfig>, ApiError> {
    state
        .get_chain_config(commitment)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_leaf_chain<S: v1::CatchupApi>(
    State(state): State<S>,
    Path(height): Path<u64>,
) -> Result<ApiJson<S::LeafChain>, ApiError> {
    state
        .get_leaf_chain(height)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_cert2<S: v1::CatchupApi>(
    State(state): State<S>,
    Path(height): Path<u64>,
) -> Result<ApiJson<S::Cert2>, ApiError> {
    state
        .get_cert2(height)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_reward_account_v1<S: v1::CatchupApi>(
    State(state): State<S>,
    Path((height, view, address)): Path<(u64, u64, String)>,
) -> Result<ApiJson<S::RewardAccountQueryDataV1>, ApiError> {
    state
        .get_reward_account_v1(height, view, address)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_reward_accounts_v1<S: v1::CatchupApi>(
    State(state): State<S>,
    Path((height, view)): Path<(u64, u64)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let accounts: Vec<S::RewardAccountV1> = decode_body(&headers, &body)?;
    let tree = state
        .get_reward_accounts_v1(height, view, accounts)
        .await
        .map_err(classify_availability_error)?;
    encode_response(&headers, tree)
}

async fn get_reward_account_v2<S: v1::CatchupApi>(
    State(state): State<S>,
    Path((height, view, address)): Path<(u64, u64, String)>,
) -> Result<ApiJson<S::RewardAccountQueryDataV2>, ApiError> {
    state
        .get_reward_account_v2(height, view, address)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

// Deprecated: always 404.
async fn reward_accounts_v2_deprecated<S: v1::CatchupApi>(
    State(_): State<S>,
    Path((_height, _view)): Path<(u64, u64)>,
) -> Result<Json<()>, ApiError> {
    Err(ApiError::NotFound(anyhow::anyhow!(
        "catchup/reward-accounts-v2 is deprecated"
    )))
}

async fn reward_amounts_deprecated<S: v1::CatchupApi>(
    State(_): State<S>,
    Path((_height, _limit, _offset)): Path<(u64, u64, u64)>,
) -> Result<Json<()>, ApiError> {
    Err(ApiError::NotFound(anyhow::anyhow!(
        "catchup/reward-amounts is deprecated"
    )))
}

async fn get_reward_merkle_tree_v2<S: v1::CatchupApi>(
    State(state): State<S>,
    Path((height, view)): Path<(u64, u64)>,
) -> Result<ApiJson<S::RewardMerkleTreeV2Data>, ApiError> {
    state
        .get_reward_merkle_tree_v2(height, view)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_state_cert<S: v1::CatchupApi>(
    State(state): State<S>,
    Path(epoch): Path<u64>,
) -> Result<ApiJson<S::StateCert>, ApiError> {
    state
        .get_state_cert(epoch)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

pub(crate) fn router_catchup<S>(state: S) -> ApiRouter
where
    S: v1::CatchupApi + Clone + Send + Sync + 'static,
{
    let catchup = ApiRouter::new()
        .api_route(
            "/{height}/{view}/account/{address}",
            get_with(get_account::<S>, |op| {
                op.summary("Catch up fee account balance").description(
                    "Get the fee account balance and Merkle proof for an address at the given \
                     block height and view, for catchup.",
                )
            }),
        )
        .api_route(
            "/{height}/{view}/accounts",
            post_with(get_accounts::<S>, |op| {
                op.summary("Catch up fee accounts (bulk)").description(
                    "Bulk version of the fee account endpoint; request body is a JSON array of \
                     TaggedBase64 fee accounts, response is a FeeMerkleTree.",
                )
            }),
        )
        .api_route(
            "/{height}/{view}/blocks",
            get_with(get_blocks_frontier::<S>, |op| {
                op.summary("Catch up blocks Merkle frontier").description(
                    "Get the blocks Merkle tree frontier at the given block height and view, for \
                     catchup.",
                )
            }),
        )
        .api_route(
            "/chain-config/{commitment}",
            get_with(get_chain_config::<S>, |op| {
                op.summary("Catch up chain config").description(
                    "Retrieve the chain config matching the given commitment from a peer; used \
                     when a node missed a protocol upgrade.",
                )
            }),
        )
        .api_route(
            "/{height}/leafchain",
            get_with(get_leaf_chain::<S>, |op| {
                op.summary("Catch up leaf chain").description(
                    "Fetch a leaf chain that decides the block at the given height, for catching \
                     up the stake table.",
                )
            }),
        )
        .api_route(
            "/{height}/cert2",
            get_with(get_cert2::<S>, |op| {
                op.summary("Catch up cert2").description(
                    "Fetch the cert2 stored at exactly the given height, if one exists; 404 \
                     otherwise.",
                )
            }),
        )
        .api_route(
            "/{height}/{view}/reward-account/{address}",
            get_with(get_reward_account_v1::<S>, |op| {
                op.summary("Catch up reward account (V1)").description(
                    "Get the reward account balance for an address at the given height and view.",
                )
            }),
        )
        .api_route(
            "/{height}/{view}/reward-accounts",
            post_with(get_reward_accounts_v1::<S>, |op| {
                op.summary("Catch up reward accounts (bulk, V1)")
                    .description(
                        "Bulk version of the reward account endpoint; request body is a JSON \
                         array of TaggedBase64 reward accounts, response is a RewardMerkleTreeV1.",
                    )
            }),
        )
        .api_route(
            "/{height}/{view}/reward-account-v2/{address}",
            get_with(get_reward_account_v2::<S>, |op| {
                op.summary("Catch up reward account (V2)").description(
                    "Get the reward account balance for an address at the given height and view, \
                     from RewardMerkleTreeV2.",
                )
            }),
        )
        .api_route(
            "/{height}/{view}/reward-accounts-v2",
            post_with(reward_accounts_v2_deprecated::<S>, |op| {
                op.summary("Catch up reward accounts (bulk, V2) — deprecated")
                    .description("Deprecated: this endpoint always returns 404 Not Found.")
            }),
        )
        .api_route(
            "/{height}/reward-amounts/{limit}/{offset}",
            get_with(reward_amounts_deprecated::<S>, |op| {
                op.summary("List reward amounts — deprecated")
                    .description("Deprecated: this endpoint always returns 404 Not Found.")
            }),
        )
        .api_route(
            "/reward-merkle-tree-v2/{height}/{view}",
            get_with(get_reward_merkle_tree_v2::<S>, |op| {
                op.summary("Catch up RewardMerkleTreeV2").description(
                    "Get the RewardMerkleTreeV2 from consensus state at the given height and \
                     view, serialized as RewardMerkleTreeV2Data.",
                )
            }),
        )
        .api_route(
            "/{epoch}/state-cert",
            get_with(get_state_cert::<S>, |op| {
                op.summary("Catch up state certificate")
                    .description("Get the light client state certificate for the given epoch.")
            }),
        );

    ApiRouter::new().nest("/catchup", catchup).with_state(state)
}
