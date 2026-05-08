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
    /// Path: GET /v1/availability/block/{height}/namespace/{namespace}
    pub const NAMESPACE_PROOF_BY_HEIGHT_ROUTE: &str =
        "/v1/availability/block/{height}/namespace/{namespace}";

    /// Get namespace proof by block hash
    /// Path: GET /v1/availability/block/hash/{hash}/namespace/{namespace}
    pub const NAMESPACE_PROOF_BY_HASH_ROUTE: &str =
        "/v1/availability/block/hash/{hash}/namespace/{namespace}";

    /// Get namespace proof by payload hash
    /// Path: GET /v1/availability/block/payload-hash/{payload-hash}/namespace/{namespace}
    pub const NAMESPACE_PROOF_BY_PAYLOAD_HASH_ROUTE: &str =
        "/v1/availability/block/payload-hash/{payload-hash}/namespace/{namespace}";

    /// Get namespace proofs for a range of blocks
    /// Path: GET /v1/availability/block/{from}/{until}/namespace/{namespace}
    pub const NAMESPACE_PROOF_RANGE_ROUTE: &str =
        "/v1/availability/block/{from}/{until}/namespace/{namespace}";

    /// Generate proof of incorrect encoding
    /// Path: GET /v1/availability/incorrect-encoding-proof/{block_number}/{namespace}
    pub const INCORRECT_ENCODING_PROOF_ROUTE: &str =
        "/v1/availability/incorrect-encoding-proof/{block_number}/{namespace}";

    /// Get light client state certificate (V1)
    /// Path: GET /v1/availability/state-cert/{epoch}
    pub const STATE_CERT_V1_ROUTE: &str = "/v1/availability/state-cert/{epoch}";

    /// Get light client state certificate (V2)
    /// Path: GET /v1/availability/state-cert-v2/{epoch}
    pub const STATE_CERT_V2_ROUTE: &str = "/v1/availability/state-cert-v2/{epoch}";

    //=========================================================================
    // HotShot Availability API Routes (mirrored from hotshot-query-service)
    //=========================================================================

    pub const LEAF_BY_HEIGHT_ROUTE: &str = "/v1/availability/leaf/{height}";
    pub const LEAF_BY_HASH_ROUTE: &str = "/v1/availability/leaf/hash/{hash}";
    pub const LEAF_RANGE_ROUTE: &str = "/v1/availability/leaf/{from}/{until}";

    pub const HEADER_BY_HEIGHT_ROUTE: &str = "/v1/availability/header/{height}";
    pub const HEADER_BY_HASH_ROUTE: &str = "/v1/availability/header/hash/{hash}";
    pub const HEADER_BY_PAYLOAD_HASH_ROUTE: &str =
        "/v1/availability/header/payload-hash/{payload_hash}";
    pub const HEADER_RANGE_ROUTE: &str = "/v1/availability/header/{from}/{until}";

    pub const BLOCK_BY_HEIGHT_ROUTE: &str = "/v1/availability/block/{height}";
    pub const BLOCK_BY_HASH_ROUTE: &str = "/v1/availability/block/hash/{hash}";
    pub const BLOCK_BY_PAYLOAD_HASH_ROUTE: &str =
        "/v1/availability/block/payload-hash/{payload_hash}";
    pub const BLOCK_RANGE_ROUTE: &str = "/v1/availability/block/{from}/{until}";

    pub const PAYLOAD_BY_HEIGHT_ROUTE: &str = "/v1/availability/payload/{height}";
    pub const PAYLOAD_BY_HASH_ROUTE: &str = "/v1/availability/payload/hash/{hash}";
    pub const PAYLOAD_BY_BLOCK_HASH_ROUTE: &str =
        "/v1/availability/payload/block-hash/{block_hash}";
    pub const PAYLOAD_RANGE_ROUTE: &str = "/v1/availability/payload/{from}/{until}";

    pub const VID_COMMON_BY_HEIGHT_ROUTE: &str = "/v1/availability/vid/common/{height}";
    pub const VID_COMMON_BY_HASH_ROUTE: &str = "/v1/availability/vid/common/hash/{hash}";
    pub const VID_COMMON_BY_PAYLOAD_HASH_ROUTE: &str =
        "/v1/availability/vid/common/payload-hash/{payload_hash}";
    pub const VID_COMMON_RANGE_ROUTE: &str = "/v1/availability/vid/common/{from}/{until}";

    pub const TRANSACTION_BY_POSITION_NOPROOF_ROUTE: &str =
        "/v1/availability/transaction/{height}/{index}/noproof";
    pub const TRANSACTION_BY_HASH_NOPROOF_ROUTE: &str =
        "/v1/availability/transaction/hash/{hash}/noproof";
    pub const TRANSACTION_PROOF_BY_POSITION_ROUTE: &str =
        "/v1/availability/transaction/{height}/{index}/proof";
    pub const TRANSACTION_PROOF_BY_HASH_ROUTE: &str =
        "/v1/availability/transaction/hash/{hash}/proof";
    pub const TRANSACTION_BY_POSITION_ROUTE: &str = "/v1/availability/transaction/{height}/{index}";
    pub const TRANSACTION_BY_HASH_ROUTE: &str = "/v1/availability/transaction/hash/{hash}";

    pub const BLOCK_SUMMARY_BY_HEIGHT_ROUTE: &str = "/v1/availability/block/summary/{height}";
    pub const BLOCK_SUMMARY_RANGE_ROUTE: &str = "/v1/availability/block/summaries/{from}/{until}";

    pub const LIMITS_ROUTE: &str = "/v1/availability/limits";
    pub const CERT2_BY_HEIGHT_ROUTE: &str = "/v1/availability/cert2/{height}";
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
}
