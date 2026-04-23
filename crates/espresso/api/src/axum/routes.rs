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
}

//=============================================================================
// V2 Routes - Rewards API
//=============================================================================

pub mod v2 {
    use super::Route;

    /// Get reward claim input for L1 contract submission
    ///
    /// HTTP: GET /v2/rewards/claim-input/{address}
    /// gRPC: /espresso.api.v2.RewardService/GetRewardClaimInput
    pub const REWARD_CLAIM_INPUT_ROUTE: Route = Route {
        http: "/v2/rewards/claim-input/{address}",
        grpc: "/espresso.api.v2.RewardService/GetRewardClaimInput",
    };

    /// Get reward balance at the latest finalized height
    ///
    /// HTTP: GET /v2/rewards/balance/{address}
    /// gRPC: /espresso.api.v2.RewardService/GetRewardBalance
    pub const REWARD_BALANCE_ROUTE: Route = Route {
        http: "/v2/rewards/balance/{address}",
        grpc: "/espresso.api.v2.RewardService/GetRewardBalance",
    };

    /// Get reward account Merkle proof at the latest finalized height
    ///
    /// HTTP: GET /v2/rewards/proof/{address}
    /// gRPC: /espresso.api.v2.RewardService/GetRewardAccountProof
    pub const REWARD_ACCOUNT_PROOF_ROUTE: Route = Route {
        http: "/v2/rewards/proof/{address}",
        grpc: "/espresso.api.v2.RewardService/GetRewardAccountProof",
    };

    /// Get paginated list of reward balances
    ///
    /// HTTP: GET /v2/rewards/balances/{height}/{offset}/{limit}
    /// gRPC: /espresso.api.v2.RewardService/GetRewardBalances
    pub const REWARD_BALANCES_ROUTE: Route = Route {
        http: "/v2/rewards/balances/{height}/{offset}/{limit}",
        grpc: "/espresso.api.v2.RewardService/GetRewardBalances",
    };

    /// Get raw RewardMerkleTreeV2 snapshot
    ///
    /// HTTP: GET /v2/rewards/tree/{height}
    /// gRPC: /espresso.api.v2.RewardService/GetRewardMerkleTreeV2
    pub const REWARD_MERKLE_TREE_V2_ROUTE: Route = Route {
        http: "/v2/rewards/tree/{height}",
        grpc: "/espresso.api.v2.RewardService/GetRewardMerkleTreeV2",
    };

    //=========================================================================
    // Documentation Routes (V2 only)
    //=========================================================================

    /// OpenAPI specification endpoint
    pub const OPENAPI_SPEC_ROUTE: &str = "/v2/docs/openapi.json";

    /// Swagger documentation UI endpoint
    pub const SWAGGER_ROUTE: &str = "/v2";

    /// Scalar documentation UI endpoint
    pub const SCALAR_ROUTE: &str = "/v2/scalar";

    /// Redoc documentation UI endpoint
    pub const REDOC_ROUTE: &str = "/v2/redoc";

    //=========================================================================
    // Data API Routes
    //=========================================================================

    /// Get namespace proof(s) for a namespace
    ///
    /// HTTP: GET /v2/data/finalized/namespace-proof/{namespace_id}?block={height}
    ///       or GET /v2/data/finalized/namespace-proof/{namespace_id}?from={start}&to={end}
    /// gRPC: /espresso.api.v2.DataService/GetNamespaceProof (single)
    ///       /espresso.api.v2.DataService/GetNamespaceProofRange (range)
    pub const NAMESPACE_PROOF_ROUTE: Route = Route {
        http: "/v2/data/finalized/namespace-proof/{namespace_id}",
        grpc: "/espresso.api.v2.DataService/GetNamespaceProof",
    };

    /// Get incorrect encoding proof
    ///
    /// HTTP: GET /v2/data/finalized/incorrect-encoding-proof/{namespace_id}?block={block_height}
    /// gRPC: /espresso.api.v2.DataService/GetIncorrectEncodingProof
    pub const INCORRECT_ENCODING_PROOF_ROUTE: Route = Route {
        http: "/v2/data/finalized/incorrect-encoding-proof/{namespace_id}",
        grpc: "/espresso.api.v2.DataService/GetIncorrectEncodingProof",
    };

    //=========================================================================
    // Consensus API Routes
    //=========================================================================

    /// Get state certificate for an epoch
    ///
    /// HTTP: GET /v2/consensus/state-certificate/{epoch}
    pub const STATE_CERTIFICATE_ROUTE: Route = Route {
        http: "/v2/consensus/state-certificate/{epoch}",
        grpc: "/espresso.api.v2.ConsensusService/GetStateCertificate",
    };

    /// Get stake table for an epoch
    ///
    /// HTTP: GET /v2/consensus/stake-table/{epoch}
    pub const STAKE_TABLE_ROUTE: Route = Route {
        http: "/v2/consensus/stake-table/{epoch}",
        grpc: "/espresso.api.v2.ConsensusService/GetStakeTable",
    };
}
