//! Route path definitions shared between the axum router and clients that build request URLs.
//!
//! v1 routes are registered inline at their handlers in [`super::routers`]; only the version
//! prefix and docs paths need naming here. v2 keeps a [`Route`] table (path + gRPC + OpenAPI
//! metadata) because those routes are shared with the Tonic gRPC service.

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

pub mod v1 {
    /// Version prefix under which every route in this module is mounted (see
    /// `axum::finish_v1_docs`). Routes register version-agnostic paths; the prefix is applied once,
    /// at mount time (a single `.nest`), so bumping the served version is a change here rather than
    /// across every route. The legacy-URI rewrite layer maps an unversioned request onto this
    /// prefix.
    pub const VERSION_PREFIX: &str = "/v1";

    // API docs, mounted under `VERSION_PREFIX` alongside the routes: `SWAGGER_ROUTE` ("/") is
    // served at the bare `/v1`. Use `openapi_spec_url()` for the absolute URL the docs pages fetch.
    pub const OPENAPI_SPEC_ROUTE: &str = "/docs/openapi.json";
    pub const SWAGGER_ROUTE: &str = "/";
    pub const SCALAR_ROUTE: &str = "/scalar";

    /// Absolute, browser-facing path of the v1 OpenAPI spec
    /// (`VERSION_PREFIX` + [`OPENAPI_SPEC_ROUTE`]), for the Swagger/Scalar pages that fetch it.
    pub fn openapi_spec_url() -> String {
        format!("{VERSION_PREFIX}{OPENAPI_SPEC_ROUTE}")
    }

    /// Client-side builders for the routes that peer-to-peer catchup requests by URL. Each returns
    /// the full `/v1/...` path so a peer client can `join` it onto its base URL.
    pub fn config_hotshot() -> String {
        format!("{VERSION_PREFIX}/config/hotshot")
    }
    pub fn reward_merkle_tree_v2(height: impl std::fmt::Display) -> String {
        format!("{VERSION_PREFIX}/reward-state-v2/reward-merkle-tree-v2/{height}")
    }
    pub fn catchup_accounts(
        height: impl std::fmt::Display,
        view: impl std::fmt::Display,
    ) -> String {
        format!("{VERSION_PREFIX}/catchup/{height}/{view}/accounts")
    }
    pub fn catchup_blocks(height: impl std::fmt::Display, view: impl std::fmt::Display) -> String {
        format!("{VERSION_PREFIX}/catchup/{height}/{view}/blocks")
    }
    pub fn catchup_chainconfig(commitment: impl std::fmt::Display) -> String {
        format!("{VERSION_PREFIX}/catchup/chain-config/{commitment}")
    }
    pub fn catchup_leafchain(height: impl std::fmt::Display) -> String {
        format!("{VERSION_PREFIX}/catchup/{height}/leafchain")
    }
    pub fn catchup_cert2(height: impl std::fmt::Display) -> String {
        format!("{VERSION_PREFIX}/catchup/{height}/cert2")
    }
    pub fn catchup_reward_accounts(
        height: impl std::fmt::Display,
        view: impl std::fmt::Display,
    ) -> String {
        format!("{VERSION_PREFIX}/catchup/{height}/{view}/reward-accounts")
    }
    pub fn catchup_reward_merkle_tree_v2(
        height: impl std::fmt::Display,
        view: impl std::fmt::Display,
    ) -> String {
        format!("{VERSION_PREFIX}/catchup/reward-merkle-tree-v2/{height}/{view}")
    }
    pub fn catchup_state_cert(epoch: impl std::fmt::Display) -> String {
        format!("{VERSION_PREFIX}/catchup/{epoch}/state-cert")
    }
}

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
