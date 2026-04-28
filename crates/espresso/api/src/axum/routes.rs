//! Route constants and URL builders for Axum HTTP API
//!
//! All routes are absolute paths including version prefixes for external use.

/// Route correlation between HTTP and gRPC endpoints
///
/// V2 routes expose both HTTP (Axum) and gRPC (Tonic) paths for the same functionality.
///
/// # Example
/// ```
/// use espresso_api::routes::v2::REWARD_BALANCE_ROUTE;
///
/// // Access HTTP path for Axum
/// let http_path = REWARD_BALANCE_ROUTE.http;
///
/// // Access gRPC path for Tonic (matches generated service code)
/// let grpc_path = REWARD_BALANCE_ROUTE.grpc;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Route {
    /// HTTP path for Axum handler (e.g., "/v2/rewards/balance/{height}/{address}")
    pub http: &'static str,
    /// gRPC path for Tonic service (e.g., "/espresso.api.v2.RewardService/GetRewardBalance")
    pub grpc: &'static str,
    /// OpenAPI description for the endpoint
    pub description: &'static str,
    /// OpenAPI tag grouping for the endpoint
    pub tag: &'static str,
}

//=============================================================================
// V1 Routes - Legacy Reward State V2 (backward compatibility)
//=============================================================================

pub mod v1 {
    /// Get reward claim input for L1 contract submission
    /// Path: GET /v1/reward-state-v2/reward-claim-input/{height}/{address}
    pub const REWARD_CLAIM_INPUT_ROUTE: &str =
        "/v1/reward-state-v2/reward-claim-input/{height}/{address}";

    /// Get reward balance at a specific height
    /// Path: GET /v1/reward-state-v2/reward-balance/{height}/{address}
    pub const REWARD_BALANCE_ROUTE: &str = "/v1/reward-state-v2/reward-balance/{height}/{address}";

    /// Get latest reward balance
    /// Path: GET /v1/reward-state-v2/reward-balance/latest/{address}
    pub const LATEST_REWARD_BALANCE_ROUTE: &str =
        "/v1/reward-state-v2/reward-balance/latest/{address}";

    /// Get reward account Merkle proof at a specific height
    /// Path: GET /v1/reward-state-v2/proof/{height}/{address}
    pub const REWARD_ACCOUNT_PROOF_ROUTE: &str = "/v1/reward-state-v2/proof/{height}/{address}";

    /// Get latest reward account Merkle proof
    /// Path: GET /v1/reward-state-v2/proof/latest/{address}
    pub const LATEST_REWARD_ACCOUNT_PROOF_ROUTE: &str =
        "/v1/reward-state-v2/proof/latest/{address}";

    /// Get paginated list of reward amounts
    /// Path: GET /v1/reward-state-v2/reward-amounts/{height}/{offset}/{limit}
    pub const REWARD_AMOUNTS_ROUTE: &str =
        "/v1/reward-state-v2/reward-amounts/{height}/{offset}/{limit}";

    /// Get raw RewardMerkleTreeV2 snapshot
    /// Path: GET /v1/reward-state-v2/reward-merkle-tree-v2/{height}
    pub const REWARD_MERKLE_TREE_V2_ROUTE: &str =
        "/v1/reward-state-v2/reward-merkle-tree-v2/{height}";

    //=========================================================================
    // Availability API Routes
    //=========================================================================

    /// Get namespace proof by block height
    /// Path: GET /v1/availability/namespace/{namespace}/block/{height}
    pub const NAMESPACE_PROOF_BY_HEIGHT_ROUTE: &str =
        "/v1/availability/namespace/{namespace}/block/{height}";

    /// Get namespace proof by block hash
    /// Path: GET /v1/availability/namespace/{namespace}/block/hash/{hash}
    pub const NAMESPACE_PROOF_BY_HASH_ROUTE: &str =
        "/v1/availability/namespace/{namespace}/block/hash/{hash}";

    /// Get namespace proof by payload hash
    /// Path: GET /v1/availability/namespace/{namespace}/block/payload-hash/{payload_hash}
    pub const NAMESPACE_PROOF_BY_PAYLOAD_HASH_ROUTE: &str =
        "/v1/availability/namespace/{namespace}/block/payload-hash/{payload_hash}";

    /// Get namespace proofs for a range of blocks
    /// Path: GET /v1/availability/namespace/{namespace}/blocks/{from}/{until}
    pub const NAMESPACE_PROOF_RANGE_ROUTE: &str =
        "/v1/availability/namespace/{namespace}/blocks/{from}/{until}";

    /// Generate proof of incorrect encoding
    /// Path: GET /v1/availability/incorrect-encoding/{namespace}/block/{block_number}
    pub const INCORRECT_ENCODING_PROOF_ROUTE: &str =
        "/v1/availability/incorrect-encoding/{namespace}/block/{block_number}";

    /// Get light client state certificate (V1)
    /// Path: GET /v1/availability/state-cert/{epoch}
    pub const STATE_CERT_V1_ROUTE: &str = "/v1/availability/state-cert/{epoch}";

    /// Get light client state certificate (V2)
    /// Path: GET /v1/availability/state-cert-v2/{epoch}
    pub const STATE_CERT_V2_ROUTE: &str = "/v1/availability/state-cert-v2/{epoch}";

    //=========================================================================
    // Block-State API Routes
    //=========================================================================

    /// Get block merkle path proof by snapshot height
    /// Path: GET /v1/block-state/{height}/{key}
    pub const BLOCK_MERKLE_PATH_BY_HEIGHT_ROUTE: &str = "/v1/block-state/{height}/{key}";

    /// Get block merkle path proof by snapshot commitment
    /// Path: GET /v1/block-state/commit/{commit}/{key}
    pub const BLOCK_MERKLE_PATH_BY_COMMIT_ROUTE: &str = "/v1/block-state/commit/{commit}/{key}";

    /// Get latest block height for which merklized state is available
    /// Path: GET /v1/block-state/block-height
    pub const BLOCK_MERKLE_HEIGHT_ROUTE: &str = "/v1/block-state/block-height";
}

//=============================================================================
// V2 Routes - Rewards API
//=============================================================================

pub mod v2 {
    use super::Route;

    pub const REWARD_CLAIM_INPUT_ROUTE: Route = Route {
        http: "/v2/rewards/claim-input",
        grpc: "/espresso.api.v2.RewardService/GetRewardClaimInput",
        description: "Get reward claim input for L1 contract submission. Returns lifetime rewards \
                      and Merkle proof needed to call claimRewards() on the L1 contract.",
        tag: "Rewards",
    };

    pub const REWARD_BALANCE_ROUTE: Route = Route {
        http: "/v2/rewards/balance",
        grpc: "/espresso.api.v2.RewardService/GetRewardBalance",
        description: "Get reward balance for an address at the latest finalized height",
        tag: "Rewards",
    };

    pub const REWARD_ACCOUNT_PROOF_ROUTE: Route = Route {
        http: "/v2/rewards/proof",
        grpc: "/espresso.api.v2.RewardService/GetRewardAccountProof",
        description: "Get Merkle proof for a reward account at the latest finalized height. \
                      Returns V2 proof with Keccak256 hashing",
        tag: "Rewards",
    };

    pub const REWARD_BALANCES_ROUTE: Route = Route {
        http: "/v2/rewards/balances",
        grpc: "/espresso.api.v2.RewardService/GetRewardBalances",
        description: "Get paginated list of all reward balances at a specific height. Limit must \
                      be ≤ 10000",
        tag: "Rewards",
    };

    pub const REWARD_MERKLE_TREE_V2_ROUTE: Route = Route {
        http: "/v2/rewards/tree",
        grpc: "/espresso.api.v2.RewardService/GetRewardMerkleTreeV2",
        description: "Get raw RewardMerkleTreeV2 snapshot at a given height. Returns serialized \
                      merkle tree data",
        tag: "Rewards",
    };

    pub const OPENAPI_SPEC_ROUTE: &str = "/v2/docs/openapi.json";
    pub const SWAGGER_ROUTE: &str = "/v2";
    pub const SCALAR_ROUTE: &str = "/v2/scalar";
    pub const REDOC_ROUTE: &str = "/v2/redoc";

    pub const NAMESPACE_PROOF_ROUTE: Route = Route {
        http: "/v2/data/finalized/namespace-proof",
        grpc: "/espresso.api.v2.DataService/GetNamespaceProof",
        description: "Get namespace proof(s) for the specified namespace. Use '?block={height}' \
                      for a single block, or '?from={start}&to={end}' for a range. Returns \
                      transactions for the namespace along with cryptographic proof(s) of \
                      completeness.",
        tag: "Data",
    };

    pub const INCORRECT_ENCODING_PROOF_ROUTE: Route = Route {
        http: "/v2/data/finalized/incorrect-encoding-proof",
        grpc: "/espresso.api.v2.DataService/GetIncorrectEncodingProof",
        description: "Generate a fraud proof showing incorrect namespace encoding for a specific \
                      block. Query param 'block' specifies the block height. Used to challenge \
                      invalid block proposals.",
        tag: "Data",
    };

    pub const STATE_CERTIFICATE_ROUTE: Route = Route {
        http: "/v2/consensus/state-certificate",
        grpc: "/espresso.api.v2.ConsensusService/GetStateCertificate",
        description: "Get light client state update certificate for an epoch. Used to update L1 \
                      contracts with new stake table information.",
        tag: "Consensus",
    };

    pub const STAKE_TABLE_ROUTE: Route = Route {
        http: "/v2/consensus/stake-table",
        grpc: "/espresso.api.v2.ConsensusService/GetStakeTable",
        description: "Get stake table for an epoch.",
        tag: "Consensus",
    };

    pub const BLOCK_MERKLE_PATH_ROUTE: Route = Route {
        http: "/v2/data/finalized/block-proof",
        grpc: "/espresso.api.v2.DataService/GetBlockMerklePath",
        description: "Get merkle proof for a block commitment. Supply either `snapshot_height` \
                      or `snapshot_commit` to identify the tree snapshot, plus `block_height` \
                      (block index being proven). Returns a JSON-serialized MerkleProof.",
        tag: "Data",
    };

    pub const BLOCK_MERKLE_HEIGHT_ROUTE: Route = Route {
        http: "/v2/diagnostics/state-storage-loop/block-height",
        grpc: "/espresso.api.v2.DataService/GetBlockMerkleHeight",
        description: "Latest block height processed by the state storage loop. Updated once per \
                      finalized leaf after the block merkle tree nodes are committed to storage. \
                      Lags behind chain tip since state storage runs asynchronously.",
        tag: "Diagnostics",
    };
}
