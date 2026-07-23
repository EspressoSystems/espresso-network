use super::*;

async fn get_namespace_proof_by_height<S: v1::AvailabilityApi>(
    State(state): State<S>,
    Path((height, namespace)): Path<(u64, u32)>,
) -> Result<ApiJson<S::NamespaceProofQueryData>, ApiError> {
    state
        .get_namespace_proof(v1::availability::BlockId::Height(height), namespace)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_namespace_proof_by_hash<S: v1::AvailabilityApi>(
    State(state): State<S>,
    Path((hash, namespace)): Path<(String, u32)>,
) -> Result<ApiJson<S::NamespaceProofQueryData>, ApiError> {
    state
        .get_namespace_proof(v1::availability::BlockId::Hash(hash), namespace)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_namespace_proof_by_payload_hash<S: v1::AvailabilityApi>(
    State(state): State<S>,
    Path((payload_hash, namespace)): Path<(String, u32)>,
) -> Result<ApiJson<S::NamespaceProofQueryData>, ApiError> {
    state
        .get_namespace_proof(
            v1::availability::BlockId::PayloadHash(payload_hash),
            namespace,
        )
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_namespace_proof_range<S: v1::AvailabilityApi>(
    State(state): State<S>,
    Path((from, until, namespace)): Path<(u64, u64, u32)>,
) -> Result<ApiJson<Vec<S::NamespaceProofQueryData>>, ApiError> {
    state
        .get_namespace_proof_range(from, until, namespace)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_incorrect_encoding_proof<S: v1::AvailabilityApi>(
    State(state): State<S>,
    Path((block_number, namespace)): Path<(u64, u32)>,
) -> Result<ApiJson<S::IncorrectEncodingProof>, ApiError> {
    state
        .get_incorrect_encoding_proof(v1::availability::BlockId::Height(block_number), namespace)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_state_cert_v1<S: v1::AvailabilityApi>(
    State(state): State<S>,
    Path(epoch): Path<u64>,
) -> Result<ApiJson<S::StateCertQueryDataV1>, ApiError> {
    state
        .get_state_cert(epoch)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_state_cert_v2<S: v1::AvailabilityApi>(
    State(state): State<S>,
    Path(epoch): Path<u64>,
) -> Result<ApiJson<S::StateCertQueryDataV2>, ApiError> {
    state
        .get_state_cert_v2(epoch)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_leaf_by_height<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path(height): Path<u64>,
) -> Result<ApiJson<S::Leaf>, ApiError> {
    state
        .get_leaf(v1::LeafId::Height(height))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_leaf_by_hash<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path(hash): Path<String>,
) -> Result<ApiJson<S::Leaf>, ApiError> {
    state
        .get_leaf(v1::LeafId::Hash(hash))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_leaf_range<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path((from, until)): Path<(usize, usize)>,
) -> Result<ApiJson<Vec<S::Leaf>>, ApiError> {
    state
        .get_leaf_range(from, until)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_header_by_height<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path(height): Path<u64>,
) -> Result<ApiJson<S::Header>, ApiError> {
    state
        .get_header(v1::BlockId::Height(height))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_header_by_hash<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path(hash): Path<String>,
) -> Result<ApiJson<S::Header>, ApiError> {
    state
        .get_header(v1::BlockId::Hash(hash))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_header_by_payload_hash<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path(payload_hash): Path<String>,
) -> Result<ApiJson<S::Header>, ApiError> {
    state
        .get_header(v1::BlockId::PayloadHash(payload_hash))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_header_range<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path((from, until)): Path<(usize, usize)>,
) -> Result<ApiJson<Vec<S::Header>>, ApiError> {
    state
        .get_header_range(from, until)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_block_by_height<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path(height): Path<u64>,
) -> Result<ApiJson<S::Block>, ApiError> {
    state
        .get_block(v1::BlockId::Height(height))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_block_by_hash<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path(hash): Path<String>,
) -> Result<ApiJson<S::Block>, ApiError> {
    state
        .get_block(v1::BlockId::Hash(hash))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_block_by_payload_hash<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path(payload_hash): Path<String>,
) -> Result<ApiJson<S::Block>, ApiError> {
    state
        .get_block(v1::BlockId::PayloadHash(payload_hash))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_block_range<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path((from, until)): Path<(usize, usize)>,
) -> Result<ApiJson<Vec<S::Block>>, ApiError> {
    state
        .get_block_range(from, until)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_payload_by_height<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path(height): Path<u64>,
) -> Result<ApiJson<S::Payload>, ApiError> {
    state
        .get_payload(v1::PayloadId::Height(height))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_payload_by_hash<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path(hash): Path<String>,
) -> Result<ApiJson<S::Payload>, ApiError> {
    state
        .get_payload(v1::PayloadId::Hash(hash))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_payload_by_block_hash<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path(block_hash): Path<String>,
) -> Result<ApiJson<S::Payload>, ApiError> {
    state
        .get_payload(v1::PayloadId::BlockHash(block_hash))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_payload_range<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path((from, until)): Path<(usize, usize)>,
) -> Result<ApiJson<Vec<S::Payload>>, ApiError> {
    state
        .get_payload_range(from, until)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_vid_common_by_height<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path(height): Path<u64>,
) -> Result<ApiJson<S::VidCommon>, ApiError> {
    state
        .get_vid_common(v1::BlockId::Height(height))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_vid_common_by_hash<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path(hash): Path<String>,
) -> Result<ApiJson<S::VidCommon>, ApiError> {
    state
        .get_vid_common(v1::BlockId::Hash(hash))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_vid_common_by_payload_hash<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path(payload_hash): Path<String>,
) -> Result<ApiJson<S::VidCommon>, ApiError> {
    state
        .get_vid_common(v1::BlockId::PayloadHash(payload_hash))
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_vid_common_range<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path((from, until)): Path<(usize, usize)>,
) -> Result<ApiJson<Vec<S::VidCommon>>, ApiError> {
    state
        .get_vid_common_range(from, until)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_transaction_by_position<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path((height, index)): Path<(u64, u64)>,
) -> Result<ApiJson<S::Transaction>, ApiError> {
    state
        .get_transaction_by_position(height, index)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_transaction_by_hash<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path(hash): Path<String>,
) -> Result<ApiJson<S::Transaction>, ApiError> {
    state
        .get_transaction_by_hash(hash)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

// Serves both `/transaction/{height}/{index}/proof` and the bare `/transaction/{height}/{index}`.
async fn get_transaction_proof_by_position<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path((height, index)): Path<(u64, u64)>,
) -> Result<ApiJson<S::TransactionWithProof>, ApiError> {
    state
        .get_transaction_proof_by_position(height, index)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

// Serves both `/transaction/hash/{hash}/proof` and the bare `/transaction/hash/{hash}`.
async fn get_transaction_proof_by_hash<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path(hash): Path<String>,
) -> Result<ApiJson<S::TransactionWithProof>, ApiError> {
    state
        .get_transaction_proof_by_hash(hash)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_block_summary_by_height<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path(height): Path<usize>,
) -> Result<ApiJson<S::BlockSummary>, ApiError> {
    state
        .get_block_summary(height)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_block_summary_range<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path((from, until)): Path<(usize, usize)>,
) -> Result<ApiJson<Vec<S::BlockSummary>>, ApiError> {
    state
        .get_block_summary_range(from, until)
        .await
        .map(ApiJson)
        .map_err(classify_availability_error)
}

async fn get_limits<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
) -> Result<ApiJson<S::Limits>, ApiError> {
    state
        .get_limits()
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn get_cert2<S: v1::HotShotAvailabilityApi>(
    State(state): State<S>,
    Path(height): Path<u64>,
) -> Result<ApiJson<Option<S::Cert2>>, ApiError> {
    state
        .get_cert2(height)
        .await
        .map(ApiJson)
        .map_err(ApiError::Internal)
}

async fn stream_leaves<S: v1::HotShotAvailabilityApi + Send + Sync + 'static>(
    ws: WebSocketUpgrade,
    State(state): State<S>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response {
    let format = ws_format(&headers);
    ws.on_upgrade(move |socket| async move {
        match state.stream_leaves(height).await {
            Ok(stream) => drive_ws_stream(socket, stream, format).await,
            Err(e) => tracing::warn!("stream_leaves: {e}"),
        }
    })
}

async fn stream_headers<S: v1::HotShotAvailabilityApi + Send + Sync + 'static>(
    ws: WebSocketUpgrade,
    State(state): State<S>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response {
    let format = ws_format(&headers);
    ws.on_upgrade(move |socket| async move {
        match state.stream_headers(height).await {
            Ok(stream) => drive_ws_stream(socket, stream, format).await,
            Err(e) => tracing::warn!("stream_headers: {e}"),
        }
    })
}

async fn stream_blocks<S: v1::HotShotAvailabilityApi + Send + Sync + 'static>(
    ws: WebSocketUpgrade,
    State(state): State<S>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response {
    let format = ws_format(&headers);
    ws.on_upgrade(move |socket| async move {
        match state.stream_blocks(height).await {
            Ok(stream) => drive_ws_stream(socket, stream, format).await,
            Err(e) => tracing::warn!("stream_blocks: {e}"),
        }
    })
}

async fn stream_payloads<S: v1::HotShotAvailabilityApi + Send + Sync + 'static>(
    ws: WebSocketUpgrade,
    State(state): State<S>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response {
    let format = ws_format(&headers);
    ws.on_upgrade(move |socket| async move {
        match state.stream_payloads(height).await {
            Ok(stream) => drive_ws_stream(socket, stream, format).await,
            Err(e) => tracing::warn!("stream_payloads: {e}"),
        }
    })
}

async fn stream_vid_common<S: v1::HotShotAvailabilityApi + Send + Sync + 'static>(
    ws: WebSocketUpgrade,
    State(state): State<S>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response {
    let format = ws_format(&headers);
    ws.on_upgrade(move |socket| async move {
        match state.stream_vid_common(height).await {
            Ok(stream) => drive_ws_stream(socket, stream, format).await,
            Err(e) => tracing::warn!("stream_vid_common: {e}"),
        }
    })
}

async fn stream_transactions<S: v1::HotShotAvailabilityApi + Send + Sync + 'static>(
    ws: WebSocketUpgrade,
    State(state): State<S>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response {
    let format = ws_format(&headers);
    ws.on_upgrade(move |socket| async move {
        match state.stream_transactions(height, None).await {
            Ok(stream) => drive_ws_stream(socket, stream, format).await,
            Err(e) => tracing::warn!("stream_transactions: {e}"),
        }
    })
}

async fn stream_transactions_ns<S: v1::HotShotAvailabilityApi + Send + Sync + 'static>(
    ws: WebSocketUpgrade,
    State(state): State<S>,
    headers: HeaderMap,
    Path((height, namespace)): Path<(usize, u32)>,
) -> Response {
    let format = ws_format(&headers);
    ws.on_upgrade(move |socket| async move {
        match state.stream_transactions(height, Some(namespace)).await {
            Ok(stream) => drive_ws_stream(socket, stream, format).await,
            Err(e) => tracing::warn!("stream_transactions_ns: {e}"),
        }
    })
}

async fn stream_namespace_proofs<S: v1::AvailabilityApi + Send + Sync + 'static>(
    ws: WebSocketUpgrade,
    State(state): State<S>,
    headers: HeaderMap,
    Path((height, namespace)): Path<(usize, u32)>,
) -> Response {
    let format = ws_format(&headers);
    ws.on_upgrade(move |socket| async move {
        match state.stream_namespace_proofs(height, namespace).await {
            Ok(stream) => drive_ws_stream(socket, stream, format).await,
            Err(e) => tracing::warn!("stream_namespace_proofs: {e}"),
        }
    })
}

pub(crate) fn router_availability<S>(state: S) -> ApiRouter
where
    S: v1::AvailabilityApi + v1::HotShotAvailabilityApi + Clone + Send + Sync + 'static,
{
    let availability = ApiRouter::new()
        .api_route(
            "/block/{height}/namespace/{namespace}",
            get_with(get_namespace_proof_by_height::<S>, |op| {
                op.summary("Get namespace proof").description(
                    "Get the transactions in a namespace of the given block, along with a proof \
                     of completeness.",
                )
            }),
        )
        .api_route(
            "/block/hash/{hash}/namespace/{namespace}",
            get_with(get_namespace_proof_by_hash::<S>, |op| {
                op.summary("Get namespace proof").description(
                    "Get the transactions in a namespace of the given block, along with a proof \
                     of completeness.",
                )
            }),
        )
        .api_route(
            "/block/payload-hash/{payload_hash}/namespace/{namespace}",
            get_with(get_namespace_proof_by_payload_hash::<S>, |op| {
                op.summary("Get namespace proof").description(
                    "Get the transactions in a namespace of the given block, along with a proof \
                     of completeness.",
                )
            }),
        )
        .api_route(
            "/block/{from}/{until}/namespace/{namespace}",
            get_with(get_namespace_proof_range::<S>, |op| {
                op.summary("Get namespace proofs for a range").description(
                    "Get the transactions in the specified namespace from each block in a range, \
                     with proofs.",
                )
            }),
        )
        .api_route(
            "/incorrect-encoding-proof/{block_number}/{namespace}",
            get_with(get_incorrect_encoding_proof::<S>, |op| {
                op.summary("Get incorrect-encoding proof").description(
                    "Generate a proof of incorrect namespace encoding for the given block number.",
                )
            }),
        )
        .api_route(
            "/state-cert/{epoch}",
            get_with(get_state_cert_v1::<S>, |op| {
                op.summary("Get state certificate (V1)").description(
                    "Get the light client state update certificate (V1) for the given epoch, used \
                     to update the light client contract's stake table.",
                )
            }),
        )
        .api_route(
            "/state-cert-v2/{epoch}",
            get_with(get_state_cert_v2::<S>, |op| {
                op.summary("Get state certificate (V2)").description(
                    "Get the light client state update certificate (V2) for the given epoch; \
                     includes the auth_root Keccak-256 hash of the reward Merkle tree roots.",
                )
            }),
        )
        .api_route(
            "/leaf/{height}",
            get_with(get_leaf_by_height::<S>, |op| {
                op.summary("Get leaf").description(
                    "Get a leaf by its position in the ledger (0 is genesis) or its hash.",
                )
            }),
        )
        .api_route(
            "/leaf/hash/{hash}",
            get_with(get_leaf_by_hash::<S>, |op| {
                op.summary("Get leaf").description(
                    "Get a leaf by its position in the ledger (0 is genesis) or its hash.",
                )
            }),
        )
        .api_route(
            "/leaf/{from}/{until}",
            get_with(get_leaf_range::<S>, |op| {
                op.summary("Get leaves in range").description(
                    "Get leaves by position in the ledger, from the given `from` up to `until`.",
                )
            }),
        )
        .api_route(
            "/header/{height}",
            get_with(get_header_by_height::<S>, |op| {
                op.summary("Get header").description(
                    "Get a header by its position in the ledger (0 is genesis) or its hash.",
                )
            }),
        )
        .api_route(
            "/header/hash/{hash}",
            get_with(get_header_by_hash::<S>, |op| {
                op.summary("Get header").description(
                    "Get a header by its position in the ledger (0 is genesis) or its hash.",
                )
            }),
        )
        .api_route(
            "/header/payload-hash/{payload_hash}",
            get_with(get_header_by_payload_hash::<S>, |op| {
                op.summary("Get header").description(
                    "Get a header by its position in the ledger (0 is genesis) or its hash.",
                )
            }),
        )
        .api_route(
            "/header/{from}/{until}",
            get_with(get_header_range::<S>, |op| {
                op.summary("Get headers in range").description(
                    "Get headers by position in the ledger, from the given `from` up to `until`.",
                )
            }),
        )
        .api_route(
            "/block/{height}",
            get_with(get_block_by_height::<S>, |op| {
                op.summary("Get block").description(
                    "Get a block (header, payload, hash, size) by its position in the ledger or \
                     its hash.",
                )
            }),
        )
        .api_route(
            "/block/hash/{hash}",
            get_with(get_block_by_hash::<S>, |op| {
                op.summary("Get block").description(
                    "Get a block (header, payload, hash, size) by its position in the ledger or \
                     its hash.",
                )
            }),
        )
        .api_route(
            "/block/payload-hash/{payload_hash}",
            get_with(get_block_by_payload_hash::<S>, |op| {
                op.summary("Get block").description(
                    "Get a block (header, payload, hash, size) by its position in the ledger or \
                     its hash.",
                )
            }),
        )
        .api_route(
            "/block/{from}/{until}",
            get_with(get_block_range::<S>, |op| {
                op.summary("Get blocks in range").description(
                    "Get blocks by position in the ledger, from the given `from` up to `until`.",
                )
            }),
        )
        .api_route(
            "/payload/{height}",
            get_with(get_payload_by_height::<S>, |op| {
                op.summary("Get payload").description(
                    "Get the payload of a block by its position in the ledger or its hash.",
                )
            }),
        )
        .api_route(
            "/payload/hash/{hash}",
            get_with(get_payload_by_hash::<S>, |op| {
                op.summary("Get payload").description(
                    "Get the payload of a block by its position in the ledger or its hash.",
                )
            }),
        )
        .api_route(
            "/payload/block-hash/{block_hash}",
            get_with(get_payload_by_block_hash::<S>, |op| {
                op.summary("Get payload").description(
                    "Get the payload of a block by its position in the ledger or its hash.",
                )
            }),
        )
        .api_route(
            "/payload/{from}/{until}",
            get_with(get_payload_range::<S>, |op| {
                op.summary("Get payloads in range").description(
                    "Get payloads by block position, from the given `from` up to `until`.",
                )
            }),
        )
        .api_route(
            "/vid/common/{height}",
            get_with(get_vid_common_by_height::<S>, |op| {
                op.summary("Get VID common data").description(
                    "Get common VID data for a block; data shared by all storage nodes, not a VID \
                     share.",
                )
            }),
        )
        .api_route(
            "/vid/common/hash/{hash}",
            get_with(get_vid_common_by_hash::<S>, |op| {
                op.summary("Get VID common data").description(
                    "Get common VID data for a block; data shared by all storage nodes, not a VID \
                     share.",
                )
            }),
        )
        .api_route(
            "/vid/common/payload-hash/{payload_hash}",
            get_with(get_vid_common_by_payload_hash::<S>, |op| {
                op.summary("Get VID common data").description(
                    "Get common VID data for a block; data shared by all storage nodes, not a VID \
                     share.",
                )
            }),
        )
        .api_route(
            "/vid/common/{from}/{until}",
            get_with(get_vid_common_range::<S>, |op| {
                op.summary("Get VID common data in range").description(
                    "Get VID common objects by block position, from the given `from` up to \
                     `until`.",
                )
            }),
        )
        .api_route(
            "/transaction/{height}/{index}/noproof",
            get_with(get_transaction_by_position::<S>, |op| {
                op.summary("Get transaction (no proof)").description(
                    "Get a transaction by its index in a block or by its hash, without an \
                     inclusion proof.",
                )
            }),
        )
        .api_route(
            "/transaction/hash/{hash}/noproof",
            get_with(get_transaction_by_hash::<S>, |op| {
                op.summary("Get transaction (no proof)").description(
                    "Get a transaction by its index in a block or by its hash, without an \
                     inclusion proof.",
                )
            }),
        )
        .api_route(
            "/transaction/{height}/{index}/proof",
            get_with(get_transaction_proof_by_position::<S>, |op| {
                op.summary("Get transaction with inclusion proof")
                    .description(
                        "Get a transaction by its index in a block or by its hash, along with an \
                         application-defined inclusion proof.",
                    )
            }),
        )
        .api_route(
            "/transaction/hash/{hash}/proof",
            get_with(get_transaction_proof_by_hash::<S>, |op| {
                op.summary("Get transaction with inclusion proof")
                    .description(
                        "Get a transaction by its index in a block or by its hash, along with an \
                         application-defined inclusion proof.",
                    )
            }),
        )
        .api_route(
            "/transaction/{height}/{index}",
            get_with(get_transaction_proof_by_position::<S>, |op| {
                op.summary("Get transaction with inclusion proof")
                    .description(
                        "Get a transaction by its index in a block or by its hash, along with an \
                         application-defined inclusion proof.",
                    )
            }),
        )
        .api_route(
            "/transaction/hash/{hash}",
            get_with(get_transaction_proof_by_hash::<S>, |op| {
                op.summary("Get transaction with inclusion proof")
                    .description(
                        "Get a transaction by its index in a block or by its hash, along with an \
                         application-defined inclusion proof.",
                    )
            }),
        )
        .api_route(
            "/block/summary/{height}",
            get_with(get_block_summary_by_height::<S>, |op| {
                op.summary("Get block summary").description(
                    "Get the block summary for a block based on its position in the ledger.",
                )
            }),
        )
        .api_route(
            "/block/summaries/{from}/{until}",
            get_with(get_block_summary_range::<S>, |op| {
                op.summary("Get block summaries in range").description(
                    "Get block summaries by position, from the given `from` up to `until`.",
                )
            }),
        )
        .api_route(
            "/limits",
            get_with(get_limits::<S>, |op| {
                op.summary("Get availability limits").description(
                    "Get implementation-defined limits restricting availability range queries \
                     (small/large object range limits).",
                )
            }),
        )
        .api_route(
            "/cert2/{height}",
            get_with(get_cert2::<S>, |op| {
                op.summary("Get finality certificate").description(
                    "Get the finality certificate (Certificate2) at the given block height.",
                )
            }),
        )
        .api_route(
            "/stream/leaves/{height}",
            get_with(stream_leaves::<S>, |op| {
                op.summary("Stream leaves (websocket)").description(
                    "Websocket endpoint: subscribe to a stream of leaves in sequence order, \
                     starting at the given height.",
                )
            }),
        )
        .api_route(
            "/stream/headers/{height}",
            get_with(stream_headers::<S>, |op| {
                op.summary("Stream headers (websocket)").description(
                    "Websocket endpoint: subscribe to a stream of headers in sequence order, \
                     starting at the given height.",
                )
            }),
        )
        .api_route(
            "/stream/blocks/{height}",
            get_with(stream_blocks::<S>, |op| {
                op.summary("Stream blocks (websocket)").description(
                    "Websocket endpoint: subscribe to a stream of blocks in sequence order, \
                     starting at the given height.",
                )
            }),
        )
        .api_route(
            "/stream/payloads/{height}",
            get_with(stream_payloads::<S>, |op| {
                op.summary("Stream payloads (websocket)").description(
                    "Websocket endpoint: subscribe to a stream of block payloads in sequence \
                     order, starting at the given height.",
                )
            }),
        )
        .api_route(
            "/stream/vid/common/{height}",
            get_with(stream_vid_common::<S>, |op| {
                op.summary("Stream VID common data (websocket)")
                    .description(
                        "Websocket endpoint: subscribe to a stream of VID common data in sequence \
                         order, starting at the given height.",
                    )
            }),
        )
        .api_route(
            "/stream/transactions/{height}",
            get_with(stream_transactions::<S>, |op| {
                op.summary("Stream transactions (websocket)").description(
                    "Websocket endpoint: subscribe to a stream of all transactions starting at \
                     the given height.",
                )
            }),
        )
        .api_route(
            "/stream/transactions/{height}/namespace/{namespace}",
            get_with(stream_transactions_ns::<S>, |op| {
                op.summary("Stream namespace transactions (websocket)")
                    .description(
                        "Websocket endpoint: subscribe to a stream of transactions in one \
                         namespace, starting at the given height.",
                    )
            }),
        )
        .api_route(
            "/stream/blocks/{height}/namespace/{namespace}",
            get_with(stream_namespace_proofs::<S>, |op| {
                op.summary("Stream namespace proofs (websocket)")
                    .description(
                        "Websocket endpoint: subscribe to namespace data and proofs for each \
                         block, starting at the given height.",
                    )
            }),
        );

    ApiRouter::new()
        .nest("/availability", availability)
        .with_state(state)
}
