use super::*;

async fn block_height<S: v1::NodeApi>(State(state): State<S>) -> Result<ApiJson<u64>, ApiError> {
    state
        .block_height()
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn count_transactions<S: v1::NodeApi>(
    State(state): State<S>,
) -> Result<ApiJson<u64>, ApiError> {
    state
        .count_transactions(None, None, None)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn count_transactions_ns<S: v1::NodeApi>(
    State(state): State<S>,
    Path(namespace): Path<u64>,
) -> Result<ApiJson<u64>, ApiError> {
    state
        .count_transactions(None, None, Some(namespace))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn count_transactions_ns_to<S: v1::NodeApi>(
    State(state): State<S>,
    Path((namespace, to)): Path<(u64, u64)>,
) -> Result<ApiJson<u64>, ApiError> {
    state
        .count_transactions(None, Some(to), Some(namespace))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn count_transactions_ns_from_to<S: v1::NodeApi>(
    State(state): State<S>,
    Path((namespace, from, to)): Path<(u64, u64, u64)>,
) -> Result<ApiJson<u64>, ApiError> {
    state
        .count_transactions(Some(from), Some(to), Some(namespace))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn count_transactions_to<S: v1::NodeApi>(
    State(state): State<S>,
    Path(to): Path<u64>,
) -> Result<ApiJson<u64>, ApiError> {
    state
        .count_transactions(None, Some(to), None)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn count_transactions_from_to<S: v1::NodeApi>(
    State(state): State<S>,
    Path((from, to)): Path<(u64, u64)>,
) -> Result<ApiJson<u64>, ApiError> {
    state
        .count_transactions(Some(from), Some(to), None)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn payload_size<S: v1::NodeApi>(State(state): State<S>) -> Result<ApiJson<u64>, ApiError> {
    state
        .payload_size(None, None, None)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn payload_size_ns<S: v1::NodeApi>(
    State(state): State<S>,
    Path(namespace): Path<u64>,
) -> Result<ApiJson<u64>, ApiError> {
    state
        .payload_size(None, None, Some(namespace))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn payload_size_ns_to<S: v1::NodeApi>(
    State(state): State<S>,
    Path((namespace, to)): Path<(u64, u64)>,
) -> Result<ApiJson<u64>, ApiError> {
    state
        .payload_size(None, Some(to), Some(namespace))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn payload_size_ns_from_to<S: v1::NodeApi>(
    State(state): State<S>,
    Path((namespace, from, to)): Path<(u64, u64, u64)>,
) -> Result<ApiJson<u64>, ApiError> {
    state
        .payload_size(Some(from), Some(to), Some(namespace))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn payload_size_to<S: v1::NodeApi>(
    State(state): State<S>,
    Path(to): Path<u64>,
) -> Result<ApiJson<u64>, ApiError> {
    state
        .payload_size(None, Some(to), None)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn payload_size_from_to<S: v1::NodeApi>(
    State(state): State<S>,
    Path((from, to)): Path<(u64, u64)>,
) -> Result<ApiJson<u64>, ApiError> {
    state
        .payload_size(Some(from), Some(to), None)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_vid_share_by_hash<S: v1::NodeApi>(
    State(state): State<S>,
    Path(hash): Path<String>,
) -> Result<ApiJson<S::VidShare>, ApiError> {
    state
        .get_vid_share(v1::VidShareId::Hash(hash))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_vid_share_by_payload_hash<S: v1::NodeApi>(
    State(state): State<S>,
    Path(payload_hash): Path<String>,
) -> Result<ApiJson<S::VidShare>, ApiError> {
    state
        .get_vid_share(v1::VidShareId::PayloadHash(payload_hash))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_vid_share_by_height<S: v1::NodeApi>(
    State(state): State<S>,
    Path(height): Path<u64>,
) -> Result<ApiJson<S::VidShare>, ApiError> {
    state
        .get_vid_share(v1::VidShareId::Height(height))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn sync_status<S: v1::NodeApi>(
    State(state): State<S>,
) -> Result<ApiJson<S::SyncStatus>, ApiError> {
    state
        .sync_status()
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn header_window_by_hash<S: v1::NodeApi>(
    State(state): State<S>,
    Path((hash, end)): Path<(String, u64)>,
) -> Result<ApiJson<S::HeaderWindow>, ApiError> {
    state
        .get_header_window(v1::HeaderWindowStart::Hash(hash), end)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn header_window_by_height<S: v1::NodeApi>(
    State(state): State<S>,
    Path((height, end)): Path<(u64, u64)>,
) -> Result<ApiJson<S::HeaderWindow>, ApiError> {
    state
        .get_header_window(v1::HeaderWindowStart::Height(height), end)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn header_window_by_time<S: v1::NodeApi>(
    State(state): State<S>,
    Path((start, end)): Path<(u64, u64)>,
) -> Result<ApiJson<S::HeaderWindow>, ApiError> {
    state
        .get_header_window(v1::HeaderWindowStart::Time(start), end)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn limits<S: v1::NodeApi>(State(state): State<S>) -> Result<ApiJson<S::Limits>, ApiError> {
    state
        .limits()
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn stake_table_current<S: v1::NodeApi>(
    State(state): State<S>,
) -> Result<ApiJson<S::StakeTableCurrent>, ApiError> {
    state
        .stake_table_current()
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn stake_table<S: v1::NodeApi>(
    State(state): State<S>,
    Path(epoch_number): Path<u64>,
) -> Result<ApiJson<S::StakeTable>, ApiError> {
    state
        .stake_table(epoch_number)
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn da_stake_table_current<S: v1::NodeApi>(
    State(state): State<S>,
) -> Result<ApiJson<S::StakeTableCurrent>, ApiError> {
    state
        .da_stake_table_current()
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn da_stake_table<S: v1::NodeApi>(
    State(state): State<S>,
    Path(epoch_number): Path<u64>,
) -> Result<ApiJson<S::StakeTable>, ApiError> {
    state
        .da_stake_table(epoch_number)
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn get_validators<S: v1::NodeApi>(
    State(state): State<S>,
    Path(epoch_number): Path<u64>,
) -> Result<ApiJson<S::Validators>, ApiError> {
    state
        .get_validators(epoch_number)
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn get_all_validators<S: v1::NodeApi>(
    State(state): State<S>,
    Path((epoch_number, offset, limit)): Path<(u64, u64, u64)>,
) -> Result<ApiJson<S::AllValidators>, ApiError> {
    state
        .get_all_validators(epoch_number, offset, limit)
        .await
        .map(ApiJson)
        .map_err(ApiError::BadRequest)
}

async fn current_proposal_participation<S: v1::NodeApi>(
    State(state): State<S>,
) -> Result<ApiJson<S::Participation>, ApiError> {
    state
        .current_proposal_participation()
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn proposal_participation<S: v1::NodeApi>(
    State(state): State<S>,
    Path(epoch): Path<u64>,
) -> Result<ApiJson<S::Participation>, ApiError> {
    state
        .proposal_participation(epoch)
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn current_vote_participation<S: v1::NodeApi>(
    State(state): State<S>,
) -> Result<ApiJson<S::Participation>, ApiError> {
    state
        .current_vote_participation()
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn vote_participation<S: v1::NodeApi>(
    State(state): State<S>,
    Path(epoch): Path<u64>,
) -> Result<ApiJson<S::Participation>, ApiError> {
    state
        .vote_participation(epoch)
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn block_reward<S: v1::NodeApi>(
    State(state): State<S>,
) -> Result<ApiJson<S::BlockReward>, ApiError> {
    state
        .get_block_reward(None)
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn block_reward_epoch<S: v1::NodeApi>(
    State(state): State<S>,
    Path(epoch_number): Path<u64>,
) -> Result<ApiJson<S::BlockReward>, ApiError> {
    state
        .get_block_reward(Some(epoch_number))
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn oldest_block<S: v1::NodeApi>(
    State(state): State<S>,
) -> Result<ApiJson<Option<S::Block>>, ApiError> {
    state
        .get_oldest_block()
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn oldest_leaf<S: v1::NodeApi>(
    State(state): State<S>,
) -> Result<ApiJson<Option<S::Leaf>>, ApiError> {
    state
        .get_oldest_leaf()
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

pub(crate) fn router_node<S>(state: S) -> ApiRouter
where
    S: v1::NodeApi + Clone + Send + Sync + 'static,
{
    let node = ApiRouter::new()
        .api_route(
            "/block-height",
            get_with(block_height::<S>, |op| {
                op.summary("Get node's block height")
                    .description("The current height of the chain, as observed by this node.")
            }),
        )
        .api_route(
            "/transactions/count",
            get_with(count_transactions::<S>, |op| {
                op.summary("Count transactions").description(
                    "Get the number of transactions in the chain, optionally restricted by block \
                     range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/count/namespace/{namespace}",
            get_with(count_transactions_ns::<S>, |op| {
                op.summary("Count transactions").description(
                    "Get the number of transactions in the chain, optionally restricted by block \
                     range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/count/namespace/{namespace}/{to}",
            get_with(count_transactions_ns_to::<S>, |op| {
                op.summary("Count transactions").description(
                    "Get the number of transactions in the chain, optionally restricted by block \
                     range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/count/namespace/{namespace}/{from}/{to}",
            get_with(count_transactions_ns_from_to::<S>, |op| {
                op.summary("Count transactions").description(
                    "Get the number of transactions in the chain, optionally restricted by block \
                     range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/count/{to}",
            get_with(count_transactions_to::<S>, |op| {
                op.summary("Count transactions").description(
                    "Get the number of transactions in the chain, optionally restricted by block \
                     range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/count/{from}/{to}",
            get_with(count_transactions_from_to::<S>, |op| {
                op.summary("Count transactions").description(
                    "Get the number of transactions in the chain, optionally restricted by block \
                     range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/payloads/size",
            get_with(payload_size::<S>, |op| {
                op.summary("Get payload size").description(
                    "Get the cumulative size (bytes) of payload data in the chain, optionally \
                     restricted by block range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/payloads/total-size",
            get_with(payload_size::<S>, |op| {
                op.summary("Get payload size")
                    .description("Deprecated alias for payloads/size.")
            }),
        )
        .api_route(
            "/payloads/size/namespace/{namespace}",
            get_with(payload_size_ns::<S>, |op| {
                op.summary("Get payload size").description(
                    "Get the cumulative size (bytes) of payload data in the chain, optionally \
                     restricted by block range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/payloads/size/namespace/{namespace}/{to}",
            get_with(payload_size_ns_to::<S>, |op| {
                op.summary("Get payload size").description(
                    "Get the cumulative size (bytes) of payload data in the chain, optionally \
                     restricted by block range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/payloads/size/namespace/{namespace}/{from}/{to}",
            get_with(payload_size_ns_from_to::<S>, |op| {
                op.summary("Get payload size").description(
                    "Get the cumulative size (bytes) of payload data in the chain, optionally \
                     restricted by block range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/payloads/size/{to}",
            get_with(payload_size_to::<S>, |op| {
                op.summary("Get payload size").description(
                    "Get the cumulative size (bytes) of payload data in the chain, optionally \
                     restricted by block range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/payloads/size/{from}/{to}",
            get_with(payload_size_from_to::<S>, |op| {
                op.summary("Get payload size").description(
                    "Get the cumulative size (bytes) of payload data in the chain, optionally \
                     restricted by block range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/vid/share/hash/{hash}",
            get_with(get_vid_share_by_hash::<S>, |op| {
                op.summary("Get this node's VID share").description(
                    "Get information needed to run the VID reconstruction protocol for a block: \
                     this node's VID share, if available.",
                )
            }),
        )
        .api_route(
            "/vid/share/payload-hash/{payload_hash}",
            get_with(get_vid_share_by_payload_hash::<S>, |op| {
                op.summary("Get this node's VID share").description(
                    "Get information needed to run the VID reconstruction protocol for a block: \
                     this node's VID share, if available.",
                )
            }),
        )
        .api_route(
            "/vid/share/{height}",
            get_with(get_vid_share_by_height::<S>, |op| {
                op.summary("Get this node's VID share").description(
                    "Get information needed to run the VID reconstruction protocol for a block: \
                     this node's VID share, if available.",
                )
            }),
        )
        .api_route(
            "/sync-status",
            get_with(sync_status::<S>, |op| {
                op.summary("Get node sync status").description(
                    "Get the node's progress syncing with the latest chain state \
                     (missing/present/pruned ranges for blocks, leaves, and VID common).",
                )
            }),
        )
        .api_route(
            "/header/window/from/hash/{hash}/{end}",
            get_with(header_window_by_hash::<S>, |op| {
                op.summary("Get header window").description(
                    "Get block headers whose timestamps fall in a time window, plus one header \
                     before and after to prove completeness.",
                )
            }),
        )
        .api_route(
            "/header/window/from/{height}/{end}",
            get_with(header_window_by_height::<S>, |op| {
                op.summary("Get header window").description(
                    "Get block headers whose timestamps fall in a time window, plus one header \
                     before and after to prove completeness.",
                )
            }),
        )
        .api_route(
            "/header/window/{start}/{end}",
            get_with(header_window_by_time::<S>, |op| {
                op.summary("Get header window").description(
                    "Get block headers whose timestamps fall in a time window, plus one header \
                     before and after to prove completeness.",
                )
            }),
        )
        .api_route(
            "/limits",
            get_with(limits::<S>, |op| {
                op.summary("Get node limits").description(
                    "Get implementation-defined limits restricting node API requests (e.g. \
                     header/window query size).",
                )
            }),
        )
        .api_route(
            "/stake-table/current",
            get_with(stake_table_current::<S>, |op| {
                op.summary("Get current stake table")
                    .description("Get the stake table for the current epoch.")
            }),
        )
        .api_route(
            "/stake-table/{epoch_number}",
            get_with(stake_table::<S>, |op| {
                op.summary("Get stake table for epoch")
                    .description("Get the stake table for the given epoch.")
            }),
        )
        .api_route(
            "/da-stake-table/current",
            get_with(da_stake_table_current::<S>, |op| {
                op.summary("Get current DA stake table")
                    .description("Get the DA stake table for the current epoch.")
            }),
        )
        .api_route(
            "/da-stake-table/{epoch_number}",
            get_with(da_stake_table::<S>, |op| {
                op.summary("Get DA stake table for epoch")
                    .description("Get the DA stake table for the given epoch.")
            }),
        )
        .api_route(
            "/validators/{epoch_number}",
            get_with(get_validators::<S>, |op| {
                op.summary("Get validators for epoch")
                    .description("Get the validators map for the given epoch.")
            }),
        )
        .api_route(
            "/all-validators/{epoch_number}/{offset}/{limit}",
            get_with(get_all_validators::<S>, |op| {
                op.summary("Get all validators for epoch").description(
                    "Get all validators, including inactive ones, for the given epoch, paginated \
                     by offset and limit.",
                )
            }),
        )
        .api_route(
            "/participation/proposal/current",
            get_with(current_proposal_participation::<S>, |op| {
                op.summary("Get current proposal participation")
                    .description(
                        "Get the mapping from leader key to the fraction of views proposed \
                         properly as leader.",
                    )
            }),
        )
        .api_route(
            "/participation/proposal/{epoch}",
            get_with(proposal_participation::<S>, |op| {
                op.summary("Get proposal participation for epoch")
                    .description(
                        "Get the mapping from leader key to proposal participation rate for the \
                         given epoch.",
                    )
            }),
        )
        .api_route(
            "/participation/vote/current",
            get_with(current_vote_participation::<S>, |op| {
                op.summary("Get current vote participation").description(
                    "Get the mapping from node key to the fraction of views properly voted.",
                )
            }),
        )
        .api_route(
            "/participation/vote/{epoch}",
            get_with(vote_participation::<S>, |op| {
                op.summary("Get vote participation for epoch").description(
                    "Get the mapping from node key to vote participation rate for the given epoch.",
                )
            }),
        )
        .api_route(
            "/block-reward",
            get_with(block_reward::<S>, |op| {
                op.summary("Get block reward")
                    .description("Get the block reward.")
            }),
        )
        .api_route(
            "/block-reward/epoch/{epoch_number}",
            get_with(block_reward_epoch::<S>, |op| {
                op.summary("Get block reward for epoch")
                    .description("Get the block reward for the given epoch.")
            }),
        )
        .api_route(
            "/oldest-block",
            get_with(oldest_block::<S>, |op| {
                op.summary("Get oldest block").description(
                    "Get the oldest (smallest height) block present in storage, or null if none \
                     is stored.",
                )
            }),
        )
        .api_route(
            "/oldest-leaf",
            get_with(oldest_leaf::<S>, |op| {
                op.summary("Get oldest leaf").description(
                    "Get the oldest (smallest height) leaf present in storage, or null if none is \
                     stored.",
                )
            }),
        );

    ApiRouter::new().nest("/node", node).with_state(state)
}
