use aide::transform::TransformOperation;

use super::*;

async fn leaf_proof_by_height<S: v1::LightClientApi>(
    State(state): State<S>,
    Path(height): Path<u64>,
) -> Result<ApiJson<S::LeafProof>, ApiError> {
    state
        .get_leaf_proof(v1::LeafQuery::Height(height), None)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn leaf_proof_by_height_finalized<S: v1::LightClientApi>(
    State(state): State<S>,
    Path((height, finalized)): Path<(u64, u64)>,
) -> Result<ApiJson<S::LeafProof>, ApiError> {
    state
        .get_leaf_proof(v1::LeafQuery::Height(height), Some(finalized))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn leaf_proof_by_hash<S: v1::LightClientApi>(
    State(state): State<S>,
    Path(hash): Path<String>,
) -> Result<ApiJson<S::LeafProof>, ApiError> {
    state
        .get_leaf_proof(v1::LeafQuery::Hash(hash), None)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn leaf_proof_by_hash_finalized<S: v1::LightClientApi>(
    State(state): State<S>,
    Path((hash, finalized)): Path<(String, u64)>,
) -> Result<ApiJson<S::LeafProof>, ApiError> {
    state
        .get_leaf_proof(v1::LeafQuery::Hash(hash), Some(finalized))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn leaf_proof_by_block_hash<S: v1::LightClientApi>(
    State(state): State<S>,
    Path(block_hash): Path<String>,
) -> Result<ApiJson<S::LeafProof>, ApiError> {
    state
        .get_leaf_proof(v1::LeafQuery::BlockHash(block_hash), None)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn leaf_proof_by_block_hash_finalized<S: v1::LightClientApi>(
    State(state): State<S>,
    Path((block_hash, finalized)): Path<(String, u64)>,
) -> Result<ApiJson<S::LeafProof>, ApiError> {
    state
        .get_leaf_proof(v1::LeafQuery::BlockHash(block_hash), Some(finalized))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn leaf_proof_by_payload_hash<S: v1::LightClientApi>(
    State(state): State<S>,
    Path(payload_hash): Path<String>,
) -> Result<ApiJson<S::LeafProof>, ApiError> {
    state
        .get_leaf_proof(v1::LeafQuery::PayloadHash(payload_hash), None)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn leaf_proof_by_payload_hash_finalized<S: v1::LightClientApi>(
    State(state): State<S>,
    Path((payload_hash, finalized)): Path<(String, u64)>,
) -> Result<ApiJson<S::LeafProof>, ApiError> {
    state
        .get_leaf_proof(v1::LeafQuery::PayloadHash(payload_hash), Some(finalized))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

fn leaf_proof_operation(op: TransformOperation) -> TransformOperation {
    op.summary("Get leaf with finality proof").description(
        "Fetch a leaf plus a proof of its finality, optionally relative to an \
         already-known-finalized height.",
    )
}

async fn header_proof_by_height<S: v1::LightClientApi>(
    State(state): State<S>,
    Path((root, height)): Path<(u64, u64)>,
) -> Result<ApiJson<S::HeaderProof>, ApiError> {
    state
        .get_header_proof(root, v1::HeaderQuery::Height(height))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn header_proof_by_hash<S: v1::LightClientApi>(
    State(state): State<S>,
    Path((root, hash)): Path<(u64, String)>,
) -> Result<ApiJson<S::HeaderProof>, ApiError> {
    state
        .get_header_proof(root, v1::HeaderQuery::Hash(hash))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn header_proof_by_payload_hash<S: v1::LightClientApi>(
    State(state): State<S>,
    Path((root, payload_hash)): Path<(u64, String)>,
) -> Result<ApiJson<S::HeaderProof>, ApiError> {
    state
        .get_header_proof(root, v1::HeaderQuery::PayloadHash(payload_hash))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

fn header_proof_operation(op: TransformOperation) -> TransformOperation {
    op.summary("Get header with inclusion proof").description(
        "Fetch a header plus a Merkle proof that it belongs to the blocks Merkle tree rooted at \
         the given root height.",
    )
}

async fn stake_table<S: v1::LightClientApi>(
    State(state): State<S>,
    Path(epoch): Path<u64>,
) -> Result<ApiJson<S::StakeTableEvents>, ApiError> {
    state
        .get_light_client_stake_table(epoch)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn payload_proof<S: v1::LightClientApi>(
    State(state): State<S>,
    Path(height): Path<u64>,
) -> Result<ApiJson<S::PayloadProof>, ApiError> {
    state
        .get_payload_proof(height)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn payload_proof_range<S: v1::LightClientApi>(
    State(state): State<S>,
    Path((start, end)): Path<(u64, u64)>,
) -> Result<ApiJson<Vec<S::PayloadProof>>, ApiError> {
    state
        .get_payload_proof_range(start, end)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn namespace_proof<S: v1::LightClientApi>(
    State(state): State<S>,
    Path((height, namespace)): Path<(u64, u64)>,
) -> Result<ApiJson<S::NamespaceProof>, ApiError> {
    state
        .get_lc_namespace_proof(height, namespace)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn namespace_proof_range<S: v1::LightClientApi>(
    State(state): State<S>,
    Path((start, end, namespace)): Path<(u64, u64, u64)>,
) -> Result<ApiJson<Vec<S::NamespaceProof>>, ApiError> {
    state
        .get_lc_namespace_proof_range(start, end, namespace)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn namespaces_proof_range<S: v1::LightClientApi>(
    State(state): State<S>,
    Path((start, end, namespaces)): Path<(u64, u64, String)>,
) -> Result<ApiJson<Vec<std::collections::HashMap<u64, S::NamespaceProof>>>, ApiError> {
    state
        .get_lc_namespaces_proof_range(start, end, namespaces)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

pub(crate) fn router_light_client<S>(state: S) -> ApiRouter
where
    S: v1::LightClientApi + Clone + Send + Sync + 'static,
{
    let light_client = ApiRouter::new()
        .api_route(
            "/leaf/{height}",
            get_with(leaf_proof_by_height::<S>, leaf_proof_operation),
        )
        .api_route(
            "/leaf/{height}/{finalized}",
            get_with(leaf_proof_by_height_finalized::<S>, leaf_proof_operation),
        )
        .api_route(
            "/leaf/hash/{hash}",
            get_with(leaf_proof_by_hash::<S>, leaf_proof_operation),
        )
        .api_route(
            "/leaf/hash/{hash}/{finalized}",
            get_with(leaf_proof_by_hash_finalized::<S>, leaf_proof_operation),
        )
        .api_route(
            "/leaf/block-hash/{block_hash}",
            get_with(leaf_proof_by_block_hash::<S>, leaf_proof_operation),
        )
        .api_route(
            "/leaf/block-hash/{block_hash}/{finalized}",
            get_with(
                leaf_proof_by_block_hash_finalized::<S>,
                leaf_proof_operation,
            ),
        )
        .api_route(
            "/leaf/payload-hash/{payload_hash}",
            get_with(leaf_proof_by_payload_hash::<S>, leaf_proof_operation),
        )
        .api_route(
            "/leaf/payload-hash/{payload_hash}/{finalized}",
            get_with(
                leaf_proof_by_payload_hash_finalized::<S>,
                leaf_proof_operation,
            ),
        )
        .api_route(
            "/header/{root}/{height}",
            get_with(header_proof_by_height::<S>, header_proof_operation),
        )
        .api_route(
            "/header/{root}/hash/{hash}",
            get_with(header_proof_by_hash::<S>, header_proof_operation),
        )
        .api_route(
            "/header/{root}/payload-hash/{payload_hash}",
            get_with(header_proof_by_payload_hash::<S>, header_proof_operation),
        )
        .api_route(
            "/stake-table/{epoch}",
            get_with(stake_table::<S>, |op| {
                op.summary("Get stake table events for epoch").description(
                    "Get the events needed to transform the stake table from the previous epoch \
                     into the given epoch.",
                )
            }),
        )
        .api_route(
            "/payload/{height}",
            get_with(payload_proof::<S>, |op| {
                op.summary("Get payload with VID common data").description(
                    "Fetch a payload plus the VID common data needed to recompute and verify its \
                     hash.",
                )
            }),
        )
        .api_route(
            "/payload/{start}/{end}",
            get_with(payload_proof_range::<S>, |op| {
                op.summary("Get payload proofs in range").description(
                    "Fetch a list of payload proofs for each block in the given range.",
                )
            }),
        )
        .api_route(
            "/namespace/{height}/{namespace}",
            get_with(namespace_proof::<S>, |op| {
                op.summary("Get namespace proof with VID common data")
                    .description(
                        "Fetch a namespace proof plus the VID common data needed to verify it.",
                    )
            }),
        )
        .api_route(
            "/namespace/{start}/{end}/{namespace}",
            get_with(namespace_proof_range::<S>, |op| {
                op.summary("Get namespace proofs in range").description(
                    "Fetch a list of namespace proofs for each block in the given range.",
                )
            }),
        )
        .api_route(
            "/namespaces/{start}/{end}/{namespaces}",
            get_with(namespaces_proof_range::<S>, |op| {
                op.summary("Get proofs for multiple namespaces in range")
                    .description(
                        "Fetch namespace proofs for each block in the given range, restricted to \
                         a caller-specified set of namespaces.",
                    )
            }),
        );

    ApiRouter::new()
        .nest("/light-client", light_client)
        .with_state(state)
}
