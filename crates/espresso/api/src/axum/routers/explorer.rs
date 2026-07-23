use aide::transform::TransformOperation;

use super::*;

async fn get_block_detail_by_height<S: v1::ExplorerApi>(
    State(state): State<S>,
    Path(height): Path<u64>,
) -> Result<ApiJson<S::BlockDetail>, ApiError> {
    state
        .get_block_detail(v1::BlockIdent::Height(height))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_block_detail_by_hash<S: v1::ExplorerApi>(
    State(state): State<S>,
    Path(hash): Path<String>,
) -> Result<ApiJson<S::BlockDetail>, ApiError> {
    state
        .get_block_detail(v1::BlockIdent::Hash(hash))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

fn block_detail_operation(op: TransformOperation) -> TransformOperation {
    op.summary("Get block detail")
        .description("Get details for a block identified by height or hash.")
}

async fn get_block_summaries_latest<S: v1::ExplorerApi>(
    State(state): State<S>,
    Path(limit): Path<u64>,
) -> Result<ApiJson<S::BlockSummaries>, ApiError> {
    state
        .get_block_summaries(v1::BlockIdent::Latest, limit)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_block_summaries_from<S: v1::ExplorerApi>(
    State(state): State<S>,
    Path((from, limit)): Path<(u64, u64)>,
) -> Result<ApiJson<S::BlockSummaries>, ApiError> {
    state
        .get_block_summaries(v1::BlockIdent::Height(from), limit)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

fn block_summaries_operation(op: TransformOperation) -> TransformOperation {
    op.summary("List block summaries").description(
        "Retrieve up to `limit` block summaries, targeting the latest block or a block identified \
         by height.",
    )
}

async fn get_transaction_detail_by_position<S: v1::ExplorerApi>(
    State(state): State<S>,
    Path((height, offset)): Path<(u64, u64)>,
) -> Result<ApiJson<S::TransactionDetail>, ApiError> {
    state
        .get_transaction_detail(v1::TxIdent::HeightAndOffset(height, offset))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_transaction_detail_by_hash<S: v1::ExplorerApi>(
    State(state): State<S>,
    Path(hash): Path<String>,
) -> Result<ApiJson<S::TransactionDetail>, ApiError> {
    state
        .get_transaction_detail(v1::TxIdent::Hash(hash))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

fn transaction_detail_operation(op: TransformOperation) -> TransformOperation {
    op.summary("Get transaction detail")
        .description("Get details for a transaction identified by height and offset, or by hash.")
}

// One handler per (target × filter) combination; all share `transaction_summaries_operation`.

async fn tx_summaries_latest_block<S: v1::ExplorerApi>(
    State(state): State<S>,
    Path((limit, block)): Path<(u64, u64)>,
) -> Result<ApiJson<S::TransactionSummaries>, ApiError> {
    state
        .get_transaction_summaries(
            v1::TxIdent::Latest,
            limit,
            v1::TxSummaryFilter::Block(block),
        )
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn tx_summaries_from_block<S: v1::ExplorerApi>(
    State(state): State<S>,
    Path((height, offset, limit, block)): Path<(u64, u64, u64, u64)>,
) -> Result<ApiJson<S::TransactionSummaries>, ApiError> {
    state
        .get_transaction_summaries(
            v1::TxIdent::HeightAndOffset(height, offset),
            limit,
            v1::TxSummaryFilter::Block(block),
        )
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn tx_summaries_hash_block<S: v1::ExplorerApi>(
    State(state): State<S>,
    Path((hash, limit, block)): Path<(String, u64, u64)>,
) -> Result<ApiJson<S::TransactionSummaries>, ApiError> {
    state
        .get_transaction_summaries(
            v1::TxIdent::Hash(hash),
            limit,
            v1::TxSummaryFilter::Block(block),
        )
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn tx_summaries_latest_ns<S: v1::ExplorerApi>(
    State(state): State<S>,
    Path((limit, namespace)): Path<(u64, i64)>,
) -> Result<ApiJson<S::TransactionSummaries>, ApiError> {
    state
        .get_transaction_summaries(
            v1::TxIdent::Latest,
            limit,
            v1::TxSummaryFilter::Namespace(namespace),
        )
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn tx_summaries_from_ns<S: v1::ExplorerApi>(
    State(state): State<S>,
    Path((height, offset, limit, namespace)): Path<(u64, u64, u64, i64)>,
) -> Result<ApiJson<S::TransactionSummaries>, ApiError> {
    state
        .get_transaction_summaries(
            v1::TxIdent::HeightAndOffset(height, offset),
            limit,
            v1::TxSummaryFilter::Namespace(namespace),
        )
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn tx_summaries_hash_ns<S: v1::ExplorerApi>(
    State(state): State<S>,
    Path((hash, limit, namespace)): Path<(String, u64, i64)>,
) -> Result<ApiJson<S::TransactionSummaries>, ApiError> {
    state
        .get_transaction_summaries(
            v1::TxIdent::Hash(hash),
            limit,
            v1::TxSummaryFilter::Namespace(namespace),
        )
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn tx_summaries_latest<S: v1::ExplorerApi>(
    State(state): State<S>,
    Path(limit): Path<u64>,
) -> Result<ApiJson<S::TransactionSummaries>, ApiError> {
    state
        .get_transaction_summaries(v1::TxIdent::Latest, limit, v1::TxSummaryFilter::None)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn tx_summaries_from<S: v1::ExplorerApi>(
    State(state): State<S>,
    Path((height, offset, limit)): Path<(u64, u64, u64)>,
) -> Result<ApiJson<S::TransactionSummaries>, ApiError> {
    state
        .get_transaction_summaries(
            v1::TxIdent::HeightAndOffset(height, offset),
            limit,
            v1::TxSummaryFilter::None,
        )
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn tx_summaries_hash<S: v1::ExplorerApi>(
    State(state): State<S>,
    Path((hash, limit)): Path<(String, u64)>,
) -> Result<ApiJson<S::TransactionSummaries>, ApiError> {
    state
        .get_transaction_summaries(v1::TxIdent::Hash(hash), limit, v1::TxSummaryFilter::None)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

fn transaction_summaries_operation(op: TransformOperation) -> TransformOperation {
    op.summary("List transaction summaries").description(
        "Retrieve up to `limit` transaction summaries, targeting the latest transaction, one \
         identified by height/offset, or by hash; optionally filtered by block or namespace.",
    )
}

async fn get_explorer_summary<S: v1::ExplorerApi>(
    State(state): State<S>,
) -> Result<ApiJson<S::ExplorerSummary>, ApiError> {
    state
        .get_explorer_summary()
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_search_result<S: v1::ExplorerApi>(
    State(state): State<S>,
    Path(query): Path<String>,
) -> Result<ApiJson<S::SearchResult>, ApiError> {
    state
        .get_search_result(query)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

pub(crate) fn router_explorer<S>(state: S) -> ApiRouter
where
    S: v1::ExplorerApi + Clone + Send + Sync + 'static,
{
    let explorer = ApiRouter::new()
        .api_route(
            "/block/{height}",
            get_with(get_block_detail_by_height::<S>, block_detail_operation),
        )
        .api_route(
            "/block/hash/{hash}",
            get_with(get_block_detail_by_hash::<S>, block_detail_operation),
        )
        .api_route(
            "/blocks/latest/{limit}",
            get_with(get_block_summaries_latest::<S>, block_summaries_operation),
        )
        .api_route(
            "/blocks/{from}/{limit}",
            get_with(get_block_summaries_from::<S>, block_summaries_operation),
        )
        .api_route(
            "/transaction/{height}/{offset}",
            get_with(
                get_transaction_detail_by_position::<S>,
                transaction_detail_operation,
            ),
        )
        .api_route(
            "/transaction/hash/{hash}",
            get_with(
                get_transaction_detail_by_hash::<S>,
                transaction_detail_operation,
            ),
        )
        .api_route(
            "/transactions/latest/{limit}/block/{block}",
            get_with(
                tx_summaries_latest_block::<S>,
                transaction_summaries_operation,
            ),
        )
        .api_route(
            "/transactions/from/{height}/{offset}/{limit}/block/{block}",
            get_with(
                tx_summaries_from_block::<S>,
                transaction_summaries_operation,
            ),
        )
        .api_route(
            "/transactions/hash/{hash}/{limit}/block/{block}",
            get_with(
                tx_summaries_hash_block::<S>,
                transaction_summaries_operation,
            ),
        )
        .api_route(
            "/transactions/latest/{limit}/namespace/{namespace}",
            get_with(tx_summaries_latest_ns::<S>, transaction_summaries_operation),
        )
        .api_route(
            "/transactions/from/{height}/{offset}/{limit}/namespace/{namespace}",
            get_with(tx_summaries_from_ns::<S>, transaction_summaries_operation),
        )
        .api_route(
            "/transactions/hash/{hash}/{limit}/namespace/{namespace}",
            get_with(tx_summaries_hash_ns::<S>, transaction_summaries_operation),
        )
        .api_route(
            "/transactions/latest/{limit}",
            get_with(tx_summaries_latest::<S>, transaction_summaries_operation),
        )
        .api_route(
            "/transactions/from/{height}/{offset}/{limit}",
            get_with(tx_summaries_from::<S>, transaction_summaries_operation),
        )
        .api_route(
            "/transactions/hash/{hash}/{limit}",
            get_with(tx_summaries_hash::<S>, transaction_summaries_operation),
        )
        .api_route(
            "/explorer-summary",
            get_with(get_explorer_summary::<S>, |op| {
                op.summary("Get explorer summary")
                    .description("Get the current chain explorer summary.")
            }),
        )
        .api_route(
            "/search/{query}",
            get_with(get_search_result::<S>, |op| {
                op.summary("Search blocks and transactions").description(
                    "Search for blocks or transactions matching the given query string; currently \
                     matched against hash.",
                )
            }),
        );

    ApiRouter::new()
        .nest("/explorer", explorer)
        .with_state(state)
}
