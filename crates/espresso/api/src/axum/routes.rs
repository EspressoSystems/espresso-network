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
}

//=============================================================================
// V2 Routes - Rewards API
//=============================================================================

pub mod v2 {
    use super::Route;

    /// Get reward claim input for L1 contract submission
    ///
    /// HTTP: GET /v2/rewards/claim-input/{height}/{address}
    /// gRPC: /espresso.api.v2.RewardService/GetRewardClaimInput
    pub const REWARD_CLAIM_INPUT_ROUTE: Route = Route {
        http: "/v2/rewards/claim-input/{height}/{address}",
        grpc: "/espresso.api.v2.RewardService/GetRewardClaimInput",
    };

    /// Get reward balance at a specific height
    ///
    /// HTTP: GET /v2/rewards/balance/{height}/{address}
    /// gRPC: /espresso.api.v2.RewardService/GetRewardBalance
    pub const REWARD_BALANCE_ROUTE: Route = Route {
        http: "/v2/rewards/balance/{height}/{address}",
        grpc: "/espresso.api.v2.RewardService/GetRewardBalance",
    };

    /// Get latest reward balance
    ///
    /// HTTP: GET /v2/rewards/balance/latest/{address}
    /// gRPC: /espresso.api.v2.RewardService/GetLatestRewardBalance
    pub const LATEST_REWARD_BALANCE_ROUTE: Route = Route {
        http: "/v2/rewards/balance/latest/{address}",
        grpc: "/espresso.api.v2.RewardService/GetLatestRewardBalance",
    };

    /// Get reward account Merkle proof at a specific height
    ///
    /// HTTP: GET /v2/rewards/proof/{height}/{address}
    /// gRPC: /espresso.api.v2.RewardService/GetRewardAccountProof
    pub const REWARD_ACCOUNT_PROOF_ROUTE: Route = Route {
        http: "/v2/rewards/proof/{height}/{address}",
        grpc: "/espresso.api.v2.RewardService/GetRewardAccountProof",
    };

    /// Get latest reward account Merkle proof
    ///
    /// HTTP: GET /v2/rewards/proof/latest/{address}
    /// gRPC: /espresso.api.v2.RewardService/GetLatestRewardAccountProof
    pub const LATEST_REWARD_ACCOUNT_PROOF_ROUTE: Route = Route {
        http: "/v2/rewards/proof/latest/{address}",
        grpc: "/espresso.api.v2.RewardService/GetLatestRewardAccountProof",
    };

    /// Get paginated list of reward amounts
    ///
    /// HTTP: GET /v2/rewards/amounts/{height}/{offset}/{limit}
    /// gRPC: /espresso.api.v2.RewardService/GetRewardAmounts
    pub const REWARD_AMOUNTS_ROUTE: Route = Route {
        http: "/v2/rewards/amounts/{height}/{offset}/{limit}",
        grpc: "/espresso.api.v2.RewardService/GetRewardAmounts",
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
}
