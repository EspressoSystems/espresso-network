//! Axum route registration for every v1 API module.
//!
//! One `router_*` builder per module; each nests its routes under the module's base prefix and
//! registers version-agnostic sub-paths inline, next to the handler. The shared
//! response/encoding/websocket helpers live in the parent [`super`] module ([`mod.rs`](super)).

use super::*;

pub(crate) fn router_reward<S>(state: S) -> ApiRouter
where
    S: v1::RewardApi + Clone + Send + Sync + 'static,
{
    // Create handler closures that capture the generic state type
    let get_latest_reward_balance = |State(state): State<S>, Path(address): Path<String>| async move {
        state
            .get_latest_reward_balance(address)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_latest_reward_account_proof = |State(state): State<S>, Path(address): Path<String>| async move {
        state
            .get_latest_reward_account_proof(address)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    let get_reward_amounts =
        |State(state): State<S>, Path((height, offset, limit)): Path<(u64, u64, u64)>| async move {
            state
                .get_reward_amounts(height, offset, limit)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let get_reward_merkle_tree_v2 = |State(state): State<S>, Path(height): Path<u64>| async move {
        <S as v1::RewardApi>::get_reward_merkle_tree_v2(&state, height)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    // Same underlying V2-tree lookup as `reward-state-v2/reward-balance`; tide registers this
    // route unconditionally for both merklized-state modules regardless of tree version.
    // Merklized-state `get_path` handlers, inherited by both reward mounts from
    // `hotshot-query-service`'s base `state.toml` routes (mirrors router_block_state /
    // router_fee_state below).
    // `/reward-state-v2` is the primary merklized-reward mount.
    let reward_state_v2 = ApiRouter::new()
        .api_route(
            "/reward-claim-input/{height}/{address}",
            get_with(
                |State(state): State<S>, Path((height, address)): Path<(u64, String)>| async move {
                    state
                        .get_reward_claim_input(height, address)
                        .await
                        .map(ApiJson)
                        .map_err(classify_availability_error)
                },
                |op| {
                    op.summary("Get reward claim input").description(
                        "Returns the RewardClaimInput needed to call claimRewards() on L1: \
                         lifetime rewards, Merkle proof, and auth root inputs, for the account at \
                         the given block height finalized by the light client contract.",
                    )
                },
            ),
        )
        .api_route(
            "/reward-balance/{height}/{address}",
            get_with(
                |State(state): State<S>, Path((height, address)): Path<(u64, String)>| async move {
                    state
                        .get_reward_balance(height, address)
                        .await
                        .map(ApiJson)
                        .map_err(classify_availability_error)
                },
                |op| {
                    op.summary("Get reward balance at height").description(
                        "Get balance in reward state at a specific height for an Ethereum address.",
                    )
                },
            ),
        )
        .api_route(
            "/reward-balance/latest/{address}",
            get_with(get_latest_reward_balance, |op| {
                op.summary("Get latest reward balance")
                    .description("Get current balance in reward state for an Ethereum address.")
            }),
        )
        .api_route(
            "/proof/{height}/{address}",
            get_with(
                |State(state): State<S>, Path((height, address)): Path<(u64, String)>| async move {
                    state
                        .get_reward_account_proof(height, address)
                        .await
                        .map(ApiJson)
                        .map_err(classify_availability_error)
                },
                |op| {
                    op.summary("Get reward account proof").description(
                        "Get the Merkle proof for a reward account at a given block height \
                         (RewardAccountProofV1 pre-V4, RewardAccountProofV2 from V4 onward).",
                    )
                },
            ),
        )
        .api_route(
            "/proof/latest/{address}",
            get_with(get_latest_reward_account_proof, |op| {
                op.summary("Get latest reward account proof").description(
                    "Get the Merkle proof (RewardAccountProofV2) for a reward account at the \
                     latest block height finalized by the light client contract.",
                )
            }),
        )
        .api_route(
            "/reward-amounts/{height}/{offset}/{limit}",
            get_with(get_reward_amounts, |op| {
                op.summary("List reward amounts").description(
                    "Return all RewardMerkleTreeV2 accounts stored for the requested height, \
                     paginated by offset and limit (limit must be <= 10000).",
                )
            }),
        )
        .api_route(
            "/reward-merkle-tree-v2/{height}",
            get_with(get_reward_merkle_tree_v2, |op| {
                op.summary("Get RewardMerkleTreeV2 snapshot").description(
                    "Get the snapshot of this node's RewardMerkleTreeV2 at the given block \
                     height, serialized as RewardMerkleTreeV2Data.",
                )
            }),
        )
        .api_route(
            "/block-height",
            get_with(
                |State(state): State<S>| async move {
                    state
                        .get_reward_state_v2_height()
                        .await
                        .map(ApiJson)
                        .map_err(classify_availability_error)
                },
                |op| {
                    op.summary("Get reward-state-v2 block height").description(
                        "Latest block height for which the merklized reward state (V2) is \
                         available.",
                    )
                },
            ),
        )
        .api_route(
            "/{height}/{key}",
            get_with(
                |State(state): State<S>, Path((height, key)): Path<(u64, String)>| async move {
                    <S as v1::RewardApi>::get_reward_state_path_v2(
                        &state,
                        v1::Snapshot::Height(height),
                        key,
                    )
                    .await
                    .map(ApiJson)
                    .map_err(classify_availability_error)
                },
                |op| {
                    op.summary("Get reward-state-v2 Merkle path by height")
                        .description(
                            "Retrieve the Merkle path for the membership proof of a leaf in the \
                             reward-state-v2 tree, by block height and key.",
                        )
                },
            ),
        )
        .api_route(
            "/commit/{commit}/{key}",
            get_with(
                |State(state): State<S>, Path((commit, key)): Path<(String, String)>| async move {
                    <S as v1::RewardApi>::get_reward_state_path_v2(
                        &state,
                        v1::Snapshot::Commit(commit),
                        key,
                    )
                    .await
                    .map(ApiJson)
                    .map_err(classify_availability_error)
                },
                |op| {
                    op.summary("Get reward-state-v2 Merkle path by commitment")
                        .description(
                            "Retrieve the Merkle path for the membership proof of a leaf in the \
                             reward-state-v2 tree, by tree commitment and key.",
                        )
                },
            ),
        );

    // `/reward-state` mirrors the V2 mount: tide-disco shared these handlers across both
    // merklized-state modules, so the same routes are served under this prefix too.
    let reward_state = ApiRouter::new()
        .api_route(
            "/block-height",
            get_with(
                |State(state): State<S>| async move {
                    state
                        .get_reward_state_height()
                        .await
                        .map(ApiJson)
                        .map_err(classify_availability_error)
                },
                |op| {
                    op.summary("Get reward-state block height").description(
                        "Latest block height for which the merklized reward state (V1) is \
                         available.",
                    )
                },
            ),
        )
        .api_route(
            "/reward-balance/{height}/{address}",
            get_with(
                |State(state): State<S>, Path((height, address)): Path<(u64, String)>| async move {
                    state
                        .get_reward_balance(height, address)
                        .await
                        .map(ApiJson)
                        .map_err(classify_availability_error)
                },
                |op| {
                    op.summary("Get reward balance at height (v1 mount)")
                        .description(
                            "Same handler as reward-state-v2/reward-balance, registered on the \
                             reward-state mount; tide-disco shared this handler across both \
                             merklized-state mounts.",
                        )
                },
            ),
        )
        .api_route(
            "/proof/{height}/{address}",
            get_with(
                |State(state): State<S>, Path((height, address)): Path<(u64, String)>| async move {
                    state
                        .get_reward_account_proof_v1(height, address)
                        .await
                        .map(ApiJson)
                        .map_err(classify_availability_error)
                },
                |op| {
                    op.summary("Get reward account proof (v1 mount)")
                        .description(
                            "Same handler as reward-state-v2/proof, registered on the \
                             reward-state mount; tide-disco shared this handler across both \
                             merklized-state mounts.",
                        )
                },
            ),
        )
        .api_route(
            "/reward-balance/latest/{address}",
            get_with(get_latest_reward_balance, |op| {
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
            get_with(get_latest_reward_account_proof, |op| {
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
            get_with(get_reward_amounts, |op| {
                op.summary("List reward amounts (v1 mount)").description(
                    "Same handler as reward-state-v2/reward-amounts, registered on the \
                     reward-state mount; tide-disco shared this handler across both \
                     merklized-state mounts.",
                )
            }),
        )
        .api_route(
            "/reward-merkle-tree-v2/{height}",
            get_with(get_reward_merkle_tree_v2, |op| {
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
            get_with(
                |State(state): State<S>, Path((height, key)): Path<(u64, String)>| async move {
                    <S as v1::RewardApi>::get_reward_state_path_v1(
                        &state,
                        v1::Snapshot::Height(height),
                        key,
                    )
                    .await
                    .map(ApiJson)
                    .map_err(classify_availability_error)
                },
                |op| {
                    op.summary("Get reward-state Merkle path by height")
                        .description(
                            "Retrieve the Merkle path for the membership proof of a leaf in the \
                             reward-state (V1) tree, by block height and key.",
                        )
                },
            ),
        )
        .api_route(
            "/commit/{commit}/{key}",
            get_with(
                |State(state): State<S>, Path((commit, key)): Path<(String, String)>| async move {
                    <S as v1::RewardApi>::get_reward_state_path_v1(
                        &state,
                        v1::Snapshot::Commit(commit),
                        key,
                    )
                    .await
                    .map(ApiJson)
                    .map_err(classify_availability_error)
                },
                |op| {
                    op.summary("Get reward-state Merkle path by commitment")
                        .description(
                            "Retrieve the Merkle path for the membership proof of a leaf in the \
                             reward-state (V1) tree, by tree commitment and key.",
                        )
                },
            ),
        );

    ApiRouter::new()
        .nest("/reward-state-v2", reward_state_v2)
        .nest("/reward-state", reward_state)
        .with_state(state)
}

pub(crate) fn router_availability<S>(state: S) -> ApiRouter
where
    S: v1::AvailabilityApi + v1::HotShotAvailabilityApi + Clone + Send + Sync + 'static,
{
    // Availability API handlers
    // Route: /v1/availability/block/{height}/namespace/{namespace}
    // Route: /v1/availability/block/hash/{hash}/namespace/{namespace}
    // Route: /v1/availability/block/payload-hash/{payload-hash}/namespace/{namespace}
    // Route: /v1/availability/block/{from}/{until}/namespace/{namespace}
    // HotShot availability API handlers
    let get_transaction_proof_by_position =
        |State(state): State<S>, Path((height, index)): Path<(u64, u64)>| async move {
            state
                .get_transaction_proof_by_position(height, index)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        };

    let get_transaction_proof_by_hash = |State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_transaction_proof_by_hash(hash)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    // WebSocket streaming handlers
    ApiRouter::new()
        .nest(
            "/availability",
            ApiRouter::new()
        .api_route(
            "/block/{height}/namespace/{namespace}",
            get_with(|State(state): State<S>, Path((height, namespace)): Path<(u64, u32)>| async move {
            state
                .get_namespace_proof(v1::availability::BlockId::Height(height), namespace)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get namespace proof").description(
                    "Get the transactions in a namespace of the given block, along with a proof \
                     of completeness.",
                )
            }),
        )
        .api_route(
            "/block/hash/{hash}/namespace/{namespace}",
            get_with(|State(state): State<S>, Path((hash, namespace)): Path<(String, u32)>| async move {
            state
                .get_namespace_proof(v1::availability::BlockId::Hash(hash), namespace)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get namespace proof").description(
                    "Get the transactions in a namespace of the given block, along with a proof \
                     of completeness.",
                )
            }),
        )
        .api_route(
            "/block/payload-hash/{payload_hash}/namespace/{namespace}",
            get_with(|State(state): State<S>, Path((payload_hash, namespace)): Path<(String, u32)>| async move {
            state
                .get_namespace_proof(
                    v1::availability::BlockId::PayloadHash(payload_hash),
                    namespace,
                )
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get namespace proof").description(
                    "Get the transactions in a namespace of the given block, along with a proof \
                     of completeness.",
                )
            }),
        )
        .api_route(
            "/block/{from}/{until}/namespace/{namespace}",
            get_with(|State(state): State<S>, Path((from, until, namespace)): Path<(u64, u64, u32)>| async move {
            state
                .get_namespace_proof_range(from, until, namespace)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get namespace proofs for a range").description(
                    "Get the transactions in the specified namespace from each block in a range, \
                     with proofs.",
                )
            }),
        )
        .api_route(
            "/incorrect-encoding-proof/{block_number}/{namespace}",
            get_with(|State(state): State<S>, Path((block_number, namespace)): Path<(u64, u32)>| async move {
            state
                .get_incorrect_encoding_proof(
                    v1::availability::BlockId::Height(block_number),
                    namespace,
                )
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get incorrect-encoding proof").description(
                    "Generate a proof of incorrect namespace encoding for the given block number.",
                )
            }),
        )
        .api_route(
            "/state-cert/{epoch}",
            get_with(|State(state): State<S>, Path(epoch): Path<u64>| async move {
        <S as v1::AvailabilityApi>::get_state_cert(&state, epoch)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get state certificate (V1)").description(
                    "Get the light client state update certificate (V1) for the given epoch, used \
                     to update the light client contract's stake table.",
                )
            }),
        )
        .api_route(
            "/state-cert-v2/{epoch}",
            get_with(|State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .get_state_cert_v2(epoch)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get state certificate (V2)").description(
                    "Get the light client state update certificate (V2) for the given epoch; \
                     includes the auth_root Keccak-256 hash of the reward Merkle tree roots.",
                )
            }),
        )
        .api_route(
            "/leaf/{height}",
            get_with(|State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_leaf(v1::LeafId::Height(height))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get leaf").description(
                    "Get a leaf by its position in the ledger (0 is genesis) or its hash.",
                )
            }),
        )
        .api_route(
            "/leaf/hash/{hash}",
            get_with(|State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_leaf(v1::LeafId::Hash(hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get leaf").description(
                    "Get a leaf by its position in the ledger (0 is genesis) or its hash.",
                )
            }),
        )
        .api_route(
            "/leaf/{from}/{until}",
            get_with(|State(state): State<S>, Path((from, until)): Path<(usize, usize)>| async move {
        state
            .get_leaf_range(from, until)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get leaves in range").description(
                    "Get leaves by position in the ledger, from the given `from` up to `until`.",
                )
            }),
        )
        .api_route(
            "/header/{height}",
            get_with(|State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_header(v1::BlockId::Height(height))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get header").description(
                    "Get a header by its position in the ledger (0 is genesis) or its hash.",
                )
            }),
        )
        .api_route(
            "/header/hash/{hash}",
            get_with(|State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_header(v1::BlockId::Hash(hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get header").description(
                    "Get a header by its position in the ledger (0 is genesis) or its hash.",
                )
            }),
        )
        .api_route(
            "/header/payload-hash/{payload_hash}",
            get_with(|State(state): State<S>, Path(payload_hash): Path<String>| async move {
        state
            .get_header(v1::BlockId::PayloadHash(payload_hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get header").description(
                    "Get a header by its position in the ledger (0 is genesis) or its hash.",
                )
            }),
        )
        .api_route(
            "/header/{from}/{until}",
            get_with(|State(state): State<S>, Path((from, until)): Path<(usize, usize)>| async move {
        state
            .get_header_range(from, until)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get headers in range").description(
                    "Get headers by position in the ledger, from the given `from` up to `until`.",
                )
            }),
        )
        .api_route(
            "/block/{height}",
            get_with(|State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_block(v1::BlockId::Height(height))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get block").description(
                    "Get a block (header, payload, hash, size) by its position in the ledger or \
                     its hash.",
                )
            }),
        )
        .api_route(
            "/block/hash/{hash}",
            get_with(|State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_block(v1::BlockId::Hash(hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get block").description(
                    "Get a block (header, payload, hash, size) by its position in the ledger or \
                     its hash.",
                )
            }),
        )
        .api_route(
            "/block/payload-hash/{payload_hash}",
            get_with(|State(state): State<S>, Path(payload_hash): Path<String>| async move {
        state
            .get_block(v1::BlockId::PayloadHash(payload_hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get block").description(
                    "Get a block (header, payload, hash, size) by its position in the ledger or \
                     its hash.",
                )
            }),
        )
        .api_route(
            "/block/{from}/{until}",
            get_with(|State(state): State<S>, Path((from, until)): Path<(usize, usize)>| async move {
        state
            .get_block_range(from, until)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get blocks in range").description(
                    "Get blocks by position in the ledger, from the given `from` up to `until`.",
                )
            }),
        )
        .api_route(
            "/payload/{height}",
            get_with(|State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_payload(v1::PayloadId::Height(height))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get payload").description(
                    "Get the payload of a block by its position in the ledger or its hash.",
                )
            }),
        )
        .api_route(
            "/payload/hash/{hash}",
            get_with(|State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_payload(v1::PayloadId::Hash(hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get payload").description(
                    "Get the payload of a block by its position in the ledger or its hash.",
                )
            }),
        )
        .api_route(
            "/payload/block-hash/{block_hash}",
            get_with(|State(state): State<S>, Path(block_hash): Path<String>| async move {
        state
            .get_payload(v1::PayloadId::BlockHash(block_hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get payload").description(
                    "Get the payload of a block by its position in the ledger or its hash.",
                )
            }),
        )
        .api_route(
            "/payload/{from}/{until}",
            get_with(|State(state): State<S>, Path((from, until)): Path<(usize, usize)>| async move {
        state
            .get_payload_range(from, until)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get payloads in range").description(
                    "Get payloads by block position, from the given `from` up to `until`.",
                )
            }),
        )
        .api_route(
            "/vid/common/{height}",
            get_with(|State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_vid_common(v1::BlockId::Height(height))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get VID common data").description(
                    "Get common VID data for a block; data shared by all storage nodes, not a VID \
                     share.",
                )
            }),
        )
        .api_route(
            "/vid/common/hash/{hash}",
            get_with(|State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_vid_common(v1::BlockId::Hash(hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get VID common data").description(
                    "Get common VID data for a block; data shared by all storage nodes, not a VID \
                     share.",
                )
            }),
        )
        .api_route(
            "/vid/common/payload-hash/{payload_hash}",
            get_with(|State(state): State<S>, Path(payload_hash): Path<String>| async move {
            state
                .get_vid_common(v1::BlockId::PayloadHash(payload_hash))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get VID common data").description(
                    "Get common VID data for a block; data shared by all storage nodes, not a VID \
                     share.",
                )
            }),
        )
        .api_route(
            "/vid/common/{from}/{until}",
            get_with(|State(state): State<S>, Path((from, until)): Path<(usize, usize)>| async move {
            state
                .get_vid_common_range(from, until)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get VID common data in range").description(
                    "Get VID common objects by block position, from the given `from` up to \
                     `until`.",
                )
            }),
        )
        .api_route(
            "/transaction/{height}/{index}/noproof",
            get_with(|State(state): State<S>, Path((height, index)): Path<(u64, u64)>| async move {
            state
                .get_transaction_by_position(height, index)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get transaction (no proof)").description(
                    "Get a transaction by its index in a block or by its hash, without an \
                     inclusion proof.",
                )
            }),
        )
        .api_route(
            "/transaction/hash/{hash}/noproof",
            get_with(|State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_transaction_by_hash(hash)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get transaction (no proof)").description(
                    "Get a transaction by its index in a block or by its hash, without an \
                     inclusion proof.",
                )
            }),
        )
        .api_route(
            "/transaction/{height}/{index}/proof",
            get_with(get_transaction_proof_by_position, |op| {
                op.summary("Get transaction with inclusion proof")
                    .description(
                        "Get a transaction by its index in a block or by its hash, along with an \
                         application-defined inclusion proof.",
                    )
            }),
        )
        .api_route(
            "/transaction/hash/{hash}/proof",
            get_with(get_transaction_proof_by_hash, |op| {
                op.summary("Get transaction with inclusion proof")
                    .description(
                        "Get a transaction by its index in a block or by its hash, along with an \
                         application-defined inclusion proof.",
                    )
            }),
        )
        .api_route(
            "/transaction/{height}/{index}",
            get_with(get_transaction_proof_by_position, |op| {
                op.summary("Get transaction with inclusion proof")
                    .description(
                        "Get a transaction by its index in a block or by its hash, along with an \
                         application-defined inclusion proof.",
                    )
            }),
        )
        .api_route(
            "/transaction/hash/{hash}",
            get_with(get_transaction_proof_by_hash, |op| {
                op.summary("Get transaction with inclusion proof")
                    .description(
                        "Get a transaction by its index in a block or by its hash, along with an \
                         application-defined inclusion proof.",
                    )
            }),
        )
        .api_route(
            "/block/summary/{height}",
            get_with(|State(state): State<S>, Path(height): Path<usize>| async move {
        state
            .get_block_summary(height)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get block summary").description(
                    "Get the block summary for a block based on its position in the ledger.",
                )
            }),
        )
        .api_route(
            "/block/summaries/{from}/{until}",
            get_with(|State(state): State<S>, Path((from, until)): Path<(usize, usize)>| async move {
            state
                .get_block_summary_range(from, until)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get block summaries in range").description(
                    "Get block summaries by position, from the given `from` up to `until`.",
                )
            }),
        )
        .api_route(
            "/limits",
            get_with(|State(state): State<S>| async move {
        state
            .get_limits()
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    }, |op| {
                op.summary("Get availability limits").description(
                    "Get implementation-defined limits restricting availability range queries \
                     (small/large object range limits).",
                )
            }),
        )
        .api_route(
            "/cert2/{height}",
            get_with(|State(state): State<S>, Path(height): Path<u64>| async move {
        <S as v1::HotShotAvailabilityApi>::get_cert2(&state, height)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    }, |op| {
                op.summary("Get finality certificate").description(
                    "Get the finality certificate (Certificate2) at the given block height.",
                )
            }),
        )
        .api_route(
            "/stream/leaves/{height}",
            get_with(|ws: WebSocketUpgrade,
                         State(state): State<S>,
                         headers: HeaderMap,
                         Path(height): Path<usize>| async move {
        let format = ws_format(&headers);
        ws.on_upgrade(move |socket| async move {
            match state.stream_leaves(height).await {
                Ok(stream) => drive_ws_stream(socket, stream, format).await,
                Err(e) => tracing::warn!("stream_leaves: {e}"),
            }
        })
    }, |op| {
                op.summary("Stream leaves (websocket)").description(
                    "Websocket endpoint: subscribe to a stream of leaves in sequence order, \
                     starting at the given height.",
                )
            }),
        )
        .api_route(
            "/stream/headers/{height}",
            get_with(|ws: WebSocketUpgrade,
                          State(state): State<S>,
                          headers: HeaderMap,
                          Path(height): Path<usize>| async move {
        let format = ws_format(&headers);
        ws.on_upgrade(move |socket| async move {
            match state.stream_headers(height).await {
                Ok(stream) => drive_ws_stream(socket, stream, format).await,
                Err(e) => tracing::warn!("stream_headers: {e}"),
            }
        })
    }, |op| {
                op.summary("Stream headers (websocket)").description(
                    "Websocket endpoint: subscribe to a stream of headers in sequence order, \
                     starting at the given height.",
                )
            }),
        )
        .api_route(
            "/stream/blocks/{height}",
            get_with(|ws: WebSocketUpgrade,
                         State(state): State<S>,
                         headers: HeaderMap,
                         Path(height): Path<usize>| async move {
        let format = ws_format(&headers);
        ws.on_upgrade(move |socket| async move {
            match state.stream_blocks(height).await {
                Ok(stream) => drive_ws_stream(socket, stream, format).await,
                Err(e) => tracing::warn!("stream_blocks: {e}"),
            }
        })
    }, |op| {
                op.summary("Stream blocks (websocket)").description(
                    "Websocket endpoint: subscribe to a stream of blocks in sequence order, \
                     starting at the given height.",
                )
            }),
        )
        .api_route(
            "/stream/payloads/{height}",
            get_with(|ws: WebSocketUpgrade,
                           State(state): State<S>,
                           headers: HeaderMap,
                           Path(height): Path<usize>| async move {
        let format = ws_format(&headers);
        ws.on_upgrade(move |socket| async move {
            match state.stream_payloads(height).await {
                Ok(stream) => drive_ws_stream(socket, stream, format).await,
                Err(e) => tracing::warn!("stream_payloads: {e}"),
            }
        })
    }, |op| {
                op.summary("Stream payloads (websocket)").description(
                    "Websocket endpoint: subscribe to a stream of block payloads in sequence \
                     order, starting at the given height.",
                )
            }),
        )
        .api_route(
            "/stream/vid/common/{height}",
            get_with(|ws: WebSocketUpgrade,
                             State(state): State<S>,
                             headers: HeaderMap,
                             Path(height): Path<usize>| async move {
        let format = ws_format(&headers);
        ws.on_upgrade(move |socket| async move {
            match state.stream_vid_common(height).await {
                Ok(stream) => drive_ws_stream(socket, stream, format).await,
                Err(e) => tracing::warn!("stream_vid_common: {e}"),
            }
        })
    }, |op| {
                op.summary("Stream VID common data (websocket)")
                    .description(
                        "Websocket endpoint: subscribe to a stream of VID common data in sequence \
                         order, starting at the given height.",
                    )
            }),
        )
        .api_route(
            "/stream/transactions/{height}",
            get_with(|ws: WebSocketUpgrade,
                               State(state): State<S>,
                               headers: HeaderMap,
                               Path(height): Path<usize>| async move {
        let format = ws_format(&headers);
        ws.on_upgrade(move |socket| async move {
            match state.stream_transactions(height, None).await {
                Ok(stream) => drive_ws_stream(socket, stream, format).await,
                Err(e) => tracing::warn!("stream_transactions: {e}"),
            }
        })
    }, |op| {
                op.summary("Stream transactions (websocket)").description(
                    "Websocket endpoint: subscribe to a stream of all transactions starting at \
                     the given height.",
                )
            }),
        )
        .api_route(
            "/stream/transactions/{height}/namespace/{namespace}",
            get_with(|ws: WebSocketUpgrade,
         State(state): State<S>,
         headers: HeaderMap,
         Path((height, namespace)): Path<(usize, u32)>| async move {
            let format = ws_format(&headers);
            ws.on_upgrade(move |socket| async move {
                match state.stream_transactions(height, Some(namespace)).await {
                    Ok(stream) => drive_ws_stream(socket, stream, format).await,
                    Err(e) => tracing::warn!("stream_transactions_ns: {e}"),
                }
            })
        }, |op| {
                op.summary("Stream namespace transactions (websocket)")
                    .description(
                        "Websocket endpoint: subscribe to a stream of transactions in one \
                         namespace, starting at the given height.",
                    )
            }),
        )
        .api_route(
            "/stream/blocks/{height}/namespace/{namespace}",
            get_with(|ws: WebSocketUpgrade,
         State(state): State<S>,
         headers: HeaderMap,
         Path((height, namespace)): Path<(usize, u32)>| async move {
            let format = ws_format(&headers);
            ws.on_upgrade(move |socket| async move {
                match state.stream_namespace_proofs(height, namespace).await {
                    Ok(stream) => drive_ws_stream(socket, stream, format).await,
                    Err(e) => tracing::warn!("stream_namespace_proofs: {e}"),
                }
            })
        }, |op| {
                op.summary("Stream namespace proofs (websocket)")
                    .description(
                        "Websocket endpoint: subscribe to namespace data and proofs for each \
                         block, starting at the given height.",
                    )
            }),
        )
        )
        .with_state(state)
}

pub(crate) fn router_block_state<S>(state: S) -> ApiRouter
where
    S: v1::BlockStateApi + Clone + Send + Sync + 'static,
{
    // Merklized state handlers: block-state
    ApiRouter::new()
        .nest(
            "/block-state",
            ApiRouter::new()
        .api_route(
            "/block-height",
            get_with(|State(state): State<S>| async move {
        <S as v1::BlockStateApi>::get_block_state_height(&state)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get block-state height").description(
                    "Latest block height for which the merklized blocks-Merkle-tree state is \
                     available.",
                )
            }),
        )
        .api_route(
            "/commit/{commit}/{key}",
            get_with(|State(state): State<S>, Path((commit, key)): Path<(String, String)>| async move {
            <S as v1::BlockStateApi>::get_block_state_path(
                &state,
                v1::Snapshot::Commit(commit),
                key,
            )
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get block-state Merkle path by commitment")
                    .description(
                        "Retrieve the Merkle path for a leaf in the blocks Merkle tree, by tree \
                         commitment and key.",
                    )
            }),
        )
        .api_route(
            "/{height}/{key}",
            get_with(|State(state): State<S>, Path((height, key)): Path<(u64, String)>| async move {
            <S as v1::BlockStateApi>::get_block_state_path(
                &state,
                v1::Snapshot::Height(height),
                key,
            )
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get block-state Merkle path by height")
                    .description(
                        "Retrieve the Merkle path for a leaf in the blocks Merkle tree, by block \
                         height and key.",
                    )
            }),
        )
        )
        .with_state(state)
}

pub(crate) fn router_fee_state<S>(state: S) -> ApiRouter
where
    S: v1::FeeStateApi + Clone + Send + Sync + 'static,
{
    // Merklized state handlers: fee-state
    ApiRouter::new()
        .nest(
            "/fee-state",
            ApiRouter::new()
        .api_route(
            "/block-height",
            get_with(|State(state): State<S>| async move {
        <S as v1::FeeStateApi>::get_fee_state_height(&state)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get fee-state height").description(
                    "Latest block height for which the merklized fee state is available.",
                )
            }),
        )
        .api_route(
            "/fee-balance/latest/{address}",
            get_with(|State(state): State<S>, Path(address): Path<String>| async move {
        state
            .get_fee_balance_latest(address)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get latest fee balance").description(
                    "Get the latest fee account balance for an address from the fee Merkle tree.",
                )
            }),
        )
        .api_route(
            "/commit/{commit}/{key}",
            get_with(|State(state): State<S>, Path((commit, key)): Path<(String, String)>| async move {
            <S as v1::FeeStateApi>::get_fee_state_path(&state, v1::Snapshot::Commit(commit), key)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get fee-state Merkle path by commitment")
                    .description(
                        "Retrieve the Merkle path for a leaf in the fee state tree, by tree \
                         commitment and key.",
                    )
            }),
        )
        .api_route(
            "/{height}/{key}",
            get_with(|State(state): State<S>, Path((height, key)): Path<(u64, String)>| async move {
            <S as v1::FeeStateApi>::get_fee_state_path(&state, v1::Snapshot::Height(height), key)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get fee-state Merkle path by height")
                    .description(
                        "Retrieve the Merkle path for a leaf in the fee state tree, by block \
                         height and key.",
                    )
            }),
        )
        )
        .with_state(state)
}

pub(crate) fn router_status<S>(state: S) -> ApiRouter
where
    S: v1::StatusApi + Clone + Send + Sync + 'static,
{
    let routes = ApiRouter::new()
        .api_route(
            "/block-height",
            get_with(
                |State(state): State<S>| async move {
                    state
                        .block_height()
                        .await
                        .map(ApiJson)
                        .map_err(ApiError::Internal)
                },
                |op| {
                    op.summary("Get latest committed block height")
                        .description("Get the height of the latest committed block.")
                },
            ),
        )
        .api_route(
            "/success-rate",
            get_with(
                |State(state): State<S>| async move {
                    state
                        .success_rate()
                        .await
                        .map(ApiJson)
                        .map_err(ApiError::Internal)
                },
                |op| {
                    op.summary("Get view success rate").description(
                        "Get the fraction of views which resulted in a committed block.",
                    )
                },
            ),
        )
        .api_route(
            "/time-since-last-decide",
            get_with(
                |State(state): State<S>| async move {
                    state
                        .time_since_last_decide()
                        .await
                        .map(ApiJson)
                        .map_err(ApiError::Internal)
                },
                |op| {
                    op.summary("Get time since last decide")
                        .description("Get the time elapsed in seconds since the last decided view.")
                },
            ),
        )
        .api_route(
            "/metrics",
            get_with(
                |State(state): State<S>| async move {
                    match state.metrics().await {
                        Ok(text) => (
                            [(
                                axum::http::header::CONTENT_TYPE,
                                "text/plain; charset=utf-8",
                            )],
                            text,
                        )
                            .into_response(),
                        Err(e) => ApiError::Internal(e).into_response(),
                    }
                },
                |op| {
                    op.summary("Get Prometheus metrics")
                        .description("Prometheus endpoint exposing consensus-related metrics.")
                },
            ),
        );

    ApiRouter::new().nest("/status", routes).with_state(state)
}

pub(crate) fn router_config<S>(state: S) -> ApiRouter
where
    S: v1::ConfigApi + Clone + Send + Sync + 'static,
{
    ApiRouter::new()
        .nest(
            "/config",
            ApiRouter::new()
                .api_route(
                    "/hotshot",
                    get_with(
                        |State(state): State<S>| async move {
                            <S as v1::ConfigApi>::hotshot_config(&state)
                                .await
                                .map(ApiJson)
                                .map_err(ApiError::Internal)
                        },
                        |op| {
                            op.summary("Get HotShot config")
                                .description("Get the HotShot configuration for the current node.")
                        },
                    ),
                )
                .api_route(
                    "/env",
                    get_with(
                        |State(state): State<S>| async move {
                            <S as v1::ConfigApi>::env(&state)
                                .await
                                .map(ApiJson)
                                .map_err(ApiError::Internal)
                        },
                        |op| {
                            op.summary("Get environment variables").description(
                                "Get all ESPRESSO_ environment variables set for the current node.",
                            )
                        },
                    ),
                )
                .api_route(
                    "/runtime",
                    get_with(
                        |State(state): State<S>| async move {
                            <S as v1::ConfigApi>::runtime_config(&state)
                                .await
                                .map(ApiJson)
                                .map_err(classify_availability_error)
                        },
                        |op| {
                            op.summary("Get runtime config").description(
                                "Get the merged runtime configuration (CLI flags + env vars + \
                                 defaults); secrets and L1 RPC URLs are redacted.",
                            )
                        },
                    ),
                ),
        )
        .with_state(state)
}

pub(crate) fn router_node<S>(state: S) -> ApiRouter
where
    S: v1::NodeApi + Clone + Send + Sync + 'static,
{
    let node_payload_size = |State(state): State<S>| async move {
        state
            .payload_size(None, None, None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    };

    ApiRouter::new()
        .nest(
            "/node",
            ApiRouter::new()
        .api_route(
            "/block-height",
            get_with(|State(state): State<S>| async move {
        <S as v1::NodeApi>::block_height(&state)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get node's block height")
                    .description("The current height of the chain, as observed by this node.")
            }),
        )
        .api_route(
            "/transactions/count",
            get_with(|State(state): State<S>| async move {
        state
            .count_transactions(None, None, None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Count transactions").description(
                    "Get the number of transactions in the chain, optionally restricted by block \
                     range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/count/namespace/{namespace}",
            get_with(|State(state): State<S>, Path(namespace): Path<u64>| async move {
        state
            .count_transactions(None, None, Some(namespace))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Count transactions").description(
                    "Get the number of transactions in the chain, optionally restricted by block \
                     range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/count/namespace/{namespace}/{to}",
            get_with(|State(state): State<S>, Path((namespace, to)): Path<(u64, u64)>| async move {
        state
            .count_transactions(None, Some(to), Some(namespace))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Count transactions").description(
                    "Get the number of transactions in the chain, optionally restricted by block \
                     range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/count/namespace/{namespace}/{from}/{to}",
            get_with(|State(state): State<S>, Path((namespace, from, to)): Path<(u64, u64, u64)>| async move {
            state
                .count_transactions(Some(from), Some(to), Some(namespace))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Count transactions").description(
                    "Get the number of transactions in the chain, optionally restricted by block \
                     range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/count/{to}",
            get_with(|State(state): State<S>, Path(to): Path<u64>| async move {
        state
            .count_transactions(None, Some(to), None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Count transactions").description(
                    "Get the number of transactions in the chain, optionally restricted by block \
                     range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/count/{from}/{to}",
            get_with(|State(state): State<S>, Path((from, to)): Path<(u64, u64)>| async move {
        state
            .count_transactions(Some(from), Some(to), None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Count transactions").description(
                    "Get the number of transactions in the chain, optionally restricted by block \
                     range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/payloads/size",
            get_with(node_payload_size, |op| {
                op.summary("Get payload size").description(
                    "Get the cumulative size (bytes) of payload data in the chain, optionally \
                     restricted by block range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/payloads/total-size",
            get_with(node_payload_size, |op| {
                op.summary("Get payload size")
                    .description("Deprecated alias for payloads/size.")
            }),
        )
        .api_route(
            "/payloads/size/namespace/{namespace}",
            get_with(|State(state): State<S>, Path(namespace): Path<u64>| async move {
        state
            .payload_size(None, None, Some(namespace))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get payload size").description(
                    "Get the cumulative size (bytes) of payload data in the chain, optionally \
                     restricted by block range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/payloads/size/namespace/{namespace}/{to}",
            get_with(|State(state): State<S>, Path((namespace, to)): Path<(u64, u64)>| async move {
            state
                .payload_size(None, Some(to), Some(namespace))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get payload size").description(
                    "Get the cumulative size (bytes) of payload data in the chain, optionally \
                     restricted by block range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/payloads/size/namespace/{namespace}/{from}/{to}",
            get_with(|State(state): State<S>, Path((namespace, from, to)): Path<(u64, u64, u64)>| async move {
            state
                .payload_size(Some(from), Some(to), Some(namespace))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get payload size").description(
                    "Get the cumulative size (bytes) of payload data in the chain, optionally \
                     restricted by block range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/payloads/size/{to}",
            get_with(|State(state): State<S>, Path(to): Path<u64>| async move {
        state
            .payload_size(None, Some(to), None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get payload size").description(
                    "Get the cumulative size (bytes) of payload data in the chain, optionally \
                     restricted by block range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/payloads/size/{from}/{to}",
            get_with(|State(state): State<S>, Path((from, to)): Path<(u64, u64)>| async move {
        state
            .payload_size(Some(from), Some(to), None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get payload size").description(
                    "Get the cumulative size (bytes) of payload data in the chain, optionally \
                     restricted by block range and/or namespace.",
                )
            }),
        )
        .api_route(
            "/vid/share/hash/{hash}",
            get_with(|State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_vid_share(v1::VidShareId::Hash(hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get this node's VID share").description(
                    "Get information needed to run the VID reconstruction protocol for a block: \
                     this node's VID share, if available.",
                )
            }),
        )
        .api_route(
            "/vid/share/payload-hash/{payload_hash}",
            get_with(|State(state): State<S>, Path(payload_hash): Path<String>| async move {
            state
                .get_vid_share(v1::VidShareId::PayloadHash(payload_hash))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get this node's VID share").description(
                    "Get information needed to run the VID reconstruction protocol for a block: \
                     this node's VID share, if available.",
                )
            }),
        )
        .api_route(
            "/vid/share/{height}",
            get_with(|State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_vid_share(v1::VidShareId::Height(height))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get this node's VID share").description(
                    "Get information needed to run the VID reconstruction protocol for a block: \
                     this node's VID share, if available.",
                )
            }),
        )
        .api_route(
            "/sync-status",
            get_with(|State(state): State<S>| async move {
        state
            .sync_status()
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get node sync status").description(
                    "Get the node's progress syncing with the latest chain state \
                     (missing/present/pruned ranges for blocks, leaves, and VID common).",
                )
            }),
        )
        .api_route(
            "/header/window/from/hash/{hash}/{end}",
            get_with(|State(state): State<S>, Path((hash, end)): Path<(String, u64)>| async move {
            state
                .get_header_window(v1::HeaderWindowStart::Hash(hash), end)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get header window").description(
                    "Get block headers whose timestamps fall in a time window, plus one header \
                     before and after to prove completeness.",
                )
            }),
        )
        .api_route(
            "/header/window/from/{height}/{end}",
            get_with(|State(state): State<S>, Path((height, end)): Path<(u64, u64)>| async move {
            state
                .get_header_window(v1::HeaderWindowStart::Height(height), end)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get header window").description(
                    "Get block headers whose timestamps fall in a time window, plus one header \
                     before and after to prove completeness.",
                )
            }),
        )
        .api_route(
            "/header/window/{start}/{end}",
            get_with(|State(state): State<S>, Path((start, end)): Path<(u64, u64)>| async move {
        state
            .get_header_window(v1::HeaderWindowStart::Time(start), end)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get header window").description(
                    "Get block headers whose timestamps fall in a time window, plus one header \
                     before and after to prove completeness.",
                )
            }),
        )
        .api_route(
            "/limits",
            get_with(|State(state): State<S>| async move {
        <S as v1::NodeApi>::limits(&state)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    }, |op| {
                op.summary("Get node limits").description(
                    "Get implementation-defined limits restricting node API requests (e.g. \
                     header/window query size).",
                )
            }),
        )
        .api_route(
            "/stake-table/current",
            get_with(|State(state): State<S>| async move {
        state
            .stake_table_current()
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    }, |op| {
                op.summary("Get current stake table")
                    .description("Get the stake table for the current epoch.")
            }),
        )
        .api_route(
            "/stake-table/{epoch_number}",
            get_with(|State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .stake_table(epoch)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    }, |op| {
                op.summary("Get stake table for epoch")
                    .description("Get the stake table for the given epoch.")
            }),
        )
        .api_route(
            "/da-stake-table/current",
            get_with(|State(state): State<S>| async move {
        state
            .da_stake_table_current()
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    }, |op| {
                op.summary("Get current DA stake table")
                    .description("Get the DA stake table for the current epoch.")
            }),
        )
        .api_route(
            "/da-stake-table/{epoch_number}",
            get_with(|State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .da_stake_table(epoch)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    }, |op| {
                op.summary("Get DA stake table for epoch")
                    .description("Get the DA stake table for the given epoch.")
            }),
        )
        .api_route(
            "/validators/{epoch_number}",
            get_with(|State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .get_validators(epoch)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    }, |op| {
                op.summary("Get validators for epoch")
                    .description("Get the validators map for the given epoch.")
            }),
        )
        .api_route(
            "/all-validators/{epoch_number}/{offset}/{limit}",
            get_with(|State(state): State<S>, Path((epoch, offset, limit)): Path<(u64, u64, u64)>| async move {
            state
                .get_all_validators(epoch, offset, limit)
                .await
                .map(ApiJson)
                .map_err(ApiError::BadRequest)
        }, |op| {
                op.summary("Get all validators for epoch").description(
                    "Get all validators, including inactive ones, for the given epoch, paginated \
                     by offset and limit.",
                )
            }),
        )
        .api_route(
            "/participation/proposal/current",
            get_with(|State(state): State<S>| async move {
        state
            .current_proposal_participation()
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    }, |op| {
                op.summary("Get current proposal participation")
                    .description(
                        "Get the mapping from leader key to the fraction of views proposed \
                         properly as leader.",
                    )
            }),
        )
        .api_route(
            "/participation/proposal/{epoch}",
            get_with(|State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .proposal_participation(epoch)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    }, |op| {
                op.summary("Get proposal participation for epoch")
                    .description(
                        "Get the mapping from leader key to proposal participation rate for the \
                         given epoch.",
                    )
            }),
        )
        .api_route(
            "/participation/vote/current",
            get_with(|State(state): State<S>| async move {
        state
            .current_vote_participation()
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    }, |op| {
                op.summary("Get current vote participation").description(
                    "Get the mapping from node key to the fraction of views properly voted.",
                )
            }),
        )
        .api_route(
            "/participation/vote/{epoch}",
            get_with(|State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .vote_participation(epoch)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    }, |op| {
                op.summary("Get vote participation for epoch").description(
                    "Get the mapping from node key to vote participation rate for the given epoch.",
                )
            }),
        )
        .api_route(
            "/block-reward",
            get_with(|State(state): State<S>| async move {
        state
            .get_block_reward(None)
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    }, |op| {
                op.summary("Get block reward")
                    .description("Get the block reward.")
            }),
        )
        .api_route(
            "/block-reward/epoch/{epoch_number}",
            get_with(|State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .get_block_reward(Some(epoch))
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    }, |op| {
                op.summary("Get block reward for epoch")
                    .description("Get the block reward for the given epoch.")
            }),
        )
        .api_route(
            "/oldest-block",
            get_with(|State(state): State<S>| async move {
        state
            .get_oldest_block()
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    }, |op| {
                op.summary("Get oldest block").description(
                    "Get the oldest (smallest height) block present in storage, or null if none \
                     is stored.",
                )
            }),
        )
        .api_route(
            "/oldest-leaf",
            get_with(|State(state): State<S>| async move {
        state
            .get_oldest_leaf()
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    }, |op| {
                op.summary("Get oldest leaf").description(
                    "Get the oldest (smallest height) leaf present in storage, or null if none is \
                     stored.",
                )
            }),
        )
        )
        .with_state(state)
}

pub(crate) fn router_catchup<S>(state: S) -> ApiRouter
where
    S: v1::CatchupApi + Clone + Send + Sync + 'static,
{
    // Catchup handlers
    ApiRouter::new()
        .nest(
            "/catchup",
            ApiRouter::new()
        .api_route(
            "/{height}/{view}/account/{address}",
            get_with(|State(state): State<S>, Path((height, view, address)): Path<(u64, u64, String)>| async move {
            state
                .get_account(height, view, address)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Catch up fee account balance").description(
                    "Get the fee account balance and Merkle proof for an address at the given \
                     block height and view, for catchup.",
                )
            }),
        )
        .api_route(
            "/{height}/{view}/accounts",
            post_with(|State(state): State<S>,
                            Path((height, view)): Path<(u64, u64)>,
                            headers: HeaderMap,
                            body: Bytes| async move {
        let accounts: Vec<<S as v1::CatchupApi>::FeeAccount> = decode_body(&headers, &body)?;
        let tree = state
            .get_accounts(height, view, accounts)
            .await
            .map_err(classify_availability_error)?;
        encode_response(&headers, tree)
    }, |op| {
                op.summary("Catch up fee accounts (bulk)").description(
                    "Bulk version of the fee account endpoint; request body is a JSON array of \
                     TaggedBase64 fee accounts, response is a FeeMerkleTree.",
                )
            }),
        )
        .api_route(
            "/{height}/{view}/blocks",
            get_with(|State(state): State<S>, Path((height, view)): Path<(u64, u64)>| async move {
        state
            .get_blocks_frontier(height, view)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Catch up blocks Merkle frontier").description(
                    "Get the blocks Merkle tree frontier at the given block height and view, for \
                     catchup.",
                )
            }),
        )
        .api_route(
            "/chain-config/{commitment}",
            get_with(|State(state): State<S>, Path(commitment): Path<String>| async move {
        <S as v1::CatchupApi>::get_chain_config(&state, commitment)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Catch up chain config").description(
                    "Retrieve the chain config matching the given commitment from a peer; used \
                     when a node missed a protocol upgrade.",
                )
            }),
        )
        .api_route(
            "/{height}/leafchain",
            get_with(|State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_leaf_chain(height)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Catch up leaf chain").description(
                    "Fetch a leaf chain that decides the block at the given height, for catching \
                     up the stake table.",
                )
            }),
        )
        .api_route(
            "/{height}/cert2",
            get_with(|State(state): State<S>, Path(height): Path<u64>| async move {
        <S as v1::CatchupApi>::get_cert2(&state, height)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Catch up cert2").description(
                    "Fetch the cert2 stored at exactly the given height, if one exists; 404 \
                     otherwise.",
                )
            }),
        )
        .api_route(
            "/{height}/{view}/reward-account/{address}",
            get_with(|State(state): State<S>, Path((height, view, address)): Path<(u64, u64, String)>| async move {
            state
                .get_reward_account_v1(height, view, address)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Catch up reward account (V1)").description(
                    "Get the reward account balance for an address at the given height and view.",
                )
            }),
        )
        .api_route(
            "/{height}/{view}/reward-accounts",
            post_with(|State(state): State<S>,
                                   Path((height, view)): Path<(u64, u64)>,
                                   headers: HeaderMap,
                                   body: Bytes| async move {
        let accounts: Vec<<S as v1::CatchupApi>::RewardAccountV1> = decode_body(&headers, &body)?;
        let tree = state
            .get_reward_accounts_v1(height, view, accounts)
            .await
            .map_err(classify_availability_error)?;
        encode_response(&headers, tree)
    }, |op| {
                op.summary("Catch up reward accounts (bulk, V1)")
                    .description(
                        "Bulk version of the reward account endpoint; request body is a JSON \
                         array of TaggedBase64 reward accounts, response is a RewardMerkleTreeV1.",
                    )
            }),
        )
        .api_route(
            "/{height}/{view}/reward-account-v2/{address}",
            get_with(|State(state): State<S>, Path((height, view, address)): Path<(u64, u64, String)>| async move {
            state
                .get_reward_account_v2(height, view, address)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Catch up reward account (V2)").description(
                    "Get the reward account balance for an address at the given height and view, \
                     from RewardMerkleTreeV2.",
                )
            }),
        )
        .api_route(
            "/{height}/{view}/reward-accounts-v2",
            post_with(|State(_): State<S>, Path((_height, _view)): Path<(u64, u64)>| async move {
            Err::<Json<()>, ApiError>(ApiError::NotFound(anyhow::anyhow!(
                "catchup/reward-accounts-v2 is deprecated"
            )))
        }, |op| {
                op.summary("Catch up reward accounts (bulk, V2) — deprecated")
                    .description("Deprecated: this endpoint always returns 404 Not Found.")
            }),
        )
        .api_route(
            "/{height}/reward-amounts/{limit}/{offset}",
            get_with(|State(_): State<S>, Path((_height, _limit, _offset)): Path<(u64, u64, u64)>| async move {
            Err::<Json<()>, ApiError>(ApiError::NotFound(anyhow::anyhow!(
                "catchup/reward-amounts is deprecated"
            )))
        }, |op| {
                op.summary("List reward amounts — deprecated")
                    .description("Deprecated: this endpoint always returns 404 Not Found.")
            }),
        )
        .api_route(
            "/reward-merkle-tree-v2/{height}/{view}",
            get_with(|State(state): State<S>, Path((height, view)): Path<(u64, u64)>| async move {
            <S as v1::CatchupApi>::get_reward_merkle_tree_v2(&state, height, view)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Catch up RewardMerkleTreeV2").description(
                    "Get the RewardMerkleTreeV2 from consensus state at the given height and \
                     view, serialized as RewardMerkleTreeV2Data.",
                )
            }),
        )
        .api_route(
            "/{epoch}/state-cert",
            get_with(|State(state): State<S>, Path(epoch): Path<u64>| async move {
        <S as v1::CatchupApi>::get_state_cert(&state, epoch)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Catch up state certificate")
                    .description("Get the light client state certificate for the given epoch.")
            }),
        )
        )
        .with_state(state)
}

pub(crate) fn router_submit<S>(state: S) -> ApiRouter
where
    S: v1::SubmitApi + Clone + Send + Sync + 'static,
{
    // Submit handler — body is decoded as VBS (binary) or JSON based on Content-Type, matching
    // tide-disco's `body_auto`.
    ApiRouter::new()
        .nest(
            "/submit",
            ApiRouter::new().api_route(
                "/submit",
                post_with(
                    |State(state): State<S>, headers: HeaderMap, body: Bytes| async move {
                        let tx: <S as v1::SubmitApi>::Transaction = decode_body(&headers, &body)?;
                        let hash = state.submit(tx).await.map_err(ApiError::Internal)?;
                        encode_response(&headers, hash)
                    },
                    |op| {
                        op.summary("Submit transaction").description(
                            "Submit a transaction to the HotShot handle for sequencing.",
                        )
                    },
                ),
            ),
        )
        .with_state(state)
}

pub(crate) fn router_state_signature<S>(state: S) -> ApiRouter
where
    S: v1::StateSignatureApi + Clone + Send + Sync + 'static,
{
    // State signature handler
    ApiRouter::new()
        .nest(
            "/state-signature",
            ApiRouter::new().api_route(
                "/block/{height}",
                get_with(
                    |State(state): State<S>, Path(height): Path<u64>| async move {
                        state
                            .get_state_signature(height)
                            .await
                            .map(ApiJson)
                            .map_err(classify_availability_error)
                    },
                    |op| {
                        op.summary("Get light client state signature").description(
                            "Get this node's signature for the light client state at the given \
                             block height.",
                        )
                    },
                ),
            ),
        )
        .with_state(state)
}

pub(crate) fn router_hotshot_events<S>(state: S) -> ApiRouter
where
    S: v1::HotShotEventsApi + Clone + Send + Sync + 'static,
{
    // HotShot events handlers
    ApiRouter::new()
        .nest(
            "/hotshot-events",
            ApiRouter::new()
        .api_route(
            "/startup_info",
            get_with(|State(state): State<S>| async move {
        state
            .startup_info()
            .await
            .map(ApiJson)
            .map_err(ApiError::Internal)
    }, |op| {
                op.summary("Get startup info").description(
                    "Get startup info: known nodes with stake and their public keys, and the \
                     count of non-staked nodes.",
                )
            }),
        )
        .api_route(
            "/events",
            get_with(|State(state): State<S>, headers: HeaderMap, ws: WebSocketUpgrade| async move {
            let format = ws_format(&headers);
            match <S as v1::HotShotEventsApi>::events(&state).await {
                Ok(stream) => ws.on_upgrade(move |socket| async move {
                    drive_ws_stream(socket, stream, format).await
                }),
                Err(err) => ApiError::Internal(err).into_response(),
            }
        }, |op| {
                op.summary("Stream HotShot events (websocket)")
                    .description("Websocket endpoint: get legacy HotShot events starting now.")
            }),
        )
        )
        .with_state(state)
}

pub(crate) fn router_light_client<S>(state: S) -> ApiRouter
where
    S: v1::LightClientApi + Clone + Send + Sync + 'static,
{
    // Light-client handlers
    ApiRouter::new()
        .nest(
            "/light-client",
            ApiRouter::new()
        .api_route(
            "/leaf/{height}",
            get_with(|State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_leaf_proof(v1::LeafQuery::Height(height), None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get leaf with finality proof").description(
                    "Fetch a leaf by height plus a proof of its finality, optionally relative to \
                     an already-known-finalized height.",
                )
            }),
        )
        .api_route(
            "/leaf/{height}/{finalized}",
            get_with(|State(state): State<S>, Path((height, finalized)): Path<(u64, u64)>| async move {
            state
                .get_leaf_proof(v1::LeafQuery::Height(height), Some(finalized))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get leaf with finality proof").description(
                    "Fetch a leaf by height plus a proof of its finality, optionally relative to \
                     an already-known-finalized height.",
                )
            }),
        )
        .api_route(
            "/leaf/hash/{hash}",
            get_with(|State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_leaf_proof(v1::LeafQuery::Hash(hash), None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get leaf with finality proof").description(
                    "Fetch a leaf by hash plus a proof of its finality, optionally relative to an \
                     already-known-finalized height.",
                )
            }),
        )
        .api_route(
            "/leaf/hash/{hash}/{finalized}",
            get_with(|State(state): State<S>, Path((hash, finalized)): Path<(String, u64)>| async move {
            state
                .get_leaf_proof(v1::LeafQuery::Hash(hash), Some(finalized))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get leaf with finality proof").description(
                    "Fetch a leaf by hash plus a proof of its finality, optionally relative to an \
                     already-known-finalized height.",
                )
            }),
        )
        .api_route(
            "/leaf/block-hash/{block_hash}",
            get_with(|State(state): State<S>, Path(block_hash): Path<String>| async move {
        state
            .get_leaf_proof(v1::LeafQuery::BlockHash(block_hash), None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get leaf with finality proof").description(
                    "Fetch a leaf by block hash plus a proof of its finality, optionally relative \
                     to an already-known-finalized height.",
                )
            }),
        )
        .api_route(
            "/leaf/block-hash/{block_hash}/{finalized}",
            get_with(|State(state): State<S>, Path((block_hash, finalized)): Path<(String, u64)>| async move {
            state
                .get_leaf_proof(v1::LeafQuery::BlockHash(block_hash), Some(finalized))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get leaf with finality proof").description(
                    "Fetch a leaf by block hash plus a proof of its finality, optionally relative \
                     to an already-known-finalized height.",
                )
            }),
        )
        .api_route(
            "/leaf/payload-hash/{payload_hash}",
            get_with(|State(state): State<S>, Path(payload_hash): Path<String>| async move {
        state
            .get_leaf_proof(v1::LeafQuery::PayloadHash(payload_hash), None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get leaf with finality proof").description(
                    "Fetch a leaf by payload hash plus a proof of its finality, optionally \
                     relative to an already-known-finalized height.",
                )
            }),
        )
        .api_route(
            "/leaf/payload-hash/{payload_hash}/{finalized}",
            get_with(|State(state): State<S>, Path((payload_hash, finalized)): Path<(String, u64)>| async move {
            state
                .get_leaf_proof(v1::LeafQuery::PayloadHash(payload_hash), Some(finalized))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get leaf with finality proof").description(
                    "Fetch a leaf by payload hash plus a proof of its finality, optionally \
                     relative to an already-known-finalized height.",
                )
            }),
        )
        .api_route(
            "/header/{root}/{height}",
            get_with(|State(state): State<S>, Path((root, height)): Path<(u64, u64)>| async move {
        state
            .get_header_proof(root, v1::HeaderQuery::Height(height))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get header with inclusion proof").description(
                    "Fetch a header plus a Merkle proof that it belongs to the blocks Merkle tree \
                     rooted at the given root height.",
                )
            }),
        )
        .api_route(
            "/header/{root}/hash/{hash}",
            get_with(|State(state): State<S>, Path((root, hash)): Path<(u64, String)>| async move {
        state
            .get_header_proof(root, v1::HeaderQuery::Hash(hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get header with inclusion proof").description(
                    "Fetch a header plus a Merkle proof that it belongs to the blocks Merkle tree \
                     rooted at the given root height.",
                )
            }),
        )
        .api_route(
            "/header/{root}/payload-hash/{payload_hash}",
            get_with(|State(state): State<S>, Path((root, payload_hash)): Path<(u64, String)>| async move {
            state
                .get_header_proof(root, v1::HeaderQuery::PayloadHash(payload_hash))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get header with inclusion proof").description(
                    "Fetch a header plus a Merkle proof that it belongs to the blocks Merkle tree \
                     rooted at the given root height.",
                )
            }),
        )
        .api_route(
            "/stake-table/{epoch}",
            get_with(|State(state): State<S>, Path(epoch): Path<u64>| async move {
        state
            .get_light_client_stake_table(epoch)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get stake table events for epoch").description(
                    "Get the events needed to transform the stake table from the previous epoch \
                     into the given epoch.",
                )
            }),
        )
        .api_route(
            "/payload/{height}",
            get_with(|State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_payload_proof(height)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get payload with VID common data").description(
                    "Fetch a payload plus the VID common data needed to recompute and verify its \
                     hash.",
                )
            }),
        )
        .api_route(
            "/payload/{start}/{end}",
            get_with(|State(state): State<S>, Path((start, end)): Path<(u64, u64)>| async move {
        state
            .get_payload_proof_range(start, end)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get payload proofs in range").description(
                    "Fetch a list of payload proofs for each block in the given range.",
                )
            }),
        )
        .api_route(
            "/namespace/{height}/{namespace}",
            get_with(|State(state): State<S>, Path((height, namespace)): Path<(u64, u64)>| async move {
        state
            .get_lc_namespace_proof(height, namespace)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get namespace proof with VID common data")
                    .description(
                        "Fetch a namespace proof plus the VID common data needed to verify it.",
                    )
            }),
        )
        .api_route(
            "/namespace/{start}/{end}/{namespace}",
            get_with(|State(state): State<S>, Path((start, end, namespace)): Path<(u64, u64, u64)>| async move {
            state
                .get_lc_namespace_proof_range(start, end, namespace)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get namespace proofs in range").description(
                    "Fetch a list of namespace proofs for each block in the given range.",
                )
            }),
        )
        .api_route(
            "/namespaces/{start}/{end}/{namespaces}",
            get_with(|State(state): State<S>, Path((start, end, namespaces)): Path<(u64, u64, String)>| async move {
            state
                .get_lc_namespaces_proof_range(start, end, namespaces)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get proofs for multiple namespaces in range")
                    .description(
                        "Fetch namespace proofs for each block in the given range, restricted to \
                         a caller-specified set of namespaces.",
                    )
            }),
        )
        )
        .with_state(state)
}

pub(crate) fn router_explorer<S>(state: S) -> ApiRouter
where
    S: v1::ExplorerApi + Clone + Send + Sync + 'static,
{
    // Explorer handlers
    ApiRouter::new()
        .nest(
            "/explorer",
            ApiRouter::new()
        .api_route(
            "/block/{height}",
            get_with(|State(state): State<S>, Path(height): Path<u64>| async move {
        state
            .get_block_detail(v1::BlockIdent::Height(height))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get block detail")
                    .description("Get details for a block identified by height or hash.")
            }),
        )
        .api_route(
            "/block/hash/{hash}",
            get_with(|State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_block_detail(v1::BlockIdent::Hash(hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get block detail")
                    .description("Get details for a block identified by height or hash.")
            }),
        )
        .api_route(
            "/blocks/latest/{limit}",
            get_with(|State(state): State<S>, Path(limit): Path<u64>| async move {
        state
            .get_block_summaries(v1::BlockIdent::Latest, limit)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("List block summaries").description(
                    "Retrieve up to `limit` block summaries, targeting the latest block or a \
                     block identified by height.",
                )
            }),
        )
        .api_route(
            "/blocks/{from}/{limit}",
            get_with(|State(state): State<S>, Path((from, limit)): Path<(u64, u64)>| async move {
            state
                .get_block_summaries(v1::BlockIdent::Height(from), limit)
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("List block summaries").description(
                    "Retrieve up to `limit` block summaries, targeting the latest block or a \
                     block identified by height.",
                )
            }),
        )
        .api_route(
            "/transaction/{height}/{offset}",
            get_with(|State(state): State<S>, Path((height, offset)): Path<(u64, u64)>| async move {
            state
                .get_transaction_detail(v1::TxIdent::HeightAndOffset(height, offset))
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("Get transaction detail").description(
                    "Get details for a transaction identified by height and offset, or by hash.",
                )
            }),
        )
        .api_route(
            "/transaction/hash/{hash}",
            get_with(|State(state): State<S>, Path(hash): Path<String>| async move {
        state
            .get_transaction_detail(v1::TxIdent::Hash(hash))
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get transaction detail").description(
                    "Get details for a transaction identified by height and offset, or by hash.",
                )
            }),
        )
        .api_route(
            "/transactions/latest/{limit}/block/{block}",
            get_with(|State(state): State<S>, Path((limit, block)): Path<(u64, u64)>| async move {
            state
                .get_transaction_summaries(
                    v1::TxIdent::Latest,
                    limit,
                    v1::TxSummaryFilter::Block(block),
                )
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("List transaction summaries").description(
                    "Retrieve up to `limit` transaction summaries, targeting the latest \
                     transaction, one identified by height/offset, or by hash; optionally \
                     filtered by block or namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/from/{height}/{offset}/{limit}/block/{block}",
            get_with(|State(state): State<S>,
         Path((height, offset, limit, block)): Path<(u64, u64, u64, u64)>| async move {
            state
                .get_transaction_summaries(
                    v1::TxIdent::HeightAndOffset(height, offset),
                    limit,
                    v1::TxSummaryFilter::Block(block),
                )
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("List transaction summaries").description(
                    "Retrieve up to `limit` transaction summaries, targeting the latest \
                     transaction, one identified by height/offset, or by hash; optionally \
                     filtered by block or namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/hash/{hash}/{limit}/block/{block}",
            get_with(|State(state): State<S>, Path((hash, limit, block)): Path<(String, u64, u64)>| async move {
            state
                .get_transaction_summaries(
                    v1::TxIdent::Hash(hash),
                    limit,
                    v1::TxSummaryFilter::Block(block),
                )
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("List transaction summaries").description(
                    "Retrieve up to `limit` transaction summaries, targeting the latest \
                     transaction, one identified by height/offset, or by hash; optionally \
                     filtered by block or namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/latest/{limit}/namespace/{namespace}",
            get_with(|State(state): State<S>, Path((limit, namespace)): Path<(u64, i64)>| async move {
            state
                .get_transaction_summaries(
                    v1::TxIdent::Latest,
                    limit,
                    v1::TxSummaryFilter::Namespace(namespace),
                )
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("List transaction summaries").description(
                    "Retrieve up to `limit` transaction summaries, targeting the latest \
                     transaction, one identified by height/offset, or by hash; optionally \
                     filtered by block or namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/from/{height}/{offset}/{limit}/namespace/{namespace}",
            get_with(|State(state): State<S>,
         Path((height, offset, limit, namespace)): Path<(u64, u64, u64, i64)>| async move {
            state
                .get_transaction_summaries(
                    v1::TxIdent::HeightAndOffset(height, offset),
                    limit,
                    v1::TxSummaryFilter::Namespace(namespace),
                )
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("List transaction summaries").description(
                    "Retrieve up to `limit` transaction summaries, targeting the latest \
                     transaction, one identified by height/offset, or by hash; optionally \
                     filtered by block or namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/hash/{hash}/{limit}/namespace/{namespace}",
            get_with(|State(state): State<S>,
                                            Path((hash, limit, namespace)): Path<(
        String,
        u64,
        i64,
    )>| async move {
        state
            .get_transaction_summaries(
                v1::TxIdent::Hash(hash),
                limit,
                v1::TxSummaryFilter::Namespace(namespace),
            )
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("List transaction summaries").description(
                    "Retrieve up to `limit` transaction summaries, targeting the latest \
                     transaction, one identified by height/offset, or by hash; optionally \
                     filtered by block or namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/latest/{limit}",
            get_with(|State(state): State<S>, Path(limit): Path<u64>| async move {
        state
            .get_transaction_summaries(v1::TxIdent::Latest, limit, v1::TxSummaryFilter::None)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("List transaction summaries").description(
                    "Retrieve up to `limit` transaction summaries, targeting the latest \
                     transaction, one identified by height/offset, or by hash; optionally \
                     filtered by block or namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/from/{height}/{offset}/{limit}",
            get_with(|State(state): State<S>, Path((height, offset, limit)): Path<(u64, u64, u64)>| async move {
            state
                .get_transaction_summaries(
                    v1::TxIdent::HeightAndOffset(height, offset),
                    limit,
                    v1::TxSummaryFilter::None,
                )
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("List transaction summaries").description(
                    "Retrieve up to `limit` transaction summaries, targeting the latest \
                     transaction, one identified by height/offset, or by hash; optionally \
                     filtered by block or namespace.",
                )
            }),
        )
        .api_route(
            "/transactions/hash/{hash}/{limit}",
            get_with(|State(state): State<S>, Path((hash, limit)): Path<(String, u64)>| async move {
            state
                .get_transaction_summaries(
                    v1::TxIdent::Hash(hash),
                    limit,
                    v1::TxSummaryFilter::None,
                )
                .await
                .map(ApiJson)
                .map_err(classify_availability_error)
        }, |op| {
                op.summary("List transaction summaries").description(
                    "Retrieve up to `limit` transaction summaries, targeting the latest \
                     transaction, one identified by height/offset, or by hash; optionally \
                     filtered by block or namespace.",
                )
            }),
        )
        .api_route(
            "/explorer-summary",
            get_with(|State(state): State<S>| async move {
        state
            .get_explorer_summary()
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Get explorer summary")
                    .description("Get the current chain explorer summary.")
            }),
        )
        .api_route(
            "/search/{query}",
            get_with(|State(state): State<S>, Path(query): Path<String>| async move {
        state
            .get_search_result(query)
            .await
            .map(ApiJson)
            .map_err(classify_availability_error)
    }, |op| {
                op.summary("Search blocks and transactions").description(
                    "Search for blocks or transactions matching the given query string; currently \
                     matched against hash.",
                )
            }),
        )
        )
        .with_state(state)
}

pub(crate) fn router_token<S>(state: S) -> ApiRouter
where
    S: v1::TokenApi + Clone + Send + Sync + 'static,
{
    // Token handlers
    ApiRouter::new()
        .nest(
            "/token",
            ApiRouter::new()
                .api_route(
                    "/total-minted-supply",
                    get_with(
                        |State(state): State<S>| async move {
                            state
                                .total_minted_supply()
                                .await
                                .map(ApiJson)
                                .map_err(classify_availability_error)
                        },
                        |op| {
                            op.summary("Get total minted supply").description(
                                "Total supply of the ESP token minted on Ethereum; excludes \
                                 unclaimed rewards. Cached for an hour.",
                            )
                        },
                    ),
                )
                .api_route(
                    "/circulating-supply",
                    get_with(
                        |State(state): State<S>| async move {
                            state
                                .circulating_supply()
                                .await
                                .map(ApiJson)
                                .map_err(classify_availability_error)
                        },
                        |op| {
                            op.summary("Get circulating supply").description(
                                "Circulating supply: initial_supply + reward_distributed - \
                                 locked, following the mainnet unlock schedule.",
                            )
                        },
                    ),
                )
                .api_route(
                    "/circulating-supply-ethereum",
                    get_with(
                        |State(state): State<S>| async move {
                            state
                                .circulating_supply_ethereum()
                                .await
                                .map(ApiJson)
                                .map_err(classify_availability_error)
                        },
                        |op| {
                            op.summary("Get circulating supply (Ethereum L1)")
                                .description(
                                    "Circulating supply of ESP tokens on Ethereum L1: \
                                     total_supply_l1 - locked.",
                                )
                        },
                    ),
                )
                .api_route(
                    "/total-issued-supply",
                    get_with(
                        |State(state): State<S>| async move {
                            state
                                .total_issued_supply()
                                .await
                                .map(ApiJson)
                                .map_err(classify_availability_error)
                        },
                        |op| {
                            op.summary("Get total issued supply").description(
                                "Total issued supply: initial_supply + total_reward_distributed, \
                                 including rewards not yet claimed on Ethereum.",
                            )
                        },
                    ),
                )
                .api_route(
                    "/total-reward-distributed",
                    get_with(
                        |State(state): State<S>| async move {
                            state
                                .total_reward_distributed()
                                .await
                                .map(ApiJson)
                                .map_err(classify_availability_error)
                        },
                        |op| {
                            op.summary("Get total reward distributed").description(
                                "Total rewards distributed by consensus, including rewards not \
                                 yet claimed on Ethereum.",
                            )
                        },
                    ),
                ),
        )
        .with_state(state)
}

pub(crate) fn router_database<S>(state: S) -> ApiRouter
where
    S: v1::DatabaseApi + Clone + Send + Sync + 'static,
{
    // Database handlers
    ApiRouter::new()
        .nest(
            "/database",
            ApiRouter::new()
                .api_route(
                    "/table-sizes",
                    get_with(
                        |State(state): State<S>| async move {
                            <S as v1::DatabaseApi>::get_table_sizes(&state)
                                .await
                                .map(ApiJson)
                                .map_err(ApiError::Internal)
                        },
                        |op| {
                            op.summary("Get database table sizes").description(
                                "Get the sizes of all database tables: row counts and disk usage.",
                            )
                        },
                    ),
                )
                .api_route(
                    "/migration-status",
                    get_with(
                        |State(state): State<S>| async move {
                            <S as v1::DatabaseApi>::get_migration_status(&state)
                                .await
                                .map(ApiJson)
                                .map_err(ApiError::Internal)
                        },
                        |op| {
                            op.summary("Get migration status").description(
                                "Get the status of all deferred background migrations: \
                                 start/completion time and last processed offset.",
                            )
                        },
                    ),
                ),
        )
        .with_state(state)
}
